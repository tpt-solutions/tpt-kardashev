// SPDX-License-Identifier: MIT OR Apache-2.0
//! Causal ordering and hierarchical consensus (domains and federation) with consistency bounds stated per level.
//!
//! Implemented so far: the hybrid logical clock ([`clock`]) and fixed-size vector clocks
//! ([`vector`]). The operation log, the domain and federation layers, and the consistency levels
//! are not written yet. See the workspace `todo.md` for this crate's v0.1 scope.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod causal;
pub mod clock;
pub mod error;
pub mod vector;

#[cfg(feature = "alloc")]
pub use causal::{CausalBroadcast, Envelope};
pub use clock::{HybridClock, Timestamp};
pub use error::ClockError;
pub use vector::{CausalRelation, VectorClock};
