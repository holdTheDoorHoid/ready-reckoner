//! Coverage: how many days of each duration bucket a household's items cover.
//!
//! The catalogue item (`rr_types::Item`) says which buckets an item helps with but not by how
//! much. That comes from **item metadata** the caller supplies ([`ItemMeta`]): for each duration
//! bucket, days of cover per unit, either per person (divided by the household size) or per
//! household. [`ContributionTable`] is the default [`CoverageRule`] built from that metadata;
//! `rr-plan` can derive the rates from `rr-supply`'s requirement lines with
//! [`apply_requirements`], or supply its own rule.
//!
//! ## Parts
//!
//! Some buckets need more than one kind of cover: a fan does nothing in an ice storm and a
//! sleeping bag nothing in a heat wave. A contribution may name a `part` of its bucket
//! (`thermal`: `"heat"`, `"cold"`). Each part is covered separately; the bucket's coverage is the
//! smallest part's. A contribution without a part covers every part.

use std::collections::{BTreeMap, BTreeSet};

use rr_types::{BucketId, BucketKind, ItemId, PlanInput, RequirementLine};
use serde::{Deserialize, Serialize};

/// Why item metadata was rejected. Bad metadata is a bug in the caller.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MetaError {
    /// A contribution gives both rates, or neither.
    #[error(
        "item `{item}`, bucket `{bucket}`: give exactly one of days_per_unit_per_person and days_per_unit_per_household"
    )]
    RateChoice {
        /// The item.
        item: ItemId,
        /// The bucket.
        bucket: BucketId,
    },
    /// A rate, step, quantity or harm estimate is negative or not finite (rates and steps must be
    /// above zero).
    #[error(
        "item `{item}`: `{field}` must be a finite number above zero (or zero or more for quantities)"
    )]
    Number {
        /// The item.
        item: ItemId,
        /// The field.
        field: &'static str,
    },
    /// A contribution names a bucket that is not a duration bucket, or a readiness credit names
    /// one that is not a readiness bucket.
    #[error("item `{item}`: bucket `{bucket}` is the wrong kind for `{field}`")]
    BucketKind {
        /// The item.
        item: ItemId,
        /// The bucket.
        bucket: BucketId,
        /// The field.
        field: &'static str,
    },
    /// Two metadata entries for the same item.
    #[error("item `{0}` has more than one metadata entry")]
    Duplicate(ItemId),
}

/// Days of cover one unit of an item adds to one duration bucket (or one part of it).
///
/// In TOML or JSON: `{ bucket = "water_out", days_per_unit_per_person = 1.0 }` (one gallon is one
/// person-day of water) or `{ bucket = "power", days_per_unit_per_household = 3.0 }` (a set of
/// lights covers the household for three days). Give exactly one of the two rates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contributes {
    /// A duration bucket.
    pub bucket: BucketId,
    /// The part of the bucket this covers (see the module docs). Absent: every part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// Days per unit for one person; divided by the number of people in the household.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days_per_unit_per_person: Option<f64>,
    /// Days per unit for the whole household.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days_per_unit_per_household: Option<f64>,
}

impl Contributes {
    /// A per-person contribution to the whole bucket.
    pub fn per_person(bucket: BucketId, days: f64) -> Self {
        Self {
            bucket,
            part: None,
            days_per_unit_per_person: Some(days),
            days_per_unit_per_household: None,
        }
    }

    /// A per-household contribution to the whole bucket.
    pub fn per_household(bucket: BucketId, days: f64) -> Self {
        Self {
            bucket,
            part: None,
            days_per_unit_per_person: None,
            days_per_unit_per_household: Some(days),
        }
    }

    /// The same contribution, limited to one part of the bucket.
    pub fn part(mut self, part: &str) -> Self {
        self.part = Some(part.to_owned());
        self
    }

    /// Days per unit for a household of `people`.
    pub fn household_days_per_unit(&self, people: usize) -> f64 {
        match (
            self.days_per_unit_per_person,
            self.days_per_unit_per_household,
        ) {
            (Some(p), None) => p / people.max(1) as f64,
            (None, Some(h)) => h,
            _ => 0.0,
        }
    }
}

