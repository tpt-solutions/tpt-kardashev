// SPDX-License-Identifier: MIT OR Apache-2.0
//! Earliest-arrival routing over a contact plan, with the constraints of contact-graph routing.
//!
//! A route is a sequence of contacts. A bundle may wait at a node until a contact opens, and it is
//! forwarded only once it has arrived in full (store and forward). For a bundle that is ready at
//! the source at a given time, the router finds the route that delivers it earliest.
//!
//! The constraints follow CCSDS 734.3-B-1 (SABR, §3.2.6.9). A route is ignored if its projected
//! arrival is after the bundle's expiration time. Arrival is projected with an OWLT margin, the
//! worst-case change in light time during transit. A contact is not used for a bundle whose
//! estimated volume consumption (EVC) exceeds the contact volume still unreserved (the bundle would
//! not fit there). The volume a route reserves is up to the caller: the simulator reserves the EVC
//! on every contact of a route when it hands the bundle on.
//!
//! Not modelled: fragmentation of bundles across contacts, critical bundles sent along every
//! plausible route, excluded neighbours, backlog measured in volume (the backlog is measured in
//! time, through `free_at`), and route lists kept between computations.
//!
//! Dijkstra is exact here. Arrival time never decreases along a route, and waiting is allowed, so
//! a node's earliest arrival is the right label to extend from. Ties are broken by hop count and
//! then by node number, so results are deterministic.

use crate::contact::{Contact, ContactPlan};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

/// One hop of a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hop {
    /// The contact used for this hop.
    pub contact: Contact,
    /// When transmission starts (DTN ms). At least the time the bundle is ready at `contact.from`.
    pub departure: u64,
    /// When the bundle is fully received at `contact.to` (DTN ms), margin included.
    pub arrival: u64,
}

/// A route from source to destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Hops in order. Empty if the source is the destination.
    pub hops: Vec<Hop>,
    /// Arrival time at the destination (DTN ms).
    pub arrival: u64,
}

/// Something that picks a route for a bundle.
pub trait Router {
    /// The earliest-delivering route for a bundle of `size` bytes, ready at `source` at DTN time
    /// `now`, or `None` if no route exists in the plan.
    fn route(
        &self,
        plan: &ContactPlan,
        source: u64,
        destination: u64,
        now: u64,
        size: u64,
    ) -> Option<Route>;
}

/// Limits on the routes for one bundle, as SABR applies them.
#[derive(Clone, Copy)]
pub struct RouteConstraints<'a> {
    /// Expiration time of the bundle (DTN ms). A route that projects an arrival after this is
    /// ignored.
    pub expires: u64,
    /// OWLT margin (ms), added to each contact's light time when projecting arrival.
    pub owlt_margin: u64,
    /// Estimated volume consumption of the bundle (bytes): what it takes from a contact.
    pub evc: u64,
    /// Volume (bytes) still unreserved on a contact. A contact with less than `evc` available is
    /// not used.
    pub available: &'a dyn Fn(&Contact) -> u64,
    /// Nodes that the first hop may not enter: the node a non-critical bundle was received from,
    /// so it is not sent straight back (CCSDS 734.3-B-1 §3.2.5.2). A route whose entry node is
    /// listed is ignored.
    pub excluded: &'a [u64],
}

/// Earliest-arrival routing (see the module documentation).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EarliestArrival;

impl Router for EarliestArrival {
    fn route(
        &self,
        plan: &ContactPlan,
        source: u64,
        destination: u64,
        now: u64,
        size: u64,
    ) -> Option<Route> {
        self.route_with_free_at(plan, source, destination, now, size, &|_| 0)
    }
}

/// Label of a node during the search: the best arrival found there, the hop count of that
/// route, the hop that reached it, and the node before it.
#[derive(Clone, Copy, Debug)]
struct Label {
    arrival: u64,
    hops: u32,
    via: Option<Hop>,
    pred: u64,
}

impl EarliestArrival {
    /// As [`Router::route`], but no contact can carry data before `free_at(contact)`: the time its
    /// transmitter is free, including datagrams already queued for it. A simulator passes its
    /// backlog here, so that a bundle is not routed onto a link that is already full.
    pub fn route_with_free_at(
        &self,
        plan: &ContactPlan,
        source: u64,
        destination: u64,
        now: u64,
        size: u64,
        free_at: &dyn Fn(&Contact) -> u64,
    ) -> Option<Route> {
        let unlimited = |_: &Contact| u64::MAX;
        let constraints = RouteConstraints {
            expires: u64::MAX,
            owlt_margin: 0,
            evc: 0,
            available: &unlimited,
            excluded: &[],
        };
        self.route_constrained(plan, source, destination, now, size, free_at, &constraints)
    }

    /// The earliest-delivering route for a bundle of `size` bytes, subject to `constraints` (see
    /// [`RouteConstraints`]).
    #[allow(clippy::too_many_arguments)]
    pub fn route_constrained(
        &self,
        plan: &ContactPlan,
        source: u64,
        destination: u64,
        now: u64,
        size: u64,
        free_at: &dyn Fn(&Contact) -> u64,
        constraints: &RouteConstraints<'_>,
    ) -> Option<Route> {
        if source == destination {
            return Some(Route {
                hops: Vec::new(),
                arrival: now,
            });
        }
        let mut best: BTreeMap<u64, Label> = BTreeMap::new();
        best.insert(
            source,
            Label {
                arrival: now,
                hops: 0,
                via: None,
                pred: source,
            },
        );
        let mut done: BTreeSet<u64> = BTreeSet::new();

        loop {
            // Settle the unsettled node with the earliest arrival; ties by hop count, then node.
            let next = best
                .iter()
                .filter(|(node, _)| !done.contains(*node))
                .min_by_key(|(node, label)| (label.arrival, label.hops, **node))
                .map(|(node, label)| (*node, label.arrival));
            let Some((node, ready)) = next else { break };
            if node == destination {
                break;
            }
            done.insert(node);
            let hops_here = best.get(&node).map_or(0, |l| l.hops);
            for c in plan.outgoing(node) {
                if c.end <= ready {
                    continue; // the window closed before the bundle was ready
                }
                if (constraints.available)(c) < constraints.evc {
                    continue; // the contact cannot carry the bundle's volume
                }
                if node == source && constraints.excluded.contains(&c.to) {
                    continue; // the first hop may not enter an excluded node
                }
                let departure = ready.max(c.start).max(free_at(c));
                let finish = departure.saturating_add(c.transmit_ms(size));
                if finish > c.end {
                    continue; // the bundle does not fit in this window
                }
                let arrival = finish
                    .saturating_add(c.owlt)
                    .saturating_add(constraints.owlt_margin);
                if arrival > constraints.expires {
                    continue; // the bundle would expire before it arrives
                }
                let hops = hops_here + 1;
                let better = match best.get(&c.to) {
                    None => true,
                    Some(current) => (arrival, hops) < (current.arrival, current.hops),
                };
                if better && !done.contains(&c.to) {
                    let hop = Hop {
                        contact: *c,
                        departure,
                        arrival,
                    };
                    best.insert(
                        c.to,
                        Label {
                            arrival,
                            hops,
                            via: Some(hop),
                            pred: node,
                        },
                    );
                }
            }
        }

        let arrival = best.get(&destination)?.arrival;
        let mut hops = Vec::new();
        let mut node = destination;
        while node != source {
            let label = best.get(&node)?;
            hops.push(label.via?);
            node = label.pred;
        }
        hops.reverse();
        Some(Route { hops, arrival })
    }
}
