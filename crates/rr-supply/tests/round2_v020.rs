//! Round 2, v0.2.0: the practitioner review's failure cases (Coos Bay batteries and toilet paper,
//! Hays generator and drought, Sugar Land cold chain, Chicago free water), the new capabilities
//! (recharging, cooking, clean air, rain and hauling), the household decisions, the bare-minimum
//! kit and the storage estimate. Every number is checked against the rule's cited source or the
//! arithmetic the rule documents in `docs/QUANTITY_RULES.md`.

mod common;

use rr_supply::{
    LineKind, SizedLine, SupplyContext, minimum_kit, sized_requirements, storage_by_tier,
};
use rr_types::{BucketId, HazardId, TierId, fixtures};

fn get<'a>(lines: &'a [SizedLine], id: &str) -> &'a SizedLine {
    lines.iter().find(|l| l.line.id == id).unwrap_or_else(|| {
        panic!(
            "no line {id}; have {:?}",
            lines.iter().map(|l| &l.line.id).collect::<Vec<_>>()
        )
    })
}

fn has(lines: &[SizedLine], id: &str) -> bool {
    lines.iter().any(|l| l.line.id == id)
}

/// Coos Bay at the 1-in-500 dial as the practitioner planned it: six months without power, a year
/// without well water after a Cascadia earthquake, two months without a store, 90 days of medicine.
fn coos_bay_long() -> Vec<rr_types::BucketAssessment> {
    vec![
        common::days(BucketId::Power, 180.0),
        common::driven_by(
            common::days(BucketId::WaterOut, 365.0),
            &[(HazardId::Earthquake, 1.0)],
        ),
        common::days(BucketId::Supplies, 60.0),
        common::days(BucketId::Medication, 90.0),
        common::days(BucketId::Comms, 30.0),
    ]
}

/// Round-2 review P-04: long targets were met with consumables. Coos Bay bought 26 packs of
/// batteries, 146 rolls of toilet paper and "1 gallon adds 7.2 days" of fuel.
#[test]
fn coos_bay_stores_two_weeks_of_batteries_and_paper_for_the_store_target() {
    let input = fixtures::get("coos-bay-well-owner-2").unwrap();
    let lines = sized_requirements(&input, &coos_bay_long(), &SupplyContext::default());
    // 2 packs (14 days at a pack a week), not 26; the line says to recharge beyond.
    let packs = get(&lines, "power.battery_packs");
    assert_eq!(packs.quantity, 2.0);
    assert!(packs.line.plain.contains("way to recharge"));
    // The household owns a generator: it can recharge, so no charger or solar need.
    assert!(!has(&lines, "power.recharge_capability"));
    assert!(!has(&lines, "power.solar_panel_units"));
    // Toilet paper follows the 60-day store target: 2 people × 60 days ÷ 5 = 24 rolls, not 146.
    let paper = get(&lines, "supplies.toilet_paper_rolls");
    assert_eq!(paper.quantity, 24.0);
    // Phone power: 2 people aged 13+ × 15 Wh × 14 days = 420 Wh, not 180 days of it.
    assert_eq!(get(&lines, "comms.phone_power_wh").quantity, 420.0);
    // Fuel: 25 gallons, which run a 2.2 kW inverter about 214 hours at light load: its days are the
    // 8.9 the fuel lasts, never the 180-day target.
    let fuel = get(&lines, "power.generator_fuel_gallons");
    assert_eq!(fuel.quantity, 25.0);
    assert!((fuel.days.unwrap() - 25.0 / 2.8).abs() < 1e-9);
    assert!(fuel.line.plain.contains("214 hours"), "{}", fuel.line.plain);
    // Medicine: the 30-day reserve, and the other 60 days by 60- to 90-day fills.
    assert_eq!(get(&lines, "medication.medication_days").quantity, 30.0);
    let fills = get(&lines, "medication.medication_fills");
    assert_eq!(fills.quantity, 60.0);
    assert!(
        fills.line.plain.contains("60 to 90 day fills"),
        "{}",
        fills.line.plain
    );
    assert!(
        fills
            .line
            .citations
            .iter()
            .any(|c| c == "medicare_drugs_disaster")
    );
    // An earthquake drives the year without water, so the sewer may be broken: toilet bags for
    // the whole year (2 × 0.45 × 365 = 329); without the earthquake they stop at 30 days.
    assert_eq!(get(&lines, "water_out.toilet_bags").quantity, 329.0);
    let mut no_quake = coos_bay_long();
    no_quake[1] = common::days(BucketId::WaterOut, 365.0);
    let calm = sized_requirements(&input, &no_quake, &SupplyContext::default());
    assert_eq!(get(&calm, "water_out.toilet_bags").quantity, 27.0);
    // The wood stove cooks: a note, nothing to buy, and the propane fuel stays optional.
    let cooking = get(&lines, "supplies.cooking_capability.note");
    assert!(cooking.line.plain.contains("wood stove"));
    assert!(has(&lines, "supplies.propane_cylinders.optional"));
    // The well is the raw-water source; a hand pump is offered, rain barrels are not.
    assert!(
        get(&lines, "water_out.water_treatment_capacity")
            .line
            .plain
            .contains("your well")
    );
    assert_eq!(
        get(&lines, "water_out.well_hand_pump.optional").quantity,
        1.0
    );
    assert!(!lines.iter().any(|l| l.line.rule == "rain_catchment_units"));
    // Carriers for hauling (365 days is past the 14 stored); two people carry two.
    assert_eq!(get(&lines, "water_out.water_carriers").quantity, 2.0);
    // Paper tableware for the first month of the year-long target (30 days at a kit per 14 days =
    // 3 kits), then dishes are washed with treated water: never 27 kits for a year.
    let kits = get(&lines, "water_out.household_ops_kits");
    assert_eq!(kits.quantity, 3.0);
    assert!(
        kits.line.plain.contains("the water you treat"),
        "{}",
        kits.line.plain
    );
}

