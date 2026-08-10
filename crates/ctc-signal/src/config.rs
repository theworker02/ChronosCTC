use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Maximum residual \(\|F(x)-x\|_2\) permitted after injection (strict mode).
    pub deutsch_tolerance: f64,
    /// When true, run a full residual check against a provided evolution map.
    pub strict_deutsch: bool,
    /// Maximum payload cells per packet.
    pub max_payload_cells: usize,
    /// Require cryptographic worldline binding match.
    pub require_binding: bool,
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            deutsch_tolerance: 1e-8,
            strict_deutsch: false,
            max_payload_cells: 4096,
            require_binding: true,
        }
    }
}
