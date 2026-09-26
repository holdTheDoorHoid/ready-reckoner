//! `web/src/engine/types.ts` mirrors the Rust types by hand. These tests hold the two together:
//! every TypeScript interface is checked against serialised Rust samples (same fields, same
//! optional markers, values of the declared types, enum values from the declared lists), every
//! id list equals the Rust enum, and the contract version agrees in Rust, TypeScript and the docs.
//!
//! When you add a type or field, add it to `tests/common/samples.rs` and to `types.ts`; the tests
//! say what is missing.

mod common;

use std::collections::BTreeSet;

use common::{read_repo_file, samples, ts};
use rr_types::*;
use serde::de::DeserializeOwned;
use serde_json::Value;

fn types_ts() -> ts::TsFile {
    ts::parse_file(&read_repo_file("web/src/engine/types.ts"))
}

fn parses<T: DeserializeOwned>(v: &Value) -> bool {
    serde_json::from_value::<T>(v.clone()).is_ok()
}

fn json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn every_ts_interface_matches_the_rust_types() {
    let file = types_ts();
    let mut c = ts::Checker::new(&file);

    c.check_root(
        "PlanInput",
        &json(&samples::plan_input()),
        &parses::<PlanInput>,
    );
    c.check_root(
        "PlanInput",
        &json(&PlanInput::defaults()),
        &parses::<PlanInput>,
    );
    for (_, input) in fixtures::all() {
        c.check_root("PlanInput", &json(&input), &parses::<PlanInput>);
    }
    c.check_root(
        "PlanOutput",
        &json(&samples::plan_output()),
        &parses::<PlanOutput>,
    );
    c.check_root(
        "Catalogue",
        &json(&samples::catalogue()),
        &parses::<Catalogue>,
    );
    c.check_root(
        "EngineInfo",
        &json(&samples::engine_info()),
        &parses::<EngineInfo>,
    );
    c.check_root(
        "PackInfo",
        &json(&samples::pack_info()),
        &parses::<PackInfo>,
    );
    c.check_root(
        "ExplainRequest",
        &json(&samples::explain_request()),
        &parses::<ExplainRequest>,
    );
    c.check_root(
        "Explanation",
        &json(&samples::explanation()),
        &parses::<Explanation>,
    );
    c.check_root(
        "EngineError",
        &json(&samples::engine_error()),
        &parses::<EngineError>,
    );
    c.check_root(
        "BadInputDetails",
        &json(&samples::bad_input_details()),
        &parses::<BadInputDetails>,
    );
    c.check_root(
        "LocationSuggestions",
        &json(&samples::location_suggestions()),
        &parses::<LocationSuggestions>,
    );
    c.check_root(
        "Effect",
        &json(&samples::effect(samples::log_normal())),
        &parses::<Effect>,
    );
    c.check_root(
        "Effect",
        &json(&samples::effect(samples::fixed())),
        &parses::<Effect>,
    );
    c.check_root(
        "Envelope",
        &json(&Envelope::Value(samples::pack_info())),
        &parses::<Envelope<PackInfo>>,
    );
    c.check_root(
        "Envelope",
        &json(&Envelope::<PackInfo>::Error(samples::engine_error())),
        &parses::<Envelope<PackInfo>>,
    );

    assert!(
        c.errors.is_empty(),
        "types.ts disagrees with rr-types:\n{}",
        c.errors.join("\n")
    );

    let unchecked: Vec<&String> = file
        .interfaces
        .keys()
        .filter(|k| !c.visited.contains(*k))
        .collect();
    assert!(
        unchecked.is_empty(),
        "interfaces in types.ts that no Rust sample reaches: {unchecked:?}"
    );
    for (name, fields) in &file.interfaces {
        let seen = c.seen.get(name).cloned().unwrap_or_default();
        let missing: Vec<&String> = fields
            .iter()
            .map(|f| &f.name)
            .filter(|f| !seen.contains(*f))
            .collect();
        assert!(
            missing.is_empty(),
            "{name}: no sample fills in {missing:?}; extend samples.rs"
        );
    }
}

