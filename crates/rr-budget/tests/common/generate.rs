//! A tiny random-case generator for property tests, driven by `rr_types::rng::SplitMix64` so every
//! case is reproducible from its seed.

use std::collections::BTreeMap;

use rr_budget::{
    BucketCurve, Cliff, Contributes, GuardrailContext, ItemMeta, ItemRole, ReadinessCredit, Risks,
};
use rr_types::rng::SplitMix64;
use rr_types::{
    AgeBand, BackupPower, BucketId, BucketKind, HazardId, Item, ItemId, Owned, PlanInput,
    PoweredDevice, Target, Tenure, TierId,
};

use super::{assessment, days_target, item, readiness_target};

pub struct Gen(SplitMix64);

impl Gen {
    pub fn new(seed: u64) -> Self {
        Gen(SplitMix64::new(seed))
    }
    pub fn unit(&mut self) -> f64 {
        self.0.next_f64()
    }
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
    pub fn below(&mut self, n: usize) -> usize {
        self.0.next_below(n as u64) as usize
    }
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
    fn cents(&mut self, lo: f64, hi: f64) -> f32 {
        ((self.range(lo, hi) * 100.0).round() / 100.0) as f32
    }
}

/// One random planning problem.
#[derive(Debug, Clone)]
pub struct Case {
    pub seed: u64,
    pub household: PlanInput,
    pub items: Vec<Item>,
    pub meta: Vec<ItemMeta>,
    pub risks: Risks,
    pub context: GuardrailContext,
}

const DURATION: [BucketId; 7] = [
    BucketId::Power,
    BucketId::WaterBoil,
    BucketId::WaterOut,
    BucketId::Supplies,
    BucketId::Thermal,
    BucketId::Medication,
    BucketId::Comms,
];
const READINESS: [BucketId; 5] = [
    BucketId::Evacuate,
    BucketId::GetHome,
    BucketId::MedicalEmergency,
    BucketId::Fire,
    BucketId::Security,
];
const GRID: [f64; 18] = [
    1.0 / 24.0,
    0.125,
    0.25,
    0.5,
    1.0,
    2.0,
    3.0,
    5.0,
    7.0,
    10.0,
    14.0,
    21.0,
    30.0,
    45.0,
    60.0,
    90.0,
    180.0,
    365.0,
];

pub fn case(seed: u64) -> Case {
    let mut g = Gen::new(seed);
    let household = household(&mut g);
    let (items, meta) = catalogue(&mut g);
    let risks = risks(&mut g);
    let mut household = household;
    household.existing = existing(&mut g, &items);
    let context = GuardrailContext {
        flood_zone: g.chance(0.3),
        quake_zone: g.chance(0.2),
        cliffs: if g.chance(0.2) {
            vec![Cliff {
                bucket: g.pick(&DURATION),
                driver: "a rare event".into(),
            }]
        } else {
            Vec::new()
        },
    };
    Case {
        seed,
        household,
        items,
        meta,
        risks,
        context,
    }
}

fn household(g: &mut Gen) -> PlanInput {
    let mut h = PlanInput::defaults();
    let n = 1 + g.below(6);
    let template = h.people[0].clone();
    h.people = (0..n)
        .map(|_| {
            let mut p = template.clone();
            p.age_band = g.pick(&[
                AgeBand::Infant,
                AgeBand::Child,
                AgeBand::Adult,
                AgeBand::Adult,
                AgeBand::Senior,
            ]);
            p.earner = p.age_band == AgeBand::Adult && g.chance(0.7);
            p.medical.daily_rx = g.chance(0.25);
            p.medical.refrigerated_rx = g.chance(0.1);
            p.medical.powered_device = if g.chance(0.1) {
                PoweredDevice::Cpap
            } else {
                PoweredDevice::None
            };
            p
        })
        .collect();
    h.finances.income.earners = h.people.iter().filter(|p| p.earner).count() as u8;
    h.finances.monthly_budget_usd = g.pick(&[0.0, 5.0, 15.0, 30.0, 60.0, 100.0, 250.0, 17.5]);
    h.finances.one_off_budget_usd = g.pick(&[0.0, 0.0, 0.0, 40.0, 200.0, 1000.0]);
    h.finances.emergency_fund_months = g.range(0.0, 6.0) as f32;
    h.finances.monthly_expenses_usd = if g.chance(0.8) {
        Some(g.range(800.0, 6000.0) as f32)
    } else {
        None
    };
    h.housing.tenure = if g.chance(0.5) {
        Tenure::Own
    } else {
        Tenure::Rent
    };
    h.housing.backup_power = if g.chance(0.15) {
        BackupPower::Generator
    } else {
        BackupPower::None
    };
    h.dials.horizon_years = g.pick(&[1, 10, 10, 30]);
    h
}

