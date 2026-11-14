// SPDX-License-Identifier: MIT OR Apache-2.0
//! Epochs, UTC and Julian dates.
//!
//! An [`Epoch`] is a Terrestrial Time (TT) instant, stored as seconds past J2000, which is
//! 2000-01-01 12:00:00 TT (Julian date 2451545.0 TT). TAI is TT − 32.184 s by definition. UTC
//! differs from TAI by a whole number of seconds that changes at leap seconds. Those values come
//! from a table the caller supplies ([`LeapSeconds`]), because the crate bundles no data.
//!
//! Precision: the stored value is an `f64` of seconds, so it resolves about 0.5 µs at the present
//! epoch.
//!
//! Not modelled: TDB, whose periodic difference from TT needs an ephemeris-grade series; UT1 and
//! Earth orientation; leap-second insertion (a UTC second of 60 is rejected); and instants before
//! the first entry of the table.

use crate::Error;

/// TT − TAI in seconds, exact by definition.
const TT_MINUS_TAI: f64 = 32.184;
/// Days from 1970-01-01 to 2000-01-01.
const J2000_DAYS: i64 = 10_957;
/// Julian date of J2000 (TT).
const J2000_JD: f64 = 2_451_545.0;
const SECONDS_PER_DAY: i64 = 86_400;
/// J2000 is at noon, so seconds past J2000 are measured from midday.
const HALF_DAY: f64 = 43_200.0;

/// Days from 1970-01-01 to the given proleptic Gregorian date (Howard Hinnant's civil-date
/// algorithm, integer only).
pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The proleptic Gregorian date of `days` after 1970-01-01, as `(year, month, day)`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Seconds on the UTC scale past J2000 of a date (days since 1970-01-01) and seconds of the day.
fn utc_seconds(days: i64, seconds_of_day: f64) -> f64 {
    (days - J2000_DAYS) as f64 * SECONDS_PER_DAY as f64 + seconds_of_day - HALF_DAY
}

/// A UTC calendar time. The second is in `[0, 60)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UtcTime {
    year: i64,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: f64,
}

impl UtcTime {
    /// A UTC time. Fails with [`Error::InvalidCalendar`] if the date does not exist, or if the
    /// time of day is out of range (a second of 60 or more is rejected).
    pub fn new(
        year: i64,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: f64,
    ) -> Result<Self, Error> {
        let valid_date = (1..=12).contains(&month)
            && day >= 1
            && i64::from(day) <= days_in_month(year, i64::from(month));
        let valid_time =
            hour < 24 && minute < 60 && !second.is_nan() && (0.0..60.0).contains(&second);
        if !valid_date || !valid_time {
            return Err(Error::InvalidCalendar);
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    /// The year.
    pub fn year(&self) -> i64 {
        self.year
    }

    /// The month, 1 to 12.
    pub fn month(&self) -> u8 {
        self.month
    }

    /// The day of the month.
    pub fn day(&self) -> u8 {
        self.day
    }

    /// The hour, 0 to 23.
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The minute, 0 to 59.
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// The second, in `[0, 60)`.
    pub fn second(&self) -> f64 {
        self.second
    }

    fn days(&self) -> i64 {
        days_from_civil(self.year, i64::from(self.month), i64::from(self.day))
    }

    fn seconds_of_day(&self) -> f64 {
        f64::from(self.hour) * 3600.0 + f64::from(self.minute) * 60.0 + self.second
    }
}

/// One row of a leap-second table: from 00:00 UTC on the given date, TAI − UTC is `tai_minus_utc`
/// seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeapEntry {
    /// The year the offset takes effect.
    pub year: i64,
    /// The month the offset takes effect.
    pub month: u8,
    /// The day the offset takes effect.
    pub day: u8,
    /// TAI − UTC from that instant, in whole seconds.
    pub tai_minus_utc: i64,
}

/// A leap-second table supplied by the caller. Entries must be in increasing date order, and the
/// offsets must not decrease.
#[derive(Clone, Copy, Debug)]
pub struct LeapSeconds<'a> {
    entries: &'a [LeapEntry],
}

