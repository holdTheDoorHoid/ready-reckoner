//! The fixture households: every file is embedded, parses, survives canonical re-serialisation
//! byte for byte without losing or inventing anything, and validates clean.

mod common;

use std::collections::BTreeSet;

use rr_types::*;
use serde_json::Value;

/// Structural equality in which numbers compare by value (the files write `3`, Rust writes `3.0`).
fn same_json(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_json(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| same_json(v, w)))
        }
        _ => a == b,
    }
}

#[test]
fn every_fixture_file_is_embedded() {
    let dir = common::repo_path("fixtures/households");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter_map(|n| n.strip_suffix(".json").map(str::to_owned))
        .collect();
    let embedded: BTreeSet<String> = fixtures::RAW.iter().map(|(n, _)| (*n).to_owned()).collect();
    assert_eq!(
        embedded, on_disk,
        "update rr_types::fixtures::RAW and web/src/engine/fixtures.ts"
    );
    assert_eq!(fixtures::RAW.len(), 7);
    for (name, raw) in fixtures::RAW {
        let file = std::fs::read_to_string(dir.join(format!("{name}.json"))).unwrap();
        assert_eq!(*raw, file, "{name} is stale");
    }

    // The staged households in pending/ are embedded in PENDING, and only there.
    let pending_dir = dir.join("pending");
    let staged: BTreeSet<String> = std::fs::read_dir(&pending_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter_map(|n| n.strip_suffix(".json").map(str::to_owned))
        .collect();
    let embedded: BTreeSet<String> = fixtures::PENDING
        .iter()
        .map(|(n, _)| (*n).to_owned())
        .collect();
    assert_eq!(embedded, staged, "update rr_types::fixtures::PENDING");
    for (name, raw) in fixtures::PENDING {
        let file = std::fs::read_to_string(pending_dir.join(format!("{name}.json"))).unwrap();
        assert_eq!(*raw, file, "{name} is stale");
        assert!(
            fixtures::RAW.iter().all(|(n, _)| n != name),
            "{name} is both staged and in RAW"
        );
    }
    let names: Vec<&str> = fixtures::PENDING.iter().map(|(n, _)| *n).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "PENDING is sorted by name");
}

/// Removes from `canon` every field that defaults when absent and that `file` leaves out: the
/// top-level `assume_basics`, the dials' optional fields, and the contract v2 additions that
/// default (`housing.below_grade_bedroom`, `finances.benefits`, each person's `access_needs`).
fn strip_defaults_absent_from(file: &Value, canon: &mut Value) {
    fn strip(file: &Value, canon: &mut Value, keys: &[&str]) {
        for key in keys {
            if file.get(*key).is_none() {
                canon.as_object_mut().unwrap().remove(*key);
            }
        }
    }
    strip(file, canon, &["assume_basics"]);
    strip(
        &file["dials"],
        &mut canon["dials"],
        &[
            "water_level",
            "scenario_overrides",
            "rare_catastrophic_opt_in",
            "rare_opt_in",
            "minimum_kit",
            "long_horizon",
        ],
    );
    strip(
        &file["housing"],
        &mut canon["housing"],
        &["below_grade_bedroom"],
    );
    strip(&file["finances"], &mut canon["finances"], &["benefits"]);
    let people = file["people"].as_array().unwrap();
    for (i, person) in people.iter().enumerate() {
        strip(person, &mut canon["people"][i], &["access_needs"]);
    }
}

#[test]
fn every_fixture_parses_round_trips_and_validates() {
    for (name, raw) in fixtures::RAW.iter().chain(fixtures::PENDING) {
        let input: PlanInput =
            serde_json::from_str(raw).unwrap_or_else(|e| panic!("{name} does not parse: {e}"));

        // Canonical re-serialisation is stable, byte for byte.
        let canonical = serde_json::to_string_pretty(&input).unwrap();
        let again: PlanInput = serde_json::from_str(&canonical).unwrap();
        assert_eq!(again, input, "{name}");
        assert_eq!(
            serde_json::to_string_pretty(&again).unwrap(),
            canonical,
            "{name}"
        );

        // Nothing is lost or invented: the canonical form equals the file, apart from the fields
        // that default when absent (which the engine always writes out).
        let file: Value = serde_json::from_str(raw).unwrap();
        let mut canon: Value = serde_json::from_str(&canonical).unwrap();
        strip_defaults_absent_from(&file, &mut canon);
        assert!(
            same_json(&file, &canon),
            "{name}: canonical form differs\n{canonical}"
        );

        // Valid, and accepted by the single entry point the engine uses.
        assert_eq!(input.validate(), vec![], "{name}");
        assert_eq!(PlanInput::from_json(raw).unwrap(), input, "{name}");
    }
}

