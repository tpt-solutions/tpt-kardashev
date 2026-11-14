// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hash-chained epoch summaries: an intact chain verifies, and changing, reordering, dropping or
//! misattributing any summary is detected.
#![cfg(feature = "alloc")]

use tpt_kardashev_causal_consensus::federation::{
    EpochSummary, FederationError, GENESIS, SummaryChain, digest_commands, verify_chain,
};

fn sealed(domain: u64, epochs: usize) -> Vec<EpochSummary> {
    let mut chain = SummaryChain::new(domain);
    for e in 0..epochs {
        chain.seal(&[e as u32, e as u32 + 1]);
    }
    chain.summaries().to_vec()
}

#[test]
fn an_intact_chain_verifies_and_its_head_is_the_last_hash() {
    let summaries = sealed(7, 10);
    let head = verify_chain(7, &summaries).expect("intact");
    assert_eq!(head, summaries.last().expect("last").hash());
}

#[test]
fn an_empty_chain_verifies_to_genesis() {
    assert_eq!(verify_chain(7, &[]), Ok(GENESIS));
    assert_eq!(SummaryChain::new(7).head(), GENESIS);
}

#[test]
fn the_chain_head_moves_with_each_seal() {
    let mut chain = SummaryChain::new(1);
    let mut heads = vec![chain.head()];
    for e in 0..5u32 {
        chain.seal(&[e]);
        heads.push(chain.head());
    }
    for pair in heads.windows(2) {
        assert_ne!(pair[0], pair[1]);
    }
}

#[test]
fn changing_the_commands_of_one_epoch_breaks_the_chain_at_the_next() {
    let mut summaries = sealed(7, 6);
    summaries[2].commands = digest_commands(&[999]);
    // The change itself is at position 2 (its own hash changed, so position 3 names a stale hash).
    assert_eq!(
        verify_chain(7, &summaries),
        Err(FederationError::BrokenLink { position: 3 })
    );
}

#[test]
fn reordering_two_summaries_is_detected() {
    let mut summaries = sealed(7, 5);
    summaries.swap(1, 2);
    assert!(matches!(
        verify_chain(7, &summaries),
        Err(FederationError::BadEpoch { .. })
    ));
}

#[test]
fn dropping_a_summary_is_detected() {
    let mut summaries = sealed(7, 5);
    summaries.remove(2);
    assert!(verify_chain(7, &summaries).is_err());
}

#[test]
fn a_summary_from_another_domain_is_detected() {
    let mut summaries = sealed(7, 3);
    summaries[1].domain = 8;
    assert_eq!(
        verify_chain(7, &summaries),
        Err(FederationError::WrongDomain { position: 1 })
    );
}

#[test]
fn the_commands_digest_distinguishes_empty_from_zero() {
    assert_ne!(digest_commands(&[]), digest_commands(&[0]));
    assert_ne!(digest_commands(&[1, 2]), digest_commands(&[2, 1]));
}