fn catalogue(g: &mut Gen) -> (Vec<Item>, Vec<ItemMeta>) {
    let n = 6 + g.below(25);
    let mut items = Vec::new();
    let mut meta = Vec::new();
    for k in 0..n {
        let id = format!("item_{k:02}");
        let free = g.chance(0.15);
        let tier = if free {
            TierId::Now
        } else {
            g.pick(&[TierId::H72, TierId::H72, TierId::W2, TierId::M1, TierId::M3])
        };
        let low = if free { 0.0 } else { g.cents(0.0, 80.0) };
        let high = if free { 0.0 } else { low + g.cents(0.0, 60.0) };
        let mut m = ItemMeta::new(id.as_str());
        let mut buckets: Vec<BucketId> = Vec::new();
        if g.chance(0.75) {
            for _ in 0..(1 + g.below(2)) {
                let b = g.pick(&DURATION);
                let rate = g.range(0.1, 6.0);
                let mut c = if g.chance(0.3) {
                    Contributes::per_person(b, rate)
                } else {
                    Contributes::per_household(b, rate)
                };
                if b == BucketId::Thermal && g.chance(0.5) {
                    c = c.part(g.pick(&["heat", "cold"]));
                }
                if b == BucketId::Comms && g.chance(0.4) {
                    c = c.part(g.pick(&["phone", "payments"]));
                }
                if !buckets.contains(&b) {
                    buckets.push(b);
                }
                m.contributes.push(c);
            }
        }
        if g.chance(0.3) || m.contributes.is_empty() {
            let b = g.pick(&READINESS);
            m.readiness.push(ReadinessCredit {
                bucket: b,
                harm_day_equivalents: g.range(0.05, 5.0),
            });
            buckets.push(b);
        }
        if free && g.chance(0.3) {
            buckets.push(BucketId::HomeLoss);
        }
        if g.chance(0.35) {
            m.step = Some(g.pick(&[0.5, 1.0, 2.0, 5.0]));
        } else if g.chance(0.5) {
            m.set_quantity = Some(g.pick(&[1.0, 2.0, 4.0]));
        }
        if g.chance(0.1) {
            m.roles
                .push(g.pick(&[ItemRole::DevicePower, ItemRole::ColdChain, ItemRole::GoBag]));
        }
        let life_safety = g.chance(0.15);
        let mut it = item(
            &id,
            &format!("Item {k}"),
            "unit",
            &buckets,
            tier,
            free,
            life_safety,
            (low, high),
        );
        it.rare_catastrophic = !free && g.chance(0.05);
        items.push(it);
        meta.push(m);
    }
    (items, meta)
}

/// A random decreasing Λ table on a fixed grid.
pub fn random_curve(g: &mut Gen) -> BucketCurve {
    let mut lambda = Vec::with_capacity(GRID.len());
    let mut l = g.range(0.01, 2.0);
    let dies = g.chance(0.05);
    for i in 0..GRID.len() {
        if dies && i > 10 {
            l = 0.0;
        }
        lambda.push(l);
        l *= g.range(0.2, 1.0);
    }
    // The target is set the way rr-consequence sets it: the smallest day-ladder value whose Λ is at
    // most the dial's rate (DESIGN §4.4), so Λ is above zero everywhere below the target. One
    // bucket in ten has no target at all.
    let mut curve = BucketCurve::new(GRID.to_vec(), lambda, 0.0);
    if !g.chance(0.1) {
        let dial = g.pick(&[0.1, 0.02, 0.010_536, 0.002]);
        curve.target_days = rr_types::TARGET_LADDER_DAYS
            .iter()
            .map(|&d| f64::from(d))
            .find(|&d| curve.lambda_at(d) <= dial)
            .unwrap_or(365.0);
    }
    curve
}

fn risks(g: &mut Gen) -> Risks {
    let mut curves = BTreeMap::new();
    let mut assessments = BTreeMap::new();
    let hazards = [
        HazardId::StrongWind,
        HazardId::WinterWeather,
        HazardId::HeatWave,
        HazardId::Pandemic,
        HazardId::HouseFire,
    ];
    for b in DURATION {
        if g.chance(0.1) {
            continue;
        }
        let mut c = random_curve(g);
        if b == BucketId::Thermal && g.chance(0.5) {
            c.part_shares.insert("heat".into(), g.range(0.2, 0.8));
        }
        assessments.insert(
            b,
            assessment(
                b,
                days_target(c.target_days),
                &[(g.pick(&hazards), 0.6), (g.pick(&hazards), 0.4)],
            ),
        );
        curves.insert(b, c);
    }
    for b in READINESS {
        let p = g.range(0.0, 0.9);
        let target = if b == BucketId::Evacuate {
            Target::Evacuate {
                p_need_10yr: p,
                notice_hours_low: 1.0,
                notice_hours_high: 24.0,
                days_away: 3.0,
            }
        } else {
            readiness_target(p)
        };
        assessments.insert(b, assessment(b, target, &[(g.pick(&hazards), 1.0)]));
    }
    let months = g.pick(&[0.0, 1.5, 4.0, 6.0]) as f32;
    assessments.insert(
        BucketId::Income,
        assessment(
            BucketId::Income,
            Target::Months {
                value: months,
                low: months,
                high: months,
            },
            &[(HazardId::JobLoss, 1.0)],
        ),
    );
    debug_assert!(
        curves
            .keys()
            .all(|b: &BucketId| b.kind() == BucketKind::Duration)
    );
    Risks {
        curves,
        assessments,
    }
}

fn existing(g: &mut Gen, items: &[Item]) -> Vec<Owned> {
    let mut out = Vec::new();
    for it in items {
        if g.chance(0.12) {
            let qty = if it.free {
                1.0
            } else {
                g.pick(&[0.5, 1.0, 3.0]) as f32
            };
            let paid = if !it.free && g.chance(0.5) {
                Some(g.range(0.0, 50.0) as f32)
            } else {
                None
            };
            out.push(Owned {
                item_id: ItemId::from(it.id.as_str()),
                qty,
                paid_usd: paid,
                tested_on: None,
            });
        }
    }
    out
}
