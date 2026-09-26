//! The allocator (DESIGN §4.7, research risk-model §4.2).
//!
//! 1. **Month 0** applies what the household already has (`existing`), then every free action,
//!    most valuable first.
//! 2. **What to buy next.** Walk the tiers (three days, two weeks, one month, three, six, twelve
//!    months) with every bucket's target capped at the tier's horizon. The first tier with a
//!    positive-value candidate is the current tier. Candidates are items unlocked at or below it:
//!    a set not yet bought, or the next chunk of a divisible item (water, food, medicine) up to the
//!    next step on the day ladder. A later-tier item joins when its value per dollar is at least
//!    five times the tier's best (promotion). Life-safety items come first, then value per dollar.
//! 3. **When to buy it** ([`Schedule`]). Strict (default): buy the next item when the money is
//!    there, otherwise save for it, in an envelope when it costs more than a month's budget. The
//!    research shortcuts buy the best affordable item instead unless the sinking-fund rule says
//!    to wait.
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
use crate::curve::BucketCurve;
use crate::explain::{self, DurationText, Lead, ReadinessText, WhyParts};
use crate::guardrails::{self, Facts};
use crate::input::{BudgetError, BudgetInput, BudgetResult, MonthCoverage, Purchase, Schedule};
use crate::savings;
use crate::value::{
    PROMOTION_FACTOR, RARE_CATASTROPHIC_SHARE, READINESS_MIN_P_NEED_10YR, SINKING_FUND_MAX_MONTHS,
    SINKING_FUND_VALUE_RATIO, annual_rate_from_10yr, duration_value, per_100, readiness_value,
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
        cand: Candidate,
        rare: bool,
    },
    Reserve {
        offer: usize,
        tier: TierId,
        deposit: f64,
        saved: f64,
        needed: f64,
    },
}

/// Money set aside for one item.
#[derive(Debug, Clone)]
struct Envelope {
    offer: usize,
    qty: f64,
    saved: f64,
}

