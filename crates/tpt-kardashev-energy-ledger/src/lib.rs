// SPDX-License-Identifier: MIT OR Apache-2.0
//! Double-entry energy accounting with explicit waste heat, Landauer bounds, Stefan-Boltzmann
//! radiator capacity and an `EnergyOracle` hook for schedulers.
//!
//! SI units throughout. The crate is `no_std`, uses no allocation and has no runtime dependencies.
//! It does not model supply systems, power electronics or thermal transients. Those belong to
//! `tpt-equinox-power` and `tpt-thermodynamics` in the older repositories.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod error;
pub mod ledger;
pub mod oracle;
pub mod physics;
pub mod units;

pub use error::Error;
pub use ledger::{AccountId, AccountKind, Ledger};
pub use oracle::{EnergyOracle, Erase, ErasureOracle, HeatBudget};
pub use physics::{landauer_limit, radiator_capacity};
pub use units::{Joules, Kelvin, Watts};
