// SPDX-License-Identifier: MIT OR Apache-2.0
//! Seeded fault injection for tests: single bit flips and contiguous bursts.
//!
//! The generator is xorshift64*, with a fixed seed chosen by the caller. The same seed gives the
//! same faults, so failing tests reproduce exactly.

use crate::error::Error;
use crate::secded::Codeword72;

/// Deterministic fault source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaultInjector {
    state: u64,
}

impl FaultInjector {
    /// Creates an injector from `seed`. A zero seed is replaced by a fixed non-zero constant.
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A uniformly chosen index in `0..bound`. `bound` must be non-zero.
    pub fn index(&mut self, bound: u32) -> Result<u32, Error> {
        if bound == 0 {
            return Err(Error::BitOutOfRange { position: 0 });
        }
        Ok((self.next_u64() % u64::from(bound)) as u32)
    }

    /// Flips one random bit of `word`. Returns the bit flipped.
    pub fn flip_u64(&mut self, word: &mut u64) -> u32 {
        let bit = (self.next_u64() % 64) as u32;
        *word ^= 1u64 << bit;
        bit
    }

    /// Flips a contiguous burst of `len` bits (1 to 64) at a random position. Returns the first bit.
    pub fn burst_u64(&mut self, word: &mut u64, len: u32) -> Result<u32, Error> {
        if len == 0 || len > 64 {
            return Err(Error::BitOutOfRange { position: len });
        }
        let start = (self.next_u64() % u64::from(65 - len)) as u32;
        let mask = if len == 64 {
            u64::MAX
        } else {
            ((1u64 << len) - 1) << start
        };
        *word ^= mask;
        Ok(start)
    }

    /// Flips one random bit of a codeword. Returns the position flipped.
    pub fn flip_codeword(&mut self, cw: &mut Codeword72) -> u32 {
        let pos = (self.next_u64() % 72) as u32;
        // `pos` is below 72 by construction, so `flip` cannot fail here.
        let _ = cw.flip(pos);
        pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_faults() {
        let mut a = FaultInjector::new(7);
        let mut b = FaultInjector::new(7);
        for _ in 0..100 {
            let mut x = 0u64;
            let mut y = 0u64;
            assert_eq!(a.flip_u64(&mut x), b.flip_u64(&mut y));
            assert_eq!(x, y);
        }
    }

    #[test]
    fn burst_flips_exactly_len_contiguous_bits() {
        let mut inj = FaultInjector::new(3);
        for len in 1..=64 {
            let mut w = 0u64;
            let start = inj.burst_u64(&mut w, len).expect("valid length");
            assert_eq!(w.count_ones(), len);
            assert_eq!(w.trailing_zeros(), start);
        }
        assert!(inj.burst_u64(&mut 0, 0).is_err());
        assert!(inj.burst_u64(&mut 0, 65).is_err());
    }
}