fn run(
    input: &BudgetInput<'_>,
    meta: &[ItemMeta],
    rule: &dyn CoverageRule,
) -> Result<BudgetResult, BudgetError> {
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
        month0_todo_free.push(Event::Free { cand, done: false });
    }

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
    let mut sequence: Vec<Purchase> = Vec::new();
    let mut coverage_by_month: Vec<MonthCoverage> = Vec::new();
    let mut envelopes: Vec<SavingsEnvelope> = Vec::new();
    let mut purchase_months: BTreeMap<usize, u16> = BTreeMap::new();
    let mut cash = 0.0_f64;
    let mut rare_cash = 0.0_f64;
    let mut env: Option<Envelope> = None;
    let mut rare_env: Option<Envelope> = None;
    let mut stopped: Option<u16> = None;
    let schedule = input.options.schedule;
    let future_money = monthly > EPS;

    for m in 0..=input.options.max_months {
        if m > 0 && !future_money {
            break;
        }
        let mut month_events: Vec<Event> = Vec::new();
        let new = if m == 0 { one_off } else { monthly };
        let rare_new = if rare_queue.is_empty() {
            0.0
        } else {
            RARE_CATASTROPHIC_SHARE * new
        };
        cash += new - rare_new;
        rare_cash += rare_new;

        // Main track.
        while stopped.is_none() {
            let Some((_, cands)) = ordered_candidates(&ctx, &state) else {
                stopped = Some(m);
                break;
            };
            let top = &cands[0];
            if top.cost <= cash + EPS {
                let cand = top.clone();
                buy(
                    &ctx,
                    &mut state,
                    &mut cash,
                    &mut env,
                    &mut envelopes,
                    &cand,
                    m,
                    false,
                    &mut sequence,
                    &mut purchase_months,
                    &mut month_events,
                );
                continue;
            }
            // A sinking fund, once started, is kept until the item is bought.
            let saving_for_top = env.as_ref().is_some_and(|e| e.offer == top.offer);
            let pick: Option<Candidate> = if !future_money {
                // No later month brings money: saving is pointless, buy the best that fits.
                cands.iter().find(|c| c.cost <= cash + EPS).cloned()
            } else if schedule == Schedule::ResearchShortcuts && !saving_for_top {
                cands.iter().find(|c| c.cost <= cash + EPS).and_then(|p| {
                    let wait = top.cost <= SINKING_FUND_MAX_MONTHS * monthly + EPS
                        && p.density() < SINKING_FUND_VALUE_RATIO * top.density();
                    (!wait).then(|| p.clone())
                })
            } else {
                None
            };
            if let Some(cand) = pick {
                buy(
                    &ctx,
                    &mut state,
                    &mut cash,
                    &mut env,
                    &mut envelopes,
                    &cand,
                    m,
                    false,
                    &mut sequence,
                    &mut purchase_months,
                    &mut month_events,
                );
                continue;
            }
            if future_money && top.cost > monthly + EPS {
                save_toward(&mut env, top, cash, &mut month_events);
            }
            break;
        }

        // Rare-catastrophe allowance: its own queue, strictly in order.
        while let Some(next) = rare_queue.last() {
            if next.cost <= rare_cash + EPS {
                let cand = rare_queue.pop().expect("checked");
                buy(
                    &ctx,
                    &mut state,
                    &mut rare_cash,
                    &mut rare_env,
                    &mut envelopes,
                    &cand,
                    m,
                    true,
                    &mut sequence,
                    &mut purchase_months,
                    &mut month_events,
                );
                continue;
            }
            if !future_money {
                let fits = rare_queue.iter().rposition(|c| c.cost <= rare_cash + EPS);
                if let Some(pos) = fits {
                    let cand = rare_queue.remove(pos);
                    buy(
                        &ctx,
                        &mut state,
                        &mut rare_cash,
                        &mut rare_env,
                        &mut envelopes,
                        &cand,
                        m,
                        true,
                        &mut sequence,
                        &mut purchase_months,
                        &mut month_events,
                    );
                    continue;
                }
            } else if next.cost > RARE_CATASTROPHIC_SHARE * monthly + EPS {
                let next = next.clone();
                save_toward(&mut rare_env, &next, rare_cash, &mut month_events);
            }
            break;
        }
        if rare_queue.is_empty() && rare_cash > 0.0 {
            cash += rare_cash;
            rare_cash = 0.0;
        }

        coverage_by_month.push(snapshot(&ctx, &state, m));
        events.push(month_events);
        if stopped.is_some() && rare_queue.is_empty() {
            break;
        }
    }
    // Envelopes still open when the plan ends are reported with what they hold.
    for e in [env, rare_env].into_iter().flatten() {
        if e.saved > EPS {
            envelopes.push(SavingsEnvelope {
                item_id: ctx.offers[e.offer].item.id.clone(),
                saved_usd: money(e.saved),
                needed_usd: money(e.qty * ctx.offers[e.offer].unit_price),
            });
        }
    }

    // ---- Assemble the plan. ----
    let mut first = Vec::new();
    first.extend(month0_todo_free);
    first.extend(month0_done_free);
    first.extend(month0_owned);
    if events.is_empty() {
        events.push(Vec::new());
        coverage_by_month.push(snapshot(&ctx, &state, 0));
    }
    let mut month0 = first;
    month0.append(&mut events[0]);
    events[0] = month0;
    while events.len() > 1 && events.last().is_some_and(|e| e.is_empty()) {
        events.pop();
        coverage_by_month.pop();
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

    let covered = covered_targets(&ctx, &state);
    let facts = guardrail_facts(&ctx, &state, &coverage_by_month, &purchase_months, stopped);
    let warnings = guardrails::check(input.household, input.context, input.risks, &facts);

    Ok(BudgetResult {
        plan,
        covered,
        coverage_by_month,
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

fn curve<'a>(ctx: &Ctx<'a>, bucket: BucketId) -> &'a BucketCurve {
    &ctx.input.risks.curves[&bucket]
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
        let value = duration_value(tr.weight, tr.share, curve(ctx, tr.bucket), x0, x0 + dx, cap);
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
        Some(_) => chunk_qty(ctx, state, i, horizon)?,
    };
    let mut c = evaluate_fixed(ctx, state, i, qty, tier, horizon);
    c.value = c.core;
    Some(c)
}

