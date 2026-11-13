// SPDX-License-Identifier: MIT OR Apache-2.0
//! Convergence layers: how a node hands whole payloads (encoded bundles) to a neighbour and takes
//! them back. A convergence layer does no I/O; the caller moves its datagrams and passes in the
//! virtual time.
//!
//! [`LtpConvergence`] carries each payload as one LTP red-part block, using the sessions in
//! [`crate::ltp::session`]. It keeps one sender per outgoing payload and one receiver per session
//! from each neighbour.

use crate::error::Error;
use crate::ltp::segment::{Segment, SessionId};
use crate::ltp::session::{Config, Receiver, Sender, Status};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

/// Client service that receives the bundles carried over LTP.
pub const BUNDLE_CLIENT_SERVICE: u64 = 1;

/// A convergence layer: moves payloads between this node and its neighbours.
pub trait ConvergenceLayer {
    /// Queues `payload` for delivery to node `to`.
    fn send(&mut self, to: u64, payload: Vec<u8>) -> Result<(), Error>;

    /// The next datagram to transmit at virtual time `now`, with its destination node.
    fn poll_outbound(&mut self, now: u64) -> Option<(u64, Vec<u8>)>;

    /// Takes in a datagram that arrived from node `from`.
    fn on_inbound(&mut self, from: u64, datagram: &[u8], now: u64) -> Result<(), Error>;

    /// Payloads that have been fully received, as `(from, payload)`, in the order completed.
    /// Each payload is returned once.
    fn take_received(&mut self) -> Vec<(u64, Vec<u8>)>;

    /// Payloads that could not be delivered to a neighbour, as `(to, payload)`: their sessions were
    /// cancelled, typically at a retransmission limit. Each is returned once.
    fn take_failed(&mut self) -> Vec<(u64, Vec<u8>)>;

    /// When the next timer of this layer expires, if any.
    fn next_timeout(&self) -> Option<u64>;
}

/// LTP as a convergence layer: one red-part block per payload.
#[derive(Clone, Debug)]
pub struct LtpConvergence {
    local: u64,
    config: Config,
    next_number: u64,
    /// Outgoing payloads, keyed by destination and session number.
    senders: BTreeMap<(u64, u64), Sender>,
    /// Incoming sessions, keyed by source node and session.
    receivers: BTreeMap<(u64, SessionId), Receiver>,
    /// Sessions whose payload has been handed out, so that each is returned once.
    emitted: BTreeSet<(u64, SessionId)>,
    received: Vec<(u64, Vec<u8>)>,
    /// Copies of outgoing payloads, kept until their session completes or is cancelled, so that a
    /// failed payload can be handed back.
    outgoing: BTreeMap<(u64, u64), Vec<u8>>,
    failed: Vec<(u64, Vec<u8>)>,
    /// Round-trip estimate per neighbour, from the checkpoints this node has sent to it.
    rtt: BTreeMap<u64, RttEstimate>,
}

/// Smoothed round-trip time and its variance for one neighbour (Jacobson and Karels, as used by
/// TCP retransmission timers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RttEstimate {
    srtt: u64,
    rttvar: u64,
    measured: bool,
}

impl RttEstimate {
    fn observe(&mut self, sample: u64) {
        if self.measured {
            let err = sample.abs_diff(self.srtt);
            self.rttvar = (3 * self.rttvar + err) / 4;
            self.srtt = (7 * self.srtt + sample) / 8;
        } else {
            self.srtt = sample;
            self.rttvar = sample / 2;
            self.measured = true;
        }
    }
}

/// The retransmission timeout for a neighbour: the smoothed round trip plus four times its
/// variance, kept at most `ceiling` (the configured timeout). Until a neighbour has been measured,
/// the ceiling is used.
fn rto(rtt: &BTreeMap<u64, RttEstimate>, peer: u64, ceiling: u64) -> u64 {
    match rtt.get(&peer) {
        Some(e) if e.measured => e.srtt.saturating_add(4 * e.rttvar).clamp(1, ceiling.max(1)),
        _ => ceiling,
    }
}

