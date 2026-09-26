//! Outputs of `assess` and `resolve_location` (docs/ENGINE-API.md).
//!
//! Units: money is `f32` US dollars and days are `f32`, as the contract says. Probabilities,
//! severity, expected annual loss and coordinates are `f64`.

use serde::{Deserialize, Serialize};

use crate::{BucketId, Citation, CitationId, HazardId, HazardTier, ItemId, TargetKind, TierId};

/// A location the engine recognised: a county, plus the ZIP code if one was given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationResolved {
    /// ISO 3166-1 alpha-2 country code (`"US"`).
    pub country: String,
    /// Five-digit county FIPS code, for example `"42101"`.
    pub county_fips: String,
    /// County name without the word "County", for example `"Philadelphia"`.
    pub county_name: String,
    /// Two-letter state abbreviation, for example `"PA"`.
    pub state_abbr: String,
    /// State name, for example `"Pennsylvania"`.
    pub state_name: String,
    /// The ZIP code, when the location came from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip: Option<String>,
    /// Share of the ZIP code's addresses inside this county, 0 to 1 (from the crosswalk).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip_county_share: Option<f32>,
    /// The county's centre.
    pub centroid: LatLon,
    /// Fifth National Climate Assessment region id, for example `"northeast"`.
    pub nca_region: String,
    /// The county touches the coast.
    pub coastal: bool,
    /// The county has a tsunami hazard zone.
    pub tsunami_zone: bool,
    /// Nearby facilities that change hazard chances.
    pub facility_flags: FacilityFlags,
    /// A note about the data, for example "your county; tract-level data not yet loaded".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_note: Option<String>,
}

/// A point on the map, in decimal degrees (WGS 84).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatLon {
    /// Latitude, −90 to 90.
    pub lat: f64,
    /// Longitude, −180 to 180.
    pub lon: f64,
}

/// Facilities near the county that change hazard chances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacilityFlags {
    /// A nuclear power plant within 16 km (10 miles, the plume emergency planning zone).
    pub nuclear_plant_within_16km: bool,
    /// A nuclear power plant within 80 km (50 miles, the ingestion planning zone).
    pub nuclear_plant_within_80km: bool,
    /// Facilities handling hazardous chemicals within 5 km.
    pub hazmat_facilities_within_5km: u16,
}

string_enum! {
    /// How much to trust a hazard estimate.
    pub enum DataConfidence: "data confidence" {
        /// Good local data.
        High = "high",
        /// Reasonable data with known gaps.
        Medium = "medium",
        /// Thin or indirect data.
        Low = "low",
        /// Expert judgement, not data. Shown to the user as such.
        Prior = "prior",
    }
}

string_enum! {
    /// Where a hazard appears in the register.
    pub enum HazardDisplay: "hazard display" {
        /// In the ranked list, ordered by what it means for this household.
        Ranked = "ranked",
        /// In a separate box for rare catastrophes (nuclear attack and EMP, war, terrorism), with
        /// likelihood and severity as two columns; never ranked by expected loss.
        RareCatastrophic = "rare_catastrophic",
    }
}

/// One hazard in the household's risk register.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardProfile {
    /// Which hazard.
    pub id: HazardId,
    /// Plain name ([`HazardId::name`]).
    pub name: String,
    /// Natural, societal or personal.
    pub tier: HazardTier,
    /// Ranked list or the rare-catastrophe box.
    pub display: HazardDisplay,
    /// Expected household-significant events per year, r_h (can exceed 1; climate multiplier
    /// already applied).
    pub rate_per_year: f64,
    /// A plausible low and high for `rate_per_year`, as `[low, high]`.
    pub rate_range: [f64; 2],
    /// Chance of at least one household-significant event in a year, 0 to 1: 1 − e^(−r_h), close
    /// to `rate_per_year` for rare events.
    pub annual_probability: f64,
    /// A plausible low and high for `annual_probability`, as `[low, high]`.
    pub probability_range: [f64; 2],
    /// Relative severity, 0 to 1, for ranking and copy.
    pub severity: f64,
    /// Expected annual loss per household in US dollars, where the National Risk Index gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eal_per_household_usd: Option<f64>,
    /// Multiplier applied for the climate dial; 1.0 for today.
    pub climate_multiplier: f64,
    /// How much to trust the estimate.
    pub confidence: DataConfidence,
    /// Where the numbers come from.
    pub sources: Vec<CitationId>,
    /// The natural-frequency sentence, for example "About 12 of 100 households like yours ...".
    pub frequency_sentence: String,
    /// The consequence buckets this hazard feeds.
    pub buckets: Vec<BucketId>,
}

