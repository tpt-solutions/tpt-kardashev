// SPDX-License-Identifier: MIT OR Apache-2.0
//! The periodic difference between Barycentric Dynamical Time and Terrestrial Time: the
//! Fairhead and Bretagnon (1990) series in the FB2001 form (Irwin 2003, mass-corrected coefficients).
//!
//! `TDB − TT = Σ_k t^k Σ_i A_i sin(ω_i t + φ_i)`, with the amplitudes in microseconds, the frequencies
//! in radians per Julian millennium, and `t` the time since J2000 in Julian millennia (365 250 days).
//! The series is evaluated at the TT epoch, as the IERS Conventions 2010 (chapter 10) describe for the
//! "FB" model. Its amplitude is about 1.6 ms. Relative to TE405 it is good to about 15 ns over
//! 1600 to 2200, and better near 2000 (Irwin and Fukushima 1999; the FB2001 README).
//!
//! The constant offset `TDB0` and the linear TCB–TDB rate are not in this series; they belong to
//! [`crate::time`]'s TCB relation, not in this crate yet. Only the periodic part is
//! evaluated here.
//!
//! The coefficient table is `fb2001_terms`, generated from the IERS data file; see its header.

use crate::fb2001_terms::FB2001_TERMS;

/// The number of days in a Julian millennium.
const DAYS_PER_MILLENNIUM: f64 = 365_250.0;

/// The highest power of `t` in the series.
const MAX_ORDER: usize = 5;

/// The periodic part of `TDB − TT`, in seconds, at the TT epoch given as a two-part Julian date
/// (`tt_hi + tt_lo`, in days, with `tt_hi` half-integral for best precision, as the IERS code does).
///
/// The result is the TDB minus TT difference; it includes no constant or linear term. For the
/// difference between two epochs, subtract the value at each.
pub fn tdb_minus_tt_seconds(tt_hi: f64, tt_lo: f64) -> f64 {
    let t = ((tt_hi - 2_451_545.0) + tt_lo) / DAYS_PER_MILLENNIUM;
    // Sum the terms of each power of t first, then evaluate the polynomial in t.
    let mut per_power = [0.0_f64; MAX_ORDER + 1];
    for term in FB2001_TERMS.iter() {
        let order = usize::from(term.order);
        per_power[order] += term.amplitude_us * (term.frequency * t + term.phase).sin();
    }
    let mut total_us = 0.0;
    let mut power = 1.0;
    for sum in per_power.iter() {
        total_us += sum * power;
        power *= t;
    }
    total_us * 1.0e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_every_term_and_the_orders_are_in_range() {
        assert_eq!(FB2001_TERMS.len(), 1697);
        assert!(
            FB2001_TERMS
                .iter()
                .all(|t| usize::from(t.order) <= MAX_ORDER)
        );
    }

    #[test]
    fn at_j2000_the_difference_is_the_sum_of_the_zero_order_sines() {
        // At t = 0 only the order-0 terms survive, so the value is their sum of A sin(φ), in seconds.
        let direct: f64 = FB2001_TERMS
            .iter()
            .filter(|t| t.order == 0)
            .map(|t| t.amplitude_us * t.phase.sin())
            .sum::<f64>()
            * 1.0e-6;
        let value = tdb_minus_tt_seconds(2_451_545.0, 0.0);
        assert!((value - direct).abs() < 1e-15, "{value} vs {direct}");
    }
}