#[test]
fn all_and_get_agree_with_raw() {
    let all = fixtures::all();
    assert_eq!(all.len(), fixtures::RAW.len());
    for ((name, parsed), (raw_name, raw)) in all.iter().zip(fixtures::RAW) {
        assert_eq!(name, raw_name);
        assert_eq!(parsed, &serde_json::from_str::<PlanInput>(raw).unwrap());
        assert_eq!(fixtures::get(name).as_ref(), Some(parsed));
    }
    let pending = fixtures::pending();
    assert_eq!(pending.len(), fixtures::PENDING.len());
    for ((name, parsed), (raw_name, raw)) in pending.iter().zip(fixtures::PENDING) {
        assert_eq!(name, raw_name);
        assert_eq!(parsed, &serde_json::from_str::<PlanInput>(raw).unwrap());
        assert_eq!(
            fixtures::get(name).as_ref(),
            Some(parsed),
            "get finds staged ones"
        );
    }
    assert!(fixtures::get("no-such-household").is_none());
}

/// The six contract v2 households (DESIGN-DELTA §3, rr-plan's list; brief item 6): each place
/// is what its name says, and together they use every new input.
#[test]
fn the_v2_households_use_the_new_inputs() {
    let get = |n: &str| fixtures::get(n).unwrap_or_else(|| panic!("no household {n}"));
    let county = |n: &str| {
        let input = get(n);
        (
            input.location.zip.clone(),
            input.location.county_fips.clone(),
        )
    };

    // A missile-field county (Ward County, ND: Minot AFB, class A; high magnetic latitude).
    let minot = get("minot-missile-field-3");
    assert_eq!(
        county("minot-missile-field-3"),
        (Some("58701".into()), None)
    );
    assert_eq!(minot.finances.benefits, [Benefit::FederalPay]);
    assert_eq!(
        minot.dials.rare_families(),
        ["geomagnetic_storm", "nuclear_attack"]
    );
    assert!(minot.existing.iter().all(|o| o.tested_on.is_some()));

    // A leveed county (Natomas, Sacramento: a basin behind levees), no flood policy.
    let sac = get("sacramento-leveed-2");
    assert_eq!(county("sacramento-leveed-2").0.as_deref(), Some("95834"));
    assert!(!sac.finances.insurance.flood);
    assert_eq!(sac.finances.insurance.sewer_backup, Some(false));

    // A smoke county (Missoula, MT), no air conditioning, a river within reach, a senior.
    let missoula = get("missoula-smoke-2");
    assert_eq!(county("missoula-smoke-2").0.as_deref(), Some("59801"));
    assert_eq!(missoula.housing.cooling, Cooling::None);
    assert_eq!(
        missoula.housing.raw_water_source,
        Some(RawWaterSource::SurfaceNearby)
    );
    assert!(
        missoula
            .people
            .iter()
            .any(|p| p.age_band == AgeBand::Senior)
    );

    // A SNAP household (Detroit): $10 a month, bare-minimum mode, a basement bedroom, a hearing
    // need, no car, no alarms, and the one filled-in family plan.
    let detroit = get("detroit-snap-3");
    assert_eq!(county("detroit-snap-3").0.as_deref(), Some("48227"));
    assert_eq!(detroit.finances.benefits, [Benefit::SnapWic]);
    assert_eq!(detroit.finances.monthly_budget_usd, 10.0);
    assert!(detroit.dials.minimum_kit && detroit.housing.below_grade_bedroom);
    assert!(detroit.mobility.vehicles.is_empty());
    assert!(
        detroit
            .people
            .iter()
            .any(|p| p.access_needs == [AccessNeed::Hearing])
    );
    let plan = detroit.family_plan.as_ref().expect("the family plan");
    assert!(plan.meeting_place_near.is_some() && plan.out_of_area_contact.is_some());
    assert_eq!(plan.routes.len(), ROUTES_MAX);
    assert_eq!(plan.trusted_circle.len(), 3);
    assert!(
        plan.trusted_circle
            .iter()
            .any(|p| p.holds.contains(&Holds::MedicalPoa))
    );
    assert!(plan.lawyer.is_some() && !plan.numbers_by_heart.is_empty());
    assert!(plan.roadside_assistance.is_none(), "no car");

    // A high-rise household in a surge zone (Galveston island), alone, limited mobility.
    let galveston = get("galveston-highrise-1");
    assert_eq!(county("galveston-highrise-1").0.as_deref(), Some("77550"));
    assert_eq!(galveston.housing.kind, HousingKind::ApartmentHighRise);
    assert!(galveston.housing.floor >= 9 && galveston.people.len() == 1);
    assert_eq!(galveston.people[0].access_needs, [AccessNeed::HomeHealth]);
    assert_eq!(galveston.finances.benefits, [Benefit::SsiSsdi]);

    // Puerto Rico (San Juan): Spanish speakers, insulin, a cistern, frequent water problems.
    let san_juan = get("san-juan-2");
    assert_eq!(county("san-juan-2").0.as_deref(), Some("00907"));
    assert!(
        san_juan
            .people
            .iter()
            .all(|p| p.access_needs == [AccessNeed::LimitedEnglish])
    );
    assert!(san_juan.people.iter().any(|p| p.medical.refrigerated_rx));
    assert_eq!(
        san_juan.housing.water_system_record,
        Some(WaterSystemRecord::FrequentProblems)
    );
    assert!(san_juan.dials.long_horizon);

    // Together they use every contract v2 input.
    let v2: Vec<PlanInput> = [
        "minot-missile-field-3",
        "sacramento-leveed-2",
        "missoula-smoke-2",
        "detroit-snap-3",
        "galveston-highrise-1",
        "san-juan-2",
    ]
    .iter()
    .map(|n| get(n))
    .collect();
    let any = |f: &dyn Fn(&PlanInput) -> bool| v2.iter().any(f);
    assert!(any(&|i| i
        .people
        .iter()
        .any(|p| !p.access_needs.is_empty())));
    assert!(any(&|i| i.housing.below_grade_bedroom));
    assert!(any(&|i| i.housing.cooking == Some(CookingFuel::Gas)));
    assert!(any(&|i| i.housing.cooking == Some(CookingFuel::Electric)));
    assert!(any(&|i| i.housing.raw_water_source.is_some()));
    assert!(any(&|i| i.housing.water_system_record.is_some()));
    assert!(any(&|i| !i.finances.benefits.is_empty()));
    assert!(any(&|i| i.finances.insurance.sewer_backup.is_some()));
    assert!(any(&|i| i.finances.insurance.life_or_disability.is_some()));
    assert!(any(&|i| i.existing.iter().any(|o| o.tested_on.is_some())));
    assert!(any(&|i| !i.dials.rare_opt_in.is_empty()));
    assert!(any(&|i| i.dials.minimum_kit));
    assert!(any(&|i| i.dials.long_horizon));
    assert_eq!(v2.iter().filter(|i| i.family_plan.is_some()).count(), 1);
}

