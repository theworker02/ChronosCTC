use crate::agent::{AgentId, AgentReport, AgentRole, ChronalAgent};
use crate::config::AgentConfig;
use crate::error::{AgentError, AgentResult};
use ctc_ledger::{OmniversalLedger, UniverseId};
use ctc_signal::SignalDaemon;
use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FleetReport {
    pub agents_deployed: usize,
    pub universes_probed: usize,
    pub findings: usize,
    pub corrections_injected: usize,
    pub reports: Vec<AgentReport>,
}

/// Fleet manager for autonomous chronal agents.
pub struct AgentFleet {
    pub config: AgentConfig,
    next_id: RwLock<u64>,
    agents: RwLock<FxHashMap<u64, ChronalAgent>>,
    signal: Option<Arc<SignalDaemon>>,
}

impl AgentFleet {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            next_id: RwLock::new(1),
            agents: RwLock::new(FxHashMap::default()),
            signal: None,
        }
    }

    pub fn with_signal(mut self, signal: Arc<SignalDaemon>) -> Self {
        self.signal = Some(signal);
        self
    }

    pub fn spawn(&self, role: AgentRole, home: UniverseId) -> AgentId {
        let mut n = self.next_id.write();
        let id = AgentId(*n);
        let seed = (*n).wrapping_mul(2654435761);
        *n += 1;
        let agent = ChronalAgent::new(id, role, home, self.config.clone(), seed);
        self.agents.write().insert(id.0, agent);
        id
    }

    /// Deploy the default triad (warden, auditor, scout) onto each child universe.
    pub fn deploy_triad(&self, homes: &[UniverseId]) -> Vec<AgentId> {
        let mut ids = Vec::new();
        for home in homes {
            ids.push(self.spawn(AgentRole::ParadoxWarden, *home));
            ids.push(self.spawn(AgentRole::ConvergenceAuditor, *home));
            ids.push(self.spawn(AgentRole::FutureScout, *home));
        }
        ids
    }

    /// Run one exploration tick across all active agents on their home universes.
    pub fn explore_all(&self, ledger: &OmniversalLedger) -> AgentResult<FleetReport> {
        let agents: Vec<ChronalAgent> = self
            .agents
            .read()
            .values()
            .filter(|a| a.active)
            .cloned()
            .collect();

        let mut report = FleetReport {
            agents_deployed: agents.len(),
            ..FleetReport::default()
        };
        let mut probed = FxHashSet::default();
        let signal = self.signal.as_deref();

        for agent in &agents {
            let r = agent.explore(ledger, agent.home, signal)?;
            probed.insert(r.universe);
            report.findings += r.findings.len();
            report.corrections_injected += r.injected;
            report.reports.push(r);
        }
        report.universes_probed = probed.len();
        Ok(report)
    }

    pub fn decommission(&self, id: AgentId) -> AgentResult<()> {
        let mut agents = self.agents.write();
        let a = agents
            .get_mut(&id.0)
            .ok_or(AgentError::UnknownAgent(id.0))?;
        a.active = false;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.agents.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.read().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_dag::WorldlineDag;
    use ctc_kernel::{ConvergenceClass, FixedPointSolution, SolverStats};
    use ctc_ledger::{ForkCause, OmniversalLedger};

    #[test]
    fn triad_probes_bifurcated_universes() {
        let ledger = OmniversalLedger::default();
        let root = ledger.bootstrap("prime", WorldlineDag::new());
        let sol = FixedPointSolution {
            class: ConvergenceClass::MultiWeighted,
            states: vec![vec![0.1], vec![0.9]],
            weights: vec![0.8, 0.2],
            stats: SolverStats {
                iterations: 4,
                final_residual: 1e-12,
                restarts_used: 2,
                fixed_points_found: 2,
            },
        };
        let event = ledger
            .bifurcate_from_solution(root, &sol, ForkCause::MultiFixedPoint)
            .unwrap();

        let fleet = AgentFleet::new(AgentConfig::default());
        fleet.deploy_triad(&event.children);
        let report = fleet.explore_all(&ledger).unwrap();
        assert_eq!(report.agents_deployed, 6);
        assert!(report.findings >= 6);
    }
}
