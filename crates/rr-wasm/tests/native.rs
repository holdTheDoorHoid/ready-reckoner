//! The string-level engine run natively: the envelopes every export returns, the built-in sample
//! counties the goldens come from, and the real data packs in `data/`.
//!
//! Each test runs on its own thread and so has its own engine (the engine is thread-local), which
//! lets one test load packs without changing what another sees.
#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};

use rr_types::{
    Catalogue, EngineError, EngineInfo, Envelope, ErrorCode, Explanation, LocationResolved,
    PackInfo, PlanInput, PlanOutput, ProblemCode,
};
use rr_wasm::api;
use serde::de::DeserializeOwned;

mod common;
use common::{minify, value_text};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> Vec<u8> {
    std::fs::read(repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn fixture_json(name: &str) -> String {
    String::from_utf8(read(&format!("fixtures/households/{name}.json"))).unwrap()
}

/// The value of an `ok` envelope, checking the envelope's exact JSON shape on the way.
fn value<T: DeserializeOwned>(json: &str) -> T {
    assert!(
        json.starts_with(r#"{"ok":true,"value":"#),
        "not an ok envelope: {json:.300}"
    );
    match serde_json::from_str::<Envelope<T>>(json).expect("envelope parses") {
        Envelope::Value(v) => v,
        Envelope::Error(e) => panic!("unexpected error {e}"),
    }
}

/// The error of an `ok: false` envelope.
fn error(json: &str) -> EngineError {
    assert!(
        json.starts_with(r#"{"ok":false,"error":{"code":"#),
        "not an error envelope: {json:.300}"
    );
    match serde_json::from_str::<Envelope<serde_json::Value>>(json).expect("envelope parses") {
        Envelope::Error(e) => e,
        Envelope::Value(_) => unreachable!(),
    }
}

fn with_location(name: &str, location: serde_json::Value) -> String {
    let mut input: serde_json::Value = serde_json::from_str(&fixture_json(name)).unwrap();
    input["location"] = location;
    input.to_string()
}

// ---------------------------------------------------------------------------------------------
// Before any pack: the seven built-in sample counties
// ---------------------------------------------------------------------------------------------

#[test]
fn before_any_pack_the_sample_counties_answer_and_engine_info_says_so() {
    let info: EngineInfo = value(&api::engine_info());
    assert_eq!(info.api_version, rr_types::ENGINE_API_VERSION);
    assert_eq!(info.engine_version, rr_plan::ENGINE_VERSION);
    assert!(info.packs_loaded.is_empty(), "{:?}", info.packs_loaded);
    assert_eq!(
        info.data_pack_version, None,
        "absent until a pack is loaded"
    );
    // The FEMA National Risk Index statement comes first and carries version and access date.
    let nri = &info.attributions[0];
    assert_eq!(nri.source, "FEMA National Risk Index");
    assert!(nri.text.contains("not endorsed by FEMA"), "{}", nri.text);
    assert!(nri.version.is_some());
    // The raw JSON leaves data_pack_version out rather than writing null.
    assert!(!api::engine_info().contains("data_pack_version"));
}

#[test]
fn every_fixture_matches_its_golden_byte_for_byte() {
    for (name, _) in rr_types::fixtures::RAW {
        let envelope = api::assess(&fixture_json(name));
        let golden = String::from_utf8(read(&format!("fixtures/golden/{name}.json"))).unwrap();
        let ours = value_text(&envelope);
        if ours != minify(&golden) {
            // Show where, using the pretty form of both.
            let pretty: serde_json::Value = serde_json::from_str(ours).unwrap();
            let pretty = rr_plan::to_json(&pretty);
            panic!(
                "{name}: the engine's answer differs from fixtures/golden/{name}.json:\n{}",
                rr_plan::golden::diff(&golden, &pretty)
            );
        }
        let output: PlanOutput = value(&envelope);
        assert!(output.data_pack_version.starts_with("fixtures+"));
    }
}

#[test]
fn a_place_outside_the_sample_counties_is_an_unknown_location_with_suggestions() {
    let los_angeles = with_location(
        "philadelphia-renters-4",
        serde_json::json!({ "country": "US", "county_fips": "06037", "setting": "urban" }),
    );
    let e = error(&api::assess(&los_angeles));
    assert_eq!(e.code, ErrorCode::UnknownCounty);
    assert!(
        e.suggestions().is_some(),
        "details carry a suggestions list"
    );

    let beverly_hills = with_location(
        "philadelphia-renters-4",
        serde_json::json!({ "country": "US", "zip": "90210", "setting": "urban" }),
    );
    let e = error(&api::assess(&beverly_hills));
    assert_eq!(e.code, ErrorCode::UnknownZip);
    assert!(e.suggestions().is_some());

    // The defaults' placeholder ZIP code is well formed but not real.
    let defaults: PlanInput = value(&api::defaults());
    let e = error(&api::assess(&serde_json::to_string(&defaults).unwrap()));
    assert_eq!(e.code, ErrorCode::UnknownZip);

    // resolve_location answers the same errors as assess.
    let e = error(&api::resolve_location(
        r#"{"country":"US","county_fips":"06037","setting":"urban"}"#,
    ));
    assert_eq!(e.code, ErrorCode::UnknownCounty);
}

#[test]
fn resolve_location_finds_the_sample_counties_by_zip_and_by_county() {
    let by_zip: LocationResolved = value(&api::resolve_location(
        r#"{"country":"US","zip":"19147","setting":"urban"}"#,
    ));
    assert_eq!(by_zip.county_fips, "42101");
    assert_eq!(by_zip.zip.as_deref(), Some("19147"));
    let by_county: LocationResolved = value(&api::resolve_location(
        r#"{"country":"US","county_fips":"41011","setting":"rural"}"#,
    ));
    assert_eq!(by_county.county_name, "Coos");
    assert!(by_county.data_note.unwrap().contains("sample counties"));
}

#[test]
fn bad_json_and_invalid_answers_are_bad_input_with_problems() {
    for (call, json) in [
        ("assess", api::assess("not json")),
        ("assess", api::assess("{}")),
        ("resolve_location", api::resolve_location("[]")),
        ("explain", api::explain(r#"{"kind":"nonsense","id":"x"}"#)),
    ] {
        let e = error(&json);
        assert_eq!(e.code, ErrorCode::BadInput, "{call}");
        assert_eq!(e.problems().unwrap()[0].code, ProblemCode::Schema, "{call}");
    }

    let mut input: serde_json::Value =
        serde_json::from_str(&fixture_json("philadelphia-renters-4")).unwrap();
    input["people"] = serde_json::json!([]);
    input["finances"]["income"]["earners"] = serde_json::json!(0);
    let e = error(&api::assess(&input.to_string()));
    assert_eq!(e.code, ErrorCode::BadInput);
    let problems = e.problems().unwrap();
    assert_eq!(problems[0].code, ProblemCode::NoPeople);
    assert_eq!(problems[0].field, "people");
}

#[test]
fn resolve_location_checks_a_location_exactly_as_assess_does() {
    let e = error(&api::resolve_location(
        r#"{"country":"CA","zip":"1914","setting":"urban"}"#,
    ));
    let problems = e.problems().unwrap();
    let codes: Vec<(ProblemCode, &str)> = problems
        .iter()
        .map(|p| (p.code, p.field.as_str()))
        .collect();
    assert_eq!(
        codes,
        [
            (ProblemCode::UnsupportedCountry, "location.country"),
            (ProblemCode::ZipFormat, "location.zip"),
        ]
    );
    // The same location inside a whole input gives the same problems and messages.
    let whole = with_location(
        "philadelphia-renters-4",
        serde_json::json!({ "country": "CA", "zip": "1914", "setting": "urban" }),
    );
    assert_eq!(error(&api::assess(&whole)).problems().unwrap(), problems);

    let e = error(&api::resolve_location(
        r#"{"country":"US","setting":"urban"}"#,
    ));
    assert_eq!(e.problems().unwrap()[0].code, ProblemCode::LocationMissing);
}

#[test]
fn county_search_catalogue_defaults_and_explain_answer() {
    let found: Vec<LocationResolved> = value(&api::county_search("phila"));
    assert_eq!(found[0].county_fips, "42101");
    let found: Vec<LocationResolved> = value(&api::county_search("Cook, IL"));
    assert_eq!(found[0].county_fips, "17031");
    let none: Vec<LocationResolved> = value(&api::county_search("   "));
    assert!(none.is_empty());

    let catalogue: Catalogue = value(&api::catalogue());
    assert!(catalogue.items.len() > 50 && catalogue.citations.len() > 50);
    assert_eq!(catalogue.buckets.len(), 14);
    assert_eq!(catalogue.hazards.len(), 35);

    let defaults: PlanInput = value(&api::defaults());
    assert!(defaults.validate().is_empty());
    assert_eq!(defaults.location.zip.as_deref(), Some("00000"));

    let input = fixture_json("philadelphia-renters-4");
    let output: PlanOutput = value(&api::assess(&input));
    let parsed: serde_json::Value = serde_json::from_str(&input).unwrap();
    let mut asked = vec![
        ("hazard", output.register[0].id.as_str().to_owned()),
        ("bucket", "power".to_owned()),
        ("item", output.plan.months[1].items[0].item_id.to_string()),
        ("requirement", output.requirements[0].id.clone()),
    ];
    if let Some(w) = output.warnings.first() {
        asked.push(("warning", w.id.clone()));
    }
    for (kind, id) in asked {
        let request = serde_json::json!({ "kind": kind, "id": id, "input": parsed });
        let explanation: Explanation = value(&api::explain(&request.to_string()));
        assert!(!explanation.title.is_empty(), "{kind} {id}");
        assert!(!explanation.plain.is_empty(), "{kind} {id}");
    }
    let request = serde_json::json!({ "kind": "hazard", "id": "no_such_hazard", "input": parsed });
    assert_eq!(
        error(&api::explain(&request.to_string())).code,
        ErrorCode::BadInput
    );
}

#[test]
fn a_rejected_pack_file_leaves_the_sample_counties_answering() {
    let e = error(&api::load_pack("core/secrets.csv", b"a,b\n1,2\n"));
    assert_eq!(e.code, ErrorCode::BadInput);
    let problems = e.problems().expect("bad_input carries problems");
    assert_eq!(problems[0].field, "name");
    let e = error(&api::load_pack("  ", b"x"));
    assert_eq!(e.problems().unwrap()[0].field, "name");
    // Nothing was loaded, so nothing changed.
    let info: EngineInfo = value(&api::engine_info());
    assert!(info.packs_loaded.is_empty());
    let output: PlanOutput = value(&api::assess(&fixture_json("coos-bay-well-owner-2")));
    assert!(output.data_pack_version.starts_with("fixtures+"));
}

// ---------------------------------------------------------------------------------------------
// The data packs in data/
// ---------------------------------------------------------------------------------------------

/// Paths of the core pack's files, as the manifest lists them.
fn core_files(manifest: &serde_json::Value) -> Vec<String> {
    manifest["packs"]["core"]["files"]
        .as_array()
        .expect("core files")
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_data_packs_load_file_by_file_and_then_answer_for_every_fixture() {
    let manifest_bytes = read("data/manifest.json");
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let pack_version = manifest["pack_version"].as_str().unwrap().to_owned();

    let info: PackInfo = value(&api::load_pack("manifest.json", &manifest_bytes));
    assert_eq!(info.version, pack_version);

    // Part-way through the core pack the engine does not plan from half the data.
    let mut files = core_files(&manifest);
    files.retain(|f| f != "core/counties.csv");
    files.push("core/counties.csv".to_owned());
    let (first, rest) = files.split_at(3);
    for f in first {
        let info: PackInfo = value(&api::load_pack(f, &read(&format!("data/{f}"))));
        assert_eq!(info.name, *f);
    }
    let e = error(&api::assess(&fixture_json("philadelphia-renters-4")));
    assert_eq!(e.code, ErrorCode::PackMissing);
    assert!(e.message.contains("3 of"), "{}", e.message);
    assert_eq!(
        error(&api::county_search("phila")).code,
        ErrorCode::PackMissing
    );
    let info: EngineInfo = value(&api::engine_info());
    assert_eq!(
        info.data_pack_version.as_deref(),
        Some(pack_version.as_str())
    );
    assert!(info.packs_loaded.contains(&"manifest.json".to_owned()));
    assert!(
        info.packs_loaded.contains(&first[0]),
        "{:?}",
        info.packs_loaded
    );

    for f in rest {
        let _: PackInfo = value(&api::load_pack(f, &read(&format!("data/{f}"))));
    }
    let info: EngineInfo = value(&api::engine_info());
    assert_eq!(info.packs_loaded, ["core"]);
    assert_eq!(
        info.data_pack_version.as_deref(),
        Some(pack_version.as_str())
    );
    // Attributions now come from the manifest, the National Risk Index statement first.
    assert_eq!(
        info.attributions.len(),
        manifest["attributions"].as_array().unwrap().len()
    );
    assert_eq!(info.attributions[0].source, "FEMA National Risk Index");
    assert!(info.attributions[0].text.contains("not endorsed by FEMA"));

    // Every fixture household plans from the national data, in its own county.
    for (name, raw) in rr_types::fixtures::RAW {
        let input: PlanInput = serde_json::from_str(raw).unwrap();
        let output: PlanOutput = value(&api::assess(raw));
        assert_eq!(output.data_pack_version, pack_version, "{name}");
        if let Some(fips) = &input.location.county_fips {
            assert_eq!(&output.location.county_fips, fips, "{name}");
        }
        assert!(
            !output.register.is_empty() && !output.plan.months.is_empty(),
            "{name}"
        );
        assert!(
            !output
                .location
                .data_note
                .unwrap_or_default()
                .contains("sample counties"),
            "{name} is not planned from the sample counties"
        );
        // Deterministic with the packs too.
        assert_eq!(api::assess(raw), api::assess(raw), "{name}");
    }

    // Lookups use the whole country now.
    let found: Vec<LocationResolved> = value(&api::county_search("Los Angeles"));
    assert_eq!(found[0].county_fips, "06037");
    let e = error(&api::resolve_location(
        r#"{"country":"US","county_fips":"99999","setting":"urban"}"#,
    ));
    assert_eq!(e.code, ErrorCode::UnknownCounty);

    // A ZIP code no county holds 80 % of is ambiguous; picking a county resolves it.
    let zip = ambiguous_zip();
    let e = error(&api::resolve_location(&format!(
        r#"{{"country":"US","zip":"{zip}","setting":"suburban"}}"#
    )));
    assert_eq!(e.code, ErrorCode::AmbiguousZip, "{zip}");
    let suggestions = e.suggestions().unwrap();
    assert!(suggestions.len() >= 2);
    let shares: Vec<f32> = suggestions
        .iter()
        .map(|s| s.zip_county_share.unwrap())
        .collect();
    assert!(
        shares.windows(2).all(|w| w[0] >= w[1]),
        "largest share first: {shares:?}"
    );
    let picked: LocationResolved = value(&api::resolve_location(&format!(
        r#"{{"country":"US","zip":"{zip}","county_fips":"{}","setting":"suburban"}}"#,
        suggestions[1].county_fips
    )));
    assert_eq!(picked.county_fips, suggestions[1].county_fips);

    // The lazy map pack adds a second pack name.
    let _: PackInfo = value(&api::load_pack(
        "geo/counties.json",
        &read("data/geo/counties.json"),
    ));
    let info: EngineInfo = value(&api::engine_info());
    assert_eq!(info.packs_loaded, ["core", "geo"]);
}

/// A ZIP code in `data/core/zip_county.csv` whose largest county holds less than 80 % of it.
fn ambiguous_zip() -> String {
    let text = String::from_utf8(read("data/core/zip_county.csv")).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let zip_col = header.iter().position(|h| *h == "zip").unwrap();
    let share_col = header
        .iter()
        .position(|h| h.contains("share"))
        .expect("a share column");
    let mut best: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    for line in lines {
        let cols: Vec<&str> = line.split(',').collect();
        let share: f64 = cols[share_col].parse().unwrap_or(0.0);
        let e = best.entry(cols[zip_col].to_owned()).or_insert(0.0);
        *e = e.max(share);
    }
    best.into_iter()
        .find(|(_, top)| *top > 0.3 && *top < 0.6)
        .map(|(zip, _)| zip)
        .expect("an ambiguous ZIP code")
}

#[test]
fn a_file_that_does_not_match_the_manifest_is_pack_corrupt_and_nothing_plans_from_it() {
    let manifest_bytes = read("data/manifest.json");
    let _: PackInfo = value(&api::load_pack("manifest.json", &manifest_bytes));
    let mut states = read("data/core/states.csv");
    states.extend_from_slice(b"99,XX,Nowhere,none\n");
    let e = error(&api::load_pack("core/states.csv", &states));
    assert_eq!(e.code, ErrorCode::PackCorrupt);
    // With the manifest loaded the packs decide, and the core pack is not complete.
    let e = error(&api::assess(&fixture_json("philadelphia-renters-4")));
    assert_eq!(e.code, ErrorCode::PackMissing);
}
