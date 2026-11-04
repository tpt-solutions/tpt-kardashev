// SPDX-License-Identifier: MIT OR Apache-2.0
//! Special and weak-field relativity: intervals, boosts, proper time, causality and time scales.
//!
//! Units are SI: seconds, metres, metres per second, and `GM` in m^3/s^2. Internally the
//! crate uses `c` explicitly and does not set `c = 1`.
//!
//! Numeric comparisons follow `docs/tolerance-policy.md`. Interval classification uses
//! [`event::LIGHT_TOLERANCE`].
//!
//! This crate has no runtime dependencies. Its elementary functions are in `math`.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod causal;
pub mod error;
pub mod event;
pub mod frame;
pub mod gravity;
pub mod kinematics;
pub mod lorentz;
pub mod metric;
pub mod timescale;
pub mod units;

mod math;

pub use causal::{CausalOrder, causally_disconnected, light_cone_contains};
pub use error::Error;
pub use event::{Event, FourVector, Interval, IntervalKind, interval};
pub use frame::Frame;
pub use kinematics::{
    contracted_length, dilated_duration, doppler_ratio_radial, lorentz_gamma, proper_duration,
    proper_time,
};
pub use lorentz::{Lorentz, Matrix4};
pub use metric::{Metric, Metric4, Minkowski, Schwarzschild};
pub use units::{C, Metres, Seconds};
