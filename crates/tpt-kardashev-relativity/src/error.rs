// SPDX-License-Identifier: MIT OR Apache-2.0
//! Error type for the relativity crate.

use core::fmt;

use crate::units::C;

/// Errors returned by this crate. Library code never panics.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// An input was NaN or infinite.
    NonFinite,
    /// A speed was at or above the speed of light, or a velocity magnitude was not below `c`.
    SpeedNotSubluminal {
        /// The offending speed in m/s.
        speed: f64,
    },
    /// A parameter was outside its valid domain, such as a negative mass or speed.
    InvalidParameter {
        /// Name of the parameter.
        name: &'static str,
    },
    /// A radius lies at or inside the Schwarzschild radius, where the static-observer formulas do not apply.
    InsideHorizon {
        /// The radius supplied, in metres.
        radius: f64,
        /// The Schwarzschild radius for the given mass, in metres.
        schwarzschild_radius: f64,
    },
    /// A proper-time path was empty.
    EmptyPath,
    /// A path segment runs backwards in coordinate time.
    PastDirectedSegment,
    /// A path segment is spacelike, so it has no proper time.
    SpacelikeSegment,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NonFinite => write!(f, "input is not finite"),
            Error::SpeedNotSubluminal { speed } => {
                write!(f, "speed {speed} m/s is not below c = {C} m/s")
            }
            Error::InvalidParameter { name } => write!(f, "parameter `{name}` is out of range"),
            Error::InsideHorizon {
                radius,
                schwarzschild_radius,
            } => write!(
                f,
                "radius {radius} m is not outside the Schwarzschild radius {schwarzschild_radius} m"
            ),
            Error::EmptyPath => write!(f, "path has no events"),
            Error::PastDirectedSegment => write!(f, "path segment runs backwards in time"),
            Error::SpacelikeSegment => write!(f, "path segment is spacelike"),
        }
    }
}

impl core::error::Error for Error {}

/// Checks a radius: finite and strictly positive.
pub(crate) fn validate_radius(r: f64) -> Result<f64, Error> {
    if !r.is_finite() {
        return Err(Error::NonFinite);
    }
    if r <= 0.0 {
        return Err(Error::InvalidParameter { name: "radius" });
    }
    Ok(r)
}

/// Checks a speed magnitude: finite, non-negative and strictly below `c`.
pub(crate) fn validate_speed(speed: f64) -> Result<f64, Error> {
    if !speed.is_finite() {
        return Err(Error::NonFinite);
    }
    if speed < 0.0 {
        return Err(Error::InvalidParameter { name: "speed" });
    }
    if speed >= C {
        return Err(Error::SpeedNotSubluminal { speed });
    }
    Ok(speed)
}
