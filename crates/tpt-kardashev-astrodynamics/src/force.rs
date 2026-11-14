// SPDX-License-Identifier: MIT OR Apache-2.0
//! Accelerations for the numerical propagator.
//!
//! A [`ForceModel`] gives the acceleration of a spacecraft at a time and state. Models add: a
//! pair `(A, B)` of models is itself a model whose acceleration is the sum. Written here: central-
//! body gravity and the J2 zonal harmonic. Not yet: higher zonal harmonics, third bodies,
//! radiation pressure, drag and thrust.

use crate::atmosphere::density;
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

/// The perturbing acceleration of the zonal harmonics of degrees 2 to 4 of an oblate central body,
/// in the same inertial frame as [`J2Zonal`]. With the geopotential written as
/// `U = (μ/r)·(1 − Σₙ Jₙ (R/r)ⁿ Pₙ(z/r))`, and `s = z/r`, `q = R/r`, the perturbation is
///
/// `a = (μ/r³)·S₁·(x, y, z) + (μ/r⁴)·S₂·(z·x, z·y, −(x² + y²))`, where
/// `S₁ = Σₙ (n+1)·Jₙ·qⁿ·Pₙ(s)` and `S₂ = Σₙ Jₙ·qⁿ·Pₙ′(s)`.
///
/// The coefficients are supplied by the caller. The crate does not bundle J3 or J4: the values
/// recalled for EGM96 have not been checked against a primary source (see the provenance file).
/// With `j3 = j4 = 0` this reduces to [`J2Zonal`] exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZonalHarmonics {
    mu: f64,
    radius: f64,
    j2: f64,
    j3: f64,
    j4: f64,
}

impl ZonalHarmonics {
    /// Zonal harmonics for a body with parameter `mu` (km³/s²), reference radius `radius` (km), and
    /// coefficients `j2`, `j3`, `j4`. Validation is as for [`J2Zonal::new`].
    pub fn new(mu: f64, radius: f64, j2: f64, j3: f64, j4: f64) -> Result<Self, Error> {
        if !(mu.is_finite() && mu > 0.0) {
            return Err(Error::InvalidMu);
        }
        if !radius.is_finite() || radius <= 0.0 {
            return Err(Error::NonFinite {
                what: "reference radius (km)",
            });
        }
        for (what, value) in [("J2", j2), ("J3", j3), ("J4", j4)] {
            if !value.is_finite() {
                return Err(Error::NonFinite { what });
            }
        }
        Ok(Self {
            mu,
            radius,
            j2,
            j3,
            j4,
        })
    }
}

