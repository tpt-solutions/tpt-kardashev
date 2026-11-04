// SPDX-License-Identifier: MIT OR Apache-2.0
//! N-modular redundancy with majority voting and scrub-on-mismatch.
//!
//! `Nmr<T, N>` holds `N` replicas of a value. A value is the voted result if a strict majority
//! of replicas agree on it. `Tmr<T>` is `Nmr<T, 3>`. Odd `N` is recommended: with even `N`, a
//! tie produces `NoMajority`.
//!
//! Typestate: `vote` returns `Voted<T>`, and only voting creates one. A single replica read
//! returns `Unvoted<T>`. Code that must act only on agreed values takes `Voted<T>`.

use crate::error::Error;

/// A value produced by majority voting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voted<T>(T);

impl<T: Copy> Voted<T> {
    /// The agreed value.
    pub fn value(&self) -> T {
        self.0
    }
}

/// A value read from one replica, without voting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unvoted<T>(T);

impl<T: Copy> Unvoted<T> {
    /// The raw replica value.
    pub fn value(&self) -> T {
        self.0
    }
}

/// `N` redundant replicas of a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nmr<T, const N: usize> {
    replicas: [T; N],
}

/// Triple modular redundancy.
pub type Tmr<T> = Nmr<T, 3>;

impl<T: Copy + PartialEq, const N: usize> Nmr<T, N> {
    /// All replicas set to `value`.
    pub fn new(value: T) -> Self {
        Self {
            replicas: [value; N],
        }
    }

    /// Replicas as given.
    pub fn from_replicas(replicas: [T; N]) -> Self {
        Self { replicas }
    }

    /// Writes `value` to every replica.
    pub fn set(&mut self, value: T) {
        self.replicas = [value; N];
    }

    /// The raw value of replica `index`.
    pub fn replica(&self, index: usize) -> Result<Unvoted<T>, Error> {
        self.replicas
            .get(index)
            .map(|&v| Unvoted(v))
            .ok_or(Error::IndexOutOfRange { index })
    }

    /// Overwrites replica `index`. Used by fault injection and by callers that repair a replica.
    pub fn set_replica(&mut self, index: usize, value: T) -> Result<(), Error> {
        match self.replicas.get_mut(index) {
            Some(slot) => {
                *slot = value;
                Ok(())
            }
            None => Err(Error::IndexOutOfRange { index }),
        }
    }

    /// The value held by a strict majority of replicas, if any.
    pub fn majority(&self) -> Option<T> {
        self.replicas
            .iter()
            .copied()
            .find(|&candidate| self.replicas.iter().filter(|&&r| r == candidate).count() > N / 2)
    }

    /// Votes without modifying the replicas.
    pub fn vote(&self) -> Result<Voted<T>, Error> {
        self.majority().map(Voted).ok_or(Error::NoMajority)
    }

    /// Rewrites every replica that disagrees with the majority. Returns how many were rewritten.
    pub fn scrub(&mut self) -> Result<usize, Error> {
        let agreed = self.majority().ok_or(Error::NoMajority)?;
        let mut repaired = 0;
        for slot in self.replicas.iter_mut() {
            if *slot != agreed {
                *slot = agreed;
                repaired += 1;
            }
        }
        Ok(repaired)
    }

    /// Votes and scrubs: the usual read path for a protected value.
    pub fn read(&mut self) -> Result<Voted<T>, Error> {
        let agreed = self.majority().ok_or(Error::NoMajority)?;
        self.scrub()?;
        Ok(Voted(agreed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn votes_past_one_corrupt_replica() {
        let mut t = Tmr::new(42u32);
        t.set_replica(1, 7).expect("in range");
        assert_eq!(t.vote().expect("majority").value(), 42);
        assert_eq!(t.scrub(), Ok(1));
        assert_eq!(t.replica(1).expect("in range").value(), 42);
    }

    #[test]
    fn three_different_values_have_no_majority() {
        let mut t = Tmr::from_replicas([1u8, 2, 3]);
        assert_eq!(t.vote(), Err(Error::NoMajority));
        assert_eq!(t.read(), Err(Error::NoMajority));
    }

    #[test]
    fn exhaustive_single_corruption_never_changes_the_vote() {
        // For every value and every single-replica corruption, the vote is unchanged.
        for v in 0..=255u8 {
            for pos in 0..3 {
                for bad in 0..=255u8 {
                    let mut t = Tmr::new(v);
                    t.set_replica(pos, bad).expect("in range");
                    assert_eq!(
                        t.vote().expect("majority").value(),
                        v,
                        "v {v} pos {pos} bad {bad}"
                    );
                }
            }
        }
    }

    #[test]
    fn five_way_redundancy_tolerates_two_corruptions() {
        let mut n = Nmr::<u16, 5>::new(0xBEEF);
        n.set_replica(0, 1).expect("ok");
        n.set_replica(4, 2).expect("ok");
        assert_eq!(n.vote().expect("majority").value(), 0xBEEF);
        assert_eq!(n.scrub(), Ok(2));
    }

    #[test]
    fn replica_index_is_checked() {
        let t = Tmr::new(0u8);
        assert_eq!(t.replica(3), Err(Error::IndexOutOfRange { index: 3 }));
    }
}
