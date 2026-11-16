// SPDX-License-Identifier: MIT OR Apache-2.0
//! The FB2001 periodic TDB−TT series against the IERS reference output (`fb2001.out`, Irwin 2003,
//! stored as `tests/data/fb2001_reference.txt`). The reference gives the time ephemeris relative to its
//! value at the epoch t0 = JD 2443144.5 + 0.0003725 days, in days, for a grid of epochs.
#![cfg(feature = "std")]
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::tdb::tdb_minus_tt_seconds;

const REFERENCE: &str = include_str!("data/fb2001_reference.txt");

/// The value of the series at the reference epoch, in days.
fn at_t0() -> f64 {
    tdb_minus_tt_seconds(2_443_144.5, 0.0003725) / 86_400.0
}

#[test]
fn the_series_matches_the_iers_reference_output_to_the_nanosecond() {
    let base = at_t0();
    let mut checked = 0;
    let mut worst_ns: f64 = 0.0;
    for line in REFERENCE.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // Data lines: epoch (days), value relative to t0 (days), derivative.
        if fields.len() != 3 || fields[0].starts_with(|c: char| c.is_alphabetic()) {
            continue;
        }
        let Ok(epoch) = fields[0].parse::<f64>() else {
            continue;
        };
        let Ok(expected_days) = fields[1].parse::<f64>() else {
            continue;
        };
        let ours_days = tdb_minus_tt_seconds(epoch, 0.0) / 86_400.0 - base;
        let error_ns = (ours_days - expected_days).abs() * 86_400.0 * 1e9;
        worst_ns = worst_ns.max(error_ns);
        checked += 1;
    }
    assert!(checked > 200, "only {checked} reference lines read");
    // The reference's own accuracy against TE405 is about 15 ns; the implementation should reproduce it
    // to a small fraction of that. The bound is 1 ns, which is well below the physical accuracy.
    assert!(
        worst_ns < 1.0,
        "worst difference from the reference: {worst_ns} ns over {checked} epochs"
    );
}
