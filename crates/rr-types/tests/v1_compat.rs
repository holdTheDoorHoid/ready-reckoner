//! Contract v2 keeps every v1 file readable (DESIGN-DELTA §1): the seven v0.1 fixture households
//! that the goldens were planned from, the other v1 households in the repository, and the golden
//! `PlanOutput` files themselves. Inputs parse unchanged and default every v2 field; outputs
//! round-trip byte for byte, because every v2 output field is left out when empty. A v2 output
//! never names a retired hazard.

mod common;

use std::path::{Path, PathBuf};

use common::repo_path;
use rr_types::*;
use serde_json::Value;

/// The seven fixture households of v0.1, whose outputs are the goldens in `fixtures/golden/`.
const V1_FIXTURES: [&str; 7] = [
    "chicago-student-zero-budget-1",
    "coos-bay-well-owner-2",
    "hays-kansas-farm-5",
    "miami-condo-retiree-1",
    "philadelphia-renters-4",
    "phoenix-apartment-cpap-1",
    "sugar-land-ev-household-3",
];

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    files
}

/// The household staged in `pending/` for v0.1.1, written against contract v1 (the other staged
/// households are the contract v2 fixtures).
const V1_PENDING: [&str; 1] = ["cameron-insulin-well-farm-2"];

/// Every v1 household in the repository: the seven v0.1 fixtures, the v1 one staged in
/// `pending/`, and rr-plan's backtest households.
fn v1_inputs() -> Vec<(String, String)> {
    let mut paths: Vec<PathBuf> = V1_FIXTURES
        .iter()
        .map(|n| repo_path(&format!("fixtures/households/{n}.json")))
        .collect();
    paths.extend(
        V1_PENDING
            .iter()
            .map(|n| repo_path(&format!("fixtures/households/pending/{n}.json"))),
    );
    paths.extend(json_files(&repo_path("crates/rr-plan/tests/data/backtest")));
    paths
        .into_iter()
        .map(|p| {
            let text =
                std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            (p.display().to_string(), text)
        })
        .collect()
}

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
fn every_v1_household_parses_unchanged_and_defaults_every_v2_field() {
    let inputs = v1_inputs();
    assert!(inputs.len() >= 7 + 1 + 5, "found {}", inputs.len());
    for (name, raw) in &inputs {
        let file: Value = serde_json::from_str(raw).unwrap();
        // Written before contract v2: none of its keys exist anywhere in the file.
        for key in [
            "family_plan",
            "access_needs",
            "below_grade_bedroom",
            "cooking",
            "raw_water_source",
            "water_system_record",
            "benefits",
            "sewer_backup",
            "life_or_disability",
            "tested_on",
            "rare_opt_in",
            "minimum_kit",
            "long_horizon",
        ] {
            assert!(
                !raw.contains(&format!("\"{key}\"")),
                "{name} already has {key}"
            );
        }

        let input: PlanInput =
            serde_json::from_str(raw).unwrap_or_else(|e| panic!("{name} does not parse: {e}"));
        assert_eq!(input.validate(), vec![], "{name}");
        assert_eq!(PlanInput::from_json(raw).unwrap(), input, "{name}");

        // Every v2 field takes its default.
        assert!(input.family_plan.is_none(), "{name}");
        for p in &input.people {
            assert!(p.access_needs.is_empty(), "{name}");
        }
        let h = &input.housing;
        assert!(!h.below_grade_bedroom, "{name}");
        assert!(
            h.cooking.is_none() && h.raw_water_source.is_none(),
            "{name}"
        );
        assert!(h.water_system_record.is_none(), "{name}");
        assert!(input.finances.benefits.is_empty(), "{name}");
        let ins = input.finances.insurance;
        assert!(ins.sewer_backup.is_none() && ins.life_or_disability.is_none());
        assert!(input.existing.iter().all(|o| o.tested_on.is_none()));
        let d = &input.dials;
        assert!(d.rare_opt_in.is_empty() && !d.minimum_kit && !d.long_horizon);
        // The v1 switch carries straight over: on means every family, off means none.
        let families = d.rare_families();
        if d.rare_catastrophic_opt_in {
            assert_eq!(families.len(), HazardId::RARE.len(), "{name}");
        } else {
            assert!(families.is_empty(), "{name}");
        }

        // Nothing is lost or invented: re-serialised, only the v2 defaults are new.
        let mut canon = serde_json::to_value(&input).unwrap();
        let dials = canon["dials"].as_object_mut().unwrap();
        for key in ["rare_opt_in", "minimum_kit", "long_horizon"] {
            assert!(dials.remove(key).is_some(), "{name}: {key} is written out");
        }
        for key in [
            "water_level",
            "scenario_overrides",
            "rare_catastrophic_opt_in",
        ] {
            if file["dials"].get(key).is_none() {
                dials.remove(key);
            }
        }
        if file.get("assume_basics").is_none() {
            canon.as_object_mut().unwrap().remove("assume_basics");
        }
        canon["housing"]
            .as_object_mut()
            .unwrap()
            .remove("below_grade_bedroom");
        canon["finances"]
            .as_object_mut()
            .unwrap()
            .remove("benefits");
        for p in canon["people"].as_array_mut().unwrap() {
            p.as_object_mut().unwrap().remove("access_needs");
        }
        assert!(
            same_json(&file, &canon),
            "{name}: the v2 reading differs from the file"
        );
    }
}

