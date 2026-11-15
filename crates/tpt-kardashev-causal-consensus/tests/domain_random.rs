// SPDX-License-Identifier: MIT OR Apache-2.0
//! Domain consensus under random loss, duplication, partitions and crashes. Each crashed node keeps its
//! state and is not ticked while down, as the transport's crash documentation describes. The Raft
//! invariants are checked after every step: one leader per term, commit indexes that never fall, and
//! agreement on every committed entry.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use tpt_kardashev_causal_consensus::domain::{Message, Outbound, RaftNode, Role};
use tpt_kardashev_causal_consensus::transport::{NodeId, SimConfig, SimNetwork, Transport};

const STEP_MS: u64 = 5;
const RUN_MS: u64 = 8_000;
const SIZE: u64 = 5;

/// A seeded generator for the schedule, separate from the network's randomness.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

struct Cluster {
    nodes: BTreeMap<NodeId, RaftNode<u32>>,
    net: SimNetwork<Message<u32>>,
    now: u64,
    crashes: Vec<(NodeId, u64, u64)>,
    last_commit: BTreeMap<NodeId, u64>,
}

impl Cluster {
    fn new(config: SimConfig) -> Self {
        let ids: Vec<NodeId> = (1..=SIZE).collect();
        let nodes = ids
            .iter()
            .map(|&id| {
                let peers = ids.iter().copied().filter(|&p| p != id).collect();
                (id, RaftNode::new(id, peers, config.seed, 0))
            })
            .collect();
        Self {
            nodes,
            net: SimNetwork::new(config).expect("config"),
            now: 0,
            crashes: Vec::new(),
            last_commit: ids.iter().map(|&id| (id, 0)).collect(),
        }
    }

    fn is_down(&self, node: NodeId, at: u64) -> bool {
        self.crashes
            .iter()
            .any(|&(n, from, until)| n == node && (from..until).contains(&at))
    }

    fn send_all(&mut self, from: NodeId, out: Vec<Outbound<u32>>) {
        for (to, message) in out {
            self.net.send(self.now, from, to, message).expect("send");
        }
    }

    fn step(&mut self, seed: u64) {
        self.now += STEP_MS;
        let ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for id in ids {
            if self.is_down(id, self.now) {
                continue;
            }
            let out = self.nodes.get_mut(&id).expect("node").tick(self.now);
            self.send_all(id, out);
        }
        for d in self.net.deliver(self.now) {
            if self.is_down(d.to, self.now) {
                continue;
            }
            let out = self
                .nodes
                .get_mut(&d.to)
                .expect("node")
                .handle(self.now, d.from, d.message);
            self.send_all(d.to, out);
        }
        self.check(seed);
    }

    fn check(&mut self, seed: u64) {
        let mut leaders: BTreeMap<u64, Vec<NodeId>> = BTreeMap::new();
        for (&id, node) in &self.nodes {
            if node.role() == Role::Leader {
                leaders.entry(node.term()).or_default().push(id);
            }
        }
        for (term, ids) in &leaders {
            assert!(
                ids.len() <= 1,
                "seed {seed}: term {term} has leaders {ids:?}"
            );
        }
        for (&id, node) in &self.nodes {
            let previous = self.last_commit[&id];
            assert!(
                node.commit_index() >= previous,
                "seed {seed}: node {id}'s commit went backwards"
            );
            self.last_commit.insert(id, node.commit_index());
        }
        let nodes: Vec<&RaftNode<u32>> = self.nodes.values().collect();
        for a in &nodes {
            for b in &nodes {
                let shared = a.commit_index().min(b.commit_index());
                for i in 1..=shared {
                    assert_eq!(
                        a.entry(i).map(|e| (e.term, e.command)),
                        b.entry(i).map(|e| (e.term, e.command)),
                        "seed {seed}: nodes {} and {} disagree at committed index {i}",
                        a.id(),
                        b.id()
                    );
                }
            }
        }
    }

    /// A live leader, if one is up now.
    fn live_leader(&self) -> Option<NodeId> {
        self.nodes
            .values()
            .filter(|n| n.role() == Role::Leader && !self.is_down(n.id(), self.now))
            .max_by_key(|n| n.term())
            .map(|n| n.id())
    }

    fn propose(&mut self, to: NodeId, command: u32) {
        let out = self.nodes.get_mut(&to).expect("node").propose(command);
        if let Ok((_, out)) = out {
            self.send_all(to, out);
        }
    }
}

/// Runs one schedule and returns how many entries committed, how many messages the partitions and
/// crashes dropped, and the highest commit index reached.
fn run_one(seed: u64) -> (u64, usize, u64) {
    let mut rng = Rng(seed);
    let config = SimConfig {
        base_delay_ms: 10,
        jitter_ms: rng.below(25),
        loss_per_mille: rng.below(200) as u32,
        duplicate_per_mille: rng.below(100) as u32,
        seed,
    };
    let mut cluster = Cluster::new(config);
    for _ in 0..(1 + rng.below(3)) {
        let a = 1 + rng.below(SIZE);
        let b = 1 + rng.below(SIZE);
        if a != b {
            let start = rng.below(RUN_MS);
            let length = 300 + rng.below(2_000);
            cluster
                .net
                .partition(a, b, start, start + length)
                .expect("window");
        }
    }
    for _ in 0..rng.below(3) {
        let node = 1 + rng.below(SIZE);
        let from = rng.below(RUN_MS);
        let until = from + 300 + rng.below(2_500);
        cluster.net.crash(node, from, until).expect("crash");
        cluster.crashes.push((node, from, until));
    }

    let mut next_command = 0u32;
    let steps = RUN_MS / STEP_MS;
    for _ in 0..steps {
        let proposing = rng.below(100) < 20;
        if let Some(leader) = cluster.live_leader().filter(|_| proposing) {
            cluster.propose(leader, next_command);
            next_command += 1;
        }
        cluster.step(seed);
    }
    // Settle with the network healed and no new proposals.
    for _ in 0..1_000 {
        cluster.step(seed);
    }

    let committed = cluster
        .nodes
        .values()
        .map(|n| n.commit_index())
        .max()
        .unwrap_or(0);
    (
        committed,
        cluster.net.stats().partitioned as usize,
        next_command as u64,
    )
}

#[test]
fn raft_safety_holds_across_random_loss_partitions_and_crashes() {
    let mut highest = 0u64;
    let mut dropped = 0usize;
    let mut proposed = 0u64;
    for seed in 1..=40u64 {
        let (commit, lost, sent) = run_one(seed);
        highest = highest.max(commit);
        dropped += lost;
        proposed += sent;
    }
    assert!(dropped > 0, "no message was lost to a partition or crash");
    assert!(proposed > 100, "only {proposed} commands were proposed");
    assert!(highest > 0, "nothing committed in any schedule");
}

#[test]
fn a_seed_replays_to_the_same_outcome() {
    assert_eq!(run_one(9), run_one(9));
}
