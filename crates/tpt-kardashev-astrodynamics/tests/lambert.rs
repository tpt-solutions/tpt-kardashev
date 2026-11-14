// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lambert's problem: known answers on circular arcs, round trips through the propagator for
//! prograde and retrograde orbits, and rejection of degenerate and invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::PI;
use tpt_kardashev_astrodynamics::lambert::lambert;
use tpt_kardashev_astrodynamics::propagate::propagate;
use tpt_kardashev_astrodynamics::{
    Error, Keplerian, Kilometres, Radians, Seconds, StateVector, Vec3,
};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
/// Round trips and closed forms (iterative-solver policy: 1e-9 relative).
const REL: f64 = 1e-9;

fn assert_vec_close(actual: Vec3, expected: Vec3, rel: f64) {
    let scale = expected.norm().max(1.0);
    let diff = (actual - expected).norm();
    assert!(
        diff <= rel * scale,
        "actual {actual:?}, expected {expected:?}, diff {diff}"
    );
}

fn secs(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

fn orbit(inclination: f64) -> StateVector {
    Keplerian::new(
        Kilometres::new(7_000.0).expect("finite"),
        0.1,
        Radians::new(inclination).expect("finite"),
        Radians::new(1.0).expect("finite"),
        Radians::new(2.0).expect("finite"),
        Radians::new(0.7).expect("finite"),
    )
    .expect("elliptic")
    .to_state(MU)
    .expect("state")
}

#[test]
fn quarter_circle_on_a_circular_orbit_is_known_exactly() {
    // From (r, 0, 0) to (0, r, 0) in a quarter period: the spacecraft flies a 90° arc at the
    // circular speed, so v1 = (0, v, 0) and v2 = (−v, 0, 0).
    let r = 7_000.0;
    let v = (MU / r).sqrt();
    let n = (MU / (r * r * r)).sqrt();
    let (v1, v2) = lambert(
        MU,
        Vec3::new(r, 0.0, 0.0),
        Vec3::new(0.0, r, 0.0),
        secs((PI / 2.0) / n),
        true,
    )
    .expect("solves");
    assert_vec_close(v1, Vec3::new(0.0, v, 0.0), REL);
    assert_vec_close(v2, Vec3::new(-v, 0.0, 0.0), REL);
}

#[test]
fn three_quarter_circle_clockwise_is_known_exactly() {
    // The same endpoints, flown clockwise over 270° in three quarters of a period.
    let r = 7_000.0;
    let v = (MU / r).sqrt();
    let n = (MU / (r * r * r)).sqrt();
    let (v1, v2) = lambert(
        MU,
        Vec3::new(r, 0.0, 0.0),
        Vec3::new(0.0, r, 0.0),
        secs((3.0 * PI / 2.0) / n),
        false,
    )
    .expect("solves");
    assert_vec_close(v1, Vec3::new(0.0, -v, 0.0), REL);
    assert_vec_close(v2, Vec3::new(v, 0.0, 0.0), REL);
}

#[test]
fn prograde_transfer_round_trips_through_the_propagator() {
    // Take an orbit, propagate it 1500 s, and ask Lambert for the velocities that join the two
    // positions in that time. They must be the orbit's own velocities.
    let start = orbit(0.6);
    let end = propagate(&start, MU, secs(1_500.0)).expect("propagates");
    let (v1, v2) = lambert(MU, start.position, end.position, secs(1_500.0), true).expect("solves");
    assert_vec_close(v1, start.velocity, REL);
    assert_vec_close(v2, end.velocity, REL);
}

#[test]
fn retrograde_transfer_round_trips_through_the_propagator() {
    // A retrograde orbit (inclination above 90°) flies clockwise about +z, so it is joined with
    // prograde = false.
    let start = orbit(2.2);
    assert!(
        start.position.cross(start.velocity).z < 0.0,
        "the orbit must be clockwise"
    );
    let end = propagate(&start, MU, secs(1_500.0)).expect("propagates");
    let (v1, v2) = lambert(MU, start.position, end.position, secs(1_500.0), false).expect("solves");
    assert_vec_close(v1, start.velocity, REL);
    assert_vec_close(v2, end.velocity, REL);
}

#[test]
fn degenerate_geometry_is_rejected() {
    let r1 = Vec3::new(7_000.0, 0.0, 0.0);
    // Opposite points: the transfer plane is undefined.
    assert_eq!(
        lambert(MU, r1, Vec3::new(-7_000.0, 0.0, 0.0), secs(1_000.0), true),
        Err(Error::Degenerate)
    );
    // Collinear with the origin.
    assert_eq!(
        lambert(MU, r1, Vec3::new(14_000.0, 0.0, 0.0), secs(1_000.0), true),
        Err(Error::Degenerate)
    );
    // The transfer plane contains the z axis, so its sense is not defined.
    assert_eq!(
        lambert(MU, r1, Vec3::new(0.0, 0.0, 7_000.0), secs(1_000.0), true),
        Err(Error::Degenerate)
    );
    assert_eq!(
        lambert(MU, Vec3::new(0.0, 0.0, 0.0), r1, secs(1_000.0), true),
        Err(Error::Degenerate)
    );
}

#[test]
fn invalid_inputs_are_rejected() {
    let r1 = Vec3::new(7_000.0, 0.0, 0.0);
    let r2 = Vec3::new(0.0, 7_000.0, 0.0);
    assert_eq!(
        lambert(0.0, r1, r2, secs(1_000.0), true),
        Err(Error::InvalidMu)
    );
    assert!(matches!(
        lambert(MU, Vec3::new(f64::NAN, 0.0, 0.0), r2, secs(1_000.0), true),
        Err(Error::NonFinite { .. })
    ));
    assert_eq!(
        lambert(MU, r1, r2, secs(0.0), true),
        Err(Error::InvalidTimeOfFlight)
    );
}
