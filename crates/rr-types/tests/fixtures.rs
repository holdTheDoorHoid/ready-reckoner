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
}

#[test]
fn every_fixture_parses_round_trips_and_validates() {
    for (name, raw) in fixtures::RAW {
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

        // Nothing is lost or invented: the canonical form equals the file, apart from the dial
        // fields that default when absent.
        let file: Value = serde_json::from_str(raw).unwrap();
        let mut canon: Value = serde_json::from_str(&canonical).unwrap();
        for key in [
            "water_level",
            "scenario_overrides",
            "rare_catastrophic_opt_in",
        ] {
            if file["dials"].get(key).is_none() {
                canon["dials"].as_object_mut().unwrap().remove(key);
            }
        }
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
    assert!(fixtures::get("no-such-household").is_none());
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
