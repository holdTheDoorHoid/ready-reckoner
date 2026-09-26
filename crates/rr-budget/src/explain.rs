//! The "why" sentence on every plan item: which bucket it covers, how far it moves the household
//! toward the goal, how often households like this one need it, and what causes that here
//! (DESIGN §4.7, UI copy rules). Numbers follow research risk-model §7.1: natural frequencies
//! out of 100, "fewer than 1 in 100" below one, whole numbers up to ten and the nearest five above
//! that; days have no decimals above ten.

use rr_types::{BucketId, HazardId};

/// One bucket (or part) a purchase moves, with the numbers for its sentence.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DurationText {
    pub bucket: BucketId,
    pub part: Option<String>,
    pub from_days: f64,
    pub to_days: f64,
    pub target_days: f64,
    /// The "d" in "for d or more".
    pub ref_days: f64,
    /// Households per 100 facing a disruption longer than `ref_days` over `years`.
    pub per_100: f64,
}

/// One readiness bucket an item serves.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReadinessText {
    pub bucket: BucketId,
    /// Households per 100 needing it over `years`.
    pub per_100: f64,
}

/// What the sentence is about.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct WhyParts {
    /// Duration effects, largest value first.
    pub durations: Vec<DurationText>,
    /// Readiness effects, largest value first.
    pub readiness: Vec<ReadinessText>,
    /// Other buckets the item helps with (for "Also helps with").
    pub also: Vec<BucketId>,
    /// Main causes of the headline bucket, largest share first.
    pub causes: Vec<HazardId>,
}

/// Opening words for the sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lead {
    Purchase,
    Free,
    AlreadyDone,
    AlreadyOwned,
    RareAllowance,
}

pub(crate) fn why(lead: Lead, parts: &WhyParts, people: usize, years: u8) -> String {
    let mut out: Vec<String> = Vec::new();
    match lead {
        Lead::Purchase => {}
        Lead::Free => out.push("Free.".into()),
        Lead::AlreadyDone => out.push("Already done.".into()),
        Lead::AlreadyOwned => out.push("You already have this.".into()),
        Lead::RareAllowance => out.push(
            "Paid from your rare-emergency allowance (at most a tenth of each month's money)."
                .into(),
        ),
    }
    let mut mentioned: Vec<BucketId> = Vec::new();
    if let Some(d) = parts.durations.first() {
        out.push(duration_sentence(d, people));
        out.push(frequency_sentence(
            d.per_100,
            &event_phrase(d.bucket, d.part.as_deref()),
            Some(d.ref_days),
            years,
        ));
        mentioned.push(d.bucket);
    } else if let Some(r) = parts.readiness.first() {
        out.push(format!(
            "Gets you ready for {}.",
            readiness_phrase(r.bucket)
        ));
        out.push(frequency_sentence(r.per_100, "need this", None, years));
        mentioned.push(r.bucket);
    }
    if !mentioned.is_empty() {
        if let Some(c) = causes_sentence(&parts.causes) {
            out.push(c);
        }
    }
    let mut also: Vec<BucketId> = Vec::new();
    for b in parts
        .durations
        .iter()
        .map(|d| d.bucket)
        .chain(parts.readiness.iter().map(|r| r.bucket))
        .chain(parts.also.iter().copied())
    {
        if !mentioned.contains(&b) && !also.contains(&b) {
            also.push(b);
        }
    }
    if !also.is_empty() {
        let names: Vec<&str> = also.iter().map(|b| short_name(*b)).collect();
        let verb = if mentioned.is_empty() {
            "Helps with"
        } else {
            "Also helps with"
        };
        out.push(format!("{verb} {}.", join_and(&names)));
    }
    out.join(" ")
}

fn duration_sentence(d: &DurationText, people: usize) -> String {
    let added = (d.to_days - d.from_days).max(0.0);
    let (supply, per_person) = supply(d.bucket, d.part.as_deref());
    // Water, food and the like scale with the number of people; other cover is for the household
    // (or for whoever a part names: the baby, the pets, the animals).
    let who = match (per_person, people) {
        (false, _) => String::new(),
        (true, 1) => " for 1 person".to_owned(),
        (true, n) => format!(" for {n} people"),
    };
    let goal = goal_text(d.target_days);
    let progress = if d.to_days + 1e-9 >= d.target_days {
        format!("which completes the {goal}")
    } else {
        format!("bringing you to {} of the {goal}", days_number(d.to_days))
    };
    format!("Adds {} of {supply}{who}, {progress}.", days_text(added))
}

