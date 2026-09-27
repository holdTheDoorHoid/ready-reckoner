//! Job `hospitals` — hospitals with emergency services, from CMS's *Hospital General Information*
//! dataset (data.cms.gov Provider Data Catalog, dataset `xubh-q36u`, public domain): for the
//! binder's Neighbourhood page (DESIGN-DELTA-v3 §8), a county's nearby emergency rooms with a
//! phone number, "check before you need it".
//!
//! The dataset's own download link is a content-hashed path that changes whenever CMS refreshes
//! the file, so the job first reads the stable metastore record for the dataset id and follows
//! its `distribution[0].downloadURL` (the same pattern CMS's Provider Data Catalog expects of any
//! scripted reader); the metastore record's `released` date is what the binder should print as
//! "hospital list current as of".
//!
//! County FIPS comes from the dataset's own `State` and `County/Parish` columns, matched against
//! the canonical county list (a light suffix normalisation handles "County"/"Parish"/"Borough"
//! etc.); a name that still does not match (Connecticut's old counties, a handful of typos and
//! retired Alaska boroughs, DC's "The District") falls back to the hospital's ZIP code through
//! `zip_county.csv`'s largest-share county. A row whose ZIP is not in `zip_county.csv` either
//! (a handful of institutional ZIPs with no ZCTA) is dropped and counted in a note.
//!
//! `fips,ccn,name,city,state,zip,phone,emergency_services,type`: `ccn` (CMS's own Medicare
//! provider number) is not in the brief's column list but is added because it is the row's only
//! guaranteed-unique field — seven hospitals nationwide share their exact name, city and county
//! with a second CMS record (a re-designation under a new CCN, most often Critical Access
//! Hospital status), so `(fips, name)` alone collides and the pack's `sort_and_check` would fail
//! without it.

use super::{Ctx, JobOutput, PUBLIC_DOMAIN, load_counties, source_from};
use crate::csvout::{Table, col, parse_delimited, read_table};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};

/// County hospitals with emergency services.
pub const HOSPITALS: &str = "places/hospitals.csv";

/// The dataset's stable id in CMS's Provider Data Catalog (data.cms.gov); its download link is
/// re-hashed on every refresh, so the job resolves it through this id each run.
const DATASET_ID: &str = "xubh-q36u";
const METASTORE: &str =
    "https://data.cms.gov/provider-data/api/1/metastore/schemas/dataset/items/xubh-q36u";
const PAGE: &str = "https://data.cms.gov/provider-data/dataset/xubh-q36u";

/// Suffixes stripped (case-insensitively) before matching a dataset county name against the
/// canonical list's `name` column.
const COUNTY_SUFFIXES: &[&str] = &[
    " county",
    " parish",
    " borough",
    " census area",
    " municipio",
    " municipality",
    " city and borough",
];

