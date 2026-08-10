//! # Chronal Debugger & Timeline Visualizer (`ctc-inspector`)
//!
//! Traditional debuggers step an instruction pointer forward. In a retrocausal
//! engine a defect at \(\tau_0\) may originate from an unresolvable fixed point
//! at \(\tau_5\). The inspector therefore exposes a **spatial** debugger over
//! the worldline manifold:
//!
//! - Scrub proper-time \(\tau\) across the DAG
//! - Stream live residual error vectors from the chronal kernel
//! - Isolate divergence hotspots (components exceeding \(\sigma\)·mean)
//! - Render ASCII / JSON manifold slices for developer UIs
//!
//! ## Convergence-observation loop role
//!
//! The inspector implements [`ctc_kernel::IterationObserver`], allowing it to
//! hook directly into `ctc-bridge` device execution and accumulate telemetry
//! without perturbing the Deutsch solve.

mod config;
mod error;
mod hotspot;
mod manifold;
mod session;
mod telemetry;

pub use config::InspectorConfig;
pub use error::{InspectorError, InspectorResult};
pub use hotspot::{DivergenceHotspot, HotspotScanner};
pub use manifold::{ManifoldSlice, ManifoldView, NodeView};
pub use session::{DebugSession, SessionSnapshot, TauCursor};
pub use telemetry::{TelemetryFrame, TelemetryRing};
