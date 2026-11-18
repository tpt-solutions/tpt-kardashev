// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bundle-level custody transfer: the custody request block and the custody signal, with their codec.
//!
//! RFC 9171 (BPv7) defines no custody transfer. The procedure here follows RFC 5050 (BPv6) §5.10 to §5.12
//! and the custody signal of §6.1.2, which CCSDS 734.2-B-1 §6.2.1.3 names for custody. The wire format is
//! BPv7's: the request is a canonical block and the signal is an administrative record. Both use type codes
//! from the private-use range (block types and administrative record types 192 to 255 in RFC 9171 §9), so
//! the codes are local to this crate and are not IANA-registered. A BPv6 node cannot read them.
//!
//! Custody requested by a bundle is a block of type [`CUSTODY_REQUEST_BLOCK`] whose data names the current
//! custodian; each custodian that accepts custody rewrites it to itself, as RFC 5050 §5.10.1 changes the
//! current custodian. A custody signal is an administrative record of type [`CUSTODY_SIGNAL_RECORD`]. It
//! carries the RFC 5050 status byte (one bit for "custody transfer succeeded" and a seven-bit reason code),
//! the time of the signal, and the identity of the bundle it applies to.

use crate::cbor::{ArrayLen, Reader, Writer};
use crate::error::Error;
use alloc::vec::Vec;

/// Block type of the custody request: a private-use code (RFC 9171 §9.1 reserves 192 to 255).
pub const CUSTODY_REQUEST_BLOCK: u64 = 192;

/// Administrative record type of the custody signal: a private-use code (RFC 9171 §9.4).
pub const CUSTODY_SIGNAL_RECORD: u64 = 192;

/// Custody signal reason codes, as RFC 5050 §6.1.2 (Figure 14) defines them.
pub mod reason {
    /// No additional information.
    pub const NO_ADDITIONAL_INFORMATION: u8 = 0x00;
    /// Redundant reception: the node receiving the bundle is already a custodial node for it.
    pub const REDUNDANT_RECEPTION: u8 = 0x03;
    /// Depleted storage: the node cannot take custody for lack of room.
    pub const DEPLETED_STORAGE: u8 = 0x04;
    /// The destination endpoint ID is unintelligible.
    pub const DESTINATION_UNINTELLIGIBLE: u8 = 0x05;
    /// No known route to the destination from here.
    pub const NO_KNOWN_ROUTE: u8 = 0x06;
    /// No timely contact with the next node on the route.
    pub const NO_TIMELY_CONTACT: u8 = 0x07;
    /// A block is unintelligible.
    pub const BLOCK_UNINTELLIGIBLE: u8 = 0x08;
}

/// Counts of custody events at a network's nodes, for inspection after a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CustodyStats {
    /// Custody taken by a node (a "succeeded" signal was sent).
    pub accepted: u64,
    /// Custody refused by a node for lack of storage (a "depleted storage" signal was sent).
    pub refused: u64,
    /// Custody copies released on a signal from the next custodian, or on a redundancy signal.
    pub released: u64,
    /// Custody copies sent again after their transfer timed out.
    pub retransmitted: u64,
    /// Custody copies given up after the attempt limit.
    pub failed: u64,
    /// Bundles received again by a node that already held their custody (a "redundant reception" signal).
    pub redundant: u64,
    /// Bundles routed again after a custodian refused them, around the refusing node.
    pub rerouted: u64,
}

/// What identifies a bundle, or one fragment of it, for custody: its source node, creation timestamp, and
/// fragment offset. A copy of a bundle that is handed on keeps this identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustodyKey {
    /// Source node number (an `ipn` endpoint).
    pub source: u64,
    /// Creation timestamp time.
    pub time: u64,
    /// Creation timestamp sequence number.
    pub sequence: u64,
    /// Offset of the fragment's application data, for a fragment.
    pub fragment_offset: Option<u64>,
}

/// A custody signal: the outcome of a custody transfer for one bundle, sent to the custodian that offered it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustodySignal {
    /// Whether custody was taken ("custody transfer succeeded").
    pub succeeded: bool,
    /// The reason code, one of [`reason`].
    pub reason: u8,
    /// The bundle the signal is about.
    pub subject: CustodyKey,
    /// The time at which the signal was generated, in DTN milliseconds.
    pub signal_time: u64,
}

