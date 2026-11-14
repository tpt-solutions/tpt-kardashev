// SPDX-License-Identifier: MIT OR Apache-2.0
//! Critical bundles: one copy through each neighbour that can carry the bundle (CCSDS 734.3-B-1
//! §2.4.5 and §3.2.8.1.3).
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CrcType, CreationTimestamp, PrimaryBlock};
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

fn bundle_bytes(src: u64, dst: u64, text: &str) -> Vec<u8> {
    Bundle {
        primary: PrimaryBlock {
            flags: 0,
            crc: CrcType::Crc32C,
            destination: Eid::Ipn {
                node: dst,
                service: 1,
            },
            source: Eid::Ipn {
                node: src,
                service: 1,
            },
            report_to: Eid::Null,
            created: CreationTimestamp {
                time: 1,
                sequence: 1,
            },
            lifetime: 1_000_000,
            fragment: None,
        },
        blocks: vec![CanonicalBlock {
            block_type: 1,
            number: 1,
            flags: 0,
            crc: CrcType::Crc32C,
            data: text.as_bytes().to_vec(),
        }],
    }
    .encode()
    .expect("encode")
}

/// Both directions of each pair, open over `[0, 100_000)`, 10 kB/s with 10 ms light time.
fn duplex(pairs: &[(u64, u64)]) -> Vec<Contact> {
    pairs
        .iter()
        .flat_map(|&(a, b)| {
            [
                Contact::new(a, b, 0, 100_000, 10_000, 10).expect("contact"),
                Contact::new(b, a, 0, 100_000, 10_000, 10).expect("contact"),
            ]
        })
        .collect()
}

fn network(pairs: &[(u64, u64)], nodes: &[u64]) -> Network {
    let mut net = Network::new(ContactPlan::new(duplex(pairs)), Impairments::default(), 11);
    net.set_routing(true);
    for &id in nodes {
        net.add_node(id, config()).expect("node");
    }
    net
}

#[test]
fn a_critical_bundle_travels_every_plausible_route() {
    // Node 1 has two neighbours, 2 and 3, and both reach the destination 4.
    let pairs = [(1, 2), (1, 3), (2, 4), (3, 4)];
    let mut net = network(&pairs, &[1, 2, 3, 4]);
    let payload = bundle_bytes(1, 4, "critical");
    assert!(net.send_critical(1, payload.clone()).expect("send"));
    net.run_until(60_000);

    let inbox = net.inbox(4);
    assert_eq!(inbox.len(), 2, "one copy per route arrives");
    let mut via: Vec<u64> = inbox.iter().map(|(from, _)| *from).collect();
    via.sort_unstable();
    assert_eq!(via, vec![2, 3], "one copy came through each neighbour");
    assert!(inbox.iter().all(|(_, p)| *p == payload));
    assert_eq!(net.discarded(), 0);
}

#[test]
fn an_ordinary_bundle_travels_one_route() {
    // The same network: a non-critical bundle takes a single route, so it arrives once.
    let pairs = [(1, 2), (1, 3), (2, 4), (3, 4)];
    let mut net = network(&pairs, &[1, 2, 3, 4]);
    let payload = bundle_bytes(1, 4, "ordinary");
    assert!(net.send_routed(1, payload.clone()).expect("send"));
    net.run_until(60_000);
    assert_eq!(net.inbox(4).len(), 1);
}

#[test]
fn a_critical_bundle_with_one_neighbour_is_sent_once() {
    // Node 1 has only neighbour 2, so there is one copy and it arrives once.
    let pairs = [(1, 2), (2, 4)];
    let mut net = network(&pairs, &[1, 2, 4]);
    let payload = bundle_bytes(1, 4, "single");
    assert!(net.send_critical(1, payload.clone()).expect("send"));
    net.run_until(60_000);
    assert_eq!(net.inbox(4), &[(2, payload)]);
}
