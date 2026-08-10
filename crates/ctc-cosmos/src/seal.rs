use crate::host::HostPhysics;
use ctc_genesis::PhysicalLaws;
use serde::{Deserialize, Serialize};

/// Diff produced when Genesis Λ* is sealed onto the live host stack.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimePatch {
    pub laws: PhysicalLaws,
    pub solver_tolerance: f64,
    pub solver_anderson_beta: f64,
    pub signal_deutsch_tolerance: f64,
    pub mesh_hop_latency_us: u64,
    pub holo_boundary_ratio: f64,
    pub entropy_zero_energy_residual: f64,
    pub gc_toxic_residual: f64,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LawSealReport {
    pub patch: RuntimePatch,
    pub before_signal_eps: f64,
    pub after_signal_eps: f64,
    pub before_hop_us: u64,
    pub after_hop_us: u64,
    pub before_boundary: f64,
    pub after_boundary: f64,
}

/// Build a runtime patch from locked physical laws relative to the current host.
pub fn plan_seal(laws: &PhysicalLaws, host: &HostPhysics) -> RuntimePatch {
    let hop = ((host.mesh.hop_latency_us as f64) / laws.signal_speed.max(1e-9))
        .round()
        .clamp(1.0, 10_000.0) as u64;
    // Toxic residual scales with Deutsch ε — tighter laws, earlier paradox cull.
    let toxic = (laws.deutsch_tolerance * 1e12).clamp(0.1, 100.0);

    RuntimePatch {
        message: format!(
            "seal Λ*: ε={:.3e} signal×{:.3} boundary={:.3} hop={}µs",
            laws.deutsch_tolerance, laws.signal_speed, laws.boundary_ratio, hop
        ),
        laws: laws.clone(),
        solver_tolerance: laws.deutsch_tolerance,
        solver_anderson_beta: laws.anderson_beta,
        signal_deutsch_tolerance: laws.deutsch_tolerance,
        mesh_hop_latency_us: hop,
        holo_boundary_ratio: laws.boundary_ratio,
        entropy_zero_energy_residual: laws.deutsch_tolerance,
        gc_toxic_residual: toxic,
    }
}

/// Apply a sealed patch onto the mutable host physics bundle.
pub fn apply_patch(host: &mut HostPhysics, patch: &RuntimePatch) -> LawSealReport {
    let before_signal_eps = host.signal.deutsch_tolerance;
    let before_hop_us = host.mesh.hop_latency_us;
    let before_boundary = host.holo.boundary_ratio;

    host.solver.tolerance = patch.solver_tolerance;
    host.solver.anderson_beta = patch.solver_anderson_beta;
    host.signal.deutsch_tolerance = patch.signal_deutsch_tolerance;
    host.mesh.hop_latency_us = patch.mesh_hop_latency_us;
    host.holo.boundary_ratio = patch.holo_boundary_ratio;
    host.entropy.zero_energy_residual = patch.entropy_zero_energy_residual;
    host.gc.toxic_residual = patch.gc_toxic_residual;

    LawSealReport {
        before_signal_eps,
        after_signal_eps: host.signal.deutsch_tolerance,
        before_hop_us,
        after_hop_us: host.mesh.hop_latency_us,
        before_boundary,
        after_boundary: host.holo.boundary_ratio,
        patch: patch.clone(),
    }
}
