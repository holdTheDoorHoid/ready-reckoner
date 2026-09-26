//! Round-2 review (v0.1.1) changes to the allocator: the first three free steps for every
//! household (N7), the extinguisher valued on the small fires nobody reports (P-06), and the
//! cold-chain guardrail that stays on until refrigerated medicine has a power source (S1).

mod common;

use common::item;
use common::setup::{Setup, buy, set};
use rr_budget::{
    COLD_MEDICINE_POWER_PART, Contributes, FIRST_FREE_STEPS, ItemMeta, ItemRole, LAST_FREE_STEPS,
    READINESS_NEED_OVERRIDES, ReadinessCredit,
};
use rr_types::{BucketId, PlanItemKind, TierId};

/// A free step with a readiness credit (`harm` day-equivalents on `bucket`).
fn step(id: &str, bucket: BucketId, harm: f64, life_safety: bool) -> (rr_types::Item, ItemMeta) {
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
            life_safety,
            (0.0, 0.0),
        ),
        m,
    )
}

/// The free steps month 0 lists, in order.
fn month0_free(s: &Setup) -> Vec<String> {
    s.run().plan.months[0]
        .items
        .iter()
        .filter(|i| i.kind == PlanItemKind::FreeAction && !i.done)
        .map(|i| i.item_id.as_str().to_owned())
        .collect()
}

/// A catalogue where the three first steps are worth little and not flagged life-safety, the
/// antibiotics card is worth a lot, and `others` other steps (half of them life-safety) compete.
fn free_step_setup(fixture: &str, others: usize) -> Setup {
    let mut s = Setup::new(fixture, 50.0, 0.0)
        .readiness(BucketId::Security, 0.5)
        .readiness(BucketId::MedicalEmergency, 0.9)
        .readiness(BucketId::Fire, 0.3);
    for id in FIRST_FREE_STEPS {
        let (i, m) = step(id, BucketId::Security, 0.01, false);
        s = s.add(i, m);
    }
    for id in LAST_FREE_STEPS {
        let (i, m) = step(id, BucketId::MedicalEmergency, 5.0, false);
        s = s.add(i, m);
    }
    for j in 0..others {
        let id = format!("other_step_{j}");
        let (i, m) = step(&id, BucketId::Fire, 1.0 + j as f64, j % 2 == 0);
        s = s.add(i, m);
    }
    s
}

/// N7: alerts, the household plan and fire safety lead month 0 for every household, whatever
/// their value; the antibiotics card takes a month-0 slot only when one is left over.
#[test]
fn the_first_three_free_steps_lead_month_0_for_every_fixture() {
    assert_eq!(
        FIRST_FREE_STEPS,
        [
            "comms_wea_alerts_on",
            "comms_contact_card",
            "fire_test_alarms"
        ]
    );
    for (name, _) in rr_types::fixtures::all() {
        let busy = month0_free(&free_step_setup(name, 9));
        assert_eq!(busy.len(), 8, "{name}: {busy:?}");
        assert_eq!(busy[..3], FIRST_FREE_STEPS, "{name}: {busy:?}");
        assert!(
            !busy.iter().any(|id| id == LAST_FREE_STEPS[0]),
            "{name}: the antibiotics card leaves month 0 when it is full: {busy:?}"
        );
        // After the fixed three, life-safety steps first, then the rest by value.
        let rest = &busy[3..];
        assert!(
            rest[..3].iter().all(|id| id
                .trim_start_matches("other_step_")
                .parse::<usize>()
                .unwrap()
                % 2
                == 0),
            "{name}: {busy:?}"
        );
        // It is scheduled later, never dropped.
        let r = free_step_setup(name, 9).run();
        assert!(
            r.plan
                .months
                .iter()
                .flat_map(|m| &m.items)
                .any(|i| i.item_id == LAST_FREE_STEPS[0]),
            "{name}"
        );
        // With room to spare it stays in month 0, after everything else.
        let quiet = month0_free(&free_step_setup(name, 3));
        assert_eq!(quiet[..3], FIRST_FREE_STEPS, "{name}: {quiet:?}");
        assert_eq!(
            quiet.last().map(String::as_str),
            Some(LAST_FREE_STEPS[0]),
            "{name}: {quiet:?}"
        );
    }
}

/// A household that has already done one of the first steps skips it; the others keep their
/// order.
#[test]
fn a_first_step_already_done_is_skipped() {
    let mut s = free_step_setup("philadelphia-renters-4", 9);
    s.household.existing = vec![rr_types::Owned {
        item_id: rr_types::ItemId::from("comms_contact_card"),
        qty: 1.0,
        paid_usd: None,
    }];
    let m0 = month0_free(&s);
    assert_eq!(
        m0[..2],
        ["comms_wea_alerts_on", "fire_test_alarms"],
        "{m0:?}"
    );
    assert!(!m0.iter().any(|id| id == "comms_contact_card"));
}

