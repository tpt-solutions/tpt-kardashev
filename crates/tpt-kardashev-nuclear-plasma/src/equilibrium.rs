// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fixed-boundary Grad-Shafranov solve on a rectangular (R, Z) grid, and the Solov'ev benchmark.
//!
//! The equation is `Δ*ψ = κ R²`, with `Δ*ψ = ψ_RR − ψ_R / R + ψ_ZZ`. This is the Grad-Shafranov
//! equation with pressure-driven current only, which is the case for which the Solov'ev function
//! `ψ = A R⁴ + B R² Z²` is exact with `κ = 8A + 2B`. The solver discretises Δ* with central
//! differences (second order in the grid spacing) and solves the system by successive over-relaxation,
//! with the boundary values of ψ fixed by the caller.
//!
//! Scope: one linear elliptic solve. The toroidal-field function `F(ψ)` is not included, so the
//! general equation is not solved. The flux-surface and safety-factor diagnostics are not written.

use alloc::vec;
use alloc::vec::Vec;

use crate::error::NeutronicsError;

/// A rectangular grid in (R, Z), with `nr` by `nz` nodes, both at least 3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    /// The smallest R, in metres. Must be positive, so that `1/R` is finite on the grid.
    pub r_min: f64,
    /// The largest R.
    pub r_max: f64,
    /// The smallest Z.
    pub z_min: f64,
    /// The largest Z.
    pub z_max: f64,
    /// Nodes in R.
    pub nr: usize,
    /// Nodes in Z.
    pub nz: usize,
}

impl Grid {
    /// A grid, validated: `0 < r_min < r_max`, `z_min < z_max`, both finite, and at least three nodes
    /// in each direction.
    pub fn new(
        r_min: f64,
        r_max: f64,
        z_min: f64,
        z_max: f64,
        nr: usize,
        nz: usize,
    ) -> Result<Self, NeutronicsError> {
        let finite = [r_min, r_max, z_min, z_max].iter().all(|v| v.is_finite());
        if !(finite && r_min > 0.0 && r_min < r_max && z_min < z_max && nr >= 3 && nz >= 3) {
            return Err(NeutronicsError::BadGrid);
        }
        Ok(Self {
            r_min,
            r_max,
            z_min,
            z_max,
            nr,
            nz,
        })
    }

    /// The spacing in R.
    pub fn dr(&self) -> f64 {
        (self.r_max - self.r_min) / (self.nr - 1) as f64
    }

    /// The spacing in Z.
    pub fn dz(&self) -> f64 {
        (self.z_max - self.z_min) / (self.nz - 1) as f64
    }

    /// The R coordinate of column `i`.
    pub fn r(&self, i: usize) -> f64 {
        self.r_min + i as f64 * self.dr()
    }

    /// The Z coordinate of row `j`.
    pub fn z(&self, j: usize) -> f64 {
        self.z_min + j as f64 * self.dz()
    }

    fn index(&self, i: usize, j: usize) -> usize {
        j * self.nr + i
    }

    fn on_boundary(&self, i: usize, j: usize) -> bool {
        i == 0 || j == 0 || i == self.nr - 1 || j == self.nz - 1
    }
}

/// The Solov'ev function `ψ = A R⁴ + B R² Z²`, which solves `Δ*ψ = (8A + 2B) R²` exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solovev {
    /// The coefficient of `R⁴`.
    pub a: f64,
    /// The coefficient of `R² Z²`.
    pub b: f64,
}

impl Solovev {
    /// The flux function at `(r, z)`.
    pub fn psi(&self, r: f64, z: f64) -> f64 {
        let r2 = r * r;
        self.a * r2 * r2 + self.b * r2 * z * z
    }

    /// The source coefficient κ in `Δ*ψ = κ R²`.
    pub fn kappa(&self) -> f64 {
        8.0 * self.a + 2.0 * self.b
    }
}

/// The result of a solve.
#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    /// The flux at each node, row-major: index `j * nr + i` is the node `(R_i, Z_j)`.
    pub psi: Vec<f64>,
    /// The sweeps used.
    pub iterations: usize,
    /// The largest change in a node in the last sweep.
    pub last_change: f64,
}

impl Solution {
    /// The flux at node `(i, j)` of `grid`.
    pub fn at(&self, grid: &Grid, i: usize, j: usize) -> f64 {
        self.psi[grid.index(i, j)]
    }
}

/// Successive over-relaxation parameter. Chosen for the grids used in the tests; any value in (1, 2)
/// converges for this operator.
pub const OMEGA: f64 = 1.8;

/// Solves `Δ*ψ = κ R²` on `grid`, with `ψ = boundary(R, Z)` on the edge. Iterates until no node
/// changes by more than `tolerance` in a sweep, or fails with [`NeutronicsError::NotConverged`] after
/// `max_iterations` sweeps. Fails with [`NeutronicsError::BadTolerance`] for a tolerance that is not
/// positive and finite.
pub fn solve_fixed_boundary<F>(
    grid: &Grid,
    kappa: f64,
    boundary: F,
    tolerance: f64,
    max_iterations: usize,
) -> Result<Solution, NeutronicsError>
where
    F: Fn(f64, f64) -> f64,
{
    if !(tolerance > 0.0 && tolerance.is_finite()) {
        return Err(NeutronicsError::BadTolerance);
    }
    let (nr, nz) = (grid.nr, grid.nz);
    let (h, k) = (grid.dr(), grid.dz());
    let mut psi = vec![0.0_f64; nr * nz];
    for j in 0..nz {
        for i in 0..nr {
            if grid.on_boundary(i, j) {
                psi[grid.index(i, j)] = boundary(grid.r(i), grid.z(j));
            }
        }
    }
    let center = -2.0 / (h * h) - 2.0 / (k * k);

    for iteration in 1..=max_iterations {
        let mut largest: f64 = 0.0;
        for j in 1..nz - 1 {
            for i in 1..nr - 1 {
                let r = grid.r(i);
                // Δ*ψ at a node: the R-neighbours carry the first-derivative correction -1/(2hR).
                let east = 1.0 / (h * h) - 1.0 / (2.0 * h * r);
                let west = 1.0 / (h * h) + 1.0 / (2.0 * h * r);
                let north = 1.0 / (k * k);
                let south = 1.0 / (k * k);
                let idx = grid.index(i, j);
                let neighbours = east * psi[grid.index(i + 1, j)]
                    + west * psi[grid.index(i - 1, j)]
                    + north * psi[grid.index(i, j + 1)]
                    + south * psi[grid.index(i, j - 1)];
                let gauss_seidel = (kappa * r * r - neighbours) / center;
                let updated = (1.0 - OMEGA) * psi[idx] + OMEGA * gauss_seidel;
                largest = largest.max((updated - psi[idx]).abs());
                psi[idx] = updated;
            }
        }
        let last_change = largest;
        if largest <= tolerance {
            return Ok(Solution {
                psi,
                iterations: iteration,
                last_change,
            });
        }
    }
    Err(NeutronicsError::NotConverged)
}

/// The largest difference between the solution and the exact function at the interior nodes.
pub fn max_interior_error(
    grid: &Grid,
    solution: &Solution,
    exact: impl Fn(f64, f64) -> f64,
) -> f64 {
    let mut worst: f64 = 0.0;
    for j in 1..grid.nz - 1 {
        for i in 1..grid.nr - 1 {
            let error = (solution.at(grid, i, j) - exact(grid.r(i), grid.z(j))).abs();
            worst = worst.max(error);
        }
    }
    worst
}
