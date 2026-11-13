// SPDX-License-Identifier: MIT OR Apache-2.0
//! CCSDS dual-basis (Berlekamp) symbols for the Reed-Solomon code, CCSDS 131.0-B-5 §4.3.9 and
//! Annex F.
//!
//! A conventional symbol `u` is the element `sum_j u_j alpha^j` of GF(2^8) with field polynomial
//! 0x187. CCSDS transmits the dual-basis representation instead: with basis `(1, b, ..., b^7)`,
//! `b = alpha^117`, the dual basis `(l_0, ..., l_7)` satisfies `Tr(l_i b^j) = delta_ij`, and the
//! symbol is `z_0 l_0 + ... + z_7 l_7`. Equivalently `z_c = Tr(x * b^c)`.
//!
//! Byte order: `z_0` is the most significant bit and is transmitted first (§4.3.9.2).
//!
//! The standard's codeword is the conventional codeword with each symbol mapped by this bijection
//! (Annex F), so this module maps symbols in and out and then uses the conventional
//! [`ReedSolomon`] code unchanged.

use crate::codec::{BlockDecoder, BlockEncoder, ErasureCoder};
use crate::error::{CodeError, DecodeReport};
use crate::rs::ReedSolomon;

/// Low byte of the field polynomial 0x187: `x^8 = x^7 + x^2 + x + 1`.
const REDUCTION: u8 = 0x87;
/// Exponent of the basis element `b = alpha^117` (§4.4.2).
const BASIS_EXPONENT: u32 = 117;

/// Multiplication in GF(2^8) with field polynomial 0x187, usable in const evaluation.
const fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut product = 0u8;
    while b != 0 {
        if b & 1 != 0 {
            product ^= a;
        }
        let carry = a & 0x80;
        a <<= 1;
        if carry != 0 {
            a ^= REDUCTION;
        }
        b >>= 1;
    }
    product
}

/// `alpha^e` by repeated multiplication by alpha = 2.
const fn alpha_pow(e: u32) -> u8 {
    let mut r = 1u8;
    let mut i = 0;
    while i < e {
        r = gf_mul(r, 2);
        i += 1;
    }
    r
}

/// Absolute trace `Tr(z) = sum_k z^(2^k)`. The result is 0 or 1.
const fn trace(z: u8) -> u8 {
    let mut sum = 0u8;
    let mut t = z;
    let mut k = 0;
    while k < 8 {
        sum ^= t;
        t = gf_mul(t, t);
        k += 1;
    }
    sum
}

/// `b^c` for `c` in `0..8`, with `b = alpha^117`.
const BASIS: [u8; 8] = {
    let mut out = [0u8; 8];
    let mut c = 0;
    while c < 8 {
        out[c] = alpha_pow(BASIS_EXPONENT * c as u32 % 255);
        c += 1;
    }
    out
};

/// Dual-basis representation of a conventional symbol.
const fn to_dual_raw(x: u8) -> u8 {
    let mut z = 0u8;
    let mut c = 0;
    while c < 8 {
        // Trace of a field element lies in GF(2); bit c of the dual form is z_c.
        z |= trace(gf_mul(x, BASIS[c])) << (7 - c);
        c += 1;
    }
    z
}

const fn build_to_dual() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut x = 0;
    while x < 256 {
        table[x] = to_dual_raw(x as u8);
        x += 1;
    }
    table
}

const fn build_from_dual(to: &[u8; 256]) -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut x = 0;
    while x < 256 {
        table[to[x] as usize] = x as u8;
        x += 1;
    }
    table
}

const TO_DUAL: [u8; 256] = build_to_dual();
const FROM_DUAL: [u8; 256] = build_from_dual(&TO_DUAL);

/// Maps a conventional symbol (coefficients of `alpha^7..alpha^0`, bit 7 = `alpha^7`) to its
/// dual-basis form (bit 7 = `z_0`).
pub fn conventional_to_dual(symbol: u8) -> u8 {
    TO_DUAL[usize::from(symbol)]
}

