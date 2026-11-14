// SPDX-License-Identifier: MIT OR Apache-2.0
//! Numerical propagation (Cowell, RK4) against the analytic two-body solution: convergence order,
//! periods, energy drift, force-model sums, and rejection of invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::TAU;
use tpt_kardashev_astrodynamics::propagate::propagate;
use tpt_kardashev_astrodynamics::{
    CentralBody, Cowell, Error, ForceModel, Keplerian, Kilometres, Radians, Seconds, StateVector,
    Vec3,
};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;

/// Bound on the RK4 position error relative to |r| over the 5000 s test arc with a 5 s step. The
/// measured error is 1.4e-10 at 5 s (2.3e-9 at 10 s, 8.7e-12 at 2.5 s); 1e-9 leaves a margin of
/// about seven.
const RK4_REL: f64 = 1e-9;
/// Minimum factor by which halving the step reduces the error. A fourth-order method gives 16,
/// and the measured ratio is 16.4 (10 s to 5 s). 10 allows for the error not yet being in its
/// asymptotic regime.
const MIN_ORDER_RATIO: f64 = 10.0;
/// Energy drift bound over one period, relative to the energy's magnitude.
const ENERGY_REL: f64 = 1e-6;

fn assert_close(actual: f64, expected: f64, rel: f64, abs: f64) {
    let diff = (actual - expected).abs();
    let bound = abs.max(rel * expected.abs());
    assert!(
        diff <= bound,
        "actual {actual}, expected {expected}, diff {diff} > bound {bound}"
    );
}

fn seconds(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

fn elliptic_state() -> StateVector {
    Keplerian::new(
        Kilometres::new(7_000.0).expect("finite"),
        0.1,
        Radians::new(0.6).expect("finite"),
        Radians::new(1.0).expect("finite"),
        Radians::new(2.0).expect("finite"),
        Radians::new(0.7).expect("finite"),
    )
    .expect("elliptic")
    .to_state(MU)
    .expect("state")
}

fn position_error(a: &StateVector, b: &StateVector) -> f64 {
    (a.position - b.position).norm() / b.position.norm()
}

/// A constant acceleration, for checking that force models add.
#[derive(Clone, Copy)]
struct Constant(Vec3);

impl ForceModel for Constant {
    fn acceleration(&self, _t: Seconds, _state: &StateVector) -> Result<Vec3, Error> {
        Ok(self.0)
    }
}

#[test]
fn rk4_error_falls_as_the_fourth_power_of_the_step() {
    // Propagate 5000 s two ways, with steps of 10 s and 5 s, and compare each with the analytic
    // two-body state. The ratio of the errors shows the order of the method.
    let start = elliptic_state();
    let exact = propagate(&start, MU, seconds(5_000.0)).expect("analytic");
    let model = CentralBody::new(MU).expect("valid mu");
    let coarse = Cowell::new(model, seconds(10.0))
        .expect("valid step")
        .propagate(&start, seconds(5_000.0))
        .expect("integrates");
    let fine = Cowell::new(model, seconds(5.0))
        .expect("valid step")
        .propagate(&start, seconds(5_000.0))
        .expect("integrates");
    let e_coarse = position_error(&coarse, &exact);
    let e_fine = position_error(&fine, &exact);
    assert!(
        e_fine <= RK4_REL,
        "5 s step error {e_fine} exceeds {RK4_REL}"
    );
    assert!(
        e_coarse / e_fine >= MIN_ORDER_RATIO,
        "halving the step reduced the error only {}-fold",
        e_coarse / e_fine
    );
}

#[test]
fn a_full_circular_period_returns_to_the_start() {
    let start = Keplerian::new(
        Kilometres::new(7_000.0).expect("finite"),
        0.0,
        Radians::new(0.9).expect("finite"),
        Radians::new(0.4).expect("finite"),
        Radians::new(0.0).expect("finite"),
        Radians::new(2.2).expect("finite"),
    )
    .expect("elliptic")
    .to_state(MU)
    .expect("state");
    let period = TAU * (7_000.0_f64.powi(3) / MU).sqrt();
    let model = CentralBody::new(MU).expect("valid mu");
    let back = Cowell::new(model, seconds(5.0))
        .expect("valid step")
        .propagate(&start, seconds(period))
        .expect("integrates");
    assert!(
        position_error(&back, &start) <= RK4_REL,
        "the orbit did not close: error {}",
        position_error(&back, &start)
    );
}

#[test]
fn energy_drift_over_one_period_is_small() {
    let start = elliptic_state();
    let energy = |s: &StateVector| 0.5 * s.velocity.dot(s.velocity) - MU / s.position.norm();
    let e0 = energy(&start);
    let model = CentralBody::new(MU).expect("valid mu");
    let end = Cowell::new(model, seconds(5.0))
        .expect("valid step")
        .propagate(&start, seconds(5_828.0))
        .expect("integrates");
    assert_close(energy(&end), e0, ENERGY_REL, 0.0);
}

#[test]
fn a_pair_of_models_is_their_sum() {
    let state = elliptic_state();
    let gravity = CentralBody::new(MU).expect("valid mu");
    let thrust = Constant(Vec3::new(1e-7, -2e-7, 3e-7));
    let both = (gravity, thrust);
    let sum = gravity.acceleration(seconds(0.0), &state).expect("gravity")
        + thrust.acceleration(seconds(0.0), &state).expect("thrust");
    let combined = both.acceleration(seconds(0.0), &state).expect("sum");
    assert_eq!(combined, sum);
}

#[test]
fn negative_duration_integrates_backwards() {
    let start = elliptic_state();
    let model = CentralBody::new(MU).expect("valid mu");
    let propagator = Cowell::new(model, seconds(5.0)).expect("valid step");
    let there = propagator
        .propagate(&start, seconds(2_000.0))
        .expect("forward");
    let back = propagator
        .propagate(&there, seconds(-2_000.0))
        .expect("backward");
    assert!(position_error(&back, &start) <= RK4_REL);
}

#[test]
fn zero_duration_returns_the_same_state() {
    let start = elliptic_state();
    let model = CentralBody::new(MU).expect("valid mu");
    let same = Cowell::new(model, seconds(5.0))
        .expect("valid step")
        .propagate(&start, seconds(0.0))
        .expect("integrates");
    assert_eq!(same, start);
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(CentralBody::new(0.0), Err(Error::InvalidMu));
    assert_eq!(CentralBody::new(f64::NAN), Err(Error::InvalidMu));
    let model = CentralBody::new(MU).expect("valid mu");
    assert!(matches!(
        Cowell::new(model, seconds(0.0)),
        Err(Error::InvalidStep)
    ));
    let origin = StateVector {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert_eq!(
        model.acceleration(seconds(0.0), &origin),
        Err(Error::Degenerate)
    );
}