/// What a readiness item does: the harm it avoids each time it is needed, in day-equivalents of
/// harm (a `Prior`). Its value is 10 · w · r_need · harm (DESIGN §4.7), with r_need from the
/// bucket's ten-year need probability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessCredit {
    /// A readiness bucket (`evacuate`, `get_home`, `medical_emergency`, `fire`, `security`).
    pub bucket: BucketId,
    /// Day-equivalents of harm avoided per use.
    pub harm_day_equivalents: f64,
}

/// Roles the guardrails look for. Optional: without an explicit role the guardrails recognise
/// items by their buckets (see `guardrails.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRole {
    /// Backup power for a medical device.
    DevicePower,
    /// Keeps refrigerated medicine cold in a power cut.
    ColdChain,
    /// A go-bag (or the bag part of one).
    GoBag,
}

/// Everything the allocator needs to know about a catalogue item beyond `rr_types::Item`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemMeta {
    /// The catalogue item.
    pub item_id: ItemId,
    /// Days of cover per unit for duration buckets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributes: Vec<Contributes>,
    /// Harm avoided for readiness buckets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readiness: Vec<ReadinessCredit>,
    /// Bought in steps of this many units, as much as the current tier needs (water in gallons,
    /// food in person-days). Absent: a set, bought once in `set_quantity`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// How many units make the set for this household (for example four headlamps). Absent: the
    /// requirement line's quantity if [`apply_requirements`] found one, else 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_quantity: Option<f64>,
    /// Roles for the guardrails.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<ItemRole>,
}

impl ItemMeta {
    /// Metadata with no contributions (fill the fields you need).
    pub fn new(item_id: impl Into<ItemId>) -> Self {
        Self {
            item_id: item_id.into(),
            contributes: Vec::new(),
            readiness: Vec::new(),
            step: None,
            set_quantity: None,
            roles: Vec::new(),
        }
    }

    /// Checks rates, steps, quantities and bucket kinds.
    pub fn validate(&self) -> Result<(), MetaError> {
        let item = || self.item_id.clone();
        let positive = |x: f64| x.is_finite() && x > 0.0;
        for c in &self.contributes {
            if c.bucket.kind() != BucketKind::Duration {
                return Err(MetaError::BucketKind {
                    item: item(),
                    bucket: c.bucket,
                    field: "contributes",
                });
            }
            match (c.days_per_unit_per_person, c.days_per_unit_per_household) {
                (Some(x), None) | (None, Some(x)) => {
                    if !positive(x) {
                        return Err(MetaError::Number {
                            item: item(),
                            field: "days_per_unit",
                        });
                    }
                }
                _ => {
                    return Err(MetaError::RateChoice {
                        item: item(),
                        bucket: c.bucket,
                    });
                }
            }
        }
        for r in &self.readiness {
            if r.bucket.kind() != BucketKind::Readiness {
                return Err(MetaError::BucketKind {
                    item: item(),
                    bucket: r.bucket,
                    field: "readiness",
                });
            }
            if !positive(r.harm_day_equivalents) {
                return Err(MetaError::Number {
                    item: item(),
                    field: "harm_day_equivalents",
                });
            }
        }
        // No let-chains: the workspace's minimum Rust is 1.85.
        if self.step.is_some_and(|s| !positive(s)) {
            return Err(MetaError::Number {
                item: item(),
                field: "step",
            });
        }
        if self
            .set_quantity
            .is_some_and(|q| !(q.is_finite() && q >= 0.0))
        {
            return Err(MetaError::Number {
                item: item(),
                field: "set_quantity",
            });
        }
        Ok(())
    }
}

/// Turns what a household has into days of cover per duration bucket.
///
/// `items` lists `(item, quantity)` pairs in the item's unit; the same item appears at most once.
/// Implementations must be **monotone** (more of anything never lowers coverage) and
/// deterministic; the allocator's guarantees rest on both.
pub trait CoverageRule {
    /// Days of cover `items` give `bucket` for this household. For a bucket with parts, the
    /// smallest part's coverage.
    fn coverage(&self, bucket: BucketId, items: &[(ItemId, f64)], household: &PlanInput) -> f64;

