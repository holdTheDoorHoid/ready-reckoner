//! Loader for `places/hospitals.csv` (its own pack, `places`, like `geo`): hospitals with
//! emergency services, for the binder's Neighbourhood page (DESIGN-DELTA-v3 §8).

use std::collections::BTreeMap;

use crate::table::{Csv, b};
use rr_types::EngineError;

/// The pack file this module reads.
pub const FILE: &str = "places/hospitals.csv";

/// One hospital with emergency services (CMS Hospital General Information; only rows with
/// emergency services are in the pack, so `emergency_services` is always `true`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Hospital {
    /// CMS's Medicare provider number (CCN): the row's only guaranteed-unique field.
    pub ccn: String,
    /// Facility name, as CMS lists it.
    pub name: String,
    /// City or town.
    pub city: String,
    /// Two-letter state or territory abbreviation.
    pub state: String,
    /// Five-digit ZIP code.
    pub zip: String,
    /// Telephone number, as CMS lists it.
    pub phone: String,
    /// Always `true` in this pack (see the struct docs).
    pub emergency_services: bool,
    /// CMS's hospital type ("Acute Care Hospitals", "Critical Access Hospitals", ...).
    pub kind: String,
}

/// Parses `places/hospitals.csv` into hospitals by county FIPS, largest name first within a
/// county (CMS's own row order; the file is sorted `fips, ccn`).
pub(crate) fn parse(t: &Csv) -> Result<BTreeMap<String, Vec<Hospital>>, EngineError> {
    let (i_f, i_ccn, i_n, i_c, i_s, i_z, i_p, i_e, i_k) = (
        t.col("fips")?,
        t.col("ccn")?,
        t.col("name")?,
        t.col("city")?,
        t.col("state")?,
        t.col("zip")?,
        t.col("phone")?,
        t.col("emergency_services")?,
        t.col("type")?,
    );
    let mut out: BTreeMap<String, Vec<Hospital>> = BTreeMap::new();
    for r in &t.rows {
        out.entry(r[i_f].clone()).or_default().push(Hospital {
            ccn: r[i_ccn].clone(),
            name: r[i_n].clone(),
            city: r[i_c].clone(),
            state: r[i_s].clone(),
            zip: r[i_z].clone(),
            phone: r[i_p].clone(),
            emergency_services: b(&r[i_e]),
            kind: r[i_k].clone(),
        });
    }
    Ok(out)
}
