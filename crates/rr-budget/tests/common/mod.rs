//! Test fixtures for `rr-budget`: stub curves, a stub catalogue, a random-case generator and a
//! cash-flow audit. Nothing here ships; the real curves come from `rr-consequence` and the real
//! catalogue from `rr-content`.
#![allow(dead_code)]

pub mod generate;
pub mod philadelphia;
pub mod setup;

use std::collections::BTreeMap;

use rr_budget::{BucketCurve, BudgetResult};
use rr_types::math::{self, Z_90};
use rr_types::{
    BucketAssessment, BucketId, CitationId, Contribution, HazardId, Item, ItemId, PlanItemKind,
    PriceBand, Target, TierId,
};

/// One event class of a research-prototype bucket: household rate per year, median and 90th
/// percentile duration in days (lognormal).
#[derive(Debug, Clone, Copy)]
pub struct EventClass {
    pub rate: f64,
    pub median: f64,
    pub p90: f64,
}

pub const fn ev(rate: f64, median: f64, p90: f64) -> EventClass {
    EventClass { rate, median, p90 }
}

/// Λ(d) = Σ rate · P(D > d) for lognormal durations, computed exactly (rm_proto `Lam`).
pub fn lambda_exact(events: &[EventClass], d: f64) -> f64 {
    events
        .iter()
        .map(|e| {
            if d <= 0.0 {
                return e.rate;
            }
            let sigma = (math::ln(e.p90) - math::ln(e.median)) / Z_90;
            e.rate * math::norm_sf((math::ln(d) - math::ln(e.median)) / sigma)
        })
        .sum()
}

/// A dense log grid of durations from 15 minutes to two years, plus every ladder value.
pub fn grid() -> Vec<f64> {
    let mut days: Vec<f64> = (0..=96)
        .map(|i| {
            math::exp(
                math::ln(1.0 / 96.0)
                    + (math::ln(730.0) - math::ln(1.0 / 96.0)) * f64::from(i) / 96.0,
            )
        })
        .collect();
    days.extend(rr_types::TARGET_LADDER_DAYS.iter().map(|&d| f64::from(d)));
    days.sort_by(f64::total_cmp);
    days.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    days
}

/// Tabulates a lognormal mixture as a curve, with the target set to the smallest ladder value
/// whose Λ is at most `dial_rate` (DESIGN §4.4 point 1).
pub fn tabulate(events: &[EventClass], dial_rate: f64) -> BucketCurve {
    let days = grid();
    let lambda: Vec<f64> = days.iter().map(|&d| lambda_exact(events, d)).collect();
    let target = ladder_target(events, dial_rate);
    BucketCurve::new(days, lambda, target)
}

pub fn ladder_target(events: &[EventClass], dial_rate: f64) -> f64 {
    rr_types::TARGET_LADDER_DAYS
        .iter()
        .map(|&d| f64::from(d))
        .find(|&d| lambda_exact(events, d) <= dial_rate)
        .unwrap_or(365.0)
}