/// The golden outputs (`fixtures/golden/*.json`), by fixture name.
fn goldens() -> Vec<(String, String)> {
    json_files(&repo_path("fixtures/golden"))
        .into_iter()
        .map(|p| {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect()
}

#[test]
fn every_golden_output_round_trips_byte_for_byte() {
    let goldens = goldens();
    assert!(goldens.len() >= V1_FIXTURES.len());
    for name in V1_FIXTURES {
        assert!(
            goldens.iter().any(|(n, _)| n == name),
            "no golden for {name}"
        );
    }
    for (name, raw) in &goldens {
        let out: PlanOutput = serde_json::from_str(raw)
            .unwrap_or_else(|e| panic!("golden {name} does not parse: {e}"));
        // The bytes rr-plan writes (`rr_plan::to_json`): pretty JSON and a newline.
        let again = serde_json::to_string_pretty(&out).unwrap() + "\n";
        assert!(
            again == *raw,
            "golden {name} changes when read and written again"
        );
        if out.api_version == 1 {
            // A v1 output holds none of the v2 fields, so it reads back with them empty.
            assert!(out.location.exposure.is_empty(), "{name}");
            assert!(out.recovery.is_empty(), "{name}");
            assert!(out.buckets.iter().all(|b| b.stress_test.is_none()));
            assert!(out.register.iter().all(|h| h.family.is_none()
                && h.sub_causes.is_empty()
                && h.location_factor.is_none()
                && !h.range_only
                && h.anchor_sentence.is_none()
                && h.if_it_reaches_you.is_none()
                && h.what_it_changes.is_none()));
            let p = &out.plan;
            assert!(p.first_milestone.is_none() && !p.minimum_kit && p.long_horizon.is_empty());
            let items = p.months.iter().flat_map(|m| &m.items);
            assert!(items.clone().all(|i| i.requires.is_empty() && !i.decision));
        }
    }
}

/// Every hazard id a plan output mentions: the register, the bucket contributions and the plan's
/// items.
fn hazards_named(out: &PlanOutput) -> Vec<HazardId> {
    let mut ids: Vec<HazardId> = out.register.iter().map(|h| h.id).collect();
    for b in &out.buckets {
        ids.extend(b.contributions.iter().map(|c| c.hazard));
    }
    let items = out
        .plan
        .months
        .iter()
        .flat_map(|m| &m.items)
        .chain(&out.plan.long_horizon);
    for item in items {
        ids.extend(item.hazards.iter().copied());
    }
    ids
}

/// DESIGN-DELTA §1: `terrorism` is retired and never emitted. The goldens are the engine's
/// recorded output for every fixture (rr-plan's golden test keeps them equal to what the engine
/// prints), so once they are regenerated under contract v2 this checks the whole pipeline. The
/// v1 goldens are records of the old contract: they may name it, and must still parse.
#[test]
fn no_v2_output_names_a_retired_hazard() {
    for (name, raw) in goldens() {
        let out: PlanOutput = serde_json::from_str(&raw).unwrap();
        if out.api_version < 2 {
            continue;
        }
        let retired: Vec<HazardId> = hazards_named(&out)
            .into_iter()
            .filter(|h| h.is_retired())
            .collect();
        assert!(retired.is_empty(), "golden {name} names {retired:?}");
        for h in &out.register {
            assert!(
                h.family.as_deref() == h.id.family(),
                "golden {name}: {} carries family {:?}",
                h.id,
                h.family
            );
        }
    }
    // The same check on a hand-built v2 output: a retired id is caught wherever it appears.
    #[allow(deprecated)]
    let retired = HazardId::Terrorism;
    let mut out = common::samples::plan_output();
    assert!(hazards_named(&out).iter().all(|h| !h.is_retired()));
    out.buckets[0].contributions[0].hazard = retired;
    assert!(hazards_named(&out).contains(&retired));
}
