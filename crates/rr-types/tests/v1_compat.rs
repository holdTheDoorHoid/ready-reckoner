//! Contract v2 kept every v1 file readable (DESIGN-DELTA §1), and contract v3 keeps every v1 and
//! v2 input readable (DESIGN-DELTA-v3 §3): the seven v0.1 fixture households that the goldens were
//! planned from, the other v1 households in the repository, and the v2 households. Inputs parse
//! unchanged and default every newer field. Outputs are recomputed on load, and contract v3
//! removed `packet_markdown` (the binder replaces it), so a golden `PlanOutput` of the current
//! contract round-trips byte for byte and an older one is only checked for retired ids. No output
//! names a retired hazard.

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

/// The v0.1.1 household, written against contract v1 (the other households added in v0.2.0 are
/// the contract v2 fixtures).
const V1_ROUND2: [&str; 1] = ["cameron-insulin-well-farm-2"];

/// Fixture households that gained contract v3 sample answers in v0.3.0 (DESIGN-DELTA-v3 §10):
/// people's `profile` and a `family_plan` holding only the v3 groups. Their older fields are
/// untouched, so with those additions taken out each is still the household it was.
const V3_SAMPLES: [&str; 2] = ["philadelphia-renters-4", "minot-missile-field-3"];

/// The family-plan groups contract v3 adds.
const V3_FAMILY_GROUPS: [&str; 5] = ["home", "neighbourhood", "pets", "vehicles", "documents"];

/// `raw` without its contract v3 additions: each person's `profile`, the v3 family-plan groups,
/// and the family plan itself when nothing older is left in it.
fn without_v3_answers(raw: &str) -> String {
    let mut v: Value = serde_json::from_str(raw).unwrap();
    for p in v["people"].as_array_mut().unwrap() {
        p.as_object_mut().unwrap().remove("profile");
    }
    let root = v.as_object_mut().unwrap();
    if let Some(plan) = root.get_mut("family_plan").and_then(Value::as_object_mut) {
        for key in V3_FAMILY_GROUPS {
            plan.remove(key);
        }
        if plan.is_empty() {
            root.remove("family_plan");
        }
    }
    serde_json::to_string(&v).unwrap()
}

#[test]
fn the_v3_samples_differ_from_their_older_selves_only_by_v3_answers() {
    for name in V3_SAMPLES {
        let raw = std::fs::read_to_string(repo_path(&format!("fixtures/households/{name}.json")))
            .unwrap();
        let input = PlanInput::from_json(&raw).unwrap();
        assert!(
            input.people.iter().any(|p| p.profile.is_some()),
            "{name} has no profile"
        );
        let plan = input.family_plan.as_ref().expect("a family plan");
        assert!(plan.home.is_some(), "{name} has no home");
        let older = PlanInput::from_json(&without_v3_answers(&raw)).unwrap();
        let mut stripped = input.clone();
        for p in &mut stripped.people {
            p.profile = None;
        }
        if let Some(plan) = stripped.family_plan.as_mut() {
            plan.home = None;
            plan.neighbourhood = None;
            plan.pets.clear();
            plan.vehicles.clear();
            plan.documents = None;
        }
        stripped.tidy();
        assert_eq!(stripped, older, "{name}");
    }
}

