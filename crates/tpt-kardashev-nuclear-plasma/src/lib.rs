// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fixed-boundary Grad-Shafranov equilibrium and fixed-source Monte Carlo neutronics with a cross-section reader.
//!
//! Implemented: the error type and a seeded generator (always, `no_std`), and with the `std` feature a
//! one-group fixed-source Monte Carlo for a slab and an infinite medium ([`slab`]), with analog and
//! implicit-capture estimators. Not written yet: the equilibrium solver, the cross-section reader and
//! the multigroup types, the constructive solid geometry, and the statistical suite beyond the slab
//! checks. See the workspace `todo.md` for this crate's v0.1 scope.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod equilibrium;
pub mod error;
#[cfg(feature = "std")]
pub mod layers;
pub mod rng;
#[cfg(feature = "std")]
pub mod slab;
#[cfg(all(feature = "std", feature = "alloc"))]
pub mod surface;
#[cfg(all(feature = "std", feature = "alloc"))]
pub mod xs;

pub use error::NeutronicsError;
pub use rng::SplitMix64;
