// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hybrid logical clock (HLC): a timestamp that stays close to physical time, orders events
//! consistently with causality, and never goes backwards on one node.
//!
//! The rules are those of Kulkarni, Demirbas, Madappa, Avva and Leone (2014), written here from
//! that description. The physical clock is supplied by the caller, so the clock is deterministic
//! and `no_std`.

use crate::error::ClockError;

/// An HLC timestamp. Ordering is `(wall_ms, counter, node)`, which is total and agrees with
/// causality: an event that happened before another has a smaller timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    /// The wall-clock part, in milliseconds. At least the physical time when it was issued.
    pub wall_ms: u64,
    /// Orders events that share a `wall_ms`.
    pub counter: u32,
    /// The issuing node. Breaks any remaining tie, so timestamps from different nodes never compare
    /// equal.
    pub node: u64,
}

/// A hybrid logical clock for one node.
///
/// `max_drift_ms` bounds how far ahead of local physical time an accepted remote timestamp may be.
/// The bound is a configured limit, not something the clock derives. A timestamp beyond it is
/// rejected without changing the clock, so one faulty node cannot pull the others' clocks forward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HybridClock {
    node: u64,
    max_drift_ms: u64,
    wall_ms: u64,
    counter: u32,
}

impl HybridClock {
    /// A clock for `node` that accepts remote timestamps at most `max_drift_ms` ahead of physical
    /// time.
    pub fn new(node: u64, max_drift_ms: u64) -> Self {
        Self {
            node,
            max_drift_ms,
            wall_ms: 0,
            counter: 0,
        }
    }

    /// The latest timestamp this clock has issued or accepted.
    pub fn latest(&self) -> Timestamp {
        Timestamp {
            wall_ms: self.wall_ms,
            counter: self.counter,
            node: self.node,
        }
    }

    /// Stamps a local or send event at physical time `physical_ms`.
    pub fn now(&mut self, physical_ms: u64) -> Result<Timestamp, ClockError> {
        let wall = self.wall_ms.max(physical_ms);
        let counter = if wall == self.wall_ms {
            self.counter
                .checked_add(1)
                .ok_or(ClockError::CounterOverflow)?
        } else {
            0
        };
        self.wall_ms = wall;
        self.counter = counter;
        Ok(self.latest())
    }

    /// Stamps a receive event: merges the remote timestamp `remote` into this clock at physical
    /// time `physical_ms`. The result is strictly after both the remote timestamp and every
    /// timestamp this clock issued before. Fails, without changing the clock, when `remote` is
    /// more than `max_drift_ms` ahead of `physical_ms`.
    pub fn receive(
        &mut self,
        remote: Timestamp,
        physical_ms: u64,
    ) -> Result<Timestamp, ClockError> {
        let limit = physical_ms.saturating_add(self.max_drift_ms);
        if remote.wall_ms > limit {
            return Err(ClockError::DriftExceeded {
                ahead_ms: remote.wall_ms - physical_ms,
            });
        }
        let old = self.wall_ms;
        let wall = old.max(remote.wall_ms).max(physical_ms);
        let counter = if wall == old && wall == remote.wall_ms {
            self.counter.max(remote.counter).checked_add(1)
        } else if wall == old {
            self.counter.checked_add(1)
        } else if wall == remote.wall_ms {
            remote.counter.checked_add(1)
        } else {
            Some(0)
        }
        .ok_or(ClockError::CounterOverflow)?;
        self.wall_ms = wall;
        self.counter = counter;
        Ok(self.latest())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_counter_is_an_error_and_changes_nothing() {
        let mut clock = HybridClock::new(1, 0);
        clock.wall_ms = 1_000;
        clock.counter = u32::MAX;
        assert_eq!(clock.now(1_000), Err(ClockError::CounterOverflow));
        assert_eq!(clock.latest().counter, u32::MAX);
        assert_eq!(clock.latest().wall_ms, 1_000);
    }
}
