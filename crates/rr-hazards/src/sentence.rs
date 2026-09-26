//! Natural-frequency sentences: "Of 100 households like yours, about 12 will … in the next ten
//! years."
//!
//! The count is `100 · (1 − e^(−T·r))` for a rate `r` a year over `T = dials.horizon_years`
//! (DESIGN §4.4). Rounding (research §7.1–7.2, brief): at most two significant figures; whole
//! numbers between 1 and 10; below 1 in 100 the sentence switches to "in 1,000" and then to
//! "1 in N" so rare hazards keep their size; "nearly every household" from 99.5 in 100. Ranges
//! are given when the estimate rests on expert judgement.

use rr_types::math;

/// Rounds `x > 0` to two significant figures.
pub(crate) fn round_sig2(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return 0.0;
    }
    let mut e = (math::ln(x) / core::f64::consts::LN_10).floor();
    // Guard against ln rounding just across a power of ten.
    if math::pow(10.0, e) > x {
        e -= 1.0;
    } else if math::pow(10.0, e + 1.0) <= x {
        e += 1.0;
    }
    let scale = math::pow(10.0, e - 1.0);
    (x / scale).round() * scale
}

/// A whole number with thousands separators: 12500 -> "12,500".
pub(crate) fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// A number to at most two significant figures, without trailing zeros: 1.9, 0.083, 81, 2,500.
pub(crate) fn sig2(x: f64) -> String {
    let r = round_sig2(x);
    if r >= 100.0 {
        thousands(r as u64)
    } else if r >= 10.0 {
        format!("{r:.0}")
    } else {
        // Two significant figures below 10: 1.9, 0.26, 0.0083.
        let decimals = if r >= 1.0 {
            1
        } else {
            let e = (math::ln(r) / core::f64::consts::LN_10).floor();
            (1.0 - e) as usize
        };
        let s = format!("{r:.decimals$}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_owned()
    }
}

fn number_word(n: u8) -> Option<&'static str> {
    Some(match n {
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        10 => "ten",
        _ => return None,
    })
}

/// "in the next ten years", "in the next year", "in the next 30 years"; with `around_2050`,
/// "in a ten-year stretch around 2050".
pub(crate) fn horizon_phrase(years: u8, around_2050: bool) -> String {
    let years = years.max(1);
    match (around_2050, years) {
        (false, 1) => "in the next year".to_owned(),
        (true, 1) => "in a year around 2050".to_owned(),
        (false, n) => match number_word(n) {
            Some(w) => format!("in the next {w} years"),
            None => format!("in the next {n} years"),
        },
        (true, n) => match number_word(n) {
            Some(w) => format!("in a {w}-year stretch around 2050"),
            None => format!("in a {n}-year stretch around 2050"),
        },
    }
}

/// Chance of at least one event in `years` at `rate` a year.
pub(crate) fn chance_within(rate: f64, years: u8) -> f64 {
    if rate <= 0.0 {
        return 0.0;
    }
    -math::exp_m1(-rate * f64::from(years.max(1)))
}

/// How a count out of 100 reads: "fewer than 1", "5", "45", "nearly all".
fn per_100_word(per100: f64) -> String {
    if per100 >= 99.5 {
        "nearly all".to_owned()
    } else if per100 < 0.5 {
        "fewer than 1".to_owned()
    } else if per100 < 10.0 {
        format!("{:.0}", per100.round())
    } else {
        format!("{:.0}", round_sig2(per100))
    }
}

/// How a count out of 1,000 reads.
fn per_1000_word(per1000: f64) -> String {
    if per1000 < 0.5 {
        "fewer than 1".to_owned()
    } else if per1000 < 10.0 {
        format!("{:.0}", per1000.round())
    } else {
        thousands(round_sig2(per1000) as u64)
    }
}

/// "1 in 2,500" for a chance `p`.
fn one_in(p: f64) -> String {
    // 0.95 in a million rounds to "1 in 1,000,000" (two significant figures).
    if p < 0.95e-6 {
        "fewer than 1 in 1,000,000".to_owned()
    } else {
        format!("1 in {}", thousands(round_sig2(1.0 / p) as u64))
    }
}

fn range(lo: &str, hi: &str) -> String {
    let numeric = |s: &str| s.chars().all(|c| c.is_ascii_digit() || c == ',');
    if lo == hi {
        String::new()
    } else if numeric(lo) && numeric(hi) {
        format!(" ({lo}–{hi})")
    } else {
        format!(" ({lo} to {hi})")
    }
}

