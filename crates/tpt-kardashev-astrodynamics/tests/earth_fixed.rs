// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Earth-fixed position transform, given the Earth rotation angle: a rotation about z, which
//! preserves length and the z component, and agrees with the visibility module's convention.
#![cfg(feature = "std")]

use tpt_kardashev_astrodynamics::frames::inertial_to_earth_fixed;
use tpt_kardashev_astrodynamics::visibility::EarthRotation;
use tpt_kardashev_astrodynamics::{Radians, Vec3};

fn close(a: Vec3, b: Vec3) -> bool {
    (a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12 && (a.z - b.z).abs() < 1e-12
}

#[test]
fn a_quarter_turn_moves_the_x_axis_to_minus_y() {
    let r = inertial_to_earth_fixed(Vec3::new(7000.0, 0.0, 0.0), std::f64::consts::FRAC_PI_2);
    assert!(close(r, Vec3::new(0.0, -7000.0, 0.0)), "got {r:?}");
}

#[test]
fn zero_angle_is_the_identity_and_z_is_unchanged() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    assert!(close(inertial_to_earth_fixed(v, 0.0), v));
    assert_eq!(inertial_to_earth_fixed(v, 1.234).z, 3.0);
}

#[test]
fn length_is_preserved() {
    let v = Vec3::new(3.0e3, -4.0e3, 1.2e3);
    let r = inertial_to_earth_fixed(v, 2.7);
    let n0 = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    let n1 = (r.x * r.x + r.y * r.y + r.z * r.z).sqrt();
    assert!((n0 - n1).abs() < 1e-9 * n0);
}

#[test]
fn agrees_with_the_visibility_rotation_at_the_start_instant() {
    // `EarthRotation::to_inertial` at t = 0 maps Earth-fixed to inertial. Its inverse is this
    // transform, so the two round-trip.
    let angle = 0.8;
    let rot = EarthRotation::new(Radians::new(angle).expect("finite"));
    let inertial = Vec3::new(5000.0, 1000.0, -2000.0);
    let fixed = inertial_to_earth_fixed(inertial, angle);
    let back = rot.to_inertial(0.0, fixed);
    assert!(close(back, inertial), "got {back:?}");
}
