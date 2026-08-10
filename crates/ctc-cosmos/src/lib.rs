//! # Novikov Closed Cosmos (`ctc-cosmos`)
//!
//! Phase 6 orchestration layer: Genesis-locked physical laws \(\Lambda^\star\)
//! are sealed onto the live Deutsch / signal / mesh / holographic / thermo / GC
//! stack, then the universe advances as a self-sustaining retrocausal tick loop.
//!
//! ## Lifecycle
//!
//! 1. **Bootstrap** — `ctc-genesis` meta-compiles \(\Lambda^\star\)
//! 2. **Seal** — patch solver ε, signal Deutsch gate, mesh hop latency, holo ratio
//! 3. **Tick** — boundary Deutsch solve → Landauer account → thermo-modulated GC
//! 4. **Horizon** — persist sealed state via `ctc-horizon` for process resurrection
//!
//! The past creates the future; sealed laws write the substrate the past runs on.

mod config;
mod error;
mod host;
mod runtime;
mod seal;

pub use config::CosmosConfig;
pub use error::{CosmosError, CosmosResult};
pub use host::HostPhysics;
pub use runtime::{CosmosRuntime, SustainReport, TickReport};
pub use seal::{apply_patch, plan_seal, LawSealReport, RuntimePatch};
