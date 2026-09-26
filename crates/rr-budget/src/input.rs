//! What the allocator takes and what it returns.
//!
//! Everything here is plain data so tests can stub it; `rr-plan` fills it from the real crates:
//!
//! | Field | Comes from |
//! | --- | --- |
//! | [`BudgetInput::household`] | the user (`PlanInput`: people, finances, `existing`, dials) |
//! | [`BudgetInput::catalogue`] | `rr-content` (the items offered to this household) |
//! | [`BudgetInput::meta`], [`BudgetInput::requirements`] | `rr-supply` (rates and quantities; see [`crate::coverage::apply_requirements`]) |
//! | [`Risks`] | `rr-consequence` (Λ curves, targets, hazard shares, ten-year need, the simultaneous-need check) and `rr-hazards` (the register's yearly rates) |
//! | [`GuardrailContext`] | `rr-hazards` / `rr-consequence` / `rr-data` (flood and quake exposure, cliffs, storm surge) |

use std::collections::BTreeMap;

use rr_types::{
    BucketAssessment, BucketId, HazardId, Item, ItemId, Plan, PlanInput, RequirementLine, Target,
    TierId, Warning,
};
use serde::{Deserialize, Serialize};

use crate::coverage::ItemMeta;
use crate::curve::BucketCurve;

/// The risk side of the plan, from `rr-consequence`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Risks {
    /// Λ curve and target for each duration bucket. A duration bucket without a curve has no
    /// value to cover.
    pub curves: BTreeMap<BucketId, BucketCurve>,
    /// Bucket assessments. The allocator reads `contributions` (which hazards drive a bucket, for
    /// the "why" text), the readiness targets' `p_need_10yr`, the income target in months, and
    /// the income bucket's `frequency_sentences`.
    pub assessments: BTreeMap<BucketId, BucketAssessment>,
    /// Every hazard's yearly household rate from the register (`rr-hazards`'s
    /// `HazardProfile::rate_per_year`, the central estimate). The allocator reads the rare
    /// families' rates (a family's local ten-year chance decides whether the rare allowance may
    /// buy for it) and whether the arrest row applies (the savings track's legal line). Empty:
    /// no rare family qualifies.
    pub register: BTreeMap<HazardId, f64>,
    /// DESIGN §4.7's simultaneous-need check from `rr-consequence`
    /// (`ConsequenceAssessment::simultaneous`): for each event that sets a duration target, the
    /// other needs the same event brings at once. May be empty.
    pub simultaneous: Vec<SimultaneousNeed>,
}

/// The needs one design event brings at the same time (`rr-consequence`'s `SimultaneousNeed`,
/// copied as plain data).
#[derive(Debug, Clone, PartialEq)]
pub struct SimultaneousNeed {
    /// The event in words ("major hurricanes (category 3 or a direct hit)").
    pub event: String,
    /// Its hazard.
    pub hazard: HazardId,
    /// Every duration need the event brings: (bucket, days at the design event's severity, the
    /// chance the event brings that need at all, 0 to 1).
    pub needs: Vec<(BucketId, f64, f64)>,
}

impl Risks {
    /// The ten-year chance of needing a readiness bucket, from its target (0 when unknown).
    pub fn p_need_10yr(&self, bucket: BucketId) -> f64 {
        match self.assessments.get(&bucket).map(|a| a.target) {
            Some(Target::Readiness { p_need_10yr, .. })
            | Some(Target::Evacuate { p_need_10yr, .. }) => p_need_10yr.clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
}

/// A bucket whose target rests mostly on one rare event (the cliff rule, DESIGN §4.4), as
/// detected by `rr-consequence`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cliff {
    /// The bucket.
    pub bucket: BucketId,
    /// The event in plain words, for example "a Cascadia earthquake".
    pub driver: String,
}

/// Facts the guardrails need that the allocator cannot work out itself.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardrailContext {
    /// The home is in a flood-prone area (flood zone share or flood rate above the hazards
    /// crate's threshold).
    pub flood_zone: bool,
    /// The home is in an earthquake-prone area.
    pub quake_zone: bool,
    /// Buckets whose target rests on one rare event.
    pub cliffs: Vec<Cliff>,
    /// Share of the ZIP code inside the Category 1–3 storm-surge zone (NOAA/NHC, the optional
    /// surge pack), 0 to 1, when known (`LocationResolved::exposure.surge_cat3_share`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surge_zip_share: Option<f64>,
    /// The county's storm-surge proxy class from the core pack (`none`, `low`, `moderate`,
    /// `high`), when known (`LocationResolved::exposure.surge_proxy_class`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surge_county_class: Option<String>,
}

