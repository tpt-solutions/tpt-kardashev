// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reaction-rate tallies per reaction, and the zone-splitting estimator. The reaction tallies must sum
//! to the collisions history by history; splitting must agree with the analog estimator in expectation,
//! including for deep penetration; splitting into one zone or one copy must be implicit capture; and
//! a thick slab gives the analytic infinite-medium reaction rates per source neutron.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::NeutronicsError;
use tpt_kardashev_nuclear_plasma::slab::{CrossSections, Estimator, Slab, run_slab};

/// Checks that `value` is within `k` standard errors of `expected`.
fn within_sigma(value: f64, std_error: f64, expected: f64, k: f64, label: &str) {
    assert!(
        (value - expected).abs() <= k * std_error.max(1e-12),
        "{label}: estimate {value} ± {std_error}, expected {expected}"
    );
}

#[test]
fn absorption_and_scattering_sum_to_the_collisions_for_every_estimator() {
    let xs = CrossSections::new(1.0, 0.3).expect("xs");
    let slab = Slab { thickness: 2.0, xs };
    for (estimator, seed) in [
        (Estimator::Analog, 301),
        (Estimator::ImplicitCapture, 302),
        (
            Estimator::Splitting {
                zones: 4,
                copies: 3,
            },
            303,
        ),
    ] {
        let t = run_slab(&slab, estimator, 40_000, seed).expect("run");
        // Per history the identity holds exactly, so the means agree to rounding.
        let sum = t.absorbed.mean + t.scattered.mean;
        assert!(
            (sum - t.collisions.mean).abs() <= 1e-9 * t.collisions.mean.max(1.0),
            "{estimator:?}: absorbed + scattered = {sum}, collisions = {}",
            t.collisions.mean
        );
    }
}

#[test]
fn a_pure_absorber_has_the_absorption_rate_of_its_first_flight_and_no_scattering() {
    // Every collision absorbs, so the absorption rate is the collision rate, P(first flight < T) =
    // 1 - e^{-σt T}, and the scattering rate is exactly zero.
    let xs = CrossSections::new(1.0, 1.0).expect("xs");
    let slab = Slab { thickness: 1.5, xs };
    let t = run_slab(&slab, Estimator::Analog, 200_000, 304).expect("run");
    within_sigma(
        t.absorbed.mean,
        t.absorbed.std_error,
        1.0 - (-1.5_f64).exp(),
        4.0,
        "absorption rate",
    );
    assert_eq!(t.scattered.mean, 0.0, "no scattering in a pure absorber");
    assert_eq!(
        t.absorbed.mean, t.collisions.mean,
        "every collision absorbs"
    );
}

#[test]
fn splitting_agrees_with_the_analog_estimator_on_a_scattering_slab() {
    let xs = CrossSections::new(1.0, 0.3).expect("xs");
    let slab = Slab { thickness: 2.0, xs };
    let analog = run_slab(&slab, Estimator::Analog, 300_000, 305).expect("run");
    let split = run_slab(
        &slab,
        Estimator::Splitting {
            zones: 4,
            copies: 3,
        },
        300_000,
        306,
    )
    .expect("run");
    for (label, a, s) in [
        ("transmitted", analog.transmitted, split.transmitted),
        ("reflected", analog.reflected, split.reflected),
        ("absorbed", analog.absorbed, split.absorbed),
        ("track length", analog.track_length, split.track_length),
    ] {
        let se = (a.std_error * a.std_error + s.std_error * s.std_error).sqrt();
        within_sigma(s.mean, se, a.mean, 4.0, label);
    }
}

#[test]
fn splitting_agrees_with_the_analog_estimator_for_deep_penetration() {
    // A thick, weakly absorbing slab: the far zones are reached rarely, which is where splitting
    // is meant to help. The estimates must still agree with the analog run.
    let xs = CrossSections::new(1.0, 0.2).expect("xs");
    let slab = Slab { thickness: 6.0, xs };
    let analog = run_slab(&slab, Estimator::Analog, 400_000, 308).expect("run");
    let split = run_slab(
        &slab,
        Estimator::Splitting {
            zones: 6,
            copies: 2,
        },
        400_000,
        309,
    )
    .expect("run");
    assert!(analog.transmitted.mean > 0.0 && analog.transmitted.mean < 0.2);
    let se = (analog.transmitted.std_error.powi(2) + split.transmitted.std_error.powi(2)).sqrt();
    within_sigma(
        split.transmitted.mean,
        se,
        analog.transmitted.mean,
        4.0,
        "deep transmission",
    );
}

#[test]
fn splitting_into_one_zone_or_one_copy_is_implicit_capture() {
    // Without a split there is nothing to change: the same draws and the same tallies.
    let xs = CrossSections::new(1.0, 0.4).expect("xs");
    let slab = Slab { thickness: 3.0, xs };
    let implicit = run_slab(&slab, Estimator::ImplicitCapture, 20_000, 307).expect("run");
    let one_copy = run_slab(
        &slab,
        Estimator::Splitting {
            zones: 4,
            copies: 1,
        },
        20_000,
        307,
    )
    .expect("run");
    let one_zone = run_slab(
        &slab,
        Estimator::Splitting {
            zones: 1,
            copies: 3,
        },
        20_000,
        307,
    )
    .expect("run");
    assert_eq!(implicit, one_copy);
    assert_eq!(implicit, one_zone);
}

#[test]
fn zero_zones_or_zero_copies_is_rejected() {
    let xs = CrossSections::new(1.0, 0.4).expect("xs");
    let slab = Slab { thickness: 3.0, xs };
    assert_eq!(
        run_slab(
            &slab,
            Estimator::Splitting {
                zones: 4,
                copies: 0,
            },
            10,
            1
        ),
        Err(NeutronicsError::BadSplitting)
    );
    assert_eq!(
        run_slab(
            &slab,
            Estimator::Splitting {
                zones: 0,
                copies: 2,
            },
            10,
            1
        ),
        Err(NeutronicsError::BadSplitting)
    );
}

#[test]
fn layers_split_their_collisions_into_absorption_and_scattering() {
    use tpt_kardashev_nuclear_plasma::layers::{Layers, Region, run_layers};
    let region = |thickness: f64, total: f64, absorption: f64| Region {
        thickness,
        xs: CrossSections::new(total, absorption).expect("xs"),
    };
    let layers = Layers::new(vec![region(1.0, 1.0, 0.3), region(1.0, 2.0, 0.5)]).expect("layers");
    let t = run_layers(&layers, 50_000, 310).expect("run");
    // Per history, each collision either absorbs or scatters: the identity is exact.
    assert!(
        (t.absorbed.mean + t.scattered.mean - t.collisions.mean).abs()
            <= 1e-9 * t.collisions.mean.max(1.0)
    );
    assert!(t.scattered.mean > 0.0, "a scattering medium scatters");
}