/// Round-2 review P-02, P-03: Hays' 60-day target is a drought, so hauling is the real work.
#[test]
fn hays_hauls_water_in_a_drought() {
    let input = fixtures::get("hays-kansas-farm-5").unwrap();
    let targets = vec![
        common::days(BucketId::Power, 3.0),
        common::driven_by(
            common::days(BucketId::WaterOut, 60.0),
            &[(HazardId::Drought, 1.0)],
        ),
        common::days(BucketId::Supplies, 10.0),
    ];
    let lines = sized_requirements(&input, &targets, &SupplyContext::default());
    // A tote for the 12 animals (79 gallons a day, about 3.5 days a load), in the animals' water.
    let tote = get(&lines, "water_out.livestock_haul_tank");
    assert_eq!(
        (tote.quantity, tote.line.item_class.as_str()),
        (1.0, "livestock_water")
    );
    // Five people haul with four carriers; a drought brings no rain, so no barrels.
    assert_eq!(get(&lines, "water_out.water_carriers").quantity, 4.0);
    assert!(!lines.iter().any(|l| l.line.rule == "rain_catchment_units"));
    // The animals store 14 days until pump power exists (v0.1.1), and a hand pump is offered.
    assert_eq!(get(&lines, "water_out.livestock_water").quantity, 1109.5);
    assert!(has(&lines, "water_out.well_hand_pump.optional"));
}

