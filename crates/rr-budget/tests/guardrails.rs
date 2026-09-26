//! Every guardrail (DESIGN §4.7): each fires when the plan looks off and stays quiet when it does
//! not. Guardrails are warnings; the plan is produced either way.

mod common;

use common::item;
use common::setup::{Setup, buy, set};
use rr_budget::{Cliff, ItemMeta, ItemRole, ReadinessCredit};
use rr_types::{BackupPower, BucketId, Tenure, TierId, WarningSeverity};

fn ids(s: &Setup) -> Vec<String> {
    s.run().warnings.into_iter().map(|w| w.id).collect()
}

fn water_bottles() -> (rr_types::Item, ItemMeta) {
    (
        item(
            "bottles",
            "Fill reused bottles",
            "gallon",
            &[BucketId::WaterOut],
            TierId::Now,
            true,
            true,
            (0.0, 0.0),
        ),
        {
            let mut m = set("bottles", BucketId::WaterOut, 0.25);
            m.set_quantity = Some(6.0);
            m
        },
    )
}

/// A water jug that stores drinking water (the stored-water guardrail's role).
fn jug(price: f32) -> (rr_types::Item, ItemMeta) {
    let mut m = set("jug", BucketId::WaterOut, 3.0);
    m.roles = vec![ItemRole::StoredWater];
    (buy("jug", BucketId::WaterOut, TierId::H72, price), m)
}

/// A plan with water (refilled bottles and a jug the household owns), so only the guardrail
/// under test fires.
fn base(fixture: &str, monthly: f32) -> Setup {
    let (b, m) = water_bottles();
    let (j, jm) = jug(20.0);
    let mut s = Setup::new(fixture, monthly, 0.0)
        .flat(BucketId::WaterOut, 0.1, 3.0)
        .add(b, m)
        .add(j, jm);
    s.household.existing = vec![rr_types::Owned {
        item_id: rr_types::ItemId::from("jug"),
        qty: 1.0,
        paid_usd: None,
        tested_on: None,
    }];
    s
}

#[test]
fn zero_budget_is_a_note() {
    let s = base("chicago-student-zero-budget-1", 0.0);
    let r = s.run();
    let w = r
        .warnings
        .iter()
        .find(|w| w.id == "zero_budget")
        .expect("zero_budget");
    assert_eq!(w.severity, WarningSeverity::Note);
    assert!(
        !r.plan.months[0].items.is_empty(),
        "the plan still lists free steps"
    );
    assert!(!ids(&base("chicago-student-zero-budget-1", 5.0)).contains(&"zero_budget".to_owned()));
}

#[test]
fn powered_device_without_backup_by_month_three() {
    // Phoenix: one adult with a CPAP, no backup power at home.
    let s = base("phoenix-apartment-cpap-1", 40.0).flat(BucketId::Power, 0.5, 3.0);
    assert!(ids(&s).contains(&"device_power_plan".to_owned()));
    // A life-safety power item bought in month 1 settles it.
    let mut battery = buy("cpap_battery", BucketId::Power, TierId::H72, 30.0);
    battery.life_safety = true;
    let fixed = base("phoenix-apartment-cpap-1", 40.0)
        .flat(BucketId::Power, 0.5, 3.0)
        .add(battery.clone(), set("cpap_battery", BucketId::Power, 1.0));
    assert!(!ids(&fixed).contains(&"device_power_plan".to_owned()));
    // Bought too late (month 4 at $10 a month for a $39 battery) still warns.
    let mut late = base("phoenix-apartment-cpap-1", 10.0).flat(BucketId::Power, 0.5, 3.0);
    let mut dear = battery.clone();
    dear.price_band_usd.low = 39.0;
    dear.price_band_usd.high = 39.0;
    late = late.add(dear, set("cpap_battery", BucketId::Power, 1.0));
    assert_eq!(
        late.run()
            .sequence
            .iter()
            .find(|p| p.item_id == "cpap_battery")
            .map(|p| p.month),
        Some(4)
    );
    assert!(ids(&late).contains(&"device_power_plan".to_owned()));
    // Backup power at home already counts.
    let mut home = base("phoenix-apartment-cpap-1", 40.0).flat(BucketId::Power, 0.5, 3.0);
    home.household.housing.backup_power = BackupPower::Generator;
    assert!(!ids(&home).contains(&"device_power_plan".to_owned()));
    // Nobody with a device: no warning.
    assert!(!ids(&base("philadelphia-renters-4", 40.0)).contains(&"device_power_plan".to_owned()));
}