/// "About 26 of 100 households like yours lose power for a day or more in the next 10 years."
pub(crate) fn frequency_sentence(
    per_100: f64,
    event: &str,
    ref_days: Option<f64>,
    years: u8,
) -> String {
    let lasting = match ref_days {
        Some(d) if (d - 1.0).abs() < 0.05 => " for a day or more".to_owned(),
        Some(d) => format!(" for {} or more", days_text(d)),
        None => String::new(),
    };
    let when = if years == 1 {
        "in the next year".to_owned()
    } else {
        format!("in the next {years} years")
    };
    let subject = households_phrase(per_100);
    format!("{subject} {event}{lasting} {when}.")
}

/// Research risk-model §7.1 rounding.
pub(crate) fn households_phrase(per_100: f64) -> String {
    if per_100 < 1.0 {
        return "Fewer than 1 in 100 households like yours".into();
    }
    let shown = if per_100 <= 10.0 {
        per_100.round()
    } else {
        (per_100 / 5.0).round() * 5.0
    };
    if shown >= 100.0 {
        "Nearly all households like yours".into()
    } else {
        format!("About {} of 100 households like yours", shown as u32)
    }
}

fn causes_sentence(causes: &[HazardId]) -> Option<String> {
    let names: Vec<String> = causes
        .iter()
        .take(2)
        .map(|h| lower_first(h.name()))
        .collect();
    match names.len() {
        0 => None,
        1 => Some(format!("Main cause here: {}.", names[0])),
        _ => Some(format!("Main causes here: {} and {}.", names[0], names[1])),
    }
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn join_and(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// A number of days for a sentence: "half a day", "1 day", "2.5 days", "14 days".
pub(crate) fn days_text(d: f64) -> String {
    if (d - 0.5).abs() < 0.05 {
        return "half a day".into();
    }
    let n = days_number(d);
    if n == "1" {
        "1 day".into()
    } else {
        format!("{n} days")
    }
}

/// A day count without the unit: one decimal below ten (dropping ".0"), whole days above.
pub(crate) fn days_number(d: f64) -> String {
    if d >= 10.0 {
        return format!("{}", d.round() as i64);
    }
    let r = (d * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r:.1}")
    }
}

fn goal_text(target: f64) -> String {
    if (target - 0.5).abs() < 0.05 {
        "half-day goal".into()
    } else {
        format!("{}-day goal", days_number(target))
    }
}

/// Whole dollars with thousands separators: "$30", "$16,800"; amounts under a dollar show as
/// "under $1".
pub(crate) fn dollars(x: f64) -> String {
    if x > 0.0 && x < 0.5 {
        return "under $1".into();
    }
    let digits = (x.round() as i64).abs().to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if x.round() < 0.0 { "-" } else { "" };
    format!("{sign}${grouped}")
}

/// What a purchase adds days of, in words, and whether that supply is counted per person.
///
/// A part of a bucket (as `rr-plan` names them: "toilet", "pet food", "lights" ...) gets words of
/// its own rather than the bucket's supply with the part in brackets, which read as "drinking and
/// washing water (toilet)". A part without words of its own is named as it is.
fn supply(bucket: BucketId, part: Option<&str>) -> (String, bool) {
    if let Some(p) = part {
        let (words, per_person) = match p {
            // Power.
            "lights" => ("light during power cuts", false),
            "batteries" => ("spare batteries for lights and a radio", false),
            "medical device power" => ("backup power for a medical device", false),
            "generator fuel" => ("generator fuel", false),
            "wheelchair battery" => ("power for a wheelchair", false),
            // Water.
            "stored water" => ("stored drinking and washing water", true),
            "bleach" => ("bleach for treating water", true),
            "water treatment" => ("safe water during boil notices", true),
            "toilet" => ("emergency toilet supplies", true),
            "water for animals" => ("water for your animals", false),
            // Food and supplies.
            "food" => ("food", true),
            "toilet paper" => ("toilet paper", true),
            "soap" => ("soap", true),
            "pet food" => ("pet food", false),
            "baby formula" => ("baby formula", false),
            "nursing supplies" => ("nursing supplies", false),
            "diapers and wipes" => ("diapers and wipes", false),
            "period products" => ("period products", false),
            // Heat and cold.
            "heat" => ("protection from dangerous heat", false),
            "cold" => ("protection from dangerous cold", false),
            // Medicine.
            "prescriptions" => ("daily medicine", false),
            "cold storage" => ("cold storage for medicine", false),
            "epinephrine" => ("epinephrine on hand", false),
            // Phones and payments.
            "phone" => ("phone power and news", false),
            "payments" => ("cash for when card payments are down", false),
            "two-way radios" => ("two-way radio contact", false),
            other => (other, false),
        };
        return (words.to_owned(), per_person);
    }
    let words = match bucket {
        BucketId::Power => "cover for power cuts",
        BucketId::WaterOut => "drinking and washing water",
        BucketId::WaterBoil => "safe water during boil notices",
        BucketId::Supplies => "food and supplies",
        BucketId::Thermal => "protection from dangerous heat or cold",
        BucketId::Medication => "medicine",
        BucketId::Comms => "backup for phones, news and payments",
        _ => short_name(bucket),
    };
    let per_person = matches!(
        bucket,
        BucketId::WaterOut | BucketId::WaterBoil | BucketId::Supplies
    );
    (words.to_owned(), per_person)
}

fn event_phrase(bucket: BucketId, part: Option<&str>) -> String {
    match bucket {
        BucketId::Power => "lose power",
        BucketId::WaterOut => "lose tap water",
        BucketId::WaterBoil => "are told to boil their tap water",
        BucketId::Supplies => "can't get to a store",
        BucketId::Thermal => match part {
            Some("heat") => "face dangerous heat at home",
            Some("cold") => "face dangerous cold at home",
            _ => "face dangerous heat or cold at home",
        },
        BucketId::Medication => "can't get medicine refilled",
        BucketId::Comms => match part {
            Some("phone") => "lose phone and internet",
            Some("payments") => "can't pay by card",
            _ => "lose phone, internet or card payments",
        },
        _ => "are affected",
    }
    .to_owned()
}

fn readiness_phrase(bucket: BucketId) -> &'static str {
    match bucket {
        BucketId::Evacuate => "leaving home in a hurry",
        BucketId::GetHome => "getting home if you are stranded",
        BucketId::MedicalEmergency => "a medical emergency before help arrives",
        BucketId::Fire => "a fire at home",
        BucketId::Security => "keeping your home and family safe",
        _ => short_name(bucket),
    }
}

