// SPDX-License-Identifier: MIT OR Apache-2.0
//! Adaptive numerical propagation with the Dormand–Prince 5(4) embedded Runge–Kutta pair.
//!
//! Each step is computed to fifth order, and the difference from the embedded fourth-order result
//! estimates the local error. A step is accepted when the error, scaled by the absolute and relative
//! tolerances on each of the six state components, is at most one; otherwise it is retried with a
//! smaller step. The next step is chosen from the error with the usual fifth-root rule. The
//! propagation ends exactly at the requested time.
//!
//! Not modelled: event location, dense output, and a bound on the total step count beyond a large
//! fixed limit (a propagation that exceeds it returns [`Error::NoConvergence`]).

use crate::force::ForceModel;
use crate::{Error, Seconds, StateVector, Vec3};

/// Largest number of steps (accepted and rejected) in one propagation.
const MAX_STEPS: usize = 1_000_000;

/// A position and velocity pair, used for the stage arithmetic.
#[derive(Clone, Copy, Debug)]
struct Pair {
    r: Vec3,
    v: Vec3,
}

impl Pair {
    fn plus(self, other: Pair) -> Pair {
        Pair {
            r: self.r + other.r,
            v: self.v + other.v,
        }
    }

    fn times(self, s: f64) -> Pair {
        Pair {
            r: self.r * s,
            v: self.v * s,
        }
    }
}

/// An adaptive propagator: a force model, a relative and an absolute tolerance.
#[derive(Clone, Copy, Debug)]
pub struct AdaptiveCowell<M> {
    model: M,
    rtol: f64,
    atol: f64,
}

impl<M: ForceModel> AdaptiveCowell<M> {
    /// A propagator with relative tolerance `rtol` and absolute tolerance `atol` (in the units of the
    /// state: km and km/s). Fails with [`Error::InvalidTolerance`] unless both are positive and finite.
    pub fn new(model: M, rtol: f64, atol: f64) -> Result<Self, Error> {
        if rtol.is_finite() && rtol > 0.0 && atol.is_finite() && atol > 0.0 {
            Ok(Self { model, rtol, atol })
        } else {
            Err(Error::InvalidTolerance)
        }
    }

    /// The state `duration` seconds after `state`; a negative duration integrates backwards.
    pub fn propagate(&self, state: &StateVector, duration: Seconds) -> Result<StateVector, Error> {
        let total = duration.get();
        if !total.is_finite() {
            return Err(Error::NonFinite { what: "duration" });
        }
        if total == 0.0 {
            return Ok(*state);
        }
        let direction = total.signum();
        let span = total.abs();
        let mut y = Pair {
            r: state.position,
            v: state.velocity,
        };
        let mut t = 0.0f64;
        let mut h = span / 100.0;
        for _ in 0..MAX_STEPS {
            if t >= span {
                return Ok(StateVector {
                    position: y.r,
                    velocity: y.v,
                });
            }
            let remaining = span - t;
            let trial = h.min(remaining);
            let (y_new, err) = self.step(t * direction, y, trial * direction)?;
            if !err.is_finite() {
                return Err(Error::NoConvergence);
            }
            if err <= 1.0 {
                t += trial;
                y = y_new;
                // Grow by at most a factor of five when the error is well inside the tolerance.
                let grow = if err == 0.0 {
                    5.0
                } else {
                    (0.9 * err.powf(-0.2)).clamp(0.2, 5.0)
                };
                h = trial * grow;
            } else {
                // Shrink, by at least a factor of ten, and retry from the same time.
                h = trial * (0.9 * err.powf(-0.2)).clamp(0.1, 0.9);
            }
        }
        Err(Error::NoConvergence)
    }

    /// The derivative of the state at time `t` (seconds past the start): velocity and acceleration.
    fn derivative(&self, t: f64, y: Pair) -> Result<Pair, Error> {
        let state = StateVector {
            position: y.r,
            velocity: y.v,
        };
        let a = self.model.acceleration(Seconds::new(t)?, &state)?;
        Ok(Pair { r: y.v, v: a })
    }

    /// One Dormand–Prince step of size `h` from time `t`: the fifth-order result and the scaled error
    /// norm. The step's error is the difference from the embedded fourth-order result.
    fn step(&self, t: f64, y: Pair, h: f64) -> Result<(Pair, f64), Error> {
        // Butcher tableau of the Dormand–Prince 5(4) pair.
        let k1 = self.derivative(t, y)?;
        let k2 = self.derivative(t + h / 5.0, y.plus(k1.times(h / 5.0)))?;
        let k3 = self.derivative(
            t + 3.0 * h / 10.0,
            y.plus(k1.times(3.0 * h / 40.0).plus(k2.times(9.0 * h / 40.0))),
        )?;
        let k4 = self.derivative(
            t + 4.0 * h / 5.0,
            y.plus(
                k1.times(44.0 * h / 45.0)
                    .plus(k2.times(-56.0 * h / 15.0))
                    .plus(k3.times(32.0 * h / 9.0)),
            ),
        )?;
        let k5 = self.derivative(
            t + 8.0 * h / 9.0,
            y.plus(
                k1.times(19_372.0 * h / 6_561.0)
                    .plus(k2.times(-25_360.0 * h / 2_187.0))
                    .plus(k3.times(64_448.0 * h / 6_561.0))
                    .plus(k4.times(-212.0 * h / 729.0)),
            ),
        )?;
        let k6 = self.derivative(
            t + h,
            y.plus(
                k1.times(9_017.0 * h / 3_168.0)
                    .plus(k2.times(-355.0 * h / 33.0))
                    .plus(k3.times(46_732.0 * h / 5_247.0))
                    .plus(k4.times(49.0 * h / 176.0))
                    .plus(k5.times(-5_103.0 * h / 18_656.0)),
            ),
        )?;
        // Fifth-order solution, and the derivative at its end for the embedded fourth-order one.
        let y5 = y.plus(
            k1.times(35.0 * h / 384.0)
                .plus(k3.times(500.0 * h / 1_113.0))
                .plus(k4.times(125.0 * h / 192.0))
                .plus(k5.times(-2_187.0 * h / 6_784.0))
                .plus(k6.times(11.0 * h / 84.0)),
        );
        let k7 = self.derivative(t + h, y5)?;
        let y4 = y.plus(
            k1.times(5_179.0 * h / 57_600.0)
                .plus(k3.times(7_571.0 * h / 16_695.0))
                .plus(k4.times(393.0 * h / 640.0))
                .plus(k5.times(-92_097.0 * h / 339_200.0))
                .plus(k6.times(187.0 * h / 2_100.0))
                .plus(k7.times(h / 40.0)),
        );
        // Scaled RMS error over the six components.
        let scaled = |a: f64, b: f64, e: f64| {
            let sc = self.atol + self.rtol * a.abs().max(b.abs());
            (e / sc) * (e / sc)
        };
        let mut sum = 0.0;
        let errs = [
            (y.r.x, y5.r.x, y5.r.x - y4.r.x),
            (y.r.y, y5.r.y, y5.r.y - y4.r.y),
            (y.r.z, y5.r.z, y5.r.z - y4.r.z),
            (y.v.x, y5.v.x, y5.v.x - y4.v.x),
            (y.v.y, y5.v.y, y5.v.y - y4.v.y),
            (y.v.z, y5.v.z, y5.v.z - y4.v.z),
        ];
        for (a, b, e) in errs {
            sum += scaled(a, b, e);
        }
        Ok((y5, (sum / 6.0).sqrt()))
    }
}
