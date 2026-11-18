// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cross-check of the drag force against the old closed-form decay rate from `tpt-sci-astro`
//! (`OrbitalElements::drag_da_dt`, Vallado 4th ed. §8.6.3, King-Hele 1964):
//!
//! `da/dt = -(rho * Cd*A/m) * n * a^2 * sqrt((1+e)/(1-e))`
//!
//! That formula takes the speed relative to the air to be the inertial speed. This crate's force
//! uses the speed relative to the rotating atmosphere, `v - omega x r`. For a circular, equatorial,
//! prograde orbit the two differ by an exact factor, so the cross-check tests that factor rather
//! than a tolerance: the force model's decay rate, divided by the old formula's rate at the same
//! density, equals `(1 - omega r / v)^2`.
//!
//! The density is this crate's table in both cases, so the comparison isolates the drag law.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::atmosphere::density;
use tpt_kardashev_astrodynamics::force::{
    AtmosphericDrag, EARTH_ROTATION_RATE_RAD_S, WGS84_EQUATORIAL_RADIUS_KM,
};
use tpt_kardashev_astrodynamics::{ForceModel, Seconds, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
const CD: f64 = 2.2;
const A_OVER_M: f64 = 0.01;

/// Relative agreement with the exact factor. Round-off through a handful of operations.
const REL: f64 = 1e-9;

/// The old closed form, in km/s, for a circular orbit (eccentricity zero, so the eccentricity
/// factor is one). `density_km` is in kg/m³, `b` is Cd·A/m in m²/kg, `a` is in km.
fn old_closed_form_da_dt(a: f64, density_kg_m3: f64, b: f64) -> f64 {
    let n = (MU / (a * a * a)).sqrt();
    let a_m = a * 1000.0;
    -density_kg_m3 * b * n * a_m * a_m / 1000.0
}

#[test]
fn force_decay_is_the_old_formula_times_the_rotation_factor_squared() {
    // A circular equatorial prograde orbit about 500 km up.
    let r = 6_878.0;
    let v = (MU / r).sqrt();
    let state = StateVector {
        position: Vec3::new(r, 0.0, 0.0),
        velocity: Vec3::new(0.0, v, 0.0),
    };
    let drag = AtmosphericDrag::new(CD, A_OVER_M).expect("valid");
    let a_d = drag
        .acceleration(Seconds::new(0.0).expect("finite"), &state)
        .expect("acceleration");

    // Secular decay from the force: da/dt = (2 a^2 / mu) * (a_d . v), the rate of change of
    // -mu/(2a) per unit mass.
    let power = a_d.dot(state.velocity);
    let da_dt_force = 2.0 * r * r / MU * power;

    let altitude = r - WGS84_EQUATORIAL_RADIUS_KM;
    let da_dt_old = old_closed_form_da_dt(r, density(altitude), CD * A_OVER_M);

    let omega_r_over_v = EARTH_ROTATION_RATE_RAD_S * r / v;
    let expected_ratio = (1.0 - omega_r_over_v).powi(2);
    let ratio = da_dt_force / da_dt_old;

    assert!(da_dt_force < 0.0, "drag must lower the orbit");
    assert!(
        (ratio - expected_ratio).abs() <= REL * expected_ratio,
        "ratio {ratio}, expected (1 - omega r/v)^2 = {expected_ratio}"
    );
    // The rotation makes the air move with the spacecraft, so the force model is the smaller one.
    assert!(ratio < 1.0);
}

#[test]
fn old_formula_overstates_the_decay_by_about_fifteen_percent_at_five_hundred_km() {
    // The old formula's rate divided by the force model's is 1/(1 - x)^2, with x = omega r / v.
    // At about 500 km, x is near 0.066, so the old formula overstates the decay by about 15 %.
    let r = 6_878.0;
    let v = (MU / r).sqrt();
    let x = EARTH_ROTATION_RATE_RAD_S * r / v;
    let overstatement = 1.0 / (1.0 - x).powi(2) - 1.0;
    assert!(
        (0.14..0.16).contains(&overstatement),
        "overstatement {overstatement}"
    );
}
