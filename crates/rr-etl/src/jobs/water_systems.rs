//! Job — drinking-water systems with health-based violations, per county (EPA ECHO SDWA data,
//! public domain).
//!
//! - **System list** (`SDWA_system_search_download.zip`, refreshed weekly): every public water
//!   system with its type, activity status, population served and the county FIPS codes it
//!   serves. Kept: active community water systems (`PWS_TYPE_CODE = CWS`, `PWS_ACTIVITY_CODE = A`).
//! - **Violations** (`SDWA_latest_downloads.zip`, refreshed quarterly; 424 MB, the violations
//!   table alone 4 GB uncompressed): streamed from a temporary copy under `data/raw/water/`, never
//!   extracted. A system counts as violating when it has at least one health-based violation
//!   (`IS_HEALTH_BASED_IND = Y`: a maximum contaminant level, maximum residual disinfectant level
//!   or treatment-technique violation) whose non-compliance period overlaps the five years before
//!   the file's latest recorded begin date: it began in that window, or began earlier and was
//!   still open in it (an open period's end date reads `--->`).
//!
//! Per county: each system's population is split over the counties it lists in proportion to
//! their residents, then
//! `sdwis_violation_pop_share` = population served by violating systems / population served by
//! all community systems, and `cws_pop_share` = community-system population / county population
//! (capped at 1; the rest are mostly on private wells, which the violation data never see).
//! Violations measure regulatory compliance, not physical fragility: Asheville had none before
//! Helene broke its mains, and one long-running treatment-technique case flags all of New York
//! City. Use as a bounded multiplier, never as a stand-alone warning.

use super::{Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::{Table, col, read_table};
use crate::ct::{Crosswalk, is_old_ct, successors};
use crate::manifest::{Attribution, SourceRecord};
use crate::num::sig;
use crate::timefmt::days_from_civil;
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::{BufReader, Read};

/// Violation shares per county.
pub const WATER_SYSTEMS: &str = "core/water_systems.csv";

const SYSTEMS_URL: &str =
    "https://echo.epa.gov/files/echodownloads/SDWA_system_search_download.zip";
const LATEST_URL: &str = "https://echo.epa.gov/files/echodownloads/SDWA_latest_downloads.zip";
const ECHO_PAGE: &str = "https://echo.epa.gov/tools/data-downloads/sdwa-download-summary";

/// Years of violation history counted.
pub const WINDOW_YEARS: i64 = 5;

/// One active community water system.
#[derive(Debug, Clone, PartialEq)]
pub struct System {
    /// PWSID.
    pub id: String,
    /// Population served.
    pub population: f64,
    /// County FIPS codes served (five digits; state-only codes dropped).
    pub counties: Vec<String>,
}

/// Parse the system-search CSV (a reader over the zip entry), keeping active community systems.
/// Returns the systems and the number with no county code.
pub fn parse_systems(reader: impl Read) -> Result<(Vec<System>, usize, f64)> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::with_capacity(1 << 20, reader));
    let h: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let (i_id, i_type, i_act, i_pop, i_fips) = (
        col(&h, "PWSID")?,
        col(&h, "PWS_TYPE_CODE")?,
        col(&h, "PWS_ACTIVITY_CODE")?,
        col(&h, "POPULATION_SERVED_COUNT")?,
        col(&h, "FIPS_CODES")?,
    );
    let mut out = Vec::new();
    let (mut no_county, mut no_county_pop) = (0usize, 0.0);
    let mut rec = csv::StringRecord::new();
    while rdr.read_record(&mut rec)? {
        if rec[i_type].trim() != "CWS" || rec[i_act].trim() != "A" {
            continue;
        }
        let population = rec[i_pop].trim().parse::<f64>().unwrap_or(0.0).max(0.0);
        let counties: Vec<String> = rec[i_fips]
            .split(',')
            .map(str::trim)
            .filter(|c| c.len() == 5 && c.bytes().all(|b| b.is_ascii_digit()))
            .map(str::to_string)
            .collect();
        if counties.is_empty() {
            no_county += 1;
            no_county_pop += population;
            continue;
        }
        out.push(System {
            id: rec[i_id].trim().to_string(),
            population,
            counties,
        });
    }
    Ok((out, no_county, no_county_pop))
}

