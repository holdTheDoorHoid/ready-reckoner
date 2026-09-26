//! The allocator (DESIGN §4.7, research risk-model §4.2).
//!
//! 1. **Free actions.** What the household already has (`existing`) counts from month 0. Free
//!    actions still to do are ordered life-safety first, then by value; month 0 lists at most eight
//!    and the rest follow in months 1 and 2 (about four a month, before that month's purchases),
//!    so no step shows more than eight. Their coverage counts in the plan's numbers from their
//!    month; purchases are planned knowing they are coming.
//! 2. **What to buy next.** Walk the tiers (three days, two weeks, one month, three, six, twelve
//!    months) with every bucket's target capped at the tier's horizon. The first tier with a
//!    positive-value candidate is the current tier. Candidates are items unlocked at or below it:
//!    a set not yet bought, or the next chunk of a divisible item (water, food, medicine) up to the
//!    next step on the day ladder. A later-tier item joins when its value per dollar is at least
//!    five times the tier's best (promotion). Life-safety items come first, then value per dollar.
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
//! Specialised rare-catastrophe items never enter step 2. On opt-in they are bought, in order of
//! value per dollar, from a separate allowance of 10 % of each month's money.

use std::collections::{BTreeMap, BTreeSet};

use rr_types::{
    BucketId, BucketKind, CostRange, HazardId, Item, ItemId, Plan, PlanItem, PlanItemKind,
    PlanMonth, SavingsEnvelope, TARGET_LADDER_DAYS, Target, TierId,
};

