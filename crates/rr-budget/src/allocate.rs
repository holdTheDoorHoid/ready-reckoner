//! The allocator (DESIGN §4.7, research risk-model §4.2; contract v2 additions from
//! DESIGN-DELTA §3).
//!
//! 1. **Free actions.** What the household already has (`existing`) counts from month 0. Free
//!    actions still to do are ordered: the fixed first three (alerts, the household plan, fire
//!    safety), then life-safety steps, then capabilities (a readiness share of 1: the thing that
//!    makes the household ready, such as the evacuation plan), then the rest by value; month 0
//!    lists at most eight and the rest follow in months 1 and 2 (about four a month, before that
//!    month's purchases), so no month shows more than eight ordinary free steps. Decisions (the
//!    insurance, ID and home-repair items), the long-horizon pointer, the clean-room plan and the
//!    90-day-fills step are free too but sit outside that count: each is scheduled in month 0 if
//!    it ranks among the first eight, otherwise in month 1. Their coverage counts in the plan's
//!    numbers from their month; purchases are planned knowing they are coming.
//! 2. **What to buy next.** Walk the tiers (three days, two weeks, one month, three, six, twelve
//!    months) with every bucket's target capped at the tier's horizon. The first tier with a
//!    positive-value candidate is the current tier. Candidates are items unlocked at or below it:
//!    a set not yet bought, or the next chunk of a divisible item (water, food, medicine) up to the
//!    next step on the day ladder. An accessory is a candidate only once one of the items it
//!    `requires` (any one of them) is in hand, so it never comes before its device. A later-tier
//!    item joins when its value per dollar is at least five times the tier's best (promotion). The
//!    order within the tier: life-safety items, then capabilities, then (in the default schedule)
//!    seasonal items whose season is under way or near ([`crate::season`]), then the rest by
//!    value per dollar, and long-horizon items last.
//! 3. **When to buy it** ([`Schedule`]). Split (default): when the next item costs more than the
//!    month's money, put a share of each month's money into a sinking fund for it and spend the
//!    rest on the best affordable items; otherwise buy in priority order. In month 0 the one-off
//!    money goes to life-safety items first (`one_off_to_life_safety`): the top life-safety item
//!    that a month's money cannot buy is bought outright when the one-off covers it; otherwise
//!    half the one-off opens its fund, the rest buys the cheaper life-safety items, cheapest
//!    first, and whatever they leave joins the fund. Fixed order: buy the next item when the money
//!    is there, otherwise save everything for it. The research shortcuts buy the best affordable
//!    item instead unless the sinking-fund rule says to wait.
//! 4. **Stop** when no tier has a positive-value candidate; later money goes to the savings track.
//!
//! **Readiness value** is the bucket's `10 · w · r_need · harm` times the item's
//! `readiness_share` (a whistle carries 5 % of what the go-bag does), and items with the same
//! `alternative_group` share one credit: once one is owned or scheduled, the others earn no
//! readiness value.
//!
//! **Bare-minimum mode** (`Dials::minimum_kit`, or a plan that would run past
//! [`PLAN_TOO_LONG_MONTHS`] months in the normal order): before the tiers, the allocator buys the
//! bare-minimum kit `rr-supply` marks (three days of water, one light and one pack of batteries,
//! warmth, three days of medicine and device power), part sets included (one headlamp, not four),
//! life-safety first then value per dollar; everything else follows in the usual order. Every
//! plan reports the month the kit is complete (`Plan::minimum_done_month`) beside the month
//! everything is (`Plan::done_month`).
//!
//! Specialised rare-catastrophe items never enter step 2. With an opt-in they are bought, in
//! order of value per dollar, from a separate allowance of 10 % of each month's money, only for
//! ticked families likely enough here, and only once the three-day life-safety items are in hand
//! ([`crate::rare`]).

use std::collections::{BTreeMap, BTreeSet};

use rr_types::{
    BucketId, BucketKind, CostRange, Date, HazardId, Item, ItemId, Plan, PlanItem, PlanItemKind,
    PlanMonth, SavingsEnvelope, TARGET_LADDER_DAYS, Target, TierId, WaterSource,
};

use crate::coverage::{ContributionTable, CoverageRule, ItemMeta, ItemRole, apply_requirements};
use crate::curve::Prepared;
use crate::explain::{self, DurationText, Lead, ReadinessText, WhyParts};
use crate::guardrails::{self, Facts, Shortfall, TooLong};
use crate::input::{
    BudgetError, BudgetInput, BudgetResult, MonthCoverage, MonthMoney, Purchase, Schedule,
};
use crate::rare;
use crate::savings;
use crate::season;
use crate::value::{
    PROMOTION_FACTOR, RARE_CATASTROPHIC_SHARE, READINESS_MIN_P_NEED_10YR, SINKING_FUND_MAX_MONTHS,
    SINKING_FUND_VALUE_RATIO, VALUE_HORIZON_YEARS, annual_rate_from_10yr, duration_value_with,
    per_100, readiness_value,
};
use crate::weights::harm_weight;

/// Tiers walked for purchases, in plan order (`now` holds only free actions).
const WALK: [TierId; 6] = [
    TierId::H72,
    TierId::W2,
    TierId::M1,
    TierId::M3,
    TierId::M6,
    TierId::Y1,
];

/// The quantity rule of items that make raw water safe (`rr-supply`'s `water_treatment_capacity`:
/// the gravity filter). In the stored-water chain they fill the days after the stored water.
const TREATMENT_RULE: &str = "water_treatment_capacity";

/// Money and day comparisons tolerate this much rounding.
const EPS: f64 = 1e-9;

/// Values at or below this are treated as zero (no value).
const VALUE_EPS: f64 = 1e-12;

/// The value a bare-minimum kit item is ranked with when what the household has already covers
/// its three days: above zero, so the kit is completed, and below any item that adds value.
const KIT_VALUE_FLOOR: f64 = 1e-9;

/// At most this many free actions still to do in month 0 (and in any month): the prior-art research
/// on choice overload and one-next-action puts the useful limit at 7 to 10 per step.
pub const FREE_ACTIONS_MONTH_0: usize = 8;

/// Free actions left over from month 0 are scheduled at about this many a month.
pub const FREE_ACTIONS_PER_MONTH: usize = 4;

/// Every free action is scheduled by this month (the first three months are 0, 1 and 2).
pub const FREE_ACTIONS_BY_MONTH: u16 = 2;

/// The first free steps for every household, in this order (round-2 review N7, RR-P14): being
/// warned comes before every other protective action (the Protective Action Decision Model in the
/// research's prior art), then how the household reaches each other and where it meets, then fire
/// safety at home. Catalogue ids; a household that has done one, or is not offered it, skips it.
pub const FIRST_FREE_STEPS: [&str; 3] = [
    "comms_wea_alerts_on",
    "comms_contact_card",
    "fire_test_alarms",
];

/// Free steps that take a month-0 slot only when one is left over: they matter to some
/// households only (a standby antibiotic prescription is for remote travel or a known recurring
/// condition), so they never push a step everyone needs out of month 0. Catalogue ids.
pub const LAST_FREE_STEPS: [&str; 1] = ["med_antibiotics_clinician_card"];

/// Readiness items whose need is not their bucket's own chance: (catalogue id, bucket, events a
/// year per household, the event in words). The fire extinguisher is for the small fires nobody
/// reports: about 6.6 a year for every 100 households, of which fire departments attend about
/// 3.4 % (CPSC's national survey of unreported residential fires, Greene and Andres 2009,
/// `cpsc_unreported_fires_2009`), not the reported house fires the fire bucket's chance counts
/// (round-2 review P-06). Its value and its "how often" sentence use this rate.
pub const READINESS_NEED_OVERRIDES: &[(&str, BucketId, f64, &str)] = &[(
    "fire_extinguisher",
    BucketId::Fire,
    0.066,
    "have a fire at home, even a small one,",
)];

/// Free steps outside the monthly count of free steps, like the decisions (`Item::decision`) and
/// the long-horizon pointer (a free `Item::long_horizon` item): the clean-room plan and the
/// 90-day-fills step (the supply workstream's catalogue counting rules, 2026-09-26). Each is
/// scheduled in month 0 when it ranks among the first eight free steps, otherwise in
/// [`EXEMPT_BY_MONTH`]. Catalogue ids.
pub const EXEMPT_FREE_STEPS: [&str; 2] = ["fire_clean_room_plan", "med_90_day_fills"];

/// Decisions and the other steps outside the monthly count are all scheduled by this month.
pub const EXEMPT_BY_MONTH: u16 = 1;

/// A plan that runs past this many months in the normal buying order switches to bare-minimum
/// mode and warns `plan_too_long` (REVIEW R6, model review M-12: three years).
pub const PLAN_TOO_LONG_MONTHS: u16 = 36;

/// The long-horizon section is shown when some duration target reaches this many days, or the
/// household asks for it (`rr-supply`'s `long_horizon_min_days`, DESIGN-DELTA §1.1).
pub const LONG_HORIZON_MIN_DAYS: f64 = 30.0;

/// Quantity rules the guardrails look for: a way to cook without power, and rain catchment (a raw
/// water source a filter can use).
const COOKING_RULE: &str = "cooking_capability";
const RAIN_RULE: &str = "rain_catchment_units";

/// The ten-year chance of needing readiness bucket `bucket`, for offer `i`: the bucket's own
/// chance, or the item's override ([`READINESS_NEED_OVERRIDES`]).
fn readiness_p10(ctx: &Ctx<'_>, i: usize, bucket: BucketId) -> f64 {
    match readiness_override(ctx, i, bucket) {
        Some((rate, _)) => -rr_types::math::exp_m1(-VALUE_HORIZON_YEARS * rate),
        None => ctx.input.risks.p_need_10yr(bucket),
    }
}

/// The override for offer `i` in `bucket`, if any: (events a year, the event in words).
fn readiness_override(ctx: &Ctx<'_>, i: usize, bucket: BucketId) -> Option<(f64, &'static str)> {
    let id = ctx.offers[i].item.id.as_str();
    READINESS_NEED_OVERRIDES
        .iter()
        .find(|(item, b, _, _)| *item == id && *b == bucket)
        .map(|(_, _, rate, words)| (*rate, *words))
}

/// Months for the free actions still to do, in the order given (life-safety first, then value):
/// up to [`FREE_ACTIONS_MONTH_0`] in month 0, then [`FREE_ACTIONS_PER_MONTH`] a month, raised just
/// enough (never above [`FREE_ACTIONS_MONTH_0`]) to finish by [`FREE_ACTIONS_BY_MONTH`]. Only a
/// catalogue with more than 24 free actions runs past month 2, at eight a month.
fn free_action_months(n: usize) -> Vec<u16> {
    let first = n.min(FREE_ACTIONS_MONTH_0);
    let mut out: Vec<u16> = std::iter::repeat_n(0, first).collect();
    let mut rest = n - first;
    let mut m: u16 = 1;
    while rest > 0 {
        let months_left = usize::from((FREE_ACTIONS_BY_MONTH + 1).saturating_sub(m).max(1));
        let k = rest
            .div_ceil(months_left)
            .clamp(FREE_ACTIONS_PER_MONTH, FREE_ACTIONS_MONTH_0)
            .min(rest);
        out.extend(std::iter::repeat_n(m, k));
        rest -= k;
        m += 1;
    }
    out
}

/// Months for the free steps still to do, in rank order, where `exempt[k]` marks a step outside
/// the monthly count (a decision, the long-horizon pointer, the clean-room plan, 90-day fills).
/// Month 0 lists the first [`FREE_ACTIONS_MONTH_0`] steps whatever their kind; the other ordinary
/// steps follow at the pace of [`free_action_months`], and the other exempt ones all go in
/// [`EXEMPT_BY_MONTH`].
fn schedule_free(exempt: &[bool]) -> Vec<u16> {
    let n = exempt.len();
    let first = n.min(FREE_ACTIONS_MONTH_0);
    let mut out = vec![0u16; n];
    let regular: Vec<usize> = (first..n).filter(|&k| !exempt[k]).collect();
    let paced = free_action_months(FREE_ACTIONS_MONTH_0 + regular.len());
    for (j, &k) in regular.iter().enumerate() {
        out[k] = paced[FREE_ACTIONS_MONTH_0 + j];
    }
    for k in first..n {
        if exempt[k] {
            out[k] = EXEMPT_BY_MONTH;
        }
    }
    out
}

/// Runs the allocator with the default coverage rule: [`ContributionTable`] built from the item
/// metadata after folding in the requirement lines.
pub fn allocate(input: &BudgetInput<'_>) -> Result<BudgetResult, BudgetError> {
    let meta = prepared_meta(input);
    let table = ContributionTable::new(&meta)?;
    run(input, &meta, &table)
}

/// Runs the allocator with a caller-supplied coverage rule (the item metadata still supplies set
/// sizes, steps, readiness credits and roles).
pub fn allocate_with_rule(
    input: &BudgetInput<'_>,
    rule: &dyn CoverageRule,
) -> Result<BudgetResult, BudgetError> {
    let meta = prepared_meta(input);
    for m in &meta {
        m.validate()?;
    }
    run(input, &meta, rule)
}

fn prepared_meta(input: &BudgetInput<'_>) -> Vec<ItemMeta> {
    let targets: BTreeMap<BucketId, f64> = input
        .risks
        .curves
        .iter()
        .map(|(b, c)| (*b, c.target_days))
        .collect();
    apply_requirements(input.meta, input.requirements, &targets)
}

/// A duration bucket, or one part of it, whose coverage the allocator tracks.
#[derive(Debug, Clone)]
struct Track {
    bucket: BucketId,
    part: Option<String>,
    /// Share of the bucket's disruption this track addresses (1 for a bucket without parts).
    share: f64,
    weight: f64,
    target: f64,
}

/// A catalogue item as the allocator sees it.
struct Offer<'a> {
    item: &'a Item,
    /// Price per unit: what the household recorded paying, else the middle of the price band.
    unit_price: f64,
    /// Units in the set (sets only).
    set_quantity: f64,
    /// Step for divisible items; `None` for sets.
    step: Option<f64>,
    /// Tracks this item may add to, in the order used to size chunks.
    tracks: Vec<usize>,
    /// Readiness credits: (bucket, harm day-equivalents).
    readiness: Vec<(BucketId, f64)>,
    roles: Vec<ItemRole>,
    /// The share of its readiness value the item carries (`Item::readiness_share`, 1 when absent).
    share: f64,
    /// A capability: its readiness share is 1 and it has a readiness credit.
    capability: bool,
    /// The item lists prerequisites (`Item::requires`, any one of them).
    has_requires: bool,
    /// The offers among those prerequisites.
    requires: Vec<usize>,
    /// The prerequisite is met outside the catalogue (always met): the household owns a listed
    /// device that is not offered, or none of the listed devices is offered at all.
    requires_owned: bool,
    /// Other offers in its alternative group.
    group: Vec<usize>,
    /// Its part in the bare-minimum kit: (index into `Ctx::min_lines`, line units per item unit).
    minimum: Vec<(usize, f64)>,
    /// The rare families it names in `hazard_extras`.
    families: Vec<HazardId>,
    /// A free step outside the monthly count of free steps (see [`schedule_free`]).
    exempt: bool,
}

/// One line of the bare-minimum kit and the offers that can meet it.
struct MinLine {
    /// The requirement line's id, for tests and debugging.
    #[allow(dead_code)]
    id: String,
    /// Line units the kit needs.
    need: f64,
    /// (offer, line units per item unit).
    contrib: Vec<(usize, f64)>,
}

struct Ctx<'a> {
    input: &'a BudgetInput<'a>,
    rule: &'a dyn CoverageRule,
    /// Each duration bucket's curve, with its segments worked out once.
    curves: BTreeMap<BucketId, Prepared<'a>>,
    offers: Vec<Offer<'a>>,
    tracks: Vec<Track>,
    weights: BTreeMap<BucketId, f64>,
    people: usize,
    years: u8,
    /// The bare-minimum kit's lines.
    min_lines: Vec<MinLine>,
    /// The rare families the household allows the allowance to buy for.
    families_on: BTreeSet<HazardId>,
    /// The planning date, for the season rule.
    planning: Date,
    /// The monthly budget.
    monthly: f64,
    /// The default split schedule (the season rule applies to it only).
    split: bool,
}

