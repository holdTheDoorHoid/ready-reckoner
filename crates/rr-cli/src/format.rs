//! How numbers and names read on the terminal, and a plain aligned table.
//!
//! The phrases follow the packet's rules (`rr-plan`'s packet text, `web/src/lib/format.ts`,
//! `docs/CONTENT_STANDARDS.md` §6) so the CLI, the app and the packet say the same thing: natural
//! frequencies out of 100, days on the ladder in the unit a person would use ("2 weeks"), whole
//! dollars with bands as "$30–45". rr-plan keeps its own copy private, so the few that the tables
//! need are repeated here.

use rr_types::{AgeBand, DataConfidence, PlanInput, ReturnPeriod};

/// Whole dollars with thousands separators: "$1,234"; under half a dollar is "under $1".
pub fn usd(x: f64) -> String {
    if x > 0.0 && x < 0.5 {
        return "under $1".to_owned();
    }
    format!("${}", thousands(x.round().max(0.0) as u64))
}

/// "1,234,567".
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A price band: "$30–45"; "$30" when both ends match; "free" at $0. Bands under $10 keep cents
/// ("$0–1.50") so cheap consumables do not all read "$0–2".
pub fn band(low: f64, high: f64) -> String {
    if low <= 0.0 && high <= 0.0 {
        return "free".to_owned();
    }
    let cents = high < 10.0 && (high.fract() != 0.0 || low.fract() != 0.0);
    let one = |x: f64| {
        if cents {
            format!("{x:.2}")
        } else {
            thousands(x.round().max(0.0) as u64)
        }
    };
    if (low - high).abs() < 0.005 {
        return format!("${}", one(low));
    }
    format!("${}–{}", one(low), one(high))
}

/// A quantity: whole numbers with separators, else one decimal.
pub fn number(q: f64) -> String {
    let r = (q * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-9 {
        thousands(r.round().max(0.0) as u64)
    } else {
        format!("{r:.1}")
    }
}

/// A rate with two significant figures: "0.083", "0.00012", "1.4", "12". The decimal places come
/// from the value as rounded to two figures, so 0.00999 prints "0.010", not "0.0100".
pub fn sig2(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return "0".to_owned();
    }
    let sci = format!("{x:.1e}");
    let exp: i32 = sci
        .split_once('e')
        .and_then(|(_, e)| e.parse().ok())
        .unwrap_or(0);
    let places = (1 - exp).max(0) as usize;
    format!("{x:.places$}")
}

/// Chance of at least one event in `years` years at `rate` a year: 1 − e^(−rate·years).
pub fn chance(rate: f64, years: f64) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -rr_types::math::exp_m1(-rate * years)
    }
}

/// Of 100 households, rounded as the app does: under 1 is "fewer than 1"; 1–10 whole; above 10
/// the nearest 5; 97.5 and above "almost all".
pub fn per_100(p: f64) -> String {
    let n = 100.0 * p.clamp(0.0, 1.0);
    if n < 1.0 {
        "fewer than 1".to_owned()
    } else if n >= 97.5 {
        "almost all".to_owned()
    } else if n <= 10.0 {
        format!("{}", n.round().max(1.0) as i64)
    } else {
        format!("{}", ((n / 5.0).round() * 5.0) as i64)
    }
}

/// "about 10 of 100", "fewer than 1 of 100", "almost all".
pub fn households(p: f64) -> String {
    match per_100(p).as_str() {
        "almost all" => "almost all".to_owned(),
        "fewer than 1" => "fewer than 1 of 100".to_owned(),
        n => format!("about {n} of 100"),
    }
}

/// A day count on the ladder, the way the app says it: "half a day", "3 days", "2 weeks",
/// "1½ months", "1 year".
pub fn day_phrase(d: f64) -> String {
    if d <= 0.0 {
        return "0 days".to_owned();
    }
    if (d - 0.5).abs() < 1e-6 {
        return "half a day".to_owned();
    }
    let (n, unit) = unit_of(d);
    format!(
        "{} {unit}{}",
        n_text(n),
        if (n - 1.0).abs() < 1e-9 { "" } else { "s" }
    )
}

