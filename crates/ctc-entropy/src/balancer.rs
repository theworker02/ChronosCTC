use crate::config::EntropyConfig;
use crate::error::{EntropyError, EntropyResult};
use crate::landauer::{
    event_for_convergence, event_for_erase, event_for_prune, LandauerOp, ThermoEvent,
};
use ctc_collapse::RealitySynthesisReport;
use ctc_dag::WorldlineDag;
use ctc_gc::CollectionReport;
use ctc_holo::ProjectionReport;
use ctc_ledger::{OmniversalLedger, UniverseId};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EnergyLedger {
    /// Cumulative dissipated energy (J).
    pub dissipated_j: f64,
    /// Cumulative harvested energy (J).
    pub harvested_j: f64,
    /// Net work = dissipated - harvested.
    pub net_work_j: f64,
    pub events: usize,
}

impl EnergyLedger {
    pub fn apply(&mut self, ev: &ThermoEvent) {
        if ev.signed_work_j >= 0.0 {
            self.dissipated_j += ev.signed_work_j;
        } else {
            self.harvested_j += -ev.signed_work_j;
        }
        self.net_work_j = self.dissipated_j - self.harvested_j;
        self.events += 1;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThermoReport {
    pub ledger: EnergyLedger,
    pub last_events: Vec<ThermoEvent>,
    pub zero_energy_converged: bool,
    pub message: String,
}

/// Balances thermodynamic cost of chronal operations against harvested free energy.
pub struct ThermoBalancer {
    pub config: EntropyConfig,
    ledger: RwLock<EnergyLedger>,
}

impl Default for ThermoBalancer {
    fn default() -> Self {
        Self::new(EntropyConfig::default())
    }
}

impl ThermoBalancer {
    pub fn new(config: EntropyConfig) -> Self {
        Self {
            config,
            ledger: RwLock::new(EnergyLedger::default()),
        }
    }

    pub fn snapshot(&self) -> EnergyLedger {
        self.ledger.read().clone()
    }

    pub fn record(&self, ev: ThermoEvent) -> ThermoEvent {
        self.ledger.write().apply(&ev);
        ev
    }

    /// Charge Landauer cost for erasing `bits` of information.
    pub fn erase_bits(&self, bits: f64, note: impl Into<String>) -> EntropyResult<ThermoEvent> {
        let ev = event_for_erase(&self.config, bits, note)?;
        Ok(self.record(ev))
    }

    /// Thermodynamic accounting for pruning a reality branch.
    pub fn prune_branch(&self, cells: usize) -> EntropyResult<ThermoEvent> {
        let ev = event_for_prune(&self.config, cells)?;
        Ok(self.record(ev))
    }

    /// Harvest free energy from residual contraction toward a fixed point.
    pub fn converge(&self, residual_before: f64, residual_after: f64) -> ThermoEvent {
        let ev = event_for_convergence(&self.config, residual_before, residual_after);
        self.record(ev)
    }

    /// Couple holographic boundary entropy change to thermodynamic work.
    pub fn account_holographic(&self, proj: &ProjectionReport) -> EntropyResult<ThermoEvent> {
        // Boundary compression erases bulk degrees of freedom.
        let erased_bits = (proj.bulk_dim.saturating_sub(proj.boundary_dim)) as f64 * 64.0;
        let mut ev = event_for_erase(
            &self.config,
            erased_bits,
            format!(
                "holographic compression {:.2}× (S_EE={:.4})",
                1.0 / proj.compression_ratio.max(1e-12),
                proj.von_neumann
            ),
        )?;
        // Entanglement structure partially offsets erasure cost.
        let offset = proj.rt_entropy * self.config.boltzmann_j_per_k * self.config.temperature_k;
        ev.signed_work_j = (ev.energy_j - offset).max(0.0);
        ev.op = LandauerOp::EraseBits;
        Ok(self.record(ev))
    }

    /// Account for a multiversal collapse synthesis (nominal cell counts).
    pub fn account_collapse(&self, synthesis: &RealitySynthesisReport) -> EntropyResult<ThermoReport> {
        self.account_collapse_detailed(synthesis, None)
    }

    /// Account collapse using real DAG cell counts from the omniversal ledger.
    pub fn account_collapse_with_ledger(
        &self,
        synthesis: &RealitySynthesisReport,
        omni: &OmniversalLedger,
    ) -> EntropyResult<ThermoReport> {
        self.account_collapse_detailed(synthesis, Some(omni))
    }

    fn account_collapse_detailed(
        &self,
        synthesis: &RealitySynthesisReport,
        omni: Option<&OmniversalLedger>,
    ) -> EntropyResult<ThermoReport> {
        let mut events = Vec::new();
        for id in &synthesis.pruned {
            let cells = omni
                .map(|l| universe_cell_count(l, UniverseId(*id)))
                .unwrap_or(8)
                .max(1);
            events.push(self.prune_branch(cells)?);
        }
        // Winner merge converges residual toward zero-energy state.
        let resid = synthesis
            .consensus
            .scores
            .iter()
            .find(|s| s.universe == synthesis.winner)
            .map(|s| 1.0 / s.consistency.max(1e-12) - 1.0)
            .unwrap_or(0.0);
        events.push(self.converge(resid + 1.0, resid));

        let ledger = self.snapshot();
        let zero_energy_converged = resid <= self.config.zero_energy_residual;
        Ok(ThermoReport {
            message: format!(
                "thermo balance: net={:.6e} J  harvested={:.6e}  dissipated={:.6e}  zero_E={}",
                ledger.net_work_j,
                ledger.harvested_j,
                ledger.dissipated_j,
                zero_energy_converged
            ),
            ledger,
            last_events: events,
            zero_energy_converged,
        })
    }

    /// Couple a timeline GC collection cycle into the Landauer ledger.
    pub fn account_collection(&self, report: &CollectionReport) -> EntropyResult<ThermoEvent> {
        let erased_cells = report.stats.nodes_pruned
            + report.stats.branches_culled
            + report.stats.nodes_sealed / 2;
        let bits = (erased_cells.max(1) as f64) * 64.0;
        let mut ev = event_for_erase(
            &self.config,
            bits,
            format!(
                "GC collect pruned={} sealed={} culled={} reclaim≈{}B",
                report.stats.nodes_pruned,
                report.stats.nodes_sealed,
                report.stats.branches_culled,
                report.stats.bytes_reclaimed_est
            ),
        )?;
        // Pressure relief harvests free energy proportional to Δheap.
        let relief = (report.heap_pressure_before - report.heap_pressure_after).max(0.0);
        let harvest = relief * self.config.harvest_per_residual * 1e3;
        ev.signed_work_j -= harvest;
        ev.op = LandauerOp::EraseBits;
        Ok(self.record(ev))
    }

    /// Account pruning a single universe by measuring its manifold size.
    pub fn account_universe_prune(
        &self,
        ledger: &OmniversalLedger,
        id: UniverseId,
    ) -> EntropyResult<ThermoEvent> {
        let cells = universe_cell_count(ledger, id).max(1);
        self.prune_branch(cells)
    }

    /// Hint for GC aggressiveness from the energy ledger.
    ///
    /// - Net dissipation → raise heap pressure (cull)
    /// - Net harvest → retain (lower effective pressure)
    pub fn gc_pressure_hint(&self, base_pressure: f64) -> f64 {
        let snap = self.snapshot();
        let scale = if snap.net_work_j > 0.0 {
            1.0 + (snap.dissipated_j / (snap.dissipated_j + snap.harvested_j + 1e-40)).min(0.5)
        } else {
            (1.0 - (snap.harvested_j / (snap.dissipated_j + snap.harvested_j + 1e-40)).min(0.4))
                .max(0.35)
        };
        (base_pressure * scale).clamp(0.0, 1.0)
    }

    /// Drive toward equilibrium: require residual ≤ zero-energy gate.
    pub fn assert_equilibrium(&self, residual: f64) -> EntropyResult<()> {
        if residual > self.config.zero_energy_residual {
            return Err(EntropyError::EquilibriumUnreachable(residual));
        }
        Ok(())
    }
}

fn universe_cell_count(ledger: &OmniversalLedger, id: UniverseId) -> usize {
    ledger
        .with_universe(id, |u| dag_cell_count(&u.dag))
        .unwrap_or(0)
}

fn dag_cell_count(dag: &WorldlineDag) -> usize {
    match dag.snapshot() {
        Ok(snap) => snap.nodes.len(),
        Err(_) => dag.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landauer_erase_costs_positive_energy() {
        let bal = ThermoBalancer::default();
        let ev = bal.erase_bits(8.0, "test").unwrap();
        assert!(ev.energy_j > 0.0);
        assert!(bal.snapshot().dissipated_j > 0.0);
    }

    #[test]
    fn convergence_harvests_energy() {
        let bal = ThermoBalancer::default();
        let ev = bal.converge(1.0, 1e-12);
        assert!(ev.signed_work_j < 0.0);
        assert!(bal.snapshot().harvested_j > 0.0);
    }
}
