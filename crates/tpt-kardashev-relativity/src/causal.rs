// SPDX-License-Identifier: MIT OR Apache-2.0
//! Causal predicates over events.
//!
//! `precedes` is the non-strict causal order: reflexive, antisymmetric up to tolerance and
//! transitive. It holds when `b` lies in the future causal cone of `a`, including its boundary.

use crate::event::{Event, IntervalKind, interval};

/// Neutral causal-order trait, consumed by other crates in this repository (spec §5.2).
pub trait CausalOrder {
    /// True if `self` can causally influence `other`.
    fn precedes(&self, other: &Self) -> bool;
}

impl CausalOrder for Event {
    fn precedes(&self, other: &Self) -> bool {
        let dt = other.t.0 - self.t.0;
        dt >= 0.0 && interval(self, other).kind() != IntervalKind::Spacelike
    }
}

/// True if `p` lies on or inside the double light cone of `apex` (past or future).
pub fn light_cone_contains(apex: &Event, p: &Event) -> bool {
    interval(apex, p).kind() != IntervalKind::Spacelike
}

/// True if `a` and `b` are spacelike separated, so neither can influence the other.
pub fn causally_disconnected(a: &Event, b: &Event) -> bool {
    interval(a, b).kind() == IntervalKind::Spacelike
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{C, Metres, Seconds};

    fn ev(t: f64, x: f64) -> Event {
        Event::new(Seconds(t), [Metres(x), Metres(0.0), Metres(0.0)])
    }

    #[test]
    fn future_cone_and_disconnection() {
        let o = ev(0.0, 0.0);
        assert!(o.precedes(&ev(1.0, 0.0)));
        assert!(o.precedes(&ev(1.0, C)));
        assert!(!o.precedes(&ev(1.0, 2.0 * C)));
        assert!(!o.precedes(&ev(-1.0, 0.0)));
        assert!(causally_disconnected(&o, &ev(0.0, 5.0)));
        assert!(light_cone_contains(&o, &ev(-1.0, C)));
        assert!(!light_cone_contains(&o, &ev(0.0, 5.0)));
    }
}