use crate::coverage::{ContributionTable, CoverageRule, ItemMeta, ItemRole, apply_requirements};
use crate::curve::Prepared;
use crate::explain::{self, DurationText, Lead, ReadinessText, WhyParts};
use crate::guardrails::{self, Facts};
use crate::input::{
    BudgetError, BudgetInput, BudgetResult, MonthCoverage, MonthMoney, Purchase, Schedule,
};
use crate::savings;
use crate::value::{
    PROMOTION_FACTOR, RARE_CATASTROPHIC_SHARE, READINESS_MIN_P_NEED_10YR, SINKING_FUND_MAX_MONTHS,
    SINKING_FUND_VALUE_RATIO, annual_rate_from_10yr, duration_value_with, per_100, readiness_value,
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

/// Money and day comparisons tolerate this much rounding.
const EPS: f64 = 1e-9;

/// Values at or below this are treated as zero (no value).
const VALUE_EPS: f64 = 1e-12;

/// At most this many free actions still to do in month 0 (and in any month): the prior-art research
/// on choice overload and one-next-action puts the useful limit at 7 to 10 per step.
pub const FREE_ACTIONS_MONTH_0: usize = 8;

/// Free actions left over from month 0 are scheduled at about this many a month.
pub const FREE_ACTIONS_PER_MONTH: usize = 4;

/// Every free action is scheduled by this month (the first three months are 0, 1 and 2).
pub const FREE_ACTIONS_BY_MONTH: u16 = 2;

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
    let mut state = State {
        inventory: Vec::new(),
        track_cov: vec![0.0; ctx.tracks.len()],
        owned: vec![0.0; ctx.offers.len()],
        readiness_used: vec![false; ctx.offers.len()],
    };
    for t in 0..ctx.tracks.len() {
        state.track_cov[t] = track_coverage(&ctx, t, &state.inventory);
    }

    // ---- Month 0: what the household has, then free actions. ----
    let mut month0_owned: Vec<Event> = Vec::new();
    let mut month0_done_free: Vec<Event> = Vec::new();
    let mut month0_todo_free: Vec<Event> = Vec::new();
    let existing = existing_by_offer(&ctx);
    for (i, offer) in ctx.offers.iter().enumerate() {
        let Some((qty, paid)) = existing.get(&i).copied() else {
            continue;
        };
        if offer.item.free {
            continue;
        }
        let cand = evaluate_fixed(&ctx, &state, i, qty, TierId::Y1, f64::INFINITY);
        apply(&ctx, &mut state, &cand);
        month0_owned.push(Event::Owned { cand, paid });
    }
    let free: Vec<usize> = (0..ctx.offers.len())
        .filter(|&i| ctx.offers[i].item.free)
        .collect();
    for &i in &free {
        let done = existing.get(&i).is_some_and(|(q, _)| *q >= 1.0);
        if done {
            let qty = ctx.offers[i].set_quantity.max(1.0);
            let cand = evaluate_fixed(&ctx, &state, i, qty, TierId::Now, f64::INFINITY);
            apply(&ctx, &mut state, &cand);
            month0_done_free.push(Event::Free { cand, done: true });
        }
    }
    let so_far = state.clone();
    let mut free_order: Vec<(usize, f64)> = Vec::new();
    let mut todo: Vec<usize> = free
        .iter()
        .copied()
        .filter(|i| !existing.get(i).is_some_and(|(q, _)| *q >= 1.0))
        .collect();
    while !todo.is_empty() {
        // Most valuable first (life-safety first), so each action's value is its marginal value.
        let mut best: Option<(usize, Candidate)> = None;
        for (pos, &i) in todo.iter().enumerate() {
            let qty = ctx.offers[i].set_quantity.max(1.0);
            let c = evaluate_fixed(&ctx, &state, i, qty, TierId::Now, f64::INFINITY);
            let better = match &best {
                None => true,
                Some((_, b)) => {
                    let (li, lb) = (
                        ctx.offers[i].item.life_safety,
                        ctx.offers[b.offer].item.life_safety,
                    );
                    (li && !lb) || (li == lb && c.value > b.value + VALUE_EPS)
                }
            };
            if better {
                best = Some((pos, c));
            }
        }
        let (pos, cand) = best.expect("todo is not empty");
        todo.remove(pos);
        apply(&ctx, &mut state, &cand);
        free_order.push((cand.offer, cand.qty));
    }
    // Free actions are spread over the first months (at most eight to do in any month). The
    // allocator values purchases as if all of them were done, since they cost nothing and all
    // come within three months, so it never buys what a scheduled free step will cover; the plan's
    // coverage numbers (`credited`) count each one only from its month.
    let free_months = free_action_months(free_order.len());
    let last_free_month = free_months.last().copied().unwrap_or(0);
    let scheduled_free: Vec<(u16, usize, f64)> = free_months
        .iter()
        .zip(&free_order)
        .map(|(m, (i, q))| (*m, *i, *q))
        .collect();
    let mut credited = so_far.clone();
    credit_free_actions(
        &ctx,
        &mut credited,
        &scheduled_free,
        0,
        &mut month0_todo_free,
    );

    // ---- Rare-catastrophe allowance. ----
    let finances = &input.household.finances;
    let monthly = f64::from(finances.monthly_budget_usd).max(0.0);
    let one_off = f64::from(finances.one_off_budget_usd).max(0.0);
    let rare_offers: Vec<usize> = (0..ctx.offers.len())
        .filter(|&i| !ctx.offers[i].item.free && ctx.offers[i].item.rare_catastrophic)
        .collect();
    let mut rare_skipped: Vec<ItemId> = Vec::new();
    let mut rare_queue: Vec<Candidate> = Vec::new();
    if input.options.rare_catastrophic_opt_in {
        for &i in &rare_offers {
            let remaining = remaining_set_qty(&ctx, &state, i);
            if remaining > EPS {
                rare_queue.push(evaluate_fixed(
                    &ctx,
                    &state,
                    i,
                    remaining,
                    TierId::Y1,
                    f64::INFINITY,
                ));
            }
        }
        rare_queue.sort_by(|a, b| {
            b.density()
                .total_cmp(&a.density())
                .then(a.cost.total_cmp(&b.cost))
                .then(a.offer.cmp(&b.offer))
        });
        rare_queue.reverse(); // pop from the back
    } else {
        rare_skipped = rare_offers
            .iter()
            .map(|&i| ctx.offers[i].item.id.clone())
            .collect();
    }

    // ---- Month by month. ----
    let mut events: Vec<Vec<Event>> = Vec::new();
    let mut coverage_by_month: Vec<MonthCoverage> = Vec::new();
    let mut money_by_month: Vec<MonthMoney> = Vec::new();
    let mut ledger = Ledger::default();
    let mut main = Purse::default();
    let mut rare = Purse::default();
    let mut stopped: Option<u16> = None;
    let schedule = input.options.schedule;
    let future_money = monthly > EPS;
    let mut cache = Cache::new(&ctx);
    cache.screen_rarely_needed(&ctx, &state);
    let checklist = readiness_checklist(
        &ctx,
        &cache.low_p_ok,
        input.options.rare_catastrophic_opt_in,
    );

    for m in 0..=input.options.max_months.max(last_free_month) {
        if m > 0 && !future_money && m > last_free_month {
            break;
        }
        let mut month_events: Vec<Event> = Vec::new();
        if m > 0 {
            credit_free_actions(&ctx, &mut credited, &scheduled_free, m, &mut month_events);
        }
        let new = if m == 0 { one_off } else { monthly };
        let rare_share = if rare_queue.is_empty() {
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
                &ctx,
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
            if let Some((_, picks)) = cache.ordered(&ctx, &state) {
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
            cache.ordered(&ctx, &state).and_then(|(_, picks)| {
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
            let Some((_, picks)) = cache.ordered(&ctx, &state) else {
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
                        &ctx,
                        &mut state,
                        &mut credited,
                        &mut main,
                        use_fund,
                        &mut ledger,
                        &cand,
                        m,
                        false,
                        false,
                        &mut month_events,
                    );
                    cache.invalidate(&ctx, pick.offer);
                }
                None => break,
            }
        }

        // Rare-catastrophe allowance: its own queue, strictly in order.
        while let Some(next) = rare_queue.last() {
            let (free, fund) = (rare.free, rare.fund);
            if next.cost <= free + fund + EPS {
                let cand = rare_queue.pop().expect("checked");
                buy(
                    &ctx,
                    &mut state,
                    &mut credited,
                    &mut rare,
                    true,
                    &mut ledger,
                    &cand,
                    m,
                    true,
                    false,
                    &mut month_events,
                );
                continue;
            }
            if !future_money {
                let fits = rare_queue.iter().rposition(|c| c.cost <= free + EPS);
                if let Some(pos) = fits {
                    let cand = rare_queue.remove(pos);
                    buy(
                        &ctx,
                        &mut state,
                        &mut credited,
                        &mut rare,
                        false,
                        &mut ledger,
                        &cand,
                        m,
                        true,
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
            _ => snapshot(&ctx, &credited, &checklist, m),
        };
        coverage_by_month.push(snap);
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
    let (sequence, envelopes, purchase_months) = ledger.into_envelopes();

    // ---- Assemble the plan. ----
    let mut first = Vec::new();
    first.extend(month0_todo_free);
    first.extend(month0_done_free);
    first.extend(month0_owned);
    if events.is_empty() {
        events.push(Vec::new());
        coverage_by_month.push(snapshot(&ctx, &credited, &checklist, 0));
        money_by_month.push(MonthMoney {
            month: 0,
            available_usd: main.free,
            saved_usd: main.fund,
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
    let months: Vec<PlanMonth> = events
        .iter()
        .enumerate()
        .map(|(m, evs)| PlanMonth {
            index: m as u16,
            budget_usd: money(if m == 0 { one_off } else { monthly }),
            items: plan_items(&ctx, evs),
        })
        .collect();

    let all_covered = ctx
        .tracks
        .iter()
        .enumerate()
        .all(|(t, tr)| state.track_cov[t] + 1e-6 >= tr.target);
    let done_month = stopped.filter(|_| all_covered);
    let savings_track = savings::track(input.household, input.risks, stopped);
    let plan = Plan {
        months,
        done_month,
        envelopes,
        savings_track,
    };

    let covered = covered_targets(&ctx, &state, &checklist);
    let free_month_of: BTreeMap<usize, u16> =
        scheduled_free.iter().map(|&(m, i, _)| (i, m)).collect();
    let facts = guardrail_facts(
        &ctx,
        &state,
        &coverage_by_month,
        &purchase_months,
        &free_month_of,
        stopped,
    );
    let warnings = guardrails::check(input.household, input.context, input.risks, &facts);

    Ok(BudgetResult {
        plan,
        covered,
        coverage_by_month,
        money_by_month,
        sequence,
        tier_reached: tier_met(&ctx, &so_far.track_cov),
        tier_at_plan_end: tier_met(&ctx, &state.track_cov),
        tier_recommended: tier_recommended(&ctx),
        warnings,
        rare_catastrophic_skipped: rare_skipped,
        stopped_month: stopped,
    })
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
        offers.push(Offer {
            item,
            unit_price,
            set_quantity: m.and_then(|m| m.set_quantity).unwrap_or(1.0),
            step: m.and_then(|m| m.step),
            tracks: track_ids,
            readiness: m
                .map(|m| {
                    m.readiness
                        .iter()
                        .map(|r| (r.bucket, r.harm_day_equivalents))
                        .collect()
                })
                .unwrap_or_default(),
            roles: m.map(|m| m.roles.clone()).unwrap_or_default(),
        });
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
    if !state.readiness_used[i] {
        for &(b, harm) in &o.readiness {
            let p = ctx.input.risks.p_need_10yr(b);
            let v = readiness_value(ctx.weights[&b], p, harm);
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
/// offer's own purchase, or a purchase touching one of the offer's tracks. (This is why a
/// [`CoverageRule`]'s answer for a bucket may depend only on items that serve that bucket.)
struct Cache {
    slots: Vec<[Option<Option<Candidate>>; WALK.len()]>,
    /// Offers touching each track.
    by_track: Vec<Vec<usize>>,
    /// Offers the main plan can buy (not free, not rare-catastrophe).
    main: Vec<usize>,
    /// Per offer and tier, a number that changes only where the offer's capped targets change: an
    /// offer valued at two tiers with the same number has the same valuation at both.
    cap_group: Vec<[u8; WALK.len()]>,
    /// The buying order for the current state, until the next purchase.
    ordered: Option<Option<(TierId, Vec<Pick>)>>,
    /// Per offer: a capability needed less often than the threshold counts in its value
    /// (screened once, against the household's position before any purchase; see
    /// [`Cache::screen_rarely_needed`]).
    low_p_ok: Vec<bool>,
}

impl Cache {
    fn new(ctx: &Ctx<'_>) -> Self {
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
        }
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
        let mut c = self.slots[p.offer][p.ti]
            .as_ref()
            .and_then(|c| c.as_ref())
            .expect("a pick points at a computed valuation")
            .clone();
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
    }

    /// The current tier and its candidates in buying order (`None` when nothing is worth
    /// buying), computed once per state.
    fn ordered(&mut self, ctx: &Ctx<'_>, state: &State) -> Option<&(TierId, Vec<Pick>)> {
        if self.ordered.is_none() {
            let o = self.compute_order(ctx, state);
            self.ordered = Some(o);
        }
        self.ordered.as_ref().and_then(|o| o.as_ref())
    }

    fn compute_order(&mut self, ctx: &Ctx<'_>, state: &State) -> Option<(TierId, Vec<Pick>)> {
        let unlocked = |i: usize, k: TierId| ctx.offers[i].item.tier.max(TierId::H72) <= k;
        let main = std::mem::take(&mut self.main);
        let mut result = None;
        for (ki, &k) in WALK.iter().enumerate() {
            // Candidates of tier k with their value: duration value plus readiness value, where a
            // capability needed less often than the threshold counts only if it passed the
            // screening (DESIGN §4.4).
            let mut picks: Vec<Pick> = Vec::new();
            for &i in main.iter().filter(|&&i| unlocked(i, k)) {
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
                        });
                    }
                }
            }
            let Some(best) = picks.iter().map(Pick::density).reduce(f64::max) else {
                continue;
            };
            // Promotion: a later-tier item at least five times the tier's best per dollar joins,
            // valued at the first later tier where it qualifies. Tiers at which an offer's capped
            // targets do not change give the same valuation, so only one of them is looked at.
            let taken: BTreeSet<usize> = picks.iter().map(|p| p.offer).collect();
            for &i in &main {
                if taken.contains(&i) {
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
                            });
                            break;
                        }
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
                    .then(a.ti.cmp(&b.ti))
                    .then(a.offer.cmp(&b.offer))
            });
            result = Some((k, picks));
            break;
        }
        self.main = main;
        result
    }
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
    });
    events.push(Event::Buy {
        cand: cand.clone(),
        shown,
        rare,
        from_savings,
        one_off_first,
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
        let Some((_, picks)) = books.cache.ordered(ctx, books.state) else {
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
            let Some((_, picks)) = books.cache.ordered(ctx, books.state) else {
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
    checklist: &[Vec<BucketId>],
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
        let n = (0..ctx.offers.len())
            .filter(|&i| checklist[i].contains(b) && state.owned[i] > 0.0)
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
fn plan_items(ctx: &Ctx<'_>, events: &[Event]) -> Vec<PlanItem> {
    // Merge Buy events per (offer, rare) into the first occurrence.
    let mut merged: Vec<Event> = Vec::new();
    for e in events {
        if let Event::Buy {
            cand,
            shown,
            rare,
            from_savings,
            one_off_first,
        } = e
        {
            let found = merged.iter_mut().find_map(|m| match m {
                Event::Buy {
                    cand: c,
                    shown: sh,
                    rare: r,
                    from_savings: f,
                    one_off_first: o,
                } if c.offer == cand.offer && r == rare => Some((c, sh, f, o)),
                _ => None,
            });
            if let Some((c, sh, f, o)) = found {
                merge_into(c, cand);
                merge_into(sh, shown);
                *f += from_savings;
                *o |= *one_off_first;
                continue;
            }
        }
        merged.push(e.clone());
    }
    merged.iter().map(|e| plan_item(ctx, e)).collect()
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

fn plan_item(ctx: &Ctx<'_>, e: &Event) -> PlanItem {
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
            // Explain with coverage as the plan reports it (free actions count from their month).
            let parts = why_parts(ctx, shown);
            item.why = explain::why(lead, &parts, ctx.people, ctx.years);
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
                annual_rate_from_10yr(ctx.input.risks.p_need_10yr(r.0)),
                years,
            ),
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

/// The checklist of each readiness (and money) bucket, as buckets per offer: the free actions that
/// name it, the items whose readiness credit for it counts (needed often enough, or screened in),
/// and, with the opt-in, the specialised rare-catastrophe items that name it. An item bought for
/// another need that merely lists the bucket is not a step on its checklist.
fn readiness_checklist(ctx: &Ctx<'_>, low_p_ok: &[bool], rare_opt_in: bool) -> Vec<Vec<BucketId>> {
    let named = |o: &Offer<'_>| -> Vec<BucketId> {
        o.item
            .buckets
            .iter()
            .copied()
            .filter(|b| b.kind() != BucketKind::Duration)
            .collect()
    };
    ctx.offers
        .iter()
        .enumerate()
        .map(|(i, o)| {
            if o.item.free || (o.item.rare_catastrophic && rare_opt_in) {
                named(o)
            } else if o.item.rare_catastrophic {
                Vec::new()
            } else {
                let mut out: Vec<BucketId> = Vec::new();
                for &(b, _) in &o.readiness {
                    let counts =
                        ctx.input.risks.p_need_10yr(b) >= READINESS_MIN_P_NEED_10YR || low_p_ok[i];
                    if counts && !out.contains(&b) {
                        out.push(b);
                    }
                }
                out
            }
        })
        .collect()
}

fn covered_targets(
    ctx: &Ctx<'_>,
    state: &State,
    checklist: &[Vec<BucketId>],
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
                let of = checklist.iter().filter(|c| c.contains(b)).count();
                let done = (0..ctx.offers.len())
                    .filter(|&i| checklist[i].contains(b) && state.owned[i] > 0.0)
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

fn guardrail_facts(
    ctx: &Ctx<'_>,
    state: &State,
    coverage_by_month: &[MonthCoverage],
    purchase_months: &BTreeMap<usize, u16>,
    free_month_of: &BTreeMap<usize, u16>,
    stopped: Option<u16>,
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
    let water_after_month_1 = coverage_by_month
        .iter()
        .take_while(|c| c.month <= 1)
        .last()
        .and_then(|c| c.days.get(&BucketId::WaterOut).copied())
        .unwrap_or(0.0);
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
    Facts {
        device_power_month: first_month(ItemRole::DevicePower),
        cold_chain_month: first_month(ItemRole::ColdChain),
        go_bag_month: first_month(ItemRole::GoBag),
        water_after_month_1,
        uncovered_when_stopped: if stopped.is_some() {
            uncovered
        } else {
            Vec::new()
        },
    }
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
}
