use crate::classify::{BlockClass, WorkBlock, WorkClassifier};
use crate::config::BridgeConfig;
use crate::device::{
    default_device_pool, device_tuned_config, DeviceBackend, DeviceId, DeviceKind,
};
use crate::error::{BridgeError, BridgeResult};
use ctc_compiler::CompiledProgram;
use ctc_kernel::{FixedPointSolution, IterationObserver};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Planned device assignment prior to execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DispatchPlan {
    pub block_name: String,
    pub class: BlockClass,
    pub device_id: String,
    pub device_kind: DeviceKind,
    pub estimated_cost_ns: u64,
    pub dimension: usize,
}

/// Receipt after a dispatched solve completes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DispatchReceipt {
    pub plan: DispatchPlan,
    pub solution: FixedPointSolution,
}

/// Full routed result including optional fallback chain taken.
#[derive(Clone, Debug)]
pub struct RoutedSolution {
    pub receipt: DispatchReceipt,
    pub fallbacks_attempted: Vec<String>,
}

/// Hardware abstraction router — selects and executes on the cheapest admissible device.
pub struct OffloadRouter {
    pub config: BridgeConfig,
    pub classifier: WorkClassifier,
    devices: RwLock<Vec<Arc<dyn DeviceBackend>>>,
}

impl Default for OffloadRouter {
    fn default() -> Self {
        Self::new(BridgeConfig::default())
    }
}

impl OffloadRouter {
    pub fn new(config: BridgeConfig) -> Self {
        let devices = default_device_pool(&config);
        let classifier = WorkClassifier::new(config.ctc_offload_min_dim);
        Self {
            config,
            classifier,
            devices: RwLock::new(devices),
        }
    }

    pub fn with_devices(config: BridgeConfig, devices: Vec<Arc<dyn DeviceBackend>>) -> Self {
        let classifier = WorkClassifier::new(config.ctc_offload_min_dim);
        Self {
            config,
            classifier,
            devices: RwLock::new(devices),
        }
    }

    pub fn list_devices(&self) -> Vec<(DeviceId, DeviceKind, bool)> {
        self.devices
            .read()
            .iter()
            .map(|d| {
                (
                    d.id().clone(),
                    d.capabilities().kind,
                    d.is_online(),
                )
            })
            .collect()
    }

    /// Plan dispatch for a work block without executing.
    pub fn plan(&self, block: &WorkBlock) -> BridgeResult<DispatchPlan> {
        let device = self.select_device(block)?;
        let kind = device.capabilities().kind;
        Ok(DispatchPlan {
            block_name: block.name.clone(),
            class: block.class,
            device_id: device.id().0.clone(),
            device_kind: kind,
            estimated_cost_ns: self.config.estimate_cost_ns(kind, block.dimension),
            dimension: block.dimension,
        })
    }

    /// Classify a compiled program, plan, and execute with observation.
    pub fn dispatch_program(
        &self,
        compiled: &CompiledProgram,
        obs: &mut dyn IterationObserver,
    ) -> BridgeResult<RoutedSolution> {
        let block = self.classifier.classify_program(compiled)?;
        self.dispatch_block(&block, obs)
    }

