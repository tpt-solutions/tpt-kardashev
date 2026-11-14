// SPDX-License-Identifier: MIT OR Apache-2.0
//! Domain consensus: Raft-style leader election and log replication within one domain (a lunar
//! base, a habitat ring), written from the description in Ongaro and Ousterhout (2014).
//!
//! A [`RaftNode`] is a pure state machine. `tick` and `handle` return the messages to send; the
//! caller moves them, with whatever transport it has, and passes time in. Nothing here reads a
//! clock or a random source the caller does not supply, so runs are reproducible.
//!
//! What this gives: at most one leader per term, and a command is committed only once a majority
//! of the domain holds it in the leader's term. What it does not give: membership change, log
//! compaction, or any guarantee under Byzantine faults (deferred by the spec).

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec;
use alloc::vec::Vec;

use crate::transport::NodeId;

/// Shortest election timeout, in milliseconds. Each node adds a seeded random spread to it.
pub const ELECTION_MIN_MS: u64 = 150;
/// Spread of the random part of the election timeout, in milliseconds.
pub const ELECTION_SPREAD_MS: u64 = 150;
/// Interval between leader heartbeats, in milliseconds.
pub const HEARTBEAT_MS: u64 = 50;

/// A node's role in the current term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Follows the leader of the current term.
    Follower,
    /// Standing for election in the current term.
    Candidate,
    /// Leads the current term.
    Leader,
}

/// One entry of the replicated log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry<C> {
    /// The term in which the leader appended the entry.
    pub term: u64,
    /// The command.
    pub command: C,
}

/// A message between domain nodes. The sender is carried by the transport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message<C> {
    /// A candidate asks for a vote. `last_index` and `last_term` describe its log.
    RequestVote {
        /// The candidate's term.
        term: u64,
        /// Index of the candidate's last log entry (0 when empty).
        last_index: u64,
        /// Term of the candidate's last log entry (0 when empty).
        last_term: u64,
    },
    /// The answer to a vote request.
    VoteReply {
        /// The voter's term, so a stale candidate can step down.
        term: u64,
        /// Whether the vote was granted.
        granted: bool,
    },
    /// A leader replicates entries after `prev_index`, and tells followers how far to commit.
    AppendEntries {
        /// The leader's term.
        term: u64,
        /// Index of the entry just before the new ones.
        prev_index: u64,
        /// Term of that entry.
        prev_term: u64,
        /// The entries to append.
        entries: Vec<Entry<C>>,
        /// The leader's commit index.
        leader_commit: u64,
    },
    /// The answer to an append.
    AppendReply {
        /// The follower's term.
        term: u64,
        /// Whether the entries matched and were appended.
        success: bool,
        /// On success, the index of the follower's last entry that matches the leader's log.
        match_index: u64,
    },
}

/// A message to send: the destination and the message.
pub type Outbound<C> = (NodeId, Message<C>);

/// An error of the domain layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomainError {
    /// The command was offered to a node that is not the leader. `leader` is the node this node
    /// last heard from as leader, if any.
    NotLeader {
        /// The leader this node knows of.
        leader: Option<NodeId>,
    },
}

impl core::fmt::Display for DomainError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DomainError::NotLeader { leader } => match leader {
                Some(id) => write!(f, "not the leader; the leader is node {id}"),
                None => f.write_str("not the leader, and no leader is known"),
            },
        }
    }
}

/// One node of a domain.
#[derive(Clone, Debug)]
pub struct RaftNode<C> {
    id: NodeId,
    peers: Vec<NodeId>,
    role: Role,
    term: u64,
    voted_for: Option<NodeId>,
    log: Vec<Entry<C>>,
    commit: u64,
    leader: Option<NodeId>,
    votes: BTreeSet<NodeId>,
    next: BTreeMap<NodeId, u64>,
    matched: BTreeMap<NodeId, u64>,
    deadline: u64,
    heartbeat_due: u64,
    rng: u64,
}

impl<C: Clone> RaftNode<C> {
    /// A node `id` of a domain whose other members are `peers`. `seed` drives the random part of
    /// the election timeout; `now` is the time the node starts at.
    pub fn new(id: NodeId, peers: Vec<NodeId>, seed: u64, now: u64) -> Self {
        let mut node = Self {
            id,
            peers,
            role: Role::Follower,
            term: 0,
            voted_for: None,
            log: Vec::new(),
            commit: 0,
            leader: None,
            votes: BTreeSet::new(),
            next: BTreeMap::new(),
            matched: BTreeMap::new(),
            deadline: 0,
            heartbeat_due: 0,
            rng: (seed ^ id.wrapping_mul(0x9E37_79B9_7F4A_7C15)) | 1,
        };
        node.reset_deadline(now);
        node
    }

