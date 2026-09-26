//! The allocator's rules one at a time, on small hand-built cases with flat curves (Λ constant, so
//! a day of cover in bucket b is worth 10 · w_b · Λ and every number can be checked by hand).
//! The Philadelphia household (four people, a 70-year-old on daily medicine) supplies the weights:
//! water and medication 3, heat and cold 2, everything else 1.

mod common;

use common::item;
use common::setup::{Setup, buy, divisible, set};
use rr_budget::{
    BucketCurve, BudgetInput, BudgetOptions, BudgetResult, Contributes, ContributionTable,
    CoverageRule, GuardrailContext, ItemMeta, ReadinessCredit, Schedule, allocate,
    allocate_with_rule,
};
use rr_types::{
    BucketId, ItemId, Owned, Per, PlanInput, PlanItemKind, RequirementLine, TierId, fixtures,
};

fn order(r: &BudgetResult) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for p in &r.sequence {
        if out.last() != Some(&p.item_id.as_str()) {
            out.push(p.item_id.as_str());
        }
    }
    out
}

fn month_of(r: &BudgetResult, id: &str) -> Option<u16> {
    r.sequence.iter().find(|p| p.item_id == id).map(|p| p.month)
}

/// Targets are capped at the tier's horizon: food (worth 10 · 1 · 0.5 = 5 per day for $4, 1.25
/// per dollar, at every day) is bought only to three days before the lights (10 · 1 · 0.5 · 3 =
/// 15 for $40, 0.375 per dollar) that finish the three-day tier; days 3 to 14 of food follow in
/// the two-week tier. (Food is not promoted: 1.25 is less than five times 0.375.)
#[test]
fn targets_are_capped_at_the_tier_horizon() {
    let s = Setup::new("philadelphia-renters-4", 100.0, 0.0)
        .flat(BucketId::Supplies, 0.5, 14.0)
        .flat(BucketId::Power, 0.5, 3.0)
        .add(
            item(
                "food",
                "Food",
                "person-day",
                &[BucketId::Supplies],
                TierId::H72,
                false,
                false,
                (1.0, 1.0),
            ),
            divisible(
                "food",
                Contributes::per_person(BucketId::Supplies, 1.0),
                1.0,
            ),
        )
        .add(
            buy("lights", BucketId::Power, TierId::H72, 40.0),
            set("lights", BucketId::Power, 3.0),
        );
    let r = s.run();
    assert_eq!(order(&r), ["food", "lights", "food"]);
    let food_h72: f64 = r
        .sequence
        .iter()
        .take_while(|p| p.item_id == "food")
        .map(|p| p.quantity)
        .sum();
    assert_eq!(food_h72, 12.0, "3 days x 4 people");
    assert!(
        r.sequence
            .iter()
            .take_while(|p| p.item_id == "food")
            .all(|p| p.tier == TierId::H72)
    );
    assert!(
        r.sequence
            .iter()
            .skip_while(|p| p.item_id != "lights")
            .skip(1)
            .all(|p| p.tier == TierId::W2 && !p.promoted)
    );
    // With dimmer lights (Λ = 0.05: 0.0375 per dollar) food is over five times better per dollar,
    // so its two-week chunks are promoted ahead of the lights.
    let mut dim = Setup::new("philadelphia-renters-4", 100.0, 0.0)
        .flat(BucketId::Supplies, 0.5, 14.0)
        .flat(BucketId::Power, 0.05, 3.0);
    dim.items = s.items.clone();
    dim.meta = s.meta.clone();
    let promoted = dim.run();
    assert_eq!(order(&promoted), ["food", "lights"]);
    assert!(
        promoted
            .sequence
            .iter()
            .any(|p| p.promoted && p.tier == TierId::W2)
    );
    // Value of the first chunk (0 to half a day): 10 · 1 · 0.5 · 0.5 = 2.5.
    assert!((r.sequence[0].value - 2.5).abs() < 1e-12);
    // Each ladder chunk of food gets its own line internally; the plan merges each month's chunks.
    let lines = r.plan.months[1]
        .items
        .iter()
        .filter(|i| i.item_id == "food")
        .count();
    assert_eq!(lines, 1);
}

