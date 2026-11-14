// SPDX-License-Identifier: MIT OR Apache-2.0
//! Keplerian elements and state vectors for elliptic two-body orbits.
//!
//! Units: positions in km, velocities in km/s, gravitational parameter in km³/s². Angles are in
//! radians, returned in `[0, 2π)`. The reference plane is the x–y plane, and the orbit normal is
//! `h = r × v`.
//!
//! Singular cases use these conventions, so that every state has one element set:
//! - Circular orbits (`e` below `SINGULAR_EPS`) take `ω = 0`, and `ν` is the argument of latitude.
//! - Equatorial orbits (`sin i` below `SINGULAR_EPS`) take `Ω = 0`, and `ω` is the longitude of
//!   periapsis measured from the x axis. Retrograde orbits measure both angles in the sense of
//!   the orbit.
//! - Circular equatorial orbits take `ω = 0`, and `ν` is the true longitude.
//!
//! Not modelled: parabolic and hyperbolic orbits, and equinoctial elements.

use crate::{Error, Kilometres, Mat3, Radians, Vec3};
use core::f64::consts::TAU;

/// Below this, an eccentricity, or the sine of an inclination relative to the angular-momentum
/// scale, counts as zero: the orbit is treated as circular or equatorial.
const SINGULAR_EPS: f64 = 1e-12;

/// A position and velocity at one instant: position in km, velocity in km/s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateVector {
    /// Position (km).
    pub position: Vec3,
    /// Velocity (km/s).
    pub velocity: Vec3,
}

/// Keplerian elements of an elliptic orbit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keplerian {
    a: Kilometres,
    e: f64,
    i: Radians,
    raan: Radians,
    argp: Radians,
    nu: Radians,
}

fn check_mu(mu: f64) -> Result<(), Error> {
    if mu.is_finite() && mu > 0.0 {
        Ok(())
    } else {
        Err(Error::InvalidMu)
    }
}

fn check_finite(v: Vec3, what: &'static str) -> Result<(), Error> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(Error::NonFinite { what })
    }
}

/// The angle from `from` to `to`, measured about the unit vector `axis`, in `(-π, π]`.
fn angle_about(from: Vec3, to: Vec3, axis: Vec3) -> f64 {
    from.cross(to).dot(axis).atan2(from.dot(to))
}

fn wrap(angle: f64) -> f64 {
    angle.rem_euclid(TAU)
}

impl Keplerian {
    /// Elements of an elliptic orbit: semi-major axis `a > 0`, eccentricity `0 ≤ e < 1`, and the
    /// three angles. Fails with [`Error::NotElliptic`] otherwise.
    pub fn new(
        a: Kilometres,
        e: f64,
        inclination: Radians,
        raan: Radians,
        arg_periapsis: Radians,
        true_anomaly: Radians,
    ) -> Result<Self, Error> {
        if !e.is_finite() {
            return Err(Error::NonFinite {
                what: "eccentricity",
            });
        }
        if a.get() <= 0.0 || !(0.0..1.0).contains(&e) {
            return Err(Error::NotElliptic);
        }
        Ok(Self {
            a,
            e,
            i: inclination,
            raan,
            argp: arg_periapsis,
            nu: true_anomaly,
        })
    }

    /// The semi-major axis.
    pub fn semi_major_axis(&self) -> Kilometres {
        self.a
    }

    /// The eccentricity.
    pub fn eccentricity(&self) -> f64 {
        self.e
    }

    /// The inclination.
    pub fn inclination(&self) -> Radians {
        self.i
    }

    /// The right ascension of the ascending node.
    pub fn raan(&self) -> Radians {
        self.raan
    }

    /// The argument of periapsis (the longitude of periapsis for equatorial orbits).
    pub fn arg_periapsis(&self) -> Radians {
        self.argp
    }

    /// The true anomaly (the true longitude for circular equatorial orbits).
    pub fn true_anomaly(&self) -> Radians {
        self.nu
    }

