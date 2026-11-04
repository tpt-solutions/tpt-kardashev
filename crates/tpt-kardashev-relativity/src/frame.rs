// SPDX-License-Identifier: MIT OR Apache-2.0
//! Inertial frames and frame-to-frame coordinate transforms.

use crate::error::Error;
use crate::event::{Event, FourVector};
use crate::lorentz::Lorentz;

/// An inertial frame, specified by its velocity relative to a parent frame.
///
/// Coordinates of an event in the frame are obtained from the parent coordinates by the boost
/// `Lorentz::boost(velocity)`. Origins are not translated: this is a pure Lorentz transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    velocity: [f64; 3],
    to_frame: Lorentz,
    from_frame: Lorentz,
}

impl Frame {
    /// Creates a frame moving with `velocity` (m/s) relative to the parent. Requires `|v| < c`.
    pub fn new(velocity: [f64; 3]) -> Result<Self, Error> {
        let to_frame = Lorentz::boost(velocity)?;
        let from_frame = Lorentz::boost([-velocity[0], -velocity[1], -velocity[2]])?;
        Ok(Self {
            velocity,
            to_frame,
            from_frame,
        })
    }

    /// The frame's velocity relative to its parent, in m/s.
    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }

    /// Coordinates of a parent-frame four-vector, as seen in this frame.
    pub fn to_frame_vector(&self, v: FourVector) -> FourVector {
        self.to_frame.apply(v)
    }

    /// Parent-frame coordinates of a four-vector given in this frame.
    pub fn from_frame_vector(&self, v: FourVector) -> FourVector {
        self.from_frame.apply(v)
    }

    /// Coordinates of a parent-frame event, as seen in this frame.
    pub fn to_frame(&self, e: &Event) -> Event {
        Event::from_four_vector(self.to_frame_vector(e.four_vector()))
    }

    /// Parent-frame coordinates of an event given in this frame.
    pub fn from_frame(&self, e: &Event) -> Event {
        Event::from_four_vector(self.from_frame_vector(e.four_vector()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Metres, Seconds};

    #[test]
    fn round_trip_is_identity() {
        let frame = Frame::new([0.3 * crate::units::C, 0.1 * crate::units::C, 0.0]).expect("ok");
        let e = Event::new(
            Seconds(12.5),
            [Metres(1.0e6), Metres(-2.0e6), Metres(3.0e5)],
        );
        let back = frame.from_frame(&frame.to_frame(&e));
        assert!((back.t.0 - e.t.0).abs() < 1e-9);
        for (a, b) in back.x.iter().zip(e.x.iter()) {
            assert!((a.0 - b.0).abs() < 1e-3);
        }
    }

    #[test]
    fn rejects_superluminal_frame() {
        assert!(Frame::new([crate::units::C, 0.0, 0.0]).is_err());
    }
}
