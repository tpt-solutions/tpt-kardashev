// SPDX-License-Identifier: MIT OR Apache-2.0
//! Link-state scheduling of LTP sessions: a session is paused while its link is closed, so the
//! closed time is not counted against its checkpoints, and the explicit reordering knob.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, Contact, ContactPlan, Eid};

fn tight_config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x500,
        max_rto_factor: 4,
    }
}

/// Three attempts per checkpoint at a fixed two-second timeout: a budget of about six seconds, less
/// than the eight-and-a-half-second closed gap. Queueing in an open window (up to about 1.5 s here)
/// stays inside one timeout, so the session cancels only if its timers run while the link is closed.
fn gap_config() -> Config {
    Config {
        mtu: 200,
        timeout_ms: 2_000,
        max_transmissions: 3,
        first_serial: 0x500,
        max_rto_factor: 1,
    }
}

fn bundle_bytes(src: u64, dst: u64, len: usize) -> Vec<u8> {
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
            data: (0..len).map(|i| (i % 251) as u8).collect(),
        }],
    }
    .encode()
    .expect("encode")
}

fn c(from: u64, to: u64, start: u64, end: u64, rate: u64, owlt: u64) -> Contact {
    Contact::new(from, to, start, end, rate, owlt).expect("contact")
}

#[test]
fn a_session_survives_a_closed_gap_longer_than_its_retransmission_budget() {
    // 1 kB/s: a 2 kB bundle needs more than the first window (0 to 1.5 s), so the rest goes in
    // the second window (30 s to 32 s). The link is closed for 28.5 s in between.
    let plan = ContactPlan::new(vec![
        c(1, 2, 0, 1_500, 1_000, 10),
        c(1, 2, 30_000, 32_000, 1_000, 10),
        c(2, 1, 0, 32_000, 1_000, 10),
    ]);
    let mut net = Network::new(plan, Impairments::default(), 3);
    net.add_node(1, gap_config()).expect("node");
    net.add_node(2, gap_config()).expect("node");
    let payload = bundle_bytes(1, 2, 2_000);
    net.send(1, 2, payload.clone()).expect("send");
    net.run_until(40_000);

    let inbox = net.inbox(2);
    assert_eq!(inbox.len(), 1, "the bundle must arrive exactly once");
    assert_eq!(inbox[0].1, payload);
    assert!(
        net.suspect_links().is_empty(),
        "the closed gap must not make the link look failed"
    );
    // No datagram starts while the link is closed.
    for t in net.transmissions().iter().filter(|t| t.from == 1) {
        let open = (t.start < 1_500) || (t.start >= 30_000 && t.start < 32_000);
        assert!(open, "datagram started at {} while closed", t.start);
    }
}

#[test]
fn a_session_begun_while_the_link_is_closed_waits_for_the_window() {
    let plan = ContactPlan::new(vec![
        c(1, 2, 5_000, 9_000, 10_000, 10),
        c(2, 1, 0, 9_000, 10_000, 10),
    ]);
    let mut net = Network::new(plan, Impairments::default(), 5);
    net.add_node(1, tight_config()).expect("node");
    net.add_node(2, tight_config()).expect("node");
    net.send(1, 2, bundle_bytes(1, 2, 300)).expect("send");
    net.run_until(20_000);
    assert_eq!(net.inbox(2).len(), 1);
    let first = net
        .transmissions()
        .iter()
        .filter(|t| t.from == 1)
        .map(|t| t.start)
        .min()
        .expect("transmitted");
    assert!(
        first >= 5_000,
        "transmitted at {first}, before the window opened"
    );
}

#[test]
fn the_reordering_knob_reorders_without_corrupting_bundles() {
    // Half the datagrams are held back by up to 800 ms, so later ones overtake them. The bundle
    // is split into many segments and must still reassemble exactly.
    let plan = ContactPlan::new(vec![
        c(1, 2, 0, 60_000, 50_000, 10),
        c(2, 1, 0, 60_000, 50_000, 10),
    ]);
    let imp = Impairments {
        reorder_percent: 50,
        reorder_ms: 800,
        ..Impairments::default()
    };
    let mut net = Network::new(plan, imp, 21);
    net.add_node(1, tight_config()).expect("node");
    net.add_node(2, tight_config()).expect("node");
    let payload = bundle_bytes(1, 2, 3_000);
    net.send(1, 2, payload.clone()).expect("send");
    net.run_until(60_000);
    let inbox = net.inbox(2);
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].1, payload);
}
