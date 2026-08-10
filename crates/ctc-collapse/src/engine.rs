use crate::config::CollapseConfig;
use crate::consensus::{score_branches, ConsensusReport};
use crate::error::{CollapseError, CollapseResult};
use crate::proof::ProofOfConsistency;
use ctc_agents::FleetReport;
use ctc_dag::{NodeState, SpacetimeAddr, WorldlineDag};
use ctc_ledger::{OmniversalLedger, UniverseId, UniverseStatus};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealitySynthesisReport {
    pub consensus: ConsensusReport,
    pub winner: u64,
    pub merged_into_primary: u64,
    pub pruned: Vec<u64>,
    pub condemned: Vec<u64>,
    pub message: String,
}

/// Multiversal collapse engine — Proof-of-Consistency + entropy-gradient selection.
pub struct CollapseEngine {
    pub config: CollapseConfig,
    pub prover: ProofOfConsistency,
}

impl Default for CollapseEngine {
    fn default() -> Self {
        Self::new(CollapseConfig::default())
    }
}

impl CollapseEngine {
    pub fn new(config: CollapseConfig) -> Self {
        Self {
            prover: ProofOfConsistency::default(),
            config,
        }
    }

    /// Evaluate all live universes, pick a winner, synthesize into primary, purge losers.
    pub fn synthesize(
        &self,
        ledger: &OmniversalLedger,
        fleet: Option<&FleetReport>,
    ) -> CollapseResult<RealitySynthesisReport> {
        let all_live = ledger.live_ids();
        if all_live.is_empty() {
            return Err(CollapseError::EmptyMultiverse);
        }

        // Score forked child universes only — the primary trunk is the merge
        // target, not a competing reality (it retains weight 1.0 by design).
        let mut candidates: Vec<UniverseId> = all_live
            .iter()
            .copied()
            .filter(|id| {
                ledger
                    .with_universe(*id, |u| u.parent.is_some())
                    .unwrap_or(false)
            })
            .collect();
        if candidates.is_empty() {
            candidates = all_live.clone();
        }

        let mut proofs = Vec::new();
        let mut condemned = Vec::new();
        for id in &candidates {
            match self.prover.prove(ledger, *id) {
                Ok(p) => proofs.push((*id, p)),
                Err(_) => {
                    let _ = ledger.condemn(*id);
                    condemned.push(id.0);
                }
            }
        }
        if proofs.is_empty() {
            return Err(CollapseError::EmptyMultiverse);
        }

        let consensus = score_branches(&self.config, &proofs, fleet);
        // Exact ties (flat Deutsch mixture) collapse via deterministic sort order.
        // A positive but insufficient margin is true ambiguity — refuse to synthesize.
        if consensus.runner_up.is_some()
            && consensus.margin > 1e-9
            && consensus.margin < self.config.consensus_margin
        {
            return Err(CollapseError::AmbiguousConsensus {
                margin: consensus.margin,
                threshold: self.config.consensus_margin,
            });
        }

        let winner = UniverseId(consensus.winner);

        // Merge winner's fixed point into the primary universe manifold.
        let primary = ledger.primary();
        let winner_fp = ledger
            .fixed_point(winner)
            .ok_or(CollapseError::Ledger(format!("winner {}", winner.0)))?;
        let winner_residual = ledger.residual(winner).unwrap_or(0.0);

        ledger
            .with_universe_mut(primary, |u| {
                for (i, v) in winner_fp.iter().enumerate() {
                    let addr = SpacetimeAddr::new(i as u64, 0);
                    if u.dag.lookup(addr).is_none() {
                        u.dag.allocate(addr, NodeState::scalar(*v));
                    } else {
                        let _ = u.dag.retro_write(addr, Arc::from([*v]));
                    }
                }
                u.fixed_point = winner_fp.clone();
                u.weight = 1.0;
                u.residual = winner_residual;
            })
            .map_err(|e| CollapseError::Ledger(e.to_string()))?;

        // If winner is not primary, mark it collapsed (merged).
        if winner != primary {
            let _ = ledger.mark_collapsed(winner);
            let _ = ledger.set_primary(primary);
        }

        let mut pruned = Vec::new();
        if self.config.purge_losers {
            for id in all_live {
                if id == primary {
                    continue;
                }
                // Purge non-winning forks (including the winner child after merge).
                if matches!(
                    ledger.status(id),
                    Some(UniverseStatus::Active)
                        | Some(UniverseStatus::Condemned)
                        | Some(UniverseStatus::Collapsed)
                ) {
                    let _ = ledger.mark_pruned(id);
                    pruned.push(id.0);
                }
            }
        }

        Ok(RealitySynthesisReport {
            message: format!(
                "reality synthesized: winner=U{} → primary=U{} (margin={:.4}, pruned={})",
                consensus.winner,
                primary.0,
                consensus.margin,
                pruned.len()
            ),
            consensus,
            winner: winner.0,
            merged_into_primary: primary.0,
            pruned,
            condemned,
        })
    }

    /// Convenience: export the synthesized primary manifold.
    pub fn export_primary_manifold(&self, ledger: &OmniversalLedger) -> CollapseResult<WorldlineDag> {
        let primary = ledger.primary();
        ledger
            .with_universe(primary, |u| {
                let mut out = WorldlineDag::new();
                if let Ok(snap) = u.dag.snapshot() {
                    for (addr, val, weight) in snap.nodes {
                        out.allocate(
                            addr,
                            NodeState {
                                value: Arc::clone(&val),
                                weight,
                                revision: 0,
                                pruned: false,
                            },
                        );
                    }
                }
                out
            })
            .map_err(|e| CollapseError::Ledger(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_agents::{AgentConfig, AgentFleet};
    use ctc_kernel::{ConvergenceClass, FixedPointSolution, SolverStats};
    use ctc_ledger::{ForkCause, OmniversalLedger};

    #[test]
    fn collapses_multiverse_onto_highest_weight_branch() {
        let ledger = OmniversalLedger::default();
        let root = ledger.bootstrap("prime", WorldlineDag::new());
        let sol = FixedPointSolution {
            class: ConvergenceClass::MultiWeighted,
            states: vec![vec![0.1], vec![0.7], vec![0.9]],
            weights: vec![0.1, 0.7, 0.2],
            stats: SolverStats {
                iterations: 8,
                final_residual: 1e-12,
                restarts_used: 4,
                fixed_points_found: 3,
            },
        };
        let event = ledger
            .bifurcate_from_solution(root, &sol, ForkCause::MultiFixedPoint)
            .unwrap();

        let fleet = AgentFleet::new(AgentConfig::default());
        fleet.deploy_triad(&event.children);
        let fleet_report = fleet.explore_all(&ledger).unwrap();

        let engine = CollapseEngine::new(CollapseConfig {
            consensus_margin: 0.01,
            ..CollapseConfig::default()
        });
        let report = engine
            .synthesize(&ledger, Some(&fleet_report))
            .unwrap();

        assert_eq!(report.winner, event.children[1].0); // weight 0.7
        let primary_fp = ledger.fixed_point(ledger.primary()).unwrap();
        assert!((primary_fp[0] - 0.7).abs() < 1e-9);
        assert!(!report.pruned.is_empty());
    }
}
