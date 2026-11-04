// SPDX-License-Identifier: MIT OR Apache-2.0
//! Self-describing archival frames.
//!
//! Layout, all multi-byte fields big-endian:
//!
//! ```text
//! offset  size  field
//! 0       1     version (1)
//! 1       1     code kind (0 uncoded, 1 Reed-Solomon, 2 convolutional K=7)
//! 2       1     nroots (Reed-Solomon parity symbols, else 0)
//! 3       1     interleave depth (1 = none)
//! 4       2     fcr (Reed-Solomon first root exponent, else 0)
//! 6       1     prim (Reed-Solomon root step, else 0)
//! 7       2     payload length in bytes
//! 9       2     CRC-16/CCITT-FALSE of bytes 0..9
//! 11      len   payload
//! 11+len  4     CRC-32C of the payload
//! ```
//!
//! The frame describes how its payload is coded, so a reader needs no out-of-band
//! configuration. The header CRC protects the parameters and the trailer protects the payload.
//! Neither CRC corrects errors. The caller applies forward error correction to the payload
//! using the recorded parameters.

use crate::crc::{crc16_ccitt_false, crc32c};
use crate::error::CodeError;

/// Frame format version implemented by this crate.
pub const VERSION: u8 = 1;
/// Bytes in the header, including its CRC.
pub const HEADER_LEN: usize = 11;
/// Bytes in the payload CRC trailer.
pub const TRAILER_LEN: usize = 4;

/// How the payload is coded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodeKind {
    /// No forward error correction.
    Uncoded,
    /// Systematic Reed-Solomon over GF(2^8).
    ReedSolomon,
    /// Rate-1/2, constraint-length-7 convolutional code.
    ConvolutionalK7,
}

impl CodeKind {
    fn code(self) -> u8 {
        match self {
            CodeKind::Uncoded => 0,
            CodeKind::ReedSolomon => 1,
            CodeKind::ConvolutionalK7 => 2,
        }
    }

    fn from_code(code: u8) -> Result<Self, CodeError> {
        match code {
            0 => Ok(CodeKind::Uncoded),
            1 => Ok(CodeKind::ReedSolomon),
            2 => Ok(CodeKind::ConvolutionalK7),
            _ => Err(CodeError::InvalidParameter { name: "code kind" }),
        }
    }
}

/// Coding parameters recorded in a frame header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameParams {
    /// The code.
    pub kind: CodeKind,
    /// Reed-Solomon parity symbols, or 0.
    pub nroots: u8,
    /// Interleave depth, at least 1.
    pub depth: u8,
    /// Reed-Solomon first root exponent, or 0.
    pub fcr: u16,
    /// Reed-Solomon root step, or 0.
    pub prim: u8,
}

impl FrameParams {
    /// Uncoded payload, no interleaving.
    pub fn uncoded() -> Self {
        Self {
            kind: CodeKind::Uncoded,
            nroots: 0,
            depth: 1,
            fcr: 0,
            prim: 0,
        }
    }

    /// Reed-Solomon parameters, with the given interleave depth.
    pub fn reed_solomon(nroots: u8, fcr: u16, prim: u8, depth: u8) -> Self {
        Self {
            kind: CodeKind::ReedSolomon,
            nroots,
            depth,
            fcr,
            prim,
        }
    }

    /// The convolutional code (K=7, rate 1/2).
    pub fn convolutional_k7() -> Self {
        Self {
            kind: CodeKind::ConvolutionalK7,
            nroots: 0,
            depth: 1,
            fcr: 0,
            prim: 0,
        }
    }
}

/// A parsed frame, with its payload borrowed from the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame<'a> {
    /// Coding parameters from the header.
    pub params: FrameParams,
    /// The payload, coded as `params` describes.
    pub payload: &'a [u8],
}

/// Total encoded length of a frame carrying `payload_len` bytes.
pub fn encoded_len(payload_len: usize) -> usize {
    HEADER_LEN + payload_len + TRAILER_LEN
}