impl<'a> LeapSeconds<'a> {
    /// A table of `entries`. Fails with [`Error::InvalidLeapSeconds`] if it is empty or not in
    /// increasing order.
    pub fn new(entries: &'a [LeapEntry]) -> Result<Self, Error> {
        if entries.is_empty() {
            return Err(Error::InvalidLeapSeconds);
        }
        for pair in entries.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let a_date = (a.year, a.month, a.day);
            let b_date = (b.year, b.month, b.day);
            if b_date <= a_date || b.tai_minus_utc < a.tai_minus_utc {
                return Err(Error::InvalidLeapSeconds);
            }
        }
        Ok(Self { entries })
    }

    /// The UTC instant (seconds past J2000) at which an entry takes effect.
    fn start_utc(entry: &LeapEntry) -> f64 {
        utc_seconds(
            days_from_civil(entry.year, i64::from(entry.month), i64::from(entry.day)),
            0.0,
        )
    }

    /// TAI − UTC at a UTC instant (seconds past J2000 on the UTC scale).
    fn offset_at_utc(&self, utc: f64) -> Result<f64, Error> {
        self.entries
            .iter()
            .rfind(|entry| Self::start_utc(entry) <= utc)
            .map(|entry| entry.tai_minus_utc as f64)
            .ok_or(Error::NoLeapSecondEntry)
    }

    /// TAI − UTC at a TAI instant (seconds past J2000 on the TAI scale). An entry takes effect at
    /// the TAI instant that corresponds to its UTC start under the new offset.
    fn offset_at_tai(&self, tai: f64) -> Result<f64, Error> {
        self.entries
            .iter()
            .rfind(|entry| Self::start_utc(entry) + entry.tai_minus_utc as f64 <= tai)
            .map(|entry| entry.tai_minus_utc as f64)
            .ok_or(Error::NoLeapSecondEntry)
    }
}

/// An instant on the TT scale, stored as seconds past J2000 (2000-01-01 12:00:00 TT).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Epoch {
    tt_seconds: f64,
}

impl Epoch {
    /// J2000: 2000-01-01 12:00:00 TT.
    pub const J2000: Self = Self { tt_seconds: 0.0 };

    /// The instant `seconds` past J2000 on the TT scale. Fails if `seconds` is not finite.
    pub fn from_tt_seconds(seconds: f64) -> Result<Self, Error> {
        if seconds.is_finite() {
            Ok(Self {
                tt_seconds: seconds,
            })
        } else {
            Err(Error::NonFinite { what: "epoch" })
        }
    }

    /// The instant with TT Julian date `jd`. Fails if `jd` is not finite.
    pub fn from_tt_julian_date(jd: f64) -> Result<Self, Error> {
        Self::from_tt_seconds((jd - J2000_JD) * SECONDS_PER_DAY as f64)
    }

    /// Seconds past J2000 on the TT scale.
    pub fn tt_seconds(self) -> f64 {
        self.tt_seconds
    }

    /// The TT Julian date.
    pub fn tt_julian_date(self) -> f64 {
        J2000_JD + self.tt_seconds / SECONDS_PER_DAY as f64
    }

    /// Seconds past J2000 on the TAI scale (TT − 32.184 s).
    pub fn tai_seconds(self) -> f64 {
        self.tt_seconds - TT_MINUS_TAI
    }

    /// The instant of a UTC time, using `leap` for TAI − UTC. Fails with
    /// [`Error::NoLeapSecondEntry`] if the time is before the table's first entry.
    pub fn from_utc(utc: UtcTime, leap: &LeapSeconds<'_>) -> Result<Self, Error> {
        let utc_s = utc_seconds(utc.days(), utc.seconds_of_day());
        let offset = leap.offset_at_utc(utc_s)?;
        Self::from_tt_seconds(utc_s + offset + TT_MINUS_TAI)
    }

    /// The UTC time of this instant, using `leap` for TAI − UTC. Fails with
    /// [`Error::NoLeapSecondEntry`] if the instant is before the table's first entry.
    pub fn to_utc(self, leap: &LeapSeconds<'_>) -> Result<UtcTime, Error> {
        let tai = self.tai_seconds();
        let offset = leap.offset_at_tai(tai)?;
        let whole_and_fraction = tai - offset + HALF_DAY;
        // Split into whole seconds and a fraction in [0, 1), without floor().
        let mut whole = whole_and_fraction as i64;
        let mut fraction = whole_and_fraction - whole as f64;
        if fraction < 0.0 {
            whole -= 1;
            fraction += 1.0;
        }
        let days = J2000_DAYS + whole.div_euclid(SECONDS_PER_DAY);
        let seconds_of_day = whole.rem_euclid(SECONDS_PER_DAY) as f64 + fraction;
        let (year, month, day) = civil_from_days(days);
        let hour = (seconds_of_day / 3600.0) as u8;
        let minute = ((seconds_of_day - f64::from(hour) * 3600.0) / 60.0) as u8;
        let second = seconds_of_day - f64::from(hour) * 3600.0 - f64::from(minute) * 60.0;
        UtcTime::new(year, month as u8, day as u8, hour, minute, second)
    }
}
