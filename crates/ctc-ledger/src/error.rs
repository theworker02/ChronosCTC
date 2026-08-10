use thiserror::Error;

pub type LedgerResult<T> = Result<T, LedgerError>;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("universe `{0}` is not recorded in the omniversal ledger")]
    UnknownUniverse(u64),

    #[error("bifurcation `{0}` is not recorded")]
    UnknownBifurcation(u64),

    #[error("cannot fork universe `{0}`: status is not Active")]
    InactiveParent(u64),

    #[error("probability weights must be positive and sum to ~1, got sum={0}")]
    InvalidWeights(f64),

    #[error("ledger is sealed — no further bifurcations permitted")]
    LedgerSealed,

    #[error("universe `{0}` has already been collapsed/pruned")]
    AlreadyTerminal(u64),

    #[error("worldline fabric error: {0}")]
    Dag(String),
}
