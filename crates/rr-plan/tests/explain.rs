//! explain: plain sentences first, then the arithmetic, then sources, for every kind of id.

mod common;

use common::{engine, household};
use rr_types::{ErrorCode, ExplainKind, ExplainRequest};

#[test]
fn every_kind_explains_with_plain_words_math_and_sources() {
    let input = household("philadelphia-renters-4");
    let out = common::assess(&input);
    let warning = out
        .warnings
        .first()
        .expect("Philadelphia has a warning")
        .id
        .clone();
    let cases = [
        (ExplainKind::Hazard, "heat_wave", true),
        (ExplainKind::Bucket, "power", true),
        (ExplainKind::Bucket, "income", true),
        (ExplainKind::Bucket, "evacuate", true),
        (ExplainKind::Item, "water_stored_bottled", true),
        (ExplainKind::Requirement, "water_out.water_gallons", true),
        (ExplainKind::Warning, warning.as_str(), false),
    ];
    for (kind, id, math) in cases {
        let e = engine()
            .explain(kind, id, &input)
            .unwrap_or_else(|err| panic!("{kind} {id}: {err}"));
        assert!(!e.title.is_empty(), "{kind} {id}");
        assert!(!e.plain.is_empty() && !e.plain[0].is_empty(), "{kind} {id}");
        assert_eq!(e.math.is_some(), math, "{kind} {id}");
        if kind != ExplainKind::Warning {
            assert!(!e.sources.is_empty(), "{kind} {id} has no sources");
        }
    }
}

#[test]
fn the_bucket_explanation_shows_lambda_at_the_target_and_the_value_integral() {
    let input = household("philadelphia-renters-4");
    let e = engine()
        .explain(ExplainKind::Bucket, "power", &input)
        .unwrap();
    let math = e.math.unwrap().join("\n");
    assert!(math.contains("Λ(5) ="), "{math}");
    assert!(math.contains("∫₀^5 Λ(t) dt"), "{math}");
    assert!(math.contains("1 in 100: 5 days"), "{math}");
    let hazard = engine()
        .explain(ExplainKind::Hazard, "heat_wave", &input)
        .unwrap();
    assert!(hazard.math.unwrap()[0].contains("r = λ · a · m"));
    let item = engine()
        .explain(ExplainKind::Item, "water_stored_bottled", &input)
        .unwrap();
    assert!(
        item.math
            .unwrap()
            .iter()
            .any(|l| l.contains("water_out.water_gallons")),
        "the item names the line it meets"
    );
}

#[test]
fn unknown_ids_are_bad_input_and_the_request_shape_works() {
    let input = household("philadelphia-renters-4");
    for (kind, id) in [
        (ExplainKind::Hazard, "hurrican"),
        (ExplainKind::Bucket, "water"),
        (ExplainKind::Item, "no_such_item"),
        (ExplainKind::Requirement, "power.nothing"),
        (ExplainKind::Warning, "no_such_warning"),
    ] {
        let err = engine().explain(kind, id, &input).unwrap_err();
        assert_eq!(err.code, ErrorCode::BadInput, "{kind} {id}");
    }
    let req = ExplainRequest {
        kind: ExplainKind::Item,
        id: "power_generator".into(),
        input,
    };
    let e = engine().explain_request(&req).unwrap();
    assert!(
        e.plain
            .iter()
            .any(|p| p.contains("optional") || p.contains("does not need")),
        "{:?}",
        e.plain
    );
}