    /// Execute a work block with automatic fallback along the preference chain.
    pub fn dispatch_block(
        &self,
        block: &WorkBlock,
        obs: &mut dyn IterationObserver,
    ) -> BridgeResult<RoutedSolution> {
        let preference = match block.class {
            BlockClass::Retrocausal => &self.config.ctc_preference,
            BlockClass::Classical => &self.config.classical_preference,
        };

        let mut fallbacks = Vec::new();
        let mut last_err = None;

        for kind in preference {
            let Some(device) = self.find_online(*kind, block) else {
                continue;
            };
            let mut tuned = block.clone_with_config(device_tuned_config(
                &block.solver_config,
                *kind,
            ));
            // Keep identity; only solver config changes.
            let _ = &mut tuned;

            let plan = DispatchPlan {
                block_name: block.name.clone(),
                class: block.class,
                device_id: device.id().0.clone(),
                device_kind: *kind,
                estimated_cost_ns: self.config.estimate_cost_ns(*kind, block.dimension),
                dimension: block.dimension,
            };

            match device.execute(&tuned, obs) {
                Ok(solution) => {
                    return Ok(RoutedSolution {
                        receipt: DispatchReceipt { plan, solution },
                        fallbacks_attempted: fallbacks,
                    });
                }
                Err(e) => {
                    fallbacks.push(format!("{}: {e}", device.id().0));
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or(BridgeError::NoDevice(block.class)))
    }

    fn select_device(&self, block: &WorkBlock) -> BridgeResult<Arc<dyn DeviceBackend>> {
        let preference = match block.class {
            BlockClass::Retrocausal => &self.config.ctc_preference,
            BlockClass::Classical => &self.config.classical_preference,
        };
        for kind in preference {
            if let Some(d) = self.find_online(*kind, block) {
                return Ok(d);
            }
        }
        Err(BridgeError::NoDevice(block.class))
    }

    fn find_online(&self, kind: DeviceKind, block: &WorkBlock) -> Option<Arc<dyn DeviceBackend>> {
        let devices = self.devices.read();
        let mut best: Option<(Arc<dyn DeviceBackend>, u64)> = None;
        for d in devices.iter() {
            if !d.is_online() || d.capabilities().kind != kind {
                continue;
            }
            let caps = d.capabilities();
            let class_ok = match block.class {
                BlockClass::Classical => caps.supports_classical,
                BlockClass::Retrocausal => caps.supports_retrocausal,
            };
            if !class_ok || block.dimension > caps.max_dimension {
                continue;
            }
            // Prefer lower inflight load within the same kind.
            let load = d.inflight();
            let cost = self
                .config
                .estimate_cost_ns(kind, block.dimension)
                .saturating_add(load.saturating_mul(1000));
            match &best {
                None => best = Some((Arc::clone(d), cost)),
                Some((_, c)) if cost < *c => best = Some((Arc::clone(d), cost)),
                _ => {}
            }
        }
        best.map(|(d, _)| d)
    }
}

impl WorkBlock {
    fn clone_with_config(&self, solver_config: ctc_kernel::SolverConfig) -> Self {
        Self {
            name: self.name.clone(),
            class: self.class,
            system: Arc::clone(&self.system),
            solver_config,
            dimension: self.dimension,
            ctc_registers: self.ctc_registers.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_compiler::{lower, parse_module};
    use ctc_kernel::ConvergenceClass;

    #[test]
    fn routes_retrocausal_block_to_fpga_first() {
        let src = r#"
            chronal reg x @ τ0;
            chronal reg y @ τ1;
            retrocausal { y -> x };
            evolve x = 0.5 * x + 0.25 * y;
            evolve y = 0.25 * x + 0.5 * y + 0.5;
        "#;
        let module = parse_module("bridge_ctc", src).unwrap();
        let compiled = lower(&module).unwrap();
        let router = OffloadRouter::default();
        let block = router.classifier.classify_program(&compiled).unwrap();
        assert_eq!(block.class, BlockClass::Retrocausal);
        let plan = router.plan(&block).unwrap();
        assert_eq!(plan.device_kind, DeviceKind::Fpga);

        let routed = router
            .dispatch_block(&block, &mut ())
            .unwrap();
        assert_eq!(routed.receipt.plan.device_kind, DeviceKind::Fpga);
        assert_eq!(routed.receipt.solution.class, ConvergenceClass::Unique);
    }

    #[test]
    fn falls_back_when_preferred_device_offline() {
        let src = r#"
            chronal reg x @ τ0;
            chronal reg y @ τ1;
            retrocausal { y -> x };
            evolve x = 0.5 * x + 0.25 * y;
            evolve y = 0.25 * x + 0.5 * y + 0.5;
        "#;
        let compiled = lower(&parse_module("fb", src).unwrap()).unwrap();
        let cfg = BridgeConfig::default();
        let devices = default_device_pool(&cfg);
        // Take FPGA offline.
        for d in &devices {
            if d.capabilities().kind == DeviceKind::Fpga {
                // SimulatedDevice specific — downcast not available via trait.
                // Rebuild pool with FPGA missing instead.
            }
        }
        let devices: Vec<Arc<dyn DeviceBackend>> = devices
            .into_iter()
            .filter(|d| d.capabilities().kind != DeviceKind::Fpga)
            .collect();
        let router = OffloadRouter::with_devices(cfg, devices);
        let block = router.classifier.classify_program(&compiled).unwrap();
        let plan = router.plan(&block).unwrap();
        assert_eq!(plan.device_kind, DeviceKind::QuantumAnnealer);
    }
}
