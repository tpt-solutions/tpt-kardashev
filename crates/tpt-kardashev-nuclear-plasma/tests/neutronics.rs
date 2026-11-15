// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fixed-source neutronics validated against analytic results: the uncollided transmission law for a
//! pure absorber, its track length, the infinite-medium track length 1/σa, weight conservation, the
//! agreement of the two estimators, reproducibility from a seed, and the error of the estimates.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::slab::{CrossSections, Estimator, Slab, run_infinite, run_slab};
use tpt_kardashev_nuclear_plasma::{NeutronicsError, SplitMix64};

/// Checks that `value` is within `k` standard errors of `expected`.
fn within_sigma(value: f64, std_error: f64, expected: f64, k: f64, label: &str) {
    assert!(
        (value - expected).abs() <= k * std_error.max(1e-12),
        "{label}: estimate {value} ± {std_error}, expected {expected}"
    );
}

#[test]
fn a_pure_absorber_transmits_the_uncollided_fraction_exp_minus_sigma_t() {
    // With no scattering, a neutron passes unless its first flight is shorter than the slab.
    let xs = CrossSections::new(1.0, 1.0).expect("xs");
    let slab = Slab { thickness: 1.5, xs };
    let tally = run_slab(&slab, Estimator::Analog, 200_000, 11).expect("run");
    let expected = (-1.5_f64).exp();
    within_sigma(
        tally.transmitted.mean,
        tally.transmitted.std_error,
        expected,
        4.0,
        "analog transmission",
    );
    assert_eq!(tally.reflected.mean, 0.0, "no scattering, no reflection");
}

#[test]
fn a_pure_absorber_has_the_track_length_of_its_first_flight_capped_at_the_slab() {
    // Expected track length: E[min(flight, T)] = (1 - e^{-σt T}) / σt.
    let xs = CrossSections::new(0.8, 0.8).expect("xs");
    let thickness = 2.0;
    let slab = Slab { thickness, xs };
    let tally = run_slab(&slab, Estimator::Analog, 200_000, 12).expect("run");
    let expected = (1.0 - (-0.8 * thickness).exp()) / 0.8;
    within_sigma(
        tally.track_length.mean,
        tally.track_length.std_error,
        expected,
        4.0,
        "track length",
    );
}

#[test]
fn an_infinite_medium_has_a_track_length_of_one_over_sigma_a() {
    // Every neutron is absorbed in the end, and the expected track length is 1/σa, whatever σs is.
    let xs = CrossSections::new(1.0, 0.3).expect("xs");
    let estimate = run_infinite(xs, 200_000, 13).expect("run");
    within_sigma(
        estimate.mean,
        estimate.std_error,
        1.0 / 0.3,
        4.0,
        "infinite-medium track length",
    );
}

#[test]
fn analog_weight_is_conserved_history_by_history() {
    // Analog histories end in exactly one of transmission, reflection or absorption, so the mean
    // scores sum to exactly one per source neutron.
    let xs = CrossSections::new(1.0, 0.4).expect("xs");
    let slab = Slab { thickness: 1.0, xs };
    let tally = run_slab(&slab, Estimator::Analog, 50_000, 14).expect("run");
    let total = tally.transmitted.mean + tally.reflected.mean + tally.absorbed.mean;
    assert!((total - 1.0).abs() < 1e-12, "sum of fates {total}");
}

#[test]
fn both_estimators_agree_on_a_scattering_slab() {
    let xs = CrossSections::new(1.0, 0.2).expect("xs");
    let slab = Slab { thickness: 1.2, xs };
    let analog = run_slab(&slab, Estimator::Analog, 300_000, 15).expect("run");
    let implicit = run_slab(&slab, Estimator::ImplicitCapture, 300_000, 16).expect("run");
    let combined =
        (analog.transmitted.std_error.powi(2) + implicit.transmitted.std_error.powi(2)).sqrt();
    within_sigma(
        implicit.transmitted.mean,
        combined,
        analog.transmitted.mean,
        4.0,
        "implicit against analog transmission",
    );
    let combined_abs =
        (analog.absorbed.std_error.powi(2) + implicit.absorbed.std_error.powi(2)).sqrt();
    within_sigma(
        implicit.absorbed.mean,
        combined_abs,
        analog.absorbed.mean,
        4.0,
        "implicit against analog absorption",
    );
}

#[test]
fn the_same_seed_gives_the_same_tally_and_another_seed_does_not() {
    let xs = CrossSections::new(1.0, 0.5).expect("xs");
    let slab = Slab { thickness: 1.0, xs };
    let a = run_slab(&slab, Estimator::Analog, 10_000, 99).expect("run");
    let b = run_slab(&slab, Estimator::Analog, 10_000, 99).expect("run");
    let c = run_slab(&slab, Estimator::Analog, 10_000, 100).expect("run");
    assert_eq!(a, b);
    assert_ne!(a.transmitted.mean, c.transmitted.mean);
}

