// SPDX-License-Identifier: MIT OR Apache-2.0
//! Elementary functions for `no_std` builds.
//!
//! `core` does not provide `sqrt`, `ln`, `exp` or `sin`, and this crate has no runtime
//! dependencies, so they are implemented here. They are deterministic and accurate to a few
//! units in the last place over the ranges this crate uses. They are not a general-purpose libm.

use core::f64::consts::{LN_2, SQRT_2, TAU};

// Cody-Waite split of ln 2 so that k * ln 2 is exact for the reduction in `exp`.
const LN_2_HI: f64 = 6.931_471_803_691_238e-1;
const LN_2_LO: f64 = 1.908_214_929_270_587_7e-10;

const EXP_MASK: u64 = 0x7ff;
const MANT_MASK: u64 = (1 << 52) - 1;
const ONE_BITS: u64 = 1023 << 52;

/// Returns 2^k, clamped to the normal exponent range.
fn pow2(k: i64) -> f64 {
    let k = k.clamp(-1022, 1023);
    f64::from_bits(((k + 1023) as u64) << 52)
}

/// Unbiased binary exponent of a normal, positive, finite `x`.
fn exponent(x: f64) -> i64 {
    ((x.to_bits() >> 52) & EXP_MASK) as i64 - 1023
}

/// Rounds to the nearest integer, ties away from zero, without `f64::round` (std-only).
fn round_i64(v: f64) -> i64 {
    if v >= 0.0 {
        (v + 0.5) as i64
    } else {
        (v - 0.5) as i64
    }
}

/// Square root by Newton iteration from an exponent-based seed.
pub(crate) fn sqrt(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 || x == f64::INFINITY {
        return x;
    }
    if x < f64::MIN_POSITIVE {
        return sqrt(x * pow2(100)) * pow2(-50);
    }
    let mut g = 1.5 * pow2(exponent(x).div_euclid(2));
    for _ in 0..8 {
        g = 0.5 * (g + x / g);
    }
    g
}

/// Natural logarithm from exponent extraction and an atanh series.
pub(crate) fn ln(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x == f64::INFINITY {
        return f64::INFINITY;
    }
    let (x, adjust) = if x < f64::MIN_POSITIVE {
        (x * pow2(100), -100.0 * LN_2)
    } else {
        (x, 0.0)
    };
    let mut e = exponent(x);
    let mut m = f64::from_bits((x.to_bits() & MANT_MASK) | ONE_BITS);
    if m > SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    // ln m = 2 atanh s with s = (m - 1) / (m + 1), |s| <= 0.1716 on this range.
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let mut term = s;
    let mut sum = 0.0;
    for k in 0..16 {
        sum += term / (2.0 * k as f64 + 1.0);
        term *= s2;
    }
    let ef = e as f64;
    ef * LN_2_HI + ef * LN_2_LO + 2.0 * sum + adjust
}

/// Exponential by range reduction to |r| <= ln 2 / 2 and a Taylor series.
pub(crate) fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x > 709.782_712_893_384 {
        return f64::INFINITY;
    }
    if x < -745.2 {
        return 0.0;
    }
    let k = round_i64(x / LN_2);
    let kf = k as f64;
    let r = (x - kf * LN_2_HI) - kf * LN_2_LO;
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..=24 {
        term *= r / f64::from(n);
        sum += term;
    }
    let half = k / 2;
    sum * pow2(half) * pow2(k - half)
}

/// Sine by reduction modulo 2 pi and a Taylor series. Accurate for moderate |x|.
pub(crate) fn sin(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    let k = round_i64(x / TAU) as f64;
    let r = x - k * TAU;
    let r2 = r * r;
    let mut term = r;
    let mut sum = r;
    for n in 1..=20 {
        let n2 = 2.0 * f64::from(n);
        term *= -r2 / (n2 * (n2 + 1.0));
        sum += term;
    }
    sum
}

/// Inverse hyperbolic tangent for |x| < 1.
pub(crate) fn atanh(x: f64) -> f64 {
    0.5 * ln((1.0 + x) / (1.0 - x))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, rel: f64) {
        let bound = rel * expected.abs().max(f64::MIN_POSITIVE);
        assert!(
            (actual - expected).abs() <= bound,
            "actual {actual}, expected {expected}"
        );
    }

    #[test]
    fn sqrt_matches_std() {
        for &x in &[1e-300, 1e-10, 0.5, 1.0, 2.0, 3.0, 1e8, 1e300] {
            close(sqrt(x), x.sqrt(), 1e-15);
        }
        assert_eq!(sqrt(0.0), 0.0);
        assert!(sqrt(-1.0).is_nan());
    }

    #[test]
    fn ln_and_exp_match_std() {
        for &x in &[1e-200, 0.001, 0.5, 1.0, 1.5, 2.0, 10.0, 1e10] {
            close(ln(x), x.ln(), 1e-14);
        }
        for &x in &[-700.0, -5.0, -0.3, 0.0, 0.3, 1.0, 5.0, 700.0] {
            close(exp(x), x.exp(), 1e-14);
        }
        close(exp(ln(3.7)), 3.7, 1e-14);
    }

    #[test]
    fn sin_matches_std() {
        for &x in &[-10.0, -3.0, -1.0, 0.0, 0.5, 1.0, 3.0, 6.0, 100.0] {
            assert!((sin(x) - x.sin()).abs() < 1e-13, "x = {x}");
        }
    }
}
