// SPDX-License-Identifier: MIT OR Apache-2.0
//! Domain consensus on a simulated network: one leader per term, commitment, agreement on committed
//! entries, recovery after a partition heals, and the safety invariants under loss and partitions.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use tpt_kardashev_causal_consensus::domain::{Message, Outbound, RaftNode, Role};
use tpt_kardashev_causal_consensus::transport::{NodeId, SimConfig, SimNetwork, Transport};

const STEP_MS: u64 = 5;

struct Cluster {
    nodes: BTreeMap<NodeId, RaftNode<u32>>,
    net: SimNetwork<Message<u32>>,
    now: u64,
    last_commit: BTreeMap<NodeId, u64>,
}

impl Cluster {
    fn new(size: u64, config: SimConfig) -> Self {
        let ids: Vec<NodeId> = (1..=size).collect();
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
            last_commit: ids.iter().map(|&id| (id, 0)).collect(),
        }
    }

    fn send_all(&mut self, from: NodeId, out: Vec<Outbound<u32>>) {
        for (to, message) in out {
            self.net.send(self.now, from, to, message).expect("send");
        }
    }

    fn step(&mut self) {
        self.now += STEP_MS;
        let ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for id in ids {
            let out = self.nodes.get_mut(&id).expect("node").tick(self.now);
            self.send_all(id, out);
        }
        for d in self.net.deliver(self.now) {
            let out = self
                .nodes
                .get_mut(&d.to)
                .expect("node")
                .handle(self.now, d.from, d.message);
            self.send_all(d.to, out);
        }
        self.check_invariants();
    }

    fn run_for(&mut self, ms: u64) {
        let target = self.now + ms;
        while self.now < target {
            self.step();
        }
    }

    fn check_invariants(&mut self) {
        // At most one leader per term.
        let mut leaders: BTreeMap<u64, Vec<NodeId>> = BTreeMap::new();
        for (&id, node) in &self.nodes {
            if node.role() == Role::Leader {
                leaders.entry(node.term()).or_default().push(id);
            }
        }
        for (term, ids) in &leaders {
            assert!(ids.len() <= 1, "term {term} has leaders {ids:?}");
        }
        // Commit indexes never go backwards, and committed entries agree across nodes.
        for (&id, node) in &self.nodes {
            let previous = self.last_commit[&id];
            assert!(
                node.commit_index() >= previous,
                "node {id}'s commit went backwards"
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
                        "nodes {} and {} disagree at committed index {i}",
                        a.id(),
                        b.id()
                    );
                }
            }
        }
    }

    fn leader(&self) -> Option<NodeId> {
        self.nodes
            .values()
            .filter(|n| n.role() == Role::Leader)
            .max_by_key(|n| n.term())
            .map(|n| n.id())
    }

    fn propose(
        &mut self,
        to: NodeId,
        command: u32,
    ) -> Result<u64, tpt_kardashev_causal_consensus::domain::DomainError> {
        let (index, out) = self.nodes.get_mut(&to).expect("node").propose(command)?;
        self.send_all(to, out);
        Ok(index)
    }

    fn committed(&self, id: NodeId) -> Vec<u32> {
        self.nodes[&id].committed()
    }
}

fn reliable(seed: u64) -> SimConfig {
    SimConfig {
        base_delay_ms: 10,
        jitter_ms: 5,
        loss_per_mille: 0,
        duplicate_per_mille: 0,
        seed,
    }
}

#[test]
fn one_leader_is_elected_and_everyone_follows_it() {
    let mut cluster = Cluster::new(5, reliable(1));
    cluster.run_for(3_000);
    let leader = cluster.leader().expect("a leader after 3 s");
    for node in cluster.nodes.values() {
        if node.id() != leader {
            assert_eq!(node.known_leader(), Some(leader), "node {}", node.id());
        }
    }
}

#[test]
fn commands_commit_on_every_node_in_the_same_order() {
    let mut cluster = Cluster::new(5, reliable(2));
    cluster.run_for(1_000);
    let leader = cluster.leader().expect("leader");
    for command in 0..20u32 {
        cluster
            .propose(leader, command)
            .expect("the leader accepts");
        cluster.run_for(30);
    }
    cluster.run_for(2_000);
    let expected: Vec<u32> = (0..20).collect();
    for id in cluster.nodes.keys().copied().collect::<Vec<_>>() {
        assert_eq!(cluster.committed(id), expected, "node {id}");
    }
}

#[test]
fn a_follower_refuses_a_command_and_names_the_leader() {
    let mut cluster = Cluster::new(3, reliable(4));
    cluster.run_for(1_000);
    let leader = cluster.leader().expect("leader");
    let follower = cluster
        .nodes
        .keys()
        .copied()
        .find(|&id| id != leader)
        .expect("follower");
    let refused = cluster.propose(follower, 1);
    assert_eq!(
        refused,
        Err(
            tpt_kardashev_causal_consensus::domain::DomainError::NotLeader {
                leader: Some(leader)
            }
        )
    );
}

#[test]
fn an_isolated_leader_cannot_commit_and_the_majority_recovers() {
    let mut cluster = Cluster::new(5, reliable(5));
    cluster.run_for(1_500);
    let old = cluster.leader().expect("leader");
    // Cut the old leader off from everyone for 3 s.
    let others: Vec<NodeId> = cluster
        .nodes
        .keys()
        .copied()
        .filter(|&id| id != old)
        .collect();
    for &other in &others {
        cluster
            .net
            .partition(old, other, cluster.now, cluster.now + 3_000)
            .expect("window");
    }
    // The old leader accepts a command it can never commit.
    cluster.propose(old, 999).expect("still believes it leads");
    cluster.run_for(2_500);
    let new = cluster.leader().expect("the majority elects a leader");
    assert_ne!(new, old, "a new leader in a later term");
    cluster.propose(new, 100).expect("new leader");
    cluster.run_for(500);
    // Heal and let the old leader catch up.
    cluster.run_for(3_000);
    let committed = cluster.committed(new);
    assert!(
        committed.contains(&100),
        "the new leader's command is committed"
    );
    assert!(
        !committed.contains(&999),
        "the isolated leader's command is never committed"
    );
    for id in cluster.nodes.keys().copied().collect::<Vec<_>>() {
        assert_eq!(
            cluster.committed(id),
            committed,
            "node {id} agrees after the heal"
        );
    }
}

#[test]
fn safety_holds_under_loss_duplication_and_partitions() {
    for seed in 1..=8u64 {
        let config = SimConfig {
            base_delay_ms: 10,
            jitter_ms: 20,
            loss_per_mille: 100,
            duplicate_per_mille: 50,
            seed,
        };
        let mut cluster = Cluster::new(5, config);
        // Random-looking cuts from the seed, all within the first 8 s.
        for k in 0..6u64 {
            let a = 1 + (seed + k) % 5;
            let b = 1 + (seed * 3 + k * 7) % 5;
            if a != b {
                let start = (seed * 131 + k * 971) % 6_000;
                cluster
                    .net
                    .partition(a, b, start, start + 1_500)
                    .expect("window");
            }
        }
        for step in 0..150u32 {
            cluster.run_for(50);
            if let Some(leader) = cluster.leader() {
                let _ = cluster.propose(leader, step);
            }
        }
        // Safety was checked at every step by `check_invariants`.
        cluster.run_for(3_000);
        assert!(
            cluster.leader().is_some(),
            "seed {seed}: no leader at the end"
        );
        assert!(
            cluster.nodes.values().any(|n| n.commit_index() > 0),
            "seed {seed}: nothing committed despite the healed network"
        );
    }
}