/// Life-safety items come first within a tier, whatever their value per dollar.
#[test]
fn life_safety_items_come_first() {
    let mut alarm = buy("co_alarm", BucketId::Fire, TierId::H72, 40.0);
    alarm.life_safety = true;
    let mut alarm_meta = ItemMeta::new("co_alarm");
    alarm_meta.readiness = vec![ReadinessCredit {
        bucket: BucketId::Fire,
        harm_day_equivalents: 1.0,
    }];
    let s = Setup::new("philadelphia-renters-4", 100.0, 0.0)
        .flat(BucketId::Power, 0.5, 3.0)
        .readiness(BucketId::Fire, 0.05)
        .add(
            buy("lights", BucketId::Power, TierId::H72, 10.0),
            set("lights", BucketId::Power, 3.0),
        )
        .add(alarm, alarm_meta);
    let r = s.run();
    assert_eq!(order(&r), ["co_alarm", "lights"]);
}

/// Promotion: a two-week item joins the three-day tier when its value per dollar is at least five
/// times the tier's best. Medicine days 7-10 are worth 10 · 3 · 0.1 · 3 = 9. The three-day
/// tier's best is the lights at 3 / 30 = 0.1, so the medicine is promoted at $5.50 (3 days cost
/// $16.50: 0.545 per dollar) but not at $6.50 (0.46 per dollar); once the lights are bought the
/// tier's best is the radio at 0.05 and the $6.50 medicine is promoted then.
#[test]
fn promotion_needs_five_times_the_tiers_best() {
    let setup = |price: f32| {
        let mut rule = item(
            "refill_rule",
            "Refill rule",
            "action",
            &[BucketId::Medication],
            TierId::Now,
            true,
            false,
            (0.0, 0.0),
        );
        rule.life_safety = false;
        Setup::new("philadelphia-renters-4", 200.0, 0.0)
            .flat(BucketId::Power, 0.1, 3.0)
            .flat(BucketId::Comms, 0.1, 3.0)
            .flat(BucketId::Medication, 0.1, 14.0)
            .add(
                buy("lights", BucketId::Power, TierId::H72, 30.0),
                set("lights", BucketId::Power, 3.0),
            )
            .add(
                buy("radio", BucketId::Comms, TierId::H72, 60.0),
                set("radio", BucketId::Comms, 3.0),
            )
            .add(rule, set("refill_rule", BucketId::Medication, 7.0))
            .add(
                item(
                    "medicine",
                    "Medicine",
                    "day",
                    &[BucketId::Medication],
                    TierId::W2,
                    false,
                    false,
                    (price, price),
                ),
                divisible(
                    "medicine",
                    Contributes::per_household(BucketId::Medication, 1.0),
                    1.0,
                ),
            )
    };
    let cheap = setup(5.5).run();
    assert_eq!(order(&cheap)[0], "medicine");
    assert!(cheap.sequence[0].promoted);
    assert_eq!(cheap.sequence[0].tier, TierId::W2);
    assert!((cheap.sequence[0].value - 9.0).abs() < 1e-9);
    let dearer = setup(6.5).run();
    assert_eq!(order(&dearer)[..3], ["lights", "medicine", "radio"]);
    assert!(dearer.sequence[1].promoted);
}