/// The day values targets are rounded to: ½, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90, 180 and
/// 365 days. The engine rounds `Target::Days::value`; the app can use the same steps for sliders.
pub const TARGET_LADDER_DAYS: [f32; 15] = [
    0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 14.0, 21.0, 30.0, 45.0, 60.0, 90.0, 180.0, 365.0,
];

/// A bucket's target or coverage (the JSON `kind` tag says which shape). The kind matches
/// [`BucketId::target_kind`]: days for duration buckets, months for `income`, `evacuate` for
/// `evacuate`, readiness for the other readiness buckets and `home_loss`.
///
/// `low` and `high` are the 10th and 90th percentiles of the target under uncertainty in the
/// model's parameters. In [`BucketAssessment::covered`] and [`BucketAssessment::covered_today`]
/// there is no uncertainty, so `low` and `high` equal `value`, and `p_need_10yr` repeats the
/// target's.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    /// A number of days (duration buckets).
    Days {
        /// Days, rounded to [`TARGET_LADDER_DAYS`].
        value: f32,
        /// 10th percentile, in days.
        low: f32,
        /// 90th percentile, in days.
        high: f32,
    },
    /// A number of months (`income`: months of income gap to have saved).
    Months {
        /// Months.
        value: f32,
        /// 10th percentile, in months.
        low: f32,
        /// 90th percentile, in months.
        high: f32,
    },
    /// Leaving home quickly (`evacuate`).
    Evacuate {
        /// Chance of having to leave at least once in the next ten years, 0 to 1.
        p_need_10yr: f64,
        /// Least warning to expect, in hours (minutes are fractions of an hour).
        notice_hours_low: f32,
        /// Most warning to expect, in hours.
        notice_hours_high: f32,
        /// Days away from home to plan for.
        days_away: f32,
    },
    /// A checklist (the other readiness buckets, and `home_loss`: insurance and documents).
    Readiness {
        /// Chance of needing it at least once in the next ten years, 0 to 1.
        p_need_10yr: f64,
        /// Steps done.
        done: u8,
        /// Steps in the checklist.
        of: u8,
    },
}

impl Target {
    /// Which shape this is.
    pub const fn kind(&self) -> TargetKind {
        match self {
            Target::Days { .. } => TargetKind::Days,
            Target::Months { .. } => TargetKind::Months,
            Target::Evacuate { .. } => TargetKind::Evacuate,
            Target::Readiness { .. } => TargetKind::Readiness,
        }
    }
}

/// One hazard's share of a bucket's target.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    /// The hazard.
    pub hazard: HazardId,
    /// Its share of the bucket, 0 to 1; a bucket's shares add up to 1.
    pub share: f32,
}

/// When relief comes for the design event (the Oregon Resilience Plan's two-tier rating).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relief {
    /// Days until outside help plausibly arrives.
    pub help_arrives_days: f32,
    /// Days until service is mostly restored.
    pub mostly_restored_days: f32,
    /// Where these numbers come from.
    pub sources: Vec<CitationId>,
}

