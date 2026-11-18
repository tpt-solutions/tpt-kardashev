// SPDX-License-Identifier: MIT OR Apache-2.0
//! Custody transfer through the simulator: hop-by-hop custody and its release, refusal and rerouting,
//! lost signals recovered by timers with exactly-once delivery, the attempt limit, and storage.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, Contact, ContactPlan, Eid};

fn config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x700,
        max_rto_factor: 4,
    }
}

/// A config whose sessions give up after about a second, so that a dead link is learned quickly.
fn impatient() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 3,
        first_serial: 0x700,
        max_rto_factor: 1,
    }
}

fn bundle_to(src: u64, dst: u64, len: usize, seq: u64, lifetime: u64) -> Vec<u8> {
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
                sequence: seq,
            },
            lifetime,
            fragment: None,
        },
        blocks: vec![CanonicalBlock {
            block_type: 1,
            number: 1,
            flags: 0,
            crc: CrcType::Crc32C,
            data: (0..len).map(|i| (i % 251) as u8).collect(),
        }],
    }
    .encode()
    .expect("encode")
}

fn c(from: u64, to: u64, start: u64, end: u64, owlt: u64) -> Contact {
    Contact::new(from, to, start, end, 10_000, owlt).expect("contact")
}

/// Every link of `pairs`, in both directions, open until `end`.
fn duplex(pairs: &[(u64, u64)], end: u64) -> Vec<Contact> {
    pairs
        .iter()
        .flat_map(|&(a, b)| [c(a, b, 0, end, 10), c(b, a, 0, end, 10)])
        .collect()
}

#[test]
fn custody_is_released_hop_by_hop_as_each_next_custodian_accepts() {
    let plan = ContactPlan::new(duplex(&[(1, 2), (2, 3)], 600_000));
    let mut net = Network::new(plan, Impairments::default(), 3);
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    net.set_custody_capacity(2, 100_000);
    let payload = bundle_to(1, 3, 500, 1, 1_000_000);
    assert!(net.send_custody(1, payload).expect("send"));
    net.run_until(120_000);

    assert_eq!(net.inbox(3).len(), 1, "delivered exactly once");
    assert_eq!(net.custody_copies(1), 0, "the origin released its copy");
    assert_eq!(net.custody_copies(2), 0, "the custodian released its copy");
    let stats = net.custody_stats();
    assert_eq!(stats.accepted, 1, "node 2 took custody once");
    assert_eq!(stats.released, 2, "nodes 1 and 2 released their copies");
    assert_eq!(stats.refused, 0);
}

#[test]
fn a_custodian_without_storage_refuses_and_the_previous_custodian_routes_around_it() {
    // Node 1 reaches node 4 through node 2 (the fast route) and node 3. Node 2 has no storage.
    let mut contacts = duplex(&[(1, 2), (2, 4), (1, 3)], 600_000);
    contacts.push(c(3, 4, 0, 600_000, 2_000));
    contacts.push(c(4, 3, 0, 600_000, 2_000));
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 5);
    net.set_routing(true);
    for n in 1..=4 {
        net.add_node(n, config()).expect("node");
    }
    net.set_custody_capacity(3, 100_000);
    assert!(
        net.send_custody(1, bundle_to(1, 4, 400, 2, 1_000_000))
            .expect("send")
    );
    net.run_until(120_000);

    assert_eq!(net.inbox(4).len(), 1, "delivered once, over node 3");
    let stats = net.custody_stats();
    assert!(stats.refused >= 1, "node 2 refused custody");
    assert!(stats.rerouted >= 1, "node 1 routed again around node 2");
    assert!(
        net.transmissions().iter().any(|t| t.from == 1 && t.to == 3),
        "the bundle went through node 3"
    );
    assert_eq!(net.custody_copies(1), 0);
}

#[test]
fn a_lost_signal_is_recovered_by_the_timer_and_the_bundle_is_delivered_once() {
    // The signal from node 3 to node 2 is lost for the first ten seconds (its sessions give up), so
    // node 2 keeps its copy and resends after its three-second timeout. Node 3 acknowledges the copy
    // again without delivering it a second time.
    let mut net = Network::new(
        ContactPlan::new(duplex(&[(1, 2), (2, 3)], 600_000)),
        Impairments::default(),
        7,
    );
    // The sessions keep their patience through the outage, so only the signal is delayed past the
    // timeout: the data and its acknowledgments both use the failed link, and must survive it.
    net.fail_link_between(3, 2, 0, 10_000);
    net.set_routing(true);
    net.add_node(1, config()).expect("node");
    net.add_node(2, config()).expect("node");
    net.add_node(3, config()).expect("node");
    net.set_custody_capacity(2, 100_000);
    net.set_custody_timeout(3_000);
    net.set_custody_attempts(10);
    assert!(
        net.send_custody(1, bundle_to(1, 3, 300, 3, 1_000_000))
            .expect("send")
    );
    net.run_until(200_000);

    assert_eq!(net.inbox(3).len(), 1, "delivered exactly once");
    assert_eq!(
        net.custody_copies(2),
        0,
        "custody released once a signal arrived"
    );
    assert!(
        net.custody_stats().retransmitted >= 1,
        "the timer resent the copy"
    );
}

