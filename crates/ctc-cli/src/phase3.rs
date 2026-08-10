//! Phase 3 inter-epoch data lifecycle.
//!
//! ```text
//! τ_final resolve ──► ctc-signal package ──► mesh publish
//!                                               │
//!                                               ▼
//! τ_0 oracle await ◄── mesh deliver ──► pre-cognitive collapse
//! ```

use crate::config::RuntimeConfig;
use ctc_dag::{Epoch, LogicalAddr, NodeState, SpacetimeAddr, WorldlineDag};
use ctc_mesh::{MeshCluster, MeshNode, MeshRouter, NodeRole};
use ctc_oracle::{InjectionPoint, OracleEngine, SuperpositionState};
use ctc_signal::{
    ExpectedFootprint, PayloadCell, SignalConfig, SignalDaemon, TransmitReport,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TeleportReport {
    pub packet_id: u64,
    pub cycles_saved: u64,
    pub injected_value: f64,
    pub transmit: TransmitReport,
    pub message: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MeshTeleportReport {
    pub entanglement: u64,
    pub from: String,
    pub to: String,
    pub cycles_saved: u64,
    pub injected_value: f64,
    pub message: String,
}

/// Local (single-node) retrocausal teleportation via signal + oracle.
#[allow(dead_code)]
pub fn run_local_teleport(runtime: &RuntimeConfig) -> Result<TeleportReport, String> {
    let signal = Arc::new(SignalDaemon::new(runtime.signal.clone()));
    let mut oracle = OracleEngine::new(Arc::clone(&signal));
    oracle.default_timeout = Some(runtime.oracle.timeout());

    let mut dag = WorldlineDag::new();
    let addr = LogicalAddr(0xA);
    dag.allocate(SpacetimeAddr::new(0xA, 0), NodeState::scalar(0.0));
    dag.allocate(SpacetimeAddr::new(0xA, 4), NodeState::scalar(0.0));

    oracle
        .register(
            &dag,
            InjectionPoint {
                name: "opt_seed".into(),
                epoch: Epoch(0),
                slots: vec![(addr, 1)],
                estimated_cycles: 250_000,
            },
        )
        .map_err(|e| e.to_string())?;

    let future_value = 0.742_187_5;
    let collapse = oracle
        .intercept_and_collapse(&mut dag, "opt_seed", |_| {
            oracle
                .stage_future_scalars("opt_seed", Epoch(4), &[(addr, future_value)])
                .map(|_| ())
        })
        .map_err(|e| e.to_string())?;

    assert_eq!(
        oracle.status("opt_seed"),
        Some(SuperpositionState::Collapsed)
    );
    let injected = dag
        .lookup(SpacetimeAddr::new(0xA, 0))
        .ok_or("missing past cell")?
        .state
        .value[0];

    Ok(TeleportReport {
        packet_id: collapse.packet_id,
        cycles_saved: collapse.cycles_saved,
        injected_value: injected,
        transmit: collapse.transmit,
        message: collapse.message,
    })
}

/// Distributed teleportation: future solver node → past worker node via mesh.
#[allow(dead_code)]
pub fn run_mesh_teleport(runtime: &RuntimeConfig) -> Result<MeshTeleportReport, String> {
    let cluster = Arc::new(MeshCluster::new(runtime.mesh.clone()));
    let past_signal = Arc::new(SignalDaemon::new(runtime.signal.clone()));
    let future_signal = Arc::new(SignalDaemon::new(SignalConfig::default()));

    let past_h = cluster
        .register_node(MeshNode::new(
            "worker-α",
            NodeRole::PastWorker,
            past_signal,
        ))
        .map_err(|e| e.to_string())?;
    let _future_h = cluster
        .register_node(MeshNode::new(
            "solver-Ω",
            NodeRole::FutureSolver,
            future_signal,
        ))
        .map_err(|e| e.to_string())?;
    let eid = cluster
        .entangle("worker-α", "solver-Ω")
        .map_err(|e| e.to_string())?;

    let router = MeshRouter::new(Arc::clone(&cluster));
    let mut past_dag = WorldlineDag::new();
    let addr = LogicalAddr(0xBEEF);
    past_dag.allocate(SpacetimeAddr::new(0xBEEF, 0), NodeState::scalar(0.0));

    {
        let past = past_h.write();
        past.oracle
            .register(
                &past_dag,
                InjectionPoint {
                    name: "ml_convergence".into(),
                    epoch: Epoch(0),
                    slots: vec![(addr, 1)],
                    estimated_cycles: 2_000_000,
                },
            )
            .map_err(|e| e.to_string())?;
        past.oracle
            .enter_await("ml_convergence")
            .map_err(|e| e.to_string())?;
    }

    let converged = 0.918_273_645;
    let packet = {
        let past = past_h.write();
        let id = past
            .oracle
            .stage_future_scalars("ml_convergence", Epoch(12), &[(addr, converged)])
            .map_err(|e| e.to_string())?;
        past.signal.take(id).ok_or("staged packet missing")?
    };

    router
        .publish(eid, "solver-Ω", "worker-α", "ml_convergence", packet)
        .map_err(|e| e.to_string())?;

    let delivery = router
        .deliver(eid, "worker-α", &mut past_dag)
        .map_err(|e| e.to_string())?;

    let injected = past_dag
        .lookup(SpacetimeAddr::new(0xBEEF, 0))
        .ok_or("missing injected cell")?
        .state
        .value[0];

    Ok(MeshTeleportReport {
        entanglement: delivery.entanglement,
        from: delivery.from,
        to: delivery.to,
        cycles_saved: delivery.cycles_saved,
        injected_value: injected,
        message: delivery.message,
    })
}

/// Strict Deutsch-gated teleport of a known fixed-point into the past.
#[allow(dead_code)]
pub fn run_deutsch_gated_teleport(runtime: &RuntimeConfig) -> Result<String, String> {
    use ctc_kernel::{ChronalKernel, FnEvolution, NonlinearSystem, SolverConfig};
    use nalgebra::DVector;

    let mut cfg = runtime.signal.clone();
    cfg.strict_deutsch = true;
    cfg.deutsch_tolerance = 1e-8;

    let signal = SignalDaemon::new(cfg);
    let mut dag = WorldlineDag::new();
    let past = SpacetimeAddr::new(1, 0);
    let future = SpacetimeAddr::new(1, 2);
    dag.allocate(past, NodeState::scalar(0.0));
    dag.allocate(future, NodeState::scalar(0.0));

    // F(x) = 0.5 x + 1  ⇒  x* = 2
    let evo = Arc::new(FnEvolution::new(1, |x| {
        DVector::from_vec(vec![0.5 * x[0] + 1.0])
    }));
    let system =
        NonlinearSystem::new("contract", evo, vec!["x".into()]).map_err(|e| e.to_string())?;
    let sol = ChronalKernel::new(SolverConfig {
        num_restarts: 4,
        ..SolverConfig::default()
    })
    .solve(&system)
    .map_err(|e| e.to_string())?;
    let x_star = sol.states[0][0];

    let binding = signal.bind(&dag);
    let cells = vec![PayloadCell {
        addr: past,
        values: vec![x_star],
        blob: vec![],
    }];
    let fp = ExpectedFootprint::from_cells(Epoch(0), &cells, binding);
    signal.register_footprint(fp);

    let packet = signal
        .package_scalars(Epoch(2), Epoch(0), binding, &[(past, x_star)])
        .map_err(|e| e.to_string())?;
    let report = signal
        .transmit(&mut dag, &packet, Some(&system))
        .map_err(|e| e.to_string())?;

    Ok(format!(
        "Deutsch-gated teleport ok: x*={:.6} residual={:?} cells={}",
        x_star, report.injection.residual, report.injection.cells_written
    ))
}
