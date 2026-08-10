use crate::error::{CollapseError, CollapseResult};
use ctc_ledger::{OmniversalLedger, UniverseId, UniverseStatus};
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Cryptographic attestation that a universe branch is Deutsch-consistent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsistencyProof {
    pub universe: u64,
    pub fingerprint: u64,
    pub residual: f64,
    pub weight: f64,
    pub valid: bool,
}

/// Proof-of-Consistency verifier.
pub struct ProofOfConsistency {
    pub residual_ceiling: f64,
}

impl Default for ProofOfConsistency {
    fn default() -> Self {
        Self {
            residual_ceiling: 1.0,
        }
    }
}

impl ProofOfConsistency {
    pub fn prove(
        &self,
        ledger: &OmniversalLedger,
        id: UniverseId,
    ) -> CollapseResult<ConsistencyProof> {
        let status = ledger
            .status(id)
            .ok_or(CollapseError::Ledger(format!("unknown {}", id.0)))?;
        if !matches!(status, UniverseStatus::Active) {
            return Err(CollapseError::ProofFailed(
                id.0,
                format!("status {:?}", status),
            ));
        }
        let residual = ledger.residual(id).unwrap_or(f64::INFINITY);
        let weight = ledger.weight(id).unwrap_or(0.0);
        let fp = ledger.fixed_point(id).unwrap_or_default();

        let mut hasher = FxHasher::default();
        id.0.hash(&mut hasher);
        residual.to_bits().hash(&mut hasher);
        weight.to_bits().hash(&mut hasher);
        for v in &fp {
            v.to_bits().hash(&mut hasher);
        }
        let fingerprint = hasher.finish();
        let valid = residual <= self.residual_ceiling && weight > 0.0;

        let proof = ConsistencyProof {
            universe: id.0,
            fingerprint,
            residual,
            weight,
            valid,
        };
        if !valid {
            return Err(CollapseError::ProofFailed(
                id.0,
                format!("residual={residual:.3e} weight={weight:.4}"),
            ));
        }
        Ok(proof)
    }
}
