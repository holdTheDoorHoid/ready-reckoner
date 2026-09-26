//! Checks on the item catalogue that go beyond the validator: every price band matches its
//! observations, the items the brief requires exist with the right shape, and numbers in item text
//! match the sources they cite.

use std::collections::BTreeMap;

use rr_content::policy::{self, find_phrases, tokens};
use rr_content::validate::is_money_reserve;
use rr_types::{BucketId, Item, TierId};

const OBSERVATIONS: &str = include_str!("../../../docs/PRICE_OBSERVATIONS.md");

fn content() -> &'static rr_content::Content {
    rr_content::content()
}

fn item(id: &str) -> &'static Item {
    content()
        .item(id)
        .unwrap_or_else(|| panic!("the catalogue has no item `{id}`"))
}

fn text(i: &Item) -> String {
    let mut parts = vec![i.name.clone(), i.spec.clone()];
    parts.extend(i.look_for.iter().cloned());
    parts.extend(i.avoid.iter().cloned());
    if let Some(n) = &i.price_band_usd.note {
        parts.push(n.clone());
    }
    parts.join("\n")
}

/// `(item id) -> [(unit price, per)]` from the Markdown table in docs/PRICE_OBSERVATIONS.md.
fn observations() -> BTreeMap<String, Vec<(f64, String)>> {
    let mut map: BTreeMap<String, Vec<(f64, String)>> = BTreeMap::new();
    for line in OBSERVATIONS.lines() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let cells: Vec<&str> = rest.split(" | ").collect();
        let id = cells[0].trim_end_matches('`').to_owned();
        let unit_price: f64 = cells[5]
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("bad unit price in {line}"));
        let per = cells[6].trim().to_owned();
        map.entry(id).or_default().push((unit_price, per));
    }
    map
}

#[test]
fn price_bands_match_the_observation_log() {
    let obs = observations();
    assert!(obs.len() >= 60, "only {} items observed", obs.len());
    for i in &content().items {
        if i.free || is_money_reserve(i) {
            assert!(
                !obs.contains_key(i.id.as_str()),
                "{} is free or a reserve but has observations",
                i.id
            );
            continue;
        }
        let rows = obs
            .get(i.id.as_str())
            .unwrap_or_else(|| panic!("{} has no price observations", i.id));
        assert!(
            rows.len() >= 2,
            "{} has {} observation(s); two are required",
            i.id,
            rows.len()
        );
        for (_, per) in rows {
            assert_eq!(
                per, &i.price_band_usd.per,
                "{}: observation unit differs from the band",
                i.id
            );
        }
        let min = rows.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
        let max = rows.iter().map(|r| r.0).fold(f64::NEG_INFINITY, f64::max);
        let low = f64::from(i.price_band_usd.low);
        let high = f64::from(i.price_band_usd.high);
        assert!(
            (low - min).abs() < 0.006,
            "{}: band low {low} but lowest observation {min}",
            i.id
        );
        assert!(
            (high - max).abs() < 0.006,
            "{}: band high {high} but highest observation {max}",
            i.id
        );
    }
    for id in obs.keys() {
        let i = content()
            .item(id)
            .unwrap_or_else(|| panic!("observation for unknown item {id}"));
        assert!(!i.free, "observation for free item {id}");
    }
}

#[test]
fn every_category_has_a_file_and_the_catalogue_size_is_in_range() {
    let n = content().items.len();
    assert!(
        (100..=140).contains(&n),
        "{n} items; the brief asks for about 100–140"
    );
    for cat in policy::CATEGORIES {
        assert!(
            content().items_in_category(cat).count() >= 1,
            "no items in category {cat}"
        );
    }
}

#[test]
fn the_free_actions_named_in_the_brief_exist() {
    // research risk-model §4.3 and the content brief
    for id in [
        "docs_effak",                    // documents copied
        "comms_family_plan",             // family plan
        "comms_out_of_area_contact",     // out-of-area contact
        "community_know_two_neighbours", // neighbours' numbers
        "fire_test_alarms",              // alarm tests
        "med_refill_at_seven",           // refill-at-seven rule
        "water_heater_reserve",          // water-heater reserve
        "evac_half_tank",                // half-tank rule
        "comms_county_alerts",           // sign up for county alerts
        "evac_know_zone",                // know your evacuation zone
        "fire_learn_shutoffs",           // learn shut-offs
        "evac_if_then_triggers",         // if-then triggers
        "evac_go_stay_card",             // go/stay card
        "evac_ten_minute_drills",        // ten-minute drills
        "community_block_group_cert",    // CERT / block group
        "thermal_cooling_center",        // cooling centre location
        "med_988_saved",                 // 988 saved
        "water_reused_bottles",          // water in containers already owned
    ] {
        let i = item(id);
        assert!(i.free, "{id} must be free");
        assert_eq!(i.tier, TierId::Now, "{id} must be in tier now");
        assert_eq!(i.price_band_usd.high, 0.0, "{id} must cost nothing");
    }
}

