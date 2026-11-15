// SPDX-License-Identifier: MIT OR Apache-2.0
//! Containment certificates: an accepted certificate proves containment; each way a certificate can
//! fail is reported; and, as a soundness property, any accepted certificate holds on every sampled
//! point of the update.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_reflect::{Certificate, Constraints, Rational, ReflectError, Row, verify};

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

fn certificate(multipliers: &[&[i128]]) -> Certificate {
    Certificate {
        multipliers: multipliers
            .iter()
            .map(|m| m.iter().map(|&v| int(v)).collect())
            .collect(),
    }
}

/// The invariant 0 <= x <= 10 in one parameter: -x <= 0 and x <= 10.
fn invariant() -> Constraints {
    system(1, vec![row(&[-1], 0), row(&[1], 10)])
}

#[test]
fn a_narrower_update_is_contained_and_the_certificate_proves_it() {
    // The update is 1 <= x <= 5. For -x <= 0: use -x <= -1 (multiplier 1). For x <= 10: use x <= 5.
    let update = system(1, vec![row(&[1], 5), row(&[-1], -1)]);
    let cert = certificate(&[&[0, 1], &[1, 0]]);
    assert_eq!(verify(&invariant(), &update, &cert), Ok(()));
}

#[test]
fn a_wider_update_is_not_proved_by_the_honest_certificate() {
    // The update is x <= 20: it reaches outside the invariant, so no certificate can prove it. The
    // natural candidate fails the bound check.
    let update = system(1, vec![row(&[1], 20)]);
    let cert = certificate(&[&[0], &[1]]);
    assert_eq!(
        verify(&invariant(), &update, &cert),
        Err(ReflectError::CoefficientMismatch { row: 0, column: 0 })
    );
    let cert2 = certificate(&[&[0], &[1]]);
    assert_eq!(
        verify(&invariant(), &update, &cert2),
        Err(ReflectError::CoefficientMismatch { row: 0, column: 0 })
    );
}

#[test]
fn a_bound_that_is_too_large_is_reported() {
    // Same coefficient, weaker bound: x <= 20 against x <= 10.
    let update = system(1, vec![row(&[1], 20), row(&[-1], -1)]);
    let cert = certificate(&[&[0, 1], &[1, 0]]);
    assert_eq!(
        verify(&invariant(), &update, &cert),
        Err(ReflectError::BoundExceeded { row: 1 })
    );
}

#[test]
fn a_negative_multiplier_is_rejected() {
    let update = system(1, vec![row(&[1], 5), row(&[-1], -1)]);
    let cert = certificate(&[&[0, -1], &[1, 0]]);
    assert_eq!(
        verify(&invariant(), &update, &cert),
        Err(ReflectError::NegativeMultiplier { row: 0 })
    );
}

#[test]
fn a_certificate_of_the_wrong_shape_is_rejected() {
    let update = system(1, vec![row(&[1], 5)]);
    assert_eq!(
        verify(&invariant(), &update, &certificate(&[&[1]])),
        Err(ReflectError::CertificateShape)
    );
    assert_eq!(
        verify(&invariant(), &update, &certificate(&[&[1, 0], &[1]])),
        Err(ReflectError::CertificateShape)
    );
}

#[test]
fn different_dimensions_are_rejected() {
    let update = system(2, vec![row(&[1, 0], 5)]);
    assert_eq!(
        verify(&invariant(), &update, &certificate(&[&[1], &[1]])),
        Err(ReflectError::DimensionMismatch)
    );
}

#[test]
fn a_row_of_the_wrong_length_is_rejected_when_building() {
    assert_eq!(
        Constraints::new(2, vec![row(&[1], 5)]),
        Err(ReflectError::ShapeMismatch)
    );
}

#[test]
fn contains_is_exact() {
    let rect = system(2, vec![row(&[1, 0], 3), row(&[0, 1], 3), row(&[-1, -1], 0)]);
    assert_eq!(rect.contains(&[int(3), int(0)]), Ok(true));
    assert_eq!(rect.contains(&[int(4), int(0)]), Ok(false));
    assert_eq!(rect.contains(&[int(-1), int(2)]), Ok(true));
    assert_eq!(rect.contains(&[int(-2), int(1)]), Ok(false));
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
fn an_accepted_certificate_holds_on_every_sampled_point_of_the_update() {
    // Soundness as a property. Each invariant row is built as a non-negative combination of the
    // update's rows, plus a slack on the bound. With slack >= 0 the certificate must be accepted;
    // with slack < 0 it must not be. Whatever is accepted must hold on every sampled point.
    let mut rng = Lcg(2024);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for _ in 0..2_000 {
        let update_rows: Vec<Row> = (0..3)
            .map(|_| row(&[rng.range(-3, 3), rng.range(-3, 3)], rng.range(-4, 8)))
            .collect();
        let multipliers: Vec<Vec<i128>> = (0..2)
            .map(|_| (0..3).map(|_| rng.range(0, 2)).collect())
            .collect();
        let slacks: Vec<i128> = (0..2).map(|_| rng.range(-1, 3)).collect();
        let mut invariant_rows = Vec::new();
        for (i, lambda) in multipliers.iter().enumerate() {
            let mut coefficients = [0i128; 2];
            let mut bound = 0i128;
            for (j, update_row) in update_rows.iter().enumerate() {
                for (c, a) in coefficients.iter_mut().zip(update_row.coefficients.iter()) {
                    *c += lambda[j] * a.numerator();
                }
                bound += lambda[j] * update_row.bound.numerator();
            }
            invariant_rows.push(row(&coefficients, bound + slacks[i]));
        }
        let inv = system(2, invariant_rows);
        let upd = system(2, update_rows);
        let refs: Vec<&[i128]> = multipliers.iter().map(|m| m.as_slice()).collect();
        let cert = certificate(&refs);
        let outcome = verify(&inv, &upd, &cert);
        if slacks.iter().all(|&s| s >= 0) {
            assert_eq!(outcome, Ok(()), "a valid combination must be accepted");
        }
        if outcome.is_ok() {
            accepted += 1;
            for x in -12..=12i128 {
                for y in -12..=12i128 {
                    let point = [int(x), int(y)];
                    if upd.contains(&point).expect("small") {
                        assert!(
                            inv.contains(&point).expect("small"),
                            "accepted, but ({x}, {y}) is in the update and not the invariant"
                        );
                    }
                }
            }
        } else {
            rejected += 1;
        }
    }
    assert!(accepted > 1_000, "accepted {accepted}");
    assert!(
        rejected > 0,
        "some certificates with negative slack must be rejected"
    );
}
