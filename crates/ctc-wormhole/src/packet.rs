use ctc_signal::TemporalPacket;
use serde::{Deserialize, Serialize};

/// Payload envelope stamped for wormhole transit between continuum regions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WormholePacket {
    pub portal_id: u64,
    pub sequence: u64,
    pub from_region: String,
    pub to_region: String,
    pub payload: TemporalPacket,
    pub hop_count: u32,
}

impl WormholePacket {
    pub fn new(
        portal_id: u64,
        sequence: u64,
        from_region: impl Into<String>,
        to_region: impl Into<String>,
        payload: TemporalPacket,
    ) -> Self {
        Self {
            portal_id,
            sequence,
            from_region: from_region.into(),
            to_region: to_region.into(),
            payload,
            hop_count: 0,
        }
    }
}
