// SPDX-License-Identifier: MIT OR Apache-2.0
//! Arithmetic in GF(2^8) with primitive polynomial `x^8 + x^7 + x^2 + x + 1` (0x187) and
//! primitive element `alpha = x` (0x02), the field used by CCSDS Reed-Solomon in the
//! conventional (non-dual) basis.
//!
//! Log and antilog tables are built at compile time. Each lookup is a table access, so the code
//! is constant-time only in the sense that no branch depends on secret data beyond zero tests.

/// The field's primitive polynomial, including the x^8 term.
pub const PRIMITIVE_POLY: u32 = 0x187;

const fn build_tables() -> ([u8; 512], [u8; 256]) {
    let mut exp = [0u8; 512];
    let mut log = [0u8; 256];
    let mut x: u32 = 1;
    let mut i = 0;
    while i < 255 {
        exp[i] = x as u8;
        log[x as usize] = i as u8;
        x <<= 1;
        if x & 0x100 != 0 {
            x ^= PRIMITIVE_POLY;
        }
        i += 1;
    }
    // Duplicate so that products of two logs index without a modulo.
    while i < 512 {
        exp[i] = exp[i - 255];
        i += 1;
    }
    (exp, log)
}

const TABLES: ([u8; 512], [u8; 256]) = build_tables();
const EXP: [u8; 512] = TABLES.0;
const LOG: [u8; 256] = TABLES.1;

/// Field multiplication.
pub(crate) fn mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        0
    } else {
        EXP[LOG[a as usize] as usize + LOG[b as usize] as usize]
    }
}

/// Multiplicative inverse. Returns 0 for 0, which callers must not use as a divisor.
pub(crate) fn inv(a: u8) -> u8 {
    if a == 0 {
        0
    } else {
        EXP[255 - LOG[a as usize] as usize]
    }
}

/// `alpha^e` for any integer exponent.
pub(crate) fn alpha_pow(e: i64) -> u8 {
    EXP[e.rem_euclid(255) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_generates_the_whole_multiplicative_group() {
        let mut seen = [false; 256];
        for &v in &EXP[..255] {
            assert_ne!(v, 0);
            assert!(!seen[v as usize], "repeat {v}");
            seen[v as usize] = true;
        }
        assert_eq!(EXP[255], 1);
    }

    #[test]
    fn inverse_and_multiplication_laws() {
        for a in 1..=255u8 {
            assert_eq!(mul(a, inv(a)), 1, "a = {a}");
        }
        // Commutativity and distributivity on a sample.
        for a in [3u8, 77, 200, 255] {
            for b in [5u8, 91, 128] {
                for c in [9u8, 250] {
                    assert_eq!(mul(a, b), mul(b, a));
                    assert_eq!(mul(a, b ^ c), mul(a, b) ^ mul(a, c));
                }
            }
        }
    }

    #[test]
    fn alpha_pow_wraps() {
        assert_eq!(alpha_pow(0), 1);
        assert_eq!(alpha_pow(255), 1);
        assert_eq!(alpha_pow(-1), alpha_pow(254));
        assert_eq!(mul(alpha_pow(7), alpha_pow(-7)), 1);
    }
}
