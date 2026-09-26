//! Every contract type survives a JSON round trip unchanged, and absent optional fields are left
//! out rather than written as `null`.

mod common;

use std::fmt::Debug;

use common::samples;
use rr_types::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + Debug>(name: &str, value: &T) {
    let json = serde_json::to_string_pretty(value).unwrap();
    let back: T = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{name}: {e}\n{json}"));
    assert_eq!(&back, value, "{name} changed in a round trip");
    assert_eq!(
        serde_json::to_string_pretty(&back).unwrap(),
        json,
        "{name} serialises differently the second time"
    );
}

#[test]
fn every_contract_type_round_trips() {
    round_trip("PlanInput", &samples::plan_input());
    round_trip("PlanInput::defaults", &PlanInput::defaults());
    round_trip("PlanOutput", &samples::plan_output());
    round_trip("Catalogue", &samples::catalogue());
    round_trip("EngineInfo", &samples::engine_info());
    round_trip("PackInfo", &samples::pack_info());
    round_trip("ExplainRequest", &samples::explain_request());
    round_trip("Explanation", &samples::explanation());
    round_trip("EngineError", &samples::engine_error());
    round_trip("BadInputDetails", &samples::bad_input_details());
    round_trip("LocationSuggestions", &samples::location_suggestions());
    round_trip(
        "Effect (log-normal)",
        &samples::effect(samples::log_normal()),
    );
    round_trip("Effect (fixed)", &samples::effect(samples::fixed()));
    round_trip("Envelope ok", &Envelope::Value(samples::plan_output()));
    round_trip(
        "Envelope error",
        &Envelope::<PlanOutput>::Error(samples::engine_error()),
    );
    round_trip(
        "HouseholdEventRate",
        &HouseholdEventRate {
            hazard: HazardId::HouseFire,
            rate_per_year: 0.003,
            low: 0.002,
            high: 0.004,
            evidence: Evidence::Prior,
            sources: vec!["nfpa_home_fires".into()],
        },
    );
}

#[test]
fn the_maximal_input_sample_is_valid() {
    assert_eq!(samples::plan_input().validate(), vec![]);
}

#[test]
fn absent_optional_fields_are_omitted_not_null() {
    let mut out = samples::plan_output();
    out.location.zip = None;
    out.location.zip_county_share = None;
    out.location.data_note = None;
    for h in &mut out.register {
        h.eal_per_household_usd = None;
    }
    for b in &mut out.buckets {
        b.relief = None;
    }
    for m in &mut out.plan.months {
        for item in &mut m.items {
            item.done = false;
            item.paid_usd = None;
        }
    }
    out.plan.done_month = None;
    out.plan.savings_track = None;
    for c in &mut out.provenance {
        c.year = None;
        c.quote = None;
    }
    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains("null"), "{json}");
    let value = serde_json::to_value(&out).unwrap();
    for item in value["plan"]["months"][0]["items"].as_array().unwrap() {
        assert!(item.get("done").is_none(), "done: false is omitted");
    }

    let mut cat = samples::catalogue();
    for item in &mut cat.items {
        item.maintenance = None;
        item.price_band_usd.note = None;
        item.energy_kcal_per_unit = None;
        item.volume_l_per_unit = None;
        item.retrieved = None;
    }
    let json = serde_json::to_string(&cat).unwrap();
    assert!(!json.contains("null"), "{json}");

    let mut info = samples::engine_info();
    info.data_pack_version = None;
    info.attributions[0].version = None;
    assert!(!serde_json::to_string(&info).unwrap().contains("null"));

    let mut ex = samples::explanation();
    ex.math = None;
    assert!(!serde_json::to_string(&ex).unwrap().contains("null"));

    let err = EngineError::new(ErrorCode::PackMissing, "Load the core pack first.");
    assert!(!serde_json::to_string(&err).unwrap().contains("null"));

    let json = serde_json::to_string(&PlanInput::defaults()).unwrap();
    assert!(!json.contains("null"), "{json}");
}

#[test]
fn content_flags_default_to_false_when_absent() {
    let mut v = serde_json::to_value(samples::item()).unwrap();
    let obj = v.as_object_mut().unwrap();
    obj.remove("life_safety");
    obj.remove("rare_catastrophic");
    obj.remove("retrieved");
    let item: Item = serde_json::from_value(v).unwrap();
    assert!(!item.life_safety && !item.rare_catastrophic && item.retrieved.is_none());

    let mut v = serde_json::to_value(samples::citation()).unwrap();
    v.as_object_mut().unwrap().remove("prior");
    let c: Citation = serde_json::from_value(v).unwrap();
    assert!(!c.prior);
}