impl Ctx<'_> {
    /// Whether one of offer `i`'s prerequisites is in hand in `in_hand` (always true for an item
    /// with none).
    fn requires_met(&self, in_hand: &State, i: usize) -> bool {
        let o = &self.offers[i];
        !o.has_requires || o.requires_owned || o.requires.iter().any(|&j| in_hand.owned[j] > 0.0)
    }

    /// Whether another member of offer `i`'s alternative group is owned or scheduled.
    fn group_taken(&self, state: &State, i: usize) -> bool {
        self.offers[i].group.iter().any(|&j| state.owned[j] > 0.0)
    }

    /// Whether offer `i` is a seasonal item due in plan month `m` (see [`crate::season`]).
    fn season_due(&self, i: usize, m: u16) -> bool {
        self.split
            && self.offers[i]
                .item
                .season
                .is_some_and(|s| season::due(s, season::calendar_month(self.planning, m)))
    }

    /// Line units `state` holds toward kit line `l`.
    fn min_have(&self, state: &State, l: usize) -> f64 {
        self.min_lines[l]
            .contrib
            .iter()
            .map(|&(j, u)| state.owned[j].max(0.0) * u)
            .sum()
    }

    /// Whether every line of the bare-minimum kit is met in `state` (false with no kit at all).
    fn minimum_met(&self, state: &State) -> bool {
        !self.min_lines.is_empty()
            && (0..self.min_lines.len())
                .all(|l| self.min_have(state, l) + 1e-6 >= self.min_lines[l].need)
    }
}

#[derive(Debug, Clone)]
struct State {
    /// What the household has, sorted by item id, quantities merged.
    inventory: Vec<(ItemId, f64)>,
    track_cov: Vec<f64>,
    /// Quantity owned per offer (existing, free actions and purchases).
    owned: Vec<f64>,
    readiness_used: Vec<bool>,
}

/// One track a purchase moves.
#[derive(Debug, Clone)]
struct Gain {
    track: usize,
    x0: f64,
    x1: f64,
    value: f64,
}

#[derive(Debug, Clone)]
struct Candidate {
    offer: usize,
    qty: f64,
    cost: f64,
    tier: TierId,
    promoted: bool,
    /// Duration value plus readiness value from buckets needed often enough.
    core: f64,
    /// Readiness value from buckets needed less often than the threshold.
    low_p: f64,
    /// The value it is ranked and credited with.
    value: f64,
    gains: Vec<Gain>,
    /// (bucket, value, below the need threshold).
    ready: Vec<(BucketId, f64, bool)>,
}

impl Candidate {
    fn density(&self) -> f64 {
        density(self.value, self.cost)
    }
}

/// A candidate in the buying order: a pointer into the valuation cache plus what ranking needs.
#[derive(Debug, Clone, Copy)]
struct Pick {
    offer: usize,
    /// Index into [`WALK`] of the tier the candidate was valued against.
    ti: usize,
    cost: f64,
    /// The value it is ranked and credited with (includes rarely needed readiness value when it
    /// beats the tier's best).
    value: f64,
    promoted: bool,
    /// A bare-minimum kit purchase (its candidate lives in `Cache::min_cands`).
    minimum: bool,
    /// Its place among the non-life-safety items of its tier: 0 a capability, 1 a seasonal item
    /// due now, 2 the rest, 3 a long-horizon item.
    class: u8,
}

impl Pick {
    fn density(&self) -> f64 {
        density(self.value, self.cost)
    }
}

fn density(value: f64, cost: f64) -> f64 {
    if cost <= 0.0 {
        if value > 0.0 { f64::INFINITY } else { 0.0 }
    } else {
        value / cost
    }
}

/// Something that happened in the plan, before plan lines are assembled.
#[derive(Debug, Clone)]
enum Event {
    Free {
        cand: Candidate,
        done: bool,
    },
    Owned {
        cand: Candidate,
        paid: Option<f64>,
    },
    Buy {
        /// As valued by the allocator.
        cand: Candidate,
        /// Its effect on coverage as the plan reports it, for the explanation.
        shown: Candidate,
        rare: bool,
        from_savings: f64,
        /// Bought first with the one-off money (the top life-safety item it covers).
        one_off_first: bool,
        /// Bought for the bare-minimum kit.
        minimum: bool,
    },
    Reserve {
        offer: usize,
        tier: TierId,
        deposit: f64,
        saved: f64,
        needed: f64,
        how: SaveHow,
        carried_from: Option<usize>,
    },
}

/// What a sinking fund is for.
#[derive(Debug, Clone, Copy)]
struct FundFor {
    offer: usize,
    cost: f64,
    tier: TierId,
}

impl FundFor {
    fn of(p: &Pick) -> Self {
        FundFor {
            offer: p.offer,
            cost: p.cost,
            tier: WALK[p.ti],
        }
    }
}

/// The money of one track of the plan (the main plan, or the rare-catastrophe allowance).
#[derive(Debug, Clone, Default)]
struct Purse {
    /// Money free to spend.
    free: f64,
    /// Money in the sinking fund.
    fund: f64,
    /// What the fund is for; `None` when there is no fund, or its item is no longer needed (the
    /// money then pays for the next top-priority purchase).
    target: Option<FundFor>,
    /// The item the fund money was first put aside for, when it has since moved to another item.
    carried_from: Option<usize>,
}

impl Purse {
    /// Moves `amount` of free money into the fund for `target`, recording the deposit.
    fn deposit(&mut self, amount: f64, target: FundFor, how: SaveHow, events: &mut Vec<Event>) {
        // Whole cents and never more than is free, so the plan's lines add up exactly.
        let wanted = amount.min(self.free).max(0.0);
        let mut amount = (wanted * 100.0).round() / 100.0;
        if amount > self.free {
            amount = (self.free * 100.0).floor() / 100.0;
        }
        if amount <= EPS && self.fund <= EPS {
            // A fund opens only when money goes into it.
            return;
        }
        if let Some(old) = self.target {
            if old.offer != target.offer && self.fund > EPS {
                self.carried_from = Some(old.offer);
            }
        }
        self.target = Some(target);
        if amount <= EPS {
            return;
        }
        self.free -= amount;
        self.fund += amount;
        events.push(Event::Reserve {
            offer: target.offer,
            tier: target.tier,
            deposit: amount,
            saved: self.fund,
            needed: target.cost,
            how,
            carried_from: self.carried_from,
        });
    }

    /// Forgets the fund's item (it is no longer needed); the money stays in the fund.
    fn drop_target(&mut self) {
        if let Some(t) = self.target.take() {
            if self.fund > EPS {
                self.carried_from = Some(t.offer);
            }
        }
    }
}

/// How a sinking-fund deposit came about, for its explanation.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SaveHow {
    /// Everything on hand, because the next item costs more than a month's money.
    AllMoney,
    /// A share of each month's money; the rest buys other items.
    Share(f64),
    /// Month 0: this share of the one-off money, plus whatever the cheaper life-safety items leave
    /// of the rest.
    OneOff(f64),
    /// The rare-catastrophe allowance.
    Rare,
}

/// What every purchase adds to.
#[derive(Debug, Default)]
struct Ledger {
    sequence: Vec<Purchase>,
    /// Sinking funds per item id, in the order first used: (item, saved, needed).
    envelopes: Vec<(ItemId, f64, f64)>,
    purchase_months: BTreeMap<usize, u16>,
}

impl Ledger {
    /// Records money saved toward `item` (drawn by a purchase, or still in a fund when the plan
    /// ends). `Plan.envelopes` holds one entry per item id (ENGINE-API): an item saved for more
    /// than once (the chunks of a divisible item) adds up in one entry, whose `needed` is then the
    /// cost of all the purchases the savings went to.
    fn add_envelope(&mut self, item: &ItemId, saved: f64, needed: f64) {
        match self.envelopes.iter_mut().find(|(id, _, _)| id == item) {
            Some((_, s, n)) => {
                *s += saved;
                *n += needed;
            }
            None => self.envelopes.push((item.clone(), saved, needed)),
        }
    }

    fn into_envelopes(self) -> (Vec<Purchase>, Vec<SavingsEnvelope>, BTreeMap<usize, u16>) {
        let envelopes = self
            .envelopes
            .into_iter()
            .map(|(item_id, saved, needed)| SavingsEnvelope {
                item_id,
                saved_usd: money(saved),
                needed_usd: money(needed),
            })
            .collect();
        (self.sequence, envelopes, self.purchase_months)
    }
}

/// Whether a pass of the allocator buys the bare-minimum kit before the tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mode {
    minimum_first: bool,
}

/// Why the rare allowance may buy an item: the ticked, likely-enough families it is for, and the
/// name of the one cause inside them it is gated on, if any ([`rare::RARE_CAUSES`]).
#[derive(Debug, Clone, Default)]
struct RareFor {
    families: Vec<HazardId>,
    cause: Option<String>,
}

/// Everything one pass of the allocator produces, before the plan is assembled.
struct Outcome {
    events: Vec<Vec<Event>>,
    coverage_by_month: Vec<MonthCoverage>,
    money_by_month: Vec<MonthMoney>,
    /// Supplies' food coverage (or the bucket's weakest part) at the end of each month.
    food_by_month: Vec<Option<f64>>,
    sequence: Vec<Purchase>,
    envelopes: Vec<SavingsEnvelope>,
    purchase_months: BTreeMap<usize, u16>,
    month0_todo_free: Vec<Event>,
    month0_done_free: Vec<Event>,
    month0_owned: Vec<Event>,
    /// The allocator's state at the end (free actions all applied).
    state: State,
    /// What the household has before the plan buys anything.
    so_far: State,
    /// The plan's own state at the end (free actions from their month).
    credited: State,
    after_free: Vec<f64>,
    scheduled_free: Vec<(u16, usize, f64)>,
    stopped: Option<u16>,
    main_free: f64,
    main_fund: f64,
    rare_skipped: Vec<ItemId>,
    /// The ticked, likely-enough families of each item the rare allowance may buy.
    rare_families: BTreeMap<usize, RareFor>,
    checklist: Vec<ChecklistEntry>,
    minimum_done_month: Option<u16>,
    /// Offers still worth buying when the plan ended without running out of things to buy.
    left_over: Vec<usize>,
    minimum_first: bool,
}

fn run(
    input: &BudgetInput<'_>,
    meta: &[ItemMeta],
    rule: &dyn CoverageRule,
) -> Result<BudgetResult, BudgetError> {
    if let Schedule::Split { reserve_share } = input.options.schedule {
        if !(reserve_share.is_finite() && reserve_share > 0.0 && reserve_share <= 1.0) {
            return Err(BudgetError::ReserveShare(reserve_share));
        }
    }
    for (bucket, curve) in &input.risks.curves {
        if bucket.kind() != BucketKind::Duration {
            return Err(BudgetError::CurveBucket(*bucket));
        }
        curve.validate().map_err(|source| BudgetError::Curve {
            bucket: *bucket,
            source,
        })?;
    }
    let ctx = build_ctx(input, meta, rule);
    // The normal buying order first. Bare-minimum mode reorders it when the household asks, or
    // when the normal plan would run past three years (REVIEW R6): the smallest three-day kit
    // comes first and everything else follows in the usual order.
    let normal = plan_once(
        &ctx,
        Mode {
            minimum_first: false,
        },
    );
    let future_money = ctx.monthly > EPS;
    let full_stop = normal.stopped.filter(|_| future_money);
    // The automatic switch belongs to the default split schedule: under the fixed-order schedule
    // (the monotone reference) and the research one, only the household's own dial reorders.
    let too_long = ctx.split
        && future_money
        && match normal.stopped {
            Some(m) => m > PLAN_TOO_LONG_MONTHS,
            None => input.options.max_months >= PLAN_TOO_LONG_MONTHS,
        };
    let asked = input.household.dials.minimum_kit;
    let outcome = if asked || too_long {
        plan_once(
            &ctx,
            Mode {
                minimum_first: true,
            },
        )
    } else {
        normal
    };
    Ok(finish(&ctx, outcome, too_long, full_stop))
}

