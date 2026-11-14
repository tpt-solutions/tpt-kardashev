// SPDX-License-Identifier: MIT OR Apache-2.0
//! Solar radiation pressure: its magnitude and direction, the cylindrical Earth shadow, linearity in
//! the coefficients, rejection of invalid input, and a day's drift on a LEO orbit.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::ephemeris::{AU_KM, Body, Ephemeris, LowPrecision};
use tpt_kardashev_astrodynamics::force::{
    CentralBody, SOLAR_PRESSURE_1AU_PA, SolarRadiationPressure, WGS84_EQUATORIAL_RADIUS_KM,
};
use tpt_kardashev_astrodynamics::frames::ecliptic_to_equatorial;
use tpt_kardashev_astrodynamics::{Cowell, Epoch, Error, ForceModel, Seconds, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
/// Tolerance on closed-form magnitudes (policy: 1e-12 relative).
const CLOSED_REL: f64 = 1e-12;

const CR: f64 = 1.5;
const A_OVER_M: f64 = 0.01;

fn seconds(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

fn epoch() -> Epoch {
    Epoch::from_tt_julian_date(2_451_545.0).expect("finite")
}

/// The Sun's unit direction from the Earth, in the equatorial frame, at J2000.
fn sun_unit() -> Vec3 {
    let s = ecliptic_to_equatorial(
        LowPrecision
            .geocentric_position(Body::Sun, epoch())
            .expect("sun"),
    );
    s * (1.0 / s.norm())
}

fn srp() -> SolarRadiationPressure<LowPrecision> {
    SolarRadiationPressure::new(LowPrecision, epoch(), CR, A_OVER_M).expect("valid")
}

fn at(position: Vec3) -> StateVector {
    StateVector {
        position,
        velocity: Vec3::new(0.0, 7.5, 0.0),
    }
}

/// A unit vector perpendicular to `u`.
fn perpendicular(u: Vec3) -> Vec3 {
    let candidate = if u.z.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let p = candidate.cross(u);
    p * (1.0 / p.norm())
}

#[test]
fn sunlit_pressure_pushes_directly_away_from_the_sun() {
    // On the Sun side of the Earth the force points away from the Sun, so it is anti-parallel to
    // the Sun's direction from the Earth.
    let u = sun_unit();
    let r = u * 7_000.0;
    let a = srp().acceleration(seconds(0.0), &at(r)).expect("accel");
    assert!(a.dot(u) < 0.0, "the push is away from the Sun");
    let parallel = a.cross(u).norm();
    assert!(
        parallel <= 1e-12 * a.norm(),
        "the push is along the Sun line"
    );
}

#[test]
fn pressure_magnitude_follows_the_law() {
    // |a| = ν·C_R·(A/m)·P·(AU/d)²·1e-3 km/s², with d the distance from the Sun.
    let u = sun_unit();
    let r = u * 7_000.0;
    let sun_distance = (ecliptic_to_equatorial(
        LowPrecision
            .geocentric_position(Body::Sun, epoch())
            .expect("sun"),
    ) - r)
        .norm();
    let expected = CR * A_OVER_M * SOLAR_PRESSURE_1AU_PA * (AU_KM / sun_distance).powi(2) * 1e-3;
    let a = srp().acceleration(seconds(0.0), &at(r)).expect("accel");
    assert!(
        (a.norm() - expected).abs() <= CLOSED_REL * expected,
        "magnitude {}, expected {expected}",
        a.norm()
    );
}

#[test]
fn the_force_is_linear_in_reflectivity_and_area_to_mass() {
    let u = sun_unit();
    let r = u * 7_000.0;
    let base = srp().acceleration(seconds(0.0), &at(r)).expect("accel");
    let doubled = SolarRadiationPressure::new(LowPrecision, epoch(), 2.0 * CR, A_OVER_M)
        .expect("valid")
        .acceleration(seconds(0.0), &at(r))
        .expect("accel");
    assert!((doubled - base * 2.0).norm() <= CLOSED_REL * base.norm());
}

#[test]
fn the_earths_night_side_shadow_is_a_cylinder() {
    let u = sun_unit();
    let p = perpendicular(u);
    let radius = WGS84_EQUATORIAL_RADIUS_KM;
    // Directly behind the Earth, on the axis: in shadow, no force.
    let on_axis = u * -7_000.0;
    assert_eq!(
        srp()
            .acceleration(seconds(0.0), &at(on_axis))
            .expect("accel"),
        Vec3::new(0.0, 0.0, 0.0)
    );
    // Behind the Earth, just inside one Earth radius of the axis: in shadow.
    let inside = u * -10_000.0 + p * (radius - 1.0);
    assert_eq!(
        srp()
            .acceleration(seconds(0.0), &at(inside))
            .expect("accel"),
        Vec3::new(0.0, 0.0, 0.0)
    );
    // Just outside: sunlit.
    let outside = u * -10_000.0 + p * (radius + 1.0);
    assert!(
        srp()
            .acceleration(seconds(0.0), &at(outside))
            .expect("accel")
            .norm()
            > 0.0
    );
    // On the Sun side the shadow never applies, however close to the axis.
    let sunward = u * 10_000.0;
    assert!(!SolarRadiationPressure::<LowPrecision>::in_shadow(
        sunward,
        u * AU_KM
    ));
}

#[test]
fn invalid_coefficients_are_rejected() {
    assert_eq!(
        SolarRadiationPressure::new(LowPrecision, epoch(), -1.0, A_OVER_M).map(|_| ()),
        Err(Error::Negative {
            what: "reflectivity"
        })
    );
    assert_eq!(
        SolarRadiationPressure::new(LowPrecision, epoch(), CR, -0.1).map(|_| ()),
        Err(Error::Negative {
            what: "area-to-mass ratio"
        })
    );
    assert!(matches!(
        SolarRadiationPressure::new(LowPrecision, epoch(), f64::NAN, A_OVER_M).map(|_| ()),
        Err(Error::NonFinite { .. })
    ));
}

#[test]
fn a_day_of_pressure_moves_a_leo_orbit_only_slightly() {
    // The pressure at this area-to-mass ratio is about 7e-11 km/s². Over a day that moves a LEO
    // orbit by about a quarter of a kilometre (½·a·t²): small, but not zero.
    let start = at(Vec3::new(7_000.0, 0.0, 0.0));
    let central = CentralBody::new(MU).expect("valid");
    let plain = Cowell::new(central, seconds(60.0))
        .expect("step")
        .propagate(&start, seconds(86_400.0))
        .expect("integrates");
    let pushed = Cowell::new((central, srp()), seconds(60.0))
        .expect("step")
        .propagate(&start, seconds(86_400.0))
        .expect("integrates");
    let shift = (pushed.position - plain.position).norm() / plain.position.norm();
    assert!((1e-7..1e-2).contains(&shift), "relative shift {shift}");
}
