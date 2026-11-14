// SPDX-License-Identifier: MIT OR Apache-2.0
//! A bundle is not routed straight back to the node it came from (CCSDS 734.3-B-1 §3.2.5.2).
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::route::RouteConstraints;
use tpt_kardashev_dtn::{Contact, ContactPlan, EarliestArrival};

fn c(from: u64, to: u64, start: u64, end: u64) -> Contact {
    // 1000 B/s, no light time: a 50-byte bundle takes 50 ms.
    Contact::new(from, to, start, end, 1000, 0).expect("valid contact")
}

#[test]
fn a_bundle_is_not_routed_back_through_the_node_it_came_from() {
    // Node 2 holds a bundle for node 3 that it received from node 1. The route back through node
    // 1 arrives at 100 (2 -> 1 at 0-50, then 1 -> 3 at 50-100). The direct contact opens at 500
    // and arrives at 550.
    let plan = ContactPlan::new(vec![c(2, 1, 0, 1000), c(1, 3, 0, 1000), c(2, 3, 500, 1000)]);
    let unlimited = |_: &Contact| u64::MAX;
    let free = |_: &Contact| 0;
    let route = |excluded: &[u64]| {
        let constraints = RouteConstraints {
            expires: u64::MAX,
            owlt_margin: 0,
            evc: 0,
            available: &unlimited,
            excluded,
        };
        EarliestArrival
            .route_constrained(&plan, 2, 3, 0, 50, &free, &constraints)
            .expect("a route exists")
    };

    // Without the exclusion, the bounce wins.
    let bounced = route(&[]);
    assert_eq!(bounced.arrival, 100);
    assert_eq!(bounced.hops.len(), 2);

    // With node 1 excluded, the first hop may not enter it, so the direct contact is used.
    let direct = route(&[1]);
    assert_eq!(direct.arrival, 550);
    assert_eq!(direct.hops.len(), 1);
    assert_eq!(direct.hops[0].contact.to, 3);
}
