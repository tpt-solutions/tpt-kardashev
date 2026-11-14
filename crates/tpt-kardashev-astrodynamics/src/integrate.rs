// SPDX-License-Identifier: MIT OR Apache-2.0
//! Numerical propagation: a fixed-step classical Runge–Kutta (RK4) integrator on position and
//! velocity, with the accelerations from a [`ForceModel`] (Cowell's formulation).
//!
//! Error: RK4 is fourth order, so over a fixed arc the global error falls as the fourth power of
//! the step. The tests check that ratio. Step size is the caller's choice; the propagator does not
//! adapt it. Adaptive high-order methods are a separate todo item.

use crate::force::ForceModel;
use crate::{Error, Seconds, StateVector, Vec3};

/// Position and velocity derivatives at one stage: `(dr/dt, dv/dt) = (v, a)`.
fn derivative<M: ForceModel + ?Sized>(
    model: &M,
    t: f64,
    position: Vec3,
    velocity: Vec3,
) -> Result<(Vec3, Vec3), Error> {
    let state = StateVector { position, velocity };
    let a = model.acceleration(Seconds::new(t)?, &state)?;
    Ok((velocity, a))
}

/// One classical RK4 step of `h` seconds, starting at time `t` (seconds past the epoch).
///
/// `h` may be negative, which integrates backwards.
pub fn rk4_step<M: ForceModel + ?Sized>(
    model: &M,
    t: f64,
    state: &StateVector,
    h: f64,
) -> Result<StateVector, Error> {
    let (r, v) = (state.position, state.velocity);
    let (k1r, k1v) = derivative(model, t, r, v)?;
    let (k2r, k2v) = derivative(model, t + 0.5 * h, r + k1r * (0.5 * h), v + k1v * (0.5 * h))?;
    let (k3r, k3v) = derivative(model, t + 0.5 * h, r + k2r * (0.5 * h), v + k2v * (0.5 * h))?;
    let (k4r, k4v) = derivative(model, t + h, r + k3r * h, v + k3v * h)?;
    let weight = h / 6.0;
    Ok(StateVector {
        position: r + (k1r + k2r * 2.0 + k3r * 2.0 + k4r) * weight,
        velocity: v + (k1v + k2v * 2.0 + k3v * 2.0 + k4v) * weight,
    })
}

/// A numerical propagator: a force model and a fixed step.
#[derive(Clone, Copy, Debug)]
pub struct Cowell<M> {
    model: M,
    step: Seconds,
}

impl<M: ForceModel> Cowell<M> {
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
            y = rk4_step(&self.model, t, &y, direction * h)?;
            t += direction * h;
        }
        let remainder = span - full_steps as f64 * h;
        if remainder > 0.0 {
            y = rk4_step(&self.model, t, &y, direction * remainder)?;
        }
        Ok(y)
    }
}
