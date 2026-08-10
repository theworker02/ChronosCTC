use ctc_entropy::EntropyConfig;
use ctc_gc::GcConfig;
use ctc_holo::HoloConfig;
use ctc_kernel::SolverConfig;
use ctc_mesh::MeshConfig;
use ctc_signal::SignalConfig;
use serde::{Deserialize, Serialize};

/// Patchable live-stack physics — the substrate Genesis rewrites.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostPhysics {
    pub solver: SolverConfig,
    pub signal: SignalConfig,
    pub mesh: MeshConfig,
    pub holo: HoloConfig,
    pub entropy: EntropyConfig,
    pub gc: GcConfig,
}

impl Default for HostPhysics {
    fn default() -> Self {
        Self {
            solver: SolverConfig::default(),
            signal: SignalConfig::default(),
            mesh: MeshConfig::default(),
            holo: HoloConfig::default(),
            entropy: EntropyConfig::default(),
            gc: GcConfig::default(),
        }
    }
}

impl HostPhysics {
    pub fn from_parts(
        solver: SolverConfig,
        signal: SignalConfig,
        mesh: MeshConfig,
        holo: HoloConfig,
        entropy: EntropyConfig,
        gc: GcConfig,
    ) -> Self {
        Self {
            solver,
            signal,
            mesh,
            holo,
            entropy,
            gc,
        }
    }
}
