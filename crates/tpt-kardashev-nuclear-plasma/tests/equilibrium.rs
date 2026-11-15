// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fixed-boundary Grad-Shafranov solve against the Solov'ev benchmark: the discrete operator's
//! consistency, second-order convergence, exact boundary values, convergence reporting, and rejection
//! of bad grids and tolerances.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::NeutronicsError;
use tpt_kardashev_nuclear_plasma::equilibrium::{
    Grid, Solovev, max_interior_error, solve_fixed_boundary,
};

const EXACT: Solovev = Solovev { a: 0.5, b: -0.2 };

fn grid(n: usize) -> Grid {
    Grid::new(1.0, 3.0, -1.0, 1.0, n, n).expect("grid")
}

fn solve(n: usize) -> (Grid, tpt_kardashev_nuclear_plasma::equilibrium::Solution) {
    let g = grid(n);
    let solution = solve_fixed_boundary(&g, EXACT.kappa(), |r, z| EXACT.psi(r, z), 1e-11, 200_000)
        .expect("converges");
    (g, solution)
}

#[test]
fn the_solovev_function_satisfies_the_continuum_equation() {
    // Δ*ψ for ψ = A R⁴ + B R² Z² is (8A + 2B) R². Check the discrete operator on the exact function:
    // its residual should shrink as h² when the grid is halved.
    let residual = |n: usize| {
        let g = grid(n);
        let (h, k) = (g.dr(), g.dz());
        let mut worst: f64 = 0.0;
        for j in 1..g.nz - 1 {
            for i in 1..g.nr - 1 {
                let r = g.r(i);
                let z = g.z(j);
                let east = EXACT.psi(g.r(i + 1), z);
                let west = EXACT.psi(g.r(i - 1), z);
                let north = EXACT.psi(r, g.z(j + 1));
                let south = EXACT.psi(r, g.z(j - 1));
                let centre = EXACT.psi(r, z);
                let operator = (east - 2.0 * centre + west) / (h * h)
                    - (east - west) / (2.0 * h * r)
                    + (north - 2.0 * centre + south) / (k * k);
                worst = worst.max((operator - EXACT.kappa() * r * r).abs());
            }
        }
        worst
    };
    let coarse = residual(21);
    let fine = residual(41);
    let ratio = coarse / fine;
    assert!(
        (3.0..5.0).contains(&ratio),
        "residual ratio {ratio}; expected about 4"
    );
}

#[test]
fn the_solution_matches_the_benchmark_and_the_error_falls_as_h_squared() {
    let (g_coarse, coarse) = solve(21);
    let (g_fine, fine) = solve(41);
    let e_coarse = max_interior_error(&g_coarse, &coarse, |r, z| EXACT.psi(r, z));
    let e_fine = max_interior_error(&g_fine, &fine, |r, z| EXACT.psi(r, z));
    let ratio = e_coarse / e_fine;
    assert!(
        (3.0..5.0).contains(&ratio),
        "error ratio {ratio}; second order expects about 4"
    );
    assert!(e_fine < 0.01, "finest interior error {e_fine}");
}

#[test]
fn the_boundary_values_are_kept_exactly() {
    let (g, solution) = solve(21);
    for j in 0..g.nz {
        for i in 0..g.nr {
            if i == 0 || j == 0 || i == g.nr - 1 || j == g.nz - 1 {
                let exact = EXACT.psi(g.r(i), g.z(j));
                assert_eq!(solution.at(&g, i, j), exact, "boundary node ({i}, {j})");
            }
        }
    }
}

#[test]
fn a_solve_that_runs_out_of_sweeps_says_so() {
    let g = grid(21);
    let result = solve_fixed_boundary(&g, EXACT.kappa(), |r, z| EXACT.psi(r, z), 1e-11, 2);
    assert_eq!(result, Err(NeutronicsError::NotConverged));
}

#[test]
fn the_solver_reports_its_sweeps_and_a_final_change_below_tolerance() {
    let (_, solution) = solve(21);
    assert!(solution.iterations > 1);
    assert!(solution.last_change <= 1e-11);
}

#[test]
fn bad_grids_and_tolerances_are_rejected() {
    assert_eq!(
        Grid::new(0.0, 3.0, -1.0, 1.0, 5, 5),
        Err(NeutronicsError::BadGrid),
        "R must be positive"
    );
    assert_eq!(
        Grid::new(3.0, 1.0, -1.0, 1.0, 5, 5),
        Err(NeutronicsError::BadGrid)
    );
    assert_eq!(
        Grid::new(1.0, 3.0, -1.0, 1.0, 2, 5),
        Err(NeutronicsError::BadGrid)
    );
    let g = grid(5);
    assert_eq!(
        solve_fixed_boundary(&g, 1.0, |_, _| 0.0, 0.0, 10),
        Err(NeutronicsError::BadTolerance)
    );
}
