// SPDX-License-Identifier: MIT OR Apache-2.0
//! LTP segments (RFC 5326 §3): header, content and trailer, with encoding and strict decoding.
//!
//! Layout of one segment, which is one transport datagram:
//!
//! ```text
//! control byte: version (4 bits, 0) | segment type code (4 bits)
//! session ID:   originator (SDNV), session number (SDNV)
//! extensions:   header TLV count (4 bits) | trailer TLV count (4 bits)
//! header TLVs   each: tag (1 octet), length (SDNV), value
//! content       depends on the type code
//! trailer TLVs  as above
//! ```
//!
//! Type codes (§3.1.2): 0 to 3 are red-part data (0 plain, 1 checkpoint, 2 checkpoint with end of
//! red-part, 3 checkpoint with end of red-part and end of block). 4 and 7 are green-part data (4
//! plain, 7 end of block). 8 is a report segment, 9 a report acknowledgment, 12 and 13 cancel from
//! the block sender and its acknowledgment, 14 and 15 cancel from the block receiver and its
//! acknowledgment. Codes 5, 6, 10 and 11 are undefined and are rejected.

use crate::error::Error;
use crate::sdnv;
use alloc::vec::Vec;

/// Identifies a session: the originator's engine ID and the session number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId {
    /// Engine ID of the session's sender.
    pub originator: u64,
    /// Session number chosen by the sender.
    pub number: u64,
}

/// One extension TLV in a header or trailer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    /// Extension tag. 0x00 and 0x01 are registered; 0xC0 to 0xFF are experimental.
    pub tag: u8,
    /// Extension value, carried uninterpreted.
    pub value: Vec<u8>,
}

/// Colour of a data segment: red (reliably delivered) or green (best effort).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// Red-part data: retransmitted until acknowledged.
    Red,
    /// Green-part data: sent once, not retransmitted.
    Green,
}

/// Checkpoint serial numbers carried by a checkpoint data segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    /// This checkpoint's serial number. Never zero.
    pub serial: u64,
    /// Serial number of the report segment that caused this checkpoint, or zero.
    pub report_serial: u64,
}

/// A data segment: a piece of a block, with its position in the block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataSegment {
    /// Red or green.
    pub color: Color,
    /// Present if and only if the segment is a checkpoint (red data only).
    pub checkpoint: Option<Checkpoint>,
    /// End of red-part. Red data only; implies a checkpoint.
    pub eorp: bool,
    /// End of block. Implies end of red-part for red data.
    pub eob: bool,
    /// Client service that receives the block.
    pub client_service: u64,
    /// Offset of `data` within the block.
    pub offset: u64,
    /// The block bytes carried by this segment.
    pub data: Vec<u8>,
}

/// One reception claim: `length` contiguous bytes received from `offset` past the report's lower
/// bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claim {
    /// Offset from the report's lower bound.
    pub offset: u64,
    /// Number of contiguous bytes received.
    pub length: u64,
}

/// A report segment: what the receiver has of the block, within a scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportSegment {
    /// Serial number of this report. Never zero.
    pub report_serial: u64,
    /// Checkpoint that caused this report, or zero for an asynchronous report.
    pub checkpoint_serial: u64,
    /// Upper bound of the scope: a block prefix length.
    pub upper: u64,
    /// Lower bound of the scope. Must be below `upper`.
    pub lower: u64,
    /// Reception claims, in increasing order, within the scope.
    pub claims: Vec<Claim>,
}

/// The party a cancellation concerns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Party {
    /// The block sender.
    Sender,
    /// The block receiver.
    Receiver,
}

/// The content of a segment, by type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// Red or green data.
    Data(DataSegment),
    /// Report of reception claims.
    Report(ReportSegment),
    /// Acknowledgment of the report with this serial number.
    ReportAck {
        /// Serial number of the acknowledged report.
        report_serial: u64,
    },
    /// Cancellation of the session, from `from`, with a reason code (RFC 5326 §3.2.4).
    Cancel {
        /// Which party cancels.
        from: Party,
        /// Reason code: 0 USR_CNCLD, 1 UNREACH, 2 RLEXC, 3 MISCOLORED, 4 SYS_CNCLD, 5 RXMTCYCEXC.
        reason: u8,
    },
    /// Acknowledgment of a cancellation. `to` is the party whose cancellation is acknowledged.
    CancelAck {
        /// The party whose cancellation this acknowledges.
        to: Party,
    },
}

/// One LTP segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// The session this segment belongs to.
    pub session: SessionId,
    /// Header extension TLVs (at most 15).
    pub header_extensions: Vec<Extension>,
    /// Trailer extension TLVs (at most 15).
    pub trailer_extensions: Vec<Extension>,
    /// The content.
    pub body: Body,
}

impl Segment {
    /// A segment with no extensions.
    pub fn new(session: SessionId, body: Body) -> Self {
        Self {
            session,
            header_extensions: Vec::new(),
            trailer_extensions: Vec::new(),
            body,
        }
    }

