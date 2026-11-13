// SPDX-License-Identifier: MIT OR Apache-2.0
//! Maximum-likelihood checks and BER regression for the K=7, rate-1/2 code.
//!
//! * The weight spectrum used for the union bound reproduces the known free distance.
//! * The soft Viterbi decoder reaches the minimum metric found by exhaustive search.
//! * Simulated BER at 3 dB stays within the union bound, within a statistical margin.
#![allow(clippy::expect_used)]

#[path = "common/ber.rs"]
mod ber;

use ber::{
    Rng, simulate, union_bound_hard, union_bound_soft, union_bound_soft_upto, weight_spectrum,
};
use tpt_kardashev_archive_codes::conv;

/// Allowed excess over the union bound, in standard errors of the observed error count.
const SIGMAS: f64 = 4.0;
/// Interior soft-decision errors each BER point must accumulate before it is judged.
const TARGET_ERRORS: u64 = 1000;

/// Byte-metric cost of hypothesis `c` for soft value `s` (the crate's documented convention).
fn cost(s: u8, c: u8) -> u32 {
    if c == 0 {
        u32::from(s)
    } else {
        255 - u32::from(s)
    }
}

#[test]
fn weight_spectrum_matches_the_known_free_distance() {
    let b = weight_spectrum();
    assert_eq!(b[10], 36.0, "B_10 (free distance 10)");
    assert_eq!(b[12], 211.0, "B_12");
    assert_eq!(b[14], 1404.0, "B_14");
    assert_eq!(b[16], 11633.0, "B_16");
    assert!(
        b[..10].iter().all(|&x| x == 0.0),
        "no events below the free distance"
    );
    assert!(b.iter().all(|x| x.is_finite()), "spectrum overflowed");
}

#[test]
fn soft_decoder_reaches_the_exhaustive_minimum_metric() {
    // Random soft words of 10 information bits (no tail; free end state). The decoder's returned
    // metric must equal the smallest metric over all 2^10 inputs.
    let n = 10usize;
    let mut rng = Rng::new(0x51DE_0001);
    for trial in 0..300 {
        let soft: Vec<u8> = (0..2 * n).map(|_| (rng.next_u64() & 0xFF) as u8).collect();
        let mut out = vec![0u8; n];
        let mut scratch = vec![0u64; n];
        let metric = conv::viterbi_decode_soft(&soft, &mut out, &mut scratch).expect("decode");

        let mut best = u32::MAX;
        for word in 0u32..(1 << n) {
            let bits: Vec<u8> = (0..n).map(|i| ((word >> i) & 1) as u8).collect();
            let mut coded = vec![0u8; 2 * n];
            conv::encode(&bits, &mut coded).expect("encode");
            let m: u32 = coded.iter().zip(&soft).map(|(&c, &s)| cost(s, c)).sum();
            best = best.min(m);
        }
        assert_eq!(
            metric, best,
            "trial {trial}: decoder metric is not the minimum"
        );

        // The decoder's own output must achieve that metric.
        let mut coded = vec![0u8; 2 * n];
        conv::encode(&out, &mut coded).expect("encode");
        let achieved: u32 = coded.iter().zip(&soft).map(|(&c, &s)| cost(s, c)).sum();
        assert_eq!(
            achieved, metric,
            "trial {trial}: output does not achieve the metric"
        );
    }
}

#[test]
fn simulated_ber_is_bounded_by_the_union_bound_at_3_db() {
    let spectrum = weight_spectrum();
    let db = 3.0f64;
    let gamma = 10f64.powf(db / 10.0);
    let p = simulate(db, 4096, 4_000, TARGET_ERRORS, 0x0BE8_0003);
    let soft_bound = union_bound_soft(gamma, &spectrum);
    // The hard-decision union bound exceeds 1 at this SNR, so it is reported but not asserted.
    let hard_bound = union_bound_hard(gamma, &spectrum);
    eprintln!(
        "Eb/N0 {db} dB: interior bits {} soft {:.3e} (bound {:.3e}) hard {:.3e} (bound {:.3e})",
        p.bits,
        p.soft_ber(),
        soft_bound,
        p.hard_ber(),
        hard_bound
    );
    assert!(
        p.soft_errors >= TARGET_ERRORS,
        "only {} soft errors; raise the frame budget",
        p.soft_errors
    );
    let soft_allowance = 1.0 + SIGMAS / (p.soft_errors as f64).sqrt();
    assert!(
        p.soft_ber() <= soft_bound * soft_allowance,
        "soft BER {:.3e} above the union bound {:.3e} by more than the allowance",
        p.soft_ber(),
        soft_bound
    );
    assert!(
        p.soft_ber() < p.hard_ber(),
        "soft decisions did not beat hard decisions"
    );
    // The bound is an upper bound only once its sum has converged. Check that the terms beyond
    // d = 100 change it by less than 0.1%.
    let truncated = union_bound_soft_upto(gamma, &spectrum, 100);
    assert!(
        (soft_bound - truncated).abs() <= 1e-3 * soft_bound,
        "union bound not converged at 3 dB: full {soft_bound:.6e}, to d=100 {truncated:.6e}"
    );
}
