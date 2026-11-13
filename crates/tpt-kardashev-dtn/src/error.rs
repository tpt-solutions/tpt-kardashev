// SPDX-License-Identifier: MIT OR Apache-2.0
//! Error type for the dtn crate.

use core::fmt;

/// Errors returned by this crate. Library code never panics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input ended before an item was complete.
    Truncated,
    /// The input is not valid CBOR (malformed head, reserved additional information, bad UTF-8).
    MalformedCbor {
        /// Short description of what was wrong.
        what: &'static str,
    },
    /// The encoding is valid CBOR but not the shortest or deterministic form that BPv7 requires.
    NonCanonical {
        /// What was encoded non-minimally.
        what: &'static str,
    },
    /// A data item had a different major type from the one the structure requires.
    UnexpectedItem {
        /// What the structure required at this position.
        expected: &'static str,
    },
    /// An indefinite-length item appeared where BPv7 requires a definite-length one.
    IndefiniteNotAllowed {
        /// The structure that must be definite-length.
        what: &'static str,
    },
    /// Bytes remain after the bundle's closing array.
    TrailingBytes,
    /// The primary block declares a Bundle Protocol version other than 7.
    UnsupportedVersion {
        /// Version found in the block.
        version: u64,
    },
    /// A CRC type code other than 0, 1 or 2.
    UnknownCrcType {
        /// Code found in the block.
        code: u64,
    },
    /// A CRC value does not match the block contents.
    CrcMismatch {
        /// Block number, or 0 for the primary block.
        block: u64,
    },
    /// An endpoint ID uses a URI scheme this crate does not implement.
    UnknownScheme {
        /// The scheme code number.
        code: u64,
    },
    /// An endpoint ID is syntactically invalid.
    InvalidEid {
        /// Which rule failed.
        reason: &'static str,
    },
    /// The bundle breaks a rule of RFC 9171 that is checked here.
    InvalidBundle {
        /// The rule that failed.
        rule: &'static str,
    },
    /// A bundle with the same identity (source and creation timestamp) is already stored.
    DuplicateBundle,
    /// The store has reached its capacity.
    StoreFull,
    /// No stored bundle has the given identity.
    NotFound,
    /// A contact or contact plan is invalid.
    InvalidContact {
        /// The rule that failed.
        rule: &'static str,
    },
    /// A file-backed store could not read or write its directory.
    Io,
    /// A simulator operation named a node that was never added.
    UnknownNode,
    /// An LTP segment or SDNV is malformed or breaks a rule of RFC 5326.
    Ltp {
        /// What was wrong.
        what: &'static str,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(f, "input ended early"),
            Error::MalformedCbor { what } => write!(f, "malformed CBOR: {what}"),
            Error::NonCanonical { what } => write!(f, "non-canonical CBOR: {what}"),
            Error::UnexpectedItem { expected } => write!(f, "expected {expected}"),
            Error::IndefiniteNotAllowed { what } => {
                write!(f, "{what} must have a definite length")
            }
            Error::TrailingBytes => write!(f, "bytes after the bundle"),
            Error::UnsupportedVersion { version } => write!(f, "BP version {version} is not 7"),
            Error::UnknownCrcType { code } => write!(f, "unknown CRC type {code}"),
            Error::CrcMismatch { block } => write!(f, "CRC mismatch in block {block}"),
            Error::UnknownScheme { code } => write!(f, "unsupported URI scheme code {code}"),
            Error::InvalidEid { reason } => write!(f, "invalid endpoint ID: {reason}"),
            Error::InvalidBundle { rule } => write!(f, "invalid bundle: {rule}"),
            Error::DuplicateBundle => write!(f, "a bundle with this identity is already stored"),
            Error::StoreFull => write!(f, "bundle store is full"),
            Error::NotFound => write!(f, "no bundle with this identity"),
            Error::InvalidContact { rule } => write!(f, "invalid contact: {rule}"),
            Error::Io => write!(f, "file store I/O failed"),
            Error::UnknownNode => write!(f, "no such node in the network"),
            Error::Ltp { what } => write!(f, "LTP: {what}"),
        }
    }
}

impl core::error::Error for Error {}
