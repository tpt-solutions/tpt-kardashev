// SPDX-License-Identifier: MIT OR Apache-2.0
//! The WGS 84 reference-ellipsoid zonal harmonics (EGM96 report, eq. 11.4-4). The equatorial acceleration
//! of the zonal field has a closed form, `a_x = μ/r² · Σₙ (n+1) Jₙ qⁿ Pₙ(0)` with `q = R/r`, that does
//! not use the crate's implementation; the set must agree with it, and the odd zonals must be zero.
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::force::{
    WGS84_EQUATORIAL_RADIUS_KM, WGS84_J2, WGS84_ZONAL_J2_TO_J10, ZonalHarmonics,
};
use tpt_kardashev_astrodynamics::{ForceModel, Seconds, StateVector, Vec3};

const MU: f64 = 398_600.441_8;

/// Legendre polynomials at the equator, `P₂(0)` through `P₁₀(0)`, for the even degrees.
const P_EVEN_AT_EQUATOR: [(usize, f64); 5] = [
    (2, -0.5),
    (4, 3.0 / 8.0),
    (6, -5.0 / 16.0),
    (8, 35.0 / 128.0),
    (10, -63.0 / 256.0),
];

#[test]
fn the_primary_source_values_are_the_constants() {
    assert_eq!(WGS84_J2, 1.082_629_821_31e-3);
    assert_eq!(WGS84_ZONAL_J2_TO_J10.len(), 9);
    // The reference ellipsoid is symmetric about the equator, so the odd zonals are zero.
    for (i, j) in WGS84_ZONAL_J2_TO_J10.iter().enumerate() {
        if (i + 2) % 2 == 1 {
            assert_eq!(*j, 0.0, "J{} must be zero", i + 2);
        }
    }
}

#[test]
fn the_equatorial_acceleration_matches_the_closed_form_series() {
    let r = 8_000.0;
    let zonal =
        ZonalHarmonics::new(MU, WGS84_EQUATORIAL_RADIUS_KM, &WGS84_ZONAL_J2_TO_J10).expect("valid");
    let state = StateVector {
        position: Vec3::new(r, 0.0, 0.0),
        velocity: Vec3::new(0.0, 0.0, 0.0),
    };
    let a = zonal
        .acceleration(Seconds::new(0.0).expect("time"), &state)
        .expect("acceleration");
    let q = WGS84_EQUATORIAL_RADIUS_KM / r;
    let series: f64 = P_EVEN_AT_EQUATOR
        .iter()
        .map(|&(n, pn0)| {
            let jn = WGS84_ZONAL_J2_TO_J10[n - 2];
            (n as f64 + 1.0) * jn * q.powi(n as i32) * pn0
        })
        .sum();
    let expected = MU / (r * r) * series;
    assert!(
        (a.x - expected).abs() <= 1e-12 * expected.abs(),
        "a_x = {}, closed form {expected}",
        a.x
    );
    assert!(
        a.y.abs() < 1e-18 && a.z.abs() < 1e-18,
        "no off-axis component at the equator"
    );
}
