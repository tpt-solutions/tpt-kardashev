// SPDX-License-Identifier: MIT OR Apache-2.0
//! Impulsive manoeuvres checked by propagating the transfer orbits, not by re-using the formulas
//! that produced them: each burn must put the spacecraft on the orbit that reaches the next radius
//! with the speed the manoeuvre predicts.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use core::f64::consts::PI;
use tpt_kardashev_astrodynamics::manoeuvre::{bi_elliptic, hohmann, plane_change};
use tpt_kardashev_astrodynamics::propagate::propagate;
use tpt_kardashev_astrodynamics::{Error, Seconds, StateVector, Vec3};

/// Earth's gravitational parameter in km³/s², as in the other tests.
const MU: f64 = 398_600.441_8;
/// A 300 km orbit above a 6378.137 km Earth, and geostationary radius (km).
const R_LEO: f64 = 6_678.137;
const R_GEO: f64 = 42_164.0;

/// Propagation tolerance (iterative-solver policy: 1e-9 relative).
const PROP_REL: f64 = 1e-9;

fn assert_close(actual: f64, expected: f64, rel: f64) {
    let bound = rel * expected.abs();
    assert!(
        (actual - expected).abs() <= bound,
        "actual {actual}, expected {expected}, bound {bound}"
    );
}

/// The state at radius `r` on the x axis, moving along +y at `speed`.
fn at_radius(r: f64, speed: f64) -> StateVector {
    StateVector {
        position: Vec3::new(r, 0.0, 0.0),
        velocity: Vec3::new(0.0, speed, 0.0),
    }
}

fn secs(value: f64) -> Seconds {
    Seconds::new(value).expect("finite")
}

#[test]
fn hohmann_transfer_reaches_the_outer_radius_with_the_predicted_speed() {
    let h = hohmann(MU, R_LEO, R_GEO).expect("valid");
    // Burn 1: from the circular speed at R_LEO to the transfer ellipse's perigee speed.
    let v_circ1 = (MU / R_LEO).sqrt();
    let start = at_radius(R_LEO, v_circ1 + h.dv1);
    // Half the transfer ellipse's period takes the spacecraft to apogee at R_GEO.
    let apogee = propagate(&start, MU, secs(h.time_of_flight)).expect("propagates");
    assert_close(apogee.position.norm(), R_GEO, PROP_REL);
    // At apogee the speed is the transfer speed; burn 2 brings it to the circular speed.
    let v_circ2 = (MU / R_GEO).sqrt();
    let speed_at_apogee = apogee.velocity.norm();
    assert_close(v_circ2 - speed_at_apogee, h.dv2, PROP_REL);
}

#[test]
fn hohmann_leo_to_geo_has_the_textbook_totals() {
    // From the vis-viva equation: the transfer's perigee speed is about 10.149 km/s against a
    // circular 7.727 km/s, so the first burn is 2.4257 km/s. At apogee the transfer is about
    // 1.607 km/s against a circular 3.075 km/s, so the second is 1.4669 km/s. The total is
    // 3.8926 km/s. The transfer takes about 18 990 s.
    let h = hohmann(MU, R_LEO, R_GEO).expect("valid");
    assert!((3.88..3.90).contains(&h.total), "total {} km/s", h.total);
    assert_close(h.dv1, 2.4257, 1e-4);
    assert_close(h.dv2, 1.4669, 1e-4);
    assert!(
        (18_500.0..19_500.0).contains(&h.time_of_flight),
        "time of flight {} s",
        h.time_of_flight
    );
    assert_close(h.total, h.dv1 + h.dv2, PROP_REL);
}

#[test]
fn hohmann_is_symmetric_in_its_radii() {
    let up = hohmann(MU, R_LEO, R_GEO).expect("valid");
    let down = hohmann(MU, R_GEO, R_LEO).expect("valid");
    assert_close(up.total, down.total, PROP_REL);
    assert_close(up.time_of_flight, down.time_of_flight, PROP_REL);
}

