use serde::{Deserialize, Serialize};
use std::fmt;

/// Proper-time / epoch coordinate \(\tau \in \mathbb{Z}\).
///
/// Negative epochs are permitted: they represent pre-boundary conditions
/// injected into a CTC loop before the chronological horizon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Epoch(pub i64);

impl Epoch {
    pub const ORIGIN: Epoch = Epoch(0);

    #[inline]
    pub fn succ(self) -> Self {
        Epoch(self.0.saturating_add(1))
    }

    #[inline]
    pub fn pred(self) -> Self {
        Epoch(self.0.saturating_sub(1))
    }

    #[inline]
    pub fn delta(self, other: Self) -> i64 {
        self.0 - other.0
    }
}

impl fmt::Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "τ={}", self.0)
    }
}

/// Logical address \(a\) — identity of a chronal register, independent of epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LogicalAddr(pub u64);

impl fmt::Display for LogicalAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a={:#x}", self.0)
    }
}

/// Spacetime coordinate \((a, \tau)\) — the sole legal memory key in Cronos-CTC.
///
/// ## Ordering
///
/// Lexicographic on \((a, \tau)\). This is **not** a causal order; causal order
/// is encoded by edges in [`crate::WorldlineDag`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpacetimeAddr {
    pub address: LogicalAddr,
    pub tau: Epoch,
}

impl SpacetimeAddr {
    #[inline]
    pub fn new(address: u64, tau: i64) -> Self {
        Self {
            address: LogicalAddr(address),
            tau: Epoch(tau),
        }
    }

    /// Shift the epoch coordinate while preserving logical identity.
    #[inline]
    pub fn at_epoch(self, tau: Epoch) -> Self {
        Self {
            address: self.address,
            tau,
        }
    }

    /// True when `self` is chronologically before `other` on the same worldline
    /// (same logical address, strictly earlier \(\tau\)).
    #[inline]
    pub fn precedes_on_worldline(self, other: Self) -> bool {
        self.address == other.address && self.tau < other.tau
    }
}

impl fmt::Display for SpacetimeAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.address, self.tau)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacetime_ordering_is_lexicographic() {
        let a = SpacetimeAddr::new(1, 5);
        let b = SpacetimeAddr::new(1, 6);
        let c = SpacetimeAddr::new(2, 0);
        assert!(a < b);
        assert!(b < c);
        assert!(a.precedes_on_worldline(b));
        assert!(!a.precedes_on_worldline(c));
    }
}