#[test]
fn the_standard_error_falls_as_the_square_root_of_the_histories() {
    let xs = CrossSections::new(1.0, 0.5).expect("xs");
    let slab = Slab { thickness: 1.0, xs };
    let small = run_slab(&slab, Estimator::Analog, 20_000, 21).expect("run");
    let large = run_slab(&slab, Estimator::Analog, 80_000, 22).expect("run");
    let ratio = small.transmitted.std_error / large.transmitted.std_error;
    assert!(
        (1.7..2.3).contains(&ratio),
        "ratio {ratio} should be near 2"
    );
}

#[test]
fn the_generator_is_uniform_enough_for_the_tallies() {
    // A coarse check on the generator: the mean of 100 000 uniforms is near one half, and no bucket
    // of ten is far from its tenth.
    let mut rng = SplitMix64::new(7);
    let mut buckets = [0usize; 10];
    let mut sum = 0.0;
    for _ in 0..100_000 {
        let u = rng.next_f64();
        sum += u;
        buckets[((u * 10.0) as usize).min(9)] += 1;
    }
    assert!((sum / 100_000.0 - 0.5).abs() < 0.005);
    for count in buckets {
        assert!((9_000..11_000).contains(&count), "bucket count {count}");
    }
}

#[test]
fn bad_inputs_are_rejected() {
    assert_eq!(
        CrossSections::new(0.0, 0.0),
        Err(NeutronicsError::BadCrossSection)
    );
    assert_eq!(
        CrossSections::new(1.0, 1.5),
        Err(NeutronicsError::BadCrossSection)
    );
    assert_eq!(
        CrossSections::new(1.0, -0.1),
        Err(NeutronicsError::BadCrossSection)
    );
    let xs = CrossSections::new(1.0, 0.5).expect("xs");
    assert_eq!(
        run_slab(&Slab { thickness: 0.0, xs }, Estimator::Analog, 10, 1),
        Err(NeutronicsError::BadThickness)
    );
    assert_eq!(
        run_slab(&Slab { thickness: 1.0, xs }, Estimator::Analog, 0, 1),
        Err(NeutronicsError::NoHistories)
    );
    assert_eq!(
        run_infinite(CrossSections::new(1.0, 0.0).expect("xs"), 10, 1),
        Err(NeutronicsError::BadCrossSection)
    );
}

#[test]
fn scattering_is_the_difference_of_total_and_absorption() {
    let xs = CrossSections::new(1.0, 0.25).expect("xs");
    assert!((xs.scattering() - 0.75).abs() < 1e-15);
}

#[test]
fn the_binned_flux_of_a_pure_absorber_matches_the_exponential_in_each_bin() {
    // With no scattering every flight is along the normal. A track is counted in a bin where the flight
    // reaches, so the expected track length in [a, b] is the integral of P(flight > x) = e^{-sigma x} over
    // the bin, which is (e^{-sigma a} - e^{-sigma b}) / sigma.
    let sigma = 0.9;
    let thickness = 2.0;
    let bins = 4;
    let slab = Slab {
        thickness,
        xs: CrossSections::new(sigma, sigma).expect("xs"),
    };
    let tally =
        tpt_kardashev_nuclear_plasma::slab::run_slab_binned(&slab, 200_000, 31, bins).expect("run");
    let width = thickness / bins as f64;
    for (bin, estimate) in tally.iter().enumerate() {
        let a = bin as f64 * width;
        let b = a + width;
        let expected = ((-sigma * a).exp() - (-sigma * b).exp()) / sigma;
        within_sigma(
            estimate.mean,
            estimate.std_error,
            expected,
            4.0,
            "bin track length",
        );
    }
}

#[test]
fn the_bins_of_a_run_sum_to_its_total_track_length() {
    let xs = CrossSections::new(1.0, 0.3).expect("xs");
    let slab = Slab { thickness: 1.5, xs };
    let bins =
        tpt_kardashev_nuclear_plasma::slab::run_slab_binned(&slab, 20_000, 32, 5).expect("run");
    let total: f64 = bins.iter().map(|b| b.mean).sum();
    let analog = run_slab(&slab, Estimator::Analog, 20_000, 32).expect("run");
    assert!(
        (total - analog.track_length.mean).abs() < 1e-9,
        "{total} against {}",
        analog.track_length.mean
    );
}

#[test]
fn an_empty_binning_is_rejected() {
    let slab = Slab {
        thickness: 1.0,
        xs: CrossSections::new(1.0, 0.5).expect("xs"),
    };
    assert_eq!(
        tpt_kardashev_nuclear_plasma::slab::run_slab_binned(&slab, 10, 1, 0),
        Err(NeutronicsError::NoHistories)
    );
}
