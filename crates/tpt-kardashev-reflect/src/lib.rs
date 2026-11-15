// SPDX-License-Identifier: MIT OR Apache-2.0
//! Verified self-modification: linear-invariant containment certificates, a checker and an audited update gate.
//!
//! Implemented: exact rational arithmetic and errors (always), and, with the `alloc` feature, linear
//! constraint systems and certificates (`invariant`) and the containment checker (`verify`). Only
//! linear bounds are supported. The update gate, receipts, the audit chain and the hash are not
//! written yet.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod error;
#[cfg(feature = "alloc")]
pub mod invariant;
pub mod rational;
#[cfg(feature = "alloc")]
pub mod verify;

pub use error::ReflectError;
#[cfg(feature = "alloc")]
pub use invariant::{Certificate, Constraints, Invariant, Row, Update};
pub use rational::Rational;
#[cfg(feature = "alloc")]
pub use verify::verify;
