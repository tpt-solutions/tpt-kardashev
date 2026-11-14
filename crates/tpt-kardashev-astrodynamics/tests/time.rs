// SPDX-License-Identifier: MIT OR Apache-2.0
//! Epochs, UTC conversion through a leap-second table, and Julian dates.
//!
//! The table in this file is a test fixture, not data bundled with the crate: the offsets are the
//! IERS values (TAI − UTC) as recalled, and should be checked against IERS Bulletin C before any
//! reliance on them. The tests that depend on individual entries use only 2000-01-01 (32 s) and
//! 2017-01-01 (37 s).
#![allow(clippy::expect_used)]

use tpt_kardashev_astrodynamics::{Epoch, Error, LeapEntry, LeapSeconds, UtcTime};

/// Seconds tolerance for instants: the policy's closed-form bound, expressed in time. The stored
/// value resolves about 0.5 µs, so 1 µs is the bound.
const TIME_TOL_S: f64 = 1e-6;

const TABLE: [LeapEntry; 28] = [
    LeapEntry {
        year: 1972,
        month: 1,
        day: 1,
        tai_minus_utc: 10,
    },
    LeapEntry {
        year: 1972,
        month: 7,
        day: 1,
        tai_minus_utc: 11,
    },
    LeapEntry {
        year: 1973,
        month: 1,
        day: 1,
        tai_minus_utc: 12,
    },
    LeapEntry {
        year: 1974,
        month: 1,
        day: 1,
        tai_minus_utc: 13,
    },
    LeapEntry {
        year: 1975,
        month: 1,
        day: 1,
        tai_minus_utc: 14,
    },
    LeapEntry {
        year: 1976,
        month: 1,
        day: 1,
        tai_minus_utc: 15,
    },
    LeapEntry {
        year: 1977,
        month: 1,
        day: 1,
        tai_minus_utc: 16,
    },
    LeapEntry {
        year: 1978,
        month: 1,
        day: 1,
        tai_minus_utc: 17,
    },
    LeapEntry {
        year: 1979,
        month: 1,
        day: 1,
        tai_minus_utc: 18,
    },
    LeapEntry {
        year: 1980,
        month: 1,
        day: 1,
        tai_minus_utc: 19,
    },
    LeapEntry {
        year: 1981,
        month: 7,
        day: 1,
        tai_minus_utc: 20,
    },
    LeapEntry {
        year: 1982,
        month: 7,
        day: 1,
        tai_minus_utc: 21,
    },
    LeapEntry {
        year: 1983,
        month: 7,
        day: 1,
        tai_minus_utc: 22,
    },
    LeapEntry {
        year: 1985,
        month: 7,
        day: 1,
        tai_minus_utc: 23,
    },
    LeapEntry {
        year: 1988,
        month: 1,
        day: 1,
        tai_minus_utc: 24,
    },
    LeapEntry {
        year: 1990,
        month: 1,
        day: 1,
        tai_minus_utc: 25,
    },
    LeapEntry {
        year: 1991,
        month: 1,
        day: 1,
        tai_minus_utc: 26,
    },
    LeapEntry {
        year: 1992,
        month: 7,
        day: 1,
        tai_minus_utc: 27,
    },
    LeapEntry {
        year: 1993,
        month: 7,
        day: 1,
        tai_minus_utc: 28,
    },
    LeapEntry {
        year: 1994,
        month: 7,
        day: 1,
        tai_minus_utc: 29,
    },
    LeapEntry {
        year: 1996,
        month: 1,
        day: 1,
        tai_minus_utc: 30,
    },
    LeapEntry {
        year: 1997,
        month: 7,
        day: 1,
        tai_minus_utc: 31,
    },
    LeapEntry {
        year: 1999,
        month: 1,
        day: 1,
        tai_minus_utc: 32,
    },
    LeapEntry {
        year: 2006,
        month: 1,
        day: 1,
        tai_minus_utc: 33,
    },
    LeapEntry {
        year: 2009,
        month: 1,
        day: 1,
        tai_minus_utc: 34,
    },
    LeapEntry {
        year: 2012,
        month: 7,
        day: 1,
        tai_minus_utc: 35,
    },
    LeapEntry {
        year: 2015,
        month: 7,
        day: 1,
        tai_minus_utc: 36,
    },
    LeapEntry {
        year: 2017,
        month: 1,
        day: 1,
        tai_minus_utc: 37,
    },
];

fn leap() -> LeapSeconds<'static> {
    LeapSeconds::new(&TABLE).expect("the fixture table is valid")
}

fn utc(year: i64, month: u8, day: u8, hour: u8, minute: u8, second: f64) -> UtcTime {
    UtcTime::new(year, month, day, hour, minute, second).expect("valid UTC time")
}

#[test]
fn j2000_is_noon_on_2000_01_01_tt() {
    assert_eq!(Epoch::J2000.tt_seconds(), 0.0);
    assert_eq!(Epoch::J2000.tt_julian_date(), 2_451_545.0);
}

#[test]
fn tai_is_tt_minus_32_184_seconds() {
    let t = Epoch::from_tt_seconds(1_000.0).expect("finite");
    assert!((t.tt_seconds() - t.tai_seconds() - 32.184).abs() <= TIME_TOL_S);
}

