// SPDX-License-Identifier: MIT OR Apache-2.0
//! Positions of solar-system bodies: an [`Ephemeris`] trait and a low-precision analytic provider.
//!
//! Positions are geocentric Cartesian vectors in km, in the mean ecliptic and equinox of J2000.
//! The provider [`LowPrecision`] gives the Sun from the standard low-precision series (mean
//! longitude and anomaly, with the first two equation-of-centre terms). Its longitude is good to
//! about 0.01°, and its distance to about 10⁻⁴ AU, over a few centuries either side of 2000. It
//! uses TT in place of UT, which moves the result by under 0.001°.
//!
//! Not written: the Moon, and the other planets. The trait takes a [`Body`], so they can be added
//! without changing callers. A bundled ephemeris (SPK) is a separate item.

use crate::{Epoch, Error, Vec3};

/// One astronomical kilometre-scale unit: the astronomical unit in km (IAU 2012, exact).
const AU_KM: f64 = 149_597_870.7;

/// A body whose position can be queried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Body {
    /// The Sun.
    Sun,
}

/// A source of geocentric positions.
pub trait Ephemeris {
    /// The position of `body` relative to the Earth at `epoch`, in km, in the mean ecliptic and
    /// equinox of J2000.
    fn geocentric_position(&self, body: Body, epoch: Epoch) -> Result<Vec3, Error>;
}

/// The low-precision analytic ephemeris (see the module documentation).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LowPrecision;

impl Ephemeris for LowPrecision {
    fn geocentric_position(&self, body: Body, epoch: Epoch) -> Result<Vec3, Error> {
        match body {
            Body::Sun => Ok(sun_position(epoch)),
        }
    }
}

/// The geocentric position of the Sun, in km, in the ecliptic of J2000.
fn sun_position(epoch: Epoch) -> Vec3 {
    // Days from J2000, using TT for UT.
    let n = epoch.tt_julian_date() - 2_451_545.0;
    // Mean longitude and mean anomaly of the Sun (degrees).
    let mean_longitude = 280.460 + 0.985_647_4 * n;
    let mean_anomaly = (357.528 + 0.985_600_3 * n).to_radians();
    // Ecliptic longitude: the equation of centre, to second order.
    let longitude =
        (mean_longitude + 1.915 * mean_anomaly.sin() + 0.020 * (2.0 * mean_anomaly).sin())
            .to_radians();
    // Distance in AU.
    let distance = 1.000_14 - 0.016_71 * mean_anomaly.cos() - 0.000_14 * (2.0 * mean_anomaly).cos();
    let r = distance * AU_KM;
    Vec3::new(r * longitude.cos(), r * longitude.sin(), 0.0)
}
