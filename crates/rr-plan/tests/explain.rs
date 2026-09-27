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
    // Philadelphia's power target is 3 days (research: 2.8), since the major-hurricane share
    // is taken against tropical-storm passages (verification V-02).
    assert!(math.contains("Λ(3) ="), "{math}");
    assert!(math.contains("∫₀^3 Λ(t) dt"), "{math}");
    assert!(math.contains("1 in 100: 3 days"), "{math}");
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

/// `explain warning <id>` covers every warning the plan emits, on every fixture, including the
/// ones added once the output is assembled.
#[test]
fn every_warning_of_every_fixture_explains() {
    let mut seen = 0;
    for (name, input, out) in common::outputs() {
        let a = common::run(input);
        assert_eq!(
            engine().warnings(&a),
            out.warnings,
            "{name}: Engine::warnings is the output's list"
        );
        for w in &out.warnings {
            let e = engine()
                .explain(ExplainKind::Warning, &w.id, input)
                .unwrap_or_else(|err| panic!("{name}: {}: {err}", w.id));
            assert_eq!(e.title, w.message, "{name}: {}", w.id);
            assert_eq!(
                e.plain.as_slice(),
                std::slice::from_ref(&w.why),
                "{name}: {}",
                w.id
            );
            seen += 1;
        }
    }
    assert!(seen > 20, "{seen}");
}

/// `citation_missing` is added after the assessment (a number whose source is still being added
/// to the registry); explaining it reads the output's warnings, not the assessment's.
#[test]
fn a_missing_citation_warning_explains_too() {
    let input = household("philadelphia-renters-4");
    let a = common::run(&input);
    let w = rr_plan::provenance::missing_warning(&[rr_types::CitationId::from(
        "county_boil_water_records",
    )])
    .expect("a warning for an unknown id");
    assert_eq!(w.id, "citation_missing");
    assert!(!a.warnings.iter().any(|x| x.id == w.id));
    let mut warnings = engine().warnings(&a);
    warnings.push(w.clone());
    let e = rr_plan::explain::warning_in(&a, rr_content::content(), &warnings, "citation_missing")
        .expect("explained");
    assert_eq!(e.title, w.message);
    assert_eq!(e.plain, [w.why]);
    // Without it in the list, it is an unknown id like any other.
    let err =
        rr_plan::explain::warning_in(&a, rr_content::content(), &a.warnings, "citation_missing")
            .unwrap_err();
    assert_eq!(err.code, ErrorCode::BadInput);
}

/// A rare family explains its chain: the household's range, the location factor, each sub-cause
/// with its range and what it means, the ten-year chance, and the sources of all of them.
#[test]
fn a_rare_family_explains_its_chain() {
    let input = household("minot-missile-field-3");
    let e = engine()
        .explain(ExplainKind::Hazard, "nuclear_attack", &input)
        .unwrap();
    let math = e.math.expect("the arithmetic").join("\n");
    assert!(math.contains("Your household's rate: between"), "{math}");
    assert!(math.contains("Location factor (class A)"), "{math}");
    assert!(math.contains("Electromagnetic pulse (EMP)"), "{math}");
    assert!(math.contains("Chance in 10 years"), "{math}");
    let plain = e.plain.join("\n");
    assert!(plain.contains("Minot Air Force Base"), "{plain}");
    assert!(plain.contains("One free step"), "{plain}");
    assert!(e.sources.len() >= 3, "{:?}", e.sources);
    // The months-long blackout family's chain ends with the county's own power curve.
    let e = engine()
        .explain(
            ExplainKind::Hazard,
            "multi_month_blackout",
            &household("coos-bay-well-owner-2"),
        )
        .unwrap();
    let math = e.math.unwrap().join("\n");
    assert!(math.contains("Your county's own outage record"), "{math}");
}

/// Where no restoration records match the event behind a target, the packet's targets table and
/// `explain bucket` name the worst event on record instead, and read how long it lasted as the
/// app does (verification R3-05): up to the first mark after the last one with at least 0.5 in
/// 100 homes still out. Hays: a winter storm with 3.5 in 100 still out on day 7 and none on day
/// 14 reads "up to 2 weeks" (it read "up to 7 days"); Phoenix: 1.6 in 100 still out on day 3 and
/// none on day 7 reads "up to 7 days" (it read "up to 3 days").
#[test]
fn a_bucket_without_relief_names_the_worst_event_on_record() {
    for (name, still_out, up_to) in [
        ("hays-kansas-farm-5", (7.0, 0.035), "up to 2 weeks"),
        ("phoenix-apartment-cpap-1", (3.0, 0.016), "up to 7 days"),
    ] {
        let input = household(name);
        let out = common::assess(&input);
        let power = out
            .buckets
            .iter()
            .find(|b| b.id == rr_types::BucketId::Power)
            .unwrap();
        assert!(power.relief.is_none(), "{name}: {:?}", power.relief);
        let st = power.stress_test.as_ref().expect("a stress event");
        // The record the rule reads: the last mark with homes still out, and none at the next.
        let marks = &st.share_out_at_days;
        let at = marks.iter().position(|(d, _)| *d == still_out.0).unwrap();
        assert!(
            (marks[at].1 - still_out.1).abs() < 0.001 && marks[at + 1].1 < 0.005,
            "{name}: {marks:?}"
        );
        assert!(
            out.packet_markdown
                .contains(&format!("| not known | worst on record: {up_to} |")),
            "{name}: the targets table"
        );
        let e = engine()
            .explain(ExplainKind::Bucket, "power", &input)
            .unwrap();
        let plain = e.plain.join("\n");
        assert!(
            plain.contains("the worst event on record stands in: ")
                && plain.contains(&st.event)
                && plain.contains(&format!("kept some homes waiting {up_to}.")),
            "{name}: {plain}"
        );
        for c in &st.sources {
            assert!(e.sources.iter().any(|s| s.id == *c), "{name}: {c}");
        }
    }
}
