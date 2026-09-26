//! `docs/CITATION_IDS.md` is the index other workstreams use: every citation id it names must exist
//! in `content/citations.toml`.

const INDEX: &str = include_str!("../../../docs/CITATION_IDS.md");

/// Backticked words in the index that are not registry ids: the data-pack names to replace, the
/// scenario ids, and the prior tag.
const NOT_CITATIONS: &[&str] = &[
    "usfa_residential_fire_estimates",
    "nchs_fastats_emergency_department",
    "nchs_fastats_accidental_injury",
    "census_cps_hh1_households",
    "cascadia_m9",
    "new_madrid_m7",
    "hayward_m7",
];

#[test]
fn every_id_in_the_citation_index_is_in_the_registry() {
    let content = rr_content::content();
    let mut checked = 0;
    for piece in INDEX.split('`').skip(1).step_by(2) {
        if !rr_types::is_well_formed_id(piece) || NOT_CITATIONS.contains(&piece) {
            continue;
        }
        assert!(
            content.citation(piece).is_some(),
            "docs/CITATION_IDS.md names `{piece}`, which is not in content/citations.toml"
        );
        checked += 1;
    }
    assert!(checked > 150, "only {checked} ids checked");
}
