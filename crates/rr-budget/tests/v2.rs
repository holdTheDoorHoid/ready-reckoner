//! Contract v2 allocator changes (DESIGN-DELTA §3, brief-budget.md), one at a time on small
//! hand-built cases with flat curves: prerequisites (`requires`), readiness shares and alternative
//! groups, the order within a tier, season-aware ordering, the rare allowance by family,
//! bare-minimum mode and the two done months, decisions outside the free-step count, the
//! long-horizon section, the savings steps and the legal line, and the six new guardrails.

mod common;

use common::item;
use common::setup::{Setup, buy, divisible, set};
use rr_budget::{
    BudgetResult, Contributes, EXEMPT_BY_MONTH, FREE_ACTIONS_MONTH_0, ItemMeta, ItemRole,
    LEGAL_COST_CITATION, MinimumShare, ReadinessCredit, Schedule,
};
use rr_types::{
    Benefit, BucketId, HazardId, Item, ItemId, Owned, PlanItem, PlanItemKind, RawWaterSource,
    Season, Target, TierId,
};

const PHL: &str = "philadelphia-renters-4";

fn month_of(r: &BudgetResult, id: &str) -> Option<u16> {
    r.sequence.iter().find(|p| p.item_id == id).map(|p| p.month)
}

fn position(r: &BudgetResult, id: &str) -> usize {
    r.sequence
        .iter()
        .position(|p| p.item_id == id)
        .unwrap_or_else(|| panic!("{id} is bought"))
}

fn lines(r: &BudgetResult) -> Vec<(u16, &PlanItem)> {
    r.plan
        .months
        .iter()
        .flat_map(|m| m.items.iter().map(move |i| (m.index, i)))
        .collect()
}

fn warned(r: &BudgetResult, id: &str) -> bool {
    r.warnings.iter().any(|w| w.id == id)
}

/// A purchase that only makes the household ready: `harm` day-equivalents on `bucket`.
fn ready_item(
    id: &str,
    bucket: BucketId,
    price: f32,
    harm: f64,
    share: Option<f32>,
) -> (Item, ItemMeta) {
    let mut m = ItemMeta::new(id);
    m.readiness = vec![ReadinessCredit {
        bucket,
        harm_day_equivalents: harm,
    }];
    let mut i = buy(id, bucket, TierId::H72, price);
    i.readiness_share = share;
    (i, m)
}

/// A set that covers `days` of one part of a duration bucket.
fn part_set(id: &str, bucket: BucketId, part: &str, days: f64, price: f32) -> (Item, ItemMeta) {
    let mut m = ItemMeta::new(id);
    m.contributes = vec![Contributes::per_household(bucket, days).part(part)];
    (buy(id, bucket, TierId::H72, price), m)
}

/// A free step with a readiness credit.
fn free_step(id: &str, bucket: BucketId, harm: f64) -> (Item, ItemMeta) {
    let mut m = ItemMeta::new(id);
    m.readiness = vec![ReadinessCredit {
        bucket,
        harm_day_equivalents: harm,
    }];
    (
        item(
            id,
            id,
            "action",
            &[bucket],
            TierId::Now,
            true,
            false,
            (0.0, 0.0),
        ),
        m,
    )
}

// ------------------------------------------------------------------------------------------------
// Prerequisites
// ------------------------------------------------------------------------------------------------

/// P-12: batteries are far better value than the light they need, and the light costs more than
/// a month's money, so without `requires` the batteries come first. With it they wait for the
/// light (the same month, after it), a light the household owns counts from month 0, and a device
/// the catalogue does not offer at all is the household's own (its generator), so the accessory is
/// not held back.
#[test]
fn an_accessory_never_comes_before_its_device() {
    let (light, lm) = part_set("light", BucketId::Power, "lights", 3.0, 45.0);
    let (mut batteries, bm) = part_set("batteries", BucketId::Power, "batteries", 3.0, 6.0);
    batteries.requires = vec![ItemId::from("light"), ItemId::from("radio")];
    let base = || Setup::new(PHL, 30.0, 0.0).flat(BucketId::Power, 0.5, 3.0);

    let mut unlinked = batteries.clone();
    unlinked.requires.clear();
    let r0 = base()
        .add(light.clone(), lm.clone())
        .add(unlinked, bm.clone())
        .run();
    assert!(month_of(&r0, "batteries") < month_of(&r0, "light"));

    let r = base()
        .add(light.clone(), lm.clone())
        .add(batteries.clone(), bm.clone())
        .run();
    assert_eq!(month_of(&r, "light"), month_of(&r, "batteries"));
    assert!(position(&r, "light") < position(&r, "batteries"));
    let line = lines(&r)
        .into_iter()
        .find(|(_, i)| i.item_id == "batteries" && i.kind == PlanItemKind::Purchase)
        .unwrap()
        .1;
    assert_eq!(
        line.requires,
        [ItemId::from("light"), ItemId::from("radio")]
    );

    let mut owned = base()
        .add(light.clone(), lm.clone())
        .add(batteries.clone(), bm.clone());
    owned.household.existing = vec![Owned {
        item_id: ItemId::from("light"),
        qty: 1.0,
        paid_usd: None,
        tested_on: None,
    }];
    assert_eq!(month_of(&owned.run(), "batteries"), Some(1));

    let mut fuel = batteries;
    fuel.requires = vec![ItemId::from("generator")];
    let r2 = base().add(light, lm).add(fuel, bm).run();
    assert!(month_of(&r2, "batteries") < month_of(&r2, "light"));
}