/// The inputs to one sentence.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Frequency {
    /// Events a year.
    pub rate: f64,
    /// Low end of the rate.
    pub low: f64,
    /// High end of the rate.
    pub high: f64,
    /// Show the range (expert estimates).
    pub show_range: bool,
    /// `dials.horizon_years`.
    pub years: u8,
    /// The rate is a projection for around 2050.
    pub around_2050: bool,
}

/// The natural-frequency sentence for `f`; `verb` completes "households like yours will …",
/// for example "have someone lose a job".
pub(crate) fn natural_frequency(f: Frequency, verb: &str) -> String {
    let when = horizon_phrase(f.years, f.around_2050);
    let p = chance_within(f.rate, f.years);
    let (pl, ph) = (
        chance_within(f.low, f.years),
        chance_within(f.high, f.years),
    );
    let mut s = if p * 100.0 >= 99.5 {
        format!("Nearly every household like yours will {verb} {when}.")
    } else if p * 100.0 >= 0.95 {
        let r = if f.show_range {
            range(&per_100_word(pl * 100.0), &per_100_word(ph * 100.0))
        } else {
            String::new()
        };
        format!(
            "Of 100 households like yours, about {}{r} will {verb} {when}.",
            per_100_word(p * 100.0)
        )
    } else if p * 1000.0 >= 0.95 {
        let r = if f.show_range {
            range(&per_1000_word(pl * 1000.0), &per_1000_word(ph * 1000.0))
        } else {
            String::new()
        };
        format!(
            "About {}{r} in 1,000 households like yours will {verb} {when}.",
            per_1000_word(p * 1000.0)
        )
    } else if p >= 1.0e-5 {
        let r = if f.show_range {
            range(&one_in(pl), &one_in(ph))
        } else {
            String::new()
        };
        format!(
            "About {}{r} households like yours will {verb} {when}.",
            one_in(p)
        )
    } else {
        format!("Fewer than 1 in 100,000 households like yours will {verb} {when}.")
    };
    if f.rate >= 1.0 {
        // "Nearly every household" hides how often; say it.
        s.pop();
        s.push_str(&format!(" ({}).", about_times_a_year(f.rate)));
    }
    s
}

/// A follow-on sentence about one part of a hazard's events, after the card's own sentence has
/// said "households like yours" and the horizon: "About 10 (4–28) of 100 will have one damage
/// their home."
pub(crate) fn part_frequency(f: Frequency, verb: &str) -> String {
    let p = chance_within(f.rate, f.years);
    let (pl, ph) = (
        chance_within(f.low, f.years),
        chance_within(f.high, f.years),
    );
    if p * 100.0 >= 99.5 {
        format!("Nearly all will {verb}.")
    } else if p * 100.0 >= 0.95 {
        let r = if f.show_range {
            range(&per_100_word(pl * 100.0), &per_100_word(ph * 100.0))
        } else {
            String::new()
        };
        format!("About {}{r} of 100 will {verb}.", per_100_word(p * 100.0))
    } else if p * 1000.0 >= 0.95 {
        let r = if f.show_range {
            range(&per_1000_word(pl * 1000.0), &per_1000_word(ph * 1000.0))
        } else {
            String::new()
        };
        format!(
            "About {}{r} in 1,000 will {verb}.",
            per_1000_word(p * 1000.0)
        )
    } else if p >= 1.0e-5 {
        let r = if f.show_range {
            range(&one_in(pl), &one_in(ph))
        } else {
            String::new()
        };
        format!("About {}{r} will {verb}.", one_in(p))
    } else {
        format!("Fewer than 1 in 100,000 will {verb}.")
    }
}

/// A yearly rate in words: "about 2.4 times a year", "about once a year" (never "about 1 times
/// a year"), "about once every 3 years".
pub(crate) fn about_times_a_year(rate: f64) -> String {
    if rate >= 1.05 {
        format!("about {} times a year", sig2(rate))
    } else if rate >= 0.75 {
        "about once a year".to_owned()
    } else if rate > 0.0 {
        format!(
            "about once every {} years",
            thousands((1.0 / rate).round().max(2.0) as u64)
        )
    } else {
        "never".to_owned()
    }
}

