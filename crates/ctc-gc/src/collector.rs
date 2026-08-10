use crate::checkpoint::CheckpointStore;
use crate::config::GcConfig;
use crate::entropy::{BranchEntropy, EntropyBalancer};
use crate::error::{GcError, GcResult};
use ctc_dag::{NodeId, SpacetimeAddr, WorldlineDag};
use ctc_pruner::{BranchId, BranchManager};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CollectionStats {
    pub nodes_pruned: usize,
    pub nodes_sealed: usize,
    pub checkpoints_created: usize,
    pub bytes_reclaimed_est: usize,
    pub branches_culled: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectionReport {
    pub stats: CollectionStats,
    pub culled_branches: Vec<BranchEntropy>,
    pub checkpoint_ids: Vec<u64>,
    pub heap_pressure_before: f64,
    pub heap_pressure_after: f64,
    pub message: String,
}

/// Pins protect nodes that participate in active retrocausal IPC channels.
#[derive(Default)]
pub struct RetrocausalPinSet {
    pins: FxHashSet<NodeId>,
}

impl RetrocausalPinSet {
    pub fn pin(&mut self, id: NodeId) {
        self.pins.insert(id);
    }

    pub fn unpin(&mut self, id: NodeId) {
        self.pins.remove(&id);
    }

    pub fn is_pinned(&self, id: NodeId) -> bool {
        self.pins.contains(&id)
    }

    /// Auto-pin every endpoint of a retrocausal edge in the fabric.
    pub fn refresh_from_dag(&mut self, dag: &WorldlineDag) {
        self.pins.clear();
        for (from, to) in dag.retrocausal_edges() {
            self.pins.insert(from);
            self.pins.insert(to);
        }
    }
}

/// Entropy-aware timeline garbage collector.
pub struct TimelineGc {
    pub config: GcConfig,
    pub balancer: EntropyBalancer,
    pub checkpoints: CheckpointStore,
    pub pins: RetrocausalPinSet,
}

impl Default for TimelineGc {
    fn default() -> Self {
        Self::new(GcConfig::default())
    }
}

impl TimelineGc {
    pub fn new(config: GcConfig) -> Self {
        let checkpoints = CheckpointStore::new(config.max_checkpoints);
        let balancer = EntropyBalancer::new(config.clone());
        Self {
            config,
            balancer,
            checkpoints,
            pins: RetrocausalPinSet::default(),
        }
    }

    /// Estimate heap pressure as live_nodes / (live_nodes + compressed_equiv).
    pub fn heap_pressure(&self, dag: &WorldlineDag) -> f64 {
        let live = dag.len().max(1) as f64;
        let cold = (self.checkpoints.total_compressed_bytes() / 32).max(1) as f64;
        live / (live + cold)
    }

    /// Full collection cycle: score → cull toxic branches → compress sealed epochs → prune.
    pub fn collect(
        &mut self,
        dag: &mut WorldlineDag,
        branches: &BranchManager,
    ) -> GcResult<CollectionReport> {
        self.pins.refresh_from_dag(dag);
        let pressure_before = self.heap_pressure(dag);
        let mut stats = CollectionStats::default();
        let mut culled = Vec::new();
        let mut checkpoint_ids = Vec::new();

        // 1. Score all known stable/unstable branches from the manager.
        let mut scores = Vec::new();
        for id in branches.list_stable() {
            if let Some(b) = branches.get(id) {
                scores.push(self.balancer.score_branch(&b));
            }
        }
        // Also consider the active branch if it's unstable (paradox pending).
        if let Some(active) = branches.active() {
            if let Some(b) = branches.get(active) {
                let e = self.balancer.score_branch(&b);
                if !scores.iter().any(|s| s.branch == e.branch) {
                    scores.push(e);
                }
            }
        }

        let ranked = self.balancer.rank(scores);
        let aggressive = pressure_before >= self.config.heap_pressure_trigger
            || ranked.len() > self.config.max_live_branches;

        // 2. Cull toxic / dead branches (never the sole active stable root without alternative).
        for ent in &ranked {
            if !(ent.toxic || ent.dead || aggressive) {
                continue;
            }
            let bid = BranchId(ent.branch);
            if branches.active() == Some(bid) && branches.list_stable().len() <= 1 && !ent.toxic {
                // Protect last stable active branch unless toxic.
                continue;
            }
            if ent.toxic || ent.dead {
                let _ = branches.invalidate(bid);
                culled.push(ent.clone());
                stats.branches_culled += 1;
            }
        }

        // 3. Compress sealed historical epochs beyond the seal horizon.
        let max_tau = max_epoch(dag);
        let horizon = max_tau - self.config.seal_horizon;
        let compressable = collect_compressable(dag, horizon, &self.pins);
        if !compressable.is_empty() {
            let mut cells = Vec::new();
            let mut to_prune = Vec::new();
            for nid in &compressable {
                if let Some(node) = dag.node(*nid) {
                    if self.pins.is_pinned(*nid) {
                        return Err(GcError::PinnedNode(nid.0));
                    }
                    cells.push((node.addr, Arc::clone(&node.state.value)));
                    to_prune.push(*nid);
                }
            }
            if !cells.is_empty() {
                let bytes_raw: usize = cells.iter().map(|(_, v)| v.len() * 8).sum();
                let cp = self.checkpoints.compress(cells)?;
                checkpoint_ids.push(cp.id);
                stats.checkpoints_created += 1;
                stats.bytes_reclaimed_est += bytes_raw.saturating_sub(cp.bytes_compressed);

                // Seal then prune from hot fabric — IPC pins already excluded.
                for nid in &to_prune {
                    if let Some(node) = dag.node(*nid) {
                        let addr = node.addr;
                        let _ = dag.seal(addr);
                        stats.nodes_sealed += 1;
                    }
                }
                dag.prune_nodes(&to_prune);
                stats.nodes_pruned += to_prune.len();
            }
        }

        // 4. Prune already-flagged pruned/dirty paradox debris.
        let debris: Vec<NodeId> = debris_nodes(dag);
        let debris_unpinned: Vec<NodeId> = debris
            .into_iter()
            .filter(|n| !self.pins.is_pinned(*n))
            .collect();
        if !debris_unpinned.is_empty() {
            stats.nodes_pruned += debris_unpinned.len();
            dag.prune_nodes(&debris_unpinned);
        }

        let pressure_after = self.heap_pressure(dag);
        let message = format!(
            "GC complete: pruned={} checkpoints={} culled_branches={} pressure {:.3}->{:.3}",
            stats.nodes_pruned,
            stats.checkpoints_created,
            stats.branches_culled,
            pressure_before,
            pressure_after
        );
        Ok(CollectionReport {
            stats,
            culled_branches: culled,
            checkpoint_ids,
            heap_pressure_before: pressure_before,
            heap_pressure_after: pressure_after,
            message,
        })
    }

    /// Force-converge helper used by the Phase-2 loop after paradox: GC then
    /// signal caller to re-dispatch on the nearest valid manifold.
    pub fn force_reconverge_cleanup(
        &mut self,
        dag: &mut WorldlineDag,
        branches: &BranchManager,
    ) -> GcResult<CollectionReport> {
        let mut report = self.collect(dag, branches)?;
        report.message = format!("reconverge-cleanup: {}", report.message);
        Ok(report)
    }
}

fn max_epoch(dag: &WorldlineDag) -> i64 {
    dag.snapshot()
        .map(|s| s.nodes.iter().map(|(a, _, _)| a.tau.0).max().unwrap_or(0))
        .unwrap_or(0)
}

fn collect_compressable(
    dag: &WorldlineDag,
    horizon: i64,
    pins: &RetrocausalPinSet,
) -> Vec<NodeId> {
    let Ok(snap) = dag.snapshot() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (addr, _, _) in snap.nodes {
        if addr.tau.0 > horizon {
            continue;
        }
        if let Some(id) = dag.node_id(addr) {
            if pins.is_pinned(id) {
                continue;
            }
            if let Some(node) = dag.node(id) {
                // Prefer already-sealed; otherwise seal candidates that are clean.
                if node.sealed || !node.dirty {
                    out.push(id);
                }
            }
        }
    }
    out
}

fn debris_nodes(dag: &WorldlineDag) -> Vec<NodeId> {
    // Nodes marked pruned stay mapped; collect dirty+pruned for re-prune noop safety.
    // WorldlineDag doesn't expose iterator — use snapshot addresses + dirty list.
    let mut ids = dag.dirty_nodes();
    if let Ok(snap) = dag.snapshot() {
        for (addr, _, _) in snap.nodes {
            if let Some(id) = dag.node_id(SpacetimeAddr {
                address: addr.address,
                tau: addr.tau,
            }) {
                if let Some(n) = dag.node(id) {
                    if n.state.pruned {
                        ids.push(id);
                    }
                }
            }
        }
    }
    ids.sort_by_key(|n| n.0);
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_dag::{DependencyKind, NodeState};
    use ctc_pruner::BranchManager;

    #[test]
    fn gc_compresses_old_epochs_while_preserving_retrocausal_pins() {
        let mut gc = TimelineGc::new(GcConfig {
            seal_horizon: 0, // everything at max_tau is kept; older compresses
            ..GcConfig::default()
        });
        let mut dag = WorldlineDag::new();
        let a0 = dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.1));
        let a1 = dag.allocate(SpacetimeAddr::new(0, 1), NodeState::scalar(0.2));
        let a2 = dag.allocate(SpacetimeAddr::new(0, 2), NodeState::scalar(0.3));
        dag.add_dependency(a2, a1, DependencyKind::Retrocausal).unwrap();
        // a0 is historical, not pinned by retrocausal edge (a2↔a1).
        let _ = a0;

        let branches = BranchManager::new();
        let _ = branches.seed_root(&dag, 1e-12);

        let report = gc.collect(&mut dag, &branches).unwrap();
        assert!(report.stats.checkpoints_created >= 1 || report.stats.nodes_pruned >= 1);
        // Retrocausal endpoints must remain present and unpruned.
        assert!(!dag.node(a1).unwrap().state.pruned);
        assert!(!dag.node(a2).unwrap().state.pruned);
    }
}