    /// The elements of the orbit through `state`, under gravitational parameter `mu` (km³/s²).
    ///
    /// Fails with [`Error::Degenerate`] if the state has no angular momentum, and with
    /// [`Error::NotElliptic`] if its energy is not negative.
    pub fn from_state(state: &StateVector, mu: f64) -> Result<Self, Error> {
        check_mu(mu)?;
        check_finite(state.position, "position (km)")?;
        check_finite(state.velocity, "velocity (km/s)")?;
        let r_vec = state.position;
        let v_vec = state.velocity;
        let r = r_vec.norm();
        let h_vec = r_vec.cross(v_vec);
        let h = h_vec.norm();
        if !(r > 0.0 && h > 0.0) {
            return Err(Error::Degenerate);
        }
        let energy = 0.5 * v_vec.dot(v_vec) - mu / r;
        if energy.is_nan() || energy >= 0.0 {
            return Err(Error::NotElliptic);
        }
        let a = -mu / (2.0 * energy);
        // Eccentricity vector: e = (v × h)/μ − r̂.
        let e_vec = v_vec.cross(h_vec) * (1.0 / mu) - r_vec * (1.0 / r);
        let e = e_vec.norm();
        let h_hat = h_vec * (1.0 / h);
        // The node vector k × h, which points at the ascending node.
        let node = Vec3::new(-h_vec.y, h_vec.x, 0.0);
        let equatorial = node.norm() <= SINGULAR_EPS * h;
        let circular = e <= SINGULAR_EPS;

        let (raan, argp, nu) = if equatorial {
            let x_axis = Vec3::new(1.0, 0.0, 0.0);
            let argp = if circular {
                0.0
            } else {
                angle_about(x_axis, e_vec, h_hat)
            };
            let nu = if circular {
                angle_about(x_axis, r_vec, h_hat)
            } else {
                angle_about(e_vec, r_vec, h_hat)
            };
            (0.0, argp, nu)
        } else {
            let raan = node.y.atan2(node.x);
            let argp = if circular {
                0.0
            } else {
                angle_about(node, e_vec, h_hat)
            };
            let nu = if circular {
                angle_about(node, r_vec, h_hat)
            } else {
                angle_about(e_vec, r_vec, h_hat)
            };
            (raan, argp, nu)
        };
        let inclination = h_vec.x.hypot(h_vec.y).atan2(h_vec.z);
        Self::new(
            Kilometres::new(a)?,
            e,
            Radians::new(inclination)?,
            Radians::new(wrap(raan))?,
            Radians::new(wrap(argp))?,
            Radians::new(wrap(nu))?,
        )
    }

    /// The state at these elements, under gravitational parameter `mu` (km³/s²).
    pub fn to_state(&self, mu: f64) -> Result<StateVector, Error> {
        check_mu(mu)?;
        let a = self.a.get();
        let e = self.e;
        let (sin_nu, cos_nu) = self.nu.get().sin_cos();
        let p = a * (1.0 - e * e);
        let r = p / (1.0 + e * cos_nu);
        // Position and velocity in the perifocal frame: x towards periapsis, z along h.
        let position = Vec3::new(r * cos_nu, r * sin_nu, 0.0);
        let s = (mu / p).sqrt();
        let velocity = Vec3::new(-s * sin_nu, s * (e + cos_nu), 0.0);
        Ok(StateVector {
            position: self.rotate(position),
            velocity: self.rotate(velocity),
        })
    }

    /// The rotation from the perifocal frame (x towards periapsis, z along the orbit normal) to
    /// the reference frame: Rz(Ω) · Rx(i) · Rz(ω).
    pub fn perifocal_to_inertial(&self) -> Mat3 {
        let rotation_z = |theta: f64| {
            let (s, c) = theta.sin_cos();
            Mat3::from_rows([[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]])
        };
        let (si, ci) = self.i.get().sin_cos();
        let rotation_x = Mat3::from_rows([[1.0, 0.0, 0.0], [0.0, ci, -si], [0.0, si, ci]]);
        rotation_z(self.raan.get()) * rotation_x * rotation_z(self.argp.get())
    }

    /// Rotates a perifocal vector into the reference frame.
    fn rotate(&self, v: Vec3) -> Vec3 {
        self.perifocal_to_inertial().mul_vec(v)
    }
}
