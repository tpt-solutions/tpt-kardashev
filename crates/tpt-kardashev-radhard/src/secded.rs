// SPDX-License-Identifier: MIT OR Apache-2.0
//! SECDED(72, 64): single-error-correcting, double-error-detecting extended Hamming code.
//!
//! Layout in bit positions 0..72:
//! - position 0 is the overall parity bit, making the total parity of all 72 bits even;
//! - positions 1..=71 form a Hamming code. Positions that are powers of two (1, 2, 4, ... 64) are
//!   check bits; the remaining 64 positions carry the data, least significant data bit first.
//!
//! The Hamming syndrome of a single error is the position of the flipped bit. A non-zero syndrome
//! with even overall parity is a double error.

use crate::error::Error;

/// Bits in a codeword.
pub const CODE_BITS: u32 = 72;
/// Data bits per codeword.
pub const DATA_BITS: u32 = 64;

const HAMMING_MAX: u32 = 71;

const fn data_positions() -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut p = 1u32;
    let mut i = 0;
    while p <= HAMMING_MAX {
        if !p.is_power_of_two() {
            out[i] = p as u8;
            i += 1;
        }
        p += 1;
    }
    out
}

const DATA_POS: [u8; 64] = data_positions();

/// A 72-bit codeword. Bit `p` of the raw value is position `p`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Codeword72(u128);

/// Outcome of a successful decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outcome {
    /// No error.
    Clean,
    /// One bit was in error and has been corrected. `position` is the codeword position.
    Corrected {
        /// Position of the corrected bit, 0 to 71.
        position: u32,
    },
}

const MASK: u128 = (1u128 << CODE_BITS) - 1;

impl Codeword72 {
    /// The raw 72 bits.
    pub fn raw(&self) -> u128 {
        self.0
    }

    /// Builds a codeword from raw bits. Rejects values with bits above position 71.
    pub fn from_raw(raw: u128) -> Result<Self, Error> {
        if raw & !MASK != 0 {
            return Err(Error::BitOutOfRange {
                position: 127 - raw.leading_zeros(),
            });
        }
        Ok(Self(raw))
    }

    /// Flips the bit at `position` (0 to 71). Used for fault injection.
    pub fn flip(&mut self, position: u32) -> Result<(), Error> {
        if position >= CODE_BITS {
            return Err(Error::BitOutOfRange { position });
        }
        self.0 ^= 1u128 << position;
        Ok(())
    }
}

/// Hamming syndrome: XOR of the positions of all set bits in 1..=71.
fn syndrome(bits: u128) -> u32 {
    let mut s = 0u32;
    let mut rest = (bits >> 1) & ((1u128 << HAMMING_MAX) - 1);
    while rest != 0 {
        let p = rest.trailing_zeros() + 1;
        s ^= p;
        rest &= rest - 1;
    }
    s
}

/// Encodes 64 data bits.
pub fn encode(data: u64) -> Codeword72 {
    let mut bits: u128 = 0;
    for (i, &p) in DATA_POS.iter().enumerate() {
        if (data >> i) & 1 == 1 {
            bits |= 1u128 << p;
        }
    }
    // Setting the check bits to the syndrome of the data zeroes the Hamming syndrome.
    let s = syndrome(bits);
    let mut check = 0u128;
    for k in 0..7 {
        if (s >> k) & 1 == 1 {
            check |= 1u128 << (1u32 << k);
        }
    }
    bits |= check;
    // Overall parity over positions 1..=71 makes the total parity even.
    if bits.count_ones() & 1 == 1 {
        bits |= 1;
    }
    Codeword72(bits)
}

/// Decodes a codeword, correcting one error and detecting two.
pub fn decode(cw: Codeword72) -> Result<(u64, Outcome), Error> {
    let s = syndrome(cw.0);
    let odd = cw.0.count_ones() & 1 == 1;
    let (bits, outcome) = match (s, odd) {
        (0, false) => (cw.0, Outcome::Clean),
        (0, true) => (cw.0 ^ 1, Outcome::Corrected { position: 0 }),
        (s, true) if s <= HAMMING_MAX => (cw.0 ^ (1u128 << s), Outcome::Corrected { position: s }),
        (_, true) => return Err(Error::Uncorrectable),
        (_, false) => return Err(Error::DoubleBitError),
    };
    let mut data = 0u64;
    for (i, &p) in DATA_POS.iter().enumerate() {
        if (bits >> p) & 1 == 1 {
            data |= 1u64 << i;
        }
    }
    Ok((data, outcome))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_samples() -> [u64; 8] {
        [
            0,
            u64::MAX,
            1,
            0x8000_0000_0000_0000,
            0xDEAD_BEEF_0BAD_F00D,
            0x0123_4567_89AB_CDEF,
            0x5555_5555_5555_5555,
            0xAAAA_AAAA_AAAA_AAAA,
        ]
    }

    #[test]
    fn clean_round_trip() {
        for d in data_samples() {
            let cw = encode(d);
            assert_eq!(decode(cw), Ok((d, Outcome::Clean)));
        }
    }

    #[test]
    fn corrects_every_single_bit_error() {
        for d in data_samples() {
            for pos in 0..CODE_BITS {
                let mut cw = encode(d);
                cw.flip(pos).expect("in range");
                let (got, outcome) = decode(cw).expect("single errors are correctable");
                assert_eq!(got, d, "data {d:#x}, flipped {pos}");
                assert_eq!(outcome, Outcome::Corrected { position: pos });
            }
        }
    }

    #[test]
    fn detects_every_double_bit_error() {
        for d in data_samples() {
            for a in 0..CODE_BITS {
                for b in (a + 1)..CODE_BITS {
                    let mut cw = encode(d);
                    cw.flip(a).expect("in range");
                    cw.flip(b).expect("in range");
                    assert_eq!(decode(cw), Err(Error::DoubleBitError), "flips {a},{b}");
                }
            }
        }
    }

    #[test]
    fn codewords_have_even_parity_and_zero_syndrome() {
        for d in data_samples() {
            let cw = encode(d);
            assert_eq!(cw.raw().count_ones() & 1, 0);
            assert_eq!(syndrome(cw.raw()), 0);
        }
    }

    #[test]
    fn rejects_out_of_range_raw_and_flip() {
        assert!(Codeword72::from_raw(1u128 << 72).is_err());
        let mut cw = encode(0);
        assert_eq!(cw.flip(72), Err(Error::BitOutOfRange { position: 72 }));
    }
}
