// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bundles routed through a contact plan by the simulator: the router chooses the next hop at
//! every node, with no manual forwarding.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::extension::{BUNDLE_AGE, bundle_age_data};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, Contact, ContactPlan, Eid};

fn config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x200,
    }
}

/// An encoded bundle from node `src` to node `dst`, created at DTN time `created` and living
/// `lifetime` milliseconds.
fn bundle_bytes(src: u64, dst: u64, created: u64, lifetime: u64, text: &str) -> Vec<u8> {
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
                time: created,
                sequence: 1,
            },
            lifetime,
            fragment: None,
        },
        blocks: {
            let payload = CanonicalBlock {
                block_type: 1,
                number: 1,
                flags: 0,
                crc: CrcType::Crc32C,
                data: text.as_bytes().to_vec(),
            };
            // A bundle created at time zero must carry a Bundle Age block (RFC 9171 §4.4.2).
            if created == 0 {
                let age = CanonicalBlock {
                    block_type: BUNDLE_AGE,
                    number: 2,
                    flags: 0,
                    crc: CrcType::Crc32C,
                    data: bundle_age_data(0),
                };
                vec![age, payload]
            } else {
                vec![payload]
            }
        },
    }
    .encode()
    .expect("encode")
}

fn c(from: u64, to: u64, start: u64, end: u64, owlt: u64) -> Contact {
    Contact::new(from, to, start, end, 10_000, owlt).expect("contact")
}

/// Both directions of each listed pair, open over `[start, end)`.
fn duplex(pairs: &[(u64, u64)], start: u64, end: u64) -> Vec<Contact> {
    pairs
        .iter()
        .flat_map(|&(a, b)| [c(a, b, start, end, 10), c(b, a, start, end, 10)])
        .collect()
}

fn network(contacts: Vec<Contact>, nodes: &[u64]) -> Network {
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 9);
    net.set_routing(true);
    for &id in nodes {
        net.add_node(id, config()).expect("node");
    }
    net
}

fn run_to_idle(net: &mut Network, until: u64) {
    net.run_until(until);
}

#[test]
fn the_router_finds_the_relay_without_manual_forwarding() {
    // Only 1 <-> 2 and 2 <-> 3 exist, so the only route from 1 to 3 is through 2.
    let contacts = duplex(&[(1, 2), (2, 3)], 0, 100_000);
    let mut net = network(contacts, &[1, 2, 3]);
    let payload = bundle_bytes(1, 3, 0, 1_000_000, "routed");

    assert!(net.send_routed(1, payload.clone()).expect("send"));
    run_to_idle(&mut net, 20_000);

    assert_eq!(
        net.inbox(3),
        &[(2, payload.clone())],
        "delivered once, via node 2"
    );
    assert!(
        net.inbox(2).is_empty(),
        "the relay forwards, it does not keep the bundle"
    );
    // Node 1 transmitted towards 2, and node 2 towards 3; nothing went from 1 straight to 3.
    assert!(net.transmissions().iter().any(|t| (t.from, t.to) == (1, 2)));
    assert!(net.transmissions().iter().any(|t| (t.from, t.to) == (2, 3)));
    assert!(net.transmissions().iter().all(|t| (t.from, t.to) != (1, 3)));
    assert_eq!(net.discarded(), 0);
}

#[test]
fn an_earlier_direct_contact_is_preferred_to_the_relay() {
    // 1 -> 3 is open now; the relay path 1 -> 2 -> 3 would arrive later.
    let mut contacts = duplex(&[(1, 2), (2, 3)], 0, 100_000);
    contacts.extend(duplex(&[(1, 3)], 0, 100_000));
    let mut net = network(contacts, &[1, 2, 3]);
    let payload = bundle_bytes(1, 3, 0, 1_000_000, "direct");

    assert!(net.send_routed(1, payload.clone()).expect("send"));
    run_to_idle(&mut net, 20_000);

    assert_eq!(
        net.inbox(3),
        &[(1, payload)],
        "delivered directly from node 1"
    );
    assert!(net.transmissions().iter().all(|t| (t.from, t.to) != (1, 2)));
}

#[test]
fn a_bundle_with_no_route_is_held_and_not_transmitted() {
    // Node 3 is not connected to anything yet. The bundle waits at node 1 (it is not discarded,
    // because its lifetime is long), and nothing is sent.
    let contacts = duplex(&[(1, 2)], 0, 100_000);
    let mut net = network(contacts, &[1, 2, 3]);
    let payload = bundle_bytes(1, 3, 0, 1_000_000, "unreachable");

    assert!(net.send_routed(1, payload).expect("send"));
    run_to_idle(&mut net, 5_000);
    assert_eq!(net.held(), 1);
    assert!(net.inbox(3).is_empty());
    assert!(net.transmissions().is_empty());
    assert_eq!(net.discarded(), 0);
}