fn unit_of(d: f64) -> (f64, &'static str) {
    if (d - 365.0).abs() < 1e-6 {
        (1.0, "year")
    } else if (d - 14.0).abs() < 1e-6 || (d - 21.0).abs() < 1e-6 {
        (d / 7.0, "week")
    } else if (d - 45.0).abs() < 1e-6 {
        (1.5, "month")
    } else if d >= 30.0 && (d % 30.0).abs() < 1e-6 {
        (d / 30.0, "month")
    } else if d > 30.0 && d < 365.0 {
        ((d / 30.0).round(), "month")
    } else if d > 365.0 {
        (((d / 365.0) * 10.0).round() / 10.0, "year")
    } else {
        (d, "day")
    }
}

fn n_text(n: f64) -> String {
    if (n - 0.5).abs() < 1e-9 {
        return "½".to_owned();
    }
    if (n - n.round()).abs() < 1e-9 {
        return format!("{}", n.round() as i64);
    }
    if ((n % 1.0) - 0.5).abs() < 1e-9 {
        return format!("{}½", n.floor() as i64);
    }
    format!("{n:.1}")
}

/// A target with its range: "about 3 days (2–5)", "about 2 weeks (10 days to 1 month)".
pub fn target_days(value: f64, low: f64, high: f64) -> String {
    if value <= 0.0 {
        return "not needed at this setting".to_owned();
    }
    let main = format!("about {}", day_phrase(value));
    if (low - value).abs() < 1e-6 && (high - value).abs() < 1e-6 {
        return main;
    }
    if (low - value).abs() < 1e-6 {
        return format!("{main} (up to {})", day_phrase(high));
    }
    let (ul, uv, uh) = (unit_of(low), unit_of(value), unit_of(high));
    if ul.1 == uv.1 && uv.1 == uh.1 && (value - 0.5).abs() > 1e-6 && (low - 0.5).abs() > 1e-6 {
        return format!("{main} ({}–{})", n_text(ul.0), n_text(uh.0));
    }
    format!("{main} ({} to {})", day_phrase(low), day_phrase(high))
}

/// Months of income: "about 4 months (2–7)"; "no savings goal" at 0.
pub fn target_months(value: f64, low: f64, high: f64) -> String {
    if value <= 0.0 {
        return "no savings goal".to_owned();
    }
    let m = |v: f64| {
        if (v - 0.5).abs() < 1e-9 {
            "half a month".to_owned()
        } else if (v - 1.0).abs() < 1e-9 {
            "1 month".to_owned()
        } else {
            format!("{} months", n_text(v))
        }
    };
    if (low - value).abs() < 1e-6 && (high - value).abs() < 1e-6 {
        return format!("about {}", m(value));
    }
    format!("about {} ({}–{})", m(value), n_text(low), n_text(high))
}

/// Months saved: "half a month", "2 months", "no savings yet".
pub fn months_phrase(v: f64) -> String {
    if v <= 0.0 {
        "no savings yet".to_owned()
    } else if (v - 0.5).abs() < 1e-9 {
        "half a month".to_owned()
    } else if (v - 1.0).abs() < 1e-9 {
        "1 month".to_owned()
    } else {
        format!("{} months", n_text((v * 10.0).round() / 10.0))
    }
}

