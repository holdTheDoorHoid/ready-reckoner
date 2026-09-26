//! The engine's other functions (engine_info, catalogue, defaults), the coverage table's links
//! to the catalogue and rr-supply, and the citation ids waiting for content.

mod common;

use common::engine;
use rr_plan::coverage::{DIVISIBLE_CLASSES, ITEM_LINES, LONG_STORE_FOOD, Units};
use rr_plan::provenance::AWAITING_CONTENT;

fn nri_disclaimer() -> String {
    rr_content::content()
        .citation("fema_nri_disclaimer")
        .and_then(|c| c.quote.clone())
        .unwrap()
}

#[test]
fn engine_info_carries_versions_and_the_nri_statement() {
    let info = engine().engine_info();
    assert_eq!(info.api_version, rr_types::ENGINE_API_VERSION);
    assert_eq!(info.engine_version, rr_plan::ENGINE_VERSION);
    assert_eq!(info.content_version, rr_content::CONTENT_VERSION);
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(rr_plan::golden::data_dir().join("manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        info.data_pack_version.as_deref(),
        manifest["pack_version"].as_str()
    );
    assert_eq!(info.packs_loaded, ["core"]);
    let first = &info.attributions[0];
    assert!(
        first.source.contains("National Risk Index"),
        "NRI statement first"
    );
    assert!(
        first.text.contains(&nri_disclaimer()),
        "the manifest's NRI text carries the statement the terms require"
    );
    assert!(first.version.as_deref().unwrap().contains("1.20"));
}

#[test]
fn with_no_packs_the_sample_counties_still_plan() {
    let e = common::fixture_engine();
    let info = e.engine_info();
    assert!(info.data_pack_version.unwrap().starts_with("fixtures+"));
    assert_eq!(info.packs_loaded, ["fixtures"]);
    assert_eq!(
        info.attributions[0].text,
        nri_disclaimer(),
        "exact statement"
    );
    let out = e
        .assess(&common::household("philadelphia-renters-4"))
        .unwrap();
    assert_eq!(out.location.county_fips, "42101");
    assert_eq!(
        out.location.data_note.as_deref(),
        Some(rr_plan::source::FIXTURE_DATA_NOTE)
    );
    assert!(out.packet_markdown.contains("**Sample data.**"));
}

#[test]
fn catalogue_and_defaults() {
    let c = engine().catalogue();
    assert_eq!(c.items.len(), rr_content::content().items.len());
    assert_eq!(c.buckets.len(), 14);
    assert_eq!(c.hazards.len(), 35);
    let d = engine().defaults();
    assert!(d.validate().is_empty());
    // The placeholder ZIP code fails loudly rather than planning for somewhere else.
    assert_eq!(
        engine().assess(&d).unwrap_err().code,
        rr_types::ErrorCode::UnknownZip
    );
}

#[test]
fn the_item_table_names_real_items_and_real_lines() {
    let content = rr_content::content();
    let rules = rr_supply::LINE_RULES;
    for (item, lines) in ITEM_LINES {
        assert!(
            content.item(item).is_some(),
            "{item} is not in the catalogue"
        );
        for (key, units) in *lines {
            let (bucket, rule) = key.split_once('.').unwrap_or_else(|| panic!("{key}"));
            assert!(bucket.parse::<rr_types::BucketId>().is_ok(), "{key}");
            assert!(
                rules.contains(&rule),
                "{key}: {rule} is not an rr-supply line rule"
            );
            if let Units::Per(u) | Units::Share(u) | Units::Rest(u) = units {
                assert!(*u > 0.0, "{key}");
            }
        }
    }
    for id in LONG_STORE_FOOD {
        assert_eq!(
            content.item(id).map(|i| i.quantity_rule.as_str()),
            Some("food_kcal"),
            "{id}"
        );
    }
    assert!(DIVISIBLE_CLASSES.contains(&"water_stored"));
}

/// The cooler bag counts rr-supply's `cooler_hold_days` of the cold-storage line, and the power
/// station the rest (round-2 review S1): the table's numbers follow the constant.
#[test]
fn the_cooler_bag_counts_one_day_and_the_station_the_rest() {
    let hold = rr_supply::constants().value(rr_supply::constants::keys::COOLER_HOLD_DAYS);
    let find = |item: &str| {
        ITEM_LINES
            .iter()
            .find(|(id, _)| *id == item)
            .and_then(|(_, lines)| {
                lines
                    .iter()
                    .find(|(key, _)| *key == "medication.rx_cold_storage")
                    .map(|(_, u)| *u)
            })
    };
    assert_eq!(find("med_cooler_refrigerated_rx"), Some(Units::Share(hold)));
    assert_eq!(find("power_station"), Some(Units::Rest(hold)));
}

/// Every catalogue-looking id the packet and coverage code name is a real item (or an rr-supply
/// rule or item class of the same shape), so an item the content merges into a parent is not
/// skipped silently.
#[test]
fn ids_named_in_the_code_are_real_items() {
    let content = rr_content::content();
    let classes: std::collections::BTreeSet<String> = common::outputs()
        .iter()
        .flat_map(|(_, _, out)| out.requirements.iter().map(|r| r.item_class.clone()))
        .collect();
    let prefixes = [
        "comms_",
        "community_",
        "docs_",
        "evac_",
        "fire_",
        "food_",
        "gethome_",
        "med_",
        "power_",
        "rare_",
        "san_",
        "security_",
        "special_",
        "thermal_",
        "water_",
    ];
    let files = [
        ("coverage.rs", include_str!("../src/coverage.rs")),
        ("pipeline.rs", include_str!("../src/pipeline.rs")),
        ("packet/people.rs", include_str!("../src/packet/people.rs")),
        (
            "packet/summary.rs",
            include_str!("../src/packet/summary.rs"),
        ),
        ("packet/plan.rs", include_str!("../src/packet/plan.rs")),
        (
            "packet/checklists.rs",
            include_str!("../src/packet/checklists.rs"),
        ),
        (
            "packet/targets.rs",
            include_str!("../src/packet/targets.rs"),
        ),
        (
            "packet/calendar.rs",
            include_str!("../src/packet/calendar.rs"),
        ),
    ];
    let mut missing = Vec::new();
    for (file, text) in files {
        for (i, quoted) in text.split('"').enumerate() {
            let is_literal = i % 2 == 1;
            let shaped = quoted
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if !is_literal || !shaped || !prefixes.iter().any(|p| quoted.starts_with(p)) {
                continue;
            }
            if content.item(quoted).is_none()
                && !rr_supply::LINE_RULES.contains(&quoted)
                && !classes.contains(quoted)
            {
                missing.push(format!("{file}: {quoted}"));
            }
        }
    }
    assert!(missing.is_empty(), "not in the catalogue: {missing:?}");
}

#[test]
fn awaiting_ids_are_still_missing_from_the_registry_and_listed_as_requested() {
    let content = rr_content::content();
    let index = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/CITATION_IDS.md"
    ))
    .unwrap();
    for id in AWAITING_CONTENT {
        assert!(
            content.citation(id).is_none(),
            "{id} is now in the registry: remove it from AWAITING_CONTENT"
        );
        assert!(
            index.contains(&format!("`{id}`")),
            "{id} is not requested in CITATION_IDS.md"
        );
    }
}

#[test]
fn every_citation_id_the_engine_crates_can_emit_resolves_or_is_awaited() {
    let content = rr_content::content();
    let mut ids: Vec<String> = rr_consequence::citation_ids()
        .into_iter()
        .map(|c| c.as_str().to_owned())
        .collect();
    ids.extend(rr_hazards::CITATION_IDS.iter().map(|s| (*s).to_owned()));
    ids.extend(
        rr_supply::citations_used()
            .into_iter()
            .map(|c| c.as_str().to_owned()),
    );
    ids.extend(
        rr_plan::coverage::COVERAGE_CITATIONS
            .iter()
            .map(|s| (*s).to_owned()),
    );
    ids.push(rr_budget::weights::HARM_WEIGHT_CITATION.to_owned());
    for item in &content.items {
        ids.extend(item.citations.iter().map(|c| c.as_str().to_owned()));
    }
    for id in ids {
        assert!(
            content.citation(&id).is_some() || AWAITING_CONTENT.contains(&id.as_str()),
            "{id} does not resolve"
        );
    }
}
