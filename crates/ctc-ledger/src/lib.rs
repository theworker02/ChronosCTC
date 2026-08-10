//! # Omniversal State Ledger (`ctc-ledger`)
//!
//! Append-only distributed ledger tracking divergent reality branches.
//! When a fixed-point solver yields multiple valid roots of \(U(\rho)=\rho\),
//! the ledger records a **Chronal Bifurcation Event** and spawns isolated
//! child worldline branches, each with an independent memory manifold and
//! probability weight.

mod bifurcation;
mod config;
mod error;
mod ledger;
mod universe;

pub use bifurcation::{BifurcationEvent, BifurcationId, ForkCause};
pub use config::LedgerConfig;
pub use error::{LedgerError, LedgerResult};
pub use ledger::{LedgerSnapshot, OmniversalLedger};
pub use universe::{UniverseBranch, UniverseId, UniverseStatus};
