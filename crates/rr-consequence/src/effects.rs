//! The effects table (`effects.toml`, embedded at build time): what each hazard and named scenario
//! does to each consequence bucket, the income streams, and the parameters of the coupling rules.
//!
//! The table is parsed once per process ([`table`]) and validated: every row cites, every share is
//! a probability, the shares of one (hazard, bucket, household type) add up to 1 at most, duration
//! buckets have a duration, evacuate rows have a notice band, and classes are unique.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use rr_types::{BucketId, BucketKind, CitationId, DurationDist, Effect, Evidence, HazardId};
use serde::Deserialize;

/// The raw TOML, embedded so the engine never reads files at run time.
pub const EFFECTS_TOML: &str = include_str!("effects.toml");

/// Which households a row applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    /// Public water supply.
    Municipal,
    /// Private well.
    Well,
    /// At least one person with a commute.
    Commuter,
    /// No air conditioning.
    NoCooling,
    /// No heating system.
    NoHeating,
    /// A mobile or manufactured home.
    MobileHome,
    /// A coastal county.
    Coastal,
    /// Someone sleeps below street level (contract v2 `Housing::below_grade_bedroom`).
    BelowGrade,
    /// The household relies on SNAP or WIC for food (contract v2 `Finances::benefits`).
    FoodBenefit,
    /// The household relies on federal pay, SSI or SSDI, VA benefits or unemployment insurance
    /// for income (contract v2 `Finances::benefits`).
    IncomeBenefit,
    /// The county's records show a grid emergency in extreme cold (rolling blackouts), or, where
    /// the regional outage table is missing, it lies in a state on the ERCOT, SPP or MISO-South
    /// grids (model review M-11).
    ColdGridRegion,
}

impl Requirement {
    /// Plain-language description, for the expert view.
    pub const fn describe(self) -> &'static str {
        match self {
            Requirement::Municipal => "homes on public water",
            Requirement::Well => "homes on a private well",
            Requirement::Commuter => "households with a commuter",
            Requirement::NoCooling => "homes without air conditioning",
            Requirement::NoHeating => "homes without heating",
            Requirement::MobileHome => "mobile homes",
            Requirement::Coastal => "coastal counties",
            Requirement::BelowGrade => "homes where someone sleeps below street level",
            Requirement::FoodBenefit => "households that rely on SNAP or WIC",
            Requirement::IncomeBenefit => "households that rely on federal pay or a benefit",
            Requirement::ColdGridRegion => "places whose grid has failed in extreme cold",
        }
    }

    /// Snake-case name, for row keys.
    pub const fn key(self) -> &'static str {
        match self {
            Requirement::Municipal => "municipal",
            Requirement::Well => "well",
            Requirement::Commuter => "commuter",
            Requirement::NoCooling => "no_cooling",
            Requirement::NoHeating => "no_heating",
            Requirement::MobileHome => "mobile_home",
            Requirement::Coastal => "coastal",
            Requirement::BelowGrade => "below_grade",
            Requirement::FoodBenefit => "food_benefit",
            Requirement::IncomeBenefit => "income_benefit",
            Requirement::ColdGridRegion => "cold_grid_region",
        }
    }
}

/// Relief rating attached to a row: when outside help plausibly arrives and when service is
/// mostly (90 %) restored for this event.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReliefSpec {
    /// Days until outside help plausibly arrives.
    pub help_arrives_days: f64,
    /// Days until service is mostly restored.
    pub mostly_restored_days: f64,
    /// Where the numbers come from.
    pub sources: Vec<CitationId>,
}

/// Restoration-curve classes a power row may name in `curve_class` (the causes the regional
/// outage model keeps out of its pooled tail, plus the pooled storm causes).
pub const CURVE_CLASSES: [&str; 8] = [
    "hurricane",
    "cold_grid",
    "flood",
    "grid",
    "wildfire",
    "ice",
    "wind",
    "winter",
];