/// Warning time: "15 minutes to 12 hours", "1 to 3 days".
pub fn notice_range(low_hours: f64, high_hours: f64) -> String {
    let one = |h: f64| -> String {
        if h <= 0.0 {
            "no warning".to_owned()
        } else if h < 1.0 && (h * 60.0).round() < 60.0 {
            // Singular and plural ("1 minute", review W1); 59.5 minutes and up read "1 hour".
            let m = (h * 60.0).round().max(1.0);
            format!("{} minute{}", m as i64, if m == 1.0 { "" } else { "s" })
        } else if h < 48.0 {
            let r = ((h * 10.0).round() / 10.0).max(1.0);
            format!(
                "{} hour{}",
                n_text(r),
                if (r - 1.0).abs() < 1e-9 { "" } else { "s" }
            )
        } else {
            let d = (h / 24.0).round();
            format!("{} day{}", d as i64, if d == 1.0 { "" } else { "s" })
        }
    };
    if (low_hours - high_hours).abs() < 1e-9 {
        one(low_hours)
    } else {
        format!("{} to {}", one(low_hours), one(high_hours))
    }
}

/// Severity as a word (the app pairs it with a pattern).
pub fn severity(s: f64) -> &'static str {
    if s < 0.2 {
        "Minor"
    } else if s < 0.4 {
        "Moderate"
    } else if s < 0.6 {
        "Serious"
    } else if s < 0.8 {
        "Severe"
    } else {
        "Very severe"
    }
}

/// How sure an estimate is, in words.
pub fn confidence(c: DataConfidence) -> &'static str {
    match c {
        DataConfidence::High => "Based on data",
        DataConfidence::Medium => "Mostly data",
        DataConfidence::Low => "Rough data",
        DataConfidence::Prior => "Expert estimate",
    }
}

/// "Very serious (about 1 in 100 years)".
pub fn dial_phrase(rp: ReturnPeriod) -> String {
    format!("{} (about 1 in {} years)", rp.name(), rp.years())
}

/// "1 in 100".
pub fn dial_short(rp: ReturnPeriod) -> String {
    format!("1 in {}", rp.years())
}

/// "2 adults, 1 older adult and 1 child, with 1 dog".
pub fn household(input: &PlanInput) -> String {
    const ORDER: [(AgeBand, &str, &str); 6] = [
        (AgeBand::Adult, "adult", "adults"),
        (AgeBand::Senior, "older adult", "older adults"),
        (AgeBand::Teen, "teenager", "teenagers"),
        (AgeBand::Child, "child", "children"),
        (AgeBand::Toddler, "toddler", "toddlers"),
        (AgeBand::Infant, "baby", "babies"),
    ];
    let parts: Vec<String> = ORDER
        .iter()
        .filter_map(|(band, one, many)| {
            let n = input.people.iter().filter(|p| p.age_band == *band).count();
            (n > 0).then(|| format!("{n} {}", if n == 1 { one } else { many }))
        })
        .collect();
    let people = join_and(&parts);
    let p = &input.pets;
    let pets: Vec<String> = [
        (p.dogs, "dog", "dogs"),
        (p.cats, "cat", "cats"),
        (p.small, "small pet", "small pets"),
        (p.large_animals, "large animal", "large animals"),
    ]
    .iter()
    .filter(|(n, _, _)| *n > 0)
    .map(|(n, one, many)| format!("{n} {}", if *n == 1 { one } else { many }))
    .collect();
    if pets.is_empty() {
        people
    } else {
        format!("{people}, with {}", join_and(&pets))
    }
}

/// "a, b and c".
pub fn join_and(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// Lower-cases the first letter (for names used mid-sentence), unless the second letter is also
/// upper case (an acronym such as "NOAA").
pub fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match (chars.next(), s.chars().nth(1)) {
        (Some(_), Some(second)) if second.is_uppercase() => s.to_owned(),
        (Some(f), _) => f.to_lowercase().chain(chars).collect(),
        (None, _) => String::new(),
    }
}

/// Wraps text at `width` columns, each line after the first indented by `indent` spaces.
pub fn wrap(text: &str, width: usize, indent: usize) -> String {
    let mut out = String::new();
    let mut line_len = 0usize;
    for word in text.split_whitespace() {
        let w = word.chars().count();
        if line_len > 0 && line_len + 1 + w > width {
            out.push('\n');
            out.push_str(&" ".repeat(indent));
            line_len = indent;
        } else if line_len > 0 {
            out.push(' ');
            line_len += 1;
        }
        out.push_str(word);
        line_len += w;
    }
    out
}

