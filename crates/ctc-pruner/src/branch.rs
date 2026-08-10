use crate::error::{PrunerError, PrunerResult};
use ctc_dag::WorldlineDag;
use ctc_kernel::ConvergenceClass;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(pub u64);

/// A timeline branch: a worldline DAG revision plus convergence status.
#[derive(Clone, Debug)]
pub struct TimelineBranch {
    pub id: BranchId,
    pub parent: Option<BranchId>,
    pub generation: u64,
    pub status: ConvergenceClass,
    pub residual_norm: f64,
    pub stable: bool,
    /// Compact state checkpoint: (logical address index, epoch, values).
    pub checkpoint: Vec<(u64, i64, Vec<f64>)>,
}

/// Manages parallel timeline search — concurrent candidate worldlines.
pub struct BranchManager {
    next_id: RwLock<u64>,
    branches: RwLock<FxHashMap<BranchId, TimelineBranch>>,
    active: RwLock<Option<BranchId>>,
}

impl Default for BranchManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchManager {
    pub fn new() -> Self {
        Self {
            next_id: RwLock::new(1),
            branches: RwLock::new(FxHashMap::default()),
            active: RwLock::new(None),
        }
    }

    pub fn active(&self) -> Option<BranchId> {
        *self.active.read()
    }

    pub fn get(&self, id: BranchId) -> Option<TimelineBranch> {
        self.branches.read().get(&id).cloned()
    }

    /// Register a root branch from the current worldline fabric.
    pub fn seed_root(&self, dag: &WorldlineDag, residual_norm: f64) -> BranchId {
        let id = self.alloc_id();
        let branch = TimelineBranch {
            id,
            parent: None,
            generation: dag.branch_generation,
            status: ConvergenceClass::Unique,
            residual_norm,
            stable: true,
            checkpoint: snapshot_dag(dag),
        };
        self.branches.write().insert(id, branch);
        *self.active.write() = Some(id);
        id
    }

    /// Fork a speculative alternative from `parent` (parallel timeline search).
    pub fn fork(&self, parent: BranchId, dag: &WorldlineDag) -> PrunerResult<BranchId> {
        let parent_branch = self
            .branches
            .read()
            .get(&parent)
            .cloned()
            .ok_or(PrunerError::UnknownBranch(parent.0))?;
        if matches!(parent_branch.status, ConvergenceClass::Paradox) {
            return Err(PrunerError::ForkFromParadox);
        }
        let id = self.alloc_id();
        let branch = TimelineBranch {
            id,
            parent: Some(parent),
            generation: dag.branch_generation,
            status: ConvergenceClass::Unique,
            residual_norm: parent_branch.residual_norm,
            stable: true,
            checkpoint: snapshot_dag(dag),
        };
        self.branches.write().insert(id, branch);
        Ok(id)
    }

    pub fn update_status(
        &self,
        id: BranchId,
        status: ConvergenceClass,
        residual_norm: f64,
    ) -> PrunerResult<()> {
        let mut branches = self.branches.write();
        let b = branches
            .get_mut(&id)
            .ok_or(PrunerError::UnknownBranch(id.0))?;
        b.status = status.clone();
        b.residual_norm = residual_norm;
        b.stable = !matches!(status, ConvergenceClass::Paradox);
        Ok(())
    }

    pub fn invalidate(&self, id: BranchId) -> PrunerResult<()> {
        let mut branches = self.branches.write();
        let b = branches
            .get_mut(&id)
            .ok_or(PrunerError::UnknownBranch(id.0))?;
        b.stable = false;
        b.status = ConvergenceClass::Paradox;
        if self.active.read().as_ref() == Some(&id) {
            // Active cleared; collapse must select a replacement.
            *self.active.write() = None;
        }
        Ok(())
    }

    /// Select the nearest stable alternative to `failed` by residual proximity
    /// and genealogical distance.
    pub fn nearest_stable(&self, failed: BranchId) -> PrunerResult<BranchId> {
        let branches = self.branches.read();
        let failed_branch = branches
            .get(&failed)
            .ok_or(PrunerError::UnknownBranch(failed.0))?;

        let mut best: Option<(BranchId, f64)> = None;
        for (id, b) in branches.iter() {
            if *id == failed || !b.stable {
                continue;
            }
            if matches!(b.status, ConvergenceClass::Paradox) {
                continue;
            }
            let gene = genealogical_distance(&branches, failed, *id);
            let resid_delta = (b.residual_norm - failed_branch.residual_norm).abs();
            let score = gene as f64 + resid_delta;
            match best {
                None => best = Some((*id, score)),
                Some((_, s)) if score < s => best = Some((*id, score)),
                _ => {}
            }
        }
        best.map(|(id, _)| id).ok_or(PrunerError::NoStableAlternative)
    }

    pub fn collapse_to(&self, id: BranchId) -> PrunerResult<Arc<TimelineBranch>> {
        let branches = self.branches.read();
        let b = branches
            .get(&id)
            .ok_or(PrunerError::UnknownBranch(id.0))?;
        if !b.stable {
            return Err(PrunerError::NoStableAlternative);
        }
        *self.active.write() = Some(id);
        Ok(Arc::new(b.clone()))
    }

    pub fn list_stable(&self) -> Vec<BranchId> {
        self.branches
            .read()
            .iter()
            .filter(|(_, b)| b.stable)
            .map(|(id, _)| *id)
            .collect()
    }

    fn alloc_id(&self) -> BranchId {
        let mut n = self.next_id.write();
        let id = BranchId(*n);
        *n += 1;
        id
    }
}

fn snapshot_dag(dag: &WorldlineDag) -> Vec<(u64, i64, Vec<f64>)> {
    match dag.snapshot() {
        Ok(snap) => snap
            .nodes
            .iter()
            .map(|(addr, val, _)| (addr.address.0, addr.tau.0, val.iter().copied().collect()))
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn genealogical_distance(
    branches: &FxHashMap<BranchId, TimelineBranch>,
    a: BranchId,
    b: BranchId,
) -> usize {
    let ancestors = |mut id: BranchId| {
        let mut chain = vec![id];
        while let Some(parent) = branches.get(&id).and_then(|br| br.parent) {
            chain.push(parent);
            id = parent;
        }
        chain
    };
    let aa = ancestors(a);
    let bb = ancestors(b);
    for (i, x) in aa.iter().enumerate() {
        if let Some(j) = bb.iter().position(|y| y == x) {
            return i + j;
        }
    }
    aa.len() + bb.len()
}
