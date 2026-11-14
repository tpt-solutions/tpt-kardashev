// SPDX-License-Identifier: MIT OR Apache-2.0
//! In-situ resource planning: elemental mass-balance checks, LP planning over a horizon and closure scores.
//!
//! Implemented: elements with atomic masses ([`element`]), materials as elemental compositions and
//! the elemental mass balance of a process ([`material`]), and the errors ([`error`]). Planning
//! (the processes and the linear program) needs the `alloc` feature and is in `plan`. Closure
//! scores, the energy account and the worked regolith example are not written yet.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod element;
pub mod error;
pub mod material;
#[cfg(feature = "alloc")]
pub mod plan;
#[cfg(feature = "alloc")]
pub mod simplex;

pub use element::{ELEMENT_COUNT, Element};
pub use error::IsruError;
pub use material::{Composition, Material, check_balance};
#[cfg(feature = "alloc")]
pub use plan::{Plan, Process, plan_maximum};