#[test]
fn refrigerated_medicine_without_a_cooling_plan() {
    // Miami: a retiree on refrigerated medicine.
    let s = base("miami-condo-retiree-1", 40.0);
    assert!(ids(&s).contains(&"cold_chain_plan".to_owned()));
    let mut plan = ItemMeta::new("insulin_cooling_plan");
    plan.roles = vec![ItemRole::ColdChain];
    let fixed = base("miami-condo-retiree-1", 40.0).add(
        item(
            "insulin_cooling_plan",
            "Plan to keep medicine cold",
            "plan",
            &[BucketId::Medication],
            TierId::Now,
            true,
            true,
            (0.0, 0.0),
        ),
        plan,
    );
    assert!(!ids(&fixed).contains(&"cold_chain_plan".to_owned()));
}

/// Refilled bottles are a start, not a supply. When they leave the no-water target short and no
/// stored water is owned or bought by month 3, the plan warns (verification V-16: the old "no
/// water after month 1" could never fire, because the free step always gave some water).
#[test]
fn no_stored_water_beyond_refilled_bottles_by_month_3() {
    let id = "no_stored_water_by_month_3";
    let bottles_only = |monthly: f32, target: f64| {
        let (b, m) = water_bottles();
        Setup::new("philadelphia-renters-4", monthly, 0.0)
            .flat(BucketId::WaterOut, 0.1, target)
            .add(b, m)
    };
    // Refilled bottles give 1.5 of 3 days, and nothing stores water: warns.
    let r = bottles_only(60.0, 3.0).run();
    let w = r.warnings.iter().find(|w| w.id == id).expect("warns");
    assert_eq!(w.severity, WarningSeverity::Warn);
    assert_eq!(w.related, ["water_out"]);
    assert!(w.message.contains("refilled bottles by month 3"));
    // A jug bought by month 3: quiet.
    let (j, jm) = jug(20.0);
    let r = bottles_only(60.0, 3.0).add(j, jm).run();
    assert!(
        r.sequence
            .iter()
            .any(|p| p.item_id == "jug" && p.month <= 3),
        "{:?}",
        r.sequence
    );
    assert!(!r.warnings.iter().any(|w| w.id == id));
    // A jug the household already owns: quiet.
    assert!(!ids(&base("philadelphia-renters-4", 60.0)).contains(&id.to_owned()));
    // A jug that arrives only after month 3 ($3 a month toward $20): warns.
    let (j, jm) = jug(20.0);
    let r = bottles_only(3.0, 3.0).add(j, jm).run();
    assert!(r.sequence.iter().any(|p| p.item_id == "jug" && p.month > 3));
    assert!(r.warnings.iter().any(|w| w.id == id));
    // Refilled bottles that cover the whole target leave nothing to store: quiet.
    let (j, jm) = jug(20.0);
    assert!(!ids(&bottles_only(60.0, 1.0).add(j, jm)).contains(&id.to_owned()));
    // No no-water target: quiet.
    let (b, m) = water_bottles();
    let dry_land = Setup::new("philadelphia-renters-4", 0.0, 0.0).add(b, m);
    assert!(!ids(&dry_land).contains(&id.to_owned()));
}

