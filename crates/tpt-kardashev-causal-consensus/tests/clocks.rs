// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hybrid logical clocks and vector clocks: strict monotonicity on one node even when physical time
//! stands still or goes backwards, causality across nodes, the drift bound, the bound on the
//! wall-clock part, and the vector-clock relations.
#![allow(clippy::expect_used)]

use tpt_kardashev_causal_consensus::{CausalRelation, ClockError, HybridClock, VectorClock};

/// Deterministic pseudo-random numbers (Knuth's LCG constants): the tests are reproducible.
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

const DRIFT_MS: u64 = 60_000;

#[test]
fn a_node_never_issues_a_timestamp_twice_or_backwards() {
    // Physical time jumps forward, stands still, and goes backwards. Every issued timestamp must
    // still be strictly after the one before.
    let mut rng = Lcg(7);
    let mut clock = HybridClock::new(1, DRIFT_MS);
    let mut previous = None;
    let mut physical = 1_000u64;
    for _ in 0..10_000 {
        match rng.below(3) {
            0 => physical += rng.below(50),
            1 => physical = physical.saturating_sub(rng.below(200)),
            _ => {}
        }
        let stamp = clock.now(physical).expect("no overflow");
        if let Some(prev) = previous {
            assert!(stamp > prev, "{stamp:?} is not after {prev:?}");
        }
        previous = Some(stamp);
    }
}

#[test]
fn causality_is_preserved_along_a_chain_of_messages() {
    // A stamps a send; B receives it, then sends to C; C receives it. The timestamps must increase
    // along the chain, even though the three physical clocks disagree by up to 40 ms.
    let mut a = HybridClock::new(10, DRIFT_MS);
    let mut b = HybridClock::new(20, DRIFT_MS);
    let mut c = HybridClock::new(30, DRIFT_MS);
    let sent_by_a = a.now(1_000).expect("send");
    let received_by_b = b.receive(sent_by_a, 1_010).expect("receive");
    let sent_by_b = b.now(1_010).expect("send");
    let received_by_c = c.receive(sent_by_b, 1_000).expect("receive");
    assert!(sent_by_a < received_by_b);
    assert!(received_by_b < sent_by_b);
    assert!(sent_by_b < received_by_c);
}

#[test]
fn a_receive_is_after_the_remote_stamp_and_the_local_history() {
    let mut clock = HybridClock::new(1, DRIFT_MS);
    let local = clock.now(5_000).expect("local");
    // The remote stamp is far ahead of the local history but within the drift bound.
    let remote = HybridClock::new(2, DRIFT_MS).now(5_020).expect("remote");
    let merged = clock.receive(remote, 5_000).expect("receive");
    assert!(merged > remote);
    assert!(merged > local);
}

#[test]
fn a_timestamp_beyond_the_drift_bound_is_rejected_and_changes_nothing() {
    let mut clock = HybridClock::new(1, DRIFT_MS);
    clock.now(1_000).expect("local");
    let before = clock.latest();
    let far = tpt_kardashev_causal_consensus::Timestamp {
        wall_ms: 1_000 + DRIFT_MS + 1,
        counter: 0,
        node: 2,
    };
    assert_eq!(
        clock.receive(far, 1_000),
        Err(ClockError::DriftExceeded {
            ahead_ms: DRIFT_MS + 1
        })
    );
    assert_eq!(
        clock.latest(),
        before,
        "the rejected receive changed the clock"
    );
}

#[test]
fn the_wall_clock_part_never_exceeds_the_largest_time_seen() {
    // Three nodes with physical clocks that drift apart and exchange stamps at random. Every
    // timestamp's wall part must stay at or below the largest physical reading seen so far, since
    // a receive adds only a remote wall part already within the drift bound of some reading.
    let mut rng = Lcg(99);
    let mut clocks = [
        HybridClock::new(1, DRIFT_MS),
        HybridClock::new(2, DRIFT_MS),
        HybridClock::new(3, DRIFT_MS),
    ];
    let mut physical = [1_000u64, 1_000, 1_000];
    let mut largest_reading = 1_000u64;
    let mut in_flight: Vec<(usize, tpt_kardashev_causal_consensus::Timestamp)> = Vec::new();
    for _ in 0..5_000 {
        let node = rng.below(3) as usize;
        physical[node] += rng.below(30);
        largest_reading = largest_reading.max(physical[node]);
        if rng.below(2) == 0 || in_flight.is_empty() {
            let stamp = clocks[node].now(physical[node]).expect("send");
            in_flight.push((node, stamp));
        } else {
            let index = rng.below(in_flight.len() as u64) as usize;
            let (_, stamp) = in_flight.swap_remove(index);
            match clocks[node].receive(stamp, physical[node]) {
                Ok(merged) => assert!(merged.wall_ms <= largest_reading + DRIFT_MS),
                Err(ClockError::DriftExceeded { .. }) => {}
                Err(other) => panic!("unexpected error {other:?}"),
            }
        }
        for clock in &clocks {
            assert!(clock.latest().wall_ms <= largest_reading + DRIFT_MS);
        }
    }
}

#[test]
fn vector_clocks_report_each_causal_relation() {
    let mut a = VectorClock::<3>::new();
    let mut b = VectorClock::<3>::new();
    assert_eq!(a.relation(&b), CausalRelation::Equal);
    a.tick(0).expect("tick");
    assert_eq!(a.relation(&b), CausalRelation::After);
    assert_eq!(b.relation(&a), CausalRelation::Before);
    b.tick(1).expect("tick");
    assert_eq!(a.relation(&b), CausalRelation::Concurrent);
    b.merge(&a);
    assert_eq!(b.relation(&a), CausalRelation::After);
    assert_eq!(b.get(0), Some(1));
    assert_eq!(b.get(1), Some(1));
    assert_eq!(b.get(2), Some(0));
    assert_eq!(b.get(3), None);
}

#[test]
fn merge_is_commutative_and_idempotent() {
    let mut x = VectorClock::<4>::new();
    let mut y = VectorClock::<4>::new();
    for i in 0..4 {
        for _ in 0..=i {
            x.tick(i).expect("tick");
        }
        for _ in 0..(4 - i) {
            y.tick(i).expect("tick");
        }
    }
    let mut xy = x;
    xy.merge(&y);
    let mut yx = y;
    yx.merge(&x);
    assert_eq!(xy, yx);
    let mut again = xy;
    again.merge(&xy);
    assert_eq!(again, xy);
}

#[test]
fn an_index_outside_the_clock_is_rejected() {
    let mut clock = VectorClock::<2>::new();
    assert_eq!(clock.tick(2), Err(ClockError::NodeOutOfRange { index: 2 }));
    assert_eq!(clock.dimension(), 2);
}
