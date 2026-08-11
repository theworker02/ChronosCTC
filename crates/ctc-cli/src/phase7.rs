//! Phase 7 Chronal Continuum federation demo.
//!
//! ```text
//! admit regions ──► wormhole link ──► federation ticks ──► ContinuumReport
//! ```

use ctc_continuum::{ContinuumConfig, ContinuumRuntime};
use crate::config::RuntimeConfig;

/// Run a minimal Phase-7 continuum federation lifecycle.
pub fn run_continuum_federation(_runtime: &RuntimeConfig) -> Result<ctc_continuum::ContinuumReport, String> {
    let mut rt = ContinuumRuntime::new(ContinuumConfig {
        max_regions: 4,
        portal_capacity: 8,
        federation_ticks: 3,
    });
    rt.admit("sol").map_err(|e| e.to_string())?;
    rt.admit("alpha-centauri").map_err(|e| e.to_string())?;
    rt.admit("proxima").map_err(|e| e.to_string())?;
    rt.link("sol", "alpha-centauri").map_err(|e| e.to_string())?;
    rt.link("alpha-centauri", "proxima").map_err(|e| e.to_string())?;
    rt.tick_federation().map_err(|e| e.to_string())
}
