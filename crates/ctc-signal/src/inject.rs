use crate::config::SignalConfig;
use crate::consistency::DeutschGate;
use crate::error::{SignalError, SignalResult};
use crate::packet::{ExpectedFootprint, TemporalPacket};
use ctc_dag::WorldlineDag;
use ctc_kernel::NonlinearSystem;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InjectionReceipt {
    pub packet_id: u64,
    pub cells_written: usize,
    pub cascade_invalidated: usize,
    pub residual: Option<f64>,
    pub message: String,
}

/// Injects a validated temporal packet into past epoch register space.
pub struct MemoryInjector {
    pub gate: DeutschGate,
}

impl MemoryInjector {
    pub fn new(config: SignalConfig) -> Self {
        Self {
            gate: DeutschGate::new(config),
        }
    }

    /// Validate then retro-write each payload cell into the worldline fabric.
    pub fn inject(
        &self,
        dag: &mut WorldlineDag,
        packet: &TemporalPacket,
        expected: &ExpectedFootprint,
        system: Option<&NonlinearSystem>,
    ) -> SignalResult<InjectionReceipt> {
        // Pre-check without residual (state not yet injected).
        self.gate.validate(packet, expected, None, None)?;

        let mut cascade_invalidated = 0usize;
        let mut cells_written = 0usize;

        for cell in &packet.cells {
            if dag.lookup(cell.addr).is_none() {
                // Allocate missing past cell so injection can proceed.
                dag.allocate(
                    cell.addr,
                    ctc_dag::NodeState::from_slice(&cell.values),
                );
            }
            if let Some(node) = dag.lookup(cell.addr) {
                if node.sealed {
                    return Err(SignalError::SealedTarget(format!("{}", cell.addr)));
                }
            }
            let value: Arc<[f64]> = if cell.values.is_empty() {
                // Hot-patch: encode blob length as a sentinel scalar channel.
                Arc::from([cell.blob.len() as f64])
            } else {
                Arc::from(cell.values.as_slice())
            };
            let report = dag
                .retro_write(cell.addr, value)
                .map_err(|e| SignalError::Dag(e.to_string()))?;
            cascade_invalidated += report.invalidated.len();
            cells_written += 1;
        }

        // Optional strict Deutsch on concatenated state vector.
        let mut residual = None;
        if self.gate.config.strict_deutsch {
            if let Some(sys) = system {
                let state: Vec<f64> = packet.cells.iter().flat_map(|c| c.values.clone()).collect();
                let report = self
                    .gate
                    .validate(packet, expected, Some(sys), Some(&state))?;
                residual = report.residual;
            }
        }

        Ok(InjectionReceipt {
            packet_id: packet.id,
            cells_written,
            cascade_invalidated,
            residual,
            message: format!(
                "injected packet {} → τ{} ({} cells, cascade={})",
                packet.id, packet.target_tau.0, cells_written, cascade_invalidated
            ),
        })
    }
}
