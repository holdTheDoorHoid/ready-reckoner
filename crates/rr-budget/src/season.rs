//! Season-aware ordering (brief item 5; REVIEW N8): with an item's season anchor
//! (`Item::season`, "have it before the season starts, or check it then") and the planning date,
//! a fan lands before the hot season and the fall checks (the generator, the carbon monoxide
//! alarm, blankets, the camp stove) before winter, when the cost order allows.
//!
//! **The rule.** In plan month *m* (calendar month = the planning date's month plus *m*), an item
//! anchored to a season is *due* while its season is under way or starts within
//! [`SEASON_LEAD_MONTHS`] months. Within the current tier, after life-safety items and
//! capabilities, a due item costing no more than one month's money moves ahead of the other
//! items, which keep their value-per-dollar order.
//!
//! **Its limits.** The rule only reorders the current tier's candidates: it never moves an item
//! into an earlier tier, never ahead of a life-safety item or a capability, never opens a sinking
//! fund for a seasonal item (an item dearer than a month's money waits for its usual turn), and
//! never buys what the month's money cannot. So a two-week-tier fan still arrives after summer
//! starts when the three-day basics take the whole spring. It applies to the default split
//! schedule only: the fixed-order schedule keeps an order that never depends on the calendar, so
//! more money still only ever moves purchases earlier there.

use rr_types::{Date, Season};

/// A seasonal item is due this many months before its season starts (and while it lasts).
pub const SEASON_LEAD_MONTHS: u8 = 2;

/// The calendar month (1 to 12) of plan month `m`, counting month 0 as the planning date's month.
pub fn calendar_month(planning: Date, m: u16) -> u8 {
    let start = u16::from(planning.month().clamp(1, 12)) - 1;
    ((start + m % 12) % 12 + 1) as u8
}

/// Whether calendar month `month` (1 to 12) falls inside `season`.
pub fn in_season(season: Season, month: u8) -> bool {
    let start = season.start_month();
    (0..3).any(|k| (start - 1 + k) % 12 + 1 == month)
}

/// Months from calendar month `month` until `season` next starts (0 when it starts this month).
pub fn months_until(season: Season, month: u8) -> u8 {
    (season.start_month() + 12 - month) % 12
}

/// Whether an item anchored to `season` is due in calendar month `month`: its season is under
/// way, or starts within [`SEASON_LEAD_MONTHS`] months.
pub fn due(season: Season, month: u8) -> bool {
    in_season(season, month) || months_until(season, month) <= SEASON_LEAD_MONTHS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_months_wrap() {
        let oct = Date::from_ymd(2026, 10, 1).unwrap();
        assert_eq!(calendar_month(oct, 0), 10);
        assert_eq!(calendar_month(oct, 2), 12);
        assert_eq!(calendar_month(oct, 3), 1);
        assert_eq!(calendar_month(oct, 8), 6);
        assert_eq!(calendar_month(oct, 26), 12);
    }

    #[test]
    fn due_windows() {
        // Summer (June to August): due from April.
        let summer: Vec<u8> = (1..=12).filter(|&m| due(Season::Summer, m)).collect();
        assert_eq!(summer, [4, 5, 6, 7, 8]);
        // Fall (September to November), the heating checks: due from July, and all through
        // October and November, before winter.
        let fall: Vec<u8> = (1..=12).filter(|&m| due(Season::Fall, m)).collect();
        assert_eq!(fall, [7, 8, 9, 10, 11]);
        // Winter wraps the year: December to February, due from October.
        let winter: Vec<u8> = (1..=12).filter(|&m| due(Season::Winter, m)).collect();
        assert_eq!(winter, [1, 2, 10, 11, 12]);
        assert!(in_season(Season::Winter, 1) && !in_season(Season::Winter, 3));
        assert_eq!(months_until(Season::Summer, 10), 8);
    }
}
