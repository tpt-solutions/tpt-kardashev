// SPDX-License-Identifier: MIT OR Apache-2.0
//! Events, four-vectors and interval classification, signature (+, -, -, -).

use crate::units::{C, Metres, Seconds};

/// Relative tolerance for classifying an interval as lightlike.
///
/// Compared against `(c dt)^2 + |dx|^2`. See `docs/tolerance-policy.md`.
pub const LIGHT_TOLERANCE: f64 = 1e-12;

/// A point in spacetime: coordinate time and Cartesian position in one inertial frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    /// Coordinate time.
    pub t: Seconds,
    /// Position as (x, y, z).
    pub x: [Metres; 3],
}

impl Event {
    /// Creates an event from coordinate time and position.
    pub const fn new(t: Seconds, x: [Metres; 3]) -> Self {
        Self { t, x }
    }

    /// Returns the four-vector `(c t, x, y, z)`, in metres.
    pub fn four_vector(&self) -> FourVector {
        FourVector {
            ct: Metres(C * self.t.0),
            x: self.x,
        }
    }

    /// Creates an event from a four-vector `(c t, x, y, z)`.
    pub fn from_four_vector(v: FourVector) -> Self {
        Self {
            t: Seconds(v.ct.0 / C),
            x: v.x,
        }
    }
}

/// A four-vector `(c t, x, y, z)` in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FourVector {
    /// Speed of light times coordinate time.
    pub ct: Metres,
    /// Spatial components.
    pub x: [Metres; 3],
}

/// How two events are separated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IntervalKind {
    /// One event can influence the other (positive `s^2`).
    Timelike,
    /// Neither event can influence the other (negative `s^2`).
    Spacelike,
    /// Connected by a light signal (zero `s^2`, within tolerance). Also the degenerate
    /// case of two identical events.
    Lightlike,
}

/// The invariant interval between two events, held as its two parts in m^2.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    ct2: f64,
    dx2: f64,
}

impl Interval {
    /// The invariant `s^2 = (c dt)^2 - |dx|^2`, in m^2.
    pub fn squared(&self) -> f64 {
        self.ct2 - self.dx2
    }

    /// Classifies the separation, with the tolerance `LIGHT_TOLERANCE`.
    pub fn kind(&self) -> IntervalKind {
        let scale = self.ct2 + self.dx2;
        if scale == 0.0 {
            return IntervalKind::Lightlike;
        }
        let tol = LIGHT_TOLERANCE * scale;
        let s2 = self.squared();
        if s2 > tol {
            IntervalKind::Timelike
        } else if s2 < -tol {
            IntervalKind::Spacelike
        } else {
            IntervalKind::Lightlike
        }
    }

    /// Proper time along a straight timelike segment, or `None` if the segment is not timelike.
    pub fn proper_time(&self) -> Option<Seconds> {
        match self.kind() {
            IntervalKind::Timelike => Some(Seconds(crate::math::sqrt(self.squared()) / C)),
            _ => None,
        }
    }
}

/// Computes the interval from `a` to `b`.
pub fn interval(a: &Event, b: &Event) -> Interval {
    let dt = b.t.0 - a.t.0;
    let ct2 = (C * dt) * (C * dt);
    let mut dx2 = 0.0;
    for (ax, bx) in a.x.iter().zip(b.x.iter()) {
        let d = bx.0 - ax.0;
        dx2 += d * d;
    }
    Interval { ct2, dx2 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(t: f64, x: f64, y: f64, z: f64) -> Event {
        Event::new(Seconds(t), [Metres(x), Metres(y), Metres(z)])
    }

    #[test]
    fn classifies_timelike_spacelike_lightlike() {
        let o = ev(0.0, 0.0, 0.0, 0.0);
        assert_eq!(
            interval(&o, &ev(2.0, 0.0, 0.0, 0.0)).kind(),
            IntervalKind::Timelike
        );
        assert_eq!(
            interval(&o, &ev(0.0, 1.0, 0.0, 0.0)).kind(),
            IntervalKind::Spacelike
        );
        assert_eq!(
            interval(&o, &ev(1.0, C, 0.0, 0.0)).kind(),
            IntervalKind::Lightlike
        );
        assert_eq!(interval(&o, &o).kind(), IntervalKind::Lightlike);
    }

    #[test]
    fn proper_time_of_timelike_interval() {
        let o = ev(0.0, 0.0, 0.0, 0.0);
        let iv = interval(&o, &ev(5.0, 0.0, 0.0, 0.0));
        assert_eq!(iv.proper_time(), Some(Seconds(5.0)));
        assert_eq!(interval(&o, &ev(0.0, 1.0, 0.0, 0.0)).proper_time(), None);
    }

    #[test]
    fn four_vector_round_trip() {
        let e = ev(3.0, 1.0, -2.0, 0.5);
        let back = Event::from_four_vector(e.four_vector());
        assert_eq!(back.t.0, e.t.0);
        assert_eq!(back.x, e.x);
    }
}