/// What the household recorded paying replaces the band's midpoint, and what it already has is
/// the starting point: owned food counts toward coverage and is shown as done in month 0.
#[test]
fn recorded_prices_and_existing_inventory() {
    let mut s = Setup::new("philadelphia-renters-4", 100.0, 0.0)
        .flat(BucketId::Supplies, 0.5, 3.0)
        .add(
            item(
                "food",
                "Food",
                "person-day",
                &[BucketId::Supplies],
                TierId::H72,
                false,
                false,
                (2.15, 2.85),
            ),
            divisible(
                "food",
                Contributes::per_person(BucketId::Supplies, 1.0),
                1.0,
            ),
        );
    s.household.existing = vec![Owned {
        item_id: ItemId::from("food"),
        qty: 4.0,
        paid_usd: Some(6.0),
    }];
    let r = s.run();
    // Four person-days already owned: one day of food for four people, then bought up to three.
    assert!((common::days_at(&r, 0)[&BucketId::Supplies] - 1.0).abs() < 1e-9);
    assert!((common::days_at(&r, usize::MAX)[&BucketId::Supplies] - 3.0).abs() < 1e-9);
    let owned = &r.plan.months[0].items[0];
    assert!(owned.done);
    assert_eq!((owned.est_cost_usd, owned.paid_usd), (6.0, Some(6.0)));
    // Further food costs the recorded $1.50 per person-day, not the band's $2.50.
    for p in &r.sequence {
        assert!((p.cost_usd / p.quantity - 1.5).abs() < 1e-9, "{p:?}");
    }
    let bought: f64 = r.sequence.iter().map(|p| p.quantity).sum();
    assert_eq!(bought, 8.0, "from 1 day to 3 days for 4 people");
}

/// A set owned in part is completed, not bought again: one headlamp of four is owned.
#[test]
fn partly_owned_sets_are_completed() {
    let mut lamps = ItemMeta::new("headlamp");
    lamps.contributes = vec![Contributes::per_person(BucketId::Power, 3.0)];
    lamps.set_quantity = Some(4.0);
    let mut s = Setup::new("philadelphia-renters-4", 100.0, 0.0)
        .flat(BucketId::Power, 0.5, 3.0)
        .add(buy("headlamp", BucketId::Power, TierId::H72, 8.0), lamps);
    s.household.existing = vec![Owned {
        item_id: ItemId::from("headlamp"),
        qty: 1.0,
        paid_usd: None,
    }];
    let r = s.run();
    assert_eq!(r.sequence.len(), 1);
    assert_eq!(r.sequence[0].quantity, 3.0);
    assert_eq!(r.sequence[0].cost_usd, 24.0);
    assert!((common::days_at(&r, 1)[&BucketId::Power] - 3.0).abs() < 1e-9);
}

/// Requirement lines from rr-supply size sets and set per-day rates for divisible items: four
/// headlamps for four people; 12.3 gallons for a 3-day water target (the dog included).
#[test]
fn requirement_lines_size_purchases() {
    let line = |class: &str, bucket: BucketId, quantity: f32| RequirementLine {
        id: format!("{class}_line"),
        bucket,
        item_class: class.into(),
        quantity,
        unit: "unit".into(),
        per: Per::Household,
        rule: "test".into(),
        citations: vec![],
        plain: String::new(),
    };
    let mut s = Setup::new("philadelphia-renters-4", 200.0, 0.0)
        .flat(BucketId::Power, 0.5, 3.0)
        .flat(BucketId::WaterOut, 0.1, 3.0)
        .add(buy("headlamp", BucketId::Power, TierId::H72, 6.0), {
            let mut m = ItemMeta::new("headlamp");
            m.contributes = vec![Contributes::per_person(BucketId::Power, 3.0)];
            m
        })
        .add(
            item(
                "water",
                "Water",
                "gallon",
                &[BucketId::WaterOut],
                TierId::H72,
                false,
                true,
                (1.0, 1.0),
            ),
            divisible(
                "water",
                Contributes::per_person(BucketId::WaterOut, 1.0),
                1.0,
            ),
        );
    s.requirements = vec![
        line("headlamp", BucketId::Power, 4.0),
        line("water", BucketId::WaterOut, 12.3),
    ];
    let r = s.run();
    let lamps: Vec<_> = r
        .sequence
        .iter()
        .filter(|p| p.item_id == "headlamp")
        .collect();
    assert_eq!(
        (lamps.len(), lamps[0].quantity, lamps[0].cost_usd),
        (1, 4.0, 24.0)
    );
    let water: f64 = r
        .sequence
        .iter()
        .filter(|p| p.item_id == "water")
        .map(|p| p.quantity)
        .sum();
    assert!((12.3..=16.3).contains(&water), "{water} gallons");
    assert!(common::days_at(&r, usize::MAX)[&BucketId::WaterOut] >= 3.0);
}

