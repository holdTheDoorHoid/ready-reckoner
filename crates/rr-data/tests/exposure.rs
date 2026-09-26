//! Data pack v2 exposure columns, checked against known places (the data-hazard brief's list):
//! Minot is class A, Hays class B, Coos Bay class E, Miami and Phoenix C1; Philadelphia has
//! almost no smoke days and Missoula many; Sacramento lives largely behind levees; Hinds County's
//! water system has health-based violations. Each test skips a column whose file is not in the
//! pack yet, so the suite works while the jobs land one by one.

use rr_data::{DataStore, Manifest};
use rr_types::{CountyExposure, StrategicClass};
use std::path::PathBuf;
use std::sync::OnceLock;

fn data_dir() -> PathBuf {
    std::env::var("RR_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"))
}

fn manifest() -> Manifest {
    let bytes = std::fs::read(data_dir().join("manifest.json")).expect("data/manifest.json");
    serde_json::from_slice(&bytes).expect("manifest parses")
}

/// Every file of every pack (core, geo and the optional packs), manifest first.
fn store() -> &'static DataStore {
    static STORE: OnceLock<DataStore> = OnceLock::new();
    STORE.get_or_init(|| {
        let m = manifest();
        let mut files: Vec<(String, Vec<u8>)> = vec![(
            "manifest.json".into(),
            std::fs::read(data_dir().join("manifest.json")).unwrap(),
        )];
        for f in m.packs.values().flat_map(|p| p.files.iter()) {
            let bytes = std::fs::read(data_dir().join(&f.path))
                .unwrap_or_else(|e| panic!("{}: {e}", f.path));
            files.push((f.path.clone(), bytes));
        }
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let mut s = DataStore::new();
        s.load_many(&refs).unwrap_or_else(|e| panic!("{e}"));
        s
    })
}

fn has(path: &str) -> bool {
    manifest().file(path).is_some()
}

fn exposure(fips: &str) -> &'static CountyExposure {
    &store()
        .county(fips)
        .unwrap_or_else(|| panic!("county {fips}"))
        .exposure
}

#[test]
fn strategic_classes_match_the_fixtures() {
    if !has("core/strategic.csv") {
        return;
    }
    for (fips, want, place) in [
        ("38101", StrategicClass::A, "Ward County, ND (Minot)"),
        ("20051", StrategicClass::B, "Ellis County, KS (Hays)"),
        ("41011", StrategicClass::E, "Coos County, OR (Coos Bay)"),
        ("12086", StrategicClass::C1, "Miami-Dade County, FL"),
        ("04013", StrategicClass::C1, "Maricopa County, AZ (Phoenix)"),
        ("42101", StrategicClass::C1, "Philadelphia"),
        (
            "48157",
            StrategicClass::C1,
            "Fort Bend County, TX (Sugar Land)",
        ),
        ("17031", StrategicClass::C1, "Cook County, IL (Chicago)"),
    ] {
        assert_eq!(exposure(fips).strategic_class, Some(want), "{place}");
    }
    // Every county has a class; about 60-70 are A.
    let a = store()
        .counties()
        .filter(|c| c.exposure.strategic_class == Some(StrategicClass::A))
        .count();
    assert!(
        store()
            .counties()
            .all(|c| c.exposure.strategic_class.is_some())
    );
    assert!((50..=90).contains(&a), "{a} class A counties");
}

#[test]
fn strategic_reasons_resolve_to_named_places() {
    if !has("core/strategic_sites.toml") {
        return;
    }
    // Minot: the base and the missile field, both resolved with their plain-language role.
    let minot = exposure("38101");
    let ids: Vec<&str> = minot
        .strategic_site_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert!(
        ids.contains(&"minot_afb") && ids.contains(&"minot_field"),
        "{ids:?}"
    );
    assert_eq!(minot.strategic_places.len(), minot.strategic_site_ids.len());
    let field = minot
        .strategic_places
        .iter()
        .find(|p| p.id == "minot_field")
        .unwrap();
    assert!(field.role_plain.contains("missile"), "{field:?}");
    assert_eq!(field.state, "ND");
    // Hays: downwind of the Warren field, a few hundred km to the north-west.
    let hays = exposure("20051");
    assert_eq!(
        hays.strategic_site_ids.first().map(String::as_str),
        Some("warren_field")
    );
    let km = hays.strategic_km.unwrap();
    assert!((300.0..450.0).contains(&km), "{km}");
    let b = hays.strategic_bearing.unwrap();
    assert!((270.0..330.0).contains(&b), "{b}");
    assert!(hays.strategic_places[0].state.contains("WY"));
    // Philadelphia: its metro, named.
    let phl = exposure("42101");
    assert!(
        phl.strategic_places[0]
            .name
            .starts_with("Philadelphia-Camden-Wilmington")
    );
    assert_eq!(phl.strategic_places[0].kind, "metro");
    // Coos Bay: nothing.
    assert!(exposure("41011").strategic_places.is_empty());
    // The class table carries the priors, labelled as such.
    let t = store().strategic_sites().unwrap();
    assert_eq!(t.class.len(), 6);
    assert!(
        t.class
            .iter()
            .all(|c| c.prior && c.f_s_low <= c.f_s && c.f_s <= c.f_s_high)
    );
}

