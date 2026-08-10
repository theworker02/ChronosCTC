use crate::config::SignalConfig;
use crate::error::{SignalError, SignalResult};
use crate::packet::{ExpectedFootprint, TemporalPacket};
use ctc_kernel::NonlinearSystem;
use nalgebra::DVector;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsistencyReport {
    pub footprint_ok: bool,
    pub binding_ok: bool,
    pub deutsch_ok: bool,
    pub residual: Option<f64>,
    pub message: String,
}

/// Enforces Deutsch-consistency on retrocausal teleportation packets.
pub struct DeutschGate {
    pub config: SignalConfig,
}

impl DeutschGate {
    pub fn new(config: SignalConfig) -> Self {
        Self { config }
    }

    /// Validate packet against an expected past footprint.
    ///
    /// Binding is checked against the footprint registered at \(\tau_{\mathrm{past}}\)
    /// (topology epoch), not the post-injection fabric hash.
    pub fn validate(
        &self,
        packet: &TemporalPacket,
        expected: &ExpectedFootprint,
        system: Option<&NonlinearSystem>,
        injected_state: Option<&[f64]>,
    ) -> SignalResult<ConsistencyReport> {
        if packet.target_tau != expected.target_tau {
            return Err(SignalError::NonRetrocausal {
                from_tau: packet.source_tau.0,
                to_tau: packet.target_tau.0,
            });
        }

        if packet.cells.len() != expected.slots.len() {
            return Err(SignalError::FootprintMismatch {
                expected: expected.slots.len(),
                got: packet.cells.len(),
            });
        }

        for (i, (cell, (addr, dim))) in packet.cells.iter().zip(expected.slots.iter()).enumerate()
        {
            if cell.addr.address != *addr || cell.addr.tau != expected.target_tau {
                return Err(SignalError::AddressMismatch {
                    slot: i,
                    expected: format!("({}, {})", addr, expected.target_tau),
                    got: format!("{}", cell.addr),
                });
            }
            if !cell.values.is_empty() && cell.values.len() != *dim {
                return Err(SignalError::FootprintMismatch {
                    expected: *dim,
                    got: cell.values.len(),
                });
            }
        }

        if packet.footprint_hash != expected.hash {
            return Err(SignalError::FootprintMismatch {
                expected: expected.slots.len(),
                got: packet.cells.len(),
            });
        }

        let binding_ok = !self.config.require_binding
            || packet.binding.fingerprint == expected.binding.fingerprint;

        if self.config.require_binding && !binding_ok {
            return Err(SignalError::BindingMismatch {
                packet: packet.binding.fingerprint,
                live: expected.binding.fingerprint,
            });
        }

        // Strict residual check only when both system and candidate state are provided
        // (post-injection). Pre-flight calls pass None to validate footprint/binding alone.
        let mut residual = None;
        let mut deutsch_ok = true;
        if self.config.strict_deutsch {
            if let (Some(sys), Some(state)) = (system, injected_state) {
                if state.len() != sys.dimension() {
                    return Err(SignalError::Kernel(format!(
                        "state dim {} != system dim {}",
                        state.len(),
                        sys.dimension()
                    )));
                }
                let x = DVector::from_vec(state.to_vec());
                let r = sys
                    .residual_norm(&x)
                    .map_err(|e| SignalError::Kernel(e.to_string()))?;
                residual = Some(r);
                deutsch_ok = r <= self.config.deutsch_tolerance;
                if !deutsch_ok {
                    return Err(SignalError::DeutschViolation {
                        residual: r,
                        tolerance: self.config.deutsch_tolerance,
                    });
                }
            }
        }

        Ok(ConsistencyReport {
            footprint_ok: true,
            binding_ok,
            deutsch_ok,
            residual,
            message: "Deutsch gate passed — teleportation authorized".into(),
        })
    }
}
