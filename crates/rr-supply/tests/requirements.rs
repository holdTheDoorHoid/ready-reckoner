//! Requirement lines for the fixture households: the numbers the research gives, every line cited
//! and well formed, determinism, and the content policy.

mod common;

use std::collections::BTreeSet;

use rr_supply::{
    LineKind, ONE_MONTH_NOTE, SupplyContext, citations_used, requirements, requirements_with,
    rule_ids, sized_requirements, tier_recommended,
};
use rr_types::{BucketId, Per, RequirementLine, TierId, fixtures};

fn line<'a>(lines: &'a [RequirementLine], id: &str) -> &'a RequirementLine {
    lines.iter().find(|l| l.id == id).unwrap_or_else(|| {
        panic!(
            "no line {id}; have {:?}",
            lines.iter().map(|l| &l.id).collect::<Vec<_>>()
        )
    })
}

fn has(lines: &[RequirementLine], id: &str) -> bool {
    lines.iter().any(|l| l.id == id)
}

#[test]
fn philadelphia_matches_the_research() {
    let input = fixtures::get("philadelphia-renters-4").unwrap();
    let targets = common::philadelphia();
    let lines = requirements(&input, &targets);

    // Brief: 3-day water 12-13 gallons at the basic level (4 people × 1 gal × 3 days + a 40 lb dog).
    let water = line(&lines, "water_out.water_gallons");
    assert!(
        (12.0..=13.0).contains(&water.quantity),
        "{}",
        water.quantity
    );
    assert_eq!(water.quantity, 12.9);
    assert_eq!(water.unit, "gallon");
    assert!(
        water
            .plain
            .starts_with("4 people × 1 gallon a day × 3 days = 12 gallons")
    );
    assert!(
        !has(&lines, "water_out.water_treatment_capacity"),
        "3 days is under the 14 stored days"
    );

    assert_eq!(
        line(&lines, "water_boil.water_treatment_capacity").quantity,
        13.3
    );
    assert_eq!(line(&lines, "supplies.food_kcal").quantity, 82_000.0);
    assert_eq!(
        line(&lines, "supplies.food_kcal.alt.pantry").quantity,
        338.0
    );
    assert_eq!(line(&lines, "medication.medication_days").quantity, 14.0);
    assert_eq!(line(&lines, "medication.antibiotics_none").quantity, 0.0);
    assert_eq!(line(&lines, "comms.phone_power_wh").quantity, 140.0);
    assert_eq!(line(&lines, "comms.cash_reserve_usd").quantity, 100.0);
    assert_eq!(line(&lines, "fire.co_alarm_count").quantity, 2.0);
    assert!(
        !has(&lines, "fire.smoke_alarm_count"),
        "the household has smoke alarms"
    );
    assert_eq!(line(&lines, "income.emergency_fund_months").quantity, 3.8);
    assert_eq!(
        line(&lines, "get_home.get_home_water.person_1").quantity,
        2.0
    );
    assert_eq!(
        line(&lines, "get_home.get_home_water.person_2").quantity,
        0.6
    );
    assert_eq!(line(&lines, "evacuate.go_bag").quantity, 4.0);
    assert_eq!(line(&lines, "evacuate.pet_carrier").quantity, 1.0);
    // Thermal is driven by both heat and cold.
    assert!(has(&lines, "thermal.battery_fan") && has(&lines, "thermal.sleeping_bag_or_blanket"));
    // Flooding and fire drive home loss, not earthquakes.
    assert!(has(&lines, "home_loss.insurance_flood"));
    assert!(!has(&lines, "home_loss.insurance_earthquake"));

    // Nothing reaches the one-month tier, so the note is not said.
    assert!(lines.iter().all(|l| !l.plain.contains(ONE_MONTH_NOTE)));
    assert_eq!(tier_recommended(&targets), TierId::W2);
}

