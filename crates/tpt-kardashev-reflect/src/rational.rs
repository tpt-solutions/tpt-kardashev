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

fn gcd(mut a: i128, mut b: i128) -> i128 {
    // Both arguments are non-negative and at most `i128::MAX` in magnitude, so this cannot overflow.
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
        let g = gcd(num.unsigned_abs().min(i128::MAX as u128) as i128, den);
        let g = if g == 0 { 1 } else { g };
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
