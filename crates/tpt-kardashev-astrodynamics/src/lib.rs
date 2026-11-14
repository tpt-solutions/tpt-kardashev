// SPDX-License-Identifier: MIT OR Apache-2.0
//! Time scales, reference frames, orbital elements, propagation, Lambert solving and impulsive manoeuvres.
//!
//! Implemented so far: unit newtypes ([`Kilometres`], [`Seconds`], [`Radians`]), a small vector
//! type, and Keplerian elements for elliptic two-body orbits, with conversions to and from state
//! vectors ([`elements`], which needs the `std` feature for trigonometry). Not yet: time scales,
//! reference frames, propagation, force models, Lambert solving, manoeuvres and ephemerides.
//! See the workspace `todo.md` for this crate's v0.1 scope.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
pub mod adaptive;
#[cfg(feature = "std")]
pub mod atmosphere;
#[cfg(feature = "std")]
pub mod elements;
#[cfg(feature = "std")]
pub mod ephemeris;
pub mod error;
#[cfg(feature = "std")]
pub mod force;
#[cfg(feature = "std")]
pub mod frames;
#[cfg(feature = "std")]
pub mod integrate;
#[cfg(feature = "std")]
pub mod lambert;
#[cfg(feature = "std")]
pub mod manoeuvre;
pub mod matrix;
#[cfg(feature = "std")]
pub mod propagate;
#[cfg(feature = "std")]
pub mod sdp4;
#[cfg(feature = "std")]
pub mod sgp4;
#[cfg(feature = "std")]
pub mod spk;
#[cfg(feature = "std")]
pub mod symplectic;
pub mod time;
pub mod units;
pub mod vector;
#[cfg(feature = "std")]
pub mod visibility;

#[cfg(feature = "std")]
pub use elements::{Keplerian, StateVector};
pub use error::Error;
#[cfg(feature = "std")]
pub use force::{CentralBody, ForceModel, J2Zonal};
#[cfg(feature = "std")]
pub use integrate::Cowell;
pub use matrix::Mat3;
pub use time::{Epoch, LeapEntry, LeapSeconds, UtcTime};
pub use units::{Kilometres, Radians, Seconds};
pub use vector::Vec3;
