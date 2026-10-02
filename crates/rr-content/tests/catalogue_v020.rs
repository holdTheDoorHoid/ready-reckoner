//! The v0.2.0 catalogue (supply workstream): the round-2 practitioner items, clean air, the
//! long-horizon section, decisions, prerequisites (`requires`), readiness shares, season anchors
//! and test intervals (contract v2's item fields; docs/QUANTITY_RULES.md, "Round 2, v0.2.0").

use std::collections::{BTreeMap, BTreeSet};

use rr_types::{BucketId, BucketKind, HazardId, Item, Season, TierId};

fn content() -> &'static rr_content::Content {
    rr_content::content()
}

fn item(id: &str) -> &'static Item {
    content()
        .item(id)
        .unwrap_or_else(|| panic!("the catalogue has no item `{id}`"))
}

fn cites(i: &Item, id: &str) -> bool {
    i.citations.iter().any(|c| c.as_str() == id)
}

#[test]
fn the_practitioner_items_exist_with_their_rules() {
    // practitioner-review.md, "New items proposed". N-10 (a gas range the household owns) is the
    // housing input `Housing.cooking`, which the cooking line reads, not something to buy.
    for (n, id, rule) in [
        (
            "N-01",
            "power_transfer_interlock",
            "generator_connection_units",
        ),
        ("N-02", "power_fuel_cans", "fuel_cans"),
        ("N-03", "water_heater_strap_kit", "once_if_house"),
        ("N-04", "water_rain_barrel", "rain_catchment_units"),
        ("N-05", "water_carriers", "water_carriers"),
        ("N-06", "water_livestock_haul_tank", "livestock_haul_tank"),
        ("N-07", "water_well_hand_pump", "well_hand_pump"),
        ("N-08", "food_propane_cylinders", "propane_cylinders"),
        ("N-09", "food_grill_spare_tank", "grill_propane_tank"),
        ("N-11", "food_infant_formula_rtf", "infant_formula_rtf"),
        ("N-12", "med_wound_splint_addon", "wound_care_addon"),
        ("N-13", "san_nitrile_gloves", "once"),
        ("N-14", "power_car_charger_inverter", "recharge_capability"),
        ("N-15", "med_rx_12v_fridge", "rx_fridge_units"),
        ("N-16", "san_household_ops_kit", "household_ops_kits"),
        ("N-17", "special_kids_activity_kit", "once_if_children"),
        ("N-18", "med_cleanup_ppe", "per_person_13_plus"),
        ("N-19", "fire_home_tarp_kit", "once_if_house"),
        ("N-20", "med_insect_repellent", "once"),
        ("N-21", "evac_forecast_48h_checklist", "once"),
        ("N-22", "power_generator_upkeep", "once_if_generator"),
    ] {
        assert_eq!(item(id).quantity_rule, rule, "{n} {id}");
    }
    assert!(content().item("gas_stove").is_none());
    // The two free parents are free actions in tier now.
    for id in ["evac_forecast_48h_checklist", "power_generator_upkeep"] {
        let i = item(id);
        assert!(
            i.free && i.tier == TierId::Now && i.price_band_usd.high == 0.0,
            "{id}"
        );
    }
    // The 48-hour list follows the Red Cross checklist: tub water for flushing, charging, fuel,
    // cash, refills, the fridge at its coldest, loose things tied down, the trigger, neighbours.
    let list = item("evac_forecast_48h_checklist").look_for.join(" ");
    for step in [
        "bathtub",
        "Charge",
        "tank",
        "cash",
        "prescriptions",
        "coldest",
        "tie down",
        "trigger",
        "neighbors",
    ] {
        assert!(list.contains(step), "the 48-hour list lacks `{step}`");
    }
    assert!(cites(
        item("evac_forecast_48h_checklist"),
        "redcross_hurricane_checklist"
    ));
    // Items the practitioner listed for revision.
    let stove = item("food_camp_stove");
    assert_eq!(stove.quantity_rule, "cooking_capability");
    assert!(stove.spec.contains("propane") && cites(stove, "nist_butane"));
    assert_eq!(
        item("water_personal_filter").quantity_rule,
        "get_home_filter"
    );
    assert_eq!(item("fire_utility_wrench").quantity_rule, "once_if_house");
    assert_eq!(
        item("security_motion_light").quantity_rule,
        "once_if_house_ground_floor"
    );
    assert_eq!(
        item("thermal_warm_room_plan").quantity_rule,
        "warm_room_plan"
    );
    assert_eq!(item("thermal_cool_room_plan").quantity_rule, "cooling_plan");
    assert!(
        item("thermal_warm_room_plan")
            .look_for
            .join(" ")
            .contains("2-1-1")
    );
    assert!(item("fire_extinguisher").life_safety);
    assert_eq!(item("med_bleeding_control_kit").tier, TierId::H72);
    let bleach = item("water_bleach_unscented");
    assert!(bleach.avoid.iter().any(|a| a.contains("Splashless")));
    // CDC's emergency page: chlorine dioxide kills Cryptosporidium when the directions are
    // followed, and boiling works better against parasites.
    let cd = item("water_chlorine_dioxide");
    assert!(cd.spec.contains("follow the directions exactly") && cd.spec.contains("Boiling"));
}