impl CustodySignal {
    /// Encodes the signal as the payload of an administrative record, `[record type, content]` (RFC 9171 §4.2
    /// and Appendix B), where the content is `[status, signal time, source, creation time, creation sequence,
    /// fragment flag, fragment offset]`. The status byte is the RFC 5050 one: bit 7 is "succeeded" and the low
    /// seven bits are the reason.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.array(2);
        w.uint(CUSTODY_SIGNAL_RECORD);
        w.array(7);
        let status = (u8::from(self.succeeded) << 7) | (self.reason & 0x7f);
        w.uint(u64::from(status));
        w.uint(self.signal_time);
        w.uint(self.subject.source);
        w.uint(self.subject.time);
        w.uint(self.subject.sequence);
        w.uint(u64::from(self.subject.fragment_offset.is_some()));
        w.uint(self.subject.fragment_offset.unwrap_or(0));
        w.into_bytes()
    }

    /// Decodes a custody signal from the payload of an administrative record. Fails with
    /// [`Error::InvalidBundle`] when the record is not a custody signal or is malformed.
    pub fn decode(data: &[u8]) -> Result<Self, Error> {
        let mut r = Reader::new(data);
        match r.read_array_header()? {
            ArrayLen::Definite(2) => {}
            _ => {
                return Err(Error::InvalidBundle {
                    rule: "an administrative record has two items",
                });
            }
        }
        if r.read_uint()? != CUSTODY_SIGNAL_RECORD {
            return Err(Error::InvalidBundle {
                rule: "the administrative record is not a custody signal",
            });
        }
        match r.read_array_header()? {
            ArrayLen::Definite(7) => {}
            _ => {
                return Err(Error::InvalidBundle {
                    rule: "a custody signal has seven content items",
                });
            }
        }
        let status = r.read_uint()?;
        if status > 0xff {
            return Err(Error::InvalidBundle {
                rule: "custody signal status is one byte",
            });
        }
        let signal_time = r.read_uint()?;
        let source = r.read_uint()?;
        let time = r.read_uint()?;
        let sequence = r.read_uint()?;
        let has_fragment = r.read_uint()?;
        let offset = r.read_uint()?;
        if r.remaining() != 0 {
            return Err(Error::InvalidBundle {
                rule: "custody signal has bytes after its items",
            });
        }
        let fragment_offset = match has_fragment {
            0 => {
                // Canonical form: an offset is written only with the fragment flag, and as zero without it.
                if offset != 0 {
                    return Err(Error::InvalidBundle {
                        rule: "custody signal has a fragment offset without the fragment flag",
                    });
                }
                None
            }
            1 => Some(offset),
            _ => {
                return Err(Error::InvalidBundle {
                    rule: "custody signal fragment flag must be 0 or 1",
                });
            }
        };
        Ok(CustodySignal {
            succeeded: status & 0x80 != 0,
            reason: (status & 0x7f) as u8,
            subject: CustodyKey {
                source,
                time,
                sequence,
                fragment_offset,
            },
            signal_time,
        })
    }
}

/// The data of a custody request block: the current custodian's node number.
pub fn request_data(custodian: u64) -> Vec<u8> {
    let mut w = Writer::new();
    w.array(1);
    w.uint(custodian);
    w.into_bytes()
}

/// The current custodian named by a custody request block's data.
pub fn parse_request(data: &[u8]) -> Result<u64, Error> {
    let mut r = Reader::new(data);
    match r.read_array_header()? {
        ArrayLen::Definite(1) => {}
        _ => {
            return Err(Error::InvalidBundle {
                rule: "a custody request names one custodian",
            });
        }
    }
    let custodian = r.read_uint()?;
    if r.remaining() != 0 {
        return Err(Error::InvalidBundle {
            rule: "custody request has bytes after its item",
        });
    }
    Ok(custodian)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn a_signal_round_trips_with_and_without_a_fragment() {
        for subject in [
            CustodyKey {
                source: 1,
                time: 1000,
                sequence: 7,
                fragment_offset: None,
            },
            CustodyKey {
                source: 9,
                time: 2,
                sequence: 3,
                fragment_offset: Some(500),
            },
        ] {
            for (succeeded, reason) in [
                (true, reason::NO_ADDITIONAL_INFORMATION),
                (false, reason::DEPLETED_STORAGE),
                (false, reason::REDUNDANT_RECEPTION),
            ] {
                let signal = CustodySignal {
                    succeeded,
                    reason,
                    subject,
                    signal_time: 4_321,
                };
                let back = CustodySignal::decode(&signal.encode()).expect("decodes");
                assert_eq!(back, signal);
            }
        }
    }

    #[test]
    fn the_status_byte_follows_rfc_5050() {
        let accepted = CustodySignal {
            succeeded: true,
            reason: 0,
            subject: CustodyKey {
                source: 1,
                time: 1,
                sequence: 1,
                fragment_offset: None,
            },
            signal_time: 0,
        };
        // Array(2), type 192, Array(7), then the status byte 0x80: the succeeded bit and reason zero.
        let bytes = accepted.encode();
        let mut r = Reader::new(&bytes);
        assert_eq!(
            r.read_array_header().expect("record"),
            ArrayLen::Definite(2)
        );
        assert_eq!(r.read_uint().expect("type"), CUSTODY_SIGNAL_RECORD);
        assert_eq!(
            r.read_array_header().expect("content"),
            ArrayLen::Definite(7)
        );
        assert_eq!(r.read_uint().expect("status"), 0x80);
        let refused = CustodySignal {
            succeeded: false,
            reason: reason::DEPLETED_STORAGE,
            ..accepted
        };
        let bytes = refused.encode();
        let mut r = Reader::new(&bytes);
        let _ = r.read_array_header().expect("record");
        let _ = r.read_uint().expect("type");
        let _ = r.read_array_header().expect("content");
        assert_eq!(r.read_uint().expect("status"), 0x04);
    }

    #[test]
    fn a_request_names_its_custodian() {
        assert_eq!(parse_request(&request_data(42)).expect("parses"), 42);
    }

    #[test]
    fn malformed_signals_and_requests_are_rejected() {
        assert!(CustodySignal::decode(&[]).is_err());
        assert!(CustodySignal::decode(&[0x80]).is_err());
        assert!(parse_request(&[0x80]).is_err());
        let mut w = Writer::new();
        w.array(8);
        for _ in 0..8 {
            w.uint(0);
        }
        // Type 0 is not a custody signal.
        assert!(CustodySignal::decode(&w.into_bytes()).is_err());

        // An offset with no fragment flag is not the canonical form the encoder writes.
        let mut w = Writer::new();
        w.array(2);
        w.uint(CUSTODY_SIGNAL_RECORD);
        w.array(7);
        w.uint(0x80);
        for v in [4, 1, 2, 3] {
            w.uint(v);
        }
        w.uint(0);
        w.uint(5);
        assert!(CustodySignal::decode(&w.into_bytes()).is_err());
    }
}
