use crate::error::{HoloError, HoloResult};
use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntanglementSpectrum {
    pub eigenvalues: Vec<f64>,
    pub von_neumann: f64,
}

/// Boundary entanglement kernel \(K_{ij} = \langle \phi_i | \phi_j \rangle\).
#[derive(Clone, Debug)]
pub struct EntanglementMatrix {
    pub matrix: DMatrix<f64>,
}

impl EntanglementMatrix {
    /// Build a Gram-like entanglement matrix from boundary amplitudes.
    pub fn from_amplitudes(amps: &[f64]) -> Self {
        let n = amps.len();
        let mut m = DMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                let dist = (i as isize - j as isize).unsigned_abs() as f64;
                let kernel = (-dist / (n as f64).max(1.0)).exp();
                m[(i, j)] = amps[i] * amps[j] * kernel;
            }
        }
        for i in 0..n {
            m[(i, i)] += 1e-9;
        }
        Self { matrix: m }
    }

    pub fn spectrum(&self) -> HoloResult<EntanglementSpectrum> {
        let sym = (&self.matrix + self.matrix.transpose()).scale(0.5);
        let decomp = sym.symmetric_eigen();
        let mut eigenvalues: Vec<f64> = decomp.eigenvalues.iter().copied().collect();
        eigenvalues.sort_by(|a, b| {
            b.partial_cmp(a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let sum: f64 = eigenvalues.iter().map(|e| e.max(0.0)).sum();
        if sum <= 0.0 {
            return Err(HoloError::SingularEntanglement);
        }
        let von_neumann = eigenvalues
            .iter()
            .map(|e| {
                let p = e.max(0.0) / sum;
                if p > 0.0 {
                    -p * p.ln()
                } else {
                    0.0
                }
            })
            .sum();
        Ok(EntanglementSpectrum {
            eigenvalues,
            von_neumann,
        })
    }

    /// Tikhonov-regularized contraction: \(x = (K^{\mathsf T}K + \lambda I)^{-1} K^{\mathsf T} b\).
    pub fn contract(&self, boundary: &DVector<f64>) -> HoloResult<DVector<f64>> {
        if boundary.len() != self.matrix.nrows() {
            return Err(HoloError::DimensionCollapse {
                bulk: boundary.len(),
                boundary: self.matrix.nrows(),
            });
        }
        let kt = self.matrix.transpose();
        let gram = &kt * &self.matrix;
        let n = gram.nrows();
        let system = gram + DMatrix::identity(n, n).scale(1e-8);
        let rhs = &kt * boundary;
        system
            .lu()
            .solve(&rhs)
            .ok_or(HoloError::SingularEntanglement)
    }
}
