// SPDX-License-Identifier: MIT OR Apache-2.0
//! Zonal harmonics of degrees 2 to 4: agreement with `J2Zonal` for the J2 term, agreement with the
//! finite-difference gradient of the geopotential, the equatorial-plane closed form for J3, the
//! symmetry of the odd harmonic, and rejection of invalid input.
//!
//! The J3 and J4 values here are test inputs of a typical size, not bundled constants: the checks
//! hold for any coefficients.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::force::WGS84_J2;
use tpt_kardashev_astrodynamics::force::ZonalHarmonics;
use tpt_kardashev_astrodynamics::{Error, ForceModel, J2Zonal, Seconds, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
const R: f64 = 6_378.137;
const J3_TEST: f64 = -2.5e-6;
const J4_TEST: f64 = -1.6e-6;

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

/// The geopotential term of the zonal harmonics, `−(μ/r)·Σ Jₙ (R/r)ⁿ Pₙ(z/r)`, whose gradient is
/// the perturbation.
fn potential(p: Vec3, j2: f64, j3: f64, j4: f64) -> f64 {
    let r = p.norm();
    let s = p.z / r;
    let q = R / r;
    let p2 = 0.5 * (3.0 * s * s - 1.0);
    let p3 = 0.5 * (5.0 * s * s * s - 3.0 * s);
    let p4 = (35.0 * s.powi(4) - 30.0 * s * s + 3.0) / 8.0;
    -(MU / r) * (j2 * q * q * p2 + j3 * q.powi(3) * p3 + j4 * q.powi(4) * p4)
}

fn gradient(p: Vec3, j2: f64, j3: f64, j4: f64) -> Vec3 {
    let h = 0.5;
    let d = |dx: Vec3| (potential(p + dx, j2, j3, j4) - potential(p - dx, j2, j3, j4)) / (2.0 * h);
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
    let zonal = ZonalHarmonics::new(MU, R, WGS84_J2, 0.0, 0.0).expect("valid");
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
fn acceleration_is_the_gradient_of_the_geopotential() {
    // The closed-form perturbation must match the finite-difference gradient of the potential it
    // is derived from, for all three harmonics together and for J3 and J4 alone.
    for (j2, j3, j4) in [
        (WGS84_J2, J3_TEST, J4_TEST),
        (0.0, J3_TEST, 0.0),
        (0.0, 0.0, J4_TEST),
    ] {
        let model = ZonalHarmonics::new(MU, R, j2, j3, j4).expect("valid");
        for p in sample_points() {
            // The acceleration is the gradient of the perturbing potential U_J = −(μ/r)ΣJₙ(R/r)ⁿPₙ.
            let a = at(p, &model);
            let g = gradient(p, j2, j3, j4);
            let diff = (a - g).norm();
            assert!(
                diff <= 1e-6 * a.norm(),
                "at {p:?} (J {j2}, {j3}, {j4}): closed form {a:?}, gradient {g:?}, diff {diff}"
            );
        }
    }
}

#[test]
fn j3_on_the_equator_has_the_closed_form_axial_pull() {
    // In the equatorial plane at (r, 0, 0), the J3 term has no radial or along-track part, and
    // its axial component is (3/2)·μ·J3·R³/r⁵.
    let model = ZonalHarmonics::new(MU, R, 0.0, J3_TEST, 0.0).expect("valid");
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
fn odd_harmonic_flips_the_radial_part_under_reflection() {
    // J3 is odd in z: reflecting the position through the equator flips the horizontal part of
    // the acceleration and leaves the axial part unchanged.
    let model = ZonalHarmonics::new(MU, R, 0.0, J3_TEST, 0.0).expect("valid");
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
fn zero_coefficients_give_no_perturbation() {
    let model = ZonalHarmonics::new(MU, R, 0.0, 0.0, 0.0).expect("valid");
    for p in sample_points() {
        assert_eq!(at(p, &model), Vec3::new(0.0, 0.0, 0.0));
    }
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(
        ZonalHarmonics::new(0.0, R, 0.0, 0.0, 0.0),
        Err(Error::InvalidMu)
    );
    assert!(matches!(
        ZonalHarmonics::new(MU, 0.0, 0.0, 0.0, 0.0),
        Err(Error::NonFinite { .. })
    ));
    assert!(matches!(
        ZonalHarmonics::new(MU, R, 0.0, f64::NAN, 0.0),
        Err(Error::NonFinite { what: "J3" })
    ));
    let model = ZonalHarmonics::new(MU, R, WGS84_J2, J3_TEST, J4_TEST).expect("valid");
    let origin = StateVector {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.0, 0.0),
    };
    assert_eq!(
        model.acceleration(Seconds::new(0.0).expect("finite"), &origin),
        Err(Error::Degenerate)
    );
}
