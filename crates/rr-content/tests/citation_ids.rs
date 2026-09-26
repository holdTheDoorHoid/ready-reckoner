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
    // A placeholder rr-consequence reserves until the data workstream names the source.
    "county_boil_water_records",
    // Requested by rr-hazards for v0.2.0 ("Requested by hazards for v0.2.0" in the index; also in
    // rr_plan::provenance::AWAITING_CONTENT). Remove each once content/citations.toml has it.
    "rr_strategic_sites_2026",
    "fema_protection_nuclear_age_1985",
    "fema_napb90",
    "philippe_2023_icbm_fallout",
    "fema_uasi_fy2026",
    "nerc_tpl007_benchmark",
    "igrf14_coefficients",
    "noaa_hms_smoke",
    "epa_aqs_daily_pm25",
    "usgs_karst_2014",
    "usace_nid",
    "usace_nld",
    "asdso_dam_failures",
    "eviction_lab",
    "iii_water_damage",
    "fcc_att_outage_2024",
    "ashp_shortages",
    "openfda_drug_shortages",
    "crs_rs20348_funding_gaps",
    "snap_lapse_2025",
    "csis_terrorism_2025",
    "fbi_cde_arrests",
    "fbi_active_shooter_2024",
    "start_poicn",
    "xpt_2023_karger",
    "rp_2019_nuclear",
    "barrett_2013_inadvertent",
    "fema_nuclear_72h_2023",
    "epri_2019_hemp",
    "riley_2012_carrington",
    "morina_2019_carrington",
    "love_carrington",
    "lloyds_2013_solar",
    "cassidy_mani_2022",
    "usgs_yvo",
    "nasa_tunguska_2019",
    "fdic_failed_banks",
    "npr_maria_2018",
    "utah_wguep_2016",
    "usgs_seattle_fault",
    "pnnl_oe417_linkage",
    "cdc_co_quickstats",
    "ftc_sentinel_2024",
    "usgs_barry_arm",
    "cdc_h5n1_situation",
    "iv_fluids_helene_2024",
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
