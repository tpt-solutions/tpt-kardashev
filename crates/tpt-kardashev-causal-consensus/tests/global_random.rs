// SPDX-License-Identifier: MIT OR Apache-2.0
//! `GlobalAgreed` under random delays, loss, duplication and partitions, with the invariants checked
//! after every step against the domains' own records rather than the proposers' view.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use tpt_kardashev_causal_consensus::federation::verify_chain;
use tpt_kardashev_causal_consensus::global::{Domain, GlobalMessage, Proposal, Proposer};
use tpt_kardashev_causal_consensus::transport::{NodeId, SimConfig, SimNetwork, Transport};

const STEP_MS: u64 = 5;
const RUN_MS: u64 = 4_000;
const SIZE: u64 = 5;

/// A small seeded generator for the schedule, separate from the network's own randomness.
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

struct Run {
    domains: BTreeMap<NodeId, Domain<u32>>,
    proposers: BTreeMap<NodeId, Proposer>,
    net: SimNetwork<GlobalMessage<u32>>,
    now: u64,
    /// The key and operation of every proposal ever started, by identifier.
    proposals: BTreeMap<u64, (u32, u32)>,
}

impl Run {
    fn new(config: SimConfig) -> Self {
        Self {
            domains: (1..=SIZE)
                .map(|id| (id, Domain::new(id, SIZE as usize)))
                .collect(),
            proposers: (1..=SIZE).map(|id| (id, Proposer::new())).collect(),
            net: SimNetwork::new(config).expect("config"),
            now: 0,
            proposals: BTreeMap::new(),
        }
    }

    fn majority(&self) -> usize {
        SIZE as usize / 2 + 1
    }

    fn propose(&mut self, origin: NodeId, id: u64, key: u32, op: u32) {
        let proposal = Proposal { id, key, op };
        self.proposals.insert(id, (key, op));
        let now = self.now;
        let majority = self.majority();
        self.proposers
            .get_mut(&origin)
            .expect("proposer")
            .started(id, now);
        if let GlobalMessage::Reply { id, accepted } = self
            .domains
            .get_mut(&origin)
            .expect("domain")
            .propose(&proposal)
        {
            self.proposers
                .get_mut(&origin)
                .expect("proposer")
                .reply(id, origin, accepted, now, majority);
        }
        for to in 1..=SIZE {
            if to != origin {
                self.net
                    .send(now, origin, to, GlobalMessage::Propose(proposal.clone()))
                    .expect("send");
            }
        }
    }

    fn step(&mut self) {
        self.now += STEP_MS;
        let majority = self.majority();
        for d in self.net.deliver(self.now) {
            match d.message {
                GlobalMessage::Propose(proposal) => {
                    let reply = self
                        .domains
                        .get_mut(&d.to)
                        .expect("domain")
                        .propose(&proposal);
                    self.net.send(self.now, d.to, d.from, reply).expect("send");
                }
                GlobalMessage::Reply { id, accepted } => {
                    let now = self.now;
                    self.proposers
                        .get_mut(&d.to)
                        .expect("proposer")
                        .reply(id, d.from, accepted, now, majority);
                }
            }
        }
    }

    /// Every committed outcome, as (id, key, op, accepting domains).
    fn committed(&self) -> Vec<(u64, u32, u32, BTreeSet<NodeId>)> {
        let mut out = Vec::new();
        for proposer in self.proposers.values() {
            for (&id, outcome) in proposer.committed_outcomes() {
                let (key, op) = self.proposals[&id];
                out.push((id, key, op, outcome.accepted_by.clone()));
            }
        }
        out.sort_by_key(|(id, ..)| *id);
        out
    }

    /// The invariants, checked against what the domains hold.
    fn check(&self, seed: u64) {
        let mut value_by_key: BTreeMap<u32, u32> = BTreeMap::new();
        for (id, key, op, _) in self.committed() {
            if let Some(&existing) = value_by_key.get(&key) {
                assert_eq!(
                    existing, op,
                    "seed {seed}: key {key} committed two values (proposal {id})"
                );
            }
            value_by_key.insert(key, op);
            // The commit is backed by a majority of domains that really hold this operation.
            let holding = self
                .domains
                .values()
                .filter(|d| d.accepted(key) == Some(&op))
                .count();
            assert!(
                holding >= self.majority(),
                "seed {seed}: proposal {id} committed with {holding} domains holding it"
            );
        }
    }
}

fn run_one(seed: u64) -> (usize, usize) {
    let mut rng = Rng(seed);
    let config = SimConfig {
        base_delay_ms: 20,
        jitter_ms: rng.below(60),
        loss_per_mille: rng.below(250) as u32,
        duplicate_per_mille: rng.below(150) as u32,
        seed,
    };
    let mut run = Run::new(config);

    // One random partition window, splitting the domains into two non-empty groups.
    let split = 1 + rng.below(SIZE - 1);
    let start = rng.below(RUN_MS / 2);
    let length = 200 + rng.below(RUN_MS / 2);
    let left: Vec<NodeId> = (1..=split).collect();
    let right: Vec<NodeId> = (split + 1..=SIZE).collect();
    run.net
        .partition_sets(&left, &right, start, start + length)
        .expect("window");

    let mut next_id = 1u64;
    let steps = RUN_MS / STEP_MS;
    for _ in 0..steps {
        if rng.below(100) < 15 {
            let origin = 1 + rng.below(SIZE);
            let key = rng.below(3) as u32;
            let op = rng.below(3) as u32;
            run.propose(origin, next_id, key, op);
            next_id += 1;
        }
        run.step();
        run.check(seed);
    }
    // Let the last messages settle, with no new proposals.
    for _ in 0..200 {
        run.step();
        run.check(seed);
    }

    for (&id, domain) in run.domains.iter_mut() {
        let upto = domain.seal_epoch(0);
        assert!(upto <= domain.accepted_ids().len());
        let summaries = domain.chain().summaries().to_vec();
        assert!(
            verify_chain(id, &summaries).is_ok(),
            "seed {seed}, domain {id}"
        );
    }
    (run.committed().len(), next_id as usize - 1)
}

#[test]
fn invariants_hold_across_random_schedules() {
    let mut committed = 0usize;
    let mut started = 0usize;
    for seed in 1..=60u64 {
        let (c, s) = run_one(seed);
        committed += c;
        started += s;
    }
    assert!(started > 500, "only {started} proposals were started");
    // With three keys and three values most proposals conflict by design, so a quarter committing
    // is the expected rate. The bound only guards against a run where nothing commits.
    assert!(
        committed * 10 > started,
        "only {committed} of {started} proposals committed: too few to test the invariants"
    );
}

#[test]
fn a_seed_replays_to_the_same_outcome() {
    assert_eq!(run_one(17), run_one(17));
}