    /// This node's id.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// The current role.
    pub fn role(&self) -> Role {
        self.role
    }

    /// The current term.
    pub fn term(&self) -> u64 {
        self.term
    }

    /// The leader this node knows of in the current term.
    pub fn known_leader(&self) -> Option<NodeId> {
        self.leader
    }

    /// The commit index: the number of committed entries.
    pub fn commit_index(&self) -> u64 {
        self.commit
    }

    /// The number of entries in the log, committed or not.
    pub fn log_len(&self) -> u64 {
        self.log.len() as u64
    }

    /// The entry at `index` (1-based), if present.
    pub fn entry(&self, index: u64) -> Option<&Entry<C>> {
        if index == 0 {
            return None;
        }
        self.log.get((index - 1) as usize)
    }

    /// The committed commands, in order.
    pub fn committed(&self) -> Vec<C> {
        self.log
            .iter()
            .take(self.commit as usize)
            .map(|e| e.command.clone())
            .collect()
    }

    fn majority(&self) -> usize {
        (self.peers.len() + 1) / 2 + 1
    }

    fn next_random(&mut self) -> u64 {
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.rng >> 33
    }

    fn reset_deadline(&mut self, now: u64) {
        let spread = self.next_random() % ELECTION_SPREAD_MS;
        self.deadline = now + ELECTION_MIN_MS + spread;
    }

    fn last_index(&self) -> u64 {
        self.log.len() as u64
    }

    fn term_at(&self, index: u64) -> Option<u64> {
        if index == 0 {
            Some(0)
        } else {
            self.log.get((index - 1) as usize).map(|e| e.term)
        }
    }

    fn last_term(&self) -> u64 {
        self.term_at(self.last_index()).unwrap_or(0)
    }

    /// Advances the node to time `now`: starts an election when the timeout has passed, or sends
    /// the leader's heartbeats when they are due.
    pub fn tick(&mut self, now: u64) -> Vec<Outbound<C>> {
        match self.role {
            Role::Leader => {
                if now >= self.heartbeat_due {
                    self.heartbeat_due = now + HEARTBEAT_MS;
                    self.append_to_all()
                } else {
                    Vec::new()
                }
            }
            Role::Follower | Role::Candidate => {
                if now >= self.deadline {
                    self.start_election(now)
                } else {
                    Vec::new()
                }
            }
        }
    }

    fn start_election(&mut self, now: u64) -> Vec<Outbound<C>> {
        self.term += 1;
        self.role = Role::Candidate;
        self.voted_for = Some(self.id);
        self.leader = None;
        self.votes.clear();
        self.votes.insert(self.id);
        self.reset_deadline(now);
        if self.votes.len() >= self.majority() {
            let mut out = Vec::new();
            self.become_leader(now, &mut out);
            return out;
        }
        let request = Message::RequestVote {
            term: self.term,
            last_index: self.last_index(),
            last_term: self.last_term(),
        };
        self.peers.iter().map(|&p| (p, request.clone())).collect()
    }

    fn become_leader(&mut self, now: u64, out: &mut Vec<Outbound<C>>) {
        self.role = Role::Leader;
        self.leader = Some(self.id);
        self.next.clear();
        self.matched.clear();
        let next = self.last_index() + 1;
        for &p in &self.peers {
            self.next.insert(p, next);
            self.matched.insert(p, 0);
        }
        self.heartbeat_due = now + HEARTBEAT_MS;
        out.extend(self.append_to_all());
    }

    fn step_down(&mut self, term: u64, now: u64) {
        self.term = term;
        self.role = Role::Follower;
        self.voted_for = None;
        self.leader = None;
        self.votes.clear();
        self.reset_deadline(now);
    }

    fn append_for(&self, to: NodeId) -> Message<C> {
        let next = self.next.get(&to).copied().unwrap_or(self.last_index() + 1);
        let prev_index = next.saturating_sub(1);
        let prev_term = self.term_at(prev_index).unwrap_or(0);
        let entries = self.log.iter().skip(prev_index as usize).cloned().collect();
        Message::AppendEntries {
            term: self.term,
            prev_index,
            prev_term,
            entries,
            leader_commit: self.commit,
        }
    }

