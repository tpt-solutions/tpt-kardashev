// SPDX-License-Identifier: MIT OR Apache-2.0
//! A causally ordered operation log, by causal broadcast (Birman, Schiper and Stephenson, 1991).
//!
//! Each broadcast carries the sender's vector clock at send time. A receiver delivers a message
//! from sender `s` with clock `V` only when it has delivered every message that `V` says the
//! sender had seen: `V[s] = delivered[s] + 1` and `V[k] <= delivered[k]` for every other node `k`.
//! Messages that arrive early wait in a buffer. Every delivered log is therefore a causal order:
//! no operation appears before one it depends on.
//!
//! The delivery rule and the buffer are written from that description; no reference code was
//! consulted. This layer does not reorder for consensus: all nodes deliver the causally ordered
//! messages, but two concurrent messages may be delivered in different orders at different nodes.

use alloc::vec::Vec;

use crate::error::ClockError;
use crate::vector::{CausalRelation, VectorClock};

/// A message on the wire: the operation, its sender, and the sender's vector clock at send time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope<T, const N: usize> {
    /// The index of the sending node.
    pub sender: usize,
    /// The sender's vector clock when it broadcast this message, including this message.
    pub clock: VectorClock<N>,
    /// The operation.
    pub op: T,
}

/// One node's causally ordered view of the broadcast group of `N` nodes.
#[derive(Clone, Debug)]
pub struct CausalBroadcast<T, const N: usize> {
    me: usize,
    /// Messages delivered so far, in delivery order.
    log: Vec<Envelope<T, N>>,
    /// What this node has delivered, per sender (its own sends count as delivered).
    delivered: VectorClock<N>,
    /// Received but not yet deliverable.
    pending: Vec<Envelope<T, N>>,
}

impl<T: Clone, const N: usize> CausalBroadcast<T, N> {
    /// A node with index `me` in a group of `N` nodes. Fails with [`ClockError::NodeOutOfRange`]
    /// when `me` is not below `N`.
    pub fn new(me: usize) -> Result<Self, ClockError> {
        if me >= N {
            return Err(ClockError::NodeOutOfRange { index: me });
        }
        Ok(Self {
            me,
            log: Vec::new(),
            delivered: VectorClock::new(),
            pending: Vec::new(),
        })
    }

    /// Broadcasts `op` from this node. The message is delivered locally at once and is returned for
    /// the other nodes.
    pub fn broadcast(&mut self, op: T) -> Result<Envelope<T, N>, ClockError> {
        self.delivered.tick(self.me)?;
        let envelope = Envelope {
            sender: self.me,
            clock: self.delivered,
            op,
        };
        self.log.push(envelope.clone());
        Ok(envelope)
    }

    /// Receives a message from another node. It is delivered now if it is deliverable, and any
    /// buffered messages that it unblocks are delivered too. Returns how many messages were
    /// delivered by this call. A duplicate, or a message from this node, is discarded.
    pub fn receive(&mut self, envelope: Envelope<T, N>) -> usize {
        if envelope.sender == self.me || envelope.sender >= N {
            return 0;
        }
        let already = self.delivered.get(envelope.sender).unwrap_or(0);
        let sent = envelope.clock.get(envelope.sender).unwrap_or(0);
        if sent <= already {
            return 0;
        }
        // A second copy of a message that is already buffered adds nothing.
        let buffered = self
            .pending
            .iter()
            .any(|p| p.sender == envelope.sender && p.clock.get(envelope.sender) == Some(sent));
        if !buffered {
            self.pending.push(envelope);
        }
        self.deliver_ready()
    }

    fn deliverable(&self, envelope: &Envelope<T, N>) -> bool {
        let s = envelope.sender;
        let next = self.delivered.get(s).unwrap_or(0) + 1;
        if envelope.clock.get(s) != Some(next) {
            return false;
        }
        (0..N).all(|k| {
            k == s || envelope.clock.get(k).unwrap_or(0) <= self.delivered.get(k).unwrap_or(0)
        })
    }

    fn deliver_ready(&mut self) -> usize {
        let mut count = 0;
        // A buffered copy whose sequence number is already delivered is a duplicate and can never
        // become deliverable: drop it.
        let delivered = self.delivered;
        self.pending.retain(|e| {
            let s = e.sender;
            e.clock.get(s).unwrap_or(0) > delivered.get(s).unwrap_or(0)
        });
        loop {
            let position = self.pending.iter().position(|e| self.deliverable(e));
            let Some(index) = position else {
                return count;
            };
            let envelope = self.pending.swap_remove(index);
            // The sender is below N (checked on receive), so the observe cannot fail; if it ever did,
            // the message would not be recorded as delivered, and it would stay buffered.
            let sent = envelope.clock.get(envelope.sender).unwrap_or(0);
            if self.delivered.observe(envelope.sender, sent).is_err() {
                return count;
            }
            self.log.push(envelope);
            count += 1;
        }
    }

    /// The operations delivered so far, in delivery order.
    pub fn log(&self) -> impl Iterator<Item = &T> {
        self.log.iter().map(|e| &e.op)
    }

    /// The delivered messages, with their clocks, in delivery order.
    pub fn entries(&self) -> &[Envelope<T, N>] {
        &self.log
    }

    /// How many messages are waiting for something they depend on.
    pub fn waiting(&self) -> usize {
        self.pending.len()
    }

    /// Whether the delivered log is a causal order: no entry is causally before an earlier entry.
    pub fn is_causally_ordered(&self) -> bool {
        for (i, later) in self.log.iter().enumerate() {
            for earlier in &self.log[..i] {
                if later.clock.relation(&earlier.clock) == CausalRelation::Before {
                    return false;
                }
            }
        }
        true
    }
}