#[test]
fn a_copy_is_given_up_after_its_attempt_limit() {
    // The link from node 2 to node 3 never works. Node 2 takes custody and its hand-offs fail; after two
    // attempts it gives up. Node 1 released its copy when node 2 accepted the bundle.
    let mut contacts = duplex(&[(1, 2)], 600_000);
    contacts.push(c(2, 3, 0, 600_000, 10));
    contacts.push(c(3, 2, 0, 600_000, 10));
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 9);
    net.fail_link(2, 3, 0);
    net.set_routing(true);
    net.set_suspect_hold(1_000);
    net.add_node(1, impatient()).expect("node");
    net.add_node(2, impatient()).expect("node");
    net.add_node(3, impatient()).expect("node");
    net.set_custody_capacity(2, 100_000);
    net.set_custody_timeout(3_000);
    net.set_custody_attempts(2);
    assert!(
        net.send_custody(1, bundle_to(1, 3, 300, 4, 1_000_000))
            .expect("send")
    );
    net.run_until(60_000);

    assert!(net.inbox(3).is_empty());
    assert_eq!(net.custody_stats().failed, 1, "node 2 gave up once");
    assert_eq!(net.custody_copies(2), 0);
    assert_eq!(
        net.custody_copies(1),
        0,
        "node 1 released its copy on acceptance"
    );
}

#[test]
fn custody_needs_room_for_the_whole_bundle() {
    let mut net = Network::new(
        ContactPlan::new(duplex(&[(1, 2), (2, 3)], 600_000)),
        Impairments::default(),
        11,
    );
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    // Node 2 is an intermediate custodian with room for 100 bytes; the bundle is 500 bytes.
    net.set_custody_capacity(2, 100);
    assert!(
        net.send_custody(1, bundle_to(1, 3, 500, 5, 1_000_000))
            .expect("send")
    );
    net.run_until(60_000);
    assert_eq!(
        net.custody_stats().refused,
        1,
        "a bundle larger than the store is refused"
    );
    assert_eq!(net.custody_copies(2), 0);
    assert!(
        net.inbox(3).is_empty(),
        "a refused bundle is not forwarded through the refusing node"
    );
}

#[test]
fn custody_requests_need_routing_and_a_known_node() {
    let mut net = Network::new(
        ContactPlan::new(duplex(&[(1, 2)], 1_000)),
        Impairments::default(),
        1,
    );
    net.add_node(1, config()).expect("node");
    assert!(
        net.send_custody(1, bundle_to(1, 2, 10, 6, 1_000)).is_err(),
        "routing is off"
    );
    net.set_routing(true);
    assert!(
        net.send_custody(9, bundle_to(1, 2, 10, 6, 1_000)).is_err(),
        "unknown node"
    );
}

#[test]
fn copies_over_two_routes_are_each_taken_into_custody_and_delivered_once() {
    // Two routes leave node 1: through node 2 and through node 3, both to node 4. Each copy goes to its
    // own custodian, which takes custody of it. The destination delivers the bundle once.
    let contacts = duplex(&[(1, 2), (1, 3), (2, 4), (3, 4)], 600_000);
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 17);
    net.set_routing(true);
    for n in 1..=4 {
        net.add_node(n, config()).expect("node");
    }
    net.set_custody_capacity(2, 100_000);
    net.set_custody_capacity(3, 100_000);
    assert!(
        net.send_custody_copies(1, bundle_to(1, 4, 300, 8, 1_000_000), 2)
            .expect("send")
    );
    net.run_until(120_000);
    assert_eq!(
        net.inbox(4).len(),
        1,
        "delivered once although two copies arrive"
    );
    assert_eq!(
        net.custody_stats().accepted,
        2,
        "both custodians took custody"
    );
    assert_eq!(net.custody_copies(2), 0);
    assert_eq!(net.custody_copies(3), 0);
    assert_eq!(net.custody_copies(1), 0);
}

#[test]
fn a_custody_bundle_waiting_for_a_route_is_routed_when_the_plan_changes() {
    // The plan has no contacts at first, so the bundle waits at node 1. At 5 s the chain 1-2-3 opens, and the
    // bundle is routed over it and taken into custody at node 2.
    let mut net = Network::new(ContactPlan::new(Vec::new()), Impairments::default(), 13);
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    net.set_custody_capacity(2, 100_000);
    assert!(
        net.send_custody(1, bundle_to(1, 3, 300, 9, 1_000_000))
            .expect("send")
    );
    net.schedule_plan(5_000, ContactPlan::new(duplex(&[(1, 2), (2, 3)], 600_000)));
    net.run_until(120_000);
    assert_eq!(
        net.inbox(3).len(),
        1,
        "delivered once, after the plan changed"
    );
    assert_eq!(net.custody_stats().accepted, 1);
    assert_eq!(net.custody_copies(1), 0);
    assert_eq!(net.custody_copies(2), 0);
}

#[test]
fn each_fragment_is_taken_into_custody_and_the_fragments_reassemble() {
    use tpt_kardashev_dtn::fragment::reassemble_encoded;
    let original = Bundle::decode(&bundle_to(1, 3, 3_000, 10, 1_000_000)).expect("decodes");
    let encoded = original.encode().expect("encodes");
    let mut net = Network::new(
        ContactPlan::new(duplex(&[(1, 2), (2, 3)], 600_000)),
        Impairments::default(),
        15,
    );
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    net.set_custody_capacity(2, 1_000_000);
    assert_eq!(
        net.send_custody_fragmented(1, &encoded, 500).expect("send"),
        6
    );
    net.run_until(300_000);

    let pieces: Vec<Vec<u8>> = net.inbox(3).iter().map(|(_, p)| p.clone()).collect();
    assert_eq!(pieces.len(), 6, "every fragment arrives once");
    let whole = reassemble_encoded(&pieces).expect("reassembles");
    assert_eq!(
        whole.payload(),
        original.payload(),
        "the application data is restored"
    );
    assert_eq!(
        net.custody_stats().accepted,
        6,
        "each fragment was taken into custody"
    );
    assert_eq!(net.custody_copies(1), 0);
    assert_eq!(net.custody_copies(2), 0);
}
