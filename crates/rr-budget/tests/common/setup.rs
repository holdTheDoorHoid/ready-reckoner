//! A small builder for hand-built allocator cases with flat curves.

use rr_budget::{
    BucketCurve, BudgetInput, BudgetOptions, BudgetResult, Contributes, GuardrailContext, ItemMeta,
    Risks, allocate,
};
use rr_types::{BucketId, Item, PlanInput, RequirementLine, Target, TierId, fixtures};

use super::item;

pub struct Setup {
    pub household: PlanInput,
    pub items: Vec<Item>,
    pub meta: Vec<ItemMeta>,
    pub risks: Risks,
    pub context: GuardrailContext,
    pub requirements: Vec<RequirementLine>,
    pub options: BudgetOptions,
}

impl Setup {
    pub fn new(fixture: &str, monthly: f32, one_off: f32) -> Self {
        let mut household = fixtures::get(fixture).unwrap();
        household.finances.monthly_budget_usd = monthly;
        household.finances.one_off_budget_usd = one_off;
        household.existing.clear();
        Setup {
            household,
            items: Vec::new(),
            meta: Vec::new(),
            risks: Risks::default(),
            context: GuardrailContext::default(),
            requirements: Vec::new(),
            options: BudgetOptions::default(),
        }
    }
    pub fn flat(mut self, bucket: BucketId, lambda: f64, target: f64) -> Self {
        self.risks
            .curves
            .insert(bucket, BucketCurve::new(vec![1.0], vec![lambda], target));
        self
    }
    pub fn readiness(mut self, bucket: BucketId, p_need_10yr: f64) -> Self {
        let target = if bucket == BucketId::Evacuate {
            Target::Evacuate {
                p_need_10yr,
                notice_hours_low: 1.0,
                notice_hours_high: 24.0,
                days_away: 3.0,
            }
        } else {
            super::readiness_target(p_need_10yr)
        };
        self.risks
            .assessments
            .insert(bucket, super::assessment(bucket, target, &[]));
        self
    }
    pub fn add(mut self, it: Item, m: ItemMeta) -> Self {
        self.items.push(it);
        self.meta.push(m);
        self
    }
    pub fn run(&self) -> BudgetResult {
        allocate(&self.input()).unwrap()
    }
    pub fn input(&self) -> BudgetInput<'_> {
        BudgetInput {
            household: &self.household,
            catalogue: &self.items,
            meta: &self.meta,
            requirements: &self.requirements,
            risks: &self.risks,
            context: &self.context,
            options: self.options,
        }
    }
}

pub fn set(id: &str, bucket: BucketId, days: f64) -> ItemMeta {
    let mut m = ItemMeta::new(id);
    m.contributes = vec![Contributes::per_household(bucket, days)];
    m
}

pub fn divisible(id: &str, c: Contributes, step: f64) -> ItemMeta {
    let mut m = ItemMeta::new(id);
    m.contributes = vec![c];
    m.step = Some(step);
    m
}

pub fn buy(id: &str, bucket: BucketId, tier: TierId, price: f32) -> Item {
    item(
        id,
        id,
        "each",
        &[bucket],
        tier,
        false,
        false,
        (price, price),
    )
}