/// One row of the effects table.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectRow {
    /// The hazard (for scenario rows, the hazard family the scenario is counted under).
    pub hazard: HazardId,
    /// The named scenario this row belongs to, if any (for example `cascadia_m9`).
    #[serde(default)]
    pub scenario: Option<String>,
    /// Scenario variant (`coast` or `valley` for Cascadia).
    #[serde(default)]
    pub variant: Option<String>,
    /// The consequence bucket.
    pub bucket: BucketId,
    /// The event class within the hazard; rows with the same class are the same event.
    pub class: String,
    /// Plain-language name of the event class.
    pub label: String,
    /// Chance that a household-significant event is of this class and causes this consequence.
    pub p_given_event: f64,
    /// How long the consequence lasts (unused for readiness buckets other than `evacuate`).
    #[serde(default)]
    pub duration: Option<DurationDist>,
    /// What the numbers rest on.
    pub evidence: Evidence,
    /// What the duration alone rests on, when it differs from `evidence` (for example a measured
    /// restoration curve paired with an expert-estimated share). Defaults to `evidence`.
    #[serde(default)]
    pub duration_evidence: Option<Evidence>,
    /// Citation ids.
    pub sources: Vec<CitationId>,
    /// Households the row applies to (all when absent).
    #[serde(default)]
    pub requires: Option<Requirement>,
    /// A second condition, for rows that need two (a grid emergency in extreme cold that brings a
    /// boil-water notice: the place and public water).
    #[serde(default)]
    pub also_requires: Option<Requirement>,
    /// A short storm outage whose county-scale part comes from county outage statistics.
    #[serde(default)]
    pub pool: bool,
    /// County event types whose durations replace this row's.
    #[serde(default)]
    pub event_keys: Vec<String>,
    /// Share of these power cuts that happen during dangerous heat.
    #[serde(default)]
    pub heat_share: f64,
    /// Share that happen when heating is needed.
    #[serde(default)]
    pub cold_share: f64,
    /// Dropped when this named scenario is offered.
    #[serde(default)]
    pub replaced_by: Option<String>,
    /// Evacuate rows: least and most warning, in hours.
    #[serde(default)]
    pub notice_hours: Option<[f64; 2]>,
    /// Relief rating for this event.
    #[serde(default)]
    pub relief: Option<ReliefSpec>,
    /// 90th-percentile multiplier on the duration (uncertainty); defaults by evidence.
    #[serde(default)]
    pub duration_factor: Option<f64>,
    /// 90th-percentile multiplier on the share (uncertainty); defaults by evidence.
    #[serde(default)]
    pub p_factor: Option<f64>,
    /// The part of the hazard's rate this row's events come from, when `rr-hazards` splits the
    /// rate into separate event classes (see [`PartRow`]); absent, the whole rate.
    #[serde(default)]
    pub part: Option<String>,
    /// The share is scaled by how easily the household's public water system breaks: the county's
    /// drinking-water violations and the household's own answer (model review M-03). Only rows
    /// for homes on public water.
    #[serde(default)]
    pub fragile: bool,
    /// The row's events are county-scale and happen at the county's recorded rate of these NOAA
    /// Storm Events episodes (damaging share applied), not at the household's rate: a flood can
    /// shut the water plant without reaching the home. `p_given_event` is then the share of those
    /// episodes that cause the consequence. The row is left out where the county has no record.
    #[serde(default)]
    pub county_rate_keys: Vec<String>,
    /// The row's events happen at the rate of this `[params]` entry instead of the hazard's rate
    /// (a class the hazard's own count does not see, such as a grid emergency in extreme cold).
    #[serde(default)]
    pub fixed_rate: Option<String>,
    /// The regional restoration-curve class (`core/outage_curves.csv`) that scales this power
    /// row's duration: `hurricane`, `cold_grid`, `flood`, `grid` (model review M-10).
    #[serde(default)]
    pub curve_class: Option<String>,
    /// On an island grid with a hand-copied historic curve (Puerto Rico, the US Virgin Islands),
    /// that curve is this row's duration (model review M-10: Maria's restoration).
    #[serde(default)]
    pub island_curve: bool,
    /// Why we think this.
    #[serde(default)]
    pub note: String,
}

impl EffectRow {
    /// The row as the contract's [`Effect`] (for the expert view). Readiness rows without a
    /// duration get `{ kind: "fixed", days: 0 }`.
    pub fn to_effect(&self) -> Effect {
        Effect {
            hazard: self.hazard,
            bucket: self.bucket,
            p_given_event: self.p_given_event,
            duration: self.duration.unwrap_or(DurationDist::Fixed { days: 0.0 }),
            evidence: self.evidence,
            sources: self.sources.clone(),
        }
    }

    /// What the duration rests on (`duration_evidence`, or `evidence` when absent).
    pub fn duration_basis(&self) -> Evidence {
        self.duration_evidence.unwrap_or(self.evidence)
    }

    /// A stable key naming this row, used to seed its uncertainty draws.
    pub fn key(&self) -> String {
        let owner = match &self.scenario {
            Some(s) => format!("scenario:{s}:{}", self.variant.as_deref().unwrap_or("any")),
            None => format!("hazard:{}", self.hazard),
        };
        let req = self.requires.map_or("all", Requirement::key);
        match self.also_requires {
            Some(r) => format!("{owner}:{}:{}:{req}+{}", self.bucket, self.class, r.key()),
            None => format!("{owner}:{}:{}:{req}", self.bucket, self.class),
        }
    }
}

