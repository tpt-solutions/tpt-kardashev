// SPDX-License-Identifier: MIT OR Apache-2.0
//! Symplectic N-body propagation: velocity Verlet for N point masses that attract each other.
//!
//! Each body feels the gravity of all the others: `a_i = Σ_{j≠i} μ_j (r_j − r_i) / |r_j − r_i|³`, with
//! `μ_j` the gravitational parameter (G times the mass) of body `j`. The accelerations depend only on
//! positions, so the velocity Verlet method of [`crate::symplectic`] applies, and its energy error
//! stays bounded over long arcs. Units are the crate's: kilometres, seconds, and μ in km³/s².
//!
//! Needs `std` (for the square root) and `alloc` (for the body lists).
//!
//! Two bodies at the same point are an error, not a softened force. A close pass needs a step small
//! enough to resolve it; the propagator does not adapt its step. Relativistic and non-gravitational
//! forces are not modelled.

use alloc::vec::Vec;

use crate::vector::Vec3;
use crate::{Error, Seconds};

/// Positions and velocities of N bodies, in kilometres and kilometres per second.
#[derive(Clone, Debug, PartialEq)]
pub struct NBodyState {
    /// The position of each body.
    pub positions: Vec<Vec3>,
    /// The velocity of each body.
    pub velocities: Vec<Vec3>,
}

/// N point masses, each with its gravitational parameter `μ = G m`.
#[derive(Clone, Debug, PartialEq)]
pub struct NBodySystem {
    mu: Vec<f64>,
}

impl NBodySystem {
    /// A system of bodies with gravitational parameters `mu`. Fails with [`Error::BodyCount`] for an
    /// empty list, and with [`Error::InvalidMu`] for a parameter that is not positive and finite.
    pub fn new(mu: Vec<f64>) -> Result<Self, Error> {
        if mu.is_empty() {
            return Err(Error::BodyCount);
        }
        if mu.iter().any(|&m| !(m.is_finite() && m > 0.0)) {
            return Err(Error::InvalidMu);
        }
        Ok(Self { mu })
    }

    /// The number of bodies. Always at least one.
    pub fn len(&self) -> usize {
        self.mu.len()
    }

    /// Whether the system has no bodies. Always `false`, since [`NBodySystem::new`] refuses an empty list.
    pub fn is_empty(&self) -> bool {
        self.mu.is_empty()
    }

    /// The gravitational parameter of each body.
    pub fn mu(&self) -> &[f64] {
        &self.mu
    }

    /// The gravitational acceleration of each body at `positions`, one per body. Fails with
    /// [`Error::BodyCount`] when the number of positions differs from the number of bodies, with
    /// [`Error::Degenerate`] when two bodies share a point, and with [`Error::NonFinite`] when a
    /// position is not finite.
    pub fn accelerations(&self, positions: &[Vec3]) -> Result<Vec<Vec3>, Error> {
        if positions.len() != self.mu.len() {
            return Err(Error::BodyCount);
        }
        let mut out = Vec::with_capacity(positions.len());
        for (i, &xi) in positions.iter().enumerate() {
            if !xi.is_finite() {
                return Err(Error::NonFinite { what: "position" });
            }
            let mut a = Vec3::new(0.0, 0.0, 0.0);
            for (j, &xj) in positions.iter().enumerate() {
                if i == j {
                    continue;
                }
                let d = xj - xi;
                let r2 = d.dot(d);
                if r2.is_nan() || r2 <= 0.0 {
                    return Err(Error::Degenerate);
                }
                a = a + d * (self.mu[j] / (r2 * r2.sqrt()));
            }
            out.push(a);
        }
        Ok(out)
    }
}

/// One velocity Verlet step of `h` seconds for the N bodies, from `state`. `h` may be negative, which
/// integrates backwards. Fails as [`NBodySystem::accelerations`], with [`Error::BodyCount`] when the
/// velocities and positions differ in number, and with [`Error::NonFinite`] for a step that is not finite.
pub fn nbody_verlet_step(
    system: &NBodySystem,
    state: &NBodyState,
    h: f64,
) -> Result<NBodyState, Error> {
    if !h.is_finite() {
        return Err(Error::NonFinite { what: "step" });
    }
    if state.velocities.len() != state.positions.len() {
        return Err(Error::BodyCount);
    }
    let a0 = system.accelerations(&state.positions)?;
    let positions: Vec<Vec3> = state
        .positions
        .iter()
        .zip(state.velocities.iter())
        .zip(a0.iter())
        .map(|((&x, &v), &a)| x + v * h + a * (0.5 * h * h))
        .collect();
    // The accelerations do not read velocity, so the predicted velocity is not needed for them.
    let a1 = system.accelerations(&positions)?;
    let velocities: Vec<Vec3> = state
        .velocities
        .iter()
        .zip(a0.iter().zip(a1.iter()))
        .map(|(&v, (&a, &b))| v + (a + b) * (0.5 * h))
        .collect();
    Ok(NBodyState {
        positions,
        velocities,
    })
}

/// A symplectic N-body propagator: a system and a fixed step.
#[derive(Clone, Debug, PartialEq)]
pub struct NBodyVerlet {
    system: NBodySystem,
    step: Seconds,
}

impl NBodyVerlet {
    /// A propagator for `system` with steps of `step`. Fails with [`Error::InvalidStep`] unless `step`
    /// is positive and finite.
    pub fn new(system: NBodySystem, step: Seconds) -> Result<Self, Error> {
        let h = step.get();
        if h.is_finite() && h > 0.0 {
            Ok(Self { system, step })
        } else {
            Err(Error::InvalidStep)
        }
    }

    /// The system being propagated.
    pub fn system(&self) -> &NBodySystem {
        &self.system
    }

    /// The state `duration` seconds after `state`. A negative duration integrates backwards. The last
    /// step is shortened to land exactly on the end time. Fails as [`nbody_verlet_step`].
    pub fn propagate(&self, state: &NBodyState, duration: Seconds) -> Result<NBodyState, Error> {
        let total = duration.get();
        let direction = if total < 0.0 { -1.0 } else { 1.0 };
        let span = total.abs();
        let h = self.step.get();
        let full_steps = (span / h) as usize;
        let mut y = state.clone();
        for _ in 0..full_steps {
            y = nbody_verlet_step(&self.system, &y, direction * h)?;
        }
        let remainder = span - full_steps as f64 * h;
        if remainder > 0.0 {
            y = nbody_verlet_step(&self.system, &y, direction * remainder)?;
        }
        Ok(y)
    }
}
