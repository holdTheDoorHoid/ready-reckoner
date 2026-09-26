//! Job — eviction filings per renter household, per county (Princeton University Eviction Lab,
//! *Estimating Eviction Prevalence across the United States*, county estimates 2000-2018).
//!
//! **Gated.** The data are licensed ODC-BY 1.0: redistribution is allowed with attribution, but
//! the repository's rule 5 asks for the owner's approval before an attribution licence enters the
//! packs. The job writes `core/eviction.csv` only when `data/manifest.json` has
//! `sign_offs.eviction_lab_odc_by.approved = true`; otherwise it downloads nothing, writes
//! nothing (removing an earlier file) and says why in the manifest notes. The attribution the
//! licence requires is recorded here ([`ATTRIBUTION`]) and in `docs/DATA_SOURCES.md`.
//!
//! `eviction_filing_rate` = mean over 2014-2018 of eviction filings / renter households (the
//! Lab's model estimates; about four in five county-years are modelled rather than observed
//! court counts). Filings are not completed evictions, and informal evictions are invisible.

use super::{Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::{Table, col, parse_delimited};
use crate::ct::Crosswalk;
use crate::manifest::{Attribution, Manifest};
use crate::num::sig;
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};

/// Eviction filing rates per county (only with the owner's sign-off).
pub const EVICTION: &str = "core/eviction.csv";

/// The manifest sign-off key this job waits for.
pub const SIGN_OFF: &str = "eviction_lab_odc_by";
/// What the sign-off approves (shown in the manifest).
pub const SIGN_OFF_WHAT: &str = "Ship Eviction Lab county eviction filing rates (ODC-BY 1.0: attribution required; the credit line below goes on the About screen and in the packet sources). Set approved = true, fill in by, and run `rr-etl refresh --out data --only eviction`.";

const URL: &str = "https://eviction-lab-data-downloads.s3.amazonaws.com/estimating-eviction-prevalance-across-us/county_eviction_estimates_2000_2018.csv";
const PAGE: &str = "https://data-downloads.evictionlab.org/";
/// Licence name.
pub const LICENSE: &str = "Open Data Commons Attribution License (ODC-BY 1.0)";
/// The credit line the licence requires: the Lab's own "How to cite" text for the data
/// (README_estimating_eviction.txt, read 2026-09-26) and the licence statement from
/// data-downloads.evictionlab.org ("This data is shared under the terms of the Open Data Commons
/// Attribution License (ODC-BY 1.0).").
pub const ATTRIBUTION: &str = "Eviction filing rates derived by Ready Reckoner from: Ashley Gromis, Ian Fellows, James R. Hendrickson, Lavar Edmonds, Lillian Leung, Adam Porton, and Matthew Desmond. Estimating Eviction Prevalence across the United States. Princeton University Eviction Lab. https://data-downloads.evictionlab.org/#estimating-eviction-prevalance-across-us/. Deposited May 13, 2022. This data is shared under the terms of the Open Data Commons Attribution License (ODC-BY 1.0).";

/// Years averaged.
pub const YEARS: (u32, u32) = (2014, 2018);

