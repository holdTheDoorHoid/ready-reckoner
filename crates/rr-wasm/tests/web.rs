//! The WebAssembly build itself, run in Node: `wasm-pack test --node crates/rr-wasm`.
//!
//! The exports are called as JavaScript calls them (a JSON string in, the envelope's JSON out).
//! Every test here runs in one WebAssembly instance, and the engine lives in that instance, so
//! the one test that loads data spells out the order of events itself: first the built-in sample
//! counties (the engine before any data arrives), then the packs in `data/` handed over file by
//! file (embedded at compile time; the site fetches the same files), then every fixture's answer
//! compared with its golden file as text. The goldens are planned by the native engine from the
//! same packs, so equality means the browser build and the CLI agree to the last digit. The other
//! tests hold in either state.
#![cfg(target_arch = "wasm32")]

use rr_types::{EngineError, EngineInfo, Envelope, ErrorCode, PackInfo, PlanOutput, ProblemCode};
use wasm_bindgen_test::wasm_bindgen_test;

mod common;
use common::{minify, value_text};

const PHILADELPHIA: &str = include_str!("../../../fixtures/households/philadelphia-renters-4.json");

/// Every fixture household and its golden output, by name.
const FIXTURES: &[(&str, &str, &str)] = &[
    (
        "chicago-student-zero-budget-1",
        include_str!("../../../fixtures/households/chicago-student-zero-budget-1.json"),
        include_str!("../../../fixtures/golden/chicago-student-zero-budget-1.json"),
    ),
    (
        "coos-bay-well-owner-2",
        include_str!("../../../fixtures/households/coos-bay-well-owner-2.json"),
        include_str!("../../../fixtures/golden/coos-bay-well-owner-2.json"),
    ),
    (
        "hays-kansas-farm-5",
        include_str!("../../../fixtures/households/hays-kansas-farm-5.json"),
        include_str!("../../../fixtures/golden/hays-kansas-farm-5.json"),
    ),
    (
        "miami-condo-retiree-1",
        include_str!("../../../fixtures/households/miami-condo-retiree-1.json"),
        include_str!("../../../fixtures/golden/miami-condo-retiree-1.json"),
    ),
    (
        "philadelphia-renters-4",
        PHILADELPHIA,
        include_str!("../../../fixtures/golden/philadelphia-renters-4.json"),
    ),
    (
        "phoenix-apartment-cpap-1",
        include_str!("../../../fixtures/households/phoenix-apartment-cpap-1.json"),
        include_str!("../../../fixtures/golden/phoenix-apartment-cpap-1.json"),
    ),
    (
        "sugar-land-ev-household-3",
        include_str!("../../../fixtures/households/sugar-land-ev-household-3.json"),
        include_str!("../../../fixtures/golden/sugar-land-ev-household-3.json"),
    ),
];

const MANIFEST: &[u8] = include_bytes!("../../../data/manifest.json");

