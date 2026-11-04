// SPDX-License-Identifier: MIT OR Apache-2.0
//! Volatile register access. This is the crate's only `unsafe` code, behind the `volatile` feature.
//!
//! A `VolatileRegister` wraps a raw pointer. Its constructor is `unsafe` because the caller must
//! prove the address is valid. Reads and writes then go through `read_volatile` and
//! `write_volatile`, so the compiler cannot elide or reorder them relative to other volatile
//! accesses. This is needed for memory-mapped registers and for memory that an upset-scrubbing
//! loop must re-read on every pass.

#![allow(unsafe_code)]

/// A volatile view of a `T` at a fixed address.
#[derive(Debug, PartialEq, Eq)]
pub struct VolatileRegister<T: Copy> {
    ptr: *mut T,
}

impl<T: Copy> VolatileRegister<T> {
    /// Wraps `ptr` as a volatile register.
    ///
    /// # Safety
    ///
    /// For the whole lifetime of the returned value, `ptr` must be non-null, aligned for `T`, and
    /// valid for reads and writes of `T`. No other live reference may alias the same memory in a
    /// way that conflicts with these accesses.
    pub unsafe fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }

    /// Reads the value with a volatile load.
    pub fn read(&self) -> T {
        // SAFETY: `new` requires `ptr` to be valid for reads and writes of `T` for `self`'s lifetime.
        unsafe { core::ptr::read_volatile(self.ptr) }
    }

    /// Writes `value` with a volatile store.
    pub fn write(&mut self, value: T) {
        // SAFETY: as for `read`; `&mut self` gives exclusive use of the register through this handle.
        unsafe { core::ptr::write_volatile(self.ptr, value) }
    }

    /// Reads, applies `f`, and writes back. Two volatile accesses, in that order.
    pub fn update(&mut self, f: impl FnOnce(T) -> T) {
        let next = f(self.read());
        self.write(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_through_the_pointer() {
        let mut word: u32 = 0x1234;
        {
            // SAFETY: `word` lives for the whole test and is accessed only through `reg`.
            let mut reg = unsafe { VolatileRegister::new(&mut word as *mut u32) };
            assert_eq!(reg.read(), 0x1234);
            reg.write(0xBEEF);
            assert_eq!(reg.read(), 0xBEEF);
            reg.update(|v| v ^ 0xFFFF);
            assert_eq!(reg.read(), 0x4110);
        }
        assert_eq!(word, 0x4110);
    }

    #[test]
    fn works_with_byte_arrays_through_element_pointers() {
        let mut bytes = [1u8, 2, 3, 4];
        let base = bytes.as_mut_ptr();
        for i in 0..bytes.len() {
            // SAFETY: `i` is in bounds of `bytes`, which outlives the register.
            let mut reg = unsafe { VolatileRegister::new(base.wrapping_add(i)) };
            reg.update(|v| v.wrapping_mul(2));
        }
        assert_eq!(bytes, [2, 4, 6, 8]);
    }
}
