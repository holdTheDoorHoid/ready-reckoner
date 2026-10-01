//! How likely a hazard is over the dial's horizon, in the words the binder and the Prepare sheet
//! print: [`chance`] (at least one event in so many years) and [`range_words`] (a rare family's
//! range, never a point). The v2 packet's risk cards are retired: the binder gives every ranked
//! hazard its own checklist page, and the cards' advice stays on the app's Risks screen.

use super::text;

/// A hazard that strikes with minutes of warning puts leaving first from this chance in ten
/// years (the v2 packet's card threshold, `super::summary::leave_first`).
pub const CARD_MIN_P10: f64 = 0.10;

/// The chance of at least one event in `years` at a yearly rate.
pub(crate) fn chance(rate: f64, years: u8) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -rr_types::math::exp_m1(-rate * f64::from(years.max(1)))
    }
}

/// "1 in 2,600": two significant figures, whole numbers below 10; "fewer than 1 in 1,000,000".
fn one_in(p: f64) -> String {
    if p < 1e-6 {
        return "fewer than 1 in 1,000,000".to_owned();
    }
    let n = 1.0 / p;
    if n < 10.0 {
        return format!("1 in {}", n.round().max(2.0) as u64);
    }
    // Two significant figures.
    let mut scale = 1.0_f64;
    while n / scale >= 100.0 {
        scale *= 10.0;
    }
    let r = ((n / scale).round() * scale) as u64;
    format!("1 in {}", text::thousands(r))
}

/// A rare row's chance over the horizon as a range only (never a point; `range_only`), worded
/// as the app words it (`web/src/lib/format.ts` `rangeOnly`): "between 1 in 3,300 and 1 in 28",
/// or "very unlikely: less than 1 in 25,000" when the range spans more than a thousandfold.
pub(crate) fn range_words(low: f64, high: f64, years: u8) -> String {
    let lo = chance(low.max(0.0), years);
    let hi = chance(high.max(0.0), years);
    if hi <= 0.0 {
        return "no known chance".to_owned();
    }
    if low <= 0.0 || high / low > 1000.0 || lo < 1e-6 {
        return format!("very unlikely: less than {}", one_in(hi));
    }
    let (a, b) = (one_in(lo), one_in(hi));
    if a == b {
        format!("about {a}")
    } else {
        format!("between {a} and {b}")
    }
}
