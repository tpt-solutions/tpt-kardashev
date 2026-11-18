// SPDX-License-Identifier: MIT OR Apache-2.0
//! The subset of CBOR (RFC 8949) that BPv7 uses, in strict form.
//!
//! Reading is strict: integers and lengths must use the shortest form, reserved additional
//! information values are errors, and byte and text strings must be definite-length. RFC 9171
//! allows indefinite-length arrays, which the reader accepts; it does not accept indefinite-length
//! strings, which BPv7 never uses.
//!
//! Writing always uses the shortest form and definite-length strings.

use crate::error::Error;

const MAJOR_UINT: u8 = 0;
const MAJOR_BYTES: u8 = 2;
const MAJOR_TEXT: u8 = 3;
const MAJOR_ARRAY: u8 = 4;

/// Initial byte of the CBOR "break" stop code, which ends an indefinite-length item.
pub const BREAK: u8 = 0xFF;

/// Length of an array as declared by its header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayLen {
    /// Definite length: this many items follow.
    Definite(u64),
    /// Indefinite length: items follow, terminated by a break code.
    Indefinite,
}

/// Reads CBOR items from a byte slice, tracking the position.
#[derive(Clone, Debug)]
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// A reader over `buf`, positioned at its start.
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Current offset from the start of the buffer.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Bytes not yet read.
    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// The byte at the current position, without consuming it.
    pub fn peek(&self) -> Result<u8, Error> {
        self.buf.get(self.pos).copied().ok_or(Error::Truncated)
    }

    /// The bytes `start..end` of the underlying buffer.
    pub fn slice(&self, start: usize, end: usize) -> Result<&'a [u8], Error> {
        self.buf.get(start..end).ok_or(Error::Truncated)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated)?;
        let out = self.buf.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(out)
    }

    fn take_byte(&mut self) -> Result<u8, Error> {
        let b = self.peek()?;
        self.pos += 1;
        Ok(b)
    }

    /// Reads an initial byte and its argument. Returns `(major, argument, indefinite)`, where
    /// `argument` is zero when `indefinite` is set.
    fn head(&mut self) -> Result<(u8, u64, bool), Error> {
        let initial = self.take_byte()?;
        let major = initial >> 5;
        let info = initial & 0x1F;
        match info {
            0..=23 => Ok((major, u64::from(info), false)),
            24 => {
                let v = u64::from(self.take_byte()?);
                if v < 24 {
                    return Err(Error::NonCanonical { what: "integer" });
                }
                Ok((major, v, false))
            }
            25 => {
                let v = u64::from(u16::from_be_bytes(self.array::<2>()?));
                if v < 0x100 {
                    return Err(Error::NonCanonical { what: "integer" });
                }
                Ok((major, v, false))
            }
            26 => {
                let v = u64::from(u32::from_be_bytes(self.array::<4>()?));
                if v < 0x1_0000 {
                    return Err(Error::NonCanonical { what: "integer" });
                }
                Ok((major, v, false))
            }
            27 => {
                let v = u64::from_be_bytes(self.array::<8>()?);
                if v < 0x1_0000_0000 {
                    return Err(Error::NonCanonical { what: "integer" });
                }
                Ok((major, v, false))
            }
            31 => Ok((major, 0, true)),
            _ => Err(Error::MalformedCbor {
                what: "reserved additional information",
            }),
        }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// Reads an unsigned integer.
    pub fn read_uint(&mut self) -> Result<u64, Error> {
        match self.head()? {
            (MAJOR_UINT, v, false) => Ok(v),
            (MAJOR_UINT, _, true) => Err(Error::IndefiniteNotAllowed { what: "integer" }),
            _ => Err(Error::UnexpectedItem {
                expected: "unsigned integer",
            }),
        }
    }

    /// Reads a definite-length byte string.
    pub fn read_bytes(&mut self) -> Result<&'a [u8], Error> {
        match self.head()? {
            (MAJOR_BYTES, len, false) => {
                let n = usize::try_from(len).map_err(|_| Error::Truncated)?;
                self.take(n)
            }
            (MAJOR_BYTES, _, true) => Err(Error::IndefiniteNotAllowed {
                what: "byte string",
            }),
            _ => Err(Error::UnexpectedItem {
                expected: "byte string",
            }),
        }
    }

    /// Reads a definite-length text string, which must be valid UTF-8.
    pub fn read_text(&mut self) -> Result<&'a str, Error> {
        match self.head()? {
            (MAJOR_TEXT, len, false) => {
                let n = usize::try_from(len).map_err(|_| Error::Truncated)?;
                core::str::from_utf8(self.take(n)?).map_err(|_| Error::MalformedCbor {
                    what: "text is not UTF-8",
                })
            }
            (MAJOR_TEXT, _, true) => Err(Error::IndefiniteNotAllowed {
                what: "text string",
            }),
            _ => Err(Error::UnexpectedItem {
                expected: "text string",
            }),
        }
    }

    /// Reads an array header.
    pub fn read_array_header(&mut self) -> Result<ArrayLen, Error> {
        match self.head()? {
            (MAJOR_ARRAY, n, false) => Ok(ArrayLen::Definite(n)),
            (MAJOR_ARRAY, _, true) => Ok(ArrayLen::Indefinite),
            _ => Err(Error::UnexpectedItem { expected: "array" }),
        }
    }

    /// True if the next byte is the break code.
    pub fn at_break(&self) -> Result<bool, Error> {
        Ok(self.peek()? == BREAK)
    }

    /// Consumes the break code that ends an indefinite-length array.
    pub fn read_break(&mut self) -> Result<(), Error> {
        if self.take_byte()? == BREAK {
            Ok(())
        } else {
            Err(Error::UnexpectedItem {
                expected: "break code",
            })
        }
    }

    /// Reads a boolean: the simple values `false` (0xF4) and `true` (0xF5).
    pub fn read_bool(&mut self) -> Result<bool, Error> {
        match self.take_byte()? {
            0xF4 => Ok(false),
            0xF5 => Ok(true),
            _ => Err(Error::UnexpectedItem {
                expected: "boolean",
            }),
        }
    }
}

