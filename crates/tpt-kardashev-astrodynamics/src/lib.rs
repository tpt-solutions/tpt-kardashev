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
pub mod elements;
pub mod error;
pub mod units;
pub mod vector;

#[cfg(feature = "std")]
pub use elements::{Keplerian, StateVector};
pub use error::Error;
pub use units::{Kilometres, Radians, Seconds};
pub use vector::Vec3;
