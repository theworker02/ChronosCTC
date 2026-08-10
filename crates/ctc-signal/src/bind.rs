use ctc_dag::WorldlineDag;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Cryptographic (keyed-hash) binding of a packet to a worldline snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldlineBinding {
    pub fingerprint: u64,
    /// Generation counter from the DAG at bind time.
    pub generation: u64,
}

/// Compute a stable fingerprint over non-pruned worldline cells.
///
/// \[
/// H(W) = \mathrm{hash}\big(\{(a_i,\tau_i,x_i,r_i)\}\big)
/// \]
///
/// Ordering is lexicographic on \((a,\tau)\) so the hash is deterministic.
pub fn worldline_fingerprint(dag: &WorldlineDag) -> WorldlineBinding {
    let mut hasher = FxHasher::default();
    dag.branch_generation.hash(&mut hasher);
    if let Ok(snap) = dag.snapshot() {
        for (addr, val, weight) in &snap.nodes {
            addr.address.0.hash(&mut hasher);
            addr.tau.0.hash(&mut hasher);
            for v in val.iter() {
                // Hash bits of f64 for stability.
                v.to_bits().hash(&mut hasher);
            }
            weight.to_bits().hash(&mut hasher);
        }
    }
    // Mix in retrocausal topology.
    let mut edges = dag.retrocausal_edges();
    edges.sort_by_key(|(a, b)| (a.0, b.0));
    for (a, b) in edges {
        a.0.hash(&mut hasher);
        b.0.hash(&mut hasher);
    }
    WorldlineBinding {
        fingerprint: hasher.finish(),
        generation: dag.branch_generation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_dag::{NodeState, SpacetimeAddr};

    #[test]
    fn fingerprint_stable_for_identical_fabric() {
        let mut dag = WorldlineDag::new();
        dag.allocate(SpacetimeAddr::new(1, 0), NodeState::scalar(0.5));
        let a = worldline_fingerprint(&dag);
        let b = worldline_fingerprint(&dag);
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_changes_on_mutation() {
        let mut dag = WorldlineDag::new();
        dag.allocate(SpacetimeAddr::new(1, 0), NodeState::scalar(0.5));
        let a = worldline_fingerprint(&dag);
        dag.allocate(SpacetimeAddr::new(1, 1), NodeState::scalar(0.9));
        let b = worldline_fingerprint(&dag);
        assert_ne!(a.fingerprint, b.fingerprint);
    }
}
