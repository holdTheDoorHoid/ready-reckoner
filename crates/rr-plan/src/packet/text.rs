//! How numbers and names read in the packet. The rules follow `web/src/lib/format.ts` (the app's
//! screens) and `docs/CONTENT_STANDARDS.md` §6: natural frequencies out of 100 (whole numbers to
//! 10, the nearest 5 above), days on the ladder in the unit a person would use, whole dollars with
//! bands as "$30–45", dates as "October 1, 2026".

use rr_types::{AgeBand, Date, PlanInput};

/// Escapes text for inline Markdown and table cells, so a name can never start emphasis, a link,
/// a table cell break or anything that looks like HTML.
pub fn md(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(
            c,
            '\\' | '*' | '_' | '`' | '[' | ']' | '|' | '<' | '>' | '#'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Month names for dates.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// "October 1, 2026".
pub fn date(d: Date) -> String {
    format!(
        "{} {}, {}",
        MONTHS[usize::from(d.month().clamp(1, 12)) - 1],
        d.day(),
        d.year()
    )
}

/// "October 2026".
pub fn month_year(d: Date) -> String {
    format!(
        "{} {}",
        MONTHS[usize::from(d.month().clamp(1, 12)) - 1],
        d.year()
    )
}

/// The first day of plan month `index` (month 0 begins on the planning date).
pub fn month_start(planning: Date, index: u16) -> Date {
    planning.add_months(i32::from(index)).unwrap_or(planning)
}

/// Whole dollars with thousands separators: "$1,234"; under half a dollar shows as "under $1".
pub fn usd(x: f64) -> String {
    if x > 0.0 && x < 0.5 {
        return "under $1".to_owned();
    }
    format!("${}", thousands(x.round().max(0.0) as u64))
}

fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A price band: "$30–45"; "$30" when both ends match; "free" at $0.
pub fn band(low: f64, high: f64) -> String {
    let (l, h) = (low.round().max(0.0), high.round().max(0.0));
    if l == 0.0 && h == 0.0 {
        return "free".to_owned();
    }
    if l == h {
        return usd(l);
    }
    format!("{}–{}", usd(l), thousands(h as u64))
}

/// Irregular plurals of catalogue and requirement units.
fn plural(unit: &str, qty: f64) -> String {
    if (qty - 1.0).abs() < 1e-9 {
        return unit.to_owned();
    }
    match unit {
        "each" | "Wh" | "kcal" | "oz" | "lb" => return unit.to_owned(),
        "box" => return "boxes".to_owned(),
        "pouch" => return "pouches".to_owned(),
        "person_day" | "person-day" => return "person-days".to_owned(),
        "pet_day" | "pet-day" => return "pet-days".to_owned(),
        "person_month" | "person-month" => return "person-months".to_owned(),
        "day of one person's medicine" => return "days of one person's medicine".to_owned(),
        "pound of dry food" => return "pounds of dry food".to_owned(),
        "ounce of powder" => return "ounces of powder".to_owned(),
        "cycle's supply" => return "cycles' supplies".to_owned(),
        "8-ounce canister" => return "8-ounce canisters".to_owned(),
        "12-ounce bottle" => return "12-ounce bottles".to_owned(),
        "24-pack" => return "24-packs".to_owned(),
        _ => {}
    }
    let bytes = unit.as_bytes();
    let n = bytes.len();
    if n >= 2 && bytes[n - 1] == b'y' && !b"aeiou".contains(&bytes[n - 2]) {
        return format!("{}ies", &unit[..n - 1]);
    }
    if unit.ends_with('s') || unit.ends_with('x') || unit.ends_with("ch") || unit.ends_with("sh") {
        return format!("{unit}es");
    }
    format!("{unit}s")
}

/// A number for a quantity: whole numbers with separators, else one decimal.
pub fn number(q: f64) -> String {
    let r = (q * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-9 {
        thousands(r.round().max(0.0) as u64)
    } else {
        format!("{r:.1}")
    }
}

/// "12 gallons", "1 kit", "$100" for dollars, "26,000 kcal" for food counted in 2,000 kcal
/// units, "4 (one per person)" for items counted per person.
pub fn quantity(q: f64, unit: &str) -> String {
    if let Some(per) = unit
        .strip_suffix(" kcal")
        .and_then(|n| n.replace(',', "").parse::<f64>().ok())
    {
        return format!("{} kcal", number(q * per));
    }
    match unit {
        "dollar" | "usd" => usd(q),
        "action" | "plan" | "decision" => String::new(),
        "person" | "pet" => format!("{} (one per {unit})", number(q)),
        // A unit that is itself an amount ("3 days of food for one person") is counted, not
        // pluralised: "4 × 3 days of food for one person".
        u if u.starts_with(|c: char| c.is_ascii_digit()) => format!("{} × {u}", number(q)),
        _ => format!("{} {}", number(q), plural(unit, q)),
    }
}

/// A day count for the target ladder, the way the app says it: "half a day", "1 day", "3 days",
/// "2 weeks", "3 weeks", "1 month", "6 weeks", "3 months", "1 year".
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
    format!("{:.1}", n)
}

/// A target with its range: "about 3 days (2–5)", "about 2 weeks (10 days to 3 weeks)".
pub fn target_days(value: f64, low: f64, high: f64) -> String {
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

/// Months of income: "about 4 months (2–7)".
pub fn target_months(value: f64, low: f64, high: f64) -> String {
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
        } else if h < 1.0 {
            let m = (h * 60.0).round().max(1.0);
            format!("{} minute{}", m as i64, if m == 1.0 { "" } else { "s" })
        } else if h < 48.0 {
            let r = (h * 10.0).round() / 10.0;
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

/// Of 100 households, rounded as the app does: under 1 → "fewer than 1"; 1–10 whole; above 10 the
/// nearest 5; 97.5 and above "almost all".
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

/// "about 10 of 100" for table cells ("almost all", "fewer than 1 of 100").
pub fn households(p: f64) -> String {
    match per_100(p).as_str() {
        "almost all" => "almost all".to_owned(),
        "fewer than 1" => "fewer than 1 of 100".to_owned(),
        n => format!("about {n} of 100"),
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

/// How sure the estimate is, in words.
pub fn confidence(c: rr_types::DataConfidence) -> &'static str {
    match c {
        rr_types::DataConfidence::High => "Based on data",
        rr_types::DataConfidence::Medium => "Mostly data",
        rr_types::DataConfidence::Low => "Rough data",
        rr_types::DataConfidence::Prior => "Expert estimate",
    }
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
    let mut pets: Vec<String> = Vec::new();
    for (n, one, many) in [
        (p.dogs, "dog", "dogs"),
        (p.cats, "cat", "cats"),
        (p.small, "small pet", "small pets"),
        (p.large_animals, "large animal", "large animals"),
    ] {
        if n > 0 {
            pets.push(format!("{n} {}", if n == 1 { one } else { many }));
        }
    }
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

/// Capitalises the first letter.
pub fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Lower-cases the first letter (for names used mid-sentence), unless the second letter is also
/// upper case (an acronym such as "NOAA").
pub fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match (chars.next(), s.chars().nth(1)) {
        (Some(f), Some(second)) if second.is_uppercase() => {
            let _ = f;
            s.to_owned()
        }
        (Some(f), _) => f.to_lowercase().chain(chars).collect(),
        (None, _) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_match_the_app() {
        assert_eq!(day_phrase(0.5), "half a day");
        assert_eq!(day_phrase(1.0), "1 day");
        assert_eq!(day_phrase(14.0), "2 weeks");
        assert_eq!(day_phrase(45.0), "1½ months");
        assert_eq!(day_phrase(90.0), "3 months");
        assert_eq!(day_phrase(365.0), "1 year");
        assert_eq!(target_days(3.0, 2.0, 5.0), "about 3 days (2–5)");
        assert_eq!(
            target_days(14.0, 10.0, 30.0),
            "about 2 weeks (10 days to 1 month)"
        );
        assert_eq!(target_days(5.0, 5.0, 10.0), "about 5 days (up to 10 days)");
        assert_eq!(band(29.0, 45.0), "$29–45");
        assert_eq!(band(0.0, 0.0), "free");
        assert_eq!(usd(1234.4), "$1,234");
        assert_eq!(quantity(12.9, "gallon"), "12.9 gallons");
        assert_eq!(quantity(1.0, "kit"), "1 kit");
        assert_eq!(quantity(4.0, "battery"), "4 batteries");
        assert_eq!(quantity(100.0, "dollar"), "$100");
        assert_eq!(quantity(13.0, "2,000 kcal"), "26,000 kcal");
        assert_eq!(quantity(4.0, "person"), "4 (one per person)");
        assert_eq!(per_100(0.254), "25");
        assert_eq!(per_100(0.004), "fewer than 1");
        assert_eq!(per_100(0.99), "almost all");
        assert_eq!(notice_range(0.25, 12.0), "15 minutes to 12 hours");
        assert_eq!(md("a|b*c"), "a\\|b\\*c");
        assert_eq!(
            date(Date::from_ymd(2026, 10, 1).unwrap()),
            "October 1, 2026"
        );
        assert_eq!(lower_first("NOAA radio"), "NOAA radio");
        assert_eq!(lower_first("Stored water"), "stored water");
    }
}
