// SPDX-License-Identifier: MIT OR Apache-2.0
//! Velocity Verlet on the two-body problem: second-order convergence against the analytic
//! solution, bounded energy and angular-momentum error over many orbits, backward integration,
//! and rejection of invalid steps.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::TAU;
use tpt_kardashev_astrodynamics::force::CentralBody;
use tpt_kardashev_astrodynamics::integrate::rk4_step;
use tpt_kardashev_astrodynamics::propagate::propagate;
use tpt_kardashev_astrodynamics::symplectic::{Verlet, verlet_step};
use tpt_kardashev_astrodynamics::{Error, Keplerian, Kilometres, Radians, Seconds, StateVector};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
const A_KM: f64 = 7_000.0;

fn start() -> StateVector {
    Keplerian::new(
        Kilometres::new(A_KM).expect("finite"),
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

fn period() -> f64 {
    TAU * (A_KM.powi(3) / MU).sqrt()
}

fn energy(s: &StateVector) -> f64 {
    0.5 * s.velocity.dot(s.velocity) - MU / s.position.norm()
}

fn position_error(a: &StateVector, b: &StateVector) -> f64 {
    (a.position - b.position).norm()
}

#[test]
fn error_falls_as_the_square_of_the_step() {
    let s0 = start();
    let exact = propagate(&s0, MU, Seconds::new(period()).expect("finite")).expect("analytic");
    let error = |n: f64| {
        let model = CentralBody::new(MU).expect("valid");
        let step = Seconds::new(period() / n).expect("finite");
        let verlet = Verlet::new(model, step).expect("valid");
        let got = verlet
            .propagate(&s0, Seconds::new(period()).expect("finite"))
            .expect("integrates");
        position_error(&got, &exact)
    };
    let coarse = error(200.0);
    let fine = error(400.0);
    let ratio = coarse / fine;
    eprintln!("MEASURED coarse {coarse:e} fine {fine:e} ratio {ratio}");
    assert!(
        (3.8..=4.2).contains(&ratio),
        "error ratio {ratio} on halving the step"
    );
}

/// The largest relative energy error within each orbit, sampled at every step.
fn per_orbit_energy_error<F>(steps_per_orbit: usize, orbits: usize, advance: F) -> Vec<f64>
where
    F: Fn(f64, &StateVector, f64) -> Result<StateVector, Error>,
{
    let e0 = energy(&start());
    let h = period() / steps_per_orbit as f64;
    let mut state = start();
    let mut t = 0.0;
    let mut worst = Vec::with_capacity(orbits);
    for _ in 0..orbits {
        let mut peak = 0.0f64;
        for _ in 0..steps_per_orbit {
            state = advance(t, &state, h).expect("integrates");
            t += h;
            peak = peak.max(((energy(&state) - e0) / e0).abs());
        }
        worst.push(peak);
    }
    worst
}

fn window_max(values: &[f64]) -> f64 {
    values.iter().copied().fold(0.0, f64::max)
}

/// Over a thousand orbits, the peak energy error in the last hundred orbits must not exceed twice
/// the peak in the first hundred. RK4 at the same step is measured alongside for comparison.
#[test]
fn energy_error_stays_bounded_over_a_thousand_orbits() {
    let model = CentralBody::new(MU).expect("valid");
    for steps_per_orbit in [50, 200] {
        let verlet = per_orbit_energy_error(steps_per_orbit, 1000, |t, s, h| {
            verlet_step(&model, t, s, h)
        });
        let rk4 =
            per_orbit_energy_error(steps_per_orbit, 1000, |t, s, h| rk4_step(&model, t, s, h));
        let (v_early, v_late) = (window_max(&verlet[..100]), window_max(&verlet[900..]));
        let (r_early, r_late) = (window_max(&rk4[..100]), window_max(&rk4[900..]));
        eprintln!("MEASURED {steps_per_orbit}/orbit verlet early {v_early:e} late {v_late:e}");
        eprintln!("MEASURED {steps_per_orbit}/orbit rk4 early {r_early:e} late {r_late:e}");
        assert!(
            v_late <= 2.0 * v_early,
            "verlet energy error grew at {steps_per_orbit} steps per orbit: {v_early} to {v_late}"
        );
    }
}

#[test]
fn angular_momentum_error_stays_bounded_over_a_hundred_orbits() {
    let model = CentralBody::new(MU).expect("valid");
    let verlet =
        Verlet::new(model, Seconds::new(period() / 200.0).expect("finite")).expect("valid");
    let s0 = start();
    let h0 = s0.position.cross(s0.velocity);
    let mut state = s0;
    let mut worst = 0.0f64;
    for _ in 0..100 {
        state = verlet
            .propagate(&state, Seconds::new(period()).expect("finite"))
            .expect("integrates");
        let h = state.position.cross(state.velocity);
        worst = worst.max((h - h0).norm() / h0.norm());
    }
    eprintln!("MEASURED angular momentum worst {worst:e}");
    // Measured at 1.5e-14 over a hundred orbits; the bound is about seventy times that.
    assert!(worst <= 1e-12, "relative angular-momentum error {worst}");
}

#[test]
fn backward_integration_returns_to_the_start() {
    let model = CentralBody::new(MU).expect("valid");
    let verlet =
        Verlet::new(model, Seconds::new(period() / 200.0).expect("finite")).expect("valid");
    let s0 = start();
    let forward = verlet
        .propagate(&s0, Seconds::new(5.0 * period()).expect("finite"))
        .expect("integrates");
    let back = verlet
        .propagate(&forward, Seconds::new(-5.0 * period()).expect("finite"))
        .expect("integrates");
    let err = position_error(&back, &s0);
    eprintln!("MEASURED round trip {err:e}");
    // Measured at 2.9e-9 km after five orbits forward and back at P/200; the bound is about 3.4 times that.
    assert!(err <= 1e-8, "round-trip position error {err} km");
}

#[test]
fn zero_duration_returns_the_start() {
    let model = CentralBody::new(MU).expect("valid");
    let verlet = Verlet::new(model, Seconds::new(10.0).expect("finite")).expect("valid");
    let s0 = start();
    let got = verlet
        .propagate(&s0, Seconds::new(0.0).expect("finite"))
        .expect("integrates");
    assert_eq!(got, s0);
}

#[test]
fn invalid_steps_are_rejected() {
    let model = CentralBody::new(MU).expect("valid");
    assert!(matches!(
        Verlet::new(model, Seconds::new(0.0).expect("finite")),
        Err(Error::InvalidStep)
    ));
    assert!(matches!(
        Verlet::new(model, Seconds::new(-5.0).expect("finite")),
        Err(Error::InvalidStep)
    ));
}