/// One income stream (research §3.5).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomeRow {
    /// The hazard (the family, for scenario streams).
    pub hazard: HazardId,
    /// The named scenario, if any.
    #[serde(default)]
    pub scenario: Option<String>,
    /// Plain-language name.
    pub label: String,
    /// Chance each earner loses income when the event happens (job loss: 1, the rate is already
    /// per household).
    pub per_earner: f64,
    /// Spell length in WEEKS (the `median_days`/`p90_days` fields hold weeks).
    pub spell_weeks: DurationDist,
    /// Unemployment insurance replaces part of the wages for the first weeks.
    pub unemployment_insurance: bool,
    /// The hazard's rate is already per household (like job loss): the stream runs at
    /// `r_h × per_earner`, and each spell costs one earner's share. Otherwise `r_h × per_earner ×
    /// earners`.
    #[serde(default)]
    pub household_rate: bool,
    /// Households the stream applies to (all when absent).
    #[serde(default)]
    pub requires: Option<Requirement>,
    /// What the numbers rest on.
    pub evidence: Evidence,
    /// Citation ids.
    pub sources: Vec<CitationId>,
    /// Why we think this.
    #[serde(default)]
    pub note: String,
}

/// A named parameter with its uncertainty and sources.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Param {
    /// Central value.
    pub value: f64,
    /// Low end (10th percentile), where uncertain.
    #[serde(default)]
    pub low: Option<f64>,
    /// High end (90th percentile), where uncertain.
    #[serde(default)]
    pub high: Option<f64>,
    /// Unit, in words.
    pub unit: String,
    /// A duration attached to the parameter (for example how long a well-pump repair takes).
    #[serde(default)]
    pub duration: Option<DurationDist>,
    /// What it rests on.
    pub evidence: Evidence,
    /// Citation ids.
    pub sources: Vec<CitationId>,
    /// Why we think this.
    #[serde(default)]
    pub note: String,
}

impl Param {
    /// The attached duration; the table validation guarantees one where the code asks for it.
    pub fn duration_or(&self, fallback: DurationDist) -> DurationDist {
        self.duration.unwrap_or(fallback)
    }
}

/// Every parameter in `[params]`, by name. Unknown or missing names fail the table validation.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(missing_docs)] // each field is documented by its `note` in effects.toml
pub struct Params {
    pub well_pump_failure: Param,
    pub wood_heat_shortfall: Param,
    pub booster_pump_floor: Param,
    pub cold_onset_days: Param,
    pub cold_chain_days: Param,
    pub walk_speed_mph: Param,
    pub walk_water_l_per_hour: Param,
    pub share_hours_away: Param,
    pub storm_pool_fallback_ratio: Param,
    pub relief_standard_days: Param,
    pub readiness_threshold: Param,
    pub ui_replacement: Param,
    pub ui_weeks: Param,
    pub job_spell_weeks: Param,
    pub weeks_per_month: Param,
    pub job_loss_base_rate: Param,
    pub stability_stable: Param,
    pub stability_variable: Param,
    pub stability_seasonal: Param,
    pub stability_gig: Param,
    pub displaced_back_within_week: Param,
    pub displaced_over_six_months: Param,
    pub never_return_renter: Param,
    pub never_return_owner: Param,
    pub duration_factor_empirical: Param,
    pub duration_factor_prior: Param,
    pub p_factor_empirical: Param,
    pub p_factor_prior: Param,
    pub outage_stats_rate_factor: Param,
    pub fragility_county_clean: Param,
    pub fragility_county_flagged: Param,
    pub fragility_record_fine: Param,
    pub fragility_record_occasional: Param,
    pub fragility_record_frequent: Param,
    pub fragility_cap: Param,
    pub fragility_floor: Param,
    pub fragility_uncertainty: Param,
    pub cold_emergency_rate: Param,
    pub public_water_power_share: Param,
    pub public_water_power_days: Param,
    pub housing_share_of_expenses: Param,
    pub curve_min_events: Param,
}