/// Inverse of [`conventional_to_dual`].
pub fn dual_to_conventional(symbol: u8) -> u8 {
    FROM_DUAL[usize::from(symbol)]
}

/// Largest codeword length of the CCSDS RS(255, k) code.
const N: usize = 255;

/// The CCSDS Reed-Solomon code with dual-basis symbols on its interface.
///
/// Messages and codewords use the transmitted (dual) symbol representation. Internally the
/// symbols are mapped to the conventional basis, encoded or decoded with [`ReedSolomon`], and
/// mapped back. A failed decode leaves the word unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DualBasisRs {
    code: ReedSolomon,
}

impl DualBasisRs {
    /// CCSDS RS(255, 223): 32 parity symbols, fcr 112, prim 11.
    pub fn ccsds_255_223() -> Result<Self, CodeError> {
        Ok(Self {
            code: ReedSolomon::ccsds_255_223()?,
        })
    }

    /// The underlying conventional-basis code.
    pub fn conventional(&self) -> &ReedSolomon {
        &self.code
    }

    fn check_len(&self, n: usize) -> Result<(), CodeError> {
        if n > N {
            return Err(CodeError::InvalidParameter {
                name: "codeword length",
            });
        }
        Ok(())
    }
}

impl BlockEncoder for DualBasisRs {
    /// Writes `message || parity` in the dual basis. `out.len()` must be `message.len() +
    /// parity_len()`.
    fn encode(&self, message: &[u8], out: &mut [u8]) -> Result<(), CodeError> {
        let n = message.len() + self.code.parity_len();
        self.check_len(n)?;
        if out.len() != n {
            return Err(CodeError::LengthMismatch {
                expected: n,
                actual: out.len(),
            });
        }
        let mut conv_msg = [0u8; N];
        for (c, &m) in conv_msg.iter_mut().zip(message) {
            *c = dual_to_conventional(m);
        }
        let mut conv = [0u8; N];
        self.code
            .encode(&conv_msg[..message.len()], &mut conv[..n])?;
        for (o, &c) in out.iter_mut().zip(&conv[..n]) {
            *o = conventional_to_dual(c);
        }
        Ok(())
    }
}

impl BlockDecoder for DualBasisRs {
    /// Corrects up to `parity_len() / 2` symbol errors in place, in the dual basis.
    fn decode(&self, word: &mut [u8]) -> Result<DecodeReport, CodeError> {
        self.correct(word, &[])
    }
}

impl ErasureCoder for DualBasisRs {
    /// Corrects errors and erasures in place, in the dual basis. Erasure positions are symbol
    /// indices and do not depend on the basis.
    fn decode_erasures(
        &self,
        word: &mut [u8],
        erasures: &[usize],
    ) -> Result<DecodeReport, CodeError> {
        self.correct(word, erasures)
    }
}