/// One pass of the allocator over the whole plan horizon.
fn plan_once(ctx: &Ctx<'_>, mode: Mode) -> Outcome {
    let input = ctx.input;
    let mut state = State {
        inventory: Vec::new(),
        track_cov: vec![0.0; ctx.tracks.len()],
        owned: vec![0.0; ctx.offers.len()],
        readiness_used: vec![false; ctx.offers.len()],
    };
    for t in 0..ctx.tracks.len() {
        state.track_cov[t] = track_coverage(ctx, t, &state.inventory);
    }

    // ---- Month 0: what the household has, then free actions. ----
    let mut month0_owned: Vec<Event> = Vec::new();
    let mut month0_done_free: Vec<Event> = Vec::new();
    let mut month0_todo_free: Vec<Event> = Vec::new();
    let existing = existing_by_offer(ctx);
    for (i, offer) in ctx.offers.iter().enumerate() {
        let Some((qty, paid)) = existing.get(&i).copied() else {
            continue;
        };
        if offer.item.free {
            continue;
        }
        let cand = evaluate_fixed(ctx, &state, i, qty, TierId::Y1, f64::INFINITY);
        apply(ctx, &mut state, &cand);
        month0_owned.push(Event::Owned { cand, paid });
    }
    let free: Vec<usize> = (0..ctx.offers.len())
        .filter(|&i| ctx.offers[i].item.free)
        .collect();
    for &i in &free {
        let done = existing.get(&i).is_some_and(|(q, _)| *q >= 1.0);
        if done {
            let qty = ctx.offers[i].set_quantity.max(1.0);
            let cand = evaluate_fixed(ctx, &state, i, qty, TierId::Now, f64::INFINITY);
            apply(ctx, &mut state, &cand);
            month0_done_free.push(Event::Free { cand, done: true });
        }
    }
    let so_far = state.clone();
    let free_order = order_free_steps(ctx, &mut state, &free, &existing, mode);
    // Coverage with what the household has and every free step done, before any purchase (the
    // stored-water guardrail asks whether free steps leave the water short).
    let after_free = state.track_cov.clone();
    // Free actions are spread over the first months (at most eight ordinary ones to do in any
    // month; decisions and the other exempt steps by month 1). The allocator values purchases as
    // if all of them were done, since they cost nothing and all come within a few months, so it
    // never buys what a scheduled free step will cover; the plan's coverage numbers (`credited`)
    // count each one only from its month.
    let exempt: Vec<bool> = free_order
        .iter()
        .map(|(i, _)| ctx.offers[*i].exempt)
        .collect();
    let free_months = schedule_free(&exempt);
    let last_free_month = free_months.iter().copied().max().unwrap_or(0);
    let scheduled_free: Vec<(u16, usize, f64)> = free_months
        .iter()
        .zip(&free_order)
        .map(|(m, (i, q))| (*m, *i, *q))
        .collect();
    let mut credited = so_far.clone();
    credit_free_actions(
        ctx,
        &mut credited,
        &scheduled_free,
        0,
        &mut month0_todo_free,
    );

    // ---- Rare-catastrophe allowance. ----
    let finances = &input.household.finances;
    let monthly = ctx.monthly;
    let one_off = f64::from(finances.one_off_budget_usd).max(0.0);
    let (mut rare_queue, mut rare_skipped, rare_families) = rare_queue(ctx, &state, one_off);

    // ---- Month by month. ----
    let mut events: Vec<Vec<Event>> = Vec::new();
    let mut coverage_by_month: Vec<MonthCoverage> = Vec::new();
    let mut money_by_month: Vec<MonthMoney> = Vec::new();
    let mut food_by_month: Vec<Option<f64>> = Vec::new();
    let mut ledger = Ledger::default();
    let mut main = Purse::default();
    let mut rare = Purse::default();
    let mut stopped: Option<u16> = None;
    let schedule = input.options.schedule;
    let future_money = monthly > EPS;
    let mut cache = Cache::new(ctx, mode);
    cache.screen_rarely_needed(ctx, &state);
    let checklist = readiness_checklist(ctx, &cache.low_p_ok);
    // The rare allowance starts once the three-day tier's life-safety items are in hand.
    let mut basics_done = false;
    let mut minimum_done_month: Option<u16> = None;

    for m in 0..=input.options.max_months.max(last_free_month) {
        if m > 0 && !future_money && m > last_free_month {
            break;
        }
        cache.new_month(m);
        let mut month_events: Vec<Event> = Vec::new();
        if m > 0 {
            credit_free_actions(ctx, &mut credited, &scheduled_free, m, &mut month_events);
        }
        if !basics_done {
            basics_done = cache.three_day_life_safety_done(ctx, &state);
        }
        let new = if m == 0 { one_off } else { monthly };
        let rare_share = if rare_queue.is_empty() || !basics_done {
            0.0
        } else {
            RARE_CATASTROPHIC_SHARE
        };
        let rare_new = rare_share * new;
        rare.free += rare_new;
        main.free += new - rare_new;
        // What the main plan receives each month (all of it unless the rare allowance takes 10 %),
        // and what it receives this month (month 0 brings the one-off amount instead).
        let main_monthly = monthly * (1.0 - rare_share);
        let main_new = new - rare_new;

        // Split, month 0: the one-off money goes to life-safety items first.
        let mut one_off_fund = false;
        if let (Schedule::Split { reserve_share }, true, None, 0) =
            (schedule, future_money, stopped, m)
        {
            one_off_fund = one_off_to_life_safety(
                ctx,
                &mut Books {
                    state: &mut state,
                    credited: &mut credited,
                    ledger: &mut ledger,
                    cache: &mut cache,
                },
                &mut main,
                reserve_share,
                main_monthly,
                &mut month_events,
            );
        }
        // Split: at the start of the month, keep the fund's item while it is still worth buying
        // (or pick the top item if it costs more than this month's money and cannot be bought
        // now), then put its share of this month's money into the fund. (A fund the one-off money
        // has just opened already holds this month's share.)
        if let (Schedule::Split { reserve_share }, true, None, false) =
            (schedule, future_money, stopped, one_off_fund)
        {
            if let Some((_, picks)) = cache.ordered(ctx, &state, &credited) {
                let kept = main
                    .target
                    .and_then(|t| picks.iter().find(|p| p.offer == t.offer));
                let target = match kept {
                    Some(p) => Some(FundFor::of(p)),
                    None => {
                        main.drop_target();
                        let top = picks[0];
                        let big =
                            top.cost > main_new + EPS && top.cost > main.free + main.fund + EPS;
                        big.then(|| FundFor::of(&top))
                    }
                };
                if let Some(t) = target {
                    let need = (t.cost - main.fund).max(0.0);
                    let amount = (reserve_share * main_new).min(need);
                    main.deposit(amount, t, SaveHow::Share(reserve_share), &mut month_events);
                }
            }
        }
        let available_usd = main.free;
        let cheapest_usd = if stopped.is_none() {
            let target = main.target.map(|t| t.offer);
            cache
                .ordered(ctx, &state, &credited)
                .and_then(|(_, picks)| {
                    picks
                        .iter()
                        .filter(|p| Some(p.offer) != target)
                        .map(|p| p.cost)
                        .reduce(f64::min)
                })
        } else {
            None
        };

        // Main plan.
        while stopped.is_none() {
            let Some((_, picks)) = cache.ordered(ctx, &state, &credited) else {
                // Nothing is left worth buying, so neither is a fund's item (the last purchase
                // covered its need): the plan lists no envelope for it, and its money is surplus
                // like every later month's.
                main.drop_target();
                stopped = Some(m);
                break;
            };
            let chosen: Option<(Pick, bool)> = if let Schedule::Split { .. } = schedule {
                // The fund's item, as soon as the fund (with any free money) covers it. While a
                // fund is running, or the top item costs more than this month's money, the rest of
                // the money buys the best affordable items in priority order; otherwise the plan
                // keeps to the priority order, so a cheap item never delays a better one. Savings
                // pay only for the fund's item, or for the top item when the fund has no item.
                let kept = main
                    .target
                    .and_then(|t| picks.iter().find(|p| p.offer == t.offer));
                if main.target.is_some() && kept.is_none() {
                    main.drop_target();
                }
                let (free, fund) = (main.free, main.fund);
                let top = picks[0];
                let target = kept.map(|p| p.offer);
                let savings_for = |pos: usize| {
                    if target.is_none() && pos == 0 {
                        fund
                    } else {
                        0.0
                    }
                };
                match kept {
                    Some(p) if p.cost <= free + fund + EPS => Some((*p, true)),
                    // (With no later money coming, the best item that fits is bought too.)
                    _ if !future_money || target.is_some() || top.cost > main_new + EPS => picks
                        .iter()
                        .enumerate()
                        .find(|(pos, p)| {
                            Some(p.offer) != target && p.cost <= free + savings_for(*pos) + EPS
                        })
                        .map(|(pos, p)| (*p, savings_for(pos) > 0.0)),
                    _ => (top.cost <= free + savings_for(0) + EPS).then(|| (top, target.is_none())),
                }
            } else {
                let top = picks[0];
                // In these schedules the fund is always for the top item; if the order changed,
                // the money pays for whatever is now at the top.
                if main.target.is_some_and(|t| t.offer != top.offer) {
                    main.drop_target();
                }
                let saving_for_top = main.target.is_some_and(|t| t.offer == top.offer);
                let (free, fund) = (main.free, main.fund);
                // Savings pay only for the top item; everything else is paid from free money.
                let affordable =
                    |pos: usize, p: &Pick| p.cost <= free + if pos == 0 { fund } else { 0.0 } + EPS;
                let first_affordable = || {
                    picks
                        .iter()
                        .enumerate()
                        .find(|(pos, p)| affordable(*pos, p))
                        .map(|(pos, p)| (*p, pos == 0))
                };
                let chosen = if affordable(0, &top) {
                    Some((top, true))
                } else if !future_money {
                    // No later month brings money: saving is pointless, buy the best that fits.
                    first_affordable()
                } else if schedule == Schedule::ResearchShortcuts && !saving_for_top {
                    first_affordable().filter(|(p, _)| {
                        let wait = top.cost <= SINKING_FUND_MAX_MONTHS * main_monthly + EPS
                            && p.density() < SINKING_FUND_VALUE_RATIO * top.density();
                        !wait
                    })
                } else {
                    None
                };
                // Nothing bought: everything on hand goes into the fund for the top item when it
                // costs more than a month's money (otherwise next month's money buys it).
                if chosen.is_none() && future_money && top.cost > main_monthly + EPS {
                    main.deposit(
                        main.free,
                        FundFor::of(&top),
                        SaveHow::AllMoney,
                        &mut month_events,
                    );
                }
                chosen
            };
            match chosen {
                Some((pick, use_fund)) => {
                    let cand = cache.candidate(&pick);
                    buy(
                        ctx,
                        &mut state,
                        &mut credited,
                        &mut main,
                        use_fund,
                        &mut ledger,
                        &cand,
                        m,
                        false,
                        false,
                        pick.minimum,
                        &mut month_events,
                    );
                    cache.invalidate(ctx, pick.offer);
                }
                None => break,
            }
        }

        // Rare-catastrophe allowance: its own queue, in order, where an item waits for one of
        // its devices (and is dropped once the main plan is done and the device never came).
        loop {
            if stopped.is_some() {
                let before = rare_queue.len();
                let dropped: Vec<ItemId> = rare_queue
                    .iter()
                    .filter(|c| !ctx.requires_met(&credited, c.offer))
                    .map(|c| ctx.offers[c.offer].item.id.clone())
                    .collect();
                rare_queue.retain(|c| ctx.requires_met(&credited, c.offer));
                if rare_queue.len() < before {
                    rare_skipped.extend(dropped);
                }
            }
            let Some(pos) = rare_queue
                .iter()
                .rposition(|c| ctx.requires_met(&credited, c.offer))
            else {
                break;
            };
            let next = rare_queue[pos].clone();
            let (free, fund) = (rare.free, rare.fund);
            if next.cost <= free + fund + EPS {
                let cand = rare_queue.remove(pos);
                buy(
                    ctx,
                    &mut state,
                    &mut credited,
                    &mut rare,
                    true,
                    &mut ledger,
                    &cand,
                    m,
                    true,
                    false,
                    false,
                    &mut month_events,
                );
                continue;
            }
            if !future_money {
                let fits = rare_queue
                    .iter()
                    .rposition(|c| c.cost <= free + EPS && ctx.requires_met(&credited, c.offer));
                if let Some(p) = fits {
                    let cand = rare_queue.remove(p);
                    buy(
                        ctx,
                        &mut state,
                        &mut credited,
                        &mut rare,
                        false,
                        &mut ledger,
                        &cand,
                        m,
                        true,
                        false,
                        false,
                        &mut month_events,
                    );
                    continue;
                }
            } else if next.cost > RARE_CATASTROPHIC_SHARE * monthly + EPS {
                let target = FundFor {
                    offer: next.offer,
                    cost: next.cost,
                    tier: next.tier,
                };
                rare.deposit(rare.free, target, SaveHow::Rare, &mut month_events);
            }
            break;
        }
        if rare_queue.is_empty() && rare.free + rare.fund > 0.0 {
            main.free += rare.free + rare.fund;
            rare = Purse::default();
        }

        // Coverage only changes when something is bought or a free action is done.
        let bought = month_events
            .iter()
            .any(|e| matches!(e, Event::Buy { .. } | Event::Free { .. }));
        let snap = match coverage_by_month.last() {
            Some(prev) if !bought && m > 0 => MonthCoverage {
                month: m,
                ..prev.clone()
            },
            _ => snapshot(ctx, &credited, &checklist, m),
        };
        coverage_by_month.push(snap);
        food_by_month.push(food_days(ctx, &credited));
        if minimum_done_month.is_none() && ctx.minimum_met(&credited) {
            minimum_done_month = Some(m);
        }
        money_by_month.push(MonthMoney {
            month: m,
            available_usd,
            saved_usd: main.fund,
            saving_for: main
                .target
                .filter(|_| main.fund > EPS)
                .map(|t| (ctx.offers[t.offer].item.id.clone(), t.cost)),
            cheapest_usd,
        });
        events.push(month_events);
        if stopped.is_some() && rare_queue.is_empty() && m >= last_free_month {
            break;
        }
    }
    // Funds still open when the plan ends are reported with what they hold.
    for purse in [&main, &rare] {
        if let (Some(t), true) = (purse.target, purse.fund > EPS) {
            ledger.add_envelope(&ctx.offers[t.offer].item.id, purse.fund, t.cost);
        }
    }
    // What the plan still wanted when the horizon ran out.
    let left_over: Vec<usize> = if stopped.is_none() {
        cache.worth_buying(ctx, &state)
    } else {
        Vec::new()
    };
    let (sequence, envelopes, purchase_months) = ledger.into_envelopes();
    Outcome {
        events,
        coverage_by_month,
        money_by_month,
        food_by_month,
        sequence,
        envelopes,
        purchase_months,
        month0_todo_free,
        month0_done_free,
        month0_owned,
        state,
        so_far,
        credited,
        after_free,
        scheduled_free,
        stopped,
        main_free: main.free,
        main_fund: main.fund,
        rare_skipped,
        rare_families,
        checklist,
        minimum_done_month,
        left_over,
        minimum_first: mode.minimum_first,
    }
}

/// The free steps still to do, in the order they are scheduled: the fixed first steps, then
/// life-safety steps, then (in bare-minimum mode) the kit's free steps such as the cooling plan,
/// then capabilities, then the rest by value (each valued after the ones before it), and last the
/// steps that wait for a spare slot. Applies each to `state`.
fn order_free_steps(
    ctx: &Ctx<'_>,
    state: &mut State,
    free: &[usize],
    existing: &BTreeMap<usize, (f64, Option<f64>)>,
    mode: Mode,
) -> Vec<(usize, f64)> {
    let mut free_order: Vec<(usize, f64)> = Vec::new();
    let mut todo: Vec<usize> = free
        .iter()
        .copied()
        .filter(|i| !existing.get(i).is_some_and(|(q, _)| *q >= 1.0))
        .collect();
    // The fixed first steps (alerts, the household plan, fire safety), in their order.
    for id in FIRST_FREE_STEPS {
        if let Some(pos) = todo.iter().position(|&i| ctx.offers[i].item.id == id) {
            let i = todo.remove(pos);
            let qty = ctx.offers[i].set_quantity.max(1.0);
            let cand = evaluate_fixed(ctx, state, i, qty, TierId::Now, f64::INFINITY);
            apply(ctx, state, &cand);
            free_order.push((cand.offer, cand.qty));
        }
    }
    // Steps that wait for a spare slot go last, in their order.
    let last: Vec<usize> = LAST_FREE_STEPS
        .iter()
        .filter_map(|id| todo.iter().copied().find(|&i| ctx.offers[i].item.id == *id))
        .collect();
    todo.retain(|i| !last.contains(i));
    // Life-safety first, then the kit's steps (bare-minimum mode), then capabilities, then value,
    // so each step's value is its marginal value.
    let rank = |i: usize| {
        let o = &ctx.offers[i];
        (
            o.item.life_safety,
            mode.minimum_first && !o.minimum.is_empty(),
            o.capability,
        )
    };
    while !todo.is_empty() {
        let mut best: Option<(usize, Candidate)> = None;
        for (pos, &i) in todo.iter().enumerate() {
            let qty = ctx.offers[i].set_quantity.max(1.0);
            let c = evaluate_fixed(ctx, state, i, qty, TierId::Now, f64::INFINITY);
            let better = match &best {
                None => true,
                Some((_, b)) => {
                    let (ri, rb) = (rank(i), rank(b.offer));
                    ri > rb || (ri == rb && c.value > b.value + VALUE_EPS)
                }
            };
            if better {
                best = Some((pos, c));
            }
        }
        let (pos, cand) = best.expect("todo is not empty");
        todo.remove(pos);
        apply(ctx, state, &cand);
        free_order.push((cand.offer, cand.qty));
    }
    for i in last {
        let qty = ctx.offers[i].set_quantity.max(1.0);
        let cand = evaluate_fixed(ctx, state, i, qty, TierId::Now, f64::INFINITY);
        apply(ctx, state, &cand);
        free_order.push((cand.offer, cand.qty));
    }
    free_order
}

