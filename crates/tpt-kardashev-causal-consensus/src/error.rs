// SPDX-License-Identifier: MIT OR Apache-2.0
//! Errors of the causal layer.

use core::fmt;

/// An error from a clock or a causal operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClockError {
    /// A remote timestamp is further ahead of the local physical clock than the configured bound.
    /// The clock state is unchanged.
    DriftExceeded {
        /// How far ahead the remote timestamp is, in milliseconds.
        ahead_ms: u64,
    },
    /// The logical counter would overflow at this physical time.
    CounterOverflow,
    /// A node index is outside the vector clock's dimension.
    NodeOutOfRange {
        /// The index given.
        index: usize,
    },
}

impl fmt::Display for ClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClockError::DriftExceeded { ahead_ms } => {
                write!(
                    f,
                    "remote timestamp is {ahead_ms} ms ahead of the local clock"
                )
            }
            ClockError::CounterOverflow => f.write_str("the logical counter overflowed"),
            ClockError::NodeOutOfRange { index } => {
                write!(f, "node index {index} is outside the vector clock")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ClockError {}