impl Params {
    /// Every parameter with its name, in declaration order (for the expert view and the docs).
    pub fn all(&self) -> Vec<(&'static str, &Param)> {
        vec![
            ("well_pump_failure", &self.well_pump_failure),
            ("wood_heat_shortfall", &self.wood_heat_shortfall),
            ("booster_pump_floor", &self.booster_pump_floor),
            ("cold_onset_days", &self.cold_onset_days),
            ("cold_chain_days", &self.cold_chain_days),
            ("walk_speed_mph", &self.walk_speed_mph),
            ("walk_water_l_per_hour", &self.walk_water_l_per_hour),
            ("share_hours_away", &self.share_hours_away),
            ("storm_pool_fallback_ratio", &self.storm_pool_fallback_ratio),
            ("relief_standard_days", &self.relief_standard_days),
            ("readiness_threshold", &self.readiness_threshold),
            ("ui_replacement", &self.ui_replacement),
            ("ui_weeks", &self.ui_weeks),
            ("job_spell_weeks", &self.job_spell_weeks),
            ("weeks_per_month", &self.weeks_per_month),
            ("job_loss_base_rate", &self.job_loss_base_rate),
            ("stability_stable", &self.stability_stable),
            ("stability_variable", &self.stability_variable),
            ("stability_seasonal", &self.stability_seasonal),
            ("stability_gig", &self.stability_gig),
            (
                "displaced_back_within_week",
                &self.displaced_back_within_week,
            ),
            ("displaced_over_six_months", &self.displaced_over_six_months),
            ("never_return_renter", &self.never_return_renter),
            ("never_return_owner", &self.never_return_owner),
            ("duration_factor_empirical", &self.duration_factor_empirical),
            ("duration_factor_prior", &self.duration_factor_prior),
            ("p_factor_empirical", &self.p_factor_empirical),
            ("p_factor_prior", &self.p_factor_prior),
            ("outage_stats_rate_factor", &self.outage_stats_rate_factor),
            ("fragility_county_clean", &self.fragility_county_clean),
            ("fragility_county_flagged", &self.fragility_county_flagged),
            ("fragility_record_fine", &self.fragility_record_fine),
            (
                "fragility_record_occasional",
                &self.fragility_record_occasional,
            ),
            ("fragility_record_frequent", &self.fragility_record_frequent),
            ("fragility_cap", &self.fragility_cap),
            ("fragility_floor", &self.fragility_floor),
            ("fragility_uncertainty", &self.fragility_uncertainty),
            ("cold_emergency_rate", &self.cold_emergency_rate),
            ("public_water_power_share", &self.public_water_power_share),
            ("public_water_power_days", &self.public_water_power_days),
            ("housing_share_of_expenses", &self.housing_share_of_expenses),
            ("curve_min_events", &self.curve_min_events),
        ]
    }

    /// The parameter named `name`, if there is one (for rows with a `fixed_rate`).
    pub fn get(&self, name: &str) -> Option<&Param> {
        self.all()
            .into_iter()
            .find_map(|(n, p)| (n == name).then_some(p))
    }
}

/// A part of a hazard's rate that rows can take instead of the whole rate: `rr-hazards` passes
/// each part's own rate (`rr_hazards::RatePart`), so events of different classes that the
/// hazard crate counts separately (wildfire warnings to leave and safety power shutoffs) are
/// never added together and re-split with fixed shares (model review M-06).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartRow {
    /// The hazard.
    pub hazard: HazardId,
    /// The part's name, as `rr-hazards` gives it.
    pub part: String,
    /// The part's share of the hazard's rate when no split is passed (callers with whole rates
    /// only, such as the research calibration).
    pub fallback_share: f64,
    /// Plain-language name of the events the part counts.
    pub label: String,
    /// Why we think this.
    #[serde(default)]
    pub note: String,
}

/// A named scenario whose long-run share is already inside its parent hazard's rate.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlapRow {
    /// The scenario id.
    pub scenario: String,
    /// The parent hazard whose typical rows give up the scenario's share.
    pub hazard: HazardId,
    /// Why.
    #[serde(default)]
    pub note: String,
}

/// A well-documented public water failure, for the "worst event on record" line of the water
/// buckets (`BucketAssessment::stress_test`). The data pack's worst-event table covers power only,
/// so these few are copied by hand from the sources named, and a county is matched by state.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterEvent {
    /// Stable id.
    pub id: String,
    /// The event, as the stress line names it ("Hurricane Helene").
    pub event: String,
    /// Where it was recorded ("Asheville, North Carolina").
    pub place: String,
    /// States whose households see this event as their record (two-letter abbreviations).
    pub states: Vec<String>,
    /// The day it began (`YYYY-MM-DD`).
    pub date: String,
    /// `water_out` (no tap water) or `water_boil` (a boil-water notice).
    pub bucket: BucketId,
    /// Days until about half of the affected households had service back.
    pub median_days: f64,
    /// Days until about nine in ten had it back.
    pub p90_days: f64,
    /// Citation ids.
    pub sources: Vec<CitationId>,
    /// What the numbers rest on and what is not checked.
    #[serde(default)]
    pub note: String,
}

/// The whole table.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectsTable {
    /// Version label of the table.
    pub version: String,
    /// Coupling, money and range parameters.
    pub params: Params,
    /// Hazard and scenario effects.
    #[serde(rename = "effect")]
    pub effects: Vec<EffectRow>,
    /// Income streams.
    #[serde(rename = "income")]
    pub income: Vec<IncomeRow>,
    /// Scenario shares inside parent hazard rates.
    #[serde(rename = "overlap")]
    pub overlaps: Vec<OverlapRow>,
    /// Parts of hazard rates that rows can take instead of the whole rate.
    #[serde(rename = "part", default)]
    pub parts: Vec<PartRow>,
    /// Documented water failures for the water buckets' stress line.
    #[serde(rename = "water_event", default)]
    pub water_events: Vec<WaterEvent>,
}

