// SPDX-License-Identifier: MIT OR Apache-2.0
//! Delay-tolerant networking: BPv7 bundles, bundle stores, contact-graph routing and an LTP convergence layer.
//!
//! Phase 0 scaffold: no public API yet. See the workspace `todo.md` for this crate's v0.1 scope.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;
