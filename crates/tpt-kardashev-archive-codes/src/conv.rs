// SPDX-License-Identifier: MIT OR Apache-2.0
//! Rate-1/2, constraint-length-7 convolutional code with polynomials 171 and 133 (octal), as in
//! CCSDS 131.0-B-3 (draft), with hard- and soft-decision Viterbi decoders. Known-answer vectors
//! from the standard are not yet checked.
//!
//! Bits are stored one per byte, with values 0 or 1. The encoder starts in the all-zero state
//! and does not terminate the trellis; the decoder chooses the best final state.
//!
//! Soft input is one byte per coded bit: 0 means certainly 0, 255 means certainly 1, and 128
//! means no information (an erasure).

use crate::error::CodeError;

/// Generator polynomial 1 (171 octal), in the register layout used here.
const G1: u8 = 0o171;
/// Generator polynomial 2 (133 octal).
const G2: u8 = 0o133;
const STATES: usize = 64;
const INF: u32 = u32::MAX / 4;

fn parity(v: u8) -> u8 {
    v.count_ones() as u8 & 1
}

/// Output pair for a 7-bit register `r`, with the newest input in bit 6.
fn outputs(r: u8) -> (u8, u8) {
    (parity(r & G1), parity(r & G2))
}

fn check_bit(b: u8) -> Result<u8, CodeError> {
    if b > 1 {
        return Err(CodeError::InvalidSymbol);
    }
    Ok(b)
}

/// Encodes `bits` into `out`, two output bits per input bit: `out.len()` must be `2 * bits.len()`.
pub fn encode(bits: &[u8], out: &mut [u8]) -> Result<(), CodeError> {
    if out.len() != 2 * bits.len() {
        return Err(CodeError::LengthMismatch {
            expected: 2 * bits.len(),
            actual: out.len(),
        });
    }
    let mut state: u8 = 0;
    for (t, &b) in bits.iter().enumerate() {
        let u = check_bit(b)?;
        let r = (u << 6) | state;
        let (c1, c2) = outputs(r);
        out[2 * t] = c1;
        out[2 * t + 1] = c2;
        state = r >> 1;
    }
    Ok(())
}

fn check_lengths(
    coded_len: usize,
    out_len: usize,
    decisions_len: usize,
) -> Result<usize, CodeError> {
    if !coded_len.is_multiple_of(2) {
        return Err(CodeError::LengthMismatch {
            expected: coded_len + 1,
            actual: coded_len,
        });
    }
    let steps = coded_len / 2;
    if out_len != steps {
        return Err(CodeError::LengthMismatch {
            expected: steps,
            actual: out_len,
        });
    }
    if decisions_len < steps {
        return Err(CodeError::LengthMismatch {
            expected: steps,
            actual: decisions_len,
        });
    }
    Ok(steps)
}

/// Hard-decision Viterbi decoder.
///
/// `decisions` must hold at least `received.len() / 2` entries. It stores one 64-bit survivor
/// word per trellis step, so no allocation is needed. Returns the Hamming distance between the
/// received word and the chosen codeword.
pub fn viterbi_decode(
    received: &[u8],
    out: &mut [u8],
    decisions: &mut [u64],
) -> Result<u32, CodeError> {
    let steps = check_lengths(received.len(), out.len(), decisions.len())?;
    for &r in received {
        check_bit(r)?;
    }
    let branch = |t: usize, c1: u8, c2: u8| -> u32 {
        u32::from(c1 != received[2 * t]) + u32::from(c2 != received[2 * t + 1])
    };
    run(steps, branch, out, decisions)
}

/// Soft-decision Viterbi decoder. Each input byte is a confidence, 0 (certain 0) to 255 (certain 1).
///
/// The branch cost for a coded bit is the distance from its confidence to the expected bit:
/// `s` if the expected bit is 0, `255 - s` if it is 1. Erasures (128) cost the same either way.
pub fn viterbi_decode_soft(
    soft: &[u8],
    out: &mut [u8],
    decisions: &mut [u64],
) -> Result<u32, CodeError> {
    let steps = check_lengths(soft.len(), out.len(), decisions.len())?;
    let cost = |s: u8, expected: u8| -> u32 {
        let s = u32::from(s);
        if expected == 0 { s } else { 255 - s }
    };
    let branch =
        |t: usize, c1: u8, c2: u8| -> u32 { cost(soft[2 * t], c1) + cost(soft[2 * t + 1], c2) };
    run(steps, branch, out, decisions)
}