#[test]
fn uasi_shares_sum_to_one_and_new_york_leads() {
    if !has("core/strategic.csv") {
        return;
    }
    let total: f64 = store()
        .counties()
        .filter_map(|c| c.exposure.uasi_share)
        .map(f64::from)
        .sum();
    assert!((total - 1.0).abs() < 0.002, "{total}");
    // Kings County (Brooklyn) is in New York-White Plains, the largest allocation.
    let kings = exposure("36047");
    assert_eq!(
        kings.uasi_area.as_deref(),
        Some("New York-White Plains, NY")
    );
    // Per resident, the New York-White Plains counties get the most (the share is split by
    // population, so single-county areas like Los Angeles have the largest county share).
    let per_resident = |c: &rr_types::CountyRecord| {
        f64::from(c.exposure.uasi_share.unwrap_or(0.0))
            / f64::from(c.population.unwrap_or(1).max(1))
    };
    let top = store()
        .counties()
        .max_by(|a, b| per_resident(a).total_cmp(&per_resident(b)))
        .unwrap();
    assert!(
        top.fips.starts_with("36"),
        "largest UASI share per resident: {}",
        top.fips
    );
    let la = exposure("06037").uasi_share.unwrap();
    assert!((la - 0.0581).abs() < 0.001, "{la}");
    // A rural county gets none.
    assert_eq!(exposure("41011").uasi_share, Some(0.0));
    assert_eq!(exposure("41011").uasi_area, None);
}

#[test]
fn geomagnetic_factor_rises_to_the_north() {
    if !has("core/geomag.csv") {
        return;
    }
    // NERC alpha: floored at 0.1 in Miami, about 0.29 in Philadelphia, 0.63 in Minot, capped at
    // 1 in Anchorage.
    assert_eq!(exposure("12086").geomag_factor, Some(0.1));
    assert_eq!(exposure("42101").geomag_factor, Some(0.29));
    assert_eq!(exposure("38101").geomag_factor, Some(0.63));
    assert_eq!(exposure("02020").geomag_factor, Some(1.0));
    let lat = exposure("42101").geomag_lat.unwrap();
    assert!((48.5..50.0).contains(&lat), "{lat}");
}

#[test]
fn karst_and_landslide_shares_match_known_ground() {
    if !has("core/ground.csv") {
        return;
    }
    // Central Florida and Kentucky's cave country sit on karst; Philadelphia does not.
    assert!(
        exposure("12083").karst_share.unwrap() > 0.9,
        "Marion County, FL"
    );
    assert!(
        exposure("21227").karst_share.unwrap() > 0.8,
        "Warren County, KY"
    );
    assert!(
        exposure("42101").karst_share.unwrap() < 0.05,
        "Philadelphia"
    );
    // Puerto Rico's mountains are landslide country; South Florida is not.
    assert!(
        exposure("72141").landslide_susceptible_share.unwrap() > 0.9,
        "Utuado"
    );
    assert!(
        exposure("12086").landslide_susceptible_share.unwrap() < 0.05,
        "Miami-Dade"
    );
    // Shares are shares.
    for c in store().counties() {
        for v in [
            c.exposure.karst_share,
            c.exposure.landslide_susceptible_share,
        ]
        .into_iter()
        .flatten()
        {
            assert!((0.0..=1.0).contains(&v), "{}: {v}", c.fips);
        }
    }
}

#[test]
fn levee_shares_match_known_places() {
    if !has("core/levees.csv") {
        return;
    }
    // Sacramento lives largely behind levees (the brief's check); so does St. Charles Parish.
    assert!(
        exposure("06067").leveed_pop_share.unwrap() > 0.2,
        "Sacramento"
    );
    assert!(
        exposure("22089").leveed_pop_share.unwrap() > 0.5,
        "St. Charles Parish"
    );
    assert!(
        exposure("04001").leveed_pop_share.unwrap() < 0.01,
        "Apache County, AZ"
    );
    // Every county has a value (0 where there is no levee), all shares within [0, 1].
    for c in store().counties() {
        let (p, h) = (
            c.exposure.leveed_pop_share,
            c.exposure.levee_risk_high_share,
        );
        assert!(p.is_some() && h.is_some(), "{}", c.fips);
        assert!((0.0..=1.0).contains(&p.unwrap()) && (0.0..=1.0).contains(&h.unwrap()));
    }
}

#[test]
fn water_system_violations_flag_jackson() {
    if !has("core/water_systems.csv") {
        return;
    }
    // Jackson, Mississippi's system (most of Hinds County) had health-based violations.
    assert!(
        exposure("28049").sdwis_violation_pop_share.unwrap() > 0.4,
        "Hinds County, MS"
    );
    // Philadelphia's did not.
    assert_eq!(exposure("42101").sdwis_violation_pop_share, Some(0.0));
    for c in store().counties() {
        for v in [
            c.exposure.sdwis_violation_pop_share,
            c.exposure.cws_pop_share,
        ]
        .into_iter()
        .flatten()
        {
            assert!((0.0..=1.0).contains(&v), "{}: {v}", c.fips);
        }
    }
}