// ------------------------------------------------------------------------------------------------
// Readiness shares and alternative groups
// ------------------------------------------------------------------------------------------------

/// P-05: an item's readiness value is its bucket's value times its share, so a whistle (0.05)
/// is worth a twentieth of the go-bag (1.0) with the same harm estimate, and a share of 0 is
/// worth nothing.
#[test]
fn readiness_value_is_multiplied_by_the_share() {
    let (bag, bagm) = ready_item("bag", BucketId::Evacuate, 10.0, 2.0, Some(1.0));
    let (whistle, wm) = ready_item("whistle", BucketId::Evacuate, 10.0, 2.0, Some(0.05));
    let (card, cm) = ready_item("card", BucketId::Evacuate, 10.0, 2.0, Some(0.0));
    let r = Setup::new(PHL, 100.0, 0.0)
        .readiness(BucketId::Evacuate, 0.3)
        .add(bag, bagm)
        .add(whistle, wm)
        .add(card, cm)
        .run();
    let value = |id: &str| r.sequence.iter().find(|p| p.item_id == id).map(|p| p.value);
    let ratio = value("whistle").unwrap() / value("bag").unwrap();
    assert!((ratio - 0.05).abs() < 1e-9, "{ratio}");
    assert_eq!(value("card"), None, "a share of 0 buys nothing");
}

/// Two ways to do one readiness job (a HEPA cleaner and a box fan with a filter) share one
/// credit: the better value per dollar is bought, the other never is for that reason alone, and
/// the checklist counts them as one step.
#[test]
fn alternatives_share_one_readiness_credit() {
    let (mut hepa, hm) = ready_item("hepa", BucketId::CleanAir, 120.0, 0.5, Some(1.0));
    let (mut filter, fm) = ready_item("fan_filter", BucketId::CleanAir, 45.0, 0.5, Some(1.0));
    let run = |grouped: bool| {
        let (mut h, mut f) = (hepa.clone(), filter.clone());
        if grouped {
            h.alternative_group = Some("clean_room".into());
            f.alternative_group = Some("clean_room".into());
        }
        Setup::new(PHL, 200.0, 0.0)
            .readiness(BucketId::CleanAir, 0.1)
            .add(h, hm.clone())
            .add(f, fm.clone())
            .run()
    };
    let apart = run(false);
    assert!(month_of(&apart, "hepa").is_some() && month_of(&apart, "fan_filter").is_some());
    let grouped = run(true);
    assert!(month_of(&grouped, "fan_filter").is_some());
    assert_eq!(month_of(&grouped, "hepa"), None);
    match grouped.covered[&BucketId::CleanAir] {
        Target::Readiness { done, of, .. } => assert_eq!((done, of), (1, 1)),
        other => panic!("{other:?}"),
    }
    // Owning one counts: the other is not bought.
    hepa.alternative_group = Some("clean_room".into());
    filter.alternative_group = Some("clean_room".into());
    let mut owned = Setup::new(PHL, 200.0, 0.0)
        .readiness(BucketId::CleanAir, 0.1)
        .add(hepa, hm)
        .add(filter, fm);
    owned.household.existing = vec![Owned {
        item_id: ItemId::from("hepa"),
        qty: 1.0,
        paid_usd: None,
        tested_on: None,
    }];
    assert_eq!(month_of(&owned.run(), "fan_filter"), None);
}

// ------------------------------------------------------------------------------------------------
// The order within a tier
// ------------------------------------------------------------------------------------------------

/// Within a tier: life-safety items, then capabilities (a readiness share of 1), then the rest by
/// value per dollar, and long-horizon items last, even when they are better value per dollar.
/// With money for everything, month 1 buys in exactly that order.
#[test]
fn life_safety_then_capabilities_then_value_and_long_horizon_last() {
    let (mut alarm, am) = ready_item("alarm", BucketId::Fire, 30.0, 5.0, Some(0.5));
    alarm.life_safety = true;
    let (kit, km) = ready_item("kit", BucketId::MedicalEmergency, 40.0, 0.1, Some(1.0));
    let food = buy("food", BucketId::Supplies, TierId::H72, 5.0);
    let radio = buy("radio", BucketId::Comms, TierId::H72, 20.0);
    let mut carriers = buy("carriers", BucketId::WaterOut, TierId::H72, 10.0);
    carriers.long_horizon = true;
    let r = Setup::new(PHL, 1000.0, 0.0)
        .flat(BucketId::Supplies, 0.5, 3.0)
        .flat(BucketId::Comms, 0.5, 3.0)
        .flat(BucketId::WaterOut, 0.5, 3.0)
        .readiness(BucketId::Fire, 0.3)
        .readiness(BucketId::MedicalEmergency, 0.9)
        .add(alarm, am)
        .add(kit, km)
        .add(food, set("food", BucketId::Supplies, 3.0))
        .add(radio, set("radio", BucketId::Comms, 3.0))
        .add(carriers, set("carriers", BucketId::WaterOut, 3.0))
        .run();
    let order: Vec<&str> = r.sequence.iter().map(|p| p.item_id.as_str()).collect();
    assert_eq!(order, ["alarm", "kit", "food", "radio", "carriers"]);
    // The carriers were the best value per dollar of the non-life-safety items.
    let density = |id: &str| {
        let p = r.sequence.iter().find(|p| p.item_id == id).unwrap();
        p.value / p.cost_usd
    };
    assert!(density("carriers") > density("food") && density("food") > density("radio"));
}

