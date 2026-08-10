//! # Chronal Teleportation Protocol (`ctc-signal`)
//!
//! Low-level binary transport that serializes state differentials from a solved
//! future worldline node and injects them into the memory-mapped register space
//! of a past epoch — *before* that past epoch completes its initial execution.
//!
//! ## Deutsch gate
//!
//! Every packet is cryptographically bound to a worldline fingerprint \(H(W)\).
//! Injection at \(\tau_{\mathrm{past}}\) is accepted iff:
//!
//! 1. The packet's target footprint matches the registered past variable layout
//! 2. The worldline binding hash verifies against the live DAG
//! 3. Post-injection residual \(\|F(x)-x\|_2 \le \varepsilon\) (optional strict mode)
//!
//! Violations are rejected; the Paradox Pruner is never bypassed.

mod bind;
mod config;
mod consistency;
mod error;
mod inject;
mod packet;
mod transport;

pub use bind::{worldline_fingerprint, WorldlineBinding};
pub use config::SignalConfig;
pub use consistency::{ConsistencyReport, DeutschGate};
pub use error::{SignalError, SignalResult};
pub use inject::{InjectionReceipt, MemoryInjector};
pub use packet::{
    cell_from_arc, ExpectedFootprint, PacketKind, PayloadCell, TemporalPacket,
    TemporalPacketBuilder,
};
pub use transport::{SharedSignalDaemon, SignalDaemon, TransmitReport};