#[test]
fn coos_bay_prefers_a_filter_and_a_source_to_a_hundred_gallons() {
    let input = fixtures::get("coos-bay-well-owner-2").unwrap();
    let targets = common::coos_bay();
    let lines = requirements(&input, &targets);

    let stored = line(&lines, "water_out.water_gallons");
    assert!(stored.quantity < 100.0, "{}", stored.quantity);
    assert_eq!(stored.quantity, 37.6); // 14 days × 2.6875 gal
    assert!(stored.plain.contains("filter"), "{}", stored.plain);
    let treat = line(&lines, "water_out.water_treatment_capacity");
    assert_eq!(treat.per, Per::Household);
    assert_eq!(treat.quantity, 96.8); // 36 days × 2.6875 gal
    assert!(treat.plain.contains("filter") && treat.plain.contains("source"));

    assert_eq!(line(&lines, "power.generator_fuel_gallons").quantity, 25.0);
    assert!(has(&lines, "power.well_pump_wh.optional"));
    assert!(has(&lines, "home_loss.insurance_earthquake"));
    assert!(
        lines.iter().all(|l| l.bucket != BucketId::Thermal),
        "wood heat: thermal target 0"
    );
    assert!(lines.iter().all(|l| l.bucket != BucketId::WaterBoil));
    assert!(line(&lines, "evacuate.go_bag").plain.contains("minutes"));

    // 17 days of food is the first line in the one-month tier: the note is said there, once.
    let noted: Vec<&RequirementLine> = lines
        .iter()
        .filter(|l| l.plain.contains(ONE_MONTH_NOTE))
        .collect();
    assert_eq!(noted.len(), 1);
    assert_eq!(noted[0].id, "supplies.food_kcal");
    assert!(
        noted[0]
            .citations
            .iter()
            .any(|c| c == "church_home_storage_2007")
    );
    assert_eq!(tier_recommended(&targets), TierId::M3);
}

#[test]
fn every_line_is_cited_and_well_formed() {
    let known = citations_used();
    let rules: BTreeSet<&str> = rule_ids().into_iter().collect();
    let units = [
        "gallon",
        "litre",
        "kcal",
        "usd",
        "day",
        "person_day",
        "pet_day",
        "person_month",
        "month",
        "lb",
        "oz",
        "Wh",
        "course",
        "person",
        "light",
        "radio",
        "card",
        "map",
        "bucket",
        "bag",
        "cup",
        "roll",
        "cycle",
        "diaper",
        "pack",
        "kit",
        "package",
        "mask",
        "thermometer",
        "packet",
        "fan",
        "towel",
        "plan",
        "item",
        "set",
        "alarm",
        "extinguisher",
        "ladder",
        "contact",
        "decision",
        "carrier",
        "vehicle",
        "battery",
        "tablet",
        "bottle",
        "canister",
        "power station",
        "generator",
        "panel",
    ];
    for (name, input, targets) in common::all() {
        for ctx in [
            SupplyContext::default(),
            SupplyContext {
                days_at_or_above_95f: Some(120.0),
                latitude: Some(33.4),
                ..SupplyContext::default()
            },
        ] {
            let lines = sized_requirements(&input, &targets, &ctx);
            assert!(lines.len() >= 30, "{name}: only {} lines", lines.len());
            let mut ids = BTreeSet::new();
            for s in &lines {
                let l = &s.line;
                assert!(ids.insert(l.id.clone()), "{name}: duplicate id {}", l.id);
                assert!(l.id.starts_with(&format!("{}.", l.bucket)), "{}", l.id);
                assert!(l.id.split('.').all(rr_types::is_well_formed_id), "{}", l.id);
                assert_eq!(LineKind::of(&l.id), s.kind, "{}", l.id);
                assert!(
                    rules.contains(l.rule.as_str()),
                    "{name}: unknown rule {}",
                    l.rule
                );
                assert!(
                    units.contains(&l.unit.as_str()),
                    "{name}: unit {} on {}",
                    l.unit,
                    l.id
                );
                assert!(
                    l.quantity.is_finite() && l.quantity >= 0.0,
                    "{}: {}",
                    l.id,
                    l.quantity
                );
                if s.kind == LineKind::Need && l.rule != "antibiotics_none" {
                    assert!(l.quantity > 0.0, "{name}: zero need {}", l.id);
                }
                assert!(!l.citations.is_empty(), "{name}: {} cites nothing", l.id);
                for c in &l.citations {
                    assert!(
                        known.contains(c) || c == common::TARGET_SOURCE,
                        "{name}: {} cites unknown {c}",
                        l.id
                    );
                }
                assert_eq!(
                    s.prior,
                    l.citations.iter().any(|c| c == "rr_expert_prior"),
                    "{}",
                    l.id
                );
                assert!(
                    !l.plain.is_empty() && l.plain.ends_with(')'),
                    "{}: {}",
                    l.id,
                    l.plain
                );
                for bad in ["  ", " ,", "..", "-0 "] {
                    assert!(!l.plain.contains(bad), "{}: {bad:?} in {}", l.id, l.plain);
                }
                let words: Vec<&str> = l.plain.split(|c: char| !c.is_alphanumeric()).collect();
                for bad in ["NaN", "inf", "infinity"] {
                    assert!(!words.contains(&bad), "{}: {bad} in {}", l.id, l.plain);
                }
                assert!(!l.item_class.is_empty());
                if let Some(of) = LineKind::alternative_to(&l.id) {
                    assert!(
                        lines.iter().any(|n| n.line.id == of),
                        "{}: no need line {of}",
                        l.id
                    );
                }
                if let (Some(days), Some(per_day)) = (s.days, s.per_day) {
                    assert!(days > 0.0 && per_day >= 0.0, "{}", l.id);
                }
            }
        }
    }
}

