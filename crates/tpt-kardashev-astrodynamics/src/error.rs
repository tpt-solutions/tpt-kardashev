// SPDX-License-Identifier: MIT OR Apache-2.0
//! Errors from the astrodynamics crate.

use core::fmt;

/// Errors from validating inputs to the astrodynamics crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A value is NaN or infinite. `what` names the quantity.
    NonFinite {
        /// The quantity that was not finite.
        what: &'static str,
    },
    /// The gravitational parameter must be positive and finite.
    InvalidMu,
    /// The orbit is not elliptic: the semi-major axis must be positive and the eccentricity in
    /// `[0, 1)`. Parabolic and hyperbolic orbits are not supported yet.
    NotElliptic,
    /// The state has zero angular momentum (zero radius, or position and velocity parallel), so no
    /// orbital plane is defined.
    Degenerate,
    /// A calendar date or time of day is out of range, or a UTC second is 60 or more (leap-second
    /// insertion is not modelled).
    InvalidCalendar,
    /// The instant is before the first entry of the leap-second table.
    NoLeapSecondEntry,
    /// The leap-second table is empty, or its dates or offsets are not in increasing order.
    InvalidLeapSeconds,
    /// The Newton iteration for the universal Kepler equation did not converge.
    NoConvergence,
    /// An integration step must be positive and finite.
    InvalidStep,
    /// A radius must be positive and finite, and for a bi-elliptic transfer the intermediate
    /// radius must exceed both end radii.
    InvalidRadius,
    /// A time of flight must be positive and finite.
    InvalidTimeOfFlight,
    /// A coefficient that must not be negative is negative. `what` names it.
    Negative {
        /// The quantity that is negative.
        what: &'static str,
    },
    /// A burn window must have a start no later than its end.
    InvalidWindow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NonFinite { what } => write!(f, "{what} is not finite"),
            Error::InvalidMu => {
                f.write_str("the gravitational parameter must be positive and finite")
            }
            Error::NotElliptic => f.write_str("the orbit is not elliptic"),
            Error::Degenerate => f.write_str("the state has zero angular momentum"),
            Error::InvalidCalendar => f.write_str("the calendar date or time is out of range"),
            Error::NoConvergence => f.write_str("the universal Kepler equation did not converge"),
            Error::InvalidStep => f.write_str("the integration step must be positive and finite"),
            Error::InvalidRadius => f.write_str("a radius is out of range for this transfer"),
            Error::Negative { what } => write!(f, "{what} must not be negative"),
            Error::InvalidWindow => f.write_str("the burn window must not end before it starts"),
            Error::InvalidTimeOfFlight => {
                f.write_str("the time of flight must be positive and finite")
            }
            Error::NoLeapSecondEntry => {
                f.write_str("the instant is before the first leap-second table entry")
            }
            Error::InvalidLeapSeconds => {
                f.write_str("the leap-second table is empty or not in increasing order")
            }
        }
    }
}
