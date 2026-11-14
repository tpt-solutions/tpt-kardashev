// SPDX-License-Identifier: MIT OR Apache-2.0
//! `GlobalAgreed` operations over a simulated federation: the latency bound, the conflict invariant
//! under random delays, a minority that cannot commit while the majority does, and sealed records.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use tpt_kardashev_causal_consensus::federation::verify_chain;
use tpt_kardashev_causal_consensus::global::{Domain, GlobalMessage, Proposal, Proposer};
use tpt_kardashev_causal_consensus::transport::{NodeId, SimConfig, SimNetwork, Transport};
use tpt_kardashev_causal_consensus::{ConsistencyLevel, Delays};

const STEP_MS: u64 = 5;

struct Federation {
    domains: BTreeMap<NodeId, Domain<u32>>,
    proposers: BTreeMap<NodeId, Proposer>,
    net: SimNetwork<GlobalMessage<u32>>,
    now: u64,
    sealed_upto: BTreeMap<NodeId, usize>,
}

impl Federation {
    fn new(size: u64, config: SimConfig) -> Self {
        let size_usize = size as usize;
        Self {
            domains: (1..=size)
                .map(|id| (id, Domain::new(id, size_usize)))
                .collect(),
            proposers: (1..=size).map(|id| (id, Proposer::new())).collect(),
            net: SimNetwork::new(config).expect("config"),
            now: 0,
            sealed_upto: (1..=size).map(|id| (id, 0)).collect(),
        }
    }

    fn majority(&self) -> usize {
        self.domains.len() / 2 + 1
    }

    /// Starts a proposal from `origin`: it accepts locally at once, and asks every other domain.
    fn propose(&mut self, origin: NodeId, id: u64, key: u32, op: u32) {
        let proposal = Proposal { id, key, op };
        self.proposers
            .get_mut(&origin)
            .expect("proposer")
            .started(id, self.now);
        let local = self
            .domains
            .get_mut(&origin)
            .expect("domain")
            .propose(&proposal);
        if let GlobalMessage::Reply { id, accepted } = local {
            let majority = self.majority();
            let now = self.now;
            self.proposers
                .get_mut(&origin)
                .expect("proposer")
                .reply(id, origin, accepted, now, majority);
        }
        let others: Vec<NodeId> = self
            .domains
            .keys()
            .copied()
            .filter(|&d| d != origin)
            .collect();
        for to in others {
            self.net
                .send(
                    self.now,
                    origin,
                    to,
                    GlobalMessage::Propose(proposal.clone()),
                )
                .expect("send");
        }
    }

    fn step(&mut self) {
        self.now += STEP_MS;
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
                    let majority = self.majority();
                    let now = self.now;
                    self.proposers
                        .get_mut(&d.to)
                        .expect("proposer")
                        .reply(id, d.from, accepted, now, majority);
                }
            }
        }
    }

    fn run_for(&mut self, ms: u64) {
        let target = self.now + ms;
        while self.now < target {
            self.step();
        }
    }

    /// Every committed proposal, across all proposers, with its key and operation.
    fn committed(&self, keys: &BTreeMap<u64, (u32, u32)>) -> Vec<(u64, u32, u32)> {
        let mut out = Vec::new();
        for proposer in self.proposers.values() {
            for (&id, _) in proposer.committed_outcomes() {
                let (key, op) = keys[&id];
                out.push((id, key, op));
            }
        }
        out.sort();
        out
    }
}

fn config(seed: u64, jitter: u64) -> SimConfig {
    SimConfig {
        base_delay_ms: 40,
        jitter_ms: jitter,
        loss_per_mille: 0,
        duplicate_per_mille: 0,
        seed,
    }
}

