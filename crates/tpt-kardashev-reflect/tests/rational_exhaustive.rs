// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exhaustive check of the rational arithmetic over a small range: every pair of fractions with
//! numerators and denominators in [-6, 6] (denominator nonzero). This replaces the Kani proofs of
//! checked add and checked multiply, which did not finish in a bounded run (see the todo). Each result
//! must equal the exact result, computed in its reduced form, and the comparison must match the exact
//! order.
#![allow(clippy::expect_used)]

use core::cmp::Ordering;
use tpt_kardashev_reflect::rational::Rational;

fn fractions() -> Vec<(i128, i128)> {
    let mut out = Vec::new();
    for num in -6..=6 {
        for den in -6..=6 {
            if den != 0 {
                out.push((num, den));
            }
        }
    }
    out
}

#[test]
fn checked_add_matches_the_exact_sum_for_every_small_pair() {
    for (an, ad) in fractions() {
        for (bn, bd) in fractions() {
            let a = Rational::new(an, ad).expect("nonzero");
            let b = Rational::new(bn, bd).expect("nonzero");
            let exact = Rational::new(an * bd + bn * ad, ad * bd).expect("nonzero");
            assert_eq!(a.checked_add(b), Ok(exact), "{an}/{ad} + {bn}/{bd}");
        }
    }
}

#[test]
fn checked_mul_matches_the_exact_product_for_every_small_pair() {
    for (an, ad) in fractions() {
        for (bn, bd) in fractions() {
            let a = Rational::new(an, ad).expect("nonzero");
            let b = Rational::new(bn, bd).expect("nonzero");
            let exact = Rational::new(an * bn, ad * bd).expect("nonzero");
            assert_eq!(a.checked_mul(b), Ok(exact), "{an}/{ad} * {bn}/{bd}");
        }
    }
}

#[test]
fn checked_cmp_matches_the_exact_order_for_every_small_pair() {
    for (an, ad) in fractions() {
        for (bn, bd) in fractions() {
            let a = Rational::new(an, ad).expect("nonzero");
            let b = Rational::new(bn, bd).expect("nonzero");
            // Denominators are positive after reduction, so comparing the cross products is exact.
            let exact = (an * bd).cmp(&(bn * ad));
            let exact: Ordering = if ad * bd < 0 { exact.reverse() } else { exact };
            assert_eq!(a.checked_cmp(b), Ok(exact), "{an}/{ad} vs {bn}/{bd}");
        }
    }
}
