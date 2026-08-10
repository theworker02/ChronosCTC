use crate::device::DeviceKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub ctc_preference: Vec<DeviceKind>,
    pub classical_preference: Vec<DeviceKind>,
    pub cpu_ns_per_unknown: u64,
    pub gpu_ns_per_unknown: u64,
    pub fpga_ns_per_unknown: u64,
    pub annealer_ns_per_unknown: u64,
    pub emulator_ns_per_unknown: u64,
    pub ctc_offload_min_dim: usize,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            ctc_preference: vec![
                DeviceKind::Fpga,
                DeviceKind::QuantumAnnealer,
                DeviceKind::Gpu,
                DeviceKind::CpuEmulator,
            ],
            classical_preference: vec![DeviceKind::Cpu, DeviceKind::Gpu],
            cpu_ns_per_unknown: 50,
            gpu_ns_per_unknown: 8,
            fpga_ns_per_unknown: 3,
            annealer_ns_per_unknown: 1,
            emulator_ns_per_unknown: 120,
            ctc_offload_min_dim: 2,
        }
    }
}

impl BridgeConfig {
    pub fn ns_per_unknown(&self, kind: DeviceKind) -> u64 {
        match kind {
            DeviceKind::Cpu => self.cpu_ns_per_unknown,
            DeviceKind::Gpu => self.gpu_ns_per_unknown,
            DeviceKind::Fpga => self.fpga_ns_per_unknown,
            DeviceKind::QuantumAnnealer => self.annealer_ns_per_unknown,
            DeviceKind::CpuEmulator => self.emulator_ns_per_unknown,
        }
    }

    pub fn estimate_cost_ns(&self, kind: DeviceKind, dim: usize) -> u64 {
        self.ns_per_unknown(kind).saturating_mul(dim as u64)
    }
}
