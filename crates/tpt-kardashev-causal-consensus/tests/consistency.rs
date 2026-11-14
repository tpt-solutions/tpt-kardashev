// SPDX-License-Identifier: MIT OR Apache-2.0
//! Consistency levels and their latency lower bounds.
#![allow(clippy::expect_used)]

use tpt_kardashev_causal_consensus::{ConsistencyLevel, Delays};

const LEVELS: [ConsistencyLevel; 4] = [
    ConsistencyLevel::Commutative,
    ConsistencyLevel::Causal,
    ConsistencyLevel::DomainLinearizable,
    ConsistencyLevel::GlobalAgreed,
];

fn delays(intra: u64, cross: u64) -> Delays {
    Delays {
        intra_domain_ms: intra,
        cross_domain_ms: cross,
    }
}

#[test]
fn only_the_global_level_needs_a_cross_domain_round_trip() {
    for level in LEVELS {
        assert_eq!(
            level.needs_cross_domain_round_trip(),
            level == ConsistencyLevel::GlobalAgreed,
            "{level:?}"
        );
    }
}

#[test]
fn the_levels_without_agreement_have_no_latency_bound() {
    let d = delays(50, 1_300);
    assert_eq!(ConsistencyLevel::Commutative.latency_lower_bound_ms(d), 0);
    assert_eq!(ConsistencyLevel::Causal.latency_lower_bound_ms(d), 0);
}

#[test]
fn domain_and_global_bounds_are_two_one_way_delays() {
    // A lunar base to Earth is about 1.3 s one way; a base's own nodes are a few tens of ms apart.
    let d = delays(40, 1_300);
    assert_eq!(
        ConsistencyLevel::DomainLinearizable.latency_lower_bound_ms(d),
        80
    );
    assert_eq!(
        ConsistencyLevel::GlobalAgreed.latency_lower_bound_ms(d),
        2_600
    );
}

#[test]
fn the_global_bound_is_never_below_the_domain_bound_when_cross_links_are_slower() {
    for (intra, cross) in [(0u64, 0u64), (10, 10), (40, 1_300), (1_300, 40)] {
        let d = delays(intra, cross);
        let domain = ConsistencyLevel::DomainLinearizable.latency_lower_bound_ms(d);
        let global = ConsistencyLevel::GlobalAgreed.latency_lower_bound_ms(d);
        if cross >= intra {
            assert!(global >= domain, "intra {intra}, cross {cross}");
        }
    }
}

#[test]
fn bounds_grow_with_the_delays_and_saturate_instead_of_overflowing() {
    let near = delays(10, 100);
    let far = delays(10, 200);
    assert!(
        ConsistencyLevel::GlobalAgreed.latency_lower_bound_ms(far)
            > ConsistencyLevel::GlobalAgreed.latency_lower_bound_ms(near)
    );
    let huge = delays(u64::MAX, u64::MAX);
    assert_eq!(
        ConsistencyLevel::GlobalAgreed.latency_lower_bound_ms(huge),
        u64::MAX
    );
}

#[test]
fn agreement_levels_are_classified_as_the_spec_states() {
    assert!(!ConsistencyLevel::Causal.needs_domain_round_trip());
    assert!(ConsistencyLevel::DomainLinearizable.needs_domain_round_trip());
    assert!(ConsistencyLevel::GlobalAgreed.needs_domain_round_trip());
}
