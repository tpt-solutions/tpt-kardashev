// SPDX-License-Identifier: MIT OR Apache-2.0
//! A replicated register over domain consensus, checked for linearizability. Writes and reads are both
//! log entries. Each operation's interval runs from its proposal to the first step at which a node's
//! committed log holds it. A read's value is the register's value at its place in the committed log.
//! Under partitions, loss and crashes, the committed history must be linearizable, and the nodes must agree on
//! every committed entry. A weakened commit quorum breaks election safety, which `tests/domain_random.rs`
//! checks; this test does not detect that mutation on its own.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use tpt_kardashev_causal_consensus::domain::{Message, Outbound, RaftNode, Role};
use tpt_kardashev_causal_consensus::linearizable::{Event, Op, is_linearizable};
use tpt_kardashev_causal_consensus::transport::{NodeId, SimConfig, SimNetwork, Transport};

const STEP_MS: u64 = 5;
const RUN_MS: u64 = 12_000;
const SIZE: u64 = 5;
/// The checker's limit is 16 operations, so each run proposes at most this many.
const MAX_OPS: usize = 14;
/// Set on a command that is a read. A write's command is its unique identifier, which is never zero.
const READ_BIT: u32 = 1 << 31;

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

/// One proposal of the run: what it was, where it was put in the log, and when it was invoked and
/// first seen committed.
#[derive(Clone, Debug)]
struct Pending {
    command: u32,
    index: u64,
    invoke: u64,
    committed_at: Option<u64>,
}

struct Cluster {
    nodes: BTreeMap<NodeId, RaftNode<u32>>,
    net: SimNetwork<Message<u32>>,
    now: u64,
    crashes: Vec<(NodeId, u64, u64)>,
    pending: Vec<Pending>,
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
            pending: Vec::new(),
        }
    }

    fn is_down(&self, node: NodeId, at: u64) -> bool {
        self.crashes
            .iter()
            .any(|&(n, from, until)| n == node && (from..until).contains(&at))
    }

    fn live_leader(&self) -> Option<NodeId> {
        self.nodes
            .values()
            .filter(|n| n.role() == Role::Leader && !self.is_down(n.id(), self.now))
            .max_by_key(|n| n.term())
            .map(|n| n.id())
    }

    fn send_all(&mut self, from: NodeId, out: Vec<Outbound<u32>>) {
        for (to, message) in out {
            self.net.send(self.now, from, to, message).expect("send");
        }
    }

    /// Proposes `command` at the live leader, if there is one. Returns whether it was proposed.
    fn propose(&mut self, command: u32) -> bool {
        let Some(leader) = self.live_leader() else {
            return false;
        };
        let Ok((index, out)) = self.nodes.get_mut(&leader).expect("node").propose(command) else {
            return false;
        };
        self.send_all(leader, out);
        self.pending.push(Pending {
            command,
            index,
            invoke: self.now,
            committed_at: None,
        });
        true
    }

    fn step(&mut self) {
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
        self.observe_commits();
    }

    /// Records the first step at which each pending proposal is in some node's committed log.
    fn observe_commits(&mut self) {
        let now = self.now;
        for p in self.pending.iter_mut().filter(|p| p.committed_at.is_none()) {
            let seen = self.nodes.values().any(|n| {
                n.commit_index() >= p.index
                    && n.entry(p.index).map(|e| e.command) == Some(p.command)
            });
            if seen {
                p.committed_at = Some(now);
            }
        }
    }
}

/// Runs one schedule and returns the committed history, or `None` when nothing was proposed.
fn run_one(seed: u64) -> Option<Vec<Event<u32>>> {
    let mut rng = Rng(seed);
    let config = SimConfig {
        base_delay_ms: 10,
        jitter_ms: rng.below(25),
        loss_per_mille: rng.below(150) as u32,
        duplicate_per_mille: rng.below(80) as u32,
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

    let mut next_id = 1u32;
    let steps = RUN_MS / STEP_MS;
    for _ in 0..steps {
        if cluster.pending.len() < MAX_OPS && rng.below(100) < 8 {
            let is_read = rng.below(3) == 0;
            let command = if is_read { next_id | READ_BIT } else { next_id };
            if cluster.propose(command) {
                next_id += 1;
            }
        }
        cluster.step();
    }
    // Settle with the network healed and no new proposals.
    for _ in 0..1_500 {
        cluster.step();
    }
    assert_agreement(seed, &cluster);
    history(&cluster)
}

/// Every node agrees on each committed entry: the register is only well defined if they do.
fn assert_agreement(seed: u64, cluster: &Cluster) {
    let nodes: Vec<&RaftNode<u32>> = cluster.nodes.values().collect();
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

/// The committed operations in log order, as a history for the checker. A write sets the register to
/// its identifier; a read observes the value the register has at its place in the log.
fn history(cluster: &Cluster) -> Option<Vec<Event<u32>>> {
    let best = cluster.nodes.values().max_by_key(|n| n.commit_index())?;
    let mut committed: Vec<(u64, &Pending)> = cluster
        .pending
        .iter()
        .filter(|p| p.index <= best.commit_index())
        .filter(|p| best.entry(p.index).map(|e| e.command) == Some(p.command))
        .map(|p| (p.index, p))
        .collect();
    committed.sort_by_key(|(index, _)| *index);
    if committed.is_empty() {
        return None;
    }
    let mut register = 0u32;
    let mut events = Vec::new();
    for (_, p) in committed {
        let respond = p.committed_at.unwrap_or(cluster.now);
        let op = if p.command & READ_BIT != 0 {
            Op::Read(register)
        } else {
            register = p.command;
            Op::Write(p.command)
        };
        events.push(Event {
            invoke: p.invoke,
            respond: respond.max(p.invoke),
            op,
        });
    }
    Some(events)
}

#[test]
fn committed_histories_of_a_replicated_register_are_linearizable() {
    let mut operations = 0usize;
    let mut reads = 0usize;
    let mut runs = 0usize;
    for seed in 1..=300u64 {
        let Some(events) = run_one(seed) else {
            continue;
        };
        runs += 1;
        operations += events.len();
        reads += events
            .iter()
            .filter(|e| matches!(e.op, Op::Read(_)))
            .count();
        assert_eq!(
            is_linearizable(0, &events),
            Ok(true),
            "seed {seed}: committed history is not linearizable: {events:?}"
        );
    }
    assert!(runs >= 30, "only {runs} runs committed anything");
    assert!(
        reads > 50,
        "only {reads} reads among {operations} operations"
    );
}

#[test]
fn a_seed_replays_to_the_same_history() {
    assert_eq!(run_one(5), run_one(5));
}
