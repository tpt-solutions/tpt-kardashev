// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typed extension blocks of RFC 9171 §4.4: the previous node, the bundle age and the hop count.
//!
//! Each block's data is a definite-length byte string holding the CBOR encoding of its value:
//!
//! - Previous Node (type 6): an endpoint ID, the node that forwarded the bundle to this node.
//! - Bundle Age (type 7): an unsigned integer, the age in milliseconds at the last forwarding.
//! - Hop Count (type 10): an array of two unsigned integers, the hop limit (1 to 255) and the hop
//!   count.
//!
//! [`validate_blocks`] checks the occurrence rules of §4.4 for these blocks, and that their data
//! parses. It is called by [`crate::Bundle::validate`].

use crate::bundle::{Bundle, CanonicalBlock};
use crate::cbor::{ArrayLen, Reader, Writer};
use crate::eid::Eid;
use crate::error::Error;
use alloc::vec::Vec;

/// Block type code of the Previous Node block.
pub const PREVIOUS_NODE: u64 = 6;
/// Block type code of the Bundle Age block.
pub const BUNDLE_AGE: u64 = 7;
/// Block type code of the Hop Count block.
pub const HOP_COUNT: u64 = 10;

/// Data of a Previous Node block for the forwarding node `node`.
pub fn previous_node_data(node: &Eid) -> Vec<u8> {
    let mut w = Writer::new();
    node.encode(&mut w);
    w.into_bytes()
}

/// The forwarding node named by a Previous Node block's data.
pub fn parse_previous_node(data: &[u8]) -> Result<Eid, Error> {
    let mut r = Reader::new(data);
    let node = Eid::decode(&mut r)?;
    if r.remaining() != 0 {
        return Err(Error::InvalidBundle {
            rule: "previous node data has bytes after the node ID",
        });
    }
    Ok(node)
}

/// Data of a Bundle Age block for an age of `age_ms` milliseconds.
pub fn bundle_age_data(age_ms: u64) -> Vec<u8> {
    let mut w = Writer::new();
    w.uint(age_ms);
    w.into_bytes()
}

/// The age in milliseconds named by a Bundle Age block's data.
pub fn parse_bundle_age(data: &[u8]) -> Result<u64, Error> {
    let mut r = Reader::new(data);
    let age = r.read_uint()?;
    if r.remaining() != 0 {
        return Err(Error::InvalidBundle {
            rule: "bundle age data has bytes after the age",
        });
    }
    Ok(age)
}

/// Data of a Hop Count block with the given limit and count. The limit must be 1 to 255.
pub fn hop_count_data(limit: u8, count: u64) -> Result<Vec<u8>, Error> {
    if limit == 0 {
        return Err(Error::InvalidBundle {
            rule: "hop limit must be between 1 and 255",
        });
    }
    let mut w = Writer::new();
    w.array(2);
    w.uint(u64::from(limit));
    w.uint(count);
    Ok(w.into_bytes())
}

/// The hop limit and hop count named by a Hop Count block's data.
pub fn parse_hop_count(data: &[u8]) -> Result<(u8, u64), Error> {
    let mut r = Reader::new(data);
    match r.read_array_header()? {
        ArrayLen::Definite(2) => {}
        _ => {
            return Err(Error::InvalidBundle {
                rule: "hop count data must be a two-item array",
            });
        }
    }
    let limit = r.read_uint()?;
    let count = r.read_uint()?;
    if r.remaining() != 0 {
        return Err(Error::InvalidBundle {
            rule: "hop count data has bytes after the array",
        });
    }
    let limit = u8::try_from(limit)
        .ok()
        .filter(|&l| l >= 1)
        .ok_or(Error::InvalidBundle {
            rule: "hop limit must be between 1 and 255",
        })?;
    Ok((limit, count))
}

/// The occurrences of one block type in a bundle.
fn occurrences(bundle: &Bundle, block_type: u64) -> impl Iterator<Item = &CanonicalBlock> {
    bundle
        .blocks
        .iter()
        .filter(move |b| b.block_type == block_type)
}

/// Checks the typed extension blocks of a bundle: at most one of each type (§4.4.1 to §4.4.3),
/// exactly one Bundle Age block when the creation time is zero (§4.4.2), and data that parses.
pub fn validate_blocks(bundle: &Bundle) -> Result<(), Error> {
    for (block_type, what) in [
        (PREVIOUS_NODE, "more than one Previous Node block"),
        (HOP_COUNT, "more than one Hop Count block"),
        (BUNDLE_AGE, "more than one Bundle Age block"),
    ] {
        if occurrences(bundle, block_type).count() > 1 {
            return Err(Error::InvalidBundle { rule: what });
        }
    }
    let ages = occurrences(bundle, BUNDLE_AGE).count();
    if bundle.primary.created.time == 0 && ages != 1 {
        return Err(Error::InvalidBundle {
            rule: "a bundle created at time zero needs exactly one Bundle Age block",
        });
    }
    for b in occurrences(bundle, PREVIOUS_NODE) {
        parse_previous_node(&b.data)?;
    }
    for b in occurrences(bundle, BUNDLE_AGE) {
        parse_bundle_age(&b.data)?;
    }
    for b in occurrences(bundle, HOP_COUNT) {
        parse_hop_count(&b.data)?;
    }
    Ok(())
}