/// The rare allowance's queue (popped from the back, best value per dollar last), the specialised
/// items it leaves out, and each queued item's ticked, likely-enough families ([`crate::rare`]).
fn rare_queue(
    ctx: &Ctx<'_>,
    state: &State,
    one_off: f64,
) -> (Vec<Candidate>, Vec<ItemId>, BTreeMap<usize, RareFor>) {
    let risks = ctx.input.risks;
    let mut queue: Vec<Candidate> = Vec::new();
    let mut skipped: Vec<ItemId> = Vec::new();
    let mut families: BTreeMap<usize, RareFor> = BTreeMap::new();
    for (i, o) in ctx.offers.iter().enumerate() {
        if o.item.free || !o.item.rare_catastrophic {
            continue;
        }
        let remaining = remaining_set_qty(ctx, state, i);
        if remaining <= EPS {
            continue;
        }
        // The local ten-year chance behind the item in family `f`: the family's, or, for an item
        // that protects against one cause inside it, that cause's (0 when the family does not list
        // the cause), with the words that name the cause in its sentence.
        let cause = rare::cause_of(o.item.id.as_str());
        let chance_in = |f: &HazardId| -> (f64, Option<String>) {
            match cause {
                None => (
                    rare::p10_from_rate(risks.register.get(f).copied().unwrap_or(0.0)),
                    None,
                ),
                Some((id, words)) => (
                    risks
                        .sub_causes
                        .get(f)
                        .and_then(|subs| subs.iter().find(|s| s.id == id))
                        .map_or(0.0, |s| rare::p10_from_rate(rare::sub_cause_rate(s))),
                    Some(words.to_owned()),
                ),
            }
        };
        let eligible: Vec<(HazardId, f64, Option<String>)> = o
            .families
            .iter()
            .copied()
            .filter(|f| ctx.families_on.contains(f))
            .map(|f| {
                let (p, name) = chance_in(&f);
                (f, p, name)
            })
            .filter(|(_, p, _)| *p >= rare::RARE_MIN_P10)
            .collect();
        if eligible.is_empty() {
            skipped.push(o.item.id.clone());
            continue;
        }
        // V = the families' (or the cause's) local ten-year chance × the harm-days the item
        // avoids.
        let chance: f64 = eligible.iter().map(|(_, p, _)| *p).sum();
        let value = chance.min(1.0) * rare::harm_days(o.item.id.as_str());
        queue.push(Candidate {
            offer: i,
            qty: remaining,
            cost: remaining * o.unit_price,
            tier: TierId::Y1,
            promoted: false,
            core: value,
            low_p: 0.0,
            value,
            gains: Vec::new(),
            ready: Vec::new(),
        });
        families.insert(
            i,
            RareFor {
                families: eligible.iter().map(|(f, _, _)| *f).collect(),
                cause: eligible.iter().find_map(|(_, _, name)| name.clone()),
            },
        );
    }
    queue.sort_by(|a, b| {
        b.density()
            .total_cmp(&a.density())
            .then(a.cost.total_cmp(&b.cost))
            .then(a.offer.cmp(&b.offer))
    });
    // No family takes more than half of the allowance over the plan horizon.
    let total =
        RARE_CATASTROPHIC_SHARE * (one_off + ctx.monthly * f64::from(ctx.input.options.max_months));
    let cap = rare::RARE_FAMILY_MAX_SHARE * total;
    let mut spent: BTreeMap<HazardId, f64> = BTreeMap::new();
    queue.retain(|c| {
        let fams = &families[&c.offer].families;
        let fits = fams
            .iter()
            .all(|f| spent.get(f).copied().unwrap_or(0.0) + c.cost <= cap + EPS);
        if fits {
            for f in fams {
                *spent.entry(*f).or_insert(0.0) += c.cost;
            }
        } else {
            skipped.push(ctx.offers[c.offer].item.id.clone());
        }
        fits
    });
    families.retain(|i, _| queue.iter().any(|c| c.offer == *i));
    queue.reverse(); // pop from the back
    (queue, skipped, families)
}

/// Days of food the household has in `state`: the supplies bucket's food part, or its weakest
/// part when no track is named food. `None` without a supplies track.
fn food_days(ctx: &Ctx<'_>, state: &State) -> Option<f64> {
    let supplies: Vec<usize> = (0..ctx.tracks.len())
        .filter(|&t| ctx.tracks[t].bucket == BucketId::Supplies)
        .collect();
    if let Some(&t) = supplies
        .iter()
        .find(|&&t| ctx.tracks[t].part.as_deref() == Some("food"))
    {
        return Some(state.track_cov[t]);
    }
    supplies
        .iter()
        .map(|&t| state.track_cov[t])
        .reduce(f64::min)
}

/// Assembles the plan, the coverage and the warnings from the pass the plan uses.
fn finish(ctx: &Ctx<'_>, out: Outcome, too_long: bool, full_stop: Option<u16>) -> BudgetResult {
    let input = ctx.input;
    let Outcome {
        mut events,
        mut coverage_by_month,
        mut money_by_month,
        food_by_month,
        sequence,
        envelopes,
        purchase_months,
        month0_todo_free,
        month0_done_free,
        month0_owned,
        state,
        so_far,
        credited,
        after_free,
        scheduled_free,
        stopped,
        main_free,
        main_fund,
        rare_skipped,
        rare_families,
        checklist,
        minimum_done_month,
        left_over,
        minimum_first,
    } = out;
    let monthly = ctx.monthly;
    let one_off = f64::from(input.household.finances.one_off_budget_usd).max(0.0);

    // ---- Assemble the plan. ----
    let mut first = Vec::new();
    first.extend(month0_todo_free);
    first.extend(month0_done_free);
    first.extend(month0_owned);
    if events.is_empty() {
        events.push(Vec::new());
        coverage_by_month.push(snapshot(ctx, &credited, &checklist, 0));
        money_by_month.push(MonthMoney {
            month: 0,
            available_usd: main_free,
            saved_usd: main_fund,
            saving_for: None,
            cheapest_usd: None,
        });
    }
    let mut month0 = first;
    month0.append(&mut events[0]);
    events[0] = month0;
    while events.len() > 1 && events.last().is_some_and(|e| e.is_empty()) {
        events.pop();
        coverage_by_month.pop();
        money_by_month.pop();
    }
    let rare_monthly = RARE_CATASTROPHIC_SHARE * monthly;
    let months: Vec<PlanMonth> = events
        .iter()
        .enumerate()
        .map(|(m, evs)| PlanMonth {
            index: m as u16,
            budget_usd: money(if m == 0 { one_off } else { monthly }),
            items: plan_items(ctx, evs, &rare_families, rare_monthly),
        })
        .collect();

    let all_covered = ctx
        .tracks
        .iter()
        .enumerate()
        .all(|(t, tr)| state.track_cov[t] + 1e-6 >= tr.target);
    let done_month = stopped.filter(|_| all_covered);
    let savings_track = savings::track(input.household, input.risks, stopped);
    let first_milestone = savings::first_milestone(input.household, input.risks, stopped);
    let long_horizon = long_horizon_section(ctx, &months);
    let plan = Plan {
        months,
        done_month,
        minimum_done_month,
        envelopes,
        savings_track,
        first_milestone,
        minimum_kit: minimum_first,
        long_horizon,
    };

    let covered = covered_targets(ctx, &state, &checklist);
    let covered_today = covered_targets(ctx, &so_far, &checklist);
    let free_month_of: BTreeMap<usize, u16> =
        scheduled_free.iter().map(|&(m, i, _)| (i, m)).collect();
    let mut facts = guardrail_facts(
        ctx,
        &state,
        &after_free,
        &purchase_months,
        &free_month_of,
        stopped,
        &food_by_month,
    );
    if too_long {
        // What falls beyond three years even in bare-minimum mode: purchases after month 36, and
        // what the plan still wanted when the horizon ran out.
        let mut deferred: Vec<usize> = Vec::new();
        for p in sequence
            .iter()
            .filter(|p| p.month > PLAN_TOO_LONG_MONTHS && !p.rare_catastrophic)
        {
            if let Some(i) = ctx.offers.iter().position(|o| o.item.id == p.item_id) {
                if !deferred.contains(&i) {
                    deferred.push(i);
                }
            }
        }
        for i in left_over {
            if !deferred.contains(&i) {
                deferred.push(i);
            }
        }
        facts.too_long = Some(TooLong {
            full_plan_month: full_stop,
            minimum_month: minimum_done_month,
            deferred: deferred
                .iter()
                .map(|&i| {
                    (
                        ctx.offers[i].item.id.clone(),
                        lower_first(&ctx.offers[i].item.name),
                    )
                })
                .collect(),
        });
    }
    let warnings = guardrails::check(input.household, input.context, input.risks, &facts);

    BudgetResult {
        plan,
        covered,
        covered_today,
        coverage_by_month,
        money_by_month,
        sequence,
        tier_reached: tier_met(ctx, &so_far.track_cov),
        tier_at_plan_end: tier_met(ctx, &state.track_cov),
        tier_recommended: tier_recommended(ctx),
        warnings,
        rare_catastrophic_skipped: rare_skipped,
        stopped_month: stopped,
        minimum_done_month,
        full_plan_stopped_month: full_stop,
        citations: savings::citations(input.household, input.risks),
    }
}

/// Whether the long-horizon section applies: some duration target reaches
/// [`LONG_HORIZON_MIN_DAYS`], or the household asked for it (`Dials::long_horizon`).
fn long_horizon_applies(ctx: &Ctx<'_>) -> bool {
    let risks = ctx.input.risks;
    ctx.input.household.dials.long_horizon
        || risks
            .curves
            .values()
            .any(|c| c.target_days + 1e-9 >= LONG_HORIZON_MIN_DAYS)
        || risks.assessments.values().any(|a| {
            matches!(a.target, Target::Days { value, .. }
                if f64::from(value) + 1e-9 >= LONG_HORIZON_MIN_DAYS)
        })
}

/// The long-horizon section (`Plan::long_horizon`): every long-horizon item the plan lists, one
/// line each with its quantities added up over the months, when the section applies. The items
/// stay in the months too: the flag only groups them.
fn long_horizon_section(ctx: &Ctx<'_>, months: &[PlanMonth]) -> Vec<PlanItem> {
    if !long_horizon_applies(ctx) {
        return Vec::new();
    }
    let flagged = |id: &ItemId| {
        ctx.offers
            .iter()
            .any(|o| o.item.id == *id && o.item.long_horizon)
    };
    let mut out: Vec<PlanItem> = Vec::new();
    for m in months {
        for it in m.items.iter().filter(|i| i.kind != PlanItemKind::Reserve) {
            if !flagged(&it.item_id) {
                continue;
            }
            match out.iter_mut().find(|x| x.item_id == it.item_id) {
                Some(x) => {
                    x.quantity = ((f64::from(x.quantity) + f64::from(it.quantity)) * 1000.0).round()
                        as f32
                        / 1000.0;
                    x.est_cost_usd = money(f64::from(x.est_cost_usd) + f64::from(it.est_cost_usd));
                    x.price_band.low =
                        money(f64::from(x.price_band.low) + f64::from(it.price_band.low));
                    x.price_band.high =
                        money(f64::from(x.price_band.high) + f64::from(it.price_band.high));
                    x.risk_reduction += it.risk_reduction;
                    x.done &= it.done;
                }
                None => out.push(it.clone()),
            }
        }
    }
    out
}

fn build_ctx<'a>(
    input: &'a BudgetInput<'a>,
    meta: &'a [ItemMeta],
    rule: &'a dyn CoverageRule,
) -> Ctx<'a> {
    let household = input.household;
    let mut weights = BTreeMap::new();
    for b in BucketId::ALL {
        weights.insert(*b, harm_weight(*b, household).0);
    }
    // Tracks: one per duration bucket with a curve, or one per part.
    let mut tracks: Vec<Track> = Vec::new();
    for (bucket, curve) in &input.risks.curves {
        let parts = rule.parts(*bucket);
        let weight = weights[bucket];
        if parts.is_empty() {
            tracks.push(Track {
                bucket: *bucket,
                part: None,
                share: 1.0,
                weight,
                target: curve.target_days,
            });
            continue;
        }
        let listed: f64 = parts.iter().filter_map(|p| curve.part_shares.get(p)).sum();
        let unlisted = parts
            .iter()
            .filter(|p| !curve.part_shares.contains_key(*p))
            .count();
        let rest = if curve.part_shares.is_empty() {
            1.0
        } else {
            (1.0 - listed).max(0.0)
        };
        for p in parts {
            let share = curve
                .part_shares
                .get(&p)
                .copied()
                .unwrap_or(rest / unlisted.max(1) as f64);
            tracks.push(Track {
                bucket: *bucket,
                part: Some(p),
                share,
                weight,
                target: curve.target_days,
            });
        }
    }
    let meta_by_id: BTreeMap<&ItemId, &ItemMeta> = meta.iter().map(|m| (&m.item_id, m)).collect();
    let recorded = recorded_prices(input);
    let mut seen: BTreeSet<&ItemId> = BTreeSet::new();
    let mut offers = Vec::new();
    for item in input.catalogue {
        if !seen.insert(&item.id) {
            continue; // first entry wins for a duplicated id
        }
        let m = meta_by_id.get(&item.id).copied();
        let mut buckets: Vec<BucketId> = item.buckets.clone();
        if let Some(m) = m {
            for c in &m.contributes {
                if !buckets.contains(&c.bucket) {
                    buckets.push(c.bucket);
                }
            }
        }
        let track_ids: Vec<usize> = buckets
            .iter()
            .flat_map(|b| {
                tracks
                    .iter()
                    .enumerate()
                    .filter(move |(_, t)| t.bucket == *b)
                    .map(|(i, _)| i)
            })
            .collect();
        let midpoint =
            0.5 * (f64::from(item.price_band_usd.low) + f64::from(item.price_band_usd.high));
        let unit_price = if item.free {
            0.0
        } else {
            recorded.get(&item.id).copied().unwrap_or(midpoint).max(0.0)
        };
        let readiness: Vec<(BucketId, f64)> = m
            .map(|m| {
                m.readiness
                    .iter()
                    .map(|r| (r.bucket, r.harm_day_equivalents))
                    .collect()
            })
            .unwrap_or_default();
        let share = item
            .readiness_share
            .map_or(1.0, |s| f64::from(s).clamp(0.0, 1.0));
        let capability =
            item.readiness_share.is_some_and(|s| s >= 1.0 - 1e-6) && !readiness.is_empty();
        let exempt = item.free
            && (item.decision
                || item.long_horizon
                || EXEMPT_FREE_STEPS.contains(&item.id.as_str()));
        offers.push(Offer {
            item,
            unit_price,
            set_quantity: m.and_then(|m| m.set_quantity).unwrap_or(1.0),
            step: m.and_then(|m| m.step),
            tracks: track_ids,
            readiness,
            roles: m.map(|m| m.roles.clone()).unwrap_or_default(),
            share,
            capability,
            has_requires: !item.requires.is_empty(),
            requires: Vec::new(),
            requires_owned: false,
            group: Vec::new(),
            minimum: Vec::new(),
            families: rare::families_of(&item.hazard_extras),
            exempt,
        });
    }
    // Prerequisites, alternative groups and the bare-minimum kit need every offer's index.
    let index: BTreeMap<ItemId, usize> = offers
        .iter()
        .enumerate()
        .map(|(i, o)| (o.item.id.clone(), i))
        .collect();
    let owned_ids: BTreeSet<&ItemId> = household
        .existing
        .iter()
        .filter(|o| o.qty.is_finite() && o.qty > 0.0)
        .map(|o| &o.item_id)
        .collect();
    for i in 0..offers.len() {
        let item = offers[i].item;
        let requires: Vec<usize> = item
            .requires
            .iter()
            .filter_map(|id| index.get(id).copied())
            .filter(|&j| j != i)
            .collect();
        // Met outside the catalogue: the household owns a listed device that is not offered, or
        // none of the listed devices is offered at all, because the household's own equipment
        // (a generator it already has) is why the accessory is offered.
        let requires_owned = requires.is_empty()
            || item
                .requires
                .iter()
                .any(|id| owned_ids.contains(id) && !index.contains_key(id));
        let group: Vec<usize> = match &item.alternative_group {
            Some(g) => (0..offers.len())
                .filter(|&j| j != i && offers[j].item.alternative_group.as_ref() == Some(g))
                .collect(),
            None => Vec::new(),
        };
        offers[i].requires = requires;
        offers[i].requires_owned = requires_owned;
        offers[i].group = group;
    }
    // The bare-minimum kit's lines (a line that needs nothing is met already), and each offer's
    // part in them.
    let mut min_lines: Vec<MinLine> = Vec::new();
    for (i, o) in offers.iter().enumerate() {
        let Some(m) = meta_by_id.get(&o.item.id) else {
            continue;
        };
        for share in m.minimum.iter().filter(|s| s.need > 1e-9) {
            let l = match min_lines.iter().position(|l| l.id == share.line) {
                Some(l) => l,
                None => {
                    min_lines.push(MinLine {
                        id: share.line.clone(),
                        need: 0.0,
                        contrib: Vec::new(),
                    });
                    min_lines.len() - 1
                }
            };
            min_lines[l].need = min_lines[l].need.max(share.need);
            min_lines[l].contrib.push((i, share.units_per_item));
        }
    }
    for (l, line) in min_lines.iter().enumerate() {
        for &(i, u) in &line.contrib {
            offers[i].minimum.push((l, u));
        }
    }
    // The families the allowance may buy for: the ones the household ticked, or all of them.
    let mut families_on: BTreeSet<HazardId> = household
        .dials
        .rare_families()
        .into_iter()
        .filter_map(HazardId::from_family)
        .collect();
    if input.options.rare_catastrophic_opt_in {
        families_on.extend(HazardId::RARE.iter().copied());
    }
    Ctx {
        input,
        rule,
        curves: input
            .risks
            .curves
            .iter()
            .map(|(b, c)| (*b, c.prepared()))
            .collect(),
        offers,
        tracks,
        weights,
        people: household.people.len().max(1),
        years: household.dials.horizon_years.max(1),
        min_lines,
        families_on,
        planning: household.planning_date,
        monthly: f64::from(household.finances.monthly_budget_usd).max(0.0),
        split: matches!(input.options.schedule, Schedule::Split { .. }),
    }
}

