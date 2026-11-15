// SPDX-License-Identifier: MIT OR Apache-2.0
//! The update gate: the only way the policy changes. Each submitted update is checked against the
//! invariant with its certificate. Accepted and rejected submissions are both recorded in a
//! hash-chained log of receipts, so the record of attempts cannot be edited without breaking the chain.
//!
//! The invariant is fixed when the gate is built. There is no method that changes it. Replacing the
//! invariant is a design question for an external attestation path (see the README), and is not
//! done here.
//!
//! What the audit proves: the receipts are in sequence, each names the hash of the one before, and
//! each receipt's hash matches its contents. It cannot re-check the updates themselves, because the
//! log keeps only digests of them; keep the updates to re-verify them.

use alloc::vec::Vec;

use crate::error::ReflectError;
use crate::hash::{Sha256, sha256};
use crate::invariant::{Certificate, Constraints};
use crate::rational::Rational;
use crate::verify::verify;

/// A 32-byte SHA-256 value.
pub type Hash = [u8; 32];

/// The hash before the first receipt.
pub const GENESIS: Hash = [0; 32];

/// What the gate decided about one submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The update was contained in the invariant, and is now the policy.
    Accepted,
    /// The invariant was replaced, under an attestation by the authority.
    InvariantReplaced,
    /// The update was refused, for the reason given.
    Rejected(ReflectError),
}

/// A record of one submission, chained to the one before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// The position in the log, from 0.
    pub sequence: u64,
    /// The hash of the receipt before this one, or [`GENESIS`].
    pub previous: Hash,
    /// The digest of the submitted update.
    pub update_digest: Hash,
    /// What the gate decided.
    pub outcome: Outcome,
    /// The hash of this receipt, over all the fields above.
    pub hash: Hash,
}

/// An error in an audit log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuditError {
    /// The receipt at this position does not carry the sequence number its position implies.
    BadSequence {
        /// The position of the receipt.
        position: usize,
    },
    /// The receipt at this position does not name the hash of the receipt before it.
    BrokenLink {
        /// The position of the receipt.
        position: usize,
    },
    /// The receipt at this position does not hash to its own stored hash.
    BadHash {
        /// The position of the receipt.
        position: usize,
    },
}

impl core::fmt::Display for AuditError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AuditError::BadSequence { position } => {
                write!(f, "receipt {position} is out of sequence")
            }
            AuditError::BrokenLink { position } => {
                write!(f, "receipt {position} does not follow the one before it")
            }
            AuditError::BadHash { position } => {
                write!(f, "receipt {position} does not match its hash")
            }
        }
    }
}

fn absorb_rational(hash: &mut Sha256, value: &Rational) {
    hash.update(&value.numerator().to_be_bytes());
    hash.update(&value.denominator().to_be_bytes());
}

/// The digest of a constraint system: its dimension, its rows, and each rational exactly.
pub fn digest_constraints(system: &Constraints) -> Hash {
    let mut hash = Sha256::new();
    hash.update(b"tpt-kardashev/reflect/update/v1");
    hash.update(&(system.dimension() as u64).to_be_bytes());
    hash.update(&(system.rows().len() as u64).to_be_bytes());
    for row in system.rows() {
        for coefficient in &row.coefficients {
            absorb_rational(&mut hash, coefficient);
        }
        absorb_rational(&mut hash, &row.bound);
    }
    hash.finalize()
}

fn receipt_hash(sequence: u64, previous: &Hash, update_digest: &Hash, outcome: &Outcome) -> Hash {
    let mut hash = Sha256::new();
    hash.update(b"tpt-kardashev/reflect/receipt/v1");
    hash.update(&sequence.to_be_bytes());
    hash.update(previous);
    hash.update(update_digest);
    match outcome {
        Outcome::Accepted => hash.update(&[1u8]),
        Outcome::InvariantReplaced => hash.update(&[2u8]),
        Outcome::Rejected(reason) => {
            hash.update(&[0u8]);
            hash.update(reason_code(*reason).as_bytes());
        }
    }
    hash.finalize()
}

/// A stable name for each refusal reason, so that receipts do not depend on the enum's layout.
fn reason_code(reason: ReflectError) -> &'static str {
    match reason {
        ReflectError::ZeroDenominator => "zero-denominator",
        ReflectError::Overflow => "overflow",
        ReflectError::ShapeMismatch => "shape-mismatch",
        ReflectError::DimensionMismatch => "dimension-mismatch",
        ReflectError::NegativeMultiplier { .. } => "negative-multiplier",
        ReflectError::CoefficientMismatch { .. } => "coefficient-mismatch",
        ReflectError::BoundExceeded { .. } => "bound-exceeded",
        ReflectError::CertificateShape => "certificate-shape",
        ReflectError::BadAttestation => "bad-attestation",
    }
}