// ------------------------------------------------------------------------------------------------
// Season-aware ordering
// ------------------------------------------------------------------------------------------------

/// Eight readiness items and a fan, $20 each, one bought a month. The fan is the worst value per
/// dollar, so it comes last (month 9) unless it is anchored to summer: then it is bought in month
/// 6, April (the planning date is 1 October), before the hot season. A fan dearer than a month's
/// money is never moved (the rule opens no sinking fund), and the fixed-order schedule ignores
/// the calendar.
#[test]
fn a_fan_lands_before_the_hot_season_when_the_month_s_money_allows() {
    let run = |season: Option<Season>, price: f32, schedule: Schedule| {
        let mut s = Setup::new(PHL, 20.0, 0.0)
            .flat(BucketId::Thermal, 0.02, 3.0)
            .readiness(BucketId::Security, 0.5);
        for k in 0..8 {
            let (i, m) = ready_item(&format!("r{k}"), BucketId::Security, 20.0, 3.0, None);
            s = s.add(i, m);
        }
        let (mut fan, fm) = part_set("fan", BucketId::Thermal, "heat", 3.0, price);
        fan.season = season;
        s = s.add(fan, fm);
        s.options.schedule = schedule;
        s.run()
    };
    let split = Schedule::default();
    assert_eq!(s_month(&run(None, 20.0, split)), Some(9));
    assert_eq!(s_month(&run(Some(Season::Summer), 20.0, split)), Some(6));
    // A fall anchor is due now (October), so it moves to the front of the ordinary items.
    assert_eq!(s_month(&run(Some(Season::Fall), 20.0, split)), Some(1));
    assert_eq!(
        s_month(&run(Some(Season::Summer), 60.0, split)),
        s_month(&run(None, 60.0, split))
    );
    assert_eq!(
        s_month(&run(Some(Season::Summer), 20.0, Schedule::FixedOrder)),
        Some(9)
    );
}

fn s_month(r: &BudgetResult) -> Option<u16> {
    month_of(r, "fan")
}

// ------------------------------------------------------------------------------------------------
// The rare allowance by family
// ------------------------------------------------------------------------------------------------

/// REVIEW §2.4: a specialised item is bought only for a family the household ticked whose local
/// ten-year chance is at least 1 in 1,000, from at most a tenth of each month's money, never
/// before the three-day life-safety items, and no family takes more than half the allowance over
/// the plan horizon. Each purchase says what it is for.
#[test]
fn the_rare_allowance_buys_only_for_ticked_families_likely_enough_here() {
    let rare = |id: &str, family: Option<HazardId>, price: f32| {
        let mut i = buy(id, BucketId::Security, TierId::Y1, price);
        i.rare_catastrophic = true;
        i.hazard_extras = family.into_iter().collect();
        (i, ItemMeta::new(id))
    };
    let setup = |families: &[&str]| {
        let mut water = buy("water", BucketId::WaterOut, TierId::H72, 150.0);
        water.life_safety = true;
        let (m, mm) = rare("meter", Some(HazardId::NuclearAttack), 25.0);
        let (f, fm) = rare("nuclear_extra", Some(HazardId::NuclearAttack), 68.0);
        let (sol, sm) = rare("solar_shield", Some(HazardId::GeomagneticStorm), 30.0);
        let (k, km) = rare("kitless", None, 10.0);
        let mut s = Setup::new(PHL, 100.0, 0.0)
            .flat(BucketId::WaterOut, 0.5, 3.0)
            .add(water, set("water", BucketId::WaterOut, 3.0))
            .add(m, mm)
            .add(f, fm)
            .add(sol, sm)
            .add(k, km);
        // Philadelphia's nuclear family (about 2.4 in 1,000 over ten years) passes the line;
        // this solar storm (about 5 in 10,000) does not.
        s.risks.register.insert(HazardId::NuclearAttack, 2.4e-4);
        s.risks.register.insert(HazardId::GeomagneticStorm, 5e-5);
        s.household.dials.rare_opt_in = families.iter().map(|f| (*f).to_owned()).collect();
        s
    };

    let r = setup(&["nuclear_attack", "geomagnetic_storm"]).run();
    let skipped: Vec<&str> = r
        .rare_catastrophic_skipped
        .iter()
        .map(|i| i.as_str())
        .collect();
    assert!(skipped.contains(&"solar_shield") && skipped.contains(&"kitless"));
    let (water, meter, extra) = (
        month_of(&r, "water").unwrap(),
        month_of(&r, "meter").unwrap(),
        month_of(&r, "nuclear_extra").unwrap(),
    );
    assert!(meter > water, "nothing before the three-day basics");
    assert!(extra > meter, "best value per dollar first");
    assert!(r.sequence.iter().filter(|p| p.rare_catastrophic).count() == 2);
    // No allowance money moves before the basics are in hand.
    let first_rare_deposit = lines(&r)
        .into_iter()
        .filter(|(_, i)| i.kind == PlanItemKind::Reserve && i.item_id == "meter")
        .map(|(m, _)| m)
        .min();
    assert!(first_rare_deposit.is_some_and(|m| m > water));
    let line = lines(&r)
        .into_iter()
        .find(|(_, i)| i.item_id == "meter" && i.kind == PlanItemKind::Purchase)
        .unwrap()
        .1;
    assert!(
        line.why
            .contains("the nuclear attack or EMP row you ticked"),
        "{}",
        line.why
    );
    assert!(line.why.contains("1 in 1,000"), "{}", line.why);
    assert_eq!(line.hazards, [HazardId::NuclearAttack]);

    // Not ticked: nothing for the nuclear family.
    let solar_only = setup(&["geomagnetic_storm"]).run();
    assert!(!solar_only.sequence.iter().any(|p| p.rare_catastrophic));
    // The v1 switch ticks every family.
    let mut v1 = setup(&[]);
    v1.options.rare_catastrophic_opt_in = true;
    assert!(month_of(&v1.run(), "meter").is_some());
    // Half the allowance over ten months is $50: the $25 meter fits, the $68 second nuclear item
    // would not.
    let mut short = setup(&["nuclear_attack"]);
    short.options.max_months = 10;
    let r = short.run();
    assert!(month_of(&r, "meter").is_some());
    assert_eq!(month_of(&r, "nuclear_extra"), None);
    assert!(
        r.rare_catastrophic_skipped
            .contains(&ItemId::from("nuclear_extra"))
    );
}

