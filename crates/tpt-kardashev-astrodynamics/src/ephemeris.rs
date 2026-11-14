// SPDX-License-Identifier: MIT OR Apache-2.0
//! Positions of solar-system bodies: an [`Ephemeris`] trait and a low-precision analytic provider.
//!
//! Positions are geocentric Cartesian vectors in km, in the mean ecliptic and equinox of J2000.
//! The provider [`LowPrecision`] gives:
//! - the Sun, from the standard low-precision series (mean longitude and anomaly, with the first
//!   two equation-of-centre terms). Its longitude is good to about 0.01°, and its distance to about
//!   10⁻⁴ AU, over a few centuries either side of 2000;
//! - the Moon, from the leading terms of the standard low-precision lunar series (six longitude
//!   terms, four latitude terms, four distance terms). Its accuracy is a few tenths of a degree in
//!   longitude, and a few thousand km in distance.
//!
//! Both use TT in place of UT, which moves the result by under 0.001°.
//!
//! Not written: the planets. The trait takes a [`Body`], so they can be added without changing
//! callers. A bundled ephemeris (SPK) is a separate item.

use crate::{Epoch, Error, Vec3};

/// One astronomical unit in km (IAU 2012, exact).
pub const AU_KM: f64 = 149_597_870.7;

/// A body whose position can be queried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Body {
    /// The Sun.
    Sun,
    /// The Moon.
    Moon,
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
            Body::Moon => Ok(moon_position(epoch)),
        }
    }
}

/// Days from J2000, using TT for UT.
fn days_from_j2000(epoch: Epoch) -> f64 {
    epoch.tt_julian_date() - 2_451_545.0
}

/// The geocentric position of the Sun, in km, in the ecliptic of J2000.
fn sun_position(epoch: Epoch) -> Vec3 {
    let n = days_from_j2000(epoch);
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

/// The geocentric position of the Moon, in km, in the ecliptic of J2000.
///
/// Mean longitude `L′`, mean anomaly `M′`, mean elongation `D`, argument of latitude `F` (all in
/// degrees, linear in days), and the Sun's mean anomaly `M`. The longitude, latitude and distance
/// are the leading periodic terms of the standard series.
fn moon_position(epoch: Epoch) -> Vec3 {
    let n = days_from_j2000(epoch);
    let l_prime = 218.316 + 13.176_396 * n;
    let m_moon = (134.963 + 13.064_993 * n).to_radians();
    let f = (93.272 + 13.229_350 * n).to_radians();
    let d = (297.850 + 12.190_749 * n).to_radians();
    let m_sun = (357.529 + 0.985_600_3 * n).to_radians();

    let longitude = (l_prime
        + 6.288_774 * m_moon.sin()
        + 1.274_027 * (2.0 * d - m_moon).sin()
        + 0.658_314 * (2.0 * d).sin()
        + 0.213_618 * (2.0 * m_moon).sin()
        - 0.185_116 * m_sun.sin()
        - 0.114_332 * (2.0 * f).sin())
    .to_radians();
    let latitude = (5.128_122 * f.sin()
        + 0.280_602 * (m_moon + f).sin()
        + 0.277_693 * (m_moon - f).sin()
        + 0.173_237 * (2.0 * d - f).sin())
    .to_radians();
    let distance = 385_000.56
        - 20_905.355 * m_moon.cos()
        - 3_699.111 * (2.0 * d - m_moon).cos()
        - 2_955.968 * (2.0 * d).cos()
        - 569.925 * (2.0 * m_moon).cos();

    let cos_lat = latitude.cos();
    Vec3::new(
        distance * cos_lat * longitude.cos(),
        distance * cos_lat * longitude.sin(),
        distance * latitude.sin(),
    )
}
