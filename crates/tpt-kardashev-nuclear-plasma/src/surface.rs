// SPDX-License-Identifier: MIT OR Apache-2.0
//! Flux surfaces and the safety factor, from a flux function sampled on an (R, Z) grid.
//!
//! A flux surface `ψ = ψ_s` is found by casting rays from a magnetic axis supplied by the caller and
//! bisecting the crossing of the level along each ray, on the bilinear interpolant of the grid. This
//! assumes the surface is star-shaped about the axis, which holds for the shapes this crate
//! considers.
//!
//! The safety factor is `q = (F / 2π) ∮ dl / (R |∇ψ|)`, with the toroidal field `B_φ = F / R` and
//! `B_p = |∇ψ| / R`. The poloidal flux is taken per radian. The gradient is the central difference
//! of the bilinear interpolant at the midpoints of the contour's segments.

use alloc::vec::Vec;

use crate::equilibrium::Grid;
use crate::error::NeutronicsError;

/// Bilinear interpolation of the node values `psi` (row-major, as in [`crate::equilibrium::Solution`])
/// at `(r, z)`. Returns `None` outside the grid.
pub fn bilinear(grid: &Grid, psi: &[f64], r: f64, z: f64) -> Option<f64> {
    if !(r >= grid.r_min && r <= grid.r_max && z >= grid.z_min && z <= grid.z_max) {
        return None;
    }
    let fi = (r - grid.r_min) / grid.dr();
    let fj = (z - grid.z_min) / grid.dz();
    let i = (fi.floor() as usize).min(grid.nr - 2);
    let j = (fj.floor() as usize).min(grid.nz - 2);
    let tr = fi - i as f64;
    let tz = fj - j as f64;
    let at = |a: usize, b: usize| psi[b * grid.nr + a];
    Some(
        (1.0 - tr) * (1.0 - tz) * at(i, j)
            + tr * (1.0 - tz) * at(i + 1, j)
            + (1.0 - tr) * tz * at(i, j + 1)
            + tr * tz * at(i + 1, j + 1),
    )
}

/// The magnitude of `∇ψ` at `(r, z)`, from central differences of the bilinear interpolant.
pub fn gradient_magnitude(grid: &Grid, psi: &[f64], r: f64, z: f64) -> Option<f64> {
    let dr = 0.25 * grid.dr();
    let dz = 0.25 * grid.dz();
    let p_r_plus = bilinear(grid, psi, r + dr, z)?;
    let p_r_minus = bilinear(grid, psi, r - dr, z)?;
    let p_z_plus = bilinear(grid, psi, r, z + dz)?;
    let p_z_minus = bilinear(grid, psi, r, z - dz)?;
    let g_r = (p_r_plus - p_r_minus) / (2.0 * dr);
    let g_z = (p_z_plus - p_z_minus) / (2.0 * dz);
    Some((g_r * g_r + g_z * g_z).sqrt())
}

/// The points of the flux surface `ψ = level` about the magnetic axis `axis = (R, Z)`, found along
/// `rays` equally spaced directions. Fails with [`NeutronicsError::NoSurface`] when a ray does not
/// cross the level inside the grid, and with [`NeutronicsError::BadGrid`] for fewer than three rays.
pub fn contour(
    grid: &Grid,
    psi: &[f64],
    axis: (f64, f64),
    level: f64,
    rays: usize,
) -> Result<Vec<(f64, f64)>, NeutronicsError> {
    if rays < 3 {
        return Err(NeutronicsError::BadGrid);
    }
    let start = bilinear(grid, psi, axis.0, axis.1).ok_or(NeutronicsError::NoSurface)?;
    let inside = start < level;
    let max_length = (grid.r_max - grid.r_min).hypot(grid.z_max - grid.z_min);
    let step = 0.25 * grid.dr().min(grid.dz());
    let mut points = Vec::with_capacity(rays);
    for k in 0..rays {
        let angle = 2.0 * core::f64::consts::PI * k as f64 / rays as f64;
        let (dr, dz) = (angle.cos(), angle.sin());
        let at = |t: f64| bilinear(grid, psi, axis.0 + t * dr, axis.1 + t * dz);
        // March outwards to the first sample on the other side of the level.
        let mut lo = 0.0_f64;
        let mut hi = None;
        let mut t = step;
        while t <= max_length {
            match at(t) {
                Some(value) if (value < level) != inside => {
                    hi = Some(t);
                    break;
                }
                Some(_) => lo = t,
                None => break,
            }
            t += step;
        }
        let mut hi = hi.ok_or(NeutronicsError::NoSurface)?;
        // Bisect the bracket [lo, hi] for the crossing.
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            match at(mid) {
                Some(value) if (value < level) != inside => hi = mid,
                Some(_) => lo = mid,
                None => break,
            }
        }
        let t_cross = 0.5 * (lo + hi);
        points.push((axis.0 + t_cross * dr, axis.1 + t_cross * dz));
    }
    Ok(points)
}

/// The safety factor of the closed surface `contour` (as returned by [`contour`]), for the toroidal
/// flux function `F` (in T·m). Fails with [`NeutronicsError::NoSurface`] when the contour leaves the
/// grid or has fewer than three points.
pub fn safety_factor(
    grid: &Grid,
    psi: &[f64],
    contour: &[(f64, f64)],
    f_toroidal: f64,
) -> Result<f64, NeutronicsError> {
    if contour.len() < 3 {
        return Err(NeutronicsError::NoSurface);
    }
    let mut integral = 0.0_f64;
    for k in 0..contour.len() {
        let (r0, z0) = contour[k];
        let (r1, z1) = contour[(k + 1) % contour.len()];
        let length = (r1 - r0).hypot(z1 - z0);
        let (rm, zm) = (0.5 * (r0 + r1), 0.5 * (z0 + z1));
        let grad = gradient_magnitude(grid, psi, rm, zm).ok_or(NeutronicsError::NoSurface)?;
        if grad <= 0.0 {
            return Err(NeutronicsError::NoSurface);
        }
        integral += length / (rm * grad);
    }
    Ok(f_toroidal / (2.0 * core::f64::consts::PI) * integral)
}
