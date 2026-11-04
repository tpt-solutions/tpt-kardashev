// SPDX-License-Identifier: MIT OR Apache-2.0
//! Error type for the energy-ledger crate.

use core::fmt;

/// Errors returned by this crate. Library code never panics.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// An input was NaN or infinite.
    NonFinite,
    /// A quantity that must be non-negative was negative.
    Negative {
        /// Name of the quantity.
        name: &'static str,
    },
    /// A value was outside its valid domain, such as an emissivity above 1.
    OutOfRange {
        /// Name of the quantity.
        name: &'static str,
    },
    /// The account table is full.
    LedgerFull,
    /// An account id does not belong to this ledger.
    UnknownAccount,
    /// A transfer would overdraw its source account.
    InsufficientFunds,
    /// A transfer named the same account as source and destination.
    SameAccount,
    /// The account kind does not allow this direction of transfer, or initial funding.
    ForbiddenFlow,
    /// A heat load would exceed the remaining radiator capacity.
    OverBudget,
    /// The ledger's total energy differs from the declared total.
    NotConserved,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NonFinite => write!(f, "input is not finite"),
            Error::Negative { name } => write!(f, "`{name}` must not be negative"),
            Error::OutOfRange { name } => write!(f, "`{name}` is out of range"),
            Error::LedgerFull => write!(f, "ledger has no free account slots"),
            Error::UnknownAccount => write!(f, "account id does not belong to this ledger"),
            Error::InsufficientFunds => write!(f, "transfer exceeds the source balance"),
            Error::SameAccount => write!(f, "source and destination are the same account"),
            Error::ForbiddenFlow => write!(f, "account kind does not allow this flow"),
            Error::OverBudget => write!(f, "heat load exceeds remaining radiator capacity"),
            Error::NotConserved => write!(f, "ledger total does not match the declared total"),
        }
    }
}

impl core::error::Error for Error {}

/// Checks that a value is finite and not negative.
pub(crate) fn non_negative(value: f64, name: &'static str) -> Result<f64, Error> {
    if !value.is_finite() {
        return Err(Error::NonFinite);
    }
    if value < 0.0 {
        return Err(Error::Negative { name });
    }
    Ok(value)
}
