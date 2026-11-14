// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit newtypes. Each wraps a finite `f64`, so a value of one of these types is always finite.

use crate::Error;

/// A length in kilometres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Kilometres(f64);

/// A duration in seconds.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Seconds(f64);

/// An angle in radians.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Radians(f64);

fn finite(value: f64, what: &'static str) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::NonFinite { what })
    }
}

impl Kilometres {
    /// A length of `value` km. Fails if `value` is NaN or infinite.
    pub fn new(value: f64) -> Result<Self, Error> {
        finite(value, "length (km)").map(Self)
    }

    /// The length in km.
    pub fn get(self) -> f64 {
        self.0
    }
}

impl Seconds {
    /// A duration of `value` s. Fails if `value` is NaN or infinite.
    pub fn new(value: f64) -> Result<Self, Error> {
        finite(value, "duration (s)").map(Self)
    }

    /// The duration in s.
    pub fn get(self) -> f64 {
        self.0
    }
}

impl Radians {
    /// An angle of `value` rad. Fails if `value` is NaN or infinite.
    pub fn new(value: f64) -> Result<Self, Error> {
        finite(value, "angle (rad)").map(Self)
    }

    /// The angle in rad.
    pub fn get(self) -> f64 {
        self.0
    }
}
