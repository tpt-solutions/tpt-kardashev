// SPDX-License-Identifier: MIT OR Apache-2.0
//! The containment checker. An update is contained in an invariant when every invariant row has a
//! certificate: non-negative multipliers of the update's rows that reproduce the row's coefficients
//! exactly, with a combined bound no larger than the row's bound.
//!
//! Soundness: for any parameter vector `x` that satisfies the update, the certificate gives
//! `c·x = Σ λ_j (a_j·x) <= Σ λ_j b_j <= d`, because `λ_j >= 0` and `a_j·x <= b_j`. So every point of the
//! update satisfies the invariant. The check is exact, so a certificate that passes is a proof.

use crate::error::ReflectError;
use crate::invariant::{Certificate, Constraints};
use crate::rational::Rational;

/// Checks that `update` is contained in `invariant`, using `certificate`. Returns `Ok(())` only when
/// the certificate proves the containment; every other case is an error that names the failing part.
pub fn verify(
    invariant: &Constraints,
    update: &Constraints,
    certificate: &Certificate,
) -> Result<(), ReflectError> {
    if invariant.dimension() != update.dimension() {
        return Err(ReflectError::DimensionMismatch);
    }
    if certificate.multipliers.len() != invariant.rows().len() {
        return Err(ReflectError::CertificateShape);
    }
    let dimension = invariant.dimension();
    for (i, (row, multipliers)) in invariant
        .rows()
        .iter()
        .zip(certificate.multipliers.iter())
        .enumerate()
    {
        if multipliers.len() != update.rows().len() {
            return Err(ReflectError::CertificateShape);
        }
        if multipliers.iter().any(|m| m.is_negative()) {
            return Err(ReflectError::NegativeMultiplier { row: i });
        }
        for column in 0..dimension {
            let mut combined = Rational::zero();
            for (multiplier, update_row) in multipliers.iter().zip(update.rows().iter()) {
                combined = combined
                    .checked_add(multiplier.checked_mul(update_row.coefficients[column])?)?;
            }
            if combined.checked_cmp(row.coefficients[column])? != core::cmp::Ordering::Equal {
                return Err(ReflectError::CoefficientMismatch { row: i, column });
            }
        }
        let mut bound = Rational::zero();
        for (multiplier, update_row) in multipliers.iter().zip(update.rows().iter()) {
            bound = bound.checked_add(multiplier.checked_mul(update_row.bound)?)?;
        }
        if bound.checked_cmp(row.bound)? == core::cmp::Ordering::Greater {
            return Err(ReflectError::BoundExceeded { row: i });
        }
    }
    Ok(())
}

/// Convenience: an exact rational from an integer, for building certificates in tests and examples.
pub fn int(n: i128) -> Rational {
    Rational::integer(n)
}
