use crate::error::{WormholeError, WormholeResult};
use crate::packet::WormholePacket;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortalId(pub u64);

/// Bidirectional wormhole between two named continuum regions.
pub struct WormholePortal {
    pub id: PortalId,
    pub region_a: String,
    pub region_b: String,
    capacity: usize,
    open: bool,
    sequence: u64,
    queue: Mutex<VecDeque<WormholePacket>>,
}

impl WormholePortal {
    pub fn new(
        id: u64,
        region_a: impl Into<String>,
        region_b: impl Into<String>,
        capacity: usize,
    ) -> Self {
        Self {
            id: PortalId(id),
            region_a: region_a.into(),
            region_b: region_b.into(),
            capacity: capacity.max(1),
            open: true,
            sequence: 0,
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn other_endpoint(&self, region: &str) -> WormholeResult<&str> {
        if region == self.region_a {
            Ok(&self.region_b)
        } else if region == self.region_b {
            Ok(&self.region_a)
        } else {
            Err(WormholeError::EndpointMismatch {
                portal_id: self.id.0,
                expected: format!("{} or {}", self.region_a, self.region_b),
                actual: region.to_string(),
            })
        }
    }

    pub fn enqueue(&mut self, mut packet: WormholePacket) -> WormholeResult<u64> {
        if !self.open {
            return Err(WormholeError::PortalClosed(self.id.0));
        }
        self.other_endpoint(&packet.from_region)?;
        if packet.to_region != *self.other_endpoint(&packet.from_region)? {
            return Err(WormholeError::EndpointMismatch {
                portal_id: self.id.0,
                expected: self.other_endpoint(&packet.from_region)?.to_string(),
                actual: packet.to_region.clone(),
            });
        }
        let mut q = self.queue.lock();
        if q.len() >= self.capacity {
            return Err(WormholeError::QueueFull {
                portal_id: self.id.0,
                capacity: self.capacity,
            });
        }
        self.sequence += 1;
        packet.sequence = self.sequence;
        packet.portal_id = self.id.0;
        packet.hop_count += 1;
        let seq = packet.sequence;
        q.push_back(packet);
        Ok(seq)
    }

    pub fn dequeue_for(&self, region: &str) -> WormholeResult<WormholePacket> {
        if !self.open {
            return Err(WormholeError::PortalClosed(self.id.0));
        }
        self.other_endpoint(region)?;
        let mut q = self.queue.lock();
        let idx = q
            .iter()
            .position(|p| p.to_region == region)
            .ok_or_else(|| WormholeError::NoPacket {
                portal_id: self.id.0,
                endpoint: region.to_string(),
            })?;
        Ok(q.remove(idx).expect("index valid"))
    }

    pub fn pending(&self) -> usize {
        self.queue.lock().len()
    }
}

/// Open a wormhole portal between two continuum regions.
pub fn open_portal(
    id: u64,
    region_a: impl Into<String>,
    region_b: impl Into<String>,
    capacity: usize,
) -> WormholePortal {
    WormholePortal::new(id, region_a, region_b, capacity)
}