#[test]
fn every_duration_and_readiness_bucket_gets_lines() {
    for (name, input, targets) in common::all() {
        let lines = requirements(&input, &targets);
        for b in BucketId::ALL {
            let expect = match (name, b) {
                // Coos Bay: the wood stove covers cold and a well has no boil-water notices.
                ("coos-bay-well-owner-2", BucketId::Thermal | BucketId::WaterBoil) => false,
                _ => true,
            };
            assert_eq!(lines.iter().any(|l| l.bucket == *b), expect, "{name}: {b}");
        }
    }
}

#[test]
fn deterministic_byte_for_byte() {
    for (name, input, targets) in common::all() {
        let a = requirements(&input, &targets);
        let b = requirements(&input.clone(), &targets.clone());
        assert_eq!(a, b, "{name}");
        let ja = serde_json::to_string(&a).unwrap();
        let jb = serde_json::to_string(&b).unwrap();
        assert_eq!(ja, jb, "{name}");
        // And the lines survive a JSON round trip unchanged (they are contract types).
        let back: Vec<RequirementLine> = serde_json::from_str(&ja).unwrap();
        assert_eq!(back, a, "{name}");
    }
}

#[test]
fn content_policy_no_dosing_no_firearms_no_brands() {
    let brands = [
        "thermos",
        "band-aid",
        "lifestraw",
        "sawyer",
        "jackery",
        "goal zero",
        "honda",
        "midland",
        "mountain house",
        "readywise",
        "augason",
        "berkey",
        "yeti",
        "coleman",
        "garmin",
        "baofeng",
        "anker",
        "ecoflow",
        "bluetti",
        "zippo",
        "leatherman",
        "nalgene",
        "aquatainer",
        "aqua-tainer",
        "waterbrick",
        "reliance",
        "luggable",
        "westinghouse",
        "resmed",
    ];
    let firearms = [
        "firearm",
        "gun",
        "ammunition",
        "ammo",
        "rifle",
        "pistol",
        "weapon",
    ];
    let dosing = [
        " mg",
        "milligram",
        "tablets a day",
        "every 8 hours",
        "twice a day",
        "dose of",
    ];
    for (name, input, targets) in common::all() {
        for l in requirements(&input, &targets) {
            let text = l.plain.to_lowercase();
            for w in brands.iter().chain(firearms.iter()) {
                assert!(!text.contains(w), "{name}: {w:?} in {}: {}", l.id, l.plain);
            }
            if l.bucket == BucketId::Medication || l.bucket == BucketId::MedicalEmergency {
                for w in dosing {
                    assert!(!text.contains(w), "{name}: dosing {w:?} in {}", l.id);
                }
            }
        }
    }
}

#[test]
fn missing_buckets_give_no_lines() {
    let input = fixtures::get("philadelphia-renters-4").unwrap();
    assert!(requirements(&input, &[]).is_empty());
    let only = [common::days(BucketId::WaterOut, 3.0)];
    let lines = requirements(&input, &only);
    assert!(!lines.is_empty());
    assert!(lines.iter().all(|l| l.bucket == BucketId::WaterOut));
    // A duration bucket with a readiness-shaped target is skipped rather than guessed.
    let wrong = [common::readiness(BucketId::WaterOut, 0.5)];
    assert!(requirements(&input, &wrong).is_empty());
}

