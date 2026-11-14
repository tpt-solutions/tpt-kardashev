// SPDX-License-Identifier: MIT OR Apache-2.0
//! Consistency levels, each with a latency lower bound from the physics of the links.
//!
//! The spec classifies each operation by the agreement it needs. Only the global level needs a
//! round trip across domains, and the API reports the lower bound so that callers see the delay
//! rather than a hidden one. The bounds here are the ones that follow from one-way delays:
//!
//! - `Commutative` and `Causal` need no agreement, so their bound is zero.
//! - `DomainLinearizable` needs a round trip inside its domain: at least two one-way delays between
//!   the domain's nodes.
//! - `GlobalAgreed` needs a round trip between domains: at least two one-way delays of the slowest
//!   link between domains.
//!
//! These are lower bounds. A protocol that needs more rounds, or a path with more hops, takes
//! longer. The bounds are not a promise about a particular protocol's latency.

/// A consistency level, requested per operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConsistencyLevel {
    /// CRDT-style merge: no agreement is needed, and replicas converge.
    Commutative,
    /// Delivered in causal order, without agreement.
    Causal,
    /// Linearizable within one domain.
    DomainLinearizable,
    /// All domains agree before the operation commits.
    GlobalAgreed,
}

/// The one-way delays of the links that bound the latency, in milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delays {
    /// The slowest one-way delay between two nodes of one domain.
    pub intra_domain_ms: u64,
    /// The slowest one-way delay between two domains.
    pub cross_domain_ms: u64,
}

impl ConsistencyLevel {
    /// Whether the level needs a round trip across domains.
    pub fn needs_cross_domain_round_trip(self) -> bool {
        matches!(self, ConsistencyLevel::GlobalAgreed)
    }

    /// Whether the level needs agreement inside a domain.
    pub fn needs_domain_round_trip(self) -> bool {
        matches!(
            self,
            ConsistencyLevel::DomainLinearizable | ConsistencyLevel::GlobalAgreed
        )
    }

    /// The latency lower bound for this level, in milliseconds, given the link delays. Saturates
    /// instead of overflowing on very large delays.
    pub fn latency_lower_bound_ms(self, delays: Delays) -> u64 {
        match self {
            ConsistencyLevel::Commutative | ConsistencyLevel::Causal => 0,
            ConsistencyLevel::DomainLinearizable => delays.intra_domain_ms.saturating_mul(2),
            ConsistencyLevel::GlobalAgreed => delays.cross_domain_ms.saturating_mul(2),
        }
    }
}
