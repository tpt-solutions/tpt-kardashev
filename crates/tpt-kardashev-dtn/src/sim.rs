// SPDX-License-Identifier: MIT OR Apache-2.0
//! A deterministic network simulator for convergence layers, on a virtual clock.
//!
//! Links exist only inside the windows of a [`ContactPlan`]. A datagram is sent only when it fits
//! entirely inside a window: transmission takes `size * 1000 / rate` milliseconds, one datagram at a
//! time per directed link, and the datagram must finish before the window closes. Arrival is the
//! end of transmission plus the contact's one-way light time, plus seeded jitter. Loss and
//! duplication are seeded per datagram. The same seed and inputs give the same run.
//!
//! Routing (when enabled) holds each bundle at its node and re-routes it whenever the plan or the
//! link backlog changes; a bundle is handed to the layer only when its route can start.
//!
//! Not modelled: queueing competition between nodes beyond one transmitter per directed link,
//! partition events not in the plan, and clock drift.

use crate::bundle::{Bundle, BundleId};
use crate::contact::{Contact, ContactPlan};
use crate::convergence::{ConvergenceLayer, LtpConvergence};
use crate::eid::Eid;
use crate::error::Error;
use crate::extension::{HOP_COUNT, hop_count_data, parse_hop_count};
use crate::ltp::session::Config;
use crate::route::EarliestArrival;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

/// Default limit on how many times a bundle is handled at relays before it is discarded.
pub const DEFAULT_MAX_HOPS: u32 = 16;

/// Seeded impairments applied on top of the contact plan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Impairments {
    /// Extra arrival delay, uniform in `0..=jitter_ms` milliseconds. Jitter reorders datagrams.
    pub jitter_ms: u64,
    /// Percentage of datagrams lost in transit, 0 to 100.
    pub loss_percent: u64,
    /// Percentage of datagrams delivered twice, 0 to 100.
    pub duplicate_percent: u64,
}

/// One transmission made by the simulator, for inspection after a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transmission {
    /// Transmitting node.
    pub from: u64,
    /// Receiving node.
    pub to: u64,
    /// Start of transmission, DTN milliseconds.
    pub start: u64,
    /// End of transmission, DTN milliseconds.
    pub finish: u64,
    /// Size of the datagram in bytes.
    pub bytes: usize,
    /// Whether the datagram was lost in transit.
    pub lost: bool,
}

/// A bundle held at a node until its route can start.
#[derive(Clone, Debug)]
struct Held {
    /// Node holding the bundle.
    at: u64,
    /// Destination node.
    dest: u64,
    /// End of the bundle's lifetime, DTN time.
    expires: u64,
    /// The encoded bundle.
    payload: Vec<u8>,
    /// When its current route's first hop starts, if known.
    wake: Option<u64>,
}

#[derive(Clone, Debug)]
struct InFlight {
    at: u64,
    seq: u64,
    from: u64,
    to: u64,
    bytes: Vec<u8>,
}

/// SplitMix64, the simulator's only source of randomness.
#[derive(Clone, Copy, Debug)]
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// True with probability `percent` / 100.
    fn percent(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }

    /// Uniform in `0..=max`.
    fn up_to(&mut self, max: u64) -> u64 {
        if max == 0 {
            0
        } else {
            self.next_u64() % (max + 1)
        }
    }
}

/// A set of nodes with LTP convergence layers, connected by the links of a contact plan.
pub struct Network {
    now: u64,
    plan: ContactPlan,
    impairments: Impairments,
    rng: Rng,
    seq: u64,
    nodes: BTreeMap<u64, LtpConvergence>,
    /// Datagrams waiting for a window, per transmitting node, in the order they were produced.
    pending: BTreeMap<u64, VecDeque<(u64, Vec<u8>)>>,
    /// When each directed link's transmitter is next free.
    busy_until: BTreeMap<(u64, u64), u64>,
    in_flight: Vec<InFlight>,
    inbox: BTreeMap<u64, Vec<(u64, Vec<u8>)>>,
    transmissions: Vec<Transmission>,
    /// Whether arriving bundles are routed on to their destinations.
    routing: bool,
    max_hops: u32,
    /// Times each bundle has been handled at a node, for the hop limit.
    handled: BTreeMap<BundleId, u32>,
    /// Bundles discarded because their lifetime ended or the hop limit was reached.
    discarded: u64,
    /// Bundles held at nodes, waiting for a route to open.
    held: Vec<Held>,
    /// Scheduled contact-plan replacements, in time order.
    plan_changes: Vec<(u64, ContactPlan)>,
    /// Links that a node has learned to have failed, as `(from, to)` with the time until which the
    /// link is avoided. The suspicion lapses at that time, so the link is tried again, and it is
    /// cleared at once when a transfer over it completes.
    suspect: BTreeMap<(u64, u64), u64>,
    /// How long a suspicion lasts, in milliseconds.
    suspect_hold: u64,
    /// Scheduled link failures: `(from, to, start, end)`. A datagram that starts on the link in
    /// `[start, end)` is lost. The contact plan is not changed, so nodes learn of the failure only
    /// from the protocol.
    failures: Vec<(u64, u64, u64, u64)>,
}

