// SPDX-License-Identifier: MIT OR Apache-2.0
//! Linear constraint systems over policy parameters, the invariants and updates the checker compares,
//! and the certificate that shows one contained in the other.
//!
//! Only linear bounds are supported: each row says `coefficients · x <= bound`, over exact rationals.

use alloc::vec::Vec;

use crate::error::ReflectError;
use crate::rational::Rational;

/// One linear bound: `coefficients · x <= bound`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The coefficients, one per parameter.
    pub coefficients: Vec<Rational>,
    /// The bound.
    pub bound: Rational,
}

/// A system of linear bounds over `dimension` parameters. The feasible set is the set of parameter
/// vectors that satisfy every row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constraints {
    dimension: usize,
    rows: Vec<Row>,
}

/// An invariant the policy must keep: the system a policy must stay inside.
pub type Invariant = Constraints;

/// A proposed update to the policy: the system the new policy would be inside.
pub type Update = Constraints;

impl Constraints {
    /// A system over `dimension` parameters. Fails with [`ReflectError::ShapeMismatch`] when a row
    /// does not have `dimension` coefficients.
    pub fn new(dimension: usize, rows: Vec<Row>) -> Result<Self, ReflectError> {
        if rows.iter().any(|r| r.coefficients.len() != dimension) {
            return Err(ReflectError::ShapeMismatch);
        }
        Ok(Self { dimension, rows })
    }

    /// The number of parameters.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// The rows.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Whether `point` satisfies every row. Exact: no tolerance. Fails on overflow or a wrong length.
    pub fn contains(&self, point: &[Rational]) -> Result<bool, ReflectError> {
        if point.len() != self.dimension {
            return Err(ReflectError::ShapeMismatch);
        }
        for row in &self.rows {
            let mut value = Rational::zero();
            for (c, x) in row.coefficients.iter().zip(point.iter()) {
                value = value.checked_add(c.checked_mul(*x)?)?;
            }
            if value.checked_cmp(row.bound)? == core::cmp::Ordering::Greater {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// A Farkas-style certificate that an update is contained in an invariant. For each row of the
/// invariant there is one vector of non-negative multipliers, one per row of the update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    /// `multipliers[i][j]` is the multiplier of update row `j` for invariant row `i`.
    pub multipliers: Vec<Vec<Rational>>,
}