/// The current tier and its candidates in buying order, or `None` when nothing is worth buying.
fn ordered_candidates(ctx: &Ctx<'_>, state: &State) -> Option<(TierId, Vec<Candidate>)> {
    let main: Vec<usize> = (0..ctx.offers.len())
        .filter(|&i| !ctx.offers[i].item.free && !ctx.offers[i].item.rare_catastrophic)
        .collect();
    let unlocked = |i: usize, k: TierId| ctx.offers[i].item.tier.max(TierId::H72) <= k;
    for (ki, &k) in WALK.iter().enumerate() {
        let evals: Vec<Candidate> = main
            .iter()
            .filter(|&&i| unlocked(i, k))
            .filter_map(|&i| evaluate(ctx, state, i, k))
            .collect();
        let best = evals
            .iter()
            .filter(|c| c.core > VALUE_EPS)
            .map(|c| density(c.core, c.cost))
            .fold(None, |acc: Option<f64>, d| {
                Some(acc.map_or(d, |a| a.max(d)))
            });
        let Some(best) = best else {
            continue;
        };
        let mut cands: Vec<Candidate> = Vec::new();
        for mut c in evals {
            // A capability needed less often than the threshold joins only when it beats the
            // tier's best item on value per dollar (DESIGN §4.4).
            if c.low_p > VALUE_EPS && density(c.core + c.low_p, c.cost) > best {
                c.value = c.core + c.low_p;
                cands.push(c);
            } else if c.core > VALUE_EPS {
                c.value = c.core;
                cands.push(c);
            }
        }
        let mut taken: BTreeSet<usize> = cands.iter().map(|c| c.offer).collect();
        for &j in &WALK[ki + 1..] {
            for &i in &main {
                if taken.contains(&i) || !unlocked(i, j) {
                    continue;
                }
                if let Some(mut c) = evaluate(ctx, state, i, j) {
                    if c.core > VALUE_EPS && density(c.core, c.cost) >= PROMOTION_FACTOR * best {
                        c.promoted = true;
                        taken.insert(i);
                        cands.push(c);
                    }
                }
            }
        }
        cands.sort_by(|a, b| {
            let (la, lb) = (
                ctx.offers[a.offer].item.life_safety,
                ctx.offers[b.offer].item.life_safety,
            );
            lb.cmp(&la)
                .then(b.density().total_cmp(&a.density()))
                .then(a.tier.cmp(&b.tier))
                .then(a.offer.cmp(&b.offer))
        });
        return Some((k, cands));
    }
    None
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

#[allow(clippy::too_many_arguments)]
fn buy(
    ctx: &Ctx<'_>,
    state: &mut State,
    cash: &mut f64,
    env: &mut Option<Envelope>,
    envelopes: &mut Vec<SavingsEnvelope>,
    cand: &Candidate,
    month: u16,
    rare: bool,
    sequence: &mut Vec<Purchase>,
    purchase_months: &mut BTreeMap<usize, u16>,
    events: &mut Vec<Event>,
) {
    if let Some(e) = env.take() {
        if e.offer == cand.offer {
            envelopes.push(SavingsEnvelope {
                item_id: ctx.offers[e.offer].item.id.clone(),
                saved_usd: money(e.saved),
                needed_usd: money(cand.cost),
            });
        }
        // Money in an envelope for another item stays in `cash`; the envelope simply closes.
    }
    *cash = (*cash - cand.cost).max(0.0);
    apply(ctx, state, cand);
    purchase_months.entry(cand.offer).or_insert(month);
    sequence.push(Purchase {
        month,
        item_id: ctx.offers[cand.offer].item.id.clone(),
        quantity: cand.qty,
        cost_usd: cand.cost,
        value: cand.value,
        tier: cand.tier,
        promoted: cand.promoted,
        rare_catastrophic: rare,
    });
    events.push(Event::Buy {
        cand: cand.clone(),
        rare,
    });
}

/// Puts everything on hand toward `top` (a sinking fund), recording this month's deposit.
fn save_toward(env: &mut Option<Envelope>, top: &Candidate, cash: f64, events: &mut Vec<Event>) {
    let previous = match env {
        Some(e) if e.offer == top.offer => e.saved,
        _ => 0.0,
    };
    let deposit = cash - previous;
    *env = Some(Envelope {
        offer: top.offer,
        qty: top.qty,
        saved: cash,
    });
    if deposit > EPS {
        events.push(Event::Reserve {
            offer: top.offer,
            tier: top.tier,
            deposit,
            saved: cash,
            needed: top.cost,
        });
    }
}

fn snapshot(ctx: &Ctx<'_>, state: &State, month: u16) -> MonthCoverage {
    let h = ctx.input.household;
    let mut days = BTreeMap::new();
    for b in BucketId::ALL
        .iter()
        .filter(|b| b.kind() == BucketKind::Duration)
    {
        days.insert(*b, ctx.rule.coverage(*b, &state.inventory, h));
    }
    let mut readiness_done = BTreeMap::new();
    for b in BucketId::ALL
        .iter()
        .filter(|b| b.kind() != BucketKind::Duration)
    {
        let n = ctx
            .offers
            .iter()
            .enumerate()
            .filter(|(i, o)| o.item.buckets.contains(b) && state.owned[*i] > 0.0)
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
        if let Event::Buy { cand, rare } = e {
            let found = merged.iter_mut().find_map(|m| match m {
                Event::Buy { cand: c, rare: r } if c.offer == cand.offer && r == rare => Some(c),
                _ => None,
            });
            if let Some(c) = found {
                merge_into(c, cand);
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
        Event::Buy { cand, rare } => {
            let lead = if *rare {
                Lead::RareAllowance
            } else {
                Lead::Purchase
            };
            line(
                ctx,
                cand,
                PlanItemKind::Purchase,
                cand.tier,
                lead,
                false,
                None,
            )
        }
        Event::Reserve {
            offer,
            tier,
            deposit,
            saved,
            needed,
        } => {
            let item = ctx.offers[*offer].item;
            let why = format!(
                "Sets aside {} toward {} ({} of {} saved). It costs more than a month's budget, so \
                 the plan saves for it instead of buying something worth less.",
                explain::dollars(*deposit),
                lower_first(&item.name),
                explain::dollars(*saved),
                explain::dollars(*needed),
            );
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

fn covered_targets(ctx: &Ctx<'_>, state: &State) -> BTreeMap<BucketId, Target> {
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
                let of = ctx
                    .offers
                    .iter()
                    .filter(|o| o.item.buckets.contains(b))
                    .count();
                let done = ctx
                    .offers
                    .iter()
                    .enumerate()
                    .filter(|(i, o)| o.item.buckets.contains(b) && state.owned[*i] > 0.0)
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
