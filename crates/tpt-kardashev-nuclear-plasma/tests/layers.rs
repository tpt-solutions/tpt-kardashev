// SPDX-License-Identifier: MIT OR Apache-2.0
//! Layered geometry: the transmission law through several regions, invariance when one region is cut
//! into two of the same material, weight conservation, agreement with the slab run, and rejection of
//! invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::NeutronicsError;
use tpt_kardashev_nuclear_plasma::layers::{Layers, Region, run_layers};
use tpt_kardashev_nuclear_plasma::slab::{CrossSections, Estimator, Slab, run_slab};

fn within_sigma(value: f64, std_error: f64, expected: f64, k: f64, label: &str) {
    assert!(
        (value - expected).abs() <= k * std_error.max(1e-12),
        "{label}: estimate {value} ± {std_error}, expected {expected}"
    );
}

fn region(thickness: f64, total: f64, absorption: f64) -> Region {
    Region {
        thickness,
        xs: CrossSections::new(total, absorption).expect("xs"),
    }
}

#[test]
fn a_pure_absorber_through_two_regions_transmits_exp_minus_the_sum_of_sigma_t() {
    // Uncollided transmission: exp(-(σ1 t1 + σ2 t2)) = exp(-(0.5 * 1.0 + 2.0 * 0.5)) = exp(-1.5).
    let layers = Layers::new(vec![region(1.0, 0.5, 0.5), region(0.5, 2.0, 2.0)]).expect("layers");
    let tally = run_layers(&layers, 200_000, 41).expect("run");
    within_sigma(
        tally.transmitted.mean,
        tally.transmitted.std_error,
        (-1.5_f64).exp(),
        4.0,
        "two-region transmission",
    );
}

#[test]
fn cutting_a_region_into_two_of_the_same_material_does_not_change_the_transmission() {
    let whole = Layers::new(vec![region(1.5, 1.0, 1.0)]).expect("layers");
    let cut = Layers::new(vec![region(0.5, 1.0, 1.0), region(1.0, 1.0, 1.0)]).expect("layers");
    let a = run_layers(&whole, 200_000, 42).expect("run");
    let b = run_layers(&cut, 200_000, 43).expect("run");
    let combined = (a.transmitted.std_error.powi(2) + b.transmitted.std_error.powi(2)).sqrt();
    within_sigma(
        b.transmitted.mean,
        combined,
        a.transmitted.mean,
        4.0,
        "cut against whole region",
    );
}

#[test]
fn a_single_region_agrees_with_the_slab_run() {
    let layers = Layers::new(vec![region(1.2, 1.0, 0.2)]).expect("layers");
    let slab = Slab {
        thickness: 1.2,
        xs: CrossSections::new(1.0, 0.2).expect("xs"),
    };
    let layered = run_layers(&layers, 300_000, 44).expect("run");
    let plain = run_slab(&slab, Estimator::Analog, 300_000, 45).expect("run");
    let combined =
        (layered.transmitted.std_error.powi(2) + plain.transmitted.std_error.powi(2)).sqrt();
    within_sigma(
        layered.transmitted.mean,
        combined,
        plain.transmitted.mean,
        4.0,
        "layered against slab transmission",
    );
}

#[test]
fn every_analog_history_ends_in_exactly_one_fate() {
    let layers = Layers::new(vec![region(0.5, 1.0, 0.3), region(0.7, 2.0, 0.5)]).expect("layers");
    let tally = run_layers(&layers, 50_000, 46).expect("run");
    let total = tally.transmitted.mean + tally.reflected.mean + tally.absorbed.mean;
    assert!((total - 1.0).abs() < 1e-12, "fates sum to {total}");
    assert!(tally.collisions.mean > 0.0);
}

#[test]
fn a_scattering_stack_reflects_some_neutrons() {
    // A thick, weakly absorbing region reflects a noticeable share of the normal-incidence beam.
    let layers = Layers::new(vec![region(3.0, 1.0, 0.05)]).expect("layers");
    let tally = run_layers(&layers, 100_000, 47).expect("run");
    assert!(
        tally.reflected.mean > 0.05,
        "reflected {}",
        tally.reflected.mean
    );
}

#[test]
fn invalid_layers_are_rejected() {
    assert_eq!(Layers::new(Vec::new()), Err(NeutronicsError::NoGroups));
    assert_eq!(
        Layers::new(vec![region(0.0, 1.0, 0.5)]),
        Err(NeutronicsError::BadThickness)
    );
    let layers = Layers::new(vec![region(1.0, 1.0, 0.5)]).expect("layers");
    assert_eq!(run_layers(&layers, 0, 1), Err(NeutronicsError::NoHistories));
    assert!((layers.thickness() - 1.0).abs() < 1e-15);
}
