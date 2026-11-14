// SPDX-License-Identifier: MIT OR Apache-2.0
//! Errors of the resource planner.

use core::fmt;

/// An error from building a material or process, or from planning.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum IsruError {
    /// A mass fraction is outside `[0, 1]`, or an element is given twice.
    BadFraction {
        /// The element concerned.
        element: &'static str,
    },
    /// The mass fractions of a composition do not total 1.
    FractionsDoNotSum {
        /// The total found.
        total: f64,
    },
    /// A process does not balance: the mass, or the mass of this element, differs between its inputs
    /// and outputs beyond the tolerance. Such a process is rejected when it is built.
    Unbalanced {
        /// `"mass"` for the total, or the element's symbol.
        element: &'static str,
    },
    /// The balance tolerance is not a positive finite number.
    BadTolerance,
    /// A process refers to a species index that does not exist.
    UnknownSpecies {
        /// The index given.
        index: usize,
    },
    /// A quantity must be positive and finite (a process's amount per run, a run limit, a stock).
    NotPositive {
        /// The quantity concerned.
        what: &'static str,
    },
    /// The linear program did not converge within its iteration limit.
    IterationLimit,
    /// The target species is not produced by any process: no plan can make it.
    NoProducer,
    /// The objective can grow without limit under the constraints.
    Unbounded,
}

impl fmt::Display for IsruError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IsruError::BadFraction { element } => {
                write!(
                    f,
                    "mass fraction of {element} is outside [0, 1] or repeated"
                )
            }
            IsruError::FractionsDoNotSum { total } => {
                write!(f, "mass fractions total {total}, not 1")
            }
            IsruError::Unbalanced { element } => {
                write!(f, "the process does not balance in {element}")
            }
            IsruError::BadTolerance => f.write_str("the balance tolerance is not positive"),
            IsruError::UnknownSpecies { index } => {
                write!(f, "species index {index} does not exist")
            }
            IsruError::NotPositive { what } => write!(f, "{what} must be positive and finite"),
            IsruError::IterationLimit => f.write_str("the linear program hit its iteration limit"),
            IsruError::NoProducer => f.write_str("no process produces the target"),
            IsruError::Unbounded => f.write_str("the objective is unbounded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IsruError {}
