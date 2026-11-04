// SPDX-License-Identifier: MIT OR Apache-2.0
//! Archival and error-correcting codes: Reed-Solomon, interleaving, convolutional codes with
//! Viterbi decoding, CRC variants and attached sync markers.
//!
//! Every codec works on caller-provided buffers and needs no allocation. The `alloc` feature is
//! reserved for later APIs and is not used yet.
//!
//! Standards parameters in this crate (Reed-Solomon RS(255, 223) and the K=7 convolutional code)
//! match the ECSS-adopted draft of CCSDS 131.0-B-3. They have not yet been checked against the
//! current revision or against known-answer vectors, so this crate makes no compliance claim.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod codec;
pub mod conv;
pub mod crc;
pub mod error;
pub mod frame;
pub mod interleave;
pub mod rs;
pub mod sync;

mod gf;

pub use codec::{BlockDecoder, BlockEncoder};
pub use error::{CodeError, DecodeReport};
pub use rs::ReedSolomon;
