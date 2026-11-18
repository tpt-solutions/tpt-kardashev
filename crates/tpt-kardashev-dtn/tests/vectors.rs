// SPDX-License-Identifier: MIT OR Apache-2.0
//! Byte-exact BPv7 vectors from an independent encoder (`tests/vectors/gen_bpv7.py`, written from RFC 9171's
//! CDDL and CRC rules and not from this crate). Each vector must decode, and re-encode to the same bytes.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::custody::{
    CUSTODY_REQUEST_BLOCK, CustodyKey, CustodySignal, parse_request, reason,
};
use tpt_kardashev_dtn::extension::{parse_hop_count, parse_previous_node};
use tpt_kardashev_dtn::{Bundle, Eid};

const VECTORS: &str = include_str!("vectors/bpv7.txt");

fn decode_hex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "odd hex length");
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex digit"))
        .collect()
}

/// The vectors as `(name, bytes)`, in file order.
fn vectors() -> Vec<(&'static str, Vec<u8>)> {
    VECTORS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|line| {
            let (name, hex) = line.split_once(' ').expect("name and hex");
            (name, decode_hex(hex))
        })
        .collect()
}

fn vector(name: &str) -> Vec<u8> {
    vectors()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, bytes)| bytes)
        .expect("named vector")
}

#[test]
fn every_vector_decodes_and_re_encodes_to_the_same_bytes() {
    let all = vectors();
    assert!(all.len() >= 8, "the vector file holds the expected vectors");
    for (name, bytes) in all {
        let bundle = Bundle::decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let again = bundle.encode().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(
            again, bytes,
            "{name}: re-encoding differs from the independent bytes"
        );
    }
}

#[test]
fn the_minimal_vector_has_the_expected_fields() {
    let bundle = Bundle::decode(&vector("min_ipn_crc32c")).expect("decodes");
    assert_eq!(
        bundle.primary.destination,
        Eid::Ipn {
            node: 2,
            service: 1
        }
    );
    assert_eq!(
        bundle.primary.source,
        Eid::Ipn {
            node: 1,
            service: 1
        }
    );
    assert_eq!(bundle.primary.created.time, 1000);
    assert_eq!(bundle.primary.created.sequence, 1);
    assert_eq!(bundle.primary.lifetime, 60_000);
    assert_eq!(bundle.payload(), Some(&b"hello"[..]));
}

#[test]
fn the_fragment_vector_carries_its_offset_and_total_length() {
    let bundle = Bundle::decode(&vector("fragment_offset_100_of_1000")).expect("decodes");
    let fragment = bundle.primary.fragment.expect("a fragment");
    assert_eq!(fragment.offset, 100);
    assert_eq!(fragment.total_len, 1000);
    assert_eq!(bundle.payload().map(|p| p.len()), Some(50));
}

#[test]
fn the_extension_vector_parses_each_block() {
    let bundle = Bundle::decode(&vector("extension_blocks")).expect("decodes");
    let previous = bundle
        .blocks
        .iter()
        .find(|b| b.block_type == 6)
        .expect("previous node block");
    assert_eq!(
        parse_previous_node(&previous.data).expect("parses"),
        Eid::Ipn {
            node: 5,
            service: 1
        }
    );
    let hops = bundle
        .blocks
        .iter()
        .find(|b| b.block_type == 10)
        .expect("hop count block");
    assert_eq!(parse_hop_count(&hops.data).expect("parses"), (16, 3));
}

#[test]
fn the_custody_signal_vectors_decode_to_the_recorded_outcomes() {
    let accepted = Bundle::decode(&vector("custody_signal_accepted")).expect("decodes");
    let signal = CustodySignal::decode(accepted.payload().expect("payload")).expect("signal");
    assert_eq!(
        signal,
        CustodySignal {
            succeeded: true,
            reason: reason::NO_ADDITIONAL_INFORMATION,
            subject: CustodyKey {
                source: 2,
                time: 1000,
                sequence: 9,
                fragment_offset: None,
            },
            signal_time: 500,
        }
    );

    let refused = Bundle::decode(&vector("custody_signal_refused_fragment")).expect("decodes");
    let signal = CustodySignal::decode(refused.payload().expect("payload")).expect("signal");
    assert!(!signal.succeeded);
    assert_eq!(signal.reason, reason::DEPLETED_STORAGE);
    assert_eq!(signal.subject.fragment_offset, Some(500));
    assert_eq!(signal.subject.source, 3);
}

#[test]
fn the_custody_request_vector_names_its_custodian() {
    let bundle = Bundle::decode(&vector("custody_request_block")).expect("decodes");
    let block = bundle
        .blocks
        .iter()
        .find(|b| b.block_type == CUSTODY_REQUEST_BLOCK)
        .expect("custody request block");
    assert_eq!(parse_request(&block.data).expect("parses"), 2);
}

#[test]
fn a_corrupted_payload_fails_its_crc() {
    let mut bytes = vector("min_ipn_crc32c");
    let at = bytes
        .windows(5)
        .position(|w| w == b"hello")
        .expect("the payload is in the vector");
    bytes[at] ^= 0x01;
    assert!(
        Bundle::decode(&bytes).is_err(),
        "a changed payload byte must not decode"
    );
}