#[test]
fn a_route_that_appears_later_is_taken_by_the_held_bundle() {
    // At time 0 there is no route to node 3. A contact 2 -> 3 is added at t = 300, and the bundle
    // held at node 1 is routed then: it goes 1 -> 2 -> 3, and arrives after 300.
    let base = duplex(&[(1, 2)], 0, 100_000);
    let mut net = network(base.clone(), &[1, 2, 3]);
    let mut later = base;
    later.extend(duplex(&[(2, 3)], 300, 100_000));
    net.schedule_plan(300, ContactPlan::new(later));
    let payload = bundle_bytes(1, 3, 0, 1_000_000, "waited for a route");

    assert!(net.send_routed(1, payload.clone()).expect("send"));
    run_to_idle(&mut net, 20_000);

    assert_eq!(net.inbox(3), &[(2, payload)]);
    assert_eq!(net.held(), 0);
    assert_eq!(net.discarded(), 0);
}

#[test]
fn a_new_contact_re_routes_a_bundle_already_waiting_at_a_relay() {
    // The first relay hop is open now, but 2 -> 3 only opens at t = 1000. At t = 500 the plan is
    // replaced with one where 2 -> 3 opens at 500. The bundle waiting at node 2 is re-routed and
    // leaves at 500, so it reaches node 3 before the original window would have let it.
    let first = duplex(&[(1, 2)], 0, 100_000);
    let mut plan = first.clone();
    plan.extend(duplex(&[(2, 3)], 1000, 100_000));
    let mut net = network(plan, &[1, 2, 3]);
    let payload = bundle_bytes(1, 3, 0, 1_000_000, "re-routed at a relay");

    let mut changed = first;
    changed.extend(duplex(&[(2, 3)], 500, 100_000));
    net.schedule_plan(500, ContactPlan::new(changed));

    assert!(net.send_routed(1, payload.clone()).expect("send"));
    net.run_until(900);
    assert_eq!(
        net.inbox(3),
        &[(2, payload)],
        "delivered before the original window opened"
    );
}

#[test]
fn an_expired_bundle_is_not_forwarded() {
    // Created at 0 with lifetime 0: it has expired by the time it reaches the relay at a virtual
    // time after 0, so the relay discards it.
    let contacts = duplex(&[(1, 2), (2, 3)], 0, 100_000);
    let mut net = network(contacts, &[1, 2, 3]);
    let payload = bundle_bytes(1, 3, 0, 0, "expired");

    assert!(net.send_routed(1, payload).expect("send"));
    run_to_idle(&mut net, 20_000);
    assert!(net.inbox(3).is_empty());
    assert!(net.discarded() >= 1);
}

#[test]
fn the_hop_limit_stops_a_bundle_that_is_forwarded_too_often() {
    // A chain 1 - 2 - 3 - 4. The bundle is handled at 1 (origin), 2 and 3 before node 4 gets it.
    // With a limit of 2, the third handling, at node 3, is refused.
    let contacts = duplex(&[(1, 2), (2, 3), (3, 4)], 0, 100_000);
    let payload = bundle_bytes(1, 4, 0, 1_000_000, "too far");

    let mut limited = network(contacts.clone(), &[1, 2, 3, 4]);
    limited.set_max_hops(2);
    assert!(limited.send_routed(1, payload.clone()).expect("send"));
    run_to_idle(&mut limited, 40_000);
    assert!(
        limited.inbox(4).is_empty(),
        "delivered despite the hop limit"
    );

    let mut generous = network(contacts, &[1, 2, 3, 4]);
    assert!(generous.send_routed(1, payload.clone()).expect("send"));
    run_to_idle(&mut generous, 40_000);
    assert_eq!(generous.inbox(4), &[(3, payload)]);
}

#[test]
fn routing_off_leaves_forwarding_to_the_caller() {
    let contacts = duplex(&[(1, 2), (2, 3)], 0, 100_000);
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 9);
    for id in [1, 2, 3] {
        net.add_node(id, config()).expect("node");
    }
    let payload = bundle_bytes(1, 3, 0, 1_000_000, "manual");
    net.send(1, 2, payload.clone()).expect("send");
    run_to_idle(&mut net, 20_000);
    // Without routing, node 2 keeps what it receives, and nothing is forwarded.
    assert_eq!(net.inbox(2), &[(1, payload)]);
    assert!(net.inbox(3).is_empty());
}