/// Writes a frame into `out`. `out.len()` must equal `encoded_len(payload.len())`.
pub fn encode(params: FrameParams, payload: &[u8], out: &mut [u8]) -> Result<(), CodeError> {
    if payload.len() > usize::from(u16::MAX) {
        return Err(CodeError::InvalidParameter {
            name: "payload length",
        });
    }
    if params.depth == 0 {
        return Err(CodeError::InvalidParameter { name: "depth" });
    }
    let total = encoded_len(payload.len());
    if out.len() != total {
        return Err(CodeError::LengthMismatch {
            expected: total,
            actual: out.len(),
        });
    }
    out[0] = VERSION;
    out[1] = params.kind.code();
    out[2] = params.nroots;
    out[3] = params.depth;
    out[4..6].copy_from_slice(&params.fcr.to_be_bytes());
    out[6] = params.prim;
    out[7..9].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    let header_crc = crc16_ccitt_false(&out[..9]);
    out[9..11].copy_from_slice(&header_crc.to_be_bytes());
    out[HEADER_LEN..HEADER_LEN + payload.len()].copy_from_slice(payload);
    out[HEADER_LEN + payload.len()..].copy_from_slice(&crc32c(payload).to_be_bytes());
    Ok(())
}

/// Parses and validates a frame. The payload is borrowed, not copied.
pub fn decode(bytes: &[u8]) -> Result<Frame<'_>, CodeError> {
    if bytes.len() < HEADER_LEN + TRAILER_LEN {
        return Err(CodeError::LengthMismatch {
            expected: HEADER_LEN + TRAILER_LEN,
            actual: bytes.len(),
        });
    }
    let header = &bytes[..HEADER_LEN];
    let stored_header_crc = u16::from_be_bytes([header[9], header[10]]);
    if crc16_ccitt_false(&header[..9]) != stored_header_crc {
        return Err(CodeError::InvalidParameter { name: "header crc" });
    }
    if header[0] != VERSION {
        return Err(CodeError::InvalidParameter { name: "version" });
    }
    let kind = CodeKind::from_code(header[1])?;
    let depth = header[3];
    if depth == 0 {
        return Err(CodeError::InvalidParameter { name: "depth" });
    }
    let params = FrameParams {
        kind,
        nroots: header[2],
        depth,
        fcr: u16::from_be_bytes([header[4], header[5]]),
        prim: header[6],
    };
    let len = usize::from(u16::from_be_bytes([header[7], header[8]]));
    let expected = encoded_len(len);
    if bytes.len() != expected {
        return Err(CodeError::LengthMismatch {
            expected,
            actual: bytes.len(),
        });
    }
    let payload = &bytes[HEADER_LEN..HEADER_LEN + len];
    let trailer = &bytes[HEADER_LEN + len..];
    let stored = u32::from_be_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    if crc32c(payload) != stored {
        return Err(CodeError::InvalidParameter {
            name: "payload crc",
        });
    }
    Ok(Frame { params, payload })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_params_and_payload() {
        let payload = b"archival payload bytes";
        let params = FrameParams::reed_solomon(32, 112, 11, 4);
        let mut buf = [0u8; HEADER_LEN + 22 + TRAILER_LEN];
        encode(params, payload, &mut buf).expect("ok");
        let frame = decode(&buf).expect("valid");
        assert_eq!(frame.params, params);
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn any_single_byte_corruption_is_detected() {
        let payload = [0x5Au8; 16];
        let mut buf = [0u8; HEADER_LEN + 16 + TRAILER_LEN];
        encode(FrameParams::uncoded(), &payload, &mut buf).expect("ok");
        for i in 0..buf.len() {
            let mut bad = buf;
            bad[i] ^= 0x01;
            assert!(decode(&bad).is_err(), "byte {i} corruption accepted");
        }
    }

    #[test]
    fn rejects_unknown_version_even_with_valid_header_crc() {
        let mut buf = [0u8; HEADER_LEN + TRAILER_LEN];
        encode(FrameParams::uncoded(), &[], &mut buf).expect("ok");
        buf[0] = 2;
        let crc = crc16_ccitt_false(&buf[..9]).to_be_bytes();
        buf[9..11].copy_from_slice(&crc);
        assert_eq!(
            decode(&buf),
            Err(CodeError::InvalidParameter { name: "version" })
        );
    }

    #[test]
    fn arbitrary_bytes_never_panic() {
        // Deterministic pseudo-random inputs of every small length.
        let mut x: u64 = 0x1234_5678;
        for len in 0..80 {
            for _ in 0..50 {
                let bytes: std::vec::Vec<u8> = (0..len)
                    .map(|_| {
                        x = x
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1_442_695_040_888_963_407);
                        (x >> 56) as u8
                    })
                    .collect();
                let _ = decode(&bytes);
            }
        }
    }
}