/// A range-only sentence for a rare catastrophe (research §6.3): never a point estimate.
#[allow(dead_code)] // kept for sentences that quote a published yearly range
pub(crate) fn range_only(lead: &str, low: f64, high: f64, tail: &str) -> String {
    format!(
        "{lead} about {} to about {} a year.{tail}",
        one_in(low),
        one_in(high)
    )
}

/// Ranges wider than this (low to high) are said in words, not numbers (REVIEW §2.4).
pub(crate) const RANGE_IN_WORDS_ABOVE: f64 = 1000.0;

/// The range-only sentence of a rare row or a stacked-prior rate (REVIEW §2.4, H-02): "Between
/// 1 in 1,700 and 1 in 26 households like yours would … in the next ten years." Never a point
/// estimate. A range spanning more than a thousandfold is said in words.
pub(crate) fn range_sentence(verb: &str, low: f64, high: f64, years: u8) -> String {
    let when = horizon_phrase(years, false);
    let (pl, ph) = (chance_within(low, years), chance_within(high, years));
    if ph <= 0.0 {
        return format!("No household like yours is expected to {verb} {when}.");
    }
    if low <= 0.0 || high / low > RANGE_IN_WORDS_ABOVE {
        return format!(
            "Expert estimates for this span more than a thousandfold. At most about {} \
             households like yours would {verb} {when}.",
            one_in(ph)
        );
    }
    if pl < 0.95e-6 {
        return format!(
            "At most about {} households like yours would {verb} {when}, and perhaps fewer than \
             1 in 1,000,000.",
            one_in(ph)
        );
    }
    format!(
        "Between {} and {} households like yours would {verb} {when}.",
        one_in(pl),
        one_in(ph)
    )
}

/// A chance over the horizon in words, for the anchor: "about 5 in 100", "about 3 in 1,000",
/// "about 1 in 2,500", "nearly certain".
pub(crate) fn chance_words(p: f64) -> String {
    if p >= 0.995 {
        "nearly certain".to_owned()
    } else if p * 100.0 >= 0.95 {
        format!("about {} in 100", per_100_word(p * 100.0))
    } else if p * 1000.0 >= 0.95 {
        format!("about {} in 1,000", per_1000_word(p * 1000.0))
    } else if p >= 1.0e-6 {
        format!("about {}", one_in(p))
    } else {
        "fewer than 1 in 1,000,000".to_owned()
    }
}

/// The anchor of a rare row: "Less likely than a house fire (about 5 in 100 for you in the next
/// ten years)."
pub(crate) fn anchor(phrase: &str, rate: f64, years: u8) -> String {
    format!(
        "Less likely than {phrase} ({} for you {}).",
        chance_words(chance_within(rate, years)),
        horizon_phrase(years, false)
    )
}

/// A yearly rate in words for the "Also checked" line: "about 1 in 110,000 a year", "fewer than
/// 1 in 1,000,000 a year", "none recorded here".
pub(crate) fn per_year_words(rate: f64) -> String {
    if rate <= 0.0 {
        "none recorded here".to_owned()
    } else if rate >= 1.0e-6 {
        format!("about {} a year", one_in(rate))
    } else {
        "fewer than 1 in 1,000,000 a year".to_owned()
    }
}