#[test]
fn community_items_carry_real_weight() {
    for id in [
        "community_know_two_neighbours",
        "community_check_in_agreement",
        "community_block_group_cert",
    ] {
        let i = item(id);
        assert_eq!(i.category, "community");
        assert!(i.buckets.len() >= 3, "{id} should help several buckets");
        assert!(
            i.citations.iter().any(|c| !c.as_str().starts_with("rr_")),
            "{id} cites outside evidence"
        );
    }
}

#[test]
fn the_antibiotics_card_is_a_free_clinician_route_with_no_drugs_named() {
    let i = item("med_antibiotics_clinician_card");
    assert!(i.free && i.tier == TierId::Now && i.quantity_rule == "once");
    assert_eq!(i.category, "medical");
    let t = text(i);
    let toks = tokens(&t);
    assert!(
        find_phrases(&toks, policy::DRUG_NAMES).is_empty(),
        "the antibiotics card names a drug"
    );
    let avoid = i.avoid.join(" ").to_lowercase();
    for must in [
        "aquarium",
        "veterinary",
        "sharing",
        "expired",
        "on your own",
        "$300–400",
    ] {
        assert!(
            avoid.contains(must),
            "antibiotics card avoid list lacks `{must}`"
        );
    }
    assert!(i.spec.contains("never sets an antibiotic quantity"));
    for c in [
        "cdc_antibiotic_use",
        "fda_fish_antibiotics_warning_2023",
        "mo_med_2026_antibiotic_kits",
    ] {
        assert!(i.citations.iter().any(|x| x.as_str() == c), "cites {c}");
    }
}

#[test]
fn firearms_appear_once_as_a_free_security_action_with_the_public_health_sentence() {
    let mut with_firearm_words = Vec::new();
    for i in &content().items {
        if !find_phrases(&tokens(&text(i)), policy::FIREARM_TOKENS).is_empty() {
            with_firearm_words.push(i.id.as_str());
        }
    }
    assert_eq!(with_firearm_words, vec![policy::PERMITTED_FIREARM_ITEM]);
    let i = item(policy::PERMITTED_FIREARM_ITEM);
    assert!(i.free && i.category == "security" && i.price_band_usd.high == 0.0);
    assert!(i.spec.contains("never budgets"));
    assert!(i.spec.contains("higher risk of suicide"));
    assert!(
        i.citations
            .iter()
            .any(|c| c.as_str() == "anglemyer_2014_firearm_access")
    );
}

#[test]
fn rare_catastrophic_items_are_flagged_in_the_one_year_tier() {
    for id in [
        "rare_radiation_meter",
        "rare_potassium_iodide",
        "rare_faraday_storage",
    ] {
        let i = item(id);
        assert!(i.rare_catastrophic, "{id} flagged");
        assert_eq!(i.tier, TierId::Y1, "{id} in tier y1");
    }
    let ki = item("rare_potassium_iodide");
    assert!(ki.free);
    assert_eq!(ki.quantity_rule, "once_if_near_nuclear_plant");
    assert!(ki.spec.contains("10-mile emergency planning zone"));
    assert!(ki.spec.contains("officials"));
    // no other item is flagged
    let flagged: Vec<&str> = content()
        .items
        .iter()
        .filter(|i| i.rare_catastrophic)
        .map(|i| i.id.as_str())
        .collect();
    assert_eq!(flagged.len(), 3, "{flagged:?}");
}

