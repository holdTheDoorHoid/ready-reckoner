//! Job 7 — flood priors per county from OpenFEMA (FEMA's public API; redistribution allowed with
//! citation, access date and FEMA's "not endorsed" statement).
//!
//! - **NFIP residential penetration rates** (v1): residential structures, structures in the
//!   Special Flood Hazard Area (the 1%-annual-chance floodplain) and policies in force. The share
//!   of homes in the SFHA is the county prior for "is my home in the flood zone?".
//! - **NFIP redacted claims** (v3; v2 is withdrawn on 2026-10-15): residential claims with a loss
//!   in 1996-2025, counted per county, and the mean amount paid on claims that were paid.
//!
//! `claims_per_1000_policies_year` divides the yearly claim count by the county's residential
//! policies in force today (the only county policy count OpenFEMA publishes in a small form), so
//! it is a rough rate: counties whose insured base changed a lot since 1996 are off accordingly.

use super::{Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::Table;
use crate::ct::{Crosswalk, is_old_ct};
use crate::http::Sha256Acc;
use crate::manifest::{Attribution, SourceRecord};
use crate::num::sig4;
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};

/// Flood pack file.
pub const FLOOD: &str = "core/flood.csv";

const API: &str = "https://www.fema.gov/api/open";
const PENETRATION_PAGE: &str =
    "https://www.fema.gov/openfema-data-page/nfip-residential-penetration-rates-v1";
const CLAIMS_PAGE: &str = "https://www.fema.gov/openfema-data-page/nfip-redacted-claims-v3";

/// FEMA's required statement for OpenFEMA data (OpenFEMA Terms and Conditions, "Citing Data";
/// read from the Internet Archive copy of https://www.fema.gov/about/openfema/terms-conditions,
/// 2026-09-22, because fema.gov refuses scripted clients).
pub const OPENFEMA_STATEMENT: &str = "This product uses the Federal Emergency Management Agency\u{2019}s OpenFEMA API, but is not endorsed by FEMA. The Federal Government or FEMA cannot vouch for the data or analyses derived from these data after the data have been retrieved from the Agency's website(s).";

/// First year of claims counted.
pub const FIRST_YEAR: i64 = 1996;

/// NFIP occupancy types that are residential (v3 data dictionary): 1, 2, 3 (legacy codes) and
/// 11-16 (single family, 2-4 units, 5+ units, mobile home, condominium association, single unit
/// in a multi-unit building).
pub const RESIDENTIAL_OCCUPANCY: &[i64] = &[1, 2, 3, 11, 12, 13, 14, 15, 16];

#[derive(Debug, Default, Clone)]
struct ClaimAcc {
    claims: f64,
    paid_claims: f64,
    paid_total: f64,
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let cw = Crosswalk::load(&ctx.data)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let last_year: i64 = crate::timefmt::today_utc()[..4]
        .parse::<i64>()
        .unwrap_or(2026)
        - 1;

    // --- Penetration rates ---------------------------------------------------------------------
    let mut acc = Sha256Acc::new();
    let started = crate::timefmt::now_utc();
    let mut pen: Vec<serde_json::Value> = Vec::new();
    let mut as_of = String::new();
    loop {
        let url = format!(
            "{API}/v1/NfipResidentialPenetrationRates?$top=1000&$skip={}&$orderby=id",
            pen.len()
        );
        let f = ctx.http.get(&url, None)?;
        acc.update(&f.bytes);
        let v: serde_json::Value = serde_json::from_slice(&f.bytes)?;
        let rows = v["NfipResidentialPenetrationRates"]
            .as_array()
            .ok_or_else(|| data_err("OpenFEMA penetration: no rows"))?;
        let n = rows.len();
        pen.extend(rows.iter().cloned());
        if n < 1000 {
            break;
        }
    }
    if let Some(d) = pen.first().and_then(|r| r["asOfDate"].as_str()) {
        as_of = d[..10.min(d.len())].to_string();
    }
    let bytes = acc.len();
    out.source(SourceRecord {
        name: "OpenFEMA NFIP Residential Penetration Rates v1".into(),
        url: format!("{API}/v1/NfipResidentialPenetrationRates (pages of 1000 ordered by id)"),
        version: format!("v1; asOfDate {as_of}"),
        retrieved: started,
        sha256: acc.finish(),
        bytes,
        license: "OpenFEMA Terms and Conditions (public data; citation and statement required)"
            .into(),
        obligations: format!(
            "Cite {PENETRATION_PAGE} with access date; show: {OPENFEMA_STATEMENT}"
        ),
    });
    out.rows_in += pen.len() as u64;
    struct Pen {
        structures: f64,
        sfha: f64,
        policies: f64,
        policies_sfha: f64,
    }
    let mut pens: BTreeMap<String, Pen> = BTreeMap::new();
    for r in &pen {
        let Some(f) = r["fipsCode"].as_str() else {
            continue;
        };
        let g = |k: &str| r[k].as_f64().unwrap_or(0.0);
        pens.insert(
            f.to_string(),
            Pen {
                structures: g("totalResStructures"),
                sfha: g("totalResStructuresSfha"),
                policies: g("resContractsInForce"),
                policies_sfha: g("resContractsInForceSfha"),
            },
        );
    }