/// Why the effects table was rejected.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TableError {
    /// The TOML does not parse or does not match the schema.
    #[error("effects.toml does not parse: {0}")]
    Parse(String),
    /// A row is invalid.
    #[error("effects.toml row {row} ({key}): {problem}")]
    Row {
        /// Index of the row among `[[effect]]` rows.
        row: usize,
        /// The row's key.
        key: String,
        /// What is wrong.
        problem: String,
    },
    /// An income stream or a parameter is invalid.
    #[error("effects.toml: {0}")]
    Other(String),
}

impl EffectsTable {
    /// Parses and validates a table from TOML text.
    pub fn parse(text: &str) -> Result<EffectsTable, TableError> {
        let table: EffectsTable =
            toml::from_str(text).map_err(|e| TableError::Parse(e.to_string()))?;
        table.validate()?;
        Ok(table)
    }

    /// Checks every row, stream and parameter (see the module docs).
    pub fn validate(&self) -> Result<(), TableError> {
        let mut keys: BTreeMap<String, usize> = BTreeMap::new();
        let mut shares: BTreeMap<String, f64> = BTreeMap::new();
        for (i, row) in self.effects.iter().enumerate() {
            let key = row.key();
            let fail = |problem: String| TableError::Row {
                row: i,
                key: key.clone(),
                problem,
            };
            if let Some(prev) = keys.insert(key.clone(), i) {
                return Err(fail(format!("duplicate of row {prev}")));
            }
            row.to_effect()
                .validate()
                .map_err(|e| fail(e.to_string()))?;
            if row.sources.iter().any(|s| !s.is_well_formed()) {
                return Err(fail("a citation id is not snake_case".into()));
            }
            if !rr_types::is_well_formed_id(&row.class) {
                return Err(fail("class is not snake_case".into()));
            }
            if row.label.trim().is_empty() {
                return Err(fail("label is empty".into()));
            }
            let needs_duration =
                row.bucket.kind() == BucketKind::Duration || row.bucket == BucketId::Evacuate;
            if needs_duration && row.duration.is_none() {
                return Err(fail("duration buckets and evacuate need a duration".into()));
            }
            if row.bucket == BucketId::Income {
                return Err(fail(
                    "income is modelled in [[income]], not [[effect]]".into(),
                ));
            }
            match (row.bucket == BucketId::Evacuate, row.notice_hours) {
                (true, None) => return Err(fail("evacuate rows need notice_hours".into())),
                (true, Some([lo, hi]))
                    if lo.is_nan() || lo < 0.0 || hi.is_nan() || hi < lo || !hi.is_finite() =>
                {
                    return Err(fail(format!("notice_hours [{lo}, {hi}] is not a band")));
                }
                (false, Some(_)) => {
                    return Err(fail("only evacuate rows take notice_hours".into()));
                }
                _ => {}
            }
            for (name, v) in [
                ("heat_share", row.heat_share),
                ("cold_share", row.cold_share),
            ] {
                if !(0.0..=1.0).contains(&v) {
                    return Err(fail(format!("{name} {v} is not a share")));
                }
            }
            if (row.heat_share > 0.0 || row.cold_share > 0.0 || row.pool)
                && row.bucket != BucketId::Power
            {
                return Err(fail(
                    "heat_share, cold_share and pool apply to power rows".into(),
                ));
            }
            if row.variant.is_some() && row.scenario.is_none() {
                return Err(fail("variant needs a scenario".into()));
            }
            if row.scenario.is_some() && (row.pool || row.replaced_by.is_some()) {
                return Err(fail("scenario rows cannot be pooled or replaced".into()));
            }
            for f in [row.duration_factor, row.p_factor].into_iter().flatten() {
                if !f.is_finite() || f < 1.0 {
                    return Err(fail(format!("uncertainty factor {f} must be at least 1")));
                }
            }
            if let Some(r) = &row.relief {
                let ok = r.help_arrives_days >= 0.0
                    && r.mostly_restored_days >= r.help_arrives_days
                    && r.mostly_restored_days.is_finite();
                if !ok || r.sources.is_empty() {
                    return Err(fail(
                        "relief needs 0 <= help <= restored and a source".into(),
                    ));
                }
            }
            if let Some(part) = &row.part {
                if row.scenario.is_some() {
                    return Err(fail("scenario rows cannot take a part".into()));
                }
                if self.part(row.hazard, part).is_none() {
                    return Err(fail(format!(
                        "part `{part}` is not declared for {} in [[part]]",
                        row.hazard
                    )));
                }
            }
            // Rare families are shown in their own box and never enter a bucket's curve (DESIGN
            // §4.2, §4.7); retired ids are never emitted.
            if row.hazard.is_rare() || row.hazard.is_retired() {
                return Err(fail(format!(
                    "{} is a rare family or a retired id: it never enters a bucket's curve",
                    row.hazard
                )));
            }
            if row.fragile
                && row.requires != Some(Requirement::Municipal)
                && row.also_requires != Some(Requirement::Municipal)
            {
                return Err(fail("fragile rows are for homes on public water".into()));
            }
            let own_rate = [
                row.scenario.is_some(),
                row.part.is_some(),
                !row.county_rate_keys.is_empty(),
                row.fixed_rate.is_some(),
            ];
            if own_rate.iter().filter(|x| **x).count() > 1 {
                return Err(fail(
                    "a row takes at most one of scenario, part, county_rate_keys and fixed_rate"
                        .into(),
                ));
            }
            if row
                .county_rate_keys
                .iter()
                .any(|k| !rr_types::is_well_formed_id(k))
            {
                return Err(fail("county_rate_keys must be snake_case".into()));
            }
            if let Some(name) = &row.fixed_rate {
                if self.params.get(name).is_none() {
                    return Err(fail(format!("fixed_rate `{name}` is not a parameter")));
                }
            }
            if let Some(c) = &row.curve_class {
                if row.bucket != BucketId::Power || !CURVE_CLASSES.contains(&c.as_str()) {
                    return Err(fail(format!(
                        "curve_class `{c}` needs a power row and one of {CURVE_CLASSES:?}"
                    )));
                }
            }
            if row.island_curve && row.curve_class.as_deref() != Some("hurricane") {
                return Err(fail("island_curve is for hurricane power rows".into()));
            }
            let basis = if !row.county_rate_keys.is_empty() {
                "county".to_owned()
            } else if let Some(f) = &row.fixed_rate {
                format!("fixed:{f}")
            } else {
                row.part.as_deref().unwrap_or("whole").to_owned()
            };
            let group = format!(
                "{}:{}:{}:{}:{}+{}",
                row.scenario.as_deref().unwrap_or(row.hazard.as_str()),
                row.variant.as_deref().unwrap_or("any"),
                basis,
                row.bucket,
                row.requires.map_or("all".to_owned(), |r| format!("{r:?}")),
                row.also_requires
                    .map_or("all".to_owned(), |r| format!("{r:?}"))
            );
            let total = shares.entry(group.clone()).or_insert(0.0);
            *total += row.p_given_event;
            if *total > 1.0 + 1e-9 {
                return Err(fail(format!("shares in {group} add up to {total} > 1")));
            }
        }
        let mut part_shares: BTreeMap<HazardId, f64> = BTreeMap::new();
        for (i, p) in self.parts.iter().enumerate() {
            if !rr_types::is_well_formed_id(&p.part) || p.label.trim().is_empty() {
                return Err(TableError::Other(format!(
                    "part {i} needs a snake_case name and a label"
                )));
            }
            if !(0.0..=1.0).contains(&p.fallback_share) {
                return Err(TableError::Other(format!(
                    "part {i} ({} {}): fallback_share {} is not a share",
                    p.hazard, p.part, p.fallback_share
                )));
            }
            if self.parts[..i]
                .iter()
                .any(|q| q.hazard == p.hazard && q.part == p.part)
            {
                return Err(TableError::Other(format!(
                    "part {} {} is declared twice",
                    p.hazard, p.part
                )));
            }
            let total = part_shares.entry(p.hazard).or_insert(0.0);
            *total += p.fallback_share;
            if *total > 1.0 + 1e-9 {
                return Err(TableError::Other(format!(
                    "the fallback shares of {}'s parts add up to {total} > 1",
                    p.hazard
                )));
            }
        }
        for (i, e) in self.water_events.iter().enumerate() {
            let ok = !e.states.is_empty()
                && matches!(e.bucket, BucketId::WaterOut | BucketId::WaterBoil)
                && e.median_days > 0.0
                && e.p90_days >= e.median_days
                && e.p90_days.is_finite()
                && !e.sources.is_empty()
                && rr_types::Date::parse(&e.date).is_ok()
                && rr_types::is_well_formed_id(&e.id);
            if !ok {
                return Err(TableError::Other(format!(
                    "water event {i} ({}) needs states, a water bucket, 0 < median <= p90, a date \
                     and a source",
                    e.id
                )));
            }
        }
        for (i, s) in self.income.iter().enumerate() {
            if s.hazard.is_rare() || s.hazard.is_retired() {
                return Err(TableError::Other(format!(
                    "income stream {i} ({}): rare families and retired ids never enter a target",
                    s.label
                )));
            }
            if !(0.0..=1.0).contains(&s.per_earner) || s.sources.is_empty() {
                return Err(TableError::Other(format!(
                    "income stream {i} ({}) needs a share in [0, 1] and a source",
                    s.label
                )));
            }
            s.spell_weeks
                .validate()
                .map_err(|e| TableError::Other(format!("income stream {i}: {e}")))?;
        }
        for (name, p) in self.params.all() {
            let lo = p.low.unwrap_or(p.value);
            let hi = p.high.unwrap_or(p.value);
            if !p.value.is_finite()
                || lo > p.value
                || p.value > hi
                || lo.is_nan()
                || hi.is_nan()
                || p.sources.is_empty()
            {
                return Err(TableError::Other(format!(
                    "parameter {name} needs low <= value <= high and a source"
                )));
            }
            if let Some(d) = p.duration {
                d.validate()
                    .map_err(|e| TableError::Other(format!("parameter {name}: {e}")))?;
            }
        }
        for (name, p) in [
            ("well_pump_failure", &self.params.well_pump_failure),
            ("wood_heat_shortfall", &self.params.wood_heat_shortfall),
            (
                "storm_pool_fallback_ratio",
                &self.params.storm_pool_fallback_ratio,
            ),
            ("job_spell_weeks", &self.params.job_spell_weeks),
        ] {
            if p.duration.is_none() {
                return Err(TableError::Other(format!(
                    "parameter {name} needs a duration"
                )));
            }
        }
        Ok(())
    }