impl LtpConvergence {
    /// A layer for node `local`, using `config` for every session.
    pub fn new(local: u64, config: Config) -> Result<Self, Error> {
        // Validate the configuration once, up front.
        Receiver::new(
            SessionId {
                originator: local,
                number: 0,
            },
            config,
        )?;
        Ok(Self {
            local,
            config,
            next_number: 1,
            senders: BTreeMap::new(),
            receivers: BTreeMap::new(),
            emitted: BTreeSet::new(),
            received: Vec::new(),
            outgoing: BTreeMap::new(),
            failed: Vec::new(),
            rtt: BTreeMap::new(),
        })
    }

    /// Moves payloads of cancelled sessions to the failure list, and drops the copies of completed
    /// ones. A cancelled sender is removed; its cancel segment is not sent, because the link has
    /// already failed to carry the session.
    fn harvest(&mut self) {
        let cancelled: Vec<(u64, u64)> = self
            .senders
            .iter()
            .filter(|(_, s)| matches!(s.status(), Status::Cancelled { .. }))
            .map(|(key, _)| *key)
            .collect();
        for key in cancelled {
            self.senders.remove(&key);
            if let Some(payload) = self.outgoing.remove(&key) {
                self.failed.push((key.0, payload));
            }
        }
        let completed: Vec<(u64, u64)> = self
            .senders
            .iter()
            .filter(|(_, s)| s.status() == Status::Completed)
            .map(|(key, _)| *key)
            .collect();
        for key in completed {
            self.outgoing.remove(&key);
        }
    }

    /// The node this layer belongs to.
    pub fn local(&self) -> u64 {
        self.local
    }
}

impl ConvergenceLayer for LtpConvergence {
    fn send(&mut self, to: u64, payload: Vec<u8>) -> Result<(), Error> {
        let number = self.next_number;
        self.next_number += 1;
        let session = SessionId {
            originator: self.local,
            number,
        };
        self.outgoing.insert((to, number), payload.clone());
        let sender = Sender::new(session, BUNDLE_CLIENT_SERVICE, payload, self.config)?;
        self.senders.insert((to, number), sender);
        Ok(())
    }

    fn poll_outbound(&mut self, now: u64) -> Option<(u64, Vec<u8>)> {
        for (&(to, _), sender) in self.senders.iter_mut() {
            sender.set_timeout(rto(&self.rtt, to, self.config.timeout_ms));
            if let Some(bytes) = sender.poll(now).and_then(|seg| seg.encode().ok()) {
                return Some((to, bytes));
            }
        }
        for (&(from, _), receiver) in self.receivers.iter_mut() {
            if let Some(bytes) = receiver.poll(now).and_then(|seg| seg.encode().ok()) {
                return Some((from, bytes));
            }
        }
        None
    }

    fn on_inbound(&mut self, from: u64, datagram: &[u8], now: u64) -> Result<(), Error> {
        let seg = Segment::decode(datagram)?;
        if seg.session.originator == self.local {
            // Reports, acknowledgments and cancels for a payload we sent. A report may give a
            // round-trip sample for the neighbour.
            let samples = match self.senders.get_mut(&(from, seg.session.number)) {
                Some(sender) => {
                    sender.on_segment(&seg, now);
                    sender.take_rtt_samples()
                }
                None => Vec::new(),
            };
            for sample in samples {
                self.rtt.entry(from).or_default().observe(sample);
            }
            return Ok(());
        }
        let key = (from, seg.session);
        if !self.receivers.contains_key(&key) {
            let receiver = Receiver::new(seg.session, self.config)?;
            self.receivers.insert(key, receiver);
        }
        if let Some(receiver) = self.receivers.get_mut(&key) {
            receiver.on_segment(&seg);
            let fresh = !self.emitted.contains(&key);
            if let (true, Some(payload)) = (fresh, receiver.delivered()) {
                self.emitted.insert(key);
                self.received.push((from, payload.to_vec()));
            }
        }
        Ok(())
    }

    fn take_received(&mut self) -> Vec<(u64, Vec<u8>)> {
        core::mem::take(&mut self.received)
    }

    fn take_failed(&mut self) -> Vec<(u64, Vec<u8>)> {
        self.harvest();
        core::mem::take(&mut self.failed)
    }

    fn next_timeout(&self) -> Option<u64> {
        self.senders
            .values()
            .filter_map(|s| s.next_timeout())
            .chain(self.receivers.values().filter_map(|r| r.next_timeout()))
            .min()
    }
}
