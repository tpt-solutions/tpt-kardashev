// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact rational arithmetic: normalisation, exact sums and products, comparison, and overflow
//! reported rather than wrapped.

#![allow(clippy::expect_used)]

use core::cmp::Ordering;
use tpt_kardashev_reflect::{Rational, ReflectError};

fn r(n: i128, d: i128) -> Rational {
    Rational::new(n, d).expect("denominator is non-zero")
}

#[test]
fn rationals_are_reduced_with_a_positive_denominator() {
    assert_eq!(r(4, 6), r(2, 3));
    assert_eq!(r(-1, -2), r(1, 2));
    let half_negative = r(1, -2);
    assert_eq!(half_negative.numerator(), -1);
    assert_eq!(half_negative.denominator(), 2);
    assert_eq!(r(0, 7), Rational::zero());
    assert_eq!(r(0, 7).denominator(), 1);
}

#[test]
fn a_zero_denominator_is_rejected() {
    assert_eq!(Rational::new(1, 0), Err(ReflectError::ZeroDenominator));
}

#[test]
fn sums_and_products_are_exact() {
    assert_eq!(r(1, 3).checked_add(r(1, 6)), Ok(r(1, 2)));
    assert_eq!(r(2, 3).checked_mul(r(3, 4)), Ok(r(1, 2)));
    assert_eq!(r(1, 3).checked_add(r(-1, 3)), Ok(Rational::zero()));
}

#[test]
fn comparison_is_exact_with_no_tolerance() {
    // Differ by one part in 10^30: a float comparison could not tell these apart.
    let a = r(1, 3);
    let b = Rational::new(
        333_333_333_333_333_333_333_333_333_333_i128,
        1_000_000_000_000_000_000_000_000_000_000_i128,
    )
    .expect("denominator");
    assert_eq!(a.checked_cmp(b), Ok(Ordering::Greater));
    assert_eq!(r(1, 2).checked_cmp(r(2, 4)), Ok(Ordering::Equal));
    assert_eq!(r(-1, 2).checked_cmp(r(-1, 3)), Ok(Ordering::Less));
}

#[test]
fn overflow_is_reported_not_wrapped() {
    let big = Rational::integer(i128::MAX);
    assert_eq!(
        big.checked_add(Rational::integer(1)),
        Err(ReflectError::Overflow)
    );
    assert_eq!(
        big.checked_mul(Rational::integer(2)),
        Err(ReflectError::Overflow)
    );
    assert_eq!(
        big.checked_cmp(Rational::new(i128::MAX, 2).expect("den")),
        Err(ReflectError::Overflow)
    );
}