/// "Key   value" lines, the keys padded to the longest, each line starting with `indent` spaces.
pub fn fields(rows: &[(&str, String)], indent: usize) -> String {
    let w = rows
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for (k, v) in rows {
        let pad = w - k.chars().count();
        out.push_str(&format!(
            "{}{k}{}  {v}\n",
            " ".repeat(indent),
            " ".repeat(pad)
        ));
    }
    out
}

/// Column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Pad on the right.
    Left,
    /// Pad on the left (numbers).
    Right,
}

#[derive(Debug, Clone)]
enum Row {
    Cells(Vec<String>),
    Note(String),
}

/// A plain-text table: columns padded to their widest cell, two spaces apart, no trailing
/// spaces. Notes are lines under a row (a sentence, a source list), indented past the first
/// column and wrapped at the table's width.
#[derive(Debug, Clone)]
pub struct Table {
    header: Vec<String>,
    align: Vec<Align>,
    rows: Vec<Row>,
    width: usize,
}

/// Where notes under table rows wrap.
pub const NOTE_WIDTH: usize = 100;

impl Table {
    /// A table with these column headings, all left-aligned.
    pub fn new<S: Into<String>>(header: impl IntoIterator<Item = S>) -> Self {
        let header: Vec<String> = header.into_iter().map(Into::into).collect();
        let align = vec![Align::Left; header.len()];
        Self {
            header,
            align,
            rows: Vec::new(),
            width: NOTE_WIDTH,
        }
    }

    /// Right-aligns column `i`.
    pub fn right(mut self, i: usize) -> Self {
        if let Some(a) = self.align.get_mut(i) {
            *a = Align::Right;
        }
        self
    }

    /// Adds a row (missing cells are blank, extra cells are dropped).
    pub fn row<S: Into<String>>(&mut self, cells: impl IntoIterator<Item = S>) {
        let mut cells: Vec<String> = cells.into_iter().map(Into::into).collect();
        cells.resize(self.header.len(), String::new());
        self.rows.push(Row::Cells(cells));
    }

    /// Adds a line under the last row (wrapped when rendered).
    pub fn note(&mut self, text: impl Into<String>) {
        self.rows.push(Row::Note(text.into()));
    }