impl ForceModel for ZonalHarmonics {
    fn acceleration(&self, _t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        let p = state.position;
        let r = p.norm();
        if r.is_nan() || r <= 0.0 {
            return Err(Error::Degenerate);
        }
        let s = p.z / r;
        let q = self.radius / r;
        // Legendre polynomials of degrees 2 to 4 and their derivatives with respect to s.
        let p2 = 0.5 * (3.0 * s * s - 1.0);
        let dp2 = 3.0 * s;
        let p3 = 0.5 * (5.0 * s * s * s - 3.0 * s);
        let dp3 = 0.5 * (15.0 * s * s - 3.0);
        let p4 = (35.0 * s.powi(4) - 30.0 * s * s + 3.0) / 8.0;
        let dp4 = 0.5 * (35.0 * s * s * s - 15.0 * s);
        let s1 = 3.0 * self.j2 * q * q * p2
            + 4.0 * self.j3 * q.powi(3) * p3
            + 5.0 * self.j4 * q.powi(4) * p4;
        let s2 = self.j2 * q * q * dp2 + self.j3 * q.powi(3) * dp3 + self.j4 * q.powi(4) * dp4;
        let r3 = r * r * r;
        let a = self.mu / r3 * s1;
        let b = self.mu / (r3 * r) * s2;
        let rho2 = p.x * p.x + p.y * p.y;
        Ok(Vec3::new(
            a * p.x + b * p.z * p.x,
            a * p.y + b * p.z * p.y,
            a * p.z - b * rho2,
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

/// Rotation rate of the Earth, in rad/s (WGS 84).
pub const EARTH_ROTATION_RATE_RAD_S: f64 = 7.292_115e-5;

/// Atmospheric drag on a spacecraft, with the atmosphere co-rotating with the Earth:
///
/// `a = −½·C_D·(A/m)·ρ·|v_rel|·v_rel`, with `v_rel = v − ω × r`.
///
/// The density `ρ` comes from [`crate::atmosphere::density`] at the altitude above the WGS 84
/// equatorial radius. The state is inertial, with the spin axis as z, so `ω × r` is the atmosphere's
/// velocity exactly, whatever the Earth's orientation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphericDrag {
    drag_coefficient: f64,
    area_to_mass: f64,
}

impl AtmosphericDrag {
    /// Drag with coefficient `drag_coefficient` (C_D) and area-to-mass ratio `area_to_mass` (m²/kg).
    /// Fails with [`Error::Negative`] for a negative value and [`Error::NonFinite`] for a
    /// non-finite one.
    pub fn new(drag_coefficient: f64, area_to_mass: f64) -> Result<Self, Error> {
        for (value, what) in [
            (drag_coefficient, "drag coefficient"),
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
            drag_coefficient,
            area_to_mass,
        })
    }
}

impl ForceModel for AtmosphericDrag {
    fn acceleration(&self, _t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        let r = state.position;
        let r_norm = r.norm();
        if r_norm.is_nan() || r_norm <= 0.0 {
            return Err(Error::Degenerate);
        }
        let altitude = r_norm - WGS84_EQUATORIAL_RADIUS_KM;
        let rho = density(altitude);
        // The atmosphere's velocity: ω × r, with ω along +z.
        let omega = EARTH_ROTATION_RATE_RAD_S;
        let atmosphere = Vec3::new(-omega * r.y, omega * r.x, 0.0);
        let v_rel = state.velocity - atmosphere;
        let speed = v_rel.norm();
        // Density in kg/m³ and velocities in km/s: the factor 1e3 converts m/s² to km/s².
        let coefficient = -0.5 * self.drag_coefficient * self.area_to_mass * rho * speed * 1e3;
        Ok(v_rel * coefficient)
    }
}

/// The direction of a thrust acceleration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ThrustDirection {
    /// A fixed direction in the inertial frame (a unit vector is not required; it is normalised).
    Inertial(Vec3),
    /// Along the spacecraft's velocity, which is the prograde direction on a circular orbit.
    Prograde,
}

/// A finite burn: a constant thrust acceleration, in a fixed direction or along the velocity,
/// active from `start` to `end` seconds past the epoch.
///
/// The acceleration is `magnitude` (km/s²) in the chosen direction. The spacecraft's mass is not
/// tracked, so the acceleration stays constant through the burn: this is a model of an acceleration,
/// not of a rocket's mass flow. Outside the window the thrust is zero.
///
/// The thrust is discontinuous at the window edges, and a fixed-step integrator mis-samples a step
/// that straddles an edge. Use a step much shorter than the burn: for a 10 s burn, a 0.1 s step gives
/// the impulse to about 3×10⁻⁵, and a 0.5 s step to about 8×10⁻³.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thrust {
    direction: ThrustDirection,
    magnitude: f64,
    start: f64,
    end: f64,
}

impl Thrust {
    /// A burn with `magnitude` (km/s², at least zero) in `direction`, active from `start` to `end`
    /// seconds past the epoch. Fails with [`Error::Negative`] for a negative magnitude,
    /// [`Error::NonFinite`] for non-finite inputs or a zero inertial direction, and
    /// [`Error::InvalidWindow`] if `end` is before `start`.
    pub fn new(
        direction: ThrustDirection,
        magnitude: f64,
        start: Seconds,
        end: Seconds,
    ) -> Result<Self, Error> {
        if !magnitude.is_finite() {
            return Err(Error::NonFinite {
                what: "thrust magnitude (km/s²)",
            });
        }
        if magnitude < 0.0 {
            return Err(Error::Negative {
                what: "thrust magnitude",
            });
        }
        if let ThrustDirection::Inertial(d) = direction {
            let n = d.norm();
            if !(d.is_finite() && n.is_finite() && n > 0.0) {
                return Err(Error::NonFinite {
                    what: "thrust direction",
                });
            }
        }
        if end.get() < start.get() {
            return Err(Error::InvalidWindow);
        }
        Ok(Self {
            direction,
            magnitude,
            start: start.get(),
            end: end.get(),
        })
    }
}

impl ForceModel for Thrust {
    fn acceleration(&self, t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        let time = t.get();
        if time < self.start || time >= self.end {
            return Ok(Vec3::new(0.0, 0.0, 0.0));
        }
        let unit = match self.direction {
            ThrustDirection::Inertial(d) => d * (1.0 / d.norm()),
            ThrustDirection::Prograde => {
                let v = state.velocity;
                let n = v.norm();
                if n.is_nan() || n <= 0.0 {
                    return Err(Error::Degenerate);
                }
                v * (1.0 / n)
            }
        };
        Ok(unit * self.magnitude)
    }
}
