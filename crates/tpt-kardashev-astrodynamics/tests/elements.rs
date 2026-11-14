// SPDX-License-Identifier: MIT OR Apache-2.0
//! Keplerian elements and state vectors: known values, invariants, round trips and rejection of
//! invalid input. Tolerances follow `docs/tolerance-policy.md`.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::PI;
use tpt_kardashev_astrodynamics::{Error, Keplerian, Kilometres, Radians, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as used throughout these tests.
const MU: f64 = 398_600.441_8;

/// Closed-form identities (policy: 1e-12 relative). Also used as the absolute bound for angles and
/// for eccentricities of circular orbits, which come out of cross products at round-off level
/// (about 1e-16); 1e-12 is 4 orders above that.
const CLOSED_REL: f64 = 1e-12;
const CLOSED_ABS: f64 = 1e-12;
/// Round trips through the conversions (iterative-solver policy: 1e-9 relative, 1e-9 absolute).
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

/// Compares two angles on the circle, so that 0 and 2π agree.
fn assert_angle(actual: f64, expected: f64, tol: f64) {
    let d = actual - expected;
    let diff = d.sin().atan2(d.cos()).abs();
    assert!(
        diff <= tol,
        "actual angle {actual}, expected {expected}, diff {diff} > {tol}"
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

fn elements(a: f64, e: f64, i: f64, raan: f64, argp: f64, nu: f64) -> Result<Keplerian, Error> {
    Keplerian::new(
        Kilometres::new(a)?,
        e,
        Radians::new(i)?,
        Radians::new(raan)?,
        Radians::new(argp)?,
        Radians::new(nu)?,
    )
}

#[test]
fn geo_circular_equatorial_known_values() {
    // A geostationary-radius circular orbit in the equatorial plane, at periapsis-like position on
    // the x axis with prograde velocity. Known: a = r, e = 0, i = 0, and the true longitude is 0.
    let a = 42_164.0;
    let state = StateVector {
        position: Vec3::new(a, 0.0, 0.0),
        velocity: Vec3::new(0.0, (MU / a).sqrt(), 0.0),
    };
    let el = Keplerian::from_state(&state, MU).expect("valid state");
    assert_close(el.semi_major_axis().get(), a, CLOSED_REL, CLOSED_ABS);
    assert_close(el.eccentricity(), 0.0, CLOSED_REL, CLOSED_ABS);
    assert_close(el.inclination().get(), 0.0, CLOSED_REL, CLOSED_ABS);
    assert_angle(el.true_anomaly().get(), 0.0, CLOSED_ABS);
}

#[test]
fn inclined_circular_orbit_known_values() {
    // Circular at r = 7000 km with inclination 0.5 rad: the velocity is tilted out of the x–y
    // plane by i. The node is along +x, so Ω = 0; the argument of latitude is 0 at this position.
    let inc: f64 = 0.5;
    let r = 7_000.0;
    let v = (MU / r).sqrt();
    let state = StateVector {
        position: Vec3::new(r, 0.0, 0.0),
        velocity: Vec3::new(0.0, v * inc.cos(), v * inc.sin()),
    };
    let el = Keplerian::from_state(&state, MU).expect("valid state");
    assert_close(el.semi_major_axis().get(), r, CLOSED_REL, CLOSED_ABS);
    assert_close(el.eccentricity(), 0.0, CLOSED_REL, CLOSED_ABS);
    assert_angle(el.inclination().get(), inc, CLOSED_ABS);
    assert_angle(el.raan().get(), 0.0, CLOSED_ABS);
    assert_angle(el.true_anomaly().get(), 0.0, CLOSED_ABS);
}

#[test]
fn angular_momentum_and_energy_follow_the_elements() {
    // |h| = sqrt(μ·p) with p = a(1 − e²), and the energy is −μ/(2a). Both hold for any elliptic
    // state, so this checks the conversion against two independent identities.
    let state = StateVector {
        position: Vec3::new(6_800.0, 1_200.0, -500.0),
        velocity: Vec3::new(-1.0, 6.9, 2.3),
    };
    let el = Keplerian::from_state(&state, MU).expect("elliptic state");
    let a = el.semi_major_axis().get();
    let e = el.eccentricity();
    let h = state.position.cross(state.velocity).norm();
    assert_close(h, (MU * a * (1.0 - e * e)).sqrt(), ROUND_REL, ROUND_ABS);
    let energy = 0.5 * state.velocity.dot(state.velocity) - MU / state.position.norm();
    assert_close(energy, -MU / (2.0 * a), ROUND_REL, ROUND_ABS);
}

#[test]
fn states_round_trip_through_elements_including_singular_cases() {
    // Element grid spanning circular (e = 0), nearly circular, moderate and high eccentricity; and
    // equatorial, prograde, polar, retrograde-ish and retrograde-equatorial inclinations. Each
    // state is compared, not each element: at singular cases one state has several element sets.
    let mut count = 0;
    for &a in &[7_000.0, 26_560.0, 42_164.0] {
        for &e in &[0.0, 1e-3, 0.1, 0.7] {
            for &i in &[0.0, 0.3, PI / 2.0, 2.5, PI] {
                for &raan in &[0.0, 1.1, 5.0] {
                    for &argp in &[0.0, 2.0, 4.5] {
                        for &nu in &[0.0, 1.0, 3.5, 6.0] {
                            let el = elements(a, e, i, raan, argp, nu).expect("valid elements");
                            let s1 = el.to_state(MU).expect("state");
                            let back = Keplerian::from_state(&s1, MU).expect("elements");
                            let s2 = back.to_state(MU).expect("state");
                            assert_state_close(&s2, &s1, ROUND_REL);
                            assert_close(back.semi_major_axis().get(), a, ROUND_REL, ROUND_ABS);
                            assert_close(back.eccentricity(), e, ROUND_REL, ROUND_ABS);
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(count, 3 * 4 * 5 * 3 * 3 * 4);
}

#[test]
fn nonsingular_elements_round_trip_exactly() {
    // Away from the singular cases the elements themselves come back, with their angles.
    let cases = [
        (7_000.0, 0.1, 0.7, 1.1, 2.0, 0.5),
        (26_560.0, 0.4, 2.2, 4.0, 5.5, 3.0),
        (42_164.0, 0.001, 0.05, 0.2, 1.0, 5.9),
    ];
    for (a, e, i, raan, argp, nu) in cases {
        let el = elements(a, e, i, raan, argp, nu).expect("valid elements");
        let state = el.to_state(MU).expect("state");
        let back = Keplerian::from_state(&state, MU).expect("elements");
        assert_close(back.semi_major_axis().get(), a, ROUND_REL, ROUND_ABS);
        assert_close(back.eccentricity(), e, ROUND_REL, ROUND_ABS);
        assert_angle(back.inclination().get(), i, ROUND_ABS);
        assert_angle(back.raan().get(), raan, ROUND_ABS);
        assert_angle(back.arg_periapsis().get(), argp, ROUND_ABS);
        assert_angle(back.true_anomaly().get(), nu, ROUND_ABS);
    }
}

#[test]
fn hyperbolic_state_is_rejected() {
    // Speed 12 km/s at 7000 km is above escape speed (about 10.7 km/s), so the energy is positive.
    let state = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 12.0, 0.0),
    };
    assert_eq!(Keplerian::from_state(&state, MU), Err(Error::NotElliptic));
}

#[test]
fn parallel_position_and_velocity_are_degenerate() {
    // Radial motion has no angular momentum, so there is no orbital plane.
    let state = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(1.0, 0.0, 0.0),
    };
    assert_eq!(Keplerian::from_state(&state, MU), Err(Error::Degenerate));
}

#[test]
fn invalid_inputs_are_rejected() {
    assert!(matches!(
        Kilometres::new(f64::NAN),
        Err(Error::NonFinite { .. })
    ));
    assert_eq!(
        elements(7_000.0, 1.0, 0.0, 0.0, 0.0, 0.0),
        Err(Error::NotElliptic)
    );
    assert_eq!(
        elements(0.0, 0.1, 0.0, 0.0, 0.0, 0.0),
        Err(Error::NotElliptic)
    );
    assert_eq!(
        elements(7_000.0, -0.1, 0.0, 0.0, 0.0, 0.0),
        Err(Error::NotElliptic)
    );
    assert!(matches!(
        Keplerian::new(
            Kilometres::new(7_000.0).expect("finite"),
            f64::NAN,
            Radians::new(0.0).expect("finite"),
            Radians::new(0.0).expect("finite"),
            Radians::new(0.0).expect("finite"),
            Radians::new(0.0).expect("finite"),
        ),
        Err(Error::NonFinite { .. })
    ));
    let state = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert_eq!(Keplerian::from_state(&state, 0.0), Err(Error::InvalidMu));
    assert_eq!(
        Keplerian::from_state(&state, f64::NAN),
        Err(Error::InvalidMu)
    );
    let nan_state = StateVector {
        position: Vec3::new(f64::NAN, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert!(matches!(
        Keplerian::from_state(&nan_state, MU),
        Err(Error::NonFinite { .. })
    ));
    let el = elements(7_000.0, 0.1, 0.0, 0.0, 0.0, 0.0).expect("valid elements");
    assert_eq!(el.to_state(-1.0), Err(Error::InvalidMu));
}
