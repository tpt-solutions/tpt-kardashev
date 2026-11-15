// SPDX-License-Identifier: MIT OR Apache-2.0
//! The attestation path: the invariant is replaced only under the authority's signature of the
//! replacement at the next log position, and only if the current policy is still contained in it.
//! Refusals are recorded; the audit still verifies; ordinary submissions still cannot change the
//! invariant.
#![cfg(all(feature = "crypto", feature = "alloc"))]
#![allow(clippy::expect_used)]

use ed25519_compact::{KeyPair, Seed, Signature};
use tpt_kardashev_reflect::attest::{AttestedGate, replacement_message};
use tpt_kardashev_reflect::gate::{Outcome, digest_constraints, verify_audit};
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

fn authority() -> KeyPair {
    KeyPair::from_seed(Seed::new([0x42; 32]))
}

/// Invariant 0 <= x <= 10; policy 2 <= x <= 8, contained (its rows are the invariant's rows scaled
/// by the multipliers below).
fn start() -> (Constraints, Constraints, Certificate) {
    (
        system(vec![row(&[-1], 0), row(&[1], 10)]),
        system(vec![row(&[-1], -2), row(&[1], 8)]),
        certificate(&[&[1, 0], &[0, 1]]),
    )
}

fn gate() -> AttestedGate {
    let (invariant, policy, cert) = start();
    AttestedGate::new(invariant, policy, &cert, authority().pk).expect("start")
}

/// The authority's signature of the replacement `next` at log position `position`, replacing the
/// invariant with digest `replaced`.
fn attest(replaced: &[u8; 32], next: &Constraints, position: u64) -> Signature {
    let message = replacement_message(replaced, &digest_constraints(next), position);
    authority().sk.sign(message, None)
}

#[test]
fn an_attested_replacement_takes_effect_and_is_recorded() {
    let mut g = gate();
    let (old, _, _) = start();
    // The replacement is 0 <= x <= 9. The policy 2 <= x <= 8 is still inside it.
    let next = system(vec![row(&[-1], 0), row(&[1], 9)]);
    let sig = attest(&digest_constraints(&old), &next, 0);
    let keep_policy = certificate(&[&[1, 0], &[0, 1]]);
    let receipt = g
        .replace_invariant(next.clone(), &keep_policy, &sig)
        .expect("attested");
    assert_eq!(receipt.outcome, Outcome::InvariantReplaced);
    assert_eq!(g.gate().invariant(), &next);
    assert_eq!(verify_audit(g.receipts()), Ok(g.head()));
}

#[test]
fn a_signature_for_another_position_is_refused_and_recorded() {
    let mut g = gate();
    let (old, _, _) = start();
    let next = system(vec![row(&[-1], 0), row(&[1], 9)]);
    // Signed for position 5, presented at position 0: a replay of an attestation is refused.
    let sig = attest(&digest_constraints(&old), &next, 5);
    let before = g.gate().invariant().clone();
    let refusal = g.replace_invariant(next, &certificate(&[&[1, 0], &[0, 1]]), &sig);
    assert_eq!(refusal, Err(ReflectError::BadAttestation));
    assert_eq!(
        g.gate().invariant(),
        &before,
        "the invariant did not change"
    );
    assert_eq!(
        g.receipts().last().map(|r| r.outcome),
        Some(Outcome::Rejected(ReflectError::BadAttestation))
    );
}

#[test]
fn a_signature_from_another_key_is_refused() {
    let mut g = gate();
    let (old, _, _) = start();
    let next = system(vec![row(&[-1], 0), row(&[1], 9)]);
    let message = replacement_message(&digest_constraints(&old), &digest_constraints(&next), 0);
    let impostor = KeyPair::from_seed(Seed::new([0x99; 32]));
    let sig = impostor.sk.sign(message, None);
    assert_eq!(
        g.replace_invariant(next, &certificate(&[&[1, 0], &[0, 1]]), &sig),
        Err(ReflectError::BadAttestation)
    );
}

#[test]
fn a_replacement_that_excludes_the_policy_is_refused_even_when_attested() {
    let mut g = gate();
    let (old, _, _) = start();
    // 0 <= x <= 5 does not contain the policy 2 <= x <= 8, and no certificate can show it does.
    let next = system(vec![row(&[-1], 0), row(&[1], 5)]);
    let sig = attest(&digest_constraints(&old), &next, 0);
    let refusal = g.replace_invariant(next, &certificate(&[&[1, 0], &[0, 1]]), &sig);
    assert_eq!(refusal, Err(ReflectError::BoundExceeded { row: 1 }));
    assert_eq!(g.gate().invariant(), &start().0);
}

#[test]
fn a_second_replacement_needs_a_fresh_attestation_at_its_position() {
    let mut g = gate();
    let (old, _, _) = start();
    let first = system(vec![row(&[-1], 0), row(&[1], 9)]);
    let sig_first = attest(&digest_constraints(&old), &first, 0);
    g.replace_invariant(first.clone(), &certificate(&[&[1, 0], &[0, 1]]), &sig_first)
        .expect("first");
    // The first attestation does not carry over to a second replacement.
    let second = system(vec![row(&[-1], 0), row(&[1], 8)]);
    let refusal = g.replace_invariant(
        second.clone(),
        &certificate(&[&[1, 0], &[0, 1]]),
        &sig_first,
    );
    assert_eq!(refusal, Err(ReflectError::BadAttestation));
    let position = g.receipts().len() as u64;
    let sig_second = attest(&digest_constraints(&first), &second, position);
    g.replace_invariant(second, &certificate(&[&[1, 0], &[0, 1]]), &sig_second)
        .expect("second");
    assert_eq!(verify_audit(g.receipts()), Ok(g.head()));
}

#[test]
fn ordinary_submissions_still_cannot_change_the_invariant() {
    let mut g = gate();
    let (invariant, _, _) = start();
    let widen = system(vec![row(&[1], 100)]);
    let _ = g.submit(widen, &certificate(&[&[0]]));
    assert_eq!(g.gate().invariant(), &invariant);
}
