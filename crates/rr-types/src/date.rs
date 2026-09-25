//! A minimal calendar date: ISO `YYYY-MM-DD`, proleptic Gregorian, years 1 to 9999.
//!
//! The engine never reads the wall clock; the planning date is an input. This type is enough for
//! the plan's month numbering and the maintenance calendar, builds for wasm32, and has no
//! dependencies.

use core::fmt;
use core::str::FromStr;

/// Why a date was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DateError {
    /// The text is not exactly `YYYY-MM-DD`.
    #[error("dates are written YYYY-MM-DD, like 2026-10-01; got {0:?}")]
    Format(String),
    /// The parts are well formed but not a real calendar day (or the year is outside 1 to 9999).
    #[error("{year:04}-{month:02}-{day:02} is not a real date")]
    OutOfRange {
        /// The year given.
        year: u16,
        /// The month given.
        month: u8,
        /// The day given.
        day: u8,
    },
}

/// A calendar date. Ordering is chronological. Serialises as an ISO `YYYY-MM-DD` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    // Field order matters: the derived `Ord` compares year, then month, then day.
    year: u16,
    month: u8,
    day: u8,
}

/// Days from 0001-01-01 to 1970-01-01, used to bound the epoch-day range.
const MIN_EPOCH_DAY: i32 = -719_162;
/// Days from 1970-01-01 to 9999-12-31.
const MAX_EPOCH_DAY: i32 = 2_932_896;

impl Date {
    /// The earliest supported date, 0001-01-01.
    pub const MIN: Date = Date {
        year: 1,
        month: 1,
        day: 1,
    };

    /// The latest supported date, 9999-12-31.
    pub const MAX: Date = Date {
        year: 9999,
        month: 12,
        day: 31,
    };

    /// Builds a date, checking that it exists (leap years included) and that the year is 1 to
    /// 9999.
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Date, DateError> {
        match Self::from_ymd(year, month, day) {
            Some(date) => Ok(date),
            None => Err(DateError::OutOfRange { year, month, day }),
        }
    }

    /// Like [`Date::new`], returning `None` for a date that does not exist. Usable in `const`
    /// items.
    pub const fn from_ymd(year: u16, month: u8, day: u8) -> Option<Date> {
        if year >= 1
            && year <= 9999
            && month >= 1
            && month <= 12
            && day >= 1
            && day <= Self::days_in_month(year, month)
        {
            Some(Date { year, month, day })
        } else {
            None
        }
    }

    /// The year, 1 to 9999.
    pub const fn year(self) -> u16 {
        self.year
    }

    /// The month, 1 to 12.
    pub const fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, 1 to 31.
    pub const fn day(self) -> u8 {
        self.day
    }

    /// True for Gregorian leap years (divisible by 4, except centuries not divisible by 400).
    pub const fn is_leap_year(year: u16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    /// Days in `month` of `year`; 0 for a month outside 1 to 12.
    pub const fn days_in_month(year: u16, month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            _ => 0,
        }
    }

    /// Parses exactly `YYYY-MM-DD` (four-digit year, two-digit month and day, hyphens). Anything
    /// else, including times, signs and whitespace, is rejected.
    pub fn parse(s: &str) -> Result<Date, DateError> {
        let b = s.as_bytes();
        let shape_ok = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
        if !shape_ok {
            return Err(DateError::Format(s.to_owned()));
        }
        let num = |range: core::ops::Range<usize>| {
            b[range]
                .iter()
                .fold(0u16, |acc, c| acc * 10 + u16::from(c - b'0'))
        };
        // Month and day have two digits, so they fit in a u8.
        let month = u8::try_from(num(5..7)).unwrap_or(u8::MAX);
        let day = u8::try_from(num(8..10)).unwrap_or(u8::MAX);
        Date::new(num(0..4), month, day)
    }

