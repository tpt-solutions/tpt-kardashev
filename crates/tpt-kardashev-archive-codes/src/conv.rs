// SPDX-License-Identifier: MIT OR Apache-2.0
//! Rate-1/2, constraint-length-7 convolutional code with polynomials 171 and 133 (octal), as in
//! CCSDS 131.0-B-3 (draft), and a hard-decision Viterbi decoder. Known-answer vectors from the
//! standard are not yet checked.
//!
//! Bits are stored one per byte, with values 0 or 1. The encoder starts in the all-zero state.
//! It does not terminate the trellis, so the decoder chooses the best final state. The last few
//! bits of a block are therefore less reliable unless the caller appends zero tail bits.

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

/// Hard-decision Viterbi decoder for the code above.
///
/// `decisions` must hold at least `received.len() / 2` entries. It stores one 64-bit survivor
/// word per trellis step, so no allocation is needed. Returns the best final path metric, which
/// is the Hamming distance between the received word and the chosen codeword.
pub fn viterbi_decode(
    received: &[u8],
    out: &mut [u8],
    decisions: &mut [u64],
) -> Result<u32, CodeError> {
    if !received.len().is_multiple_of(2) {
        return Err(CodeError::LengthMismatch {
            expected: received.len() + 1,
            actual: received.len(),
        });
    }
    let steps = received.len() / 2;
    if out.len() != steps {
        return Err(CodeError::LengthMismatch {
            expected: steps,
            actual: out.len(),
        });
    }
    if decisions.len() < steps {
        return Err(CodeError::LengthMismatch {
            expected: steps,
            actual: decisions.len(),
        });
    }
    for &r in received {
        check_bit(r)?;
    }

    let mut metric = [INF; STATES];
    metric[0] = 0;
    for t in 0..steps {
        let r0 = received[2 * t];
        let r1 = received[2 * t + 1];
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
                let branch = u32::from(c1 != r0) + u32::from(c2 != r1);
                let candidate = m_s + branch;
                if candidate < next[ns] {
                    next[ns] = candidate;
                    // The predecessor of `ns` differs only in bit 0 of `s`; record which one won.
                    let b = (s & 1) as u64;
                    survivors = (survivors & !(1u64 << ns)) | (b << ns);
                }
            }
        }
        metric = next;
        decisions[t] = survivors;
    }

    let (mut state, best) = metric
        .iter()
        .enumerate()
        .min_by_key(|&(_, &m)| m)
        .map(|(s, &m)| (s, m))
        .unwrap_or((0, INF));
    if best >= INF {
        return Err(CodeError::Uncorrectable);
    }
    // Trace back: the input bit at step t is bit 5 of the state after step t.
    for t in (0..steps).rev() {
        out[t] = ((state >> 5) & 1) as u8;
        let b = ((decisions[t] >> state) & 1) as usize;
        state = ((state & 0x1F) << 1) | b;
    }
    Ok(best)
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
        // At step t the register holds the 1 at bit 6 - t. G1 = 1111001, G2 = 1011011 in octal taps.
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
    fn rejects_bad_symbols_and_lengths() {
        let mut out = [0u8; 2];
        assert_eq!(encode(&[2], &mut out), Err(CodeError::InvalidSymbol));
        assert!(encode(&[0, 1], &mut out[..1]).is_err());
        let mut decisions = [0u64; 1];
        assert!(viterbi_decode(&[0, 1, 1], &mut [0u8; 1], &mut decisions).is_err());
    }
}
