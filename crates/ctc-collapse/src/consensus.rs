use crate::config::CollapseConfig;
use crate::proof::ConsistencyProof;
use ctc_agents::FleetReport;
use ctc_ledger::UniverseId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectiveWeights {
    pub probability: f64,
    pub consistency: f64,
    pub entropy: f64,
    pub agent_health: f64,
}

impl From<&CollapseConfig> for ObjectiveWeights {
    fn from(c: &CollapseConfig) -> Self {
        Self {
            probability: c.w_probability,
            consistency: c.w_consistency,
            entropy: c.w_entropy,
            agent_health: c.w_agent_health,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BranchScore {
    pub universe: u64,
    pub probability: f64,
    pub consistency: f64,
    pub entropy: f64,
    pub agent_health: f64,
    /// Composite utility \(U = \sum w_i s_i\).
    pub utility: f64,
    pub proof_fingerprint: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusReport {
    pub scores: Vec<BranchScore>,
    pub winner: u64,
    pub runner_up: Option<u64>,
    pub margin: f64,
}

/// Score live branches under the master objective.
pub fn score_branches(
    cfg: &CollapseConfig,
    proofs: &[(UniverseId, ConsistencyProof)],
    fleet: Option<&FleetReport>,
) -> ConsensusReport {
    let obj = ObjectiveWeights::from(cfg);
    let mut scores = Vec::new();

    for (id, proof) in proofs {
        let probability = proof.weight;
        let consistency = 1.0 / (1.0 + proof.residual);
        // Shannon-ish: lower weight entropy contribution for peaked mass.
        let entropy = if probability > 0.0 && probability < 1.0 {
            let p = probability.clamp(1e-12, 1.0);
            -(p * p.ln())
        } else {
            0.0
        };
        let stability = 1.0 / (1.0 + entropy);

        let agent_health = fleet
            .map(|f| {
                let relevant: Vec<_> = f
                    .reports
                    .iter()
                    .filter(|r| r.universe == id.0)
                    .collect();
                if relevant.is_empty() {
                    return 0.5;
                }
                let clean = relevant
                    .iter()
                    .flat_map(|r| r.findings.iter())
                    .filter(|f| !f.needs_correction)
                    .count() as f64;
                let total = relevant
                    .iter()
                    .map(|r| r.findings.len())
                    .sum::<usize>()
                    .max(1) as f64;
                clean / total
            })
            .unwrap_or(0.5);

        let utility = obj.probability * probability
            + obj.consistency * consistency
            + obj.entropy * stability
            + obj.agent_health * agent_health;

        scores.push(BranchScore {
            universe: id.0,
            probability,
            consistency,
            entropy,
            agent_health,
            utility,
            proof_fingerprint: proof.fingerprint,
        });
    }

    scores.sort_by(|a, b| {
        b.utility
            .partial_cmp(&a.utility)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let winner = scores.first().map(|s| s.universe).unwrap_or(0);
    let runner_up = scores.get(1).map(|s| s.universe);
    let margin = match (scores.first(), scores.get(1)) {
        (Some(a), Some(b)) => a.utility - b.utility,
        (Some(a), None) => a.utility,
        _ => 0.0,
    };

    ConsensusReport {
        scores,
        winner,
        runner_up,
        margin,
    }
}
