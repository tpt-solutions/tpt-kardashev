// SPDX-License-Identifier: MIT OR Apache-2.0
//! A bundle sent in fragments through the simulator: each fragment arrives as its own bundle, and the
//! destination's inbox reassembles to the original.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::fragment::reassemble_encoded;
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, CanonicalBlock, Contact, ContactPlan, Eid};

fn config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x400,
    }
}

fn big_bundle(len: usize) -> Bundle {
    Bundle {
        primary: PrimaryBlock {
            flags: 0,
            crc: CrcType::Crc32C,
            destination: Eid::Ipn {
                node: 2,
                service: 1,
            },
            source: Eid::Ipn {
                node: 1,
                service: 1,
            },
            report_to: Eid::Null,
            created: CreationTimestamp {
                time: 1,
                sequence: 1,
            },
            lifetime: 10_000_000,
            fragment: None,
        },
        blocks: vec![CanonicalBlock {
            block_type: 1,
            number: 1,
            flags: 0,
            crc: CrcType::Crc32C,
            data: (0..len).map(|i| (i * 7 % 256) as u8).collect(),
        }],
    }
}

fn network() -> Network {
    let contacts = vec![
        Contact::new(1, 2, 0, 1_000_000, 10_000, 10).expect("contact"),
        Contact::new(2, 1, 0, 1_000_000, 10_000, 10).expect("contact"),
    ];
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 13);
    net.set_routing(true);
    net.add_node(1, config()).expect("node");
    net.add_node(2, config()).expect("node");
    net
}

#[test]
fn a_fragmented_bundle_arrives_in_pieces_and_reassembles_to_the_original() {
    let original = big_bundle(3_000);
    let encoded = original.encode().expect("encode");
    let mut net = network();
    let sent = net.send_fragmented(1, &encoded, 500).expect("send");
    assert_eq!(sent, 6, "3000 bytes in pieces of 500");
    net.run_until(3_600_000);
    let pieces: Vec<Vec<u8>> = net.inbox(2).iter().map(|(_, p)| p.clone()).collect();
    assert_eq!(pieces.len(), 6, "every fragment arrives once");
    let whole = reassemble_encoded(&pieces).expect("reassembles");
    assert_eq!(whole, original);
}

#[test]
fn an_unfragmented_send_needs_no_reassembly() {
    let original = big_bundle(100);
    let encoded = original.encode().expect("encode");
    let mut net = network();
    assert_eq!(net.send_fragmented(1, &encoded, 500).expect("send"), 1);
    net.run_until(3_600_000);
    assert_eq!(net.inbox(2).len(), 1);
}
