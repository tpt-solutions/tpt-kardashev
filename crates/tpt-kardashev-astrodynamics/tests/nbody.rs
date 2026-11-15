// SPDX-License-Identifier: MIT OR Apache-2.0
//! The symplectic N-body propagator against analytic results: the equilateral three-body rotating
//! solution (the Lagrange point configuration of three equal masses), bounded energy and conserved
//! momentum over many periods on a stable binary, agreement with the single-body Verlet integrator
//! when one body is a test particle, and rejection of invalid inputs.
#![cfg(all(feature = "std", feature = "alloc"))]
#![allow(clippy::expect_used)]

use std::f64::consts::PI;

use tpt_kardashev_astrodynamics::force::CentralBody;
use tpt_kardashev_astrodynamics::nbody::{NBodyState, NBodySystem, NBodyVerlet};
use tpt_kardashev_astrodynamics::symplectic::Verlet;
use tpt_kardashev_astrodynamics::{Error, Seconds, StateVector, Vec3};

/// Three equal bodies (μ = 1 km³/s² each, G = 1 so the mass is 1) at the corners of an equilateral
/// triangle of side 1 km, rotating about the centroid. The centripetal acceleration is
/// ω²R = 3μ/a³ · a/√3 = √3 μ/a² on each body, which is the resultant of the two others, so the
/// configuration is a circular solution with ω = √3 and period 2π/√3.
///
/// This configuration is linearly unstable for equal masses (Routh's criterion needs
/// 27(m₁m₂ + m₂m₃ + m₃m₁) < (m₁ + m₂ + m₃)², which fails here), so it is used only for a single period,
/// where the perturbations from rounding stay small.
fn equilateral_triple() -> (NBodySystem, NBodyState, f64) {
    let system = NBodySystem::new(vec![1.0, 1.0, 1.0]).expect("system");
    let omega = 3f64.sqrt();
    let r = 1.0 / 3f64.sqrt();
    let mut positions = Vec::new();
    let mut velocities = Vec::new();
    for k in 0..3 {
        let theta = 2.0 * PI * k as f64 / 3.0;
        positions.push(Vec3::new(r * theta.cos(), r * theta.sin(), 0.0));
        velocities.push(Vec3::new(
            -omega * r * theta.sin(),
            omega * r * theta.cos(),
            0.0,
        ));
    }
    (
        system,
        NBodyState {
            positions,
            velocities,
        },
        2.0 * PI / omega,
    )
}

/// Two equal bodies (μ = 1 each) on a circular orbit about their centre of mass, separated by 1 km:
/// ω² = G(m₁ + m₂)/a³ = 2, so each body moves at ω·(a/2) about the centre. This orbit is stable.
fn circular_binary() -> (NBodySystem, NBodyState, f64) {
    let system = NBodySystem::new(vec![1.0, 1.0]).expect("system");
    let omega = 2f64.sqrt();
    let half = 0.5;
    let state = NBodyState {
        positions: vec![Vec3::new(half, 0.0, 0.0), Vec3::new(-half, 0.0, 0.0)],
        velocities: vec![
            Vec3::new(0.0, omega * half, 0.0),
            Vec3::new(0.0, -omega * half, 0.0),
        ],
    };
    (system, state, 2.0 * PI / omega)
}

/// Energy with G = 1, where each body's mass is its μ: Σ ½ m v² − Σ_{i<j} m_i m_j / r_ij.
fn energy(system: &NBodySystem, state: &NBodyState) -> f64 {
    let m = system.mu();
    let mut e = 0.0;
    for (i, v) in state.velocities.iter().enumerate() {
        e += 0.5 * m[i] * v.dot(*v);
    }
    for i in 0..m.len() {
        for j in (i + 1)..m.len() {
            let d = state.positions[j] - state.positions[i];
            e -= m[i] * m[j] / d.norm();
        }
    }
    e
}

/// Total linear momentum Σ m v, with G = 1.
fn momentum(system: &NBodySystem, state: &NBodyState) -> Vec3 {
    let mut p = Vec3::new(0.0, 0.0, 0.0);
    for (m, v) in system.mu().iter().zip(state.velocities.iter()) {
        p = p + *v * *m;
    }
    p
}

#[test]
fn the_equilateral_accelerations_point_to_the_centre_with_magnitude_root_three() {
    let (system, state, _) = equilateral_triple();
    let acc = system.accelerations(&state.positions).expect("acc");
    let centre = Vec3::new(0.0, 0.0, 0.0);
    for (i, a) in acc.iter().enumerate() {
        assert!(
            (a.norm() - 3f64.sqrt()).abs() < 1e-12,
            "body {i}: |a| = {}",
            a.norm()
        );
        // Toward the centre: the component along the inward radial direction is the whole of it.
        let inward = (centre - state.positions[i]) * (1.0 / state.positions[i].norm());
        assert!(
            (a.dot(inward) - a.norm()).abs() < 1e-12,
            "body {i} not radial"
        );
    }
}

