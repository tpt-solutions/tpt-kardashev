// SPDX-License-Identifier: MIT OR Apache-2.0
//! Physical constants and the `Seconds` and `Metres` newtypes.

/// Speed of light in vacuum, m/s. Exact by definition of the metre.
pub const C: f64 = 299_792_458.0;

/// Seconds per day, used when reporting clock offsets.
pub const SECONDS_PER_DAY: f64 = 86_400.0;

/// A duration in seconds.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Seconds(pub f64);

/// A length in metres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Metres(pub f64);
