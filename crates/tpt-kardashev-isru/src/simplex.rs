// SPDX-License-Identifier: MIT OR Apache-2.0
//! A dense simplex solver for `maximise c·x subject to A·x ≤ b, x ≥ 0`, with `b ≥ 0`.
//!
//! With `b ≥ 0` the origin is feasible, so the solver starts from the slack basis and needs no
//! phase one. Entering and leaving variables follow Bland's rule, which prevents cycling on
//! degenerate problems. The method is for small problems: a dense tableau of size
//! `(rows + 1) × (variables + rows + 1)`.

use alloc::vec;
use alloc::vec::Vec;

use crate::error::IsruError;

/// Iterations allowed before the solver gives up.
pub const MAX_ITERATIONS: usize = 100_000;

const EPS: f64 = 1e-9;

/// The optimum of a linear program.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    /// The values of the variables.
    pub x: Vec<f64>,
    /// The objective value `c·x`.
    pub objective: f64,
}

/// Maximises `c·x` subject to `a·x ≤ b` and `x ≥ 0`.
///
/// `a` has one row per constraint, each of length `c.len()`. Fails with
/// [`IsruError::NotPositive`] when a bound is negative or non-finite, with [`IsruError::Unbounded`]
/// when the objective can grow without limit, and with [`IsruError::IterationLimit`] if the solver
/// does not finish within [`MAX_ITERATIONS`].
pub fn maximise(c: &[f64], a: &[Vec<f64>], b: &[f64]) -> Result<Solution, IsruError> {
    let n = c.len();
    let m = b.len();
    if a.len() != m || a.iter().any(|row| row.len() != n) {
        return Err(IsruError::NotPositive {
            what: "constraint matrix shape",
        });
    }
    if b.iter().any(|v| !(*v >= 0.0 && v.is_finite())) {
        return Err(IsruError::NotPositive {
            what: "constraint bound",
        });
    }
    let width = n + m + 1;
    let rhs = width - 1;
    let mut tableau: Vec<Vec<f64>> = (0..m)
        .map(|i| {
            let mut row = vec![0.0; width];
            row[..n].copy_from_slice(&a[i]);
            row[n + i] = 1.0;
            row[rhs] = b[i];
            row
        })
        .collect();
    // The objective row holds -c, so a negative entry marks a profitable column.
    let mut objective: Vec<f64> = vec![0.0; width];
    for (j, value) in c.iter().enumerate() {
        objective[j] = -value;
    }
    let mut basis: Vec<usize> = (0..m).map(|i| n + i).collect();

    for _ in 0..MAX_ITERATIONS {
        // Bland's rule: the lowest-index column that improves the objective.
        let entering = match (0..n + m).find(|&j| objective[j] < -EPS) {
            Some(j) => j,
            None => {
                return Ok(finish(n, &basis, &tableau, rhs, objective[rhs]));
            }
        };
        // Ratio test, ties broken by the lowest basic variable index (also Bland).
        let mut leaving: Option<usize> = None;
        let mut best_ratio = f64::INFINITY;
        for i in 0..m {
            let entry = tableau[i][entering];
            if entry > EPS {
                let ratio = tableau[i][rhs] / entry;
                let better = match leaving {
                    None => true,
                    Some(_) => {
                        ratio < best_ratio - EPS
                            || ((ratio - best_ratio).abs() <= EPS
                                && basis[i] < basis[leaving.unwrap_or(0)])
                    }
                };
                if better {
                    leaving = Some(i);
                    best_ratio = ratio;
                }
            }
        }
        let row = match leaving {
            Some(i) => i,
            None => return Err(IsruError::Unbounded),
        };
        pivot(&mut tableau, &mut objective, row, entering);
        basis[row] = entering;
    }
    Err(IsruError::IterationLimit)
}

fn pivot(tableau: &mut [Vec<f64>], objective: &mut [f64], row: usize, col: usize) {
    let scale = tableau[row][col];
    for value in tableau[row].iter_mut() {
        *value /= scale;
    }
    let pivot_row = tableau[row].clone();
    for (i, other) in tableau.iter_mut().enumerate() {
        if i != row {
            let factor = other[col];
            if factor != 0.0 {
                for (value, p) in other.iter_mut().zip(pivot_row.iter()) {
                    *value -= factor * p;
                }
            }
        }
    }
    let factor = objective[col];
    if factor != 0.0 {
        for (value, p) in objective.iter_mut().zip(pivot_row.iter()) {
            *value -= factor * p;
        }
    }
}

fn finish(n: usize, basis: &[usize], tableau: &[Vec<f64>], rhs: usize, value: f64) -> Solution {
    let mut x = vec![0.0; n];
    for (i, &var) in basis.iter().enumerate() {
        if var < n {
            x[var] = tableau[i][rhs].max(0.0);
        }
    }
    Solution {
        x,
        objective: value,
    }
}