#[test]
fn water_items_match_their_sources() {
    // EPA table: 8 drops of 6% or 6 drops of 8.25% bleach per gallon, double for cloudy, 30 minutes
    let bleach = item("water_bleach_unscented");
    for s in [
        "8 drops of 6%",
        "6 drops of 8.25%",
        "double it",
        "30 minutes",
    ] {
        assert!(bleach.spec.contains(s), "bleach spec lacks `{s}`");
    }
    assert_eq!(bleach.maintenance.and_then(|m| m.rotate_months), Some(12)); // EPA: under a year
    // boil: 1 minute; 3 minutes above 5,000 ft (EPA, the stricter altitude)
    let boil = item("water_boil_method");
    assert!(boil.spec.contains("1 minute") && boil.spec.contains("5,000 feet"));
    // CDC: replace self-filled water every 6 months
    for id in ["water_reused_bottles", "water_jug_7gal", "water_drum_55gal"] {
        assert_eq!(
            item(id).maintenance.and_then(|m| m.rotate_months),
            Some(6),
            "{id}"
        );
    }
    // volumes: 1 US gallon = 3.785 L; 7 gal = 26.5 L; 55 gal = 208.2 L
    assert!((item("water_stored_bottled").volume_l_per_unit.unwrap() - 3.785).abs() < 1e-3);
    assert!((item("water_jug_7gal").volume_l_per_unit.unwrap() - 7.0 * 3.785).abs() < 0.05);
    assert!((item("water_drum_55gal").volume_l_per_unit.unwrap() - 55.0 * 3.785).abs() < 0.05);
    // the 7-gallon jug observation behind the research's $3.43 per gallon of storage
    let jug = item("water_jug_7gal");
    assert!((f64::from(jug.price_band_usd.high) / 7.0 - 3.43).abs() < 0.005);
    // three water levels come from one rule, and at least two kinds of item meet it
    let water_rule = content()
        .items
        .iter()
        .filter(|i| i.quantity_rule == "water_gallons")
        .count();
    assert!(water_rule >= 3);
    assert_eq!(
        item("water_heater_reserve").citations[0].as_str(),
        "doe_water_heaters"
    );
    assert!(
        item("water_heater_reserve")
            .spec
            .contains("20 to 80 gallons")
    );
}

#[test]
fn food_items_plan_in_calories_and_warn_about_kits() {
    for i in content().items_in_category("food") {
        if i.quantity_rule == "food_kcal" {
            assert_eq!(
                i.energy_kcal_per_unit,
                Some(2000.0),
                "{} counts 2,000 kcal per unit",
                i.id
            );
            assert_eq!(i.unit, "2,000 kcal");
        }
    }
    // USDA Thrifty Food Plan, August 2026: $236.30 a week for four = $8.44 a person-day
    let pantry = item("food_pantry_rotation");
    assert!((f64::from(pantry.price_band_usd.low) - 236.30 / 28.0).abs() < 0.005);
    let kits = item("food_freeze_dried");
    assert!(kits.spec.contains("1,290 to 1,730 calories") && kits.spec.contains("0.6 gallon"));
    assert!(item("food_infant_formula").life_safety);
}

#[test]
fn power_items_carry_the_safety_rules() {
    let generator = item("power_generator");
    assert!(generator.spec.contains("20 feet"));
    assert!(generator.spec.contains("2.8 gallons") && generator.spec.contains("up to 7"));
    let fuel = item("power_generator_fuel");
    assert!(fuel.spec.contains("25 gallons") && fuel.spec.contains("10 in an attached garage"));
    let solar = item("power_solar_panel");
    assert!(solar.spec.contains("December") && solar.spec.contains("not a full-size fridge"));
    assert!(item("power_device_battery").life_safety);
    for id in ["power_ev_storm_charge", "power_ev_range_plan"] {
        assert_eq!(item(id).quantity_rule, "once_if_ev");
    }
    assert!(item("comms_noaa_radio").buckets.contains(&BucketId::Comms));
}

#[test]
fn life_safety_items_include_alarms_water_medicine_and_device_power() {
    for id in [
        "fire_smoke_alarm",
        "fire_co_alarm",
        "water_stored_bottled",
        "med_reserve_supply",
        "power_device_battery",
        "med_cooler_refrigerated_rx",
    ] {
        assert!(item(id).life_safety, "{id} should be life-safety");
    }
}

#[test]
fn catalogue_includes_name_tables_and_every_item() {
    let c = rr_content::catalogue();
    assert_eq!(c.items.len(), content().items.len());
    assert_eq!(c.hazards.len(), 35);
    assert_eq!(c.buckets.len(), 14);
    assert_eq!(c.tiers.len(), 7);
    let json = serde_json::to_string(&c).expect("catalogue serialises");
    let back: rr_types::Catalogue = serde_json::from_str(&json).expect("and parses back");
    assert_eq!(back, c);
}