/// Share of each month's money a [`Schedule::Split`] plan puts toward an expensive item by default.
pub const DEFAULT_RESERVE_SHARE: f64 = 0.5;

/// How the allocator times purchases once it knows what to buy next.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Schedule {
    /// The default. When the top-priority item costs more than one month's money, put
    /// `reserve_share` of each month's new money into a sinking fund for it and spend the rest on
    /// the best affordable items in priority order; buy it as soon as the fund (with any free
    /// money) covers it. The household sees progress every month, and the expensive item still
    /// arrives within ceil(cost / (reserve_share × monthly money)) months of its first deposit.
    /// A bigger budget never ends the plan with less coverage in any bucket, but unlike
    /// [`Schedule::FixedOrder`] it can reach a bucket later in some month (the purchase order
    /// depends on the money).
    ///
    /// The one-off money (month 0) goes to life-safety items first. The top life-safety item is
    /// the first one in the buying order that costs more than a month's money: when the one-off
    /// money covers it, it is bought outright in month 0 (and the next such item is looked at);
    /// otherwise `reserve_share` of the one-off money opens its fund, the rest buys the cheaper
    /// life-safety items, cheapest first, and whatever those leave joins the fund.
    Split {
        /// Share of each month's new money set aside, above 0 and at most 1 (default 0.5).
        reserve_share: f64,
    },
    /// Buy in one fixed priority order; when the next item costs more than the money on hand,
    /// save everything for it. The order never depends on the budget, so more money only ever
    /// moves purchases earlier and every bucket is covered at least as well in every month. Two
    /// v2 rules keep this: the season rule and the automatic bare-minimum switch (a plan past
    /// three years) apply to the split schedule only. The one exception is the rare allowance: it
    /// starts once the three-day life-safety items are in hand, which comes sooner with more
    /// money, and from then on the main plan receives 90 % of each month's money instead of all
    /// of it, so with the allowance on the promise holds for the order, not month by month.
    FixedOrder,
    /// The research prototype's shortcuts (risk-model §4.2): when the next item is not
    /// affordable, buy the best affordable one instead, unless the next item costs at most twice
    /// the monthly budget and the affordable one is worth less than a quarter as much per dollar.
    ResearchShortcuts,
}

impl Default for Schedule {
    fn default() -> Self {
        Schedule::Split {
            reserve_share: DEFAULT_RESERVE_SHARE,
        }
    }
}

/// Knobs that are not part of `PlanInput`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetOptions {
    /// The longest plan, in months after month 0 (default 120, the ten-year horizon).
    pub max_months: u16,
    /// Opts in to every rare family, like the v1 switch `Dials::rare_catastrophic_opt_in`. The
    /// allocator also reads the families the household ticked (`Dials::rare_families()`); with
    /// neither, specialised rare-catastrophe items get $0. With an opt-in, an item is bought
    /// from an allowance of at most 10 % of each month's money, and only where one of its
    /// families is likely enough here (see `crate::rare`).
    pub rare_catastrophic_opt_in: bool,
    /// How purchases are timed.
    pub schedule: Schedule,
}

impl Default for BudgetOptions {
    fn default() -> Self {
        Self {
            max_months: 120,
            rare_catastrophic_opt_in: false,
            schedule: Schedule::default(),
        }
    }
}

/// Everything the allocator needs.
#[derive(Clone, Copy)]
pub struct BudgetInput<'a> {
    /// The household, its budget and what it already has.
    pub household: &'a PlanInput,
    /// The catalogue items offered to this household (free actions included).
    pub catalogue: &'a [Item],
    /// Coverage metadata for those items. Items without an entry cover nothing.
    pub meta: &'a [ItemMeta],
    /// Requirement lines from `rr-supply`: set quantities and per-day rates
    /// ([`crate::coverage::apply_requirements`]). May be empty.
    pub requirements: &'a [RequirementLine],
    /// Curves, targets and hazard shares.
    pub risks: &'a Risks,
    /// Extra facts for the guardrails.
    pub context: &'a GuardrailContext,
    /// Options.
    pub options: BudgetOptions,
}

/// Coverage at the end of one plan month.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MonthCoverage {
    /// The month (0 is the planning date).
    pub month: u16,
    /// Days covered per duration bucket (the weakest part for buckets with parts).
    pub days: BTreeMap<BucketId, f64>,
    /// Readiness items done or bought per readiness bucket.
    pub readiness_done: BTreeMap<BucketId, u32>,
}

