// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bundle status reports (RFC 9171 §5 and §6.1.1) through the simulator: each requested event is reported by
//! the node where it happens, to the bundle's report-to endpoint, and nothing is reported that was not asked.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{
    CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock, bundle_flags,
};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::report::{Event, StatusReport, reason};
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, Contact, ContactPlan, Eid};

fn config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x800,
        max_rto_factor: 4,
    }
}

fn bundle(src: u64, dst: u64, flags: u64, report_to: Eid, lifetime: u64) -> Vec<u8> {
    Bundle {
        primary: PrimaryBlock {
            flags,
            crc: CrcType::Crc32C,
            destination: Eid::Ipn {
                node: dst,
                service: 1,
            },
            source: Eid::Ipn {
                node: src,
                service: 1,
            },
            report_to,
            created: CreationTimestamp {
                time: 1000,
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
            data: vec![7u8; 120],
        }],
    }
    .encode()
    .expect("encode")
}

fn c(from: u64, to: u64, end: u64) -> Contact {
    Contact::new(from, to, 0, end, 10_000, 10).expect("contact")
}

/// The status reports that arrived at `node`, decoded from the administrative bundles in its inbox.
fn reports_at(net: &Network, node: u64) -> Vec<StatusReport> {
    net.inbox(node)
        .iter()
        .filter_map(|(_, bytes)| {
            let admin = Bundle::decode(bytes).ok()?;
            if admin.primary.flags & bundle_flags::ADMIN_RECORD == 0 {
                return None;
            }
            StatusReport::decode(admin.payload()?).ok()
        })
        .collect()
}

fn chain(end: u64) -> ContactPlan {
    ContactPlan::new(vec![c(1, 2, end), c(2, 1, end), c(2, 3, end), c(3, 2, end)])
}

#[test]
fn each_requested_event_is_reported_by_the_node_where_it_happens() {
    let flags = bundle_flags::REPORT_RECEPTION
        | bundle_flags::REPORT_FORWARDING
        | bundle_flags::REPORT_DELIVERY;
    let mut net = Network::new(chain(600_000), Impairments::default(), 3);
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    let report_to = Eid::Ipn {
        node: 1,
        service: 1,
    };
    assert!(
        net.send_routed(1, bundle(1, 3, flags, report_to, 1_000_000))
            .expect("send")
    );
    net.run_until(120_000);

    let reports = reports_at(&net, 1);
    let count = |event: Event| reports.iter().filter(|r| r.event == event).count();
    // Received at 2 and at 3; forwarded by 1 and by 2; delivered at 3; never deleted.
    assert_eq!(count(Event::Received), 2, "reception at nodes 2 and 3");
    assert_eq!(count(Event::Forwarded), 2, "forwarding at nodes 1 and 2");
    assert_eq!(count(Event::Delivered), 1, "delivery at node 3");
    assert_eq!(count(Event::Deleted), 0);
    assert_eq!(net.status_reports_sent(), 5);
    for r in &reports {
        assert_eq!(r.reason, reason::NO_ADDITIONAL_INFORMATION);
        assert_eq!(
            r.source,
            Eid::Ipn {
                node: 1,
                service: 1
            }
        );
        assert_eq!(r.creation_time, 1000);
        assert_eq!(r.creation_sequence, 1);
        assert_eq!(r.time, None, "no status time was requested");
    }
}

#[test]
fn a_bundle_that_asks_for_nothing_produces_no_report() {
    let mut net = Network::new(chain(600_000), Impairments::default(), 4);
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    assert!(
        net.send_routed(
            1,
            bundle(
                1,
                3,
                0,
                Eid::Ipn {
                    node: 1,
                    service: 1
                },
                1_000_000
            )
        )
        .expect("send")
    );
    net.run_until(120_000);
    assert_eq!(net.inbox(3).len(), 1, "the bundle is still delivered");
    assert_eq!(net.status_reports_sent(), 0);
    assert!(reports_at(&net, 1).is_empty());
}

#[test]
fn a_report_to_dtn_none_is_not_sent() {
    let mut net = Network::new(chain(600_000), Impairments::default(), 5);
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    assert!(
        net.send_routed(
            1,
            bundle(1, 3, bundle_flags::REPORT_DELIVERY, Eid::Null, 1_000_000)
        )
        .expect("send")
    );
    net.run_until(120_000);
    assert_eq!(net.inbox(3).len(), 1);
    assert_eq!(
        net.status_reports_sent(),
        0,
        "dtn:none is no report-to endpoint"
    );
}

#[test]
fn a_bundle_that_expires_while_waiting_is_reported_deleted_for_its_lifetime() {
    // Node 1 has no route to node 3, so the bundle waits at node 1 until its 5 s lifetime ends, and node 1
    // reports the deletion to the report-to endpoint, which is node 1 itself.
    let mut net = Network::new(
        ContactPlan::new(vec![c(1, 2, 600_000), c(2, 1, 600_000)]),
        Impairments::default(),
        6,
    );
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    assert!(
        net.send_routed(
            1,
            bundle(
                1,
                3,
                bundle_flags::REPORT_DELETION,
                Eid::Ipn {
                    node: 1,
                    service: 1
                },
                5_000
            )
        )
        .expect("send")
    );
    net.run_until(60_000);
    let reports = reports_at(&net, 1);
    assert_eq!(reports.len(), 1, "one deletion report");
    assert_eq!(reports[0].event, Event::Deleted);
    assert_eq!(reports[0].reason, reason::LIFETIME_EXPIRED);
    assert_eq!(net.held(), 0, "the expired bundle is gone");
}

#[test]
fn a_status_time_is_included_when_the_bundle_asks_for_it() {
    let flags = bundle_flags::REPORT_DELIVERY | bundle_flags::STATUS_TIME_REQUESTED;
    let mut net = Network::new(chain(600_000), Impairments::default(), 7);
    net.set_routing(true);
    for n in 1..=3 {
        net.add_node(n, config()).expect("node");
    }
    assert!(
        net.send_routed(
            1,
            bundle(
                1,
                3,
                flags,
                Eid::Ipn {
                    node: 1,
                    service: 1
                },
                1_000_000
            )
        )
        .expect("send")
    );
    net.run_until(120_000);
    let reports = reports_at(&net, 1);
    assert_eq!(reports.len(), 1);
    let time = reports[0].time.expect("a status time");
    assert!(
        time > 0 && time < 120_000,
        "delivery time {time} within the run"
    );
}
