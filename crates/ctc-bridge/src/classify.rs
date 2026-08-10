use crate::error::{BridgeError, BridgeResult};
use ctc_compiler::CompiledProgram;
use ctc_kernel::{NonlinearSystem, SolverConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Classification of a schedulable work block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockClass {
    /// Acyclic, chronology-respecting logic — CPU/GPU safe.
    Classical,
    /// Contains retrocausal / CTC structure — requires fixpoint accelerator.
    Retrocausal,
}

/// A unit of chronal work ready for device dispatch.
#[derive(Clone)]
pub struct WorkBlock {
    pub name: String,
    pub class: BlockClass,
    pub system: Arc<NonlinearSystem>,
    pub solver_config: SolverConfig,
    pub dimension: usize,
    /// Registers participating in CTC region (empty for classical).
    pub ctc_registers: Vec<String>,
}

/// Partitions compiled programs into classical vs retrocausal work blocks.
pub struct WorkClassifier {
    pub ctc_offload_min_dim: usize,
}

impl WorkClassifier {
    pub fn new(ctc_offload_min_dim: usize) -> Self {
        Self { ctc_offload_min_dim }
    }

    /// Classify a compiled chronal program.
    ///
    /// Phase-2 policy: the entire nonlinear system is one block. If the temporal
    /// analysis reports a nonempty CTC region of sufficient dimension, the block
    /// is Retrocausal; otherwise Classical. (Future: split affine classical
    /// prefixes from CTC cores for finer offload granularity.)
    pub fn classify_program(&self, compiled: &CompiledProgram) -> BridgeResult<WorkBlock> {
        let dim = compiled.system.dimension();
        if dim == 0 {
            return Err(BridgeError::EmptyBlock(compiled.module_name.clone()));
        }
        let ctc_registers: Vec<String> = compiled
            .meta
            .temporal
            .ctc_region
            .iter()
            .cloned()
            .collect();
        let class = if !ctc_registers.is_empty() && dim >= self.ctc_offload_min_dim {
            BlockClass::Retrocausal
        } else if !ctc_registers.is_empty() {
            // Tiny CTC still retrocausal — specialized hardware optional, class stays CTC.
            BlockClass::Retrocausal
        } else {
            BlockClass::Classical
        };

        Ok(WorkBlock {
            name: compiled.module_name.clone(),
            class,
            system: Arc::new(NonlinearSystem {
                name: compiled.system.name.clone(),
                evolution: Arc::clone(&compiled.system.evolution),
                unknowns: compiled.system.unknowns.clone(),
            }),
            // NonlinearSystem doesn't implement Clone easily because of Arc<dyn> -
            // wait, I'm reconstructing. Actually NonlinearSystem fields are public
            // and evolution is Arc - good.
            solver_config: compiled.meta.solver_config.clone(),
            dimension: dim,
            ctc_registers,
        })
    }

    /// Wrap a bare nonlinear system with an explicit class.
    pub fn wrap_system(
        &self,
        name: impl Into<String>,
        system: NonlinearSystem,
        class: BlockClass,
        solver_config: SolverConfig,
    ) -> WorkBlock {
        let dimension = system.dimension();
        WorkBlock {
            name: name.into(),
            class,
            system: Arc::new(system),
            solver_config,
            dimension,
            ctc_registers: Vec::new(),
        }
    }
}
