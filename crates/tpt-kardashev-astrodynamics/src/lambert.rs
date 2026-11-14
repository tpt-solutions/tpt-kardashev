// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lambert's problem: the orbit that joins two positions in a given time, by universal variables.
//!
//! Given the position at two instants and the time between them, [`lambert`] returns the velocity
//! at each end of the connecting orbit. The solution is found by bisection on the universal
//! variable z, using the Stumpff functions of [`crate::propagate`]. The bracket covers the
//! zero-revolution solutions of elliptic and hyperbolic transfers.
//!
//! The direction of travel is chosen by `prograde`: the transfer sweeps counter-clockwise about
//! +z when it is `true`, and clockwise when `false`. Which of the two arcs joining the positions
//! is flown (the shorter or the longer) follows from that choice and from the positions. A
//! transfer whose plane contains the z axis is not supported, because its sense cannot be read off
//! the z component of the angular momentum.
//!
//! Not modelled: multi-revolution transfers.

use crate::propagate::stumpff;
use crate::{Error, Seconds, Vec3};

/// Bracket on z for zero-revolution transfers: z < 4π² is where C(z) > 0.
const Z_BOUND: f64 = 4.0 * core::f64::consts::PI * core::f64::consts::PI;
/// Relative tolerance on the bracket width in z.
const Z_TOLERANCE: f64 = 1e-14;
/// Largest number of bisection steps.
const MAX_STEPS: usize = 300;

/// The velocities `(v1, v2)` at the start and end of the transfer from `r1` to `r2` in time `tof`,
/// under gravitational parameter `mu` (km³/s²). Positions are in km and velocities come back in
/// km/s.
///
/// Fails with [`Error::InvalidMu`], [`Error::NonFinite`], [`Error::InvalidTimeOfFlight`], or
/// [`Error::Degenerate`] when the positions are zero, collinear with the origin, or their transfer
/// plane contains the z axis. Returns [`Error::NoConvergence`] if no solution is bracketed.
pub fn lambert(
    mu: f64,
    r1: Vec3,
    r2: Vec3,
    tof: Seconds,
    prograde: bool,
) -> Result<(Vec3, Vec3), Error> {
    if !(mu.is_finite() && mu > 0.0) {
        return Err(Error::InvalidMu);
    }
    if !r1.is_finite() {
        return Err(Error::NonFinite {
            what: "start position (km)",
        });
    }
    if !r2.is_finite() {
        return Err(Error::NonFinite {
            what: "end position (km)",
        });
    }
    let t = tof.get();
    if t <= 0.0 {
        return Err(Error::InvalidTimeOfFlight);
    }
    let r1n = r1.norm();
    let r2n = r2.norm();
    if r1n.is_nan() || r1n <= 0.0 || r2n.is_nan() || r2n <= 0.0 {
        return Err(Error::Degenerate);
    }

    // The transfer's angular momentum fixes the sweep. Its z component gives the sense of the
    // counter-clockwise sweep from r1 to r2; zero means the plane contains the z axis.
    let cross = r1.cross(r2);
    let cross_norm = cross.norm();
    if cross_norm <= 1e-12 * r1n * r2n || cross.z.abs() <= 1e-12 * cross_norm {
        return Err(Error::Degenerate);
    }
    let cos_dnu = (r1.dot(r2) / (r1n * r2n)).clamp(-1.0, 1.0);
    // A = sin(Δν)·sqrt(r1·r2/(1 − cos Δν)) = ±sqrt(r1·r2·(1 + cos Δν)), where the sign is the sine
    // of the sweep angle in the chosen sense. The counter-clockwise sweep has the sign of cross.z,
    // and the clockwise sweep the opposite.
    let sense = if prograde { 1.0 } else { -1.0 };
    let a = sense * cross.z.signum() * (r1n * r2n * (1.0 + cos_dnu)).sqrt();
    if a == 0.0 {
        return Err(Error::Degenerate);
    }

    let sqrt_mu = mu.sqrt();
    // y(z) and the time residual F(z), which is increasing in z where y > 0.
    let residual = |z: f64| -> Option<(f64, f64, f64, f64)> {
        let (c, s) = stumpff(z);
        if c <= 0.0 {
            return None;
        }
        let y = r1n + r2n + a * (z * s - 1.0) / c.sqrt();
        if y <= 0.0 {
            return None;
        }
        let chi = (y / c).sqrt();
        let f = chi * chi * chi * s + a * y.sqrt() - sqrt_mu * t;
        Some((f, y, c, s))
    };

    let mut lo = -Z_BOUND;
    let mut hi = Z_BOUND;
    let mut solution = None;
    for _ in 0..MAX_STEPS {
        let mid = 0.5 * (lo + hi);
        match residual(mid) {
            None => lo = mid, // y ≤ 0: the bracket is too low
            Some((f, y, c, s)) => {
                if f.abs() <= 1e-12 * sqrt_mu * t {
                    solution = Some((mid, y, c, s));
                    break;
                }
                if f < 0.0 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
        }
        if hi - lo <= Z_TOLERANCE * mid.abs().max(1.0) {
            break;
        }
    }
    let Some((_, y, _, _)) = solution.or_else(|| {
        let mid = 0.5 * (lo + hi);
        residual(mid).map(|(_, y, c, s)| (mid, y, c, s))
    }) else {
        return Err(Error::NoConvergence);
    };

    // Lagrange coefficients of the transfer, and the end velocities.
    let f = 1.0 - y / r1n;
    let g = a * (y / mu).sqrt();
    let g_dot = 1.0 - y / r2n;
    let v1 = (r2 - r1 * f) * (1.0 / g);
    let v2 = (r2 * g_dot - r1) * (1.0 / g);
    Ok((v1, v2))
}
