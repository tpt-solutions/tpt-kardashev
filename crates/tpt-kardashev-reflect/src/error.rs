// SPDX-License-Identifier: MIT OR Apache-2.0
//! Errors of the invariant and certificate checker.

use core::fmt;

/// An error from building an invariant or checking a certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReflectError {
    /// A rational was given a zero denominator.
    ZeroDenominator,
    /// An exact operation would overflow `i128`. The checker rejects rather than approximate.
    Overflow,
    /// The rows of a constraint system do not all have the stated dimension.
    ShapeMismatch,
    /// The invariant and the update have different dimensions.
    DimensionMismatch,
    /// A certificate multiplier is negative, so it cannot justify the containment.
    NegativeMultiplier {
        /// The invariant row the multiplier belongs to.
        row: usize,
    },
    /// The certificate's combination of the update's rows does not reproduce an invariant row's
    /// coefficient.
    CoefficientMismatch {
        /// The invariant row.
        row: usize,
        /// The coefficient (column) that differs.
        column: usize,
    },
    /// The certificate's combination of the update's bounds exceeds the invariant row's bound, so
    /// it does not show the containment.
    BoundExceeded {
        /// The invariant row.
        row: usize,
    },
    /// The certificate does not have one multiplier vector per invariant row, of the update's length.
    CertificateShape,
}

impl fmt::Display for ReflectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReflectError::ZeroDenominator => f.write_str("a rational has a zero denominator"),
            ReflectError::Overflow => f.write_str("exact arithmetic would overflow; rejected"),
            ReflectError::ShapeMismatch => f.write_str("a constraint row has the wrong dimension"),
            ReflectError::DimensionMismatch => {
                f.write_str("the invariant and the update have different dimensions")
            }
            ReflectError::NegativeMultiplier { row } => {
                write!(f, "the multiplier for invariant row {row} is negative")
            }
            ReflectError::CoefficientMismatch { row, column } => write!(
                f,
                "invariant row {row} does not match the update at coefficient {column}"
            ),
            ReflectError::BoundExceeded { row } => {
                write!(
                    f,
                    "invariant row {row} is not implied by the update's bounds"
                )
            }
            ReflectError::CertificateShape => {
                f.write_str("the certificate does not have one multiplier per invariant row")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ReflectError {}