/// Rare-catastrophe items get $0 unless the household opts in, then at most 10 % of each month's
/// money: a $50 radiation meter on a $100 budget is saved for at $10 a month and bought in month
/// 5, and the main plan's $900 item takes a month longer than without the allowance.
#[test]
fn rare_catastrophic_items_are_capped() {
    let mut meter = buy("radiation_meter", BucketId::Security, TierId::Y1, 50.0);
    meter.rare_catastrophic = true;
    let mut s = Setup::new("philadelphia-renters-4", 100.0, 0.0)
        .flat(BucketId::Power, 0.5, 14.0)
        .add(
            buy("generator", BucketId::Power, TierId::H72, 900.0),
            set("generator", BucketId::Power, 14.0),
        )
        .add(meter, ItemMeta::new("radiation_meter"));
    let off = s.run();
    assert_eq!(
        off.rare_catastrophic_skipped,
        [ItemId::from("radiation_meter")]
    );
    assert_eq!(month_of(&off, "radiation_meter"), None);
    assert_eq!(month_of(&off, "generator"), Some(9));
    s.options.rare_catastrophic_opt_in = true;
    let on = s.run();
    assert!(on.rare_catastrophic_skipped.is_empty());
    assert_eq!(month_of(&on, "radiation_meter"), Some(5));
    assert!(
        on.sequence
            .iter()
            .any(|p| p.item_id == "radiation_meter" && p.rare_catastrophic)
    );
    assert_eq!(month_of(&on, "generator"), Some(10));
    for m in 1..=4 {
        let deposit: f32 = on.plan.months[m]
            .items
            .iter()
            .filter(|i| i.kind == PlanItemKind::Reserve && i.item_id == "radiation_meter")
            .map(|i| i.est_cost_usd)
            .sum();
        assert_eq!(deposit, 10.0, "month {m}");
    }
}

/// With no monthly budget there is no later month to save for: one-off money buys the best items
/// that fit. $50 buys the $40 item, then the $10 one (the $30 one no longer fits).
#[test]
fn zero_monthly_budget_spends_the_one_off_on_what_fits() {
    let s = Setup::new("philadelphia-renters-4", 0.0, 50.0)
        .flat(BucketId::Power, 0.4, 3.0)
        .flat(BucketId::Comms, 0.2, 3.0)
        .flat(BucketId::Supplies, 0.03, 3.0)
        .add(
            buy("a", BucketId::Power, TierId::H72, 40.0),
            set("a", BucketId::Power, 3.0),
        )
        .add(
            buy("b", BucketId::Comms, TierId::H72, 30.0),
            set("b", BucketId::Comms, 3.0),
        )
        .add(
            buy("c", BucketId::Supplies, TierId::H72, 10.0),
            set("c", BucketId::Supplies, 3.0),
        );
    let r = s.run();
    assert_eq!(order(&r), ["a", "c"]);
    assert_eq!(r.plan.months.len(), 1);
    assert_eq!(r.plan.done_month, None);
    assert!(r.plan.envelopes.is_empty());
}

/// Money buckets are never paid from the supplies budget.
#[test]
fn money_buckets_are_never_funded() {
    let s = Setup::new("philadelphia-renters-4", 100.0, 0.0)
        .readiness(BucketId::HomeLoss, 0.5)
        .add(
            item(
                "insurance_rider",
                "Insurance rider",
                "each",
                &[BucketId::HomeLoss],
                TierId::H72,
                false,
                false,
                (20.0, 20.0),
            ),
            ItemMeta::new("insurance_rider"),
        );
    let r = s.run();
    assert!(r.sequence.is_empty());
    assert_eq!(r.plan.done_month, Some(0));
}

