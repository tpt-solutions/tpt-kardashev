// SPDX-License-Identifier: MIT OR Apache-2.0
//! The SABR constraints on routing in the simulator: contact volume, bundle expiry and OWLT margin
//! (CCSDS 734.3-B-1 §2.4 and §3.2.6.9).
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
        first_serial: 0x300,
    }
}

/// An encoded bundle from `src` to `dst`, created at DTN time `created` (non-zero, so it needs no
/// Bundle Age block) and living `lifetime` milliseconds.
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

/// The estimated volume consumption of a bundle of `len` bytes (CCSDS 734.3-B-1 §2.4.3).
fn evc(len: usize) -> u64 {
    let len = len as u64;
    len + (3 * len).div_ceil(100).max(100)
}

fn network(contacts: Vec<Contact>, nodes: &[u64]) -> Network {
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 9);
    net.set_routing(true);
    for &id in nodes {
        net.add_node(id, config()).expect("node");
    }
    net
}

#[test]
fn a_contact_with_no_volume_left_is_skipped_for_the_relay() {
    // The direct contact 1 -> 3 is faster than the relay, so it is the earlier route while it
    // has volume. Its volume (rate x window / 1000) is just enough for two bundles of this size.
    let len = bundle_bytes(1, 3, 1, 1_000_000, "a").len();
    let window = (2 * evc(len)).div_ceil(10);
    let direct = Contact::new(1, 3, 0, window, 10_000, 10).expect("contact");
    let relay = [
        Contact::new(1, 2, 0, 100_000, 1_000, 10).expect("contact"),
        Contact::new(2, 3, 0, 100_000, 1_000, 10).expect("contact"),
    ];
    let mut contacts = vec![direct];
    contacts.extend(relay);
    let mut net = network(contacts, &[1, 2, 3]);

    let payloads: Vec<Vec<u8>> = (1..=3)
        .map(|t| bundle_bytes(1, 3, t, 1_000_000, "a"))
        .collect();
    for payload in &payloads {
        assert!(net.send_routed(1, payload.clone()).expect("send"));
    }
    net.run_until(60_000);

    let inbox = net.inbox(3);
    assert_eq!(inbox.len(), 3, "all three bundles are delivered");
    let direct_count = inbox.iter().filter(|(from, _)| *from == 1).count();
    let relay_count = inbox.iter().filter(|(from, _)| *from == 2).count();
    assert_eq!(direct_count, 2, "two bundles fit on the direct contact");
    assert_eq!(relay_count, 1, "the third is sent via the relay");
    assert_eq!(net.discarded(), 0);
}

#[test]
fn without_the_volume_limit_all_bundles_take_the_direct_contact() {
    // The same network with a direct contact large enough for all three: none needs the relay.
    let len = bundle_bytes(1, 3, 1, 1_000_000, "a").len();
    let direct = Contact::new(1, 3, 0, 3 * evc(len), 1_000, 10).expect("contact");
    let mut net = network(vec![direct], &[1, 3]);
    for t in 1..=3 {
        let payload = bundle_bytes(1, 3, t, 1_000_000, "a");
        assert!(net.send_routed(1, payload).expect("send"));
    }
    net.run_until(60_000);

    let inbox = net.inbox(3);
    assert_eq!(inbox.len(), 3);
    assert!(inbox.iter().all(|(from, _)| *from == 1));
}

#[test]
fn a_route_arriving_after_expiry_is_not_used() {
    // Created at 1 with lifetime 4: the bundle expires at 5. Its transmission and the 10 ms light
    // time cannot finish by then.
    let direct = Contact::new(1, 3, 0, 100_000, 10_000, 10).expect("contact");
    let mut net = network(vec![direct], &[1, 3]);
    let short = bundle_bytes(1, 3, 1, 4, "short");
    let _ = net.send_routed(1, short).expect("send");
    net.run_until(20_000);

    assert!(
        net.inbox(3).is_empty(),
        "the short-lived bundle is not delivered"
    );
    assert!(
        net.transmissions().iter().all(|t| (t.from, t.to) != (1, 3)),
        "no datagram is sent on the link for a route that misses expiry"
    );

    // Control: the same bundle with a long lifetime is delivered over the same link.
    let direct = Contact::new(1, 3, 0, 100_000, 10_000, 10).expect("contact");
    let mut net = network(vec![direct], &[1, 3]);
    let long = bundle_bytes(1, 3, 1, 1_000_000, "long");
    assert!(net.send_routed(1, long.clone()).expect("send"));
    net.run_until(20_000);
    assert_eq!(net.inbox(3), &[(1, long)]);
}

#[test]
fn a_large_owlt_margin_pushes_the_arrival_past_expiry() {
    // Without a margin the bundle arrives in about 20 ms, well before its expiry at 501.
    let contact = || Contact::new(1, 3, 0, 100_000, 10_000, 10).expect("contact");
    let bytes = bundle_bytes(1, 3, 1, 500, "margin");

    let mut net = network(vec![contact()], &[1, 3]);
    assert!(net.send_routed(1, bytes.clone()).expect("send"));
    net.run_until(20_000);
    assert_eq!(
        net.inbox(3),
        &[(1, bytes.clone())],
        "delivered without a margin"
    );

    // With a 1000 ms margin the projected arrival is past expiry, so the route is refused.
    let mut net = network(vec![contact()], &[1, 3]);
    net.set_owlt_margin(1_000);
    let _ = net.send_routed(1, bytes).expect("send");
    net.run_until(20_000);
    assert!(
        net.inbox(3).is_empty(),
        "the margin makes the route miss expiry"
    );
    assert!(net.transmissions().iter().all(|t| (t.from, t.to) != (1, 3)));
}
