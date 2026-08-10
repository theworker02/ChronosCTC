//! # Thermodynamic Landauer Compiler (`ctc-entropy`)
//!
//! Couples information processing to physical thermodynamics via Landauer's
//! Principle. Bit erasure and paradox / multiverse pruning are treated as
//! thermodynamic operations that harvest or dissipate energy:
//!
//! \[
//! E_{\min} = k_B T \ln 2 \cdot N_{\mathrm{erased}}
//! \]
//!
//! Absolute convergence (\(r \rightarrow 0\)) approaches a zero-energy
//! computational fixed point.

mod balancer;
mod config;
mod error;
mod landauer;

pub use balancer::{EnergyLedger, ThermoBalancer, ThermoReport};
pub use config::EntropyConfig;
pub use error::{EntropyError, EntropyResult};
pub use landauer::{landauer_energy_joules, LandauerOp, ThermoEvent};
