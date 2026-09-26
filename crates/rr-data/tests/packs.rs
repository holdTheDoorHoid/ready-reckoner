//! Integration tests against the committed data packs in `data/`.
//!
//! They load every file the manifest lists, the way the web app will, and check the promises
//! `rr-data` makes: checksums and row counts match the manifest, every fixture household's
//! location resolves, ambiguous ZIP codes are detected, every county on the map joins every core
//! pack (or is listed as missing with a reason), and every record round-trips through JSON.

use rr_data::{AMBIGUOUS_ZIP_SHARE, DataStore, Manifest};
use rr_types::{
    AfreqKind, ClimateHorizon, CountyRecord, ErrorCode, HazardId, LocationInput, Setting,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::OnceLock;

/// The committed packs, or another data directory named by `RR_DATA_DIR`.
fn data_dir() -> PathBuf {
    std::env::var("RR_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"))
}

fn manifest() -> Manifest {
    let bytes = std::fs::read(data_dir().join("manifest.json")).expect("data/manifest.json");
    serde_json::from_slice(&bytes).expect("manifest parses")
}

fn file_paths(m: &Manifest) -> Vec<String> {
    let mut v: Vec<String> = m
        .packs
        .values()
        .flat_map(|p| p.files.iter().map(|f| f.path.clone()))
        .collect();
    v.sort();
    v
}

/// A store with every pack file loaded (manifest first), shared by the tests.
fn store() -> &'static DataStore {
    static STORE: OnceLock<DataStore> = OnceLock::new();
    STORE.get_or_init(|| {
        let m = manifest();
        let mut files: Vec<(String, Vec<u8>)> = vec![(
            "manifest.json".into(),
            std::fs::read(data_dir().join("manifest.json")).unwrap(),
        )];
        for p in file_paths(&m) {
            let bytes = std::fs::read(data_dir().join(&p)).unwrap_or_else(|e| panic!("{p}: {e}"));
            files.push((p, bytes));
        }
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let mut s = DataStore::new();
        let infos = s.load_many(&refs).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(infos.len(), files.len());
        s
    })
}

#[test]
fn every_manifest_file_loads_with_matching_rows_and_checksum() {
    let m = manifest();
    let s = store();
    assert!(!m.pack_version.is_empty());
    for f in m.packs.values().flat_map(|p| p.files.iter()) {
        let rows = s
            .loaded()
            .get(&f.path)
            .copied()
            .unwrap_or_else(|| panic!("{} not loaded", f.path));
        assert_eq!(rows as u64, f.rows, "{}: rows loaded vs manifest", f.path);
    }
    // A corrupted byte is caught by the checksum.
    let mut s2 = DataStore::new();
    s2.load_pack(
        "manifest.json",
        &std::fs::read(data_dir().join("manifest.json")).unwrap(),
    )
    .unwrap();
    let mut bytes = std::fs::read(data_dir().join("core/states.csv")).unwrap();
    let last = bytes.len() - 2;
    bytes[last] ^= 1;
    assert_eq!(
        s2.load_pack("core/states.csv", &bytes).unwrap_err().code,
        ErrorCode::PackCorrupt
    );
}

#[test]
fn every_fixture_location_resolves() {
    let s = store();
    for (name, input) in rr_types::fixtures::all() {
        let loc = s
            .resolve(&input.location)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(loc.country, "US");
        assert!(s.county(&loc.county_fips).is_some(), "{name}");
        if let Some(zip) = &input.location.zip {
            assert_eq!(loc.zip.as_deref(), Some(zip.as_str()), "{name}");
            assert!(
                loc.zip_county_share.unwrap() >= AMBIGUOUS_ZIP_SHARE,
                "{name}"
            );
        }
    }
    // Spot checks against the fixtures' known places.
    let zip = |z: &str| LocationInput {
        country: "US".into(),
        zip: Some(z.into()),
        county_fips: None,
        setting: Setting::Urban,
    };
    assert_eq!(s.resolve(&zip("19147")).unwrap().county_fips, "42101"); // Philadelphia
    assert_eq!(s.resolve(&zip("33139")).unwrap().county_fips, "12086"); // Miami Beach -> Miami-Dade
    assert_eq!(s.resolve(&zip("97420")).unwrap().county_fips, "41011"); // Coos Bay -> Coos
    assert_eq!(s.resolve(&zip("77479")).unwrap().county_fips, "48157"); // Sugar Land -> Fort Bend
    let ellis = LocationInput {
        country: "US".into(),
        zip: None,
        county_fips: Some("20051".into()),
        setting: Setting::Rural,
    };
    assert_eq!(s.resolve(&ellis).unwrap().county_name, "Ellis");
}

