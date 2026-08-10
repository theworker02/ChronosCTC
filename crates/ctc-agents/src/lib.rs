//! # Autonomous Chronal Agents (`ctc-agents`)
//!
//! Lightweight non-deterministic sub-routines that live natively inside the
//! Worldline DAG. Unlike standard threads, agents traverse temporal epochs
//! \(\tau\) and parallel universe branches, scanning for impending paradoxes,
//! resource deadlocks, or sub-optimal convergence — then proactively inject
//! correction vectors into past execution frames via `ctc-signal`.

mod agent;
mod config;
mod error;
mod fleet;
mod probe;

pub use agent::{AgentId, AgentReport, AgentRole, ChronalAgent};
pub use config::AgentConfig;
pub use error::{AgentError, AgentResult};
pub use fleet::{AgentFleet, FleetReport};
pub use probe::{CorrectionVector, ProbeFinding, ProbeKind};
