// SPDX-License-Identifier: MIT OR Apache-2.0
//! Block interleaving: `depth` codewords of length `n` are written column by column.
//!
//! Input is `depth` codewords stored back to back, codeword `i` occupying `input[i*n .. (i+1)*n]`.
//! Output position `j * depth + i` holds symbol `j` of codeword `i`. A burst of up to `depth`
//! consecutive symbols on the channel then hits each codeword at most once.

use crate::error::CodeError;

fn check(len: usize, depth: usize, out_len: usize) -> Result<usize, CodeError> {
    if depth == 0 {
        return Err(CodeError::InvalidParameter { name: "depth" });
    }
    if !len.is_multiple_of(depth) {
        return Err(CodeError::LengthMismatch {
            expected: len - len % depth + depth,
            actual: len,
        });
    }
    if out_len != len {
        return Err(CodeError::LengthMismatch {
            expected: len,
            actual: out_len,
        });
    }
    Ok(len / depth)
}

/// Interleaves `depth` codewords into `out`.
pub fn interleave(input: &[u8], depth: usize, out: &mut [u8]) -> Result<(), CodeError> {
    let n = check(input.len(), depth, out.len())?;
    for i in 0..depth {
        for j in 0..n {
            out[j * depth + i] = input[i * n + j];
        }
    }
    Ok(())
}

/// Reverses `interleave`.
pub fn deinterleave(input: &[u8], depth: usize, out: &mut [u8]) -> Result<(), CodeError> {
    let n = check(input.len(), depth, out.len())?;
    for i in 0..depth {
        for j in 0..n {
            out[i * n + j] = input[j * depth + i];
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let input: [u8; 24] = core::array::from_fn(|i| i as u8);
        let mut mixed = [0u8; 24];
        let mut back = [0u8; 24];
        interleave(&input, 4, &mut mixed).expect("ok");
        assert_ne!(mixed, input);
        deinterleave(&mixed, 4, &mut back).expect("ok");
        assert_eq!(back, input);
    }

    #[test]
    fn burst_hits_each_codeword_at_most_once() {
        // Four codewords of length 6, tagged by index.
        let depth = 4;
        let n = 6;
        let mut input = [0u8; 24];
        for i in 0..depth {
            for j in 0..n {
                input[i * n + j] = i as u8;
            }
        }
        let mut channel = [0u8; 24];
        interleave(&input, depth, &mut channel).expect("ok");
        // A burst of `depth` consecutive channel symbols.
        for s in &mut channel[8..8 + depth] {
            *s = 0xFF;
        }
        let mut received = [0u8; 24];
        deinterleave(&channel, depth, &mut received).expect("ok");
        for i in 0..depth {
            let hits = received[i * n..(i + 1) * n]
                .iter()
                .filter(|&&s| s == 0xFF)
                .count();
            assert!(hits <= 1, "codeword {i} hit {hits} times");
        }
    }

    #[test]
    fn rejects_bad_lengths() {
        let mut out = [0u8; 5];
        assert!(interleave(&[0u8; 5], 2, &mut out).is_err());
        assert!(interleave(&[0u8; 4], 0, &mut out[..4]).is_err());
    }
}