#[test]
fn a_full_link_pushes_the_next_bundle_onto_the_relay() {
    // Direct 1 -> 3 is slow (6000 B/s); the relay 1 -> 2 -> 3 is fast (10000 B/s per hop). The first
    // bundle takes the direct link. The second is routed while the first is still queued on that
    // link, so the router sees the direct link as busy and takes the relay instead: it arrives
    // around t = 60 ms, where the direct link would deliver it around t = 120 ms.
    let mut contacts = vec![
        c(1, 3, 0, 100_000, 0),
        c(1, 2, 0, 100_000, 0),
        c(2, 3, 0, 100_000, 0),
    ];
    contacts.push(c(3, 1, 0, 100_000, 0));
    contacts.push(c(2, 1, 0, 100_000, 0));
    contacts.push(c(3, 2, 0, 100_000, 0));
    // Make the direct link's rate 6000 B/s, and the others 10000 B/s, as the comment says.
    let contacts: Vec<Contact> = contacts
        .into_iter()
        .map(|k| {
            let rate = if (k.from, k.to) == (1, 3) || (k.from, k.to) == (3, 1) {
                6000
            } else {
                10_000
            };
            Contact::new(k.from, k.to, k.start, k.end, rate, k.owlt).expect("contact")
        })
        .collect();
    let mut net = network(contacts, &[1, 2, 3]);
    let first = bundle_bytes(1, 3, 0, 1_000_000, "first bundle, sent on the direct link");
    let second = bundle_bytes(
        1,
        3,
        0,
        1_000_000,
        "second bundle, sent while the first is queued",
    );

    assert!(net.send_routed(1, first.clone()).expect("send"));
    assert!(net.send_routed(1, second.clone()).expect("send"));
    net.run_until(100);

    let inbox = net.inbox(3);
    assert_eq!(
        inbox.len(),
        2,
        "both bundles delivered by t = 100: {inbox:?}"
    );
    assert_eq!(
        inbox[0],
        (1, first),
        "the first bundle used the direct link"
    );
    assert_eq!(
        inbox[1],
        (2, second),
        "the second bundle took the relay around the full link"
    );
}

#[test]
fn a_link_that_fails_unannounced_is_learned_from_the_cancelled_session_and_routed_around() {
    // Two routes from 1 to 4: via 2 (fast) and via 3 (slower, longer light time). The plan lists
    // every link as open. Link 1 -> 2 fails at t = 0 without any announcement, so the first route
    // cannot carry anything. The bundle is held at node 1 until node 1's session on that link is
    // cancelled at its retransmission limit; then it is re-routed through node 3.
    let mut contacts = duplex(&[(1, 2), (2, 4)], 0, 1_000_000);
    contacts.extend(
        [(1, 3), (3, 4)]
            .iter()
            .flat_map(|&(a, b)| [c(a, b, 0, 1_000_000, 500), c(b, a, 0, 1_000_000, 500)]),
    );
    let mut net = network(contacts, &[1, 2, 3, 4]);
    net.fail_link(1, 2, 0);
    let payload = bundle_bytes(1, 4, 0, 10_000_000, "around a failed link");

    assert!(net.send_routed(1, payload.clone()).expect("send"));

    // Before the session has had time to give up, nothing is known to be wrong.
    net.run_until(1_000);
    assert!(net.suspect_links().is_empty());
    assert!(net.inbox(4).is_empty());

    // The suspicion lasts the default hold (60 s) from detection at about 18 s, so check it before it lapses.
    net.run_until(40_000);
    assert_eq!(
        net.suspect_links(),
        vec![(1, 2)],
        "the failed link was learned"
    );
    assert_eq!(net.inbox(4), &[(3, payload)], "delivered once, via node 3");
    assert_eq!(net.discarded(), 0);
}

#[test]
fn a_link_that_worked_before_is_declared_failed_sooner_than_the_configured_timeout() {
    // The configured timeout is 300 ms with 60 transmissions, so a link that never worked is only
    // declared failed after about 18 s. Here link 1 -> 2 first carries a bundle, which gives node 1
    // a round-trip estimate for node 2. When the link then fails, that estimate shortens the
    // retransmission timer, and the failure is declared well before 18 s.
    let mut contacts = duplex(&[(1, 2), (2, 4)], 0, 1_000_000);
    contacts.extend(
        [(1, 3), (3, 4)]
            .iter()
            .flat_map(|&(a, b)| [c(a, b, 0, 1_000_000, 300), c(b, a, 0, 1_000_000, 300)]),
    );
    let mut net = network(contacts, &[1, 2, 3, 4]);
    let first = bundle_bytes(1, 4, 0, 10_000_000, "first, before the failure");
    assert!(net.send_routed(1, first.clone()).expect("send"));
    net.run_until(2_000);
    assert_eq!(
        net.inbox(4).len(),
        1,
        "the first bundle was delivered while the link worked"
    );

    net.fail_link(1, 2, 2_000);
    net.run_until(3_000);
    let second = bundle_bytes(1, 4, 0, 10_000_000, "second, after the failure");
    assert!(net.send_routed(1, second.clone()).expect("send"));

    let sent_at = 3_000u64;
    let mut t = sent_at;
    while net.suspect_links().is_empty() && t < sent_at + 30_000 {
        t += 50;
        net.run_until(t);
    }
    let detected_after = t - sent_at;
    assert_eq!(
        net.suspect_links(),
        vec![(1, 2)],
        "the failed link was learned"
    );
    assert!(
        detected_after < 10_000,
        "failure declared after {detected_after} ms; the fixed timeout alone allows 18000 ms"
    );

    net.run_until(60_000);
    assert!(
        net.inbox(4)
            .iter()
            .any(|(from, p)| *from == 3 && *p == second),
        "the second bundle is delivered around the failed link"
    );
}

