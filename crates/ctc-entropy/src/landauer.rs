use crate::config::EntropyConfig;
use crate::error::{EntropyError, EntropyResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LandauerOp {
    /// Irreversible bit erasure (costs energy).
    EraseBits,
    /// Paradox / branch prune (dissipates structured information).
    PruneBranch,
    /// Fixed-point residual reduction (can harvest free energy).
    ConvergeResidual,
    /// Multiverse collapse merge.
    CollapseMerge,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThermoEvent {
    pub op: LandauerOp,
    pub bits: f64,
    pub energy_j: f64,
    /// Negative ⇒ harvested; positive ⇒ dissipated cost.
    pub signed_work_j: f64,
    pub note: String,
}

/// Landauer minimum energy for erasing \(N\) bits at temperature \(T\):
/// \(E = k_B T \ln 2 \cdot N\).
pub fn landauer_energy_joules(cfg: &EntropyConfig, bits: f64) -> EntropyResult<f64> {
    if cfg.temperature_k <= 0.0 {
        return Err(EntropyError::InvalidTemperature(cfg.temperature_k));
    }
    Ok(cfg.boltzmann_j_per_k * cfg.temperature_k * std::f64::consts::LN_2 * bits.max(0.0))
}

pub fn event_for_erase(cfg: &EntropyConfig, bits: f64, note: impl Into<String>) -> EntropyResult<ThermoEvent> {
    let e = landauer_energy_joules(cfg, bits)?;
    Ok(ThermoEvent {
        op: LandauerOp::EraseBits,
        bits,
        energy_j: e,
        signed_work_j: e, // cost
        note: note.into(),
    })
}

pub fn event_for_prune(cfg: &EntropyConfig, branch_cells: usize) -> EntropyResult<ThermoEvent> {
    // Each pruned cell ≈ 64 bits of structured information erased.
    let bits = branch_cells as f64 * 64.0;
    let e = landauer_energy_joules(cfg, bits)?;
    Ok(ThermoEvent {
        op: LandauerOp::PruneBranch,
        bits,
        energy_j: e,
        signed_work_j: e,
        note: format!("pruned {branch_cells} manifold cells"),
    })
}

pub fn event_for_convergence(cfg: &EntropyConfig, residual_before: f64, residual_after: f64) -> ThermoEvent {
    let delta = (residual_before - residual_after).max(0.0);
    let harvested = delta * cfg.harvest_per_residual;
    ThermoEvent {
        op: LandauerOp::ConvergeResidual,
        bits: delta, // informational proxy
        energy_j: harvested,
        signed_work_j: -harvested, // harvest
        note: format!("residual {residual_before:.3e}→{residual_after:.3e}"),
    }
}
