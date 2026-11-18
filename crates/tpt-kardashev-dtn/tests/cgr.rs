// SPDX-License-Identifier: MIT OR Apache-2.0
//! Contact-graph routing: route lists with distinct first hops, routing around a backlog that does not
//! drain in time, and multiple copies over the route list.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::route::{EarliestArrival, RouteConstraints};
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, Contact, ContactPlan, Eid};

fn config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 2_000,
        max_transmissions: 60,
        first_serial: 0x600,
        max_rto_factor: 4,
    }
}

fn bundle_bytes(src: u64, dst: u64, len: usize, seq: u64) -> Vec<u8> {
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
            lifetime: 1_000_000,
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

fn c(from: u64, to: u64, start: u64, end: u64, rate: u64, owlt: u64) -> Contact {
    Contact::new(from, to, start, end, rate, owlt).expect("contact")
}

/// Node 1 reaches node 4 through node 2 (fast) and through node 3 (slow to arrive, owlt 5 s on the last
/// hop). Every link has a reverse link for the acknowledgments.
fn diamond(first_window_end: u64) -> ContactPlan {
    ContactPlan::new(vec![
        c(1, 2, 0, first_window_end, 1_000, 10),
        c(1, 2, 10_000, 60_000, 1_000, 10),
        c(2, 1, 0, 60_000, 1_000, 10),
        c(1, 3, 0, 60_000, 1_000, 10),
        c(3, 1, 0, 60_000, 1_000, 10),
        c(2, 4, 0, 60_000, 1_000, 10),
        c(4, 2, 0, 60_000, 1_000, 10),
        c(3, 4, 0, 60_000, 1_000, 5_000),
        c(4, 3, 0, 60_000, 1_000, 5_000),
    ])
}

#[test]
fn the_route_list_has_one_route_per_first_hop_in_arrival_order() {
    let plan = diamond(1_500);
    let unlimited = |_: &Contact| u64::MAX;
    let free = |_: &Contact| 0u64;
    let constraints = RouteConstraints {
        expires: u64::MAX,
        owlt_margin: 0,
        evc: 0,
        available: &unlimited,
        excluded: &[],
    };
    let routes = EarliestArrival.route_list(&plan, 1, 4, 0, 100, &free, &constraints, 8);
    assert_eq!(routes.len(), 2, "one route through 2 and one through 3");
    let firsts: Vec<u64> = routes.iter().map(|r| r.hops[0].contact.to).collect();
    assert_eq!(firsts, vec![2, 3], "the fast route is first");
    assert!(routes[0].arrival < routes[1].arrival);
    // Asking for one route gives the best alone.
    let best = EarliestArrival.route_list(&plan, 1, 4, 0, 100, &free, &constraints, 1);
    assert_eq!(best.len(), 1);
    assert_eq!(best[0], routes[0]);
}

#[test]
fn a_bundle_routes_around_a_backlog_that_spills_past_its_window() {
    // Node 1 reaches node 4 through node 2 and through node 3. The first bundle (1.8 kB) cannot
    // finish in the first 1-2 window (0 to 1.5 s), so it goes in the second (from 10 s). Its backlog
    // then spills into the second window too, so a small second bundle through node 2 would finish
    // about 10.6 s, after its projected arrival through node 3 (about 10.4 s). Volume is ample, so the
    // choice is due to the backlog's drain time alone. Ignoring the backlog would send it through
    // node 2 at once.
    let plan = ContactPlan::new(vec![
        c(1, 2, 0, 1_500, 1_000, 10),
        c(1, 2, 10_000, 60_000, 1_000, 10),
        c(2, 1, 0, 60_000, 1_000, 10),
        c(1, 3, 0, 60_000, 1_000, 10),
        c(3, 1, 0, 60_000, 1_000, 10),
        c(2, 4, 0, 60_000, 1_000, 10),
        c(4, 2, 0, 60_000, 1_000, 10),
        c(3, 4, 0, 60_000, 1_000, 10_200),
        c(4, 3, 0, 60_000, 1_000, 10_200),
    ]);
    let mut net = Network::new(plan, Impairments::default(), 9);
    net.set_routing(true);
    for n in 1..=4 {
        net.add_node(n, config()).expect("node");
    }
    let first = bundle_bytes(1, 4, 1_800, 1);
    let second = bundle_bytes(1, 4, 100, 2);
    net.send_routed(1, first.clone()).expect("send");
    net.send_routed(1, second.clone()).expect("send");
    net.run_until(60_000);

    let delivered = net.inbox(4);
    assert_eq!(delivered.len(), 2, "both bundles arrive at node 4");
    let bodies: Vec<&Vec<u8>> = delivered.iter().map(|(_, p)| p).collect();
    assert!(bodies.contains(&&first) && bodies.contains(&&second));
    let first_hop_to_3 = net
        .transmissions()
        .iter()
        .filter(|t| t.from == 1 && t.to == 3)
        .count();
    assert!(
        first_hop_to_3 > 0,
        "the second bundle routes around the backlog through node 3"
    );
    let to_2_after_window_one = net
        .transmissions()
        .iter()
        .filter(|t| t.from == 1 && t.to == 2 && t.start >= 10_000)
        .count();
    assert!(
        to_2_after_window_one > 0,
        "the first bundle uses node 2's second window"
    );
}

#[test]
fn copies_go_through_distinct_neighbours_on_the_route_list() {
    let mut net = Network::new(diamond(60_000), Impairments::default(), 11);
    net.set_routing(true);
    for n in 1..=4 {
        net.add_node(n, config()).expect("node");
    }
    let payload = bundle_bytes(1, 4, 300, 3);
    assert!(net.send_copies(1, payload.clone(), 2).expect("send"));
    net.run_until(60_000);
    let copies: Vec<&Vec<u8>> = net.inbox(4).iter().map(|(_, p)| p).collect();
    assert_eq!(copies.len(), 2, "one copy per neighbour on the route list");
    assert!(copies.iter().all(|p| **p == payload));
    assert!(net.transmissions().iter().any(|t| t.from == 1 && t.to == 2));
    assert!(net.transmissions().iter().any(|t| t.from == 1 && t.to == 3));
}

#[test]
fn a_single_copy_takes_only_the_best_route() {
    let mut net = Network::new(diamond(60_000), Impairments::default(), 12);
    net.set_routing(true);
    for n in 1..=4 {
        net.add_node(n, config()).expect("node");
    }
    assert!(
        net.send_copies(1, bundle_bytes(1, 4, 300, 4), 1)
            .expect("send")
    );
    net.run_until(60_000);
    assert_eq!(net.inbox(4).len(), 1);
    assert!(
        !net.transmissions().iter().any(|t| t.from == 1 && t.to == 3),
        "no copy is sent through node 3"
    );
}

#[test]
fn a_bundle_with_no_route_waits_at_its_origin() {
    let mut net = Network::new(diamond(60_000), Impairments::default(), 13);
    net.set_routing(true);
    for n in 1..=4 {
        net.add_node(n, config()).expect("node");
    }
    // Node 9 has no contacts, so there is no route list.
    assert!(
        net.send_copies(1, bundle_bytes(1, 9, 100, 5), 3)
            .expect("send")
    );
    net.run_until(10_000);
    assert_eq!(
        net.held(),
        1,
        "the bundle is kept at node 1 until a route appears"
    );
}