#[test]
fn clean_air_items_sit_in_the_clean_air_bucket() {
    for id in [
        "fire_clean_air_room",
        "fire_clean_room_plan",
        "med_n95_respirators",
    ] {
        let i = item(id);
        assert_eq!(i.buckets[0], BucketId::CleanAir, "{id}");
        assert!(
            i.hazard_extras.is_empty(),
            "{id}: the clean-air target gates it"
        );
        assert!(
            !i.buckets.contains(&BucketId::MedicalEmergency),
            "{id} (review P-05)"
        );
    }
    assert_eq!(
        item("fire_clean_air_room").quantity_rule,
        "air_cleaner_units"
    );
    // One item for the clean room's air, so the plan never buys both: a HEPA air cleaner sized
    // by EPA's table (CADR at least two-thirds of the floor area, about 130 for 200 square feet),
    // or the cheaper box fan with a MERV 13 filter (EPA: cost-effective), whose price is the low
    // end of the band. Readiness credit is per item, so two items would both be bought.
    let air = item("fire_clean_air_room");
    assert!(air.spec.contains("about 130") && air.spec.contains("MERV 13"));
    assert!(cites(air, "epa_diy_air_cleaners") && cites(air, "epa_air_cleaner_guide"));
    assert!((f64::from(air.price_band_usd.low) - 34.97).abs() < 0.006);
    assert!(
        content()
            .items
            .iter()
            .all(|i| i.quantity_rule != "diy_filter_box")
    );
    let plan = item("fire_clean_room_plan");
    assert!(plan.free && plan.quantity_rule == "clean_room_plan");
    let n95 = item("med_n95_respirators");
    assert!(n95.spec.contains("aged 13 and over") && cites(n95, "epa_children_wildfire_smoke"));
    // The shelter-in-place kit seals the clean room for a chemical release.
    assert_eq!(
        item("evac_shelter_in_place_kit").buckets[0],
        BucketId::CleanAir
    );
}

#[test]
fn every_readiness_item_carries_a_share() {
    // docs/QUANTITY_RULES.md, "Readiness share": 1.0 for the capability itself, 0.05 to 0.2 for
    // accessories (review P-05).
    for i in &content().items {
        let readiness = i.buckets.iter().any(|b| b.kind() == BucketKind::Readiness);
        match i.readiness_share {
            Some(s) => {
                assert!(
                    (0.0..=1.0).contains(&s),
                    "{}: share {s} outside 0 to 1",
                    i.id
                );
                assert!(readiness, "{} has a share but no readiness bucket", i.id);
            }
            None => assert!(
                !readiness,
                "{} lists a readiness bucket but no readiness_share (docs/QUANTITY_RULES.md, \"Readiness share\")",
                i.id
            ),
        }
    }
    for id in [
        "evac_go_bag",
        "gethome_bag",
        "med_first_aid_kit",
        "fire_test_alarms",
        "fire_smoke_alarm",
        "comms_wea_alerts_on",
    ] {
        assert_eq!(
            item(id).readiness_share,
            Some(1.0),
            "{id} is the capability"
        );
    }
    for id in [
        "evac_whistle",
        "thermal_emergency_blankets",
        "water_chlorine_dioxide",
        "water_personal_filter",
    ] {
        let s = item(id).readiness_share.unwrap();
        assert!((0.05..=0.2).contains(&s), "{id} is an accessory: {s}");
    }
    assert_eq!(
        item("med_antibiotics_clinician_card").readiness_share,
        Some(0.0),
        "not medical-emergency care (review P-05)"
    );
    // A headlamp now outranks a whistle on the leaving-home checklist.
    assert!(item("power_headlamp").readiness_share > item("evac_whistle").readiness_share);
}

