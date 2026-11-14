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
        }
    }
}
