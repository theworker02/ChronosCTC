use crate::config::ContinuumConfig;
use crate::error::{ContinuumError, ContinuumResult};
use ctc_wormhole::{open_portal, receive, transmit, WormholePortal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionState {
    pub name: String,
    pub tick: u64,
    pub energy: f64,
    pub packets_sent: u64,
    pub packets_recv: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederationTick {
    pub tick: u64,
    pub regions_active: usize,
    pub portals_open: usize,
    pub packets_routed: u64,
    pub mean_energy: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinuumReport {
    pub ticks: Vec<FederationTick>,
    pub regions: Vec<RegionState>,
    pub portals_linked: usize,
    pub total_packets: u64,
    pub message: String,
}

/// Synthetic in-memory federated continuum runtime.
pub struct ContinuumRuntime {
    pub config: ContinuumConfig,
    regions: HashMap<String, RegionState>,
    portals: Vec<WormholePortal>,
    next_portal_id: u64,
    tick: u64,
}

impl Default for ContinuumRuntime {
    fn default() -> Self {
        Self::new(ContinuumConfig::default())
    }
}

impl ContinuumRuntime {
    pub fn new(config: ContinuumConfig) -> Self {
        Self {
            config,
            regions: HashMap::new(),
            portals: Vec::new(),
            next_portal_id: 1,
            tick: 0,
        }
    }

    /// Admit a named continuum region into the federation.
    pub fn admit(&mut self, name: impl Into<String>) -> ContinuumResult<()> {
        let name = name.into();
        if self.regions.contains_key(&name) {
            return Err(ContinuumError::DuplicateRegion(name));
        }
        if self.regions.len() >= self.config.max_regions {
            return Err(ContinuumError::TickBudget(self.config.max_regions as u64));
        }
        self.regions.insert(
            name.clone(),
            RegionState {
                name,
                tick: 0,
                energy: 1.0,
                packets_sent: 0,
                packets_recv: 0,
            },
        );
        Ok(())
    }

    /// Link two admitted regions with a wormhole portal.
    pub fn link(&mut self, region_a: &str, region_b: &str) -> ContinuumResult<u64> {
        if !self.regions.contains_key(region_a) {
            return Err(ContinuumError::UnknownRegion(region_a.to_string()));
        }
        if !self.regions.contains_key(region_b) {
            return Err(ContinuumError::UnknownRegion(region_b.to_string()));
        }
        let id = self.next_portal_id;
        self.next_portal_id += 1;
        let portal = open_portal(
            id,
            region_a,
            region_b,
            self.config.portal_capacity,
        );
        self.portals.push(portal);
        Ok(id)
    }

    /// Advance the federation by one tick: route synthetic packets through portals.
    pub fn tick_once(&mut self) -> ContinuumResult<FederationTick> {
        if self.regions.is_empty() {
            return Err(ContinuumError::EmptyFederation);
        }
        self.tick += 1;
        let mut packets_routed = 0u64;

        // Each region sends a heartbeat scalar through every linked portal.
        let region_names: Vec<String> = self.regions.keys().cloned().collect();
        for portal in &mut self.portals {
            for (idx, from) in region_names.iter().enumerate() {
                if portal.region_a == *from || portal.region_b == *from {
                    let payload = vec![(self.tick as f64) + (idx as f64) * 0.01];
                    if transmit(portal, from, payload).is_ok() {
                        if let Some(r) = self.regions.get_mut(from) {
                            r.packets_sent += 1;
                            r.energy *= 0.995;
                        }
                        packets_routed += 1;
                    }
                }
            }
        }

        for portal in &self.portals {
            for to in &region_names {
                if portal.region_a == *to || portal.region_b == *to {
                    if let Ok(pkt) = receive(portal, to) {
                        if let Some(r) = self.regions.get_mut(to) {
                            r.packets_recv += 1;
                            r.energy += 0.001 * pkt.hop_count as f64;
                        }
                        packets_routed += 1;
                    }
                }
            }
        }

        for r in self.regions.values_mut() {
            r.tick = self.tick;
        }

        let mean_energy = if self.regions.is_empty() {
            0.0
        } else {
            self.regions.values().map(|r| r.energy).sum::<f64>() / self.regions.len() as f64
        };

        Ok(FederationTick {
            tick: self.tick,
            regions_active: self.regions.len(),
            portals_open: self.portals.iter().filter(|p| p.is_open()).count(),
            packets_routed,
            mean_energy,
        })
    }

    /// Run `config.federation_ticks` federation ticks and return a report.
    pub fn tick_federation(&mut self) -> ContinuumResult<ContinuumReport> {
        let budget = self.config.federation_ticks;
        let mut ticks = Vec::with_capacity(budget as usize);
        for _ in 0..budget {
            ticks.push(self.tick_once()?);
        }
        let total_packets: u64 = self.regions.values().map(|r| r.packets_sent + r.packets_recv).sum();
        let regions: Vec<RegionState> = self.regions.values().cloned().collect();
        Ok(ContinuumReport {
            ticks,
            regions,
            portals_linked: self.portals.len(),
            total_packets,
            message: format!(
                "federation advanced {} ticks across {} regions via {} wormholes",
                budget,
                self.regions.len(),
                self.portals.len()
            ),
        })
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub fn portal_count(&self) -> usize {
        self.portals.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admit_link_and_tick() {
        let mut rt = ContinuumRuntime::default();
        rt.admit("alpha").unwrap();
        rt.admit("beta").unwrap();
        rt.admit("gamma").unwrap();
        let pid = rt.link("alpha", "beta").unwrap();
        assert!(pid >= 1);
        let report = rt.tick_federation().unwrap();
        assert_eq!(report.ticks.len(), 4);
        assert_eq!(report.regions.len(), 3);
        assert!(report.total_packets > 0);
    }

    #[test]
    fn duplicate_region_rejected() {
        let mut rt = ContinuumRuntime::default();
        rt.admit("solo").unwrap();
        assert!(rt.admit("solo").is_err());
    }
}