#[test]
fn accessories_require_their_device() {
    let ids: BTreeSet<&str> = content().items.iter().map(|i| i.id.as_str()).collect();
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for i in &content().items {
        for r in &i.requires {
            assert!(ids.contains(r.as_str()), "{} requires unknown `{r}`", i.id);
            assert_ne!(r.as_str(), i.id.as_str(), "{} requires itself", i.id);
            assert!(!item(r.as_str()).decision, "{} requires a decision", i.id);
        }
        graph.insert(
            i.id.as_str(),
            i.requires.iter().map(|r| r.as_str()).collect(),
        );
    }
    // No cycles: every chain of requirements ends.
    fn depth<'a>(id: &'a str, g: &BTreeMap<&'a str, Vec<&'a str>>, seen: &mut Vec<&'a str>) {
        assert!(
            !seen.contains(&id),
            "requires cycle through {seen:?} -> {id}"
        );
        seen.push(id);
        for r in g.get(id).map(Vec::as_slice).unwrap_or_default() {
            depth(r, g, seen);
        }
        seen.pop();
    }
    for id in graph.keys() {
        depth(id, &graph, &mut Vec::new());
    }
    // Review P-12's list; `requires` lists alternatives (any one of them will do).
    for (id, needs) in [
        (
            "power_batteries",
            &["power_headlamp", "power_lantern", "comms_noaa_radio"][..],
        ),
        ("san_toilet_bags", &["san_twin_bucket_toilet"][..]),
        ("san_baby_wipes", &["san_diapers"][..]),
        ("power_generator_fuel", &["power_fuel_cans"][..]),
        ("power_fuel_cans", &["power_generator"][..]),
        ("food_cooking_fuel", &["food_camp_stove"][..]),
        ("food_propane_cylinders", &["food_camp_stove"][..]),
    ] {
        let got: Vec<&str> = item(id).requires.iter().map(|r| r.as_str()).collect();
        assert_eq!(got, needs, "{id}");
    }
}

#[test]
fn decisions_cost_the_supplies_budget_nothing() {
    let decisions: Vec<&Item> = content().items.iter().filter(|i| i.decision).collect();
    for i in &decisions {
        assert!(
            i.free,
            "{} is a decision, so it is free in the budget",
            i.id
        );
        assert_eq!(i.tier, TierId::Now, "{}", i.id);
        assert_eq!(i.price_band_usd.high, 0.0, "{}", i.id);
        assert_eq!(i.category, "documents_money", "{}", i.id);
        assert!(
            i.citations
                .iter()
                .all(|c| content().citation(c.as_str()).is_some())
        );
    }
    // Every insurance line has its decision (review RR-P09).
    for rule in [
        "insurance_home_or_renters",
        "insurance_flood",
        "insurance_earthquake",
        "insurance_wind_deductible",
        "insurance_sewer_backup",
        "insurance_condo_unit",
        "insurance_life_disability",
    ] {
        assert!(
            decisions.iter().any(|i| i.quantity_rule == rule),
            "no decision item for `{rule}`"
        );
    }
    // Home mitigation, gated by hazard and housing (review RR-P10), each with a grant or discount
    // pointer.
    for (id, rule, hazard, pointer) in [
        (
            "decide_fortified_roof",
            "once_if_owned_house",
            HazardId::Hurricane,
            "ibhs_fortified_incentives",
        ),
        (
            "decide_safe_room",
            "once_if_owned_house",
            HazardId::Tornado,
            "fema_safe_room_funding",
        ),
        (
            "decide_backflow_sump",
            "once_if_owned_basement",
            HazardId::RiverineFlooding,
            "fema_flood_smart_protect",
        ),
        (
            "decide_seismic_retrofit",
            "once_if_owned_detached",
            HazardId::Earthquake,
            "crmp_earthquake_brace_bolt",
        ),
        (
            "decide_wildfire_hardening",
            "once_if_owned_house",
            HazardId::Wildfire,
            "cdi_safer_from_wildfires",
        ),
    ] {
        let i = item(id);
        assert!(i.decision, "{id}");
        assert_eq!(i.quantity_rule, rule, "{id}");
        assert!(i.hazard_extras.contains(&hazard), "{id}");
        assert!(cites(i, pointer), "{id} cites {pointer}");
    }
    // FloodSmart: 29 % of claims from outside high-risk areas, about 3 in 10.
    assert!(
        item("decide_flood_insurance")
            .spec
            .contains("about 3 in 10")
    );
    // SSA: about 1 in 4 twenty-year-olds become disabled before retirement age.
    assert!(item("decide_life_disability").spec.contains("about 1 in 4"));
    // The Deviant Ollam lessons: ID for every person, with the federal fee schedule (22 CFR 22.1,
    // 2025: book $50 + $80 surcharge = $130, card $30, minor book $20 + $80 = $100, minor card
    // $15, execution fee $35).
    let id = item("decide_id_for_every_person");
    for fee in ["$165", "$65", "$135", "$50", "$35"] {
        assert!(id.spec.contains(fee), "ID decision lacks {fee}");
    }
    assert!(cites(id, "cfr_22_1_consular_fees"));
    assert!(id.price_band_usd.high == 0.0 && id.decision);
}