/// Round-2 review P-01, P-17: Sugar Land's insulin and formula-fed baby.
#[test]
fn sugar_land_keeps_insulin_cold_and_feeds_the_baby_without_water() {
    let input = fixtures::get("sugar-land-ev-household-3").unwrap();
    let targets = vec![
        common::days(BucketId::Power, 5.0),
        common::days(BucketId::WaterBoil, 3.0),
        common::days(BucketId::WaterOut, 3.0),
        common::days(BucketId::Supplies, 7.0),
        common::days(BucketId::Medication, 10.0),
    ];
    let hot = SupplyContext {
        days_at_or_above_95f: Some(45.0),
        ..SupplyContext::default()
    };
    let lines = sized_requirements(&input, &targets, &hot);
    // A hot county, 5 days without power, no generator: a 12-volt fridge for the insulin, with the
    // power station to run it, both life-safety.
    let fridge = get(&lines, "medication.rx_fridge_units");
    assert!(fridge.life_safety && fridge.kind == LineKind::Need);
    assert!(fridge.line.plain.contains("86 °F"), "{}", fridge.line.plain);
    assert!(get(&lines, "power.power_station_units").life_safety);
    // Temperate: the room stays below 86 °F, no fridge.
    let mild = sized_requirements(&input, &targets, &SupplyContext::default());
    assert!(!has(&mild, "medication.rx_fridge_units"));
    // Three days of ready-to-feed formula (96 fl oz) and powder for the other four (20 oz).
    let rtf = get(&lines, "supplies.infant_formula_rtf");
    assert_eq!((rtf.quantity, rtf.tier), (96.0, TierId::H72));
    assert!(rtf.life_safety);
    assert_eq!(get(&lines, "supplies.infant_formula_oz").quantity, 20.0);
    // Powdered formula under a boil notice needs a way to boil water: a stove, in the three-day
    // tier, with its fuel.
    let stove = get(&lines, "supplies.cooking_capability");
    assert_eq!((stove.kind, stove.tier), (LineKind::Need, TierId::H72));
    assert!(stove.line.plain.contains("formula"), "{}", stove.line.plain);
    assert!(has(&lines, "supplies.propane_cylinders"));
}

/// Round-2 review P-13: the zero-budget student can meet a 5-gallon target with bottles it fills
/// for free.
#[test]
fn chicago_meets_its_water_target_with_free_bottles() {
    let input = fixtures::get("chicago-student-zero-budget-1").unwrap();
    let targets = vec![common::days(BucketId::WaterOut, 5.0)];
    let lines = sized_requirements(&input, &targets, &SupplyContext::default());
    assert_eq!(get(&lines, "water_out.water_gallons").quantity, 5.0);
    let free = get(&lines, "water_out.water_gallons.alt.reused_bottles");
    assert_eq!(free.quantity, 5.0);
    assert!(
        free.line.plain.contains("all 5 days of your water"),
        "{}",
        free.line.plain
    );
}

/// Round-2 review P-04, P-09: past two weeks of batteries, a way to recharge; the car where there
/// is one, the sun where there is not.
#[test]
fn a_long_power_target_asks_for_a_way_to_recharge() {
    let philly = fixtures::get("philadelphia-renters-4").unwrap();
    let targets = vec![common::days(BucketId::Power, 30.0)];
    let lines = sized_requirements(&philly, &targets, &SupplyContext::default());
    let charger = get(&lines, "power.recharge_capability");
    assert_eq!(
        charger.line.item_class,
        rr_supply::rules::power::RECHARGE_CLASS
    );
    assert_eq!(charger.tier, TierId::M1);
    assert!(has(&lines, "power.solar_panel_units.optional"));
    let chicago = fixtures::get("chicago-student-zero-budget-1").unwrap();
    let lines = sized_requirements(&chicago, &targets, &SupplyContext::default());
    assert!(!has(&lines, "power.recharge_capability"));
    let solar = get(&lines, "power.solar_panel_units");
    assert_eq!(solar.kind, LineKind::Need);
    assert_eq!(
        solar.line.item_class,
        rr_supply::rules::power::RECHARGE_CLASS
    );
}

