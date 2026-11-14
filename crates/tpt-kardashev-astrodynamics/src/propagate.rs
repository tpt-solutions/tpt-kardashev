// SPDX-License-Identifier: MIT OR Apache-2.0
//! Two-body propagation by universal variables.
//!
//! The state is advanced by the Lagrange (f and g) coefficients, with the universal anomaly χ found
//! by Newton's method on the universal Kepler equation. The same formulas cover elliptic,
//! parabolic and hyperbolic orbits, so the propagator is not tied to [`Keplerian`](crate::Keplerian),
//! which handles elliptic orbits only.
//!
//! The Stumpff functions C(z) and S(z) use a power series for |z| < 1 and closed forms otherwise.
//!
//! Not modelled: perturbations (force models come later in §2.2), and the time of periapsis or
//! any other event. A propagation that does not converge returns [`Error::NoConvergence`].

use crate::elements::StateVector;
use crate::{Error, Seconds};

/// Newton iterations allowed for the universal Kepler equation.
const MAX_ITERATIONS: usize = 100;
/// Relative tolerance on the step in χ.
const CHI_TOLERANCE: f64 = 1e-12;

/// The Stumpff functions `(C(z), S(z))`.
pub(crate) fn stumpff(z: f64) -> (f64, f64) {
    if z.abs() < 1.0 {
        // C(z) = Σ (−z)^k / (2k+2)!, S(z) = Σ (−z)^k / (2k+3)!, summed term by term.
        let mut c = 0.0;
        let mut s = 0.0;
        let mut power = 1.0;
        let mut fact_c = 2.0; // 2!
        let mut fact_s = 6.0; // 3!
        for k in 0..30 {
            let kf = k as f64;
            c += power / fact_c;
            s += power / fact_s;
            power *= -z;
            fact_c *= (2.0 * kf + 3.0) * (2.0 * kf + 4.0);
            fact_s *= (2.0 * kf + 4.0) * (2.0 * kf + 5.0);
        }
        (c, s)
    } else if z > 0.0 {
        let root = z.sqrt();
        ((1.0 - root.cos()) / z, (root - root.sin()) / (root * z))
    } else {
        let w = (-z).sqrt();
        ((w.cosh() - 1.0) / (-z), (w.sinh() - w) / (w * (-z)))
    }
}

/// The state `dt` seconds after `state`, under gravitational parameter `mu` (km³/s²).
///
/// Fails with [`Error::InvalidMu`] for a non-positive μ, [`Error::NonFinite`] for non-finite
/// inputs, [`Error::Degenerate`] for a zero-length position, and [`Error::NoConvergence`] if the
/// universal Kepler equation is not solved within the iteration limit.
pub fn propagate(state: &StateVector, mu: f64, dt: Seconds) -> Result<StateVector, Error> {
    if !(mu.is_finite() && mu > 0.0) {
        return Err(Error::InvalidMu);
    }
    if !state.position.is_finite() {
        return Err(Error::NonFinite {
            what: "position (km)",
        });
    }
    if !state.velocity.is_finite() {
        return Err(Error::NonFinite {
            what: "velocity (km/s)",
        });
    }
    let r0_vec = state.position;
    let v0_vec = state.velocity;
    let r0 = r0_vec.norm();
    if r0.is_nan() || r0 <= 0.0 {
        return Err(Error::Degenerate);
    }
    let t = dt.get();
    let sqrt_mu = mu.sqrt();
    let vr0 = r0_vec.dot(v0_vec) / r0;
    // 1/a for elliptic orbits (negative for hyperbolic ones).
    let alpha = 2.0 / r0 - v0_vec.dot(v0_vec) / mu;
    let mut chi = if alpha > 0.0 {
        sqrt_mu * alpha * t
    } else {
        sqrt_mu * t / r0
    };

    let a_coeff = r0 * vr0 / sqrt_mu;
    let b_coeff = 1.0 - alpha * r0;
    let mut converged = false;
    for _ in 0..MAX_ITERATIONS {
        let z = alpha * chi * chi;
        let (c, s) = stumpff(z);
        let f = a_coeff * chi * chi * c + b_coeff * chi * chi * chi * s + r0 * chi - sqrt_mu * t;
        let df = a_coeff * chi * (1.0 - z * s) + b_coeff * chi * chi * c + r0;
        if df == 0.0 || !df.is_finite() {
            return Err(Error::NoConvergence);
        }
        let step = f / df;
        chi -= step;
        if !chi.is_finite() {
            return Err(Error::NoConvergence);
        }
        if step.abs() <= CHI_TOLERANCE * chi.abs().max(1.0) {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(Error::NoConvergence);
    }

    let z = alpha * chi * chi;
    let (c, s) = stumpff(z);
    // Lagrange coefficients: r = f·r0 + g·v0, v = ḟ·r0 + ġ·v0.
    let f = 1.0 - chi * chi * c / r0;
    let g = t - chi * chi * chi * s / sqrt_mu;
    let position = r0_vec * f + v0_vec * g;
    let r = position.norm();
    if r.is_nan() || r <= 0.0 {
        return Err(Error::Degenerate);
    }
    let f_dot = sqrt_mu / (r * r0) * chi * (z * s - 1.0);
    let g_dot = 1.0 - chi * chi * c / r;
    let velocity = r0_vec * f_dot + v0_vec * g_dot;
    Ok(StateVector { position, velocity })
}
