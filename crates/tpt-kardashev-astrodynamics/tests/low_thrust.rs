// SPDX-License-Identifier: MIT OR Apache-2.0
//! Edelbaum's low-thrust approximation: closed forms (no plane change, pure plane change, the small-angle
//! ratio to the impulsive plane change, symmetry, the time of flight), rejection of invalid input, and a
//! numerical check: a constant prograde thrust integrated from a circular orbit, against the energy the
//! approximation predicts at the end.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::PI;

use tpt_kardashev_astrodynamics::force::{CentralBody, Thrust, ThrustDirection};
use tpt_kardashev_astrodynamics::low_thrust::edelbaum;
use tpt_kardashev_astrodynamics::manoeuvre::plane_change;
use tpt_kardashev_astrodynamics::{Cowell, Error, Seconds, StateVector, Vec3};

const MU: f64 = 398_600.441_8;

fn circular_speed(r: f64) -> f64 {
    (MU / r).sqrt()
}

#[test]
fn with_no_plane_change_the_delta_v_is_the_difference_of_the_circular_speeds() {
    let (r1, r2) = (7_000.0, 7_500.0);
    let t = edelbaum(MU, r1, r2, 0.0, 1e-5).expect("valid");
    let expected = (circular_speed(r1) - circular_speed(r2)).abs();
    assert!(
        (t.delta_v - expected).abs() <= 1e-12 * expected,
        "{} vs {expected}",
        t.delta_v
    );
}

#[test]
fn a_pure_plane_change_at_one_radius_is_two_v_sin_of_a_quarter_of_the_angle() {
    // With r1 = r2 = r, Edelbaum's Δv = 2·v·sin(π·Δi/4).
    let r = 7_000.0;
    let di = 0.3;
    let t = edelbaum(MU, r, r, di, 1e-5).expect("valid");
    let v = circular_speed(r);
    let expected = 2.0 * v * (PI * di / 4.0).sin();
    assert!(
        (t.delta_v - expected).abs() <= 1e-12 * expected,
        "{} vs {expected}",
        t.delta_v
    );
}

#[test]
fn for_a_small_plane_change_the_continuous_cost_is_pi_over_two_times_the_impulsive_one() {
    // Edelbaum's 2·v·sin(π·Δi/4) over the impulsive 2·v·sin(Δi/2) tends to π/2 as Δi → 0. At Δi = 1e-4 the
    // ratio is π/2 to about (Δi)²: the difference is below 1e-8.
    let r = 7_000.0;
    let v = circular_speed(r);
    for di in [1e-4, 1e-3] {
        let continuous = edelbaum(MU, r, r, di, 1e-5).expect("valid").delta_v;
        let impulsive = plane_change(v, v, di).expect("valid");
        let ratio = continuous / impulsive;
        assert!((ratio - PI / 2.0).abs() <= 1e-6, "Δi = {di}: ratio {ratio}");
    }
}

#[test]
fn the_transfer_is_symmetric_in_its_two_radii() {
    let up = edelbaum(MU, 7_000.0, 9_000.0, 0.2, 1e-5).expect("valid");
    let down = edelbaum(MU, 9_000.0, 7_000.0, 0.2, 1e-5).expect("valid");
    assert!((up.delta_v - down.delta_v).abs() <= 1e-12 * up.delta_v);
}

#[test]
fn the_time_of_flight_is_the_delta_v_over_the_acceleration() {
    let a = edelbaum(MU, 7_000.0, 7_500.0, 0.1, 1e-5).expect("valid");
    assert!((a.time_of_flight - a.delta_v / 1e-5).abs() <= 1e-9 * a.time_of_flight);
    // Doubling the acceleration halves the time and leaves the Δv alone.
    let b = edelbaum(MU, 7_000.0, 7_500.0, 0.1, 2e-5).expect("valid");
    assert_eq!(b.delta_v, a.delta_v);
    assert!((b.time_of_flight - a.time_of_flight / 2.0).abs() <= 1e-9 * a.time_of_flight);
}

#[test]
fn invalid_input_is_rejected() {
    assert_eq!(
        edelbaum(0.0, 7_000.0, 7_500.0, 0.0, 1e-5),
        Err(Error::InvalidMu)
    );
    assert_eq!(
        edelbaum(MU, 0.0, 7_500.0, 0.0, 1e-5),
        Err(Error::InvalidRadius)
    );
    assert_eq!(
        edelbaum(MU, 7_000.0, f64::NAN, 0.0, 1e-5),
        Err(Error::InvalidRadius)
    );
    assert!(matches!(
        edelbaum(MU, 7_000.0, 7_500.0, f64::INFINITY, 1e-5),
        Err(Error::NonFinite { .. })
    ));
    assert_eq!(
        edelbaum(MU, 7_000.0, 7_500.0, 0.0, 0.0),
        Err(Error::InvalidAcceleration)
    );
    assert_eq!(
        edelbaum(MU, 7_000.0, 7_500.0, 0.0, -1e-5),
        Err(Error::InvalidAcceleration)
    );
}

#[test]
fn a_constant_prograde_spiral_delivers_the_energy_the_approximation_predicts() {
    // From the circular orbit of radius 7000 km, a constant prograde acceleration for the Edelbaum time of
    // flight, to a circular orbit of 7500 km. Energy is the quantity the approximation fixes: the work of the
    // thrust is ∫ f·v dt ≈ v̄·f·T, and for circular orbits ΔE = v̄·(v1 − v2) with v̄ = (v1 + v2)/2. So the
    // energy at the end should match the circular orbit at r2. Integrated with RK4 at a 5 s step.
    let (r1, r2) = (7_000.0, 7_500.0);
    let acceleration = 1e-5;
    let transfer = edelbaum(MU, r1, r2, 0.0, acceleration).expect("valid");
    let duration = Seconds::new(transfer.time_of_flight).expect("duration");
    let burn = Thrust::new(
        ThrustDirection::Prograde,
        acceleration,
        Seconds::new(0.0).expect("start"),
        duration,
    )
    .expect("burn");
    let model = (CentralBody::new(MU).expect("mu"), burn);
    let propagator = Cowell::new(model, Seconds::new(5.0).expect("step")).expect("propagator");
    let start = StateVector {
        position: Vec3::new(r1, 0.0, 0.0),
        velocity: Vec3::new(0.0, circular_speed(r1), 0.0),
    };
    let end = propagator.propagate(&start, duration).expect("integration");

    let energy = |s: &StateVector| 0.5 * s.velocity.dot(s.velocity) - MU / s.position.norm();
    let e1 = -MU / (2.0 * r1);
    let e2 = -MU / (2.0 * r2);
    let e_final = energy(&end);
    // Fraction of the energy change the approximation predicts that the integration delivered.
    let delivered = (e_final - e1) / (e2 - e1);
    let semi_major = -MU / (2.0 * e_final);
    let a_error = (semi_major - r2).abs() / r2;
    // Measured at this step: delivered fraction 1.00004, semi-major axis error 2.9e-6. The bounds are about
    // thirty times those. They check the energy bookkeeping only, not the shape of the spiral.
    assert!(
        (delivered - 1.0).abs() <= 1e-3,
        "delivered fraction {delivered}"
    );
    assert!(a_error <= 1e-4, "semi-major-axis error {a_error}");
}
