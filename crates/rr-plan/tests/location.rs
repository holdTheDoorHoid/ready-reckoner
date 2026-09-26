//! Location: ZIP codes and county codes resolve per docs/ENGINE-API.md, `ambiguous_zip` lists the
//! counties largest first, a county code wins over a ZIP code, and `assess` returns the same
//! location errors.

mod common;

use common::{engine, household};
use rr_plan::{CountySource, Engine, FixtureSource};
use rr_types::{
    Attribution, BaseRate, CountyRecord, ErrorCode, LocationInput, LocationResolved, Setting,
};

fn loc(zip: Option<&str>, county: Option<&str>) -> LocationInput {
    LocationInput {
        country: "US".into(),
        zip: zip.map(str::to_owned),
        county_fips: county.map(str::to_owned),
        setting: Setting::Urban,
    }
}

#[test]
fn fixture_zip_codes_resolve_to_their_counties() {
    let e = engine();
    for (zip, fips) in [
        ("19147", "42101"),
        ("60637", "17031"),
        ("97420", "41011"),
        ("33139", "12086"),
        ("85008", "04013"),
        ("77479", "48157"),
    ] {
        let r = e.resolve_location(&loc(Some(zip), None)).unwrap();
        assert_eq!(r.county_fips, fips);
        assert_eq!(r.zip.as_deref(), Some(zip));
        assert_eq!(r.zip_county_share, Some(1.0));
        assert!(r.data_note.is_some());
    }
    let hays = e.resolve_location(&loc(None, Some("20051"))).unwrap();
    assert_eq!(hays.county_name, "Ellis");
    assert_eq!(hays.zip, None);
}

#[test]
fn a_county_code_wins_over_a_zip_code() {
    let r = engine()
        .resolve_location(&loc(Some("19147"), Some("17031")))
        .unwrap();
    assert_eq!(r.county_fips, "17031");
    // The ZIP code is not in Cook County, so it is not kept.
    assert_eq!(r.zip, None);
}

#[test]
fn unknown_locations_fail_with_their_codes() {
    let e = engine();
    let err = e.resolve_location(&loc(Some("00000"), None)).unwrap_err();
    assert_eq!(err.code, ErrorCode::UnknownZip);
    let err = e.resolve_location(&loc(None, Some("42999"))).unwrap_err();
    assert_eq!(err.code, ErrorCode::UnknownCounty);
    let suggestions = err.suggestions().unwrap();
    assert_eq!(suggestions.len(), 1, "{suggestions:?}");
    assert_eq!(suggestions[0].county_fips, "42101", "same state first");
    let err = e.resolve_location(&loc(None, None)).unwrap_err();
    assert_eq!(err.code, ErrorCode::BadInput);
}

#[test]
fn an_ambiguous_zip_lists_every_county_largest_first() {
    // A made-up ZIP code split between two fixture counties (no real ZIP spans these).
    let source = FixtureSource::new()
        .unwrap()
        .with_zip("99901", &[("17031", 0.35), ("42101", 0.65)]);
    let e = Engine::new(source).unwrap();
    let err = e.resolve_location(&loc(Some("99901"), None)).unwrap_err();
    assert_eq!(err.code, ErrorCode::AmbiguousZip);
    let s = err.suggestions().unwrap();
    let fips: Vec<&str> = s.iter().map(|l| l.county_fips.as_str()).collect();
    assert_eq!(fips, ["42101", "17031"]);
    assert_eq!(s[0].zip_county_share, Some(0.65));
    // assess answers the same, and the pick the app records (the county) then works.
    let mut input = household("philadelphia-renters-4");
    input.location.zip = Some("99901".into());
    assert_eq!(e.assess(&input).unwrap_err().code, ErrorCode::AmbiguousZip);
    input.location.county_fips = Some("42101".into());
    let out = e.assess(&input).unwrap();
    assert_eq!(out.location.county_fips, "42101");
    assert_eq!(out.location.zip.as_deref(), Some("99901"));
    assert_eq!(out.location.zip_county_share, Some(0.65));
}

#[test]
fn eighty_percent_of_a_zip_code_is_enough() {
    let source = FixtureSource::new()
        .unwrap()
        .with_zip("99902", &[("42101", 0.8), ("17031", 0.2)]);
    let e = Engine::new(source).unwrap();
    let r = e.resolve_location(&loc(Some("99902"), None)).unwrap();
    assert_eq!(r.county_fips, "42101");
}

#[test]
fn county_search_finds_names_codes_and_states() {
    let e = engine();
    let first = |q: &str| {
        e.county_search(q)
            .first()
            .map(|l| l.county_fips.clone())
            .unwrap_or_default()
    };
    assert_eq!(first("phila"), "42101");
    assert_eq!(first("Philadelphia County"), "42101");
    assert_eq!(first("42101"), "42101");
    assert_eq!(first("Cook, IL"), "17031");
    assert_eq!(first("miami"), "12086");
    assert!(e.county_search("Cook, TX").is_empty());
    assert!(e.county_search("").is_empty());
    assert!(e.county_search("zzz").is_empty());
    assert!(e.county_search("4").len() <= 10);
}

/// A source with no counties loaded yet.
struct Empty;

impl CountySource for Empty {
    fn county(&self, _: &str) -> Option<&CountyRecord> {
        None
    }
    fn resolve_zip(&self, _: &str) -> Vec<(String, f32)> {
        Vec::new()
    }
    fn search(&self, _: &str) -> Vec<LocationResolved> {
        Vec::new()
    }
    fn base_rates(&self) -> &[BaseRate] {
        &[]
    }
    fn attributions(&self) -> Vec<Attribution> {
        Vec::new()
    }
    fn location(&self, _: &str, _: Option<&str>) -> Option<LocationResolved> {
        None
    }
    fn pack_version(&self) -> Option<String> {
        None
    }
    fn packs_loaded(&self) -> Vec<String> {
        Vec::new()
    }
    fn has_counties(&self) -> bool {
        false
    }
}

#[test]
fn before_any_pack_loads_assess_is_pack_missing() {
    let e = Engine::new(Empty).unwrap();
    let err = e.assess(&household("philadelphia-renters-4")).unwrap_err();
    assert_eq!(err.code, ErrorCode::PackMissing);
    assert_eq!(e.engine_info().data_pack_version, None);
}

#[test]
fn bad_input_lists_its_problems() {
    let mut input = household("philadelphia-renters-4");
    input.people.clear();
    let err = engine().assess(&input).unwrap_err();
    assert_eq!(err.code, ErrorCode::BadInput);
    assert!(!err.problems().unwrap().is_empty());
}
