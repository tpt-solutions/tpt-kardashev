// SPDX-License-Identifier: MIT OR Apache-2.0
//! A 3×3 matrix of `f64`, used for frame rotations. Products are plain sums, with no trigonometry,
//! so this module is `no_std`.

use crate::Vec3;
use core::ops::Mul;

/// A 3×3 matrix, stored by rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3 {
    /// The rows of the matrix.
    pub rows: [[f64; 3]; 3],
}

impl Mat3 {
    /// The identity matrix.
    pub const IDENTITY: Self = Self {
        rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    /// The matrix with the given rows.
    pub const fn from_rows(rows: [[f64; 3]; 3]) -> Self {
        Self { rows }
    }

    /// The matrix times a vector.
    pub fn mul_vec(self, v: Vec3) -> Vec3 {
        let [a, b, c] = self.rows;
        Vec3::new(
            a[0] * v.x + a[1] * v.y + a[2] * v.z,
            b[0] * v.x + b[1] * v.y + b[2] * v.z,
            c[0] * v.x + c[1] * v.y + c[2] * v.z,
        )
    }

    /// The transpose. For a rotation, this is the inverse.
    pub fn transpose(self) -> Self {
        let m = self.rows;
        Self {
            rows: [
                [m[0][0], m[1][0], m[2][0]],
                [m[0][1], m[1][1], m[2][1]],
                [m[0][2], m[1][2], m[2][2]],
            ],
        }
    }

    /// The determinant.
    pub fn determinant(self) -> f64 {
        let m = self.rows;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }
}

impl Mul for Mat3 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let mut rows = [[0.0; 3]; 3];
        for (i, row) in rows.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.rows[i][k] * other.rows[k][j]).sum();
            }
        }
        Self { rows }
    }
}