/// Default time a suspicion of a failed link lasts, in milliseconds.
pub const DEFAULT_SUSPECT_HOLD_MS: u64 = 60_000;

impl Network {
    /// An empty network over `plan`, with the given impairments and seed.
    pub fn new(plan: ContactPlan, impairments: Impairments, seed: u64) -> Self {
        Network {
            now: 0,
            plan,
            impairments,
            rng: Rng(seed),
            seq: 0,
            nodes: BTreeMap::new(),
            pending: BTreeMap::new(),
            busy_until: BTreeMap::new(),
            in_flight: Vec::new(),
            inbox: BTreeMap::new(),
            transmissions: Vec::new(),
            routing: false,
            max_hops: DEFAULT_MAX_HOPS,
            handled: BTreeMap::new(),
            discarded: 0,
            held: Vec::new(),
            plan_changes: Vec::new(),
            suspect: BTreeMap::new(),
            suspect_hold: DEFAULT_SUSPECT_HOLD_MS,
            failures: Vec::new(),
        }
    }

    /// Turns automatic routing of bundles on or off. With routing on, a payload that arrives at a
    /// node is delivered there if it is addressed to that node, and otherwise forwarded to the
    /// next hop of the earliest-arrival route through the contact plan. With routing off (the
    /// default), payloads are only delivered, and forwarding is up to the caller.
    pub fn set_routing(&mut self, enabled: bool) {
        self.routing = enabled;
    }

    /// Sets the number of times a bundle may be handled at nodes before it is discarded.
    pub fn set_max_hops(&mut self, max_hops: u32) {
        self.max_hops = max_hops;
    }

    /// Number of bundles discarded so far: lifetime ended, or hop limit reached, or undecodable.
    pub fn discarded(&self) -> u64 {
        self.discarded
    }

    /// Accepts a bundle that originates at node `from`. It is held there and routed against the
    /// current plan; it is handed to the first hop once that hop can start. A bundle with no route
    /// is held until a route appears or its lifetime ends. Returns `false` only if the bundle was
    /// refused outright (it does not decode, or it is already expired). Requires routing to be on.
    pub fn send_routed(&mut self, from: u64, payload: Vec<u8>) -> Result<bool, Error> {
        if !self.nodes.contains_key(&from) {
            return Err(Error::UnknownNode);
        }
        Ok(self.handle_bundle(from, from, payload))
    }

    /// When the transmitter of `c`'s link can start carrying new data: after its last transmission,
    /// and after every datagram already queued for the same link has been sent.
    fn link_free_at(&self, c: &Contact) -> u64 {
        let busy = self
            .busy_until
            .get(&(c.from, c.to))
            .copied()
            .unwrap_or(0)
            .max(self.now);
        let queued = self
            .pending
            .get(&c.from)
            .map(|queue| {
                queue
                    .iter()
                    .filter(|(to, _)| *to == c.to)
                    .fold(0u64, |acc, (_, bytes)| {
                        acc.saturating_add(c.transmit_ms(bytes.len() as u64))
                    })
            })
            .unwrap_or(0);
        busy.saturating_add(queued)
    }

