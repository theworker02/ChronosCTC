use crate::config::AgentConfig;
use crate::error::{AgentError, AgentResult};
use crate::probe::{CorrectionVector, ProbeFinding, ProbeKind};
use ctc_dag::{Epoch, SpacetimeAddr};
use ctc_ledger::{OmniversalLedger, UniverseId, UniverseStatus};
use ctc_signal::{ExpectedFootprint, PayloadCell, SignalDaemon};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// Scans for paradox / residual divergence.
    ParadoxWarden,
    /// Audits convergence quality and weight mass.
    ConvergenceAuditor,
    /// Speculatively probes future failure modes.
    FutureScout,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentReport {
    pub agent: u64,
    pub role: AgentRole,
    pub universe: u64,
    pub findings: Vec<ProbeFinding>,
    pub corrections: Vec<CorrectionVector>,
    pub injected: usize,
}

/// Autonomous chronal agent bound to a home universe, free to traverse others.
#[derive(Clone)]
pub struct ChronalAgent {
    pub id: AgentId,
    pub role: AgentRole,
    pub home: UniverseId,
    pub config: AgentConfig,
    pub active: bool,
    /// Non-deterministic seed for exploration jitter.
    pub seed: u64,
}

impl ChronalAgent {
    pub fn new(id: AgentId, role: AgentRole, home: UniverseId, config: AgentConfig, seed: u64) -> Self {
        Self {
            id,
            role,
            home,
            config,
            active: true,
            seed,
        }
    }

    /// Explore a universe branch and optionally inject corrections via signal.
    pub fn explore(
        &self,
        ledger: &OmniversalLedger,
        target: UniverseId,
        signal: Option<&SignalDaemon>,
    ) -> AgentResult<AgentReport> {
        if !self.active {
            return Err(AgentError::Decommissioned(self.id.0));
        }
        let status = ledger
            .status(target)
            .ok_or(AgentError::UnnavigableUniverse(target.0))?;
        if !matches!(status, UniverseStatus::Active | UniverseStatus::Condemned) {
            return Err(AgentError::UnnavigableUniverse(target.0));
        }

        let mut findings = Vec::new();
        let mut corrections = Vec::new();

        let residual = ledger.residual(target).unwrap_or(0.0);
        let weight = ledger.weight(target).unwrap_or(0.0);
        let fp = ledger.fixed_point(target).unwrap_or_default();

        match self.role {
            AgentRole::ParadoxWarden => {
                if residual > self.config.paradox_residual {
                    findings.push(ProbeFinding {
                        kind: ProbeKind::ParadoxScan,
                        universe: target.0,
                        severity: residual,
                        message: format!("residual {residual:.3e} exceeds paradox gate"),
                        needs_correction: true,
                    });
                } else {
                    findings.push(ProbeFinding {
                        kind: ProbeKind::ParadoxScan,
                        universe: target.0,
                        severity: residual,
                        message: "paradox scan clear".into(),
                        needs_correction: false,
                    });
                }
            }
            AgentRole::ConvergenceAuditor => {
                let suboptimal = weight < self.config.suboptimal_weight;
                findings.push(ProbeFinding {
                    kind: ProbeKind::ConvergenceAudit,
                    universe: target.0,
                    severity: if suboptimal { 1.0 - weight } else { 0.0 },
                    message: format!("weight={weight:.4} residual={residual:.3e}"),
                    needs_correction: suboptimal,
                });
                if suboptimal {
                    // Nudge primary coordinate toward higher-weight basin centroid (0.5).
                    if let Some(v0) = fp.first() {
                        let delta = (0.5 - v0) * self.config.correction_scale;
                        corrections.push(CorrectionVector {
                            universe: target,
                            target_tau: 0,
                            address: 0,
                            delta,
                            reason: "suboptimal weight — micro-optimize toward basin center".into(),
                        });
                    }
                }
            }
            AgentRole::FutureScout => {
                // Non-deterministic future-failure probe using seed jitter.
                let jitter = ((self.seed.wrapping_mul(target.0.wrapping_add(1))) % 1000) as f64 / 1000.0;
                let risk = residual * 0.1 + jitter * 0.05;
                let failure = risk > 0.04;
                findings.push(ProbeFinding {
                    kind: ProbeKind::FutureFailureProbe,
                    universe: target.0,
                    severity: risk,
                    message: if failure {
                        "projected future deadlock / divergence risk".into()
                    } else {
                        "future failure probe nominal".into()
                    },
                    needs_correction: failure,
                });
                if failure {
                    if let Some(v0) = fp.first() {
                        corrections.push(CorrectionVector {
                            universe: target,
                            target_tau: 0,
                            address: 0,
                            delta: -v0.signum() * self.config.correction_scale * risk,
                            reason: "preemptive damping against projected failure".into(),
                        });
                    }
                }
            }
        }

        let mut injected = 0usize;
        if let Some(daemon) = signal {
            for c in &corrections {
                if self.inject_correction(ledger, daemon, c)? {
                    injected += 1;
                }
            }
        }

        Ok(AgentReport {
            agent: self.id.0,
            role: self.role,
            universe: target.0,
            findings,
            corrections,
            injected,
        })
    }

    fn inject_correction(
        &self,
        ledger: &OmniversalLedger,
        daemon: &SignalDaemon,
        corr: &CorrectionVector,
    ) -> AgentResult<bool> {
        let binding = ledger
            .with_universe(corr.universe, |u| daemon.bind(&u.dag))
            .map_err(|e| AgentError::Ledger(e.to_string()))?;

        let addr = SpacetimeAddr::new(corr.address, corr.target_tau);
        let current = ledger
            .with_universe(corr.universe, |u| {
                u.dag
                    .lookup(addr)
                    .map(|n| n.state.value.first().copied().unwrap_or(0.0))
                    .unwrap_or(0.0)
            })
            .map_err(|e| AgentError::Ledger(e.to_string()))?;

        let new_val = current + corr.delta;
        let cells = vec![PayloadCell {
            addr,
            values: vec![new_val],
            blob: vec![],
        }];
        let fp = ExpectedFootprint::from_cells(Epoch(corr.target_tau), &cells, binding);
        daemon.register_footprint(fp);

        let packet = daemon
            .package_scalars(
                Epoch(corr.target_tau + 1),
                Epoch(corr.target_tau),
                binding,
                &[(addr, new_val)],
            )
            .map_err(|e| AgentError::Signal(e.to_string()))?;

        ledger
            .with_universe_mut(corr.universe, |u| {
                daemon
                    .transmit(&mut u.dag, &packet, None)
                    .map_err(|e| AgentError::Signal(e.to_string()))
            })
            .map_err(|e| AgentError::Ledger(e.to_string()))??;

        // Update stored fixed point coordinate if present.
        let _ = ledger.with_universe_mut(corr.universe, |u| {
            if let Some(slot) = u.fixed_point.get_mut(corr.address as usize) {
                *slot = new_val;
            }
        });

        Ok(true)
    }
}
