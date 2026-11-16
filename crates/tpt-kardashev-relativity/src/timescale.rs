// SPDX-License-Identifier: MIT OR Apache-2.0
//! Rate relations between the IAU barycentric and geocentric time scales, as functions.
//!
//! Each function converts a duration measured in one scale to the same duration in another, using
//! the defining rate constant. Only the secular rates are modelled. The periodic terms of TDB
//! (about +/- 1.7 ms) are not, and neither is any epoch offset. Calendar handling belongs to
//! `tpt-kardashev-astrodynamics`.
//!
//! Constants, checked 2026-11-16 against the IERS Conventions 2010 appendix (IAU 2006 Resolution B3,
//! item 14) and the IAU 2000 explanatory supplement (arXiv:astro-ph/0303376); see `docs/provenance/`:
//! - `L_G = 6.969 290 134e-10` is a defining constant of IAU Resolution B1.9 (2000): TT and TCG rates.
//! - `L_B = 1.550 519 768e-8` is a defining constant of IAU Resolution B3 (2006): TDB and TCB rates,
//!   with `TDB0 = -6.55e-5 s` and `T0 = JD 2443144.5003725` (the epoch offset, which this crate does not
//!   model). The supplement's 2003 estimate, 1.55051976772e-8, is the value the resolution rounds; the
//!   defined value is the one used here.
//! - `L_C = 1.480 826 867 41e-8` is the estimate of Irwin and Fukushima (1999), used as an input. It
//!   is not defined by a resolution.

use crate::units::Seconds;

/// `L_G`: `dTT / dTCG = 1 - L_G`.
pub const L_G: f64 = 6.969_290_134e-10;
/// `L_C`: `dTCB / dTCG = 1 + L_C`.
pub const L_C: f64 = 1.480_826_867_41e-8;
/// `L_B`: `dTDB / dTCB = 1 - L_B`.
pub const L_B: f64 = 1.550_519_768e-8;

/// Converts a TT interval to the TCG interval it spans: `dTCG = dTT / (1 - L_G)`.
pub fn tcg_from_tt(tt: Seconds) -> Seconds {
    Seconds(tt.0 / (1.0 - L_G))
}

/// Converts a TCG interval to the TT interval it spans: `dTT = dTCG (1 - L_G)`.
pub fn tt_from_tcg(tcg: Seconds) -> Seconds {
    Seconds(tcg.0 * (1.0 - L_G))
}

/// Converts a TCG interval to the TCB interval it spans: `dTCB = dTCG (1 + L_C)`.
pub fn tcb_from_tcg(tcg: Seconds) -> Seconds {
    Seconds(tcg.0 * (1.0 + L_C))
}

/// Converts a TCB interval to the TDB interval it spans: `dTDB = dTCB (1 - L_B)`.
pub fn tdb_from_tcb(tcb: Seconds) -> Seconds {
    Seconds(tcb.0 * (1.0 - L_B))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tt_tcg_round_trip() {
        let back = tt_from_tcg(tcg_from_tt(Seconds(1.0e9)));
        assert!((back.0 - 1.0e9).abs() < 1e-6);
    }

    #[test]
    fn tcg_runs_faster_than_tt_by_about_twenty_two_ms_per_year() {
        // One Julian year of TT corresponds to about 22 ms more TCG.
        let year = 365.25 * 86_400.0;
        let extra = tcg_from_tt(Seconds(year)).0 - year;
        assert!((extra - 0.022).abs() < 0.001, "extra {extra} s");
    }

    #[test]
    fn tcb_and_tdb_rates() {
        assert!(tcb_from_tcg(Seconds(1.0)).0 > 1.0);
        assert!(tdb_from_tcb(Seconds(1.0)).0 < 1.0);
    }
}