fn normalize_county(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    for suf in COUNTY_SUFFIXES {
        if let Some(stripped) = lower.strip_suffix(suf) {
            return stripped.trim().to_string();
        }
    }
    lower
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();

    // The dataset's current download link and release date, via the stable metastore id.
    let meta = ctx.http.get(
        METASTORE,
        Some("hospitals/metastore_hospital_general_information.json"),
    )?;
    let meta_json: serde_json::Value = serde_json::from_slice(&meta.bytes)
        .map_err(|e| data_err(format!("CMS metastore record for {DATASET_ID}: {e}")))?;
    let download_url = meta_json["distribution"][0]["downloadURL"]
        .as_str()
        .ok_or_else(|| data_err("CMS metastore record has no distribution[0].downloadURL"))?
        .to_string();
    let released = meta_json["released"].as_str().unwrap_or("").to_string();
    out.source(source_from(
        "CMS Provider Data Catalog: dataset metadata (Hospital General Information)",
        &meta,
        format!("dataset {DATASET_ID}, released {released}"),
        PUBLIC_DOMAIN,
        "",
    ));

    let f = ctx.http.get(
        &download_url,
        Some("hospitals/hospital_general_information.csv"),
    )?;
    out.source(source_from(
        "CMS Hospital General Information",
        &f,
        format!("released {released}"),
        PUBLIC_DOMAIN,
        "",
    ));
    let (h, rows) = parse_delimited(&f.text(), b',')?;
    out.rows_in += rows.len() as u64;
    let (i_ccn, i_name, i_addr, i_city, i_state, i_zip, i_county, i_phone, i_emerg, i_type) = (
        col(&h, "Facility ID")?,
        col(&h, "Facility Name")?,
        col(&h, "Address")?,
        col(&h, "City/Town")?,
        col(&h, "State")?,
        col(&h, "ZIP Code")?,
        col(&h, "County/Parish")?,
        col(&h, "Telephone Number")?,
        col(&h, "Emergency Services")?,
        col(&h, "Hospital Type")?,
    );
    let _ = i_addr; // The street address is not in the shipped columns; kept for documentation.

    let counties = load_counties(ctx)?;
    let mut by_name: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut by_norm: BTreeMap<(String, String), String> = BTreeMap::new();
    for c in &counties {
        by_name.insert(
            (c.state_abbr.clone(), c.name.to_lowercase()),
            c.fips.clone(),
        );
        by_norm
            .entry((c.state_abbr.clone(), normalize_county(&c.name)))
            .or_insert_with(|| c.fips.clone());
    }

    let (zh, zrows) = read_table(&ctx.data, super::geography::ZIP_COUNTY)?;
    let (z_zip, z_fips, z_share) = (
        col(&zh, "zip")?,
        col(&zh, "county_fips")?,
        col(&zh, "land_share")?,
    );
    let mut zip_best: BTreeMap<String, (String, f64)> = BTreeMap::new();
    for r in &zrows {
        let share: f64 = r[z_share].parse().unwrap_or(0.0);
        zip_best
            .entry(r[z_zip].clone())
            .and_modify(|(fips, best)| {
                if share > *best || (share == *best && r[z_fips] < *fips) {
                    *fips = r[z_fips].clone();
                    *best = share;
                }
            })
            .or_insert((r[z_fips].clone(), share));
    }

    let mut table = Table::new(
        &[
            "fips",
            "ccn",
            "name",
            "city",
            "state",
            "zip",
            "phone",
            "emergency_services",
            "type",
        ],
        2,
    );
    let mut kept = 0u64;
    let mut by_name_matches = 0u64;
    let mut by_norm_matches = 0u64;
    let mut by_zip_matches = 0u64;
    let mut covered: BTreeSet<String> = BTreeSet::new();
    let mut unplaced: Vec<(String, String, String, String)> = Vec::new();
    for r in &rows {
        if !r[i_emerg].eq_ignore_ascii_case("yes") {
            continue;
        }
        let state = r[i_state].trim().to_string();
        let county = r[i_county].trim().to_string();
        let zip = r[i_zip].trim().to_string();
        let fips = by_name
            .get(&(state.clone(), county.to_lowercase()))
            .cloned()
            .inspect(|_| by_name_matches += 1)
            .or_else(|| {
                by_norm
                    .get(&(state.clone(), normalize_county(&county)))
                    .cloned()
                    .inspect(|_| by_norm_matches += 1)
            })
            .or_else(|| {
                zip_best
                    .get(&zip)
                    .map(|(f, _)| f.clone())
                    .inspect(|_| by_zip_matches += 1)
            });
        let Some(fips) = fips else {
            unplaced.push((
                r[i_ccn].clone(),
                r[i_name].clone(),
                format!("{state}/{county}"),
                zip,
            ));
            continue;
        };
        covered.insert(fips.clone());
        table.push(vec![
            fips,
            r[i_ccn].clone(),
            r[i_name].trim().to_string(),
            r[i_city].trim().to_string(),
            state,
            zip,
            r[i_phone].trim().to_string(),
            "true".to_string(),
            r[i_type].trim().to_string(),
        ]);
        kept += 1;
    }
    if kept == 0 {
        return Err(data_err(
            "hospitals: no row placed in any county; the source's columns may have changed",
        ));
    }
    out.table(ctx, HOSPITALS, &mut table)?;
    // Most rural counties genuinely have none: this is `verify`'s "county-keyed file must cover
    // every county or explain why" invariant, not a data gap, so every uncovered county gets the
    // same plain reason (the Neighbourhood page can show it as is).
    out.missing = super::missing_groups(&counties, &covered, |_| {
        "No hospital with emergency services in this county (CMS Hospital General Information)."
            .into()
    });
    out.notes.push(format!(
        "{kept} hospitals with emergency services placed by county ({by_name_matches} by county name, {by_norm_matches} after stripping \"County\"/\"Parish\"/\"Borough\", {by_zip_matches} by their ZIP code's largest-share county); {} could not be placed (a ZIP with no ZCTA in zip_county.csv, mostly hospital-specific ZIPs): {}.",
        unplaced.len(),
        unplaced
            .iter()
            .map(|(ccn, name, sc, zip)| format!("{ccn} {name} ({sc}, ZIP {zip})"))
            .collect::<Vec<_>>()
            .join("; ")
    ));
    out.definitions.insert(
        "emergency_services".into(),
        "Always \"true\": the source lists non-emergency hospitals too, and this pack keeps only the rows with emergency services.".into(),
    );
    out.definitions.insert(
        "released".into(),
        format!(
            "CMS released this dataset on {released} (data.cms.gov, dataset {DATASET_ID}, {PAGE})."
        ),
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_county_strips_known_suffixes() {
        assert_eq!(normalize_county("Autauga County"), "autauga");
        assert_eq!(normalize_county("St. Bernard Parish"), "st. bernard");
        assert_eq!(normalize_county("Denali Borough"), "denali");
        assert_eq!(normalize_county("The District"), "the district");
        assert_eq!(normalize_county("Bethel Census Area"), "bethel");
    }
}
