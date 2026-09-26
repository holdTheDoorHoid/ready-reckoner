//! Rounding and number formatting for quantities and plain-language lines.
//!
//! Only IEEE basic operations and `round`/`ceil` are used, which give the same bits on every
//! target, so the engine's output is identical in the CLI and the browser.

/// Litres in one US gallon (a definition, not a sourced quantity).
pub(crate) const L_PER_GAL: f64 = 3.785_411_784;
/// Fluid ounces in one US gallon.
pub(crate) const OZ_PER_GAL: f64 = 128.0;
/// Kilometres in one mile.
pub(crate) const KM_PER_MILE: f64 = 1.609_344;
/// Cups in one US gallon.
pub(crate) const CUPS_PER_GAL: f64 = 16.0;
/// Days in a year, for per-year amounts.
pub(crate) const DAYS_PER_YEAR: f64 = 365.0;
/// Days in a month, for per-month amounts.
pub(crate) const DAYS_PER_MONTH: f64 = 30.0;

const POW10: [f64; 4] = [1.0, 10.0, 100.0, 1000.0];

/// Rounds half away from zero to `dp` decimal places (at most 3).
pub(crate) fn round_dp(x: f64, dp: usize) -> f64 {
    let p = POW10[dp.min(3)];
    (x * p).round() / p
}

/// Rounds to the nearest multiple of `step`.
pub(crate) fn round_to(x: f64, step: f64) -> f64 {
    (x / step).round() * step
}

/// Rounds up to a whole number, ignoring floating-point dust just above an integer.
pub(crate) fn ceil_count(x: f64) -> f64 {
    if x <= 0.0 { 0.0 } else { (x - 1e-9).ceil() }
}

/// Formats `x` with at most `dp` decimals, trailing zeros trimmed and thousands separated by
/// commas: `8199.16` with 0 decimals is `"8,199"`, `12.94` with 1 is `"12.9"`.
pub(crate) fn num(x: f64, dp: usize) -> String {
    let dp = dp.min(3);
    let mut r = round_dp(x, dp);
    if r == 0.0 {
        r = 0.0; // no "-0"
    }
    let s = format!("{r:.dp$}");
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i.to_owned(), f.trim_end_matches('0').to_owned()),
        None => (s.clone(), String::new()),
    };
    let (sign, digits) = match int.strip_prefix('-') {
        Some(d) => ("-", d.to_owned()),
        None => ("", int),
    };
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if frac.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}.{frac}")
    }
}

/// `one` when `n` is exactly 1, otherwise `many`.
pub(crate) fn noun<'a>(n: f64, one: &'a str, many: &'a str) -> &'a str {
    if (n - 1.0).abs() < 1e-9 { one } else { many }
}

/// "1 person", "2 people", "2.5 days".
pub(crate) fn count(n: f64, one: &str, many: &str) -> String {
    format!("{} {}", num(n, 1), noun(n, one, many))
}

/// "1 day", "3 days", "1.4 days".
pub(crate) fn days(n: f64) -> String {
    count(n, "day", "days")
}

/// "1 gallon", "12.9 gallons".
pub(crate) fn gallons(n: f64) -> String {
    format!(
        "{} {}",
        num(n, 1),
        noun(round_dp(n, 1), "gallon", "gallons")
    )
}

/// Whole dollars: "$1,234".
pub(crate) fn usd(x: f64) -> String {
    format!("${}", num(x, 0))
}

/// "4 people" for household members.
pub(crate) fn people(n: f64) -> String {
    count(n, "person", "people")
}

/// Joins phrases as "a", "a and b", "a, b and c".
pub(crate) fn and_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_trimmed_and_grouped() {
        assert_eq!(num(12.94, 1), "12.9");
        assert_eq!(num(12.96, 1), "13");
        assert_eq!(num(8199.16, 0), "8,199");
        assert_eq!(num(81_991.6, 0), "81,992");
        assert_eq!(num(1_234_567.0, 0), "1,234,567");
        assert_eq!(num(0.3125, 2), "0.31");
        assert_eq!(num(-0.01, 1), "0");
        assert_eq!(num(-1234.5, 1), "-1,234.5");
        assert_eq!(num(100.0, 2), "100");
        assert_eq!(num(0.0, 0), "0");
    }

    #[test]
    fn rounding_helpers() {
        assert_eq!(round_dp(12.9375, 1), 12.9);
        assert_eq!(round_dp(2.25, 1), 2.3);
        assert_eq!(round_to(81_991.6, 100.0), 82_000.0);
        assert_eq!(round_to(134.4, 10.0), 130.0);
        assert_eq!(ceil_count(5.4), 6.0);
        assert_eq!(ceil_count(4.000_000_000_01), 4.0);
        assert_eq!(ceil_count(0.0), 0.0);
        assert_eq!(ceil_count(-3.0), 0.0);
    }

    #[test]
    fn words() {
        assert_eq!(people(1.0), "1 person");
        assert_eq!(people(4.0), "4 people");
        assert_eq!(days(1.4), "1.4 days");
        assert_eq!(days(1.0), "1 day");
        assert_eq!(gallons(1.0), "1 gallon");
        assert_eq!(gallons(0.94), "0.9 gallons");
        assert_eq!(usd(15_960.4), "$15,960");
        let l = |v: &[&str]| and_list(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(l(&[]), "");
        assert_eq!(l(&["a"]), "a");
        assert_eq!(l(&["a", "b"]), "a and b");
        assert_eq!(l(&["a", "b", "c"]), "a, b and c");
    }
}