    /// Encodes the segment. Validates the content first.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.header_extensions.len() > 15 || self.trailer_extensions.len() > 15 {
            return Err(Error::Ltp {
                what: "at most 15 extensions in each of the header and trailer",
            });
        }
        let code = type_code(&self.body)?;
        let mut out = Vec::with_capacity(16);
        out.push(code); // version 0 in the high nibble
        sdnv::encode(self.session.originator, &mut out);
        sdnv::encode(self.session.number, &mut out);
        out.push(((self.header_extensions.len() as u8) << 4) | self.trailer_extensions.len() as u8);
        for ext in &self.header_extensions {
            encode_extension(ext, &mut out);
        }
        encode_body(&self.body, &mut out)?;
        for ext in &self.trailer_extensions {
            encode_extension(ext, &mut out);
        }
        Ok(out)
    }

    /// Decodes one segment. The input must hold exactly one segment.
    pub fn decode(buf: &[u8]) -> Result<Self, Error> {
        let mut cur = Cursor { buf, pos: 0 };
        let control = cur.byte()?;
        if control >> 4 != 0 {
            return Err(Error::Ltp {
                what: "unsupported LTP version",
            });
        }
        let code = control & 0x0F;
        let session = SessionId {
            originator: cur.sdnv()?,
            number: cur.sdnv()?,
        };
        let counts = cur.byte()?;
        let header_count = usize::from(counts >> 4);
        let trailer_count = usize::from(counts & 0x0F);
        let mut header_extensions = Vec::with_capacity(header_count);
        for _ in 0..header_count {
            header_extensions.push(cur.extension()?);
        }
        let body = decode_body(code, &mut cur)?;
        let mut trailer_extensions = Vec::with_capacity(trailer_count);
        for _ in 0..trailer_count {
            trailer_extensions.push(cur.extension()?);
        }
        if cur.pos != buf.len() {
            return Err(Error::Ltp {
                what: "bytes after the segment",
            });
        }
        Ok(Segment {
            session,
            header_extensions,
            trailer_extensions,
            body,
        })
    }
}

fn type_code(body: &Body) -> Result<u8, Error> {
    match body {
        Body::Data(d) => match d.color {
            Color::Red => {
                validate_red_flags(d)?;
                Ok(if d.eob {
                    3
                } else if d.eorp {
                    2
                } else if d.checkpoint.is_some() {
                    1
                } else {
                    0
                })
            }
            Color::Green => {
                if d.checkpoint.is_some() || d.eorp {
                    return Err(Error::Ltp {
                        what: "green data has no checkpoint or end of red-part",
                    });
                }
                Ok(if d.eob { 7 } else { 4 })
            }
        },
        Body::Report(_) => Ok(8),
        Body::ReportAck { .. } => Ok(9),
        Body::Cancel {
            from: Party::Sender,
            ..
        } => Ok(12),
        Body::Cancel {
            from: Party::Receiver,
            ..
        } => Ok(14),
        Body::CancelAck { to: Party::Sender } => Ok(13),
        Body::CancelAck {
            to: Party::Receiver,
        } => Ok(15),
    }
}

fn validate_red_flags(d: &DataSegment) -> Result<(), Error> {
    if d.eorp && d.checkpoint.is_none() {
        return Err(Error::Ltp {
            what: "end of red-part must be a checkpoint",
        });
    }
    if d.eob && !d.eorp {
        return Err(Error::Ltp {
            what: "end of block must also end the red-part",
        });
    }
    if d.checkpoint.is_some_and(|cp| cp.serial == 0) {
        return Err(Error::Ltp {
            what: "checkpoint serial number must not be zero",
        });
    }
    Ok(())
}

fn encode_extension(ext: &Extension, out: &mut Vec<u8>) {
    out.push(ext.tag);
    sdnv::encode(ext.value.len() as u64, out);
    out.extend_from_slice(&ext.value);
}

fn encode_body(body: &Body, out: &mut Vec<u8>) -> Result<(), Error> {
    match body {
        Body::Data(d) => {
            sdnv::encode(d.client_service, out);
            sdnv::encode(d.offset, out);
            sdnv::encode(d.data.len() as u64, out);
            if let Some(cp) = d.checkpoint {
                sdnv::encode(cp.serial, out);
                sdnv::encode(cp.report_serial, out);
            }
            out.extend_from_slice(&d.data);
        }
        Body::Report(r) => {
            validate_report(r)?;
            sdnv::encode(r.report_serial, out);
            sdnv::encode(r.checkpoint_serial, out);
            sdnv::encode(r.upper, out);
            sdnv::encode(r.lower, out);
            sdnv::encode(r.claims.len() as u64, out);
            for c in &r.claims {
                sdnv::encode(c.offset, out);
                sdnv::encode(c.length, out);
            }
        }
        Body::ReportAck { report_serial } => sdnv::encode(*report_serial, out),
        Body::Cancel { reason, .. } => out.push(*reason),
        Body::CancelAck { .. } => {}
    }
    Ok(())
}

