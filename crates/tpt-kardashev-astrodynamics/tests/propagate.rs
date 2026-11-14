// SPDX-License-Identifier: MIT OR Apache-2.0
//! Two-body propagation: periods, conserved quantities, composition, reversal, agreement with the
//! Kepler equation solved independently, and hyperbolic states.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::{PI, TAU};
use tpt_kardashev_astrodynamics::propagate::propagate;
use tpt_kardashev_astrodynamics::{
    Error, Keplerian, Kilometres, Radians, Seconds, StateVector, Vec3,
};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;

/// Round trips and conserved quantities through the Newton iteration (iterative-solver policy:
/// 1e-9 relative, 1e-9 absolute).
const ROUND_REL: f64 = 1e-9;
const ROUND_ABS: f64 = 1e-9;

fn assert_close(actual: f64, expected: f64, rel: f64, abs: f64) {
    let diff = (actual - expected).abs();
    let bound = abs.max(rel * expected.abs());
    assert!(
        diff <= bound,
        "actual {actual}, expected {expected}, diff {diff} > bound {bound}"
    );
}

fn assert_state_close(actual: &StateVector, expected: &StateVector, rel: f64) {
    let dr = (actual.position - expected.position).norm();
    let dv = (actual.velocity - expected.velocity).norm();
    let r = expected.position.norm();
    let v = expected.velocity.norm();
    assert!(
        dr <= rel * r.max(1.0),
        "position differs by {dr} km (|r| = {r} km)"
    );
    assert!(
        dv <= rel * v.max(1.0),
        "velocity differs by {dv} km/s (|v| = {v} km/s)"
    );
}

