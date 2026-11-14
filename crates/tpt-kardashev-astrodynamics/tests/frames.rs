// SPDX-License-Identifier: MIT OR Apache-2.0
//! Perifocal and RTN frames: orthonormality, handedness, closed-form components, and the
//! circular-orbit property that velocity lies along the transverse axis.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::frames::{
    OBLIQUITY_J2000_ARCSEC, ecliptic_to_equatorial, rtn_matrix,
};
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

/// The inverse of `ecliptic_to_equatorial`, written here (the crate does not expose it): a rotation
/// about x by −ε.
fn equatorial_to_ecliptic(v: Vec3) -> Vec3 {
    let eps = (OBLIQUITY_J2000_ARCSEC / 3600.0).to_radians();
    let (s, c) = eps.sin_cos();
    Vec3::new(v.x, c * v.y + s * v.z, -s * v.y + c * v.z)
}

#[test]
fn ecliptic_to_equatorial_round_trips_and_keeps_length() {
    let samples = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.3, -0.8, 0.52),
        Vec3::new(-7_000.0, 1_234.5, -9_876.5),
    ];
    for v in samples {
        let eq = ecliptic_to_equatorial(v);
        assert_close(eq.norm(), v.norm(), CLOSED_REL, CLOSED_ABS);
        assert_vec_close(equatorial_to_ecliptic(eq), v, ROUND_REL, ROUND_ABS);
    }
}

#[test]
fn ecliptic_x_axis_is_unchanged_and_the_pole_is_tilted() {
    // The rotation is about x, so the vernal equinox direction does not move.
    let x = ecliptic_to_equatorial(Vec3::new(1.0, 0.0, 0.0));
    assert_vec_close(x, Vec3::new(1.0, 0.0, 0.0), CLOSED_REL, CLOSED_ABS);
    // The ecliptic pole sits at the obliquity from the equatorial pole.
    let eps = (OBLIQUITY_J2000_ARCSEC / 3600.0).to_radians();
    let pole = ecliptic_to_equatorial(Vec3::new(0.0, 0.0, 1.0));
    assert_vec_close(
        pole,
        Vec3::new(0.0, -eps.sin(), eps.cos()),
        CLOSED_REL,
        CLOSED_ABS,
    );
}

#[test]
fn rtn_components_map_back_to_the_inertial_state() {
    // Over a grid of element sets, the RTN components of a state, mapped back by the transpose,
    // reproduce its position and velocity.
    let mut count = 0;
    for a in [6_778.0, 7_000.0, 26_560.0, 42_164.0] {
        for e in [0.0, 0.05, 0.3, 0.6] {
            for i in [0.0, 0.6, 1.4, 2.6] {
                for nu in [0.1, 2.0, 4.5] {
                    let state = elements(a, e, i, 1.1, 0.7, nu).to_state(MU).expect("state");
                    let m = rtn_matrix(&state).expect("non-degenerate");
                    let components = m.mul_vec(state.velocity);
                    let back = m.transpose().mul_vec(components);
                    assert_vec_close(back, state.velocity, ROUND_REL, ROUND_ABS);
                    let back_r = m.transpose().mul_vec(m.mul_vec(state.position));
                    assert_vec_close(back_r, state.position, ROUND_REL, ROUND_ABS);
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 192);
}

#[test]
fn rtn_normal_axis_is_the_perifocal_w_axis() {
    // The normal axis of RTN is the orbit normal, which the perifocal frame calls W.
    for (e, i, raan, argp, nu) in [(0.1, 0.6, 1.0, 2.0, 0.7), (0.4, 2.2, 4.0, 5.5, 3.0)] {
        let el = elements(26_560.0, e, i, raan, argp, nu);
        let state = el.to_state(MU).expect("state");
        let rtn = rtn_matrix(&state).expect("non-degenerate");
        let w = el.perifocal_to_inertial().transpose().rows[2];
        assert_vec_close(
            Vec3::new(rtn.rows[2][0], rtn.rows[2][1], rtn.rows[2][2]),
            Vec3::new(w[0], w[1], w[2]),
            ROUND_REL,
            ROUND_ABS,
        );
    }
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

#[test]
fn lvlh_axes_follow_the_documented_convention() {
    use tpt_kardashev_astrodynamics::frames::lvlh_matrix;
    let el = elements(7_000.0, 0.2, 0.9, 0.4, 0.3, 1.7);
    let state = el.to_state(MU).expect("state");
    let lvlh = lvlh_matrix(&state).expect("non-degenerate");
    let rtn = rtn_matrix(&state).expect("non-degenerate");
    // Orthonormal and right-handed.
    assert_identity(lvlh * lvlh.transpose(), CLOSED_ABS);
    assert_close(lvlh.determinant(), 1.0, CLOSED_REL, CLOSED_ABS);
    // z points to nadir: the position is along −z, and the position has no x or y component.
    let r = state.position;
    let in_lvlh = lvlh.mul_vec(r);
    assert_close(in_lvlh.x, 0.0, CLOSED_REL, ROUND_ABS);
    assert_close(in_lvlh.y, 0.0, CLOSED_REL, ROUND_ABS);
    assert_close(in_lvlh.z, -r.norm(), ROUND_REL, ROUND_ABS);
    // The LVLH components are the RTN components of (T, −N, −R).
    let v_rtn = rtn.mul_vec(state.velocity);
    let v_lvlh = lvlh.mul_vec(state.velocity);
    assert_close(v_lvlh.x, v_rtn.y, ROUND_REL, ROUND_ABS);
    assert_close(v_lvlh.y, -v_rtn.z, ROUND_REL, ROUND_ABS);
    assert_close(v_lvlh.z, -v_rtn.x, ROUND_REL, ROUND_ABS);
    // y is the negative orbit normal.
    let h = state.position.cross(state.velocity);
    let y_axis = Vec3::new(lvlh.rows[1][0], lvlh.rows[1][1], lvlh.rows[1][2]);
    assert_vec_close(y_axis, h * (-1.0 / h.norm()), ROUND_REL, ROUND_ABS);
}

#[test]
fn circular_orbit_along_track_axis_is_the_velocity_direction() {
    use tpt_kardashev_astrodynamics::frames::lvlh_matrix;
    let el = elements(7_000.0, 0.0, 0.9, 0.4, 0.0, 2.2);
    let state = el.to_state(MU).expect("state");
    let lvlh = lvlh_matrix(&state).expect("non-degenerate");
    let along = Vec3::new(lvlh.rows[0][0], lvlh.rows[0][1], lvlh.rows[0][2]);
    let unit_velocity = state.velocity * (1.0 / state.velocity.norm());
    assert_vec_close(along, unit_velocity, ROUND_REL, ROUND_ABS);
}

#[test]
fn lvlh_rejects_degenerate_states() {
    use tpt_kardashev_astrodynamics::frames::lvlh_matrix;
    let radial = StateVector {
        position: Vec3::new(7_000.0, 0.0, 0.0),
        velocity: Vec3::new(1.0, 0.0, 0.0),
    };
    assert_eq!(lvlh_matrix(&radial), Err(Error::Degenerate));
}
