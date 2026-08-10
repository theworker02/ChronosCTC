//! # Chronal Fixed-Point Kernel (`ctc-kernel`)
//!
//! Replaces the instruction pointer with a multi-dimensional nonlinear solver.
//!
//! ## Deutsch consistency condition
//!
//! For a CTC subsystem with evolution operator \(U\), the chronal state
//! \(\rho\) must satisfy the fixed-point equation
//!
//! \[
//! U(\rho) = \rho
//! \]
//!
//! In the classical/continuous embedding used by Cronos-CTC, density operators
//! are represented by real state vectors \(x \in \mathbb{R}^n\) and evolution
//! by a nonlinear map \(F: \mathbb{R}^n \rightarrow \mathbb{R}^n\). The kernel
//! solves
//!
//! \[
//! x^\star = F(x^\star)
//! \quad\text{i.e.}\quad
//! r(x) := F(x) - x = 0
//! \]
//!
//! ## Resolution policy
//!
//! | Residual landscape | Action |
//! |---|---|
//! | Unique attractor \(\|r\| < \varepsilon\) | Lock fixed point |
//! | Multiple attractors | Probability-weight by basin measure |
//! | No convergent attractor | Emit [`ConvergenceClass::Paradox`] for the Pruner |

mod anderson;
mod error;
mod evolution;
mod residual;
mod solver;

pub use anderson::AndersonAccelerator;
pub use error::{KernelError, KernelResult};
pub use evolution::{AffineEvolution, EvolutionMap, FnEvolution, NonlinearSystem};
pub use residual::{ResidualMonitor, ResidualSample};
pub use solver::{
    ChronalKernel, ConvergenceClass, FixedPointSolution, IterationObserver, IterationTelemetry,
    SolverConfig, SolverStats,
};
