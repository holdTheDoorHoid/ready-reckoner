//! Shared helpers: the fixture counties and base rates in `tests/data`, and lookups.
#![allow(dead_code)]

use rr_hazards::HazardAssessment;
use rr_types::{
    BaseRate, CountyRecord, HazardId, HazardProfile, HazardTier, HouseholdEventRate,
    LocationResolved, PlanInput,
};

/// A fixture county: the record and the resolved location.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Fixture {
    pub county: CountyRecord,
    pub location: LocationResolved,
}

/// Every fixture household with the county it lives in.
pub const PAIRS: &[(&str, &str)] = &[
    ("chicago-student-zero-budget-1", "17031"),
    ("coos-bay-well-owner-2", "41011"),
    ("hays-kansas-farm-5", "20051"),
    ("miami-condo-retiree-1", "12086"),
    ("philadelphia-renters-4", "42101"),
    ("phoenix-apartment-cpap-1", "04013"),
    ("sugar-land-ev-household-3", "48157"),
];

fn data_dir() -> String {
    format!("{}/tests/data", env!("CARGO_MANIFEST_DIR"))
}

pub fn county(fips: &str) -> Fixture {
    let path = format!("{}/counties/{fips}.json", data_dir());
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("{path}: {e}"))
}

pub fn base_rates() -> Vec<BaseRate> {
    let path = format!("{}/base_rates.json", data_dir());
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

pub fn household(name: &str) -> PlanInput {
    rr_types::fixtures::get(name).unwrap_or_else(|| panic!("no fixture household {name}"))
}

pub fn run(input: &PlanInput, fixture: &Fixture) -> HazardAssessment {
    rr_hazards::assess(input, &fixture.county, &base_rates(), &fixture.location)
}

/// The assessment for a fixture household in its fixture county.
pub fn assess(name: &str, fips: &str) -> HazardAssessment {
    run(&household(name), &county(fips))
}

pub fn profile(a: &HazardAssessment, id: HazardId) -> &HazardProfile {
    a.profiles
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("no profile for {id}"))
}

pub fn has_profile(a: &HazardAssessment, id: HazardId) -> bool {
    a.profiles.iter().any(|p| p.id == id)
}

pub fn rate(a: &HazardAssessment, id: HazardId) -> &HouseholdEventRate {
    a.rates
        .iter()
        .find(|r| r.hazard == id)
        .unwrap_or_else(|| panic!("no rate for {id}"))
}

/// Natural-hazard profiles, most likely first.
pub fn natural(a: &HazardAssessment) -> Vec<&HazardProfile> {
    let mut v: Vec<&HazardProfile> = a
        .profiles
        .iter()
        .filter(|p| p.tier == HazardTier::Natural)
        .collect();
    v.sort_by(|x, y| y.rate_per_year.total_cmp(&x.rate_per_year));
    v
}

pub fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs())
}

/// Chance of at least one event in `years` years at `rate` a year, out of 100.
pub fn per_100(rate: f64, years: f64) -> f64 {
    100.0 * (1.0 - rr_types::math::exp(-rate * years))
}
