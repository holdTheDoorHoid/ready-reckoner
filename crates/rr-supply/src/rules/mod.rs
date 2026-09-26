//! The quantity rules, grouped by family (docs/QUANTITY_RULES.md lists every rule id).
//!
//! Each rule is a pure function of the household facts and a number of days. It reads numbers
//! only from the constants registry and returns a [`Sizing`]: quantity, unit, what it scales with,
//! the citations of the numbers it used, and a plain-language sentence that shows the arithmetic.
//! [`crate::sized_requirements`] turns sizings into requirement lines for each bucket.

pub mod comms;
pub mod evacuate;
pub mod fire;
pub mod first_aid;
pub mod food;
pub mod get_home;
pub mod medication;
pub mod money;
pub mod power;
pub mod sanitation;
pub mod thermal;
pub mod water;

use rr_types::{CitationId, Per};

use crate::basis::Basis;
use crate::format::{ceil_count, round_dp, round_to};

/// What a quantity rule produced: the numbers behind one requirement line.
#[derive(Debug, Clone, PartialEq)]
pub struct Sizing {
    /// The rule id (docs/QUANTITY_RULES.md).
    pub rule: &'static str,
    /// The class of catalogue items that can meet it.
    pub item_class: String,
    /// The whole household's amount, in `unit`, rounded for display.
    pub quantity: f64,
    /// The unit of `quantity`.
    pub unit: &'static str,
    /// What the quantity scales with.
    pub per: Per,
    /// Sources of every number used, in the order first used.
    pub citations: Vec<CitationId>,
    /// The line in plain language, with its arithmetic and a short attribution.
    pub plain: String,
    /// The formula with the numbers filled in, for the expert view.
    pub math: Vec<String>,
    /// Days of need the quantity covers, for duration lines.
    pub days: Option<f64>,
    /// The household's amount per day in `unit` (unrounded), for duration lines, so the plan
    /// can turn an owned quantity into days of coverage.
    pub per_day: Option<f64>,
    /// At least one number behind it is a planning estimate.
    pub prior: bool,
    /// The plain text without its attribution, so sentences can be appended.
    body: String,
}

impl Sizing {
    /// Builds a sizing from the basis a rule read its numbers through. The quantity is rounded
    /// for its unit ([`round_quantity`]) and the attribution is appended to `text`.
    pub(crate) fn new(
        b: &Basis,
        rule: &'static str,
        item_class: impl Into<String>,
        quantity: f64,
        unit: &'static str,
        per: Per,
        text: String,
    ) -> Sizing {
        Sizing {
            rule,
            item_class: item_class.into(),
            quantity: round_quantity(unit, quantity),
            unit,
            per,
            citations: b.cites().to_vec(),
            plain: with_attribution(&text, b),
            math: Vec::new(),
            days: None,
            per_day: None,
            prior: b.is_prior(),
            body: text,
        }
    }

    /// Records the days covered and the amount per day.
    pub(crate) fn per_day(mut self, days: f64, per_day: f64) -> Sizing {
        self.days = Some(days);
        self.per_day = Some(per_day);
        self
    }

    /// Records the days covered (for lines that are not a daily rate, such as cold storage).
    pub(crate) fn days(mut self, days: f64) -> Sizing {
        self.days = Some(days);
        self
    }

    /// Adds formula lines for the expert view.
    pub(crate) fn math(mut self, lines: Vec<String>) -> Sizing {
        self.math = lines;
        self
    }

    /// Appends a sentence (and the citations behind it) to the plain text, before the
    /// attribution is recomputed.
    pub(crate) fn extend(mut self, sentence: &str, extra: &Basis) -> Sizing {
        let mut merged = Basis::new();
        merged.cite_all(&self.citations);
        merged.cite_all(extra.cites());
        self.body = format!("{} {sentence}", self.body);
        self.plain = with_attribution(&self.body, &merged);
        self.citations = merged.cites().to_vec();
        self.prior = merged.is_prior();
        self
    }

    /// Adds citations that stand behind the line without changing its text (a bucket target's
    /// own sources: the hazard and duration data behind the days).
    pub(crate) fn also_cite(mut self, ids: &[CitationId]) -> Sizing {
        for id in ids {
            if !self.citations.contains(id) {
                self.citations.push(id.clone());
            }
        }
        self
    }
}

/// `text` followed by the basis's short attribution.
fn with_attribution(text: &str, b: &Basis) -> String {
    let attribution = b.attribution();
    if attribution.is_empty() {
        text.to_owned()
    } else {
        format!("{text} {attribution}")
    }
}

/// Rounds a quantity for its unit: whole numbers for things you buy one at a time, 100 kcal,
/// 10 Wh, watts or grams, whole dollars and ounces, and a tenth for gallons, litres, pounds,
/// days and months.
pub fn round_quantity(unit: &str, x: f64) -> f64 {
    if !x.is_finite() || x <= 0.0 {
        return 0.0;
    }
    match unit {
        "kcal" => round_to(x, 100.0),
        "Wh" | "watt" | "gram" => round_to(x, 10.0),
        "usd" | "oz" => round_dp(x, 0),
        "gallon" | "litre" | "lb" | "day" | "person_day" | "pet_day" | "month" => round_dp(x, 1),
        _ => ceil_count(x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::keys;

    #[test]
    fn rounding_by_unit() {
        assert_eq!(round_quantity("kcal", 81_991.6), 82_000.0);
        assert_eq!(round_quantity("gallon", 12.9375), 12.9);
        assert_eq!(round_quantity("bag", 5.4), 6.0);
        assert_eq!(round_quantity("Wh", 134.9), 130.0);
        assert_eq!(round_quantity("usd", 337.6), 338.0);
        assert_eq!(round_quantity("course", 0.0), 0.0);
        assert_eq!(round_quantity("gallon", f64::NAN), 0.0);
    }

    #[test]
    fn extend_keeps_one_attribution_at_the_end() {
        let mut b = Basis::new();
        let q = b.k(keys::WATER_BASIC_GAL);
        let s = Sizing::new(
            &b,
            "water_gallons",
            "stored_water",
            q,
            "gallon",
            Per::Person,
            "One gallon.".into(),
        );
        assert_eq!(s.plain, "One gallon. (Ready.gov, CDC)");
        let mut extra = Basis::new();
        extra.k(keys::WATER_STORED_CAP_DAYS);
        let s = s.extend("Store two weeks.", &extra);
        assert_eq!(
            s.plain,
            "One gallon. Store two weeks. (Ready.gov, CDC, BYU, Church of Jesus Christ)"
        );
        assert!(s.citations.iter().any(|c| c == "oregon_b2wr_toolkit"));
    }
}