/// Unit prices the household recorded (`paid_usd / qty`), per item.
fn recorded_prices(input: &BudgetInput<'_>) -> BTreeMap<ItemId, f64> {
    let mut totals: BTreeMap<ItemId, (f64, f64)> = BTreeMap::new();
    for o in &input.household.existing {
        if let Some(paid) = o.paid_usd {
            if o.qty > 0.0 && paid.is_finite() && paid >= 0.0 {
                let e = totals.entry(o.item_id.clone()).or_insert((0.0, 0.0));
                e.0 += f64::from(paid);
                e.1 += f64::from(o.qty);
            }
        }
    }
    totals
        .into_iter()
        .map(|(id, (paid, qty))| (id, paid / qty))
        .collect()
}

/// `existing` summed per offer: (quantity, total paid if any was recorded).
fn existing_by_offer(ctx: &Ctx<'_>) -> BTreeMap<usize, (f64, Option<f64>)> {
    let mut out: BTreeMap<usize, (f64, Option<f64>)> = BTreeMap::new();
    for o in &ctx.input.household.existing {
        if !(o.qty.is_finite() && o.qty > 0.0) {
            continue;
        }
        let Some(i) = ctx.offers.iter().position(|of| of.item.id == o.item_id) else {
            continue;
        };
        let e = out.entry(i).or_insert((0.0, None));
        e.0 += f64::from(o.qty);
        if let Some(p) = o.paid_usd {
            e.1 = Some(e.1.unwrap_or(0.0) + f64::from(p));
        }
    }
    out
}

fn track_coverage(ctx: &Ctx<'_>, t: usize, inventory: &[(ItemId, f64)]) -> f64 {
    let tr = &ctx.tracks[t];
    let h = ctx.input.household;
    match &tr.part {
        Some(p) => ctx.rule.part_coverage(tr.bucket, p, inventory, h),
        None => ctx.rule.coverage(tr.bucket, inventory, h),
    }
}

fn curve<'c>(ctx: &'c Ctx<'_>, bucket: BucketId) -> &'c Prepared<'c> {
    &ctx.curves[&bucket]
}

fn remaining_set_qty(ctx: &Ctx<'_>, state: &State, i: usize) -> f64 {
    let o = &ctx.offers[i];
    let rem = o.set_quantity - state.owned[i];
    if o.set_quantity <= 0.0 {
        0.0
    } else {
        rem.max(0.0)
    }
}

/// The next value on the day ladder strictly above `x`.
fn next_ladder_above(x: f64) -> f64 {
    TARGET_LADDER_DAYS
        .iter()
        .map(|&d| f64::from(d))
        .find(|&d| d > x + EPS)
        .unwrap_or(f64::INFINITY)
}

/// The largest ladder value at or below `x` (at least one day), for "for d or more" sentences.
fn ladder_floor_days(x: f64) -> f64 {
    TARGET_LADDER_DAYS
        .iter()
        .rev()
        .map(|&d| f64::from(d))
        .find(|&d| d <= x + EPS)
        .unwrap_or(1.0)
        .max(1.0)
}

/// Quantity of the next chunk of divisible item `i` for a tier with this horizon: enough to reach
/// the next ladder step (or the cap) in the first track that still has room.
fn chunk_qty(ctx: &Ctx<'_>, state: &State, i: usize, horizon: f64) -> Option<f64> {
    let o = &ctx.offers[i];
    let step = o.step?;
    let h = ctx.input.household;
    for &t in &o.tracks {
        let tr = &ctx.tracks[t];
        let cap = tr.target.min(horizon);
        let x0 = state.track_cov[t];
        if x0 + EPS >= cap {
            continue;
        }
        let per_step = ctx.rule.gain(
            tr.bucket,
            tr.part.as_deref(),
            &state.inventory,
            &o.item.id,
            step,
            h,
        );
        if per_step <= 0.0 {
            continue;
        }
        let next = next_ladder_above(x0).min(cap);
        let steps = ((next - x0) / per_step - EPS).ceil().max(1.0);
        return Some(steps * step);
    }
    None
}

/// Values buying `qty` of offer `i` now, with targets capped at `horizon` days.
fn evaluate_fixed(
    ctx: &Ctx<'_>,
    state: &State,
    i: usize,
    qty: f64,
    tier: TierId,
    horizon: f64,
) -> Candidate {
    let o = &ctx.offers[i];
    let h = ctx.input.household;
    let mut gains = Vec::new();
    let mut duration = 0.0;
    for &t in &o.tracks {
        let tr = &ctx.tracks[t];
        let dx = ctx.rule.gain(
            tr.bucket,
            tr.part.as_deref(),
            &state.inventory,
            &o.item.id,
            qty,
            h,
        );
        if dx <= 0.0 {
            continue;
        }
        let x0 = state.track_cov[t];
        let cap = tr.target.min(horizon);
        let prepared = curve(ctx, tr.bucket);
        let value = duration_value_with(tr.weight, tr.share, x0, x0 + dx, cap, |a, b| {
            prepared.integral(a, b)
        });
        duration += value;
        gains.push(Gain {
            track: t,
            x0,
            x1: x0 + dx,
            value,
        });
    }
    let mut core = duration;
    let mut low_p = 0.0;
    let mut ready = Vec::new();
    // Readiness value counts once per item, times its share, and once per alternative group.
    if !state.readiness_used[i] && !ctx.group_taken(state, i) {
        for &(b, harm) in &o.readiness {
            let p = readiness_p10(ctx, i, b);
            let v = readiness_value(ctx.weights[&b], p, harm) * o.share;
            let low = p < READINESS_MIN_P_NEED_10YR;
            if low {
                low_p += v;
            } else {
                core += v;
            }
            ready.push((b, v, low));
        }
    }
    Candidate {
        offer: i,
        qty,
        cost: qty * o.unit_price,
        tier,
        promoted: false,
        core,
        low_p,
        value: core + low_p,
        gains,
        ready,
    }
}

/// Values the next purchase of offer `i` against tier `tier`'s caps, or `None` when there is
/// nothing to buy.
fn evaluate(ctx: &Ctx<'_>, state: &State, i: usize, tier: TierId) -> Option<Candidate> {
    let horizon = f64::from(tier.days());
    let qty = match ctx.offers[i].step {
        None => {
            let rem = remaining_set_qty(ctx, state, i);
            if rem <= EPS {
                return None;
            }
            rem
        }
        Some(step) => match chunk_qty(ctx, state, i, horizon) {
            Some(q) => q,
            // Its buckets have no room left (other items covered them), but its readiness credit
            // has not been counted: one step still buys that.
            None if !state.readiness_used[i] && !ctx.offers[i].readiness.is_empty() => step,
            None => return None,
        },
    };
    let mut c = evaluate_fixed(ctx, state, i, qty, tier, horizon);
    c.value = c.core;
    Some(c)
}

/// Valuations per offer and tier, kept until a purchase changes something they depend on: the
/// offer's own purchase, a purchase touching one of the offer's tracks, or a purchase of another
/// member of its alternative group. (This is why a [`CoverageRule`]'s answer for a bucket may
/// depend only on items that serve that bucket.)
struct Cache {
    slots: Vec<[Option<Option<Candidate>>; WALK.len()]>,
    /// Offers touching each track.
    by_track: Vec<Vec<usize>>,
    /// Offers the main plan can buy (not free, not rare-catastrophe).
    main: Vec<usize>,
    /// Per offer and tier, a number that changes only where the offer's capped targets change: an
    /// offer valued at two tiers with the same number has the same valuation at both.
    cap_group: Vec<[u8; WALK.len()]>,
    /// The buying order for the current state, until the next purchase or the next month.
    ordered: Option<Option<(TierId, Vec<Pick>)>>,
    /// Per offer: a capability needed less often than the threshold counts in its value
    /// (screened once, against the household's position before any purchase; see
    /// [`Cache::screen_rarely_needed`]).
    low_p_ok: Vec<bool>,
    /// The bare-minimum kit's candidates behind the current buying order (bare-minimum mode).
    min_cands: Vec<Candidate>,
    /// The plan month, for the season rule.
    month: u16,
    mode: Mode,
}

impl Cache {
    fn new(ctx: &Ctx<'_>, mode: Mode) -> Self {
        let mut by_track = vec![Vec::new(); ctx.tracks.len()];
        for (i, o) in ctx.offers.iter().enumerate() {
            for &t in &o.tracks {
                by_track[t].push(i);
            }
        }
        let cap_group = ctx
            .offers
            .iter()
            .map(|o| {
                let caps = |ti: usize| -> Vec<f64> {
                    let h = f64::from(WALK[ti].days());
                    o.tracks
                        .iter()
                        .map(|&t| ctx.tracks[t].target.min(h))
                        .collect()
                };
                let mut groups = [0u8; WALK.len()];
                for ti in 1..WALK.len() {
                    groups[ti] = groups[ti - 1] + u8::from(caps(ti) != caps(ti - 1));
                }
                groups
            })
            .collect();
        Cache {
            slots: (0..ctx.offers.len())
                .map(|_| std::array::from_fn(|_| None))
                .collect(),
            by_track,
            cap_group,
            main: (0..ctx.offers.len())
                .filter(|&i| !ctx.offers[i].item.free && !ctx.offers[i].item.rare_catastrophic)
                .collect(),
            ordered: None,
            low_p_ok: vec![false; ctx.offers.len()],
            min_cands: Vec::new(),
            month: 0,
            mode,
        }
    }

    /// A new plan month: free steps done this month may meet a prerequisite, and the season rule
    /// looks at the calendar, so the buying order is worked out again.
    fn new_month(&mut self, m: u16) {
        self.month = m;
        self.ordered = None;
    }

    /// A capability needed less often than the threshold (DESIGN §4.4) is included when its value
    /// per dollar beats the best item of its tier. That is decided once, against the household's
    /// position after the free actions, so what the plan finally includes never depends on the
    /// order it bought things in (and so never on the budget).
    fn screen_rarely_needed(&mut self, ctx: &Ctx<'_>, state: &State) {
        let tier_of = |i: usize| ctx.offers[i].item.tier.max(TierId::H72);
        for i in self.main.clone() {
            let k = tier_of(i);
            let ki = WALK.iter().position(|t| *t == k).unwrap_or(0);
            let Some(c) = self.slot(ctx, state, i, ki).cloned() else {
                continue;
            };
            if c.low_p <= VALUE_EPS {
                continue;
            }
            let mut best: Option<f64> = None;
            for j in self.main.clone() {
                if j == i || tier_of(j) > k {
                    continue;
                }
                if let Some(o) = self.slot(ctx, state, j, ki) {
                    if o.core > VALUE_EPS {
                        let d = density(o.core, o.cost);
                        best = Some(best.map_or(d, |b| b.max(d)));
                    }
                }
            }
            self.low_p_ok[i] = best.is_some_and(|b| density(c.core + c.low_p, c.cost) > b);
        }
    }

    /// The valuation of offer `i` against tier `WALK[ti]`, computed on first use.
    fn slot(&mut self, ctx: &Ctx<'_>, state: &State, i: usize, ti: usize) -> Option<&Candidate> {
        if self.slots[i][ti].is_none() {
            self.slots[i][ti] = Some(evaluate(ctx, state, i, WALK[ti]));
        }
        self.slots[i][ti].as_ref().and_then(|c| c.as_ref())
    }

    /// The full candidate behind a pick, as ranked.
    fn candidate(&self, p: &Pick) -> Candidate {
        let mut c = if p.minimum {
            self.min_cands
                .iter()
                .find(|c| c.offer == p.offer)
                .expect("a kit pick points at a kit candidate")
                .clone()
        } else {
            self.slots[p.offer][p.ti]
                .as_ref()
                .and_then(|c| c.as_ref())
                .expect("a pick points at a computed valuation")
                .clone()
        };
        c.value = p.value;
        c.promoted = p.promoted;
        c
    }

    /// Forgets what a purchase of offer `i` may have changed.
    fn invalidate(&mut self, ctx: &Ctx<'_>, i: usize) {
        self.ordered = None;
        self.slots[i] = std::array::from_fn(|_| None);
        for &t in &ctx.offers[i].tracks {
            for &o in &self.by_track[t] {
                self.slots[o] = std::array::from_fn(|_| None);
            }
        }
        for &o in &ctx.offers[i].group {
            self.slots[o] = std::array::from_fn(|_| None);
        }
    }

    /// Whether every life-safety item of the three-day tier is in hand: none still has value at
    /// the three-day caps. The rare allowance waits for this (REVIEW §2.4: nothing before the
    /// three-day basics).
    fn three_day_life_safety_done(&mut self, ctx: &Ctx<'_>, state: &State) -> bool {
        for i in self.main.clone() {
            let item = ctx.offers[i].item;
            if !item.life_safety || item.tier.max(TierId::H72) > TierId::H72 {
                continue;
            }
            if self
                .slot(ctx, state, i, 0)
                .is_some_and(|c| c.core > VALUE_EPS)
            {
                return false;
            }
        }
        true
    }

    /// Main-plan offers still worth buying at some tier (for what a plan that never finishes
    /// leaves beyond its horizon).
    fn worth_buying(&mut self, ctx: &Ctx<'_>, state: &State) -> Vec<usize> {
        let last = WALK.len() - 1;
        let mut out = Vec::new();
        for i in self.main.clone() {
            let low_p_ok = self.low_p_ok[i];
            if let Some(c) = self.slot(ctx, state, i, last) {
                if c.core + if low_p_ok { c.low_p } else { 0.0 } > VALUE_EPS {
                    out.push(i);
                }
            }
        }
        out
    }

    /// The current tier and its candidates in buying order (`None` when nothing is worth
    /// buying), computed once per state and month. `credited` is the plan's own timeline, which
    /// says whether an accessory's device is in hand yet.
    fn ordered(
        &mut self,
        ctx: &Ctx<'_>,
        state: &State,
        credited: &State,
    ) -> Option<&(TierId, Vec<Pick>)> {
        if self.ordered.is_none() {
            let o = self.compute_order(ctx, state, credited);
            self.ordered = Some(o);
        }
        self.ordered.as_ref().and_then(|o| o.as_ref())
    }

