// SPDX-License-Identifier: MIT OR Apache-2.0
//! Earliest-arrival routing over a contact plan.
//!
//! This is the core idea of contact-graph routing (CGR) without its extensions: a route is a
//! sequence of contacts. A bundle may wait at a node until a contact opens, and it is forwarded
//! only once it has arrived in full (store and forward). The router computes, for a bundle of a
//! given size that is ready at the source at a given time, the route that delivers it earliest.
//!
//! Not modelled: volume and queue limits on contacts, several bundles competing for a contact,
//! multiple-copy or multipath routing, and the Schedule-Aware Bundle Routing (SABR) rules.
//!
//! Dijkstra is exact here. Arrival time never decreases along a route, and waiting is allowed, so
//! a node's earliest arrival is the right label to extend from. Ties are broken by node number, so
//! results are deterministic.

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
    /// When the bundle is fully received at `contact.to` (DTN ms).
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
        if source == destination {
            return Some(Route {
                hops: Vec::new(),
                arrival: now,
            });
        }
        // Best known arrival per node, and the hop that achieved it.
        // node -> (arrival, hop that achieved it, predecessor node)
        let mut best: BTreeMap<u64, (u64, Option<Hop>, u64)> = BTreeMap::new();
        best.insert(source, (now, None, source));
        let mut done: BTreeSet<u64> = BTreeSet::new();

        loop {
            // Settle the unsettled node with the earliest arrival, ties by node number.
            let next = best
                .iter()
                .filter(|(node, _)| !done.contains(*node))
                .min_by_key(|(node, (arrival, _, _))| (*arrival, **node))
                .map(|(node, (arrival, _, _))| (*node, *arrival));
            let Some((node, ready)) = next else { break };
            if node == destination {
                break;
            }
            done.insert(node);
            for c in plan.outgoing(node) {
                if c.end <= ready {
                    continue; // the window closed before the bundle was ready
                }
                let departure = ready.max(c.start).max(free_at(c));
                let finish = departure.saturating_add(c.transmit_ms(size));
                if finish > c.end {
                    continue; // the bundle does not fit in this window
                }
                let arrival = finish.saturating_add(c.owlt);
                let better = match best.get(&c.to) {
                    None => true,
                    Some((current, _, _)) => arrival < *current,
                };
                if better && !done.contains(&c.to) {
                    let hop = Hop {
                        contact: *c,
                        departure,
                        arrival,
                    };
                    best.insert(c.to, (arrival, Some(hop), node));
                }
            }
        }

        let (arrival, _, _) = *best.get(&destination)?;
        let mut hops = Vec::new();
        let mut node = destination;
        while node != source {
            let (_, hop, pred) = *best.get(&node)?;
            hops.push(hop?);
            node = pred;
        }
        hops.reverse();
        Some(Route { hops, arrival })
    }
}
