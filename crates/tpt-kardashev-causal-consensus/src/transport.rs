// SPDX-License-Identifier: MIT OR Apache-2.0
//! Message transport: a pluggable trait, and a deterministic simulated network that implements it.
//!
//! The simulated network runs on virtual time supplied by the caller. Delay, jitter, loss,
//! duplication and partitions are seeded, so a run is reproducible. Messages are delivered in
//! order of their delivery time, with ties broken by send order, so the output never depends on
//! hash-map iteration.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// A node in the group.
pub type NodeId = u64;

/// A message handed to its destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery<M> {
    /// The sender.
    pub from: NodeId,
    /// The destination.
    pub to: NodeId,
    /// The virtual time of delivery, in milliseconds.
    pub at_ms: u64,
    /// The message.
    pub message: M,
}

/// A message transport. Time is passed in by the caller, so any implementation can be driven by a
/// simulator or by a real clock.
pub trait Transport<M> {
    /// The error of an invalid operation. Losing a message is not an error: it is not reported.
    type Error;

    /// Sends `message` from `from` to `to` at time `now_ms`.
    fn send(
        &mut self,
        now_ms: u64,
        from: NodeId,
        to: NodeId,
        message: M,
    ) -> Result<(), Self::Error>;

    /// The messages due at or before `now_ms`, in delivery order. Each is returned once.
    fn deliver(&mut self, now_ms: u64) -> Vec<Delivery<M>>;
}

/// An error of the simulated network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportError {
    /// A probability is above 1000 per mille, or a partition ends before it starts.
    InvalidConfig,
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid transport configuration")
    }
}

/// Configuration of the simulated network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimConfig {
    /// The fixed one-way delay, in milliseconds.
    pub base_delay_ms: u64,
    /// Extra delay, uniform in `0..=jitter_ms`, added to each message. Jitter reorders messages.
    pub jitter_ms: u64,
    /// Probability of losing a message, in per mille (0 to 1000).
    pub loss_per_mille: u32,
    /// Probability of an extra copy of a message, in per mille (0 to 1000).
    pub duplicate_per_mille: u32,
    /// Seed of the pseudo-random generator.
    pub seed: u64,
}

/// A window during which the link between two nodes is cut in both directions. A message sent in
/// the window is lost; the link works again from `end_ms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cut {
    a: NodeId,
    b: NodeId,
    start_ms: u64,
    end_ms: u64,
}

/// Counts of what the simulated network did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimStats {
    /// Messages accepted by `send`.
    pub sent: u64,
    /// Messages handed to `deliver`.
    pub delivered: u64,
    /// Messages lost to random loss.
    pub lost: u64,
    /// Messages lost to a partition.
    pub partitioned: u64,
    /// Extra copies made by duplication.
    pub duplicated: u64,
}

/// A deterministic simulated network.
#[derive(Clone, Debug)]
pub struct SimNetwork<M> {
    config: SimConfig,
    rng: u64,
    cuts: Vec<Cut>,
    /// Nodes that are down during a window: no message to or from them gets through.
    crashes: Vec<(NodeId, u64, u64)>,
    /// Pending deliveries, keyed by (delivery time, sequence number): deterministic order.
    queue: BTreeMap<(u64, u64), Delivery<M>>,
    seq: u64,
    stats: SimStats,
}

impl<M: Clone> SimNetwork<M> {
    /// A network with `config`. Fails with [`TransportError::InvalidConfig`] when a probability
    /// is above 1000 per mille.
    pub fn new(config: SimConfig) -> Result<Self, TransportError> {
        if config.loss_per_mille > 1_000 || config.duplicate_per_mille > 1_000 {
            return Err(TransportError::InvalidConfig);
        }
        Ok(Self {
            config,
            rng: config.seed.wrapping_mul(2).wrapping_add(1),
            cuts: Vec::new(),
            crashes: Vec::new(),
            queue: BTreeMap::new(),
            seq: 0,
            stats: SimStats::default(),
        })
    }

