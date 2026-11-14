// SPDX-License-Identifier: MIT OR Apache-2.0
//! The exponential atmosphere and atmospheric drag: table values, monotone decrease, zero drag for an
//! atmosphere-matching velocity, the magnitude and direction against the rotating atmosphere, the
//! decay of a circular orbit over a day, and rejection of invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::atmosphere::{TOP_KM, density};
use tpt_kardashev_astrodynamics::force::{
    AtmosphericDrag, CentralBody, EARTH_ROTATION_RATE_RAD_S, WGS84_EQUATORIAL_RADIUS_KM,
};
use tpt_kardashev_astrodynamics::{
    Cowell, Error, ForceModel, Keplerian, Seconds, StateVector, Vec3,
};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
/// Tolerance on table lookups and closed forms (policy: 1e-12 relative).
const CLOSED_REL: f64 = 1e-12;

const CD: f64 = 2.2;
const A_OVER_M: f64 = 0.01;

fn seconds(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

/// The velocity of the atmosphere at `r`: ω × r, with ω along +z.
fn atmosphere_velocity(r: Vec3) -> Vec3 {
    let w = EARTH_ROTATION_RATE_RAD_S;
    Vec3::new(-w * r.y, w * r.x, 0.0)
}

#[test]
fn table_rows_are_reproduced_exactly() {
    assert_eq!(density(0.0), 1.225);
    assert!((density(400.0) - 3.725e-12).abs() <= CLOSED_REL * 3.725e-12);
    assert!((density(1000.0) - 3.019e-15).abs() <= 1e-3 * 3.019e-15 + 1e-30);
}

#[test]
fn density_decreases_with_altitude() {
    let mut previous = density(0.0);
    for h in 1..=(TOP_KM as i64) {
        let d = density(h as f64);
        assert!(d < previous, "density rises at {h} km");
        previous = d;
    }
}

#[test]
fn density_follows_the_row_scale_height_between_rows() {
    // Between 400 km and 450 km the row is (400, 3.725e-12, 58.515): 10 km above it, the density is
    // the row value times exp(−10/58.515).
    let expected = 3.725e-12 * (-10.0_f64 / 58.515).exp();
    assert!((density(410.0) - expected).abs() <= CLOSED_REL * expected);
}

#[test]
fn density_is_zero_above_the_table() {
    assert_eq!(density(TOP_KM + 1.0), 0.0);
}

#[test]
fn an_atmosphere_matching_velocity_feels_no_drag() {
    // A body moving with the rotating atmosphere has zero relative velocity, so no drag.
    let r = Vec3::new(WGS84_EQUATORIAL_RADIUS_KM + 400.0, 0.0, 0.0);
    let state = StateVector {
        position: r,
        velocity: atmosphere_velocity(r),
    };
    let drag = AtmosphericDrag::new(CD, A_OVER_M).expect("valid");
    assert_eq!(
        drag.acceleration(seconds(0.0), &state).expect("accel"),
        Vec3::new(0.0, 0.0, 0.0)
    );
}

#[test]
fn drag_opposes_the_relative_velocity_with_the_expected_magnitude() {
    // At 400 km, an inertial speed of 7.67 km/s. The atmosphere moves at ω·r ≈ 0.49 km/s along y,
    // so the relative speed is smaller than the inertial one: the correction matters.
    let r = Vec3::new(WGS84_EQUATORIAL_RADIUS_KM + 400.0, 0.0, 0.0);
    let v = Vec3::new(0.0, 7.67, 0.0);
    let state = StateVector {
        position: r,
        velocity: v,
    };
    let v_rel = v - atmosphere_velocity(r);
    let speed_km = v_rel.norm();
    let drag = AtmosphericDrag::new(CD, A_OVER_M).expect("valid");
    let a = drag.acceleration(seconds(0.0), &state).expect("accel");

    // Opposes the relative velocity, with no sideways part.
    assert!(a.dot(v_rel) < 0.0);
    assert!(a.cross(v_rel).norm() <= 1e-12 * a.norm() * v_rel.norm());

    // |a| = ½·C_D·(A/m)·ρ·|v_rel|², in m/s², converted to km/s².
    let rho = density(400.0);
    let expected = 0.5 * CD * A_OVER_M * rho * (speed_km * 1e3).powi(2) * 1e-3;
    assert!(
        (a.norm() - expected).abs() <= CLOSED_REL * expected,
        "magnitude {}, expected {expected}",
        a.norm()
    );
}

#[test]
fn a_circular_orbit_at_400_km_decays_at_the_textbook_rate() {
    // For a circular orbit, da/dt ≈ −C_D·(A/m)·ρ·√(μa). At 400 km that is about −0.36 km per day.
    let r = WGS84_EQUATORIAL_RADIUS_KM + 400.0;
    let v = (MU / r).sqrt();
    let start = StateVector {
        position: Vec3::new(r, 0.0, 0.0),
        velocity: Vec3::new(0.0, v, 0.0),
    };
    let central = CentralBody::new(MU).expect("valid");
    let drag = AtmosphericDrag::new(CD, A_OVER_M).expect("valid");
    let end = Cowell::new((central, drag), seconds(60.0))
        .expect("step")
        .propagate(&start, seconds(86_400.0))
        .expect("integrates");
    let a0 = Keplerian::from_state(&start, MU).expect("elliptic");
    let a1 = Keplerian::from_state(&end, MU).expect("elliptic");
    let decay = a1.semi_major_axis().get() - a0.semi_major_axis().get();
    assert!(
        (-0.5..-0.25).contains(&decay),
        "semi-major axis changed by {decay} km in a day"
    );
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(
        AtmosphericDrag::new(-1.0, A_OVER_M).map(|_| ()),
        Err(Error::Negative {
            what: "drag coefficient"
        })
    );
    assert!(matches!(
        AtmosphericDrag::new(CD, f64::NAN).map(|_| ()),
        Err(Error::NonFinite { .. })
    ));
    let drag = AtmosphericDrag::new(CD, A_OVER_M).expect("valid");
    let origin = StateVector {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert_eq!(
        drag.acceleration(seconds(0.0), &origin),
        Err(Error::Degenerate)
    );
}
