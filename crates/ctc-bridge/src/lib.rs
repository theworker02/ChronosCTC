//! # Quantum-Classical Fallback Layer (`ctc-bridge`)
//!
//! Dynamic offloading fabric that routes deterministic classical logic to
//! CPU/GPU pipelines while delegating cyclic, retrocausal blocks to specialized
//! fixpoint accelerators (FPGA co-processors, quantum annealing simulators,
//! or high-fidelity CPU emulators).
//!
//! ## Dispatch invariant
//!
//! Let \(G\) be the temporal dependency graph. A subgraph \(S \subseteq G\) is
//! classified **Retrocausal** iff it contains at least one retrocausal edge or
//! participates in a CTC region. Otherwise it is **Classical**.
//!
//! \[
//! \mathrm{target}(S)
//! =
//! \arg\min_{d \in \mathcal{D}(S)}
//! \;
//! C(d, \dim S)
//! \]
//!
//! where \(\mathcal{D}(S)\) is the admissible device set for the class of \(S\)
//! and \(C\) is the latency cost model from runtime config.

mod classify;
mod config;
mod device;
mod error;
mod router;

pub use classify::{BlockClass, WorkBlock, WorkClassifier};
pub use config::BridgeConfig;
pub use device::{
    default_device_pool, CpuBackend, DeviceBackend, DeviceCapabilities, DeviceId, DeviceKind,
    EmulatorBackend, FpgaBackend, GpuBackend, QuantumAnnealerBackend, SimulatedDevice,
};
pub use error::{BridgeError, BridgeResult};
pub use router::{DispatchPlan, DispatchReceipt, OffloadRouter, RoutedSolution};
