// SPDX-License-Identifier: MIT OR Apache-2.0
//! Finite burns: direction and magnitude in the two frames of reference, the burn window, the Δv
//! of a short burn in an integrated orbit, and rejection of invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::force::{CentralBody, Thrust, ThrustDirection};
use tpt_kardashev_astrodynamics::{Cowell, Error, ForceModel, Seconds, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
/// Tolerance on closed-form vectors (policy: 1e-12 relative).
const CLOSED_REL: f64 = 1e-12;

fn seconds(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

fn state() -> StateVector {
    StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    }
}

#[test]
fn an_inertial_burn_pushes_along_its_direction_inside_the_window() {
    let burn = Thrust::new(
        ThrustDirection::Inertial(Vec3::new(0.0, 0.0, 3.0)),
        2e-4,
        seconds(10.0),
        seconds(20.0),
    )
    .expect("valid");
    // Before the window, at the start, and inside it: the acceleration is 2e-4 km/s² along +z.
    assert_eq!(
        burn.acceleration(seconds(5.0), &state()).expect("accel"),
        Vec3::new(0.0, 0.0, 0.0)
    );
    let inside = burn.acceleration(seconds(10.0), &state()).expect("accel");
    assert!((inside - Vec3::new(0.0, 0.0, 2e-4)).norm() <= CLOSED_REL * 2e-4);
    // The window is half-open: the end is outside it.
    assert_eq!(
        burn.acceleration(seconds(20.0), &state()).expect("accel"),
        Vec3::new(0.0, 0.0, 0.0)
    );
}

#[test]
fn a_prograde_burn_is_along_the_velocity() {
    let burn =
        Thrust::new(ThrustDirection::Prograde, 1e-4, seconds(0.0), seconds(60.0)).expect("valid");
    let s = state();
    let a = burn.acceleration(seconds(1.0), &s).expect("accel");
    let v_hat = s.velocity * (1.0 / s.velocity.norm());
    assert!((a - v_hat * 1e-4).norm() <= CLOSED_REL * 1e-4);
}

#[test]
fn a_prograde_burn_needs_a_velocity() {
    let burn =
        Thrust::new(ThrustDirection::Prograde, 1e-4, seconds(0.0), seconds(60.0)).expect("valid");
    let still = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 0.0, 0.0),
    };
    assert_eq!(
        burn.acceleration(seconds(1.0), &still),
        Err(Error::Degenerate)
    );
}

#[test]
fn a_short_burn_changes_the_velocity_by_its_impulse() {
    // A 10 s prograde burn of 1e-4 km/s² in orbit about the Earth. The difference between the burn
    // and the coast is the thrust impulse a·T. The thrust switches on and off at the window edges, and
    // a fixed-step RK4 mis-samples a step that straddles an edge. Measured: 3e-5 relative at 0.1 s, 8e-3 at 0.5 s.
    let a = 1e-4;
    let t = 10.0;
    let start = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, (MU / 7_000.0).sqrt(), 0.0),
    };
    let central = CentralBody::new(MU).expect("valid");
    let burn = Thrust::new(ThrustDirection::Prograde, a, seconds(0.0), seconds(t)).expect("valid");
    let coast = Cowell::new(central, seconds(0.1))
        .expect("step")
        .propagate(&start, seconds(t))
        .expect("integrates");
    let burned = Cowell::new((central, burn), seconds(0.1))
        .expect("step")
        .propagate(&start, seconds(t))
        .expect("integrates");
    let impulse = (burned.velocity - coast.velocity).norm();
    assert!(
        (impulse - a * t).abs() <= 1e-4 * a * t,
        "impulse {impulse} km/s, expected {}",
        a * t
    );
}

#[test]
fn invalid_burns_are_rejected() {
    assert_eq!(
        Thrust::new(ThrustDirection::Prograde, -1.0, seconds(0.0), seconds(1.0)).map(|_| ()),
        Err(Error::Negative {
            what: "thrust magnitude"
        })
    );
    assert_eq!(
        Thrust::new(ThrustDirection::Prograde, 1.0, seconds(5.0), seconds(1.0)).map(|_| ()),
        Err(Error::InvalidWindow)
    );
    assert!(matches!(
        Thrust::new(
            ThrustDirection::Inertial(Vec3::new(0.0, 0.0, 0.0)),
            1.0,
            seconds(0.0),
            seconds(1.0)
        )
        .map(|_| ()),
        Err(Error::NonFinite { .. })
    ));
    assert!(matches!(
        Thrust::new(
            ThrustDirection::Prograde,
            f64::NAN,
            seconds(0.0),
            seconds(1.0)
        )
        .map(|_| ()),
        Err(Error::NonFinite { .. })
    ));
}
