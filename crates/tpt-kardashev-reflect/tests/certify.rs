// SPDX-License-Identifier: MIT OR Apache-2.0
//! The certificate generator: it finds a certificate whenever one exists in the constructed cases,
//! its output always passes the checker, and it reports when no certificate of the form exists.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_reflect::{Constraints, Rational, ReflectError, Row, certify, verify};

fn int(n: i128) -> Rational {
    Rational::integer(n)
}

fn row(coefficients: &[i128], bound: i128) -> Row {
    Row {
        coefficients: coefficients.iter().map(|&c| int(c)).collect(),
        bound: int(bound),
    }
}

fn system(dimension: usize, rows: Vec<Row>) -> Constraints {
    Constraints::new(dimension, rows).expect("shape")
}

#[test]
fn a_narrower_interval_gets_a_certificate_that_the_checker_accepts() {
    let invariant = system(1, vec![row(&[-1], 0), row(&[1], 10)]);
    let update = system(1, vec![row(&[-1], -2), row(&[1], 8)]);
    let cert = certify(&invariant, &update).expect("a certificate exists");
    assert_eq!(verify(&invariant, &update, &cert), Ok(()));
}

#[test]
fn a_wider_interval_gets_no_certificate() {
    let invariant = system(1, vec![row(&[-1], 0), row(&[1], 10)]);
    let update = system(1, vec![row(&[1], 20), row(&[-1], -1)]);
    assert_eq!(
        certify(&invariant, &update),
        Err(ReflectError::NoCertificate)
    );
}

#[test]
fn different_dimensions_are_rejected() {
    let invariant = system(1, vec![row(&[1], 10)]);
    let update = system(2, vec![row(&[1, 0], 5)]);
    assert_eq!(
        certify(&invariant, &update),
        Err(ReflectError::DimensionMismatch)
    );
}

/// Deterministic pseudo-random numbers (Knuth's LCG constants).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn range(&mut self, lo: i128, hi: i128) -> i128 {
        lo + (self.next() as i128) % (hi - lo + 1)
    }
}

#[test]
fn every_constructed_containment_gets_a_certificate() {
    // The invariant rows are built as non-negative combinations of the update's rows, with a slack of
    // zero or more on each bound. Such a certificate exists by construction, so the generator must find
    // one, and the checker must accept it.
    let mut rng = Lcg(77);
    let mut found = 0usize;
    for _ in 0..300 {
        let update_rows: Vec<Row> = (0..3)
            .map(|_| row(&[rng.range(-3, 3), rng.range(-3, 3)], rng.range(-4, 8)))
            .collect();
        let invariant_rows: Vec<Row> = (0..2)
            .map(|_| {
                let lambda: Vec<i128> = (0..3).map(|_| rng.range(0, 2)).collect();
                let slack = rng.range(0, 3);
                let mut coefficients = [0i128; 2];
                let mut bound = slack;
                for (j, update_row) in update_rows.iter().enumerate() {
                    for (c, a) in coefficients.iter_mut().zip(update_row.coefficients.iter()) {
                        *c += lambda[j] * a.numerator();
                    }
                    bound += lambda[j] * update_row.bound.numerator();
                }
                row(&coefficients, bound)
            })
            .collect();
        let invariant = system(2, invariant_rows);
        let update = system(2, update_rows);
        let cert = certify(&invariant, &update).expect("a certificate exists by construction");
        assert_eq!(verify(&invariant, &update, &cert), Ok(()));
        found += 1;
    }
    assert_eq!(found, 300);
}

#[test]
fn any_certificate_the_generator_returns_passes_the_checker() {
    // Soundness of the generator's output against the checker, on arbitrary systems: whenever a
    // certificate is returned, the checker accepts it, and the accepted containment holds on a grid.
    let mut rng = Lcg(5);
    let mut returned = 0usize;
    for _ in 0..500 {
        let invariant_rows: Vec<Row> = (0..2)
            .map(|_| row(&[rng.range(-3, 3), rng.range(-3, 3)], rng.range(-4, 8)))
            .collect();
        let update_rows: Vec<Row> = (0..3)
            .map(|_| row(&[rng.range(-3, 3), rng.range(-3, 3)], rng.range(-4, 8)))
            .collect();
        let invariant = system(2, invariant_rows);
        let update = system(2, update_rows);
        if let Ok(cert) = certify(&invariant, &update) {
            returned += 1;
            assert_eq!(verify(&invariant, &update, &cert), Ok(()));
            for x in -10..=10i128 {
                for y in -10..=10i128 {
                    let point = [int(x), int(y)];
                    if update.contains(&point).expect("small") {
                        assert!(invariant.contains(&point).expect("small"));
                    }
                }
            }
        }
    }
    assert!(returned > 0, "the generator returned some certificates");
}
