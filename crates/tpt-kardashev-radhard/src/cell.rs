// SPDX-License-Identifier: MIT OR Apache-2.0
//! CRC-protected cells and checksummed buffers.
//!
//! These detect corruption; they do not correct it. A corrupt cell returns `CorruptCell`
//! instead of a wrong value. The CRC is CRC-32C, implemented in-crate so that this crate has no
//! dependency on `tpt-kardashev-archive-codes`.

use crate::error::Error;

fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0x82F6_3B78
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn check_of(value: u64) -> u32 {
    crc32c(&value.to_le_bytes())
}

/// A 64-bit value with a CRC-32C of its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckedCell {
    value: u64,
    check: u32,
}

impl CheckedCell {
    /// A cell holding `value`.
    pub fn new(value: u64) -> Self {
        Self {
            value,
            check: check_of(value),
        }
    }

    /// Stores a new value and its check.
    pub fn set(&mut self, value: u64) {
        *self = Self::new(value);
    }

    /// The value, if its check still matches.
    pub fn get(&self) -> Result<u64, Error> {
        if check_of(self.value) == self.check {
            Ok(self.value)
        } else {
            Err(Error::CorruptCell { index: 0 })
        }
    }

    /// Flips one bit of the stored value, leaving the check unchanged. Used for fault injection.
    pub fn flip_value_bit(&mut self, bit: u32) -> Result<(), Error> {
        if bit >= 64 {
            return Err(Error::BitOutOfRange { position: bit });
        }
        self.value ^= 1u64 << bit;
        Ok(())
    }
}

/// `N` checksummed 64-bit slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChecksummedBuffer<const N: usize> {
    slots: [CheckedCell; N],
}

impl<const N: usize> ChecksummedBuffer<N> {
    /// A buffer with every slot zero.
    pub fn new() -> Self {
        Self {
            slots: [CheckedCell::new(0); N],
        }
    }

    /// Reads slot `index`.
    pub fn get(&self, index: usize) -> Result<u64, Error> {
        let cell = self
            .slots
            .get(index)
            .ok_or(Error::IndexOutOfRange { index })?;
        cell.get().map_err(|_| Error::CorruptCell { index })
    }

    /// Writes slot `index`.
    pub fn set(&mut self, index: usize, value: u64) -> Result<(), Error> {
        let cell = self
            .slots
            .get_mut(index)
            .ok_or(Error::IndexOutOfRange { index })?;
        cell.set(value);
        Ok(())
    }

    /// Mutable access to a slot, for fault injection.
    pub fn slot_mut(&mut self, index: usize) -> Result<&mut CheckedCell, Error> {
        self.slots
            .get_mut(index)
            .ok_or(Error::IndexOutOfRange { index })
    }

    /// Checks every slot. Returns the first failing index as an error.
    pub fn verify(&self) -> Result<(), Error> {
        for (index, cell) in self.slots.iter().enumerate() {
            if cell.get().is_err() {
                return Err(Error::CorruptCell { index });
            }
        }
        Ok(())
    }
}

impl<const N: usize> Default for ChecksummedBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_every_single_bit_flip_in_a_cell() {
        for bit in 0..64 {
            let mut c = CheckedCell::new(0x0F0F_1234_ABCD_9999);
            c.flip_value_bit(bit).expect("in range");
            assert_eq!(c.get(), Err(Error::CorruptCell { index: 0 }), "bit {bit}");
        }
        assert_eq!(CheckedCell::new(7).get(), Ok(7));
    }

    #[test]
    fn buffer_reports_the_corrupt_slot() {
        let mut b: ChecksummedBuffer<8> = ChecksummedBuffer::new();
        b.set(5, 99).expect("in range");
        assert_eq!(b.get(5), Ok(99));
        b.slot_mut(3)
            .expect("in range")
            .flip_value_bit(40)
            .expect("ok");
        assert_eq!(b.verify(), Err(Error::CorruptCell { index: 3 }));
        assert_eq!(b.get(3), Err(Error::CorruptCell { index: 3 }));
        assert_eq!(b.get(9), Err(Error::IndexOutOfRange { index: 9 }));
    }
}
