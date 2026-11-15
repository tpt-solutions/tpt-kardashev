// SPDX-License-Identifier: MIT OR Apache-2.0
//! A certificate generator for containment, for tests and examples. It is not trusted: the output is
//! only a candidate, and [`crate::verify()`] decides whether it proves containment.
//!
//! For each invariant row `c·x <= d` the generator looks for `λ >= 0` over the update's rows with
//! `Σ_j λ_j a_j = c` and `Σ_j λ_j b_j <= d`. This is a feasibility problem, solved by a phase-one
//! simplex over exact rationals with Bland's rule, so the multipliers it returns are exact and need no
//! rounding. If no such `λ` exists, the containment cannot be shown in this form and the generator says
//! so.

use alloc::vec;
use alloc::vec::Vec;

use crate::error::ReflectError;
use crate::invariant::{Certificate, Constraints, Row};
use crate::rational::Rational;

/// Iterations allowed in the phase-one simplex before the generator gives up.
const MAX_ITERATIONS: usize = 10_000;

/// Finds non-negative multipliers `λ` over `update` for one invariant row `target`, or `None` when no
/// certificate of this form exists for that row.
fn multipliers_for(
    update: &[Row],
    target: &Row,
    dimension: usize,
) -> Result<Option<Vec<Rational>>, ReflectError> {
    let m = update.len();
    // Rows: one per parameter (equality), one for the bound (inequality, with a slack).
    // Columns: λ (m), slack (1), artificial (one per row), right-hand side.
    let rows = dimension + 1;
    let slack = m;
    let artificial0 = m + 1;
    let width = artificial0 + rows + 1;
    let rhs_col = width - 1;
    let mut table: Vec<Vec<Rational>> = Vec::with_capacity(rows);
    for k in 0..rows {
        let mut row = vec![Rational::zero(); width];
        for (j, update_row) in update.iter().enumerate() {
            row[j] = if k < dimension {
                update_row.coefficients[k]
            } else {
                update_row.bound
            };
        }
        if k == dimension {
            row[slack] = Rational::integer(1);
        }
        let rhs = if k < dimension {
            target.coefficients[k]
        } else {
            target.bound
        };
        // Make the right-hand side non-negative, so the artificial variable starts feasible.
        let flip = rhs.is_negative();
        if flip {
            for value in row.iter_mut().take(artificial0) {
                *value = Rational::zero()
                    .checked_add(*value)?
                    .checked_mul(Rational::integer(-1))?;
            }
        }
        row[artificial0 + k] = Rational::integer(1);
        row[rhs_col] = if flip {
            rhs.checked_mul(Rational::integer(-1))?
        } else {
            rhs
        };
        table.push(row);
    }
    let mut basis: Vec<usize> = (0..rows).map(|k| artificial0 + k).collect();
    // Phase one: minimise the sum of the artificial variables.
    let mut cost = vec![Rational::zero(); width];
    for k in 0..rows {
        cost[artificial0 + k] = Rational::integer(1);
    }
    for _ in 0..MAX_ITERATIONS {
        // Reduced costs: d_j = cost_j - Σ_r cost_{basis[r]} T[r][j].
        let mut entering = None;
        for j in 0..width - 1 {
            let mut reduced = cost[j];
            for (r, &b) in basis.iter().enumerate() {
                reduced = reduced.checked_add(
                    cost[b]
                        .checked_mul(table[r][j])?
                        .checked_mul(Rational::integer(-1))?,
                )?;
            }
            if reduced.is_negative() {
                entering = Some(j);
                break;
            }
        }
        let Some(col) = entering else { break };
        // Ratio test, lowest basis index on ties (Bland).
        let mut leaving: Option<usize> = None;
        let mut best = Rational::zero();
        for r in 0..rows {
            if !table[r][col].is_negative() && !table[r][col].is_zero() {
                let ratio = divide(table[r][rhs_col], table[r][col])?;
                let better = match leaving {
                    None => true,
                    Some(current) => match ratio.checked_cmp(best)? {
                        core::cmp::Ordering::Less => true,
                        core::cmp::Ordering::Equal => basis[r] < basis[current],
                        core::cmp::Ordering::Greater => false,
                    },
                };
                if better {
                    leaving = Some(r);
                    best = ratio;
                }
            }
        }
        let Some(row) = leaving else {
            // Unbounded phase-one objective cannot happen (it is bounded below by zero); stop safely.
            return Ok(None);
        };
        // Pivot on (row, col).
        let pivot = table[row][col];
        for value in table[row].iter_mut() {
            *value = value.checked_mul(Rational::new(pivot.denominator(), pivot.numerator())?)?;
        }
        let pivot_row = table[row].clone();
        for (r, other) in table.iter_mut().enumerate() {
            if r != row && !other[col].is_zero() {
                let factor = other[col];
                for (value, p) in other.iter_mut().zip(pivot_row.iter()) {
                    *value = value
                        .checked_add(factor.checked_mul(*p)?.checked_mul(Rational::integer(-1))?)?;
                }
            }
        }
        basis[row] = col;
    }
    // Phase-one optimum: the sum of the artificial variables still in the basis.
    let mut infeasibility = Rational::zero();
    for (r, &b) in basis.iter().enumerate() {
        if b >= artificial0 {
            infeasibility = infeasibility.checked_add(table[r][rhs_col])?;
        }
    }
    if !infeasibility.is_zero() {
        return Ok(None);
    }
    let mut lambda = vec![Rational::zero(); m];
    for (r, &b) in basis.iter().enumerate() {
        if b < m {
            lambda[b] = table[r][rhs_col];
        }
    }
    Ok(Some(lambda))
}

/// Generates a candidate certificate that `update` is contained in `invariant`. Fails with
/// [`ReflectError::NoCertificate`] when no certificate of the form checked by [`crate::verify()`] exists
/// for some invariant row, and with [`ReflectError::ShapeMismatch`] when the dimensions differ.
pub fn certify(invariant: &Constraints, update: &Constraints) -> Result<Certificate, ReflectError> {
    if invariant.dimension() != update.dimension() {
        return Err(ReflectError::DimensionMismatch);
    }
    let dimension = invariant.dimension();
    let mut multipliers = Vec::with_capacity(invariant.rows().len());
    for target in invariant.rows() {
        match multipliers_for(update.rows(), target, dimension)? {
            Some(lambda) => multipliers.push(lambda),
            None => return Err(ReflectError::NoCertificate),
        }
    }
    Ok(Certificate { multipliers })
}

/// `a / b` for a non-zero `b`, with checked arithmetic.
fn divide(a: Rational, b: Rational) -> Result<Rational, ReflectError> {
    let num = a
        .numerator()
        .checked_mul(b.denominator())
        .ok_or(ReflectError::Overflow)?;
    let den = a
        .denominator()
        .checked_mul(b.numerator())
        .ok_or(ReflectError::Overflow)?;
    Rational::new(num, den)
}
