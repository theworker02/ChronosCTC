//! # Chronal Compiler (`ctc-compiler`)
//!
//! Parses cyclic / retrocausal source constructs and lowers them into
//! [`ctc_kernel::NonlinearSystem`] instances consumable by the chronal kernel.
//!
//! ## Source model
//!
//! A minimal chronal DSL:
//!
//! ```text
//! chronal reg x @ τ0;
//! chronal reg y @ τ1;
//! retrocausal { y -> x };          // edge τ1 → τ0
//! evolve x = 0.5 * x + 0.25 * y;
//! evolve y = 0.5 * y + 0.25 * x + 0.5;
//! ```
//!
//! The middle-end builds the nonlinear map \(F\) over the ordered unknown
//! vector and emits epoch-crossing dependency metadata for `ctc-dag`.

mod ast;
mod error;
mod lower;
mod parse;
mod temporal;

pub use ast::{ChronalModule, EvolveEq, Expr, RegisterDecl, Stmt};
pub use error::{CompileError, CompileResult};
pub use lower::{lower, CompiledProgram, LoweringOutput};
pub use parse::parse_module;
pub use temporal::{EpochCrossing, TemporalDependencyGraph};
