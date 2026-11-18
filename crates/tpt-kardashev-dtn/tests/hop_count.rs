// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Hop Count block in the routed simulator: each forward increments it, and a bundle whose
//! count exceeds its limit is deleted (RFC 9171 §4.4.3).
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::extension::{BUNDLE_AGE, HOP_COUNT, hop_count_data, parse_hop_count};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, Contact, ContactPlan, Eid};

fn config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x300,
        max_rto_factor: 4,
    }
}

fn c(from: u64, to: u64) -> Contact {
    Contact::new(from, to, 0, 1_000_000, 10_000, 10).expect("contact")
}

/// A chain 1 - 2 - 3 - 4, open in both directions.
fn chain() -> ContactPlan {
    let mut contacts = Vec::new();
    for (a, b) in [(1, 2), (2, 3), (3, 4)] {
        contacts.push(c(a, b));
        contacts.push(c(b, a));
    }
    ContactPlan::new(contacts)
}

/// An encoded bundle from node 1 to node 4, with a Hop Count block of the given limit.
fn bundle_with_limit(limit: u8) -> Vec<u8> {
    Bundle {
        primary: PrimaryBlock {
            flags: 0,
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
                time: 1000,
                sequence: 1,
            },
            lifetime: 1_000_000,
            fragment: None,
        },
        blocks: vec![
            CanonicalBlock {
                block_type: HOP_COUNT,
                number: 2,
                flags: 0,
                crc: CrcType::Crc32C,
                data: hop_count_data(limit, 0).expect("hop data"),
            },
            CanonicalBlock {
                block_type: 1,
                number: 1,
                flags: 0,
                crc: CrcType::Crc32C,
                data: b"hop-limited".to_vec(),
            },
        ],
    }
    .encode()
    .expect("encode")
}

fn network() -> Network {
    let mut net = Network::new(chain(), Impairments::default(), 3);
    net.set_routing(true);
    for id in 1..=4 {
        net.add_node(id, config()).expect("node");
    }
    net
}

#[test]
fn a_bundle_whose_hop_count_exceeds_its_limit_is_deleted() {
    // 1 -> 2 -> 3 -> 4 is three forwards. With a limit of 2 the third arrival, at the destination,
    // makes the count 3, which exceeds the limit.
    let mut net = network();
    assert!(net.send_routed(1, bundle_with_limit(2)).expect("send"));
    net.run_until(60_000);
    assert!(net.inbox(4).is_empty(), "delivered past its hop limit");
    assert!(net.discarded() >= 1);
}

#[test]
fn a_bundle_within_its_limit_arrives_with_its_hop_count_updated() {
    let mut net = network();
    let sent = bundle_with_limit(3);
    assert!(net.send_routed(1, sent).expect("send"));
    net.run_until(60_000);

    let delivered = net.inbox(4);
    assert_eq!(delivered.len(), 1, "delivered once: {delivered:?}");
    let bundle = Bundle::decode(&delivered[0].1).expect("decode");
    let hop = bundle
        .blocks
        .iter()
        .find(|b| b.block_type == HOP_COUNT)
        .expect("hop block");
    assert_eq!(
        parse_hop_count(&hop.data),
        Ok((3, 3)),
        "three forwards were counted"
    );
    assert!(bundle.blocks.iter().all(|b| b.block_type != BUNDLE_AGE));
    assert_eq!(bundle.payload(), Some(&b"hop-limited"[..]));
}

#[test]
fn impairments_do_not_change_the_hop_count_rule() {
    // The count depends on forwards, not on time: delivery with a limit of 3 must not change
    // with jitter and loss.
    let mut net = Network::new(
        chain(),
        Impairments {
            jitter_ms: 30,
            loss_percent: 15,
            duplicate_percent: 15,
            ..Impairments::default()
        },
        11,
    );
    net.set_routing(true);
    for id in 1..=4 {
        net.add_node(id, config()).expect("node");
    }
    assert!(net.send_routed(1, bundle_with_limit(3)).expect("send"));
    net.run_until(120_000);
    let delivered = net.inbox(4);
    assert_eq!(delivered.len(), 1, "delivered {} times", delivered.len());
    let bundle = Bundle::decode(&delivered[0].1).expect("decode");
    let hop = bundle
        .blocks
        .iter()
        .find(|b| b.block_type == HOP_COUNT)
        .expect("hop block");
    assert_eq!(parse_hop_count(&hop.data), Ok((3, 3)));
}
