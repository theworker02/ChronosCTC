use crate::error::{OracleError, OracleResult};
use crate::superposition::{SuperpositionState, TemporalSuperposition};
use crate::wait::{ensure_awaiting, WaitHandle};
use ctc_dag::{Epoch, LogicalAddr, SpacetimeAddr, WorldlineDag};
use ctc_signal::{
    ExpectedFootprint, PayloadCell, SignalDaemon, TemporalPacket, TransmitReport, WorldlineBinding,
};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Declared chronal injection point in a program.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InjectionPoint {
    pub name: String,
    pub epoch: Epoch,
    pub slots: Vec<(LogicalAddr, usize)>,
    /// Classical work estimate skipped when future packet arrives.
    pub estimated_cycles: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollapseReport {
    pub injection_point: String,
    pub packet_id: u64,
    pub cycles_saved: u64,
    pub polls: u64,
    pub transmit: TransmitReport,
    pub message: String,
}

/// Hook installed around a program block.
pub trait OracleHook: Send + Sync {
    fn on_arm(&self, point: &InjectionPoint);
    fn on_await(&self, point: &InjectionPoint);
    fn on_collapse(&self, report: &CollapseReport);
    fn on_abort(&self, point: &InjectionPoint, reason: &str);
}

/// No-op hook for headless execution.
pub struct NullHook;
impl OracleHook for NullHook {
    fn on_arm(&self, _: &InjectionPoint) {}
    fn on_await(&self, _: &InjectionPoint) {}
    fn on_collapse(&self, _: &CollapseReport) {}
    fn on_abort(&self, _: &InjectionPoint, _: &str) {}
}

/// Pre-cognitive interception engine.
pub struct OracleEngine {
    signal: Arc<SignalDaemon>,
    points: RwLock<FxHashMap<String, InjectionPoint>>,
    supers: RwLock<FxHashMap<String, TemporalSuperposition>>,
    hook: Arc<dyn OracleHook>,
    pub default_timeout: Option<Duration>,
}

impl OracleEngine {
    pub fn new(signal: Arc<SignalDaemon>) -> Self {
        Self {
            signal,
            points: RwLock::new(FxHashMap::default()),
            supers: RwLock::new(FxHashMap::default()),
            hook: Arc::new(NullHook),
            default_timeout: Some(Duration::from_millis(50)),
        }
    }

    pub fn with_hook(mut self, hook: Arc<dyn OracleHook>) -> Self {
        self.hook = hook;
        self
    }

    pub fn signal(&self) -> &Arc<SignalDaemon> {
        &self.signal
    }

    /// Register an injection point and its expected footprint on the DAG.
    pub fn register(
        &self,
        dag: &WorldlineDag,
        point: InjectionPoint,
    ) -> OracleResult<ExpectedFootprint> {
        let binding: WorldlineBinding = self.signal.bind(dag);
        // Template cells define the footprint hash the future packet must match.
        let template: Vec<PayloadCell> = point
            .slots
            .iter()
            .map(|(a, dim)| PayloadCell {
                addr: SpacetimeAddr {
                    address: *a,
                    tau: point.epoch,
                },
                values: vec![0.0; (*dim).max(1)],
                blob: Vec::new(),
            })
            .collect();
        let fp = ExpectedFootprint::from_cells(point.epoch, &template, binding);
        self.signal.register_footprint(fp.clone());

        let superpos =
            TemporalSuperposition::new(point.name.clone(), point.epoch, &fp, point.estimated_cycles);
        self.hook.on_arm(&point);
        self.supers.write().insert(point.name.clone(), superpos);
        self.points.write().insert(point.name.clone(), point);
        Ok(fp)
    }

    /// Enter temporal superposition — wait for a matching future packet, then collapse.
    pub fn intercept_and_collapse(
        &self,
        dag: &mut WorldlineDag,
        name: &str,
        // Producer called while waiting — runs the future epoch and stages a packet.
        mut produce_future: impl FnMut(&SignalDaemon) -> OracleResult<()>,
    ) -> OracleResult<CollapseReport> {
        let point = self
            .points
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| OracleError::UnknownInjectionPoint(name.into()))?;

        {
            let mut supers = self.supers.write();
            let s = supers
                .get_mut(name)
                .ok_or_else(|| OracleError::UnknownInjectionPoint(name.into()))?;
            s.enter_wait();
        }
        self.hook.on_await(&point);

        // Produce the future payload (in-process stand-in for τ_{n+1} completion).
        produce_future(&self.signal)?;

        let mut wait = WaitHandle::new(name, self.default_timeout);
        let target = point.epoch;
        let (packet, polls) = {
            let polls = wait.wait_until(|| self.signal.peek_for_target(target).is_some())?;
            let pkt = self
                .signal
                .peek_for_target(target)
                .ok_or_else(|| OracleError::WaitTimeout(name.into()))?;
            let _ = self.signal.take(pkt.id);
            (pkt, polls)
        };

        self.collapse_with_packet(dag, name, packet, polls)
    }

    /// Collapse using an already-available packet (mesh delivery path).
    pub fn collapse_with_packet(
        &self,
        dag: &mut WorldlineDag,
        name: &str,
        packet: TemporalPacket,
        polls: u64,
    ) -> OracleResult<CollapseReport> {
        {
            let supers = self.supers.read();
            let s = supers
                .get(name)
                .ok_or_else(|| OracleError::UnknownInjectionPoint(name.into()))?;
            ensure_awaiting(s.state, name)?;
            if packet.footprint_hash != s.footprint_hash {
                return Err(OracleError::Signal(format!(
                    "packet footprint {:#x} != registered {:#x}",
                    packet.footprint_hash, s.footprint_hash
                )));
            }
        }

        let transmit = self
            .signal
            .transmit(dag, &packet, None)
            .map_err(|e| OracleError::Signal(e.to_string()))?;

        let cycles_saved = {
            let mut supers = self.supers.write();
            let s = supers.get_mut(name).unwrap();
            s.collapse();
            s.cycles_saved
        };

        let report = CollapseReport {
            injection_point: name.into(),
            packet_id: packet.id,
            cycles_saved,
            polls,
            transmit,
            message: format!(
                "pre-cognitive collapse at {} — skipped {} cycles (packet {})",
                name, cycles_saved, packet.id
            ),
        };
        self.hook.on_collapse(&report);
        Ok(report)
    }

    pub fn abort(&self, name: &str, reason: &str) -> OracleResult<()> {
        let point = self
            .points
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| OracleError::UnknownInjectionPoint(name.into()))?;
        if let Some(s) = self.supers.write().get_mut(name) {
            s.abort();
        }
        self.hook.on_abort(&point, reason);
        Ok(())
    }

    pub fn status(&self, name: &str) -> Option<SuperpositionState> {
        self.supers.read().get(name).map(|s| s.state)
    }

    /// Transition an armed injection point into the Awaiting superposition state.
    ///
    /// Used by `ctc-mesh` delivery paths where the future packet arrives over
    /// the network rather than through [`intercept_and_collapse`].
    pub fn enter_await(&self, name: &str) -> OracleResult<()> {
        let point = self
            .points
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| OracleError::UnknownInjectionPoint(name.into()))?;
        {
            let mut supers = self.supers.write();
            let s = supers
                .get_mut(name)
                .ok_or_else(|| OracleError::UnknownInjectionPoint(name.into()))?;
            if s.state == SuperpositionState::Collapsed {
                return Err(OracleError::AlreadyCollapsed(name.into()));
            }
            s.enter_wait();
        }
        self.hook.on_await(&point);
        Ok(())
    }

    /// Stage a scalar state-differential packet for a registered point.
    pub fn stage_future_scalars(
        &self,
        name: &str,
        source_tau: Epoch,
        values: &[(LogicalAddr, f64)],
    ) -> OracleResult<u64> {
        let superpos = self
            .supers
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| OracleError::UnknownInjectionPoint(name.into()))?;

        let pairs: Vec<_> = values
            .iter()
            .map(|(a, v)| {
                (
                    SpacetimeAddr {
                        address: *a,
                        tau: superpos.epoch,
                    },
                    *v,
                )
            })
            .collect();

        let packet = self
            .signal
            .package_scalars(source_tau, superpos.epoch, superpos.binding, &pairs)
            .map_err(|e| OracleError::Signal(e.to_string()))?;

        if packet.footprint_hash != superpos.footprint_hash {
            return Err(OracleError::Signal(format!(
                "staged footprint {:#x} != registered {:#x}",
                packet.footprint_hash, superpos.footprint_hash
            )));
        }

        Ok(self.signal.stage(packet))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_dag::NodeState;
    use ctc_signal::SignalConfig;

    #[test]
    fn precognitive_collapse_skips_cycles() {
        let signal = Arc::new(SignalDaemon::new(SignalConfig::default()));
        let oracle = OracleEngine::new(Arc::clone(&signal));
        let mut dag = WorldlineDag::new();
        let addr = LogicalAddr(7);
        dag.allocate(
            SpacetimeAddr {
                address: addr,
                tau: Epoch(0),
            },
            NodeState::scalar(0.0),
        );
        dag.allocate(
            SpacetimeAddr {
                address: addr,
                tau: Epoch(3),
            },
            NodeState::scalar(0.0),
        );

        oracle
            .register(
                &dag,
                InjectionPoint {
                    name: "opt_entry".into(),
                    epoch: Epoch(0),
                    slots: vec![(addr, 1)],
                    estimated_cycles: 50_000,
                },
            )
            .unwrap();

        let report = oracle
            .intercept_and_collapse(&mut dag, "opt_entry", |_sig| {
                oracle
                    .stage_future_scalars("opt_entry", Epoch(3), &[(addr, 0.875)])
                    .map(|_| ())
            })
            .unwrap();

        assert_eq!(report.cycles_saved, 50_000);
        assert_eq!(
            oracle.status("opt_entry"),
            Some(SuperpositionState::Collapsed)
        );
        let v = dag
            .lookup(SpacetimeAddr {
                address: addr,
                tau: Epoch(0),
            })
            .unwrap()
            .state
            .value[0];
        assert!((v - 0.875).abs() < 1e-12);
    }
}
