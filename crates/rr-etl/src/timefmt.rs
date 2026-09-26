//! UTC timestamps for the manifest, without a date-time dependency.
//!
//! The ETL is the only place in the project allowed to read the wall clock, and only to record
//! when a source was retrieved. Pack contents never depend on it.

use std::time::{SystemTime, UNIX_EPOCH};

/// The current time as an ISO 8601 UTC timestamp with second resolution, e.g.
/// `2026-09-25T20:15:03Z`.
pub fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_unix(secs)
}

/// Today's UTC date as `YYYY-MM-DD`.
pub fn today_utc() -> String {
    now_utc()[..10].to_string()
}

/// Format seconds since the Unix epoch as an ISO 8601 UTC timestamp.
///
/// ```
/// assert_eq!(rr_etl::timefmt::format_unix(0), "1970-01-01T00:00:00Z");
/// assert_eq!(rr_etl::timefmt::format_unix(1_790_380_800), "2026-09-26T00:00:00Z");
/// ```
pub fn format_unix(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 to a proleptic Gregorian (year, month, day). Howard Hinnant's
/// `civil_from_days`.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Proleptic Gregorian (year, month, day) to days since 1970-01-01. Inverse of
/// [`civil_from_days`].
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parse `YYYY-MM-DD HH:MM[:SS]` (or with a `T` separator) into seconds since the epoch,
/// treating it as UTC. Returns `None` if the text is not in that shape.
///
/// ```
/// assert_eq!(rr_etl::timefmt::parse_datetime("1970-01-02 00:15:00"), Some(87_300));
/// assert_eq!(rr_etl::timefmt::parse_datetime("bad"), None);
/// ```
pub fn parse_datetime(s: &str) -> Option<i64> {
    let b = s.trim().as_bytes();
    if b.len() < 16 {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        std::str::from_utf8(b.get(r)?).ok()?.parse::<i64>().ok()
    };
    let y = num(0..4)?;
    let mo = num(5..7)? as u32;
    let d = num(8..10)? as u32;
    let h = num(11..13)?;
    let mi = num(14..16)?;
    let sec = if b.len() >= 19 { num(17..19)? } else { 0 };
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    Some(days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_days() {
        for z in [-800_000_i64, -1, 0, 1, 10_957, 20_721, 2_932_896] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(civil_from_days(20_721), (2026, 9, 25));
    }
}
