// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bundle fragmentation and reassembly (RFC 9171 §5.8 and §5.9): sizes and offsets, every fragment a
//! valid bundle that round-trips through the codec, reassembly in any order, the placement of blocks
//! that are and are not replicated, and refusal of bad input.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{
    CrcType, CreationTimestamp, PrimaryBlock, block_flags, bundle_flags,
};
use tpt_kardashev_dtn::fragment::{fragment, reassemble};
use tpt_kardashev_dtn::{Bundle, CanonicalBlock, Eid};

fn bundle(payload_len: usize, flags: u64, extra: Vec<CanonicalBlock>) -> Bundle {
    let mut blocks = extra;
    blocks.push(CanonicalBlock {
        block_type: 1,
        number: 1,
        flags: 0,
        crc: CrcType::Crc32C,
        data: (0..payload_len).map(|i| (i % 251) as u8).collect(),
    });
    Bundle {
        primary: PrimaryBlock {
            flags,
            crc: CrcType::Crc32C,
            destination: Eid::Ipn {
                node: 4,
                service: 1,
            },
            source: Eid::Ipn {
                node: 1,
                service: 1,
            },
            report_to: Eid::Null,
            created: CreationTimestamp {
                time: 7,
                sequence: 3,
            },
            lifetime: 1_000_000,
            fragment: None,
        },
        blocks,
    }
}

fn extra_block(number: u64, flags: u64) -> CanonicalBlock {
    CanonicalBlock {
        block_type: 200 + number,
        number,
        flags,
        crc: CrcType::Crc16,
        data: vec![number as u8; 5],
    }
}

#[test]
fn a_payload_of_1000_bytes_splits_into_fragments_of_at_most_300_that_round_trip() {
    let original = bundle(1000, 0, vec![]);
    let parts = fragment(&original, 300).expect("fragments");
    let sizes: Vec<usize> = parts
        .iter()
        .map(|b| b.payload().expect("payload").len())
        .collect();
    assert_eq!(sizes, vec![300, 300, 300, 100]);
    let offsets: Vec<u64> = parts
        .iter()
        .map(|b| b.primary.fragment.expect("fragment").offset)
        .collect();
    assert_eq!(offsets, vec![0, 300, 600, 900]);
    for part in &parts {
        assert!(part.primary.flags & bundle_flags::FRAGMENT != 0);
        assert_eq!(part.primary.fragment.expect("fragment").total_len, 1000);
        let encoded = part.encode().expect("a fragment is a valid bundle");
        assert_eq!(&Bundle::decode(&encoded).expect("decodes"), part);
    }
}

#[test]
fn reassembly_gives_the_original_in_any_order() {
    let original = bundle(1000, 0, vec![extra_block(2, 0)]);
    let mut parts = fragment(&original, 300).expect("fragments");
    parts.reverse();
    parts.swap(0, 2);
    let whole = reassemble(&parts).expect("reassembles");
    assert_eq!(whole, original);
}

#[test]
fn a_payload_that_fits_is_not_split() {
    let original = bundle(100, 0, vec![]);
    let parts = fragment(&original, 300).expect("fragments");
    assert_eq!(parts, vec![original]);
}

#[test]
fn blocks_that_are_not_replicated_go_only_in_the_first_fragment() {
    let original = bundle(
        900,
        0,
        vec![
            extra_block(2, 0),
            extra_block(3, block_flags::REPLICATE_IN_EVERY_FRAGMENT),
        ],
    );
    let parts = fragment(&original, 300).expect("fragments");
    assert_eq!(parts.len(), 3);
    let has = |b: &Bundle, number: u64| b.blocks.iter().any(|blk| blk.number == number);
    assert!(has(&parts[0], 2) && has(&parts[0], 3));
    assert!(!has(&parts[1], 2) && has(&parts[1], 3));
    assert!(!has(&parts[2], 2) && has(&parts[2], 3));
}

#[test]
fn a_missing_duplicate_or_foreign_fragment_is_refused() {
    let original = bundle(900, 0, vec![]);
    let parts = fragment(&original, 300).expect("fragments");
    assert!(reassemble(&parts[..2]).is_err(), "a missing fragment");
    let mut duplicated = parts.clone();
    duplicated.push(parts[1].clone());
    assert!(reassemble(&duplicated).is_err(), "a duplicated fragment");
    let other = bundle(900, 0, vec![]);
    let mut other_parts = fragment(&other, 300).expect("fragments");
    other_parts[2].primary.created.sequence = 99;
    assert!(
        reassemble(&other_parts).is_err(),
        "a fragment of another bundle"
    );
    assert!(
        reassemble(&[original]).is_err(),
        "a bundle that is not a fragment"
    );
    assert!(reassemble(&[]).is_err(), "nothing to reassemble");
}

#[test]
fn bad_fragmentation_requests_are_refused() {
    let original = bundle(900, 0, vec![]);
    assert!(fragment(&original, 0).is_err(), "a zero size");
    let no_fragment = bundle(900, bundle_flags::NO_FRAGMENT, vec![]);
    assert!(fragment(&no_fragment, 300).is_err(), "NO_FRAGMENT is set");
    let parts = fragment(&original, 300).expect("fragments");
    assert!(
        fragment(&parts[0], 100).is_err(),
        "a fragment is not fragmented again"
    );
}