/// An item that protects against one cause inside its family is gated, and valued, on that cause's
/// chance (the middle of its published range), not on the family total. A shielded bag answers any
/// electromagnetic pulse, so it is gated on the nuclear family's `emp` sub-cause, while the
/// dosimeter card keeps the family's local blast-or-fallout total: where that total is about 1.2 in
/// 10,000 over ten years (a remote county), the card is not bought and the bag is (its EMP part,
/// 2.2e-5 to 3.45e-3 a year, is about 2.7 in 1,000), and its sentence names the pulse and the row. A
/// pulse part of 0, or no sub-causes at all, buys no bag.
#[test]
fn an_item_for_one_cause_is_gated_on_that_cause_not_the_family_total() {
    let run = |emp: Option<[f64; 2]>| {
        let mut card = buy("rare_radiation_meter", BucketId::Security, TierId::Y1, 25.0);
        card.rare_catastrophic = true;
        card.hazard_extras = vec![HazardId::NuclearAttack];
        let mut bag = buy("rare_faraday_storage", BucketId::Comms, TierId::Y1, 68.0);
        bag.rare_catastrophic = true;
        bag.hazard_extras = vec![HazardId::NuclearAttack];
        let mut s = Setup::new(PHL, 100.0, 0.0)
            .add(card, ItemMeta::new("rare_radiation_meter"))
            .add(bag, ItemMeta::new("rare_faraday_storage"));
        s.household.dials.rare_opt_in = vec!["nuclear_attack".into()];
        // The family's local total: about 1.2 in 10,000 over ten years (a remote county).
        s.risks.register.insert(HazardId::NuclearAttack, 1.2e-5);
        if let Some(range) = emp {
            s.risks.sub_causes.insert(
                HazardId::NuclearAttack,
                vec![rr_types::SubCause {
                    id: "emp".into(),
                    name: "Electromagnetic pulse (EMP) from a high-altitude burst".into(),
                    note: String::new(),
                    rate_range: Some(range),
                    sources: Vec::new(),
                }],
            );
        }
        s.run()
    };
    let r = run(Some([2.2e-5, 3.45e-3]));
    assert_eq!(
        month_of(&r, "rare_radiation_meter"),
        None,
        "the card needs the local total"
    );
    assert!(
        month_of(&r, "rare_faraday_storage").is_some(),
        "the bag needs a pulse"
    );
    let line = lines(&r)
        .into_iter()
        .find(|(_, i)| i.item_id == "rare_faraday_storage" && i.kind == PlanItemKind::Purchase)
        .unwrap()
        .1;
    assert!(
        line.why.contains(
            "It is for the pulse from a nuclear attack, part of the nuclear attack or EMP row you \
             ticked"
        ),
        "{}",
        line.why
    );
    assert_eq!(line.hazards, [HazardId::NuclearAttack]);
    // Valued on the pulse's chance: about 2.7 in 1,000 times the bag's 0.5 harm-days.
    let value = r
        .sequence
        .iter()
        .find(|p| p.item_id == "rare_faraday_storage")
        .unwrap()
        .value;
    let want = rr_budget::rare::p10_from_rate((2.2e-5_f64 * 3.45e-3).sqrt()) * 0.5;
    assert!((value - want).abs() < 1e-12, "{value} vs {want}");
    // No pulse part (the months-long family's rule outside the lower 48), or no sub-causes: no bag.
    for emp in [Some([0.0, 0.0]), None] {
        let r = run(emp);
        assert_eq!(month_of(&r, "rare_faraday_storage"), None, "{emp:?}");
        assert!(
            r.rare_catastrophic_skipped
                .contains(&ItemId::from("rare_faraday_storage"))
        );
    }
}

