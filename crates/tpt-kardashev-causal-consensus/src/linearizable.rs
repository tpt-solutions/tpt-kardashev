// SPDX-License-Identifier: MIT OR Apache-2.0
//! A linearizability checker for small histories of a single register.
//!
//! A history is linearizable when its operations can be put in one sequence that (a) respects real
//! time, so an operation that responded before another was invoked comes first, and (b) is a legal
//! register history: each read returns the value of the latest write before it, or the initial
//! value. The checker searches for such a sequence, depth first, and memoises the failed
//! (operations done, register value) states. The search is exponential in the worst case, so the
//! history is limited to [`MAX_OPERATIONS`] operations.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

/// The largest history the checker accepts.
pub const MAX_OPERATIONS: usize = 16;

/// One operation of a register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op<V> {
    /// Writes a value.
    Write(V),
    /// Reads, and observed this value.
    Read(V),
}

/// An operation with the times it was invoked and when its response was received.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event<V> {
    /// When the operation was invoked.
    pub invoke: u64,
    /// When its response arrived. Must not be before `invoke`.
    pub respond: u64,
    /// The operation.
    pub op: Op<V>,
}

/// An error of the checker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinError {
    /// The history has more than [`MAX_OPERATIONS`] operations.
    TooLarge {
        /// The number of operations given.
        len: usize,
    },
    /// An event responded before it was invoked.
    BadInterval {
        /// The index of the event.
        index: usize,
    },
}

impl core::fmt::Display for LinError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LinError::TooLarge { len } => write!(
                f,
                "history of {len} operations is more than the checker's limit of {MAX_OPERATIONS}"
            ),
            LinError::BadInterval { index } => {
                write!(f, "event {index} responds before it is invoked")
            }
        }
    }
}

/// Whether `history`, on a register that starts at `initial`, is linearizable.
pub fn is_linearizable<V: Copy + Ord>(initial: V, history: &[Event<V>]) -> Result<bool, LinError> {
    if history.len() > MAX_OPERATIONS {
        return Err(LinError::TooLarge { len: history.len() });
    }
    for (index, event) in history.iter().enumerate() {
        if event.respond < event.invoke {
            return Err(LinError::BadInterval { index });
        }
    }
    let all: u32 = if history.is_empty() {
        0
    } else {
        (1u32 << history.len()) - 1
    };
    let mut failed: BTreeSet<(u32, V)> = BTreeSet::new();
    Ok(search(history, 0, all, initial, &mut failed))
}

fn search<V: Copy + Ord>(
    history: &[Event<V>],
    done: u32,
    all: u32,
    value: V,
    failed: &mut BTreeSet<(u32, V)>,
) -> bool {
    if done == all {
        return true;
    }
    if failed.contains(&(done, value)) {
        return false;
    }
    for i in 0..history.len() {
        let bit = 1u32 << i;
        if done & bit != 0 {
            continue;
        }
        // Real time: i may come next only if no undone operation responded before i was invoked.
        let minimal = (0..history.len())
            .all(|j| j == i || done & (1u32 << j) != 0 || history[j].respond >= history[i].invoke);
        if !minimal {
            continue;
        }
        let (legal, next) = match history[i].op {
            Op::Write(v) => (true, v),
            Op::Read(observed) => (observed == value, value),
        };
        if legal && search(history, done | bit, all, next, failed) {
            return true;
        }
    }
    failed.insert((done, value));
    false
}

/// A convenience for building histories in tests and examples.
pub fn events<V: Copy>(items: &[(u64, u64, Op<V>)]) -> Vec<Event<V>> {
    items
        .iter()
        .map(|&(invoke, respond, op)| Event {
            invoke,
            respond,
            op,
        })
        .collect()
}
