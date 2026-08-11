//! # Inter-Continuum Wormhole Transport (`ctc-wormhole`)
//!
//! Synthetic in-memory wormhole portals tunnel `TemporalPacket` payloads across
//! federated chronal regions. Each portal is a bidirectional queue between two
//! named continuum endpoints; `transmit` / `receive` wrap enqueue/dequeue with
//! Deutsch-gate compatible envelopes.

mod error;
mod packet;
mod portal;

pub use error::{WormholeError, WormholeResult};
pub use packet::WormholePacket;
pub use portal::{open_portal, PortalId, WormholePortal};

use ctc_dag::SpacetimeAddr;
use ctc_signal::{TemporalPacketBuilder, WorldlineBinding};

fn build_packet(values: &[f64]) -> ctc_signal::TemporalPacket {
    let binding = WorldlineBinding {
        fingerprint: 0xfeed,
        generation: 1,
    };
    let pairs: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(i, v)| (SpacetimeAddr::new(i as u64, 0), *v))
        .collect();
    TemporalPacketBuilder::new()
        .package_scalars(ctc_dag::Epoch(2), ctc_dag::Epoch(0), binding, &pairs)
        .expect("synthetic wormhole packet")
}

fn packet_values(packet: &ctc_signal::TemporalPacket) -> Vec<f64> {
    packet
        .cells
        .iter()
        .flat_map(|c| c.values.iter().copied())
        .collect()
}

/// Transmit a payload from `from_region` through `portal` toward the peer endpoint.
pub fn transmit(
    portal: &mut WormholePortal,
    from_region: &str,
    payload: Vec<f64>,
) -> WormholeResult<u64> {
    let to = portal.other_endpoint(from_region)?.to_string();
    let packet = build_packet(&payload);
    let env = WormholePacket::new(portal.id.0, 0, from_region, to, packet);
    portal.enqueue(env)
}

/// Receive the next packet destined for `to_region` on `portal`.
pub fn receive(portal: &WormholePortal, to_region: &str) -> WormholeResult<WormholePacket> {
    portal.dequeue_for(to_region)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_through_portal() {
        let mut portal = open_portal(1, "alpha", "beta", 8);
        let seq = transmit(&mut portal, "alpha", vec![1.0, 2.0, 3.0]).unwrap();
        assert_eq!(seq, 1);
        assert_eq!(portal.pending(), 1);

        let pkt = receive(&portal, "beta").unwrap();
        assert_eq!(pkt.sequence, 1);
        assert_eq!(pkt.from_region, "alpha");
        assert_eq!(pkt.to_region, "beta");
        assert_eq!(packet_values(&pkt.payload), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn bidirectional_transit() {
        let mut portal = open_portal(7, "east", "west", 4);
        transmit(&mut portal, "east", vec![0.5]).unwrap();
        transmit(&mut portal, "west", vec![0.9]).unwrap();
        let a = receive(&portal, "west").unwrap();
        let b = receive(&portal, "east").unwrap();
        assert_eq!(packet_values(&a.payload), &[0.5]);
        assert_eq!(packet_values(&b.payload), &[0.9]);
    }

    #[test]
    fn closed_portal_rejects() {
        let mut portal = open_portal(2, "a", "b", 2);
        portal.close();
        assert!(transmit(&mut portal, "a", vec![1.0]).is_err());
    }
}
