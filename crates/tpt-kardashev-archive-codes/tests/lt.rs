// SPDX-License-Identifier: MIT OR Apache-2.0
//! LT fountain codes: round trips, losses, reordering and argument checks.
#![cfg(feature = "std")]
// Test helpers (not `#[test]` functions) may unwrap; the workspace allows this only inside tests.
#![allow(clippy::expect_used)]

use tpt_kardashev_archive_codes::CodeError;
use tpt_kardashev_archive_codes::lt::{DegreeDistribution, LtDecoder, LtEncoder};

const BLOCK: usize = 32;

fn source(k: usize, seed: u64) -> Vec<u8> {
    let mut x = seed;
    (0..k * BLOCK)
        .map(|_| {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (x >> 56) as u8
        })
        .collect()
}

fn dist(k: usize) -> DegreeDistribution {
    DegreeDistribution::robust_soliton(k, 0.1, 0.5).expect("valid parameters")
}

/// Feeds symbols `0, 1, 2, ...` through `keep` until the decoder completes. Returns the number of
/// symbols id'd, or `None` if `limit` ids were not enough.
fn run(
    k: usize,
    src: &[u8],
    limit: u32,
    keep: impl Fn(u32) -> bool,
    order: impl Fn(u32) -> u32,
) -> Option<(u32, Vec<u8>)> {
    let d = dist(k);
    let enc = LtEncoder::new(&d, BLOCK, src).expect("encoder");
    let mut dec = LtDecoder::new(&d, k, BLOCK).expect("decoder");
    let mut buf = [0u8; BLOCK];
    for id in 0..limit {
        if !keep(id) {
            continue;
        }
        let sym = order(id);
        enc.symbol(sym, &mut buf).expect("symbol");
        dec.add_symbol(sym, &buf).expect("add");
        if dec.is_complete() {
            return Some((id + 1, dec.into_source().expect("complete")));
        }
    }
    None
}

#[test]
fn round_trip_in_order_recovers_source() {
    let k = 64;
    let src = source(k, 1);
    let (used, out) = run(k, &src, 4 * k as u32, |_| true, |id| id).expect("completes");
    assert_eq!(out, src);
    assert!(used as usize <= 3 * k, "needed {used} symbols for k={k}");
}

#[test]
fn round_trip_survives_quarter_loss() {
    let k = 64;
    let src = source(k, 2);
    // Every fourth symbol is lost.
    let (used, out) = run(k, &src, 8 * k as u32, |id| id % 4 != 3, |id| id).expect("completes");
    assert_eq!(out, src);
    assert!(used as usize <= 6 * k, "needed {used} symbols for k={k}");
}

#[test]
fn arrival_order_does_not_matter() {
    let k = 48;
    let src = source(k, 3);
    // Reverse the ids within blocks of 16, which permutes delivery without losing any.
    let permute = |id: u32| (id / 16) * 16 + (15 - id % 16);
    let (_, out) = run(k, &src, 4 * k as u32, |_| true, permute).expect("completes");
    assert_eq!(out, src);
}

#[test]
fn duplicates_are_harmless() {
    let k = 16;
    let src = source(k, 4);
    let d = dist(k);
    let enc = LtEncoder::new(&d, BLOCK, &src).expect("encoder");
    let mut dec = LtDecoder::new(&d, k, BLOCK).expect("decoder");
    let mut buf = [0u8; BLOCK];
    for id in 0..(6 * k as u32) {
        enc.symbol(id, &mut buf).expect("symbol");
        dec.add_symbol(id, &buf).expect("add");
        dec.add_symbol(id, &buf).expect("add again");
        if dec.is_complete() {
            break;
        }
    }
    assert!(dec.is_complete());
    assert_eq!(dec.into_source().expect("complete"), src);
}

#[test]
fn symbols_are_deterministic_and_ids_differ() {
    let k = 20;
    let src = source(k, 5);
    let d = dist(k);
    let enc = LtEncoder::new(&d, BLOCK, &src).expect("encoder");
    let mut a = [0u8; BLOCK];
    let mut b = [0u8; BLOCK];
    enc.symbol(7, &mut a).expect("symbol");
    enc.symbol(7, &mut b).expect("symbol");
    assert_eq!(a, b);
    let mut distinct = std::collections::HashSet::new();
    for id in 0..50 {
        enc.symbol(id, &mut a).expect("symbol");
        distinct.insert(a.to_vec());
    }
    assert!(
        distinct.len() > 40,
        "only {} distinct symbols",
        distinct.len()
    );
}

#[test]
fn max_degree_is_the_highest_positive_weight() {
    let d = DegreeDistribution::from_weights(&[1, 0, 0, 5, 0]).expect("valid");
    assert_eq!(d.max_degree(), 4);
    let single = DegreeDistribution::from_weights(&[7]).expect("valid");
    assert_eq!(single.max_degree(), 1);
}

#[test]
fn robust_soliton_has_degree_one_and_a_bounded_maximum() {
    let k = 100;
    let d = dist(k);
    assert!(d.max_degree() >= 2 && d.max_degree() <= k);
}

#[test]
fn rejects_bad_parameters() {
    assert!(matches!(
        DegreeDistribution::from_weights(&[]),
        Err(CodeError::InvalidParameter { name: "weights" })
    ));
    assert!(matches!(
        DegreeDistribution::from_weights(&[0, 0]),
        Err(CodeError::InvalidParameter { name: "weights" })
    ));
    assert!(matches!(
        DegreeDistribution::from_weights(&[u64::MAX, 1]),
        Err(CodeError::InvalidParameter { name: "weights" })
    ));
    assert!(DegreeDistribution::robust_soliton(0, 0.1, 0.5).is_err());
    assert!(DegreeDistribution::robust_soliton(10, 0.0, 0.5).is_err());
    assert!(DegreeDistribution::robust_soliton(10, 0.1, 1.0).is_err());

    let d = dist(8);
    assert!(LtEncoder::new(&d, 0, &[0u8; 8]).is_err());
    assert!(LtEncoder::new(&d, 3, &[0u8; 8]).is_err());
    assert!(LtEncoder::new(&d, 4, &[]).is_err());
    assert!(LtDecoder::new(&d, 0, 4).is_err());
    let mut dec = LtDecoder::new(&d, 8, 4).expect("decoder");
    assert!(matches!(
        dec.add_symbol(0, &[0u8; 3]),
        Err(CodeError::LengthMismatch {
            expected: 4,
            actual: 3
        })
    ));
    let mut short = [0u8; 3];
    let enc = LtEncoder::new(&d, 4, &[1u8; 32]).expect("encoder");
    assert!(enc.symbol(0, &mut short).is_err());
}

#[test]
fn incomplete_decoder_yields_no_source() {
    let k = 32;
    let src = source(k, 6);
    let d = dist(k);
    let enc = LtEncoder::new(&d, BLOCK, &src).expect("encoder");
    let mut dec = LtDecoder::new(&d, k, BLOCK).expect("decoder");
    let mut buf = [0u8; BLOCK];
    enc.symbol(0, &mut buf).expect("symbol");
    dec.add_symbol(0, &buf).expect("add");
    assert!(!dec.is_complete());
    // One symbol can resolve at most one source block.
    assert!(dec.known_count() <= 1);
    assert!(dec.into_source().is_none());
}
