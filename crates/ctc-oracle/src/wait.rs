use crate::error::{OracleError, OracleResult};
use crate::superposition::SuperpositionState;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaitStatus {
    Pending,
    Ready,
    TimedOut,
}

/// Lightweight wait handle — spin/yield loop without heavy I/O.
pub struct WaitHandle {
    pub name: String,
    pub deadline: Option<Instant>,
    pub polls: u64,
}

impl WaitHandle {
    pub fn new(name: impl Into<String>, timeout: Option<Duration>) -> Self {
        Self {
            name: name.into(),
            deadline: timeout.map(|d| Instant::now() + d),
            polls: 0,
        }
    }

    /// Poll once. Returns TimedOut if deadline exceeded.
    pub fn poll(&mut self) -> WaitStatus {
        self.polls += 1;
        if let Some(dl) = self.deadline {
            if Instant::now() >= dl {
                return WaitStatus::TimedOut;
            }
        }
        WaitStatus::Pending
    }

    /// Busy-wait with a predicate (used in tests / single-threaded demos).
    pub fn wait_until<F>(&mut self, mut ready: F) -> OracleResult<u64>
    where
        F: FnMut() -> bool,
    {
        loop {
            if ready() {
                return Ok(self.polls);
            }
            match self.poll() {
                WaitStatus::TimedOut => {
                    return Err(OracleError::WaitTimeout(self.name.clone()));
                }
                WaitStatus::Pending | WaitStatus::Ready => {
                    // Lightweight yield — no syscall-heavy I/O.
                    std::thread::yield_now();
                }
            }
        }
    }
}

pub fn ensure_awaiting(state: SuperpositionState, name: &str) -> OracleResult<()> {
    match state {
        SuperpositionState::Awaiting => Ok(()),
        SuperpositionState::Collapsed => Err(OracleError::AlreadyCollapsed(name.into())),
        _ => Err(OracleError::InvalidCollapse(name.into())),
    }
}