/// The clean-air bucket (contract v2): respirators for teens and adults by the county's smoke
/// days, an air cleaner sized for the clean room or a box-fan filter, and the plan.
#[test]
fn clean_air_lines_follow_the_smoke_days() {
    let input = fixtures::get("missoula-smoke-2").unwrap();
    let targets = vec![common::readiness(BucketId::CleanAir, 0.6)];
    let smoky = SupplyContext {
        smoke_days: Some(25.0),
        ..SupplyContext::default()
    };
    let lines = sized_requirements(&input, &targets, &smoky);
    // 2 people aged 13+ × 25 smoke days = 50 respirators.
    let masks = get(&lines, "clean_air.n95_masks");
    assert_eq!(masks.quantity, 50.0);
    assert!(
        masks.line.plain.contains("25 days of unhealthy smoke"),
        "{}",
        masks.line.plain
    );
    assert!(
        masks
            .line
            .citations
            .iter()
            .any(|c| c == common::TARGET_SOURCE)
    );
    assert!(
        get(&lines, "clean_air.air_cleaner_units")
            .line
            .plain
            .contains("at least 130")
    );
    let diy = get(&lines, "clean_air.air_cleaner_units.alt.diy_filter_box");
    assert_eq!(diy.kind, LineKind::Alternative);
    assert_eq!(get(&lines, "clean_air.clean_room_plan").tier, TierId::Now);
    // Below 2 in 100 in ten years (DESIGN §4.4's bar) the clean-air lines are left out.
    let rare = sized_requirements(
        &input,
        &[common::readiness(BucketId::CleanAir, 0.01)],
        &smoky,
    );
    assert!(!rare.iter().any(|l| l.line.bucket == BucketId::CleanAir));
    let at_bar = sized_requirements(
        &input,
        &[common::readiness(BucketId::CleanAir, 0.02)],
        &smoky,
    );
    assert!(has(&at_bar, "clean_air.clean_room_plan"));
    // No respirators counted under medical emergencies any more (round-2 review P-05).
    let old = sized_requirements(
        &input,
        &[common::readiness(BucketId::MedicalEmergency, 0.9)],
        &smoky,
    );
    assert!(!old.iter().any(|l| l.line.rule == "n95_masks"));
}

/// Round-2 review RR-P09: the insurance decisions follow the household and the hazards.
#[test]
fn insurance_decisions_follow_the_household_and_the_hazards() {
    let sl = fixtures::get("sugar-land-ev-household-3").unwrap();
    let hurricane = vec![
        common::driven_by(
            common::readiness(BucketId::HomeLoss, 0.1),
            &[(HazardId::Hurricane, 0.7), (HazardId::HouseFire, 0.3)],
        ),
        common::months(3.0),
    ];
    let lines = sized_requirements(&sl, &hurricane, &SupplyContext::default());
    assert!(has(&lines, "home_loss.insurance_wind_deductible"));
    assert!(has(&lines, "income.insurance_life_disability"));
    assert!(
        get(&lines, "home_loss.insurance_flood")
            .line
            .plain
            .contains("3 in 10")
    );
    // Hays' basement: the sewer-backup decision; the Miami condo: the unit-owners check.
    let hays = fixtures::get("hays-kansas-farm-5").unwrap();
    let home = vec![common::readiness(BucketId::HomeLoss, 0.1)];
    assert!(has(
        &sized_requirements(&hays, &home, &SupplyContext::default()),
        "home_loss.insurance_sewer_backup"
    ));
    let miami = fixtures::get("miami-condo-retiree-1").unwrap();
    let lines = sized_requirements(&miami, &home, &SupplyContext::default());
    assert!(has(&lines, "home_loss.insurance_condo_unit"));
    assert!(
        !has(&lines, "home_loss.insurance_wind_deductible"),
        "no hurricane named"
    );
}

/// Round-2 review P-09: in a hot county fans do not count; the heat plan's place to go does, and
/// it is life-safety when a power cut is part of the plan.
#[test]
fn hot_counties_count_a_place_to_go_not_a_fan() {
    let input = fixtures::get("phoenix-apartment-cpap-1").unwrap();
    let targets = vec![
        common::days(BucketId::Power, 2.0),
        common::driven_by(
            common::days(BucketId::Thermal, 3.0),
            &[(HazardId::HeatWave, 1.0)],
        ),
    ];
    let hot = SupplyContext {
        days_at_or_above_95f: Some(110.0),
        ..SupplyContext::default()
    };
    let lines = sized_requirements(&input, &targets, &hot);
    assert!(has(&lines, "thermal.battery_fan.optional"));
    assert!(has(&lines, "thermal.cooling_towel.optional"));
    let plan = get(&lines, "thermal.cooling_plan");
    assert!(plan.life_safety);
    assert!(plan.line.plain.contains("we go to"));
    // The room thermometer stays a need: it says when to go.
    assert_eq!(get(&lines, "thermal.room_thermometer").kind, LineKind::Need);
}

