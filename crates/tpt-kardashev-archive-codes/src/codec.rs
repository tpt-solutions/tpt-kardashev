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

/// Decodes soft input (one confidence byte per coded bit: 0 certainly 0, 255 certainly 1)
/// into message bits.
pub trait SoftDecoder {
    /// Writes the decoded bits into `out` and returns the path metric of the chosen path.
    ///
    /// `scratch` is caller-provided working storage, so no allocation is needed. Implementations
    /// document the length they require.
    fn decode_soft(
        &self,
        soft: &[u8],
        out: &mut [u8],
        scratch: &mut [u64],
    ) -> Result<u32, CodeError>;
}

/// Decodes a codeword whose unreliable positions are known in advance.
pub trait ErasureCoder {
    /// Corrects `word` in place given the indices of erased (known-unreliable) symbols.
    ///
    /// On error the word is left unchanged.
    fn decode_erasures(
        &self,
        word: &mut [u8],
        erasures: &[usize],
    ) -> Result<DecodeReport, CodeError>;
}