/// P-06: the extinguisher is valued on CPSC's all-fires rate (about 6.6 fires a year for every
/// 100 households, most small and never reported), not the fire bucket's reported house fires,
/// and its sentence says what the event is.
#[test]
fn the_extinguisher_is_valued_on_the_small_fires_nobody_reports() {
    let (_, bucket, rate, words) = READINESS_NEED_OVERRIDES
        .iter()
        .find(|(id, ..)| *id == "fire_extinguisher")
        .copied()
        .unwrap();
    assert_eq!((bucket, rate), (BucketId::Fire, 0.066));
    let fire_item = |id: &str| {
        let mut m = ItemMeta::new(id);
        m.readiness = vec![ReadinessCredit {
            bucket: BucketId::Fire,
            harm_day_equivalents: 5.0,
        }];
        (buy(id, BucketId::Fire, TierId::H72, 30.0), m)
    };
    let (e, em) = fire_item("fire_extinguisher");
    let (o, om) = fire_item("other_fire_item");
    let r = Setup::new("philadelphia-renters-4", 200.0, 0.0)
        .readiness(BucketId::Fire, 0.05)
        .add(e, em)
        .add(o, om)
        .run();
    let value = |id: &str| {
        r.sequence
            .iter()
            .find(|p| p.item_id == id)
            .map(|p| p.value)
            .unwrap()
    };
    // The same harm and price: the value scales with the yearly need, 0.066 against the fire
    // bucket's 5 in 100 over ten years (0.0051 a year).
    let ratio = value("fire_extinguisher") / value("other_fire_item");
    let want = 0.066 / rr_budget::value::annual_rate_from_10yr(0.05);
    assert!((ratio - want).abs() < 1e-6 * want, "{ratio} vs {want}");
    let line = r
        .plan
        .months
        .iter()
        .flat_map(|m| &m.items)
        .find(|i| i.item_id == "fire_extinguisher")
        .unwrap();
    assert!(
        line.why.contains(&format!(
            "About 50 of 100 households like yours {words} in the next 10 years."
        )),
        "{}",
        line.why
    );
}

/// S1: where refrigerated medicine needs a power source, a cooler bag alone no longer silences
/// the cold-chain guardrail: it stays on until the power part (rr-plan's name for rr-supply's
/// `power_for_cold_medicine` line) is covered too.
#[test]
fn the_cold_chain_guardrail_waits_for_the_medicine_power() {
    let bag = || {
        let mut m = set("cooler_bag", BucketId::Medication, 1.0);
        m.roles = vec![ItemRole::ColdChain];
        let mut i = buy("cooler_bag", BucketId::Medication, TierId::H72, 26.0);
        i.life_safety = true;
        (i, m)
    };
    let station = || {
        let mut m = ItemMeta::new("station");
        m.contributes =
            vec![Contributes::per_household(BucketId::Power, 5.0).part(COLD_MEDICINE_POWER_PART)];
        let mut i = buy("station", BucketId::Power, TierId::H72, 483.0);
        i.life_safety = true;
        (i, m)
    };
    let lights = || {
        let mut m = ItemMeta::new("lights");
        m.contributes = vec![Contributes::per_household(BucketId::Power, 5.0).part("lights")];
        (buy("lights", BucketId::Power, TierId::H72, 20.0), m)
    };
    let plan = |monthly: f32, one_off: f32, with_station: bool| {
        let (b, bm) = bag();
        let (l, lm) = lights();
        let mut s = Setup::new("sugar-land-ev-household-3", monthly, one_off)
            .flat(BucketId::Medication, 0.2, 10.0)
            .flat(BucketId::Power, 0.5, 5.0)
            .add(b, bm)
            .add(l, lm);
        if with_station {
            let (st, sm) = station();
            s = s.add(st, sm);
        }
        s.run()
    };
    let warns = |r: &rr_budget::BudgetResult| r.warnings.iter().any(|w| w.id == "cold_chain_plan");
    // $40 a month: the bag comes at once, the $483 station only after months of saving.
    let slow = plan(40.0, 0.0, true);
    let month = |r: &rr_budget::BudgetResult, id: &str| {
        r.sequence.iter().find(|p| p.item_id == id).map(|p| p.month)
    };
    assert!(month(&slow, "cooler_bag").is_some_and(|m| m <= 1));
    assert!(month(&slow, "station").is_some_and(|m| m > 3));
    assert!(
        warns(&slow),
        "the bag alone does not cover a 5-day power cut"
    );
    let w = slow
        .warnings
        .iter()
        .find(|w| w.id == "cold_chain_plan")
        .unwrap();
    assert!(w.message.contains("through a power cut"), "{}", w.message);
    assert!(w.why.contains("power station"), "{}", w.why);
    // The one-off money buys the station in month 0: no warning.
    let fast = plan(40.0, 1000.0, true);
    assert_eq!(month(&fast, "station"), Some(0));
    assert!(!warns(&fast));
    // Where no power part exists (a short power target, or backup power at home), the bag is
    // the cold chain, as before.
    assert!(!warns(&plan(40.0, 0.0, false)));
}

/// RR-P16: renters with no smoke alarms get no purchase (the landlord comes first), so the plan
/// says so in a warning the packet prints; owners and homes with alarms get none.
#[test]
fn renters_without_smoke_alarms_are_told_to_ask_the_landlord() {
    let run = |smoke: bool, tenure: rr_types::Tenure| {
        let mut s = Setup::new("philadelphia-renters-4", 60.0, 0.0);
        s.household.housing.alarms.smoke = smoke;
        s.household.housing.tenure = tenure;
        s.run().warnings
    };
    let w = run(false, rr_types::Tenure::Rent);
    let w = w
        .iter()
        .find(|w| w.id == "smoke_alarms_landlord")
        .expect("renters without alarms are told");
    assert!(w.message.contains("ask your landlord"), "{}", w.message);
    assert!(w.why.contains("Red Cross"), "{}", w.why);
    for (smoke, tenure) in [
        (true, rr_types::Tenure::Rent),
        (false, rr_types::Tenure::Own),
    ] {
        assert!(
            !run(smoke, tenure)
                .iter()
                .any(|w| w.id == "smoke_alarms_landlord"),
            "{smoke} {tenure:?}"
        );
    }
}