#[test]
fn hot_counties_store_more_drinking_water() {
    let input = fixtures::get("phoenix-apartment-cpap-1").unwrap();
    let targets = common::generic();
    let temperate = requirements(&input, &targets);
    let hot = requirements_with(
        &input,
        &targets,
        &SupplyContext {
            days_at_or_above_95f: Some(110.0),
            latitude: None,
            ..SupplyContext::default()
        },
    );
    assert_eq!(line(&temperate, "water_out.water_gallons").quantity, 3.0);
    // 1 person × 1.75 gal × 3 days (drinking share doubled; research §1.2)
    assert_eq!(line(&hot, "water_out.water_gallons").quantity, 5.3);
    assert!(
        line(&hot, "water_out.water_gallons")
            .plain
            .contains("95 °F")
    );
    // Philadelphia's 3.3 days a year at 95 °F is not a hot climate.
    let philly = fixtures::get("philadelphia-renters-4").unwrap();
    let cool = requirements_with(
        &philly,
        &common::philadelphia(),
        &SupplyContext {
            days_at_or_above_95f: Some(3.3),
            latitude: None,
            ..SupplyContext::default()
        },
    );
    assert_eq!(line(&cool, "water_out.water_gallons").quantity, 12.9);
    // The CPAP needs backup power; it is a life-safety line.
    let sized = sized_requirements(&input, &targets, &SupplyContext::default());
    let cpap = sized
        .iter()
        .find(|s| s.line.id == "power.medical_device_wh")
        .unwrap();
    assert!(cpap.life_safety);
    assert_eq!(cpap.line.quantity, 510.0);
}

#[test]
fn latitude_adds_the_winter_solar_figures() {
    let input = fixtures::get("philadelphia-renters-4").unwrap();
    // A 7-day power target brings the optional solar panel line (content rule: 7 days or more).
    let mut targets = common::philadelphia();
    targets[0] = common::days(BucketId::Power, 7.0);
    let with = requirements_with(
        &input,
        &targets,
        &SupplyContext {
            latitude: Some(39.95),
            ..SupplyContext::default()
        },
    );
    let solar = line(&with, "power.solar_panel_units.optional");
    assert_eq!(solar.quantity, 1.0);
    assert!(solar.plain.contains("261 Wh"), "{}", solar.plain);
    let without = requirements(&input, &targets);
    assert!(
        line(&without, "power.solar_panel_units.optional")
            .plain
            .contains("northern winter")
    );
    // A 3-day target offers no panel.
    assert!(!has(
        &requirements(&input, &common::philadelphia()),
        "power.solar_panel_units.optional"
    ));
}

#[test]
fn per_day_rates_reproduce_the_quantities() {
    for (name, input, targets) in common::all() {
        for s in sized_requirements(&input, &targets, &SupplyContext::default()) {
            let (Some(days), Some(per_day)) = (s.days, s.per_day) else {
                continue;
            };
            if s.line.rule == "generator_fuel_gallons" || s.line.rule == "cash_reserve_usd" {
                continue; // capped or clamped
            }
            let raw = per_day * days;
            let q = f64::from(s.line.quantity);
            let tol = match s.line.unit.as_str() {
                "kcal" => 50.0,
                "Wh" | "watt" | "gram" => 5.0,
                "usd" | "oz" => 0.5,
                "gallon" | "litre" | "lb" | "day" | "person_day" | "pet_day" | "month" => 0.051,
                _ => 1.0, // whole things are rounded up
            };
            assert!(
                (q - raw).abs() <= tol
                    || (s.line.id.starts_with("water_out.water_gallons") && q <= raw + tol),
                "{name}: {} quantity {q} vs {raw}",
                s.line.id
            );
            assert_eq!(
                s.quantity_for_days(days).map(|x| x.to_bits()),
                Some(raw.to_bits())
            );
        }
    }
}

