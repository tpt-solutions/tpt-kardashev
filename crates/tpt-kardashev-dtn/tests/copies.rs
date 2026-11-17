// SPDX-License-Identifier: MIT OR Apache-2.0
//! Multiple copies: a bundle is sent through the first `k` neighbours whose contacts open earliest
//! (ties by node id). With one copy it takes the earliest route; with as many copies as neighbours it is
//! sent as a critical bundle is.
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
        max_rto_factor: 4,
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

/// A diamond 1–2–4 and 1–3–4 in which the link 1–3 opens at 5 s, after the link 1–2 (open from 0).
/// Both directions of each pair are in the plan.
fn diamond() -> Network {
    let mut contacts = Vec::new();
    for (a, b, start) in [(1, 2, 0), (2, 4, 0), (1, 3, 5_000), (3, 4, 0)] {
        contacts.push(Contact::new(a, b, start, 100_000, 10_000, 10).expect("contact"));
        contacts.push(Contact::new(b, a, start, 100_000, 10_000, 10).expect("contact"));
    }
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 11);
    net.set_routing(true);
    for id in [1, 2, 3, 4] {
        net.add_node(id, config()).expect("node");
    }
    net
}

fn arrivals_via(net: &Network) -> Vec<u64> {
    let mut via: Vec<u64> = net.inbox(4).iter().map(|(from, _)| *from).collect();
    via.sort_unstable();
    via
}

#[test]
fn one_copy_takes_the_route_whose_first_hop_opens_first() {
    let mut net = diamond();
    let payload = bundle_bytes(1, 4, "one");
    assert!(net.send_copies(1, payload, 1).expect("send"));
    net.run_until(60_000);
    assert_eq!(
        arrivals_via(&net),
        vec![2],
        "the copy went through 2, which opens first"
    );
}

#[test]
fn two_copies_go_through_both_neighbours() {
    let mut net = diamond();
    let payload = bundle_bytes(1, 4, "two");
    assert!(net.send_copies(1, payload, 2).expect("send"));
    net.run_until(60_000);
    assert_eq!(arrivals_via(&net), vec![2, 3]);
}

#[test]
fn as_many_copies_as_neighbours_is_the_critical_send() {
    let payload = bundle_bytes(1, 4, "critical");
    let mut by_copies = diamond();
    assert!(
        by_copies
            .send_copies(1, payload.clone(), usize::MAX)
            .expect("send")
    );
    by_copies.run_until(60_000);
    let mut critical = diamond();
    assert!(critical.send_critical(1, payload).expect("send"));
    critical.run_until(60_000);
    assert_eq!(arrivals_via(&by_copies), arrivals_via(&critical));
    assert_eq!(arrivals_via(&critical), vec![2, 3]);
}

#[test]
fn zero_copies_is_refused() {
    let mut net = diamond();
    let payload = bundle_bytes(1, 4, "none");
    assert!(net.send_copies(1, payload, 0).is_err());
}
