// SPDX-License-Identifier: MIT OR Apache-2.0
//! Causal broadcast across a partition. Copies that cross the cut are held, as on a reliable link with
//! delay, and delivered after the heal. During the partition each side delivers its own operations and
//! nothing from the other side, so it stays available. After the heal every node delivers every
//! operation exactly once, in causal order.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_causal_consensus::causal::{CausalBroadcast, Envelope};

const NODES: usize = 4;
/// Node 0 and 1 form one side of the cut, nodes 2 and 3 the other.
const SIDE_A: [usize; 2] = [0, 1];

struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }
}

/// An operation records its origin as `origin * 1000 + sequence`, so the origin can be read back.
fn origin_of(op: u32) -> usize {
    (op / 1000) as usize
}

fn side_of(node: usize) -> bool {
    SIDE_A.contains(&node)
}

struct Group {
    nodes: Vec<CausalBroadcast<u32, NODES>>,
    /// Copies in flight on the same side of the cut as their destination.
    queue: Vec<(usize, Envelope<u32, NODES>)>,
    /// Copies that cross the cut, held until it heals.
    held: Vec<(usize, Envelope<u32, NODES>)>,
    rng: Lcg,
    sequence: Vec<u32>,
}

impl Group {
    fn new(seed: u64) -> Self {
        Self {
            nodes: (0..NODES)
                .map(|i| CausalBroadcast::new(i).expect("node"))
                .collect(),
            queue: Vec::new(),
            held: Vec::new(),
            rng: Lcg(seed),
            sequence: vec![0; NODES],
        }
    }

    fn broadcast(&mut self, from: usize) {
        self.sequence[from] += 1;
        let op = from as u32 * 1000 + self.sequence[from];
        let envelope = self.nodes[from].broadcast(op).expect("broadcast");
        for dest in (0..NODES).filter(|&d| d != from) {
            if side_of(dest) == side_of(from) {
                self.queue.push((dest, envelope.clone()));
            } else {
                self.held.push((dest, envelope.clone()));
            }
        }
    }

    /// Delivers one queued copy chosen at random, if any.
    fn deliver_one(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        let index = self.rng.below(self.queue.len() as u64) as usize;
        let (dest, envelope) = self.queue.swap_remove(index);
        self.nodes[dest].receive(envelope);
    }

    fn drain(&mut self) {
        while !self.queue.is_empty() {
            self.deliver_one();
        }
    }
}

#[test]
fn each_side_stays_available_and_only_delivers_its_own_side_during_the_cut() {
    for seed in 1..=30u64 {
        let mut group = Group::new(seed);
        for _ in 0..300 {
            if group.rng.below(3) == 0 {
                let from = group.rng.below(NODES as u64) as usize;
                group.broadcast(from);
            } else {
                group.deliver_one();
            }
        }
        group.drain();
        // Before the heal, every node's log holds only operations from its own side.
        for (node, log) in group.nodes.iter().enumerate() {
            for op in log.log() {
                assert_eq!(
                    side_of(origin_of(*op)),
                    side_of(node),
                    "seed {seed}: node {node} delivered {op} across the cut"
                );
            }
        }
        // Each side made progress on its own.
        for side in [true, false] {
            let nodes: Vec<usize> = (0..NODES).filter(|&n| side_of(n) == side).collect();
            let progressed = nodes.iter().any(|&n| group.nodes[n].log().next().is_some());
            assert!(
                progressed,
                "seed {seed}: a side delivered nothing during the cut"
            );
        }
    }
}

#[test]
fn after_the_heal_every_node_delivers_every_operation_once_in_causal_order() {
    for seed in 1..=30u64 {
        let mut group = Group::new(seed);
        for _ in 0..300 {
            if group.rng.below(3) == 0 {
                let from = group.rng.below(NODES as u64) as usize;
                group.broadcast(from);
            } else {
                group.deliver_one();
            }
        }
        group.drain();
        // Heal: the held copies arrive, in random order.
        let mut held = std::mem::take(&mut group.held);
        while !held.is_empty() {
            let index = group.rng.below(held.len() as u64) as usize;
            let (dest, envelope) = held.swap_remove(index);
            group.nodes[dest].receive(envelope);
        }
        group.drain();

        let total: usize = group.sequence.iter().map(|&s| s as usize).sum();
        for (node, log) in group.nodes.iter().enumerate() {
            assert_eq!(log.waiting(), 0, "seed {seed}: node {node} still waits");
            assert!(
                log.is_causally_ordered(),
                "seed {seed}: node {node} out of causal order"
            );
            let mut delivered: Vec<u32> = log.log().copied().collect();
            assert_eq!(delivered.len(), total, "seed {seed}: node {node} count");
            delivered.sort_unstable();
            delivered.dedup();
            assert_eq!(
                delivered.len(),
                total,
                "seed {seed}: node {node} delivered an op twice"
            );
        }
    }
}
