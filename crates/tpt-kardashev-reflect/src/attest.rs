// SPDX-License-Identifier: MIT OR Apache-2.0
//! The attestation path for replacing the invariant. The invariant is fixed by [`UpdateGate`]; the
//! only way to change it is here, and only when an external authority has signed the replacement.
//!
//! The authority signs a message that names the invariant being replaced, the replacement, and the
//! log position at which it takes effect, so an attestation cannot be reused for another replacement
//! or another point in the log. The current policy must also still be contained in the replacement,
//! shown by a certificate, so that the policy is never left outside the invariant it is now held to.
//!
//! The authority's key is fixed when the gate is built. Choosing that key, and any rotation of it, are
//! outside this crate.

use ed25519_compact::{PublicKey, Signature};

use crate::error::ReflectError;
use crate::gate::{GENESIS, Hash, Receipt, UpdateGate, digest_constraints};
use crate::hash::Sha256;
use crate::invariant::{Certificate, Constraints};
use crate::verify::verify;

/// The message the authority signs: a domain tag, the digest of the invariant being replaced, the
/// digest of the replacement, and the log position it takes effect at.
pub fn replacement_message(replaced: &Hash, replacement: &Hash, position: u64) -> Hash {
    let mut hash = Sha256::new();
    hash.update(b"tpt-kardashev/reflect/replace/v1");
    hash.update(replaced);
    hash.update(replacement);
    hash.update(&position.to_be_bytes());
    hash.finalize()
}

/// An update gate whose invariant can be replaced, once, under an attestation from one authority.
pub struct AttestedGate {
    gate: UpdateGate,
    authority: PublicKey,
}

impl AttestedGate {
    /// A gate for `invariant` and `policy` (contained, shown by `certificate`), replaceable only under
    /// attestations by `authority`.
    pub fn new(
        invariant: Constraints,
        policy: Constraints,
        certificate: &Certificate,
        authority: PublicKey,
    ) -> Result<Self, ReflectError> {
        Ok(Self {
            gate: UpdateGate::new(invariant, policy, certificate)?,
            authority,
        })
    }

    /// The underlying gate, for reading its invariant, policy and log.
    pub fn gate(&self) -> &UpdateGate {
        &self.gate
    }

    /// The attestation authority's public key.
    pub fn authority(&self) -> &PublicKey {
        &self.authority
    }

    /// Submits an ordinary update, as [`UpdateGate::submit`] does.
    pub fn submit(
        &mut self,
        update: Constraints,
        certificate: &Certificate,
    ) -> Result<Receipt, ReflectError> {
        self.gate.submit(update, certificate)
    }

    /// Replaces the invariant with `replacement`, if `attestation` is the authority's signature of the
    /// replacement at the next log position, and `policy_in_replacement` shows the current policy is
    /// contained in it. Otherwise nothing changes and the refusal is recorded in the log.
    pub fn replace_invariant(
        &mut self,
        replacement: Constraints,
        policy_in_replacement: &Certificate,
        attestation: &Signature,
    ) -> Result<Receipt, ReflectError> {
        let replaced = self.gate.invariant_digest();
        let replacement_digest = digest_constraints(&replacement);
        let position = self.gate.receipts().len() as u64;
        let message = replacement_message(&replaced, &replacement_digest, position);
        if self.authority.verify(message, attestation).is_err() {
            self.gate
                .record_refusal(replacement_digest, ReflectError::BadAttestation);
            return Err(ReflectError::BadAttestation);
        }
        if let Err(reason) = verify(&replacement, self.gate.policy(), policy_in_replacement) {
            self.gate.record_refusal(replacement_digest, reason);
            return Err(reason);
        }
        Ok(self.gate.install_invariant(replacement, replacement_digest))
    }

    /// The hash of the log's head, or the genesis value for an empty log.
    pub fn head(&self) -> Hash {
        self.gate.receipts().last().map_or(GENESIS, |r| r.hash)
    }

    /// The log's receipts, as in [`UpdateGate::receipts`].
    pub fn receipts(&self) -> &[Receipt] {
        self.gate.receipts()
    }
}