/// The gate: a fixed invariant, the current policy, and the log of submissions.
#[derive(Clone, Debug)]
pub struct UpdateGate {
    invariant: Constraints,
    policy: Constraints,
    receipts: Vec<Receipt>,
}

impl UpdateGate {
    /// A gate for `invariant`, starting from `policy`. The starting policy must itself be contained,
    /// shown by `certificate`; otherwise the gate is not built and the reason is returned.
    pub fn new(
        invariant: Constraints,
        policy: Constraints,
        certificate: &Certificate,
    ) -> Result<Self, ReflectError> {
        verify(&invariant, &policy, certificate)?;
        Ok(Self {
            invariant,
            policy,
            receipts: Vec::new(),
        })
    }

    /// The invariant the gate enforces. It cannot be changed through the gate.
    pub fn invariant(&self) -> &Constraints {
        &self.invariant
    }

    /// The current policy: the last accepted update, or the starting policy.
    pub fn policy(&self) -> &Constraints {
        &self.policy
    }

    /// The log of submissions, accepted and refused, in order.
    pub fn receipts(&self) -> &[Receipt] {
        &self.receipts
    }

    /// The hash of the latest receipt, or [`GENESIS`] when nothing was submitted.
    pub fn head(&self) -> Hash {
        self.receipts.last().map_or(GENESIS, |r| r.hash)
    }

    /// Records a submission that the attestation path refused, and returns the receipt. Crate-private:
    /// only [`crate::attest::AttestedGate`] calls it.
    pub(crate) fn record_refusal(&mut self, update_digest: Hash, reason: ReflectError) -> Receipt {
        self.push(update_digest, Outcome::Rejected(reason))
    }

    /// Replaces the invariant with one that the attestation path has checked, and records the
    /// replacement. Crate-private: only [`crate::attest::AttestedGate`] calls it, after verifying the
    /// authority's signature and the containment of the current policy.
    pub(crate) fn install_invariant(&mut self, invariant: Constraints, digest: Hash) -> Receipt {
        self.invariant = invariant;
        self.push(digest, Outcome::InvariantReplaced)
    }

    fn push(&mut self, update_digest: Hash, outcome: Outcome) -> Receipt {
        let sequence = self.receipts.len() as u64;
        let previous = self.head();
        let receipt = Receipt {
            sequence,
            previous,
            update_digest,
            outcome,
            hash: receipt_hash(sequence, &previous, &update_digest, &outcome),
        };
        self.receipts.push(receipt);
        receipt
    }

    /// The digest of the current invariant, as an attestation names it.
    pub(crate) fn invariant_digest(&self) -> Hash {
        digest_constraints(&self.invariant)
    }

    /// Submits an update with its certificate. If the certificate shows the update contained in the
    /// invariant, the update becomes the policy and an accepted receipt is recorded. Otherwise nothing
    /// changes, a rejected receipt is recorded, and the reason is returned.
    pub fn submit(
        &mut self,
        update: Constraints,
        certificate: &Certificate,
    ) -> Result<Receipt, ReflectError> {
        let sequence = self.receipts.len() as u64;
        let previous = self.head();
        let update_digest = digest_constraints(&update);
        let (outcome, result) = match verify(&self.invariant, &update, certificate) {
            Ok(()) => (Outcome::Accepted, Ok(())),
            Err(reason) => (Outcome::Rejected(reason), Err(reason)),
        };
        let receipt = Receipt {
            sequence,
            previous,
            update_digest,
            outcome,
            hash: receipt_hash(sequence, &previous, &update_digest, &outcome),
        };
        self.receipts.push(receipt);
        match result {
            Ok(()) => {
                self.policy = update;
                Ok(receipt)
            }
            Err(reason) => Err(reason),
        }
    }
}

/// Checks a log of receipts: positions, links and hashes. Returns the head, or [`GENESIS`] for an
/// empty log.
pub fn verify_audit(receipts: &[Receipt]) -> Result<Hash, AuditError> {
    let mut previous = GENESIS;
    for (position, receipt) in receipts.iter().enumerate() {
        if receipt.sequence != position as u64 {
            return Err(AuditError::BadSequence { position });
        }
        if receipt.previous != previous {
            return Err(AuditError::BrokenLink { position });
        }
        let expected = receipt_hash(
            receipt.sequence,
            &receipt.previous,
            &receipt.update_digest,
            &receipt.outcome,
        );
        if receipt.hash != expected {
            return Err(AuditError::BadHash { position });
        }
        previous = receipt.hash;
    }
    Ok(previous)
}

/// The SHA-256 of a byte string, for callers that keep their own digests of updates.
pub fn hash_bytes(data: &[u8]) -> Hash {
    sha256(data)
}
