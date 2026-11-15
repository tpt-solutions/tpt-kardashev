// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reduction to lowest terms at the edge of the `i128` range. Equality on `Rational` is derived, so
//! two equal rationals must have the same reduced form.
#![allow(clippy::expect_used)]

use tpt_kardashev_reflect::rational::Rational;

#[test]
fn the_most_negative_numerator_is_reduced_like_any_other() {
    // -2^127 / 2 is -2^126 / 1 in lowest terms.
    let a = Rational::new(i128::MIN, 2).expect("rational");
    let b = Rational::new(i128::MIN / 2, 1).expect("rational");
    assert_eq!(a, b);
    assert_eq!(a.denominator(), 1);
    assert_eq!(a.numerator(), i128::MIN / 2);
}

#[test]
fn the_most_negative_numerator_over_an_even_power_of_two_reduces_fully() {
    let a = Rational::new(i128::MIN, 1 << 100).expect("rational");
    assert_eq!(a, Rational::new(-(1 << 27), 1).expect("rational"));
}
