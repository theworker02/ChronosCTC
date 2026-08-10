use ctc_ledger::UniverseId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeKind {
    ParadoxScan,
    DeadlockScan,
    ConvergenceAudit,
    FutureFailureProbe,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbeFinding {
    pub kind: ProbeKind,
    pub universe: u64,
    pub severity: f64,
    pub message: String,
    pub needs_correction: bool,
}

/// Correction to inject into a past epoch of a universe manifold.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorrectionVector {
    pub universe: UniverseId,
    pub target_tau: i64,
    pub address: u64,
    pub delta: f64,
    pub reason: String,
}
