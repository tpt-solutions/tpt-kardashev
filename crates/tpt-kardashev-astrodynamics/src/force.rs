// SPDX-License-Identifier: MIT OR Apache-2.0
//! Accelerations for the numerical propagator.
//!
//! A [`ForceModel`] gives the acceleration of a spacecraft at a time and state. Models add: a
//! pair `(A, B)` of models is itself a model whose acceleration is the sum. Written here: central-
//! body gravity and the J2 zonal harmonic. Not yet: higher zonal harmonics, third bodies,
//! radiation pressure, drag and thrust.

use crate::{Error, Seconds, StateVector, Vec3};

/// An acceleration (km/s²) on a spacecraft at `t` seconds past the epoch, for a state in km and
/// km/s.
pub trait ForceModel {
    /// The acceleration at time `t` for `state`.
    fn acceleration(&self, t: Seconds, state: &StateVector) -> Result<Vec3, Error>;
}

/// Point-mass gravity of a central body with gravitational parameter μ (km³/s²):
/// `a = −μ r / |r|³`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CentralBody {
    mu: f64,
}

impl CentralBody {
    /// Gravity of a body with parameter `mu`. Fails with [`Error::InvalidMu`] unless `mu` is
    /// positive and finite.
    pub fn new(mu: f64) -> Result<Self, Error> {
        if mu.is_finite() && mu > 0.0 {
            Ok(Self { mu })
        } else {
            Err(Error::InvalidMu)
        }
    }

    /// The gravitational parameter (km³/s²).
    pub fn mu(&self) -> f64 {
        self.mu
    }
}

impl ForceModel for CentralBody {
    fn acceleration(&self, _t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        let r = state.position.norm();
        if r.is_nan() || r <= 0.0 {
            return Err(Error::Degenerate);
        }
        Ok(state.position * (-self.mu / (r * r * r)))
    }
}

impl<A: ForceModel, B: ForceModel> ForceModel for (A, B) {
    fn acceleration(&self, t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        Ok(self.0.acceleration(t, state)? + self.1.acceleration(t, state)?)
    }
}

/// Equatorial radius of the Earth in km, from WGS 84 (NGA.STND.0036).
pub const WGS84_EQUATORIAL_RADIUS_KM: f64 = 6_378.137;
/// The Earth's second zonal harmonic coefficient J2, from WGS 84 (the value of EGM96, which WGS 84
/// adopts).
pub const WGS84_J2: f64 = 1.082_626_683_55e-3;

/// The perturbing acceleration of the J2 zonal harmonic of an oblate central body, in an inertial
/// frame whose z axis is the body's spin axis:
///
/// `a = −(3/2)·J2·μ·R²/r⁵ · (x(1 − 5z²/r²), y(1 − 5z²/r²), z(3 − 5z²/r²))`.
///
/// This is the perturbation alone. Add it to [`CentralBody`] with a tuple to get the full
/// gravity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct J2Zonal {
    mu: f64,
    j2: f64,
    radius: f64,
}

impl J2Zonal {
    /// A J2 perturbation for a body with parameter `mu` (km³/s²), coefficient `j2`, and reference
    /// radius `radius` (km). Fails with [`Error::InvalidMu`] unless `mu` is positive and finite,
    /// and with [`Error::NonFinite`] unless `j2` and `radius` are finite and `radius` is positive.
    pub fn new(mu: f64, j2: f64, radius: f64) -> Result<Self, Error> {
        if !(mu.is_finite() && mu > 0.0) {
            return Err(Error::InvalidMu);
        }
        if !j2.is_finite() {
            return Err(Error::NonFinite { what: "J2" });
        }
        if !radius.is_finite() || radius <= 0.0 {
            return Err(Error::NonFinite {
                what: "reference radius (km)",
            });
        }
        Ok(Self { mu, j2, radius })
    }

    /// The Earth's J2 perturbation with the WGS 84 constants, for parameter `mu` (km³/s²).
    pub fn earth(mu: f64) -> Result<Self, Error> {
        Self::new(mu, WGS84_J2, WGS84_EQUATORIAL_RADIUS_KM)
    }
}

impl ForceModel for J2Zonal {
    fn acceleration(&self, _t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        let p = state.position;
        let r = p.norm();
        if r.is_nan() || r <= 0.0 {
            return Err(Error::Degenerate);
        }
        let r2 = r * r;
        let z2_over_r2 = p.z * p.z / r2;
        let k = -1.5 * self.j2 * self.mu * self.radius * self.radius / (r2 * r2 * r);
        Ok(Vec3::new(
            k * p.x * (1.0 - 5.0 * z2_over_r2),
            k * p.y * (1.0 - 5.0 * z2_over_r2),
            k * p.z * (3.0 - 5.0 * z2_over_r2),
        ))
    }
}
