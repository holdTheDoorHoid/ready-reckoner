//! Inputs the fixtures do not cover: a named scenario switched off, owned items with recorded
//! prices, a large one-off budget, the climate and water dials, and the horizon's extremes.

mod common;

use common::{assess, household};
use rr_types::{
    BucketId, ClimateHorizon, ItemId, Owned, PlanItemKind, ScenarioToggle, Target, WaterLevel,
};

fn days(out: &rr_types::PlanOutput, b: BucketId) -> f32 {
    match out.buckets.iter().find(|x| x.id == b).unwrap().target {
        Target::Days { value, .. } | Target::Months { value, .. } => value,
        _ => f32::NAN,
    }
}

#[test]
fn switching_off_cascadia_lowers_the_coos_bay_targets() {
    let on = household("coos-bay-well-owner-2");
    let mut off = on.clone();
    off.dials.scenario_overrides.push(ScenarioToggle {
        id: "cascadia_m9".into(),
        on: false,
    });
    let (a, b) = (assess(&on), assess(&off));
    assert!(a.scenarios.iter().any(|s| s.id == "cascadia_m9" && s.on));
    assert!(b.scenarios.iter().any(|s| s.id == "cascadia_m9" && !s.on));
    assert!(days(&b, BucketId::Power) < days(&a, BucketId::Power));
    assert!(days(&b, BucketId::WaterOut) < days(&a, BucketId::WaterOut));
    assert!(b.packet_markdown.contains("left out of your plan"));
}

#[test]
fn owned_items_count_and_recorded_prices_replace_the_band() {
    let mut input = household("philadelphia-renters-4");
    input.existing = vec![
        Owned {
            item_id: ItemId::from("water_stored_bottled"),
            qty: 13.0,
            paid_usd: Some(10.0),
        },
        Owned {
            item_id: ItemId::from("power_headlamp"),
            qty: 4.0,
            paid_usd: None,
        },
        Owned {
            item_id: ItemId::from("fire_test_alarms"),
            qty: 1.0,
            paid_usd: None,
        },
    ];
    let out = assess(&input);
    let items: Vec<&rr_types::PlanItem> = out.plan.months.iter().flat_map(|m| &m.items).collect();
    let owned_water = items
        .iter()
        .find(|i| i.item_id == "water_stored_bottled" && i.done)
        .expect("the owned water is listed as done");
    assert_eq!(owned_water.paid_usd, Some(10.0));
    assert!(
        !items.iter().any(|i| i.item_id == "water_stored_bottled"
            && !i.done
            && i.kind == PlanItemKind::Purchase),
        "no more bottled water to buy"
    );
    assert!(
        items
            .iter()
            .any(|i| i.item_id == "fire_test_alarms" && i.done)
    );
    assert!(
        !items
            .iter()
            .any(|i| i.item_id == "power_headlamp" && !i.done && i.kind == PlanItemKind::Purchase)
    );
    // What the household already has lifts the step reached or at least the water covered.
    let water = out
        .buckets
        .iter()
        .find(|b| b.id == BucketId::WaterOut)
        .unwrap();
    assert!(matches!(water.covered, Target::Days { value, .. } if value >= 3.0));
}

#[test]
fn a_large_one_off_budget_finishes_early_and_never_overspends() {
    let mut input = household("philadelphia-renters-4");
    input.finances.one_off_budget_usd = 5000.0;
    let out = assess(&input);
    let done = out.plan.done_month.expect("done");
    assert!(done <= 2, "done in month {done}");
    let spent: f32 = out.plan.months[0]
        .items
        .iter()
        .filter(|i| i.kind == PlanItemKind::Purchase && !i.done)
        .map(|i| i.est_cost_usd)
        .sum();
    assert!(spent <= 5000.05, "month 0 spent {spent}");
}

#[test]
fn the_dials_change_targets_in_the_expected_direction() {
    let base = household("philadelphia-renters-4");
    let mut hot = base.clone();
    hot.dials.climate = ClimateHorizon::Y2050;
    let (a, b) = (assess(&base), assess(&hot));
    assert!(b.packet_markdown.contains("around 2050"));
    assert!(days(&b, BucketId::Thermal) >= days(&a, BucketId::Thermal));
    let water = |i: &rr_types::PlanInput| {
        assess(i)
            .requirements
            .iter()
            .find(|l| l.id == "water_out.water_gallons")
            .map_or(0.0, |l| l.quantity)
    };
    let mut survival = base.clone();
    survival.dials.water_level = WaterLevel::Survival;
    let mut comfortable = base.clone();
    comfortable.dials.water_level = WaterLevel::Comfortable;
    assert!(water(&survival) < water(&base));
    assert!(water(&base) < water(&comfortable));
}

