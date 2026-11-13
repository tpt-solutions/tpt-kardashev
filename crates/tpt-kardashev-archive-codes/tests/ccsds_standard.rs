// SPDX-License-Identifier: MIT OR Apache-2.0
//! Checks taken from CCSDS 131.0-B-5 (September 2023) text, through the public API.
//!
//! The standard gives no codeword vectors. These checks use its explicit tables: the dual-basis
//! matrix of Annex F, the generator coefficients of Annex G, the attached sync marker (§9.2), the
//! symbol interleaving of §4.4.1 (Figure 4-2), and the convolutional connection vectors and G2
//! inverter of §3.3.

use tpt_kardashev_archive_codes::conv;
use tpt_kardashev_archive_codes::dual::conventional_to_dual;
use tpt_kardashev_archive_codes::interleave::interleave;
use tpt_kardashev_archive_codes::sync::attach_asm;
use tpt_kardashev_archive_codes::{BlockEncoder, ReedSolomon};

/// Multiplication by alpha in GF(2^8) with field polynomial F(x) = x^8 + x^7 + x^2 + x + 1
/// (§4.3.3). Independent of the crate's own field code.
fn times_alpha(a: u8) -> u8 {
    let carry = a & 0x80;
    let shifted = a << 1;
    if carry != 0 { shifted ^ 0x87 } else { shifted }
}

/// `alpha^e` by repeated multiplication by alpha.
fn alpha_pow(e: u32) -> u8 {
    let mut r = 1u8;
    for _ in 0..e {
        r = times_alpha(r);
    }
    r
}

#[test]
fn annex_f_dual_basis_matrix_rows() {
    // Row r of Annex F's matrix is the dual form of alpha^(7 - r), with z_0 as the most
    // significant bit. Conventional symbol 1 << j is the element alpha^j.
    let rows: [u8; 8] = [0x8D, 0xEF, 0xEC, 0x86, 0xFA, 0x99, 0xAF, 0x7B];
    for (r, &expected) in rows.iter().enumerate() {
        let j = 7 - r;
        assert_eq!(
            conventional_to_dual(1 << j),
            expected,
            "dual form of alpha^{j}"
        );
    }
}

#[test]
fn annex_g_generator_coefficients() {
    // Annex G, E = 16: G_i = alpha^e for i = 0..=16 (the rest follow from G_i = G_(32-i)).
    let exponents: [u32; 17] = [
        0, 249, 59, 66, 4, 43, 126, 251, 97, 30, 3, 213, 50, 66, 170, 5, 24,
    ];
    let code = ReedSolomon::ccsds_255_223().expect("preset");
    // With a 1 in the last message symbol, the parity bytes are G_31 .. G_0.
    let mut message = [0u8; 223];
    message[222] = 1;
    let mut word = [0u8; 255];
    code.encode(&message, &mut word).expect("encode");
    for (i, &e) in exponents.iter().enumerate() {
        assert_eq!(word[223 + 31 - i], alpha_pow(e), "G_{i} = alpha^{e}");
    }
}

#[test]
fn attached_sync_marker_is_the_standard_pattern() {
    // §9.2: ASM 1ACFFC1D, most significant byte first.
    let mut out = [0u8; 6];
    attach_asm(&[0xAA, 0x55], &mut out).expect("attach");
    assert_eq!(out, [0x1A, 0xCF, 0xFC, 0x1D, 0xAA, 0x55]);
}

#[test]
fn symbol_interleaving_follows_figure_4_2() {
    // Depth I = 5 codewords of n = 3 symbols. Symbol j of codeword i (value 10 i + j) is sent at
    // stream position j I + i, so the stream is d^1_1 d^2_1 ... d^5_1, d^1_2 ... (Figure 4-2).
    let depth = 5;
    let n = 3;
    let mut input = [0u8; 15];
    for i in 0..depth {
        for j in 0..n {
            input[i * n + j] = (10 * i + j) as u8;
        }
    }
    let mut out = [0u8; 15];
    interleave(&input, depth, &mut out).expect("interleave");
    let expected: [u8; 15] = [0, 10, 20, 30, 40, 1, 11, 21, 31, 41, 2, 12, 22, 32, 42];
    assert_eq!(out, expected);
}

#[test]
fn convolutional_connection_vectors_and_inverter() {
    // §3.3.1: G1 = 1111001 (171 octal), G2 = 1011011 (133 octal), G2 inverted. §3.3.2: the
    // output is C1(1), not C2(1), C1(2), not C2(2), ...
    // A single 1 followed by zeros shows each generator's taps, newest input first.
    let mut bits = [0u8; 7];
    bits[0] = 1;
    let mut out = [0u8; 14];
    conv::encode(&bits, &mut out).expect("encode");
    let c1 = [1u8, 1, 1, 1, 0, 0, 1];
    let c2 = [1u8, 0, 1, 1, 0, 1, 1];
    for t in 0..7 {
        assert_eq!(out[2 * t], c1[t], "C1 at {t}");
        assert_eq!(out[2 * t + 1], 1 - c2[t], "inverted C2 at {t}");
    }
}
