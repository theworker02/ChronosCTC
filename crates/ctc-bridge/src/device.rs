use crate::classify::WorkBlock;
use crate::error::{BridgeError, BridgeResult};
use ctc_kernel::{ChronalKernel, FixedPointSolution, IterationObserver, SolverConfig};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Cpu,
    Gpu,
    Fpga,
    QuantumAnnealer,
    CpuEmulator,
}

impl DeviceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DeviceKind::Cpu => "cpu",
            DeviceKind::Gpu => "gpu",
            DeviceKind::Fpga => "fpga",
            DeviceKind::QuantumAnnealer => "quantum_annealer",
            DeviceKind::CpuEmulator => "cpu_emulator",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub kind: DeviceKind,
    pub max_dimension: usize,
    pub supports_retrocausal: bool,
    pub supports_classical: bool,
    pub parallel_slots: usize,
}

/// Backend contract for a physical or simulated compute unit.
pub trait DeviceBackend: Send + Sync {
    fn id(&self) -> &DeviceId;
    fn capabilities(&self) -> &DeviceCapabilities;
    fn is_online(&self) -> bool;
    fn inflight(&self) -> u64;

    /// Execute a chronal work block, optionally streaming iteration telemetry.
    fn execute(
        &self,
        block: &WorkBlock,
        obs: &mut dyn IterationObserver,
    ) -> BridgeResult<FixedPointSolution>;
}

/// Shared simulated device substrate — all Phase-2 backends are software
/// emulations that model distinct latency / affinity profiles while sharing
/// the same Anderson kernel underneath.
pub struct SimulatedDevice {
    id: DeviceId,
    caps: DeviceCapabilities,
    online: AtomicBool,
    inflight: AtomicU64,
    /// Artificial micro-spin count per unknown to approximate device cost.
    spin_per_unknown: u64,
}

impl SimulatedDevice {
    pub fn new(kind: DeviceKind, spin_per_unknown: u64) -> Self {
        let (supports_retrocausal, supports_classical, max_dimension, parallel_slots) = match kind {
            DeviceKind::Cpu => (true, true, 4096, 4),
            DeviceKind::Gpu => (true, true, 65536, 16),
            DeviceKind::Fpga => (true, false, 8192, 8),
            DeviceKind::QuantumAnnealer => (true, false, 2048, 2),
            DeviceKind::CpuEmulator => (true, true, 1024, 1),
        };
        Self {
            id: DeviceId(format!("{}-sim-0", kind.as_str())),
            caps: DeviceCapabilities {
                kind,
                max_dimension,
                supports_retrocausal,
                supports_classical,
                parallel_slots,
            },
            online: AtomicBool::new(true),
            inflight: AtomicU64::new(0),
            spin_per_unknown,
        }
    }

    pub fn set_online(&self, online: bool) {
        self.online.store(online, Ordering::SeqCst);
    }
}

impl DeviceBackend for SimulatedDevice {
    fn id(&self) -> &DeviceId {
        &self.id
    }

    fn capabilities(&self) -> &DeviceCapabilities {
        &self.caps
    }

    fn is_online(&self) -> bool {
        self.online.load(Ordering::SeqCst)
    }

    fn inflight(&self) -> u64 {
        self.inflight.load(Ordering::SeqCst)
    }

    fn execute(
        &self,
        block: &WorkBlock,
        obs: &mut dyn IterationObserver,
    ) -> BridgeResult<FixedPointSolution> {
        if !self.is_online() {
            return Err(BridgeError::DeviceUnavailable(self.id.0.clone()));
        }
        if block.dimension > self.caps.max_dimension {
            return Err(BridgeError::DimensionMismatch {
                expected: self.caps.max_dimension,
                got: block.dimension,
            });
        }

        self.inflight.fetch_add(1, Ordering::SeqCst);
        // Cost-model spin: approximates device latency without wall-clock sleeps.
        let spins = self.spin_per_unknown.saturating_mul(block.dimension as u64);
        let mut acc = 0u64;
        for i in 0..spins {
            acc = acc.wrapping_add(i.wrapping_mul(0x9E37));
        }
        std::hint::black_box(acc);

        let kernel = ChronalKernel::new(block.solver_config.clone());
        let result = kernel
            .solve_observed(block.system.as_ref(), &mut ObserverShim(obs))
            .map_err(|source| BridgeError::DeviceSolve {
                device: self.id.0.clone(),
                source,
            });

        self.inflight.fetch_sub(1, Ordering::SeqCst);
        result
    }
}

/// Trait-object shim: DeviceBackend takes `&mut dyn IterationObserver` while
/// ChronalKernel generics want a concrete `IterationObserver` implementor.
struct ObserverShim<'a>(&'a mut dyn IterationObserver);

impl IterationObserver for ObserverShim<'_> {
    fn on_iteration(&mut self, telem: ctc_kernel::IterationTelemetry) {
        self.0.on_iteration(telem);
    }
    fn on_restart(&mut self, restart: usize) {
        self.0.on_restart(restart);
    }
}

pub type CpuBackend = SimulatedDevice;
pub type GpuBackend = SimulatedDevice;
pub type FpgaBackend = SimulatedDevice;
pub type QuantumAnnealerBackend = SimulatedDevice;
pub type EmulatorBackend = SimulatedDevice;

/// Construct the default Phase-2 simulated device pool.
pub fn default_device_pool(cfg: &crate::config::BridgeConfig) -> Vec<Arc<dyn DeviceBackend>> {
    vec![
        Arc::new(SimulatedDevice::new(DeviceKind::Cpu, cfg.cpu_ns_per_unknown)),
        Arc::new(SimulatedDevice::new(DeviceKind::Gpu, cfg.gpu_ns_per_unknown)),
        Arc::new(SimulatedDevice::new(DeviceKind::Fpga, cfg.fpga_ns_per_unknown)),
        Arc::new(SimulatedDevice::new(
            DeviceKind::QuantumAnnealer,
            cfg.annealer_ns_per_unknown,
        )),
        Arc::new(SimulatedDevice::new(
            DeviceKind::CpuEmulator,
            cfg.emulator_ns_per_unknown,
        )),
    ]
}

/// Helper to build a solver config override for device-local tuning.
pub fn device_tuned_config(base: &SolverConfig, kind: DeviceKind) -> SolverConfig {
    let mut cfg = base.clone();
    match kind {
        DeviceKind::Fpga | DeviceKind::QuantumAnnealer => {
            cfg.anderson_m = cfg.anderson_m.max(8);
            cfg.num_restarts = cfg.num_restarts.max(16);
        }
        DeviceKind::Gpu => {
            cfg.num_restarts = cfg.num_restarts.max(12);
        }
        DeviceKind::CpuEmulator => {
            cfg.max_iterations = cfg.max_iterations.min(128);
        }
        DeviceKind::Cpu => {}
    }
    cfg
}
