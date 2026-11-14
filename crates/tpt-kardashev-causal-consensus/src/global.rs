// SPDX-License-Identifier: MIT OR Apache-2.0
//! `GlobalAgreed` operations: a proposal commits when a majority of the federation's domains accept
//! it, and each domain accepts at most one operation per key.
//!
//! The safety argument is a quorum argument. Two majorities of the same set of domains always share
//! a domain, and that domain accepts at most one operation for the key. So two different operations
//! for one key cannot both commit. Each domain also seals the proposals it accepted into a
//! hash-chained epoch summary (see [`crate::federation`]), so the record can be checked afterwards.
//!
//! Scope: this is a single round of accept-or-reject, which assumes no domain crashes and loses no
//! state. Recovery after a crash, and the Byzantine case, are not covered.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use crate::federation::SummaryChain;
use crate::transport::NodeId;

/// A proposed operation on a key, with an identifier that is unique across the federation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal<Op> {
    /// The identifier, unique across the federation.
    pub id: u64,
    /// The key the operation is about.
    pub key: u32,
    /// The operation.
    pub op: Op,
}

/// A message of the global protocol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlobalMessage<Op> {
    /// Asks a domain to accept a proposal.
    Propose(Proposal<Op>),
    /// A domain's answer to a proposal.
    Reply {
        /// The proposal answered.
        id: u64,
        /// Whether the domain accepted it.
        accepted: bool,
    },
}

/// One domain of the federation: the operations it has accepted, and its record.
#[derive(Clone, Debug)]
pub struct Domain<Op> {
    id: NodeId,
    domains: usize,
    accepted_for_key: BTreeMap<u32, Op>,
    accepted_ids: Vec<u64>,
    chain: SummaryChain,
}

impl<Op: Clone + Eq> Domain<Op> {
    /// A domain `id` in a federation of `domains` domains.
    pub fn new(id: NodeId, domains: usize) -> Self {
        Self {
            id,
            domains,
            accepted_for_key: BTreeMap::new(),
            accepted_ids: Vec::new(),
            chain: SummaryChain::new(id),
        }
    }

    /// The domain's id.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// How many domains make a majority of the federation.
    pub fn majority(&self) -> usize {
        self.domains / 2 + 1
    }

    /// The operation this domain has accepted for `key`, if any.
    pub fn accepted(&self, key: u32) -> Option<&Op> {
        self.accepted_for_key.get(&key)
    }

    /// The identifiers of the proposals accepted, in acceptance order.
    pub fn accepted_ids(&self) -> &[u64] {
        &self.accepted_ids
    }

    /// The domain's summary chain, which records the accepted proposals epoch by epoch.
    pub fn chain(&self) -> &SummaryChain {
        &self.chain
    }

    /// Accepts or rejects a proposal, and returns the reply. A first operation for a key is
    /// accepted; the same operation again is accepted; a different one for the same key is rejected.
    pub fn propose(&mut self, proposal: &Proposal<Op>) -> GlobalMessage<Op> {
        let accepted = match self.accepted_for_key.get(&proposal.key) {
            None => {
                self.accepted_for_key
                    .insert(proposal.key, proposal.op.clone());
                true
            }
            Some(existing) => *existing == proposal.op,
        };
        if accepted && !self.accepted_ids.contains(&proposal.id) {
            self.accepted_ids.push(proposal.id);
        }
        GlobalMessage::Reply {
            id: proposal.id,
            accepted,
        }
    }

    /// Seals the accepted proposals so far into the next epoch of the chain. The commands in an
    /// epoch are the proposal identifiers, truncated to 32 bits.
    pub fn seal_epoch(&mut self, from: usize) -> usize {
        let ids: Vec<u32> = self.accepted_ids[from..]
            .iter()
            .map(|&id| id as u32)
            .collect();
        self.chain.seal(&ids);
        self.accepted_ids.len()
    }
}

/// The outcome of a proposal as its proposer sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// When the proposal was started.
    pub started_ms: u64,
    /// When a majority of acceptances arrived, if they did.
    pub committed_ms: Option<u64>,
    /// The domains that accepted.
    pub accepted_by: BTreeSet<NodeId>,
    /// The domains that rejected.
    pub rejected_by: BTreeSet<NodeId>,
}

/// Tracks the proposals one proposer has made and whether each has committed.
#[derive(Clone, Debug, Default)]
pub struct Proposer {
    outcomes: BTreeMap<u64, Outcome>,
}

impl Proposer {
    /// A proposer with no proposals.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that proposal `id` was started at `now_ms`.
    pub fn started(&mut self, id: u64, now_ms: u64) {
        self.outcomes.insert(
            id,
            Outcome {
                started_ms: now_ms,
                committed_ms: None,
                accepted_by: BTreeSet::new(),
                rejected_by: BTreeSet::new(),
            },
        );
    }

    /// Records a reply from `domain`, and commits the proposal at `now_ms` when a majority of the
    /// `majority` domains have accepted it. Returns the proposal's outcome.
    pub fn reply(
        &mut self,
        id: u64,
        domain: NodeId,
        accepted: bool,
        now_ms: u64,
        majority: usize,
    ) -> Option<&Outcome> {
        let outcome = self.outcomes.get_mut(&id)?;
        if accepted {
            outcome.accepted_by.insert(domain);
        } else {
            outcome.rejected_by.insert(domain);
        }
        if outcome.committed_ms.is_none() && outcome.accepted_by.len() >= majority {
            outcome.committed_ms = Some(now_ms);
        }
        self.outcomes.get(&id)
    }

    /// The outcome of proposal `id`, if it was started here.
    pub fn outcome(&self, id: u64) -> Option<&Outcome> {
        self.outcomes.get(&id)
    }
}

impl Proposer {
    /// The proposals this proposer has seen committed, with their outcomes.
    pub fn committed_outcomes(&self) -> impl Iterator<Item = (&u64, &Outcome)> {
        self.outcomes
            .iter()
            .filter(|(_, outcome)| outcome.committed_ms.is_some())
    }
}
