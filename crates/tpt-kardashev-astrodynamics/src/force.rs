// SPDX-License-Identifier: MIT OR Apache-2.0
//! Accelerations for the numerical propagator.
//!
//! A [`ForceModel`] gives the acceleration of a spacecraft at a time and state. Models add: a
//! pair `(A, B)` of models is itself a model whose acceleration is the sum. Written here: central-
//! body gravity and the J2 zonal harmonic. Not yet: higher zonal harmonics, third bodies,
//! radiation pressure, drag and thrust.

use crate::ephemeris::{AU_KM, Body, Ephemeris};
use crate::frames::ecliptic_to_equatorial;
use crate::{Epoch, Error, Seconds, StateVector, Vec3};

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

/// Gravitational parameter of the Sun, in km³/s² (DE430 value, as recalled; see the provenance
/// file).
pub const SUN_MU_KM3_S2: f64 = 1.327_124_400_41e11;
/// Gravitational parameter of the Moon, in km³/s² (DE430 value, as recalled; see the provenance
/// file).
pub const MOON_MU_KM3_S2: f64 = 4_902.800_066;

/// The third-body perturbation of a body at a position given by an ephemeris, on a spacecraft
/// orbiting the Earth:
///
/// `a = μ_b·((r_b − r)/|r_b − r|³ − r_b/|r_b|³)`,
///
/// where `r` is the spacecraft's geocentric position and `r_b` the body's. The ephemeris gives
/// positions in the ecliptic, and this force rotates them into the equatorial frame that the
/// spacecraft state uses. Time `t` is seconds past `epoch`.
#[derive(Clone, Copy, Debug)]
pub struct ThirdBody<E> {
    ephemeris: E,
    body: Body,
    mu: f64,
    epoch: Epoch,
}

impl<E: Ephemeris> ThirdBody<E> {
    /// A perturbation by `body` with parameter `mu` (km³/s²), with `t = 0` at `epoch`. Fails with
    /// [`Error::InvalidMu`] unless `mu` is positive and finite.
    pub fn new(ephemeris: E, body: Body, mu: f64, epoch: Epoch) -> Result<Self, Error> {
        if mu.is_finite() && mu > 0.0 {
            Ok(Self {
                ephemeris,
                body,
                mu,
                epoch,
            })
        } else {
            Err(Error::InvalidMu)
        }
    }

    /// The body's position relative to the Earth at time `t`, in the equatorial frame (km).
    pub fn position(&self, t: Seconds) -> Result<Vec3, Error> {
        let at = Epoch::from_tt_seconds(self.epoch.tt_seconds() + t.get())?;
        let ecliptic = self.ephemeris.geocentric_position(self.body, at)?;
        Ok(ecliptic_to_equatorial(ecliptic))
    }
}

impl<E: Ephemeris> ForceModel for ThirdBody<E> {
    fn acceleration(&self, t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        let rb = self.position(t)?;
        let d = rb - state.position;
        let d_norm = d.norm();
        let rb_norm = rb.norm();
        if d_norm.is_nan() || d_norm <= 0.0 || rb_norm.is_nan() || rb_norm <= 0.0 {
            return Err(Error::Degenerate);
        }
        let direct = d * (1.0 / (d_norm * d_norm * d_norm));
        let indirect = rb * (1.0 / (rb_norm * rb_norm * rb_norm));
        Ok((direct - indirect) * self.mu)
    }
}

/// Solar radiation pressure at one astronomical unit, in N/m² (as recalled; see the provenance
/// file).
pub const SOLAR_PRESSURE_1AU_PA: f64 = 4.56e-6;

/// Solar radiation pressure on a spacecraft, with a cylindrical Earth shadow.
///
/// The acceleration is `ν·C_R·(A/m)·P·(AU/d)²·(r − r_sun)/d`, where `r` is the spacecraft's
/// geocentric position, `r_sun` the Sun's, `d = |r − r_sun|`, `C_R` the reflectivity coefficient,
/// `A/m` the area-to-mass ratio in m²/kg, and `P` the pressure at one AU. The shadow factor `ν` is
/// 0 when the spacecraft is on the night side and within one Earth radius of the Sun–Earth axis,
/// and 1 otherwise.
///
/// Not modelled: the penumbra (the shadow is sharp), the Earth's albedo and infrared, and the
/// spacecraft's attitude and surface optics beyond `C_R`.
#[derive(Clone, Copy, Debug)]
pub struct SolarRadiationPressure<E> {
    ephemeris: E,
    epoch: Epoch,
    reflectivity: f64,
    area_to_mass: f64,
}

impl<E: Ephemeris> SolarRadiationPressure<E> {
    /// Pressure on a spacecraft with reflectivity `reflectivity` (C_R) and area-to-mass ratio
    /// `area_to_mass` (m²/kg), with `t = 0` at `epoch`. Fails with [`Error::Negative`] for a
    /// negative value, and [`Error::NonFinite`] for a non-finite one.
    pub fn new(
        ephemeris: E,
        epoch: Epoch,
        reflectivity: f64,
        area_to_mass: f64,
    ) -> Result<Self, Error> {
        for (value, what) in [
            (reflectivity, "reflectivity"),
            (area_to_mass, "area-to-mass ratio"),
        ] {
            if !value.is_finite() {
                return Err(Error::NonFinite { what });
            }
            if value < 0.0 {
                return Err(Error::Negative { what });
            }
        }
        Ok(Self {
            ephemeris,
            epoch,
            reflectivity,
            area_to_mass,
        })
    }

    /// Whether a geocentric position is in the Earth's cylindrical shadow at the Sun position
    /// `sun` (km, equatorial).
    pub fn in_shadow(position: Vec3, sun: Vec3) -> bool {
        let sun_norm = sun.norm();
        let along = position.dot(sun) / sun_norm;
        if along >= 0.0 {
            return false;
        }
        let axis = sun * (1.0 / sun_norm);
        let across = position - axis * along;
        across.norm() < WGS84_EQUATORIAL_RADIUS_KM
    }
}

impl<E: Ephemeris> ForceModel for SolarRadiationPressure<E> {
    fn acceleration(&self, t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        let at = Epoch::from_tt_seconds(self.epoch.tt_seconds() + t.get())?;
        let sun = ecliptic_to_equatorial(self.ephemeris.geocentric_position(Body::Sun, at)?);
        if Self::in_shadow(state.position, sun) {
            return Ok(Vec3::new(0.0, 0.0, 0.0));
        }
        let from_sun = state.position - sun;
        let d = from_sun.norm();
        if d.is_nan() || d <= 0.0 {
            return Err(Error::Degenerate);
        }
        let au_over_d = AU_KM / d;
        // Pressure in N/m² gives m/s² per m²/kg; convert to km/s² with 1e-3.
        let magnitude = self.reflectivity
            * self.area_to_mass
            * SOLAR_PRESSURE_1AU_PA
            * au_over_d
            * au_over_d
            * 1e-3;
        Ok(from_sun * (magnitude / d))
    }
}