// ------------------------------------------------------------------------------------------------
// Bare-minimum mode and the two done months
// ------------------------------------------------------------------------------------------------

/// The kit rr-supply marks: 3 days of water (12 gallons for four people) and one light.
fn kit_setup(monthly: f32) -> Setup {
    let mut water = divisible(
        "water",
        Contributes::per_household(BucketId::WaterOut, 0.25),
        1.0,
    );
    water.minimum = vec![MinimumShare {
        line: "water_out.water_gallons".into(),
        units_per_item: 1.0,
        need: 12.0,
    }];
    let mut lamps = ItemMeta::new("headlamps");
    lamps.contributes = vec![Contributes::per_household(BucketId::Power, 0.75).part("lights")];
    lamps.set_quantity = Some(4.0);
    lamps.minimum = vec![MinimumShare {
        line: "power.lights".into(),
        units_per_item: 1.0,
        need: 1.0,
    }];
    Setup::new(PHL, monthly, 0.0)
        .flat(BucketId::WaterOut, 0.5, 3.0)
        .flat(BucketId::Power, 0.5, 3.0)
        .flat(BucketId::Supplies, 0.5, 3.0)
        .flat(BucketId::Comms, 0.5, 3.0)
        .add(
            item(
                "water",
                "Water",
                "gallon",
                &[BucketId::WaterOut],
                TierId::H72,
                false,
                false,
                (1.0, 1.0),
            ),
            water,
        )
        .add(buy("headlamps", BucketId::Power, TierId::H72, 10.0), lamps)
        .add(
            buy("food", BucketId::Supplies, TierId::H72, 30.0),
            set("food", BucketId::Supplies, 3.0),
        )
        .add(
            buy("radio", BucketId::Comms, TierId::H72, 20.0),
            set("radio", BucketId::Comms, 3.0),
        )
}

/// M-12: with `Dials::minimum_kit` the kit comes first, part sets included (one headlamp of the
/// four, the other three later), and the plan says it is in bare-minimum mode. Every plan reports
/// the month the kit is complete beside the month everything is.
#[test]
fn bare_minimum_mode_buys_the_kit_first_with_part_sets() {
    let normal = kit_setup(30.0).run();
    assert!(!normal.plan.minimum_kit);
    assert!(position(&normal, "radio") < position(&normal, "headlamps"));
    let kit_month = normal.plan.minimum_done_month.expect("reported");
    assert!(normal.plan.done_month.is_some_and(|d| d >= kit_month));

    let mut asked = kit_setup(30.0);
    asked.household.dials.minimum_kit = true;
    let r = asked.run();
    assert!(r.plan.minimum_kit);
    let first: Vec<&str> = r
        .sequence
        .iter()
        .take(2)
        .map(|p| p.item_id.as_str())
        .collect();
    assert!(
        first.contains(&"water") && first.contains(&"headlamps"),
        "{first:?}"
    );
    let lamp_buys: Vec<f64> = r
        .sequence
        .iter()
        .filter(|p| p.item_id == "headlamps")
        .map(|p| p.quantity)
        .collect();
    assert_eq!(lamp_buys, [1.0, 3.0]);
    assert!(r.sequence.iter().any(|p| p.minimum));
    assert_eq!(r.plan.minimum_done_month, Some(1));
    assert!(r.plan.minimum_done_month <= normal.plan.minimum_done_month);
    // Asked for, but the plan is short: no warning.
    assert!(!warned(&r, "plan_too_long"));
    let kit_line = lines(&r)
        .into_iter()
        .find(|(_, i)| i.item_id == "headlamps")
        .unwrap()
        .1;
    assert!(
        kit_line.why.contains("bare-minimum kit"),
        "{}",
        kit_line.why
    );
}

/// A plan that would run past 36 months in the normal order switches to bare-minimum mode by
/// itself and warns `plan_too_long`, naming what falls beyond three years.
#[test]
fn a_plan_past_three_years_switches_to_the_bare_minimum_and_says_what_waits() {
    let mut s = kit_setup(5.0);
    s = s.flat(BucketId::Medication, 0.5, 3.0).add(
        buy("generator", BucketId::Medication, TierId::H72, 250.0),
        set("generator", BucketId::Medication, 3.0),
    );
    let r = s.run();
    assert!(r.full_plan_stopped_month.is_some_and(|m| m > 36));
    assert!(r.plan.minimum_kit);
    let w = r
        .warnings
        .iter()
        .find(|w| w.id == "plan_too_long")
        .expect("warned");
    assert!(w.message.contains("more than three years"), "{}", w.message);
    let after_36: Vec<String> = r
        .sequence
        .iter()
        .filter(|p| p.month > 36)
        .map(|p| p.item_id.as_str().to_owned())
        .collect();
    assert!(!after_36.is_empty());
    for id in &after_36 {
        assert!(w.related.contains(id), "{id} in {:?}", w.related);
    }
    assert!(w.why.contains("Beyond three years"), "{}", w.why);
    // The $22 kit (12 gallons and one headlamp) at $5 a month, half of it saved toward each
    // purchase in turn: complete in month 5, while the whole plan runs past month 36.
    assert_eq!(r.plan.minimum_done_month, Some(5));
    assert!(w.why.contains("complete by month 5"), "{}", w.why);
}