/// A house on town water with a long no-water target in a state that lets households drink
/// rainwater: rain barrels are its raw-water source, so the filter counts.
#[test]
fn rain_barrels_are_the_source_the_filter_needs() {
    let mut input = fixtures::get("sugar-land-ev-household-3").unwrap();
    input.housing.raw_water_source = None;
    let targets = vec![common::days(BucketId::WaterOut, 45.0)];
    let texas = SupplyContext {
        state_fips: SupplyContext::state_of("48157"),
        ..SupplyContext::default()
    };
    let lines = sized_requirements(&input, &targets, &texas);
    let rain = get(&lines, "water_out.rain_catchment_units");
    assert_eq!((rain.kind, rain.quantity), (LineKind::Need, 1.0));
    assert!(
        get(&lines, "water_out.water_treatment_capacity")
            .line
            .plain
            .contains("rain line")
    );
    let sizer = rr_supply::ItemSizer::new(&input, &targets, &texas);
    assert_eq!(
        sizer.quantity("water_treatment_capacity").unwrap().quantity,
        1.0
    );
    // With no state known the barrels are optional, so no source and no filter.
    let sizer = rr_supply::ItemSizer::new(&input, &targets, &SupplyContext::default());
    assert_eq!(
        sizer.quantity("water_treatment_capacity").unwrap().quantity,
        0.0
    );
    let lines = sized_requirements(&input, &targets, &SupplyContext::default());
    assert!(
        get(&lines, "water_out.water_treatment_capacity")
            .line
            .plain
            .contains("a filter adds nothing yet")
    );
}

/// Round-2 review P-11, P-15: rural homes get the wound add-on in the three-day tier; a long walk
/// home in a temperate county gets a filter.
#[test]
fn rural_wound_care_and_the_long_walk_filter() {
    let hays = fixtures::get("hays-kansas-farm-5").unwrap();
    let lines = sized_requirements(&hays, &common::generic(), &SupplyContext::default());
    let wound = get(&lines, "medical_emergency.wound_care_addon");
    assert_eq!(wound.tier, TierId::H72);
    let filter = get(&lines, "get_home.get_home_filter.person_2");
    assert_eq!(filter.quantity, 1.0);
    assert!(
        !has(&lines, "get_home.get_home_filter.person_1"),
        "a zero-km commute"
    );
    // Bleach is always a three-day item.
    assert_eq!(get(&lines, "water_boil.bleach_bottles").tier, TierId::H72);
}

/// The bare-minimum kit (contract v2's `Dials::minimum_kit`): three days of water, light, warmth
/// and medicine continuity.
#[test]
fn the_bare_minimum_kit_covers_three_days() {
    let philly = fixtures::get("philadelphia-renters-4").unwrap();
    let lines = sized_requirements(&philly, &common::philadelphia(), &SupplyContext::default());
    let kit: Vec<(&str, f64)> = minimum_kit(&lines)
        .iter()
        .map(|l| (l.line.id.as_str(), l.minimum.unwrap()))
        .collect();
    let share = |id: &str| {
        kit.iter()
            .find(|(i, _)| *i == id)
            .map(|(_, q)| *q)
            .unwrap_or_else(|| panic!("{id} not in the kit: {kit:?}"))
    };
    // 3 days of 4.3125 gallons = 12.9 (the whole 3-day target).
    assert_eq!(share("water_out.water_gallons"), 12.9);
    assert_eq!(share("water_boil.bleach_bottles"), 1.0);
    assert_eq!(share("power.lights"), 1.0);
    assert_eq!(share("power.battery_packs"), 1.0);
    assert_eq!(share("thermal.blankets"), 4.0);
    assert_eq!(share("thermal.warm_layers"), 4.0);
    assert_eq!(share("thermal.cooling_plan"), 1.0);
    // The senior's medicine: 3 of the 14 days.
    assert_eq!(share("medication.medication_days"), 3.0);
    assert!(kit.len() == 8, "{kit:?}");
    // Food, the go-bags and the smoke alarms are not in it.
    assert!(!kit.iter().any(|(i, _)| i.starts_with("supplies.food_kcal")));
    // Sugar Land: the cooler bag's day, and the baby's three days of ready-to-feed formula.
    let sl = fixtures::get("sugar-land-ev-household-3").unwrap();
    let lines = sized_requirements(&sl, &common::generic(), &SupplyContext::default());
    let get_min = |id: &str| get(&lines, id).minimum;
    assert_eq!(get_min("medication.rx_cold_storage"), Some(1.0));
    assert_eq!(get_min("supplies.infant_formula_rtf"), Some(96.0));
    assert_eq!(get_min("power.power_station_units"), None);
}

