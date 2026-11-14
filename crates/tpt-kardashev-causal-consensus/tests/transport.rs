// SPDX-License-Identifier: MIT OR Apache-2.0
//! The simulated network: delay, reordering by jitter, loss, duplication, partitions that cut and
//! heal, determinism for a fixed seed, and rejection of invalid configuration.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_causal_consensus::transport::{SimConfig, SimNetwork, Transport, TransportError};

fn config(seed: u64) -> SimConfig {
    SimConfig {
        base_delay_ms: 100,
        jitter_ms: 0,
        loss_per_mille: 0,
        duplicate_per_mille: 0,
        seed,
    }
}

#[test]
fn a_message_arrives_after_the_delay_and_not_before() {
    let mut net = SimNetwork::new(config(1)).expect("config");
    net.send(1_000, 1, 2, "hello").expect("send");
    assert!(net.deliver(1_099).is_empty(), "too early");
    let got = net.deliver(1_100);
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].from, got[0].to, got[0].at_ms), (1, 2, 1_100));
    assert_eq!(got[0].message, "hello");
    assert!(net.deliver(10_000).is_empty(), "delivered once");
}

#[test]
fn jitter_reorders_messages_sent_in_sequence() {
    let mut cfg = config(7);
    cfg.jitter_ms = 500;
    let mut net = SimNetwork::new(cfg).expect("config");
    for i in 0..200u32 {
        net.send(0, 1, 2, i).expect("send");
    }
    let got = net.deliver(10_000);
    assert_eq!(got.len(), 200, "none lost");
    let in_send_order = got.windows(2).all(|w| w[0].message < w[1].message);
    assert!(!in_send_order, "jitter should reorder some messages");
    // Delivery times never go backwards.
    assert!(got.windows(2).all(|w| w[0].at_ms <= w[1].at_ms));
}

#[test]
fn loss_drops_roughly_the_configured_share_and_is_counted() {
    let mut cfg = config(3);
    cfg.loss_per_mille = 300;
    let mut net = SimNetwork::new(cfg).expect("config");
    for i in 0..10_000u32 {
        net.send(0, 1, 2, i).expect("send");
    }
    let delivered = net.deliver(u64::MAX).len() as u64;
    let stats = net.stats();
    assert_eq!(stats.lost + delivered, 10_000);
    assert!(
        (2_500..=3_500).contains(&stats.lost),
        "lost {} of 10 000 at 30 per mille",
        stats.lost
    );
}

#[test]
fn duplication_makes_extra_copies_that_are_counted() {
    let mut cfg = config(5);
    cfg.duplicate_per_mille = 500;
    let mut net = SimNetwork::new(cfg).expect("config");
    for i in 0..2_000u32 {
        net.send(0, 1, 2, i).expect("send");
    }
    let delivered = net.deliver(u64::MAX).len() as u64;
    let stats = net.stats();
    assert_eq!(delivered, 2_000 + stats.duplicated);
    assert!(
        stats.duplicated > 800 && stats.duplicated < 1_200,
        "{}",
        stats.duplicated
    );
}

#[test]
fn a_partition_drops_messages_sent_during_it_and_heals() {
    let mut net = SimNetwork::new(config(1)).expect("config");
    net.partition(1, 2, 1_000, 2_000).expect("window");
    net.send(500, 1, 2, "before").expect("send");
    net.send(1_500, 1, 2, "during").expect("send");
    net.send(1_500, 2, 1, "during, reverse").expect("send");
    net.send(2_000, 1, 2, "after").expect("send");
    let got: Vec<&str> = net
        .deliver(u64::MAX)
        .into_iter()
        .map(|d| d.message)
        .collect();
    assert_eq!(got, vec!["before", "after"]);
    assert_eq!(net.stats().partitioned, 2, "both directions were cut");
}

#[test]
fn a_partition_between_other_nodes_does_not_touch_this_link() {
    let mut net = SimNetwork::new(config(1)).expect("config");
    net.partition(3, 4, 0, 10_000).expect("window");
    net.send(100, 1, 2, "unaffected").expect("send");
    assert_eq!(net.deliver(u64::MAX).len(), 1);
}

#[test]
fn a_fixed_seed_gives_the_same_run() {
    fn run(seed: u64) -> Vec<(u64, u64, u64, u32)> {
        let mut cfg = config(seed);
        cfg.jitter_ms = 200;
        cfg.loss_per_mille = 100;
        cfg.duplicate_per_mille = 100;
        let mut net = SimNetwork::new(cfg).expect("config");
        for i in 0..500u32 {
            net.send(u64::from(i), 1, 2, i).expect("send");
        }
        net.deliver(u64::MAX)
            .into_iter()
            .map(|d| (d.from, d.to, d.at_ms, d.message))
            .collect()
    }
    assert_eq!(run(42), run(42));
    assert_ne!(run(42), run(43));
}

#[test]
fn invalid_configuration_is_rejected() {
    let mut cfg = config(1);
    cfg.loss_per_mille = 1_001;
    assert_eq!(
        SimNetwork::<u32>::new(cfg).err(),
        Some(TransportError::InvalidConfig)
    );
    let mut net = SimNetwork::<u32>::new(config(1)).expect("config");
    assert_eq!(
        net.partition(1, 2, 500, 500),
        Err(TransportError::InvalidConfig)
    );
}
