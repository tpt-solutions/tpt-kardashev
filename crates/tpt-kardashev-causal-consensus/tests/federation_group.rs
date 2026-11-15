// SPDX-License-Identifier: MIT OR Apache-2.0
//! The federation as a whole: accept rounds across every domain, the quorum rule for one key, sealing
//! and verifying each domain's chain.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_causal_consensus::federation::{EpochSummary, FederationError};
use tpt_kardashev_causal_consensus::global::{Federation, Proposal};

fn proposal(id: u64, key: u32, op: u32) -> Proposal<u32> {
    Proposal { id, key, op }
}

#[test]
fn a_majority_is_the_smallest_number_of_domains_above_half() {
    let expected = [(1, 1), (2, 2), (3, 2), (4, 3), (5, 3), (6, 4)];
    for (size, majority) in expected {
        assert_eq!(
            Federation::<u32>::new(size).majority(),
            majority,
            "size {size}"
        );
    }
}

#[test]
fn a_proposal_on_a_fresh_key_is_committed_by_every_domain() {
    let mut federation = Federation::new(5);
    let round = federation.propose(&proposal(1, 7, 100));
    assert_eq!(round.accepted, 5);
    assert_eq!(round.rejected, 0);
    assert!(round.committed);
    for id in 0..5 {
        assert_eq!(
            federation.domain(id).expect("domain").accepted(7),
            Some(&100)
        );
    }
}

#[test]
fn a_conflicting_operation_for_the_same_key_cannot_commit() {
    let mut federation = Federation::new(5);
    assert!(federation.propose(&proposal(1, 7, 100)).committed);
    let round = federation.propose(&proposal(2, 7, 200));
    assert_eq!(
        round.accepted, 0,
        "every domain already holds another operation for key 7"
    );
    assert!(!round.committed);
    for id in 0..5 {
        assert_eq!(
            federation.domain(id).expect("domain").accepted(7),
            Some(&100)
        );
    }
}

#[test]
fn the_same_operation_again_is_accepted_and_recorded_once() {
    let mut federation = Federation::new(3);
    federation.propose(&proposal(1, 7, 100));
    let round = federation.propose(&proposal(1, 7, 100));
    assert!(round.committed);
    for id in 0..3 {
        assert_eq!(
            federation.domain(id).expect("domain").accepted_ids(),
            &[1],
            "domain {id}"
        );
    }
}

#[test]
fn different_keys_are_independent() {
    let mut federation = Federation::new(3);
    assert!(federation.propose(&proposal(1, 1, 10)).committed);
    assert!(federation.propose(&proposal(2, 2, 20)).committed);
    assert_eq!(federation.domain(0).expect("domain").accepted(1), Some(&10));
    assert_eq!(federation.domain(0).expect("domain").accepted(2), Some(&20));
}

#[test]
fn an_empty_federation_commits_nothing() {
    let mut federation = Federation::<u32>::new(0);
    let round = federation.propose(&proposal(1, 1, 1));
    assert!(!round.committed);
    assert_eq!(round.accepted, 0);
}

#[test]
fn sealed_epochs_verify_and_a_changed_copy_does_not() {
    let mut federation = Federation::new(3);
    federation.propose(&proposal(1, 1, 10));
    federation.propose(&proposal(2, 2, 20));
    federation.seal_epoch();
    federation.propose(&proposal(3, 3, 30));
    federation.seal_epoch();
    federation.verify().expect("chains verify");

    let chain = federation.domain(0).expect("domain").chain();
    assert_eq!(chain.summaries().len(), 2);
    let mut altered: Vec<EpochSummary> = chain.summaries().to_vec();
    altered[1].previous[0] ^= 1;
    assert!(matches!(
        tpt_kardashev_causal_consensus::federation::verify_chain(0, &altered),
        Err(FederationError::BrokenLink { .. })
    ));
}

#[test]
fn sealing_twice_without_new_proposals_adds_an_empty_epoch_and_still_verifies() {
    let mut federation = Federation::new(3);
    federation.propose(&proposal(1, 1, 10));
    federation.seal_epoch();
    federation.seal_epoch();
    federation.verify().expect("chains verify");
    assert_eq!(
        federation
            .domain(1)
            .expect("domain")
            .chain()
            .summaries()
            .len(),
        2
    );
}
