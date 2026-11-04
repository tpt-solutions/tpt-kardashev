// SPDX-License-Identifier: MIT OR Apache-2.0
//! CRC variants implemented in-crate, bitwise, without tables.
//!
//! Each function is checked against the standard check value for the input `"123456789"`
//! (see the tests). Parameters are listed with each function.

/// CRC-16/CCITT-FALSE: poly 0x1021, init 0xFFFF, no reflection, no final XOR. Check value 0x29B1.
pub fn crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// CRC-32 (IEEE 802.3): poly 0x04C11DB7 reflected to 0xEDB88320, init and final XOR 0xFFFFFFFF.
/// Check value 0xCBF43926.
pub fn crc32_ieee(data: &[u8]) -> u32 {
    crc32_reflected(data, 0xEDB8_8320)
}

/// CRC-32C (Castagnoli): poly 0x1EDC6F41 reflected to 0x82F63B78, init and final XOR 0xFFFFFFFF.
/// Check value 0xE3069283. Used by BPv7 bundle CRCs.
pub fn crc32c(data: &[u8]) -> u32 {
    crc32_reflected(data, 0x82F6_3B78)
}

fn crc32_reflected(data: &[u8], poly: u32) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ poly
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECK: &[u8] = b"123456789";

    #[test]
    fn crc16_check_value() {
        assert_eq!(crc16_ccitt_false(CHECK), 0x29B1);
    }

    #[test]
    fn crc32_check_values() {
        assert_eq!(crc32_ieee(CHECK), 0xCBF4_3926);
        assert_eq!(crc32c(CHECK), 0xE306_9283);
    }

    #[test]
    fn detects_single_bit_flips() {
        let mut data = *b"archival frame payload";
        let good = crc32c(&data);
        for bit in 0..(data.len() * 8) {
            data[bit / 8] ^= 1 << (bit % 8);
            assert_ne!(crc32c(&data), good, "bit {bit}");
            data[bit / 8] ^= 1 << (bit % 8);
        }
    }
}