#[test]
fn ambiguous_zips_are_detected_and_explained() {
    let s = store();
    let bytes = std::fs::read(data_dir().join("core/zip_county.csv")).unwrap();
    let mut rdr = csv::Reader::from_reader(bytes.as_slice());
    let mut best: BTreeMap<String, f32> = BTreeMap::new();
    for r in rdr.records() {
        let r = r.unwrap();
        let share: f32 = r[2].parse().unwrap();
        let e = best.entry(r[0].to_string()).or_insert(0.0);
        *e = e.max(share);
    }
    let ambiguous: Vec<&String> = best
        .iter()
        .filter(|(_, s)| **s < AMBIGUOUS_ZIP_SHARE)
        .map(|(z, _)| z)
        .collect();
    assert!(
        ambiguous.len() > 1000,
        "expected thousands of ambiguous ZIPs, found {}",
        ambiguous.len()
    );
    for z in ambiguous.iter().take(200) {
        assert!(s.is_ambiguous_zip(z));
        let input = LocationInput {
            country: "US".into(),
            zip: Some((*z).clone()),
            county_fips: None,
            setting: Setting::Suburban,
        };
        let e = s.resolve(&input).unwrap_err();
        assert_eq!(e.code, ErrorCode::AmbiguousZip, "{z}");
        let sug = e.suggestions().unwrap();
        assert!(sug.len() >= 2, "{z}");
        let shares: Vec<f32> = sug.iter().map(|l| l.zip_county_share.unwrap()).collect();
        assert!(
            shares.windows(2).all(|w| w[0] >= w[1]),
            "{z}: suggestions sorted by share"
        );
        // Choosing one of them resolves.
        let pick = LocationInput {
            country: "US".into(),
            zip: Some((*z).clone()),
            county_fips: Some(sug[1].county_fips.clone()),
            setting: Setting::Suburban,
        };
        assert_eq!(s.resolve(&pick).unwrap().county_fips, sug[1].county_fips);
    }
    // Unknown ZIPs and counties get suggestions, not a panic.
    let unknown = LocationInput {
        country: "US".into(),
        zip: Some("00000".into()),
        county_fips: None,
        setting: Setting::Urban,
    };
    assert_eq!(s.resolve(&unknown).unwrap_err().code, ErrorCode::UnknownZip);
    let bad = LocationInput {
        country: "US".into(),
        zip: None,
        county_fips: Some("99999".into()),
        setting: Setting::Urban,
    };
    assert_eq!(s.resolve(&bad).unwrap_err().code, ErrorCode::UnknownCounty);
}

#[test]
fn every_map_county_joins_every_core_pack_or_is_explained() {
    let s = store();
    let m = manifest();
    let map_ids = s.map_ids().clone();
    assert!(map_ids.len() > 3200, "map features: {}", map_ids.len());
    let canonical: BTreeSet<String> = s.counties().map(|c| c.fips.clone()).collect();
    assert_eq!(canonical.len(), 3232);
    for id in &map_ids {
        assert!(
            canonical.contains(id),
            "map county {id} missing from counties.csv"
        );
    }
    // Per job: which counties have a row.
    let present = |f: &dyn Fn(&CountyRecord) -> bool| -> BTreeSet<String> {
        s.counties()
            .filter(|c| f(c))
            .map(|c| c.fips.clone())
            .collect()
    };
    let checks: Vec<(&str, BTreeSet<String>)> = vec![
        ("nri", present(&|c| !c.nri.is_empty())),
        ("outages", present(&|c| c.outages.is_some())),
        ("events", present(&|c| !c.events.is_empty())),
        ("seismic", present(&|c| c.seismic.is_some())),
        ("climate", present(&|c| !c.climate.is_empty())),
        ("flood", present(&|c| c.flood.is_some())),
        ("facilities", present(&|c| c.facilities.is_some())),
        ("vulnerability", present(&|c| c.vulnerability.is_some())),
    ];
    for (job, have) in checks {
        let missing = m.missing(job);
        for id in &map_ids {
            assert!(
                have.contains(id) || missing.contains_key(id),
                "county {id} has no {job} data and no stated reason in the manifest"
            );
        }
        for (fips, reason) in &missing {
            assert!(!reason.is_empty(), "{job}: empty reason for {fips}");
        }
    }
    // Connecticut uses planning regions everywhere.
    assert!(s.county("09001").is_none());
    for r in [
        "09110", "09120", "09130", "09140", "09150", "09160", "09170", "09180", "09190",
    ] {
        assert!(s.county(r).is_some(), "{r}");
    }
}

#[test]
fn every_county_record_round_trips_through_json() {
    let s = store();
    for c in s.counties() {
        let json = serde_json::to_string(c).unwrap();
        let back: CountyRecord =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{}: {e}", c.fips));
        assert_eq!(&back, c);
    }
}

#[test]
fn loading_order_does_not_matter() {
    let m = manifest();
    let mut s = DataStore::new();
    let mut paths = file_paths(&m);
    paths.reverse();
    for p in &paths {
        s.load_pack(p, &std::fs::read(data_dir().join(p)).unwrap())
            .unwrap();
    }
    // Manifest last: files are checked when it arrives.
    s.load_pack(
        "manifest.json",
        &std::fs::read(data_dir().join("manifest.json")).unwrap(),
    )
    .unwrap();
    let a: Vec<&CountyRecord> = s.counties().collect();
    let b: Vec<&CountyRecord> = store().counties().collect();
    assert_eq!(a, b);
}

