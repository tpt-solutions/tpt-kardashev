// SPDX-License-Identifier: MIT OR Apache-2.0
//! The low-precision Sun: known values at J2000, the annual cycle of distance and longitude, and
//! the geometry of the output vector.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::ephemeris::{Body, Ephemeris, LowPrecision};
use tpt_kardashev_astrodynamics::{Epoch, Vec3};

/// Astronomical unit in km (IAU 2012, exact), as in the module.
const AU_KM: f64 = 149_597_870.7;

/// Longitude tolerance in degrees: the series' stated accuracy.
const LONGITUDE_DEG: f64 = 0.01;

/// The Julian date of the given number of days after J2000 (TT).
fn epoch_days(days: f64) -> Epoch {
    Epoch::from_tt_julian_date(2_451_545.0 + days).expect("finite")
}

fn sun(days: f64) -> Vec3 {
    LowPrecision
        .geocentric_position(Body::Sun, epoch_days(days))
        .expect("sun")
}

/// Ecliptic longitude of a vector in the x–y plane, in degrees in `[0, 360)`.
fn longitude_deg(v: Vec3) -> f64 {
    v.y.atan2(v.x).to_degrees().rem_euclid(360.0)
}

#[test]
fn sun_at_j2000_matches_the_published_low_precision_values() {
    // At 2000-01-01 12:00 the Sun is at ecliptic longitude about 280.37° and 0.98333 AU from the
    // Earth (close to perihelion on 3 January).
    let p = sun(0.0);
    assert!(
        (longitude_deg(p) - 280.37).abs() <= LONGITUDE_DEG,
        "longitude {}",
        longitude_deg(p)
    );
    let distance_au = p.norm() / AU_KM;
    assert!(
        (distance_au - 0.98333).abs() <= 1e-4,
        "distance {distance_au} AU"
    );
}

#[test]
fn the_sun_lies_in_the_ecliptic_plane() {
    for days in [-3_000.0, 0.0, 1_234.5, 9_000.0] {
        assert_eq!(sun(days).z, 0.0);
    }
}

#[test]
fn distance_stays_within_the_perihelion_and_aphelion_bounds() {
    // Perihelion is about 0.9833 AU and aphelion about 1.0167 AU. Sample a year day by day.
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for day in 0..366 {
        let d = sun(f64::from(day)).norm() / AU_KM;
        lo = lo.min(d);
        hi = hi.max(d);
    }
    assert!((lo - 0.9833).abs() <= 2e-4, "perihelion {lo} AU");
    assert!((hi - 1.0167).abs() <= 2e-4, "aphelion {hi} AU");
}

#[test]
fn the_sun_makes_one_turn_per_tropical_year() {
    // After one tropical year (365.2422 days) the longitude has returned to its start, to within
    // the series' accuracy. The Sun's mean rate is 0.9856° per day.
    let start = longitude_deg(sun(0.0));
    let after = longitude_deg(sun(365.2422));
    let diff = (after - start).rem_euclid(360.0);
    let diff = diff.min(360.0 - diff);
    assert!(
        diff <= LONGITUDE_DEG,
        "the year closed with an error of {diff}°"
    );
}

#[test]
fn the_sun_moves_eastward_at_about_one_degree_a_day() {
    let daily = longitude_deg(sun(1.0)) - longitude_deg(sun(0.0));
    let daily = daily.rem_euclid(360.0);
    assert!((0.95..1.02).contains(&daily), "daily motion {daily}°");
}

#[test]
fn the_output_is_a_position_in_km() {
    // The norm of the vector is the distance in km, so it lies between perihelion and aphelion
    // distances for any day in the year.
    let p = sun(100.0);
    let norm_au = p.norm() / AU_KM;
    assert!((0.98..1.02).contains(&norm_au));
}