    /// Days since 1970-01-01 (negative before it). Uses Howard Hinnant's `days_from_civil`.
    pub const fn days_since_epoch(self) -> i32 {
        let m = self.month as i32;
        let d = self.day as i32;
        let y = self.year as i32 - if m <= 2 { 1 } else { 0 };
        let era = y.div_euclid(400);
        let yoe = y - era * 400; // [0, 399]
        let mp = if m > 2 { m - 3 } else { m + 9 }; // March = 0
        let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
        era * 146_097 + doe - 719_468
    }

    /// The date `days` after 1970-01-01, or `None` outside 0001-01-01 to 9999-12-31. Uses Howard
    /// Hinnant's `civil_from_days`.
    pub const fn from_days_since_epoch(days: i32) -> Option<Date> {
        if days < MIN_EPOCH_DAY || days > MAX_EPOCH_DAY {
            return None;
        }
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
        let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
        let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
        // In range by the bounds check above; the casts cannot truncate.
        Some(Date {
            year: year as u16,
            month: month as u8,
            day: day as u8,
        })
    }

    /// The date `days` later (earlier if negative), or `None` if that leaves the supported range.
    pub const fn add_days(self, days: i32) -> Option<Date> {
        match self.days_since_epoch().checked_add(days) {
            Some(d) => Self::from_days_since_epoch(d),
            None => None,
        }
    }

    /// The same day `months` later (earlier if negative), clamped to the end of a shorter month
    /// (2026-01-31 plus one month is 2026-02-28), or `None` if that leaves the supported range.
    pub const fn add_months(self, months: i32) -> Option<Date> {
        let total = self.year as i64 * 12 + (self.month as i64 - 1) + months as i64;
        let year = total.div_euclid(12);
        if year < 1 || year > 9999 {
            return None;
        }
        let year = year as u16;
        let month = (total.rem_euclid(12) + 1) as u8;
        let last = Self::days_in_month(year, month);
        let day = if self.day > last { last } else { self.day };
        Some(Date { year, month, day })
    }

    /// Whole days from `self` to `later` (negative if `later` is earlier).
    pub const fn days_until(self, later: Date) -> i32 {
        later.days_since_epoch() - self.days_since_epoch()
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for Date {
    type Err = DateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Date::parse(s)
    }
}

impl serde::Serialize for Date {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Date {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DateVisitor;

