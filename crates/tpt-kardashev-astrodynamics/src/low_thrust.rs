// SPDX-License-Identifier: MIT OR Apache-2.0
//! Edelbaum's approximation for low-thrust transfers between circular orbits with a change of plane.
//!
//! The thrust acceleration is small and constant in magnitude, and the orbit stays nearly circular
//! throughout. Edelbaum's Δv is the law of cosines between the two circular speeds, with the angle
//! `π/2 · Δi` (Δi in radians): `Δv = sqrt(v1² + v2² − 2·v1·v2·cos(π/2 · Δi))`. The time of flight is
//! the Δv divided by the acceleration.
//!
//! This is an approximation. It is accurate when the change in radius and inclination is small, and
//! its time of flight treats the acceleration as all used for the Δv. Not modelled: the optimal
//! steering law beyond the stated result, the eccentricity and the argument of perigee that grow during
//! a real spiral, eclipses, and the mass flow of the vehicle. For a pure plane change, the continuous
//! Δv is `π/2` times the impulsive one for small angles; this module does not claim the impulsive case.

use core::f64::consts::FRAC_PI_2;

use crate::Error;
use crate::manoeuvre::plane_change;

/// An Edelbaum transfer: the total Δv (km/s) and its time of flight (s) at the given acceleration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edelbaum {
    /// The total Δv (km/s).
    pub delta_v: f64,
    /// The time of flight (s), the Δv divided by the constant acceleration.
    pub time_of_flight: f64,
}

/// Edelbaum's transfer from the circular orbit of radius `r1` to that of radius `r2`, with a change of
/// plane of `inclination_change` (rad) and a constant thrust acceleration `acceleration` (km/s²).
///
/// Fails with [`Error::InvalidMu`] for a gravitational parameter that is not positive and finite,
/// with [`Error::InvalidRadius`] for a radius that is not positive and finite, with
/// [`Error::NonFinite`] for a non-finite inclination change, and with [`Error::InvalidAcceleration`]
/// for an acceleration that is not positive and finite.
pub fn edelbaum(
    mu: f64,
    r1: f64,
    r2: f64,
    inclination_change: f64,
    acceleration: f64,
) -> Result<Edelbaum, Error> {
    if !(mu.is_finite() && mu > 0.0) {
        return Err(Error::InvalidMu);
    }
    for r in [r1, r2] {
        if !(r.is_finite() && r > 0.0) {
            return Err(Error::InvalidRadius);
        }
    }
    if !inclination_change.is_finite() {
        return Err(Error::NonFinite {
            what: "inclination change",
        });
    }
    if !(acceleration.is_finite() && acceleration > 0.0) {
        return Err(Error::InvalidAcceleration);
    }
    let v1 = (mu / r1).sqrt();
    let v2 = (mu / r2).sqrt();
    let delta_v = plane_change(v1, v2, FRAC_PI_2 * inclination_change)?;
    Ok(Edelbaum {
        delta_v,
        time_of_flight: delta_v / acceleration,
    })
}