    fn compute_order(
        &mut self,
        ctx: &Ctx<'_>,
        state: &State,
        credited: &State,
    ) -> Option<(TierId, Vec<Pick>)> {
        if self.mode.minimum_first {
            let picks = self.minimum_picks(ctx, state, credited);
            if !picks.is_empty() {
                return Some((TierId::H72, picks));
            }
        }
        let unlocked = |i: usize, k: TierId| ctx.offers[i].item.tier.max(TierId::H72) <= k;
        let available = |i: usize| ctx.requires_met(credited, i);
        let month = self.month;
        let main = std::mem::take(&mut self.main);
        let mut result = None;
        for (ki, &k) in WALK.iter().enumerate() {
            // Candidates of tier k with their value: duration value plus readiness value, where a
            // capability needed less often than the threshold counts only if it passed the
            // screening (DESIGN §4.4). An accessory waits for one of its devices.
            let mut picks: Vec<Pick> = Vec::new();
            for &i in main.iter().filter(|&&i| unlocked(i, k) && available(i)) {
                let low_p_ok = self.low_p_ok[i];
                if let Some(c) = self.slot(ctx, state, i, ki) {
                    let value = c.core + if low_p_ok { c.low_p } else { 0.0 };
                    if value > VALUE_EPS {
                        picks.push(Pick {
                            offer: i,
                            ti: ki,
                            cost: c.cost,
                            value,
                            promoted: false,
                            minimum: false,
                            class: order_class(ctx, c, low_p_ok, month),
                        });
                    }
                }
            }
            // A worthwhile accessory whose devices can never earn a place on their own (what they
            // do is covered already) pulls the cheapest one in at a token value, so the device
            // still comes first and the accessory follows it.
            for &i in main.iter().filter(|&&i| unlocked(i, k) && !available(i)) {
                let low_p_ok = self.low_p_ok[i];
                let worth = self
                    .slot(ctx, state, i, ki)
                    .is_some_and(|c| c.core + if low_p_ok { c.low_p } else { 0.0 } > VALUE_EPS);
                if !worth {
                    continue;
                }
                let devices: Vec<usize> = ctx.offers[i]
                    .requires
                    .iter()
                    .copied()
                    .filter(|j| main.contains(j))
                    .collect();
                let last = WALK.len() - 1;
                let mut stuck = !devices.is_empty();
                for &j in &devices {
                    let low = self.low_p_ok[j];
                    if self
                        .slot(ctx, state, j, last)
                        .is_some_and(|c| c.core + if low { c.low_p } else { 0.0 } > VALUE_EPS)
                    {
                        stuck = false;
                    }
                }
                if !stuck {
                    continue;
                }
                // The cheapest device that is itself available, walking up a chain of
                // prerequisites (fuel, its cans, the generator) when the device waits too.
                if devices.iter().any(|j| picks.iter().any(|p| p.offer == *j)) {
                    continue;
                }
                let mut frontier = devices;
                let mut seen: BTreeSet<usize> = BTreeSet::new();
                let mut cheapest: Option<(usize, f64)> = None;
                for _ in 0..4 {
                    let mut next: Vec<usize> = Vec::new();
                    for &j in &frontier {
                        if !seen.insert(j) {
                            continue;
                        }
                        if available(j) {
                            if let Some(c) = self.slot(ctx, state, j, ki) {
                                if cheapest.is_none_or(|(_, cost)| c.cost < cost) {
                                    cheapest = Some((j, c.cost));
                                }
                            }
                        } else {
                            next.extend(
                                ctx.offers[j]
                                    .requires
                                    .iter()
                                    .copied()
                                    .filter(|d| main.contains(d)),
                            );
                        }
                    }
                    if cheapest.is_some() || next.is_empty() {
                        break;
                    }
                    frontier = next;
                }
                if let Some((j, cost)) =
                    cheapest.filter(|(j, _)| !picks.iter().any(|p| p.offer == *j))
                {
                    picks.push(Pick {
                        offer: j,
                        ti: ki,
                        cost,
                        value: KIT_VALUE_FLOOR,
                        promoted: false,
                        minimum: false,
                        class: 2,
                    });
                }
            }
            if picks.is_empty() {
                continue;
            }
            // Promotion: a later-tier item at least five times the tier's best per dollar joins,
            // valued at the first later tier where it qualifies. Tiers at which an offer's capped
            // targets do not change give the same valuation, so only one of them is looked at. (A
            // device pulled in at a token value is never the tier's best.)
            let best = picks
                .iter()
                .filter(|p| p.value > KIT_VALUE_FLOOR * 2.0)
                .map(Pick::density)
                .reduce(f64::max)
                .unwrap_or(f64::INFINITY);
            let taken: BTreeSet<usize> = picks.iter().map(|p| p.offer).collect();
            for &i in &main {
                if taken.contains(&i) || !available(i) {
                    continue;
                }
                let mut last_group: Option<u8> = None;
                for (tj, &j) in WALK.iter().enumerate().skip(ki + 1) {
                    if !unlocked(i, j) || last_group == Some(self.cap_group[i][tj]) {
                        continue;
                    }
                    last_group = Some(self.cap_group[i][tj]);
                    let low_p_ok = self.low_p_ok[i];
                    if let Some(c) = self.slot(ctx, state, i, tj) {
                        let value = c.core + if low_p_ok { c.low_p } else { 0.0 };
                        if value > VALUE_EPS && density(value, c.cost) >= PROMOTION_FACTOR * best {
                            picks.push(Pick {
                                offer: i,
                                ti: tj,
                                cost: c.cost,
                                value,
                                promoted: true,
                                minimum: false,
                                class: order_class(ctx, c, low_p_ok, month),
                            });
                            break;
                        }
                    }
                }
            }
            // Life-safety first; then capabilities, seasonal items due now, the rest, and
            // long-horizon items last; within each, value per dollar.
            picks.sort_by(|a, b| {
                let (la, lb) = (
                    ctx.offers[a.offer].item.life_safety,
                    ctx.offers[b.offer].item.life_safety,
                );
                lb.cmp(&la)
                    .then(a.class.cmp(&b.class))
                    .then(b.density().total_cmp(&a.density()))
                    .then(a.ti.cmp(&b.ti))
                    .then(a.offer.cmp(&b.offer))
            });
            result = Some((k, picks));
            break;
        }
        self.main = main;
        result
    }

    /// Bare-minimum mode: while the kit is short, the purchases that complete it, before the
    /// tiers, together with the three-day tier's life-safety items (rr-supply leaves the smoke and
    /// carbon monoxide alarms out of the kit because the allocator orders life-safety first
    /// anyway). Life-safety first, then value per dollar at the three-day caps. For each kit line
    /// still short, every offer that can meet it is a candidate for just the amount that closes
    /// the gap: one headlamp of a set of four, three days of water. A line nothing purchasable can
    /// meet is left to the tiers. Empty when the kit is complete or out of reach.
    fn minimum_picks(&mut self, ctx: &Ctx<'_>, state: &State, credited: &State) -> Vec<Pick> {
        self.min_cands.clear();
        let mut want: BTreeMap<usize, f64> = BTreeMap::new();
        for (l, line) in ctx.min_lines.iter().enumerate() {
            let gap = line.need - ctx.min_have(state, l);
            if gap <= 1e-6 {
                continue;
            }
            for &(j, u) in &line.contrib {
                let o = &ctx.offers[j];
                if o.item.free || o.item.rare_catastrophic || !ctx.requires_met(credited, j) {
                    continue;
                }
                let units = gap / u;
                let qty = match o.step {
                    Some(step) => ((units / step) - EPS).ceil().max(1.0) * step,
                    None => {
                        let rem = remaining_set_qty(ctx, state, j);
                        if rem <= EPS {
                            continue;
                        }
                        (units - EPS).ceil().max(1.0).min(rem)
                    }
                };
                let e = want.entry(j).or_insert(0.0);
                *e = e.max(qty);
            }
        }
        if want.is_empty() {
            return Vec::new();
        }
        let horizon = f64::from(TierId::H72.days());
        let mut picks: Vec<Pick> = Vec::new();
        for (j, qty) in want {
            let mut c = evaluate_fixed(ctx, state, j, qty, TierId::H72, horizon);
            // The kit is rr-supply's definition: an item in it is bought even when what the
            // household has already covers its three days (a gas-range household's bleach
            // bottle), after the kit items that add value.
            let value =
                (c.core + if self.low_p_ok[j] { c.low_p } else { 0.0 }).max(KIT_VALUE_FLOOR);
            c.value = value;
            picks.push(Pick {
                offer: j,
                ti: 0,
                cost: c.cost,
                value,
                promoted: false,
                minimum: true,
                class: 0,
            });
            self.min_cands.push(c);
        }
        // The three-day tier's life-safety items keep their place ahead of everything else.
        for i in self.main.clone() {
            let item = ctx.offers[i].item;
            if !item.life_safety
                || item.tier.max(TierId::H72) > TierId::H72
                || picks.iter().any(|p| p.offer == i)
                || !ctx.requires_met(credited, i)
            {
                continue;
            }
            let low_p_ok = self.low_p_ok[i];
            if let Some(c) = self.slot(ctx, state, i, 0) {
                let value = c.core + if low_p_ok { c.low_p } else { 0.0 };
                if value > VALUE_EPS {
                    picks.push(Pick {
                        offer: i,
                        ti: 0,
                        cost: c.cost,
                        value,
                        promoted: false,
                        minimum: false,
                        class: 0,
                    });
                }
            }
        }
        picks.sort_by(|a, b| {
            let (la, lb) = (
                ctx.offers[a.offer].item.life_safety,
                ctx.offers[b.offer].item.life_safety,
            );
            lb.cmp(&la)
                .then(b.density().total_cmp(&a.density()))
                .then(a.offer.cmp(&b.offer))
        });
        picks
    }
}

/// A candidate's place among its tier's non-life-safety items: 0 a capability (readiness share 1
/// and readiness value that counts), 1 a seasonal item due now that the month's money can buy
/// ([`crate::season`]), 2 the rest, 3 a long-horizon item.
fn order_class(ctx: &Ctx<'_>, c: &Candidate, low_p_ok: bool, month: u16) -> u8 {
    let o = &ctx.offers[c.offer];
    if o.item.long_horizon {
        return 3;
    }
    let counts = c
        .ready
        .iter()
        .any(|r| r.1 > VALUE_EPS && (!r.2 || low_p_ok));
    if o.capability && counts {
        return 0;
    }
    if ctx.season_due(c.offer, month) && c.cost <= ctx.monthly + EPS {
        return 1;
    }
    2
}

fn apply(ctx: &Ctx<'_>, state: &mut State, cand: &Candidate) {
    let o = &ctx.offers[cand.offer];
    let id = &o.item.id;
    match state.inventory.binary_search_by(|(x, _)| x.cmp(id)) {
        Ok(pos) => state.inventory[pos].1 += cand.qty,
        Err(pos) => state.inventory.insert(pos, (id.clone(), cand.qty)),
    }
    state.owned[cand.offer] += cand.qty;
    if !o.readiness.is_empty() {
        state.readiness_used[cand.offer] = true;
    }
    for &t in &o.tracks {
        state.track_cov[t] = track_coverage(ctx, t, &state.inventory);
    }
}

/// Buys `cand` with money from `purse`: from the fund first when `use_fund`, the rest from free
/// money.
#[allow(clippy::too_many_arguments)]
fn buy(
    ctx: &Ctx<'_>,
    state: &mut State,
    credited: &mut State,
    purse: &mut Purse,
    use_fund: bool,
    ledger: &mut Ledger,
    cand: &Candidate,
    month: u16,
    rare: bool,
    one_off_first: bool,
    minimum: bool,
    events: &mut Vec<Event>,
) {
    let from_savings = if use_fund {
        purse.fund.min(cand.cost)
    } else {
        0.0
    };
    purse.fund -= from_savings;
    purse.free = (purse.free - (cand.cost - from_savings)).max(0.0);
    // Deposits are whole cents but prices are not: less than a cent left in the fund goes back to
    // free money, rather than paying for the next purchase as "savings" of $0.00.
    if purse.fund < 0.01 - EPS {
        purse.free += purse.fund.max(0.0);
        purse.fund = 0.0;
    }
    if purse.target.is_some_and(|t| t.offer == cand.offer) || purse.fund == 0.0 {
        purse.target = None;
        purse.carried_from = None;
    }
    if from_savings > EPS {
        ledger.add_envelope(&ctx.offers[cand.offer].item.id, from_savings, cand.cost);
    }
    // What the purchase does to coverage as the plan reports it (free actions count only from
    // their month), for its explanation.
    let horizon = f64::from(cand.tier.days());
    let mut shown = evaluate_fixed(ctx, credited, cand.offer, cand.qty, cand.tier, horizon);
    // Rarely needed readiness value is mentioned only if the allocator counted it.
    shown.value = if cand.value > cand.core + VALUE_EPS {
        shown.core + shown.low_p
    } else {
        shown.core
    };
    if rare {
        // The rare-catastrophe allowance never changes what the main plan sees, so its timing
        // (which depends on the budget) cannot reorder the main plan.
        state.owned[cand.offer] += cand.qty;
        credited.owned[cand.offer] += cand.qty;
    } else {
        apply(ctx, state, cand);
        apply(ctx, credited, &shown);
    }
    ledger.purchase_months.entry(cand.offer).or_insert(month);
    ledger.sequence.push(Purchase {
        month,
        item_id: ctx.offers[cand.offer].item.id.clone(),
        quantity: cand.qty,
        cost_usd: cand.cost,
        from_savings_usd: from_savings,
        value: cand.value,
        tier: cand.tier,
        promoted: cand.promoted,
        rare_catastrophic: rare,
        minimum,
    });
    events.push(Event::Buy {
        cand: cand.clone(),
        shown,
        rare,
        from_savings,
        one_off_first,
        minimum,
    });
}

/// Does the free actions scheduled for month `m` in the plan's own coverage (`credited`), recording
/// each with what it adds at that point.
fn credit_free_actions(
    ctx: &Ctx<'_>,
    credited: &mut State,
    scheduled: &[(u16, usize, f64)],
    m: u16,
    events: &mut Vec<Event>,
) {
    for &(month, i, qty) in scheduled {
        if month == m {
            let cand = evaluate_fixed(ctx, credited, i, qty, TierId::Now, f64::INFINITY);
            apply(ctx, credited, &cand);
            events.push(Event::Free { cand, done: false });
        }
    }
}

/// The allocator's running state, borrowed together by helpers that buy.
struct Books<'s> {
    state: &'s mut State,
    credited: &'s mut State,
    ledger: &'s mut Ledger,
    cache: &'s mut Cache,
}

impl Books<'_> {
    /// Buys `pick` in month 0 with free money from `main`.
    fn buy_now(
        &mut self,
        ctx: &Ctx<'_>,
        main: &mut Purse,
        pick: &Pick,
        one_off_first: bool,
        events: &mut Vec<Event>,
    ) {
        let cand = self.cache.candidate(pick);
        buy(
            ctx,
            self.state,
            self.credited,
            main,
            false,
            self.ledger,
            &cand,
            0,
            false,
            one_off_first,
            pick.minimum,
            events,
        );
        self.cache.invalidate(ctx, pick.offer);
    }
}

/// Split schedule, month 0: the one-off money goes to life-safety items first (DESIGN §4.7).
///
/// The top life-safety item is the first life-safety item in the buying order that costs more
/// than a month's money (`monthly`). Cheaper ones are bought from monthly money soon enough, and
/// the very first life-safety item in the order is nearly always one of them (two weeks of
/// medicine for $12), so the one-off money is for the dear one. If the money left covers it, it is
/// bought now and the next one is looked at. Otherwise `reserve_share` of the money left (half, by
/// default) is kept for it, the rest buys the cheaper life-safety items, cheapest first, and
/// everything those leave goes into its fund. Returns whether a fund was opened; it then stands in
/// for month 0's usual deposit.
fn one_off_to_life_safety(
    ctx: &Ctx<'_>,
    books: &mut Books<'_>,
    main: &mut Purse,
    reserve_share: f64,
    monthly: f64,
    events: &mut Vec<Event>,
) -> bool {
    let life_safety = |p: &Pick| ctx.offers[p.offer].item.life_safety;
    loop {
        let Some((_, picks)) = books.cache.ordered(ctx, books.state, books.credited) else {
            return false;
        };
        let Some(top) = picks
            .iter()
            .copied()
            .find(|p| life_safety(p) && p.cost > monthly + EPS)
        else {
            return false;
        };
        if top.cost <= main.free + EPS {
            books.buy_now(ctx, main, &top, true, events);
            continue;
        }
        if main.free <= EPS {
            return false;
        }
        let keep = reserve_share * main.free;
        let start = events.len();
        let still_wanted = loop {
            let Some((_, picks)) = books.cache.ordered(ctx, books.state, books.credited) else {
                break None;
            };
            let Some(target) = picks.iter().copied().find(|p| p.offer == top.offer) else {
                break None;
            };
            let spendable = main.free - keep;
            let cheapest = picks
                .iter()
                .copied()
                .filter(|p| p.offer != top.offer && life_safety(p) && p.cost <= spendable + EPS)
                .min_by(|a, b| a.cost.total_cmp(&b.cost));
            match cheapest {
                Some(p) => books.buy_now(ctx, main, &p, false, events),
                None => break Some(target),
            }
        };
        // A cheaper item made it unnecessary: look again with the money left.
        let Some(target) = still_wanted else {
            continue;
        };
        let mut reserve = Vec::new();
        main.deposit(
            main.free,
            FundFor::of(&target),
            SaveHow::OneOff(reserve_share),
            &mut reserve,
        );
        let opened = !reserve.is_empty();
        // The deposit is listed before the purchases it left room for, like a month's usual one.
        events.splice(start..start, reserve);
        return opened;
    }
}