    /// The parts of `bucket` that are covered separately. Empty (the default): one part.
    fn parts(&self, _bucket: BucketId) -> Vec<String> {
        Vec::new()
    }

    /// Days of cover for one part of `bucket`. The default ignores parts.
    fn part_coverage(
        &self,
        bucket: BucketId,
        _part: &str,
        items: &[(ItemId, f64)],
        household: &PlanInput,
    ) -> f64 {
        self.coverage(bucket, items, household)
    }

    /// Days that adding `qty` of `item` adds to `bucket` (or to `part` of it). The default
    /// computes the difference; a linear rule can answer directly.
    fn gain(
        &self,
        bucket: BucketId,
        part: Option<&str>,
        items: &[(ItemId, f64)],
        item: &ItemId,
        qty: f64,
        household: &PlanInput,
    ) -> f64 {
        let mut more: Vec<(ItemId, f64)> = items.to_vec();
        match more.iter_mut().find(|(id, _)| id == item) {
            Some(entry) => entry.1 += qty,
            None => more.push((item.clone(), qty)),
        }
        let measure = |list: &[(ItemId, f64)]| match part {
            Some(p) => self.part_coverage(bucket, p, list, household),
            None => self.coverage(bucket, list, household),
        };
        (measure(&more) - measure(items)).max(0.0)
    }
}

/// The default [`CoverageRule`]: linear in quantity, from [`ItemMeta::contributes`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ContributionTable {
    rates: BTreeMap<ItemId, Vec<Contributes>>,
    parts: BTreeMap<BucketId, BTreeSet<String>>,
}

impl ContributionTable {
    /// Builds the table, validating every entry.
    pub fn new(meta: &[ItemMeta]) -> Result<Self, MetaError> {
        let mut table = ContributionTable::default();
        for m in meta {
            m.validate()?;
            if table.rates.contains_key(&m.item_id) {
                return Err(MetaError::Duplicate(m.item_id.clone()));
            }
            for c in &m.contributes {
                if let Some(p) = &c.part {
                    table.parts.entry(c.bucket).or_default().insert(p.clone());
                }
            }
            table.rates.insert(m.item_id.clone(), m.contributes.clone());
        }
        Ok(table)
    }

    /// Days one unit of `item` adds to `bucket` (or `part` of it) for this household.
    pub fn rate(
        &self,
        item: &ItemId,
        bucket: BucketId,
        part: Option<&str>,
        household: &PlanInput,
    ) -> f64 {
        let people = household.people.len();
        self.rates.get(item).map_or(0.0, |cs| {
            cs.iter()
                .filter(|c| c.bucket == bucket)
                .filter(|c| match (part, c.part.as_deref()) {
                    (_, None) => true,
                    (Some(want), Some(have)) => want == have,
                    (None, Some(_)) => false,
                })
                .map(|c| c.household_days_per_unit(people))
                .sum()
        })
    }

    fn track_coverage(
        &self,
        bucket: BucketId,
        part: Option<&str>,
        items: &[(ItemId, f64)],
        household: &PlanInput,
    ) -> f64 {
        items
            .iter()
            .map(|(id, q)| q.max(0.0) * self.rate(id, bucket, part, household))
            .sum()
    }
}

impl CoverageRule for ContributionTable {
    fn coverage(&self, bucket: BucketId, items: &[(ItemId, f64)], household: &PlanInput) -> f64 {
        match self.parts.get(&bucket) {
            Some(parts) if !parts.is_empty() => parts
                .iter()
                .map(|p| self.track_coverage(bucket, Some(p), items, household))
                .fold(f64::INFINITY, f64::min),
            _ => self.track_coverage(bucket, None, items, household),
        }
    }