    /// True when no rows were added.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The table as text, every line starting with `indent` spaces.
    pub fn render(&self, indent: usize) -> String {
        let n = self.header.len();
        let mut widths: Vec<usize> = self.header.iter().map(|h| h.chars().count()).collect();
        for r in &self.rows {
            if let Row::Cells(c) = r {
                for (i, cell) in c.iter().enumerate() {
                    widths[i] = widths[i].max(cell.chars().count());
                }
            }
        }
        let pad = " ".repeat(indent);
        let note_pad = " ".repeat(indent + widths.first().copied().unwrap_or(0).min(4) + 2);
        let line = |cells: &[String]| -> String {
            let mut s = pad.clone();
            for (i, cell) in cells.iter().enumerate() {
                let w = cell.chars().count();
                let fill = widths[i].saturating_sub(w);
                let last = i + 1 == n;
                match self.align[i] {
                    Align::Right => {
                        s.push_str(&" ".repeat(fill));
                        s.push_str(cell);
                    }
                    Align::Left => {
                        s.push_str(cell);
                        if !last {
                            s.push_str(&" ".repeat(fill));
                        }
                    }
                }
                if !last {
                    s.push_str("  ");
                }
            }
            s.trim_end().to_owned()
        };
        let mut out = String::new();
        out.push_str(&line(&self.header));
        out.push('\n');
        for r in &self.rows {
            match r {
                Row::Cells(c) => out.push_str(&line(c)),
                Row::Note(t) => {
                    let room = self.width.saturating_sub(note_pad.len()).max(20);
                    let wrapped = wrap(t, room, 0);
                    for (i, l) in wrapped.lines().enumerate() {
                        if i > 0 {
                            out.push('\n');
                        }
                        out.push_str(&note_pad);
                        out.push_str(l);
                    }
                }
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_match_the_packet() {
        assert_eq!(day_phrase(0.5), "half a day");
        assert_eq!(day_phrase(14.0), "2 weeks");
        assert_eq!(day_phrase(45.0), "1½ months");
        assert_eq!(day_phrase(365.0), "1 year");
        assert_eq!(target_days(3.0, 2.0, 5.0), "about 3 days (2–5)");
        assert_eq!(
            target_days(14.0, 10.0, 30.0),
            "about 2 weeks (10 days to 1 month)"
        );
        assert_eq!(target_days(0.0, 0.0, 0.0), "not needed at this setting");
        assert_eq!(target_months(4.0, 2.0, 7.0), "about 4 months (2–7)");
        assert_eq!(band(29.0, 45.0), "$29–45");
        assert_eq!(band(0.0, 1.5), "$0.00–1.50");
        assert_eq!(band(0.0, 0.0), "free");
        assert_eq!(usd(1234.4), "$1,234");
        assert_eq!(per_100(0.254), "25");
        assert_eq!(households(0.004), "fewer than 1 of 100");
        assert_eq!(households(0.99), "almost all");
        assert_eq!(notice_range(0.25, 12.0), "15 minutes to 12 hours");
        assert_eq!(notice_range(0.02, 72.0), "1 minute to 3 days");
        assert_eq!(notice_range(0.995, 24.0), "1 hour to 24 hours");
        assert_eq!(lower_first("NOAA radio"), "NOAA radio");
        assert_eq!(
            dial_phrase(ReturnPeriod::OneIn100),
            "Very serious (about 1 in 100 years)"
        );
    }

    #[test]
    fn two_significant_figures() {
        assert_eq!(sig2(0.0834), "0.083");
        assert_eq!(sig2(0.000_123), "0.00012");
        assert_eq!(sig2(1.43), "1.4");
        assert_eq!(sig2(12.3), "12");
        assert_eq!(sig2(0.0), "0");
        assert_eq!(sig2(4.5), "4.5");
        assert_eq!(sig2(f64::from(0.01_f32)), "0.010");
        assert_eq!(sig2(9.96), "10");
    }

    #[test]
    fn chance_matches_the_design_formula() {
        // DESIGN §4.4: the one-in-100 dial, Λ* = −ln(0.9)/10, is "10 of 100 in ten years".
        let rate = -rr_types::math::ln(0.9) / 10.0;
        assert!((chance(rate, 10.0) - 0.1).abs() < 1e-12);
        assert_eq!(chance(0.0, 10.0), 0.0);
    }

    #[test]
    fn tables_align_without_trailing_spaces() {
        let mut t = Table::new(["Name", "Days"]).right(1);
        t.row(["Power", "5"]);
        t.row(["Water", "14"]);
        t.note("a note");
        let s = t.render(1);
        assert_eq!(
            s,
            " Name   Days\n Power     5\n Water    14\n       a note\n"
        );
        let mut t = Table::new(["A", "B"]);
        t.row(["x", "y"]);
        t.note("one two three four five six seven eight nine ten eleven twelve thirteen");
        let wrapped = t.render(0);
        assert!(
            wrapped
                .lines()
                .skip(2)
                .all(|l| l.starts_with("   ") && l.len() <= NOTE_WIDTH)
        );
        assert!(s.lines().all(|l| !l.ends_with(' ')));
    }

    #[test]
    fn wrapping_indents_continuation_lines() {
        assert_eq!(wrap("one two three four", 9, 2), "one two\n  three\n  four");
    }
}
