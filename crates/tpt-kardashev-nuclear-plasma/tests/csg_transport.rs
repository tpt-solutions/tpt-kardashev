// SPDX-License-Identifier: MIT OR Apache-2.0
//! Transport through the three-dimensional geometry: the leakage of a pure absorber from the centre of a
//! sphere and of a two-shell sphere (closed forms, the path from the centre being radial), the track
//! length in a sphere, conservation of the histories, the event cap, reproducibility, and rejection of
//! invalid input.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::NeutronicsError;
use tpt_kardashev_nuclear_plasma::csg::{Expr, Model, Surface};
use tpt_kardashev_nuclear_plasma::csg_transport::{Tally, run};
use tpt_kardashev_nuclear_plasma::slab::CrossSections;

const CENTRE: [f64; 3] = [0.0, 0.0, 0.0];

fn sphere(radius: f64) -> Surface {
    Surface::Sphere {
        centre: CENTRE,
        radius,
    }
}

/// One zone: the sphere of radius `r`.
fn ball(r: f64) -> Model {
    Model::new(vec![sphere(r)], vec![Expr::Inside(0)]).expect("model")
}

/// Two zones: the core inside radius `a`, and the shell between `a` and `b`.
fn two_shells(a: f64, b: f64) -> Model {
    Model::new(
        vec![sphere(a), sphere(b)],
        vec![
            Expr::Inside(0),
            Expr::Intersection(vec![
                Expr::Inside(1),
                Expr::Complement(Box::new(Expr::Inside(0))),
            ]),
        ],
    )
    .expect("model")
}

fn xs(total: f64, absorption: f64) -> CrossSections {
    CrossSections::new(total, absorption).expect("cross sections")
}

fn assert_conserved(t: &Tally) {
    assert_eq!(
        t.absorbed + t.leaked + t.unfinished,
        t.histories,
        "one fate per history"
    );
}

/// Within `k` binomial standard errors of `expected` for `n` trials.
fn within_binomial(count: usize, n: usize, expected: f64, k: f64, label: &str) {
    let p = count as f64 / n as f64;
    let se = (expected * (1.0 - expected) / n as f64).sqrt();
    assert!(
        (p - expected).abs() <= k * se,
        "{label}: fraction {p}, expected {expected} ± {se}"
    );
}

#[test]
fn a_pure_absorber_leaks_from_the_centre_with_exp_minus_sigma_r() {
    // Every direction from the centre has path R to the surface, so the leakage is exactly exp(-σR).
    let (sigma, radius, n) = (1.3, 1.0, 100_000);
    let t = run(&ball(radius), &[xs(sigma, sigma)], CENTRE, n, 1_000, 11).expect("run");
    assert_conserved(&t);
    within_binomial(t.leaked, n, (-sigma * radius).exp(), 5.0, "leakage");
    assert_eq!(
        t.collisions, t.absorbed,
        "every collision of a pure absorber absorbs"
    );
}

#[test]
fn the_track_in_a_sphere_for_a_pure_absorber_is_one_over_sigma_times_one_minus_exp() {
    // E[min(s, R)] = (1 - exp(-σR)) / σ for the exponential flight s, which is the track length per history.
    let (sigma, radius, n) = (1.3, 1.0, 100_000);
    let t = run(&ball(radius), &[xs(sigma, sigma)], CENTRE, n, 1_000, 12).expect("run");
    let per_history = t.track_length[0] / n as f64;
    let expected = (1.0 - (-sigma * radius).exp()) / sigma;
    assert!(
        (per_history - expected).abs() <= 0.01,
        "track per history {per_history}, expected {expected}"
    );
}

#[test]
fn two_shells_leak_as_exp_minus_the_sum_of_the_path_times_their_sigmas() {
    // Core radius 0.5 with σ = 2, shell out to 1.0 with σ = 0.7; the path from the centre is 0.5 in the core
    // and 0.5 in the shell, so the leakage is exp(-2·0.5 - 0.7·0.5) = exp(-1.35).
    let n = 100_000;
    let model = two_shells(0.5, 1.0);
    let t = run(&model, &[xs(2.0, 2.0), xs(0.7, 0.7)], CENTRE, n, 1_000, 13).expect("run");
    assert_conserved(&t);
    within_binomial(t.leaked, n, (-1.35f64).exp(), 5.0, "two-shell leakage");
}

#[test]
fn a_scatterer_with_no_absorption_always_leaks_and_scatters_on_the_way() {
    // σt = 5 in a ball of radius 1: a mean free path of 0.2, so many collisions. With no absorption every
    // history leaves, and none is cut short by the cap.
    let n = 2_000;
    let t = run(&ball(1.0), &[xs(5.0, 0.0)], CENTRE, n, 1_000_000, 14).expect("run");
    assert_eq!(t.absorbed, 0);
    assert_eq!(t.unfinished, 0);
    assert_eq!(t.leaked, n);
    assert!(
        t.collisions > 5 * n,
        "scattering collisions {}",
        t.collisions
    );
}

#[test]
fn the_event_cap_is_reported_as_unfinished_and_the_fates_still_add_up() {
    // A pure scatterer in a ball of radius 1 with σt = 5: a history that has not left after 3 events is
    // unfinished. Some leave within 3 events, so the unfinished count is between zero and n, not all of them.
    let n = 500;
    let t = run(&ball(1.0), &[xs(5.0, 0.0)], CENTRE, n, 3, 15).expect("run");
    assert!(
        t.unfinished > 0 && t.unfinished < n,
        "unfinished {}",
        t.unfinished
    );
    assert_eq!(
        t.leaked + t.unfinished,
        n,
        "every history leaked or hit the cap"
    );
    assert_conserved(&t);
}

#[test]
fn a_mixed_run_conserves_the_histories_for_every_seed() {
    let model = two_shells(0.5, 1.0);
    for seed in 0..5u64 {
        let t = run(
            &model,
            &[xs(2.0, 0.6), xs(0.7, 0.2)],
            CENTRE,
            3_000,
            10_000,
            seed,
        )
        .expect("run");
        assert_conserved(&t);
        assert!(t.absorbed > 0 && t.leaked > 0, "seed {seed}: {t:?}");
    }
}

#[test]
fn the_same_seed_gives_the_same_tally() {
    let model = two_shells(0.5, 1.0);
    let mats = [xs(2.0, 0.6), xs(0.7, 0.2)];
    let a = run(&model, &mats, CENTRE, 2_000, 10_000, 99).expect("run");
    let b = run(&model, &mats, CENTRE, 2_000, 10_000, 99).expect("run");
    assert_eq!(a, b);
}

#[test]
fn invalid_input_is_rejected() {
    let model = ball(1.0);
    let mats = [xs(1.0, 0.5)];
    assert_eq!(
        run(&model, &mats, CENTRE, 0, 10, 1),
        Err(NeutronicsError::NoHistories)
    );
    assert_eq!(
        run(&model, &[], CENTRE, 10, 10, 1),
        Err(NeutronicsError::BadGeometry),
        "material count"
    );
    assert_eq!(
        run(&model, &mats, CENTRE, 10, 0, 1),
        Err(NeutronicsError::BadGeometry),
        "zero events"
    );
    assert_eq!(
        run(&model, &mats, [2.0, 0.0, 0.0], 10, 10, 1),
        Err(NeutronicsError::BadGeometry),
        "source outside every zone"
    );
    assert_eq!(
        run(&model, &mats, [f64::NAN, 0.0, 0.0], 10, 10, 1),
        Err(NeutronicsError::BadGeometry),
        "source not finite"
    );
}
