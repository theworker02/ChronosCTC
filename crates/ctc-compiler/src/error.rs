use thiserror::Error;

pub type CompileResult<T> = Result<T, CompileError>;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("undefined chronal register `{0}`")]
    UndefinedRegister(String),

    #[error("duplicate chronal register `{0}`")]
    DuplicateRegister(String),

    #[error("retrocausal edge `{from}` → `{to}` references unknown register")]
    BadRetrocausalEdge { from: String, to: String },

    #[error("register `{0}` lacks an evolve equation — underdetermined CTC block")]
    MissingEvolution(String),

    #[error("lowering failed: {0}")]
    Lowering(String),

    #[error(transparent)]
    Kernel(#[from] ctc_kernel::KernelError),
}
