// SPDX-License-Identifier: MIT OR Apache-2.0
//! Signed epoch summaries: the Ed25519 implementation against RFC 8032 test vector 1, and the
//! signing of a chain with rejection of any change, another key, or a signature moved to another
//! summary.
#![cfg(all(feature = "crypto", feature = "alloc"))]
#![allow(clippy::expect_used)]

use tpt_kardashev_causal_consensus::federation::{EpochSummary, SummaryChain, digest_commands};
use tpt_kardashev_causal_consensus::signed::{key_pair_from_seed, sign_summary, verify_summary};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn seed(s: &str) -> [u8; 32] {
    hex(s).try_into().expect("32 bytes")
}

#[test]
fn ed25519_matches_rfc_8032_test_vector_1() {
    // Secret key (seed) and public key of RFC 8032, section 7.1, test 1 (empty message).
    let secret = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    let public = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    let key = key_pair_from_seed(seed(secret));
    assert_eq!(key.pk.as_ref(), hex(public).as_slice());
}

#[test]
fn a_signed_chain_verifies_and_rejects_every_change() {
    let key = key_pair_from_seed(seed(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
    ));
    let mut chain = SummaryChain::new(4);
    for e in 0..4u32 {
        chain.seal(&[e, e + 1]);
    }
    let summaries: Vec<EpochSummary> = chain.summaries().to_vec();
    let signatures: Vec<_> = summaries.iter().map(|s| sign_summary(&key, s)).collect();

    // Every summary verifies against the domain's public key.
    for (s, sig) in summaries.iter().zip(&signatures) {
        assert!(verify_summary(&key.pk, s, sig));
    }

    // A changed commitment does not verify under its old signature.
    let mut changed = summaries[1];
    changed.commands = digest_commands(&[999]);
    assert!(!verify_summary(&key.pk, &changed, &signatures[1]));

    // Another domain's key does not verify.
    let other = key_pair_from_seed([7u8; 32]);
    assert!(!verify_summary(&other.pk, &summaries[1], &signatures[1]));

    // A signature moved to another epoch does not verify.
    assert!(!verify_summary(&key.pk, &summaries[2], &signatures[1]));
}