/// Arrests for every 100 (or 1,000) households a year, in whole numbers: "about 7 arrests for
/// every 100 households", "about 3 arrests for every 1,000 households".
pub(crate) fn arrests_per_households(rate: f64) -> String {
    let per100 = rate * 100.0;
    if per100 >= 1.5 {
        format!(
            "about {:.0} arrests for every 100 households",
            per100.round()
        )
    } else if per100 >= 0.95 {
        "about 1 arrest for every 100 households".to_owned()
    } else if rate * 1000.0 >= 0.95 {
        format!(
            "about {:.0} arrests for every 1,000 households",
            (rate * 1000.0).round()
        )
    } else {
        "fewer than 1 arrest for every 1,000 households".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn freq(rate: f64, low: f64, high: f64, show_range: bool) -> Frequency {
        Frequency {
            rate,
            low,
            high,
            show_range,
            years: 10,
            around_2050: false,
        }
    }

    #[test]
    fn rounding_to_two_significant_figures() {
        for (x, want) in [
            (81.23, 81.0),
            (0.0162, 0.016),
            (2517.0, 2500.0),
            (9.96, 10.0),
            (1.0, 1.0),
            (1000.0, 1000.0),
            (0.1, 0.1),
            (123_456.0, 120_000.0),
        ] {
            let got = round_sig2(x);
            assert!((got - want).abs() <= 1e-9 * want, "{x}: {got} != {want}");
        }
        assert_eq!(sig2(1.892), "1.9");
        assert_eq!(sig2(0.0833), "0.083");
        assert_eq!(sig2(2517.0), "2,500");
        assert_eq!(sig2(44.6), "45");
        assert_eq!(thousands(1_000_000), "1,000,000");
        assert_eq!(thousands(999), "999");
    }

    #[test]
    fn job_loss_for_two_earners_reads_81_in_100() {
        // Research §8.3: two earners at 0.083 a year -> 1 − e^(−1.66) = 81 in 100 over ten years.
        let s = natural_frequency(freq(0.166, 0.12, 0.252, false), "have someone lose a job");
        assert_eq!(
            s,
            "Of 100 households like yours, about 81 will have someone lose a job in the next ten years."
        );
    }

    #[test]
    fn small_chances_switch_to_per_thousand_and_one_in_n() {
        let fire = natural_frequency(freq(0.005_244, 0.003, 0.009, false), "have a house fire");
        assert_eq!(
            fire,
            "Of 100 households like yours, about 5 will have a house fire in the next ten years."
        );
        let quake = natural_frequency(freq(0.000_5, 0.000_2, 0.001, true), "feel strong shaking");
        assert_eq!(
            quake,
            "About 5 (2–10) in 1,000 households like yours will feel strong shaking in the next ten years."
        );
        let rare = natural_frequency(freq(2.0e-5, 1.0e-5, 5.0e-5, true), "x");
        assert_eq!(
            rare,
            "About 1 in 5,000 (1 in 10,000 to 1 in 2,000) households like yours will x in the next ten years."
        );
        let tiny = natural_frequency(freq(1.0e-7, 1.0e-8, 1.0e-6, true), "x");
        assert!(tiny.starts_with("Fewer than 1 in 100,000"), "{tiny}");
    }

    #[test]
    fn frequent_events_say_how_often() {
        let s = natural_frequency(
            freq(1.892, 1.4, 2.6, true),
            "have someone need emergency care",
        );
        assert_eq!(
            s,
            "Nearly every household like yours will have someone need emergency care in the next ten years (about 1.9 times a year)."
        );
    }

    #[test]
    fn about_once_a_year_not_1_times() {
        let s = natural_frequency(freq(1.03, 0.8, 1.3, false), "go through a cold wave");
        assert!(s.ends_with("(about once a year)."), "{s}");
        assert!(!s.contains("1 times"), "{s}");
        assert_eq!(about_times_a_year(2.44), "about 2.4 times a year");
        assert_eq!(about_times_a_year(1.0), "about once a year");
        assert_eq!(about_times_a_year(0.83), "about once a year");
        assert_eq!(about_times_a_year(0.3), "about once every 3 years");
    }

    #[test]
    fn ranges_with_words() {
        let s = natural_frequency(freq(0.003, 0.0001, 0.2, true), "x");
        assert_eq!(
            s,
            "Of 100 households like yours, about 3 (fewer than 1 to 86) will x in the next ten years."
        );
    }

    #[test]
    fn horizons() {
        assert_eq!(horizon_phrase(1, false), "in the next year");
        assert_eq!(horizon_phrase(5, false), "in the next five years");
        assert_eq!(horizon_phrase(30, false), "in the next 30 years");
        assert_eq!(
            horizon_phrase(10, true),
            "in a ten-year stretch around 2050"
        );
        let mut f = freq(0.166, 0.12, 0.252, false);
        f.years = 1;
        assert_eq!(
            natural_frequency(f, "have someone lose a job"),
            "Of 100 households like yours, about 15 will have someone lose a job in the next year."
        );
    }

    #[test]
    fn rare_catastrophe_range_sentence() {
        let s = range_only(
            "Spread over those years, that is",
            1.0 / 2000.0,
            1.0 / 400.0,
            " No reliable estimate exists for effects where you live.",
        );
        assert_eq!(
            s,
            "Spread over those years, that is about 1 in 2,000 to about 1 in 400 a year. No reliable estimate exists for effects where you live."
        );
    }
}