#[test]
fn julian_date_round_trips() {
    let jd = 2_460_000.5;
    let t = Epoch::from_tt_julian_date(jd).expect("finite");
    assert!((t.tt_julian_date() - jd).abs() <= 1e-12 * jd);
}

#[test]
fn utc_at_j2000_uses_the_2000_offset_of_32_seconds() {
    // 2000-01-01 12:00:00 UTC is TAI − UTC = 32 s later than the TAI reading of J2000 (which
    // itself is 32.184 s before TT), so TT − J2000 = 32 + 32.184 = 64.184 s.
    let t = Epoch::from_utc(utc(2000, 1, 1, 12, 0, 0.0), &leap()).expect("in table");
    assert!(
        (t.tt_seconds() - 64.184).abs() <= TIME_TOL_S,
        "got {}",
        t.tt_seconds()
    );
}

#[test]
fn a_leap_second_adds_two_seconds_to_the_tai_difference() {
    // UTC advances by 1 s from 23:59:59 to 00:00:00 across 2017-01-01, but the offset rises from
    // 36 to 37 s, so TAI advances by 2 s.
    let before = Epoch::from_utc(utc(2016, 12, 31, 23, 59, 59.0), &leap()).expect("in table");
    let after = Epoch::from_utc(utc(2017, 1, 1, 0, 0, 0.0), &leap()).expect("in table");
    assert!(((after.tai_seconds() - before.tai_seconds()) - 2.0).abs() <= TIME_TOL_S);
}

#[test]
fn utc_round_trips_through_tt() {
    let samples = [
        utc(1999, 12, 31, 23, 59, 59.5),
        utc(2000, 6, 15, 6, 30, 15.25),
        utc(2015, 6, 30, 23, 59, 59.999),
        utc(2015, 7, 1, 0, 0, 0.0),
        utc(2024, 2, 29, 18, 0, 0.0),
    ];
    for original in samples {
        let t = Epoch::from_utc(original, &leap()).expect("in table");
        let back = t.to_utc(&leap()).expect("in table");
        assert_eq!(
            (
                back.year(),
                back.month(),
                back.day(),
                back.hour(),
                back.minute()
            ),
            (
                original.year(),
                original.month(),
                original.day(),
                original.hour(),
                original.minute()
            ),
        );
        assert!(
            (back.second() - original.second()).abs() <= TIME_TOL_S,
            "second {} came back as {}",
            original.second(),
            back.second()
        );
    }
}

#[test]
fn instants_before_the_table_are_rejected() {
    assert_eq!(
        Epoch::from_utc(utc(1960, 1, 1, 0, 0, 0.0), &leap()),
        Err(Error::NoLeapSecondEntry)
    );
}

#[test]
fn invalid_calendar_values_are_rejected() {
    assert_eq!(
        UtcTime::new(2001, 13, 1, 0, 0, 0.0),
        Err(Error::InvalidCalendar)
    );
    assert_eq!(
        UtcTime::new(2001, 2, 29, 0, 0, 0.0),
        Err(Error::InvalidCalendar)
    );
    assert_eq!(
        UtcTime::new(2000, 2, 29, 0, 0, 0.0).map(|t| t.day()),
        Ok(29)
    );
    assert_eq!(
        UtcTime::new(2001, 1, 1, 24, 0, 0.0),
        Err(Error::InvalidCalendar)
    );
    assert_eq!(
        UtcTime::new(2001, 1, 1, 0, 60, 0.0),
        Err(Error::InvalidCalendar)
    );
    assert_eq!(
        UtcTime::new(2001, 1, 1, 0, 0, 60.0),
        Err(Error::InvalidCalendar)
    );
    assert_eq!(
        UtcTime::new(2001, 1, 1, 0, 0, f64::NAN),
        Err(Error::InvalidCalendar)
    );
    assert_eq!(
        Epoch::from_tt_seconds(f64::INFINITY),
        Err(Error::NonFinite { what: "epoch" })
    );
}

#[test]
fn leap_second_tables_are_validated() {
    assert_eq!(
        LeapSeconds::new(&[]).map(|_| ()),
        Err(Error::InvalidLeapSeconds)
    );
    let unsorted = [
        LeapEntry {
            year: 2000,
            month: 1,
            day: 1,
            tai_minus_utc: 32,
        },
        LeapEntry {
            year: 1999,
            month: 1,
            day: 1,
            tai_minus_utc: 31,
        },
    ];
    assert_eq!(
        LeapSeconds::new(&unsorted).map(|_| ()),
        Err(Error::InvalidLeapSeconds)
    );
    let shrinking = [
        LeapEntry {
            year: 1999,
            month: 1,
            day: 1,
            tai_minus_utc: 32,
        },
        LeapEntry {
            year: 2000,
            month: 1,
            day: 1,
            tai_minus_utc: 31,
        },
    ];
    assert_eq!(
        LeapSeconds::new(&shrinking).map(|_| ()),
        Err(Error::InvalidLeapSeconds)
    );
}
