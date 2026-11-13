// SPDX-License-Identifier: MIT OR Apache-2.0
//! The two CRCs BPv7 defines for blocks: CRC-16/X-25 (CRC type 1) and CRC-32C (CRC type 2).
//!
//! Both are reflected CRCs: each byte is processed least-significant bit first.

/// CRC-16/X-25: polynomial 0x1021 (reflected 0x8408), initial value 0xFFFF, final XOR 0xFFFF.
/// Check value for "123456789" is 0x906E.
pub fn crc16_x25(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0x8408
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xFFFF
}

/// CRC-32C (Castagnoli): polynomial 0x1EDC6F41 (reflected 0x82F63B78), initial value and final XOR
/// 0xFFFFFFFF. Check value for "123456789" is 0xE3069283.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0x82F6_3B78
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_x25_check_value() {
        assert_eq!(crc16_x25(b"123456789"), 0x906E);
    }

    #[test]
    fn crc32c_check_value() {
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
    }

    #[test]
    fn empty_input() {
        assert_eq!(crc16_x25(&[]), 0x0000);
        assert_eq!(crc32c(&[]), 0x0000_0000);
    }
}
