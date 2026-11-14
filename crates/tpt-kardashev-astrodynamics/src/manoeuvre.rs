// SPDX-License-Identifier: MIT OR Apache-2.0
//! Impulsive manoeuvres between circular orbits and changes of plane.
//!
//! Each burn is instantaneous, so only the change in velocity (Δv, in km/s) and the transfer time
//! are returned. Radii are in km and μ in km³/s². The Δv values are magnitudes. Each burn is
//! applied along the velocity, so the sign of the change is set by the transfer's direction.
//!
//! Not modelled: finite burns, the gravity loss during them, and plane changes combined with
//! apsidal burns beyond the single-burn law of cosines in [`plane_change`].

use crate::Error;
use core::f64::consts::PI;

/// A two-burn Hohmann transfer between circular orbits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hohmann {
    /// Δv of the burn at the inner orbit (km/s).
    pub dv1: f64,
    /// Δv of the burn at the outer orbit (km/s).
    pub dv2: f64,
    /// Total Δv (km/s).
    pub total: f64,
    /// Time from the first burn to the second (s): half the transfer ellipse's period.
    pub time_of_flight: f64,
}

/// A three-burn bi-elliptic transfer between circular orbits, through an intermediate radius
/// `rb` beyond both ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiElliptic {
    /// Δv of the burn at the inner orbit (km/s).
    pub dv1: f64,
    /// Δv of the burn at the intermediate radius (km/s).
    pub dv2: f64,
    /// Δv of the burn at the outer orbit (km/s).
    pub dv3: f64,
    /// Total Δv (km/s).
    pub total: f64,
    /// Total transfer time (s): half the period of each of the two ellipses.
    pub time_of_flight: f64,
}

fn check_mu(mu: f64) -> Result<(), Error> {
    if mu.is_finite() && mu > 0.0 {
        Ok(())
    } else {
        Err(Error::InvalidMu)
    }
}

fn check_radius(r: f64) -> Result<(), Error> {
    if r.is_finite() && r > 0.0 {
        Ok(())
    } else {
        Err(Error::InvalidRadius)
    }
}

/// The Hohmann transfer from the circular orbit of radius `r1` to that of radius `r2`.
pub fn hohmann(mu: f64, r1: f64, r2: f64) -> Result<Hohmann, Error> {
    check_mu(mu)?;
    check_radius(r1)?;
    check_radius(r2)?;
    let a = 0.5 * (r1 + r2);
    let v1 = (mu / r1).sqrt();
    let v2 = (mu / r2).sqrt();
    let transfer_at_r1 = (mu * (2.0 / r1 - 1.0 / a)).sqrt();
    let transfer_at_r2 = (mu * (2.0 / r2 - 1.0 / a)).sqrt();
    let dv1 = (transfer_at_r1 - v1).abs();
    let dv2 = (v2 - transfer_at_r2).abs();
    Ok(Hohmann {
        dv1,
        dv2,
        total: dv1 + dv2,
        time_of_flight: PI * (a.powi(3) / mu).sqrt(),
    })
}

/// The bi-elliptic transfer from the circular orbit of radius `r1` to `r2`, through the
/// intermediate radius `rb`. Fails with [`Error::InvalidRadius`] unless `rb` exceeds both `r1` and
/// `r2`.
pub fn bi_elliptic(mu: f64, r1: f64, rb: f64, r2: f64) -> Result<BiElliptic, Error> {
    check_mu(mu)?;
    check_radius(r1)?;
    check_radius(r2)?;
    check_radius(rb)?;
    if rb <= r1 || rb <= r2 {
        return Err(Error::InvalidRadius);
    }
    let a1 = 0.5 * (r1 + rb);
    let a2 = 0.5 * (rb + r2);
    let dv1 = ((mu * (2.0 / r1 - 1.0 / a1)).sqrt() - (mu / r1).sqrt()).abs();
    let dv2 = ((mu * (2.0 / rb - 1.0 / a1)).sqrt() - (mu * (2.0 / rb - 1.0 / a2)).sqrt()).abs();
    let dv3 = ((mu / r2).sqrt() - (mu * (2.0 / r2 - 1.0 / a2)).sqrt()).abs();
    Ok(BiElliptic {
        dv1,
        dv2,
        dv3,
        total: dv1 + dv2 + dv3,
        time_of_flight: PI * ((a1.powi(3) / mu).sqrt() + (a2.powi(3) / mu).sqrt()),
    })
}

/// The Δv needed to turn a velocity of magnitude `v1` through `angle` (rad) into one of magnitude
/// `v2`, by the law of cosines: `sqrt(v1² + v2² − 2·v1·v2·cos angle)`.
///
/// With `v1 = v2 = v`, this is the pure plane change `2·v·sin(angle/2)`. Fails with
/// [`Error::NonFinite`] for non-finite inputs.
pub fn plane_change(v1: f64, v2: f64, angle: f64) -> Result<f64, Error> {
    for (value, what) in [(v1, "speed"), (v2, "speed"), (angle, "angle")] {
        if !value.is_finite() {
            return Err(Error::NonFinite { what });
        }
    }
    let squared = v1 * v1 + v2 * v2 - 2.0 * v1 * v2 * angle.cos();
    Ok(squared.max(0.0).sqrt())
}