    fn parts(&self, bucket: BucketId) -> Vec<String> {
        self.parts
            .get(&bucket)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn part_coverage(
        &self,
        bucket: BucketId,
        part: &str,
        items: &[(ItemId, f64)],
        household: &PlanInput,
    ) -> f64 {
        self.track_coverage(bucket, Some(part), items, household)
    }

    fn gain(
        &self,
        bucket: BucketId,
        part: Option<&str>,
        _items: &[(ItemId, f64)],
        item: &ItemId,
        qty: f64,
        household: &PlanInput,
    ) -> f64 {
        qty.max(0.0) * self.rate(item, bucket, part, household)
    }
}

/// Folds `rr-supply`'s requirement lines into item metadata, the recommended way for `rr-plan` to
/// wire the two crates together. For each line whose `item_class` is an item id:
///
/// - a **set** item takes the line's quantity as its set quantity (four headlamps for four people);
/// - a **divisible** item on a duration bucket whose target is known gets a per-household rate of
///   target ÷ quantity for that bucket, replacing any rate the metadata gave (12.3 gallons for a
///   3-day water target: 0.244 days per gallon, which already counts the dog and the dials).
///
/// Lines for other item classes, zero quantities and unknown targets are left alone.
pub fn apply_requirements(
    meta: &[ItemMeta],
    lines: &[RequirementLine],
    targets: &BTreeMap<BucketId, f64>,
) -> Vec<ItemMeta> {
    let mut out = meta.to_vec();
    for line in lines {
        if !(line.quantity.is_finite() && line.quantity > 0.0) {
            continue;
        }
        let quantity = f64::from(line.quantity);
        let Some(m) = out
            .iter_mut()
            .find(|m| m.item_id.as_str() == line.item_class)
        else {
            continue;
        };
        if m.step.is_none() {
            m.set_quantity = Some(quantity);
            continue;
        }
        if line.bucket.kind() != BucketKind::Duration {
            continue;
        }
        let Some(&target) = targets.get(&line.bucket) else {
            continue;
        };
        if target <= 0.0 {
            continue;
        }
        m.contributes
            .retain(|c| !(c.bucket == line.bucket && c.part.is_none()));
        m.contributes
            .push(Contributes::per_household(line.bucket, target / quantity));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::{Per, fixtures};

    fn phl() -> PlanInput {
        fixtures::get("philadelphia-renters-4").unwrap()
    }

    fn table() -> ContributionTable {
        let mut water = ItemMeta::new("water_stored");
        water.contributes = vec![
            Contributes::per_person(BucketId::WaterOut, 1.0),
            Contributes::per_person(BucketId::WaterBoil, 1.0),
        ];
        water.step = Some(1.0);
        let mut fans = ItemMeta::new("battery_fans");
        fans.contributes = vec![Contributes::per_household(BucketId::Thermal, 1.5).part("heat")];
        let mut blankets = ItemMeta::new("blankets");
        blankets.contributes =
            vec![Contributes::per_household(BucketId::Thermal, 3.0).part("cold")];
        let mut station = ItemMeta::new("power_station");
        station.contributes = vec![Contributes::per_household(BucketId::Thermal, 1.0)];
        ContributionTable::new(&[water, fans, blankets, station]).unwrap()
    }

    #[test]
    fn per_person_rates_divide_by_household_size() {
        let t = table();
        let h = phl(); // four people
        let items = vec![(ItemId::from("water_stored"), 12.0)];
        assert_eq!(t.coverage(BucketId::WaterOut, &items, &h), 3.0);
        assert_eq!(t.coverage(BucketId::WaterBoil, &items, &h), 3.0);
        assert_eq!(t.coverage(BucketId::Power, &items, &h), 0.0);
        let g = t.gain(
            BucketId::WaterOut,
            None,
            &items,
            &ItemId::from("water_stored"),
            2.0,
            &h,
        );
        assert_eq!(g, 0.5);
    }

    #[test]
    fn a_bucket_with_parts_is_covered_by_its_weakest_part() {
        let t = table();
        let h = phl();
        assert_eq!(t.parts(BucketId::Thermal), ["cold", "heat"]);
        let fans_only = vec![(ItemId::from("battery_fans"), 1.0)];
        assert_eq!(t.coverage(BucketId::Thermal, &fans_only, &h), 0.0);
        assert_eq!(
            t.part_coverage(BucketId::Thermal, "heat", &fans_only, &h),
            1.5
        );
        let both = vec![
            (ItemId::from("battery_fans"), 1.0),
            (ItemId::from("blankets"), 1.0),
        ];
        assert_eq!(t.coverage(BucketId::Thermal, &both, &h), 1.5);
        // A contribution without a part covers every part.
        let all = vec![
            (ItemId::from("battery_fans"), 1.0),
            (ItemId::from("blankets"), 1.0),
            (ItemId::from("power_station"), 1.0),
        ];
        assert_eq!(t.coverage(BucketId::Thermal, &all, &h), 2.5);
        // The trait's generic gain (by difference) agrees with the table's direct answer.
        struct Generic<'a>(&'a ContributionTable);
        impl CoverageRule for Generic<'_> {
            fn coverage(&self, b: BucketId, i: &[(ItemId, f64)], h: &PlanInput) -> f64 {
                self.0.coverage(b, i, h)
            }
            fn parts(&self, b: BucketId) -> Vec<String> {
                self.0.parts(b)
            }
            fn part_coverage(
                &self,
                b: BucketId,
                p: &str,
                i: &[(ItemId, f64)],
                h: &PlanInput,
            ) -> f64 {
                self.0.part_coverage(b, p, i, h)
            }
        }
        let g = Generic(&t);
        let id = ItemId::from("blankets");
        assert_eq!(
            g.gain(BucketId::Thermal, Some("cold"), &fans_only, &id, 1.0, &h),
            t.gain(BucketId::Thermal, Some("cold"), &fans_only, &id, 1.0, &h)
        );
    }

