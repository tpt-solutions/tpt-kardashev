// SPDX-License-Identifier: MIT OR Apache-2.0
//! A bundle sent in fragments through the simulator: each fragment arrives as its own bundle, and the
//! destination's inbox reassembles to the original.
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::fragment::reassemble_encoded;
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

fn big_bundle(len: usize) -> Bundle {
    Bundle {
        primary: PrimaryBlock {
            flags: 0,
            crc: CrcType::Crc32C,
            destination: Eid::Ipn {
                node: 2,
                service: 1,
            },
            source: Eid::Ipn {
                node: 1,
                service: 1,
            },
            report_to: Eid::Null,
            created: CreationTimestamp {
                time: 1,
                sequence: 1,
            },
            lifetime: 10_000_000,
            fragment: None,
        },
        blocks: vec![CanonicalBlock {
            block_type: 1,
            number: 1,
            flags: 0,
            crc: CrcType::Crc32C,
            data: (0..len).map(|i| (i * 7 % 256) as u8).collect(),
        }],
    }
}

fn network() -> Network {
    let contacts = vec![
        Contact::new(1, 2, 0, 1_000_000, 10_000, 10).expect("contact"),
        Contact::new(2, 1, 0, 1_000_000, 10_000, 10).expect("contact"),
    ];
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 13);
    net.set_routing(true);
    net.add_node(1, config()).expect("node");
    net.add_node(2, config()).expect("node");
    net
}

#[test]
fn a_fragmented_bundle_arrives_in_pieces_and_reassembles_to_the_original() {
    let original = big_bundle(3_000);
    let encoded = original.encode().expect("encode");
    let mut net = network();
    let sent = net.send_fragmented(1, &encoded, 500).expect("send");
    assert_eq!(sent, 6, "3000 bytes in pieces of 500");
    net.run_until(3_600_000);
    let pieces: Vec<Vec<u8>> = net.inbox(2).iter().map(|(_, p)| p.clone()).collect();
    assert_eq!(pieces.len(), 6, "every fragment arrives once");
    let whole = reassemble_encoded(&pieces).expect("reassembles");
    assert_eq!(whole, original);
}

#[test]
fn an_unfragmented_send_needs_no_reassembly() {
    let original = big_bundle(100);
    let encoded = original.encode().expect("encode");
    let mut net = network();
    assert_eq!(net.send_fragmented(1, &encoded, 500).expect("send"), 1);
    net.run_until(3_600_000);
    assert_eq!(net.inbox(2).len(), 1);
}

/// Two windows from node 1 to node 2, each carrying 20 kB (1 kB/s for 20 s), starting at 0 s and 25 s.
/// As `two_windows`, with the link segments set to `mtu` bytes. The volume charge of the simulator assumes
/// segments of about 1400 bytes (3% overhead); at 200 bytes a window carries less than its charge, and a
/// fragment's tail is retransmitted in the next window.
fn two_windows_with_mtu(mtu: usize) -> Network {
    let mut contacts = Vec::new();
    for (start, end) in [(0, 20_000), (25_000, 45_000)] {
        contacts.push(Contact::new(1, 2, start, end, 1_000, 10).expect("contact"));
        contacts.push(Contact::new(2, 1, start, end, 1_000, 10).expect("contact"));
    }
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 17);
    net.set_routing(true);
    net.add_node(1, Config { mtu, ..config() }).expect("node");
    net.add_node(2, Config { mtu, ..config() }).expect("node");
    net
}

#[test]
fn a_bundle_that_fits_the_free_volume_is_sent_whole() {
    let original = big_bundle(5_000);
    let encoded = original.encode().expect("encode");
    let mut net = two_windows_with_mtu(1400);
    assert_eq!(net.send_fitting(1, &encoded).expect("send"), 1);
    net.run_until(200_000);
    assert_eq!(net.inbox(2).len(), 1);
}

#[test]
fn the_hop_limit_counts_each_fragment_on_its_own() {
    // Fragments share a bundle identity. With the counter kept per identity, six fragments relayed over
    // two hops would exhaust a limit of three; counted per fragment, each is handled twice and arrives.
    let contacts = vec![
        Contact::new(1, 2, 0, 1_000_000, 10_000, 10).expect("contact"),
        Contact::new(2, 1, 0, 1_000_000, 10_000, 10).expect("contact"),
        Contact::new(2, 3, 0, 1_000_000, 10_000, 10).expect("contact"),
        Contact::new(3, 2, 0, 1_000_000, 10_000, 10).expect("contact"),
    ];
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 19);
    net.set_routing(true);
    net.set_max_hops(3);
    for id in [1, 2, 3] {
        net.add_node(id, config()).expect("node");
    }
    let original = Bundle {
        primary: PrimaryBlock {
            destination: Eid::Ipn {
                node: 3,
                service: 1,
            },
            ..big_bundle(3_000).primary
        },
        blocks: big_bundle(3_000).blocks,
    };
    let encoded = original.encode().expect("encode");
    let sent = net.send_fragmented(1, &encoded, 500).expect("send");
    assert_eq!(sent, 6);
    net.run_until(3_600_000);
    assert_eq!(
        net.inbox(3).len(),
        6,
        "every fragment arrives over two hops"
    );
}

/// Two 4 s windows from node 1 to node 2 at 10 kB/s (40 kB each), starting at 0 s and 10 s. At this rate
/// LTP's checkpoint round trips are short against a window, so a fragment that the volume charge admits
/// is carried within it.
fn fast_two_windows(mtu: usize) -> Network {
    let mut contacts = Vec::new();
    for (start, end) in [(0, 4_000), (10_000, 14_000)] {
        contacts.push(Contact::new(1, 2, start, end, 10_000, 10).expect("contact"));
        contacts.push(Contact::new(2, 1, start, end, 10_000, 10).expect("contact"));
    }
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 17);
    net.set_routing(true);
    net.add_node(1, Config { mtu, ..config() }).expect("node");
    net.add_node(2, Config { mtu, ..config() }).expect("node");
    net
}

#[test]
fn a_bundle_too_big_for_the_first_window_is_fragmented_to_fit_and_completes_in_the_second() {
    let original = big_bundle(50_000);
    let encoded = original.encode().expect("encode");
    let mut net = fast_two_windows(1400);
    let sent = net.send_fitting(1, &encoded).expect("send");
    assert!(sent >= 2, "the bundle must be split: {sent} fragment(s)");
    net.run_until(120_000);
    let pieces: Vec<Vec<u8>> = net.inbox(2).iter().map(|(_, p)| p.clone()).collect();
    assert_eq!(pieces.len(), sent, "every fragment arrives once");
    assert_eq!(reassemble_encoded(&pieces).expect("reassembles"), original);
}
