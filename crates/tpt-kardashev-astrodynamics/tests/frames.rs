// SPDX-License-Identifier: MIT OR Apache-2.0
//! Perifocal and RTN frames: orthonormality, handedness, closed-form components, and the
//! circular-orbit property that velocity lies along the transverse axis.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::frames::rtn_matrix;
use tpt_kardashev_astrodynamics::{Error, Keplerian, Kilometres, Mat3, Radians, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the element tests.
const MU: f64 = 398_600.441_8;

/// Closed-form identities (policy: 1e-12 relative). Round-off through a few rotations is well
/// inside this for these magnitudes; the absolute bound covers entries that are zero in exact
/// arithmetic.
const CLOSED_REL: f64 = 1e-12;
const CLOSED_ABS: f64 = 1e-12;
/// Round trips through the element conversions and rotations (1e-9, as in `elements.rs`).
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

fn assert_vec_close(actual: Vec3, expected: Vec3, rel: f64, abs: f64) {
    assert_close(actual.x, expected.x, rel, abs);
    assert_close(actual.y, expected.y, rel, abs);
    assert_close(actual.z, expected.z, rel, abs);
}

fn assert_identity(m: Mat3, tol: f64) {
    for (i, row) in m.rows.iter().enumerate() {
        for (j, &value) in row.iter().enumerate() {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!((value - expected).abs() <= tol, "entry ({i},{j}) = {value}");
        }
    }
}

fn elements(a: f64, e: f64, i: f64, raan: f64, argp: f64, nu: f64) -> Keplerian {
    Keplerian::new(
        Kilometres::new(a).expect("finite"),
        e,
        Radians::new(i).expect("finite"),
        Radians::new(raan).expect("finite"),
        Radians::new(argp).expect("finite"),
        Radians::new(nu).expect("finite"),
    )
    .expect("elliptic")
}

#[test]
fn identity_and_transpose_behave() {
    assert_identity(Mat3::IDENTITY, 0.0);
    assert_close(Mat3::IDENTITY.determinant(), 1.0, CLOSED_REL, CLOSED_ABS);
    let m = Mat3::from_rows([[1.0, 2.0, 0.0], [0.0, 1.0, 0.0], [4.0, 0.0, 1.0]]);
    assert_eq!(m * Mat3::IDENTITY, m);
    assert_eq!(m.transpose().transpose(), m);
}

#[test]
fn perifocal_rotation_is_a_proper_rotation() {
    for el in [
        elements(7_000.0, 0.1, 0.6, 1.0, 2.0, 0.7),
        elements(26_560.0, 0.4, 2.2, 4.0, 5.5, 3.0),
        elements(42_164.0, 0.0, PI_OVER_2, 0.0, 0.0, 0.0),
    ] {
        let m = el.perifocal_to_inertial();
        assert_identity(m * m.transpose(), CLOSED_ABS);
        assert_close(m.determinant(), 1.0, CLOSED_REL, CLOSED_ABS);
    }
}

const PI_OVER_2: f64 = core::f64::consts::FRAC_PI_2;

#[test]
fn perifocal_components_of_a_state_match_the_closed_form() {
    // In the perifocal frame the position is (r cos ν, r sin ν, 0) with r = p / (1 + e cos ν).
    let (a, e, nu) = (7_000.0, 0.1, 0.7);
    let el = elements(a, e, 0.6, 1.0, 2.0, nu);
    let state = el.to_state(MU).expect("state");
    let p = a * (1.0 - e * e);
    let r = p / (1.0 + e * nu.cos());
    let back = el
        .perifocal_to_inertial()
        .transpose()
        .mul_vec(state.position);
    assert_vec_close(
        back,
        Vec3::new(r * nu.cos(), r * nu.sin(), 0.0),
        ROUND_REL,
        ROUND_ABS,
    );
}

#[test]
fn rtn_axes_are_orthonormal_and_right_handed() {
    let state = StateVector {
        position: Vec3::new(6_800.0, 1_200.0, -500.0),
        velocity: Vec3::new(-1.0, 6.9, 2.3),
    };
    let m = rtn_matrix(&state).expect("non-degenerate");
    assert_identity(m * m.transpose(), CLOSED_ABS);
    assert_close(m.determinant(), 1.0, CLOSED_REL, CLOSED_ABS);
}

#[test]
fn rtn_components_of_position_and_velocity() {
    let state = StateVector {
        position: Vec3::new(6_800.0, 1_200.0, -500.0),
        velocity: Vec3::new(-1.0, 6.9, 2.3),
    };
    let m = rtn_matrix(&state).expect("non-degenerate");
    let r_norm = state.position.norm();
    // Position is purely radial.
    assert_vec_close(
        m.mul_vec(state.position),
        Vec3::new(r_norm, 0.0, 0.0),
        CLOSED_REL,
        ROUND_ABS,
    );
    // Velocity has no normal component, since it lies in the orbit plane.
    let v_rtn = m.mul_vec(state.velocity);
    assert_close(v_rtn.z, 0.0, CLOSED_REL, ROUND_ABS);
    assert_close(
        v_rtn.x,
        state.velocity.dot(state.position) / r_norm,
        ROUND_REL,
        ROUND_ABS,
    );
}

#[test]
fn circular_orbit_velocity_is_purely_transverse() {
    // A circular orbit has no radial velocity: all of v lies along T.
    let el = elements(7_000.0, 0.0, 0.9, 0.4, 0.0, 2.2);
    let state = el.to_state(MU).expect("state");
    let v = state.velocity.norm();
    let v_rtn = rtn_matrix(&state)
        .expect("non-degenerate")
        .mul_vec(state.velocity);
    assert_close(v_rtn.x, 0.0, ROUND_REL, ROUND_ABS);
    assert_close(v_rtn.y, v, ROUND_REL, ROUND_ABS);
    assert_close(v_rtn.z, 0.0, ROUND_REL, ROUND_ABS);
}

#[test]
fn rtn_rejects_degenerate_and_non_finite_states() {
    let radial = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(1.0, 0.0, 0.0),
    };
    assert_eq!(rtn_matrix(&radial), Err(Error::Degenerate));
    let nan = StateVector {
        position: Vec3::new(f64::NAN, 0.0, 0.0),
        velocity: Vec3::new(0.0, 7.5, 0.0),
    };
    assert!(matches!(rtn_matrix(&nan), Err(Error::NonFinite { .. })));
}
