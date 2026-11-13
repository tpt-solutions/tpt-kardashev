// SPDX-License-Identifier: MIT OR Apache-2.0
//! Delay-tolerant networking: BPv7 bundles, bundle stores, contact-graph routing and an LTP
//! convergence layer.
//!
//! This crate provides the bundle codec (a strict CBOR reader and writer, endpoint IDs for the
//! `dtn` and `ipn` schemes, the two block CRCs, and bundle encoding and decoding with the
//! validation rules this crate implements), bundle stores (in memory and in files), contact plans,
//! and earliest-arrival routing over a contact plan. The LTP layer and the simulator are not
//! written yet.
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
pub mod sdnv;

#[cfg(feature = "alloc")]
pub mod bundle;
#[cfg(feature = "alloc")]
pub mod contact;
#[cfg(feature = "alloc")]
pub mod convergence;
#[cfg(feature = "alloc")]
pub mod eid;
#[cfg(feature = "alloc")]
pub mod ltp;
#[cfg(feature = "alloc")]
pub mod route;
#[cfg(feature = "alloc")]
pub mod sim;
#[cfg(feature = "alloc")]
pub mod store;

pub use error::Error;

#[cfg(feature = "alloc")]
pub use bundle::{
    Bundle, BundleId, CanonicalBlock, CrcType, CreationTimestamp, Fragment, PrimaryBlock,
};
#[cfg(feature = "alloc")]
pub use contact::{Contact, ContactPlan};
#[cfg(feature = "alloc")]
pub use eid::Eid;
#[cfg(feature = "alloc")]
pub use route::{EarliestArrival, Hop, Route, Router};
#[cfg(feature = "alloc")]
pub use store::{BundleStore, MemoryStore};
