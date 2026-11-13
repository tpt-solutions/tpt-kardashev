// SPDX-License-Identifier: MIT OR Apache-2.0
//! BPv7 bundles (RFC 9171 §4): the primary block, canonical blocks, and the bundle as a whole.
//!
//! Encoding follows §4.1 and the CDDL of Appendix B. The bundle is an indefinite-length array of
//! blocks. Each block is a definite-length array, and its block-type-specific data is a definite
//! byte string. The CRC of a block is computed over the block's bytes with the CRC value zeroed.
//!
//! Decoding is strict. It verifies each CRC, rejects non-canonical CBOR, and checks the rules of
//! §4.2–4.4 that this crate implements (see [`Bundle::validate`]).

use crate::cbor::{ArrayLen, Reader, Writer};
use crate::crc::{crc16_x25, crc32c};
use crate::eid::Eid;
use crate::error::Error;
use alloc::vec::Vec;

/// Bundle Protocol version this crate implements.
pub const VERSION: u64 = 7;
/// Block type code of the bundle payload block.
pub const BLOCK_TYPE_PAYLOAD: u64 = 1;

/// Bundle processing control flags (§4.2.3), as bit masks.
pub mod bundle_flags {
    /// The bundle is a fragment.
    pub const FRAGMENT: u64 = 0x0000_0001;
    /// The payload is an administrative record.
    pub const ADMIN_RECORD: u64 = 0x0000_0002;
    /// The bundle must not be fragmented.
    pub const NO_FRAGMENT: u64 = 0x0000_0004;
    /// Acknowledgement by the application is requested.
    pub const APP_ACK_REQUESTED: u64 = 0x0000_0020;
    /// Status time is requested in status reports.
    pub const STATUS_TIME_REQUESTED: u64 = 0x0000_0040;
    /// Report reception of the bundle.
    pub const REPORT_RECEPTION: u64 = 0x0000_4000;
    /// Report forwarding of the bundle.
    pub const REPORT_FORWARDING: u64 = 0x0001_0000;
    /// Report delivery of the bundle.
    pub const REPORT_DELIVERY: u64 = 0x0002_0000;
    /// Report deletion of the bundle.
    pub const REPORT_DELETION: u64 = 0x0004_0000;
    /// All four status-report request flags.
    pub const STATUS_REPORT_REQUESTS: u64 =
        REPORT_RECEPTION | REPORT_FORWARDING | REPORT_DELIVERY | REPORT_DELETION;
}

/// Block processing control flags (§4.2.4), as bit masks.
pub mod block_flags {
    /// The block must be replicated in every fragment.
    pub const REPLICATE_IN_EVERY_FRAGMENT: u64 = 0x01;
    /// Transmit a status report if the block cannot be processed.
    pub const REPORT_IF_UNPROCESSABLE: u64 = 0x02;
    /// Delete the bundle if the block cannot be processed.
    pub const DELETE_BUNDLE_IF_UNPROCESSABLE: u64 = 0x04;
    /// Discard the block if it cannot be processed.
    pub const DISCARD_IF_UNPROCESSABLE: u64 = 0x10;
}

/// CRC type of a block (§4.2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrcType {
    /// No CRC is present.
    None,
    /// CRC-16/X-25 (type 1), two bytes.
    Crc16,
    /// CRC-32C (type 2), four bytes.
    Crc32C,
}

impl CrcType {
    /// The CRC type code.
    pub fn code(self) -> u64 {
        match self {
            CrcType::None => 0,
            CrcType::Crc16 => 1,
            CrcType::Crc32C => 2,
        }
    }

    fn from_code(code: u64) -> Result<Self, Error> {
        match code {
            0 => Ok(CrcType::None),
            1 => Ok(CrcType::Crc16),
            2 => Ok(CrcType::Crc32C),
            _ => Err(Error::UnknownCrcType { code }),
        }
    }

    /// Length of the CRC value in bytes.
    pub fn value_len(self) -> usize {
        match self {
            CrcType::None => 0,
            CrcType::Crc16 => 2,
            CrcType::Crc32C => 4,
        }
    }

    /// The CRC of `bytes`, big-endian, as stored in a block. Empty for [`CrcType::None`].
    fn compute(self, bytes: &[u8]) -> Vec<u8> {
        match self {
            CrcType::None => Vec::new(),
            CrcType::Crc16 => crc16_x25(bytes).to_be_bytes().to_vec(),
            CrcType::Crc32C => crc32c(bytes).to_be_bytes().to_vec(),
        }
    }
}

/// Creation timestamp (§4.2.7): DTN time in milliseconds and a sequence number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreationTimestamp {
    /// Milliseconds since 2000-01-01 00:00:00 UTC (DTN time).
    pub time: u64,
    /// Sequence number, distinguishing bundles created in the same millisecond.
    pub sequence: u64,
}

