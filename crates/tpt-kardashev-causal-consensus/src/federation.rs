// SPDX-License-Identifier: MIT OR Apache-2.0
//! Federation: hash-chained epoch summaries. A domain seals each epoch of committed commands into
//! a summary that names the previous summary's hash, so a summary cannot be changed, reordered or
//! dropped without the chain breaking.
//!
//! Signatures are not here. The spec's signed summaries need a signature scheme, and that is a
//! licence-checked optional dependency (`tpt-crypto`), not yet chosen. Until then a chain proves
//! integrity against a copy of its head, not authenticity against an adversary.

use alloc::vec::Vec;

use crate::hash::Sha256;

/// A 32-byte SHA-256 value.
pub type Hash = [u8; 32];

/// The hash of an empty chain: the `previous` of the first summary of every domain.
pub const GENESIS: Hash = [0; 32];

/// One sealed epoch of a domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpochSummary {
    /// The domain that sealed the epoch.
    pub domain: u64,
    /// The epoch number, from 0 with no gaps.
    pub epoch: u64,
    /// The hash of the previous summary of this domain, or [`GENESIS`] for epoch 0.
    pub previous: Hash,
    /// The digest of the commands committed in this epoch, in order (see [`digest_commands`]).
    pub commands: Hash,
}

impl EpochSummary {
    /// The hash of this summary, over a fixed-width big-endian encoding of its fields.
    pub fn hash(&self) -> Hash {
        let mut hash = Sha256::new();
        hash.update(b"tpt-kardashev/epoch/v1");
        hash.update(&self.domain.to_be_bytes());
        hash.update(&self.epoch.to_be_bytes());
        hash.update(&self.previous);
        hash.update(&self.commands);
        hash.finalize()
    }
}

/// The digest of a sequence of committed commands: the hash of each command's big-endian encoding,
/// prefixed by the count, so that the empty sequence and the sequence of one zero differ.
pub fn digest_commands(commands: &[u32]) -> Hash {
    let mut hash = Sha256::new();
    hash.update(&(commands.len() as u64).to_be_bytes());
    for command in commands {
        hash.update(&command.to_be_bytes());
    }
    hash.finalize()
}

/// An error in a chain of summaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FederationError {
    /// A summary names a different domain than the chain's.
    WrongDomain {
        /// The position in the sequence.
        position: usize,
    },
    /// The epoch numbers are not 0, 1, 2, ... in order.
    BadEpoch {
        /// The position in the sequence.
        position: usize,
    },
    /// A summary does not name the hash of the one before it.
    BrokenLink {
        /// The position in the sequence.
        position: usize,
    },
}

impl core::fmt::Display for FederationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FederationError::WrongDomain { position } => {
                write!(f, "summary {position} is from another domain")
            }
            FederationError::BadEpoch { position } => {
                write!(f, "summary {position} is out of epoch order")
            }
            FederationError::BrokenLink { position } => {
                write!(f, "summary {position} does not name the previous summary")
            }
        }
    }
}

/// The sealing side of a domain's chain.
#[derive(Clone, Debug)]
pub struct SummaryChain {
    domain: u64,
    summaries: Vec<EpochSummary>,
}

impl SummaryChain {
    /// An empty chain for `domain`.
    pub fn new(domain: u64) -> Self {
        Self {
            domain,
            summaries: Vec::new(),
        }
    }

    /// The hash of the latest summary, or [`GENESIS`] for an empty chain.
    pub fn head(&self) -> Hash {
        self.summaries.last().map_or(GENESIS, EpochSummary::hash)
    }

    /// Seals the next epoch with the commands committed in it, and returns its summary.
    pub fn seal(&mut self, commands: &[u32]) -> EpochSummary {
        let summary = EpochSummary {
            domain: self.domain,
            epoch: self.summaries.len() as u64,
            previous: self.head(),
            commands: digest_commands(commands),
        };
        self.summaries.push(summary);
        summary
    }

    /// The summaries sealed so far, in order.
    pub fn summaries(&self) -> &[EpochSummary] {
        &self.summaries
    }
}

/// Checks a sequence of summaries as one chain of `domain`, starting at epoch 0. Returns the hash
/// of the last summary (the head), or [`GENESIS`] for an empty sequence.
pub fn verify_chain(domain: u64, summaries: &[EpochSummary]) -> Result<Hash, FederationError> {
    let mut head = GENESIS;
    for (position, summary) in summaries.iter().enumerate() {
        if summary.domain != domain {
            return Err(FederationError::WrongDomain { position });
        }
        if summary.epoch != position as u64 {
            return Err(FederationError::BadEpoch { position });
        }
        if summary.previous != head {
            return Err(FederationError::BrokenLink { position });
        }
        head = summary.hash();
    }
    Ok(head)
}