/// Checks the report rules of RFC 5326 §3.2.2: a non-zero serial, `lower < upper`, and claims that
/// are non-empty, increasing, and within the scope.
fn validate_report(r: &ReportSegment) -> Result<(), Error> {
    if r.report_serial == 0 {
        return Err(Error::Ltp {
            what: "report serial number must not be zero",
        });
    }
    if r.lower >= r.upper {
        return Err(Error::Ltp {
            what: "report scope must have lower < upper",
        });
    }
    let scope = r.upper - r.lower;
    let mut next_free: u64 = 0;
    for c in &r.claims {
        if c.length == 0 {
            return Err(Error::Ltp {
                what: "reception claim length must be at least 1",
            });
        }
        if c.offset < next_free {
            return Err(Error::Ltp {
                what: "reception claims must be increasing and disjoint",
            });
        }
        let end = c.offset.checked_add(c.length).ok_or(Error::Ltp {
            what: "reception claim overflows",
        })?;
        if end > scope {
            return Err(Error::Ltp {
                what: "reception claim lies outside the report scope",
            });
        }
        next_free = end;
    }
    Ok(())
}

fn decode_body(code: u8, cur: &mut Cursor<'_>) -> Result<Body, Error> {
    match code {
        0..=3 => {
            let eob = code == 3;
            let eorp = code == 2 || code == 3;
            let checkpoint_present = code != 0;
            let client_service = cur.sdnv()?;
            let offset = cur.sdnv()?;
            let length = cur.sdnv()?;
            let checkpoint = if checkpoint_present {
                Some(Checkpoint {
                    serial: cur.sdnv()?,
                    report_serial: cur.sdnv()?,
                })
            } else {
                None
            };
            let d = DataSegment {
                color: Color::Red,
                checkpoint,
                eorp,
                eob,
                client_service,
                offset,
                data: cur.bytes(length)?.to_vec(),
            };
            validate_red_flags(&d)?;
            Ok(Body::Data(d))
        }
        4 | 7 => {
            let client_service = cur.sdnv()?;
            let offset = cur.sdnv()?;
            let length = cur.sdnv()?;
            Ok(Body::Data(DataSegment {
                color: Color::Green,
                checkpoint: None,
                eorp: false,
                eob: code == 7,
                client_service,
                offset,
                data: cur.bytes(length)?.to_vec(),
            }))
        }
        8 => {
            let report_serial = cur.sdnv()?;
            let checkpoint_serial = cur.sdnv()?;
            let upper = cur.sdnv()?;
            let lower = cur.sdnv()?;
            let count = cur.sdnv()?;
            // Each claim takes at least two octets, so a count larger than the remaining input
            // cannot be satisfied; reject it before allocating.
            if count > cur.remaining() as u64 {
                return Err(Error::Truncated);
            }
            let mut claims = Vec::with_capacity(count as usize);
            for _ in 0..count {
                claims.push(Claim {
                    offset: cur.sdnv()?,
                    length: cur.sdnv()?,
                });
            }
            let r = ReportSegment {
                report_serial,
                checkpoint_serial,
                upper,
                lower,
                claims,
            };
            validate_report(&r)?;
            Ok(Body::Report(r))
        }
        9 => Ok(Body::ReportAck {
            report_serial: cur.sdnv()?,
        }),
        12 => Ok(Body::Cancel {
            from: Party::Sender,
            reason: cur.byte()?,
        }),
        14 => Ok(Body::Cancel {
            from: Party::Receiver,
            reason: cur.byte()?,
        }),
        13 => Ok(Body::CancelAck { to: Party::Sender }),
        15 => Ok(Body::CancelAck {
            to: Party::Receiver,
        }),
        _ => Err(Error::Ltp {
            what: "undefined segment type code",
        }),
    }
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    fn byte(&mut self) -> Result<u8, Error> {
        let b = *self.buf.get(self.pos).ok_or(Error::Truncated)?;
        self.pos += 1;
        Ok(b)
    }

    fn sdnv(&mut self) -> Result<u64, Error> {
        let rest = self.buf.get(self.pos..).ok_or(Error::Truncated)?;
        let (v, used) = sdnv::decode(rest)?;
        self.pos += used;
        Ok(v)
    }

    fn bytes(&mut self, n: u64) -> Result<&'a [u8], Error> {
        let n = usize::try_from(n).map_err(|_| Error::Truncated)?;
        let end = self.pos.checked_add(n).ok_or(Error::Truncated)?;
        let out = self.buf.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(out)
    }

    fn extension(&mut self) -> Result<Extension, Error> {
        let tag = self.byte()?;
        let len = self.sdnv()?;
        Ok(Extension {
            tag,
            value: self.bytes(len)?.to_vec(),
        })
    }
}
