// SPDX-License-Identifier: MIT OR Apache-2.0
//! Error and report types for the archive-codes crate.

use core::fmt;

/// Errors returned by this crate. Library code never panics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodeError {
    /// A code parameter was outside its valid domain.
    InvalidParameter {
        /// Name of the parameter.
        name: &'static str,
    },
    /// A buffer had the wrong length for the operation.
    LengthMismatch {
        /// Length the operation required.
        expected: usize,
        /// Length supplied.
        actual: usize,
    },
    /// A symbol or bit was outside its alphabet, such as a bit value other than 0 or 1.
    InvalidSymbol,
    /// The received word has more errors than the code can correct, or the decoder could not find a consistent correction.
    Uncorrectable,
}

impl fmt::Display for CodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodeError::InvalidParameter { name } => write!(f, "parameter `{name}` is out of range"),
            CodeError::LengthMismatch { expected, actual } => {
                write!(f, "expected length {expected}, got {actual}")
            }
            CodeError::InvalidSymbol => write!(f, "symbol outside its alphabet"),
            CodeError::Uncorrectable => write!(f, "word is uncorrectable"),
        }
    }
}

impl core::error::Error for CodeError {}

/// Result of a successful block decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeReport {
    /// Number of symbols corrected in place.
    pub corrected: usize,
}
