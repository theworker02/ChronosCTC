//! # Paradox Pruner & Branch Manager (`ctc-pruner`)
//!
//! Background safety daemon that monitors the residual error vector of the
//! chronal fixed-point solver. When a computation path diverges or enters an
//! inconsistent state space, the pruner:
//!
//! 1. Invalidates the execution branch
//! 2. Rolls back affected worldline nodes
//! 3. Collapses the timeline into the nearest stable alternative branch
//!
//! ## Residual gate
//!
//! Let \(r_k = F(x_k) - x_k\). The pruner triggers when
//!
//! \[
//! \|r_k\|_2 > R_{\max}
//! \quad\text{or}\quad
//! \frac{\|r_{k}\|_2}{\|r_{k-w}\|_2} \ge \sigma
//! \quad (k \gg w,\ \|r_k\|_2 \not\approx 0)
//! \]
//!
//! or when the kernel reports [`ctc_kernel::ConvergenceClass::Paradox`].

mod branch;
mod error;
mod pruner;

pub use branch::{BranchId, BranchManager, TimelineBranch};
pub use error::{PrunerError, PrunerResult};
pub use pruner::{PruneAction, PruneReport, ParadoxPruner, PrunerConfig};
