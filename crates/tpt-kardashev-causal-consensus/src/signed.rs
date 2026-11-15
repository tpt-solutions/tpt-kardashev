// SPDX-License-Identifier: MIT OR Apache-2.0
//! Signed epoch summaries (feature `crypto`): a domain signs the hash of each summary, so that a copy
//! of the chain can be checked against the domain's public key, not only against itself.
//!
//! The signature scheme is Ed25519, from the `ed25519-compact` crate (MIT, checked by `cargo deny`).
//! The signed message is the 32-byte summary hash, which already commits to the whole summary and so,
//! through the chain, to everything before it.

use ed25519_compact::{KeyPair, PublicKey, Seed, Signature};

use crate::federation::EpochSummary;

/// A domain's key pair, from a 32-byte seed. The seed is the secret; keep it private.
pub fn key_pair_from_seed(seed: [u8; 32]) -> KeyPair {
    KeyPair::from_seed(Seed::new(seed))
}

/// Signs the hash of `summary` with `key`.
pub fn sign_summary(key: &KeyPair, summary: &EpochSummary) -> Signature {
    key.sk.sign(summary.hash(), None)
}

/// Whether `signature` is `public`'s signature of the hash of `summary`.
pub fn verify_summary(public: &PublicKey, summary: &EpochSummary, signature: &Signature) -> bool {
    public.verify(summary.hash(), signature).is_ok()
}
