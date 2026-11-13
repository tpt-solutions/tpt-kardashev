// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bundles over LTP across a contact-plan network: store-and-forward at a relay, impairments,
//! and determinism.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, Contact, ContactPlan, Eid, Error};

fn config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x100,
    }
}

/// A bundle from node `src` to node `dst` carrying `text`, encoded.
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
                time: 1000,
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

fn c(from: u64, to: u64, start: u64, end: u64, owlt: u64) -> Contact {
    Contact::new(from, to, start, end, 10_000, owlt).expect("contact")
}

/// Links 1<->2 are open for the whole run; the 2<->3 link opens only in [1000, 1500).
fn relay_plan() -> ContactPlan {
    ContactPlan::new(vec![
        c(1, 2, 0, 5000, 10),
        c(2, 1, 0, 5000, 10),
        c(2, 3, 1000, 1500, 10),
        c(3, 2, 1000, 1500, 10),
    ])
}

fn network(plan: ContactPlan, imp: Impairments, seed: u64) -> Network {
    let mut net = Network::new(plan, imp, seed);
    for id in 1..=3 {
        net.add_node(id, config()).expect("node");
    }
    net
}

/// Sends a bundle from 1 to 3 through relay 2, forwarding it at the relay as soon as it arrives,
/// and runs until node 3 has it or the horizon passes. Returns the network and the time node 3
/// received it.
fn relay_transfer(net: &mut Network, payload: &[u8], horizon: u64) -> Option<u64> {
    net.send(1, 2, payload.to_vec()).expect("send");
    let mut forwarded = false;
    let mut t = 0u64;
    while t < horizon {
        t += 50;
        net.run_until(t);
        if !forwarded && !net.inbox(2).is_empty() {
            net.send(2, 3, payload.to_vec()).expect("forward");
            forwarded = true;
        }
        if !net.inbox(3).is_empty() {
            return Some(t);
        }
    }
    None
}

#[test]
fn bundle_is_relayed_only_once_the_second_window_opens() {
    let payload = bundle_bytes(1, 3, "via relay");
    let mut net = network(relay_plan(), Impairments::default(), 1);
    let delivered_at = relay_transfer(&mut net, &payload, 4000).expect("delivered");

    assert_eq!(net.inbox(3), &[(2, payload.clone())]);
    assert_eq!(
        Bundle::decode(&net.inbox(3)[0].1)
            .expect("bundle")
            .payload(),
        Some(&b"via relay"[..])
    );
    // The relay holds the bundle until the 2 -> 3 window opens at 1000, so node 3 cannot have it
    // before then.
    assert!(
        delivered_at >= 1000,
        "delivered at {delivered_at}, before the window opened"
    );

    // Every datagram on the 2 <-> 3 link was transmitted inside that link's window.
    for tx in net.transmissions() {
        if (tx.from, tx.to) == (2, 3) || (tx.from, tx.to) == (3, 2) {
            assert!(
                tx.start >= 1000 && tx.finish <= 1500,
                "transmission outside its window: {tx:?}"
            );
        }
    }
}

#[test]
fn impairments_do_not_duplicate_or_corrupt_the_bundle() {
    let payload = bundle_bytes(1, 3, "lossy and duplicated");
    let plan = ContactPlan::new(vec![
        c(1, 2, 0, 100_000, 20),
        c(2, 1, 0, 100_000, 20),
        c(2, 3, 0, 100_000, 20),
        c(3, 2, 0, 100_000, 20),
    ]);
    let imp = Impairments {
        jitter_ms: 40,
        loss_percent: 25,
        duplicate_percent: 25,
    };
    let mut net = network(plan, imp, 42);
    relay_transfer(&mut net, &payload, 60_000).expect("delivered");
    // Run on well past delivery: duplicates and late retransmissions must not produce more copies.
    net.run_until(120_000);
    assert_eq!(
        net.inbox(3).len(),
        1,
        "bundle delivered {} times",
        net.inbox(3).len()
    );
    assert_eq!(net.inbox(3)[0].1, payload);
    assert!(
        net.transmissions().iter().any(|t| t.lost),
        "no loss was injected"
    );
}

#[test]
fn runs_are_deterministic_for_a_given_seed() {
    let payload = bundle_bytes(1, 3, "same every time");
    let plan = || {
        ContactPlan::new(vec![
            c(1, 2, 0, 100_000, 20),
            c(2, 1, 0, 100_000, 20),
            c(2, 3, 0, 100_000, 20),
            c(3, 2, 0, 100_000, 20),
        ])
    };
    let imp = Impairments {
        jitter_ms: 30,
        loss_percent: 20,
        duplicate_percent: 20,
    };
    let mut a = network(plan(), imp, 7);
    let mut b = network(plan(), imp, 7);
    let ta = relay_transfer(&mut a, &payload, 60_000);
    let tb = relay_transfer(&mut b, &payload, 60_000);
    assert_eq!(ta, tb);
    assert_eq!(a.transmissions(), b.transmissions());
    assert_eq!(a.inbox(3), b.inbox(3));
}

#[test]
fn a_node_that_was_never_added_cannot_send() {
    let mut net = network(relay_plan(), Impairments::default(), 1);
    assert_eq!(net.send(99, 1, vec![1, 2, 3]), Err(Error::UnknownNode));
}