    #[test]
    fn metadata_is_validated() {
        let mut both = ItemMeta::new("x");
        both.contributes = vec![Contributes {
            bucket: BucketId::Power,
            part: None,
            days_per_unit_per_person: Some(1.0),
            days_per_unit_per_household: Some(1.0),
        }];
        assert!(matches!(both.validate(), Err(MetaError::RateChoice { .. })));
        let mut wrong_kind = ItemMeta::new("x");
        wrong_kind.contributes = vec![Contributes::per_household(BucketId::Evacuate, 1.0)];
        assert!(matches!(
            wrong_kind.validate(),
            Err(MetaError::BucketKind { .. })
        ));
        let mut zero_rate = ItemMeta::new("x");
        zero_rate.contributes = vec![Contributes::per_household(BucketId::Power, 0.0)];
        assert!(zero_rate.validate().is_err());
        let dup = [ItemMeta::new("x"), ItemMeta::new("x")];
        assert!(matches!(
            ContributionTable::new(&dup),
            Err(MetaError::Duplicate(_))
        ));
        // The documented TOML/JSON shape parses.
        let json = r#"{"item_id":"water_stored","step":1.0,
            "contributes":[{"bucket":"water_out","days_per_unit_per_person":1.0}],
            "readiness":[{"bucket":"evacuate","harm_day_equivalents":2.0}]}"#;
        let m: ItemMeta = serde_json::from_str(json).unwrap();
        m.validate().unwrap();
        assert!(serde_json::from_str::<ItemMeta>(r#"{"item_id":"x","colour":"red"}"#).is_err());
    }

    #[test]
    fn requirement_lines_set_quantities_and_rates() {
        let mut water = ItemMeta::new("water_stored");
        water.step = Some(1.0);
        water.contributes = vec![Contributes::per_person(BucketId::WaterOut, 1.0)];
        let lamps = ItemMeta::new("headlamp");
        let line = |class: &str, bucket, qty| RequirementLine {
            id: format!("{class}_line"),
            bucket,
            item_class: class.into(),
            quantity: qty,
            unit: "unit".into(),
            per: Per::Household,
            rule: "test".into(),
            citations: vec![],
            plain: String::new(),
        };
        let lines = [
            line("water_stored", BucketId::WaterOut, 12.3),
            line("headlamp", BucketId::Power, 4.0),
            line("unknown_class", BucketId::Power, 9.0),
        ];
        let targets = BTreeMap::from([(BucketId::WaterOut, 3.0)]);
        let out = apply_requirements(&[water, lamps], &lines, &targets);
        let rate = out[0].contributes[0].household_days_per_unit(4);
        assert!((rate - 3.0 / 12.3).abs() < 1e-6, "{rate}");
        assert_eq!(out[1].set_quantity, Some(4.0));
    }
}
