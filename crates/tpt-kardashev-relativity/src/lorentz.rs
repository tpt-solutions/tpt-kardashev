// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lorentz transformations as 4x4 matrices acting on `(c t, x, y, z)`.
//!
//! Convention: `Lorentz::boost(v)` is the coordinate transformation into a frame that moves
//! with velocity `v` relative to the original frame. It is the passive transform
//! `t' = gamma (t - beta.x)`, `x' = x + (gamma - 1)(beta.x) beta / beta^2 - gamma beta t`.

use crate::error::{Error, validate_speed};
use crate::event::FourVector;
use crate::math::{atanh, exp, sqrt};
use crate::units::{C, Metres};

/// A 4x4 matrix in row-major order, indices `(c t, x, y, z)`.
pub type Matrix4 = [[f64; 4]; 4];

/// A Lorentz transformation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lorentz {
    m: Matrix4,
}

impl Lorentz {
    /// The identity transformation.
    pub fn identity() -> Self {
        let mut m = [[0.0; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Self { m }
    }

    /// Boost to a frame moving with velocity `v` (m/s). Requires `|v| < c`.
    pub fn boost(v: [f64; 3]) -> Result<Self, Error> {
        if v.iter().any(|c| !c.is_finite()) {
            return Err(Error::NonFinite);
        }
        let v2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        let speed = validate_speed(sqrt(v2))?;
        if speed == 0.0 {
            return Ok(Self::identity());
        }
        let beta = [v[0] / C, v[1] / C, v[2] / C];
        let b2 = v2 / (C * C);
        let gamma = 1.0 / sqrt(1.0 - b2);
        let mut m = [[0.0; 4]; 4];
        m[0][0] = gamma;
        for (i, &bi) in beta.iter().enumerate() {
            m[0][i + 1] = -gamma * bi;
            m[i + 1][0] = -gamma * bi;
        }
        for (i, &bi) in beta.iter().enumerate() {
            for (j, &bj) in beta.iter().enumerate() {
                let delta = if i == j { 1.0 } else { 0.0 };
                m[i + 1][j + 1] = delta + (gamma - 1.0) * bi * bj / b2;
            }
        }
        Ok(Self { m })
    }

    /// Boost along +x with rapidity `eta`: `beta = tanh(eta)`, `gamma = cosh(eta)`.
    ///
    /// Rapidities add under composition of collinear boosts, which is what makes this form useful.
    pub fn boost_x_rapidity(eta: f64) -> Result<Self, Error> {
        if !eta.is_finite() {
            return Err(Error::NonFinite);
        }
        let (e, einv) = (exp(eta), exp(-eta));
        let ch = 0.5 * (e + einv);
        let sh = 0.5 * (e - einv);
        let mut m = Self::identity().m;
        m[0][0] = ch;
        m[0][1] = -sh;
        m[1][0] = -sh;
        m[1][1] = ch;
        Ok(Self { m })
    }

    /// Rapidity `atanh(v / c)` of a speed `v` along an axis.
    pub fn rapidity_from_speed(speed: f64) -> Result<f64, Error> {
        let v = validate_speed(speed)?;
        Ok(atanh(v / C))
    }

    /// Composes two transformations: `self` applied after `other`, i.e. the matrix `self * other`.
    pub fn compose(&self, other: &Lorentz) -> Lorentz {
        let mut m = [[0.0; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..4).map(|k| self.m[i][k] * other.m[k][j]).sum();
            }
        }
        Lorentz { m }
    }

    /// Applies the transformation to a four-vector.
    pub fn apply(&self, v: FourVector) -> FourVector {
        let input = [v.ct.0, v.x[0].0, v.x[1].0, v.x[2].0];
        let mut out = [0.0; 4];
        for (i, o) in out.iter_mut().enumerate() {
            *o = (0..4).map(|k| self.m[i][k] * input[k]).sum();
        }
        FourVector {
            ct: Metres(out[0]),
            x: [Metres(out[1]), Metres(out[2]), Metres(out[3])],
        }
    }

    /// The underlying matrix.
    pub fn matrix(&self) -> Matrix4 {
        self.m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_superluminal_and_non_finite() {
        assert!(matches!(
            Lorentz::boost([C, 0.0, 0.0]),
            Err(Error::SpeedNotSubluminal { .. })
        ));
        assert!(matches!(
            Lorentz::boost([0.0, 2.0 * C, 0.0]),
            Err(Error::SpeedNotSubluminal { .. })
        ));
        assert_eq!(Lorentz::boost([f64::NAN, 0.0, 0.0]), Err(Error::NonFinite));
    }

    #[test]
    fn zero_velocity_is_identity() {
        assert_eq!(Lorentz::boost([0.0; 3]), Ok(Lorentz::identity()));
    }

    #[test]
    fn x_boost_matches_general_boost() {
        let eta: f64 = 0.7;
        // tanh(eta) written via exp, since the crate has no tanh.
        let e2 = exp(2.0 * eta);
        let beta = (e2 - 1.0) / (e2 + 1.0);
        let general = Lorentz::boost([beta * C, 0.0, 0.0]).expect("subluminal");
        let rapid = Lorentz::boost_x_rapidity(eta).expect("finite");
        for (a, b) in general
            .matrix()
            .iter()
            .flatten()
            .zip(rapid.matrix().iter().flatten())
        {
            assert!((a - b).abs() < 1e-12);
        }
    }
}