// ------------------------------------------------------------------------------------------------
// Decisions and the other exempt free steps
// ------------------------------------------------------------------------------------------------

/// Decisions (and the clean-room plan) are free steps outside the eight-a-month count: never
/// more than eight ordinary free steps in a month, and every decision by month 1, marked as a
/// decision on its plan line.
#[test]
fn decisions_are_scheduled_by_month_1_outside_the_monthly_count() {
    let mut s = Setup::new(PHL, 50.0, 0.0).readiness(BucketId::Security, 0.5);
    for k in 0..14 {
        let (i, m) = free_step(&format!("step_{k:02}"), BucketId::Security, 1.0 + k as f64);
        s = s.add(i, m);
    }
    for id in ["decide_a", "decide_b", "decide_c"] {
        let mut d = item(
            id,
            id,
            "decision",
            &[BucketId::HomeLoss],
            TierId::Now,
            true,
            false,
            (0.0, 0.0),
        );
        d.decision = true;
        s = s.add(d, ItemMeta::new(id));
    }
    let (plan_item, pm) = free_step("fire_clean_room_plan", BucketId::Security, 0.01);
    s = s.add(plan_item, pm);
    let r = s.run();
    let exempt = |id: &str| id.starts_with("decide_") || id == "fire_clean_room_plan";
    for m in &r.plan.months {
        let ordinary = m
            .items
            .iter()
            .filter(|i| i.kind == PlanItemKind::FreeAction && !exempt(i.item_id.as_str()))
            .count();
        assert!(ordinary <= FREE_ACTIONS_MONTH_0, "month {}", m.index);
    }
    for (m, i) in lines(&r) {
        if exempt(i.item_id.as_str()) {
            assert!(m <= EXEMPT_BY_MONTH, "{} in month {m}", i.item_id);
        }
        assert_eq!(i.decision, i.item_id.as_str().starts_with("decide_"));
    }
    // Fourteen ordinary steps: eight in month 0, six in month 1, beside the four exempt ones.
    let month1 = &r.plan.months[1].items;
    assert_eq!(
        month1.iter().filter(|i| exempt(i.item_id.as_str())).count(),
        4
    );
}

// ------------------------------------------------------------------------------------------------
// The long-horizon section
// ------------------------------------------------------------------------------------------------

/// Long-horizon items are listed in `Plan::long_horizon` when a target reaches 30 days or the
/// household asks, and they stay in the months too (the flag only groups them).
#[test]
fn the_long_horizon_section_groups_items_without_dropping_them() {
    let run = |target: f64, asked: bool| {
        let mut carriers = buy("carriers", BucketId::WaterOut, TierId::H72, 30.0);
        carriers.long_horizon = true;
        let mut s = Setup::new(PHL, 100.0, 0.0)
            .flat(BucketId::WaterOut, 0.5, target)
            .add(
                carriers,
                set("carriers", BucketId::WaterOut, target.max(3.0)),
            );
        s.household.dials.long_horizon = asked;
        s.run()
    };
    let long = run(30.0, false);
    assert_eq!(long.plan.long_horizon.len(), 1);
    assert_eq!(long.plan.long_horizon[0].item_id, "carriers");
    assert!(month_of(&long, "carriers").is_some(), "still in the months");
    assert!(run(14.0, false).plan.long_horizon.is_empty());
    assert_eq!(run(14.0, true).plan.long_horizon.len(), 1);
}

// ------------------------------------------------------------------------------------------------
// Savings: the first step, the three-month point and the legal line
// ------------------------------------------------------------------------------------------------

fn savings_setup(fund_months: f32, expenses: f32) -> Setup {
    let mut s = Setup::new(PHL, 100.0, 0.0)
        .flat(BucketId::Supplies, 0.5, 3.0)
        .add(
            buy("food", BucketId::Supplies, TierId::H72, 50.0),
            set("food", BucketId::Supplies, 3.0),
        );
    s.household.finances.emergency_fund_months = fund_months;
    s.household.finances.monthly_expenses_usd = Some(expenses);
    let six = Target::Months {
        value: 6.0,
        low: 6.0,
        high: 6.0,
    };
    s.risks.assessments.insert(
        BucketId::Income,
        common::assessment(BucketId::Income, six, &[(HazardId::JobLoss, 1.0)]),
    );
    s
}

/// RR-P09: the first savings step is one month of expenses or $500, whichever is smaller, reached
/// from the supplies budget once the supplies plan is done (month 1 here): $500 at $100 a month
/// by month 6. The savings sentence adds the three-month point. Already saved: no first step.
#[test]
fn the_first_savings_step_and_the_three_month_point() {
    let r = savings_setup(0.0, 3000.0).run();
    assert_eq!(r.stopped_month, Some(1));
    let m = r.plan.first_milestone.clone().expect("a first step");
    assert_eq!((m.usd, m.by_month), (500.0, 6));
    assert!((m.months - 0.17).abs() < 1e-6, "{}", m.months);
    let why = &r.plan.savings_track.as_ref().unwrap().why;
    assert!(
        why.contains("The first step, $500, comes by month 6; three months of expenses, about $9,000, in about 8 years from now."),
        "{why}"
    );
    // Expenses under $500: one month of them.
    let small = savings_setup(0.0, 400.0).run();
    let m = small.plan.first_milestone.unwrap();
    assert_eq!((m.usd, m.months, m.by_month), (400.0, 1.0, 5));
    // A fifth of a month saved is $600, past the $500 step.
    assert_eq!(savings_setup(0.2, 3000.0).run().plan.first_milestone, None);
}

