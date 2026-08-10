use ctc_dag::{Epoch, LogicalAddr};
use ctc_signal::{ExpectedFootprint, WorldlineBinding};
use serde::{Deserialize, Serialize};

/// Lifecycle of a temporally superposed execution frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuperpositionState {
    /// Frame has not yet reached the injection point.
    Armed,
    /// Holding breath — awaiting `ctc-signal` packet from the future.
    Awaiting,
    /// Future state arrived; variables instantiated; timeline collapsed.
    Collapsed,
    /// Wait aborted / paradox — frame must fall back to classical solve.
    Aborted,
}

/// A program frame held in temporal superposition at an injection boundary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalSuperposition {
    pub name: String,
    pub epoch: Epoch,
    pub state: SuperpositionState,
    pub footprint_hash: u64,
    pub binding: WorldlineBinding,
    pub slots: Vec<(LogicalAddr, usize)>,
    /// Estimated classical cycles that will be skipped on collapse.
    pub estimated_cycles_saved: u64,
    /// Cycles actually skipped (set on collapse).
    pub cycles_saved: u64,
}

impl TemporalSuperposition {
    pub fn new(
        name: impl Into<String>,
        epoch: Epoch,
        footprint: &ExpectedFootprint,
        estimated_cycles_saved: u64,
    ) -> Self {
        Self {
            name: name.into(),
            epoch,
            state: SuperpositionState::Armed,
            footprint_hash: footprint.hash,
            binding: footprint.binding,
            slots: footprint.slots.clone(),
            estimated_cycles_saved,
            cycles_saved: 0,
        }
    }

    pub fn enter_wait(&mut self) {
        if self.state == SuperpositionState::Armed {
            self.state = SuperpositionState::Awaiting;
        }
    }

    pub fn collapse(&mut self) {
        self.state = SuperpositionState::Collapsed;
        self.cycles_saved = self.estimated_cycles_saved;
    }

    pub fn abort(&mut self) {
        self.state = SuperpositionState::Aborted;
        self.cycles_saved = 0;
    }
}