#[test]
fn evacuation_heavy_household_without_a_go_bag() {
    let bag = || {
        let mut m = ItemMeta::new("go_bag");
        m.readiness = vec![ReadinessCredit {
            bucket: BucketId::Evacuate,
            harm_day_equivalents: 2.0,
        }];
        (buy("go_bag", BucketId::Evacuate, TierId::H72, 30.0), m)
    };
    // One in five households like this one have to leave within ten years, and no bag is offered.
    let heavy = base("philadelphia-renters-4", 60.0).readiness(BucketId::Evacuate, 0.2);
    let r = heavy.run();
    let w = r
        .warnings
        .iter()
        .find(|w| w.id == "evacuation_no_go_bag")
        .expect("warns");
    assert!(
        w.message.contains("About 20 of 100 households like yours"),
        "{}",
        w.message
    );
    // A bag in the plan within six months settles it.
    let (b, m) = bag();
    let with_bag = base("philadelphia-renters-4", 60.0)
        .readiness(BucketId::Evacuate, 0.2)
        .add(b, m);
    assert!(!ids(&with_bag).contains(&"evacuation_no_go_bag".to_owned()));
    // Rarely needed (5 %): no warning.
    let light = base("philadelphia-renters-4", 60.0).readiness(BucketId::Evacuate, 0.05);
    assert!(!ids(&light).contains(&"evacuation_no_go_bag".to_owned()));
}

#[test]
fn owners_in_flood_or_quake_zones_without_insurance() {
    let mut own = base("philadelphia-renters-4", 60.0);
    own.household.housing.tenure = Tenure::Own;
    own.context.flood_zone = true;
    own.context.quake_zone = true;
    let got = ids(&own);
    assert!(
        got.contains(&"insurance_flood".to_owned()) && got.contains(&"insurance_quake".to_owned())
    );
    own.household.finances.insurance.flood = true;
    own.household.finances.insurance.earthquake = true;
    let got = ids(&own);
    assert!(
        !got.contains(&"insurance_flood".to_owned())
            && !got.contains(&"insurance_quake".to_owned())
    );
    // Renters are not warned (the design scopes this guardrail to owners).
    let mut rent = base("philadelphia-renters-4", 60.0);
    rent.context.flood_zone = true;
    assert!(!ids(&rent).contains(&"insurance_flood".to_owned()));
}

#[test]
fn a_cliff_is_a_note_with_the_bucket() {
    let mut s = base("coos-bay-well-owner-2", 60.0);
    s.context.cliffs = vec![Cliff {
        bucket: BucketId::WaterOut,
        driver: "a Cascadia earthquake".into(),
    }];
    let r = s.run();
    let w = r
        .warnings
        .iter()
        .find(|w| w.id == "cliff_water_out")
        .expect("cliff note");
    assert_eq!(w.severity, WarningSeverity::Note);
    assert!(w.message.contains("a Cascadia earthquake"));
    assert!(w.why.contains("water filter"));
}

#[test]
fn a_bucket_nothing_covers_is_a_note_not_a_false_finish() {
    let s = base("philadelphia-renters-4", 60.0).flat(BucketId::Thermal, 0.05, 2.0);
    let r = s.run();
    assert!(r.warnings.iter().any(|w| w.id == "uncovered_thermal"));
    assert_eq!(r.plan.done_month, None, "not every target is met");
    assert!(
        r.stopped_month.is_some(),
        "but there is nothing left to buy"
    );
}

#[test]
fn warnings_never_block_the_plan() {
    // Everything wrong at once: zero budget, device, refrigerated medicine, no water, owner in a
    // flood zone. The plan is still produced, with every warning in a fixed order.
    let mut s =
        Setup::new("sugar-land-ev-household-3", 0.0, 0.0).flat(BucketId::WaterOut, 0.1, 3.0);
    s.household.housing.tenure = Tenure::Own;
    s.context.flood_zone = true;
    s.household.people[0].medical.powered_device = rr_types::PoweredDevice::Oxygen;
    let r = s.run();
    let got: Vec<&str> = r.warnings.iter().map(|w| w.id.as_str()).collect();
    // Sugar Land owns no flood policy, and nothing in this catalogue stores water, so all six
    // fire, in the documented order.
    assert_eq!(
        got,
        [
            "zero_budget",
            "device_power_plan",
            "cold_chain_plan",
            "no_stored_water_by_month_3",
            "insurance_flood",
            "uncovered_water_out"
        ]
    );
    assert!(!r.plan.months.is_empty());
    for w in &r.warnings {
        assert!(!w.message.is_empty() && !w.why.is_empty());
        assert!(w.why.len() < 400, "{}", w.why);
    }
}