/// Fragment fields, present only when the bundle is a fragment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fragment {
    /// Offset of this payload within the original application data unit.
    pub offset: u64,
    /// Total length of the original application data unit.
    pub total_len: u64,
}

/// The primary bundle block (§4.3.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimaryBlock {
    /// Bundle processing control flags; see [`bundle_flags`].
    pub flags: u64,
    /// CRC type of this block.
    pub crc: CrcType,
    /// Destination endpoint.
    pub destination: Eid,
    /// Source node. The null endpoint for anonymous bundles.
    pub source: Eid,
    /// Endpoint that receives status reports.
    pub report_to: Eid,
    /// Creation timestamp.
    pub created: CreationTimestamp,
    /// Lifetime in milliseconds after creation.
    pub lifetime: u64,
    /// Fragment fields, if and only if the fragment flag is set.
    pub fragment: Option<Fragment>,
}

/// A canonical (non-primary) block (§4.3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalBlock {
    /// Block type code. 1 is the payload block.
    pub block_type: u64,
    /// Block number. The payload block is always 1; the primary block is implicitly 0.
    pub number: u64,
    /// Block processing control flags; see [`block_flags`].
    pub flags: u64,
    /// CRC type of this block.
    pub crc: CrcType,
    /// Block-type-specific data. For the payload block, the application data unit.
    pub data: Vec<u8>,
}

/// Identity of a bundle: its source endpoint and creation timestamp. Two bundles with the same
/// identity are the same bundle (RFC 9171 §3.1), ignoring fragmentation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BundleId {
    /// The source endpoint.
    pub source: Eid,
    /// The creation timestamp.
    pub created: CreationTimestamp,
}

/// A bundle: a primary block followed by canonical blocks, the payload block last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    /// The primary block.
    pub primary: PrimaryBlock,
    /// Canonical blocks in transmission order. The last one is the payload block.
    pub blocks: Vec<CanonicalBlock>,
}

impl Bundle {
    /// The payload (application data unit), if the bundle has a valid payload block.
    pub fn payload(&self) -> Option<&[u8]> {
        self.blocks
            .iter()
            .find(|b| b.block_type == BLOCK_TYPE_PAYLOAD)
            .map(|b| b.data.as_slice())
    }

    /// The bundle's identity: its source and creation timestamp (RFC 9171 §3.1).
    pub fn id(&self) -> BundleId {
        BundleId {
            source: self.primary.source.clone(),
            created: self.primary.created,
        }
    }

    /// DTN time (milliseconds) at which the bundle's lifetime ends. Saturates at `u64::MAX`.
    pub fn expires_at(&self) -> u64 {
        self.primary
            .created
            .time
            .saturating_add(self.primary.lifetime)
    }

    /// Checks the rules of RFC 9171 that this crate enforces. Encoding and decoding both call it.
    ///
    /// Rules checked: a fragment flag matches the presence of fragment fields; an administrative
    /// record or a null source sets no status-report requests and no block asks for a report
    /// on failure; a null source is not fragmented; the blocks have unique, non-zero numbers; the
    /// payload block is numbered 1, appears exactly once, and is last; a fragment's offset and
    /// payload fit within its total length.
    pub fn validate(&self) -> Result<(), Error> {
        let p = &self.primary;
        if (p.flags & bundle_flags::FRAGMENT != 0) != p.fragment.is_some() {
            return Err(Error::InvalidBundle {
                rule: "fragment flag and fragment fields disagree",
            });
        }
        let anonymous = p.source == Eid::Null;
        let admin = p.flags & bundle_flags::ADMIN_RECORD != 0;
        if (admin || anonymous) && p.flags & bundle_flags::STATUS_REPORT_REQUESTS != 0 {
            return Err(Error::InvalidBundle {
                rule: "status reports requested for an administrative record or anonymous bundle",
            });
        }
        if anonymous && p.flags & bundle_flags::NO_FRAGMENT == 0 {
            return Err(Error::InvalidBundle {
                rule: "anonymous bundle must not be fragmented",
            });
        }
        if self.blocks.is_empty() {
            return Err(Error::InvalidBundle {
                rule: "a bundle needs a payload block",
            });
        }
        let mut numbers: Vec<u64> = Vec::with_capacity(self.blocks.len());
        for b in &self.blocks {
            if b.number == 0 {
                return Err(Error::InvalidBundle {
                    rule: "block number 0 is reserved for the primary block",
                });
            }
            if numbers.contains(&b.number) {
                return Err(Error::InvalidBundle {
                    rule: "duplicate block number",
                });
            }
            numbers.push(b.number);
            if (admin || anonymous) && b.flags & block_flags::REPORT_IF_UNPROCESSABLE != 0 {
                return Err(Error::InvalidBundle {
                    rule: "block requests a report in an administrative or anonymous bundle",
                });
            }
        }
        let payloads = self
            .blocks
            .iter()
            .filter(|b| b.block_type == BLOCK_TYPE_PAYLOAD)
            .count();
        let last = self.blocks.last().map(|b| b.block_type);
        if payloads != 1 || last != Some(BLOCK_TYPE_PAYLOAD) {
            return Err(Error::InvalidBundle {
                rule: "exactly one payload block, and it must be last",
            });
        }
        let payload = self.payload().unwrap_or(&[]);
        let payload_number = self
            .blocks
            .iter()
            .find(|b| b.block_type == BLOCK_TYPE_PAYLOAD)
            .map(|b| b.number);
        if payload_number != Some(1) {
            return Err(Error::InvalidBundle {
                rule: "the payload block is numbered 1",
            });
        }
        if let Some(f) = p.fragment {
            let end = f.offset.checked_add(payload.len() as u64);
            if end.is_none_or(|e| e > f.total_len) {
                return Err(Error::InvalidBundle {
                    rule: "fragment does not fit in the application data unit",
                });
            }
        }
        Ok(())
    }

