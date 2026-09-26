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
    // The walk home's water and snacks are staged from home: alternatives of each person's bag.
    assert_eq!(
        line(&lines, "get_home.get_home_bag.alt.staged_water.person_1").quantity,
        2.0
    );
    assert_eq!(
        line(&lines, "get_home.get_home_bag.alt.staged_water.person_2").quantity,
        0.6
    );
    assert_eq!(
        line(&lines, "get_home.get_home_bag.alt.staged_food.person_1").quantity,
        400.0
    );
    assert!(!has(&lines, "get_home.get_home_water.person_1"));
    assert_eq!(line(&lines, "evacuate.go_bag").quantity, 4.0);
    assert_eq!(line(&lines, "evacuate.pet_carrier").quantity, 1.0);
    // The go-bags' and the pet kit's water and food come out of the household's supplies.
    for id in [
        "evacuate.go_bag.alt.staged_water",
        "evacuate.go_bag.alt.staged_food",
        "evacuate.pet_carrier.alt.staged_water",
        "evacuate.pet_carrier.alt.staged_food",
    ] {
        assert_eq!(
            LineKind::of(&line(&lines, id).id),
            LineKind::Alternative,
            "{id}"
        );
    }
    assert_eq!(
        line(&lines, "evacuate.go_bag.alt.staged_water").quantity,
        12.0
    );
    for old in [
        "evacuate.go_bag_water",
        "evacuate.go_bag_food",
        "evacuate.pet_go_water",
        "evacuate.pet_go_food",
    ] {
        assert!(!has(&lines, old), "{old} is staged now");
    }
    // One bottle of bleach, whatever the target.
    assert_eq!(line(&lines, "water_boil.bleach_bottles").quantity, 1.0);
    // Reused bottles cite the cap and the rotation, not the dog's water.
    let reused = line(&lines, "water_out.water_gallons.alt.reused_bottles");
    assert!(!reused.citations.iter().any(|c| c == "petmd_dog_water"));
    // One extinguisher per floor of a two-floor rowhouse; no ladder at street level.
    assert_eq!(line(&lines, "fire.extinguisher_count").quantity, 2.0);
    assert!(!has(&lines, "fire.escape_ladder_count"));
    // Thermal is driven by both heat and cold. Blankets and layers for everyone; a sleeping bag
    // for the senior only (1.7 days of cold).
    assert!(has(&lines, "thermal.battery_fan") && has(&lines, "thermal.blankets"));
    assert_eq!(line(&lines, "thermal.blankets").quantity, 4.0);
    assert_eq!(line(&lines, "thermal.warm_layers").quantity, 4.0);
    assert_eq!(
        line(&lines, "thermal.sleeping_bag_or_blanket").quantity,
        1.0
    );
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
    // Fifty days without well water still means one bottle of bleach (it treats thousands of
    // gallons), with the no-water lines because there is no boil-water target.
    let bleach = line(&lines, "water_out.bleach_bottles");
    assert_eq!(bleach.quantity, 1.0);
    assert!(
        !bleach.citations.iter().any(|c| c == common::TARGET_SOURCE),
        "the target's days do not size it"
    );
    // The household has an extinguisher; a detached house at street level gets no ladder.
    assert!(!has(&lines, "fire.extinguisher_count"));
    assert!(!has(&lines, "fire.escape_ladder_count"));

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
        "blanket",
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
        "pair",
        "installed kit",
        "can",
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
fn livestock_water_stops_at_the_stored_days() {
    // Hays farm, 12 large animals, 60 days without water: 12 × 25 L × min(60, 14) days = 4,200 L.
    let input = fixtures::get("hays-kansas-farm-5").unwrap();
    assert_eq!(input.pets.large_animals, 12);
    let lines = requirements(&input, &[common::days(BucketId::WaterOut, 60.0)]);
    let l = line(&lines, "water_out.livestock_water");
    assert_eq!(l.quantity, 1109.5, "was 4,755 gallons uncapped");
    assert!(l.plain.contains("well pump") && l.plain.contains("haul water"));
    assert!(l.citations.iter().any(|c| c == common::TARGET_SOURCE));
    // Under the cap nothing changes: 12 × 25 L × 3 days = 900 L.
    let short = requirements(&input, &[common::days(BucketId::WaterOut, 3.0)]);
    assert_eq!(line(&short, "water_out.livestock_water").quantity, 237.8);
    // No power target, so no generator is offered and none is planned: no pump power.
    assert!(!has(&lines, "power.generator_units"));
    assert!(!has(&lines, "water_out.livestock_water.alt.stock_tank"));
}