    /// Handles a bundle that a node `at` has received from `from` (or originates, when `from ==
    /// at`). Returns whether the bundle was delivered or forwarded.
    fn handle_bundle(&mut self, at: u64, from: u64, payload: Vec<u8>) -> bool {
        let Ok(mut bundle) = Bundle::decode(&payload) else {
            self.discarded += 1;
            return false;
        };
        if bundle.expires_at() < self.now {
            self.discarded += 1;
            return false;
        }
        let dest = match bundle.primary.destination {
            Eid::Ipn { node, .. } => node,
            _ => {
                // Only ipn endpoints name a node here.
                self.discarded += 1;
                return false;
            }
        };
        // A bundle that arrives from another node has been forwarded once more. A Hop Count block
        // records that; a bundle whose count then exceeds its limit is deleted (RFC 9171 §4.4.3).
        // The origin's own handling is not a forward, so it does not count.
        let mut payload = payload;
        let hop_index = (from != at)
            .then(|| bundle.blocks.iter().position(|b| b.block_type == HOP_COUNT))
            .flatten();
        if let Some(index) = hop_index {
            let hop = &mut bundle.blocks[index];
            let Ok((limit, count)) = parse_hop_count(&hop.data) else {
                self.discarded += 1;
                return false;
            };
            let count = count + 1;
            if count > u64::from(limit) {
                self.discarded += 1;
                return false;
            }
            match hop_count_data(limit, count) {
                Ok(data) => hop.data = data,
                Err(_) => {
                    self.discarded += 1;
                    return false;
                }
            }
            match bundle.encode() {
                Ok(updated) => payload = updated,
                Err(_) => {
                    self.discarded += 1;
                    return false;
                }
            }
        }
        if dest == at {
            self.inbox.entry(at).or_default().push((from, payload));
            return true;
        }
        let handled = self.handled.entry(bundle.id()).or_insert(0);
        *handled += 1;
        if *handled > self.max_hops {
            self.discarded += 1;
            return false;
        }
        // Hold the bundle at this node. It is routed, and handed to the layer, only when the route's
        // first hop can start, so a plan change in the meantime can still change the route.
        self.held.push(Held {
            at,
            dest,
            expires: bundle.expires_at(),
            payload,
            wake: None,
        });
        true
    }

    /// Routes every held bundle against the current plan and backlog. A bundle whose route can
    /// start now is handed to the layer at its node; the others stay held, with the time at which
    /// their route would start, so that the simulator wakes up then. Returns whether any bundle was
    /// handed on or discarded.
    fn forward_held(&mut self) -> bool {
        let now = self.now;
        let mut progressed = false;
        let held = core::mem::take(&mut self.held);
        let mut still_held = Vec::with_capacity(held.len());
        for mut h in held {
            if now > h.expires {
                self.discarded += 1;
                progressed = true;
                continue;
            }
            let len = h.payload.len() as u64;
            let route = EarliestArrival.route_with_free_at(
                &self.plan,
                h.at,
                h.dest,
                now,
                len,
                // A suspect link is never free, so the router routes around it.
                &|c: &Contact| {
                    if self
                        .suspect
                        .get(&(c.from, c.to))
                        .is_some_and(|&until| until > now)
                    {
                        u64::MAX
                    } else {
                        self.link_free_at(c)
                    }
                },
            );
            let first =
                route.and_then(|r| r.hops.first().map(|hop| (hop.contact.to, hop.departure)));
            match first {
                Some((next, departure)) if departure <= now => {
                    if let Some(node) = self.nodes.get_mut(&h.at) {
                        // Handing the payload to the layer cannot fail for a known destination.
                        let _ = node.send(next, core::mem::take(&mut h.payload));
                    }
                    // Pull its datagrams into the queue now, so that the next routing sees them.
                    self.collect();
                    progressed = true;
                }
                Some((_, departure)) => {
                    h.wake = Some(departure);
                    still_held.push(h);
                }
                None => {
                    h.wake = None;
                    still_held.push(h);
                }
            }
        }
        self.held = still_held;
        progressed
    }

    /// Makes the link `from -> to` fail from virtual time `at`: datagrams that start on it at or
    /// after then are lost. The contact plan still lists the link, so nothing is announced; nodes
    /// find out when their sessions on it are cancelled.
    pub fn fail_link(&mut self, from: u64, to: u64, at: u64) {
        self.failures.push((from, to, at, u64::MAX));
    }

    /// Makes the link `from -> to` fail during `[start, end)` and work again afterwards.
    pub fn fail_link_between(&mut self, from: u64, to: u64, start: u64, end: u64) {
        self.failures.push((from, to, start, end));
    }

    /// Sets how long a suspicion of a failed link lasts, in milliseconds.
    pub fn set_suspect_hold(&mut self, ms: u64) {
        self.suspect_hold = ms;
    }

    fn link_failed(&self, from: u64, to: u64, start: u64) -> bool {
        self.failures
            .iter()
            .any(|&(f, t, s, e)| f == from && t == to && start >= s && start < e)
    }

