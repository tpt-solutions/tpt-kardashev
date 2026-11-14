// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Moon's low-precision position, the frame rotation from ecliptic to equatorial, and the
//! third-body perturbation: its tidal limits, its vanishing at the Earth's centre, and a short
//! integration with the Sun included.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::ephemeris::{Body, Ephemeris, LowPrecision};
use tpt_kardashev_astrodynamics::force::{CentralBody, MOON_MU_KM3_S2, SUN_MU_KM3_S2, ThirdBody};
use tpt_kardashev_astrodynamics::frames::{OBLIQUITY_J2000_ARCSEC, ecliptic_to_equatorial};
use tpt_kardashev_astrodynamics::{Cowell, Epoch, Error, ForceModel, Seconds, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;

fn seconds(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

fn epoch_days(days: f64) -> Epoch {
    Epoch::from_tt_julian_date(2_451_545.0 + days).expect("finite")
}

fn ecliptic_longitude_latitude(v: Vec3) -> (f64, f64) {
    let lon = v.y.atan2(v.x).to_degrees().rem_euclid(360.0);
    let lat = (v.z / v.norm()).asin().to_degrees();
    (lon, lat)
}

#[test]
fn moon_matches_meeus_worked_example_within_the_series_accuracy() {
    // Meeus, Astronomical Algorithms, example 47.a: 1992 April 12, 0h TD (JD 2448724.5) has the
    // Moon at ecliptic longitude 133.162655°, latitude −3.229126°, distance 368 409.7 km. The
    // tolerances are set just above the measured errors of the truncated series (0.09°, 0.08°, 75 km).
    let p = LowPrecision
        .geocentric_position(
            Body::Moon,
            Epoch::from_tt_julian_date(2_448_724.5).expect("finite"),
        )
        .expect("moon");
    let (lon, lat) = ecliptic_longitude_latitude(p);
    let dist = p.norm();
    assert!(
        (lon - 133.162_655).abs() <= 0.15,
        "longitude {lon}°, expected 133.162655°"
    );
    assert!(
        (lat - (-3.229_126)).abs() <= 0.1,
        "latitude {lat}°, expected −3.229126°"
    );
    assert!(
        (dist - 368_409.7).abs() <= 300.0,
        "distance {dist} km, expected 368 409.7 km"
    );
}

#[test]
fn moon_distance_stays_within_its_orbital_range() {
    // Perigee is near 356 500 km and apogee near 406 700 km; sample 30 days.
    for day in 0..30 {
        let d = LowPrecision
            .geocentric_position(Body::Moon, epoch_days(f64::from(day)))
            .expect("moon")
            .norm();
        assert!((356_000.0..407_000.0).contains(&d), "day {day}: {d} km");
    }
}

#[test]
fn moon_latitude_stays_within_its_inclination_band() {
    // The Moon's orbit is inclined about 5.1° to the ecliptic, so its latitude never exceeds about
    // 5.3°.
    for day in 0..60 {
        let p = LowPrecision
            .geocentric_position(Body::Moon, epoch_days(f64::from(day)))
            .expect("moon");
        let (_, lat) = ecliptic_longitude_latitude(p);
        assert!(lat.abs() <= 5.3, "day {day}: latitude {lat}°");
    }
}

#[test]
fn ecliptic_pole_maps_to_the_equatorial_tilt() {
    // The ecliptic pole has declination 90° − ε and right ascension 270°.
    let eps = (OBLIQUITY_J2000_ARCSEC / 3600.0).to_radians();
    let pole = ecliptic_to_equatorial(Vec3::new(0.0, 0.0, 1.0));
    assert!((pole.x).abs() <= 1e-15);
    assert!((pole.y + eps.sin()).abs() <= 1e-12, "y = {}", pole.y);
    assert!((pole.z - eps.cos()).abs() <= 1e-12, "z = {}", pole.z);
    // A rotation preserves length.
    let v = Vec3::new(3.0, -4.0, 12.0);
    assert!((ecliptic_to_equatorial(v).norm() - v.norm()).abs() <= 1e-12 * v.norm());
}

#[test]
fn sun_tide_is_twice_the_radial_gradient_along_the_sun_line() {
    // On the line to the Sun, the third-body acceleration of a point at distance r from the Earth
    // is 2μ r / R³ outward, to leading order in r/R.
    let sun =
        ThirdBody::new(LowPrecision, Body::Sun, SUN_MU_KM3_S2, epoch_days(0.0)).expect("valid");
    let rb = sun.position(seconds(0.0)).expect("position");
    let big_r = rb.norm();
    let unit = rb * (1.0 / big_r);
    let r = 7_000.0;
    let state = StateVector {
        position: unit * r,
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    let a = sun.acceleration(seconds(0.0), &state).expect("accel");
    let radial = a.dot(unit);
    let expected = 2.0 * SUN_MU_KM3_S2 * r / (big_r * big_r * big_r);
    assert!(
        (radial - expected).abs() <= 0.01 * expected,
        "radial tide {radial}, expected {expected}"
    );
}

#[test]
fn sun_tide_is_minus_the_radial_gradient_across_the_sun_line() {
    // Perpendicular to the Sun line the tide pulls back towards the Earth's centre: −μ r / R³.
    let sun =
        ThirdBody::new(LowPrecision, Body::Sun, SUN_MU_KM3_S2, epoch_days(0.0)).expect("valid");
    let rb = sun.position(seconds(0.0)).expect("position");
    let big_r = rb.norm();
    let unit = rb * (1.0 / big_r);
    // Any vector perpendicular to the Sun line: the cross product with the z axis.
    let perp = Vec3::new(0.0, 0.0, 1.0).cross(unit);
    let perp = perp * (1.0 / perp.norm());
    let r = 7_000.0;
    let state = StateVector {
        position: perp * r,
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    let a = sun.acceleration(seconds(0.0), &state).expect("accel");
    let across = a.dot(perp);
    let expected = -SUN_MU_KM3_S2 * r / (big_r * big_r * big_r);
    assert!(
        (across - expected).abs() <= 0.01 * expected.abs(),
        "across-track tide {across}, expected {expected}"
    );
}

#[test]
fn the_third_body_vanishes_at_the_earths_centre() {
    // At r = 0 the direct and indirect terms cancel exactly.
    let moon =
        ThirdBody::new(LowPrecision, Body::Moon, MOON_MU_KM3_S2, epoch_days(0.0)).expect("valid");
    let origin = StateVector {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert_eq!(
        moon.acceleration(seconds(0.0), &origin).expect("accel"),
        Vec3::new(0.0, 0.0, 0.0)
    );
}

#[test]
fn sun_perturbation_changes_a_leo_orbit_only_slightly_over_a_day() {
    // The Sun's pull on a 7000 km orbit moves it by a few kilometres in a day: small, but not zero.
    let state = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    let central = CentralBody::new(MU).expect("valid");
    let sun =
        ThirdBody::new(LowPrecision, Body::Sun, SUN_MU_KM3_S2, epoch_days(0.0)).expect("valid");
    let day = seconds(86_400.0);
    let plain = Cowell::new(central, seconds(60.0))
        .expect("step")
        .propagate(&state, day)
        .expect("integrates");
    let perturbed = Cowell::new((central, sun), seconds(60.0))
        .expect("step")
        .propagate(&state, day)
        .expect("integrates");
    let shift = (perturbed.position - plain.position).norm() / plain.position.norm();
    assert!(
        (1e-7..1e-2).contains(&shift),
        "relative shift {shift} from the Sun over a day"
    );
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(
        ThirdBody::new(LowPrecision, Body::Moon, 0.0, epoch_days(0.0)).map(|_| ()),
        Err(Error::InvalidMu)
    );
}
