//! `rr-etl verify`: re-check the packs against the manifest and check that counties join.
//!
//! Checks:
//! 1. every file in the manifest exists, and its sha256 and row count match;
//! 2. every county in `geo/counties.json` is in the canonical list (`core/counties.csv`);
//! 3. every county-keyed core file covers every canonical county, or lists it under the job's
//!    `missing` reasons, and contains no county code outside the canonical list;
//! 4. no pack uses an old Connecticut county code (`090xx`) except the crosswalk itself, and all
//!    nine planning regions are present;
//! 5. `zip_county.csv` points only at canonical counties and each ZIP's shares sum to at most 1.

use crate::csvout::{col, read_table};
use crate::ct::is_old_ct;
use crate::http::sha256_hex;
use crate::manifest::Manifest;
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// What `verify` found.
#[derive(Debug, Default)]
pub struct Report {
    /// Informational lines.
    pub lines: Vec<String>,
    /// Problems (verification fails if non-empty).
    pub problems: Vec<String>,
    /// Files checked.
    pub files: usize,
    /// Individual checks run.
    pub checks: usize,
}

/// Count data rows in a pack file the same way the ETL does.
pub fn count_rows(path: &str, bytes: &[u8]) -> Result<u64> {
    if path.ends_with(".csv") {
        let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_reader(bytes);
        let mut n = 0;
        for r in rdr.records() {
            r?;
            n += 1;
        }
        Ok(n)
    } else if path.ends_with(".json") || path.ends_with(".geojson") {
        let v: serde_json::Value = serde_json::from_slice(bytes)?;
        Ok(v.get("features").and_then(|f| f.as_array()).map(|a| a.len() as u64).unwrap_or(1))
    } else if path.ends_with(".toml") {
        let text = std::str::from_utf8(bytes).map_err(|e| data_err(format!("{path} is not UTF-8: {e}")))?;
        let v: toml::Table = text.parse().map_err(|e| data_err(format!("{path} is not valid TOML: {e}")))?;
        Ok(v.values().map(|x| x.as_array().map(|a| a.len() as u64).unwrap_or(0)).sum())
    } else {
        Ok(0)
    }
}

