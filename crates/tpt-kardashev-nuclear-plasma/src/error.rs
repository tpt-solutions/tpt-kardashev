// SPDX-License-Identifier: MIT OR Apache-2.0
//! Errors of the neutronics and equilibrium code.

use core::fmt;

/// An error from building a cross section or a geometry, or from a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NeutronicsError {
    /// The cross sections are not finite, the total is not positive, or the absorption is outside
    /// `[0, total]`.
    BadCrossSection,
    /// The slab thickness is not positive and finite.
    BadThickness,
    /// A run was asked for zero histories.
    NoHistories,
}

impl fmt::Display for NeutronicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeutronicsError::BadCrossSection => f.write_str(
                "cross sections must be finite, the total positive, and the absorption within [0, total]",
            ),
            NeutronicsError::BadThickness => f.write_str("the slab thickness must be positive and finite"),
            NeutronicsError::NoHistories => f.write_str("a run needs at least one history"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NeutronicsError {}
