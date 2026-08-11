//! # Chronal Continuum Federation (`ctc-continuum`)
//!
//! Phase 7 orchestration: admit named continuum regions, link them with
//! `ctc-wormhole` portals, and advance a synthetic in-memory federation via
//! `ContinuumRuntime::tick_federation`.

mod config;
mod error;
mod runtime;

pub use config::ContinuumConfig;
pub use error::{ContinuumError, ContinuumResult};
pub use runtime::{
    ContinuumReport, ContinuumRuntime, FederationTick, RegionState,
};
