// SPDX-License-Identifier: MIT OR Apache-2.0
//! Contact validation and earliest-arrival routing against hand-computed scenarios.
//!
//! Times are DTN milliseconds; rates are bytes per second, so a transmission of `size` bytes at
//! `rate` takes `size * 1000 / rate` ms. Each expected value below is worked out by hand.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::{Contact, ContactPlan, EarliestArrival, Error, Router};

fn c(from: u64, to: u64, start: u64, end: u64, rate: u64, owlt: u64) -> Contact {
    Contact::new(from, to, start, end, rate, owlt).expect("valid contact")
}

#[test]
fn malformed_contacts_are_rejected() {
    assert!(matches!(
        Contact::new(1, 1, 0, 10, 1, 0),
        Err(Error::InvalidContact { .. })
    ));
    assert!(matches!(
        Contact::new(1, 2, 10, 10, 1, 0),
        Err(Error::InvalidContact { .. })
    ));
    assert!(matches!(
        Contact::new(1, 2, 0, 10, 0, 0),
        Err(Error::InvalidContact { .. })
    ));
}

#[test]
fn transmit_time_rounds_up() {
    // 50 bytes at 1000 B/s is 50 ms; 1 byte at 3000 B/s is 1/3 ms, rounded up to 1 ms.
    assert_eq!(c(1, 2, 0, 1000, 1000, 0).transmit_ms(50), 50);
    assert_eq!(c(1, 2, 0, 1000, 3000, 0).transmit_ms(1), 1);
    assert_eq!(c(1, 2, 0, 1000, 1000, 0).transmit_ms(0), 0);
}

#[test]
fn direct_contact_delivers_with_light_time() {
    // Window [100, 200], 1000 B/s, owlt 10. A 50-byte bundle ready at 0 departs at 100, takes
    // 50 ms, finishes at 150 (inside the window), and arrives at 150 + 10 = 160.
    let plan = ContactPlan::new(vec![c(1, 2, 100, 200, 1000, 10)]);
    let r = EarliestArrival.route(&plan, 1, 2, 0, 50).expect("route");
    assert_eq!(r.arrival, 160);
    assert_eq!(r.hops.len(), 1);
    assert_eq!(r.hops[0].departure, 100);
    assert_eq!(r.hops[0].arrival, 160);
}

#[test]
fn bundle_that_does_not_fit_the_window_has_no_route() {
    // 100 bytes at 1000 B/s takes 100 ms, but the window [0, 50] is only 50 ms long.
    let plan = ContactPlan::new(vec![c(1, 2, 0, 50, 1000, 0)]);
    assert_eq!(EarliestArrival.route(&plan, 1, 2, 0, 100), None);
}

#[test]
fn relay_waits_at_the_intermediate_node_for_the_next_contact() {
    // 1 -> 2 open [0, 1000], 2 -> 3 open [500, 2000]; both 1000 B/s, no light time.
    // 100 bytes: 1 -> 2 departs 0, takes 100 ms, arrives 100. 2 -> 3 opens at 500, so it departs
    // 500, takes 100 ms, arrives 600.
    let plan = ContactPlan::new(vec![c(1, 2, 0, 1000, 1000, 0), c(2, 3, 500, 2000, 1000, 0)]);
    let r = EarliestArrival.route(&plan, 1, 3, 0, 100).expect("route");
    assert_eq!(r.arrival, 600);
    assert_eq!(r.hops.len(), 2);
    assert_eq!(r.hops[1].departure, 500);
}

#[test]
fn store_and_forward_does_not_overlap_hops() {
    // The relay forwards only after the whole bundle has arrived. The first hop finishes at 300
    // (300 bytes at 1000 B/s), so the second hop, already open from 0, departs at 300 and
    // finishes at 600.
    let plan = ContactPlan::new(vec![c(1, 2, 0, 1000, 1000, 0), c(2, 3, 0, 1000, 1000, 0)]);
    let r = EarliestArrival.route(&plan, 1, 3, 0, 300).expect("route");
    assert_eq!(r.hops[0].arrival, 300);
    assert_eq!(r.hops[1].departure, 300);
    assert_eq!(r.arrival, 600);
}

#[test]
fn the_earlier_of_two_routes_is_chosen() {
    // Via 2: arrives 600 (see the relay test above). Direct 1 -> 3 opens at 0 and arrives 100.
    let plan = ContactPlan::new(vec![
        c(1, 2, 0, 1000, 1000, 0),
        c(2, 3, 500, 2000, 1000, 0),
        c(1, 3, 0, 5000, 1000, 0),
    ]);
    let r = EarliestArrival.route(&plan, 1, 3, 0, 100).expect("route");
    assert_eq!(r.arrival, 100);
    assert_eq!(r.hops.len(), 1);
}

#[test]
fn ready_time_later_than_a_window_skips_it() {
    // The bundle is ready at 250, after the window [0, 200] has closed. The next window opens at
    // 300; a 10-byte bundle takes 10 ms at 1000 B/s; owlt 10. Departs 300, finishes 310, arrives 320.
    let plan = ContactPlan::new(vec![c(1, 2, 0, 200, 1000, 0), c(1, 2, 300, 400, 1000, 10)]);
    let r = EarliestArrival.route(&plan, 1, 2, 250, 10).expect("route");
    assert_eq!(r.hops[0].contact.start, 300);
    assert_eq!(r.arrival, 320);
}

#[test]
fn no_route_when_the_graph_is_disconnected() {
    let plan = ContactPlan::new(vec![c(1, 2, 0, 1000, 1000, 0)]);
    assert_eq!(EarliestArrival.route(&plan, 2, 1, 0, 10), None);
    assert_eq!(EarliestArrival.route(&plan, 1, 3, 0, 10), None);
}

#[test]
fn source_equals_destination_is_an_empty_route() {
    let plan = ContactPlan::new(vec![]);
    let r = EarliestArrival.route(&plan, 7, 7, 42, 10).expect("route");
    assert!(r.hops.is_empty());
    assert_eq!(r.arrival, 42);
}

#[test]
fn routing_does_not_depend_on_the_order_contacts_are_given() {
    let a = vec![
        c(1, 2, 0, 1000, 1000, 0),
        c(2, 3, 0, 1000, 1000, 0),
        c(1, 3, 0, 5000, 500, 0),
    ];
    let mut b = a.clone();
    b.reverse();
    let ra = EarliestArrival.route(&ContactPlan::new(a), 1, 3, 0, 100);
    let rb = EarliestArrival.route(&ContactPlan::new(b), 1, 3, 0, 100);
    assert_eq!(ra, rb);
}