    // --- Claims v3 (keyset pagination on the integer id) ------------------------------------------
    let fields = "id,countyCode,yearOfLoss,occupancyType,amountPaidOnBuildingClaim,amountPaidOnContentsClaim,amountPaidOnIncreasedCostOfComplianceClaim";
    let mut acc = Sha256Acc::new();
    let started = crate::timefmt::now_utc();
    let mut last_id: i64 = -1;
    let mut claims: BTreeMap<String, ClaimAcc> = BTreeMap::new();
    let mut n_claims = 0u64;
    let mut no_county = 0u64;
    let mut pages = 0u64;
    loop {
        let url = format!(
            "{API}/v3/NfipClaims?$select={fields}&$filter=yearOfLoss%20ge%20{FIRST_YEAR}%20and%20yearOfLoss%20le%20{last_year}%20and%20id%20gt%20{last_id}&$orderby=id&$top=10000"
        );
        let f = ctx.http.get(&url, None)?;
        acc.update(&f.bytes);
        pages += 1;
        let v: serde_json::Value = serde_json::from_slice(&f.bytes)?;
        if let Some(e) = v.get("error") {
            return Err(data_err(format!("OpenFEMA claims error: {e}")));
        }
        let rows = v["NfipClaims"]
            .as_array()
            .ok_or_else(|| data_err("OpenFEMA claims: no rows"))?;
        if rows.is_empty() {
            break;
        }
        for r in rows {
            last_id = r["id"].as_i64().unwrap_or(last_id);
            let occ = r["occupancyType"].as_i64().unwrap_or(-1);
            if !RESIDENTIAL_OCCUPANCY.contains(&occ) {
                continue;
            }
            n_claims += 1;
            let Some(code) = r["countyCode"].as_str().filter(|c| c.len() == 5) else {
                no_county += 1;
                continue;
            };
            let paid: f64 = [
                "amountPaidOnBuildingClaim",
                "amountPaidOnContentsClaim",
                "amountPaidOnIncreasedCostOfComplianceClaim",
            ]
            .iter()
            .map(|k| r[*k].as_f64().unwrap_or(0.0).max(0.0))
            .sum();
            let e = claims.entry(code.to_string()).or_default();
            e.claims += 1.0;
            if paid > 0.0 {
                e.paid_claims += 1.0;
                e.paid_total += paid;
            }
        }
        if pages % 25 == 0 {
            eprintln!("    claims pages {pages}, residential claims so far {n_claims}");
        }
        if rows.len() < 10_000 {
            break;
        }
    }
    let bytes = acc.len();
    out.source(SourceRecord {
        name: "OpenFEMA FIMA NFIP Redacted Claims v3".into(),
        url: format!("{API}/v3/NfipClaims?$select={fields}&$filter=yearOfLoss ge {FIRST_YEAR} and yearOfLoss le {last_year} (keyset pages of 10000 ordered by id)"),
        version: format!("v3; {pages} pages"),
        retrieved: started,
        sha256: acc.finish(),
        bytes,
        license: "OpenFEMA Terms and Conditions (public data; citation and statement required)".into(),
        obligations: format!("Cite {CLAIMS_PAGE} with access date; show: {OPENFEMA_STATEMENT}"),
    });
    out.rows_in += n_claims;

    // Connecticut claims use the old counties: split counts by land share, keep means intensive.
    let mut by_county: BTreeMap<String, ClaimAcc> = BTreeMap::new();
    let mut unknown = BTreeSet::new();
    for (code, a) in &claims {
        if canon.contains(code) {
            let e = by_county.entry(code.clone()).or_default();
            e.claims += a.claims;
            e.paid_claims += a.paid_claims;
            e.paid_total += a.paid_total;
        } else if is_old_ct(code) {
            for o in cw.overlaps.iter().filter(|o| &o.old == code) {
                let e = by_county.entry(o.region.clone()).or_default();
                e.claims += a.claims * o.share_of_old;
                e.paid_claims += a.paid_claims * o.share_of_old;
                e.paid_total += a.paid_total * o.share_of_old;
            }
        } else if let Some(new) = crate::ct::successors(code) {
            let k = new.len() as f64;
            for n in new {
                let e = by_county.entry(n.to_string()).or_default();
                e.claims += a.claims / k;
                e.paid_claims += a.paid_claims / k;
                e.paid_total += a.paid_total / k;
            }
        } else {
            unknown.insert(code.clone());
        }
    }

