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

use crate::bundle::{
    BLOCK_TYPE_PAYLOAD, Bundle, BundleId, CanonicalBlock, CrcType, CreationTimestamp, PrimaryBlock,
    bundle_flags,
};
use crate::contact::{Contact, ContactPlan};
use crate::convergence::{ConvergenceLayer, LtpConvergence};
use crate::custody::{
    self, CUSTODY_REQUEST_BLOCK, CustodyKey, CustodySignal, CustodyStats, reason,
};
use crate::eid::Eid;
use crate::error::Error;
use crate::extension::{HOP_COUNT, hop_count_data, parse_hop_count};
use crate::ltp::session::Config;
use crate::report::{self, Event, StatusReport};
use crate::route::{EarliestArrival, Hop, RouteConstraints};
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
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
    /// Percentage of datagrams held back by an extra delay, 0 to 100. A held-back datagram can be
    /// overtaken by later ones, so this is the explicit reordering knob; jitter alone reorders only
    /// when two arrival times happen to cross.
    pub reorder_percent: u64,
    /// The largest extra delay of a held-back datagram, in milliseconds, drawn uniformly in
    /// `0..=reorder_ms`.
    pub reorder_ms: u64,
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
    /// Contacts whose volume this bundle has claimed while it waits.
    claims: Vec<(u64, u64, u64)>,
    /// The node the bundle was received from, or `None` if this node originated it.
    received_from: Option<u64>,
    /// For one copy of a critical bundle, the only neighbour its first hop may enter.
    only_via: Option<u64>,
    /// The link `(node, neighbour)` whose waiting backlog this bundle is counted in, while it is
    /// routed to a window that has not opened yet (see [`Network::waiting`]).
    waiting_on: Option<(u64, u64)>,
    /// Arrival order of the bundle at the simulator, so that waiting bundles queue first-in, first-out.
    ticket: u64,
    /// For a bundle this node holds as custodian, the identity of its custody copy.
    custody: Option<CustodyKey>,
}

/// A custody copy: the bundle this node keeps until the next custodian takes custody of it (RFC 5050
/// §5.10.2), with the state of the transfer.
#[derive(Clone, Debug)]
struct CustodyCopy {
    /// The encoded bundle, with this node as its current custodian.
    payload: Vec<u8>,
    /// Destination node.
    dest: u64,
    /// End of the bundle's lifetime, DTN time.
    expires: u64,
    /// The node that offered custody, or this node if it originated the bundle.
    prev: u64,
    /// The next custodian the copy was last handed to, while its custody transfer is pending.
    awaiting: Option<u64>,
    /// When the custody transfer times out, if it is pending.
    deadline: Option<u64>,
    /// Hand-offs made so far.
    attempts: u32,
    /// Nodes that refused custody of this bundle: they are not used for its next hop (SABR §2.4.4).
    refused: Vec<u64>,
}

impl CustodyCopy {
    fn new(payload: Vec<u8>, dest: u64, expires: u64, prev: u64) -> Self {
        Self {
            payload,
            dest,
            expires,
            prev,
            awaiting: None,
            deadline: None,
            attempts: 0,
            refused: Vec::new(),
        }
    }
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
    /// Times each bundle has been handled at a node, for the hop limit. A fragment is counted on its own:
    /// the key is the bundle identity with the fragment offset, since fragments share the identity.
    handled: BTreeMap<(BundleId, Option<u64>), u32>,
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
    /// OWLT margin added to each contact's light time when routing (ms).
    owlt_margin: u64,
    /// Volume (bytes) reserved on each contact, keyed by `contact_key`.
    reserved: BTreeMap<(u64, u64, u64), u64>,
    /// Bytes of held bundles that are routed through a link but not yet handed to its layer, because
    /// their first hop opens later. They are the link's backlog for routing, as a CGR contact queue
    /// holds bundles bound for it. Keyed by `(node, neighbour)`.
    waiting: BTreeMap<(u64, u64), BTreeMap<u64, u64>>,
    /// The next ticket to give a held bundle.
    next_ticket: u64,
    /// Custody copies this node holds, keyed by `(node, bundle identity)`.
    custody_copies: BTreeMap<(u64, CustodyKey), CustodyCopy>,
    /// How much custody each node can hold, in bytes; a node not listed refuses all custody.
    custody_capacity: BTreeMap<u64, u64>,
    /// How long a custodian waits for the next custodian's signal, in milliseconds.
    custody_timeout: u64,
    /// Hand-offs a custodian makes for one bundle before it gives up.
    custody_attempts: u32,
    /// Bundles already delivered to their destination through custody, so a retransmitted copy is
    /// acknowledged but not delivered twice.
    custody_delivered: BTreeSet<(u64, CustodyKey)>,
    /// Custody events so far.
    custody_stats: CustodyStats,
    /// Status reports sent so far.
    status_reports: u64,
}

/// Default time a suspicion of a failed link lasts, in milliseconds.
pub const DEFAULT_SUSPECT_HOLD_MS: u64 = 60_000;

/// Default time a custodian waits for the next custodian's signal, in milliseconds.
pub const DEFAULT_CUSTODY_TIMEOUT_MS: u64 = 60_000;

