// SPDX-License-Identifier: MIT OR Apache-2.0
//! Attached sync marker (ASM) insertion and detection.
//!
//! The CCSDS ASM for TM frames is `0x1ACFFC1D`. This module works on byte-aligned data only.
//! Bit-level frame synchronisation is not implemented yet.

use crate::error::CodeError;

/// The 32-bit CCSDS attached sync marker, most significant byte first.
pub const CCSDS_ASM: [u8; 4] = [0x1A, 0xCF, 0xFC, 0x1D];

/// Writes `ASM || payload` to `out`. `out.len()` must be `payload.len() + 4`.
pub fn attach_asm(payload: &[u8], out: &mut [u8]) -> Result<(), CodeError> {
    let expected = payload.len() + CCSDS_ASM.len();
    if out.len() != expected {
        return Err(CodeError::LengthMismatch {
            expected,
            actual: out.len(),
        });
    }
    out[..CCSDS_ASM.len()].copy_from_slice(&CCSDS_ASM);
    out[CCSDS_ASM.len()..].copy_from_slice(payload);
    Ok(())
}

/// Finds the first byte offset where the ASM matches with at most `max_bit_errors` flipped bits.
pub fn find_asm(data: &[u8], max_bit_errors: u32) -> Option<usize> {
    if data.len() < CCSDS_ASM.len() {
        return None;
    }
    (0..=data.len() - CCSDS_ASM.len()).find(|&offset| {
        let window = &data[offset..offset + CCSDS_ASM.len()];
        let differing: u32 = window
            .iter()
            .zip(CCSDS_ASM.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        differing <= max_bit_errors
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_and_find() {
        let payload = [1u8, 2, 3, 4, 5];
        let mut framed = [0u8; 9];
        attach_asm(&payload, &mut framed).expect("ok");
        assert_eq!(find_asm(&framed, 0), Some(0));
        let mut noisy = [0xEEu8; 3].to_vec();
        noisy.extend_from_slice(&framed);
        assert_eq!(find_asm(&noisy, 0), Some(3));
    }

    #[test]
    fn tolerates_bit_errors_up_to_limit() {
        let mut framed = [0u8; 4];
        framed.copy_from_slice(&CCSDS_ASM);
        framed[1] ^= 0b0000_0011; // two bit errors
        assert_eq!(find_asm(&framed, 1), None);
        assert_eq!(find_asm(&framed, 2), Some(0));
    }

    #[test]
    fn short_input_has_no_marker() {
        assert_eq!(find_asm(&[0x1A, 0xCF], 0), None);
    }
}
