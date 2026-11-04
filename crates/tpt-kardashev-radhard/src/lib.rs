// SPDX-License-Identifier: MIT OR Apache-2.0
//! Radiation-tolerant primitives: triple and N-modular redundancy with scrubbing, SECDED(72,64)
//! words, CRC-protected cells and buffers, a checkpoint with rollback and a watchdog marker, and
//! a seeded fault injector for tests.
//!
//! This crate protects memory and computation (words, cells, state). Data in transit and at rest
//! is the job of `tpt-kardashev-archive-codes`. The two crates do not depend on each other.
//!
//! `unsafe` is forbidden unless the `volatile` feature is enabled. That feature gates the only
//! module allowed to use `unsafe`: volatile register access, which needs a caller-checked address.
//!
//! No Kani proofs are included yet. The verifier is not installed in this environment. The
//! voter and SECDED properties are covered by exhaustive tests instead.

#![no_std]
#![cfg_attr(not(feature = "volatile"), forbid(unsafe_code))]
#![cfg_attr(feature = "volatile", deny(unsafe_code))]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod cell;
pub mod checkpoint;
pub mod error;
pub mod inject;
pub mod secded;
pub mod tmr;
#[cfg(feature = "volatile")]
pub mod volatile;

pub use cell::{CheckedCell, ChecksummedBuffer};
pub use checkpoint::{Checkpoint, Watchdog};
pub use error::Error;
pub use inject::FaultInjector;
pub use secded::{Codeword72, Outcome};
pub use tmr::{Nmr, Tmr, Unvoted, Voted};
