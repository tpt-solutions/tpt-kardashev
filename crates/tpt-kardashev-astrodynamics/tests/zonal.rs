// SPDX-License-Identifier: MIT OR Apache-2.0
//! Zonal harmonics of any degree: agreement with `J2Zonal` for the J2 term, agreement with the
//! finite-difference gradient of the geopotential (the potential written with explicit Legendre
//! polynomials, so the recurrence used by the model is checked independently), the equatorial
//! closed form for J3, the parity of the odd harmonics, and rejection of invalid input.
//!
//! The J3 to J6 values here are test inputs of a typical size, not bundled constants: the checks
//! hold for any coefficients.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::force::{WGS84_J2, ZonalHarmonics};
use tpt_kardashev_astrodynamics::{Error, ForceModel, J2Zonal, Seconds, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
const R: f64 = 6_378.137;
const J3_TEST: f64 = -2.5e-6;
const J4_TEST: f64 = -1.6e-6;
const J5_TEST: f64 = 1.5e-7;
const J6_TEST: f64 = -5.0e-7;

fn at(p: Vec3, model: &impl ForceModel) -> Vec3 {
    model
        .acceleration(
            Seconds::new(0.0).expect("finite"),
            &StateVector {
                position: p,
                velocity: Vec3::new(0.0, 7.0, 0.0),
            },
        )
        .expect("finite acceleration")
}

/// Legendre polynomials P₂ to P₆ written out in closed form, independent of the recurrence.
fn legendre(n: usize, s: f64) -> f64 {
    match n {
        2 => 0.5 * (3.0 * s * s - 1.0),
        3 => 0.5 * (5.0 * s.powi(3) - 3.0 * s),
        4 => (35.0 * s.powi(4) - 30.0 * s * s + 3.0) / 8.0,
        5 => (63.0 * s.powi(5) - 70.0 * s.powi(3) + 15.0 * s) / 8.0,
        6 => (231.0 * s.powi(6) - 315.0 * s.powi(4) + 105.0 * s * s - 5.0) / 16.0,
        _ => unreachable!("degrees 2 to 6 only"),
    }
}

/// The perturbing potential, `−(μ/r)·Σ Jₙ (R/r)ⁿ Pₙ(z/r)`, whose gradient is the perturbation.
fn potential(p: Vec3, coefficients: &[f64]) -> f64 {
    let r = p.norm();
    let s = p.z / r;
    let q = R / r;
    let sum: f64 = coefficients
        .iter()
        .enumerate()
        .map(|(k, &j)| j * q.powi((k + 2) as i32) * legendre(k + 2, s))
        .sum();
    -(MU / r) * sum
}

fn gradient(p: Vec3, coefficients: &[f64]) -> Vec3 {
    let h = 0.5;
    let d =
        |dx: Vec3| (potential(p + dx, coefficients) - potential(p - dx, coefficients)) / (2.0 * h);
    Vec3::new(
        d(Vec3::new(h, 0.0, 0.0)),
        d(Vec3::new(0.0, h, 0.0)),
        d(Vec3::new(0.0, 0.0, h)),
    )
}

/// Positions off the axes and off the equator, at several radii and latitudes.
fn sample_points() -> Vec<Vec3> {
    vec![
        Vec3::new(7_000.0, 0.0, 0.0),
        Vec3::new(4_000.0, 3_000.0, 2_000.0),
        Vec3::new(-2_500.0, 5_100.0, -3_300.0),
        Vec3::new(1_000.0, -1_000.0, 6_000.0),
        Vec3::new(8_000.0, 1_500.0, 900.0),
        Vec3::new(-6_000.0, -6_000.0, -2_000.0),
    ]
}

#[test]
fn j2_term_matches_the_j2_model_exactly() {
    let zonal = ZonalHarmonics::new(MU, R, &[WGS84_J2]).expect("valid");
    let j2 = J2Zonal::new(MU, WGS84_J2, R).expect("valid");
    for p in sample_points() {
        let a = at(p, &zonal);
        let b = at(p, &j2);
        let diff = (a - b).norm();
        assert!(
            diff <= 1e-12 * b.norm(),
            "at {p:?}: zonal {a:?}, J2 {b:?}, diff {diff}"
        );
    }
}

#[test]
fn acceleration_is_the_gradient_of_the_geopotential_at_any_degree() {
    // The closed-form perturbation must match the finite-difference gradient of the potential it
    // is derived from, for several sets of coefficients up to degree 6.
    let sets: [&[f64]; 5] = [
        &[WGS84_J2, J3_TEST, J4_TEST],
        &[0.0, J3_TEST, 0.0],
        &[0.0, 0.0, J4_TEST],
        &[0.0, 0.0, 0.0, J5_TEST],
        &[WGS84_J2, J3_TEST, J4_TEST, J5_TEST, J6_TEST],
    ];
    for coefficients in sets {
        let model = ZonalHarmonics::new(MU, R, coefficients).expect("valid");
        for p in sample_points() {
            let a = at(p, &model);
            let g = gradient(p, coefficients);
            let diff = (a - g).norm();
            assert!(
                diff <= 1e-6 * a.norm(),
                "at {p:?} ({coefficients:?}): closed form {a:?}, gradient {g:?}, diff {diff}"
            );
        }
    }
}

#[test]
fn trailing_zero_coefficients_change_nothing() {
    let short = ZonalHarmonics::new(MU, R, &[WGS84_J2, J3_TEST]).expect("valid");
    let padded = ZonalHarmonics::new(MU, R, &[WGS84_J2, J3_TEST, 0.0, 0.0]).expect("valid");
    for p in sample_points() {
        assert_eq!(at(p, &short), at(p, &padded));
    }
}

#[test]
fn j3_on_the_equator_has_the_closed_form_axial_pull() {
    // In the equatorial plane at (r, 0, 0), the J3 term has no radial or along-track part, and
    // its axial component is (3/2)·μ·J3·R³/r⁵.
    let model = ZonalHarmonics::new(MU, R, &[0.0, J3_TEST]).expect("valid");
    for r in [7_000.0, 12_000.0, 42_164.0] {
        let a = at(Vec3::new(r, 0.0, 0.0), &model);
        let expected = 1.5 * MU * J3_TEST * R.powi(3) / r.powi(5);
        assert!(a.x.abs() <= 1e-15 * a.z.abs(), "radial part {}", a.x);
        assert!(
            (a.z - expected).abs() <= 1e-12 * expected.abs(),
            "axial {} vs {expected}",
            a.z
        );
    }
}

#[test]
fn odd_harmonics_flip_the_horizontal_part_under_reflection() {
    // J3 and J5 are odd in z: reflecting the position through the equator flips the horizontal
    // part of the acceleration and leaves the axial part unchanged.
    let model = ZonalHarmonics::new(MU, R, &[0.0, J3_TEST, 0.0, J5_TEST]).expect("valid");
    for p in sample_points() {
        let up = at(p, &model);
        let down = at(Vec3::new(p.x, p.y, -p.z), &model);
        let tol = 1e-12 * up.norm();
        assert!((up.x + down.x).abs() <= tol, "x: {} vs {}", up.x, down.x);
        assert!((up.y + down.y).abs() <= tol, "y: {} vs {}", up.y, down.y);
        assert!((up.z - down.z).abs() <= tol, "z: {} vs {}", up.z, down.z);
    }
}

#[test]
fn zero_or_empty_coefficients_give_no_perturbation() {
    for coefficients in [&[][..], &[0.0, 0.0, 0.0][..]] {
        let model = ZonalHarmonics::new(MU, R, coefficients).expect("valid");
        for p in sample_points() {
            assert_eq!(at(p, &model), Vec3::new(0.0, 0.0, 0.0));
        }
    }
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(ZonalHarmonics::new(0.0, R, &[]), Err(Error::InvalidMu));
    assert!(matches!(
        ZonalHarmonics::new(MU, 0.0, &[]),
        Err(Error::NonFinite { .. })
    ));
    assert!(matches!(
        ZonalHarmonics::new(MU, R, &[0.0, f64::NAN]),
        Err(Error::NonFinite {
            what: "zonal coefficient"
        })
    ));
    let model = ZonalHarmonics::new(MU, R, &[WGS84_J2, J3_TEST, J4_TEST]).expect("valid");
    let origin = StateVector {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.0, 0.0),
    };
    assert_eq!(
        model.acceleration(Seconds::new(0.0).expect("finite"), &origin),
        Err(Error::Degenerate)
    );
}
