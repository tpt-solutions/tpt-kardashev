// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fixed-boundary Grad-Shafranov equilibrium and fixed-source Monte Carlo neutronics with a cross-section reader.
//!
//! Implemented: the error type and a seeded generator (always, `no_std`); with `alloc`, the fixed-boundary
//! equilibrium solver (`equilibrium`), flux surfaces and the safety factor (`surface`) and the
//! multigroup cross-section reader (`xs`); with `std`, a one-group fixed-source Monte Carlo for a slab and
//! an infinite medium ([`slab`]) with analog and implicit-capture estimators, one-dimensional layered
//! geometry ([`layers`]), and three-dimensional constructive solid geometry for point location and ray
//! crossings ([`csg`]), and one-group analog transport through that geometry ([`csg_transport`]). Not
//! written yet: multigroup or anisotropic transport, and the statistical suite beyond the slab checks. See the workspace `todo.md` for this crate's scope.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
pub mod csg;
#[cfg(feature = "std")]
pub mod csg_transport;
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