/// Horses or livestock on a well (round-2 review P-02). Pump power exists only when the household
/// owns a generator and the interlock or transfer switch that connects it: then the animals'
/// stored water bridges 3 days, with two weeks in stock tanks as the alternative. Until then they
/// store 14 days, like people. A pump-rated generator is a need only when a power cut can outlast
/// that water, and it comes with its connection (life-safety) and fuel cans.
#[test]
fn livestock_on_a_well_store_two_weeks_until_pump_power_exists() {
    let input = fixtures::get("hays-kansas-farm-5").unwrap();
    let targets = |power: f32| {
        vec![
            common::days(BucketId::Power, power),
            common::driven_by(
                common::days(BucketId::WaterOut, 60.0),
                &[(rr_types::HazardId::Drought, 1.0)],
            ),
        ]
    };
    // Hays: a 3-day power target and 14 days of stored animal water. The generator would add
    // nothing, so it stays optional; the drought is met by hauling water.
    let lines = requirements(&input, &targets(3.0));
    let l = line(&lines, "water_out.livestock_water");
    assert_eq!(l.quantity, 1109.5, "14 days, not the 3-day bridge");
    assert!(l.plain.contains("haul water in"), "{}", l.plain);
    assert!(
        l.plain.contains("interlock or transfer switch"),
        "{}",
        l.plain
    );
    assert!(!has(&lines, "water_out.livestock_water.alt.stock_tank"));
    assert!(!has(&lines, "power.generator_units"));
    let optional = line(&lines, "power.generator_units.optional");
    assert!(
        optional.plain.contains("starting watts") && optional.plain.contains("wall outlet"),
        "{}",
        optional.plain
    );
    assert!(!has(&lines, "power.generator_connection_units"));
    assert!(!has(&lines, "power.fuel_cans"));
    assert!(!has(&lines, "power.generator_fuel_gallons.note"));

    // A power target longer than the stored water: a pump-rated generator is a need, with an
    // electrician-installed interlock (life-safety), fuel cans, and the fuel as a note so the
    // plan never buys fuel before the generator. The animals still store 14 days until it exists.
    let sized = sized_requirements(&input, &targets(30.0), &SupplyContext::default());
    let get = |id: &str| {
        sized
            .iter()
            .find(|l| l.line.id == id)
            .unwrap_or_else(|| panic!("no {id}"))
    };
    let generator = get("power.generator_units");
    assert_eq!(generator.kind, LineKind::Need);
    assert!(
        generator.line.plain.contains("240-volt outlet")
            && generator.line.plain.contains("14 days of stored water"),
        "{}",
        generator.line.plain
    );
    let connection = get("power.generator_connection_units");
    assert_eq!(connection.kind, LineKind::Need);
    assert!(connection.life_safety);
    assert_eq!(connection.line.unit, "installed kit");
    assert!(
        connection
            .line
            .citations
            .iter()
            .any(|c| c == "osha_portable_generators")
    );
    let cans = get("power.fuel_cans");
    assert_eq!(cans.kind, LineKind::Need);
    // 30 days × 2.8 gal = 84 gal, capped at the 25 gal a home may store: 5 cans.
    assert_eq!(get("power.generator_fuel_gallons.note").quantity, 25.0);
    assert_eq!(cans.quantity, 5.0);
    assert_eq!(cans.line.item_class, "generator");
    assert_eq!(get("water_out.livestock_water").quantity, 1109.5);

    // A generator of their own, but no interlock listed: pump power does not exist yet, so the
    // animals store 14 days; the connection is a need, and the fuel and its cans are needs.
    let mut owns = input.clone();
    owns.housing.backup_power = rr_types::BackupPower::Generator;
    let sized = sized_requirements(&owns, &targets(3.0), &SupplyContext::default());
    let get = |id: &str| sized.iter().find(|l| l.line.id == id);
    assert_eq!(get("water_out.livestock_water").unwrap().quantity, 1109.5);
    assert!(get("power.generator_units").is_none());
    assert_eq!(
        get("power.generator_connection_units").map(|l| l.kind),
        Some(LineKind::Need)
    );
    assert_eq!(
        get("power.generator_fuel_gallons").map(|l| l.kind),
        Some(LineKind::Need)
    );
    let cans = get("power.fuel_cans").unwrap();
    assert_eq!(
        (cans.quantity, cans.line.item_class.as_str()),
        (2.0, "generator_fuel")
    );

    // With the interlock listed as owned, pump power exists: 3 days stored, the stock tank the
    // alternative.
    owns.existing.push(rr_types::Owned {
        item_id: rr_types::ItemId::from(rr_supply::TRANSFER_INTERLOCK_ITEM),
        qty: 1.0,
        paid_usd: None,
    });
    let lines = requirements(&owns, &targets(3.0));
    let l = line(&lines, "water_out.livestock_water");
    assert_eq!(l.quantity, 237.8);
    assert!(
        l.plain.contains("until the generator runs the well pump"),
        "{}",
        l.plain
    );
    assert!(l.plain.contains("haul water in"), "a drought: {}", l.plain);
    let alt = line(&lines, "water_out.livestock_water.alt.stock_tank");
    assert_eq!(
        (alt.quantity, alt.rule.as_str()),
        (1109.5, "livestock_water_stored")
    );
    assert!(has(&lines, "power.generator_connection_units"));

    // Town water: no pump to power, no connection.
    let mut town = input.clone();
    town.housing.water = rr_types::WaterSource::Municipal;
    let lines = requirements(&town, &targets(30.0));
    assert_eq!(line(&lines, "water_out.livestock_water").quantity, 1109.5);
    assert!(!has(&lines, "power.generator_units"));
    assert!(!has(&lines, "power.generator_connection_units"));

    // No animals: the generator stays optional and needs no connection line.
    let mut none = input.clone();
    none.pets.large_animals = 0;
    let lines = requirements(&none, &targets(30.0));
    assert!(has(&lines, "power.generator_units.optional"));
    assert!(!has(&lines, "power.generator_units"));
    assert!(!has(&lines, "power.generator_connection_units"));

    // Renters ask the landlord: no connection line.
    let mut renter = owns.clone();
    renter.housing.tenure = rr_types::Tenure::Rent;
    assert!(!has(
        &requirements(&renter, &targets(3.0)),
        "power.generator_connection_units"
    ));
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
        // Insulin and a 3-day power target with no backup power: the station is a need.
        "power.power_station_units",
        // The generic targets give a medical emergency a 0.9 ten-year chance.
        "medical_emergency.bleeding_control_kit",
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
    let station = sized
        .iter()
        .find(|s| s.line.id == "power.power_station_units")
        .unwrap();
    assert_eq!(station.kind, LineKind::Need);
    assert_eq!(
        station.line.item_class,
        rr_supply::COLD_MEDICINE_POWER_CLASS
    );
    // A town household with a medical emergency unlikely and no rural setting: the kit is an
    // ordinary step.
    let mut low = common::generic();
    for b in &mut low {
        if b.id == BucketId::MedicalEmergency {
            *b = common::readiness(BucketId::MedicalEmergency, 0.3);
        }
    }
    let philly = fixtures::get("philadelphia-renters-4").unwrap();
    let sized = sized_requirements(&philly, &low, &SupplyContext::default());
    let kit = sized
        .iter()
        .find(|s| s.line.id == "medical_emergency.bleeding_control_kit")
        .unwrap();
    assert!(!kit.life_safety);
    // Rural homes: life-safety whatever the chance, and the line says why.
    let hays = fixtures::get("hays-kansas-farm-5").unwrap();
    let sized = sized_requirements(&hays, &low, &SupplyContext::default());
    let kit = sized
        .iter()
        .find(|s| s.line.id == "medical_emergency.bleeding_control_kit")
        .unwrap();
    assert!(kit.life_safety);
    assert!(kit.line.plain.contains("rural homes"), "{}", kit.line.plain);
    assert!(
        kit.line
            .citations
            .iter()
            .any(|c| c == "mell_2017_ems_response")
    );
}

