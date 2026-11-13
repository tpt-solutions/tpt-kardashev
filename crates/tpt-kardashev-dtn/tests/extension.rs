// SPDX-License-Identifier: MIT OR Apache-2.0
//! Typed extension blocks (RFC 9171 §4.4): byte-exact data, parsing rules, and occurrence rules.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::extension::{
    BUNDLE_AGE, HOP_COUNT, PREVIOUS_NODE, bundle_age_data, hop_count_data, parse_bundle_age,
    parse_hop_count, parse_previous_node, previous_node_data,
};
use tpt_kardashev_dtn::{Bundle, Eid, Error};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn primary(created: u64) -> PrimaryBlock {
    PrimaryBlock {
        flags: 0,
        crc: CrcType::Crc32C,
        destination: Eid::parse("ipn:9.1").expect("eid"),
        source: Eid::parse("ipn:1.1").expect("eid"),
        report_to: Eid::Null,
        created: CreationTimestamp {
            time: created,
            sequence: 0,
        },
        lifetime: 100_000,
        fragment: None,
    }
}

fn block(block_type: u64, number: u64, data: Vec<u8>) -> CanonicalBlock {
    CanonicalBlock {
        block_type,
        number,
        flags: 0,
        crc: CrcType::Crc32C,
        data,
    }
}

fn payload() -> CanonicalBlock {
    block(1, 1, b"payload".to_vec())
}

#[test]
fn previous_node_data_is_the_cbor_endpoint_id() {
    // ipn:1.2 is [2, [1, 2]].
    let eid = Eid::parse("ipn:1.2").expect("eid");
    assert_eq!(previous_node_data(&eid), hex("8202820102"));
    assert_eq!(parse_previous_node(&hex("8202820102")), Ok(eid));
    // dtn:none is [1, 0].
    assert_eq!(previous_node_data(&Eid::Null), hex("820100"));
    assert!(
        parse_previous_node(&hex("82010000")).is_err(),
        "trailing bytes accepted"
    );
}

#[test]
fn bundle_age_data_is_a_cbor_unsigned_integer() {
    assert_eq!(bundle_age_data(1000), hex("1903e8"));
    assert_eq!(parse_bundle_age(&hex("1903e8")), Ok(1000));
    assert_eq!(parse_bundle_age(&hex("00")), Ok(0));
    assert!(
        parse_bundle_age(&hex("6161")).is_err(),
        "text accepted as an age"
    );
}

#[test]
fn hop_count_data_is_a_two_item_array() {
    assert_eq!(hop_count_data(255, 3).expect("data"), hex("8218ff03"));
    assert_eq!(parse_hop_count(&hex("8218ff03")), Ok((255, 3)));
    assert_eq!(parse_hop_count(&hex("820103")), Ok((1, 3)));
}

#[test]
fn hop_limits_outside_one_to_255_are_rejected() {
    assert!(hop_count_data(0, 0).is_err());
    // 0 and 256 are not valid limits when parsed.
    assert!(parse_hop_count(&hex("820003")).is_err());
    assert!(parse_hop_count(&hex("82190100 03".replace(' ', "").as_str())).is_err());
    // A one-item array is not hop count data.
    assert!(parse_hop_count(&hex("8101")).is_err());
}

#[test]
fn a_bundle_with_all_three_blocks_round_trips() {
    let bundle = Bundle {
        primary: primary(1000),
        blocks: vec![
            block(
                PREVIOUS_NODE,
                2,
                previous_node_data(&Eid::parse("ipn:7.1").expect("eid")),
            ),
            block(BUNDLE_AGE, 3, bundle_age_data(250)),
            block(HOP_COUNT, 4, hop_count_data(16, 2).expect("data")),
            payload(),
        ],
    };
    let bytes = bundle.encode().expect("encode");
    let back = Bundle::decode(&bytes).expect("decode");
    assert_eq!(back, bundle);
    assert_eq!(
        parse_previous_node(&back.blocks[0].data),
        Ok(Eid::parse("ipn:7.1").expect("eid"))
    );
    assert_eq!(parse_bundle_age(&back.blocks[1].data), Ok(250));
    assert_eq!(parse_hop_count(&back.blocks[2].data), Ok((16, 2)));
}

#[test]
fn at_most_one_of_each_extension_block_is_allowed() {
    for (block_type, data) in [
        (PREVIOUS_NODE, previous_node_data(&Eid::Null)),
        (HOP_COUNT, hop_count_data(8, 0).expect("data")),
        (BUNDLE_AGE, bundle_age_data(0)),
    ] {
        let bundle = Bundle {
            primary: primary(1000),
            blocks: vec![
                block(block_type, 2, data.clone()),
                block(block_type, 3, data),
                payload(),
            ],
        };
        assert!(
            matches!(bundle.encode(), Err(Error::InvalidBundle { .. })),
            "two blocks of type {block_type} accepted"
        );
    }
}

#[test]
fn a_bundle_created_at_time_zero_needs_exactly_one_age_block() {
    let without = Bundle {
        primary: primary(0),
        blocks: vec![payload()],
    };
    assert!(matches!(without.encode(), Err(Error::InvalidBundle { .. })));

    let with = Bundle {
        primary: primary(0),
        blocks: vec![block(BUNDLE_AGE, 2, bundle_age_data(0)), payload()],
    };
    assert!(with.encode().is_ok());

    // Later creation times do not require the block.
    let later = Bundle {
        primary: primary(1),
        blocks: vec![payload()],
    };
    assert!(later.encode().is_ok());
}

#[test]
fn malformed_extension_data_is_rejected_on_encode() {
    let bundle = Bundle {
        primary: primary(1000),
        blocks: vec![block(HOP_COUNT, 2, hex("820003")), payload()],
    };
    assert!(matches!(bundle.encode(), Err(Error::InvalidBundle { .. })));

    let bundle = Bundle {
        primary: primary(1000),
        blocks: vec![block(BUNDLE_AGE, 2, b"not cbor".to_vec()), payload()],
    };
    assert!(bundle.encode().is_err());
}