impl DualBasisRs {
    fn correct(&self, word: &mut [u8], erasures: &[usize]) -> Result<DecodeReport, CodeError> {
        let n = word.len();
        self.check_len(n)?;
        let mut conv = [0u8; N];
        for (c, &w) in conv.iter_mut().zip(word.iter()) {
            *c = dual_to_conventional(w);
        }
        let report = self.code.decode_with_erasures(&mut conv[..n], erasures)?;
        for (w, &c) in word.iter_mut().zip(&conv[..n]) {
            *w = conventional_to_dual(c);
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gf;

    /// Annex F, Figure F-1 matrix T, one row per conventional basis element alpha^7 .. alpha^0,
    /// each row written as the byte with z_0 as the most significant bit.
    const ANNEX_F_ROWS: [u8; 8] = [0x8D, 0xEF, 0xEC, 0x86, 0xFA, 0x99, 0xAF, 0x7B];

    /// Annex G, E = 16: exponents `e` with `G_i = alpha^e`, for i = 0..=16.
    const ANNEX_G_EXPONENTS: [u8; 17] = [
        0, 249, 59, 66, 4, 43, 126, 251, 97, 30, 3, 213, 50, 66, 170, 5, 24,
    ];

    #[test]
    fn annex_f_matrix_rows_match() {
        for (r, &expected) in ANNEX_F_ROWS.iter().enumerate() {
            let e = 7 - r;
            assert_eq!(
                conventional_to_dual(gf::alpha_pow(e as i64)),
                expected,
                "alpha^{e}"
            );
        }
    }

    #[test]
    fn dual_map_is_a_bijection_with_inverse() {
        for x in 0..=255u8 {
            assert_eq!(dual_to_conventional(conventional_to_dual(x)), x);
            assert_eq!(conventional_to_dual(dual_to_conventional(x)), x);
        }
        let mut seen = [false; 256];
        for x in 0..=255u8 {
            seen[usize::from(conventional_to_dual(x))] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn trace_is_zero_or_one() {
        for x in 0..=255u8 {
            assert!(trace(x) <= 1);
        }
    }

    #[test]
    fn annex_g_generator_coefficients_match() {
        // Encoding a message whose last symbol is 1 gives x^32 mod g(x) in the parity bytes,
        // which are the generator coefficients G_31 .. G_0 in order.
        let code = ReedSolomon::ccsds_255_223().expect("preset");
        let mut message = [0u8; 223];
        message[222] = 1;
        let mut word = [0u8; 255];
        code.encode(&message, &mut word).expect("encode");
        let parity = &word[223..];
        for (i, &e) in ANNEX_G_EXPONENTS.iter().enumerate() {
            assert_eq!(parity[31 - i], gf::alpha_pow(i64::from(e)), "G_{i}");
        }
    }

    #[test]
    fn dual_codeword_maps_back_to_a_conventional_codeword() {
        let code = DualBasisRs::ccsds_255_223().expect("preset");
        let message: [u8; 223] = core::array::from_fn(|i| (i * 13 + 7) as u8);
        let mut word = [0u8; 255];
        code.encode(&message, &mut word).expect("encode");
        let conv: [u8; 255] = core::array::from_fn(|i| dual_to_conventional(word[i]));
        let mut check = conv;
        let report = code.conventional().decode(&mut check).expect("decode");
        assert_eq!(report.corrected, 0, "mapped codeword is not conventional");
    }

    #[test]
    fn corrects_symbol_errors_in_the_dual_basis() {
        let code = DualBasisRs::ccsds_255_223().expect("preset");
        let message: [u8; 223] = core::array::from_fn(|i| (i * 31 + 5) as u8);
        let mut word = [0u8; 255];
        code.encode(&message, &mut word).expect("encode");
        let original = word;
        for k in 0..16 {
            word[k * 15] ^= 0x5A ^ k as u8;
        }
        let report = code.decode(&mut word).expect("correctable");
        assert_eq!(report.corrected, 16);
        assert_eq!(word, original);
    }

    #[test]
    fn failed_decode_leaves_the_word_unchanged() {
        let code = DualBasisRs::ccsds_255_223().expect("preset");
        let mut word: [u8; 255] = core::array::from_fn(|i| (i * 3) as u8);
        let before = word;
        assert!(code.decode(&mut word).is_err());
        assert_eq!(word, before);
    }
}

/// Kani proofs of the dual-basis maps, over all symbolic inputs.
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn dual_map_round_trips() {
        let x: u8 = kani::any();
        assert_eq!(dual_to_conventional(conventional_to_dual(x)), x);
        assert_eq!(conventional_to_dual(dual_to_conventional(x)), x);
    }

    #[kani::proof]
    fn dual_map_is_additive() {
        // The dual map is GF(2)-linear: addition (XOR) commutes with it.
        let a: u8 = kani::any();
        let b: u8 = kani::any();
        assert_eq!(
            conventional_to_dual(a ^ b),
            conventional_to_dual(a) ^ conventional_to_dual(b)
        );
    }
}