    /// The declared part `part` of `hazard`'s rate, if any.
    pub fn part(&self, hazard: HazardId, part: &str) -> Option<&PartRow> {
        self.parts
            .iter()
            .find(|p| p.hazard == hazard && p.part == part)
    }

    /// Every citation id used anywhere in the table, sorted and without repeats.
    pub fn citation_ids(&self) -> Vec<CitationId> {
        let mut ids: Vec<CitationId> = self
            .effects
            .iter()
            .flat_map(|r| {
                r.sources
                    .iter()
                    .chain(r.relief.iter().flat_map(|x| x.sources.iter()))
            })
            .chain(self.income.iter().flat_map(|s| s.sources.iter()))
            .chain(self.water_events.iter().flat_map(|e| e.sources.iter()))
            .chain(
                self.params
                    .all()
                    .into_iter()
                    .flat_map(|(_, p)| p.sources.iter()),
            )
            .cloned()
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// The contract [`Effect`] rows for hazards (no scenarios), in table order, for the expert
    /// view.
    pub fn hazard_effects(&self) -> Vec<Effect> {
        self.effects
            .iter()
            .filter(|r| r.scenario.is_none())
            .map(EffectRow::to_effect)
            .collect()
    }

    /// The buckets a hazard feeds (in [`BucketId::ALL`] order), including income streams.
    pub fn buckets_for(&self, hazard: HazardId) -> Vec<BucketId> {
        let mut out: Vec<BucketId> = self
            .effects
            .iter()
            .filter(|r| r.hazard == hazard && r.scenario.is_none())
            .map(|r| r.bucket)
            .chain(
                self.income
                    .iter()
                    .filter(|s| s.hazard == hazard && s.scenario.is_none())
                    .map(|_| BucketId::Income),
            )
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// The embedded table, parsed and validated once.
///
/// # Panics
///
/// If the embedded `effects.toml` is invalid. The crate's tests parse it, so a shipped build
/// never panics here.
pub fn table() -> &'static EffectsTable {
    static TABLE: OnceLock<EffectsTable> = OnceLock::new();
    TABLE.get_or_init(|| match EffectsTable::parse(EFFECTS_TOML) {
        Ok(t) => t,
        Err(e) => panic!("embedded effects table is invalid: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_table_parses_and_validates() {
        let t = EffectsTable::parse(EFFECTS_TOML).unwrap();
        assert!(t.effects.len() > 150, "{}", t.effects.len());
        assert_eq!(t.income.len(), 8);
    }

    #[test]
    fn every_effect_row_cites_and_says_what_it_rests_on() {
        let t = table();
        for row in &t.effects {
            assert!(!row.sources.is_empty(), "{}", row.key());
            // Expert estimates must cite a prior so the app can show them as such.
            if row.evidence == Evidence::Prior {
                assert!(
                    row.sources.iter().any(|s| s == "rr_risk_model_priors")
                        || row
                            .sources
                            .iter()
                            .any(|s| s == "oregon_resilience_plan_2013"
                                || s == "fema_hazus_eq_restoration"
                                || s == "ornl_repowrd_2022"
                                || s == "epa_asheville_boil_notice_2024"
                                || s == "shaffer_2026_texas_boil_notices"),
                    "{} is a prior without a prior or published-estimate source",
                    row.key()
                );
            }
        }
    }

    #[test]
    fn every_ranked_hazard_reaches_some_bucket_and_no_rare_one_does() {
        let t = table();
        for h in HazardId::ACTIVE {
            let buckets = t.buckets_for(*h);
            if *h == HazardId::EarnerDeathOrDisability {
                // An insurance question, not a savings or stockpile target (see docs).
                assert!(buckets.is_empty());
            } else if h.is_rare() {
                // Rare families are shown in their own box and never enter a bucket's curve.
                assert!(buckets.is_empty(), "{h} is rare but feeds {buckets:?}");
            } else {
                assert!(!buckets.is_empty(), "{h} feeds no bucket");
            }
        }
        #[allow(deprecated)]
        let retired = t.buckets_for(HazardId::Terrorism);
        assert!(
            retired.is_empty(),
            "the retired terrorism id still has rows"
        );
    }

    #[test]
    fn rows_for_rare_or_retired_hazards_are_rejected() {
        let rare = format!(
            "{EFFECTS_TOML}\n[[effect]]\nhazard = \"geomagnetic_storm\"\nbucket = \"power\"\nclass = \"storm\"\nlabel = \"x\"\np_given_event = 0.1\nduration = {{ kind = \"log_normal\", median_days = 3.0, p90_days = 30.0 }}\nevidence = \"prior\"\nsources = [\"rr_risk_model_priors\"]\n"
        );
        assert!(matches!(
            EffectsTable::parse(&rare),
            Err(TableError::Row { .. })
        ));
        let retired = format!(
            "{EFFECTS_TOML}\n[[effect]]\nhazard = \"terrorism\"\nbucket = \"security\"\nclass = \"lockdown\"\nlabel = \"x\"\np_given_event = 0.1\nevidence = \"prior\"\nsources = [\"rr_risk_model_priors\"]\n"
        );
        assert!(EffectsTable::parse(&retired).is_err());
        // A fragile row must be for homes on public water.
        let fragile = format!(
            "{EFFECTS_TOML}\n[[effect]]\nhazard = \"burglary\"\nbucket = \"water_out\"\nclass = \"odd\"\nlabel = \"x\"\np_given_event = 0.1\nduration = {{ kind = \"log_normal\", median_days = 1.0, p90_days = 3.0 }}\nfragile = true\nevidence = \"prior\"\nsources = [\"rr_risk_model_priors\"]\n"
        );
        assert!(EffectsTable::parse(&fragile).is_err());
    }

    #[test]
    fn water_events_name_a_water_bucket_and_a_source() {
        let t = table();
        assert!(!t.water_events.is_empty());
        for e in &t.water_events {
            assert!(matches!(e.bucket, BucketId::WaterOut | BucketId::WaterBoil));
            assert!(e.median_days <= e.p90_days, "{}", e.id);
            assert!(!e.sources.is_empty(), "{}", e.id);
        }
    }

    #[test]
    fn bad_rows_are_rejected() {
        let base = EFFECTS_TOML.to_owned();
        // A share above 1.
        let over_one = base.replacen("p_given_event = 0.667", "p_given_event = 1.667", 1);
        assert!(EffectsTable::parse(&over_one).is_err());
        // A duplicate row.
        let dup = format!(
            "{base}\n[[effect]]\nhazard = \"burglary\"\nbucket = \"security\"\nclass = \"break_in\"\nlabel = \"x\"\np_given_event = 0.1\nevidence = \"prior\"\nsources = [\"rr_risk_model_priors\"]\n"
        );
        assert!(matches!(
            EffectsTable::parse(&dup),
            Err(TableError::Row { .. })
        ));
        // A duration bucket without a duration.
        let nodur = format!(
            "{base}\n[[effect]]\nhazard = \"burglary\"\nbucket = \"power\"\nclass = \"odd\"\nlabel = \"x\"\np_given_event = 0.1\nevidence = \"prior\"\nsources = [\"rr_risk_model_priors\"]\n"
        );
        assert!(EffectsTable::parse(&nodur).is_err());
        // No sources.
        let nosrc = format!(
            "{base}\n[[effect]]\nhazard = \"burglary\"\nbucket = \"fire\"\nclass = \"odd\"\nlabel = \"x\"\np_given_event = 0.1\nevidence = \"prior\"\nsources = []\n"
        );
        assert!(EffectsTable::parse(&nosrc).is_err());
        // Shares of one group above 1.
        let over = format!(
            "{base}\n[[effect]]\nhazard = \"burglary\"\nbucket = \"security\"\nclass = \"other\"\nlabel = \"x\"\np_given_event = 0.5\nevidence = \"prior\"\nsources = [\"rr_risk_model_priors\"]\n"
        );
        assert!(EffectsTable::parse(&over).is_err());
        // An unknown field.
        let typo = base.replacen("heat_share = 0.15", "heat_shar = 0.15", 1);
        assert!(EffectsTable::parse(&typo).is_err());
    }

    #[test]
    fn citation_ids_are_well_formed() {
        let ids = table().citation_ids();
        assert!(ids.len() >= 20, "{}", ids.len());
        for id in ids {
            assert!(id.is_well_formed(), "{id}");
        }
    }
}