/// A readiness item needed less than 2 % of the time over ten years joins only when it beats the
/// tier's best on value per dollar: a $30 go-bag at a 1 % chance never does; a $0.30 one does.
#[test]
fn rarely_needed_readiness_items_must_beat_the_tiers_best() {
    let setup = |price: f32| {
        let mut bag = ItemMeta::new("go_bag");
        bag.readiness = vec![ReadinessCredit {
            bucket: BucketId::Evacuate,
            harm_day_equivalents: 2.0,
        }];
        Setup::new("philadelphia-renters-4", 100.0, 0.0)
            .flat(BucketId::Power, 0.1, 3.0)
            .readiness(BucketId::Evacuate, 0.01)
            .add(
                buy("lights", BucketId::Power, TierId::H72, 30.0),
                set("lights", BucketId::Power, 3.0),
            )
            .add(buy("go_bag", BucketId::Evacuate, TierId::H72, price), bag)
    };
    assert_eq!(order(&setup(30.0).run()), ["lights"]);
    assert_eq!(order(&setup(0.3).run()), ["go_bag", "lights"]);
}

/// A caller-supplied coverage rule is used as given: the table wrapped in a rule that answers only
/// the required method (gains by difference) gives the same plan as the table itself.
#[test]
fn a_custom_coverage_rule_is_honoured() {
    struct ByDifference(ContributionTable);
    impl CoverageRule for ByDifference {
        fn coverage(&self, b: BucketId, items: &[(ItemId, f64)], h: &PlanInput) -> f64 {
            self.0.coverage(b, items, h)
        }
    }
    let s = Setup::new("philadelphia-renters-4", 60.0, 0.0)
        .flat(BucketId::Supplies, 0.5, 10.0)
        .flat(BucketId::Power, 0.05, 3.0)
        .add(
            item(
                "food",
                "Food",
                "person-day",
                &[BucketId::Supplies],
                TierId::H72,
                false,
                false,
                (2.0, 3.0),
            ),
            divisible(
                "food",
                Contributes::per_person(BucketId::Supplies, 1.0),
                1.0,
            ),
        )
        .add(
            buy("lights", BucketId::Power, TierId::H72, 35.0),
            set("lights", BucketId::Power, 3.0),
        );
    let table = ContributionTable::new(&s.meta).unwrap();
    let custom = allocate_with_rule(&s.input(), &ByDifference(table)).unwrap();
    let default = s.run();
    assert_eq!(custom.sequence, default.sequence);
    assert_eq!(custom.plan, default.plan);
}

/// Malformed inputs are errors for the caller, never silent plans.
#[test]
fn bad_curves_and_metadata_are_rejected() {
    let mut s = Setup::new("philadelphia-renters-4", 60.0, 0.0);
    s.risks.curves.insert(
        BucketId::Power,
        BucketCurve::new(vec![1.0, 2.0], vec![0.1, 0.2], 3.0),
    );
    assert!(allocate(&s.input()).is_err());
    let mut s = Setup::new("philadelphia-renters-4", 60.0, 0.0);
    s.risks.curves.insert(
        BucketId::Evacuate,
        BucketCurve::new(vec![1.0], vec![0.1], 3.0),
    );
    assert!(allocate(&s.input()).is_err());
    let mut s = Setup::new("philadelphia-renters-4", 60.0, 0.0).flat(BucketId::Power, 0.1, 3.0);
    let mut bad = ItemMeta::new("x");
    bad.contributes = vec![Contributes::per_household(BucketId::Power, -1.0)];
    s = s.add(buy("x", BucketId::Power, TierId::H72, 1.0), bad);
    assert!(allocate(&s.input()).is_err());
}

/// The research schedule is available and deterministic too.
#[test]
fn research_schedule_runs_on_the_fixtures() {
    for (name, household) in fixtures::all() {
        let (items, meta) = common::philadelphia::catalogue();
        let risks = common::philadelphia::risks();
        let context = GuardrailContext::default();
        for schedule in [Schedule::FixedOrder, Schedule::ResearchShortcuts] {
            let input = BudgetInput {
                household: &household,
                catalogue: &items,
                meta: &meta,
                requirements: &[],
                risks: &risks,
                context: &context,
                options: BudgetOptions {
                    schedule,
                    ..BudgetOptions::default()
                },
            };
            let a = allocate(&input).unwrap();
            let b = allocate(&input).unwrap();
            assert_eq!(a, b, "{name}");
            let (left, _) = common::audit(&a, &|id| {
                items.iter().any(|i| i.id == id && i.rare_catastrophic)
            });
            assert!(left.iter().all(|&x| x > -0.05), "{name} {schedule:?}");
        }
    }
}

