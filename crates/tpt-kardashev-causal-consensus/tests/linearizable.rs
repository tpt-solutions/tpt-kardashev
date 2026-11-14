// SPDX-License-Identifier: MIT OR Apache-2.0
//! The linearizability checker on hand-built register histories: sequential, overlapping and stale
//! reads, impossible values, and invalid input.
#![cfg(feature = "alloc")]

use tpt_kardashev_causal_consensus::linearizable::{
    LinError, MAX_OPERATIONS, Op, events, is_linearizable,
};

#[test]
fn an_empty_history_is_linearizable() {
    assert_eq!(is_linearizable::<u32>(0, &[]), Ok(true));
}

#[test]
fn a_sequential_history_is_linearizable_when_reads_see_the_latest_write() {
    let history = events(&[
        (0, 1, Op::Write(5)),
        (2, 3, Op::Read(5)),
        (4, 5, Op::Write(9)),
        (6, 7, Op::Read(9)),
    ]);
    assert_eq!(is_linearizable(0, &history), Ok(true));
}

#[test]
fn a_stale_read_after_a_completed_write_is_not_linearizable() {
    // The write of 5 completed before the read started, so the read must not return the old 0.
    let history = events(&[(0, 1, Op::Write(5)), (2, 3, Op::Read(0))]);
    assert_eq!(is_linearizable(0, &history), Ok(false));
}

#[test]
fn a_read_concurrent_with_a_write_may_see_either_value() {
    let old = events(&[(0, 10, Op::Write(5)), (2, 3, Op::Read(0))]);
    let new = events(&[(0, 10, Op::Write(5)), (2, 3, Op::Read(5))]);
    assert_eq!(is_linearizable(0, &old), Ok(true));
    assert_eq!(is_linearizable(0, &new), Ok(true));
}

#[test]
fn reads_that_disagree_about_the_order_of_two_writes_are_not_linearizable() {
    // Two writes overlap. One reader sees 1 after 2 was read, and a later reader sees 2 after 1 was
    // read: no single order of the writes explains both.
    let history = events(&[
        (0, 10, Op::Write(1)),
        (0, 10, Op::Write(2)),
        (1, 2, Op::Read(1)),
        (3, 4, Op::Read(2)),
        (5, 6, Op::Read(1)),
    ]);
    assert_eq!(is_linearizable(0, &history), Ok(false));
}

#[test]
fn a_value_never_written_is_not_linearizable() {
    let history = events(&[(0, 1, Op::Write(5)), (2, 3, Op::Read(7))]);
    assert_eq!(is_linearizable(0, &history), Ok(false));
}

#[test]
fn an_event_that_responds_before_it_is_invoked_is_rejected() {
    let history = events(&[(5, 3, Op::Write(1))]);
    assert_eq!(
        is_linearizable(0, &history),
        Err(LinError::BadInterval { index: 0 })
    );
}

#[test]
fn a_history_beyond_the_limit_is_rejected() {
    let history = events(
        &(0..(MAX_OPERATIONS as u64 + 1))
            .map(|i| (i, i, Op::Write(i as u32)))
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        is_linearizable(0, &history),
        Err(LinError::TooLarge {
            len: MAX_OPERATIONS + 1
        })
    );
}

#[test]
fn a_long_overlapping_history_is_checked_within_the_limit() {
    // Sixteen operations, all overlapping: writes of 1..=8 and reads that each see some written value.
    let mut items = Vec::new();
    for v in 1..=8u32 {
        items.push((0, 100, Op::Write(v)));
    }
    for _ in 0..8 {
        items.push((0, 100, Op::Read(3)));
    }
    let history = events(&items);
    assert_eq!(is_linearizable(0, &history), Ok(true));
}