/// Parse an ECHO date (`MM/DD/YYYY`, or ISO `YYYY-MM-DD`) into days since 1970-01-01.
pub fn parse_date(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.len() >= 10 && t.as_bytes()[2] == b'/' {
        let (m, d, y) = (
            t[0..2].parse().ok()?,
            t[3..5].parse().ok()?,
            t[6..10].parse().ok()?,
        );
        return Some(days_from_civil(y, m, d));
    }
    if t.len() >= 10 && t.as_bytes()[4] == b'-' {
        let (y, m, d) = (
            t[0..4].parse().ok()?,
            t[5..7].parse().ok()?,
            t[8..10].parse().ok()?,
        );
        return Some(days_from_civil(y, m, d));
    }
    None
}

/// A health-based violation: PWSID, the day non-compliance began, and the day it ended (`None`
/// while still open: the table writes `--->` or leaves it blank).
pub type Violation = (String, i64, Option<i64>);

/// Health-based violations from the violations table (`IS_HEALTH_BASED_IND = Y`). The table has
/// one row per violation and enforcement action, so a violation can repeat; that does not change
/// which systems are flagged.
pub fn health_violations(reader: impl Read) -> Result<Vec<Violation>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::with_capacity(1 << 20, reader));
    let h: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let (i_id, i_hb, i_begin, i_end) = (
        col(&h, "PWSID")?,
        col(&h, "IS_HEALTH_BASED_IND")?,
        col(&h, "NON_COMPL_PER_BEGIN_DATE")?,
        col(&h, "NON_COMPL_PER_END_DATE")?,
    );
    let mut out = Vec::new();
    let mut rec = csv::ByteRecord::new();
    let date = |rec: &csv::ByteRecord, i: usize| {
        rec.get(i)
            .and_then(|b| std::str::from_utf8(b).ok())
            .and_then(parse_date)
    };
    while rdr.read_byte_record(&mut rec)? {
        if rec.get(i_hb) != Some(b"Y") {
            continue;
        }
        if let (Some(id), Some(begin)) = (rec.get(i_id), date(&rec, i_begin)) {
            out.push((
                String::from_utf8_lossy(id).trim().to_string(),
                begin,
                date(&rec, i_end),
            ));
        }
    }
    Ok(out)
}

