//! Phase 6 Novikov closed-cosmos lifecycle.
//!
//! ```text
//! Genesis Λ* ──► seal host physics ──► holographic ticks
//!                      │                      │
//!                      ▼                      ▼
//!              signal/mesh/ε rewrite    thermo ↔ GC coupling
//!                                             │
//!                                             ▼
//!                                    ctc-horizon checkpoint
//! ```

use crate::config::RuntimeConfig;
use ctc_cosmos::{CosmosConfig, CosmosRuntime, HostPhysics, SustainReport};

/// Map CLI runtime config into the cosmos host physics bundle + cosmos knobs.
pub fn host_from_runtime(runtime: &RuntimeConfig) -> (HostPhysics, CosmosConfig) {
    let host = HostPhysics::from_parts(
        runtime.solver.to_kernel_config(),
        runtime.signal.clone(),
        runtime.mesh.clone(),
        runtime.holo.clone(),
        runtime.entropy.clone(),
        runtime.gc.clone(),
    );
    (host, runtime.cosmos.clone())
}

/// Full Phase-6 lifecycle: bootstrap → seal → sustain → horizon.
pub fn run_novikov_cosmos(runtime: &RuntimeConfig) -> Result<SustainReport, String> {
    let (host, cosmos_cfg) = host_from_runtime(runtime);
    let mut rt = CosmosRuntime::new(cosmos_cfg, host, runtime.genesis.clone());
    rt.bootstrap_and_sustain().map_err(|e| e.to_string())
}
