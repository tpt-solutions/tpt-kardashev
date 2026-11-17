// SPDX-License-Identifier: MIT OR Apache-2.0
//! Two bundles sent at once over one contact: both should arrive. Their LTP sessions share the link, so
//! each one's acknowledgements wait behind the other's data. Found 2026-11-17: this stalled, because the retransmission timeout stayed at its configured value under queueing;
//! fixed by exponential backoff of the timeout (see the todo).
#![allow(clippy::expect_used)]

use tpt_kardashev_dtn::bundle::{CrcType, CreationTimestamp, PrimaryBlock};
use tpt_kardashev_dtn::ltp::session::Config;
use tpt_kardashev_dtn::sim::{Impairments, Network};
use tpt_kardashev_dtn::{Bundle, CanonicalBlock, Contact, ContactPlan, Eid};

fn config() -> Config {
    Config {
        mtu: 1400,
        timeout_ms: 300,
        max_transmissions: 60,
        first_serial: 0x400,
        max_rto_factor: 4,
    }
}

fn bundle_bytes(len: usize, sequence: u64) -> Vec<u8> {
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
            created: CreationTimestamp { time: 1, sequence },
            lifetime: 10_000_000,
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

#[test]
fn two_bundles_in_one_contact_both_arrive() {
    let contacts = vec![
        Contact::new(1, 2, 0, 60_000, 1_000, 10).expect("contact"),
        Contact::new(2, 1, 0, 60_000, 1_000, 10).expect("contact"),
    ];
    let mut net = Network::new(ContactPlan::new(contacts), Impairments::default(), 17);
    net.set_routing(true);
    net.add_node(1, config()).expect("node");
    net.add_node(2, config()).expect("node");
    net.send_routed(1, bundle_bytes(15_000, 1)).expect("send");
    net.send_routed(1, bundle_bytes(15_000, 2)).expect("send");
    net.run_until(200_000);
    assert_eq!(
        net.inbox(2).len(),
        2,
        "both bundles arrive over the shared link"
    );
    assert_eq!(net.held(), 0);
}
