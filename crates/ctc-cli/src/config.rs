use ctc_agents::AgentConfig;
use ctc_bridge::{BridgeConfig, DeviceKind};
use ctc_collapse::CollapseConfig;
use ctc_cosmos::CosmosConfig;
use ctc_entropy::EntropyConfig;
use ctc_gc::GcConfig;
use ctc_genesis::GenesisConfig;
use ctc_holo::HoloConfig;
use ctc_inspector::InspectorConfig;
use ctc_kernel::SolverConfig;
use ctc_ledger::LedgerConfig;
use ctc_mesh::MeshConfig;
use ctc_pruner::PrunerConfig;
use ctc_signal::SignalConfig;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)] // full runtime surface retained across phase demos
pub struct RuntimeConfig {
    pub solver: SolverSection,
    pub pruner: PrunerConfig,
    pub bridge: BridgeSection,
    pub gc: GcConfig,
    pub inspector: InspectorConfig,
    #[serde(default)]
    pub signal: SignalConfig,
    #[serde(default)]
    pub oracle: OracleSection,
    #[serde(default)]
    pub mesh: MeshConfig,
    #[serde(default)]
    pub ledger: LedgerConfig,
    #[serde(default)]
    pub agents: AgentConfig,
    #[serde(default)]
    pub collapse: CollapseConfig,
    #[serde(default)]
    pub holo: HoloConfig,
    #[serde(default)]
    pub entropy: EntropyConfig,
    #[serde(default)]
    pub genesis: GenesisConfig,
    #[serde(default)]
    pub cosmos: CosmosConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct OracleSection {
    pub timeout_ms: u64,
}

impl Default for OracleSection {
    fn default() -> Self {
        Self { timeout_ms: 50 }
    }
}

impl OracleSection {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct SolverSection {
    pub max_iterations: usize,
    pub tolerance: f64,
    pub anderson_m: usize,
    pub anderson_beta: f64,
    pub num_restarts: usize,
    pub cluster_eps: f64,
    pub domain_lo: f64,
    pub domain_hi: f64,
}

impl SolverSection {
    pub fn to_kernel_config(&self) -> SolverConfig {
        SolverConfig {
            max_iterations: self.max_iterations,
            tolerance: self.tolerance,
            anderson_m: self.anderson_m,
            anderson_beta: self.anderson_beta,
            num_restarts: self.num_restarts,
            cluster_eps: self.cluster_eps,
            domain_lo: self.domain_lo,
            domain_hi: self.domain_hi,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct BridgeSection {
    pub ctc_preference: Vec<String>,
    pub classical_preference: Vec<String>,
    pub cpu_ns_per_unknown: u64,
    pub gpu_ns_per_unknown: u64,
    pub fpga_ns_per_unknown: u64,
    pub annealer_ns_per_unknown: u64,
    pub emulator_ns_per_unknown: u64,
    pub ctc_offload_min_dim: usize,
}

impl BridgeSection {
    pub fn to_bridge_config(&self) -> BridgeConfig {
        BridgeConfig {
            ctc_preference: self
                .ctc_preference
                .iter()
                .filter_map(|s| parse_kind(s))
                .collect(),
            classical_preference: self
                .classical_preference
                .iter()
                .filter_map(|s| parse_kind(s))
                .collect(),
            cpu_ns_per_unknown: self.cpu_ns_per_unknown,
            gpu_ns_per_unknown: self.gpu_ns_per_unknown,
            fpga_ns_per_unknown: self.fpga_ns_per_unknown,
            annealer_ns_per_unknown: self.annealer_ns_per_unknown,
            emulator_ns_per_unknown: self.emulator_ns_per_unknown,
            ctc_offload_min_dim: self.ctc_offload_min_dim,
        }
    }
}

fn parse_kind(s: &str) -> Option<DeviceKind> {
    match s {
        "cpu" => Some(DeviceKind::Cpu),
        "gpu" => Some(DeviceKind::Gpu),
        "fpga" => Some(DeviceKind::Fpga),
        "quantum_annealer" => Some(DeviceKind::QuantumAnnealer),
        "cpu_emulator" => Some(DeviceKind::CpuEmulator),
        _ => None,
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            solver: SolverSection {
                max_iterations: 256,
                tolerance: 1e-10,
                anderson_m: 6,
                anderson_beta: 1.0,
                num_restarts: 12,
                cluster_eps: 1e-6,
                domain_lo: 0.0,
                domain_hi: 1.0,
            },
            pruner: PrunerConfig::default(),
            bridge: BridgeSection {
                ctc_preference: vec![
                    "fpga".into(),
                    "quantum_annealer".into(),
                    "gpu".into(),
                    "cpu_emulator".into(),
                ],
                classical_preference: vec!["cpu".into(), "gpu".into()],
                cpu_ns_per_unknown: 50,
                gpu_ns_per_unknown: 8,
                fpga_ns_per_unknown: 3,
                annealer_ns_per_unknown: 1,
                emulator_ns_per_unknown: 120,
                ctc_offload_min_dim: 2,
            },
            gc: GcConfig::default(),
            inspector: InspectorConfig::default(),
            signal: SignalConfig::default(),
            oracle: OracleSection::default(),
            mesh: MeshConfig::default(),
            ledger: LedgerConfig::default(),
            agents: AgentConfig::default(),
            collapse: CollapseConfig::default(),
            holo: HoloConfig::default(),
            entropy: EntropyConfig::default(),
            genesis: GenesisConfig::default(),
            cosmos: CosmosConfig::default(),
        }
    }
}

/// Load `configs/runtime.toml`, falling back to defaults if missing/unparseable.
pub fn load_runtime_config() -> RuntimeConfig {
    let candidates = [
        Path::new("configs/runtime.toml"),
        Path::new("/agent/configs/runtime.toml"),
    ];
    for path in candidates {
        if let Ok(raw) = std::fs::read_to_string(path) {
            match toml::from_str::<RuntimeConfig>(&raw) {
                Ok(cfg) => return cfg,
                Err(e) => {
                    eprintln!("warning: failed to parse {}: {e}", path.display());
                }
            }
        }
    }
    eprintln!("warning: using default RuntimeConfig (configs/runtime.toml not found)");
    RuntimeConfig::default()
}
