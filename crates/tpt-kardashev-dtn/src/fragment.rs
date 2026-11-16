// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fragmentation and reassembly of bundles (RFC 9171 §5.8 and §5.9).
//!
//! A bundle that is larger than the volume a contact can carry is split into fragments, each a bundle
//! in its own right with the fragment fields set (CCSDS 734.3-B-1 §2.4.4 and §3.2.8.2 ask for this
//! under SABR). The destination reassembles them once they all arrive. Each fragment carries a slice of
//! the application data unit; the blocks that are not replicated go only in the first fragment, and
//! the blocks that are replicated (their `REPLICATE_IN_EVERY_FRAGMENT` flag) go in every fragment.
//!
//! Checked against RFC 9171 §5.8 and §5.9 (2026-11-16). Two rules are stricter than the RFC, on purpose:
//! a fragment is not split again (the RFC allows it, one level at a time), and reassembly refuses
//! overlapping fragments (the RFC accepts overlapping material extents from different fragmentation
//! episodes). A bundle with `NO_FRAGMENT` is refused, as the RFC requires.

use alloc::vec::Vec;

use crate::bundle::{
    BLOCK_TYPE_PAYLOAD, Bundle, CanonicalBlock, Fragment, block_flags, bundle_flags,
};
use crate::error::Error;

/// Splits `bundle` into fragments whose payloads are at most `max_payload` bytes. A bundle whose payload
/// already fits is returned unchanged, as the only element. Fails with [`Error::InvalidBundle`] for a
/// zero size, a bundle that must not be fragmented, or a bundle that is already a fragment, and with the
/// validation error of any fragment that does not form a valid bundle.
pub fn fragment(bundle: &Bundle, max_payload: usize) -> Result<Vec<Bundle>, Error> {
    bundle.validate()?;
    if max_payload == 0 {
        return Err(Error::InvalidBundle {
            rule: "a fragment must carry at least one byte",
        });
    }
    if bundle.primary.flags & bundle_flags::NO_FRAGMENT != 0 {
        return Err(Error::InvalidBundle {
            rule: "bundle must not be fragmented",
        });
    }
    if bundle.primary.fragment.is_some() {
        return Err(Error::InvalidBundle {
            rule: "a fragment is not fragmented again",
        });
    }
    let payload = bundle.payload().unwrap_or(&[]);
    if payload.len() <= max_payload {
        return Ok(alloc::vec![bundle.clone()]);
    }
    let total = payload.len() as u64;
    // Validation has placed the payload block last, so the blocks before it are the others.
    let Some((payload_block, others)) = bundle.blocks.split_last() else {
        return Err(Error::InvalidBundle {
            rule: "a bundle needs a payload block",
        });
    };
    let mut out = Vec::new();
    let mut offset = 0usize;
    while offset < payload.len() {
        let end = (offset + max_payload).min(payload.len());
        let first = offset == 0;
        let mut blocks: Vec<CanonicalBlock> = others
            .iter()
            .filter(|b| first || b.flags & block_flags::REPLICATE_IN_EVERY_FRAGMENT != 0)
            .cloned()
            .collect();
        blocks.push(CanonicalBlock {
            data: payload[offset..end].to_vec(),
            ..payload_block.clone()
        });
        let mut primary = bundle.primary.clone();
        primary.flags |= bundle_flags::FRAGMENT;
        primary.fragment = Some(Fragment {
            offset: offset as u64,
            total_len: total,
        });
        out.push(Bundle { primary, blocks });
        offset = end;
    }
    Ok(out)
}

/// Reassembles the fragments of one bundle into the original bundle. The fragments may come in any
/// order. They must all be fragments of the same bundle (same identity, destination, source, report
/// address, creation timestamp, lifetime and flags, apart from the fragment flag), and their payloads
/// must cover the application data unit exactly, with no gap and no overlap.
///
/// Fails with [`Error::InvalidBundle`] when the list is empty, a bundle is not a fragment, the bundles
/// disagree, or the fragments are missing, duplicated or overlapping.
pub fn reassemble(fragments: &[Bundle]) -> Result<Bundle, Error> {
    let Some(first) = fragments.first() else {
        return Err(Error::InvalidBundle {
            rule: "no fragments to reassemble",
        });
    };
    let mut parts: Vec<(&Bundle, Fragment)> = Vec::with_capacity(fragments.len());
    for b in fragments {
        let Some(f) = b.primary.fragment else {
            return Err(Error::InvalidBundle {
                rule: "a bundle to reassemble is not a fragment",
            });
        };
        parts.push((b, f));
    }
    let identity = |b: &Bundle| {
        let mut p = b.primary.clone();
        p.flags &= !bundle_flags::FRAGMENT;
        p.fragment = None;
        p
    };
    let expected_primary = identity(first);
    let total = parts[0].1.total_len;
    for (b, f) in &parts {
        if identity(b) != expected_primary || f.total_len != total {
            return Err(Error::InvalidBundle {
                rule: "fragments are not of the same bundle",
            });
        }
    }
    parts.sort_by_key(|(_, f)| f.offset);
    let mut expected_offset = 0u64;
    let mut data = Vec::with_capacity(total as usize);
    for (b, f) in &parts {
        if f.offset != expected_offset {
            return Err(Error::InvalidBundle {
                rule: "fragments are missing, duplicated or overlapping",
            });
        }
        let payload = b.payload().unwrap_or(&[]);
        data.extend_from_slice(payload);
        expected_offset += payload.len() as u64;
    }
    if expected_offset != total {
        return Err(Error::InvalidBundle {
            rule: "fragments do not cover the application data unit",
        });
    }
    // The first fragment carries every block that is not replicated, so its other blocks are the
    // bundle's, and its payload block gives the number and flags of the reassembled payload.
    let head = parts[0].0;
    let Some((head_payload, head_others)) = head.blocks.split_last() else {
        return Err(Error::InvalidBundle {
            rule: "a bundle needs a payload block",
        });
    };
    if head_payload.block_type != BLOCK_TYPE_PAYLOAD {
        return Err(Error::InvalidBundle {
            rule: "the payload block is last",
        });
    }
    let mut blocks: Vec<CanonicalBlock> = head_others.to_vec();
    blocks.push(CanonicalBlock {
        data,
        ..head_payload.clone()
    });
    let whole = Bundle {
        primary: expected_primary,
        blocks,
    };
    whole.validate()?;
    Ok(whole)
}

/// Reassembles encoded fragments, as a destination reads them from its inbox: each is decoded, then
/// [`reassemble`] is applied. Fails as [`reassemble`], and with the codec's error for an undecodable one.
pub fn reassemble_encoded(encoded: &[Vec<u8>]) -> Result<Bundle, Error> {
    let mut parts = Vec::with_capacity(encoded.len());
    for bytes in encoded {
        parts.push(Bundle::decode(bytes)?);
    }
    reassemble(&parts)
}
