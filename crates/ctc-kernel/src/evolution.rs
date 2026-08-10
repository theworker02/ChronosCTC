use crate::error::{KernelError, KernelResult};
use nalgebra::DVector;
use std::sync::Arc;

/// Evolution operator \(F: \mathbb{R}^n \rightarrow \mathbb{R}^n\) for a CTC block.
///
/// Implementations must be pure with respect to the chronal unknowns: given the
/// same \(x\), \(F(x)\) is deterministic. External (chronology-respecting)
/// inputs are closed over at construction time by the compiler.
pub trait EvolutionMap: Send + Sync {
    fn dimension(&self) -> usize;
    fn apply(&self, x: &DVector<f64>) -> KernelResult<DVector<f64>>;
}

/// Closure-backed evolution map — primary bridge from `ctc-compiler` lowering.
pub struct FnEvolution<F>
where
    F: Fn(&DVector<f64>) -> DVector<f64> + Send + Sync,
{
    dim: usize,
    f: F,
}

impl<F> FnEvolution<F>
where
    F: Fn(&DVector<f64>) -> DVector<f64> + Send + Sync,
{
    pub fn new(dim: usize, f: F) -> Self {
        Self { dim, f }
    }
}

impl<F> EvolutionMap for FnEvolution<F>
where
    F: Fn(&DVector<f64>) -> DVector<f64> + Send + Sync,
{
    fn dimension(&self) -> usize {
        self.dim
    }

    fn apply(&self, x: &DVector<f64>) -> KernelResult<DVector<f64>> {
        if x.len() != self.dim {
            return Err(KernelError::DimensionMismatch {
                expected: self.dim,
                got: x.len(),
            });
        }
        let y = (self.f)(x);
        if y.len() != self.dim {
            return Err(KernelError::DimensionMismatch {
                expected: self.dim,
                got: y.len(),
            });
        }
        Ok(y)
    }
}

/// Compiled nonlinear system: named unknowns + evolution operator.
///
/// The residual is \(r(x) = F(x) - x\). A Deutsch-consistent worldline exists
/// iff \(r(x^\star) = 0\) for some admissible \(x^\star\).
pub struct NonlinearSystem {
    pub name: String,
    pub evolution: Arc<dyn EvolutionMap>,
    /// Human-readable labels for each coordinate (compiler-assigned).
    pub unknowns: Vec<String>,
}

impl NonlinearSystem {
    pub fn new(
        name: impl Into<String>,
        evolution: Arc<dyn EvolutionMap>,
        unknowns: Vec<String>,
    ) -> KernelResult<Self> {
        let dim = evolution.dimension();
        if dim == 0 {
            return Err(KernelError::EmptySystem);
        }
        if unknowns.len() != dim {
            return Err(KernelError::DimensionMismatch {
                expected: dim,
                got: unknowns.len(),
            });
        }
        Ok(Self {
            name: name.into(),
            evolution,
            unknowns,
        })
    }

    pub fn dimension(&self) -> usize {
        self.evolution.dimension()
    }

    pub fn residual(&self, x: &DVector<f64>) -> KernelResult<DVector<f64>> {
        let fx = self.evolution.apply(x)?;
        Ok(fx - x)
    }

    pub fn residual_norm(&self, x: &DVector<f64>) -> KernelResult<f64> {
        Ok(self.residual(x)?.norm())
    }
}

/// Affine CTC map \(F(x) = Ax + b\) — analytically tractable benchmark class.
pub struct AffineEvolution {
    pub a: nalgebra::DMatrix<f64>,
    pub b: DVector<f64>,
}

impl EvolutionMap for AffineEvolution {
    fn dimension(&self) -> usize {
        self.b.len()
    }

    fn apply(&self, x: &DVector<f64>) -> KernelResult<DVector<f64>> {
        if x.len() != self.b.len() {
            return Err(KernelError::DimensionMismatch {
                expected: self.b.len(),
                got: x.len(),
            });
        }
        Ok(&self.a * x + &self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{DMatrix, DVector};

    #[test]
    fn affine_identity_has_all_fixed_points() {
        let evo = AffineEvolution {
            a: DMatrix::identity(2, 2),
            b: DVector::zeros(2),
        };
        let x = DVector::from_vec(vec![0.3, 0.7]);
        let fx = evo.apply(&x).unwrap();
        assert!((fx - x).norm() < 1e-15);
    }
}
