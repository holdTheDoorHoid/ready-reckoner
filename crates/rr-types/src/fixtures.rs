//! The fixture households in `fixtures/households/`, embedded at compile time so every crate can
//! test against them (`rr_types::fixtures::all()`).
//!
//! A test in this crate fails if a JSON file is added to or removed from that directory without
//! updating [`RAW`], and another checks that every fixture parses, round-trips and validates.
//! The web app imports the same files in `web/src/engine/fixtures.ts`.

use crate::PlanInput;

/// `(name, JSON)` for every fixture household, sorted by name. The name is the file stem.
pub const RAW: &[(&str, &str)] = &[
    (
        "chicago-student-zero-budget-1",
        include_str!("../../../fixtures/households/chicago-student-zero-budget-1.json"),
    ),
    (
        "coos-bay-well-owner-2",
        include_str!("../../../fixtures/households/coos-bay-well-owner-2.json"),
    ),
    (
        "hays-kansas-farm-5",
        include_str!("../../../fixtures/households/hays-kansas-farm-5.json"),
    ),
    (
        "miami-condo-retiree-1",
        include_str!("../../../fixtures/households/miami-condo-retiree-1.json"),
    ),
    (
        "philadelphia-renters-4",
        include_str!("../../../fixtures/households/philadelphia-renters-4.json"),
    ),
    (
        "phoenix-apartment-cpap-1",
        include_str!("../../../fixtures/households/phoenix-apartment-cpap-1.json"),
    ),
    (
        "sugar-land-ev-household-3",
        include_str!("../../../fixtures/households/sugar-land-ev-household-3.json"),
    ),
];

/// Every fixture household, parsed, in the order of [`RAW`].
///
/// # Panics
///
/// If a fixture does not parse. This crate's tests guarantee they all do.
pub fn all() -> Vec<(&'static str, PlanInput)> {
    RAW.iter()
        .map(|&(name, json)| (name, parse(name, json)))
        .collect()
}

/// One fixture household by name (the file stem), or `None` if there is no such fixture.
///
/// # Panics
///
/// If the fixture does not parse. This crate's tests guarantee they all do.
pub fn get(name: &str) -> Option<PlanInput> {
    RAW.iter()
        .find(|(n, _)| *n == name)
        .map(|&(n, json)| parse(n, json))
}

fn parse(name: &str, json: &str) -> PlanInput {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("fixture {name} does not parse: {e}"))
}