/// The largest distance any body is from its start after one period, with `steps` steps per period.
fn return_error(steps: f64) -> f64 {
    let (system, state, period) = equilateral_triple();
    let propagator = NBodyVerlet::new(system, Seconds::new(period / steps).expect("s")).expect("p");
    let after = propagator
        .propagate(&state, Seconds::new(period).expect("s"))
        .expect("propagate");
    after
        .positions
        .iter()
        .zip(state.positions.iter())
        .map(|(p, q)| (*p - *q).norm())
        .fold(0.0, f64::max)
}

#[test]
fn the_equilateral_triple_returns_to_its_start_and_the_error_falls_as_the_square_of_the_step() {
    // Second order: halving the step cuts the return error by about four.
    let coarse = return_error(1000.0);
    let fine = return_error(2000.0);
    assert!(fine < 1e-4, "error at 2000 steps per period: {fine} km");
    let ratio = coarse / fine;
    assert!(
        (3.5..4.5).contains(&ratio),
        "error ratio {ratio} (coarse {coarse} km, fine {fine} km)"
    );
}

#[test]
fn energy_error_is_bounded_and_momentum_is_conserved_over_ten_periods() {
    // Velocity Verlet on velocity-independent forces: the energy error oscillates within a bound and
    // does not grow secularly, and the pairwise forces cancel in the momentum sum, so the momentum is
    // conserved to rounding. The binary is stable, so the bound is the method's, not the orbit's.
    let (system, state, period) = circular_binary();
    let propagator =
        NBodyVerlet::new(system.clone(), Seconds::new(period / 1000.0).expect("s")).expect("p");
    let e0 = energy(&system, &state);
    let p0 = momentum(&system, &state);
    let mut current = state.clone();
    let mut first_period_peak = 0.0_f64;
    let mut last_period_peak = 0.0_f64;
    for k in 0..10 {
        // Sample through each period, not only at its end, so the peak is seen.
        for _ in 0..20 {
            current = propagator
                .propagate(&current, Seconds::new(period / 20.0).expect("s"))
                .expect("propagate");
            let err = ((energy(&system, &current) - e0) / e0).abs();
            if k == 0 {
                first_period_peak = first_period_peak.max(err);
            }
            if k == 9 {
                last_period_peak = last_period_peak.max(err);
            }
        }
    }
    assert!(
        first_period_peak < 1e-5,
        "first-period energy error {first_period_peak}"
    );
    assert!(
        last_period_peak < 1.5 * first_period_peak,
        "energy error grew: first period {first_period_peak}, last period {last_period_peak}"
    );
    let dp = momentum(&system, &current) - p0;
    assert!(dp.norm() < 1e-12, "momentum drift {}", dp.norm());
}

#[test]
fn a_test_particle_in_a_two_body_system_matches_the_single_body_integrator() {
    // The central body is pinned at the origin (its gravitational parameter is 1 and the particle's is
    // 1e-30, so it barely moves), and the particle's orbit is the single-body one.
    let mu_centre = 1.0;
    let system = NBodySystem::new(vec![mu_centre, 1e-30]).expect("system");
    let r0 = 7_000.0;
    let v0 = (mu_centre / r0).sqrt() * 1.1; // eccentric, so the test is not a circle
    let nbody_state = NBodyState {
        positions: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(r0, 0.0, 0.0)],
        velocities: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, v0, 0.0)],
    };
    let step = Seconds::new(5.0).expect("s");
    let duration = Seconds::new(3_000.0).expect("s");
    let nbody = NBodyVerlet::new(system, step)
        .expect("p")
        .propagate(&nbody_state, duration)
        .expect("nbody");

    let single = Verlet::new(CentralBody::new(mu_centre).expect("mu"), step).expect("p");
    let reference = single
        .propagate(
            &StateVector {
                position: Vec3::new(r0, 0.0, 0.0),
                velocity: Vec3::new(0.0, v0, 0.0),
            },
            duration,
        )
        .expect("single");

    let err = (nbody.positions[1] - reference.position).norm();
    assert!(
        err < 1e-9,
        "particle differs from the single-body result by {err} km"
    );
}

#[test]
fn invalid_systems_and_states_are_rejected() {
    assert_eq!(NBodySystem::new(vec![]), Err(Error::BodyCount));
    assert_eq!(NBodySystem::new(vec![1.0, 0.0]), Err(Error::InvalidMu));
    assert_eq!(NBodySystem::new(vec![f64::NAN]), Err(Error::InvalidMu));

    let system = NBodySystem::new(vec![1.0, 1.0]).expect("system");
    let short = NBodyState {
        positions: vec![Vec3::new(1.0, 0.0, 0.0)],
        velocities: vec![Vec3::new(0.0, 1.0, 0.0)],
    };
    assert_eq!(
        system.accelerations(&short.positions),
        Err(Error::BodyCount)
    );

    let coincident = NBodyState {
        positions: vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
        velocities: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)],
    };
    assert_eq!(
        system.accelerations(&coincident.positions),
        Err(Error::Degenerate)
    );

    assert_eq!(
        NBodyVerlet::new(system.clone(), Seconds::new(1.0).expect("s"))
            .expect("p")
            .propagate(&short, Seconds::new(1.0).expect("s")),
        Err(Error::BodyCount)
    );
    assert_eq!(
        NBodyVerlet::new(system.clone(), Seconds::new(0.0).expect("s")),
        Err(Error::InvalidStep)
    );
}
