//! Every citation id rr-hazards can attach to a number resolves: it is defined in
//! `content/citations.toml` (once the content workstream's registry is merged) or listed in
//! `docs/CITATION_IDS.md`, where requested ids wait for their entry.

use std::path::PathBuf;

fn repo_file(relative: &str) -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(path).ok()
}

#[test]
fn every_citation_id_is_defined_or_requested() {
    let listed = repo_file("docs/CITATION_IDS.md").expect("docs/CITATION_IDS.md");
    let registry = repo_file("content/citations.toml").unwrap_or_default();
    for id in rr_hazards::CITATION_IDS {
        // Registered ids appear in backticks; ids requested and not yet written appear in bold
        // (the rr-content index test checks every backticked id against the registry).
        let in_list = listed.contains(&format!("`{id}`")) || listed.contains(&format!("**{id}**"));
        let in_registry = registry.contains(&format!("id = \"{id}\""));
        assert!(
            in_list || in_registry,
            "citation id `{id}` is neither in content/citations.toml nor docs/CITATION_IDS.md"
        );
    }
}

#[test]
fn citation_ids_are_well_formed_and_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for id in rr_hazards::CITATION_IDS {
        assert!(
            rr_types::is_well_formed_id(id),
            "{id} is not a snake_case id"
        );
        assert!(seen.insert(*id), "{id} listed twice");
    }
}
