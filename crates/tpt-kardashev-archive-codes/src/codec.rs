// SPDX-License-Identifier: MIT OR Apache-2.0
//! Block coding traits.

use crate::error::{CodeError, DecodeReport};

/// Encodes a message block into a codeword.
pub trait BlockEncoder {
    /// Writes the codeword for `message` into `out`.
    fn encode(&self, message: &[u8], out: &mut [u8]) -> Result<(), CodeError>;
}

/// Decodes a received codeword in place.
pub trait BlockDecoder {
    /// Corrects `word` in place and reports how many symbols changed.
    ///
    /// On error the word is left unchanged.
    fn decode(&self, word: &mut [u8]) -> Result<DecodeReport, CodeError>;
}
