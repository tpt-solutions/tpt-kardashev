// SPDX-License-Identifier: MIT OR Apache-2.0
//! Equinoctial elements `(p, f, g, h, k, L)`: a non-singular element set for elliptic orbits.
//!
//! `p` is the semi-latus rectum `a(1 − e²)`, `f = e·cos(ω + Ω)`, `g = e·sin(ω + Ω)`,
//! `h = tan(i/2)·cos Ω`, `k = tan(i/2)·sin Ω`, and `L = Ω + ω + ν` is the true longitude. Unlike
//! the Keplerian set, no element is undefined for a circular or an equatorial orbit: the
//! singular cases of the Keplerian set are regular here, and the conversions go through
//! [`Keplerian`], so the conventions for those cases are the ones documented in the crate README.
//!
//! Standard relations (Walker, Ocampo and Russell, 1996), written here from those definitions.

use core::f64::consts::TAU;

use crate::elements::{Keplerian, StateVector};
use crate::{Error, Kilometres, Radians};

/// The equinoctial elements of an elliptic orbit. Lengths in km, angles in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Equinoctial {
    p: f64,
    f: f64,
    g: f64,
    h: f64,
    k: f64,
    l: f64,
}

impl Equinoctial {
    /// Equinoctial elements from their six values, `p` in km and `L` in radians. Fails with
    /// [`Error::NonFinite`] for a non-finite value. Whether they describe an elliptic orbit is
    /// checked by [`Equinoctial::to_keplerian`].
    pub fn new(p: f64, f: f64, g: f64, h: f64, k: f64, l: f64) -> Result<Self, Error> {
        if [p, f, g, h, k, l].iter().all(|v| v.is_finite()) {
            Ok(Self { p, f, g, h, k, l })
        } else {
            Err(Error::NonFinite {
                what: "equinoctial element",
            })
        }
    }

    /// The semi-latus rectum `p` (km).
    pub fn semi_latus_rectum(&self) -> f64 {
        self.p
    }

    /// `(f, g)`: the eccentricity vector, rotated by `ω + Ω`.
    pub fn eccentricity_components(&self) -> (f64, f64) {
        (self.f, self.g)
    }

    /// `(h, k)`: the inclination vector, `tan(i/2)` along the node direction.
    pub fn inclination_components(&self) -> (f64, f64) {
        (self.h, self.k)
    }

    /// The true longitude `L = Ω + ω + ν` (radians, in `[0, 2π)`).
    pub fn true_longitude(&self) -> f64 {
        self.l
    }

    /// The equinoctial elements of a Keplerian set. Fails with [`Error::NonFinite`] for non-finite
    /// input, which a valid [`Keplerian`] never has.
    pub fn from_keplerian(el: &Keplerian) -> Result<Self, Error> {
        let a = el.semi_major_axis().get();
        let e = el.eccentricity();
        let i = el.inclination().get();
        let raan = el.raan().get();
        let argp = el.arg_periapsis().get();
        let nu = el.true_anomaly().get();
        let w = raan + argp;
        let t = (0.5 * i).tan();
        let out = Self {
            p: a * (1.0 - e * e),
            f: e * w.cos(),
            g: e * w.sin(),
            h: t * raan.cos(),
            k: t * raan.sin(),
            l: (w + nu).rem_euclid(TAU),
        };
        if [out.p, out.f, out.g, out.h, out.k, out.l]
            .iter()
            .all(|v| v.is_finite())
        {
            Ok(out)
        } else {
            Err(Error::NonFinite {
                what: "equinoctial element",
            })
        }
    }

    /// The Keplerian set of these elements, with the conventions of the crate README for circular
    /// and equatorial orbits. Fails with [`Error::NotElliptic`] unless `f² + g² < 1`, and with
    /// [`Error::Negative`] unless `p` is positive.
    pub fn to_keplerian(&self) -> Result<Keplerian, Error> {
        if self.p.is_nan() || self.p <= 0.0 {
            return Err(Error::Negative {
                what: "semi-latus rectum",
            });
        }
        let e2 = self.f * self.f + self.g * self.g;
        if e2.is_nan() || e2 >= 1.0 {
            return Err(Error::NotElliptic);
        }
        let e = e2.sqrt();
        let a = self.p / (1.0 - e2);
        let tan_half = (self.h * self.h + self.k * self.k).sqrt();
        let i = 2.0 * tan_half.atan();
        // Equatorial orbits take Ω = 0, so ω is the longitude of periapsis.
        let raan = if tan_half == 0.0 {
            0.0
        } else {
            self.k.atan2(self.h).rem_euclid(TAU)
        };
        // Circular orbits take ω = 0, so ν is the argument of latitude.
        let (argp, nu) = if e == 0.0 {
            (0.0, (self.l - raan).rem_euclid(TAU))
        } else {
            let w_plus_raan = self.g.atan2(self.f);
            let argp = (w_plus_raan - raan).rem_euclid(TAU);
            (argp, (self.l - w_plus_raan).rem_euclid(TAU))
        };
        Keplerian::new(
            Kilometres::new(a)?,
            e,
            Radians::new(i)?,
            Radians::new(raan)?,
            Radians::new(argp)?,
            Radians::new(nu)?,
        )
    }

    /// The equinoctial elements of a state vector with gravitational parameter `mu` (km³/s²).
    pub fn from_state(state: &StateVector, mu: f64) -> Result<Self, Error> {
        Self::from_keplerian(&Keplerian::from_state(state, mu)?)
    }

    /// The state vector of these elements with gravitational parameter `mu` (km³/s²).
    pub fn to_state(&self, mu: f64) -> Result<StateVector, Error> {
        self.to_keplerian()?.to_state(mu)
    }
}