fn secs(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

fn elliptic_state() -> StateVector {
    let el = Keplerian::new(
        Kilometres::new(7_000.0).expect("finite"),
        0.1,
        Radians::new(0.6).expect("finite"),
        Radians::new(1.0).expect("finite"),
        Radians::new(2.0).expect("finite"),
        Radians::new(0.7).expect("finite"),
    )
    .expect("elliptic");
    el.to_state(MU).expect("state")
}

#[test]
fn a_full_circular_period_returns_to_the_start() {
    // A circular orbit at 7000 km: T = 2π·sqrt(a³/μ). Half a period sends the spacecraft to the
    // opposite side of the orbit.
    let a: f64 = 7_000.0;
    let el = Keplerian::new(
        Kilometres::new(a).expect("finite"),
        0.0,
        Radians::new(0.9).expect("finite"),
        Radians::new(0.4).expect("finite"),
        Radians::new(0.0).expect("finite"),
        Radians::new(2.2).expect("finite"),
    )
    .expect("elliptic");
    let start = el.to_state(MU).expect("state");
    let period = TAU * (a.powi(3) / MU).sqrt();

    let full = propagate(&start, MU, secs(period)).expect("propagates");
    assert_state_close(&full, &start, ROUND_REL);

    let half = propagate(&start, MU, secs(period / 2.0)).expect("propagates");
    assert_state_close(
        &half,
        &StateVector {
            position: start.position * -1.0,
            velocity: start.velocity * -1.0,
        },
        ROUND_REL,
    );
}

#[test]
fn energy_and_angular_momentum_are_conserved() {
    let start = elliptic_state();
    let energy0 = 0.5 * start.velocity.dot(start.velocity) - MU / start.position.norm();
    let h0 = start.position.cross(start.velocity);
    for &t in &[10.0, 500.0, 2_000.0, 5_800.0, 40_000.0] {
        let s = propagate(&start, MU, secs(t)).expect("propagates");
        let energy = 0.5 * s.velocity.dot(s.velocity) - MU / s.position.norm();
        assert_close(energy, energy0, ROUND_REL, ROUND_ABS);
        let h = s.position.cross(s.velocity);
        assert_state_close(
            &StateVector {
                position: h,
                velocity: Vec3::new(0.0, 0.0, 0.0),
            },
            &StateVector {
                position: h0,
                velocity: Vec3::new(0.0, 0.0, 0.0),
            },
            ROUND_REL,
        );
    }
}

#[test]
fn propagation_composes() {
    let start = elliptic_state();
    let (t1, t2) = (1_234.5, 4_321.0);
    let staged = propagate(
        &propagate(&start, MU, secs(t1)).expect("propagates"),
        MU,
        secs(t2),
    )
    .expect("propagates");
    let direct = propagate(&start, MU, secs(t1 + t2)).expect("propagates");
    assert_state_close(&staged, &direct, ROUND_REL);
}

#[test]
fn propagating_backwards_undoes_propagating_forwards() {
    let start = elliptic_state();
    let there = propagate(&start, MU, secs(3_000.0)).expect("propagates");
    let back = propagate(&there, MU, secs(-3_000.0)).expect("propagates");
    assert_state_close(&back, &start, ROUND_REL);
}

#[test]
fn zero_time_returns_the_same_state() {
    let start = elliptic_state();
    let same = propagate(&start, MU, secs(0.0)).expect("propagates");
    assert_state_close(&same, &start, ROUND_REL);
}

/// Solves Kepler's equation M = E − e·sin E by Newton's method, for the test's own reference.
fn eccentric_anomaly(mean: f64, e: f64) -> f64 {
    let mut big_e = if e > 0.8 { PI } else { mean };
    for _ in 0..100 {
        let f = big_e - e * big_e.sin() - mean;
        let df = 1.0 - e * big_e.cos();
        big_e -= f / df;
    }
    big_e
}

#[test]
fn agrees_with_the_classical_kepler_equation() {
    // The true anomaly advanced by the mean motion, solved independently with Kepler's equation.
    let (a, e) = (7_000.0_f64, 0.1);
    let el = Keplerian::new(
        Kilometres::new(a).expect("finite"),
        e,
        Radians::new(0.6).expect("finite"),
        Radians::new(1.0).expect("finite"),
        Radians::new(2.0).expect("finite"),
        Radians::new(0.7).expect("finite"),
    )
    .expect("elliptic");
    let start = el.to_state(MU).expect("state");
    let n = (MU / a.powi(3)).sqrt();
    let nu0 = 0.7_f64;
    let big_e0 = 2.0 * (((1.0 - e) / (1.0 + e)).sqrt() * (nu0 / 2.0).tan()).atan();
    let mean0 = big_e0 - e * big_e0.sin();

    for &t in &[300.0, 1_500.0, 4_000.0] {
        let mean = mean0 + n * t;
        let big_e = eccentric_anomaly(mean, e);
        let nu = 2.0
            * (((1.0 + e).sqrt() * (big_e / 2.0).sin())
                .atan2((1.0 - e).sqrt() * (big_e / 2.0).cos()));
        let propagated = propagate(&start, MU, secs(t)).expect("propagates");
        let nu_propagated = Keplerian::from_state(&propagated, MU)
            .expect("elliptic")
            .true_anomaly()
            .get();
        let d = nu_propagated - nu;
        let diff = d.sin().atan2(d.cos()).abs();
        assert!(
            diff <= ROUND_ABS,
            "true anomaly differs by {diff} rad at t = {t}"
        );
    }
}

#[test]
fn hyperbolic_states_propagate_and_keep_their_energy() {
    // Speed 12 km/s at 7000 km is above escape speed, so the orbit is hyperbolic.
    let start = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 12.0, 0.0),
    };
    let energy0 = 0.5 * start.velocity.dot(start.velocity) - MU / start.position.norm();
    assert!(energy0 > 0.0);
    let later = propagate(&start, MU, secs(600.0)).expect("propagates");
    let energy = 0.5 * later.velocity.dot(later.velocity) - MU / later.position.norm();
    assert_close(energy, energy0, ROUND_REL, ROUND_ABS);
    assert!(
        later.position.norm() > start.position.norm(),
        "the distance from the centre grew"
    );
}

#[test]
fn invalid_inputs_are_rejected() {
    let start = elliptic_state();
    assert_eq!(propagate(&start, 0.0, secs(1.0)), Err(Error::InvalidMu));
    assert_eq!(
        propagate(&start, f64::NAN, secs(1.0)),
        Err(Error::InvalidMu)
    );
    let zero = StateVector {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert_eq!(propagate(&zero, MU, secs(1.0)), Err(Error::Degenerate));
    let nan = StateVector {
        position: Vec3::new(f64::NAN, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert!(matches!(
        propagate(&nan, MU, secs(1.0)),
        Err(Error::NonFinite { .. })
    ));
}
