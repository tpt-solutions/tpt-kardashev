// SPDX-License-Identifier: MIT OR Apache-2.0
//! Flux surfaces and the safety factor on the circular flux function ψ = c((R − R0)² + Z²), whose
//! contours are circles about the axis and whose safety factor is known in closed form:
//! `q(r) = F / (2c √(R0² − r²))`.
#![cfg(all(feature = "std", feature = "alloc"))]
#![allow(clippy::expect_used)]

use tpt_kardashev_nuclear_plasma::NeutronicsError;
use tpt_kardashev_nuclear_plasma::equilibrium::Grid;
use tpt_kardashev_nuclear_plasma::surface::{bilinear, contour, gradient_magnitude, safety_factor};

const C: f64 = 1.0;
const R0: f64 = 2.0;
const F: f64 = 2.0;

fn psi_exact(r: f64, z: f64) -> f64 {
    C * ((r - R0) * (r - R0) + z * z)
}

fn field(grid: &Grid) -> Vec<f64> {
    let mut psi = Vec::with_capacity(grid.nr * grid.nz);
    for j in 0..grid.nz {
        for i in 0..grid.nr {
            psi.push(psi_exact(grid.r(i), grid.z(j)));
        }
    }
    psi
}

fn grid() -> Grid {
    Grid::new(1.0, 3.0, -1.0, 1.0, 81, 81).expect("grid")
}

fn q_exact(r: f64) -> f64 {
    F / (2.0 * C * (R0 * R0 - r * r).sqrt())
}

#[test]
fn bilinear_interpolation_reproduces_linear_functions_and_rejects_points_outside() {
    let g = grid();
    let linear: Vec<f64> = (0..g.nz)
        .flat_map(|j| (0..g.nr).map(move |i| (i, j)))
        .map(|(i, j)| 3.0 * g.r(i) - 2.0 * g.z(j) + 1.0)
        .collect();
    let value = bilinear(&g, &linear, 1.7, 0.3).expect("inside");
    assert!((value - (3.0 * 1.7 - 2.0 * 0.3 + 1.0)).abs() < 1e-12);
    assert_eq!(bilinear(&g, &linear, 0.5, 0.0), None);
}

#[test]
fn the_gradient_of_the_circular_flux_is_two_c_r() {
    let g = grid();
    let psi = field(&g);
    // |∇ψ| = 2c·|(R − R0, Z)| = 2c·r for a point at distance r from the axis.
    let (r, z) = (2.3, 0.2);
    let distance = ((r - R0) * (r - R0) + z * z).sqrt();
    let grad = gradient_magnitude(&g, &psi, r, z).expect("inside");
    assert!((grad - 2.0 * C * distance).abs() < 1e-3, "gradient {grad}");
}

#[test]
fn the_contour_is_the_circle_about_the_axis() {
    let g = grid();
    let psi = field(&g);
    let r_surface = 0.5;
    let level = C * r_surface * r_surface;
    let points = contour(&g, &psi, (R0, 0.0), level, 64).expect("contour");
    assert_eq!(points.len(), 64);
    for (r, z) in &points {
        let distance = ((r - R0) * (r - R0) + z * z).sqrt();
        assert!(
            (distance - r_surface).abs() < 0.01,
            "point at distance {distance}"
        );
    }
}

#[test]
fn the_safety_factor_matches_the_closed_form_on_several_surfaces() {
    let g = grid();
    let psi = field(&g);
    let mut previous = 0.0;
    for r_surface in [0.3, 0.5, 0.8] {
        let level = C * r_surface * r_surface;
        let points = contour(&g, &psi, (R0, 0.0), level, 256).expect("contour");
        let q = safety_factor(&g, &psi, &points, F).expect("q");
        let expected = q_exact(r_surface);
        let relative = (q - expected).abs() / expected;
        assert!(relative < 0.02, "r {r_surface}: q {q}, expected {expected}");
        assert!(q > previous, "q grows outward");
        previous = q;
    }
}

#[test]
fn a_level_the_grid_does_not_reach_has_no_surface() {
    let g = grid();
    let psi = field(&g);
    // Far above the flux anywhere on the grid, so no ray can cross it.
    assert_eq!(
        contour(&g, &psi, (R0, 0.0), 1.0e6, 16),
        Err(NeutronicsError::NoSurface)
    );
}

#[test]
fn too_few_rays_and_too_short_contours_are_rejected() {
    let g = grid();
    let psi = field(&g);
    assert_eq!(
        contour(&g, &psi, (R0, 0.0), 0.25, 2),
        Err(NeutronicsError::BadGrid)
    );
    assert_eq!(
        safety_factor(&g, &psi, &[(2.0, 0.0), (2.5, 0.0)], F),
        Err(NeutronicsError::NoSurface)
    );
}