/// The planner's example: $40 a month, a $300 CPAP battery as the top priority, and cheap items
/// for other needs. Split: every month buys something while cheap items remain, and the battery
/// still arrives within ceil($300 / (50 % x $40)) = 15 months. Fixed order: the battery comes
/// first, in month 8, after seven months of only saving. Research shortcuts: cheap items first;
/// the battery is saved for only once a month comes when nothing else is affordable, so it
/// arrives later than under the fixed order.
#[test]
fn cpap_battery_under_each_schedule() {
    let setup = |schedule: Schedule| {
        let mut battery = buy("cpap_battery", BucketId::Power, TierId::H72, 300.0);
        battery.life_safety = true;
        let mut first_aid = ItemMeta::new("first_aid");
        first_aid.readiness = vec![ReadinessCredit {
            bucket: BucketId::MedicalEmergency,
            harm_day_equivalents: 0.5,
        }];
        let mut s = Setup::new("phoenix-apartment-cpap-1", 40.0, 0.0)
            .flat(BucketId::Power, 0.3, 3.0)
            .flat(BucketId::WaterOut, 0.2, 3.0)
            .flat(BucketId::Supplies, 0.5, 10.0)
            .flat(BucketId::Comms, 0.1, 3.0)
            .flat(BucketId::Thermal, 0.1, 2.0)
            .readiness(BucketId::MedicalEmergency, 0.9)
            .add(battery, set("cpap_battery", BucketId::Power, 3.0))
            .add(
                buy("water", BucketId::WaterOut, TierId::H72, 20.0),
                set("water", BucketId::WaterOut, 3.0),
            )
            .add(
                item(
                    "food",
                    "Food",
                    "person-day",
                    &[BucketId::Supplies],
                    TierId::H72,
                    false,
                    false,
                    (2.5, 2.5),
                ),
                divisible(
                    "food",
                    Contributes::per_person(BucketId::Supplies, 1.0),
                    1.0,
                ),
            )
            .add(
                buy("first_aid", BucketId::MedicalEmergency, TierId::H72, 25.0),
                first_aid,
            )
            .add(
                buy("radio", BucketId::Comms, TierId::H72, 30.0),
                set("radio", BucketId::Comms, 3.0),
            )
            .add(
                buy("fans", BucketId::Thermal, TierId::H72, 30.0),
                set("fans", BucketId::Thermal, 2.0),
            );
        s.options.schedule = schedule;
        s.run()
    };
    let battery_month =
        |r: &BudgetResult| month_of(r, "cpap_battery").expect("the battery arrives");
    let others_done_by = |r: &BudgetResult| {
        r.sequence
            .iter()
            .filter(|p| p.item_id != "cpap_battery")
            .map(|p| p.month)
            .max()
            .unwrap()
    };

    let split = setup(Schedule::default());
    let b = battery_month(&split);
    assert!(b <= 15, "battery in month {b}");
    // Every month whose free money covers the cheapest item still worth buying buys something;
    // with $20 a month free, that is most months until the cheap items run out.
    let mut buying_months = 0;
    for mm in split
        .money_by_month
        .iter()
        .filter(|mm| mm.month >= 1 && mm.month < b)
    {
        let bought = split.sequence.iter().any(|p| p.month == mm.month);
        if mm
            .cheapest_usd
            .is_some_and(|c| c <= mm.available_usd + 1e-9)
        {
            assert!(
                bought,
                "split: month {} could buy something but did not",
                mm.month
            );
        }
        buying_months += usize::from(bought);
    }
    assert!(
        buying_months >= 5,
        "only {buying_months} months with a purchase before the battery"
    );
    assert!(others_done_by(&split) < b + 3);
    // Half of each month's $40 went into the battery fund from month 1.
    let deposits: Vec<f32> = split.plan.months[1..=3]
        .iter()
        .flat_map(|m| m.items.iter())
        .filter(|i| i.kind == PlanItemKind::Reserve)
        .map(|i| i.est_cost_usd)
        .collect();
    assert_eq!(deposits, [20.0, 20.0, 20.0]);

    let fixed = setup(Schedule::FixedOrder);
    assert_eq!(battery_month(&fixed), 8);
    for m in 1..8 {
        assert!(
            fixed.plan.months[m]
                .items
                .iter()
                .all(|i| i.kind == PlanItemKind::Reserve),
            "fixed order: month {m} only saves"
        );
    }

    let research = setup(Schedule::ResearchShortcuts);
    assert!(research.sequence.iter().any(|p| p.month == 1));
    assert!(battery_month(&research) > battery_month(&fixed));
}

