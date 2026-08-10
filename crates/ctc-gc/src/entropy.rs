use crate::config::GcConfig;
use ctc_kernel::{ConvergenceClass, FixedPointSolution};
use ctc_pruner::{BranchId, TimelineBranch};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BranchEntropy {
    pub branch: u64,
    /// Shannon entropy of the Deutsch mixture weights (0 for unique).
    pub mixture_entropy: f64,
    /// Total probability amplitude \(\sum w_i\).
    pub amplitude: f64,
    pub residual_norm: f64,
    /// Composite score \(H(b)\) — higher ⇒ likelier collection candidate.
    pub score: f64,
    pub toxic: bool,
    pub dead: bool,
}

/// Evaluates probability amplitudes and convergence health of timeline branches.
pub struct EntropyBalancer {
    pub config: GcConfig,
}

impl EntropyBalancer {
    pub fn new(config: GcConfig) -> Self {
        Self { config }
    }

    pub fn score_branch(&self, branch: &TimelineBranch) -> BranchEntropy {
        let amplitude = if branch.stable { 1.0 } else { 0.0 };
        let mixture_entropy = 0.0;
        self.compose(
            branch.id.0,
            mixture_entropy,
            amplitude,
            branch.residual_norm,
            &branch.status,
        )
    }

    pub fn score_solution(&self, branch: BranchId, solution: &FixedPointSolution) -> BranchEntropy {
        let amplitude: f64 = solution.weights.iter().sum();
        let mixture_entropy = shannon(&solution.weights);
        self.compose(
            branch.0,
            mixture_entropy,
            amplitude,
            solution.stats.final_residual,
            &solution.class,
        )
    }

    pub fn compose(
        &self,
        branch: u64,
        mixture_entropy: f64,
        amplitude: f64,
        residual_norm: f64,
        status: &ConvergenceClass,
    ) -> BranchEntropy {
        let toxic = matches!(status, ConvergenceClass::Paradox)
            || residual_norm > self.config.toxic_residual;
        let dead = amplitude < self.config.amplitude_floor || toxic;
        let score = mixture_entropy
            + self.config.residual_entropy_weight * (1.0 + residual_norm).ln()
            + if toxic { 10.0 } else { 0.0 }
            + if dead { 5.0 } else { 0.0 };

        BranchEntropy {
            branch,
            mixture_entropy,
            amplitude,
            residual_norm,
            score,
            toxic,
            dead,
        }
    }

    /// Order branches highest-entropy-first for collection.
    pub fn rank(&self, mut scores: Vec<BranchEntropy>) -> Vec<BranchEntropy> {
        scores.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scores
    }
}

fn shannon(weights: &[f64]) -> f64 {
    let sum: f64 = weights.iter().sum();
    if sum <= 0.0 {
        return 0.0;
    }
    weights
        .iter()
        .filter(|w| **w > 0.0)
        .map(|w| {
            let p = *w / sum;
            -p * p.ln()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paradox_scores_as_toxic_and_dead() {
        let bal = EntropyBalancer::new(GcConfig::default());
        let e = bal.compose(1, 0.0, 0.0, 100.0, &ConvergenceClass::Paradox);
        assert!(e.toxic && e.dead);
        assert!(e.score > 10.0);
    }
}