#[cfg(feature = "alloc")]
pub use writer::Writer;

#[cfg(feature = "alloc")]
mod writer {
    use super::{BREAK, MAJOR_ARRAY, MAJOR_BYTES, MAJOR_TEXT, MAJOR_UINT};
    use alloc::vec::Vec;

    /// Writes CBOR items into a growable buffer, in shortest form.
    #[derive(Clone, Debug, Default)]
    pub struct Writer {
        buf: Vec<u8>,
    }

    impl Writer {
        /// An empty writer.
        pub fn new() -> Self {
            Self { buf: Vec::new() }
        }

        /// Bytes written so far.
        pub fn len(&self) -> usize {
            self.buf.len()
        }

        /// True if nothing has been written.
        pub fn is_empty(&self) -> bool {
            self.buf.is_empty()
        }

        /// The encoded bytes.
        pub fn into_bytes(self) -> Vec<u8> {
            self.buf
        }

        /// Borrows the encoded bytes so far.
        pub fn as_bytes(&self) -> &[u8] {
            &self.buf
        }

        /// Overwrites `bytes` bytes starting at `at` (used to fill in a CRC after the fact).
        pub fn patch(&mut self, at: usize, bytes: &[u8]) {
            if let Some(dst) = self.buf.get_mut(at..at + bytes.len()) {
                dst.copy_from_slice(bytes);
            }
        }

        fn head(&mut self, major: u8, arg: u64) {
            let m = major << 5;
            if arg < 24 {
                self.buf.push(m | arg as u8);
            } else if arg <= u64::from(u8::MAX) {
                self.buf.push(m | 24);
                self.buf.push(arg as u8);
            } else if arg <= u64::from(u16::MAX) {
                self.buf.push(m | 25);
                self.buf.extend_from_slice(&(arg as u16).to_be_bytes());
            } else if arg <= u64::from(u32::MAX) {
                self.buf.push(m | 26);
                self.buf.extend_from_slice(&(arg as u32).to_be_bytes());
            } else {
                self.buf.push(m | 27);
                self.buf.extend_from_slice(&arg.to_be_bytes());
            }
        }

        /// Writes an unsigned integer.
        pub fn uint(&mut self, v: u64) {
            self.head(MAJOR_UINT, v);
        }

        /// Writes a definite-length byte string.
        pub fn bytes(&mut self, b: &[u8]) {
            self.head(MAJOR_BYTES, b.len() as u64);
            self.buf.extend_from_slice(b);
        }

        /// Writes a definite-length text string.
        pub fn text(&mut self, s: &str) {
            self.head(MAJOR_TEXT, s.len() as u64);
            self.buf.extend_from_slice(s.as_bytes());
        }

        /// Writes a definite-length array header for `n` items.
        pub fn array(&mut self, n: u64) {
            self.head(MAJOR_ARRAY, n);
        }

        /// Starts an indefinite-length array. Close it with [`Writer::brk`].
        pub fn array_indefinite(&mut self) {
            self.buf.push((MAJOR_ARRAY << 5) | 31);
        }

        /// Writes the break code.
        pub fn brk(&mut self) {
            self.buf.push(BREAK);
        }

        /// Appends already-encoded CBOR bytes unchanged.
        pub fn raw(&mut self, encoded: &[u8]) {
            self.buf.extend_from_slice(encoded);
        }
    }
}