/// Round-2 review P-20: how much room and weight the stored supplies take at each tier.
#[test]
fn storage_space_and_weight_by_tier() {
    let philly = fixtures::get("philadelphia-renters-4").unwrap();
    let lines = sized_requirements(&philly, &common::philadelphia(), &SupplyContext::default());
    let tiers = storage_by_tier(&lines);
    assert_eq!(
        tiers.iter().map(|s| s.tier).collect::<Vec<_>>(),
        [TierId::H72, TierId::W2]
    );
    let w2 = &tiers[1];
    // Water 12.9 gal = 48.8 L; food 82,000 kcal × 1.9 L per 2,000 = 77.9 L; dog food 7 lb = 7 L;
    // 8 rolls × 1.3 L = 10.4 L: about 144 L (5.1 cubic feet).
    assert!((w2.volume_l - 144.1).abs() < 0.2, "{}", w2.volume_l);
    // 48.8 + 32.8 + 3.2 + 0.8 = 85.6 kg.
    assert!((w2.mass_kg - 85.6).abs() < 0.2, "{}", w2.mass_kg);
    assert!(w2.plain.contains("5.1 cubic feet"), "{}", w2.plain);
    assert!(w2.prior && w2.citations.iter().any(|c| c == "rr_expert_prior"));
    // The three-day tier holds three of the ten days of food: less space than the two-week one.
    assert!(tiers[0].volume_l < w2.volume_l);
    assert_eq!(tiers[0].parts[0].label, "water");
    // The Chicago student's 5 gallons and 10 days of food.
    let chicago = fixtures::get("chicago-student-zero-budget-1").unwrap();
    let lines = sized_requirements(&chicago, &common::generic(), &SupplyContext::default());
    let tiers = storage_by_tier(&lines);
    assert!(tiers.last().unwrap().volume_l > 0.0);
}

/// Contract v2's long-horizon section: present when a duration target reaches 30 days
/// (`long_horizon_min_days`, the design threshold) or the household turns it on, and the rule
/// `once_if_long_horizon` switches its free pointers on with it. An income target in months does
/// not count: it is the savings track.
#[test]
fn the_long_horizon_section_follows_the_longest_target() {
    use rr_supply::{ItemSizer, long_horizon};
    let mut input = fixtures::get("philadelphia-renters-4").unwrap();
    let short = vec![
        common::days(BucketId::Power, 3.0),
        common::days(BucketId::Supplies, 29.0),
        common::months(6.0),
    ];
    assert!(!long_horizon(&input, &short));
    let q = |input: &rr_types::PlanInput, b: &[rr_types::BucketAssessment]| {
        ItemSizer::new(input, b, &SupplyContext::default())
            .quantity("once_if_long_horizon")
            .unwrap()
            .quantity
    };
    assert_eq!(q(&input, &short), 0.0);
    // Thirty days of any duration target brings the section in.
    let long = vec![common::days(BucketId::WaterOut, 30.0)];
    assert!(long_horizon(&input, &long));
    assert_eq!(q(&input, &long), 1.0);
    // Coos Bay's year without well water, too.
    let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
    assert!(long_horizon(&coos, &coos_bay_long()));
    // The household's own switch shows it below 30 days.
    input.dials.long_horizon = true;
    assert!(long_horizon(&input, &short));
    assert_eq!(q(&input, &short), 1.0);
}