        impl serde::de::Visitor<'_> for DateVisitor {
            type Value = Date;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a date written YYYY-MM-DD")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Date, E> {
                Date::parse(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(DateVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    #[test]
    fn parses_and_formats_iso() {
        let x = d("2026-10-01");
        assert_eq!((x.year(), x.month(), x.day()), (2026, 10, 1));
        assert_eq!(x.to_string(), "2026-10-01");
        assert_eq!(d("0001-01-01"), Date::MIN);
        assert_eq!(d("9999-12-31"), Date::MAX);
        assert_eq!(Date::MIN.to_string(), "0001-01-01");
    }

    #[test]
    fn rejects_malformed_text() {
        for bad in [
            "",
            "2026-10-1",
            "2026-1-01",
            "26-10-01",
            "2026/10/01",
            "2026-10-01T00:00:00",
            " 2026-10-01",
            "2026-10-01 ",
            "+2026-10-01",
            "2026-1a-01",
            "２０２６-10-01",
            "20261001",
        ] {
            assert!(
                matches!(Date::parse(bad), Err(DateError::Format(_))),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn rejects_days_that_do_not_exist() {
        for bad in [
            "2026-02-29",
            "2100-02-29",
            "2026-04-31",
            "2026-13-01",
            "2026-00-10",
            "2026-10-00",
            "0000-01-01",
            "2026-12-32",
        ] {
            assert!(
                matches!(Date::parse(bad), Err(DateError::OutOfRange { .. })),
                "{bad:?}"
            );
        }
        for good in ["2024-02-29", "2000-02-29", "2026-12-31", "2026-04-30"] {
            assert!(Date::parse(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert!(Date::is_leap_year(2024));
        assert!(Date::is_leap_year(2000));
        assert!(!Date::is_leap_year(2100));
        assert!(!Date::is_leap_year(2026));
        assert_eq!(Date::days_in_month(2024, 2), 29);
        assert_eq!(Date::days_in_month(2026, 2), 28);
        assert_eq!(Date::days_in_month(2026, 13), 0);
    }

    #[test]
    fn epoch_days_match_python_datetime() {
        // Reference values from Python: (date(y, m, d) - date(1970, 1, 1)).days
        for (s, days) in [
            ("1970-01-01", 0),
            ("1969-12-31", -1),
            ("2000-03-01", 11_017),
            ("2024-02-29", 19_782),
            ("2026-10-01", 20_727),
            ("0001-01-01", -719_162),
            ("9999-12-31", 2_932_896),
        ] {
            assert_eq!(d(s).days_since_epoch(), days, "{s}");
            assert_eq!(Date::from_days_since_epoch(days), Some(d(s)), "{s}");
        }
        assert_eq!(Date::from_days_since_epoch(-719_163), None);
        assert_eq!(Date::from_days_since_epoch(2_932_897), None);
    }

    #[test]
    fn every_day_round_trips_through_epoch_days() {
        let mut prev = Date::from_days_since_epoch(-800).unwrap();
        for n in -799..40_000 {
            let x = Date::from_days_since_epoch(n).unwrap();
            assert_eq!(x.days_since_epoch(), n);
            assert!(x > prev);
            assert_eq!(Date::parse(&x.to_string()), Ok(x));
            prev = x;
        }
    }

    #[test]
    fn adds_days_and_months() {
        assert_eq!(d("2026-10-01").add_days(90), Some(d("2026-12-30")));
        assert_eq!(d("2026-10-01").add_days(365), Some(d("2027-10-01")));
        assert_eq!(d("2024-02-29").add_days(-1), Some(d("2024-02-28")));
        assert_eq!(d("2024-02-29").add_days(366), Some(d("2025-03-01")));
        assert_eq!(Date::MAX.add_days(1), None);
        assert_eq!(Date::MIN.add_days(-1), None);
        assert_eq!(d("2026-01-31").add_months(1), Some(d("2026-02-28")));
        assert_eq!(d("2024-01-31").add_months(1), Some(d("2024-02-29")));
        assert_eq!(d("2026-10-01").add_months(6), Some(d("2027-04-01")));
        assert_eq!(d("2026-12-15").add_months(1), Some(d("2027-01-15")));
        assert_eq!(d("2026-03-31").add_months(-1), Some(d("2026-02-28")));
        assert_eq!(d("2026-01-15").add_months(-13), Some(d("2024-12-15")));
        assert_eq!(d("2026-10-01").add_months(0), Some(d("2026-10-01")));
        assert_eq!(Date::MAX.add_months(1), None);
        assert_eq!(Date::MIN.add_months(-1), None);
        assert_eq!(d("2026-10-01").days_until(d("2027-10-01")), 365);
        assert_eq!(d("2027-10-01").days_until(d("2026-10-01")), -365);
    }

    #[test]
    fn serde_uses_the_iso_string() {
        let x = d("2026-10-01");
        assert_eq!(serde_json::to_string(&x).unwrap(), "\"2026-10-01\"");
        assert_eq!(serde_json::from_str::<Date>("\"2026-10-01\"").unwrap(), x);
        let err = serde_json::from_str::<Date>("\"2026-02-30\"").unwrap_err();
        assert!(err.to_string().contains("not a real date"), "{err}");
        assert!(serde_json::from_str::<Date>("20261001").is_err());
    }

    #[test]
    fn ordering_is_chronological() {
        assert!(d("2026-10-01") < d("2026-10-02"));
        assert!(d("2026-09-30") < d("2026-10-01"));
        assert!(d("2025-12-31") < d("2026-01-01"));
    }
}