#[test]
fn a_global_commit_takes_no_less_than_the_round_trip_bound() {
    let mut fed = Federation::new(5, config(1, 0));
    fed.propose(1, 1, 7, 42);
    fed.run_for(1_000);
    let outcome = fed.proposers[&1].outcome(1).expect("started").clone();
    let committed = outcome.committed_ms.expect("committed");
    let latency = committed - outcome.started_ms;
    let bound = ConsistencyLevel::GlobalAgreed.latency_lower_bound_ms(Delays {
        intra_domain_ms: 0,
        cross_domain_ms: 40,
    });
    assert!(
        latency >= bound,
        "latency {latency} below the bound {bound}"
    );
    // Five domains, a majority of three: the proposer's own accept plus two replies, one round trip.
    assert_eq!(latency, 2 * 40, "one round trip with the fixed delay");
}

#[test]
fn conflicting_proposals_for_one_key_never_both_commit() {
    let mut total_committed = 0usize;
    let mut single_winner_runs = 0usize;
    for seed in 1..=40u64 {
        let mut fed = Federation::new(5, config(seed, 25));
        let mut keys = BTreeMap::new();
        // Two proposers race for the same key with different values, from a seeded offset.
        let offset = seed % 7;
        fed.propose(1, 1, 9, 100);
        keys.insert(1, (9, 100));
        fed.run_for(offset);
        fed.propose(4, 2, 9, 200);
        keys.insert(2, (9, 200));
        fed.run_for(2_000);
        let committed = fed.committed(&keys);
        let values: std::collections::BTreeSet<u32> = committed
            .iter()
            .filter(|(_, key, _)| *key == 9)
            .map(|(_, _, op)| *op)
            .collect();
        total_committed += committed.len();
        if !values.is_empty() {
            single_winner_runs += 1;
        }
        assert!(
            values.len() <= 1,
            "seed {seed}: two values committed for key 9: {committed:?}"
        );
    }
    assert!(
        total_committed > 0,
        "no proposal ever committed: the invariant is untested"
    );
    assert!(
        single_winner_runs > 20,
        "winners in only {single_winner_runs} of 40 runs"
    );
}

#[test]
fn proposals_for_different_keys_both_commit() {
    let mut fed = Federation::new(5, config(3, 10));
    let mut keys = BTreeMap::new();
    fed.propose(1, 1, 1, 11);
    fed.propose(2, 2, 2, 22);
    keys.insert(1, (1, 11));
    keys.insert(2, (2, 22));
    fed.run_for(1_000);
    assert_eq!(fed.committed(&keys).len(), 2);
}

#[test]
fn a_minority_cannot_commit_while_the_majority_can() {
    let mut fed = Federation::new(5, config(4, 0));
    // Domains 1 and 2 are cut off from 3, 4 and 5 for 10 s.
    fed.net
        .partition_sets(&[1, 2], &[3, 4, 5], 0, 10_000)
        .expect("window");
    fed.propose(1, 1, 5, 500);
    fed.propose(3, 2, 6, 600);
    fed.run_for(5_000);
    assert!(
        fed.proposers[&1]
            .outcome(1)
            .expect("started")
            .committed_ms
            .is_none(),
        "two of five cannot form a majority"
    );
    assert!(
        fed.proposers[&3]
            .outcome(2)
            .expect("started")
            .committed_ms
            .is_some(),
        "three of five can"
    );
    // After the heal, the minority's proposal for key 6 conflicts with the committed value and
    // cannot commit.
    fed.run_for(10_000);
    assert!(
        fed.proposers[&1]
            .outcome(1)
            .expect("started")
            .committed_ms
            .is_none()
    );
}

#[test]
fn every_domain_seals_a_chain_that_verifies_and_names_what_it_accepted() {
    let mut fed = Federation::new(3, config(6, 5));
    fed.propose(1, 10, 1, 1);
    fed.propose(2, 11, 2, 2);
    fed.propose(3, 12, 3, 3);
    fed.run_for(1_000);
    for (&id, domain) in fed.domains.iter_mut() {
        let from = fed.sealed_upto[&id];
        let upto = domain.seal_epoch(from);
        fed.sealed_upto.insert(id, upto);
        let summaries = domain.chain().summaries().to_vec();
        assert!(verify_chain(id, &summaries).is_ok(), "domain {id}");
        assert_eq!(
            domain.accepted_ids().len(),
            3,
            "domain {id} accepted all three"
        );
    }
}
