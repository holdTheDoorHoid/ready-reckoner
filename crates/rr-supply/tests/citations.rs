//! Every citation id rr-supply emits must resolve to `content/citations.toml` (CLAUDE.md rule 2).
//! The content workstream owns that file; this test runs against it whenever it is present in the
//! checkout (always after the content branch is merged) and says so when it is not.

use std::collections::BTreeSet;

#[test]
fn every_supply_citation_resolves_to_the_content_registry() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/citations.toml");
    let Ok(text) = std::fs::read_to_string(path) else {
        eprintln!("content/citations.toml is not in this checkout yet; skipping the cross-check");
        return;
    };
    let registry: BTreeSet<String> = text
        .lines()
        .filter_map(|l| l.strip_prefix("id = \""))
        .filter_map(|l| l.strip_suffix('"'))
        .map(str::to_owned)
        .collect();
    assert!(registry.len() > 50, "could not read ids from {path}");
    let missing: Vec<_> = rr_supply::citations_used()
        .into_iter()
        .filter(|id| !registry.contains(id.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "cited by rr-supply but not in content/citations.toml: {missing:?}"
    );
}

#[test]
fn the_prior_source_is_the_content_registrys_expert_estimate() {
    assert_eq!(rr_supply::constants::PRIOR_SOURCE, "rr_expert_prior");
    let src = rr_supply::constants().source("rr_expert_prior").unwrap();
    assert!(src.prior);
}