/// Mean filings per renter household over [`YEARS`], by county code as the file writes it.
pub fn filing_rates(text: &str) -> Result<BTreeMap<String, f64>> {
    let (h, rows) = parse_delimited(text, b',')?;
    let (i_f, i_y, i_r, i_fe) = (
        col(&h, "FIPS_county")?,
        col(&h, "year")?,
        col(&h, "renting_hh")?,
        col(&h, "filings_estimate")?,
    );
    let mut acc: BTreeMap<String, (f64, u32)> = BTreeMap::new();
    for r in &rows {
        let Ok(y) = r[i_y].parse::<u32>() else {
            continue;
        };
        if y < YEARS.0 || y > YEARS.1 {
            continue;
        }
        let (Ok(renters), Ok(filings)) = (r[i_r].parse::<f64>(), r[i_fe].parse::<f64>()) else {
            continue;
        };
        if renters <= 0.0 {
            continue;
        }
        let fips = format!("{:0>5}", r[i_f].trim());
        let e = acc.entry(fips).or_insert((0.0, 0));
        e.0 += filings / renters;
        e.1 += 1;
    }
    Ok(acc
        .into_iter()
        .filter(|(_, (_, n))| *n > 0)
        .map(|(k, (s, n))| (k, s / n as f64))
        .collect())
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let manifest = Manifest::load_or_default(&ctx.data)?;
    if !manifest.signed_off(SIGN_OFF) {
        out.notes.push(format!(
            "Not built: waiting for the owner's sign-off (manifest sign_offs.{SIGN_OFF}.approved). Eviction Lab data are {LICENSE}; the repository asks for approval before an attribution licence enters the packs. Nothing was downloaded."
        ));
        out.definitions
            .insert("attribution_if_approved".into(), ATTRIBUTION.into());
        return Ok(out);
    }
    let counties = load_counties(ctx)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let cw = Crosswalk::load(&ctx.data)?;
    let f = ctx.http.get(
        URL,
        Some("eviction/county_eviction_estimates_2000_2018.csv"),
    )?;
    out.source(super::source_from(
        "Princeton University Eviction Lab: county eviction estimates 2000-2018",
        &f,
        "Estimating Eviction Prevalence across the United States (deposited 2022-05-13)",
        LICENSE,
        "Attribute (credit line on the About screen and in the packet); keep the licence notice.",
    ));
    let raw = filing_rates(&f.text())?;
    out.rows_in += raw.len() as u64;
    let rates = cw.apply_intensive(raw);
    let mut table = Table::new(&["fips", "eviction_filing_rate"], 1);
    let mut covered = BTreeSet::new();
    for (fips, v) in &rates {
        if !canon.contains(fips) {
            continue;
        }
        table.push(vec![fips.clone(), sig(*v, 3)]);
        covered.insert(fips.clone());
    }
    let values: Vec<f64> = rates
        .iter()
        .filter(|(f, _)| canon.contains(*f))
        .map(|(_, v)| *v)
        .collect();
    let mean = values.iter().sum::<f64>() / values.len().max(1) as f64;
    // Rates above 1 are real where landlords file again and again against the same households
    // (Baltimore County, MD, averages 1.4 filings per renter household a year).
    if !(0.01..=0.10).contains(&mean) || values.iter().any(|v| *v < 0.0 || *v > 3.0) {
        return Err(data_err(format!(
            "eviction filing rates: county mean {mean:.4}, expected 0.01-0.10 and every rate in [0, 3]"
        )));
    }
    out.table(ctx, EVICTION, &mut table)?;
    out.missing = missing_groups(&counties, &covered, |_| {
        "No Eviction Lab county estimate (the file covers the 50 states and DC, 2000-2018)".into()
    });
    out.notes.push(format!(
        "eviction_filing_rate: mean of the Lab's yearly filings_estimate / renting_hh over {}-{}; Connecticut's retired counties converted to planning regions by land share, renamed codes carried to their successors. Most county-years are model estimates; filings are not completed evictions and can exceed one per renter household where landlords file repeatedly (several Maryland counties). The data end in 2018.",
        YEARS.0, YEARS.1
    ));
    out.definitions.insert("eviction_filing_rate".into(), format!("Eviction filings per renter household per year, {}-{} mean (Princeton University Eviction Lab estimates).", YEARS.0, YEARS.1));
    out.attributions.push(Attribution {
        source: "Eviction Lab".into(),
        text: ATTRIBUTION.into(),
        license: LICENSE.into(),
        url: PAGE.into(),
        version: Some("county estimates 2000-2018".into()),
        accessed: f.retrieved[..10].to_string(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filing_rates_average_the_window() {
        let csv = "state,county,FIPS_state,FIPS_county,year,renting_hh,filings_estimate,filings_ci_95_lower\n\
Alabama,Autauga County,01,01001,2013,1000,90,90\n\
Alabama,Autauga County,01,01001,2014,1000,50,50\n\
Alabama,Autauga County,01,01001,2015,1000,30,30\n\
Alabama,Baldwin County,01,1003,2016,0,10,10\n\
Alabama,Barbour County,01,01005,2017,2000,,\n";
        let r = filing_rates(csv).unwrap();
        assert_eq!(r.len(), 1);
        assert!((r["01001"] - 0.04).abs() < 1e-12);
    }
}