    /// Links that some node currently avoids as failed, as `(from, to)`.
    pub fn suspect_links(&self) -> Vec<(u64, u64)> {
        let now = self.now;
        self.suspect
            .iter()
            .filter(|(_, until)| **until > now)
            .map(|(&link, _)| link)
            .collect()
    }

    /// Takes in the outcomes of sessions. A cancelled session marks its link as suspect, and the
    /// bundle is held again at the node that sent it, so that it is re-routed around the link. A
    /// completed one clears the suspicion of its link.
    fn harvest_failures(&mut self) -> bool {
        let mut failed: Vec<(u64, u64, Vec<u8>)> = Vec::new();
        let mut completed: Vec<(u64, u64)> = Vec::new();
        for (&id, node) in self.nodes.iter_mut() {
            for (to, payload) in node.take_failed() {
                failed.push((id, to, payload));
            }
            for to in node.take_completed() {
                completed.push((id, to));
            }
        }
        let mut progressed = !completed.is_empty();
        for link in completed {
            self.suspect.remove(&link);
        }
        if failed.is_empty() {
            return progressed;
        }
        progressed = true;
        let until = self.now.saturating_add(self.suspect_hold);
        for (at, to, payload) in failed {
            self.suspect.insert((at, to), until);
            self.handle_bundle(at, at, payload);
        }
        progressed
    }

    /// Replaces the contact plan at virtual time `at`. Held bundles are re-routed against the new
    /// plan from then on. Plan changes are applied in time order.
    pub fn schedule_plan(&mut self, at: u64, plan: ContactPlan) {
        self.plan_changes.push((at, plan));
        self.plan_changes.sort_by_key(|(t, _)| *t);
    }

    /// Number of bundles currently held at nodes, waiting for a route to open.
    pub fn held(&self) -> usize {
        self.held.len()
    }

    /// Applies every scheduled plan change that is due at the current time.
    fn apply_plan_changes(&mut self) -> bool {
        let now = self.now;
        let due = self
            .plan_changes
            .iter()
            .take_while(|(t, _)| *t <= now)
            .count();
        if due == 0 {
            return false;
        }
        let changes: Vec<(u64, ContactPlan)> = self.plan_changes.drain(..due).collect();
        for (_, plan) in changes {
            self.plan = plan;
        }
        true
    }

    /// Adds node `id` with the given LTP configuration.
    pub fn add_node(&mut self, id: u64, config: Config) -> Result<(), Error> {
        let layer = LtpConvergence::new(id, config)?;
        self.nodes.insert(id, layer);
        Ok(())
    }

    /// Queues `payload` at node `from` for delivery to node `to`.
    pub fn send(&mut self, from: u64, to: u64, payload: Vec<u8>) -> Result<(), Error> {
        let node = self.nodes.get_mut(&from).ok_or(Error::UnknownNode)?;
        node.send(to, payload)
    }

