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
    BucketAssessment, BucketId, CitationId, Contribution, HazardId, Item, ItemId, Plan,
    PlanItemKind, PriceBand, Target, TierId,
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
        tier_enough: TierId::H72,
        contributions: shares
            .iter()
            .map(|&(hazard, share)| Contribution { hazard, share })
            .collect(),
        frequency_sentences: Vec::new(),
        sources: vec![CitationId::from("stub_research_prototype")],
        relief: None,
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
    }
}

/// Coverage at month `m`, holding the last month's value after the plan ends.
pub fn days_at(r: &BudgetResult, m: usize) -> &BTreeMap<BucketId, f64> {
    &r.coverage_by_month[m.min(r.coverage_by_month.len() - 1)].days
}

pub fn readiness_at(r: &BudgetResult, m: usize) -> &BTreeMap<BucketId, u32> {
    &r.coverage_by_month[m.min(r.coverage_by_month.len() - 1)].readiness_done
}

/// One envelope as reconstructed from the plan lines.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeLife {
    pub item_id: ItemId,
    pub opened: u16,
    pub bought: Option<u16>,
    pub needed: f64,
}

/// Replays the plan's money: every month's budget comes in, purchases go out (less any envelope
/// they draw on), reserve deposits move money into envelopes. Returns the money left at the end
/// of each month (never negative if the plan is sound) and each envelope's life.
pub fn audit(plan: &Plan) -> (Vec<f64>, Vec<EnvelopeLife>) {
    let mut carry = 0.0_f64;
    let mut held: BTreeMap<ItemId, f64> = BTreeMap::new();
    let mut open: BTreeMap<ItemId, (u16, usize)> = BTreeMap::new();
    let mut lives: Vec<EnvelopeLife> = Vec::new();
    let mut used = vec![false; plan.envelopes.len()];
    let mut left = Vec::new();
    for month in &plan.months {
        carry += f64::from(month.budget_usd);
        for it in &month.items {
            match it.kind {
                PlanItemKind::FreeAction => {
                    assert_eq!(it.est_cost_usd, 0.0, "{} is free", it.item_id)
                }
                PlanItemKind::Purchase if it.done => {}
                PlanItemKind::Purchase => {
                    let released = held.remove(&it.item_id).unwrap_or(0.0);
                    carry -= f64::from(it.est_cost_usd) - released;
                    if let Some((_, idx)) = open.remove(&it.item_id) {
                        lives[idx].bought = Some(month.index);
                    }
                }
                PlanItemKind::Reserve => {
                    *held.entry(it.item_id.clone()).or_insert(0.0) += f64::from(it.est_cost_usd);
                    carry -= f64::from(it.est_cost_usd);
                    if !open.contains_key(&it.item_id) {
                        // Envelopes are listed as they close; match by item.
                        let needed = plan
                            .envelopes
                            .iter()
                            .enumerate()
                            .find(|(i, e)| !used[*i] && e.item_id == it.item_id)
                            .map_or(0.0, |(i, e)| {
                                used[i] = true;
                                f64::from(e.needed_usd)
                            });
                        lives.push(EnvelopeLife {
                            item_id: it.item_id.clone(),
                            opened: month.index,
                            bought: None,
                            needed,
                        });
                        open.insert(it.item_id.clone(), (month.index, lives.len() - 1));
                    }
                }
            }
        }
        left.push(carry);
    }
    (left, lives)
}
