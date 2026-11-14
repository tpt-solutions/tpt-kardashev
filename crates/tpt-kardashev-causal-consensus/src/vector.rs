// SPDX-License-Identifier: MIT OR Apache-2.0
//! Vector clocks over a fixed set of `N` nodes, for deciding whether two events are causally
//! ordered or concurrent. Stored inline, so the clock needs no heap.

use core::cmp::Ordering;

use crate::error::ClockError;

/// How two vector clocks relate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CausalRelation {
    /// The first happened before the second: every entry is at most the second's, and one is less.
    Before,
    /// The first happened after the second.
    After,
    /// The two are the same.
    Equal,
    /// Neither happened before the other.
    Concurrent,
}

/// A vector clock over `N` nodes, indexed `0..N`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VectorClock<const N: usize> {
    entries: [u64; N],
}

impl<const N: usize> Default for VectorClock<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> VectorClock<N> {
    /// The zero clock.
    pub const fn new() -> Self {
        Self { entries: [0; N] }
    }

    /// The number of nodes.
    pub const fn dimension(&self) -> usize {
        N
    }

    /// The count for node `index`, or `None` when it is out of range.
    pub fn get(&self, index: usize) -> Option<u64> {
        self.entries.get(index).copied()
    }

    /// Records a local event at node `index`.
    pub fn tick(&mut self, index: usize) -> Result<(), ClockError> {
        let entry = self
            .entries
            .get_mut(index)
            .ok_or(ClockError::NodeOutOfRange { index })?;
        *entry = entry.checked_add(1).ok_or(ClockError::CounterOverflow)?;
        Ok(())
    }

    /// Takes the entry-wise maximum with `other`, as on receiving a message.
    pub fn merge(&mut self, other: &Self) {
        for (mine, theirs) in self.entries.iter_mut().zip(other.entries.iter()) {
            *mine = (*mine).max(*theirs);
        }
    }

    /// How this clock relates to `other`.
    pub fn relation(&self, other: &Self) -> CausalRelation {
        let mut less = false;
        let mut greater = false;
        for (mine, theirs) in self.entries.iter().zip(other.entries.iter()) {
            match mine.cmp(theirs) {
                Ordering::Less => less = true,
                Ordering::Greater => greater = true,
                Ordering::Equal => {}
            }
        }
        match (less, greater) {
            (false, false) => CausalRelation::Equal,
            (true, false) => CausalRelation::Before,
            (false, true) => CausalRelation::After,
            (true, true) => CausalRelation::Concurrent,
        }
    }
}
