// SPDX-License-Identifier: MIT OR Apache-2.0
//! Checkpoint with rollback, and a watchdog that checks for progress markers.
//!
//! The owner calls `save` at known-good points and `mark_progress` as work advances. A
//! `Watchdog` checks the progress counter at its own cadence and reports `Stalled` if it has not
//! advanced since the last check. Nothing in this module reads a clock.

use crate::error::Error;

/// A saved state of type `T` with a generation count and a progress counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint<T: Copy> {
    saved: T,
    generation: u64,
    progress: u64,
}

impl<T: Copy> Checkpoint<T> {
    /// A checkpoint whose initial saved state is `initial`.
    pub fn new(initial: T) -> Self {
        Self {
            saved: initial,
            generation: 0,
            progress: 0,
        }
    }

    /// Saves `state` as the new rollback point.
    pub fn save(&mut self, state: T) {
        self.saved = state;
        self.generation = self.generation.saturating_add(1);
    }

    /// The last saved state. Use this to roll back after a detected fault.
    pub fn restore(&self) -> T {
        self.saved
    }

    /// Number of saves so far.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Records that work has advanced.
    pub fn mark_progress(&mut self) {
        self.progress = self.progress.saturating_add(1);
    }

    /// Current progress marker, for a `Watchdog` to read.
    pub fn progress(&self) -> u64 {
        self.progress
    }
}

/// Reports `Stalled` when the progress marker has not advanced since the previous check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Watchdog {
    last_seen: Option<u64>,
}

impl Default for Watchdog {
    fn default() -> Self {
        Self::new()
    }
}

impl Watchdog {
    /// A watchdog that has not yet seen any progress.
    pub fn new() -> Self {
        Self { last_seen: None }
    }

    /// Checks the marker. The first check always passes. Later checks need a larger value than the last.
    pub fn check(&mut self, progress: u64) -> Result<(), Error> {
        match self.last_seen {
            Some(last) if progress <= last => Err(Error::Stalled),
            _ => {
                self.last_seen = Some(progress);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_returns_last_save() {
        let mut c = Checkpoint::new([0u32; 4]);
        c.save([1, 2, 3, 4]);
        c.save([5, 6, 7, 8]);
        assert_eq!(c.restore(), [5, 6, 7, 8]);
        assert_eq!(c.generation(), 2);
    }

    #[test]
    fn watchdog_flags_a_stall() {
        let mut c = Checkpoint::new(0u8);
        let mut wd = Watchdog::new();
        assert_eq!(wd.check(c.progress()), Ok(()));
        c.mark_progress();
        assert_eq!(wd.check(c.progress()), Ok(()));
        assert_eq!(wd.check(c.progress()), Err(Error::Stalled));
        c.mark_progress();
        assert_eq!(wd.check(c.progress()), Ok(()));
    }
}