/// The continuous target: Λ(d*) = dial_rate, by bisection in log space (rm_proto `target_days`).
pub fn raw_target(events: &[EventClass], dial_rate: f64) -> f64 {
    if lambda_exact(events, 1e-4) <= dial_rate {
        return 0.0;
    }
    let (mut lo, mut hi) = (1e-4_f64, 3650.0_f64);
    for _ in 0..200 {
        let mid = (lo * hi).sqrt();
        if lambda_exact(events, mid) > dial_rate {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    hi
}

pub fn assessment(
    bucket: BucketId,
    target: Target,
    shares: &[(HazardId, f32)],
) -> BucketAssessment {
    BucketAssessment {
        id: bucket,
        name: bucket.name().to_owned(),
        target,
        covered: target,
        covered_today: target,
        tier_enough: TierId::H72,
        contributions: shares
            .iter()
            .map(|&(hazard, share)| Contribution { hazard, share })
            .collect(),
        frequency_sentences: Vec::new(),
        sources: vec![CitationId::from("stub_research_prototype")],
        relief: None,
        stress_test: None,
    }
}

pub fn days_target(d: f64) -> Target {
    let v = d as f32;
    Target::Days {
        value: v,
        low: v,
        high: v,
    }
}

pub fn readiness_target(p_need_10yr: f64) -> Target {
    Target::Readiness {
        p_need_10yr,
        done: 0,
        of: 0,
    }
}

/// A catalogue item with the fields the allocator reads; the rest is filler.
#[allow(clippy::too_many_arguments)]
pub fn item(
    id: &str,
    name: &str,
    unit: &str,
    buckets: &[BucketId],
    tier: TierId,
    free: bool,
    life_safety: bool,
    price: (f32, f32),
) -> Item {
    Item {
        id: ItemId::from(id),
        name: name.to_owned(),
        category: "test".into(),
        unit: unit.to_owned(),
        buckets: buckets.to_vec(),
        tier,
        free,
        life_safety,
        rare_catastrophic: false,
        assumed_basic: false,
        spec: "Test item.".into(),
        look_for: Vec::new(),
        avoid: Vec::new(),
        price_band_usd: PriceBand {
            low: price.0,
            high: price.1,
            per: unit.to_owned(),
            note: Some("illustrative (research risk-model §8.6)".into()),
        },
        retrieved: None,
        quantity_rule: "once".into(),
        maintenance: None,
        citations: vec![CitationId::from("stub_research_prototype")],
        hazard_extras: Vec::new(),
        energy_kcal_per_unit: None,
        volume_l_per_unit: None,
        requires: Vec::new(),
        readiness_share: None,
        decision: false,
        long_horizon: false,
        season: None,
        test_interval_months: None,
    }
}

/// Coverage at month `m`, holding the last month's value after the plan ends.
pub fn days_at(r: &BudgetResult, m: usize) -> &BTreeMap<BucketId, f64> {
    &r.coverage_by_month[m.min(r.coverage_by_month.len() - 1)].days
}

pub fn readiness_at(r: &BudgetResult, m: usize) -> &BTreeMap<BucketId, u32> {
    &r.coverage_by_month[m.min(r.coverage_by_month.len() - 1)].readiness_done
}

/// One sinking fund as reconstructed from the plan: when money first went in, what the deposits
/// were last labelled for, and the purchase that drew on it.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeLife {
    /// The item the deposits were (last) for.
    pub item_id: ItemId,
    pub opened: u16,
    /// Month of the purchase that drew on the fund.
    pub bought: Option<u16>,
    /// The item that purchase was for (normally `item_id`).
    pub bought_item: Option<ItemId>,
    /// Cost of that purchase.
    pub needed: f64,
    /// The deposits changed item along the way (the first item was no longer needed).
    pub retargeted: bool,
}

/// Replays the plan's money. Each month's budget comes in as free money; a reserve line moves free
/// money into a sinking fund; a purchase takes `from_savings_usd` from the fund and the rest from
/// free money (read from `BudgetResult::sequence`, which lists every purchase before same-month
/// lines are merged). Returns the free money left at the end of each month (never negative if the
/// plan is sound) and each fund's life. `rare` tells which items belong to the rare-catastrophe
/// allowance, whose fund runs beside the main plan's. Panics if a fund would go negative.
pub fn audit(r: &BudgetResult, rare: &dyn Fn(&str) -> bool) -> (Vec<f64>, Vec<EnvelopeLife>) {
    let plan = &r.plan;
    let mut free = 0.0_f64;
    let mut fund = 0.0_f64;
    // Open fund per track: [main, rare].
    let mut open: [Option<usize>; 2] = [None, None];
    let mut lives: Vec<EnvelopeLife> = Vec::new();
    let mut left = Vec::new();
    for month in &plan.months {
        free += f64::from(month.budget_usd);
        // Purchases of this month, in order, each consumed once.
        let mut buys: Vec<&rr_budget::Purchase> = r
            .sequence
            .iter()
            .filter(|p| p.month == month.index)
            .collect();
        for it in &month.items {
            match it.kind {
                PlanItemKind::FreeAction => {
                    assert_eq!(it.est_cost_usd, 0.0, "{} is free", it.item_id)
                }
                PlanItemKind::Purchase if it.done => {}
                PlanItemKind::Purchase => {
                    // A merged line stands for every purchase of this item in the month.
                    let mine: Vec<&rr_budget::Purchase> = buys
                        .iter()
                        .copied()
                        .filter(|p| p.item_id == it.item_id)
                        .collect();
                    buys.retain(|p| p.item_id != it.item_id);
                    let line_cost: f64 = mine.iter().map(|p| p.cost_usd).sum();
                    assert!(
                        (line_cost - f64::from(it.est_cost_usd)).abs() < 0.02,
                        "month {} {}: line ${} vs purchases ${line_cost}",
                        month.index,
                        it.item_id,
                        it.est_cost_usd
                    );
                    for p in mine {
                        fund -= p.from_savings_usd;
                        free -= p.cost_usd - p.from_savings_usd;
                        assert!(fund > -0.05, "month {}: fund {fund}", month.index);
                        if p.from_savings_usd > 1e-9 {
                            let track = usize::from(p.rare_catastrophic);
                            if let Some(i) = open[track].take() {
                                lives[i].bought = Some(month.index);
                                lives[i].bought_item = Some(p.item_id.clone());
                                lives[i].needed = p.cost_usd;
                            }
                        }
                    }
                }
                PlanItemKind::Reserve => {
                    let d = f64::from(it.est_cost_usd);
                    free -= d;
                    fund += d;
                    let track = usize::from(rare(it.item_id.as_str()));
                    match open[track] {
                        Some(i) if lives[i].item_id != it.item_id => {
                            lives[i].item_id = it.item_id.clone();
                            lives[i].retargeted = true;
                        }
                        Some(_) => {}
                        None => {
                            lives.push(EnvelopeLife {
                                item_id: it.item_id.clone(),
                                opened: month.index,
                                bought: None,
                                bought_item: None,
                                needed: 0.0,
                                retargeted: false,
                            });
                            open[track] = Some(lives.len() - 1);
                        }
                    }
                }
            }
        }
        assert!(
            buys.is_empty(),
            "month {}: purchases without plan lines: {buys:?}",
            month.index
        );
        left.push(free);
    }
    // What a fund still open at the end needs: the main plan's is the last month's `saving_for`
    // (its envelope may also count earlier purchases of the same item); a rare-catastrophe item is
    // bought once, so its envelope is only the open fund.
    let open_main = r.money_by_month.last().and_then(|mm| mm.saving_for.clone());
    for life in lives.iter_mut().filter(|l| l.bought.is_none()) {
        let main = open_main
            .as_ref()
            .filter(|(id, _)| *id == life.item_id)
            .map(|(_, cost)| *cost);
        let envelope = plan
            .envelopes
            .iter()
            .find(|e| e.item_id == life.item_id)
            .map(|e| f64::from(e.needed_usd));
        if let Some(needed) = main.or(envelope) {
            life.needed = needed;
        }
    }
    (left, lives)
}
