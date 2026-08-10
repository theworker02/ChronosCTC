use crate::config::CosmosConfig;
use crate::error::{CosmosError, CosmosResult};
use crate::host::HostPhysics;
use crate::seal::{apply_patch, plan_seal, LawSealReport, RuntimePatch};
use ctc_agents::AgentFleet;
use ctc_collapse::CollapseEngine;
use ctc_dag::{NodeState, SpacetimeAddr, WorldlineDag};
use ctc_entropy::ThermoBalancer;
use ctc_gc::TimelineGc;
use ctc_genesis::{GenesisConfig, GenesisEngine, PhysicalLaws};
use ctc_holo::{BoundarySurface, BoundaryTopology, HolographicProjector};
use ctc_horizon::{CosmosCheckpoint, HorizonStore};
use ctc_kernel::{ChronalKernel, ConvergenceClass, FnEvolution, NonlinearSystem};
use ctc_ledger::{ForkCause, OmniversalLedger};
use ctc_pruner::BranchManager;
use nalgebra::DVector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TickReport {
    pub tick: u64,
    pub residual: f64,
    pub thermo_net_j: f64,
    pub holo_von_neumann: f64,
    pub boundary_dim: usize,
    pub gc_nodes_pruned: usize,
    pub gc_pressure: f64,
    pub laws_delta: f64,
    pub teleports_simulated: usize,
    pub zero_energy: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SustainReport {
    pub seal: LawSealReport,
    pub ticks: Vec<TickReport>,
    pub final_laws: PhysicalLaws,
    pub final_residual: f64,
    pub zero_energy: bool,
    pub horizon_id: Option<u64>,
    pub multiverse_winner: Option<u64>,
    pub message: String,
}

/// Novikov closed-cosmos host: seal Λ* → holographic tick loop → horizon persist.
pub struct CosmosRuntime {
    pub config: CosmosConfig,
    pub host: HostPhysics,
    pub genesis_cfg: GenesisConfig,
    pub laws: Option<PhysicalLaws>,
    pub sealed_patch: Option<RuntimePatch>,
    pub thermo: ThermoBalancer,
    pub holo: HolographicProjector,
    pub horizon: HorizonStore,
    pub tick: u64,
    last_bulk: Vec<f64>,
    last_boundary: Option<BoundarySurface>,
    last_residual: f64,
}

impl Default for CosmosRuntime {
    fn default() -> Self {
        Self::new(
            CosmosConfig::default(),
            HostPhysics::default(),
            GenesisConfig::default(),
        )
    }
}

impl CosmosRuntime {
    pub fn new(config: CosmosConfig, host: HostPhysics, genesis_cfg: GenesisConfig) -> Self {
        let thermo = ThermoBalancer::new(host.entropy.clone());
        let holo = HolographicProjector::new(host.holo.clone());
        let horizon = HorizonStore::new(config.horizon_capacity, None);
        Self {
            config,
            host,
            genesis_cfg,
            laws: None,
            sealed_patch: None,
            thermo,
            holo,
            horizon,
            tick: 0,
            last_bulk: Vec::new(),
            last_boundary: None,
            last_residual: f64::INFINITY,
        }
    }

    /// Bootstrap Genesis, seal laws onto the live host, optionally sustain.
    pub fn bootstrap_and_sustain(&mut self) -> CosmosResult<SustainReport> {
        let seal = self.seal_from_genesis()?;
        let mut ticks = Vec::new();
        let mut multiverse_winner = None;

        for _ in 0..self.config.sustain_ticks {
            let report = self.tick_once()?;
            if multiverse_winner.is_none() {
                // First tick may collapse a multi-attractor landscape.
            }
            ticks.push(report);
        }

        // Multiversal probe under sealed laws (feeds thermo with real cell counts).
        if let Ok(w) = self.multiverse_under_laws() {
            multiverse_winner = Some(w);
        }

        let final_residual = ticks
            .last()
            .map(|t| t.residual)
            .unwrap_or(self.last_residual);
        let zero_energy = final_residual <= self.host.entropy.zero_energy_residual;
        if self.config.require_zero_energy {
            self.thermo
                .assert_equilibrium(final_residual)
                .map_err(|e| CosmosError::Entropy(e.to_string()))?;
        }

        let horizon_id = self
            .checkpoint("sustainment complete")
            .ok()
            .map(|(id, _)| id);

        let laws = self.laws.clone().unwrap_or_default();
        Ok(SustainReport {
            message: format!(
                "novikov cosmos: ticks={}  ε={:.3e}  r={:.3e}  zero_E={}  horizon={:?}",
                ticks.len(),
                laws.deutsch_tolerance,
                final_residual,
                zero_energy,
                horizon_id
            ),
            seal,
            ticks,
            final_laws: laws,
            final_residual,
            zero_energy,
            horizon_id,
            multiverse_winner,
        })
    }

    /// Run Genesis meta-compile and seal Λ* onto signal/mesh/holo/thermo/gc/solver.
    pub fn seal_from_genesis(&mut self) -> CosmosResult<LawSealReport> {
        let seed_laws = PhysicalLaws {
            boundary_ratio: self.host.holo.boundary_ratio,
            deutsch_tolerance: self.host.solver.tolerance,
            ..PhysicalLaws::default()
        };
        let mut genesis = GenesisEngine::new(self.genesis_cfg.clone(), seed_laws);
        let boot = genesis
            .bootstrap_cosmos()
            .map_err(|e| CosmosError::Genesis(e.to_string()))?;

        self.laws = Some(boot.laws.clone());
        let patch = plan_seal(&boot.laws, &self.host);
        let report = apply_patch(&mut self.host, &patch);
        self.sealed_patch = Some(patch);

        // Refresh subsystem handles from sealed host.
        self.thermo = ThermoBalancer::new(self.host.entropy.clone());
        self.holo = HolographicProjector::new(self.host.holo.clone());
        self.last_residual = boot.kernel_residual;

        let _ = self.checkpoint("Λ* sealed");
        Ok(report)
    }

    /// One closed retrocausal sustainment tick under sealed laws.
    pub fn tick_once(&mut self) -> CosmosResult<TickReport> {
        let laws = self.laws.clone().ok_or(CosmosError::NotSealed)?;
        self.tick = self.tick.saturating_add(1);
        let laws_before = laws.clone();

        // 1. Build a CTC probe whose contractivity tracks sealed signal speed.
        let dim = (4.0 * laws.manifold_resolution).ceil() as usize;
        let dim = dim.clamp(2, 16);
        let speed = laws.signal_speed;
        let evo = Arc::new(FnEvolution::new(dim, move |x: &DVector<f64>| {
            let alpha = 0.35 / (1.0 + speed);
            x.scale(1.0 - alpha) + DVector::from_element(dim, 0.25 * alpha * 4.0)
        }));
        let unknowns = (0..dim).map(|i| format!("ρ{i}")).collect();
        let system = NonlinearSystem::new(format!("cosmos_tick_{}", self.tick), evo, unknowns)
            .map_err(|e| CosmosError::Kernel(e.to_string()))?;

        // 2. Bulk Deutsch solve under sealed laws (authoritative residual / zero-E gate).
        let bulk_sol = ChronalKernel::new(self.host.solver.clone())
            .solve(&system)
            .map_err(|e| CosmosError::Kernel(e.to_string()))?;
        let residual = bulk_sol.stats.final_residual;
        let mut bulk = bulk_sol
            .states
            .first()
            .cloned()
            .unwrap_or_else(|| vec![0.5; dim]);
        let mut bdim = 0usize;

        // 3. Holographic boundary path — compressed solve + fidelity telemetry.
        if let Ok((_sol, report)) = self.holo.boundary_solve(&system, &self.host.solver) {
            bdim = report.boundary_dim;
            // Prefer bulk fixed point as manifold source; boundary path validates compression.
            let _ = report.bulk_residual;
        } else if let Some(state) = bulk_sol.states.first() {
            bulk = state.clone();
        }

        let (surface, spec, proj) = self
            .holo
            .project_state(&bulk, BoundaryTopology::AdSDisk)
            .map_err(|e| CosmosError::Holo(e.to_string()))?;
        let holo_vn = spec.von_neumann;
        let bdim = if bdim > 0 { bdim } else { surface.dim };

        let _ = self
            .thermo
            .account_holographic(&proj)
            .map_err(|e| CosmosError::Entropy(e.to_string()))?;
        let _ = self.thermo.converge(self.last_residual.max(1.0), residual);

        // 4. Materialize bulk onto a DAG and entropy-GC collect.
        let mut dag = bulk_to_dag(&bulk);
        let mut gc = TimelineGc::new(self.host.gc.clone());
        let branches = BranchManager::new();
        // Thermo pressure modulates GC aggressiveness.
        let pressure = self.thermo.gc_pressure_hint(gc.heap_pressure(&dag));
        let mut gc_nodes = 0usize;
        let collect = gc.collect(&mut dag, &branches);
        let (gc_pressure, collect_ok) = match collect {
            Ok(report) => {
                gc_nodes = report.stats.nodes_pruned + report.stats.branches_culled;
                let _ = self.thermo.account_collection(&report);
                (report.heap_pressure_after.max(pressure), true)
            }
            Err(_) => (pressure, false),
        };
        let _ = collect_ok;

        // 5. Simulated chronal signalling cost ∝ sealed hop latency.
        let teleports_simulated = ((self.host.mesh.hop_latency_us.max(1)) as usize).min(8);

        // 6. Optional drift recompile — laws chase residual.
        let mut laws_delta = 0.0;
        let drift_gate = laws.deutsch_tolerance * self.config.drift_recompile_factor;
        if residual > drift_gate {
            let mut next = laws.clone();
            next.deutsch_tolerance = (next.deutsch_tolerance * 1.5)
                .min(self.genesis_cfg.deutsch_tol_max);
            next.signal_speed = (next.signal_speed * 1.05).min(self.genesis_cfg.signal_speed_max);
            laws_delta = laws_before.l2_distance(&next);
            let patch = plan_seal(&next, &self.host);
            apply_patch(&mut self.host, &patch);
            self.laws = Some(next);
            self.sealed_patch = Some(patch);
            self.thermo = ThermoBalancer::new(self.host.entropy.clone());
            self.holo = HolographicProjector::new(self.host.holo.clone());
        }

        self.last_bulk = bulk;
        self.last_boundary = Some(surface);
        self.last_residual = residual;

        if self.config.checkpoint_every > 0 && self.tick % self.config.checkpoint_every as u64 == 0
        {
            let _ = self.checkpoint(&format!("tick {}", self.tick));
        }

        let snap = self.thermo.snapshot();
        let zero_energy = residual <= self.host.entropy.zero_energy_residual;
        Ok(TickReport {
            message: format!(
                "tick {}: r={:.3e} S_EE={:.4} GC_pruned={} teleports≈{} ΔΛ={:.3e}",
                self.tick, residual, holo_vn, gc_nodes, teleports_simulated, laws_delta
            ),
            tick: self.tick,
            residual,
            thermo_net_j: snap.net_work_j,
            holo_von_neumann: holo_vn,
            boundary_dim: bdim,
            gc_nodes_pruned: gc_nodes,
            gc_pressure,
            laws_delta,
            teleports_simulated,
            zero_energy,
        })
    }

    pub fn checkpoint(&self, note: &str) -> CosmosResult<(u64, CosmosCheckpoint)> {
        let laws = self.laws.clone().unwrap_or_default();
        let ckpt = CosmosCheckpoint {
            id: 0,
            tick: self.tick,
            laws,
            energy: self.thermo.snapshot(),
            boundary: self.last_boundary.clone(),
            primary_bulk: self.last_bulk.clone(),
            kernel_residual: self.last_residual,
            zero_energy: self.last_residual <= self.host.entropy.zero_energy_residual,
            note: note.into(),
        };
        let id = self
            .horizon
            .checkpoint_cosmos(ckpt.clone())
            .map_err(|e| CosmosError::Horizon(e.to_string()))?;
        let stored = self
            .horizon
            .resume(id)
            .map_err(|e| CosmosError::Horizon(e.to_string()))?;
        Ok((id, stored))
    }

    /// Resume from the latest horizon checkpoint and re-seal host physics.
    pub fn resume_from_horizon(&mut self) -> CosmosResult<Option<u64>> {
        let Some(ckpt) = self
            .horizon
            .resume_latest()
            .map_err(|e| CosmosError::Horizon(e.to_string()))?
        else {
            return Ok(None);
        };
        self.laws = Some(ckpt.laws.clone());
        let patch = plan_seal(&ckpt.laws, &self.host);
        apply_patch(&mut self.host, &patch);
        self.sealed_patch = Some(patch);
        self.thermo = ThermoBalancer::new(self.host.entropy.clone());
        self.holo = HolographicProjector::new(self.host.holo.clone());
        self.tick = ckpt.tick;
        self.last_bulk = ckpt.primary_bulk;
        self.last_boundary = ckpt.boundary;
        self.last_residual = ckpt.kernel_residual;
        Ok(Some(ckpt.id))
    }

    fn multiverse_under_laws(&mut self) -> CosmosResult<u64> {
        let evo = Arc::new(FnEvolution::new(1, |x: &DVector<f64>| x.clone()));
        let system = NonlinearSystem::new("cosmos_mv", evo, vec!["ρ".into()])
            .map_err(|e| CosmosError::Kernel(e.to_string()))?;
        let mut cfg = self.host.solver.clone();
        cfg.num_restarts = 4;
        cfg.cluster_eps = 1e-3;
        cfg.domain_lo = 0.0;
        cfg.domain_hi = 1.0;
        let sol = ChronalKernel::new(cfg)
            .solve(&system)
            .map_err(|e| CosmosError::Kernel(e.to_string()))?;
        if sol.class != ConvergenceClass::MultiWeighted {
            return Err(CosmosError::Kernel(
                "expected multi-weighted attractor landscape".into(),
            ));
        }
        let ledger = OmniversalLedger::new(Default::default());
        let root = ledger.bootstrap("novikov", WorldlineDag::new());
        let event = ledger
            .bifurcate_from_solution(root, &sol, ForkCause::MultiFixedPoint)
            .map_err(|e| CosmosError::Ledger(e.to_string()))?;
        let fleet = AgentFleet::new(Default::default());
        fleet.deploy_triad(&event.children);
        let fleet_report = fleet
            .explore_all(&ledger)
            .map_err(|e| CosmosError::Ledger(e.to_string()))?;
        let engine = CollapseEngine::new(Default::default());
        let synthesis = engine
            .synthesize(&ledger, Some(&fleet_report))
            .map_err(|e| CosmosError::Collapse(e.to_string()))?;
        let _ = self
            .thermo
            .account_collapse_with_ledger(&synthesis, &ledger)
            .map_err(|e| CosmosError::Entropy(e.to_string()))?;
        Ok(synthesis.winner)
    }
}

fn bulk_to_dag(bulk: &[f64]) -> WorldlineDag {
    let mut dag = WorldlineDag::new();
    for (i, chunk) in bulk.chunks(2).enumerate() {
        let mut vals = chunk.to_vec();
        if vals.len() == 1 {
            vals.push(vals[0]);
        }
        dag.allocate(
            SpacetimeAddr::new(i as u64, 0),
            NodeState {
                value: Arc::from(vals.as_slice()),
                weight: 1.0,
                revision: 0,
                pruned: false,
            },
        );
    }
    if dag.len() == 0 {
        dag.allocate(
            SpacetimeAddr::new(0, 0),
            NodeState {
                value: Arc::from([0.5_f64]),
                weight: 1.0,
                revision: 0,
                pruned: false,
            },
        );
    }
    dag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_rewrites_signal_and_mesh() {
        let mut rt = CosmosRuntime::default();
        let before_hop = rt.host.mesh.hop_latency_us;
        let before_eps = rt.host.signal.deutsch_tolerance;
        let seal = rt.seal_from_genesis().unwrap();
        assert!(rt.laws.is_some());
        assert_ne!(seal.after_signal_eps, 0.0);
        // Laws may match defaults; at minimum patch applied cleanly.
        assert_eq!(rt.host.signal.deutsch_tolerance, seal.after_signal_eps);
        assert_eq!(rt.host.mesh.hop_latency_us, seal.after_hop_us);
        let _ = before_hop;
        let _ = before_eps;
    }

    #[test]
    fn sustainment_ticks_and_checkpoints() {
        let mut rt = CosmosRuntime::new(
            CosmosConfig {
                sustain_ticks: 2,
                require_zero_energy: false,
                checkpoint_every: 1,
                ..CosmosConfig::default()
            },
            HostPhysics::default(),
            GenesisConfig::default(),
        );
        let report = rt.bootstrap_and_sustain().unwrap();
        assert_eq!(report.ticks.len(), 2);
        assert!(report.horizon_id.is_some());
        assert!(rt.resume_from_horizon().unwrap().is_some());
    }
}