/// Two routes from 1 to 4 (fast via 2, slow via 3) and a first bundle that gives node 1 a round-trip
/// estimate for node 2.
fn outage_network() -> (Network, Vec<u8>) {
    let mut contacts = duplex(&[(1, 2), (2, 4)], 0, 1_000_000);
    contacts.extend(
        [(1, 3), (3, 4)]
            .iter()
            .flat_map(|&(a, b)| [c(a, b, 0, 1_000_000, 300), c(b, a, 0, 1_000_000, 300)]),
    );
    let mut net = network(contacts, &[1, 2, 3, 4]);
    let first = bundle_bytes(1, 4, 0, 10_000_000, "before the outage");
    assert!(net.send_routed(1, first.clone()).expect("send"));
    net.run_until(2_000);
    assert_eq!(net.inbox(4).len(), 1);
    // Link 1 -> 2 is down from t = 3 s to t = 12 s, and then works again.
    net.fail_link_between(1, 2, 3_000, 12_000);
    net.run_until(3_000);
    (net, first)
}

#[test]
fn a_failed_link_is_tried_again_once_its_suspicion_lapses() {
    let (mut net, _) = outage_network();
    net.set_suspect_hold(10_000);
    let during = bundle_bytes(1, 4, 0, 10_000_000, "during the outage");
    assert!(net.send_routed(1, during.clone()).expect("send"));

    net.run_until(15_000);
    assert_eq!(net.suspect_links(), vec![(1, 2)], "the outage was learned");
    assert!(
        net.inbox(4).iter().any(|(f, p)| *f == 3 && *p == during),
        "routed around it via 3"
    );

    // The suspicion has lapsed by t = 20 s, so the router uses link 1 -> 2 again, and it works.
    net.run_until(20_000);
    assert!(net.suspect_links().is_empty(), "the suspicion lapsed");
    let after = bundle_bytes(1, 4, 0, 10_000_000, "after the outage");
    assert!(net.send_routed(1, after.clone()).expect("send"));
    net.run_until(40_000);
    assert!(
        net.inbox(4).iter().any(|(f, p)| *f == 2 && *p == after),
        "the recovered link carries the bundle again"
    );
}

#[test]
fn a_completed_transfer_clears_a_suspicion_still_in_force() {
    let (mut net, _) = outage_network();
    // The default hold would keep the suspicion for a minute.
    let during = bundle_bytes(1, 4, 0, 10_000_000, "during the outage");
    assert!(net.send_routed(1, during).expect("send"));
    net.run_until(15_000);
    assert_eq!(net.suspect_links(), vec![(1, 2)]);

    // The link works again from t = 12 s. A transfer sent over it directly completes, and that
    // shows the link works, so the suspicion is dropped at once.
    let probe = bundle_bytes(1, 2, 0, 10_000_000, "probe");
    net.send(1, 2, probe).expect("send");
    net.run_until(20_000);
    assert!(
        net.suspect_links().is_empty(),
        "a completed transfer cleared the suspicion"
    );
}

#[test]
fn a_short_partition_delays_delivery_until_it_heals() {
    // Both directions of 1 <-> 2 are cut from 0 to 2 s. The sessions retransmit their checkpoints
    // through the outage, so the bundle arrives once the partition has healed, and a short outage
    // does not get the link marked as failed.
    let mut net = network(duplex(&[(1, 2)], 0, 100_000), &[1, 2]);
    net.partition(1, 2, 0, 2_000);
    let payload = bundle_bytes(1, 2, 0, 10_000_000, "partitioned");
    assert!(net.send_routed(1, payload.clone()).expect("send"));

    net.run_until(1_500);
    assert!(
        net.inbox(2).is_empty(),
        "nothing crosses during the partition"
    );

    net.run_until(30_000);
    assert_eq!(
        net.inbox(2),
        &[(1, payload)],
        "delivered after the partition"
    );
    assert!(
        net.suspect_links().is_empty(),
        "a short partition is not a failed link"
    );
    assert_eq!(net.discarded(), 0);
}
