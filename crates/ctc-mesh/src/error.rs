use thiserror::Error;

pub type MeshResult<T> = Result<T, MeshError>;

#[derive(Debug, Error)]
pub enum MeshError {
    #[error("mesh node `{0}` is not registered in the cluster")]
    UnknownNode(String),

    #[error("entanglement channel `{0}` does not exist")]
    UnknownChannel(u64),

    #[error("node `{0}` is offline")]
    NodeOffline(String),

    #[error("no route from `{from}` to `{to}` for entanglement {entanglement}")]
    NoRoute {
        from: String,
        to: String,
        entanglement: u64,
    },

    #[error("signal error on mesh transport: {0}")]
    Signal(String),

    #[error("oracle error on mesh delivery: {0}")]
    Oracle(String),

    #[error("duplicate node id `{0}`")]
    DuplicateNode(String),
}
