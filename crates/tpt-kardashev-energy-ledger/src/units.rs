// SPDX-License-Identifier: MIT OR Apache-2.0
//! Newtypes for energy, power and temperature. SI units throughout.

/// Energy in joules.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Joules(pub f64);

/// Power in watts.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Watts(pub f64);

/// Absolute temperature in kelvin.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Kelvin(pub f64);
