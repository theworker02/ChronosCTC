//! Phase 2 convergence-observation loop.
//!
//! ```text
//! compiler ──► bridge.dispatch ──► inspector.telemetry
//!                  │                      │
//!                  ▼                      ▼
//!              solution              manifold scrub
//!                  │
//!                  ▼
//!         pruner.observe ──► gc.collect ──► (re-dispatch if paradox)
//! ```

use crate::config::RuntimeConfig;
use ctc_bridge::{BlockClass, DeviceKind, OffloadRouter};
use ctc_compiler::CompiledProgram;
use ctc_gc::TimelineGc;
use ctc_inspector::{DebugSession, DivergenceHotspot};
use ctc_kernel::{ConvergenceClass, FixedPointSolution};
use ctc_pruner::{ParadoxPruner, PruneAction};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct LoopReport {
    pub device_kind: DeviceKind,
    pub device_id: String,
    pub block_class: BlockClass,
    pub solution: FixedPointSolution,
    pub unknown_order: Vec<String>,
    pub manifold_ascii: String,
    pub sparkline: String,
    pub hotspots: Vec<DivergenceHotspot>,
    pub telemetry_samples: usize,
    pub prune_action: PruneAction,
    pub gc_message: Option<String>,
    pub reconverge_attempts: usize,
}

pub struct ConvergenceObservationLoop {
    pub router: OffloadRouter,
    pub inspector: DebugSession,
    pub pruner: ParadoxPruner,
    pub gc: TimelineGc,
    pub max_reconverge: usize,
}

impl ConvergenceObservationLoop {
    pub fn from_runtime(runtime: &RuntimeConfig) -> Self {
        Self {
            router: OffloadRouter::new(runtime.bridge.to_bridge_config()),
            inspector: DebugSession::new(runtime.inspector.clone()),
            pruner: ParadoxPruner::new(runtime.pruner.clone()),
            gc: TimelineGc::new(runtime.gc.clone()),
            max_reconverge: 2,
        }
    }

    /// Execute the full Phase-2 loop on a compiled chronal program.
    pub fn run(&mut self, mut compiled: CompiledProgram) -> Result<LoopReport, String> {
        self.inspector
            .set_labels(compiled.meta.unknown_order.clone());
        self.inspector
            .attach(&compiled.dag)
            .map_err(|e| e.to_string())?;

        let mut reconverge_attempts = 0;

        loop {
            self.inspector.telemetry.clear();

            // 2. Parallel exploration via bridge offload fabric.
            let routed = self
                .router
                .dispatch_program(&compiled, &mut self.inspector)
                .map_err(|e| e.to_string())?;

            let block = self
                .router
                .classifier
                .classify_program(&compiled)
                .map_err(|e| e.to_string())?;

            // Apply primary attractor into the worldline fabric for manifold inspect.
            if let Some((idx, _)) = routed
                .receipt
                .solution
                .weights
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            {
                let state = &routed.receipt.solution.states[idx];
                if let Ok(snap) = compiled.dag.snapshot() {
                    for (i, (addr, _, _)) in snap.nodes.iter().enumerate() {
                        if let Some(v) = state.get(i) {
                            let _ = compiled.dag.retro_write(*addr, Arc::from([*v]));
                        }
                    }
                }
            }
            let dirty = compiled.dag.dirty_nodes();
            compiled.dag.clear_dirty(&dirty);

            self.inspector
                .attach(&compiled.dag)
                .map_err(|e| e.to_string())?;
            // Scrub to the latest epoch for the developer-facing slice.
            if let Some(c) = self.inspector.cursor {
                let _ = self.inspector.seek(c.max);
            }

            let snap = self
                .inspector
                .snapshot(&compiled.dag)
                .map_err(|e| e.to_string())?;

            // Seed / update branch manager.
            let branch = match self.pruner.branches.active() {
                Some(b) => b,
                None => self
                    .pruner
                    .branches
                    .seed_root(&compiled.dag, routed.receipt.solution.stats.final_residual),
            };

            let prune_report = self
                .pruner
                .observe_solution(branch, &routed.receipt.solution, &mut compiled.dag)
                .map_err(|e| e.to_string())?;

            // 4. Automated pruning / GC.
            let gc_report = self
                .gc
                .collect(&mut compiled.dag, &self.pruner.branches)
                .map_err(|e| e.to_string())?;

            let is_paradox = matches!(
                routed.receipt.solution.class,
                ConvergenceClass::Paradox
            ) || matches!(
                prune_report.action,
                PruneAction::HardAbort | PruneAction::Invalidate | PruneAction::Collapse { .. }
            );

            if is_paradox && matches!(routed.receipt.solution.class, ConvergenceClass::Paradox) {
                reconverge_attempts += 1;
                if reconverge_attempts <= self.max_reconverge {
                    let _ = self
                        .gc
                        .force_reconverge_cleanup(&mut compiled.dag, &self.pruner.branches);
                    // Continue loop — re-dispatch on (hopefully) cleaned manifold.
                    // For a pure mathematical paradox the second pass will also fail;
                    // we break after reporting.
                    if reconverge_attempts < self.max_reconverge {
                        continue;
                    }
                }
            }

            return Ok(LoopReport {
                device_kind: routed.receipt.plan.device_kind,
                device_id: routed.receipt.plan.device_id,
                block_class: block.class,
                solution: routed.receipt.solution,
                unknown_order: compiled.meta.unknown_order.clone(),
                manifold_ascii: snap.ascii_manifold,
                sparkline: self.inspector.render_residual_sparkline(48),
                hotspots: snap.hotspots,
                telemetry_samples: self.inspector.telemetry.len(),
                prune_action: prune_report.action,
                gc_message: Some(gc_report.message),
                reconverge_attempts,
            });
        }
    }
}
