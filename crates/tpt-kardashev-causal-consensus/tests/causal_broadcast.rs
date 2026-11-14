// SPDX-License-Identifier: MIT OR Apache-2.0
//! Causal broadcast: messages are delivered only after their causes, in any network order, with
//! duplicates discarded. A seeded simulation checks this over random sends and deliveries.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_causal_consensus::{CausalBroadcast, Envelope};

const NODES: usize = 4;

/// Deterministic pseudo-random numbers (Knuth's LCG constants).
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[test]
fn a_reply_waits_for_the_message_it_depends_on() {
    // Node 0 sends m1. Node 1 receives it and replies with m2. Node 2 gets the reply first: it must
    // wait, and deliver m1 before m2 once m1 arrives.
    let mut n0 = CausalBroadcast::<&str, 3>::new(0).expect("node");
    let mut n1 = CausalBroadcast::<&str, 3>::new(1).expect("node");
    let mut n2 = CausalBroadcast::<&str, 3>::new(2).expect("node");
    let m1 = n0.broadcast("m1").expect("broadcast");
    n1.receive(m1.clone());
    let m2 = n1.broadcast("m2").expect("broadcast");

    assert_eq!(
        n2.receive(m2.clone()),
        0,
        "the reply is not deliverable yet"
    );
    assert_eq!(n2.waiting(), 1);
    assert_eq!(n2.receive(m1), 2, "the cause unblocks the reply");
    assert_eq!(n2.log().copied().collect::<Vec<_>>(), vec!["m1", "m2"]);
    assert!(n2.is_causally_ordered());
}

#[test]
fn a_duplicate_is_delivered_once() {
    let mut n0 = CausalBroadcast::<u32, 2>::new(0).expect("node");
    let mut n1 = CausalBroadcast::<u32, 2>::new(1).expect("node");
    let m = n0.broadcast(7).expect("broadcast");
    assert_eq!(n1.receive(m.clone()), 1);
    assert_eq!(n1.receive(m), 0);
    assert_eq!(n1.log().copied().collect::<Vec<_>>(), vec![7]);
}

#[test]
fn a_node_rejects_an_index_outside_the_group() {
    assert!(CausalBroadcast::<u32, 3>::new(3).is_err());
}

/// Runs the group: random broadcasts, random deliveries of queued copies in random order, and
/// duplicates. Then every queued copy is delivered. Returns each node's log.
fn simulate(seed: u64, steps: usize) -> (Vec<CausalBroadcast<u32, NODES>>, usize) {
    let mut rng = Lcg(seed);
    let mut nodes: Vec<CausalBroadcast<u32, NODES>> = (0..NODES)
        .map(|i| CausalBroadcast::new(i).expect("node"))
        .collect();
    // Each in-flight copy: (destination, envelope).
    let mut queue: Vec<(usize, Envelope<u32, NODES>)> = Vec::new();
    let mut next_op = 0u32;
    let mut broadcasts = 0usize;
    for _ in 0..steps {
        if rng.below(3) == 0 {
            let from = rng.below(NODES as u64) as usize;
            let envelope = nodes[from].broadcast(next_op).expect("broadcast");
            next_op += 1;
            broadcasts += 1;
            for dest in (0..NODES).filter(|&d| d != from) {
                queue.push((dest, envelope.clone()));
            }
        } else if !queue.is_empty() {
            let index = rng.below(queue.len() as u64) as usize;
            let (dest, envelope) = if rng.below(10) == 0 {
                // Duplicate: deliver a copy and keep the original in flight.
                queue[index].clone()
            } else {
                queue.swap_remove(index)
            };
            nodes[dest].receive(envelope);
        }
    }
    // Drain: every copy still in flight is delivered, in random order.
    while !queue.is_empty() {
        let index = rng.below(queue.len() as u64) as usize;
        let (dest, envelope) = queue.swap_remove(index);
        nodes[dest].receive(envelope);
    }
    assert!(broadcasts > 0, "the run must broadcast something");
    (nodes, broadcasts)
}

#[test]
fn every_node_delivers_every_message_once_and_in_causal_order() {
    for seed in [1u64, 2, 3, 4, 5] {
        let (nodes, broadcasts) = simulate(seed, 3_000);
        // Every node delivers exactly the broadcasts made in the group.
        for (i, node) in nodes.iter().enumerate() {
            assert_eq!(
                node.entries().len(),
                broadcasts,
                "seed {seed}: node {i} count"
            );
        }
        // All nodes deliver the same set of operations, each once.
        let mut sets: Vec<Vec<u32>> = nodes
            .iter()
            .map(|n| {
                let mut ops: Vec<u32> = n.log().copied().collect();
                ops.sort_unstable();
                ops
            })
            .collect();
        let first = sets.remove(0);
        for other in &sets {
            assert_eq!(
                other, &first,
                "seed {seed}: the nodes disagree on the operations"
            );
        }
        let mut deduped = first.clone();
        deduped.dedup();
        assert_eq!(
            deduped, first,
            "seed {seed}: an operation was delivered twice"
        );
        for (i, node) in nodes.iter().enumerate() {
            assert!(
                node.is_causally_ordered(),
                "seed {seed}: node {i} is not causally ordered"
            );
            assert_eq!(
                node.waiting(),
                0,
                "seed {seed}: node {i} still waits for a message"
            );
        }
    }
}

#[test]
fn a_duplicate_of_a_waiting_message_leaves_nothing_behind() {
    // Node 2 receives the reply twice before its cause: the buffer keeps one copy. Once the cause
    // arrives, both copies are settled and nothing is left waiting.
    let mut n0 = CausalBroadcast::<&str, 3>::new(0).expect("node");
    let mut n1 = CausalBroadcast::<&str, 3>::new(1).expect("node");
    let mut n2 = CausalBroadcast::<&str, 3>::new(2).expect("node");
    let m1 = n0.broadcast("m1").expect("broadcast");
    n1.receive(m1.clone());
    let m2 = n1.broadcast("m2").expect("broadcast");
    assert_eq!(n2.receive(m2.clone()), 0);
    assert_eq!(n2.receive(m2), 0);
    assert_eq!(n2.waiting(), 1, "the duplicate was not buffered twice");
    assert_eq!(n2.receive(m1.clone()), 2);
    assert_eq!(n2.waiting(), 0);
    assert_eq!(n2.receive(m1), 0, "a late copy of the cause is discarded");
    assert_eq!(n2.log().copied().collect::<Vec<_>>(), vec!["m1", "m2"]);
}
