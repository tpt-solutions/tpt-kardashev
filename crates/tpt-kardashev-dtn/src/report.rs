// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bundle status reports (RFC 9171 §5 and §6.1.1): the administrative record that tells the report-to
//! endpoint of a bundle that the bundle was received, forwarded, delivered or deleted by a node.
//!
//! A status report is the payload of an administrative bundle whose payload is `[1, status-record-content]`
//! (RFC 9171 Appendix B): the bundle-status information (four status items, one per event, in the order
//! received, forwarded, delivered, deleted), a reason code, the subject's source endpoint and creation
//! timestamp, and, for a fragment, the offset and length of its payload.

use crate::cbor::{ArrayLen, Reader, Writer};
use crate::eid::Eid;
use crate::error::Error;
use alloc::vec::Vec;

/// Administrative record type of a bundle status report (RFC 9171 Appendix B: `[1, status-record-content]`).
pub const STATUS_REPORT_RECORD: u64 = 1;

/// Status report reason codes, as RFC 9171 §6.1.1 lists them.
pub mod reason {
    /// No additional information.
    pub const NO_ADDITIONAL_INFORMATION: u8 = 0;
    /// Lifetime expired.
    pub const LIFETIME_EXPIRED: u8 = 1;
    /// Depleted storage.
    pub const DEPLETED_STORAGE: u8 = 4;
    /// No known route to the destination from here.
    pub const NO_KNOWN_ROUTE: u8 = 6;
    /// No timely contact with the next node on the route.
    pub const NO_TIMELY_CONTACT: u8 = 7;
    /// Block unintelligible.
    pub const BLOCK_UNINTELLIGIBLE: u8 = 8;
    /// Hop limit exceeded.
    pub const HOP_LIMIT_EXCEEDED: u8 = 9;
    /// Block unsupported.
    pub const BLOCK_UNSUPPORTED: u8 = 11;
}

/// The event a status report asserts about its subject bundle, at the reporting node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The bundle was received (RFC 9171 §5.6).
    Received,
    /// The bundle was forwarded (RFC 9171 §5.4).
    Forwarded,
    /// The bundle was delivered to its destination application (RFC 9171 §5.7).
    Delivered,
    /// The bundle was deleted (RFC 9171 §5.10).
    Deleted,
}

impl Event {
    /// The position of the event's status item in the bundle-status information.
    fn index(self) -> usize {
        match self {
            Event::Received => 0,
            Event::Forwarded => 1,
            Event::Delivered => 2,
            Event::Deleted => 3,
        }
    }

    /// The event at position `index` of the bundle-status information, if any.
    fn from_index(index: usize) -> Option<Event> {
        [
            Event::Received,
            Event::Forwarded,
            Event::Delivered,
            Event::Deleted,
        ]
        .get(index)
        .copied()
    }
}

/// A bundle status report about one subject bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusReport {
    /// The event asserted. The other three status items are not asserted.
    pub event: Event,
    /// The reason code (see [`reason`]).
    pub reason: u8,
    /// The time of the event, when the subject asked for status times.
    pub time: Option<u64>,
    /// The subject's source endpoint.
    pub source: Eid,
    /// The subject's creation timestamp time.
    pub creation_time: u64,
    /// The subject's creation timestamp sequence number.
    pub creation_sequence: u64,
    /// For a fragment: the offset and length of its payload within the original application data unit.
    pub fragment: Option<(u64, u64)>,
}