fn snapshot(
    ctx: &Ctx<'_>,
    state: &State,
    checklist: &[ChecklistEntry],
    month: u16,
) -> MonthCoverage {
    let h = ctx.input.household;
    let mut days = BTreeMap::new();
    for b in BucketId::ALL
        .iter()
        .filter(|b| b.kind() == BucketKind::Duration)
    {
        // A tracked bucket's coverage is its weakest track's (the rule's contract for parts), which
        // the allocator already keeps up to date; only untracked buckets ask the rule.
        let tracked = ctx
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.bucket == *b)
            .map(|(i, _)| state.track_cov[i])
            .fold(None, |acc: Option<f64>, x| {
                Some(acc.map_or(x, |a| a.min(x)))
            });
        let v = tracked.unwrap_or_else(|| ctx.rule.coverage(*b, &state.inventory, h));
        days.insert(*b, v);
    }
    let mut readiness_done = BTreeMap::new();
    for b in BucketId::ALL
        .iter()
        .filter(|b| b.kind() != BucketKind::Duration)
    {
        let n = checklist
            .iter()
            .filter(|e| e.buckets.contains(b) && e.done(state))
            .count();
        readiness_done.insert(*b, n as u32);
    }
    MonthCoverage {
        month,
        days,
        readiness_done,
    }
}

/// Rounds money to cents for output.
fn money(x: f64) -> f32 {
    ((x * 100.0).round() / 100.0) as f32
}

/// Assembles one month's plan lines, merging repeated purchases of the same item.
fn plan_items(
    ctx: &Ctx<'_>,
    events: &[Event],
    rare_families: &BTreeMap<usize, RareFor>,
    rare_monthly: f64,
) -> Vec<PlanItem> {
    // Merge Buy events per (offer, rare) into the first occurrence.
    let mut merged: Vec<Event> = Vec::new();
    for e in events {
        if let Event::Buy {
            cand,
            shown,
            rare,
            from_savings,
            one_off_first,
            minimum,
        } = e
        {
            let found = merged.iter_mut().find_map(|m| match m {
                Event::Buy {
                    cand: c,
                    shown: sh,
                    rare: r,
                    from_savings: f,
                    one_off_first: o,
                    minimum: mn,
                } if c.offer == cand.offer && r == rare => Some((c, sh, f, o, mn)),
                _ => None,
            });
            if let Some((c, sh, f, o, mn)) = found {
                merge_into(c, cand);
                merge_into(sh, shown);
                *f += from_savings;
                *o |= *one_off_first;
                *mn |= *minimum;
                continue;
            }
        }
        merged.push(e.clone());
    }
    merged
        .iter()
        .map(|e| plan_item(ctx, e, rare_families, rare_monthly))
        .collect()
}

fn merge_into(into: &mut Candidate, more: &Candidate) {
    into.qty += more.qty;
    into.cost += more.cost;
    into.value += more.value;
    into.core += more.core;
    into.low_p += more.low_p;
    into.promoted |= more.promoted;
    into.tier = into.tier.max(more.tier);
    for g in &more.gains {
        match into.gains.iter_mut().find(|x| x.track == g.track) {
            Some(x) => {
                x.x1 = g.x1;
                x.value += g.value;
            }
            None => into.gains.push(g.clone()),
        }
    }
    for r in &more.ready {
        if !into.ready.iter().any(|x| x.0 == r.0) {
            into.ready.push(*r);
        }
    }
}

fn plan_item(
    ctx: &Ctx<'_>,
    e: &Event,
    rare_families: &BTreeMap<usize, RareFor>,
    rare_monthly: f64,
) -> PlanItem {
    match e {
        Event::Free { cand, done } => {
            let lead = if *done { Lead::AlreadyDone } else { Lead::Free };
            line(
                ctx,
                cand,
                PlanItemKind::FreeAction,
                TierId::Now,
                lead,
                *done,
                None,
            )
        }
        Event::Owned { cand, paid } => {
            let o = &ctx.offers[cand.offer];
            let tier = o.item.tier;
            let mut item = line(
                ctx,
                cand,
                PlanItemKind::Purchase,
                tier,
                Lead::AlreadyOwned,
                true,
                paid.map(|p| p as f32),
            );
            if let Some(p) = paid {
                item.est_cost_usd = money(*p);
            }
            item
        }
        Event::Buy {
            cand,
            shown,
            rare,
            from_savings,
            one_off_first,
            minimum,
        } => {
            let lead = if *rare {
                Lead::RareAllowance
            } else {
                Lead::Purchase
            };
            let mut item = line(
                ctx,
                cand,
                PlanItemKind::Purchase,
                cand.tier,
                lead,
                false,
                None,
            );
            if *rare {
                // What the allowance bought and why: the ticked families it is for (and the cause
                // inside them it is gated on), which pass the 1-in-1,000 line here (no point
                // estimate: rare rows show ranges only).
                let why = rare_families.get(&cand.offer).cloned().unwrap_or_default();
                item.why =
                    rare::allowance_sentence(&why.families, why.cause.as_deref(), rare_monthly);
                item.hazards = why.families;
            } else {
                // Explain with coverage as the plan reports it (free actions count from their
                // month).
                let parts = why_parts(ctx, shown);
                item.why = explain::why(lead, &parts, ctx.people, ctx.years);
            }
            if *from_savings > 0.005 {
                item.why.push_str(&format!(
                    " Paid with {} saved in earlier months.",
                    explain::dollars(*from_savings)
                ));
            }
            if *one_off_first {
                item.why.push_str(
                    " Your one-off money pays for this first: it keeps you safe and costs more \
                     than a month's budget.",
                );
            }
            if *minimum {
                item.why.push_str(explain::MINIMUM_KIT_NOTE);
            }
            item
        }
        Event::Reserve {
            offer,
            tier,
            deposit,
            saved,
            needed,
            how,
            carried_from,
        } => {
            let item = ctx.offers[*offer].item;
            let (deposit_s, name, saved_s, needed_s) = (
                explain::dollars(*deposit),
                lower_first(&item.name),
                explain::dollars(*saved),
                explain::dollars(*needed),
            );
            let mut why = match how {
                SaveHow::AllMoney => format!(
                    "Sets aside {deposit_s} toward {name} ({saved_s} of {needed_s} saved). It costs \
                     more than a month's budget, so the plan saves for it instead of buying \
                     something worth less."
                ),
                SaveHow::Share(share) => format!(
                    "Sets aside {deposit_s} toward {name} ({saved_s} of {needed_s} saved). It \
                     costs more than a month's budget, so the plan puts {} of each month's money \
                     toward it and spends the rest on other items.",
                    share_words(*share)
                ),
                SaveHow::OneOff(share) => format!(
                    "Sets aside {deposit_s} toward {name} ({saved_s} of {needed_s} saved). It \
                     keeps you safe but costs more than your one-off money, so {} of that money \
                     goes toward it, the rest buys cheaper safety items first, and anything left \
                     over is saved for it too.",
                    share_words(*share)
                ),
                SaveHow::Rare => format!(
                    "Sets aside {deposit_s} from your rare-emergency allowance toward {name} \
                     ({saved_s} of {needed_s} saved)."
                ),
            };
            if let Some(from) = carried_from {
                why.push_str(&format!(
                    " This includes money first saved for {}, which the plan no longer needs.",
                    lower_first(&ctx.offers[*from].item.name)
                ));
            }
            PlanItem {
                item_id: item.id.clone(),
                name: format!("Save toward: {}", item.name),
                kind: PlanItemKind::Reserve,
                quantity: 1.0,
                unit: "deposit".into(),
                est_cost_usd: money(*deposit),
                price_band: CostRange {
                    low: money(*deposit),
                    high: money(*deposit),
                },
                buckets: item.buckets.clone(),
                hazards: Vec::new(),
                why,
                risk_reduction: 0.0,
                tier: *tier,
                done: false,
                paid_usd: None,
                requires: Vec::new(),
                decision: false,
            }
        }
    }
}

/// "half", or a percentage for another reserve share.
fn share_words(share: f64) -> String {
    if (share - 0.5).abs() < 1e-9 {
        "half".to_owned()
    } else {
        format!("{:.0}%", share * 100.0)
    }
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn line(
    ctx: &Ctx<'_>,
    cand: &Candidate,
    kind: PlanItemKind,
    tier: TierId,
    lead: Lead,
    done: bool,
    paid: Option<f32>,
) -> PlanItem {
    let o = &ctx.offers[cand.offer];
    let item = o.item;
    let parts = why_parts(ctx, cand);
    let hazards = hazards_for(ctx, cand);
    let band = &item.price_band_usd;
    let (low, high) = if item.free {
        (0.0, 0.0)
    } else {
        (
            cand.qty * f64::from(band.low),
            cand.qty * f64::from(band.high),
        )
    };
    // `cand.cost` already uses the recorded unit price when the household gave one.
    let est = cand.cost;
    PlanItem {
        item_id: item.id.clone(),
        name: item.name.clone(),
        kind,
        quantity: ((cand.qty * 1000.0).round() / 1000.0) as f32,
        unit: item.unit.clone(),
        est_cost_usd: money(est),
        price_band: CostRange {
            low: money(low),
            high: money(high),
        },
        buckets: item.buckets.clone(),
        hazards,
        why: explain::why(lead, &parts, ctx.people, ctx.years),
        risk_reduction: cand.value as f32,
        tier,
        done,
        paid_usd: paid,
        // The accessory's devices (any one of them), which the plan never schedules it before.
        requires: item.requires.clone(),
        decision: item.decision,
    }
}

fn why_parts(ctx: &Ctx<'_>, cand: &Candidate) -> WhyParts {
    let years = f64::from(ctx.years);
    // One line per bucket: parts of a bucket moved by the same purchase (a contribution without a
    // part) are reported together, from the weakest part's coverage before to after.
    let mut groups: Vec<(BucketId, Vec<&Gain>)> = Vec::new();
    for g in cand.gains.iter().filter(|g| g.value > VALUE_EPS) {
        let b = ctx.tracks[g.track].bucket;
        match groups.iter_mut().find(|(gb, _)| *gb == b) {
            Some((_, list)) => list.push(g),
            None => groups.push((b, vec![g])),
        }
    }
    let mut scored: Vec<(f64, DurationText)> = groups
        .into_iter()
        .map(|(bucket, gains)| {
            let value: f64 = gains.iter().map(|g| g.value).sum();
            let first = &ctx.tracks[gains[0].track];
            let (part, from, to, share) = if gains.len() == 1 {
                (first.part.clone(), gains[0].x0, gains[0].x1, first.share)
            } else {
                let from = gains.iter().map(|g| g.x0).fold(f64::INFINITY, f64::min);
                let to = gains.iter().map(|g| g.x1).fold(f64::INFINITY, f64::min);
                let share: f64 = gains.iter().map(|g| ctx.tracks[g.track].share).sum();
                (None, from, to, share.min(1.0))
            };
            let ref_days = ladder_floor_days(from);
            let rate = share * curve(ctx, bucket).lambda_at(ref_days);
            // A filter fills the stored-water chain's days after the stored water: it makes raw
            // water safe, so its sentence names that water, not "stored water" (review S6).
            let item = ctx.offers[cand.offer].item;
            let part = if bucket == BucketId::WaterOut
                && part.as_deref() == Some("stored water")
                && item.quantity_rule == TREATMENT_RULE
            {
                let well = ctx.input.household.housing.water == WaterSource::Well;
                Some(
                    if well {
                        explain::WELL_WATER_TREATED
                    } else {
                        explain::RAW_WATER_TREATED
                    }
                    .to_owned(),
                )
            } else {
                part
            };
            let text = DurationText {
                bucket,
                part,
                from_days: from,
                to_days: to,
                target_days: first.target,
                ref_days,
                per_100: per_100(rate, years),
            };
            (value, text)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.bucket.cmp(&b.1.bucket)));
    let durations: Vec<DurationText> = scored.into_iter().map(|(_, t)| t).collect();
    let mut ready: Vec<&(BucketId, f64, bool)> = cand
        .ready
        .iter()
        .filter(|r| r.1 > VALUE_EPS && (!r.2 || cand.value > cand.core + VALUE_EPS))
        .collect();
    ready.sort_by(|a, b| b.1.total_cmp(&a.1));
    let readiness: Vec<ReadinessText> = ready
        .iter()
        .map(|r| ReadinessText {
            bucket: r.0,
            per_100: per_100(
                annual_rate_from_10yr(readiness_p10(ctx, cand.offer, r.0)),
                years,
            ),
            event: readiness_override(ctx, cand.offer, r.0).map(|(_, words)| words),
        })
        .collect();
    let headline = durations
        .first()
        .map(|d| (d.bucket, d.part.clone()))
        .or_else(|| readiness.first().map(|r| (r.bucket, None)));
    let causes: Vec<HazardId> = headline
        .map(|(b, part)| {
            top_hazards(ctx, b)
                .into_iter()
                .map(|(h, _)| h)
                .filter(|h| fits_part(*h, part.as_deref()))
                .collect()
        })
        .unwrap_or_default();
    WhyParts {
        durations,
        readiness,
        also: ctx.offers[cand.offer].item.buckets.clone(),
        causes,
    }
}

/// Whether a hazard can cause the named part of a bucket: heat waves do not cause dangerous cold
/// and winter hazards do not cause dangerous heat. Other parts and hazards always fit.
fn fits_part(h: HazardId, part: Option<&str>) -> bool {
    match part {
        Some("heat") => !matches!(
            h,
            HazardId::ColdWave | HazardId::WinterWeather | HazardId::IceStorm | HazardId::Avalanche
        ),
        Some("cold") => !matches!(
            h,
            HazardId::HeatWave | HazardId::Drought | HazardId::Wildfire
        ),
        _ => true,
    }
}

/// The hazards behind a bucket, largest share first (shares below a tenth left out).
fn top_hazards(ctx: &Ctx<'_>, bucket: BucketId) -> Vec<(HazardId, f32)> {
    let Some(a) = ctx.input.risks.assessments.get(&bucket) else {
        return Vec::new();
    };
    let mut list: Vec<(HazardId, f32)> = a
        .contributions
        .iter()
        .filter(|c| c.share >= 0.1)
        .map(|c| (c.hazard, c.share))
        .collect();
    list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    list
}

fn hazards_for(ctx: &Ctx<'_>, cand: &Candidate) -> Vec<HazardId> {
    let mut buckets: Vec<BucketId> = cand
        .gains
        .iter()
        .filter(|g| g.value > VALUE_EPS)
        .map(|g| ctx.tracks[g.track].bucket)
        .chain(cand.ready.iter().filter(|r| r.1 > VALUE_EPS).map(|r| r.0))
        .collect();
    if buckets.is_empty() {
        buckets = ctx.offers[cand.offer].item.buckets.clone();
    }
    let mut best: BTreeMap<HazardId, f32> = BTreeMap::new();
    for b in buckets {
        for (h, s) in top_hazards(ctx, b) {
            let e = best.entry(h).or_insert(0.0);
            *e = e.max(s);
        }
    }
    let mut list: Vec<(HazardId, f32)> = best.into_iter().collect();
    list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    list.into_iter().take(3).map(|(h, _)| h).collect()
}

/// One step on the readiness checklists: an item, or an alternative group counted once.
#[derive(Debug, Clone)]
struct ChecklistEntry {
    /// The offers that tick it (more than one for an alternative group).
    members: Vec<usize>,
    /// The readiness (and money) buckets whose checklist it is on.
    buckets: Vec<BucketId>,
}

impl ChecklistEntry {
    /// Ticked when any member is owned or bought in `state`.
    fn done(&self, state: &State) -> bool {
        self.members.iter().any(|&i| state.owned[i] > 0.0)
    }
}

/// The checklist of each readiness (and money) bucket: the free actions that name it and the items
/// whose readiness credit for it counts (needed often enough, or screened in). An item bought for
/// another need that merely lists the bucket is not a step on its checklist, the members of an
/// alternative group are one step, and specialised rare-catastrophe items are on none: the
/// allowance values them by their family, not by a bucket, and what it can buy depends on the
/// money (half the allowance per family).
fn readiness_checklist(ctx: &Ctx<'_>, low_p_ok: &[bool]) -> Vec<ChecklistEntry> {
    let named = |o: &Offer<'_>| -> Vec<BucketId> {
        o.item
            .buckets
            .iter()
            .copied()
            .filter(|b| b.kind() != BucketKind::Duration)
            .collect()
    };
    let mut out: Vec<ChecklistEntry> = Vec::new();
    let mut group_entry: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, o) in ctx.offers.iter().enumerate() {
        let buckets = if o.item.rare_catastrophic {
            Vec::new()
        } else if o.item.free {
            named(o)
        } else {
            let mut list: Vec<BucketId> = Vec::new();
            for &(b, _) in &o.readiness {
                let counts = readiness_p10(ctx, i, b) >= READINESS_MIN_P_NEED_10YR || low_p_ok[i];
                if counts && !list.contains(&b) {
                    list.push(b);
                }
            }
            list
        };
        if buckets.is_empty() {
            continue;
        }
        match o
            .item
            .alternative_group
            .as_deref()
            .and_then(|g| group_entry.get(g).copied())
        {
            Some(k) => {
                out[k].members.push(i);
                for b in buckets {
                    if !out[k].buckets.contains(&b) {
                        out[k].buckets.push(b);
                    }
                }
            }
            None => {
                if let Some(g) = o.item.alternative_group.as_deref() {
                    group_entry.insert(g, out.len());
                }
                out.push(ChecklistEntry {
                    members: vec![i],
                    buckets,
                });
            }
        }
    }
    out
}