    let years = (last_year - FIRST_YEAR + 1) as f64;
    let mut table = Table::new(
        &[
            "fips",
            "sfha_home_share",
            "claims_per_1000_policies_year",
            "mean_paid_usd",
            "residential_structures",
            "residential_structures_sfha",
            "policies_in_force",
            "sfha_policy_share",
            "claims_per_year",
            "sfha_share_basis",
        ],
        1,
    );
    let mut covered = BTreeSet::new();
    let mut lower_bound = 0u32;
    for c in &counties {
        let p = pens.get(&c.fips);
        let a = by_county.get(&c.fips);
        let Some(p) = p else { continue };
        if p.structures <= 0.0 {
            continue;
        }
        // OpenFEMA leaves the flood-zone structure count at 0 in some counties that clearly have
        // flood-zone homes (they have flood-zone policies). There, insured flood-zone homes /
        // all homes is a lower bound on the share; label it so.
        let (share, basis) = if p.sfha <= 0.0 && p.policies_sfha > 0.0 {
            lower_bound += 1;
            (p.policies_sfha / p.structures, "policies_lower_bound")
        } else {
            (p.sfha / p.structures, "structures")
        };
        let rate = match a {
            Some(a) if p.policies > 0.0 => Some(1000.0 * a.claims / years / p.policies),
            None if p.policies > 0.0 => Some(0.0),
            _ => None,
        };
        let mean_paid = a
            .filter(|a| a.paid_claims >= 1.0)
            .map(|a| a.paid_total / a.paid_claims);
        table.push(vec![
            c.fips.clone(),
            sig4(share),
            rate.map(sig4).unwrap_or_default(),
            mean_paid.map(sig4).unwrap_or_default(),
            sig4(p.structures),
            sig4(p.sfha),
            sig4(p.policies),
            if p.sfha > 0.0 {
                sig4(p.policies_sfha / p.sfha)
            } else {
                String::new()
            },
            a.map(|a| sig4(a.claims / years))
                .unwrap_or_else(|| "0".into()),
            basis.to_string(),
        ]);
        covered.insert(c.fips.clone());
    }
    out.table(ctx, FLOOD, &mut table)?;
    out.missing = missing_groups(&counties, &covered, |c| {
        if super::is_island_territory(&c.state_abbr) {
            "OpenFEMA publishes no residential penetration figures for this island area".to_string()
        } else {
            "OpenFEMA publishes no residential penetration figures for this county".to_string()
        }
    });
    out.notes.push(format!(
        "sfha_home_share = residential structures in the Special Flood Hazard Area / all residential structures (penetration file as of {as_of}). claims_per_1000_policies_year = 1000 x residential NFIP claims with a loss in {FIRST_YEAR}-{last_year} / {years:.0} / residential policies in force today. mean_paid_usd = mean building + contents + increased-cost-of-compliance payment over paid claims (nominal dollars, not inflation-adjusted). {n_claims} residential claims read; {no_county} had no county code."
    ));
    out.notes.push("sfha_policy_share (policies in the flood zone / homes in the flood zone) shows how many flood-zone homes are insured; it can exceed 1 where a policy covers several structures.".into());
    out.notes.push(format!(
        "{lower_bound} counties report flood-zone policies but zero flood-zone structures (a gap in the OpenFEMA penetration file; e.g. St. Tammany Parish, Queens, Virginia Beach). For them sfha_home_share is insured flood-zone homes / all homes, a lower bound (sfha_share_basis = policies_lower_bound)."
    ));
    if !unknown.is_empty() {
        out.notes.push(format!(
            "Claim county codes not in the 2024 county list (skipped): {}.",
            unknown.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    out.definitions
        .insert("openfema_statement".into(), OPENFEMA_STATEMENT.into());
    let date = crate::timefmt::today_utc();
    out.attributions.push(Attribution {
        source: "OpenFEMA (NFIP)".into(),
        text: format!(
            "Federal Emergency Management Agency (FEMA), OpenFEMA Datasets: NFIP Residential Penetration Rates - v1 and FIMA NFIP Redacted Claims - v3. Retrieved from {PENETRATION_PAGE} and {CLAIMS_PAGE} on {date} (UTC). {OPENFEMA_STATEMENT}"
        ),
        license: "OpenFEMA Terms and Conditions".into(),
        url: CLAIMS_PAGE.into(),
        version: Some("Penetration v1, Claims v3".into()),
        accessed: date,
    });
    Ok(out)
}
