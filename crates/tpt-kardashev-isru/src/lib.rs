// SPDX-License-Identifier: MIT OR Apache-2.0
//! In-situ resource planning: elemental mass-balance checks, LP planning over a horizon and closure scores.
//!
//! Phase 0 scaffold: no public API yet. See the workspace `todo.md` for this crate's v0.1 scope.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;
