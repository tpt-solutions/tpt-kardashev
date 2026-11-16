// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact rational arithmetic on `i128`, with every operation checked. The checker uses nothing
//! inexact: a certificate is accepted or rejected by exact comparison, never by a tolerance.

use crate::error::ReflectError;

/// An exact rational `num / den` in lowest terms, with a positive denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rational {
    num: i128,
    den: i128,
}

/// The greatest common divisor, with `gcd(0, b) = b`. Over `u128`, so the magnitude of `i128::MIN`
/// is an ordinary value.
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Rational {
    /// The rational `num / den`, reduced. Fails with [`ReflectError::ZeroDenominator`] when `den` is
    /// zero, and with [`ReflectError::Overflow`] when a sign flip would overflow.
    pub fn new(num: i128, den: i128) -> Result<Self, ReflectError> {
        if den == 0 {
            return Err(ReflectError::ZeroDenominator);
        }
        let (num, den) = if den < 0 {
            (
                num.checked_neg().ok_or(ReflectError::Overflow)?,
                den.checked_neg().ok_or(ReflectError::Overflow)?,
            )
        } else {
            (num, den)
        };
        // `den` is positive, so the gcd is at least one and at most `den`, which fits in `i128`. It
        // divides the numerator's magnitude exactly, so the division below is exact, and it cannot
        // overflow: the only quotient that does is `i128::MIN / -1`, and `g` is positive.
        let g = gcd(num.unsigned_abs(), den as u128) as i128;
        Ok(Self {
            num: num / g,
            den: den / g,
        })
    }

    /// The integer `n`.
    pub fn integer(n: i128) -> Self {
        Self { num: n, den: 1 }
    }

    /// Zero.
    pub const fn zero() -> Self {
        Self { num: 0, den: 1 }
    }

    /// The numerator, in lowest terms.
    pub fn numerator(&self) -> i128 {
        self.num
    }

    /// The denominator, in lowest terms; always positive.
    pub fn denominator(&self) -> i128 {
        self.den
    }

    /// Whether this is zero.
    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    /// Whether this is negative.
    pub fn is_negative(&self) -> bool {
        self.num < 0
    }

    /// `self + other`, or [`ReflectError::Overflow`].
    pub fn checked_add(self, other: Self) -> Result<Self, ReflectError> {
        let left = self
            .num
            .checked_mul(other.den)
            .ok_or(ReflectError::Overflow)?;
        let right = other
            .num
            .checked_mul(self.den)
            .ok_or(ReflectError::Overflow)?;
        let num = left.checked_add(right).ok_or(ReflectError::Overflow)?;
        let den = self
            .den
            .checked_mul(other.den)
            .ok_or(ReflectError::Overflow)?;
        Rational::new(num, den)
    }

    /// `self * other`, or [`ReflectError::Overflow`].
    pub fn checked_mul(self, other: Self) -> Result<Self, ReflectError> {
        let num = self
            .num
            .checked_mul(other.num)
            .ok_or(ReflectError::Overflow)?;
        let den = self
            .den
            .checked_mul(other.den)
            .ok_or(ReflectError::Overflow)?;
        Rational::new(num, den)
    }

    /// Compares `self` with `other`, or [`ReflectError::Overflow`].
    pub fn checked_cmp(self, other: Self) -> Result<core::cmp::Ordering, ReflectError> {
        let left = self
            .num
            .checked_mul(other.den)
            .ok_or(ReflectError::Overflow)?;
        let right = other
            .num
            .checked_mul(self.den)
            .ok_or(ReflectError::Overflow)?;
        Ok(left.cmp(&right))
    }
}

#[cfg(kani)]
#[allow(clippy::expect_used)]
mod kani_proofs {
    use super::*;

    /// A symbolic value with |x| < 2^8. The bound keeps the solver's problem small; the edge of the
    /// range is covered by `tests/rational_reduction.rs`.
    fn bounded() -> i128 {
        let x: i128 = kani::any();
        kani::assume(x > -(1 << 8) && x < (1 << 8));
        x
    }

    fn nonzero() -> i128 {
        let d = bounded();
        kani::assume(d != 0);
        d
    }

    /// The reduced form is in lowest terms, with a positive denominator, and has the same value.
    #[kani::proof]
    #[kani::unwind(32)]
    fn new_is_in_lowest_terms_and_keeps_the_value() {
        let num = bounded();
        let den = nonzero();
        let r = Rational::new(num, den).expect("nonzero denominator");
        assert!(r.denominator() > 0);
        assert_eq!(
            gcd(r.numerator().unsigned_abs(), r.denominator() as u128),
            1
        );
        assert_eq!(r.numerator() * den, num * r.denominator());
    }

    // The checked add and checked multiply harnesses were removed on 2026-11-16: each ran past two
    // hours in CBMC with no verdict. Their bounded exact checks are in tests/rational_exhaustive.rs.

    /// The comparison agrees with the sign of the exact difference.
    #[kani::proof]
    #[kani::unwind(32)]
    fn checked_cmp_agrees_with_the_exact_order() {
        let a = Rational::new(bounded(), nonzero()).expect("nonzero");
        let b = Rational::new(bounded(), nonzero()).expect("nonzero");
        let ordering = a.checked_cmp(b).expect("bounded operands do not overflow");
        let exact = (a.numerator() * b.denominator()).cmp(&(b.numerator() * a.denominator()));
        assert_eq!(ordering, exact);
    }
}