    /// Encodes the bundle as an indefinite-length array of blocks. Validates first.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut out = Writer::new();
        out.array_indefinite();
        let mut bytes = out.into_bytes();
        bytes.extend_from_slice(&encode_primary(&self.primary).0);
        for b in &self.blocks {
            bytes.extend_from_slice(&encode_canonical(b).0);
        }
        bytes.push(crate::cbor::BREAK);
        Ok(bytes)
    }

    /// Decodes and validates a bundle. The input must be exactly one bundle.
    pub fn decode(buf: &[u8]) -> Result<Self, Error> {
        let mut r = Reader::new(buf);
        match r.read_array_header()? {
            ArrayLen::Indefinite => {}
            ArrayLen::Definite(_) => {
                return Err(Error::InvalidBundle {
                    rule: "the bundle must be an indefinite-length array",
                });
            }
        }
        let primary = decode_primary(&mut r, buf)?;
        let mut blocks = Vec::new();
        while !r.at_break()? {
            blocks.push(decode_canonical(&mut r, buf)?);
        }
        r.read_break()?;
        if r.remaining() != 0 {
            return Err(Error::TrailingBytes);
        }
        let bundle = Bundle { primary, blocks };
        bundle.validate()?;
        Ok(bundle)
    }
}

/// Encodes the primary block. Returns its bytes and the byte range of the CRC value within them.
fn encode_primary(p: &PrimaryBlock) -> (Vec<u8>, Option<(usize, usize)>) {
    let mut w = Writer::new();
    let mut items = 8u64;
    if p.fragment.is_some() {
        items += 2;
    }
    if p.crc != CrcType::None {
        items += 1;
    }
    w.array(items);
    w.uint(VERSION);
    w.uint(p.flags);
    w.uint(p.crc.code());
    p.destination.encode(&mut w);
    p.source.encode(&mut w);
    p.report_to.encode(&mut w);
    w.array(2);
    w.uint(p.created.time);
    w.uint(p.created.sequence);
    w.uint(p.lifetime);
    if let Some(f) = p.fragment {
        w.uint(f.offset);
        w.uint(f.total_len);
    }
    let mut span = None;
    if p.crc != CrcType::None {
        let start = w.len();
        w.bytes(&[0u8; 4][..p.crc.value_len()]);
        // The CRC byte string is a one-byte head followed by the value.
        let value_start = start + 1;
        let value_end = value_start + p.crc.value_len();
        span = Some((value_start, value_end));
    }
    let mut bytes = w.into_bytes();
    if let Some((s, e)) = span {
        let crc = p.crc.compute(&bytes);
        bytes[s..e].copy_from_slice(&crc);
    }
    (bytes, span)
}

/// Encodes a canonical block. Returns its bytes and the byte range of the CRC value within them.
fn encode_canonical(b: &CanonicalBlock) -> (Vec<u8>, Option<(usize, usize)>) {
    let mut w = Writer::new();
    w.array(if b.crc == CrcType::None { 5 } else { 6 });
    w.uint(b.block_type);
    w.uint(b.number);
    w.uint(b.flags);
    w.uint(b.crc.code());
    w.bytes(&b.data);
    let mut span = None;
    if b.crc != CrcType::None {
        let start = w.len();
        w.bytes(&[0u8; 4][..b.crc.value_len()]);
        let value_start = start + 1;
        span = Some((value_start, value_start + b.crc.value_len()));
    }
    let mut bytes = w.into_bytes();
    if let Some((s, e)) = span {
        let crc = b.crc.compute(&bytes);
        bytes[s..e].copy_from_slice(&crc);
    }
    (bytes, span)
}

