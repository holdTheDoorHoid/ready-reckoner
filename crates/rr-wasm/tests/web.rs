//! The WebAssembly build itself, run in Node: `wasm-pack test --node crates/rr-wasm`.
//!
//! The exports are called as JavaScript calls them (a JSON string in, the envelope's JSON out) on
//! the Philadelphia fixture, and every fixture's answer is compared with its golden file as text:
//! the goldens are written by the native engine, so equality here means the browser build and the
//! CLI agree to the last digit.
#![cfg(target_arch = "wasm32")]

use rr_types::{EngineError, EngineInfo, Envelope, ErrorCode, PlanOutput, ProblemCode};
use wasm_bindgen_test::wasm_bindgen_test;

mod common;
use common::{minify, value_text};

const PHILADELPHIA: &str = include_str!("../../../fixtures/households/philadelphia-renters-4.json");
const PHILADELPHIA_GOLDEN: &str =
    include_str!("../../../fixtures/golden/philadelphia-renters-4.json");

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
    ("philadelphia-renters-4", PHILADELPHIA, PHILADELPHIA_GOLDEN),
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

#[wasm_bindgen_test]
fn philadelphia_round_trips_through_the_envelope() {
    let envelope = rr_wasm::assess(PHILADELPHIA);
    // The envelope as JavaScript receives it: `ok` first, then the value, nothing else.
    assert_eq!(value_text(&envelope), minify(PHILADELPHIA_GOLDEN));
    let output = match serde_json::from_str::<Envelope<PlanOutput>>(&envelope).unwrap() {
        Envelope::Value(v) => v,
        Envelope::Error(e) => panic!("{e}"),
    };
    assert_eq!(output.location.county_fips, "42101");
    assert_eq!(output.api_version, rr_types::ENGINE_API_VERSION);
    assert!(output.data_pack_version.starts_with("fixtures+"));
    assert!(output.packet_markdown.starts_with("# "));
    // Same input, same bytes.
    assert_eq!(rr_wasm::assess(PHILADELPHIA), envelope);

    // An error comes back in the same envelope, with the contract's details.
    let mut input: serde_json::Value = serde_json::from_str(PHILADELPHIA).unwrap();
    input["location"] =
        serde_json::json!({ "country": "US", "county_fips": "06037", "setting": "urban" });
    let e = error_of(&rr_wasm::assess(&input.to_string()));
    assert_eq!(e.code, ErrorCode::UnknownCounty);
    assert!(e.suggestions().is_some());
    let e = error_of(&rr_wasm::assess("{\"planning_date\": 7}"));
    assert_eq!(e.code, ErrorCode::BadInput);
    assert_eq!(e.problems().unwrap()[0].code, ProblemCode::Schema);
}

#[wasm_bindgen_test]
fn every_fixture_matches_the_native_golden_to_the_last_digit() {
    for (name, input, golden) in FIXTURES {
        let envelope = rr_wasm::assess(input);
        assert!(
            value_text(&envelope) == minify(golden),
            "{name}: the WebAssembly answer differs from fixtures/golden/{name}.json"
        );
    }
}

#[wasm_bindgen_test]
fn engine_info_and_the_other_exports_answer_in_envelopes() {
    let info = match serde_json::from_str::<Envelope<EngineInfo>>(&rr_wasm::engine_info()).unwrap()
    {
        Envelope::Value(v) => v,
        Envelope::Error(e) => panic!("{e}"),
    };
    assert!(info.packs_loaded.is_empty());
    assert_eq!(info.data_pack_version, None);
    assert_eq!(info.attributions[0].source, "FEMA National Risk Index");

    for envelope in [
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

    // A file the engine does not know is refused without leaving the sample counties.
    let e = error_of(&rr_wasm::load_pack("core/unknown.csv", b"a\n1\n"));
    assert_eq!(e.code, ErrorCode::BadInput);
    assert_eq!(e.problems().unwrap()[0].field, "name");
    assert!(rr_wasm::assess(PHILADELPHIA).starts_with(r#"{"ok":true,"#));
}
