// SPDX-License-Identifier: MIT OR Apache-2.0
//! Error type for the radhard crate.

use core::fmt;

/// Errors returned by this crate. Library code never panics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// No value is held by a strict majority of replicas.
    NoMajority,
    /// A SECDED word has two bit errors: detected, not correctable.
    DoubleBitError,
    /// A SECDED word has more errors than the code can detect reliably.
    Uncorrectable,
    /// A checksummed cell or buffer slot failed its check.
    CorruptCell {
        /// Index of the failing slot (0 for a single cell).
        index: usize,
    },
    /// An index was outside the container.
    IndexOutOfRange {
        /// The index supplied.
        index: usize,
    },
    /// A bit position was outside the codeword.
    BitOutOfRange {
        /// The position supplied.
        position: u32,
    },
    /// The watchdog saw no progress since its last check.
    Stalled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoMajority => write!(f, "no strict majority among replicas"),
            Error::DoubleBitError => write!(f, "double-bit error detected"),
            Error::Uncorrectable => write!(f, "word is uncorrectable"),
            Error::CorruptCell { index } => write!(f, "checksum failed at slot {index}"),
            Error::IndexOutOfRange { index } => write!(f, "index {index} is out of range"),
            Error::BitOutOfRange { position } => {
                write!(f, "bit position {position} is out of range")
            }
            Error::Stalled => write!(f, "no progress since the last watchdog check"),
        }
    }
}

impl core::error::Error for Error {}