/// Shared Viterbi core: forward pass with metric normalisation, then traceback.
fn run(
    steps: usize,
    branch: impl Fn(usize, u8, u8) -> u32,
    out: &mut [u8],
    decisions: &mut [u64],
) -> Result<u32, CodeError> {
    let mut metric = [INF; STATES];
    metric[0] = 0;
    // Metrics are normalised each step by subtracting the minimum, which keeps them small over
    // arbitrarily long blocks. This offset accumulates the subtracted minima to give the true cost.
    let mut offset: u64 = 0;
    for (t, decision) in decisions.iter_mut().enumerate().take(steps) {
        let mut next = [INF; STATES];
        let mut survivors: u64 = 0;
        for (s, &m_s) in metric.iter().enumerate() {
            if m_s >= INF {
                continue;
            }
            for u in 0..2u8 {
                let r = (u << 6) | s as u8;
                let ns = usize::from(r >> 1);
                let (c1, c2) = outputs(r);
                let candidate = m_s + branch(t, c1, c2);
                if candidate < next[ns] {
                    next[ns] = candidate;
                    // The predecessor of `ns` differs only in bit 0 of `s`; record which one won.
                    let b = (s & 1) as u64;
                    survivors = (survivors & !(1u64 << ns)) | (b << ns);
                }
            }
        }
        let min = next.iter().copied().min().unwrap_or(INF);
        if min >= INF {
            return Err(CodeError::Uncorrectable);
        }
        for m in next.iter_mut().filter(|m| **m < INF) {
            *m -= min;
        }
        offset += u64::from(min);
        metric = next;
        *decision = survivors;
    }

    let (mut state, best_normalised) = metric
        .iter()
        .enumerate()
        .min_by_key(|&(_, &m)| m)
        .map(|(s, &m)| (s, m))
        .unwrap_or((0, INF));
    if best_normalised >= INF {
        return Err(CodeError::Uncorrectable);
    }
    let total = offset + u64::from(best_normalised);
    // Trace back: the input bit at step t is bit 5 of the state after step t.
    for t in (0..steps).rev() {
        out[t] = ((state >> 5) & 1) as u8;
        let b = ((decisions[t] >> state) & 1) as usize;
        state = ((state & 0x1F) << 1) | b;
    }
    Ok(u32::try_from(total).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pseudo_bits(seed: u64, n: usize) -> std::vec::Vec<u8> {
        let mut x = seed;
        (0..n)
            .map(|_| {
                x = x
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                ((x >> 33) & 1) as u8
            })
            .collect()
    }

    #[test]
    fn encodes_known_impulse_response() {
        // A single 1 followed by zeros: outputs equal the generator taps, read from the newest bit.
        let mut bits = [0u8; 7];
        bits[0] = 1;
        let mut out = [0u8; 14];
        encode(&bits, &mut out).expect("ok");
        // G1 = 171 octal = 1111001, G2 = 133 octal = 1011011.
        let g1_taps: [u8; 7] = [1, 1, 1, 1, 0, 0, 1];
        let g2_taps: [u8; 7] = [1, 0, 1, 1, 0, 1, 1];
        for t in 0..7 {
            assert_eq!(out[2 * t], g1_taps[t], "g1 step {t}");
            assert_eq!(out[2 * t + 1], g2_taps[t], "g2 step {t}");
        }
    }

    #[test]
    fn noiseless_round_trip() {
        let bits = pseudo_bits(7, 200);
        let mut coded = std::vec![0u8; 400];
        encode(&bits, &mut coded).expect("ok");
        let mut decoded = std::vec![0u8; 200];
        let mut decisions = std::vec![0u64; 200];
        let metric = viterbi_decode(&coded, &mut decoded, &mut decisions).expect("ok");
        assert_eq!(metric, 0);
        assert_eq!(decoded, bits);
    }

    #[test]
    fn corrects_sparse_channel_errors() {
        // Free distance is 10, so isolated errors spaced well apart are corrected.
        let bits = pseudo_bits(11, 300);
        let mut coded = std::vec![0u8; 600];
        encode(&bits, &mut coded).expect("ok");
        for pos in (5..coded.len()).step_by(40) {
            coded[pos] ^= 1;
        }
        let mut decoded = std::vec![0u8; 300];
        let mut decisions = std::vec![0u64; 300];
        viterbi_decode(&coded, &mut decoded, &mut decisions).expect("ok");
        assert_eq!(decoded, bits);
    }

    #[test]
    fn soft_decoder_matches_hard_on_clean_input() {
        let bits = pseudo_bits(21, 150);
        let mut coded = std::vec![0u8; 300];
        encode(&bits, &mut coded).expect("ok");
        let soft: std::vec::Vec<u8> = coded
            .iter()
            .map(|&b| if b == 1 { 255 } else { 0 })
            .collect();
        let mut decoded = std::vec![0u8; 150];
        let mut decisions = std::vec![0u64; 150];
        let metric = viterbi_decode_soft(&soft, &mut decoded, &mut decisions).expect("ok");
        assert_eq!(metric, 0);
        assert_eq!(decoded, bits);
    }

    #[test]
    fn soft_decoder_tolerates_erasures() {
        // Erase every fifth coded symbol: no information there, but the code still determines the input.
        let bits = pseudo_bits(31, 200);
        let mut coded = std::vec![0u8; 400];
        encode(&bits, &mut coded).expect("ok");
        let soft: std::vec::Vec<u8> = coded
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                if i % 5 == 0 {
                    128
                } else if b == 1 {
                    255
                } else {
                    0
                }
            })
            .collect();
        let mut decoded = std::vec![0u8; 200];
        let mut decisions = std::vec![0u64; 200];
        viterbi_decode_soft(&soft, &mut decoded, &mut decisions).expect("ok");
        assert_eq!(decoded, bits);
    }

    #[test]
    fn soft_decoder_recovers_weak_wrong_bits() {
        // Flip some symbols but mark them as unreliable, with confidence close to the midpoint.
        let bits = pseudo_bits(41, 200);
        let mut coded = std::vec![0u8; 400];
        encode(&bits, &mut coded).expect("ok");
        let mut soft: std::vec::Vec<u8> = coded
            .iter()
            .map(|&b| if b == 1 { 255 } else { 0 })
            .collect();
        for pos in (3..soft.len()).step_by(17) {
            soft[pos] = if soft[pos] == 255 { 140 } else { 115 };
        }
        let mut decoded = std::vec![0u8; 200];
        let mut decisions = std::vec![0u64; 200];
        viterbi_decode_soft(&soft, &mut decoded, &mut decisions).expect("ok");
        assert_eq!(decoded, bits);
    }

    #[test]
    fn long_blocks_do_not_overflow() {
        // A long block: checks that decoding stays correct over many normalised steps.
        let n = 200_000;
        let bits = pseudo_bits(51, n);
        let mut coded = std::vec![0u8; 2 * n];
        encode(&bits, &mut coded).expect("ok");
        let soft: std::vec::Vec<u8> = coded
            .iter()
            .map(|&b| if b == 1 { 254 } else { 1 })
            .collect();
        let mut decoded = std::vec![0u8; n];
        let mut decisions = std::vec![0u64; n];
        viterbi_decode_soft(&soft, &mut decoded, &mut decisions).expect("ok");
        assert_eq!(decoded, bits);
    }

    #[test]
    fn rejects_bad_symbols_and_lengths() {
        let mut out = [0u8; 2];
        assert_eq!(encode(&[2], &mut out), Err(CodeError::InvalidSymbol));
        assert!(encode(&[0, 1], &mut out[..1]).is_err());
        let mut decisions = [0u64; 1];
        assert!(viterbi_decode(&[0, 1, 1], &mut [0u8; 1], &mut decisions).is_err());
        assert!(viterbi_decode(&[0, 2], &mut [0u8; 1], &mut decisions).is_err());
    }
}
