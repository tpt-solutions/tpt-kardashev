// SPDX-License-Identifier: MIT OR Apache-2.0
//! The general Grad-Shafranov source `Δ*ψ = S(R, ψ)`: the pressure-only source reproduces the existing
//! solver exactly; an additive constant source (a toroidal-field term with constant FF') has a closed
//! form that the solver reaches with second-order error; and a source that depends on ψ converges, with
//! a small residual of the discrete equation.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::equilibrium::{
    Grid, Solovev, max_interior_error, max_residual, solve_fixed_boundary, solve_with_source,
};

const EXACT: Solovev = Solovev { a: 0.5, b: -0.2 };

fn grid(n: usize) -> Grid {
    Grid::new(1.0, 3.0, -1.0, 1.0, n, n).expect("grid")
}

#[test]
fn the_pressure_only_source_reproduces_the_fixed_source_solver_exactly() {
    let g = grid(13);
    let kappa = EXACT.kappa();
    let boundary = |r: f64, z: f64| EXACT.psi(r, z);
    let a = solve_fixed_boundary(&g, kappa, boundary, 1e-11, 200_000).expect("converges");
    let b = solve_with_source(&g, |r, _psi| kappa * r * r, boundary, 1e-11, 200_000)
        .expect("converges");
    assert_eq!(a, b);
}

/// The exact solution with source `κ R² + c`: `ψ = A R⁴ + B R² Z² + (c/2) Z²`, since Δ*(Z²) = 2.
fn with_constant(c: f64) -> impl Fn(f64, f64) -> f64 {
    move |r: f64, z: f64| EXACT.psi(r, z) + 0.5 * c * z * z
}

fn constant_error(n: usize, c: f64) -> f64 {
    let g = grid(n);
    let exact = with_constant(c);
    let solution = solve_with_source(
        &g,
        |r, _psi| EXACT.kappa() * r * r + c,
        &exact,
        1e-11,
        200_000,
    )
    .expect("converges");
    max_interior_error(&g, &solution, exact)
}

#[test]
fn a_constant_additive_source_has_the_closed_form_and_second_order_error() {
    // Interior error should fall by about four when the grid spacing is halved (n − 1 doubles).
    let coarse = constant_error(9, 0.3);
    let fine = constant_error(17, 0.3);
    assert!(
        fine < coarse,
        "error should fall: coarse {coarse}, fine {fine}"
    );
    let ratio = coarse / fine;
    assert!(
        (3.0..5.0).contains(&ratio),
        "error ratio {ratio} (coarse {coarse}, fine {fine})"
    );
}

#[test]
fn a_flux_dependent_source_converges_and_satisfies_the_discrete_equation() {
    // S = κ R² + λ ψ with a small λ: no closed form, so check the solver converged and that the
    // converged solution satisfies the discrete equation to the tolerance the solver stops at.
    let g = grid(17);
    let kappa = EXACT.kappa();
    let lambda = 0.05;
    let source = |r: f64, psi: f64| kappa * r * r + lambda * psi;
    let solution =
        solve_with_source(&g, source, |r, z| EXACT.psi(r, z), 1e-11, 200_000).expect("converges");
    let residual = max_residual(&g, &solution, source);
    assert!(
        residual < 1e-7,
        "residual {residual} after {} sweeps",
        solution.iterations
    );
    // The flux-dependent source changes the solution: at the interior nodes it differs from the
    // pressure-only solution by an amount that is neither zero nor large.
    let plain =
        solve_fixed_boundary(&g, kappa, |r, z| EXACT.psi(r, z), 1e-11, 200_000).expect("converges");
    let mut change: f64 = 0.0;
    for j in 1..g.nz - 1 {
        for i in 1..g.nr - 1 {
            change = change.max((solution.at(&g, i, j) - plain.at(&g, i, j)).abs());
        }
    }
    assert!(
        change > 1e-6,
        "the flux-dependent term has no effect: {change}"
    );
    assert!(
        change < 1.0,
        "the flux-dependent term is too large: {change}"
    );
}