impl StatusReport {
    /// Encodes the report as the payload of an administrative record: `[1, [status information, reason,
    /// source, [time, sequence], (offset, length)?]]`, where each status item is `[false]` or `[true]` or,
    /// with a status time, `[true, time]`.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.array(2);
        w.uint(STATUS_REPORT_RECORD);
        w.array(if self.fragment.is_some() { 5 } else { 3 });
        // The bundle-status information: four status items.
        w.array(4);
        for index in 0..4 {
            let asserted = index == self.event.index();
            if asserted {
                match self.time {
                    Some(time) => {
                        w.array(2);
                        w.raw(&[0xF5]);
                        w.uint(time);
                    }
                    None => {
                        w.array(1);
                        w.raw(&[0xF5]);
                    }
                }
            } else {
                w.array(1);
                w.raw(&[0xF4]);
            }
        }
        w.uint(u64::from(self.reason));
        self.source.encode(&mut w);
        w.array(2);
        w.uint(self.creation_time);
        w.uint(self.creation_sequence);
        if let Some((offset, length)) = self.fragment {
            w.uint(offset);
            w.uint(length);
        }
        w.into_bytes()
    }

    /// Decodes the payload of a status report. Fails with [`Error::InvalidBundle`] for anything else.
    pub fn decode(data: &[u8]) -> Result<Self, Error> {
        let mut r = Reader::new(data);
        expect_array(&mut r, 2, "an administrative record has two items")?;
        if r.read_uint()? != STATUS_REPORT_RECORD {
            return Err(Error::InvalidBundle {
                rule: "the administrative record is not a status report",
            });
        }
        let content_len = match r.read_array_header()? {
            ArrayLen::Definite(n @ (3 | 5)) => n,
            _ => {
                return Err(Error::InvalidBundle {
                    rule: "a status report has three or five content items",
                });
            }
        };
        expect_array(&mut r, 4, "bundle-status information has four items")?;
        let mut event = None;
        let mut time = None;
        for index in 0..4 {
            let item_len = match r.read_array_header()? {
                ArrayLen::Definite(n @ (1 | 2)) => n,
                _ => {
                    return Err(Error::InvalidBundle {
                        rule: "a status item has one or two items",
                    });
                }
            };
            if r.read_bool()? {
                if event.is_some() {
                    return Err(Error::InvalidBundle {
                        rule: "a status report asserts one event",
                    });
                }
                event = Event::from_index(index);
                if item_len == 2 {
                    time = Some(r.read_uint()?);
                }
            } else if item_len != 1 {
                return Err(Error::InvalidBundle {
                    rule: "an unasserted status item has no time",
                });
            }
        }
        let event = event.ok_or(Error::InvalidBundle {
            rule: "a status report asserts an event",
        })?;
        let reason = r.read_uint()?;
        let reason = u8::try_from(reason).map_err(|_| Error::InvalidBundle {
            rule: "a reason code is one byte",
        })?;
        let source = Eid::decode(&mut r)?;
        expect_array(&mut r, 2, "a creation timestamp has two items")?;
        let creation_time = r.read_uint()?;
        let creation_sequence = r.read_uint()?;
        let fragment = if content_len == 5 {
            Some((r.read_uint()?, r.read_uint()?))
        } else {
            None
        };
        if r.remaining() != 0 {
            return Err(Error::InvalidBundle {
                rule: "a status report has bytes after its items",
            });
        }
        Ok(StatusReport {
            event,
            reason,
            time,
            source,
            creation_time,
            creation_sequence,
            fragment,
        })
    }
}

fn expect_array(r: &mut Reader<'_>, n: u64, rule: &'static str) -> Result<(), Error> {
    match r.read_array_header()? {
        ArrayLen::Definite(len) if len == n => Ok(()),
        _ => Err(Error::InvalidBundle { rule }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn report(event: Event, time: Option<u64>, fragment: Option<(u64, u64)>) -> StatusReport {
        StatusReport {
            event,
            reason: reason::NO_ADDITIONAL_INFORMATION,
            time,
            source: Eid::Ipn {
                node: 1,
                service: 1,
            },
            creation_time: 1000,
            creation_sequence: 7,
            fragment,
        }
    }

    #[test]
    fn every_event_round_trips_with_and_without_time_and_fragment() {
        for event in [
            Event::Received,
            Event::Forwarded,
            Event::Delivered,
            Event::Deleted,
        ] {
            for time in [None, Some(4_321)] {
                for fragment in [None, Some((100, 50))] {
                    let r = report(event, time, fragment);
                    assert_eq!(StatusReport::decode(&r.encode()).expect("decodes"), r);
                }
            }
        }
    }

    #[test]
    fn the_layout_is_the_rfc_9171_record() {
        // [1, [[[true], [false], [false], [false]], 0, ipn:1.1, [1000, 7]]]
        let bytes = report(Event::Received, None, None).encode();
        assert_eq!(
            bytes,
            [
                0x82, 0x01, 0x83, 0x84, 0x81, 0xF5, 0x81, 0xF4, 0x81, 0xF4, 0x81, 0xF4, 0x00, 0x82,
                0x02, 0x82, 0x01, 0x01, 0x82, 0x19, 0x03, 0xE8, 0x07,
            ]
        );
    }

    #[test]
    fn two_asserted_events_or_a_bad_record_are_rejected() {
        let mut two = report(Event::Received, None, None).encode();
        // Turn the second status item into an asserted one.
        let index = two.iter().position(|&b| b == 0x81).expect("status item");
        let second = two[index + 2..]
            .iter()
            .position(|&b| b == 0x81)
            .expect("second item")
            + index
            + 2;
        two[second + 1] = 0xF5;
        assert!(StatusReport::decode(&two).is_err());
        assert!(StatusReport::decode(&[0x80]).is_err());
    }
}