#[test]
fn the_horizon_reaches_the_sentences() {
    let mut one = household("philadelphia-renters-4");
    one.dials.horizon_years = 1;
    let mut fifty = one.clone();
    fifty.dials.horizon_years = 50;
    let a = assess(&one);
    let b = assess(&fifty);
    assert!(a.packet_markdown.contains("the next year"));
    assert!(b.packet_markdown.contains("the next 50 years"));
    // Targets do not depend on the horizon (it only changes the sentences).
    assert_eq!(days(&a, BucketId::Power), days(&b, BucketId::Power));
}

#[test]
fn assumed_basics_are_credited_listed_and_can_be_switched_off() {
    let content = rr_content::content();
    let flagged: Vec<&str> = content
        .items
        .iter()
        .filter(|i| i.assumed_basic)
        .map(|i| i.id.as_str())
        .collect();
    let on = household("philadelphia-renters-4");
    assert!(on.assume_basics, "on by default");
    let mut off = on.clone();
    off.assume_basics = false;
    let (a, b) = (assess(&on), assess(&off));
    assert!(!b.warnings.iter().any(|w| w.id == "assumed_basics"));
    let spend = |o: &rr_types::PlanOutput| -> f32 {
        o.plan
            .months
            .iter()
            .flat_map(|m| &m.items)
            .filter(|i| i.kind == PlanItemKind::Purchase && !i.done)
            .map(|i| i.est_cost_usd)
            .sum()
    };
    if flagged.is_empty() {
        // The catalogue flags nothing yet: nothing to assume.
        assert!(!a.warnings.iter().any(|w| w.id == "assumed_basics"));
        return;
    }
    let note = a
        .warnings
        .iter()
        .find(|w| w.id == "assumed_basics")
        .expect("the assumption is recorded");
    assert!(note.related.iter().all(|id| flagged.contains(&id.as_str())));
    let month0 = &a.plan.months[0].items;
    for id in &note.related {
        let it = month0
            .iter()
            .find(|i| i.item_id == id.as_str() && i.done)
            .unwrap_or_else(|| panic!("{id} is credited in month 0"));
        assert_eq!(it.why, rr_plan::pipeline::ASSUMED_WHY);
    }
    assert!(
        a.packet_markdown
            .contains("What the plan assumes you already have")
    );
    assert!(
        spend(&a) <= spend(&b) + 0.01,
        "assuming basics never costs more"
    );
    // A household that lists a basic (even as none) keeps its own answer.
    let mut listed = on.clone();
    listed.existing.push(Owned {
        item_id: ItemId::from(note.related[0].as_str()),
        qty: 0.0,
        paid_usd: None,
    });
    let c = assess(&listed);
    let still = c
        .warnings
        .iter()
        .find(|w| w.id == "assumed_basics")
        .map(|w| w.related.clone())
        .unwrap_or_default();
    assert!(!still.contains(&note.related[0]));
}

#[test]
fn the_rare_catastrophe_opt_in_reaches_the_budget() {
    let content = rr_content::content();
    let rare: Vec<&str> = content
        .items
        .iter()
        .filter(|i| i.rare_catastrophic && !i.free)
        .map(|i| i.id.as_str())
        .collect();
    let base = household("philadelphia-renters-4");
    let mut opt_in = base.clone();
    opt_in.dials.rare_catastrophic_opt_in = true;
    let bought = |o: &rr_types::PlanOutput| -> Vec<String> {
        o.plan
            .months
            .iter()
            .flat_map(|m| &m.items)
            .filter(|i| i.kind == PlanItemKind::Purchase && rare.contains(&i.item_id.as_str()))
            .map(|i| i.item_id.as_str().to_owned())
            .collect()
    };
    assert!(bought(&assess(&base)).is_empty(), "$0 unless opted in");
    assert!(
        !bought(&assess(&opt_in)).is_empty(),
        "the allowance buys them"
    );
}
