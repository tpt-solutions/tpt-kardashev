// SPDX-License-Identifier: MIT OR Apache-2.0
//! A three-component vector of `f64`. Units are the caller's: the orbit code uses km and km/s.

use core::ops::{Add, Mul, Sub};

/// A Cartesian vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    /// The x component.
    pub x: f64,
    /// The y component.
    pub y: f64,
    /// The z component.
    pub z: f64,
}

impl Vec3 {
    /// The vector `(x, y, z)`.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// The dot product.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// The cross product `self × other`.
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// The Euclidean length.
    #[cfg(feature = "std")]
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Whether every component is finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f64) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}