/// Round-2 review RR-P11: the plan's 40 °F and 90 °F rules need thermometers, so they are needs
/// wherever a power or heat target exists.
#[test]
fn thermometers_are_needs_with_a_power_or_heat_target() {
    let input = fixtures::get("philadelphia-renters-4").unwrap();
    let lines = requirements(&input, &common::philadelphia());
    let fridge = line(&lines, "power.fridge_thermometers");
    assert_eq!((fridge.quantity, fridge.unit.as_str()), (1.0, "pair"));
    assert!(fridge.plain.contains("40 °F"), "{}", fridge.plain);
    let room = line(&lines, "thermal.room_thermometer");
    assert_eq!(room.item_class, "thermal_heat");
    assert!(room.plain.contains("below 90 °F"), "{}", room.plain);
    // No power target, no fridge thermometer; cold alone, no room thermometer.
    let cold_only = requirements(
        &input,
        &[common::driven_by(
            common::days(BucketId::Thermal, 3.0),
            &[(rr_types::HazardId::ColdWave, 1.0)],
        )],
    );
    assert!(!has(&cold_only, "power.fridge_thermometers"));
    assert!(!has(&cold_only, "thermal.room_thermometer"));
}

/// Round-2 review RR-P16: renters ask the landlord for smoke alarms (a note, never a purchase the
/// plan saves toward), owners ask the fire department or the Red Cross first.
#[test]
fn renters_ask_the_landlord_for_smoke_alarms() {
    let mut input = fixtures::get("philadelphia-renters-4").unwrap();
    input.housing.alarms.smoke = false;
    let sized = sized_requirements(&input, &common::philadelphia(), &SupplyContext::default());
    let kind = |id: &str| sized.iter().find(|l| l.line.id == id).map(|l| l.kind);
    assert_eq!(kind("fire.smoke_alarm_count.note"), Some(LineKind::Note));
    assert_eq!(kind("fire.smoke_alarm_count"), None);
    input.housing.tenure = rr_types::Tenure::Own;
    let sized = sized_requirements(&input, &common::philadelphia(), &SupplyContext::default());
    let alarms = sized
        .iter()
        .find(|l| l.line.id == "fire.smoke_alarm_count")
        .unwrap();
    assert_eq!(alarms.kind, LineKind::Need);
    assert!(alarms.life_safety);
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
    // Per-commuter lines add up: 2 L + 0.6 L (staged alternatives size an item the same way).
    assert_eq!(q("get_home_water").unwrap().0, 2.6);
    assert_eq!(q("get_home_food").unwrap().0, 500.0);
    assert_eq!(q("get_home_bag").unwrap().0, 2.0);
    assert_eq!(q("bleach_bottles").unwrap().0, 1.0);
    assert_eq!(q("extinguisher_count").unwrap().0, 2.0);
    assert_eq!(q("escape_ladder_count").unwrap().0, 0.0);
    assert_eq!(q("blankets").unwrap().0, 4.0);
    assert_eq!(q("sleeping_bag_or_blanket").unwrap().0, 1.0);
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
                let heat = [
                    "battery_fan",
                    "cooling_towel",
                    "cooling_plan",
                    "room_thermometer",
                ]
                .contains(&l.rule.as_str());
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