/// One consequence bucket for this household: how long to be ready for, and how much is covered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BucketAssessment {
    /// Which bucket.
    pub id: BucketId,
    /// Plain name ([`BucketId::name`]).
    pub name: String,
    /// How much to be ready for: the design event at the household's return period. Its kind
    /// matches [`BucketId::target_kind`].
    pub target: Target,
    /// How much the plan and what the household already has cover once every step is done (the
    /// plan's end point), in the same kind as `target`.
    pub covered: Target,
    /// How much the household has covered today, before the plan buys anything: what it owns and
    /// has checked off (`PlanInput::existing`, with the assumed basics when `assume_basics` is
    /// on), in the same kind as `target`. Never more than `covered`.
    pub covered_today: Target,
    /// The tier that is enough for this bucket; the plan stops adding to the bucket there.
    pub tier_enough: TierId,
    /// Which hazards drive the target, largest share first.
    pub contributions: Vec<Contribution>,
    /// Natural-frequency sentences for this bucket.
    pub frequency_sentences: Vec<String>,
    /// Where the numbers come from.
    pub sources: Vec<CitationId>,
    /// When outside help arrives and service is mostly restored, where known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relief: Option<Relief>,
}

string_enum! {
    /// What a requirement line's quantity scales with.
    pub enum Per: "per" {
        /// The household as a whole.
        Household = "household",
        /// Each person.
        Person = "person",
        /// Each commuter (get-home bags).
        Commuter = "commuter",
        /// Each pet.
        Pet = "pet",
    }
}

/// A sized need: how much of something the household needs for one bucket, with its sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementLine {
    /// Stable id of this line, for `explain`.
    pub id: String,
    /// The bucket it serves.
    pub bucket: BucketId,
    /// What is needed: a catalogue item id, or a class of items that several catalogue items can
    /// meet.
    pub item_class: String,
    /// How much the whole household needs, in `unit` (already multiplied out by `per`).
    pub quantity: f32,
    /// Unit of `quantity`, for example `"gallon"`.
    pub unit: String,
    /// What the quantity scales with.
    pub per: Per,
    /// The quantity rule that produced it (implemented in `rr-supply`).
    pub rule: String,
    /// Where the quantity comes from.
    pub citations: Vec<CitationId>,
    /// The line in plain language, for example "14 gallons: 1 gallon per person per day for 14
    /// days".
    pub plain: String,
}

string_enum! {
    /// What kind of step a plan item is.
    pub enum PlanItemKind: "plan item kind" {
        /// Costs nothing: do it this month.
        FreeAction = "free_action",
        /// Buy something.
        Purchase = "purchase",
        /// Put money aside toward something that costs more than one month's budget.
        Reserve = "reserve",
    }
}

/// A cost range in US dollars for a plan item's whole quantity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostRange {
    /// Low end.
    pub low: f32,
    /// High end.
    pub high: f32,
}

/// One step in the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanItem {
    /// The catalogue item.
    pub item_id: ItemId,
    /// Plain name.
    pub name: String,
    /// Free action, purchase or reserve.
    pub kind: PlanItemKind,
    /// How many, in `unit`.
    pub quantity: f32,
    /// Unit of `quantity`.
    pub unit: String,
    /// Estimated cost of the whole quantity: the recorded price if the household gave one,
    /// otherwise the middle of the price band. Between `price_band.low` and `price_band.high`
    /// unless it is a recorded price.
    pub est_cost_usd: f32,
    /// Typical cost range of the whole quantity.
    pub price_band: CostRange,
    /// Buckets this step helps with.
    pub buckets: Vec<BucketId>,
    /// Hazards behind those buckets, for the "why".
    pub hazards: Vec<HazardId>,
    /// Why this step, in plain language.
    pub why: String,
    /// Risk reduced by this step, in the allocator's units (DESIGN §4.7); larger is better.
    pub risk_reduction: f32,
    /// The tier this step belongs to.
    pub tier: TierId,
    /// The household has done or bought this. Omitted when false.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub done: bool,
    /// What the household recorded paying, in US dollars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_usd: Option<f32>,
}

