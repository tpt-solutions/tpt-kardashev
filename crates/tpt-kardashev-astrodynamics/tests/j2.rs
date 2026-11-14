// SPDX-License-Identifier: MIT OR Apache-2.0
//! The J2 zonal perturbation: closed-form accelerations, the secular nodal regression of a
//! numerically integrated orbit against the standard secular formula, and rejection of invalid
//! input. Constants are the WGS 84 values in `force`.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::TAU;
use tpt_kardashev_astrodynamics::force::{WGS84_EQUATORIAL_RADIUS_KM, WGS84_J2};
use tpt_kardashev_astrodynamics::{
    CentralBody, Cowell, Error, ForceModel, J2Zonal, Keplerian, Kilometres, Radians, Seconds,
    StateVector, Vec3,
};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
const R: f64 = WGS84_EQUATORIAL_RADIUS_KM;

/// Relative tolerance on closed-form accelerations (policy: 1e-12).
const CLOSED_REL: f64 = 1e-12;
/// Relative tolerance on the nodal regression over ten orbits (0.05 rad here). The secular rate is
/// an orbit average, so the short-period terms it omits add error of order J2·(R/a)², about 1e-3
/// rad, i.e. about 2% of this arc. The measured ratio is 1.0022, a relative error of 0.2%; the
/// 1% bound is set just above that.
const REGRESSION_REL: f64 = 0.01;

fn seconds(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

fn assert_close(actual: f64, expected: f64, rel: f64) {
    let bound = rel * expected.abs();
    assert!(
        (actual - expected).abs() <= bound,
        "actual {actual}, expected {expected}, bound {bound}"
    );
}

/// A near-circular inclined orbit at 7000 km, with the node at 1.0 rad.
fn inclined_orbit(a: f64, inclination: f64) -> StateVector {
    Keplerian::new(
        Kilometres::new(a).expect("finite"),
        0.001,
        Radians::new(inclination).expect("finite"),
        Radians::new(1.0).expect("finite"),
        Radians::new(0.0).expect("finite"),
        Radians::new(0.3).expect("finite"),
    )
    .expect("elliptic")
    .to_state(MU)
    .expect("state")
}

#[test]
fn equatorial_acceleration_matches_the_closed_form() {
    // On the equator at r = R, with the z terms zero: a_x = −3/2·J2·μ/R².
    let j2 = J2Zonal::new(MU, WGS84_J2, R).expect("valid");
    let state = StateVector {
        position: Vec3::new(R, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    let a = j2.acceleration(seconds(0.0), &state).expect("accel");
    assert_close(a.x, -1.5 * WGS84_J2 * MU / (R * R), CLOSED_REL);
    assert_close(a.y, 0.0, CLOSED_REL);
    assert_close(a.z, 0.0, CLOSED_REL);
}

#[test]
fn polar_acceleration_matches_the_closed_form() {
    // Over the pole, at r = R: a_z = 3·J2·μ/R², pointing away from the centre.
    let j2 = J2Zonal::new(MU, WGS84_J2, R).expect("valid");
    let state = StateVector {
        position: Vec3::new(0.0, 0.0, R),
        velocity: Vec3::new(7.5, 0.0, 0.0),
    };
    let a = j2.acceleration(seconds(0.0), &state).expect("accel");
    assert_close(a.z, 3.0 * WGS84_J2 * MU / (R * R), CLOSED_REL);
    assert_close(a.x, 0.0, CLOSED_REL);
}

#[test]
fn zero_j2_gives_no_acceleration() {
    let j2 = J2Zonal::new(MU, 0.0, R).expect("valid");
    let state = inclined_orbit(7_000.0, 0.9);
    let a = j2.acceleration(seconds(0.0), &state).expect("accel");
    assert_eq!(a, Vec3::new(0.0, 0.0, 0.0));
}

#[test]
fn nodal_regression_matches_the_secular_rate() {
    // Ω̇ = −(3/2)·n·J2·(R/p)²·cos i, with n = sqrt(μ/a³) and p = a(1 − e²) (Vallado; Montenbruck &
    // Gill). The integrated orbit is compared with the regression over ten orbits.
    let a: f64 = 7_000.0;
    let inc: f64 = 0.9;
    let n = (MU / a.powi(3)).sqrt();
    let p = a * (1.0 - 0.001_f64.powi(2));
    let secular_rate = -1.5 * n * WGS84_J2 * (R / p).powi(2) * inc.cos();
    let period = TAU / n;
    let duration = 10.0 * period;

    let start = inclined_orbit(a, inc);
    let start_raan = Keplerian::from_state(&start, MU)
        .expect("elliptic")
        .raan()
        .get();
    let model = (
        CentralBody::new(MU).expect("mu"),
        J2Zonal::earth(MU).expect("j2"),
    );
    let end = Cowell::new(model, seconds(10.0))
        .expect("step")
        .propagate(&start, seconds(duration))
        .expect("integrates");
    let end_raan = Keplerian::from_state(&end, MU)
        .expect("elliptic")
        .raan()
        .get();

    let d = end_raan - start_raan;
    let integrated = d.sin().atan2(d.cos());
    let expected = secular_rate * duration;
    assert!(
        expected.abs() > 0.04,
        "the arc must show a regression well above noise, got {expected}"
    );
    assert_close(integrated, expected, REGRESSION_REL);
}

#[test]
fn without_j2_the_node_does_not_regress() {
    // The control: central gravity alone is a Keplerian orbit, which keeps its node.
    let a: f64 = 7_000.0;
    let n = (MU / a.powi(3)).sqrt();
    let duration = 10.0 * TAU / n;
    let start = inclined_orbit(a, 0.9);
    let start_raan = Keplerian::from_state(&start, MU)
        .expect("elliptic")
        .raan()
        .get();
    let model = CentralBody::new(MU).expect("mu");
    let end = Cowell::new(model, seconds(10.0))
        .expect("step")
        .propagate(&start, seconds(duration))
        .expect("integrates");
    let end_raan = Keplerian::from_state(&end, MU)
        .expect("elliptic")
        .raan()
        .get();
    let d = end_raan - start_raan;
    let drift = d.sin().atan2(d.cos()).abs();
    assert!(drift < 1e-6, "node drifted by {drift} rad without J2");
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(J2Zonal::new(0.0, WGS84_J2, R), Err(Error::InvalidMu));
    assert!(matches!(
        J2Zonal::new(MU, f64::NAN, R),
        Err(Error::NonFinite { .. })
    ));
    assert!(matches!(
        J2Zonal::new(MU, WGS84_J2, 0.0),
        Err(Error::NonFinite { .. })
    ));
    let j2 = J2Zonal::earth(MU).expect("valid");
    let origin = StateVector {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert_eq!(
        j2.acceleration(seconds(0.0), &origin),
        Err(Error::Degenerate)
    );
}