/// Run all checks.
pub fn verify(data_dir: &Path) -> Result<Report> {
    let manifest = Manifest::load(data_dir)?;
    let mut rep = Report::default();

    // 1. Checksums and row counts.
    for f in manifest.all_files() {
        rep.files += 1;
        rep.checks += 1;
        let path = data_dir.join(&f.path);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                rep.problems.push(format!("{} is listed in the manifest but cannot be read: {e}", f.path));
                continue;
            }
        };
        let sha = sha256_hex(&bytes);
        if sha != f.sha256 {
            rep.problems.push(format!("{}: sha256 {} does not match manifest {}", f.path, sha, f.sha256));
        }
        rep.checks += 1;
        match count_rows(&f.path, &bytes) {
            Ok(n) if n == f.rows => {}
            Ok(n) => rep.problems.push(format!("{}: {} rows, manifest says {}", f.path, n, f.rows)),
            Err(e) => rep.problems.push(format!("{}: cannot count rows: {e}", f.path)),
        }
    }
    rep.lines.push(format!("checked {} files against manifest (pack version {})", rep.files, manifest.pack_version));

    // Canonical counties.
    let counties_path = crate::jobs::geography::COUNTIES;
    let canon: BTreeSet<String> = match read_table(data_dir, counties_path) {
        Ok((h, rows)) => {
            let i = col(&h, "fips")?;
            rows.iter().map(|r| r[i].clone()).collect()
        }
        Err(e) => {
            rep.problems.push(format!("canonical county list missing: {e}"));
            return Ok(rep);
        }
    };
    rep.lines.push(format!("canonical county list: {} counties", canon.len()));

    // 2. Map counties.
    let geo_path = data_dir.join(crate::jobs::geography::GEO_COUNTIES);
    if let Ok(bytes) = std::fs::read(&geo_path) {
        let v: serde_json::Value = serde_json::from_slice(&bytes)?;
        let ids: Vec<String> = v["features"]
            .as_array()
            .map(|a| a.iter().filter_map(|f| f["id"].as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        rep.checks += 1;
        for id in &ids {
            if !canon.contains(id) {
                rep.problems.push(format!("{}: feature {id} is not in the canonical county list", crate::jobs::geography::GEO_COUNTIES));
            }
        }
        rep.lines.push(format!("map features: {} (all in canonical list: {})", ids.len(), ids.iter().all(|i| canon.contains(i))));
    }

    // Missing lists per job.
    let mut missing_by_job: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, job) in &manifest.jobs {
        let set = missing_by_job.entry(id.clone()).or_default();
        for m in &job.missing {
            set.extend(m.fips.iter().cloned());
        }
    }

    // 3 & 4. County-keyed files.
    let regions: BTreeSet<&str> = ["09110", "09120", "09130", "09140", "09150", "09160", "09170", "09180", "09190"].into_iter().collect();
    for f in manifest.all_files() {
        if !f.path.ends_with(".csv") || f.path == crate::ct::PATH {
            continue;
        }
        let Ok((h, rows)) = read_table(data_dir, &f.path) else { continue };
        let fips_col = ["fips", "county_fips"].iter().find_map(|c| h.iter().position(|x| x == c));
        let Some(fc) = fips_col else { continue };
        rep.checks += 1;
        let present: BTreeSet<String> = rows.iter().map(|r| r[fc].clone()).collect();
        let old_ct: Vec<&String> = present.iter().filter(|p| is_old_ct(p)).collect();
        if !old_ct.is_empty() {
            rep.problems.push(format!("{}: uses old Connecticut county codes {old_ct:?}; apply the planning-region crosswalk", f.path));
        }
        let stray: Vec<&String> = present.iter().filter(|p| !canon.contains(*p)).take(10).collect();
        if !stray.is_empty() {
            rep.problems.push(format!("{}: county codes not in the canonical list, e.g. {stray:?}", f.path));
        }
        // Coverage applies to per-county tables (not the ZIP crosswalk, whose rows are ZIPs).
        if f.key.first().map(|k| k.as_str()) == Some("fips") {
            let missing_ok = missing_by_job.get(&f.job).cloned().unwrap_or_default();
            let uncovered: Vec<&String> = canon.iter().filter(|c| !present.contains(*c) && !missing_ok.contains(*c)).collect();
            rep.checks += 1;
            if !uncovered.is_empty() {
                rep.problems.push(format!(
                    "{}: {} canonical counties have no row and no stated reason, e.g. {:?}",
                    f.path,
                    uncovered.len(),
                    uncovered.iter().take(8).collect::<Vec<_>>()
                ));
            }
            let listed_but_present: Vec<&String> = missing_ok.iter().filter(|m| present.contains(*m)).take(5).collect();
            if !listed_but_present.is_empty() {
                rep.lines.push(format!("{}: note: counties listed as missing but present: {listed_but_present:?}", f.path));
            }
            let has_regions = regions.iter().filter(|r| present.contains(**r) || missing_ok.contains(**r)).count();
            rep.checks += 1;
            if has_regions != regions.len() {
                rep.problems.push(format!("{}: only {has_regions} of 9 Connecticut planning regions present or explained", f.path));
            }
            rep.lines.push(format!(
                "{}: {} counties present, {} explained as missing",
                f.path,
                present.len(),
                missing_ok.iter().filter(|m| canon.contains(*m)).count()
            ));
        }
    }

    // 5. ZIP shares.
    if let Ok((h, rows)) = read_table(data_dir, crate::jobs::geography::ZIP_COUNTY) {
        let (iz, is) = (col(&h, "zip")?, col(&h, "land_share")?);
        let mut sums: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for r in &rows {
            let s = r[is].parse::<f64>().unwrap_or(f64::NAN);
            let e = sums.entry(r[iz].clone()).or_insert((0.0, 0.0));
            e.0 += s;
            e.1 = e.1.max(s);
        }
        rep.checks += 1;
        let bad: Vec<(&String, &(f64, f64))> =
            sums.iter().filter(|(_, (s, _))| !(*s > 0.0 && *s <= 1.0005)).take(5).collect();
        if !bad.is_empty() {
            rep.problems.push(format!("zip_county.csv: ZIP share sums out of range, e.g. {bad:?}"));
        }
        let ambiguous = sums.values().filter(|(_, max)| *max < 0.8).count();
        rep.lines.push(format!(
            "zip_county.csv: {} ZIPs, {} rows, {} ambiguous (no county holds 80% of the land)",
            sums.len(),
            rows.len(),
            ambiguous
        ));
    }
    Ok(rep)
}