/// Split each system's population over its counties in proportion to their residents (EPA
/// publishes no split; Connecticut's retired counties go to planning regions by land share first,
/// renamed codes to their successors). Returns (all, violating) population by 2024 county, and
/// the population whose county codes matched nothing.
pub fn county_population(
    systems: &[System],
    violating: &HashSet<String>,
    canon: &BTreeSet<String>,
    cw: &Crosswalk,
    residents: &BTreeMap<String, f64>,
) -> (BTreeMap<String, (f64, f64)>, f64) {
    let mut out: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut unplaced = 0.0;
    for s in systems {
        let bad = violating.contains(&s.id);
        let mut targets_all: Vec<(String, f64)> = Vec::new();
        for code in &s.counties {
            let targets: Vec<(String, f64)> = if canon.contains(code) {
                vec![(code.clone(), 1.0)]
            } else if is_old_ct(code) {
                cw.overlaps
                    .iter()
                    .filter(|o| &o.old == code)
                    .map(|o| (o.region.clone(), o.share_of_old))
                    .collect()
            } else if let Some(new) = successors(code) {
                new.iter()
                    .map(|n| (n.to_string(), 1.0 / new.len() as f64))
                    .collect()
            } else {
                Vec::new()
            };
            targets_all.extend(targets);
        }
        // Weight each listed county by its residents (equal weights when none are known).
        let weighted: Vec<(String, f64)> = targets_all
            .iter()
            .map(|(t, w)| (t.clone(), w * residents.get(t).copied().unwrap_or(0.0)))
            .collect();
        let total: f64 = weighted.iter().map(|(_, w)| w).sum();
        let parts: Vec<(String, f64)> = if total > 0.0 {
            weighted.into_iter().map(|(t, w)| (t, w / total)).collect()
        } else {
            let base: f64 = targets_all.iter().map(|(_, w)| w).sum();
            targets_all
                .into_iter()
                .map(|(t, w)| (t, if base > 0.0 { w / base } else { 0.0 }))
                .collect()
        };
        if parts.is_empty() {
            unplaced += s.population;
            continue;
        }
        for (t, w) in parts {
            let e = out.entry(t).or_insert((0.0, 0.0));
            e.0 += s.population * w;
            if bad {
                e.1 += s.population * w;
            }
        }
    }
    (out, unplaced)
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let cw = Crosswalk::load(&ctx.data)?;

    // --- Systems ----------------------------------------------------------------------------
    let a = ctx
        .http
        .get(SYSTEMS_URL, Some("water/SDWA_system_search_download.zip"))?;
    out.source(super::source_from(
        "EPA ECHO SDWA system search download (all public water systems)",
        &a,
        match &a.last_modified {
            Some(lm) => format!("SDWA_SYSTEM_SEARCH.csv; Last-Modified {lm}"),
            None => "SDWA_SYSTEM_SEARCH.csv".to_string(),
        },
        super::PUBLIC_DOMAIN,
        "",
    ));
    let (systems, no_county, no_county_pop) = {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&a.bytes))?;
        let idx = (0..zip.len())
            .find(|&i| zip.by_index(i).is_ok_and(|e| e.name().ends_with(".csv")))
            .ok_or_else(|| data_err("ECHO system search zip has no CSV"))?;
        parse_systems(zip.by_index(idx)?)?
    };
    out.rows_in += systems.len() as u64;
    let served: f64 = systems.iter().map(|s| s.population).sum();
    if !(300e6..=360e6).contains(&served) {
        return Err(data_err(format!(
            "active community water systems serve {served:.0} people; expected 300-360 million"
        )));
    }

    // --- Violations (streamed from a temporary copy of the quarterly archive) ----------------
    let path = ctx
        .http
        .raw_dir
        .join("water")
        .join("SDWA_latest_downloads.zip");
    let streamed = ctx.http.download_to(LATEST_URL, &path)?;
    out.source(SourceRecord {
        name: "EPA ECHO SDWA latest downloads: SDWA_VIOLATIONS_ENFORCEMENT.csv".into(),
        url: streamed.final_url.clone(),
        version: "quarterly national download".into(),
        retrieved: streamed.retrieved.clone(),
        sha256: streamed.sha256.clone(),
        bytes: streamed.bytes,
        license: super::PUBLIC_DOMAIN.into(),
        obligations: String::new(),
    });
    let violations = (|| -> Result<Vec<Violation>> {
        let mut zip = zip::ZipArchive::new(BufReader::new(std::fs::File::open(&path)?))?;
        let idx = (0..zip.len())
            .find(|&i| {
                zip.by_index(i).is_ok_and(|e| {
                    e.name()
                        .to_ascii_uppercase()
                        .contains("VIOLATIONS_ENFORCEMENT")
                })
            })
            .ok_or_else(|| {
                data_err("ECHO latest downloads: no SDWA_VIOLATIONS_ENFORCEMENT entry")
            })?;
        health_violations(zip.by_index(idx)?)
    })();
    if !ctx.http.keep_raw {
        crate::http::remove_raw(&path)?;
    }
    let violations = violations?;
    out.rows_in += violations.len() as u64;
    // The window ends on the latest begin date the file records (ignoring dates after the day it
    // was fetched, which are data-entry errors), so the result depends only on the file.
    let today = crate::timefmt::parse_datetime(&streamed.retrieved.replace('T', " "))
        .map(|s| s.div_euclid(86_400))
        .unwrap_or(i64::MAX);
    let data_day = violations
        .iter()
        .map(|(_, d, _)| *d)
        .filter(|d| *d <= today)
        .max()
        .ok_or_else(|| data_err("no health-based violations with a usable date"))?;
    let (y, m, d) = crate::timefmt::civil_from_days(data_day);
    let cutoff = days_from_civil(y - WINDOW_YEARS, m, d.min(28));
    // Overlapping the window: began by its end, and still open or ended after its start.
    let violating: HashSet<String> = violations
        .iter()
        .filter(|(_, begin, end)| *begin <= data_day && end.is_none_or(|e| e >= cutoff))
        .map(|(id, _, _)| id.clone())
        .collect();
    let began_in_window: HashSet<&str> = violations
        .iter()
        .filter(|(_, begin, _)| *begin >= cutoff && *begin <= data_day)
        .map(|(id, _, _)| id.as_str())
        .collect();

    let (h, rows) = read_table(&ctx.data, super::nri::NRI_COUNTIES)?;
    let (i_f, i_p) = (col(&h, "fips")?, col(&h, "population")?);
    let pop: BTreeMap<String, f64> = rows
        .iter()
        .filter_map(|r| Some((r[i_f].clone(), r[i_p].parse::<f64>().ok()?)))
        .collect();
    let (by_county, unplaced) = county_population(&systems, &violating, &canon, &cw, &pop);

    let mut table = Table::new(&["fips", "sdwis_violation_pop_share", "cws_pop_share"], 1);
    let mut covered = BTreeSet::new();
    let mut shares = BTreeMap::new();
    for c in &counties {
        let Some((all, bad)) = by_county.get(&c.fips) else {
            continue;
        };
        if *all <= 0.0 {
            continue;
        }
        let v = (bad / all).clamp(0.0, 1.0);
        let cws = pop
            .get(&c.fips)
            .filter(|p| **p > 0.0)
            .map(|p| (all / p).min(1.0));
        // Three significant figures: population-served counts are self-reported by systems.
        table.push(vec![
            c.fips.clone(),
            sig(v, 3),
            cws.map(|x| sig(x, 3)).unwrap_or_default(),
        ]);
        shares.insert(c.fips.clone(), v);
        covered.insert(c.fips.clone());
    }
    let hinds = shares.get("28049").copied().unwrap_or(0.0);
    if hinds < 0.4 {
        return Err(data_err(format!(
            "Hinds County, MS (Jackson) violation share {hinds:.3}; expected more than 0.4"
        )));
    }
    out.table(ctx, WATER_SYSTEMS, &mut table)?;
    out.missing = missing_groups(&counties, &covered, |_| {
        "No active community water system on record in EPA's SDWA data".to_string()
    });
    let violating_pop: f64 = systems
        .iter()
        .filter(|s| violating.contains(&s.id))
        .map(|s| s.population)
        .sum();
    let began_pop: f64 = systems
        .iter()
        .filter(|s| began_in_window.contains(s.id.as_str()))
        .map(|s| s.population)
        .sum();
    let fmt_day = |day: i64| {
        let (y, m, d) = crate::timefmt::civil_from_days(day);
        format!("{y:04}-{m:02}-{d:02}")
    };
    out.notes.push(format!(
        "{} active community water systems serving {:.1} million people; {} systems ({:.1} million people) had a health-based violation (MCL, MRDL or treatment technique) open at some time between {} and {} (the file's latest begin date). Counting only violations that began in the window would flag {} systems ({:.1} million people). {no_county} systems ({no_county_pop:.0} people) list no county and {unplaced:.0} people sit under county codes that match nothing; both are left out.",
        systems.len(),
        served / 1e6,
        violating.len(),
        violating_pop / 1e6,
        fmt_day(cutoff),
        fmt_day(data_day),
        began_in_window.len(),
        began_pop / 1e6
    ));
    out.notes.push("Each system's population is split over the counties it lists in proportion to their residents (NRI 2020; EPA publishes no split); Connecticut's retired county codes are split to planning regions by land share first. Violations measure compliance, not the physical state of the pipes: one large system flips a whole county (New York City's open treatment-technique case since 2017, the uncovered Hillview Reservoir, flags all five boroughs; Asheville's post-Helene violations flag Buncombe County).".into());
    out.definitions.insert("sdwis_violation_pop_share".into(), format!("Share of the county's community-water-system population served by a system with at least one health-based violation (maximum contaminant level, maximum residual disinfectant level or treatment technique) open at some time in the {WINDOW_YEARS} years to {}.", fmt_day(data_day)));
    out.definitions.insert("cws_pop_share".into(), "Population served by active community water systems / county population (NRI 2020), capped at 1.".into());
    out.attributions.push(Attribution {
        source: "EPA ECHO SDWA".into(),
        text: "Drinking-water system violations from the U.S. Environmental Protection Agency's Enforcement and Compliance History Online (ECHO), Safe Drinking Water Act data downloads.".into(),
        license: "US Government work (public domain)".into(),
        url: ECHO_PAGE.into(),
        version: None,
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systems_filter_and_split_codes() {
        let csv = "\"PWSID\",\"PWS_TYPE_CODE\",\"PWS_ACTIVITY_CODE\",\"POPULATION_SERVED_COUNT\",\"FIPS_CODES\"\n\
\"MS0250008\",\"CWS\",\"A\",\"189673\",\"28049\"\n\
\"MO0000001\",\"CWS\",\"A\",\"1000\",\"29083, 29185\"\n\
\"XX1\",\"TNCWS\",\"A\",\"25\",\"51165\"\n\
\"XX2\",\"CWS\",\"I\",\"50\",\"51165\"\n\
\"XX3\",\"CWS\",\"A\",\"70\",\"09\"\n";
        let (s, none, none_pop) = parse_systems(csv.as_bytes()).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[1].counties, vec!["29083", "29185"]);
        assert_eq!((none, none_pop), (1, 70.0));
    }

    #[test]
    fn dates_and_violations_parse() {
        assert_eq!(parse_date("07/09/2026"), Some(days_from_civil(2026, 7, 9)));
        assert_eq!(parse_date("2021-01-31"), Some(days_from_civil(2021, 1, 31)));
        assert_eq!(parse_date(""), None);
        let csv = "PWSID,IS_HEALTH_BASED_IND,NON_COMPL_PER_BEGIN_DATE,NON_COMPL_PER_END_DATE\nMS0250008,Y,10/01/2022,12/31/2022\nMS0250008,N,10/01/2025,\nNY7003493,Y,02/01/2017,--->\nX,Y,,\n";
        let v = health_violations(csv.as_bytes()).unwrap();
        assert_eq!(
            v,
            vec![
                (
                    "MS0250008".to_string(),
                    days_from_civil(2022, 10, 1),
                    Some(days_from_civil(2022, 12, 31))
                ),
                ("NY7003493".to_string(), days_from_civil(2017, 2, 1), None),
            ]
        );
    }

    #[test]
    fn population_splits_over_counties() {
        let systems = vec![
            System {
                id: "A".into(),
                population: 1000.0,
                counties: vec!["29083".into(), "29185".into()],
            },
            System {
                id: "B".into(),
                population: 300.0,
                counties: vec!["29083".into()],
            },
        ];
        let canon: BTreeSet<String> = ["29083", "29185"].iter().map(|s| s.to_string()).collect();
        let bad: HashSet<String> = ["A".to_string()].into_iter().collect();
        // Henry County has three times Ste. Genevieve's residents in this toy.
        let residents: BTreeMap<String, f64> =
            [("29083".to_string(), 3.0), ("29185".to_string(), 1.0)]
                .into_iter()
                .collect();
        let (m, unplaced) =
            county_population(&systems, &bad, &canon, &Crosswalk::default(), &residents);
        assert_eq!(m["29083"], (1050.0, 750.0));
        assert_eq!(m["29185"], (250.0, 250.0));
        assert_eq!(unplaced, 0.0);
        // Without resident counts the split is equal.
        let (m, _) = county_population(
            &systems,
            &bad,
            &canon,
            &Crosswalk::default(),
            &BTreeMap::new(),
        );
        assert_eq!(m["29185"], (500.0, 500.0));
    }
}
