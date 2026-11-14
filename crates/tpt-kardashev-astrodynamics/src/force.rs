// SPDX-License-Identifier: MIT OR Apache-2.0
//! Accelerations for the numerical propagator.
//!
//! A [`ForceModel`] gives the acceleration of a spacecraft at a time and state. Models add: a
//! pair `(A, B)` of models is itself a model whose acceleration is the sum. Only central-body
//! gravity is written here. Zonal harmonics, third bodies, radiation pressure, drag and thrust
//! are not written yet.

use crate::{Error, Seconds, StateVector, Vec3};

/// An acceleration (km/s²) on a spacecraft at `t` seconds past the epoch, for a state in km and
/// km/s.
pub trait ForceModel {
    /// The acceleration at time `t` for `state`.
    fn acceleration(&self, t: Seconds, state: &StateVector) -> Result<Vec3, Error>;
}

/// Point-mass gravity of a central body with gravitational parameter μ (km³/s²):
/// `a = −μ r / |r|³`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CentralBody {
    mu: f64,
}

impl CentralBody {
    /// Gravity of a body with parameter `mu`. Fails with [`Error::InvalidMu`] unless `mu` is
    /// positive and finite.
    pub fn new(mu: f64) -> Result<Self, Error> {
        if mu.is_finite() && mu > 0.0 {
            Ok(Self { mu })
        } else {
            Err(Error::InvalidMu)
        }
    }

    /// The gravitational parameter (km³/s²).
    pub fn mu(&self) -> f64 {
        self.mu
    }
}

impl ForceModel for CentralBody {
    fn acceleration(&self, _t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        let r = state.position.norm();
        if r.is_nan() || r <= 0.0 {
            return Err(Error::Degenerate);
        }
        Ok(state.position * (-self.mu / (r * r * r)))
    }
}

impl<A: ForceModel, B: ForceModel> ForceModel for (A, B) {
    fn acceleration(&self, t: Seconds, state: &StateVector) -> Result<Vec3, Error> {
        Ok(self.0.acceleration(t, state)? + self.1.acceleration(t, state)?)
    }
}
