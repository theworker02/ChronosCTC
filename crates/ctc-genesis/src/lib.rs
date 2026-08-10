//! # Genesis Meta-Compiler & Reality Bootstrap (`ctc-genesis`)
//!
//! Self-referential compiler that compiles not merely bytecode but the
//! **physical laws and mathematical constants** of the execution environment.
//! Observing workload behaviour across epochs and multiversal forks, Genesis
//! rewrites Deutsch tolerances, chronal signalling speed, and manifold
//! dimensionality to match the software inhabiting the timeline.
//!
//! ## Bootstrap fixed point
//!
//! Let \(\Lambda\) be the law-parameter vector. Genesis seeks
//!
//! \[
//! \Lambda^\star = G\big(W(\Lambda^\star)\big)
//! \]
//!
//! where \(W\) is the workload induced by the current laws and \(G\) is the
//! meta-compilation map — a chronal fixed point at the level of physics itself.

mod bootstrap;
mod config;
mod error;
mod laws;
mod metacompile;

pub use bootstrap::{BootstrapReport, GenesisEngine};
pub use config::GenesisConfig;
pub use error::{GenesisError, GenesisResult};
pub use laws::{PhysicalLaws, LawDelta};
pub use metacompile::{MetaCompileReport, MetaCompiler};
