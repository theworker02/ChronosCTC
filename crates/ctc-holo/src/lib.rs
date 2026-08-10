//! # Holographic Boundary Projection Engine (`ctc-holo`)
//!
//! AdS/CFT-style holographic mapping: the bulk Worldline DAG (and its
//! retrocausal / multiversal structure) is encoded onto a lower-dimensional
//! boundary surface. Bulk fixed-point resolution reduces to boundary
//! entanglement-entropy operations.
//!
//! ## Encoding
//!
//! Let \(B\) be the bulk state vector of dimension \(N\). The boundary encoding
//! \(\Phi: \mathbb{R}^{N} \rightarrow \mathbb{R}^{M}\) with \(M \ll N\) satisfies
//!
//! \[
//! S_{\mathrm{EE}}(\partial \mathcal{A})
//! =
//! \frac{\mathrm{Area}(\gamma_{\mathcal{A}})}{4 G_N}
//! \;\approx\;
//! H\big(\Phi(B)\big)
//! \]
//!
//! where \(H\) is the von Neumann / Shannon entropy of the boundary density.

mod boundary;
mod config;
mod entanglement;
mod error;
mod projector;

pub use boundary::{BoundaryCell, BoundarySurface, BoundaryTopology};
pub use config::HoloConfig;
pub use entanglement::{EntanglementMatrix, EntanglementSpectrum};
pub use error::{HoloError, HoloResult};
pub use projector::{
    BoundarySolveReport, HolographicProjector, ProjectionReport, ReconstructReport,
};