/// A few words naming a bucket inside a sentence.
pub(crate) fn short_name(bucket: BucketId) -> &'static str {
    match bucket {
        BucketId::Power => "power cuts",
        BucketId::WaterBoil => "boil-water notices",
        BucketId::WaterOut => "losing tap water",
        BucketId::Supplies => "getting food and supplies",
        BucketId::Thermal => "dangerous heat or cold",
        BucketId::Medication => "medicine",
        BucketId::Comms => "phone and internet outages",
        BucketId::Evacuate => "leaving home quickly",
        BucketId::GetHome => "getting home",
        BucketId::MedicalEmergency => "medical emergencies",
        BucketId::Fire => "house fires",
        BucketId::Security => "home security",
        BucketId::Income => "losing income",
        BucketId::HomeLoss => "a damaged home",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_frequency_rounding_follows_research_7_1() {
        assert_eq!(
            households_phrase(0.4),
            "Fewer than 1 in 100 households like yours"
        );
        assert_eq!(
            households_phrase(1.2),
            "About 1 of 100 households like yours"
        );
        assert_eq!(
            households_phrase(9.6),
            "About 10 of 100 households like yours"
        );
        assert_eq!(
            households_phrase(12.4),
            "About 10 of 100 households like yours"
        );
        assert_eq!(
            households_phrase(13.0),
            "About 15 of 100 households like yours"
        );
        assert_eq!(
            households_phrase(25.6),
            "About 25 of 100 households like yours"
        );
        assert_eq!(households_phrase(99.0), "Nearly all households like yours");
    }

    #[test]
    fn days_and_money_are_plain() {
        assert_eq!(days_text(0.5), "half a day");
        assert_eq!(days_text(1.0), "1 day");
        assert_eq!(days_text(2.5), "2.5 days");
        assert_eq!(days_text(2.96), "3 days");
        assert_eq!(days_text(13.6), "14 days");
        assert_eq!(dollars(34.6), "$35");
        assert_eq!(dollars(0.2), "under $1");
        assert_eq!(dollars(16_800.0), "$16,800");
        assert_eq!(dollars(1_234_567.4), "$1,234,567");
        assert_eq!(join_and(&["a", "b", "c"]), "a, b and c");
    }

    #[test]
    fn a_full_sentence_reads_plainly() {
        let parts = WhyParts {
            durations: vec![DurationText {
                bucket: BucketId::WaterOut,
                part: None,
                from_days: 0.5,
                to_days: 3.0,
                target_days: 3.0,
                ref_days: 1.0,
                per_100: 24.2,
            }],
            readiness: vec![],
            also: vec![BucketId::WaterBoil],
            causes: vec![HazardId::LocalUtilityOutage, HazardId::HazmatRelease],
        };
        assert_eq!(
            why(Lead::Purchase, &parts, 4, 10),
            "Adds 2.5 days of drinking and washing water for 4 people, which completes the \
             3-day goal. About 25 of 100 households like yours lose tap water for a day or more \
             in the next 10 years. Main causes here: local water or gas outage and chemical \
             spill or release. Also helps with boil-water notices."
        );
        let ready = WhyParts {
            readiness: vec![ReadinessText {
                bucket: BucketId::Evacuate,
                per_100: 5.1,
            }],
            ..WhyParts::default()
        };
        assert_eq!(
            why(Lead::Free, &ready, 4, 10),
            "Free. Gets you ready for leaving home in a hurry. About 5 of 100 households like \
             yours need this in the next 10 years."
        );
    }

    fn adds(bucket: BucketId, part: Option<&str>, people: usize) -> String {
        duration_sentence(
            &DurationText {
                bucket,
                part: part.map(str::to_owned),
                from_days: 0.0,
                to_days: 1.7,
                target_days: 5.0,
                ref_days: 1.0,
                per_100: 15.0,
            },
            people,
        )
    }

    /// A part of a bucket reads as its own supply, never as the bucket's supply with the part in
    /// brackets, and only supplies counted per person say how many people they are for.
    #[test]
    fn parts_read_as_their_own_supply() {
        assert_eq!(
            adds(BucketId::WaterOut, Some("toilet"), 1),
            "Adds 1.7 days of emergency toilet supplies for 1 person, bringing you to 1.7 of the \
             5-day goal."
        );
        assert_eq!(
            adds(BucketId::WaterOut, Some("stored water"), 4),
            "Adds 1.7 days of stored drinking and washing water for 4 people, bringing you to 1.7 \
             of the 5-day goal."
        );
        assert_eq!(
            adds(BucketId::Supplies, Some("pet food"), 2),
            "Adds 1.7 days of pet food, bringing you to 1.7 of the 5-day goal."
        );
        assert_eq!(
            adds(BucketId::Power, Some("medical device power"), 1),
            "Adds 1.7 days of backup power for a medical device, bringing you to 1.7 of the 5-day \
             goal."
        );
        assert_eq!(
            adds(BucketId::Power, Some("lights"), 3),
            "Adds 1.7 days of light during power cuts, bringing you to 1.7 of the 5-day goal."
        );
        // A part with no words of its own is named as it is.
        assert_eq!(
            adds(BucketId::Supplies, Some("cooling towels"), 2),
            "Adds 1.7 days of cooling towels, bringing you to 1.7 of the 5-day goal."
        );
        // Without a part, water and food are per person and other cover is for the household.
        assert_eq!(
            adds(BucketId::Supplies, None, 2),
            "Adds 1.7 days of food and supplies for 2 people, bringing you to 1.7 of the 5-day \
             goal."
        );
        assert_eq!(
            adds(BucketId::Power, None, 2),
            "Adds 1.7 days of cover for power cuts, bringing you to 1.7 of the 5-day goal."
        );
        for part in [
            "lights",
            "batteries",
            "medical device power",
            "generator fuel",
            "wheelchair battery",
            "stored water",
            "bleach",
            "water treatment",
            "toilet",
            "water for animals",
            "food",
            "toilet paper",
            "soap",
            "pet food",
            "baby formula",
            "nursing supplies",
            "diapers and wipes",
            "period products",
            "heat",
            "cold",
            "prescriptions",
            "cold storage",
            "epinephrine",
            "phone",
            "payments",
            "two-way radios",
        ] {
            let s = adds(BucketId::Supplies, Some(part), 3);
            assert!(!s.contains('(') && !s.contains("  "), "{part}: {s}");
        }
    }
}