    fn append_to_all(&self) -> Vec<Outbound<C>> {
        self.peers
            .iter()
            .map(|&p| (p, self.append_for(p)))
            .collect()
    }

    /// Handles a message from `from` at time `now`, and returns the messages to send in reply.
    pub fn handle(&mut self, now: u64, from: NodeId, message: Message<C>) -> Vec<Outbound<C>> {
        match message {
            Message::RequestVote {
                term,
                last_index,
                last_term,
            } => {
                if term > self.term {
                    self.step_down(term, now);
                }
                let up_to_date = last_term > self.last_term()
                    || (last_term == self.last_term() && last_index >= self.last_index());
                let granted = term == self.term
                    && (self.voted_for.is_none() || self.voted_for == Some(from))
                    && up_to_date;
                if granted {
                    self.voted_for = Some(from);
                    self.reset_deadline(now);
                }
                vec![(
                    from,
                    Message::VoteReply {
                        term: self.term,
                        granted,
                    },
                )]
            }
            Message::VoteReply { term, granted } => {
                if term > self.term {
                    self.step_down(term, now);
                    return Vec::new();
                }
                if self.role == Role::Candidate && term == self.term && granted {
                    self.votes.insert(from);
                    if self.votes.len() >= self.majority() {
                        let mut out = Vec::new();
                        self.become_leader(now, &mut out);
                        return out;
                    }
                }
                Vec::new()
            }
            Message::AppendEntries {
                term,
                prev_index,
                prev_term,
                entries,
                leader_commit,
            } => {
                if term < self.term {
                    return vec![(
                        from,
                        Message::AppendReply {
                            term: self.term,
                            success: false,
                            match_index: 0,
                        },
                    )];
                }
                if term > self.term || self.role != Role::Follower {
                    self.step_down(term, now);
                }
                self.leader = Some(from);
                self.reset_deadline(now);
                let matches =
                    prev_index <= self.last_index() && self.term_at(prev_index) == Some(prev_term);
                if !matches {
                    return vec![(
                        from,
                        Message::AppendReply {
                            term: self.term,
                            success: false,
                            match_index: 0,
                        },
                    )];
                }
                let count = entries.len() as u64;
                let mut index = prev_index;
                for entry in entries {
                    index += 1;
                    if index <= self.last_index() {
                        if self.term_at(index) != Some(entry.term) {
                            // A conflicting entry: it and everything after it are replaced.
                            self.log.truncate((index - 1) as usize);
                            self.log.push(entry);
                        }
                    } else {
                        self.log.push(entry);
                    }
                }
                let match_index = prev_index + count;
                if leader_commit > self.commit {
                    self.commit = leader_commit.min(match_index);
                }
                vec![(
                    from,
                    Message::AppendReply {
                        term: self.term,
                        success: true,
                        match_index,
                    },
                )]
            }
            Message::AppendReply {
                term,
                success,
                match_index,
            } => {
                if term > self.term {
                    self.step_down(term, now);
                    return Vec::new();
                }
                if self.role != Role::Leader || term != self.term {
                    return Vec::new();
                }
                if success {
                    let entry = self.matched.entry(from).or_insert(0);
                    *entry = (*entry).max(match_index);
                    self.next.insert(from, match_index + 1);
                    self.advance_commit();
                    Vec::new()
                } else {
                    // Back up one entry and retry at once.
                    let next = self.next.get(&from).copied().unwrap_or(1);
                    self.next.insert(from, next.saturating_sub(1).max(1));
                    vec![(from, self.append_for(from))]
                }
            }
        }
    }

    /// Moves the commit index to the highest index that a majority holds in the leader's term.
    fn advance_commit(&mut self) {
        let mut best = self.commit;
        for n in (self.commit + 1)..=self.last_index() {
            if self.term_at(n) != Some(self.term) {
                continue;
            }
            let holders = 1 + self.matched.values().filter(|&&m| m >= n).count();
            if holders >= self.majority() {
                best = n;
            }
        }
        self.commit = best;
    }

    /// Offers a command to the leader. Returns the index it was appended at, and the replication
    /// messages to send now. A node that is not the leader refuses, and names the leader it knows.
    pub fn propose(&mut self, command: C) -> Result<(u64, Vec<Outbound<C>>), DomainError> {
        if self.role != Role::Leader {
            return Err(DomainError::NotLeader {
                leader: self.leader,
            });
        }
        self.log.push(Entry {
            term: self.term,
            command,
        });
        let index = self.last_index();
        self.advance_commit();
        Ok((index, self.append_to_all()))
    }
}