#[test]
fn every_ts_id_list_matches_the_rust_enum() {
    let file = types_ts();
    let lists: &[(&str, &[&str])] = &[
        ("HAZARD_TIERS", HazardTier::STRS),
        ("HAZARD_IDS", HazardId::STRS),
        ("TARGET_KINDS", TargetKind::STRS),
        ("BUCKET_KINDS", BucketKind::STRS),
        ("BUCKET_IDS", BucketId::STRS),
        ("TIER_IDS", TierId::STRS),
        ("SETTINGS", Setting::STRS),
        ("HOUSING_KINDS", HousingKind::STRS),
        ("TENURES", Tenure::STRS),
        ("WATER_SOURCES", WaterSource::STRS),
        ("WASTEWATER_KINDS", Wastewater::STRS),
        ("HEATING_KINDS", Heating::STRS),
        ("COOLING_KINDS", Cooling::STRS),
        ("BACKUP_POWER_KINDS", BackupPower::STRS),
        ("AGE_BANDS", AgeBand::STRS),
        ("MOBILITY_LEVELS", Mobility::STRS),
        ("COMMUTE_MODES", CommuteMode::STRS),
        ("FUELS", Fuel::STRS),
        ("INCOME_STABILITIES", IncomeStability::STRS),
        ("RETURN_PERIODS", ReturnPeriod::STRS),
        ("CLIMATE_HORIZONS", ClimateHorizon::STRS),
        ("WATER_LEVELS", WaterLevel::STRS),
        ("STAGES", Stage::STRS),
        ("PROBLEM_CODES", ProblemCode::STRS),
        ("DATA_CONFIDENCE_LEVELS", DataConfidence::STRS),
        ("HAZARD_DISPLAYS", HazardDisplay::STRS),
        ("PER_VALUES", Per::STRS),
        ("PLAN_ITEM_KINDS", PlanItemKind::STRS),
        ("WARNING_SEVERITIES", WarningSeverity::STRS),
        ("EVIDENCE_KINDS", EvidenceKinds::STRS),
        ("ERROR_CODES", ErrorCode::STRS),
        ("EXPLAIN_KINDS", ExplainKind::STRS),
    ];
    let strings = |name: &str| -> Vec<String> {
        file.arrays
            .get(name)
            .unwrap_or_else(|| panic!("types.ts has no {name}"))
            .iter()
            .map(|v| {
                v.as_str()
                    .unwrap_or_else(|| panic!("{name} holds a non-string"))
                    .to_owned()
            })
            .collect()
    };
    for (name, strs) in lists {
        assert_eq!(
            strings(name),
            *strs,
            "types.ts {name} differs from the Rust enum"
        );
    }

    let simple: Vec<String> = [
        PoweredDevice::None,
        PoweredDevice::Cpap,
        PoweredDevice::Oxygen,
    ]
    .iter()
    .map(|d| json(d).as_str().unwrap().to_owned())
    .collect();
    assert_eq!(strings("SIMPLE_POWERED_DEVICES"), simple);

    let ladder: Vec<f64> = file.arrays["TARGET_LADDER_DAYS"]
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let rust_ladder: Vec<f64> = TARGET_LADDER_DAYS.iter().map(|&d| f64::from(d)).collect();
    assert_eq!(ladder, rust_ladder, "TARGET_LADDER_DAYS differs");

    let mut covered: BTreeSet<&str> = lists.iter().map(|(n, _)| *n).collect();
    covered.extend(["SIMPLE_POWERED_DEVICES", "TARGET_LADDER_DAYS"]);
    let unchecked: Vec<&String> = file
        .arrays
        .keys()
        .filter(|k| !covered.contains(k.as_str()))
        .collect();
    assert!(
        unchecked.is_empty(),
        "add these types.ts lists to this test: {unchecked:?}"
    );
}

/// Keeps the test's name list honest: `Evidence` is the Rust enum behind `EVIDENCE_KINDS`.
type EvidenceKinds = Evidence;

#[test]
fn contract_version_agrees_everywhere() {
    let file = types_ts();
    assert_eq!(
        file.numbers.get("ENGINE_API_VERSION"),
        Some(&f64::from(ENGINE_API_VERSION))
    );
    let doc = read_repo_file("docs/ENGINE-API.md");
    assert!(
        doc.contains(&format!("`ENGINE_API_VERSION = {ENGINE_API_VERSION}`")),
        "docs/ENGINE-API.md must state the version"
    );
}

#[test]
fn the_docs_and_the_engine_interface_name_every_function_and_error_code() {
    let doc = read_repo_file("docs/ENGINE-API.md");
    let index = read_repo_file("web/src/engine/index.ts");
    for f in [
        "engine_info",
        "load_pack",
        "county_search",
        "resolve_location",
        "assess",
        "explain",
        "catalogue",
        "defaults",
    ] {
        assert!(
            doc.contains(&format!("`{f}(")),
            "docs/ENGINE-API.md does not list {f}"
        );
        assert_eq!(
            index.matches(&format!("  {f}(")).count(),
            2,
            "index.ts Engine and RawEngine must both declare {f}"
        );
    }
    for code in ErrorCode::STRS {
        assert!(
            doc.contains(&format!("`{code}`")),
            "docs/ENGINE-API.md does not list error code {code}"
        );
    }
}