/// Verification R3-04: both savings dates count from today, and the sentence says so, while still
/// saying that the saving starts once the supplies are bought. A $3,000 item on $100 a month keeps
/// the supplies plan going for about thirty months, so the two starting points give different
/// years: the six-month goal ($18,000) comes about 180 months after the supplies plan ends, and
/// three months ($9,000) about 90 months after it.
#[test]
fn the_savings_dates_count_from_today() {
    let r = savings_setup(0.0, 3000.0)
        .flat(BucketId::Power, 0.5, 3.0)
        .add(
            buy("generator", BucketId::Power, TierId::H72, 3000.0),
            set("generator", BucketId::Power, 3.0),
        )
        .run();
    let done = r.stopped_month.expect("the supplies plan ends");
    assert!(done >= 25, "{done}");
    let years = |months: u16| (f64::from(months) / 12.0).round() as u16;
    let why = &r.plan.savings_track.as_ref().unwrap().why;
    assert!(
        why.contains(&format!(
            "Your supplies plan is done by month {done}. After that, your $100 a month for \
             supplies could go here, reaching the goal in about {} years from now.",
            years(done + 180)
        )),
        "{why}"
    );
    assert!(
        why.contains(&format!(
            "three months of expenses, about $9,000, in about {} years from now.",
            years(done + 90)
        )),
        "{why}"
    );
    // Counted from the end of the supplies plan, the goal would read about 15 years.
    assert_ne!(years(done + 180), 15);
}

/// The legal-emergency line shows only when the household turns it on and the arrest row is in
/// its register, apart from the months of income, with its citation handed to the provenance.
#[test]
fn the_legal_line_needs_the_opt_in_and_the_arrest_row() {
    let with = |opt_in: bool, arrest: bool| {
        let mut s = savings_setup(1.0, 3000.0);
        s.household.dials.legal_opt_in = opt_in;
        if arrest {
            s.risks.register.insert(HazardId::ArrestOrDetention, 0.03);
        }
        s.run()
    };
    let r = with(true, true);
    let why = &r.plan.savings_track.as_ref().unwrap().why;
    assert!(why.contains("Apart from these months"), "{why}");
    assert!(why.contains("$10,000"), "{why}");
    assert!(why.contains("about $1,000 on a $10,000 bail"), "{why}");
    assert!(why.contains("about 28 in 100 were under $5,000"), "{why}");
    assert_eq!(
        r.citations,
        [rr_types::CitationId::from(LEGAL_COST_CITATION)]
    );
    for (o, a) in [(false, true), (true, false)] {
        let r = with(o, a);
        let why = &r.plan.savings_track.as_ref().unwrap().why;
        assert!(!why.contains("bail"), "{why}");
        assert!(r.citations.is_empty());
    }
}

// ------------------------------------------------------------------------------------------------
// The v2 guardrails
// ------------------------------------------------------------------------------------------------

/// S2: a surge zone (most of the ZIP code, or the county's proxy class `high`) or a likely
/// evacuation, with nothing in the plan about leaving, warns; a leaving step quiets it.
#[test]
fn surge_zone_stay_home() {
    let run = |zip: Option<f64>, class: Option<&str>, p_leave: f64, leaving: bool| {
        let mut s = Setup::new(PHL, 50.0, 0.0).readiness(BucketId::Evacuate, p_leave);
        s.context.surge_zip_share = zip;
        s.context.surge_county_class = class.map(str::to_owned);
        if leaving {
            let (i, m) = free_step("evac_plan", BucketId::Evacuate, 2.0);
            s = s.add(i, m);
        }
        warned(&s.run(), "surge_zone_stay_home")
    };
    assert!(run(Some(0.8), None, 0.05, false));
    assert!(!run(Some(0.8), None, 0.05, true));
    assert!(run(None, Some("high"), 0.05, false));
    // The ZIP code's share decides over the county class when it is known.
    assert!(!run(Some(0.1), Some("high"), 0.05, false));
    assert!(run(Some(0.1), None, 0.3, false), "leaving is likely");
    assert!(!run(None, Some("moderate"), 0.05, false));
}

/// H7: a household that relies on pay or benefits a lapse can stop, with less than ten days of
/// food (or its target, if shorter) at the end of month 3, is warned; enough food or no benefit
/// is quiet.
#[test]
fn benefit_lapse() {
    let run = |benefit: bool, price: f32| {
        let food = item(
            "food",
            "Food",
            "person-day",
            &[BucketId::Supplies],
            TierId::H72,
            false,
            false,
            (price, price),
        );
        let mut s = Setup::new(PHL, 20.0, 0.0)
            .flat(BucketId::Supplies, 0.5, 14.0)
            .add(
                food,
                divisible(
                    "food",
                    Contributes::per_person(BucketId::Supplies, 1.0),
                    1.0,
                ),
            );
        if benefit {
            s.household.finances.benefits = vec![Benefit::SnapWic];
        }
        s.run()
    };
    // At $4 a person-day, $20 a month buys under ten days by month 3 (four people).
    let short = run(true, 4.0);
    let w = short
        .warnings
        .iter()
        .find(|w| w.id == "benefit_lapse")
        .expect("warned");
    assert!(w.message.contains("month 3"), "{}", w.message);
    assert!(!warned(&run(true, 0.25), "benefit_lapse"));
    assert!(!warned(&run(false, 4.0), "benefit_lapse"));
}