/// Reads the CRC value of a block whose bytes are `block` (the block's own encoding). Verifies it
/// by recomputing over the block with the value zeroed.
fn verify_crc(block: &[u8], crc: CrcType, value: &[u8], block_number: u64) -> Result<(), Error> {
    if crc == CrcType::None {
        return Ok(());
    }
    if value.len() != crc.value_len() {
        return Err(Error::InvalidBundle {
            rule: "CRC value has the wrong length for its CRC type",
        });
    }
    // Locate the value inside the block: it is the last bytes before the end of the block for a
    // canonical block, and near the end for the primary block; search by identity of position.
    let end = block.len();
    let value_start = end.checked_sub(value.len()).ok_or(Error::Truncated)?;
    let mut zeroed = block.to_vec();
    for b in &mut zeroed[value_start..] {
        *b = 0;
    }
    if crc.compute(&zeroed) != value {
        return Err(Error::CrcMismatch {
            block: block_number,
        });
    }
    Ok(())
}

fn decode_primary(r: &mut Reader<'_>, buf: &[u8]) -> Result<PrimaryBlock, Error> {
    let start = r.position();
    let n = match r.read_array_header()? {
        ArrayLen::Definite(n) => n,
        ArrayLen::Indefinite => {
            return Err(Error::IndefiniteNotAllowed {
                what: "primary block",
            });
        }
    };
    if !(8..=11).contains(&n) {
        return Err(Error::InvalidBundle {
            rule: "primary block must have 8 to 11 items",
        });
    }
    let version = r.read_uint()?;
    if version != VERSION {
        return Err(Error::UnsupportedVersion { version });
    }
    let flags = r.read_uint()?;
    let crc = CrcType::from_code(r.read_uint()?)?;
    let fragmented = flags & bundle_flags::FRAGMENT != 0;
    let expected = 8 + if fragmented { 2 } else { 0 } + if crc == CrcType::None { 0 } else { 1 };
    if n != expected {
        return Err(Error::InvalidBundle {
            rule: "primary block item count does not match its flags and CRC type",
        });
    }
    let destination = Eid::decode(r)?;
    let source = Eid::decode(r)?;
    let report_to = Eid::decode(r)?;
    match r.read_array_header()? {
        ArrayLen::Definite(2) => {}
        _ => {
            return Err(Error::InvalidBundle {
                rule: "creation timestamp must be a two-item array",
            });
        }
    }
    let created = CreationTimestamp {
        time: r.read_uint()?,
        sequence: r.read_uint()?,
    };
    let lifetime = r.read_uint()?;
    let fragment = if fragmented {
        Some(Fragment {
            offset: r.read_uint()?,
            total_len: r.read_uint()?,
        })
    } else {
        None
    };
    let mut crc_value: Option<&[u8]> = None;
    if crc != CrcType::None {
        crc_value = Some(r.read_bytes()?);
    }
    let end = r.position();
    let block = buf.get(start..end).ok_or(Error::Truncated)?;
    verify_crc(block, crc, crc_value.unwrap_or(&[]), 0)?;
    Ok(PrimaryBlock {
        flags,
        crc,
        destination,
        source,
        report_to,
        created,
        lifetime,
        fragment,
    })
}

fn decode_canonical(r: &mut Reader<'_>, buf: &[u8]) -> Result<CanonicalBlock, Error> {
    let start = r.position();
    let n = match r.read_array_header()? {
        ArrayLen::Definite(n) => n,
        ArrayLen::Indefinite => {
            return Err(Error::IndefiniteNotAllowed {
                what: "canonical block",
            });
        }
    };
    if n != 5 && n != 6 {
        return Err(Error::InvalidBundle {
            rule: "canonical block must have 5 or 6 items",
        });
    }
    let block_type = r.read_uint()?;
    let number = r.read_uint()?;
    let flags = r.read_uint()?;
    let crc = CrcType::from_code(r.read_uint()?)?;
    if (n == 6) != (crc != CrcType::None) {
        return Err(Error::InvalidBundle {
            rule: "canonical block item count does not match its CRC type",
        });
    }
    let data = r.read_bytes()?.to_vec();
    let mut crc_value: Option<&[u8]> = None;
    if crc != CrcType::None {
        crc_value = Some(r.read_bytes()?);
    }
    let end = r.position();
    let block = buf.get(start..end).ok_or(Error::Truncated)?;
    verify_crc(block, crc, crc_value.unwrap_or(&[]), number)?;
    Ok(CanonicalBlock {
        block_type,
        number,
        flags,
        crc,
        data,
    })
}
