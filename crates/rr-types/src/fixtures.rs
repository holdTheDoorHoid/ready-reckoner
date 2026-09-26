//! The fixture households in `fixtures/households/`, embedded at compile time so every crate can
//! test against them (`rr_types::fixtures::all()`).
//!
//! A test in this crate fails if a JSON file is added to or removed from that directory without
//! updating [`RAW`], and another checks that every fixture parses, round-trips and validates.
//! The web app imports the same files in `web/src/engine/fixtures.ts`.
//!
//! Households staged in `fixtures/households/pending/` ([`PENDING`]) are embedded too, but kept
//! out of [`all`]: the goldens, the sample-county engine and the web app's fixture list cover
//! [`RAW`] only. Moving a staged household up a level (and into [`RAW`] and `fixtures.ts`) is
//! the planner's step, together with its golden files and, for a new county, its sample county.
//! [`get`] finds a household in either list.

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

/// `(name, JSON)` for every household staged in `fixtures/households/pending/`, sorted by name:
/// the v0.1.1 cameron household and the six contract v2 households (a missile-field county, a
/// leveed county, a smoke county, a SNAP household with a family plan, a high-rise in a surge
/// zone, and Puerto Rico).
pub const PENDING: &[(&str, &str)] = &[
    (
        "cameron-insulin-well-farm-2",
        include_str!("../../../fixtures/households/pending/cameron-insulin-well-farm-2.json"),
    ),
    (
        "detroit-snap-3",
        include_str!("../../../fixtures/households/pending/detroit-snap-3.json"),
    ),
    (
        "galveston-highrise-1",
        include_str!("../../../fixtures/households/pending/galveston-highrise-1.json"),
    ),
    (
        "minot-missile-field-3",
        include_str!("../../../fixtures/households/pending/minot-missile-field-3.json"),
    ),
    (
        "missoula-smoke-2",
        include_str!("../../../fixtures/households/pending/missoula-smoke-2.json"),
    ),
    (
        "sacramento-leveed-2",
        include_str!("../../../fixtures/households/pending/sacramento-leveed-2.json"),
    ),
    (
        "san-juan-2",
        include_str!("../../../fixtures/households/pending/san-juan-2.json"),
    ),
];

/// Every staged household, parsed, in the order of [`PENDING`].
///
/// # Panics
///
/// If a household does not parse. This crate's tests guarantee they all do.
pub fn pending() -> Vec<(&'static str, PlanInput)> {
    PENDING
        .iter()
        .map(|&(name, json)| (name, parse(name, json)))
        .collect()
}

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

/// One fixture household by name (the file stem), from [`RAW`] or [`PENDING`], or `None` if
/// there is no such household.
///
/// # Panics
///
/// If the fixture does not parse. This crate's tests guarantee they all do.
pub fn get(name: &str) -> Option<PlanInput> {
    RAW.iter()
        .chain(PENDING)
        .find(|(n, _)| *n == name)
        .map(|&(n, json)| parse(n, json))
}

fn parse(name: &str, json: &str) -> PlanInput {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("fixture {name} does not parse: {e}"))
}
