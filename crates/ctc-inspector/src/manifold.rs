use crate::error::{InspectorError, InspectorResult};
use ctc_dag::WorldlineDag;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeView {
    pub address: u64,
    pub tau: i64,
    pub value: Vec<f64>,
    pub weight: f64,
    pub dirty: bool,
    pub pruned: bool,
    pub sealed: bool,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifoldSlice {
    pub tau: i64,
    pub nodes: Vec<NodeView>,
    pub retrocausal_degree: usize,
}

/// Spatial projection of the worldline DAG onto proper-time slices.
pub struct ManifoldView<'a> {
    dag: &'a WorldlineDag,
}

impl<'a> ManifoldView<'a> {
    pub fn new(dag: &'a WorldlineDag) -> Self {
        Self { dag }
    }

    pub fn tau_range(&self) -> InspectorResult<(i64, i64)> {
        let snap = self.dag.snapshot().map_err(|_| InspectorError::NoFabric)?;
        let min = snap.nodes.iter().map(|(a, _, _)| a.tau.0).min().unwrap_or(0);
        let max = snap.nodes.iter().map(|(a, _, _)| a.tau.0).max().unwrap_or(0);
        Ok((min, max))
    }

    pub fn slice(&self, tau: i64) -> InspectorResult<ManifoldSlice> {
        let (min, max) = self.tau_range()?;
        if tau < min || tau > max {
            return Err(InspectorError::TauOutOfRange {
                requested: tau,
                min,
                max,
            });
        }
        let snap = self.dag.snapshot().map_err(|_| InspectorError::NoFabric)?;
        let mut nodes = Vec::new();
        for (addr, val, weight) in snap.nodes {
            if addr.tau.0 != tau {
                continue;
            }
            let meta = self.dag.lookup(addr);
            nodes.push(NodeView {
                address: addr.address.0,
                tau: addr.tau.0,
                value: val.iter().copied().collect(),
                weight,
                dirty: meta.map(|n| n.dirty).unwrap_or(false),
                pruned: meta.map(|n| n.state.pruned).unwrap_or(false),
                sealed: meta.map(|n| n.sealed).unwrap_or(false),
                revision: meta.map(|n| n.state.revision).unwrap_or(0),
            });
        }
        nodes.sort_by_key(|n| n.address);

        let retrocausal_degree = self
            .dag
            .retrocausal_edges()
            .iter()
            .filter(|(from, to)| {
                let ft = self.dag.node(*from).map(|n| n.addr.tau.0);
                let tt = self.dag.node(*to).map(|n| n.addr.tau.0);
                ft == Some(tau) || tt == Some(tau)
            })
            .count();

        Ok(ManifoldSlice {
            tau,
            nodes,
            retrocausal_degree,
        })
    }

    /// Windowed scrub: slices in \([\tau - w, \tau + w] \cap [min, max]\).
    pub fn window(&self, tau: i64, half_width: i64) -> InspectorResult<Vec<ManifoldSlice>> {
        let (min, max) = self.tau_range()?;
        let lo = (tau - half_width).max(min);
        let hi = (tau + half_width).min(max);
        let mut out = Vec::new();
        for t in lo..=hi {
            out.push(self.slice(t)?);
        }
        Ok(out)
    }

    /// ASCII manifold rendering for terminal UIs.
    pub fn render_ascii(&self, tau: i64, half_width: i64) -> InspectorResult<String> {
        let slices = self.window(tau, half_width)?;
        let mut out = String::new();
        out.push_str(&format!(
            "╔══ Worldline Manifold  τ∈[{}, {}]  cursor=τ{} ══╗\n",
            slices.first().map(|s| s.tau).unwrap_or(tau),
            slices.last().map(|s| s.tau).unwrap_or(tau),
            tau
        ));
        for slice in &slices {
            let marker = if slice.tau == tau { "▶" } else { " " };
            out.push_str(&format!(
                "{} τ{:<4} │ nodes={:<3} retro={} │",
                marker,
                slice.tau,
                slice.nodes.len(),
                slice.retrocausal_degree
            ));
            for n in &slice.nodes {
                let flag = if n.dirty {
                    "D"
                } else if n.pruned {
                    "P"
                } else if n.sealed {
                    "S"
                } else {
                    "·"
                };
                let v = n.value.first().copied().unwrap_or(0.0);
                out.push_str(&format!(" a{:#x}={:.4}[{flag}]", n.address, v));
            }
            out.push('\n');
        }
        out.push_str("╚════════════════════════════════════════════════╝\n");
        Ok(out)
    }
}
