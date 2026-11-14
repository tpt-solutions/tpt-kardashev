// SPDX-License-Identifier: MIT OR Apache-2.0
//! Equinoctial elements: closed-form values, state round trips over a grid that includes the
//! singular Keplerian cases, regularity near the equatorial plane, and rejection of invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::TAU;
use tpt_kardashev_astrodynamics::equinoctial::Equinoctial;
use tpt_kardashev_astrodynamics::{Error, Keplerian, Kilometres, Radians, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
const CLOSED_REL: f64 = 1e-12;
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

fn assert_vec_close(actual: Vec3, expected: Vec3, rel: f64, abs: f64) {
    assert_close(actual.x, expected.x, rel, abs);
    assert_close(actual.y, expected.y, rel, abs);
    assert_close(actual.z, expected.z, rel, abs);
}

#[test]
fn elements_have_the_closed_form_values() {
    let (a, e, i, raan, argp, nu) = (7_000.0, 0.1, 0.6, 1.0, 2.0, 0.7);
    let eq = Equinoctial::from_keplerian(&elements(a, e, i, raan, argp, nu)).expect("valid");
    let (f, g) = eq.eccentricity_components();
    let (h, k) = eq.inclination_components();
    assert_close(eq.semi_latus_rectum(), a * (1.0 - e * e), CLOSED_REL, 0.0);
    assert_close(f, e * (raan + argp).cos(), CLOSED_REL, 1e-15);
    assert_close(g, e * (raan + argp).sin(), CLOSED_REL, 1e-15);
    assert_close(h, (0.5 * i).tan() * raan.cos(), CLOSED_REL, 1e-15);
    assert_close(k, (0.5 * i).tan() * raan.sin(), CLOSED_REL, 1e-15);
    assert_close(
        eq.true_longitude(),
        (raan + argp + nu).rem_euclid(TAU),
        CLOSED_REL,
        1e-15,
    );
}

#[test]
fn state_round_trips_over_a_grid_including_the_singular_cases() {
    // Circular, equatorial, circular-equatorial, polar and retrograde orbits are all in the grid.
    let mut count = 0;
    for a in [6_778.0, 7_000.0, 42_164.0] {
        for e in [0.0, 0.05, 0.3, 0.6] {
            for i in [0.0, 0.6, 1.57, 2.6] {
                for nu in [0.1, 2.0, 4.5] {
                    let el = elements(a, e, i, 1.1, 0.7, nu);
                    let state = el.to_state(MU).expect("state");
                    let eq = Equinoctial::from_state(&state, MU).expect("equinoctial");
                    let back: StateVector = eq.to_state(MU).expect("state");
                    assert_vec_close(back.position, state.position, ROUND_REL, ROUND_ABS);
                    assert_vec_close(back.velocity, state.velocity, ROUND_REL, ROUND_ABS);
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 144);
}

#[test]
fn a_circular_equatorial_orbit_has_zero_eccentricity_and_inclination_terms() {
    // With f = g = h = k = 0, the true longitude is the only angle left: here 0.4 + 2.2 + 0.0.
    let el = elements(7_000.0, 0.0, 0.0, 0.0, 0.0, 2.2);
    let eq = Equinoctial::from_keplerian(&el).expect("valid");
    let (f, g) = eq.eccentricity_components();
    let (h, k) = eq.inclination_components();
    assert_close(f, 0.0, 0.0, 1e-15);
    assert_close(g, 0.0, 0.0, 1e-15);
    assert_close(h, 0.0, 0.0, 1e-15);
    assert_close(k, 0.0, 0.0, 1e-15);
    assert_close(eq.true_longitude(), 2.2, CLOSED_REL, 1e-15);
}

#[test]
fn the_elements_are_regular_across_the_equatorial_plane() {
    // The Keplerian node jumps by π as the inclination crosses zero for a fixed raan argument;
    // the equinoctial h and k do not. Compare nearby inclinations on either side of zero.
    let eq_at = |i: f64| {
        let state = elements(7_000.0, 0.2, i, 1.1, 0.7, 2.0)
            .to_state(MU)
            .expect("state");
        Equinoctial::from_state(&state, MU).expect("equinoctial")
    };
    let (h_neg, k_neg) = eq_at(-1e-4).inclination_components();
    let (h_pos, k_pos) = eq_at(1e-4).inclination_components();
    // The inclination changes by 2e-4, so tan(i/2) changes by about 1e-4 and the components with it:
    // the change is continuous, not a jump of π.
    assert!((h_pos - h_neg).abs() < 1e-3, "h {h_neg} to {h_pos}");
    assert!((k_pos - k_neg).abs() < 1e-3, "k {k_neg} to {k_pos}");
}

#[test]
fn invalid_elements_are_rejected() {
    // f² + g² ≥ 1: not an elliptic orbit.
    let not_elliptic = Equinoctial::new(7_000.0, 0.9, 0.9, 0.0, 0.0, 1.0).expect("finite");
    assert_eq!(not_elliptic.to_keplerian(), Err(Error::NotElliptic));
    // A non-positive semi-latus rectum.
    let no_p = Equinoctial::new(-1.0, 0.1, 0.0, 0.0, 0.0, 1.0).expect("finite");
    assert!(matches!(no_p.to_keplerian(), Err(Error::Negative { .. })));
    // A non-finite value is rejected at construction.
    assert!(matches!(
        Equinoctial::new(7_000.0, f64::NAN, 0.0, 0.0, 0.0, 1.0),
        Err(Error::NonFinite { .. })
    ));
}
