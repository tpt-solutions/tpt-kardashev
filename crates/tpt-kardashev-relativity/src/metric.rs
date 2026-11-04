// SPDX-License-Identifier: MIT OR Apache-2.0
//! Metric tensors, signature (+, -, -, -), for later geodesic work.
//!
//! Coordinates are passed as `[ct, q1, q2, q3]`. For `Minkowski` they are Cartesian, in metres.
//! For `Schwarzschild` they are `[ct, r, theta, phi]`, with `r` in metres and angles in radians.
//! Geodesic integration is not in v0.1.

use crate::error::{Error, validate_radius};
use crate::math::sin;
use crate::units::C;

/// A 4x4 metric component matrix in the coordinate order given to `Metric::g`.
pub type Metric4 = [[f64; 4]; 4];

/// A spacetime metric.
pub trait Metric {
    /// The metric tensor components `g_{mu nu}` at coordinates `x = [ct, q1, q2, q3]`.
    fn g(&self, x: &[f64; 4]) -> Result<Metric4, Error>;
}

/// Flat Minkowski spacetime in Cartesian coordinates, `diag(1, -1, -1, -1)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Minkowski;

impl Metric for Minkowski {
    fn g(&self, x: &[f64; 4]) -> Result<Metric4, Error> {
        if x.iter().any(|v| !v.is_finite()) {
            return Err(Error::NonFinite);
        }
        let mut g = [[0.0; 4]; 4];
        g[0][0] = 1.0;
        g[1][1] = -1.0;
        g[2][2] = -1.0;
        g[3][3] = -1.0;
        Ok(g)
    }
}

/// Schwarzschild spacetime with gravitational parameter `GM` (m^3/s^2), in Schwarzschild coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Schwarzschild {
    gm: f64,
}

impl Schwarzschild {
    /// Creates the metric for gravitational parameter `gm >= 0`.
    pub fn new(gm: f64) -> Result<Self, Error> {
        if !gm.is_finite() {
            return Err(Error::NonFinite);
        }
        if gm < 0.0 {
            return Err(Error::InvalidParameter { name: "gm" });
        }
        Ok(Self { gm })
    }
}

impl Metric for Schwarzschild {
    fn g(&self, x: &[f64; 4]) -> Result<Metric4, Error> {
        if x.iter().any(|v| !v.is_finite()) {
            return Err(Error::NonFinite);
        }
        let rs = 2.0 * self.gm / (C * C);
        let r = validate_radius(x[1])?;
        if r <= rs {
            return Err(Error::InsideHorizon {
                radius: r,
                schwarzschild_radius: rs,
            });
        }
        let f = 1.0 - rs / r;
        let s = sin(x[2]);
        let mut g = [[0.0; 4]; 4];
        g[0][0] = f;
        g[1][1] = -1.0 / f;
        g[2][2] = -r * r;
        g[3][3] = -r * r * s * s;
        Ok(g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minkowski_is_diagonal() {
        let g = Minkowski.g(&[0.0; 4]).expect("ok");
        assert_eq!(g[0][0], 1.0);
        assert_eq!(g[1][1], -1.0);
        assert_eq!(g[0][1], 0.0);
    }

    #[test]
    fn schwarzschild_limits() {
        let m = Schwarzschild::new(3.986e14).expect("ok");
        // Far away, the metric approaches Minkowski.
        let g = m
            .g(&[0.0, 1.0e12, core::f64::consts::FRAC_PI_2, 0.0])
            .expect("ok");
        assert!((g[0][0] - 1.0).abs() < 1e-9);
        assert!(matches!(
            m.g(&[0.0, 1.0e-3, 1.0, 0.0]),
            Err(Error::InsideHorizon { .. })
        ));
    }
}
