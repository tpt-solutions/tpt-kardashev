// SPDX-License-Identifier: MIT OR Apache-2.0
//! Velocity Verlet: a second-order, time-reversible integrator for accelerations that depend only
//! on position, such as central gravity, the J2 zonal, and the third-body and solar-pressure terms
//! that do not read velocity.
//!
//! Over long arcs the method's error in energy stays bounded and does not grow secularly, unlike
//! a fixed-step RK4 whose energy error drifts. The guarantee holds only when the acceleration does
//! not depend on velocity. [`ForceModel`] receives the full state, so this crate cannot check that.
//! Atmospheric drag and finite thrust read velocity; for those models the integrator still runs
//! but without the bounded-energy property, so use [`crate::integrate::Cowell`] or
//! [`crate::adaptive::AdaptiveCowell`] instead.
//!
//! Error: the global error falls as the square of the step. The tests check that ratio.

use crate::force::ForceModel;
use crate::{Error, Seconds, StateVector};

/// One velocity Verlet step of `h` seconds, starting at time `t` (seconds past the epoch).
///
/// The second acceleration is evaluated at the predicted velocity `v + a·h`. For a
/// velocity-independent model this value does not affect the result.
///
/// `h` may be negative, which integrates backwards.
pub fn verlet_step<M: ForceModel + ?Sized>(
    model: &M,
    t: f64,
    state: &StateVector,
    h: f64,
) -> Result<StateVector, Error> {
    let (x, v) = (state.position, state.velocity);
    let a0 = model.acceleration(Seconds::new(t)?, state)?;
    let x1 = x + v * h + a0 * (0.5 * h * h);
    let predicted = StateVector {
        position: x1,
        velocity: v + a0 * h,
    };
    let a1 = model.acceleration(Seconds::new(t + h)?, &predicted)?;
    Ok(StateVector {
        position: x1,
        velocity: v + (a0 + a1) * (0.5 * h),
    })
}

/// A symplectic propagator: a force model and a fixed step.
#[derive(Clone, Copy, Debug)]
pub struct Verlet<M> {
    model: M,
    step: Seconds,
}

impl<M: ForceModel> Verlet<M> {
    /// A propagator with `model` and steps of `step`. Fails with [`Error::InvalidStep`] unless
    /// `step` is positive and finite.
    pub fn new(model: M, step: Seconds) -> Result<Self, Error> {
        let h = step.get();
        if h.is_finite() && h > 0.0 {
            Ok(Self { model, step })
        } else {
            Err(Error::InvalidStep)
        }
    }

    /// The state `duration` seconds after `state`. A negative duration integrates backwards. The
    /// last step is shortened to land exactly on the end time.
    pub fn propagate(&self, state: &StateVector, duration: Seconds) -> Result<StateVector, Error> {
        let total = duration.get();
        let direction = if total < 0.0 { -1.0 } else { 1.0 };
        let span = total.abs();
        let h = self.step.get();
        let full_steps = (span / h) as usize;
        let mut y = *state;
        let mut t = 0.0;
        for _ in 0..full_steps {
            y = verlet_step(&self.model, t, &y, direction * h)?;
            t += direction * h;
        }
        let remainder = span - full_steps as f64 * h;
        if remainder > 0.0 {
            y = verlet_step(&self.model, t, &y, direction * remainder)?;
        }
        Ok(y)
    }
}