    /// Payloads delivered to `node` so far, as `(from, payload)`, in delivery order.
    pub fn inbox(&self, node: u64) -> &[(u64, Vec<u8>)] {
        self.inbox.get(&node).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Every transmission made so far.
    pub fn transmissions(&self) -> &[Transmission] {
        &self.transmissions
    }

    /// The current virtual time.
    pub fn now(&self) -> u64 {
        self.now
    }

    /// Runs the network until virtual time `end` (inclusive), or until nothing more can happen.
    pub fn run_until(&mut self, end: u64) {
        loop {
            self.settle();
            match self.next_event() {
                Some(t) if t <= end => self.now = t,
                _ => break,
            }
        }
        if self.now < end {
            self.now = end;
        }
    }

    /// Does what can be done at the current time: collects datagrams, transmits those that fit
    /// a window, delivers arrivals, and repeats while that produces more work.
    fn settle(&mut self) {
        for _ in 0..16 {
            let mut progressed = false;
            progressed |= self.apply_plan_changes();
            progressed |= self.harvest_failures();
            progressed |= self.forward_held();
            progressed |= self.collect();
            progressed |= self.transmit_pending();
            progressed |= self.deliver_due();
            if !progressed {
                break;
            }
        }
    }

    fn collect(&mut self) -> bool {
        let now = self.now;
        let mut any = false;
        for (&id, node) in self.nodes.iter_mut() {
            while let Some((to, bytes)) = node.poll_outbound(now) {
                self.pending.entry(id).or_default().push_back((to, bytes));
                any = true;
            }
        }
        any
    }

    /// Finds the window that carries a datagram of `len` bytes from `from` to `to`, starting no
    /// earlier than now, and returns the transmission's start and finish.
    fn fit(&self, from: u64, to: u64, len: usize) -> Option<(u64, u64, u64)> {
        let free = self.busy_until.get(&(from, to)).copied().unwrap_or(0);
        self.plan
            .outgoing(from)
            .filter(|c| c.to == to && c.end > self.now)
            .filter_map(|c: &Contact| {
                let start = self.now.max(c.start).max(free);
                let finish = start.saturating_add(c.transmit_ms(len as u64));
                (finish <= c.end).then_some((start, finish, c.owlt))
            })
            .min_by_key(|&(_, finish, _)| finish)
    }

    fn transmit_pending(&mut self) -> bool {
        let mut any = false;
        let senders: Vec<u64> = self.pending.keys().copied().collect();
        for from in senders {
            let mut kept = VecDeque::new();
            let queue = self.pending.remove(&from).unwrap_or_default();
            for (to, bytes) in queue {
                match self.fit(from, to, bytes.len()) {
                    Some((start, finish, owlt)) => {
                        any = true;
                        self.busy_until.insert((from, to), finish);
                        let lost = self.link_failed(from, to, start)
                            || self.rng.percent(self.impairments.loss_percent);
                        self.transmissions.push(Transmission {
                            from,
                            to,
                            start,
                            finish,
                            bytes: bytes.len(),
                            lost,
                        });
                        if !lost {
                            let copies = if self.rng.percent(self.impairments.duplicate_percent) {
                                2
                            } else {
                                1
                            };
                            for _ in 0..copies {
                                let at = finish
                                    .saturating_add(owlt)
                                    .saturating_add(self.rng.up_to(self.impairments.jitter_ms));
                                self.seq += 1;
                                self.in_flight.push(InFlight {
                                    at,
                                    seq: self.seq,
                                    from,
                                    to,
                                    bytes: bytes.clone(),
                                });
                            }
                        }
                    }
                    None => kept.push_back((to, bytes)),
                }
            }
            if !kept.is_empty() {
                self.pending.insert(from, kept);
            }
        }
        any
    }

    fn deliver_due(&mut self) -> bool {
        let now = self.now;
        self.in_flight.sort_by_key(|m| (m.at, m.seq));
        let due = self.in_flight.iter().take_while(|m| m.at <= now).count();
        if due == 0 {
            return false;
        }
        let arrivals: Vec<InFlight> = self.in_flight.drain(..due).collect();
        for msg in arrivals {
            let received: Vec<(u64, Vec<u8>)> = match self.nodes.get_mut(&msg.to) {
                Some(node) => {
                    // A datagram the engine cannot parse is dropped, as a real receiver would.
                    let _ = node.on_inbound(msg.from, &msg.bytes, now);
                    node.take_received()
                }
                None => Vec::new(),
            };
            for (from, payload) in received {
                if self.routing {
                    self.handle_bundle(msg.to, from, payload);
                } else {
                    self.inbox.entry(msg.to).or_default().push((from, payload));
                }
            }
        }
        true
    }

    /// The next time at which something can happen after the current time, if any.
    fn next_event(&self) -> Option<u64> {
        let now = self.now;
        let arrivals = self.in_flight.iter().map(|m| m.at);
        let timers = self.nodes.values().filter_map(|n| n.next_timeout());
        let windows = self.pending.iter().flat_map(|(&from, queue)| {
            queue.iter().filter_map(move |(to, bytes)| {
                let free = self.busy_until.get(&(from, *to)).copied().unwrap_or(0);
                self.plan
                    .outgoing(from)
                    .filter(|c| c.to == *to && c.end > now)
                    .filter_map(|c| {
                        let start = c.start.max(free);
                        (start.saturating_add(c.transmit_ms(bytes.len() as u64)) <= c.end)
                            .then_some(start)
                    })
                    .min()
            })
        });
        let plan_changes = self.plan_changes.iter().map(|(t, _)| *t);
        let held_wakes = self
            .held
            .iter()
            .flat_map(|h| [h.wake, Some(h.expires.saturating_add(1))])
            .flatten();
        arrivals
            .chain(timers)
            .chain(windows)
            .chain(plan_changes)
            .chain(held_wakes)
            .filter(|&t| t > now)
            .min()
    }
}