#[test]
fn philadelphia_record_has_the_expected_shape() {
    let s = store();
    let c = s.county("42101").unwrap();
    assert_eq!(c.name, "Philadelphia");
    assert_eq!(c.nca_region, "northeast");
    assert!(c.population.unwrap() > 1_500_000);
    assert!(c.households.unwrap() > 600_000);
    assert!(c.nri_version.starts_with("1.20"));
    let hw = &c.nri[&HazardId::HeatWave];
    assert_eq!(hw.afreq_kind, AfreqKind::EventsPerYear);
    assert!((hw.afreq.unwrap() - 11.07).abs() < 0.01);
    assert_eq!(
        c.nri[&HazardId::Wildfire].afreq_kind,
        AfreqKind::AnnualProbability
    );
    assert_eq!(
        c.nri[&HazardId::Earthquake].afreq_kind,
        AfreqKind::AnnualProbability
    );
    let o = c.outages.as_ref().unwrap();
    assert!(
        o.events_per_customer_year > 0.0
            && o.p_ge_1d <= 1.0
            && o.p_ge_14d <= o.p_ge_7d
            && o.p_ge_7d <= o.p_ge_3d
            && o.p_ge_3d <= o.p_ge_1d
    );
    assert!(o.median_hours <= o.p90_hours);
    assert!(o.event_definition.contains("1%"));
    let sz = c.seismic.as_ref().unwrap();
    assert!(sz.p_pga_ge_0_2g_per_year < sz.p_pga_ge_0_1g_per_year);
    assert!(c.events.contains_key("winter_storm"));
    assert!(c.flood.is_some() && c.facilities.is_some() && c.vulnerability.is_some());
}

#[test]
fn search_finds_counties_by_name_code_and_state() {
    let s = store();
    assert_eq!(s.search("phila")[0].fips, "42101");
    assert_eq!(s.search("42101")[0].fips, "42101");
    assert_eq!(s.search("Cook, IL")[0].fips, "17031");
    assert_eq!(s.search("cook county, illinois")[0].fips, "17031");
    assert_eq!(
        s.search("Harris")[0].fips,
        "48201",
        "the most populous Harris first"
    );
    assert!(s.search("42").iter().all(|c| c.fips.starts_with("42")));
    assert!(s.search("zzzz").is_empty());
    assert!(s.search("Doña Ana").iter().any(|c| c.fips == "35013"));
    assert!(s.search("").is_empty());
    assert!(s.search("a").len() <= rr_data::MAX_RESULTS);
}

#[test]
fn climate_multipliers_are_projections_with_sane_bounds() {
    let s = store();
    assert_eq!(
        s.climate_multiplier("42101", HazardId::HeatWave, ClimateHorizon::Today),
        1.0
    );
    let heat = s.climate_multiplier("42101", HazardId::HeatWave, ClimateHorizon::Y2050);
    assert!(heat > 1.0 && heat <= rr_data::CLIMATE_CLAMP.1, "{heat}");
    // Fewer very cold nights in a warmer climate.
    assert!(s.climate_multiplier("17031", HazardId::ColdWave, ClimateHorizon::Y2050) < 1.0);
    // No county projection outside the contiguous US: no change.
    assert_eq!(
        s.climate_multiplier("02020", HazardId::HeatWave, ClimateHorizon::Y2050),
        1.0
    );
    assert_eq!(
        s.climate_multiplier("42101", HazardId::Tornado, ClimateHorizon::Y2050),
        1.0
    );
}

#[test]
fn base_rates_carry_sources_and_match_the_research() {
    let s = store();
    let fire = s.base_rate("house_fire_per_household_year").unwrap();
    assert!((fire.value - 0.002622).abs() < 1e-6);
    assert_eq!(fire.source.as_str(), "usfa_residential_fires");
    let p = s.base_rate("pandemic_onset_per_year").unwrap();
    assert!(p.low.unwrap() < p.value && p.value < p.high.unwrap());
    let pubs: BTreeSet<&str> = s.publications().iter().map(|p| p.id.as_str()).collect();
    for r in s.base_rates() {
        assert!(
            pubs.contains(r.source.as_str()),
            "{} cites {}",
            r.id,
            r.source.as_str()
        );
        assert!(!r.note.is_empty());
    }
}

#[test]
fn attributions_include_required_credit_lines() {
    let s = store();
    let a = s.attributions();
    let nri = a
        .iter()
        .find(|x| x.source == "FEMA National Risk Index")
        .expect("NRI attribution");
    assert!(nri.text.contains("is not endorsed by FEMA"));
    assert!(nri.text.contains("v1.20"));
    assert!(
        a.iter()
            .any(|x| x.source == "ORNL EAGLE-I" && x.text.contains("CC BY 4.0"))
    );
    assert!(a.iter().any(|x| x.text.contains("OpenFEMA")));
    assert!(a.iter().any(|x| x.source == "NCA5 Atlas and LOCA2"));
}