/// One month of the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanMonth {
    /// Month number, counted from 0: month 0 begins on the planning date and holds the free
    /// actions first.
    pub index: u16,
    /// Money available this month in US dollars (the one-off amount lands in month 0).
    pub budget_usd: f32,
    /// Steps for this month, in the order to do them.
    pub items: Vec<PlanItem>,
}

/// Money being saved toward an item that costs more than a month's budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavingsEnvelope {
    /// The item being saved for.
    pub item_id: ItemId,
    /// Saved so far, in US dollars.
    pub saved_usd: f32,
    /// Total needed, in US dollars.
    pub needed_usd: f32,
}

/// The emergency-fund goal for the `income` bucket. Income is a separate savings goal, never
/// funded from the supplies budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavingsTrack {
    /// Months of expenses to have saved.
    pub target_months: f32,
    /// The same goal in US dollars.
    pub target_usd: f32,
    /// Months of expenses saved now.
    pub current_months: f32,
    /// A suggested monthly amount to put aside, in US dollars.
    pub monthly_suggestion_usd: f32,
    /// Why this goal, in plain language.
    pub why: String,
}

/// The month-by-month plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// Months in order, starting at month 0.
    pub months: Vec<PlanMonth>,
    /// The month by which every bucket is covered to its target, if the plan gets there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_month: Option<u16>,
    /// Savings toward items that cost more than one month's budget.
    pub envelopes: Vec<SavingsEnvelope>,
    /// The emergency-fund goal, when the household has income to protect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub savings_track: Option<SavingsTrack>,
}

/// A named scenario the engine considered for this location (for example a magnitude 9 Cascadia
/// earthquake), and whether the plan includes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInfo {
    /// Stable id, used by [`crate::ScenarioToggle::id`].
    pub id: String,
    /// Plain name.
    pub name: String,
    /// Why it applies here, in plain language.
    pub applies_because: String,
    /// Whether the plan includes it (the engine's default, or the user's override).
    pub on: bool,
    /// What including it changes, in plain language.
    pub effect_summary: String,
    /// Where it comes from.
    pub sources: Vec<CitationId>,
}

string_enum! {
    /// How strongly a warning is shown. Warnings never block.
    pub enum WarningSeverity: "warning severity" {
        /// Worth knowing.
        Note = "note",
        /// Worth acting on.
        Warn = "warn",
    }
}

/// A guardrail message (DESIGN §4.7): the plan looks off, but the user may know better.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Warning {
    /// Stable id, for `explain` and "keep anyway".
    pub id: String,
    /// How strongly to show it.
    pub severity: WarningSeverity,
    /// The warning in plain language.
    pub message: String,
    /// Why it matters.
    pub why: String,
    /// Ids of related hazards, buckets or items.
    pub related: Vec<String>,
}

/// Everything `assess` returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanOutput {
    /// Engine version (semver).
    pub engine_version: String,
    /// [`crate::ENGINE_API_VERSION`].
    pub api_version: u32,
    /// Version of the data packs used.
    pub data_pack_version: String,
    /// Version of the content (catalogue, citations, guidance) used.
    pub content_version: String,
    /// The resolved location.
    pub location: LocationResolved,
    /// Every hazard: the `ranked` ones most important first, then the `rare_catastrophic` ones.
    pub register: Vec<HazardProfile>,
    /// Every bucket, in [`BucketId::ALL`] order.
    pub buckets: Vec<BucketAssessment>,
    /// Named scenarios that apply to this location, each on or off.
    pub scenarios: Vec<ScenarioInfo>,
    /// The highest tier the plan covers so far.
    pub tier_reached: TierId,
    /// The tier that is enough for this household's risk.
    pub tier_recommended: TierId,
    /// The month-by-month plan.
    pub plan: Plan,
    /// Sized needs behind the plan.
    pub requirements: Vec<RequirementLine>,
    /// Guardrail warnings.
    pub warnings: Vec<Warning>,
    /// The printable packet (DESIGN §9), as Markdown.
    pub packet_markdown: String,
    /// Every citation referenced anywhere above.
    pub provenance: Vec<Citation>,
}
