// SPDX-License-Identifier: MIT OR Apache-2.0
//! Delay-tolerant networking: BPv7 bundles, bundle stores, contact-graph routing and an LTP
//! convergence layer.
//!
//! This crate currently provides the bundle codec: a strict CBOR reader and writer, endpoint IDs
//! for the `dtn` and `ipn` schemes, the two block CRCs, and bundle encoding and decoding with the
//! validation rules this crate implements. Stores, routing, the LTP layer and the simulator are
//! not written yet.
//!
//! The bundle types need the `alloc` feature (enabled by `std`). The CBOR reader and the CRCs do
//! not allocate.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod cbor;
pub mod crc;
pub mod error;

#[cfg(feature = "alloc")]
pub mod bundle;
#[cfg(feature = "alloc")]
pub mod eid;

pub use error::Error;

#[cfg(feature = "alloc")]
pub use bundle::{Bundle, CanonicalBlock, CrcType, CreationTimestamp, Fragment, PrimaryBlock};
#[cfg(feature = "alloc")]
pub use eid::Eid;