/// Month 0 lists at most eight free actions (life-safety first, then value); the rest move to
/// month 1 (and 2), before that month's purchases, and their coverage counts only from then. The
/// allocator still plans purchases knowing they are coming, so it does not buy water containers
/// that a scheduled free step (the water-heater reserve) will make unnecessary.
#[test]
fn free_actions_beyond_eight_move_to_later_months_and_count_from_then() {
    let mut s = Setup::new("philadelphia-renters-4", 60.0, 100.0)
        .flat(BucketId::WaterOut, 0.2, 1.0)
        .flat(BucketId::Power, 0.5, 3.0)
        .readiness(BucketId::Fire, 0.3);
    for j in 0..8 {
        let id = format!("safety_step_{j}");
        let mut m = ItemMeta::new(id.as_str());
        m.readiness = vec![ReadinessCredit {
            bucket: BucketId::Fire,
            harm_day_equivalents: 0.1,
        }];
        s = s.add(
            item(
                &id,
                &id,
                "action",
                &[BucketId::Fire],
                TierId::Now,
                true,
                true,
                (0.0, 0.0),
            ),
            m,
        );
    }
    for j in 0..3 {
        let id = format!("plain_step_{j}");
        s = s.add(
            item(
                &id,
                &id,
                "action",
                &[BucketId::HomeLoss],
                TierId::Now,
                true,
                false,
                (0.0, 0.0),
            ),
            ItemMeta::new(id.as_str()),
        );
    }
    s = s
        .add(
            item(
                "heater",
                "Water-heater reserve",
                "action",
                &[BucketId::WaterOut],
                TierId::Now,
                true,
                false,
                (0.0, 0.0),
            ),
            set("heater", BucketId::WaterOut, 1.0),
        )
        .add(
            buy("containers", BucketId::WaterOut, TierId::H72, 20.0),
            set("containers", BucketId::WaterOut, 3.0),
        )
        .add(
            buy("lights", BucketId::Power, TierId::H72, 30.0),
            set("lights", BucketId::Power, 3.0),
        );
    let r = s.run();
    let free_in = |m: usize| {
        r.plan.months[m]
            .items
            .iter()
            .filter(|i| i.kind == PlanItemKind::FreeAction)
            .map(|i| i.item_id.as_str())
            .collect::<Vec<_>>()
    };
    let m0 = free_in(0);
    let m1 = free_in(1);
    assert_eq!(m0.len(), 8);
    assert!(m0.iter().all(|id| id.starts_with("safety_step_")), "{m0:?}");
    assert_eq!(
        m1.first(),
        Some(&"heater"),
        "most valuable of the rest first: {m1:?}"
    );
    assert_eq!(m1.len(), 4);
    // Month 1 lists its free steps before its purchases.
    let kinds: Vec<PlanItemKind> = r.plan.months[1].items.iter().map(|i| i.kind).collect();
    assert!(
        kinds
            .windows(2)
            .all(|w| !(w[0] != PlanItemKind::FreeAction && w[1] == PlanItemKind::FreeAction))
    );
    // Coverage counts the heater step from month 1, not month 0.
    assert_eq!(common::days_at(&r, 0)[&BucketId::WaterOut], 0.0);
    assert_eq!(common::days_at(&r, 1)[&BucketId::WaterOut], 1.0);
    // The one-off money buys lights in month 0, never the containers the free step makes
    // unnecessary.
    assert_eq!(month_of(&r, "lights"), Some(0));
    assert_eq!(month_of(&r, "containers"), None);
    assert!(!r.warnings.iter().any(|w| w.id == "no_water_after_month_1"));
}
