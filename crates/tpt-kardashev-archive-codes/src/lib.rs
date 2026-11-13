// SPDX-License-Identifier: MIT OR Apache-2.0
//! Archival and error-correcting codes: Reed-Solomon, interleaving, convolutional codes with
//! Viterbi decoding, CRC variants and attached sync markers.
//!
//! Every block codec works on caller-provided buffers and needs no allocation. The LT fountain-code
//! module (`lt`) keeps pending symbols in memory and needs the `alloc` feature, which `std` enables.
//!
//! Standards parameters in this crate are checked against CCSDS 131.0-B-5 (September 2023): the RS
//! field, generator and dual basis (`dual`), and the K=7 convolutional code with its G2 inverter.
//! The standard gives no codeword vectors, so this crate makes no compliance claim beyond those checks.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod codec;
pub mod conv;
pub mod crc;
pub mod dual;
pub mod error;
pub mod frame;
pub mod interleave;
pub mod ldpc;
#[cfg(feature = "alloc")]
pub mod lt;
pub mod rs;
pub mod sync;

#[cfg(feature = "serde")]
mod serde_impls;

mod gf;

pub use codec::{BlockDecoder, BlockEncoder, ErasureCoder, SoftDecoder};
pub use dual::DualBasisRs;
pub use error::{CodeError, DecodeReport};
pub use ldpc::LdpcCode;
pub use rs::ReedSolomon;