fn covered_targets(
    ctx: &Ctx<'_>,
    state: &State,
    checklist: &[ChecklistEntry],
) -> BTreeMap<BucketId, Target> {
    let h = ctx.input.household;
    let risks = ctx.input.risks;
    let mut out = BTreeMap::new();
    for b in BucketId::ALL {
        match b.kind() {
            BucketKind::Duration => {
                let v = ((ctx.rule.coverage(*b, &state.inventory, h) * 10.0).round() / 10.0) as f32;
                out.insert(
                    *b,
                    Target::Days {
                        value: v,
                        low: v,
                        high: v,
                    },
                );
            }
            _ if *b == BucketId::Income => {
                let v = h.finances.emergency_fund_months.max(0.0);
                out.insert(
                    *b,
                    Target::Months {
                        value: v,
                        low: v,
                        high: v,
                    },
                );
            }
            _ if *b == BucketId::Evacuate => {
                if let Some(a) = risks.assessments.get(b) {
                    out.insert(*b, a.target);
                }
            }
            _ => {
                let of = checklist.iter().filter(|e| e.buckets.contains(b)).count();
                let done = checklist
                    .iter()
                    .filter(|e| e.buckets.contains(b) && e.done(state))
                    .count();
                out.insert(
                    *b,
                    Target::Readiness {
                        p_need_10yr: risks.p_need_10yr(*b),
                        done: done.min(255) as u8,
                        of: of.min(255) as u8,
                    },
                );
            }
        }
    }
    out
}

/// The highest tier, up to the recommended one, whose (capped) targets every tracked bucket meets.
fn tier_met(ctx: &Ctx<'_>, cov: &[f64]) -> TierId {
    let recommended = tier_recommended(ctx);
    let mut reached = TierId::Now;
    for &k in WALK.iter().filter(|&&k| k <= recommended) {
        let horizon = f64::from(k.days());
        let ok = ctx
            .tracks
            .iter()
            .enumerate()
            .all(|(t, tr)| cov[t] + 1e-6 >= tr.target.min(horizon));
        if !ok {
            break;
        }
        reached = k;
    }
    reached
}

/// The smallest tier whose horizon covers every duration target (at least three days).
fn tier_recommended(ctx: &Ctx<'_>) -> TierId {
    let max_target = ctx
        .input
        .risks
        .curves
        .values()
        .map(|c| c.target_days)
        .fold(0.0, f64::max);
    WALK.iter()
        .copied()
        .find(|k| f64::from(k.days()) + 1e-9 >= max_target)
        .unwrap_or(TierId::Y1)
}

#[allow(clippy::too_many_arguments)]
fn guardrail_facts(
    ctx: &Ctx<'_>,
    state: &State,
    after_free: &[f64],
    purchase_months: &BTreeMap<usize, u16>,
    free_month_of: &BTreeMap<usize, u16>,
    stopped: Option<u16>,
    food_by_month: &[Option<f64>],
) -> Facts {
    // When each role is first in hand: month 0 for what was owned or done, else its purchase month.
    let explicit: BTreeSet<ItemRole> = ctx
        .offers
        .iter()
        .flat_map(|o| o.roles.iter().copied())
        .collect();
    let has_role = |o: &Offer<'_>, role: ItemRole| -> bool {
        if explicit.contains(&role) {
            return o.roles.contains(&role);
        }
        let b = &o.item.buckets;
        match role {
            ItemRole::DevicePower => o.item.life_safety && b.contains(&BucketId::Power),
            ItemRole::ColdChain => {
                o.item.life_safety
                    && b.contains(&BucketId::Medication)
                    && b.contains(&BucketId::Power)
            }
            ItemRole::GoBag => !o.item.free && b.contains(&BucketId::Evacuate),
            ItemRole::StoredWater => {
                !o.item.free && o.item.life_safety && b.contains(&BucketId::WaterOut)
            }
        }
    };
    let first_month = |role: ItemRole| -> Option<u16> {
        ctx.offers
            .iter()
            .enumerate()
            .filter(|(_, o)| has_role(o, role))
            .filter_map(|(i, o)| {
                if let Some(m) = purchase_months.get(&i) {
                    Some(*m)
                } else if let Some(m) = free_month_of.get(&i) {
                    // A free action still to do counts from the month it is scheduled.
                    Some(*m)
                } else if state.owned[i] > 0.0 || o.item.free {
                    Some(0)
                } else {
                    None
                }
            })
            .min()
    };
    // Stored water is needed when free steps (refilled bottles) and what the household has leave
    // the parts stored water fills short of the target; with nothing in the catalogue that stores
    // water, when they leave any part of the no-water bucket short.
    let stored_offers: Vec<usize> = (0..ctx.offers.len())
        .filter(|&i| !ctx.offers[i].item.free && has_role(&ctx.offers[i], ItemRole::StoredWater))
        .collect();
    let short = |t: usize| after_free[t] + 1e-6 < ctx.tracks[t].target;
    // The no-water parts stored water fills (the stored-water part, not the toilet): where a
    // plentiful supply of it, alone, gives cover.
    let water_tracks: BTreeSet<usize> = stored_offers
        .iter()
        .flat_map(|&i| {
            let plenty = vec![(
                ctx.offers[i].item.id.clone(),
                ctx.offers[i].set_quantity.max(1.0) * 1.0e4,
            )];
            ctx.offers[i]
                .tracks
                .iter()
                .copied()
                .filter(move |&t| ctx.tracks[t].bucket == BucketId::WaterOut)
                .filter(move |&t| track_coverage(ctx, t, &plenty) > 1e-9)
                .collect::<Vec<usize>>()
        })
        .collect();
    let water_target = ctx
        .input
        .risks
        .curves
        .get(&BucketId::WaterOut)
        .is_some_and(|c| c.target_days > 0.0);
    let water_needed = water_target
        && if water_tracks.is_empty() {
            (0..ctx.tracks.len()).any(|t| ctx.tracks[t].bucket == BucketId::WaterOut && short(t))
        } else {
            water_tracks.iter().any(|&t| short(t))
        };
    // Refrigerated medicine that needs a power source: the cold chain is in hand only once
    // something covers the power part rr-plan names after rr-supply's `power_for_cold_medicine`
    // class, as well as the cooler. The first month an offer that covers that part is in hand.
    let cold_power_tracks: Vec<usize> = (0..ctx.tracks.len())
        .filter(|&t| ctx.tracks[t].part.as_deref() == Some(guardrails::COLD_MEDICINE_POWER_PART))
        .collect();
    let in_hand = |i: usize| -> Option<u16> {
        if let Some(m) = purchase_months.get(&i) {
            Some(*m)
        } else if let Some(m) = free_month_of.get(&i) {
            Some(*m)
        } else if state.owned[i] > 0.0 || ctx.offers[i].item.free {
            Some(0)
        } else {
            None
        }
    };
    let cold_power_needed = !cold_power_tracks.is_empty();
    let cold_power_month = if cold_power_needed {
        (0..ctx.offers.len())
            .filter(|&i| {
                let plenty = vec![(
                    ctx.offers[i].item.id.clone(),
                    ctx.offers[i].set_quantity.max(1.0) * 1.0e4,
                )];
                cold_power_tracks
                    .iter()
                    .any(|&t| track_coverage(ctx, t, &plenty) > 1e-9)
            })
            .filter_map(in_hand)
            .min()
    } else {
        None
    };
    let uncovered: Vec<BucketId> = ctx
        .tracks
        .iter()
        .enumerate()
        .filter(|(t, tr)| tr.target > 0.0 && state.track_cov[*t] + 1e-6 < tr.target)
        .map(|(_, tr)| tr.bucket)
        .fold(Vec::new(), |mut acc, b| {
            if !acc.contains(&b) {
                acc.push(b);
            }
            acc
        });
    // A step about leaving home anywhere in the plan (owned, done, free or bought): a go-bag, the
    // evacuation plan, a ride out, the 48-hour list.
    let leaving_in_plan = ctx.offers.iter().enumerate().any(|(i, o)| {
        state.owned[i] > 0.0
            && (o.readiness.iter().any(|(b, _)| *b == BucketId::Evacuate)
                || has_role(o, ItemRole::GoBag)
                || o.item.buckets.first() == Some(&BucketId::Evacuate))
    });
    let owned = |i: usize| state.owned[i] > 0.0;
    let by_rule = |rule: &'static str| {
        (0..ctx.offers.len()).filter(move |&i| ctx.offers[i].item.quantity_rule == rule)
    };
    let cooking: Vec<usize> = by_rule(COOKING_RULE)
        .filter(|&i| !ctx.offers[i].item.free)
        .collect();
    Facts {
        device_power_month: first_month(ItemRole::DevicePower),
        cooler_month: first_month(ItemRole::ColdChain),
        cold_power_needed,
        cold_power_month,
        go_bag_month: first_month(ItemRole::GoBag),
        water_needed,
        // Month 0 for stored water the household already has, else the first purchase; never a
        // free step (`first_month` counts every free item from month 0).
        stored_water_month: stored_offers
            .iter()
            .filter_map(|&i| {
                let id = &ctx.offers[i].item.id;
                let owned = ctx
                    .input
                    .household
                    .existing
                    .iter()
                    .any(|e| e.item_id == *id && e.qty > 0.0);
                if owned {
                    Some(0)
                } else {
                    purchase_months.get(&i).copied()
                }
            })
            .min(),
        uncovered_when_stopped: if stopped.is_some() {
            uncovered
        } else {
            Vec::new()
        },
        leaving_in_plan,
        food_at_month_3: food_by_month
            .get(guardrails::BENEFIT_BUFFER_BY_MONTH as usize)
            .or(food_by_month.last())
            .copied()
            .flatten(),
        food_target: ctx
            .input
            .risks
            .curves
            .get(&BucketId::Supplies)
            .map(|c| c.target_days),
        filter_in_plan: by_rule(TREATMENT_RULE).any(owned),
        rain_in_plan: by_rule(RAIN_RULE).any(owned),
        cooking_needed: !cooking.is_empty(),
        cooking_in_plan: cooking.iter().any(|&i| owned(i)),
        shortfalls: if stopped.is_some() {
            simultaneous_shortfalls(ctx, state)
        } else {
            Vec::new()
        },
        too_long: None,
    }
}

/// DESIGN §4.7's simultaneous-need check against what the plan stores: for each event that sets a
/// target, the stored water, food and power it would need at once (when the event brings the need
/// at least [`guardrails::SIMULTANEOUS_MIN_CHANCE`] of the time) that the plan's end covers by
/// less than half a day.
fn simultaneous_shortfalls(ctx: &Ctx<'_>, state: &State) -> Vec<Shortfall> {
    let have = |b: BucketId| -> Option<f64> {
        (0..ctx.tracks.len())
            .filter(|&t| ctx.tracks[t].bucket == b)
            .map(|t| state.track_cov[t])
            .reduce(f64::min)
    };
    let mut out = Vec::new();
    for need in &ctx.input.risks.simultaneous {
        let mut short = Vec::new();
        for &(b, days, chance) in &need.needs {
            if !guardrails::SIMULTANEOUS_BUCKETS.contains(&b)
                || chance < guardrails::SIMULTANEOUS_MIN_CHANCE
                || !(days.is_finite() && days > 0.0)
            {
                continue;
            }
            let Some(h) = have(b) else {
                continue;
            };
            if h + guardrails::SIMULTANEOUS_SLACK_DAYS < days {
                short.push((b, days, h));
            }
        }
        if !short.is_empty() {
            out.push(Shortfall {
                event: need.event.clone(),
                hazard: need.hazard,
                short,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn per_month(n: usize) -> Vec<usize> {
        let months = free_action_months(n);
        let last = months.last().copied().unwrap_or(0);
        (0..=last)
            .map(|m| months.iter().filter(|&&x| x == m).count())
            .collect()
    }

    #[test]
    fn free_actions_are_spread_eight_then_four_and_done_by_month_two() {
        assert_eq!(per_month(0), [0]);
        assert_eq!(per_month(5), [5]);
        assert_eq!(per_month(8), [8]);
        assert_eq!(per_month(12), [8, 4]);
        assert_eq!(per_month(15), [8, 4, 3]);
        assert_eq!(per_month(16), [8, 4, 4]);
        // 22, the top of the fixture range: the pace rises so all are done by month 2.
        assert_eq!(per_month(22), [8, 7, 7]);
        assert_eq!(per_month(24), [8, 8, 8]);
        // More than 24 cannot fit three months of eight; the rest follow at eight a month.
        assert_eq!(per_month(30), [8, 8, 8, 6]);
        for n in 0..=40 {
            let months = free_action_months(n);
            assert_eq!(months.len(), n);
            assert!(
                months.windows(2).all(|w| w[0] <= w[1]),
                "order kept for {n}"
            );
            assert!(
                per_month(n).iter().all(|&k| k <= FREE_ACTIONS_MONTH_0),
                "{n}"
            );
            if n <= 24 {
                assert!(months.iter().all(|&m| m <= FREE_ACTIONS_BY_MONTH), "{n}");
            }
        }
    }

    #[test]
    fn decisions_and_exempt_steps_are_outside_the_monthly_count_by_month_1() {
        // 30 steps, five of them exempt (decisions and the like) ranked here and there.
        let exempt: Vec<bool> = (0..30).map(|k| [2, 9, 12, 20, 29].contains(&k)).collect();
        let months = schedule_free(&exempt);
        // Month 0 lists the first eight whatever their kind (one exempt among them).
        assert!(months[..8].iter().all(|&m| m == 0));
        // The other exempt steps all go in month 1.
        for k in [9, 12, 20, 29] {
            assert_eq!(months[k], EXEMPT_BY_MONTH, "{k}");
        }
        // The 18 ordinary steps after month 0 keep the usual pace: eight a month (25 ordinary
        // steps in all, more than three months of eight), in their order.
        let ordinary: Vec<u16> = (8..30).filter(|k| !exempt[*k]).map(|k| months[k]).collect();
        assert_eq!(ordinary.len(), 18);
        assert!(ordinary.windows(2).all(|w| w[0] <= w[1]));
        let per = |m: u16| ordinary.iter().filter(|&&x| x == m).count();
        assert_eq!((per(1), per(2), per(3)), (8, 8, 2));
        // Without exempt steps it is the old schedule.
        let none = vec![false; 22];
        assert_eq!(schedule_free(&none), free_action_months(22));
        // A short list stays in month 0, exempt steps included.
        assert!(schedule_free(&[true, false, true]).iter().all(|&m| m == 0));
    }
}
