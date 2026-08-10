//! # Pre-Cognitive Branch Interception (`ctc-oracle`)
//!
//! Wraps program execution blocks in a state of **temporal superposition**.
//! When a chronal injection point at \(\tau_0\) is hit, the frame does not
//! brute-force iterate or block on slow I/O — it enters a lightweight wait
//! until a `ctc-signal` packet arrives from \(\tau_{n+1}\), then collapses
//! instantly into the future-resolved state, skipping thousands of cycles.

mod error;
mod hook;
mod superposition;
mod wait;

pub use error::{OracleError, OracleResult};
pub use hook::{CollapseReport, InjectionPoint, OracleEngine, OracleHook};
pub use superposition::{SuperpositionState, TemporalSuperposition};
pub use wait::{WaitHandle, WaitStatus};