#[test]
fn life_safety_lines_are_marked() {
    let input = fixtures::get("sugar-land-ev-household-3").unwrap();
    let sized = sized_requirements(&input, &common::generic(), &SupplyContext::default());
    for id in [
        "water_out.water_gallons",
        "medication.medication_days",
        "medication.rx_cold_storage",
        "supplies.infant_formula_oz",
    ] {
        let s = sized
            .iter()
            .find(|s| s.line.id == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert!(s.life_safety, "{id}");
    }
    assert!(
        !sized
            .iter()
            .find(|s| s.line.id == "comms.noaa_radio")
            .unwrap()
            .life_safety
    );
}

#[test]
fn readiness_below_the_threshold_is_enough_at_free_actions() {
    let low = common::readiness(BucketId::GetHome, 0.01);
    assert_eq!(rr_supply::tier_enough(&low), TierId::Now);
    let high = common::readiness(BucketId::GetHome, 0.05);
    assert_eq!(rr_supply::tier_enough(&high), TierId::H72);
    assert_eq!(rr_supply::tier_enough(&common::months(4.0)), TierId::M3);
    assert_eq!(
        rr_supply::tier_enough(&common::days(BucketId::WaterOut, 50.0)),
        TierId::M3
    );
}

#[test]
fn items_are_sized_by_their_rule() {
    let input = fixtures::get("philadelphia-renters-4").unwrap();
    let targets = common::philadelphia();
    let sizer = rr_supply::ItemSizer::new(&input, &targets, &SupplyContext::default());
    let q = |r| sizer.quantity(r).map(|g| (g.quantity, g.per));
    assert_eq!(q("once"), Some((1.0, Per::Household)));
    assert_eq!(q("per_person"), Some((4.0, Per::Person)));
    assert_eq!(q("per_vehicle"), Some((1.0, Per::Household)));
    assert_eq!(q("per_pet"), Some((1.0, Per::Pet)));
    assert_eq!(q("per_commuter"), Some((2.0, Per::Commuter)));
    assert_eq!(q("once_if_ev"), Some((0.0, Per::Household)));
    // Line rules give the line's quantity.
    assert_eq!(q("water_gallons").unwrap().0, 12.9);
    assert_eq!(q("food_kcal").unwrap().0, 82_000.0);
    assert_eq!(q("medication_days").unwrap().0, 14.0);
    assert_eq!(q("water_reused_bottles").unwrap().0, 6.0);
    // Per-commuter lines add up: 2 L + 0.6 L.
    assert_eq!(q("get_home_water").unwrap().0, 2.6);
    assert_eq!(q("get_home_bag").unwrap().0, 2.0);
    // No filter for a short no-water target on city water; no line means 0.
    assert_eq!(q("water_treatment_capacity").unwrap().0, 0.0);
    assert_eq!(q("infant_formula_oz").unwrap().0, 0.0);
    assert_eq!(q("no_such_rule"), None);
    // Coos Bay: a well and a 50-day target need one filter.
    let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
    let c = rr_supply::item_quantity(
        "water_treatment_capacity",
        &coos,
        &common::coos_bay(),
        &SupplyContext::default(),
    );
    assert_eq!(c.unwrap().quantity, 1.0);
    // Every rule in the table answers for every fixture.
    for (name, input, targets) in common::all() {
        let sizer = rr_supply::ItemSizer::new(&input, &targets, &SupplyContext::default());
        for r in rule_ids() {
            let got = sizer.quantity(r).unwrap_or_else(|| panic!("{name}: {r}"));
            assert!(
                got.quantity.is_finite() && got.quantity >= 0.0,
                "{name}: {r}"
            );
        }
    }
}

#[test]
fn directional_cover_has_its_own_item_class() {
    // Heat cover is never cold cover, and stored water is not treatment (budget coverage parts).
    for (name, input, targets) in common::all() {
        for l in requirements(&input, &targets) {
            assert_ne!(l.item_class, "thermal", "{name}: {}", l.id);
            if l.bucket == BucketId::Thermal {
                let heat =
                    ["battery_fan", "cooling_towel", "cooling_plan"].contains(&l.rule.as_str());
                let want = if heat { "thermal_heat" } else { "thermal_cold" };
                assert_eq!(l.item_class, want, "{name}: {}", l.id);
            }
            match l.rule.as_str() {
                "water_gallons" | "water_reused_bottles" => {
                    assert_eq!(l.item_class, "water_stored", "{}", l.id)
                }
                "water_treatment_capacity" | "bleach_bottles" | "boil_fuel" => {
                    assert_eq!(l.item_class, "water_treatment_capacity", "{}", l.id)
                }
                _ => {}
            }
        }
    }
}