/// The core pack's files with the county list last, as `web/src/engine/loader.ts` hands them over
/// (the site loads the ZIP tables later, when a ZIP code is typed; here they come with the rest).
/// The test checks this list against the manifest, so a file added to the pack is noticed.
const CORE: &[(&str, &[u8])] = &[
    (
        "core/base_rates.toml",
        include_bytes!("../../../data/core/base_rates.toml"),
    ),
    (
        "core/climate.csv",
        include_bytes!("../../../data/core/climate.csv"),
    ),
    (
        "core/ct_crosswalk.csv",
        include_bytes!("../../../data/core/ct_crosswalk.csv"),
    ),
    (
        "core/events.csv",
        include_bytes!("../../../data/core/events.csv"),
    ),
    (
        "core/facilities.csv",
        include_bytes!("../../../data/core/facilities.csv"),
    ),
    (
        "core/flood.csv",
        include_bytes!("../../../data/core/flood.csv"),
    ),
    (
        "core/nri_counties.csv",
        include_bytes!("../../../data/core/nri_counties.csv"),
    ),
    (
        "core/nri_hazards.csv",
        include_bytes!("../../../data/core/nri_hazards.csv"),
    ),
    (
        "core/nri_semantics.toml",
        include_bytes!("../../../data/core/nri_semantics.toml"),
    ),
    (
        "core/outages.csv",
        include_bytes!("../../../data/core/outages.csv"),
    ),
    (
        "core/outages_state.csv",
        include_bytes!("../../../data/core/outages_state.csv"),
    ),
    (
        "core/seismic.csv",
        include_bytes!("../../../data/core/seismic.csv"),
    ),
    (
        "core/states.csv",
        include_bytes!("../../../data/core/states.csv"),
    ),
    (
        "core/vulnerability.csv",
        include_bytes!("../../../data/core/vulnerability.csv"),
    ),
    (
        "core/zip_centroids.csv",
        include_bytes!("../../../data/core/zip_centroids.csv"),
    ),
    (
        "core/zip_county.csv",
        include_bytes!("../../../data/core/zip_county.csv"),
    ),
    (
        "core/zip_facilities.csv",
        include_bytes!("../../../data/core/zip_facilities.csv"),
    ),
    (
        "core/counties.csv",
        include_bytes!("../../../data/core/counties.csv"),
    ),
];

fn value_of<T: serde::de::DeserializeOwned>(envelope: &str) -> T {
    match serde_json::from_str::<Envelope<T>>(envelope).unwrap() {
        Envelope::Value(v) => v,
        Envelope::Error(e) => panic!("{e}"),
    }
}

