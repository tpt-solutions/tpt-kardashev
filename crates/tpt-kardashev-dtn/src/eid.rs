// SPDX-License-Identifier: MIT OR Apache-2.0
//! Endpoint identifiers (RFC 9171 §4.2.5.1): the `dtn` and `ipn` URI schemes, with their CBOR form.
//!
//! CBOR form: a two-item array of a scheme code and a scheme-specific part. `dtn` is code 1; its
//! part is a text string, or the integer 0 for `dtn:none`. `ipn` is code 2; its part is a
//! two-item array of node and service numbers.

use crate::cbor::{ArrayLen, Reader, Writer};
use crate::error::Error;
use alloc::string::{String, ToString};
use core::fmt;

/// URI scheme code for `dtn` in the BP registry.
pub const SCHEME_DTN: u64 = 1;
/// URI scheme code for `ipn` in the BP registry.
pub const SCHEME_IPN: u64 = 2;

/// A bundle endpoint identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Eid {
    /// `dtn:none`, the null endpoint. It never has members.
    Null,
    /// `dtn://node/demux`. A demux that starts with `~` names a non-singleton endpoint.
    Dtn {
        /// The node name, without the `//` and the delimiting `/`.
        node: String,
        /// The demux, which may be empty (the node's administrative endpoint).
        demux: String,
    },
    /// `ipn:node.service`. Always a singleton endpoint.
    Ipn {
        /// Node number.
        node: u64,
        /// Service number. Zero is the node's administrative endpoint.
        service: u64,
    },
}

impl Eid {
    /// Parses a URI such as `dtn:none`, `dtn://node/demux` or `ipn:1.2`.
    pub fn parse(uri: &str) -> Result<Self, Error> {
        if uri == "dtn:none" {
            return Ok(Eid::Null);
        }
        if let Some(ssp) = uri.strip_prefix("dtn:") {
            return parse_dtn_ssp(ssp);
        }
        if let Some(ssp) = uri.strip_prefix("ipn:") {
            let (node, service) = ssp.split_once('.').ok_or(Error::InvalidEid {
                reason: "ipn needs node.service",
            })?;
            return Ok(Eid::Ipn {
                node: parse_number(node)?,
                service: parse_number(service)?,
            });
        }
        Err(Error::InvalidEid {
            reason: "unsupported URI scheme",
        })
    }

    /// Writes the CBOR form of this endpoint ID.
    pub fn encode(&self, w: &mut Writer) {
        w.array(2);
        match self {
            Eid::Null => {
                w.uint(SCHEME_DTN);
                w.uint(0);
            }
            Eid::Dtn { node, demux } => {
                w.uint(SCHEME_DTN);
                let mut ssp = String::with_capacity(node.len() + demux.len() + 3);
                ssp.push_str("//");
                ssp.push_str(node);
                ssp.push('/');
                ssp.push_str(demux);
                w.text(&ssp);
            }
            Eid::Ipn { node, service } => {
                w.uint(SCHEME_IPN);
                w.array(2);
                w.uint(*node);
                w.uint(*service);
            }
        }
    }

    /// Reads the CBOR form of an endpoint ID.
    pub fn decode(r: &mut Reader<'_>) -> Result<Self, Error> {
        match r.read_array_header()? {
            ArrayLen::Definite(2) => {}
            ArrayLen::Definite(_) => {
                return Err(Error::InvalidEid {
                    reason: "endpoint ID must be a two-item array",
                });
            }
            ArrayLen::Indefinite => {
                return Err(Error::IndefiniteNotAllowed {
                    what: "endpoint ID",
                });
            }
        }
        match r.read_uint()? {
            SCHEME_DTN => {
                // The null endpoint's part is the integer 0; every other dtn part is text.
                if r.peek()? >> 5 == 0 {
                    if r.read_uint()? != 0 {
                        return Err(Error::InvalidEid {
                            reason: "dtn part must be text or 0",
                        });
                    }
                    Ok(Eid::Null)
                } else {
                    let text = r.read_text()?;
                    if text == "none" {
                        // The null endpoint is the integer 0 in CBOR; the text form is not allowed.
                        return Err(Error::InvalidEid {
                            reason: "dtn:none must be encoded as the integer 0",
                        });
                    }
                    parse_dtn_ssp(text)
                }
            }
            SCHEME_IPN => match r.read_array_header()? {
                ArrayLen::Definite(2) => {
                    let node = r.read_uint()?;
                    let service = r.read_uint()?;
                    Ok(Eid::Ipn { node, service })
                }
                _ => Err(Error::InvalidEid {
                    reason: "ipn part must be a two-item array",
                }),
            },
            code => Err(Error::UnknownScheme { code }),
        }
    }
}

fn parse_dtn_ssp(ssp: &str) -> Result<Eid, Error> {
    if ssp == "none" {
        return Ok(Eid::Null);
    }
    let rest = ssp.strip_prefix("//").ok_or(Error::InvalidEid {
        reason: "dtn needs // before the node name",
    })?;
    let (node, demux) = rest.split_once('/').ok_or(Error::InvalidEid {
        reason: "dtn needs / after the node name",
    })?;
    if node.is_empty() {
        return Err(Error::InvalidEid {
            reason: "empty node name",
        });
    }
    if !node.bytes().all(is_reg_name_byte) {
        return Err(Error::InvalidEid {
            reason: "node name has characters outside reg-name",
        });
    }
    if !demux.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::InvalidEid {
            reason: "demux must be visible ASCII",
        });
    }
    Ok(Eid::Dtn {
        node: node.to_string(),
        demux: demux.to_string(),
    })
}

/// Characters allowed in a reg-name (RFC 3986), without percent-encoding.
fn is_reg_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

fn parse_number(s: &str) -> Result<u64, Error> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidEid {
            reason: "ipn numbers must be decimal digits",
        });
    }
    s.parse::<u64>().map_err(|_| Error::InvalidEid {
        reason: "ipn number out of range",
    })
}

impl fmt::Display for Eid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Eid::Null => write!(f, "dtn:none"),
            Eid::Dtn { node, demux } => write!(f, "dtn://{node}/{demux}"),
            Eid::Ipn { node, service } => write!(f, "ipn:{node}.{service}"),
        }
    }
}
