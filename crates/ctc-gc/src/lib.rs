//! # Timeline Garbage Collector & Entropy Balancer (`ctc-gc`)
//!
//! Entropy-aware collection daemon for the worldline fabric. As the DAG forks,
//! branches, and absorbs retroactive mutations, historical and parallel frames
//! accumulate. The GC:
//!
//! 1. Scores branches by probability amplitude and residual health
//! 2. Prunes toxic / dead-end paradox states
//! 3. Compresses collapsed historical epochs into immutable checkpoints
//! 4. Releases heap without severing active retrocausal IPC edges
//!
//! ## Entropy score
//!
//! For branch \(b\) with mixture weights \(\{w_i\}\) and residual \(r_b\):
//!
//! \[
//! H(b)
//! =
//! -\sum_i w_i \log w_i
//! +
//! \lambda \log(1 + r_b)
//! \]
//!
//! Branches with total amplitude \(\sum w_i < \varepsilon_a\) or residual
//! \(r_b > R_{\mathrm{toxic}}\) are collection candidates.

mod checkpoint;
mod config;
mod entropy;
mod error;
mod collector;

pub use checkpoint::{EpochCheckpoint, CheckpointStore};
pub use collector::{CollectionReport, CollectionStats, TimelineGc};
pub use config::GcConfig;
pub use entropy::{BranchEntropy, EntropyBalancer};
pub use error::{GcError, GcResult};
