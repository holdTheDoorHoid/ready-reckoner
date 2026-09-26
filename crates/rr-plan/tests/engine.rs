//! The engine's other functions (engine_info, catalogue, defaults), the coverage table's links
//! to the catalogue and rr-supply, and the citation ids waiting for content.

mod common;

use common::engine;
use rr_plan::coverage::{DIVISIBLE_CLASSES, ITEM_LINES, LONG_STORE_FOOD, Units};
use rr_plan::provenance::AWAITING_CONTENT;

#[test]
fn engine_info_carries_versions_and_the_nri_statement() {
    let info = engine().engine_info();
    assert_eq!(info.api_version, rr_types::ENGINE_API_VERSION);
    assert_eq!(info.engine_version, rr_plan::ENGINE_VERSION);
    assert_eq!(info.content_version, rr_content::CONTENT_VERSION);
    assert!(info.data_pack_version.unwrap().starts_with("fixtures+"));
    assert_eq!(info.packs_loaded, ["fixtures"]);
    let nri = info
        .attributions
        .iter()
        .find(|a| a.source.contains("National Risk Index"))
        .expect("the NRI statement");
    let registry = rr_content::content()
        .citation("fema_nri_disclaimer")
        .and_then(|c| c.quote.clone())
        .unwrap();
    assert_eq!(nri.text, registry, "shown exactly as the terms require");
    assert!(nri.version.as_deref().unwrap().contains("1.20"));
    assert_eq!(
        info.attributions[0].source, nri.source,
        "NRI statement first"
    );
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
            if let Units::Per(u) = units {
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
