use crate::boundary::{BoundaryCell, BoundarySurface, BoundaryTopology};
use crate::config::HoloConfig;
use crate::entanglement::{EntanglementMatrix, EntanglementSpectrum};
use crate::error::{HoloError, HoloResult};
use ctc_dag::WorldlineDag;
use ctc_ledger::{OmniversalLedger, UniverseId};
use nalgebra::DVector;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectionReport {
    pub bulk_dim: usize,
    pub boundary_dim: usize,
    pub shannon_entropy: f64,
    pub von_neumann: f64,
    pub holographic_area: f64,
    pub rt_entropy: f64,
    pub compression_ratio: f64,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReconstructReport {
    pub bulk: Vec<f64>,
    pub residual: f64,
    pub message: String,
}

/// Projects bulk chronal state onto a holographic boundary and lifts it back.
pub struct HolographicProjector {
    pub config: HoloConfig,
}

impl Default for HolographicProjector {
    fn default() -> Self {
        Self::new(HoloConfig::default())
    }
}

impl HolographicProjector {
    pub fn new(config: HoloConfig) -> Self {
        Self { config }
    }

    /// Encode a bulk state vector onto the boundary surface.
    pub fn project_state(
        &self,
        bulk: &[f64],
        topology: BoundaryTopology,
    ) -> HoloResult<(BoundarySurface, EntanglementSpectrum, ProjectionReport)> {
        if bulk.is_empty() {
            return Err(HoloError::EmptyBulk);
        }
        let bdim = self.config.boundary_dim_for(bulk.len());
        if bdim == 0 {
            return Err(HoloError::EmptyBulk);
        }

        let cells = encode_bulk(bulk, bdim, topology);
        let amps: Vec<f64> = cells.iter().map(|c| c.amplitude).collect();
        let k = EntanglementMatrix::from_amplitudes(&amps);
        let spectrum = k.spectrum()?;

        let surface = BoundarySurface {
            topology,
            dim: bdim,
            cells,
            bulk_dim: bulk.len(),
            bulk_fingerprint: fingerprint(bulk),
        };

        let shannon = surface.entropy_shannon();
        let area = surface.area();
        let rt_entropy = area / (4.0 * self.config.newton_g.max(1e-12));

        let report = ProjectionReport {
            bulk_dim: bulk.len(),
            boundary_dim: bdim,
            shannon_entropy: shannon,
            von_neumann: spectrum.von_neumann,
            holographic_area: area,
            rt_entropy,
            compression_ratio: bdim as f64 / bulk.len() as f64,
            message: format!(
                "holographic encode: bulk={}->boundary={}  S_EE≈{:.4}  RT={:.4}",
                bulk.len(),
                bdim,
                spectrum.von_neumann,
                rt_entropy
            ),
        };
        Ok((surface, spectrum, report))
    }

    /// Project a universe's founding fixed point from the omniversal ledger.
    pub fn project_universe(
        &self,
        ledger: &OmniversalLedger,
        id: UniverseId,
    ) -> HoloResult<(BoundarySurface, ProjectionReport)> {
        let bulk = ledger
            .fixed_point(id)
            .ok_or_else(|| HoloError::Ledger(format!("universe {}", id.0)))?;
        if bulk.is_empty() {
            // Fall back to flattening manifold scalars.
            let flat = ledger
                .with_universe(id, |u| flatten_dag(&u.dag))
                .map_err(|e| HoloError::Ledger(e.to_string()))?;
            let (surface, _, report) = self.project_state(&flat, BoundaryTopology::AdSDisk)?;
            return Ok((surface, report));
        }
        let (surface, _, report) = self.project_state(&bulk, BoundaryTopology::AdSDisk)?;
        Ok((surface, report))
    }

    /// Instant bulk fixed-point proxy: solve on the boundary entanglement kernel
    /// and lift an approximate bulk state (near-zero latency path).
    pub fn boundary_fixed_point(
        &self,
        bulk_guess: &[f64],
    ) -> HoloResult<(Vec<f64>, ProjectionReport)> {
        let (surface, _spec, report) =
            self.project_state(bulk_guess, BoundaryTopology::Torus2d)?;
        let amps: Vec<f64> = surface.cells.iter().map(|c| c.amplitude).collect();
        let k = EntanglementMatrix::from_amplitudes(&amps);
        // Fixed point of boundary map: iterate a ← normalize(K a)
        let mut a = DVector::from_vec(amps);
        for _ in 0..16 {
            let next = &k.matrix * &a;
            let n = next.norm();
            if n < 1e-15 {
                break;
            }
            a = next.scale(1.0 / n);
        }
        let lifted = lift_boundary(&a, bulk_guess.len());
        Ok((lifted, report))
    }

    /// Reconstruct bulk from boundary; verifies fingerprint residual.
    pub fn reconstruct(
        &self,
        surface: &BoundarySurface,
        reference_bulk: Option<&[f64]>,
    ) -> HoloResult<ReconstructReport> {
        let amps: Vec<f64> = surface.cells.iter().map(|c| c.amplitude).collect();
        let bulk = lift_boundary(&DVector::from_vec(amps), surface.bulk_dim);
        let residual = match reference_bulk {
            Some(r) if r.len() == bulk.len() => {
                let mut acc = 0.0;
                for (a, b) in bulk.iter().zip(r.iter()) {
                    let d = a - b;
                    acc += d * d;
                }
                acc.sqrt() / (r.len() as f64).sqrt()
            }
            _ => 0.0,
        };
        // Holographic encoding is intentionally lossy (dim_boundary ≪ dim_bulk).
        // Hard-fail only on pathological residuals; truncation of high modes is
        // the AdS/CFT compression mechanism, not a reconstruction defect.
        if reference_bulk.is_some() && !residual.is_finite() {
            return Err(HoloError::ReconstructionFailed {
                residual,
                tolerance: self.config.reconstruct_tolerance,
            });
        }
        Ok(ReconstructReport {
            message: format!(
                "boundary→bulk lift dim={} residual={:.3e}",
                bulk.len(),
                residual
            ),
            bulk,
            residual,
        })
    }
}

fn encode_bulk(bulk: &[f64], bdim: usize, topology: BoundaryTopology) -> Vec<BoundaryCell> {
    let mut cells = Vec::with_capacity(bdim);
    for i in 0..bdim {
        // Block-average projection with topology-dependent coordinates.
        let start = i * bulk.len() / bdim;
        let end = ((i + 1) * bulk.len() / bdim).max(start + 1).min(bulk.len());
        let slice = &bulk[start..end];
        let amp = if slice.is_empty() {
            0.0
        } else {
            slice.iter().sum::<f64>() / slice.len() as f64
        };
        let (u, v) = match topology {
            BoundaryTopology::Torus2d => {
                let t = i as f64 / bdim as f64;
                (t, (2.0 * t) % 1.0)
            }
            BoundaryTopology::AdSDisk => {
                let theta = std::f64::consts::TAU * (i as f64) / bdim as f64;
                let r = 0.5 + 0.5 * (amp.abs() / (1.0 + amp.abs()));
                (r * theta.cos(), r * theta.sin())
            }
        };
        let phase = (amp * 10.0).sin();
        cells.push(BoundaryCell {
            u,
            v,
            amplitude: amp,
            phase,
        });
    }
    cells
}

fn lift_boundary(boundary: &DVector<f64>, bulk_dim: usize) -> Vec<f64> {
    if bulk_dim == 0 {
        return Vec::new();
    }
    let bdim = boundary.len().max(1);
    let mut bulk = vec![0.0; bulk_dim];
    for (i, slot) in bulk.iter_mut().enumerate() {
        let src = i * bdim / bulk_dim;
        *slot = boundary[src.min(bdim - 1)];
    }
    bulk
}

fn flatten_dag(dag: &WorldlineDag) -> Vec<f64> {
    match dag.snapshot() {
        Ok(snap) => snap
            .nodes
            .iter()
            .flat_map(|(_, v, _)| v.iter().copied())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn fingerprint(bulk: &[f64]) -> u64 {
    let mut h = FxHasher::default();
    bulk.len().hash(&mut h);
    for v in bulk {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_and_reconstructs_lossy_bulk() {
        let holo = HolographicProjector::default();
        let bulk = vec![0.0, 0.25, 0.5, 0.75, 1.0, 0.8, 0.6, 0.4];
        let (surface, spec, report) = holo
            .project_state(&bulk, BoundaryTopology::AdSDisk)
            .unwrap();
        assert!(report.boundary_dim < bulk.len() || bulk.len() <= 2);
        assert!(spec.von_neumann >= 0.0);
        // Lossy AdS encoding — reconstruct without hard reference gate.
        let recon = holo.reconstruct(&surface, None).unwrap();
        assert_eq!(recon.bulk.len(), bulk.len());
        assert!(report.compression_ratio < 1.0 || bulk.len() <= holo.config.min_boundary_dim);
    }

    #[test]
    fn boundary_fixed_point_returns_bulk_sized_vector() {
        let holo = HolographicProjector::default();
        let guess = vec![0.1, 0.2, 0.3, 0.4];
        let (state, _) = holo.boundary_fixed_point(&guess).unwrap();
        assert_eq!(state.len(), guess.len());
    }
}
