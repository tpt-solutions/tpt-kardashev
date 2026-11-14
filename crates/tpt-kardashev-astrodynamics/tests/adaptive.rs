// SPDX-License-Identifier: MIT OR Apache-2.0
//! The adaptive Dormand–Prince integrator against the analytic two-body solution: accuracy over several
//! orbits of an eccentric orbit, the effect of the tolerances, energy conservation, backward
//! propagation, and rejection of invalid tolerances.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::TAU;
use tpt_kardashev_astrodynamics::adaptive::AdaptiveCowell;
use tpt_kardashev_astrodynamics::force::CentralBody;
use tpt_kardashev_astrodynamics::propagate::propagate;
use tpt_kardashev_astrodynamics::{Error, Keplerian, Kilometres, Radians, Seconds, StateVector};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;

fn secs(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

/// An eccentric orbit, a = 7000 km and e = 0.3, at the given angles.
fn eccentric() -> StateVector {
    Keplerian::new(
        Kilometres::new(7_000.0).expect("finite"),
        0.3,
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
    (a.position - b.position).norm()
}

fn energy(s: &StateVector) -> f64 {
    0.5 * s.velocity.dot(s.velocity) - MU / s.position.norm()
}

#[test]
fn three_orbits_of_an_eccentric_orbit_match_the_analytic_solution() {
    let start = eccentric();
    let period = TAU * (7_000.0_f64.powi(3) / MU).sqrt();
    let duration = 3.0 * period;
    let exact = propagate(&start, MU, secs(duration)).expect("analytic");
    let model = CentralBody::new(MU).expect("valid");
    let adaptive = AdaptiveCowell::new(model, 1e-12, 1e-12).expect("valid tolerances");
    let got = adaptive
        .propagate(&start, secs(duration))
        .expect("integrates");
    // Measured: 5e-7 km after three orbits with tolerances of 1e-12 (a bound of 1e-6 km is used).
    let err = position_error(&got, &exact);
    assert!(err <= 1e-6, "position error {err} km after three orbits");
}

#[test]
fn tighter_tolerances_give_smaller_errors() {
    let start = eccentric();
    let period = TAU * (7_000.0_f64.powi(3) / MU).sqrt();
    let duration = 2.0 * period;
    let exact = propagate(&start, MU, secs(duration)).expect("analytic");
    let model = CentralBody::new(MU).expect("valid");
    let loose = AdaptiveCowell::new(model, 1e-6, 1e-6)
        .expect("valid")
        .propagate(&start, secs(duration))
        .expect("integrates");
    let tight = AdaptiveCowell::new(model, 1e-12, 1e-12)
        .expect("valid")
        .propagate(&start, secs(duration))
        .expect("integrates");
    let e_loose = position_error(&loose, &exact);
    let e_tight = position_error(&tight, &exact);
    assert!(e_tight < e_loose, "tight {e_tight} km, loose {e_loose} km");
    assert!(e_tight < 1e-6);
}

#[test]
fn energy_is_conserved_over_several_orbits() {
    let start = eccentric();
    let period = TAU * (7_000.0_f64.powi(3) / MU).sqrt();
    let model = CentralBody::new(MU).expect("valid");
    let end = AdaptiveCowell::new(model, 1e-12, 1e-12)
        .expect("valid")
        .propagate(&start, secs(5.0 * period))
        .expect("integrates");
    let e0 = energy(&start);
    let drift = ((energy(&end) - e0) / e0).abs();
    assert!(drift <= 1e-10, "relative energy drift {drift}");
}

#[test]
fn backward_propagation_returns_to_the_start() {
    let start = eccentric();
    let model = CentralBody::new(MU).expect("valid");
    let adaptive = AdaptiveCowell::new(model, 1e-12, 1e-12).expect("valid");
    let there = adaptive.propagate(&start, secs(4_000.0)).expect("forward");
    let back = adaptive
        .propagate(&there, secs(-4_000.0))
        .expect("backward");
    assert!(position_error(&back, &start) <= 1e-6);
}

#[test]
fn zero_duration_returns_the_same_state() {
    let start = eccentric();
    let model = CentralBody::new(MU).expect("valid");
    let same = AdaptiveCowell::new(model, 1e-9, 1e-9)
        .expect("valid")
        .propagate(&start, secs(0.0))
        .expect("integrates");
    assert_eq!(same, start);
}

#[test]
fn invalid_tolerances_are_rejected() {
    let model = CentralBody::new(MU).expect("valid");
    assert_eq!(
        AdaptiveCowell::new(model, 0.0, 1e-9).map(|_| ()),
        Err(Error::InvalidTolerance)
    );
    assert_eq!(
        AdaptiveCowell::new(model, 1e-9, -1.0).map(|_| ()),
        Err(Error::InvalidTolerance)
    );
    assert_eq!(
        AdaptiveCowell::new(model, f64::NAN, 1e-9).map(|_| ()),
        Err(Error::InvalidTolerance)
    );
}