/// S6 and K1: a filter with no raw water source named warns (a well, a named source or a rain
/// barrel in the plan quiets it); a household that needs a way to cook and never gets one warns.
#[test]
fn no_raw_water_source_and_no_cooking_capability() {
    let with_rule = |id: &str, bucket: BucketId, rule: &str, price: f32| {
        let mut i = buy(id, bucket, TierId::H72, price);
        i.quantity_rule = rule.into();
        (i, set(id, bucket, 3.0))
    };
    let filter_plan = |source: Option<RawWaterSource>, barrel: bool| {
        let (f, fm) = with_rule(
            "filter",
            BucketId::WaterOut,
            "water_treatment_capacity",
            30.0,
        );
        let mut s = Setup::new(PHL, 100.0, 0.0)
            .flat(BucketId::WaterOut, 0.5, 6.0)
            .add(f, fm);
        if barrel {
            let (b, bm) = with_rule("barrel", BucketId::WaterOut, "rain_catchment_units", 40.0);
            s = s.add(b, bm);
        }
        s.household.housing.raw_water_source = source;
        warned(&s.run(), "no_raw_water_source")
    };
    assert!(filter_plan(None, false));
    assert!(filter_plan(Some(RawWaterSource::None), false));
    assert!(!filter_plan(Some(RawWaterSource::SurfaceNearby), false));
    assert!(!filter_plan(None, true));

    let cooking = |price: f32| {
        let (stove, sm) = with_rule("stove", BucketId::Supplies, "cooking_capability", price);
        let mut s = Setup::new(PHL, 20.0, 0.0)
            .flat(BucketId::Supplies, 0.5, 3.0)
            .add(stove, sm);
        s.options.max_months = 6;
        warned(&s.run(), "no_cooking_capability")
    };
    assert!(!cooking(30.0));
    assert!(cooking(900.0), "never bought within the horizon");
}

/// S1: refrigerated medicine that needs a power source (the power part exists) and no power in
/// the plan at all warns `cold_chain_power`; power bought late keeps the month-3 warning only.
#[test]
fn cold_chain_power_when_the_plan_never_powers_the_medicine() {
    let run = |station_price: f32, months: u16| {
        let mut bag = set("cooler_bag", BucketId::Medication, 1.0);
        bag.roles = vec![ItemRole::ColdChain];
        let mut b = buy("cooler_bag", BucketId::Medication, TierId::H72, 26.0);
        b.life_safety = true;
        let mut station = ItemMeta::new("station");
        station.contributes = vec![
            Contributes::per_household(BucketId::Power, 5.0)
                .part(rr_budget::COLD_MEDICINE_POWER_PART),
        ];
        let mut st = buy("station", BucketId::Power, TierId::H72, station_price);
        st.life_safety = true;
        let mut s = Setup::new("sugar-land-ev-household-3", 40.0, 0.0)
            .flat(BucketId::Medication, 0.2, 10.0)
            .flat(BucketId::Power, 0.5, 5.0)
            .add(b, bag)
            .add(st, station);
        s.options.max_months = months;
        let r = s.run();
        (
            warned(&r, "cold_chain_power"),
            warned(&r, "cold_chain_plan"),
        )
    };
    assert_eq!(
        run(483.0, 6),
        (true, false),
        "never powered within the plan"
    );
    assert_eq!(run(483.0, 60), (false, true), "powered, but after month 3");
    assert_eq!(run(30.0, 60), (false, false));
}

/// DESIGN §4.7's simultaneous-need check: when the plan is done but one event that sets a target
/// would need more stored water at once than the plan holds, a note says so; a need the event
/// rarely brings, or a plan still buying, says nothing.
#[test]
fn simultaneous_need_compares_one_event_with_what_the_plan_stores() {
    let run = |days: f64, chance: f64, monthly: f32| {
        let mut s = Setup::new(PHL, monthly, 0.0)
            .flat(BucketId::WaterOut, 0.5, 3.0)
            .add(
                buy("jug", BucketId::WaterOut, TierId::H72, 20.0),
                set("jug", BucketId::WaterOut, 3.0),
            );
        s.risks.simultaneous = vec![rr_budget::SimultaneousNeed {
            event: "a major hurricane".into(),
            hazard: HazardId::Hurricane,
            needs: vec![(BucketId::WaterOut, days, chance)],
        }];
        s.run()
    };
    let r = run(7.0, 1.0, 50.0);
    let w = r
        .warnings
        .iter()
        .find(|w| w.id == "simultaneous_need")
        .expect("noted");
    assert!(w.message.contains("a major hurricane"), "{}", w.message);
    assert!(w.why.contains("7 days without tap water"), "{}", w.why);
    assert!(!warned(&run(3.0, 1.0, 50.0), "simultaneous_need"));
    assert!(!warned(&run(7.0, 0.3, 50.0), "simultaneous_need"));
    assert!(!warned(&run(7.0, 1.0, 0.0), "simultaneous_need"));
}
