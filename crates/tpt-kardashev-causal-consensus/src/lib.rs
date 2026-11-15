// SPDX-License-Identifier: MIT OR Apache-2.0
//! Causal ordering and hierarchical consensus (domains and federation) with consistency bounds stated per level.
//!
//! Implemented: hybrid logical and fixed-size vector clocks ([`clock`], [`vector`]); a causally
//! ordered log by causal broadcast (`causal`, feature `alloc`); consistency levels with latency bounds
//! ([`consistency`]); a message transport and a deterministic simulated network (`transport`, feature `alloc`);
//! Raft-style domain consensus (`domain`, feature `alloc`); hash-chained epoch summaries (`federation`, feature `alloc`, with
//! [`hash`]). Not yet: signatures, the Domain/Federation/Proposal types, and the consistency checks
//! across domains. See the workspace `todo.md` for this crate's v0.1 scope.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod causal;
pub mod clock;
pub mod consistency;
#[cfg(feature = "alloc")]
pub mod domain;
pub mod error;
#[cfg(feature = "alloc")]
pub mod federation;
#[cfg(feature = "alloc")]
pub mod global;
pub mod hash;
#[cfg(feature = "alloc")]
pub mod linearizable;
#[cfg(feature = "crypto")]
pub mod signed;
#[cfg(feature = "alloc")]
pub mod transport;
pub mod vector;

#[cfg(feature = "alloc")]
pub use causal::{CausalBroadcast, Envelope};
pub use clock::{HybridClock, Timestamp};
pub use consistency::{ConsistencyLevel, Delays};
#[cfg(feature = "alloc")]
pub use domain::{DomainError, Message as DomainMessage, RaftNode, Role};
pub use error::ClockError;
pub use vector::{CausalRelation, VectorClock};