#[test]
fn long_horizon_items_are_flagged() {
    // brief-supply.md item 4: rain catchment, carriers, the hand pump, hauling for animals,
    // household operations for months, a morale kit, staples and the pointers.
    for id in [
        "water_rain_barrel",
        "water_carriers",
        "water_well_hand_pump",
        "water_livestock_haul_tank",
        "san_household_ops_kit",
        "special_kids_activity_kit",
        "food_bulk_staples",
        "food_going_further",
    ] {
        assert!(item(id).long_horizon, "{id}");
    }
    let pointers = item("food_going_further");
    assert!(pointers.free && pointers.quantity_rule == "once_if_long_horizon");
    // The rain barrel cites the rainfall normals and the state rules.
    let barrel = item("water_rain_barrel");
    assert!(cites(barrel, "noaa_nclimdiv") && cites(barrel, "ncsl_rainwater"));
    // No everyday item is in the long-horizon section.
    for id in [
        "water_stored_bottled",
        "food_pantry_rotation",
        "san_toilet_bags",
    ] {
        assert!(!item(id).long_horizon, "{id}");
    }
}

#[test]
fn the_items_to_try_have_test_intervals_and_seasons() {
    // The Deviant Ollam lessons: test the jump pack, the generator, the key safe and the lights.
    for (id, months) in [
        ("power_headlamp", 6),
        ("power_lantern", 6),
        ("power_generator", 3),
        ("gethome_jump_pack", 3),
        ("security_key_safe", 6),
    ] {
        assert_eq!(item(id).test_interval_months, Some(months), "{id}");
        assert!(
            cites(item(id), "rr_expert_prior"),
            "{id}: the interval is an estimate"
        );
    }
    let tested = content()
        .items
        .iter()
        .filter(|i| i.test_interval_months.is_some())
        .count();
    assert_eq!(tested, 5, "only the items worth trying");
    for (id, season) in [
        ("thermal_battery_fan", Season::Summer),
        ("fire_clean_air_room", Season::Summer),
        ("power_generator", Season::Fall),
        ("fire_co_alarm", Season::Fall),
        ("thermal_sleeping_bag", Season::Fall),
        ("water_jug_7gal", Season::Spring),
        ("water_bleach_unscented", Season::Spring),
    ] {
        assert_eq!(item(id).season, Some(season), "{id}");
    }
    // The key safe: an independent test, never the cheap four-dial boxes.
    let ks = item("security_key_safe");
    assert!(cites(ks, "sold_secure_key_safes"));
    assert!(ks.avoid.iter().any(|a| a.contains("four-dial")));
    // The go-bag gains old glasses and the document flash drive; the car kit the roadside number.
    let bag = item("evac_go_bag").look_for.join(" ");
    assert!(bag.contains("old pair of glasses") && bag.contains("flash drive"));
    assert!(
        item("gethome_car_kit")
            .look_for
            .join(" ")
            .contains("roadside-assistance number")
    );
}

#[test]
fn new_item_numbers_match_their_sources() {
    // NIST: butane boils near 31.7 °F, propane near −43.7 °F.
    let cyl = item("food_propane_cylinders");
    assert!(cyl.spec.contains("−44 °F") && cyl.spec.contains("32 °F"));
    assert!(
        cyl.spec.contains("no more than two inside"),
        "Lehi's two-cylinder limit"
    );
    // AAP: up to 32 oz of formula a day; three days of ready-to-feed (CDC).
    let rtf = item("food_infant_formula_rtf");
    assert!(rtf.spec.contains("32 ounces") && rtf.spec.contains("three days"));
    assert!(rtf.life_safety);
    // FDA: insulin keeps 28 days between 59 and 86 °F.
    let fridge = item("med_rx_12v_fridge");
    assert!(fridge.spec.contains("28 days") && fridge.spec.contains("86 °F"));
    // EPA ash guidance: NIOSH N95 or P100.
    let ppe = item("med_cleanup_ppe");
    assert!(ppe.spec.contains("P100") && cites(ppe, "epa_protect_from_ash"));
    // CDC: floodwater mosquitoes surge in the weeks after a flood; EPA-registered repellents.
    let rep = item("med_insect_repellent");
    assert!(
        cites(rep, "cdc_mosquitoes_after_flood") && cites(rep, "cdc_preventing_mosquito_bites")
    );
    // The water heater holds 20 to 80 gallons (DOE); FEMA's checklist names it as a quake hazard.
    let straps = item("water_heater_strap_kit");
    assert!(straps.spec.contains("20 to 80 gallons") && cites(straps, "fema_b526_eq_checklist"));
    assert_eq!(straps.hazard_extras, vec![HazardId::Earthquake]);
    // The hand pump's deep-well price is unverified, and says so.
    let pump = item("water_well_hand_pump");
    assert!(
        pump.price_band_usd
            .note
            .as_deref()
            .unwrap_or("")
            .contains("UNVERIFIED")
    );
}
