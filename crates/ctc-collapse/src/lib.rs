//! # Reality Collapse & Consensus Engine (`ctc-collapse`)
//!
//! Cryptographic / thermodynamic consensus layer using a **Proof-of-Consistency**
//! model. Periodically evaluates all active multiverse branches, computes the
//! global entropy gradient, selects the highest-utility timeline, merges it
//! into the primary thread, and prunes redundant realities.

mod config;
mod consensus;
mod error;
mod engine;
mod proof;

pub use config::CollapseConfig;
pub use consensus::{BranchScore, ConsensusReport, ObjectiveWeights};
pub use engine::{CollapseEngine, RealitySynthesisReport};
pub use error::{CollapseError, CollapseResult};
pub use proof::{ConsistencyProof, ProofOfConsistency};