/// Money at one month of the plan, for "this month" screens and the tests.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MonthMoney {
    /// The month.
    pub month: u16,
    /// Money free to spend at the start of the month, after this month's sinking-fund deposit (in
    /// month 0, after what the one-off money bought and set aside for life-safety items).
    pub available_usd: f64,
    /// Money in the main plan's sinking fund at the end of the month.
    pub saved_usd: f64,
    /// What the main plan's sinking fund is saving for at the end of the month, and what that
    /// costs now (`None` when no fund is running).
    pub saving_for: Option<(ItemId, f64)>,
    /// The cheapest item worth buying at the start of the month, other than the one being saved
    /// for. `None` when nothing is left to buy.
    pub cheapest_usd: Option<f64>,
}

/// One purchase in the order the allocator made it (before same-month purchases of one item are
/// merged into a single plan line).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Purchase {
    /// Month bought.
    pub month: u16,
    /// The item.
    pub item_id: ItemId,
    /// Quantity, in the item's unit.
    pub quantity: f64,
    /// Cost in US dollars.
    pub cost_usd: f64,
    /// The part of the cost paid from a sinking fund (money set aside in earlier months).
    pub from_savings_usd: f64,
    /// Value, in expected weighted disruption-days covered per decade (DESIGN §4.7).
    pub value: f64,
    /// The tier whose targets the purchase was valued against.
    pub tier: TierId,
    /// Bought ahead of its tier because it was at least five times better value per dollar than
    /// the tier's best item.
    pub promoted: bool,
    /// Bought from the capped rare-catastrophe allowance.
    pub rare_catastrophic: bool,
    /// Bought for the bare-minimum kit, before the tiers (bare-minimum mode).
    pub minimum: bool,
}

/// What the allocator returns. `rr-plan` copies `plan` and `warnings` into `PlanOutput`, fills
/// each `BucketAssessment::covered` from `covered` and `covered_today` from `covered_today`, and
/// takes the tiers from here.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BudgetResult {
    /// The month-by-month plan, with envelopes and the savings track.
    pub plan: Plan,
    /// Coverage at the end of the plan, per bucket, in the bucket's target kind (duration and
    /// readiness buckets; money buckets are left to the savings track and insurance).
    pub covered: BTreeMap<BucketId, Target>,
    /// Coverage before the plan buys anything, in the same shape as `covered`: what the household
    /// already has (existing inventory, including anything the caller credits as owned, and the
    /// free actions it has already done). The state `tier_reached` is measured on.
    pub covered_today: BTreeMap<BucketId, Target>,
    /// Coverage at the end of every month of the plan.
    pub coverage_by_month: Vec<MonthCoverage>,
    /// Money at every month of the plan.
    pub money_by_month: Vec<MonthMoney>,
    /// Every purchase in the order made.
    pub sequence: Vec<Purchase>,
    /// The highest tier whose targets what the household already has (existing inventory and free
    /// actions already done) meets: "so far".
    pub tier_reached: TierId,
    /// The highest tier whose targets are met at the end of the plan.
    pub tier_at_plan_end: TierId,
    /// The tier that is enough for this household's targets (DESIGN §4.5).
    pub tier_recommended: TierId,
    /// Guardrail warnings. Never errors: the user may know something the model does not.
    pub warnings: Vec<Warning>,
    /// Specialised rare-catastrophe items the allowance does not buy: the household did not tick
    /// their family, the family's local ten-year chance is under 1 in 1,000, the item names no
    /// rare family, or it would take its family past half the allowance.
    pub rare_catastrophic_skipped: Vec<ItemId>,
    /// The month the allocator ran out of anything worth buying, if the plan got there.
    pub stopped_month: Option<u16>,
    /// The month the bare-minimum kit is complete (also in `plan.minimum_done_month`).
    pub minimum_done_month: Option<u16>,
    /// When the plan in the normal buying order (before any bare-minimum reordering) runs out of
    /// things to buy: `None` when it never does within `max_months` (or has no monthly money).
    pub full_plan_stopped_month: Option<u16>,
    /// Citation ids behind numbers the plan's own sentences print that no bucket, item or line
    /// carries (the savings track's legal-emergency figure); `rr-plan` adds them to the
    /// provenance.
    pub citations: Vec<rr_types::CitationId>,
}

/// Why the allocator could not run. These are bugs in the caller, never user errors.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BudgetError {
    /// A bucket's curve is malformed.
    #[error("curve for `{bucket}`: {source}")]
    Curve {
        /// The bucket.
        bucket: BucketId,
        /// What is wrong.
        source: crate::curve::CurveError,
    },
    /// A curve was given for a bucket that is not a duration bucket.
    #[error("a curve was given for `{0}`, which is not a duration bucket")]
    CurveBucket(BucketId),
    /// Item metadata is malformed.
    #[error(transparent)]
    Meta(#[from] crate::coverage::MetaError),
    /// A split schedule's reserve share is not above 0 and at most 1.
    #[error("reserve_share must be above 0 and at most 1, got {0}")]
    ReserveShare(f64),
}
