//! # Event Horizon Persistence (`ctc-horizon`)
//!
//! Sealed cosmos state — locked physical laws \(\Lambda^\star\), holographic
//! boundary surfaces, and the Landauer energy ledger — persists across process
//! epochs. Crossing the horizon is a checkpoint; resuming reconstitutes the
//! Novikov closed-cosmos loop without re-deriving Genesis from scratch.

mod checkpoint;
mod error;
mod store;

pub use checkpoint::CosmosCheckpoint;
pub use error::{HorizonError, HorizonResult};
pub use store::HorizonStore;