/// Default number of hand-offs a custodian makes for one bundle before it gives up.
pub const DEFAULT_CUSTODY_ATTEMPTS: u32 = 3;

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
            owlt_margin: 0,
            reserved: BTreeMap::new(),
            waiting: BTreeMap::new(),
            next_ticket: 0,
            custody_copies: BTreeMap::new(),
            custody_capacity: BTreeMap::new(),
            custody_timeout: DEFAULT_CUSTODY_TIMEOUT_MS,
            custody_attempts: DEFAULT_CUSTODY_ATTEMPTS,
            custody_delivered: BTreeSet::new(),
            custody_stats: CustodyStats::default(),
            status_reports: 0,
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

    /// Accepts a critical bundle that originates at node `from`: one copy is sent through each
    /// neighbour of `from` in the contact plan, so the bundle travels over the best route and over
    /// every other plausible one (CCSDS 734.3-B-1 §2.4.5). Copies may all arrive, so the
    /// destination can receive the bundle more than once. Returns as [`Network::send_routed`].
    /// The critical flag is an argument of this call, not a field of the bundle.
    pub fn send_critical(&mut self, from: u64, payload: Vec<u8>) -> Result<bool, Error> {
        self.send_copies(from, payload, usize::MAX)
    }

    /// Accepts a bundle that originates at node `from` and sends up to `copies` copies, one through each
    /// of the first `copies` neighbours of the bundle's route list (CCSDS 734.3-B-1 §2.4.5). The route
    /// list is the best route through each neighbour of `from` that can reach the destination, in order of
    /// projected arrival, with volume and backlog taken into account (see [`crate::route::EarliestArrival::route_list`]).
    /// With `copies` of one, the bundle takes the best route, as an ordinary bundle does; with a count at
    /// least the number of usable neighbours, every such neighbour gets a copy, as
    /// [`Network::send_critical`] sends it. Returns as [`Network::send_routed`]; fails with
    /// [`Error::InvalidBundle`] for zero copies.
    pub fn send_copies(
        &mut self,
        from: u64,
        payload: Vec<u8>,
        copies: usize,
    ) -> Result<bool, Error> {
        if !self.nodes.contains_key(&from) {
            return Err(Error::UnknownNode);
        }
        if copies == 0 {
            return Err(Error::InvalidBundle {
                rule: "a bundle needs at least one copy",
            });
        }
        let before = self.held.len();
        if !self.handle_bundle(from, from, payload) {
            return Ok(false);
        }
        // handle_bundle holds the bundle once, unless it was delivered locally (then nothing is
        // held). That one entry becomes one copy per neighbour on a route. With no route it is kept
        // as it is, so that it waits for one.
        let held_once = self.held.len() == before + 1;
        if held_once {
            let Some(template) = self.held.pop() else {
                return Ok(true);
            };
            let neighbours = self.route_neighbours(&template, copies);
            if neighbours.is_empty() {
                self.held.push(template);
            } else {
                for n in neighbours {
                    let ticket = self.take_ticket();
                    self.held.push(Held {
                        only_via: Some(n),
                        ticket,
                        ..template.clone()
                    });
                }
            }
        }
        Ok(true)
    }

    /// The first hops of the best `copies` routes for a held bundle, in order of projected arrival,
    /// each neighbour at most once.
    fn route_neighbours(&self, h: &Held, copies: usize) -> Vec<u64> {
        let len = h.payload.len() as u64;
        let available = |c: &Contact| self.available(c);
        let ticket = h.ticket;
        let free_at = |c: &Contact| self.free_at(c, ticket);
        let constraints = RouteConstraints {
            expires: h.expires,
            owlt_margin: self.owlt_margin,
            evc: evc_of(len),
            available: &available,
            excluded: &[],
        };
        EarliestArrival
            .route_list(
                &self.plan,
                h.at,
                h.dest,
                self.now,
                len,
                &free_at,
                &constraints,
                copies,
            )
            .iter()
            .filter_map(|r| r.hops.first().map(|hop| hop.contact.to))
            .collect()
    }

    /// When `c` can next carry the bundle with `ticket`, for routing: its link's backlog drained through
    /// its windows, or never if a suspicion marks the link failed until later (the router then routes
    /// around it).
    fn free_at(&self, c: &Contact, ticket: u64) -> u64 {
        if self
            .suspect
            .get(&(c.from, c.to))
            .is_some_and(|&until| until > self.now)
        {
            u64::MAX
        } else {
            self.link_free_at(c, ticket)
        }
    }

    /// A fresh ticket for a held bundle, in arrival order.
    fn take_ticket(&mut self) -> u64 {
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        ticket
    }

    /// Bytes of the bundles that wait for link `key` and arrived before the bundle with `ticket`: the
    /// backlog ahead of it, first in first out.
    fn waiting_before(&self, key: (u64, u64), ticket: u64) -> u64 {
        self.waiting.get(&key).map_or(0, |entries| {
            entries
                .range(..ticket)
                .fold(0u64, |acc, (_, bytes)| acc.saturating_add(*bytes))
        })
    }

    /// Fragments a bundle and sends each fragment from node `from`, routed as [`Network::send_routed`]
    /// sends a bundle. `bundle_bytes` is an encoded bundle that is not itself a fragment; each fragment
    /// carries at most `max_payload` bytes of its application data. Returns the number of fragments
    /// accepted. The destination's inbox holds the encoded fragments, in arrival order;
    /// [`crate::fragment::reassemble_encoded`] turns them back into the bundle. Fails with [`Error::UnknownNode`], with the codec's error for an
    /// undecodable bundle, and with the fragmentation error of [`crate::fragment::fragment`].
    pub fn send_fragmented(
        &mut self,
        from: u64,
        bundle_bytes: &[u8],
        max_payload: usize,
    ) -> Result<usize, Error> {
        if !self.nodes.contains_key(&from) {
            return Err(Error::UnknownNode);
        }
        let bundle = Bundle::decode(bundle_bytes)?;
        let parts = crate::fragment::fragment(&bundle, max_payload)?;
        self.send_parts(from, &parts)
    }

    /// Fragments a bundle so that each fragment fits the volume the sender's first hops have free now,
    /// and sends them as [`Network::send_fragmented`] does (CCSDS 734.3-B-1 §2.4.4). The fragment size is
    /// the largest payload whose reservation charge (the bytes plus the convergence-layer overhead) fits
    /// the largest free volume among `from`'s outgoing contacts. A fragment that finds no room waits, as
    /// any bundle does. A bundle that fits is sent whole. Returns the number of fragments accepted.
    pub fn send_fitting(&mut self, from: u64, bundle_bytes: &[u8]) -> Result<usize, Error> {
        if !self.nodes.contains_key(&from) {
            return Err(Error::UnknownNode);
        }
        let bundle = Bundle::decode(bundle_bytes)?;
        let free = self
            .plan
            .outgoing(from)
            .map(|c| self.available(c))
            .max()
            .unwrap_or(0);
        if free == 0 {
            return self.send_parts(from, &[bundle]);
        }
        let payload_len = bundle.payload().map_or(0, |p| p.len()) as u64;
        let overhead = (bundle_bytes.len() as u64).saturating_sub(payload_len);
        let max_encoded = largest_charged_within(free);
        // The fragment header adds bytes to the encoding: leave a margin, and shrink further if a
        // fragment still does not fit once encoded.
        let mut payload = max_encoded.saturating_sub(overhead + FRAGMENT_MARGIN_BYTES);
        for _ in 0..16 {
            let parts = crate::fragment::fragment(&bundle, payload as usize)?;
            let mut oversized = false;
            for part in &parts {
                let encoded = part.encode()?.len() as u64;
                if evc_of(encoded) > free {
                    oversized = true;
                }
            }
            if !oversized {
                return self.send_parts(from, &parts);
            }
            payload = payload.saturating_sub(FRAGMENT_SHRINK_BYTES);
        }
        Err(Error::InvalidBundle {
            rule: "no fragment size fits the free volume",
        })
    }

    /// Hands each bundle in `parts` to node `from` as a new bundle; returns how many were accepted.
    fn send_parts(&mut self, from: u64, parts: &[Bundle]) -> Result<usize, Error> {
        let mut accepted = 0;
        for part in parts {
            let bytes = part.encode()?;
            if self.handle_bundle(from, from, bytes) {
                accepted += 1;
            }
        }
        Ok(accepted)
    }

    /// When the transmitter of `c`'s link can start carrying new data: after its last transmission,
    /// and after the backlog ahead of it has drained. The backlog is the bytes of every datagram
    /// queued for the same link, drained through the link's windows at their rates
    /// (see [`Network::drain_finish`]), so a backlog that does not fit one window spills into the next.
    fn link_free_at(&self, c: &Contact, ticket: u64) -> u64 {
        let busy = self
            .busy_until
            .get(&(c.from, c.to))
            .copied()
            .unwrap_or(0)
            .max(self.now);
        let backlog: u64 = self
            .pending
            .get(&c.from)
            .map(|queue| {
                queue
                    .iter()
                    .filter(|(to, _)| *to == c.to)
                    .map(|(_, bytes)| bytes.len() as u64)
                    .sum()
            })
            .unwrap_or(0);
        let waiting = self.waiting_before((c.from, c.to), ticket);
        self.drain_finish(c.from, c.to, busy, backlog.saturating_add(waiting))
    }

    /// The time at which `bytes` queued on the link `from -> to`, starting to drain at `start`, have all
    /// been sent. The link carries data only inside its contact windows, at each contact's rate, and a
    /// backlog that does not finish inside one window continues in the next. `u64::MAX` when the windows
    /// that remain cannot carry the backlog.
    fn drain_finish(&self, from: u64, to: u64, start: u64, bytes: u64) -> u64 {
        let mut windows: Vec<&Contact> = self.plan.outgoing(from).filter(|c| c.to == to).collect();
        windows.sort_by_key(|c| c.start);
        let mut remaining = bytes;
        let mut t = start;
        for c in windows {
            if remaining == 0 {
                break;
            }
            if c.end <= t {
                continue;
            }
            let s = t.max(c.start);
            let need = c.transmit_ms(remaining);
            if s.saturating_add(need) <= c.end {
                return s.saturating_add(need);
            }
            // The window closes before the backlog drains: send what fits, then continue.
            let sent = bytes_in_ms(c.rate, c.end - s);
            remaining = remaining.saturating_sub(sent);
            t = c.end;
        }
        if remaining == 0 { t } else { u64::MAX }
    }

    /// Handles a bundle that a node `at` has received from `from` (or originates, when `from ==
    /// at`). Returns whether the bundle was delivered or forwarded.
    fn handle_bundle(&mut self, at: u64, from: u64, payload: Vec<u8>) -> bool {
        let Ok(mut bundle) = Bundle::decode(&payload) else {
            self.discarded += 1;
            return false;
        };
        let payload_len = bundle.payload().map_or(0, |p| p.len() as u64);
        if bundle.expires_at() < self.now {
            self.status_report(
                at,
                &bundle.primary,
                payload_len,
                Event::Deleted,
                report::reason::LIFETIME_EXPIRED,
            );
            self.discarded += 1;
            return false;
        }
        if from != at {
            self.status_report(
                at,
                &bundle.primary,
                payload_len,
                Event::Received,
                report::reason::NO_ADDITIONAL_INFORMATION,
            );
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
                self.status_report(
                    at,
                    &bundle.primary,
                    payload_len,
                    Event::Deleted,
                    report::reason::HOP_LIMIT_EXCEEDED,
                );
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
        // A custody signal is an administrative record that a custodian sends to the node that offered it
        // custody. It changes that node's copy and is not delivered.
        if dest == at
            && bundle.primary.flags & bundle_flags::ADMIN_RECORD != 0
            && let Some(signal) = bundle.payload().and_then(|p| CustodySignal::decode(p).ok())
        {
            let custodian = ipn_node(&bundle.primary.source).unwrap_or(from);
            self.receive_signal(at, custodian, signal);
            return true;
        }
        let requested = custody_key(&bundle).filter(|_| has_custody_request(&bundle));
        if dest == at {
            if let Some(key) = requested.filter(|_| from != at) {
                // The destination takes custody by delivering, and says so to the node that offered it.
                self.send_signal(at, from, key, true, reason::NO_ADDITIONAL_INFORMATION);
                if !self.custody_delivered.insert((at, key)) {
                    return true; // a retransmitted copy: acknowledged, not delivered again
                }
            }
            self.status_report(
                at,
                &bundle.primary,
                payload_len,
                Event::Delivered,
                report::reason::NO_ADDITIONAL_INFORMATION,
            );
            self.inbox.entry(at).or_default().push((from, payload));
            return true;
        }
        // Custody. A node that receives a bundle asking for custody takes it, or refuses it. A node that
        // originates such a bundle keeps its copy until the first custodian takes custody.
        if let Some(key) = requested {
            if from != at {
                if self.custody_copies.contains_key(&(at, key)) {
                    // Already the custodian: a redundant copy. The sender releases its own custody.
                    self.custody_stats.redundant += 1;
                    self.send_signal(at, from, key, false, reason::REDUNDANT_RECEPTION);
                    return false;
                }
                let capacity = self.custody_capacity.get(&at).copied().unwrap_or(0);
                if self.custody_used(at).saturating_add(payload.len() as u64) > capacity {
                    self.custody_stats.refused += 1;
                    self.send_signal(at, from, key, false, reason::DEPLETED_STORAGE);
                    return false;
                }
                let Ok(rewritten) = rewrite_custodian(&mut bundle, at) else {
                    self.discarded += 1;
                    return false;
                };
                payload = rewritten;
                self.custody_copies.insert(
                    (at, key),
                    CustodyCopy::new(payload.clone(), dest, bundle.expires_at(), from),
                );
                self.custody_stats.accepted += 1;
                self.send_signal(at, from, key, true, reason::NO_ADDITIONAL_INFORMATION);
            } else if let Some(copy) = self.custody_copies.get_mut(&(at, key)) {
                // Re-entering after a failed session: the copy is still this node's to route.
                copy.awaiting = None;
                copy.deadline = None;
            } else {
                self.custody_copies.insert(
                    (at, key),
                    CustodyCopy::new(payload.clone(), dest, bundle.expires_at(), at),
                );
            }
        }
        let key = (bundle.id(), bundle.primary.fragment.map(|f| f.offset));
        let handled = self.handled.entry(key).or_insert(0);
        *handled += 1;
        if *handled > self.max_hops {
            self.status_report(
                at,
                &bundle.primary,
                payload_len,
                Event::Deleted,
                report::reason::HOP_LIMIT_EXCEEDED,
            );
            self.discarded += 1;
            return false;
        }
        // Hold the bundle at this node. It is routed, and handed to the layer, only when the route's
        // first hop can start, so a plan change in the meantime can still change the route.
        let ticket = self.take_ticket();
        self.held.push(Held {
            ticket,
            at,
            dest,
            expires: bundle.expires_at(),
            payload,
            wake: None,
            claims: Vec::new(),
            received_from: (from != at).then_some(from),
            only_via: None,
            waiting_on: None,
            custody: requested,
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
            let len = h.payload.len() as u64;
            let evc = evc_of(len);
            // The volume this bundle claimed on an earlier pass is claimed afresh below, so that
            // a bundle does not count against the contacts it is itself about to use.
            release(&mut self.reserved, &h.claims, evc);
            h.claims.clear();
            if let Some(key) = h.waiting_on.take() {
                unwait(&mut self.waiting, key, h.ticket);
            }
            if now > h.expires {
                if let Ok(subject) = Bundle::decode(&h.payload) {
                    let len = subject.payload().map_or(0, |p| p.len() as u64);
                    self.status_report(
                        h.at,
                        &subject.primary,
                        len,
                        Event::Deleted,
                        report::reason::LIFETIME_EXPIRED,
                    );
                }
                self.discarded += 1;
                if let Some(key) = h.custody {
                    self.custody_copies.remove(&(h.at, key));
                }
                progressed = true;
                continue;
            }
            // A suspect link is never free, so the router routes around it.
            let ticket = h.ticket;
            let free_at = |c: &Contact| self.free_at(c, ticket);
            let available = |c: &Contact| self.available(c);
            // A bundle is not sent back to the node it came from (SABR §3.2.5.2). A copy of a
            // critical bundle may take only its own first hop (SABR §2.4.5).
            let mut excluded: Vec<u64> = h.received_from.into_iter().collect();
            // Nodes that refused custody of this bundle are not its next hop (SABR §2.4.4).
            if let Some(copy) = h
                .custody
                .and_then(|key| self.custody_copies.get(&(h.at, key)))
            {
                excluded.extend(copy.refused.iter().copied());
            }
            if let Some(only) = h.only_via {
                excluded.extend(
                    self.plan
                        .outgoing(h.at)
                        .map(|c| c.to)
                        .filter(|&n| n != only),
                );
            }
            let constraints = RouteConstraints {
                expires: h.expires,
                owlt_margin: self.owlt_margin,
                evc,
                available: &available,
                excluded: &excluded,
            };
            // The bundle takes the route of earliest projected arrival, as CGR does, and waits for its
            // first hop if that hop cannot start yet.
            let route = EarliestArrival.route_constrained(
                &self.plan,
                h.at,
                h.dest,
                now,
                len,
                &free_at,
                &constraints,
            );
            let first = route
                .as_ref()
                .and_then(|r| r.hops.first().map(|hop| (hop.contact.to, hop.departure)));
            match first {
                Some((next, departure)) if departure <= now => {
                    // The bundle is forwarded: a status report if it asked for one.
                    if let Ok(subject) = Bundle::decode(&h.payload) {
                        let len = subject.payload().map_or(0, |p| p.len() as u64);
                        self.status_report(
                            h.at,
                            &subject.primary,
                            len,
                            Event::Forwarded,
                            report::reason::NO_ADDITIONAL_INFORMATION,
                        );
                    }
                    // A custody copy is now pending the next custodian's signal, until its timeout.
                    if let Some(key) = h.custody
                        && let Some(copy) = self.custody_copies.get_mut(&(h.at, key))
                    {
                        copy.awaiting = Some(next);
                        copy.deadline = Some(now.saturating_add(self.custody_timeout));
                        copy.attempts += 1;
                    }
                    // The route's volume stays reserved once the bundle is handed on. It is not
                    // released again, since the bundle is no longer held (SABR §3.2.8.1.2).
                    if let Some(route) = route.as_ref() {
                        claim(&mut self.reserved, &route.hops, evc);
                    }
                    if let Some(node) = self.nodes.get_mut(&h.at) {
                        // Handing the payload to the layer cannot fail for a known destination.
                        let _ = node.send(next, core::mem::take(&mut h.payload));
                    }
                    // Pull its datagrams into the queue now, so that the next routing sees them.
                    self.collect();
                    progressed = true;
                }
                Some((next, departure)) => {
                    // Held bundles keep their claim, so the next bundle routed sees it. Their bytes
                    // count as backlog on the link they wait for, so the next bundle sees that too.
                    if let Some(route) = route.as_ref() {
                        h.claims = claim(&mut self.reserved, &route.hops, evc);
                    }
                    let key = (h.at, next);
                    self.waiting.entry(key).or_default().insert(h.ticket, len);
                    h.waiting_on = Some(key);
                    h.wake = Some(departure);
                    still_held.push(h);
                }
                None => {
                    h.wake = None;
                    still_held.push(h);
                }
            }
        }
        // Bundles created during this pass (status reports, custody signals) were pushed to the emptied list
        // and must survive the reassignment. They are routed on the next pass.
        let created = core::mem::take(&mut self.held);
        if !created.is_empty() {
            progressed = true;
        }
        still_held.extend(created);
        self.held = still_held;
        progressed
    }

    /// Accepts a bundle that originates at node `from` and asks for custody transfer (RFC 5050 §5.10,
    /// with the CCSDS 734.2 custody signal; see [`crate::custody`]). The bundle gets a custody request
    /// block naming `from`. `from` keeps a copy until the first custodian takes custody, and resends it
    /// if no signal arrives in time. Requires routing. Returns as [`Network::send_routed`].
    pub fn send_custody(&mut self, from: u64, payload: Vec<u8>) -> Result<bool, Error> {
        let bytes = self.with_custody_request(from, payload)?;
        Ok(self.handle_bundle(from, from, bytes))
    }

    /// Fragments a bundle and sends each fragment from `from` as a bundle asking for custody. Each fragment is
    /// its own custody record (its identity includes its offset), so each can be taken, released and given up
    /// on its own. Returns the number of fragments accepted; the fragmentation is that of
    /// [`Network::send_fragmented`].
    pub fn send_custody_fragmented(
        &mut self,
        from: u64,
        bundle_bytes: &[u8],
        max_payload: usize,
    ) -> Result<usize, Error> {
        if !self.nodes.contains_key(&from) {
            return Err(Error::UnknownNode);
        }
        let bundle = Bundle::decode(bundle_bytes)?;
        let parts = crate::fragment::fragment(&bundle, max_payload)?;
        let mut accepted = 0;
        for part in &parts {
            let bytes = self.with_custody_request(from, part.encode()?)?;
            if self.handle_bundle(from, from, bytes) {
                accepted += 1;
            }
        }
        Ok(accepted)
    }

    /// Accepts a bundle that originates at `from` and asks for custody, and sends up to `copies` copies of it
    /// over the routes of its route list (see [`Network::send_copies`]). The copies share one custody record
    /// at `from`: the first custodian to take custody releases the record and the copies not yet sent. The
    /// destination delivers the bundle once. Returns as [`Network::send_custody`].
    pub fn send_custody_copies(
        &mut self,
        from: u64,
        payload: Vec<u8>,
        copies: usize,
    ) -> Result<bool, Error> {
        let bytes = self.with_custody_request(from, payload)?;
        self.send_copies(from, bytes, copies)
    }

    /// Adds a custody request block naming `from` to the bundle in `payload`, after the checks that custody
    /// transfer needs (a known node, routing on, an ipn source, no request already).
    fn with_custody_request(&self, from: u64, payload: Vec<u8>) -> Result<Vec<u8>, Error> {
        if !self.nodes.contains_key(&from) {
            return Err(Error::UnknownNode);
        }
        if !self.routing {
            return Err(Error::InvalidBundle {
                rule: "custody transfer needs routing on",
            });
        }
        let mut bundle = Bundle::decode(&payload)?;
        if ipn_node(&bundle.primary.source).is_none() {
            return Err(Error::InvalidBundle {
                rule: "custody transfer needs an ipn source",
            });
        }
        if has_custody_request(&bundle) {
            return Err(Error::InvalidBundle {
                rule: "the bundle already asks for custody",
            });
        }
        let number = bundle.blocks.iter().map(|b| b.number).max().unwrap_or(1) + 1;
        // The request sits before the payload block, which stays last.
        let index = bundle.blocks.len().saturating_sub(1);
        bundle.blocks.insert(
            index,
            CanonicalBlock {
                block_type: CUSTODY_REQUEST_BLOCK,
                number,
                flags: 0,
                crc: CrcType::Crc32C,
                data: custody::request_data(from),
            },
        );
        bundle.encode()
    }

    /// Sets how much custody a node can hold, in bytes of bundles it keeps as custodian. A node with no
    /// capacity set (the default) refuses every custody request, as a node with a full store does.
    pub fn set_custody_capacity(&mut self, node: u64, bytes: u64) {
        self.custody_capacity.insert(node, bytes);
    }

    /// Sets how long a custodian waits for the next custodian's signal before it sends the bundle again,
    /// in milliseconds.
    pub fn set_custody_timeout(&mut self, ms: u64) {
        self.custody_timeout = ms.max(1);
    }

    /// Sets how many hand-offs a custodian makes for one bundle before it gives up. At least one.
    pub fn set_custody_attempts(&mut self, attempts: u32) {
        self.custody_attempts = attempts.max(1);
    }

    /// Counts of custody events so far.
    pub fn custody_stats(&self) -> CustodyStats {
        self.custody_stats
    }

    /// The number of custody copies that `node` holds.
    pub fn custody_copies(&self, node: u64) -> usize {
        self.custody_copies
            .keys()
            .filter(|(n, _)| *n == node)
            .count()
    }

    /// Takes in a custody signal that `custodian` sent to node `at`.
    fn receive_signal(&mut self, at: u64, custodian: u64, signal: CustodySignal) {
        let key = (at, signal.subject);
        if !self.custody_copies.contains_key(&key) {
            return; // no copy here any more, so the signal is stale
        }
        if signal.succeeded || signal.reason == reason::REDUNDANT_RECEPTION {
            // The next custodian has the bundle (or already had it): this node's copy is released.
            self.custody_copies.remove(&key);
            self.drop_custody_holds(at, signal.subject);
            self.custody_stats.released += 1;
            return;
        }
        // A refusal: the bundle is routed again, not through the refusing node. A refusal for a hand-off
        // other than the pending one is recorded but changes nothing else.
        let Some(copy) = self.custody_copies.get_mut(&key) else {
            return;
        };
        if copy.awaiting.is_some_and(|pending| pending != custodian) {
            return;
        }
        copy.refused.push(custodian);
        if copy.awaiting.take().is_some() {
            copy.deadline = None;
            self.custody_stats.rerouted += 1;
            self.push_custody_held(at, signal.subject);
        }
    }

    /// Puts the custody copy of `subject` at `at` back into the held bundles, so that it is routed again.
    fn push_custody_held(&mut self, at: u64, subject: CustodyKey) {
        let Some(copy) = self.custody_copies.get(&(at, subject)) else {
            return;
        };
        let (dest, expires, payload, prev) =
            (copy.dest, copy.expires, copy.payload.clone(), copy.prev);
        let ticket = self.take_ticket();
        self.held.push(Held {
            ticket,
            at,
            dest,
            expires,
            payload,
            wake: None,
            claims: Vec::new(),
            received_from: (prev != at).then_some(prev),
            only_via: None,
            waiting_on: None,
            custody: Some(subject),
        });
    }

    /// Removes the held entries that carry the custody copy of `subject` at `at`, releasing their claims
    /// and their waiting backlog. Used when that custody is released.
    fn drop_custody_holds(&mut self, at: u64, subject: CustodyKey) {
        for h in core::mem::take(&mut self.held) {
            if h.at == at && h.custody == Some(subject) {
                if let Some(key) = h.waiting_on {
                    unwait(&mut self.waiting, key, h.ticket);
                }
                release(
                    &mut self.reserved,
                    &h.claims,
                    evc_of(h.payload.len() as u64),
                );
            } else {
                self.held.push(h);
            }
        }
    }

    /// Handles custody timeouts: a copy whose transfer timed out is sent again, until its attempts run out,
    /// when it is given up. A copy past its lifetime is dropped. Returns whether anything changed.
    fn custody_timers(&mut self) -> bool {
        let now = self.now;
        let mut progressed = false;
        let keys: Vec<(u64, CustodyKey)> = self.custody_copies.keys().copied().collect();
        for key in keys {
            let Some(copy) = self.custody_copies.get(&key) else {
                continue;
            };
            let (expires, deadline, attempts) = (copy.expires, copy.deadline, copy.attempts);
            if now > expires {
                self.custody_copies.remove(&key);
                self.drop_custody_holds(key.0, key.1);
                self.discarded += 1;
                progressed = true;
            } else if deadline.is_some_and(|d| d <= now) {
                progressed = true;
                if attempts >= self.custody_attempts {
                    self.custody_copies.remove(&key);
                    self.custody_stats.failed += 1;
                } else {
                    if let Some(copy) = self.custody_copies.get_mut(&key) {
                        copy.awaiting = None;
                        copy.deadline = None;
                    }
                    self.custody_stats.retransmitted += 1;
                    self.push_custody_held(key.0, key.1);
                }
            }
        }
        progressed
    }

    /// The bytes of the custody copies that node `at` holds: the storage in use.
    fn custody_used(&self, at: u64) -> u64 {
        self.custody_copies
            .iter()
            .filter(|((node, _), _)| *node == at)
            .fold(0u64, |acc, (_, copy)| {
                acc.saturating_add(copy.payload.len() as u64)
            })
    }

    /// Sends a bundle status report about a bundle from node `at`, when the bundle asked for that event and
    /// names an `ipn` report-to endpoint (RFC 9171 §5 and §6.1.1). A report-to of `dtn:none`, or a bundle that
    /// does not ask for the event, sends nothing. The report is an administrative bundle, which asks for no
    /// reports itself, so reports do not report on reports. `primary` and `payload_len` describe the subject.
    fn status_report(
        &mut self,
        at: u64,
        primary: &PrimaryBlock,
        payload_len: u64,
        event: Event,
        reason: u8,
    ) {
        let flags = primary.flags;
        let asked = match event {
            Event::Received => flags & bundle_flags::REPORT_RECEPTION != 0,
            Event::Forwarded => flags & bundle_flags::REPORT_FORWARDING != 0,
            Event::Delivered => flags & bundle_flags::REPORT_DELIVERY != 0,
            Event::Deleted => flags & bundle_flags::REPORT_DELETION != 0,
        };
        if !asked || flags & bundle_flags::ADMIN_RECORD != 0 {
            return;
        }
        let Some(to) = ipn_node(&primary.report_to) else {
            return;
        };
        let report = StatusReport {
            event,
            reason,
            time: (flags & bundle_flags::STATUS_TIME_REQUESTED != 0).then_some(self.now),
            source: primary.source.clone(),
            creation_time: primary.created.time,
            creation_sequence: primary.created.sequence,
            fragment: primary.fragment.map(|f| (f.offset, payload_len)),
        };
        self.seq += 1;
        let lifetime = primary
            .created
            .time
            .saturating_add(primary.lifetime)
            .saturating_sub(self.now)
            .max(1_000);
        let admin = Bundle {
            primary: PrimaryBlock {
                flags: bundle_flags::ADMIN_RECORD,
                crc: CrcType::Crc32C,
                destination: Eid::Ipn {
                    node: to,
                    service: 1,
                },
                source: Eid::Ipn {
                    node: at,
                    service: 1,
                },
                report_to: Eid::Null,
                created: CreationTimestamp {
                    time: self.now.max(1),
                    sequence: self.seq,
                },
                lifetime,
                fragment: None,
            },
            blocks: alloc::vec![CanonicalBlock {
                block_type: BLOCK_TYPE_PAYLOAD,
                number: 1,
                flags: 0,
                crc: CrcType::Crc32C,
                data: report.encode(),
            }],
        };
        if let Ok(bytes) = admin.encode() {
            self.status_reports += 1;
            self.handle_bundle(at, at, bytes);
        }
    }

    /// Bundle status reports sent so far, of all events.
    pub fn status_reports_sent(&self) -> u64 {
        self.status_reports
    }

    /// Sends a custody signal from node `at` to node `to` about the bundle `subject`, as an administrative
    /// record routed like any bundle.
    fn send_signal(&mut self, at: u64, to: u64, subject: CustodyKey, succeeded: bool, reason: u8) {
        let signal = CustodySignal {
            succeeded,
            reason,
            subject,
            signal_time: self.now,
        };
        self.seq += 1;
        let lifetime = self
            .custody_timeout
            .saturating_mul(4)
            .saturating_add(60_000);
        let bundle = Bundle {
            primary: PrimaryBlock {
                flags: bundle_flags::ADMIN_RECORD,
                crc: CrcType::Crc32C,
                destination: Eid::Ipn {
                    node: to,
                    service: 1,
                },
                source: Eid::Ipn {
                    node: at,
                    service: 1,
                },
                report_to: Eid::Null,
                created: CreationTimestamp {
                    time: self.now.max(1),
                    sequence: self.seq,
                },
                lifetime,
                fragment: None,
            },
            blocks: alloc::vec![CanonicalBlock {
                block_type: BLOCK_TYPE_PAYLOAD,
                number: 1,
                flags: 0,
                crc: CrcType::Crc32C,
                data: signal.encode(),
            }],
        };
        if let Ok(bytes) = bundle.encode() {
            self.handle_bundle(at, at, bytes);
        }
    }

    /// Sets the OWLT margin (ms) that routes add to each contact's light time, as SABR does for
    /// the worst-case change in light time during transit.
    pub fn set_owlt_margin(&mut self, ms: u64) {
        self.owlt_margin = ms;
    }

    /// Volume (bytes) of a contact: its rate times its length.
    fn volume_of(c: &Contact) -> u64 {
        c.rate.saturating_mul(c.end - c.start) / 1000
    }

    /// Volume of `c` not yet reserved for bundles routed through it.
    fn available(&self, c: &Contact) -> u64 {
        let reserved = self.reserved.get(&contact_key(c)).copied().unwrap_or(0);
        Self::volume_of(c).saturating_sub(reserved)
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

    /// Partitions `a` from `b` during `[start, end)`: datagrams that start on either direction of
    /// the link in that window are lost, and the link works again afterwards. A short partition
    /// is absorbed by the retransmissions of the sessions; a long one cancels them, and the link
    /// is then learned as failed.
    pub fn partition(&mut self, a: u64, b: u64, start: u64, end: u64) {
        self.fail_link_between(a, b, start, end);
        self.fail_link_between(b, a, start, end);
    }

    /// Splits the nodes of `group_a` from those of `group_b` during `[start, end)`: every link
    /// between the two groups is partitioned, in both directions, while links inside a group carry
    /// traffic as usual. Nodes in neither group are unaffected.
    pub fn partition_sets(&mut self, group_a: &[u64], group_b: &[u64], start: u64, end: u64) {
        for &a in group_a {
            for &b in group_b {
                self.partition(a, b, start, end);
            }
        }
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
            // A failed hand-off of a custody copy is a failed transfer attempt (RFC 5050 §5.12). The copy is
            // routed again only while this node still holds custody and has attempts left; otherwise it is
            // released (already passed on) or given up.
            if let Ok(bundle) = Bundle::decode(&payload)
                && let Some(key) = custody_key(&bundle).filter(|_| has_custody_request(&bundle))
            {
                match self
                    .custody_copies
                    .get(&(at, key))
                    .map(|copy| copy.attempts)
                {
                    None => continue,
                    Some(attempts) if attempts >= self.custody_attempts => {
                        self.custody_copies.remove(&(at, key));
                        self.custody_stats.failed += 1;
                        continue;
                    }
                    Some(_) => {}
                }
            }
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

    /// Tells every node which of its links are open at the current time, from the contact plan: a link
    /// is open inside one of its contacts. A closed link pauses the node's sessions with that peer.
    /// Failed links are not announced, since the protocol must discover them, so a failure still looks
    /// open here.
    fn update_link_states(&mut self) {
        let now = self.now;
        let plan = &self.plan;
        for (&id, node) in self.nodes.iter_mut() {
            let mut peers: Vec<u64> = plan.outgoing(id).map(|c| c.to).collect();
            peers.sort_unstable();
            peers.dedup();
            for peer in peers {
                let open = plan
                    .outgoing(id)
                    .any(|c| c.to == peer && c.start <= now && now < c.end);
                node.set_link_open(peer, open, now);
            }
        }
    }

    /// Does what can be done at the current time: collects datagrams, transmits those that fit
    /// a window, delivers arrivals, and repeats while that produces more work.
    fn settle(&mut self) {
        for _ in 0..16 {
            let mut progressed = false;
            self.update_link_states();
            progressed |= self.apply_plan_changes();
            progressed |= self.harvest_failures();
            progressed |= self.custody_timers();
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
                                let held_back =
                                    if self.rng.percent(self.impairments.reorder_percent) {
                                        self.rng.up_to(self.impairments.reorder_ms)
                                    } else {
                                        0
                                    };
                                let at = finish
                                    .saturating_add(owlt)
                                    .saturating_add(self.rng.up_to(self.impairments.jitter_ms))
                                    .saturating_add(held_back);
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
        let custody_deadlines = self.custody_copies.values().filter_map(|c| c.deadline);
        // A suspicion that lapses frees its link for routing, so a held bundle must be routed then.
        let suspicion_ends = self.suspect.values().copied();
        let held_wakes = self
            .held
            .iter()
            .flat_map(|h| [h.wake, Some(h.expires.saturating_add(1))])
            .flatten();
        // Contact boundaries: a link opening or closing changes which sessions may send.
        let boundaries = self.plan.contacts().iter().flat_map(|c| [c.start, c.end]);
        arrivals
            .chain(timers)
            .chain(windows)
            .chain(plan_changes)
            .chain(held_wakes)
            .chain(custody_deadlines)
            .chain(suspicion_ends)
            .chain(boundaries)
            .filter(|&t| t > now)
            .min()
    }
}

/// Adds `evc` bytes to the reservation of every contact on `hops`, returning their keys.
fn claim(
    reserved: &mut BTreeMap<(u64, u64, u64), u64>,
    hops: &[Hop],
    evc: u64,
) -> Vec<(u64, u64, u64)> {
    let keys: Vec<_> = hops.iter().map(|hop| contact_key(&hop.contact)).collect();
    for key in &keys {
        *reserved.entry(*key).or_insert(0) += evc;
    }
    keys
}

/// The custody identity of a bundle whose source is an `ipn` endpoint.
fn custody_key(bundle: &Bundle) -> Option<CustodyKey> {
    let source = ipn_node(&bundle.primary.source)?;
    Some(CustodyKey {
        source,
        time: bundle.primary.created.time,
        sequence: bundle.primary.created.sequence,
        fragment_offset: bundle.primary.fragment.map(|f| f.offset),
    })
}

/// The node number of an `ipn` endpoint.
fn ipn_node(eid: &Eid) -> Option<u64> {
    match eid {
        Eid::Ipn { node, .. } => Some(*node),
        _ => None,
    }
}

/// Whether a bundle carries a custody request block.
fn has_custody_request(bundle: &Bundle) -> bool {
    bundle
        .blocks
        .iter()
        .any(|b| b.block_type == CUSTODY_REQUEST_BLOCK)
}

/// Names `node` as the current custodian in a bundle's custody request block, and encodes the bundle.
fn rewrite_custodian(bundle: &mut Bundle, node: u64) -> Result<Vec<u8>, Error> {
    for block in bundle
        .blocks
        .iter_mut()
        .filter(|b| b.block_type == CUSTODY_REQUEST_BLOCK)
    {
        block.data = custody::request_data(node);
    }
    bundle.encode()
}

/// Removes the bundle with `ticket` from the waiting backlog of link `key`, dropping the link's entry when
/// it has no bundles left.
fn unwait(waiting: &mut BTreeMap<(u64, u64), BTreeMap<u64, u64>>, key: (u64, u64), ticket: u64) {
    if let Some(entries) = waiting.get_mut(&key) {
        entries.remove(&ticket);
        if entries.is_empty() {
            waiting.remove(&key);
        }
    }
}

/// Removes `evc` bytes from the reservation of each contact in `keys`.
fn release(reserved: &mut BTreeMap<(u64, u64, u64), u64>, keys: &[(u64, u64, u64)], evc: u64) {
    for key in keys {
        if let Some(total) = reserved.get_mut(key) {
            *total = total.saturating_sub(evc);
            if *total == 0 {
                reserved.remove(key);
            }
        }
    }
}

/// Identity of a contact for volume reservations: sender, receiver and start time.
fn contact_key(c: &Contact) -> (u64, u64, u64) {
    (c.from, c.to, c.start)
}

/// Bytes a link of `rate` bytes per second carries in `ms` milliseconds, rounded down.
fn bytes_in_ms(rate: u64, ms: u64) -> u64 {
    u64::try_from(u128::from(rate) * u128::from(ms) / 1000).unwrap_or(u64::MAX)
}

/// Estimated volume consumption of a bundle of `len` bytes: the bytes themselves, plus the
/// convergence-layer overhead, which is 3% of the bundle or 100 bytes, whichever is larger
/// (CCSDS 734.3-B-1 §2.4.3).
fn evc_of(len: u64) -> u64 {
    len + (3 * len).div_ceil(100).max(100)
}

/// Bytes left out of a fragment's payload budget for the fragment header, which is larger than a whole
/// bundle's (fragment offset and total length are added to the primary block).
const FRAGMENT_MARGIN_BYTES: u64 = 64;

/// Bytes taken off a fragment's payload each time it is still too large once encoded.
const FRAGMENT_SHRINK_BYTES: u64 = 16;

/// The largest encoded length `len` whose reservation charge, [`evc_of`], is at most `free`. Zero if
/// nothing fits.
fn largest_charged_within(free: u64) -> u64 {
    if evc_of(0) > free {
        return 0;
    }
    let (mut lo, mut hi) = (0u64, free);
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if evc_of(mid) <= free {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}
