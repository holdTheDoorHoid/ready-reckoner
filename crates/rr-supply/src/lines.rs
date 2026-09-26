//! Requirement lines with the facts the plan and budget crates need beyond the JSON contract.

use rr_types::{BucketId, RequirementLine, TierId};

use crate::rules::Sizing;
use crate::tiers::tier_for_days;

/// What kind of line a requirement line is, read from its id (docs/QUANTITY_RULES.md): consumers
/// add up only [`LineKind::Need`] lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LineKind {
    /// What the household needs for the bucket: `bucket.rule` or `bucket.rule.person_N`.
    Need,
    /// Another way to meet the need line `bucket.rule`, or a household supply staged for it (a
    /// go-bag's water, drawn from the stored water): `bucket.rule.alt.variant`, or
    /// `bucket.rule.alt.variant.person_N` for one person's line `bucket.rule.person_N`. Never
    /// added to anything.
    Alternative,
    /// Worth having for some households, not needed to meet the target: `bucket.rule.optional`.
    Optional,
    /// Information for the plan and the packet: `bucket.rule.note`.
    Note,
}

impl LineKind {
    /// The kind of the line with this id.
    pub fn of(id: &str) -> LineKind {
        let mut parts = id.split('.').skip(2);
        match parts.next() {
            Some("alt") => LineKind::Alternative,
            Some("optional") => LineKind::Optional,
            Some("note") => LineKind::Note,
            _ => LineKind::Need,
        }
    }

    /// For an alternative line, the id of the need line it stands in for: `bucket.rule`, or
    /// `bucket.rule.person_N` when the alternative ends in `.person_N`.
    pub fn alternative_to(id: &str) -> Option<String> {
        let parts: Vec<&str> = id.split('.').collect();
        if parts.len() < 4 || parts[2] != "alt" {
            return None;
        }
        let need = format!("{}.{}", parts[0], parts[1]);
        match parts.get(4) {
            Some(person) if person.starts_with("person_") => Some(format!("{need}.{person}")),
            _ => Some(need),
        }
    }
}

/// A requirement line plus what the budget and plan crates use to cover it.
#[derive(Debug, Clone, PartialEq)]
pub struct SizedLine {
    /// The contract line (docs/ENGINE-API.md).
    pub line: RequirementLine,
    /// The quantity as an `f64` (the contract line holds it as `f32`), for exact arithmetic.
    pub quantity: f64,
    /// Need, alternative, optional or note.
    pub kind: LineKind,
    /// The tier this line belongs to.
    pub tier: TierId,
    /// Days of need the quantity covers (duration lines).
    pub days: Option<f64>,
    /// The household's amount per day in the line's unit, unrounded, so an owned quantity can be
    /// turned into days of coverage (duration lines).
    pub per_day: Option<f64>,
    /// The formula with the numbers filled in, for the expert view and `explain`.
    pub math: Vec<String>,
    /// At least one number behind the line is a planning estimate.
    pub prior: bool,
    /// The allocator orders it first within its tier (water, a dependent's medication, powered
    /// medical backup, smoke and CO alarms, infant formula).
    pub life_safety: bool,
}

impl SizedLine {
    /// The quantity for `days` days of the same need, for day-scaled lines (a tier's share of a
    /// longer target); `None` for lines that do not scale with days.
    pub fn quantity_for_days(&self, days: f64) -> Option<f64> {
        self.per_day.map(|r| r * days.max(0.0))
    }
}

/// How a sizing becomes a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape<'a> {
    Need,
    PerPerson(usize),
    Alternative {
        of: &'a str,
        variant: &'a str,
    },
    /// An alternative for one person's line `bucket.of.person_N`.
    PersonAlternative {
        of: &'a str,
        variant: &'a str,
        person: usize,
    },
    Optional,
    Note,
}

/// Wraps a sizing into a line for `bucket`. `tier` overrides the tier read from the sizing's days.
pub(crate) fn make(
    bucket: BucketId,
    s: Sizing,
    shape: Shape<'_>,
    tier: Option<TierId>,
    life_safety: bool,
) -> SizedLine {
    let (id, kind) = match shape {
        Shape::Need => (format!("{bucket}.{}", s.rule), LineKind::Need),
        Shape::PerPerson(n) => (format!("{bucket}.{}.person_{n}", s.rule), LineKind::Need),
        Shape::Alternative { of, variant } => (
            format!("{bucket}.{of}.alt.{variant}"),
            LineKind::Alternative,
        ),
        Shape::PersonAlternative {
            of,
            variant,
            person,
        } => (
            format!("{bucket}.{of}.alt.{variant}.person_{person}"),
            LineKind::Alternative,
        ),
        Shape::Optional => (format!("{bucket}.{}.optional", s.rule), LineKind::Optional),
        Shape::Note => (format!("{bucket}.{}.note", s.rule), LineKind::Note),
    };
    let tier = tier.unwrap_or_else(|| s.days.map_or(TierId::H72, tier_for_days));
    SizedLine {
        quantity: s.quantity,
        line: RequirementLine {
            id,
            bucket,
            item_class: s.item_class,
            quantity: s.quantity as f32,
            unit: s.unit.to_owned(),
            per: s.per,
            rule: s.rule.to_owned(),
            citations: s.citations,
            plain: s.plain,
        },
        kind,
        tier,
        days: s.days,
        per_day: s.per_day,
        math: s.math,
        prior: s.prior,
        life_safety,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_follow_the_id_grammar() {
        assert_eq!(LineKind::of("water_out.water_gallons"), LineKind::Need);
        assert_eq!(
            LineKind::of("get_home.get_home_bag.person_2"),
            LineKind::Need
        );
        assert_eq!(
            LineKind::of("supplies.food_kcal.alt.pantry"),
            LineKind::Alternative
        );
        assert_eq!(LineKind::of("power.fridge_wh.optional"), LineKind::Optional);
        assert_eq!(LineKind::of("power.solar_panel_watts.note"), LineKind::Note);
        assert_eq!(
            LineKind::alternative_to("supplies.food_kcal.alt.staples_grains").as_deref(),
            Some("supplies.food_kcal")
        );
        assert_eq!(LineKind::alternative_to("supplies.food_kcal"), None);
        // A staged supply for one person's line.
        let staged = "get_home.get_home_bag.alt.staged_water.person_2";
        assert_eq!(LineKind::of(staged), LineKind::Alternative);
        assert_eq!(
            LineKind::alternative_to(staged).as_deref(),
            Some("get_home.get_home_bag.person_2")
        );
        assert_eq!(
            LineKind::alternative_to("evacuate.go_bag.alt.staged_water").as_deref(),
            Some("evacuate.go_bag")
        );
    }
}
