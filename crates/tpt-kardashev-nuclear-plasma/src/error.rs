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
    /// The grid is not usable: the radii must be positive and increasing, the heights increasing, and each
    /// direction needs at least three nodes.
    BadGrid,
    /// A solver tolerance is not positive and finite.
    BadTolerance,
    /// A cross-section file line does not parse or is inconsistent. `line` is counted from 1; 0 means
    /// the file as a whole (a missing or miscounted `groups` line).
    Malformed {
        /// The line.
        line: usize,
    },
    /// A multigroup set has no groups.
    NoGroups,
    /// A flux surface could not be found, or the contour leaves the grid.
    NoSurface,
    /// The iteration did not converge within its limit.
    NotConverged,
}

impl fmt::Display for NeutronicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeutronicsError::BadCrossSection => f.write_str(
                "cross sections must be finite, the total positive, and the absorption within [0, total]",
            ),
            NeutronicsError::BadThickness => f.write_str("the slab thickness must be positive and finite"),
            NeutronicsError::NoHistories => f.write_str("a run needs at least one history"),
            NeutronicsError::BadGrid => f.write_str("the grid is not usable"),
            NeutronicsError::BadTolerance => f.write_str("the tolerance must be positive and finite"),
            NeutronicsError::NotConverged => f.write_str("the solver did not converge within its limit"),
            NeutronicsError::NoSurface => f.write_str("no flux surface found inside the grid"),
            NeutronicsError::Malformed { line } => write!(f, "cross-section file is malformed at line {line}"),
            NeutronicsError::NoGroups => f.write_str("a multigroup set needs at least one group"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NeutronicsError {}
