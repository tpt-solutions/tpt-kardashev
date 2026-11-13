// SPDX-License-Identifier: MIT OR Apache-2.0
//! Self-Delimiting Numeric Values (RFC 5326 §2, after RFC 6256).
//!
//! The value is written big-endian in groups of 7 bits. The most significant bit of each octet is
//! a continuation flag: 1 means more octets follow, 0 marks the last octet. Values are written in
//! the shortest form. The decoder rejects a leading zero group (an encoding such as `0x80 0x01`)
//! and any value that does not fit in 64 bits, so every value has exactly one encoding.

use crate::error::Error;

/// Largest number of octets a 64-bit SDNV can occupy.
pub const MAX_LEN: usize = 10;

/// Decodes one SDNV from the start of `buf`. Returns the value and the number of octets used.
pub fn decode(buf: &[u8]) -> Result<(u64, usize), Error> {
    let mut value: u64 = 0;
    for (i, &octet) in buf.iter().enumerate() {
        if i >= MAX_LEN {
            return Err(Error::Ltp {
                what: "SDNV longer than 64 bits",
            });
        }
        if i == 0 && octet == 0x80 {
            return Err(Error::Ltp {
                what: "SDNV is not in shortest form",
            });
        }
        if value >> 57 != 0 {
            return Err(Error::Ltp {
                what: "SDNV does not fit in 64 bits",
            });
        }
        value = (value << 7) | u64::from(octet & 0x7F);
        if octet & 0x80 == 0 {
            return Ok((value, i + 1));
        }
    }
    Err(Error::Truncated)
}

/// Appends the SDNV encoding of `value` to `out`.
#[cfg(feature = "alloc")]
pub fn encode(value: u64, out: &mut alloc::vec::Vec<u8>) {
    let mut groups = [0u8; MAX_LEN];
    let mut n = 0usize;
    let mut v = value;
    loop {
        groups[n] = (v & 0x7F) as u8;
        n += 1;
        v >>= 7;
        if v == 0 {
            break;
        }
    }
    for i in (0..n).rev() {
        let continuation = if i > 0 { 0x80 } else { 0x00 };
        out.push(groups[i] | continuation);
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn enc(v: u64) -> Vec<u8> {
        let mut out = Vec::new();
        encode(v, &mut out);
        out
    }

    #[test]
    fn known_encodings() {
        assert_eq!(enc(0), [0x00]);
        assert_eq!(enc(0x7F), [0x7F]);
        assert_eq!(enc(0x80), [0x81, 0x00]);
        assert_eq!(enc(0x3FFF), [0xFF, 0x7F]);
        assert_eq!(enc(0x4000), [0x81, 0x80, 0x00]);
        assert_eq!(
            enc(u64::MAX),
            [0x81, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]
        );
    }

    #[test]
    fn decode_inverts_encode() {
        let mut x: u64 = 0x1F2E_3D4C_5B6A_7988;
        for shift in 0..64 {
            x = x.rotate_left(5).wrapping_add(shift);
            let bytes = enc(x >> shift);
            assert_eq!(decode(&bytes), Ok((x >> shift, bytes.len())));
        }
    }

    #[test]
    fn decode_reads_only_its_own_octets() {
        assert_eq!(decode(&[0x81, 0x00, 0xAA]), Ok((128, 2)));
    }

    #[test]
    fn non_minimal_and_overflowing_forms_are_rejected() {
        assert!(decode(&[0x80, 0x01]).is_err());
        assert!(decode(&[0x80]).is_err());
        // 11 octets: longer than any 64-bit value.
        assert!(decode(&[0xFF; 11]).is_err());
        // Ten octets whose value needs 65 bits.
        assert!(decode(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]).is_err());
        assert_eq!(decode(&[0x81, 0x80]), Err(Error::Truncated));
    }
}