#[test]
fn bi_elliptic_transfer_chains_through_the_intermediate_radius() {
    let rb = 5.0 * R_GEO;
    let b = bi_elliptic(MU, R_LEO, rb, R_GEO).expect("valid");

    // Burn 1 and the first half-ellipse reach the intermediate radius.
    let a1 = 0.5 * (R_LEO + rb);
    let start = at_radius(R_LEO, (MU / R_LEO).sqrt() + b.dv1);
    let at_rb = propagate(&start, MU, secs(PI * (a1.powi(3) / MU).sqrt())).expect("propagates");
    assert_close(at_rb.position.norm(), rb, PROP_REL);

    // Burn 2 at rb puts the spacecraft on the second ellipse, whose perigee is R_GEO. Its apsides
    // are rb and R_GEO, so the half-ellipse from rb reaches R_GEO.
    let a2 = 0.5 * (rb + R_GEO);
    let v_a1_at_rb = at_rb.velocity.norm();
    let v_a2_at_rb = (MU * (2.0 / rb - 1.0 / a2)).sqrt();
    assert_close((v_a1_at_rb - v_a2_at_rb).abs(), b.dv2, PROP_REL);
    let second = StateVector {
        position: at_rb.position,
        velocity: at_rb.velocity * (v_a2_at_rb / v_a1_at_rb),
    };
    let at_geo = propagate(&second, MU, secs(PI * (a2.powi(3) / MU).sqrt())).expect("propagates");
    assert_close(at_geo.position.norm(), R_GEO, PROP_REL);

    // Burn 3 circularises at R_GEO.
    let v_p2 = at_geo.velocity.norm();
    // The perigee speed of the second ellipse is above circular here, so the burn is retrograde:
    // compare magnitudes.
    assert_close(((MU / R_GEO).sqrt() - v_p2).abs(), b.dv3, PROP_REL);
}

#[test]
fn bi_elliptic_beats_hohmann_for_a_large_radius_ratio() {
    // For a ratio of about 15 the bi-elliptic transfer through a large intermediate radius uses
    // less Δv than the Hohmann transfer between the same circular orbits.
    let r2 = 15.0 * R_LEO;
    let hohmann_total = hohmann(MU, R_LEO, r2).expect("valid").total;
    let bi_total = bi_elliptic(MU, R_LEO, 60.0 * R_LEO, r2)
        .expect("valid")
        .total;
    assert!(
        bi_total < hohmann_total,
        "bi-elliptic {bi_total} km/s, Hohmann {hohmann_total} km/s"
    );
}

#[test]
fn plane_change_matches_the_geometry_of_the_two_velocities() {
    // A velocity v turned through an angle θ: its change is 2·v·sin(θ/2), and the law of cosines
    // gives the same value from the two velocity vectors.
    let v: f64 = 7.5;
    let theta: f64 = 0.4;
    let pure = plane_change(v, v, theta).expect("valid");
    assert_close(pure, 2.0 * v * (theta / 2.0).sin(), PROP_REL);

    let v_before = Vec3::new(v, 0.0, 0.0);
    let v_after = Vec3::new(v * theta.cos(), v * theta.sin(), 0.0);
    assert_close(pure, (v_after - v_before).norm(), PROP_REL);

    // Changing the speed as well: from 7.5 to 6.0 km/s through the same angle.
    let combined = plane_change(7.5, 6.0, theta).expect("valid");
    let v_after = Vec3::new(6.0 * theta.cos(), 6.0 * theta.sin(), 0.0);
    assert_close(combined, (v_after - v_before).norm(), PROP_REL);

    assert_eq!(plane_change(v, v, 0.0).expect("valid"), 0.0);
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(hohmann(0.0, R_LEO, R_GEO), Err(Error::InvalidMu));
    assert_eq!(hohmann(MU, 0.0, R_GEO), Err(Error::InvalidRadius));
    assert_eq!(hohmann(MU, R_LEO, f64::NAN), Err(Error::InvalidRadius));
    // The intermediate radius must exceed both ends.
    assert_eq!(
        bi_elliptic(MU, R_LEO, R_GEO - 1.0, R_GEO),
        Err(Error::InvalidRadius)
    );
    assert!(matches!(
        plane_change(f64::NAN, 1.0, 0.1),
        Err(Error::NonFinite { .. })
    ));
}