    /// Cuts the link between `a` and `b` in both directions during `[start_ms, end_ms)`. Fails
    /// with [`TransportError::InvalidConfig`] when the window is empty or reversed.
    pub fn partition(
        &mut self,
        a: NodeId,
        b: NodeId,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<(), TransportError> {
        if end_ms <= start_ms {
            return Err(TransportError::InvalidConfig);
        }
        self.cuts.push(Cut {
            a,
            b,
            start_ms,
            end_ms,
        });
        Ok(())
    }

    /// Cuts every link between the nodes of `group_a` and those of `group_b` during
    /// `[start_ms, end_ms)`. Links inside a group are unaffected. Fails as [`SimNetwork::partition`].
    pub fn partition_sets(
        &mut self,
        group_a: &[NodeId],
        group_b: &[NodeId],
        start_ms: u64,
        end_ms: u64,
    ) -> Result<(), TransportError> {
        for &a in group_a {
            for &b in group_b {
                self.partition(a, b, start_ms, end_ms)?;
            }
        }
        Ok(())
    }

    /// Takes `node` down during `[start_ms, end_ms)`: messages to or from it are lost. The caller
    /// decides what the node does while down (it simply is not ticked); the network only drops
    /// its traffic. Fails as [`SimNetwork::partition`].
    pub fn crash(
        &mut self,
        node: NodeId,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<(), TransportError> {
        if end_ms <= start_ms {
            return Err(TransportError::InvalidConfig);
        }
        self.crashes.push((node, start_ms, end_ms));
        Ok(())
    }

    /// The counts so far.
    pub fn stats(&self) -> SimStats {
        self.stats
    }

    /// The number of messages still in flight.
    pub fn in_flight(&self) -> usize {
        self.queue.len()
    }

    fn next_random(&mut self) -> u64 {
        // Knuth's LCG constants; the top bits are the usable ones.
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.rng >> 33
    }

    fn chance(&mut self, per_mille: u32) -> bool {
        per_mille > 0 && (self.next_random() % 1_000) < u64::from(per_mille)
    }

    fn cut(&self, now_ms: u64, from: NodeId, to: NodeId) -> bool {
        let link = self.cuts.iter().any(|c| {
            ((c.a == from && c.b == to) || (c.a == to && c.b == from))
                && now_ms >= c.start_ms
                && now_ms < c.end_ms
        });
        let down = self.crashes.iter().any(|&(node, start, end)| {
            (node == from || node == to) && now_ms >= start && now_ms < end
        });
        link || down
    }

    fn schedule(&mut self, at_ms: u64, delivery: Delivery<M>) {
        self.queue.insert((at_ms, self.seq), delivery);
        self.seq += 1;
    }

    fn delay(&mut self) -> u64 {
        let jitter = if self.config.jitter_ms == 0 {
            0
        } else {
            self.next_random() % (self.config.jitter_ms + 1)
        };
        self.config.base_delay_ms.saturating_add(jitter)
    }
}

impl<M: Clone> Transport<M> for SimNetwork<M> {
    type Error = TransportError;

    fn send(
        &mut self,
        now_ms: u64,
        from: NodeId,
        to: NodeId,
        message: M,
    ) -> Result<(), TransportError> {
        self.stats.sent += 1;
        if self.cut(now_ms, from, to) {
            self.stats.partitioned += 1;
            return Ok(());
        }
        if self.chance(self.config.loss_per_mille) {
            self.stats.lost += 1;
            return Ok(());
        }
        let copies = if self.chance(self.config.duplicate_per_mille) {
            self.stats.duplicated += 1;
            2
        } else {
            1
        };
        for _ in 0..copies {
            let at_ms = now_ms.saturating_add(self.delay());
            let delivery = Delivery {
                from,
                to,
                at_ms,
                message: message.clone(),
            };
            self.schedule(at_ms, delivery);
        }
        Ok(())
    }

    fn deliver(&mut self, now_ms: u64) -> Vec<Delivery<M>> {
        let mut out = Vec::new();
        while let Some(entry) = self.queue.first_entry() {
            if entry.key().0 > now_ms {
                break;
            }
            out.push(entry.remove());
        }
        self.stats.delivered += out.len() as u64;
        out
    }
}