#[test]
fn fixture_dials_follow_the_contract() {
    for (name, input) in fixtures::all() {
        let want = match name {
            "coos-bay-well-owner-2" => ReturnPeriod::OneIn500,
            "sugar-land-ev-household-3" => ReturnPeriod::OneIn50,
            _ => ReturnPeriod::OneIn100,
        };
        assert_eq!(input.dials.return_period, want, "{name}");
        let climate = match name {
            "miami-condo-retiree-1" | "phoenix-apartment-cpap-1" | "sugar-land-ev-household-3" => {
                ClimateHorizon::Y2050
            }
            _ => ClimateHorizon::Today,
        };
        assert_eq!(input.dials.climate, climate, "{name}");
        assert_eq!(input.dials.horizon_years, 10, "{name}");
        assert_eq!(input.dials.water_level, WaterLevel::Basic, "{name}");
        assert!(input.dials.scenario_overrides.is_empty(), "{name}");
    }
}

#[test]
fn fixtures_exercise_what_the_readme_promises() {
    let get = |n: &str| fixtures::get(n).unwrap();
    let kansas = get("hays-kansas-farm-5");
    assert!(
        kansas.location.zip.is_none() && kansas.location.county_fips.as_deref() == Some("20051")
    );
    assert!(kansas.pets.large_animals > 0 && kansas.people.iter().any(|p| p.pregnant_or_nursing));
    assert!(kansas.people.iter().any(|p| p.medical.epinephrine));
    let phoenix = get("phoenix-apartment-cpap-1");
    assert!(
        phoenix
            .people
            .iter()
            .any(|p| p.medical.powered_device == PoweredDevice::Cpap)
    );
    let chicago = get("chicago-student-zero-budget-1");
    assert_eq!(chicago.finances.monthly_budget_usd, 0.0);
    assert!(chicago.mobility.vehicles.is_empty());
    let sugar_land = get("sugar-land-ev-household-3");
    assert!(
        sugar_land
            .mobility
            .vehicles
            .iter()
            .any(|v| v.fuel == Fuel::Ev)
    );
    assert!(
        sugar_land
            .people
            .iter()
            .any(|p| p.age_band == AgeBand::Infant)
    );
    assert!(sugar_land.people.iter().any(|p| p.medical.refrigerated_rx));
    let coos_bay = get("coos-bay-well-owner-2");
    assert_eq!(coos_bay.housing.water, WaterSource::Well);
    assert_eq!(coos_bay.housing.backup_power, BackupPower::Generator);
    let miami = get("miami-condo-retiree-1");
    assert_eq!(miami.housing.floor, 14);
    assert!(miami.people[0].medical.refrigerated_rx);
}