/// Every v1 household in the repository: the seven v0.1 fixtures, the v0.1.1 one, and rr-plan's
/// backtest households.
fn v1_inputs() -> Vec<(String, String)> {
    let mut paths: Vec<PathBuf> = V1_FIXTURES
        .iter()
        .map(|n| repo_path(&format!("fixtures/households/{n}.json")))
        .collect();
    paths.extend(
        V1_ROUND2
            .iter()
            .map(|n| repo_path(&format!("fixtures/households/{n}.json"))),
    );
    paths.extend(json_files(&repo_path("crates/rr-plan/tests/data/backtest")));
    paths
        .into_iter()
        .map(|p| {
            let text =
                std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let stem = p.file_stem().unwrap().to_string_lossy();
            let text = if V3_SAMPLES.contains(&stem.as_ref()) {
                without_v3_answers(&text)
            } else {
                text
            };
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

/// The six households written against contract v2 (v0.2.0).
const V2_FIXTURES: [&str; 6] = [
    "detroit-snap-3",
    "galveston-highrise-1",
    "minot-missile-field-3",
    "missoula-smoke-2",
    "sacramento-leveed-2",
    "san-juan-2",
];

#[test]
fn every_v2_household_parses_unchanged_and_gains_no_v3_field() {
    for name in V2_FIXTURES {
        let raw = std::fs::read_to_string(repo_path(&format!("fixtures/households/{name}.json")))
            .unwrap();
        // Minot carries v3 sample answers since v0.3.0; without them it is the v2 file.
        let raw = if V3_SAMPLES.contains(&name) {
            without_v3_answers(&raw)
        } else {
            raw
        };
        // As keys (`"vehicles":` is also the v1 `mobility` field, checked below on the plan).
        for key in ["profile", "home", "neighbourhood", "documents", "address"] {
            assert!(
                !raw.contains(&format!("\"{key}\":")),
                "{name} already has {key}"
            );
        }
        let input =
            PlanInput::from_json(&raw).unwrap_or_else(|e| panic!("{name} does not parse: {e}"));
        assert!(input.people.iter().all(|p| p.profile.is_none()), "{name}");
        if let Some(plan) = &input.family_plan {
            assert!(
                plan.home.is_none() && plan.neighbourhood.is_none(),
                "{name}"
            );
            assert!(plan.pets.is_empty() && plan.vehicles.is_empty(), "{name}");
            assert!(plan.documents.is_none(), "{name}");
        }
        // Read and written again, nothing is invented: the JSON holds no v3 key.
        let again = serde_json::to_string(&input).unwrap();
        for key in ["profile", "home", "neighbourhood", "documents", "address"] {
            assert!(
                !again.contains(&format!("\"{key}\":")),
                "{name} gained {key}"
            );
        }
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

/// The contract a golden output was written under (`api_version`).
fn api_version(raw: &str) -> u64 {
    let v: Value = serde_json::from_str(raw).unwrap();
    v["api_version"].as_u64().expect("an api_version")
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
        if api_version(raw) < u64::from(ENGINE_API_VERSION) {
            // Written under an older contract: contract v3 removed its `packet_markdown` (the
            // binder and the Prepare sheet replace it), so it no longer parses. rr-plan's golden
            // test fails until the planner regenerates it.
            let v: Value = serde_json::from_str(raw).unwrap();
            assert!(v.get("packet_markdown").is_some(), "{name}");
            assert!(serde_json::from_str::<PlanOutput>(raw).is_err(), "{name}");
            continue;
        }
        let out: PlanOutput = serde_json::from_str(raw)
            .unwrap_or_else(|e| panic!("golden {name} does not parse: {e}"));
        // The bytes rr-plan writes (`rr_plan::to_json`): pretty JSON and a newline.
        let again = serde_json::to_string_pretty(&out).unwrap() + "\n";
        assert!(
            again == *raw,
            "golden {name} changes when read and written again"
        );
        assert_eq!(out.binder.check(), Vec::<String>::new(), "golden {name}");
    }
}

/// Every hazard id a plan output mentions: the register, the bucket contributions and the plan's
/// items. Read from the JSON, so it works on an output of any contract.
fn hazards_named_in(out: &Value) -> Vec<HazardId> {
    let id = |v: &Value| -> HazardId { v.as_str().unwrap().parse().unwrap() };
    let mut ids: Vec<HazardId> = out["register"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| id(&h["id"]))
        .collect();
    for b in out["buckets"].as_array().unwrap() {
        ids.extend(
            b["contributions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| id(&c["hazard"])),
        );
    }
    let months = out["plan"]["months"].as_array().unwrap();
    let items = months
        .iter()
        .flat_map(|m| m["items"].as_array().unwrap())
        .chain(out["plan"]["long_horizon"].as_array().into_iter().flatten());
    for item in items {
        ids.extend(item["hazards"].as_array().unwrap().iter().map(id));
    }
    ids
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
        let out: Value = serde_json::from_str(&raw).unwrap();
        if api_version(&raw) < 2 {
            continue;
        }
        let retired: Vec<HazardId> = hazards_named_in(&out)
            .into_iter()
            .filter(|h| h.is_retired())
            .collect();
        assert!(retired.is_empty(), "golden {name} names {retired:?}");
        for h in out["register"].as_array().unwrap() {
            let id: HazardId = h["id"].as_str().unwrap().parse().unwrap();
            assert!(
                h["family"].as_str() == id.family(),
                "golden {name}: {id} carries family {:?}",
                h["family"]
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
