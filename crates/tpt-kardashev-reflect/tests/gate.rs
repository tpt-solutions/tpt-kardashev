// SPDX-License-Identifier: MIT OR Apache-2.0
//! The update gate and its audit log: updates change the policy only when a certificate verifies;
//! refusals are recorded with their reasons; the invariant never changes; and the audit detects
//! reordering, removal, rewriting and broken links.
#![cfg(feature = "alloc")]
#![allow(clippy::expect_used)]

use tpt_kardashev_reflect::gate::{
    AuditError, GENESIS, Outcome, UpdateGate, digest_constraints, verify_audit,
};
use tpt_kardashev_reflect::{Certificate, Constraints, Rational, ReflectError, Row};

fn int(n: i128) -> Rational {
    Rational::integer(n)
}

fn row(coefficients: &[i128], bound: i128) -> Row {
    Row {
        coefficients: coefficients.iter().map(|&c| int(c)).collect(),
        bound: int(bound),
    }
}

fn system(rows: Vec<Row>) -> Constraints {
    Constraints::new(1, rows).expect("shape")
}

fn certificate(multipliers: &[&[i128]]) -> Certificate {
    Certificate {
        multipliers: multipliers
            .iter()
            .map(|m| m.iter().map(|&v| int(v)).collect())
            .collect(),
    }
}

/// The invariant 0 <= x <= 10.
fn invariant() -> Constraints {
    system(vec![row(&[-1], 0), row(&[1], 10)])
}

/// The starting policy 2 <= x <= 8, with the certificate that proves it is inside the invariant.
fn starting() -> (Constraints, Certificate) {
    (
        system(vec![row(&[-1], -2), row(&[1], 8)]),
        certificate(&[&[1, 0], &[0, 1]]),
    )
}

fn gate() -> UpdateGate {
    let (policy, cert) = starting();
    UpdateGate::new(invariant(), policy, &cert).expect("starting policy is contained")
}

#[test]
fn a_starting_policy_outside_the_invariant_is_refused() {
    let outside = system(vec![row(&[1], 20), row(&[-1], -1)]);
    let cert = certificate(&[&[1, 0], &[0, 1]]);
    assert!(UpdateGate::new(invariant(), outside, &cert).is_err());
}

#[test]
fn an_accepted_update_becomes_the_policy_and_is_recorded() {
    let mut g = gate();
    let head_before = g.head();
    let update = system(vec![row(&[-1], -3), row(&[1], 5)]);
    let receipt = g
        .submit(update.clone(), &certificate(&[&[1, 0], &[0, 1]]))
        .expect("contained");
    assert_eq!(receipt.outcome, Outcome::Accepted);
    assert_eq!(receipt.sequence, 0);
    assert_eq!(receipt.previous, GENESIS);
    assert_eq!(g.policy(), &update);
    assert_ne!(g.head(), head_before);
}

#[test]
fn a_refused_update_leaves_the_policy_and_is_recorded_with_its_reason() {
    let mut g = gate();
    let policy_before = g.policy().clone();
    let too_wide = system(vec![row(&[1], 20), row(&[-1], -1)]);
    let refusal = g.submit(too_wide, &certificate(&[&[0, 1], &[1, 0]]));
    assert_eq!(refusal, Err(ReflectError::BoundExceeded { row: 1 }));
    assert_eq!(g.policy(), &policy_before, "the policy did not change");
    let last = g.receipts().last().expect("recorded");
    assert_eq!(
        last.outcome,
        Outcome::Rejected(ReflectError::BoundExceeded { row: 1 })
    );
}

#[test]
fn the_invariant_is_fixed_through_the_gate() {
    let mut g = gate();
    let invariant_before = g.invariant().clone();
    let _ = g.submit(
        system(vec![row(&[-1], -3), row(&[1], 5)]),
        &certificate(&[&[1, 0], &[0, 1]]),
    );
    let _ = g.submit(system(vec![row(&[1], 100)]), &certificate(&[&[0]]));
    assert_eq!(g.invariant(), &invariant_before);
}

#[test]
fn the_audit_of_a_mixed_log_verifies_to_the_head() {
    let mut g = gate();
    let _ = g.submit(
        system(vec![row(&[-1], -3), row(&[1], 5)]),
        &certificate(&[&[1, 0], &[0, 1]]),
    );
    let _ = g.submit(system(vec![row(&[1], 100)]), &certificate(&[&[0]]));
    let _ = g.submit(
        system(vec![row(&[-1], -4), row(&[1], 6)]),
        &certificate(&[&[1, 0], &[0, 1]]),
    );
    assert_eq!(g.receipts().len(), 3);
    assert_eq!(verify_audit(g.receipts()), Ok(g.head()));
}

#[test]
fn tampering_with_the_log_is_detected() {
    let mut g = gate();
    let _ = g.submit(
        system(vec![row(&[-1], -3), row(&[1], 5)]),
        &certificate(&[&[1, 0], &[0, 1]]),
    );
    let _ = g.submit(system(vec![row(&[1], 100)]), &certificate(&[&[0]]));
    let _ = g.submit(
        system(vec![row(&[-1], -4), row(&[1], 6)]),
        &certificate(&[&[1, 0], &[0, 1]]),
    );
    let honest: Vec<_> = g.receipts().to_vec();

    // A refusal rewritten as an acceptance: the hash no longer matches.
    let mut rewritten = honest.clone();
    rewritten[1].outcome = Outcome::Accepted;
    assert_eq!(
        verify_audit(&rewritten),
        Err(AuditError::BadHash { position: 1 })
    );

    // Two receipts swapped: the sequence numbers no longer match their positions.
    let mut swapped = honest.clone();
    swapped.swap(1, 2);
    assert_eq!(
        verify_audit(&swapped),
        Err(AuditError::BadSequence { position: 1 })
    );

    // A receipt removed from the middle: the next one no longer follows the one before it.
    let mut removed = honest.clone();
    removed.remove(1);
    assert!(matches!(
        verify_audit(&removed),
        Err(AuditError::BadSequence { .. } | AuditError::BrokenLink { .. })
    ));

    // A link changed, and the hash fixed up to match: the link check still fails.
    let mut relinked = honest.clone();
    relinked[2].previous = [9; 32];
    assert_eq!(
        verify_audit(&relinked),
        Err(AuditError::BrokenLink { position: 2 })
    );
}

#[test]
fn an_empty_log_verifies_to_genesis() {
    assert_eq!(verify_audit(&[]), Ok(GENESIS));
}

#[test]
fn update_digests_are_exact_and_distinguish_different_systems() {
    let a = system(vec![row(&[1], 5)]);
    let same = system(vec![Row {
        coefficients: vec![Rational::new(2, 2).expect("den")],
        bound: Rational::new(10, 2).expect("den"),
    }]);
    let different = system(vec![row(&[1], 6)]);
    assert_eq!(
        digest_constraints(&a),
        digest_constraints(&same),
        "equal rationals digest equally"
    );
    assert_ne!(digest_constraints(&a), digest_constraints(&different));
}