fn error_of(envelope: &str) -> EngineError {
    assert!(
        envelope.starts_with(r#"{"ok":false,"error":{"code":"#),
        "{envelope:.300}"
    );
    match serde_json::from_str::<Envelope<serde_json::Value>>(envelope).unwrap() {
        Envelope::Error(e) => e,
        Envelope::Value(_) => unreachable!(),
    }
}

/// The top-level keys of a JSON object, in order.
fn keys(json: &str) -> Vec<String> {
    match serde_json::from_str::<serde_json::Value>(json).unwrap() {
        serde_json::Value::Object(map) => map.keys().cloned().collect(),
        other => panic!("not an object: {other:.100}"),
    }
}

#[wasm_bindgen_test]
fn sample_counties_first_then_the_packs_and_every_golden_to_the_last_digit() {
    // 1. Before any data: the seven built-in sample counties answer, and say so.
    let info: EngineInfo = value_of(&rr_wasm::engine_info());
    assert!(info.packs_loaded.is_empty(), "{:?}", info.packs_loaded);
    assert_eq!(info.data_pack_version, None);
    assert_eq!(info.attributions[0].source, "FEMA National Risk Index");
    // The sample-county shape check: a whole PlanOutput (it parses as the contract's type), with
    // the same top-level fields as a golden, stamped as sample data.
    let sample = rr_wasm::assess(PHILADELPHIA);
    let output: PlanOutput = value_of(&sample);
    assert_eq!(output.location.county_fips, "42101");
    assert_eq!(output.api_version, rr_types::ENGINE_API_VERSION);
    assert!(output.data_pack_version.starts_with("fixtures+"));
    assert!(
        output
            .location
            .data_note
            .as_deref()
            .unwrap_or_default()
            .contains("sample counties")
    );
    assert!(output.packet_markdown.starts_with("# "));
    assert!(!output.register.is_empty() && !output.plan.months.is_empty());
    assert_eq!(keys(value_text(&sample)), keys(FIXTURES[4].2));

    // 2. The packs, file by file: the manifest first, then the core pack.
    let manifest: serde_json::Value = serde_json::from_slice(MANIFEST).unwrap();
    let pack_version = manifest["pack_version"].as_str().unwrap().to_owned();
    let mut listed: Vec<&str> = manifest["packs"]["core"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    let mut embedded: Vec<&str> = CORE.iter().map(|(name, _)| *name).collect();
    listed.sort_unstable();
    embedded.sort_unstable();
    assert_eq!(
        embedded, listed,
        "data/manifest.json lists other core files than CORE embeds; update CORE"
    );
    let loaded: PackInfo = value_of(&rr_wasm::load_pack("manifest.json", MANIFEST));
    assert_eq!(loaded.version, pack_version);
    for (name, bytes) in CORE {
        let loaded: PackInfo = value_of(&rr_wasm::load_pack(name, bytes));
        assert_eq!(loaded.name, *name);
    }
    let info: EngineInfo = value_of(&rr_wasm::engine_info());
    assert_eq!(info.packs_loaded, ["core"]);
    assert_eq!(
        info.data_pack_version.as_deref(),
        Some(pack_version.as_str())
    );
    assert_eq!(info.attributions[0].source, "FEMA National Risk Index");

    // 3. Every fixture, planned from the packs, is its golden file to the last digit.
    for (name, input, golden) in FIXTURES {
        let golden_version =
            serde_json::from_str::<serde_json::Value>(golden).unwrap()["data_pack_version"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
        assert_eq!(
            golden_version, pack_version,
            "{name}: fixtures/golden/{name}.json was planned from other data than data/"
        );
        let envelope = rr_wasm::assess(input);
        assert!(
            value_text(&envelope) == minify(golden),
            "{name}: the WebAssembly answer differs from fixtures/golden/{name}.json (the native \
             test with_the_packs_loaded_every_fixture_is_its_golden_file_to_the_last_digit says \
             which side moved)"
        );
    }

    // 4. The same input gives the same bytes, and errors come back in the same envelope.
    let envelope = rr_wasm::assess(PHILADELPHIA);
    assert_eq!(rr_wasm::assess(PHILADELPHIA), envelope);
    let output: PlanOutput = value_of(&envelope);
    assert_eq!(output.location.county_fips, "42101");
    assert!(
        !output
            .location
            .data_note
            .unwrap_or_default()
            .contains("sample counties")
    );
    let found: Vec<rr_types::LocationResolved> = value_of(&rr_wasm::county_search("Los Angeles"));
    assert_eq!(found[0].county_fips, "06037");
}

/// Holds with or without the packs loaded.
#[wasm_bindgen_test]
fn errors_and_the_other_exports_answer_in_envelopes() {
    let mut input: serde_json::Value = serde_json::from_str(PHILADELPHIA).unwrap();
    input["location"] =
        serde_json::json!({ "country": "US", "county_fips": "99999", "setting": "urban" });
    let e = error_of(&rr_wasm::assess(&input.to_string()));
    assert_eq!(e.code, ErrorCode::UnknownCounty);
    assert!(e.suggestions().is_some());
    let e = error_of(&rr_wasm::assess("{\"planning_date\": 7}"));
    assert_eq!(e.code, ErrorCode::BadInput);
    assert_eq!(e.problems().unwrap()[0].code, ProblemCode::Schema);

    for envelope in [
        rr_wasm::engine_info(),
        rr_wasm::catalogue(),
        rr_wasm::defaults(),
        rr_wasm::county_search("phila"),
        rr_wasm::resolve_location(r#"{"country":"US","zip":"19147","setting":"urban"}"#),
    ] {
        assert!(
            envelope.starts_with(r#"{"ok":true,"value":"#),
            "{envelope:.200}"
        );
    }
    let request = format!(r#"{{"kind":"bucket","id":"water_out","input":{PHILADELPHIA}}}"#);
    assert!(rr_wasm::explain(&request).starts_with(r#"{"ok":true,"value":{"title":"#));

    // A file the engine does not know is refused, and changes nothing.
    let before = rr_wasm::engine_info();
    let e = error_of(&rr_wasm::load_pack("core/unknown.csv", b"a\n1\n"));
    assert_eq!(e.code, ErrorCode::BadInput);
    assert_eq!(e.problems().unwrap()[0].field, "name");
    assert_eq!(rr_wasm::engine_info(), before);
    assert!(rr_wasm::assess(PHILADELPHIA).starts_with(r#"{"ok":true,"#));
}
