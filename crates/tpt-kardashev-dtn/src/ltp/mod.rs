// SPDX-License-Identifier: MIT OR Apache-2.0
//! LTP, the Licklider Transmission Protocol (RFC 5326): segment encoding and red-part sessions.
//!
//! The segment codec is in [`segment`]. [`session`] holds the sender and receiver for reliable
//! transmission of one block. [`ranges`] holds the byte-range sets both use.

pub mod ranges;
pub mod segment;
pub mod session;
