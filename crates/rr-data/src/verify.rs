//! Checks a data directory on disk the way `rr-etl verify` does (moved here from `rr-etl` so the
//! CLI can run them without the ETL's network stack). Native builds only: it reads files.
//!
//! Checks:
//! 1. every file in the manifest exists, and its sha256 and row count match;
//! 2. every county in `geo/counties.json` is in the canonical list (`core/counties.csv`);
//! 3. every county-keyed core file covers every canonical county, or lists it under the job's
//!    `missing` reasons, and contains no county code outside the canonical list;
//! 4. no pack uses an old Connecticut county code (`090xx`) except the crosswalk itself, and all
//!    nine planning regions are present;
//! 5. `zip_county.csv` points only at canonical counties and each ZIP's shares sum to at most 1.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::manifest::Manifest;

/// The canonical county list.
pub const COUNTIES: &str = "core/counties.csv";
/// County outlines for the map.
pub const GEO_COUNTIES: &str = "geo/counties.json";
/// ZIP code to county shares.
pub const ZIP_COUNTY: &str = "core/zip_county.csv";
/// The Connecticut crosswalk, the one file allowed to hold old county codes.
pub const CT_CROSSWALK: &str = "core/ct_crosswalk.csv";
/// Connecticut's nine planning regions (NRI and Census 2024 county equivalents).
pub const CT_REGIONS: [&str; 9] = [
    "09110", "09120", "09130", "09140", "09150", "09160", "09170", "09180", "09190",
];

/// What [`verify`] found.
#[derive(Debug, Default, Clone, PartialEq)]
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

/// A retired Connecticut county code (`090xx`), replaced by the planning regions (`091xx`).
pub fn is_old_ct(fips: &str) -> bool {
    fips.len() == 5 && fips.starts_with("090")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Counts the data rows of a pack file the way the ETL does: CSV rows after the header, GeoJSON
/// features, TOML array entries.
///
/// # Errors
///
/// A file that does not parse, as text.
pub fn count_rows(path: &str, bytes: &[u8]) -> Result<u64, String> {
    if path.ends_with(".csv") {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(bytes);
        let mut n = 0;
        for r in rdr.records() {
            r.map_err(|e| format!("{path}: {e}"))?;
            n += 1;
        }
        Ok(n)
    } else if path.ends_with(".json") || path.ends_with(".geojson") {
        let v: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| format!("{path}: {e}"))?;
        Ok(v.get("features")
            .and_then(|f| f.as_array())
            .map_or(1, |a| a.len() as u64))
    } else if path.ends_with(".toml") {
        let text = std::str::from_utf8(bytes).map_err(|e| format!("{path} is not UTF-8: {e}"))?;
        let v: toml::Table = text
            .parse()
            .map_err(|e| format!("{path} is not valid TOML: {e}"))?;
        Ok(v.values()
            .map(|x| x.as_array().map_or(0, |a| a.len() as u64))
            .sum())
    } else {
        Ok(0)
    }
}

/// Reads a CSV file under `dir` into a header and rows.
fn read_table(dir: &Path, rel: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let path = dir.join(rel);
    let bytes = std::fs::read(&path).map_err(|e| {
        format!(
            "{} is needed but could not be read ({e}); run the job that builds it first",
            path.display()
        )
    })?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(bytes.as_slice());
    let header: Vec<String> = rdr
        .headers()
        .map_err(|e| format!("{rel}: {e}"))?
        .iter()
        .map(str::to_owned)
        .collect();
    let mut rows = Vec::new();
    for rec in rdr.records() {
        rows.push(
            rec.map_err(|e| format!("{rel}: {e}"))?
                .iter()
                .map(str::to_owned)
                .collect(),
        );
    }
    Ok((header, rows))
}

fn col(header: &[String], name: &str) -> Result<usize, String> {
    header
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| format!("column {name} not found in {header:?}"))
}

/// Runs every check on the data directory `dir` (the one holding `manifest.json`).
///
/// # Errors
///
/// A manifest that cannot be read or parsed, or a canonical file with a missing column, as text.
/// Problems with individual files are reported in [`Report::problems`] instead.
pub fn verify(dir: &Path) -> Result<Report, String> {
    let manifest_path = dir.join("manifest.json");
    let bytes = std::fs::read(&manifest_path)
        .map_err(|e| format!("{} could not be read: {e}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("{} is not a valid manifest: {e}", manifest_path.display()))?;
    let files: Vec<_> = manifest.packs.values().flat_map(|p| &p.files).collect();
    let mut rep = Report::default();

    // 1. Checksums and row counts.
    for f in &files {
        rep.files += 1;
        rep.checks += 1;
        let bytes = match std::fs::read(dir.join(&f.path)) {
            Ok(b) => b,
            Err(e) => {
                rep.problems.push(format!(
                    "{} is listed in the manifest but cannot be read: {e}",
                    f.path
                ));
                continue;
            }
        };
        let sha = sha256_hex(&bytes);
        if sha != f.sha256 {
            rep.problems.push(format!(
                "{}: sha256 {} does not match manifest {}",
                f.path, sha, f.sha256
            ));
        }
        rep.checks += 1;
        match count_rows(&f.path, &bytes) {
            Ok(n) if n == f.rows => {}
            Ok(n) => rep
                .problems
                .push(format!("{}: {} rows, manifest says {}", f.path, n, f.rows)),
            Err(e) => rep
                .problems
                .push(format!("{}: cannot count rows: {e}", f.path)),
        }
    }
    rep.lines.push(format!(
        "checked {} files against manifest (pack version {})",
        rep.files, manifest.pack_version
    ));

    // Canonical counties.
    let canon: BTreeSet<String> = match read_table(dir, COUNTIES) {
        Ok((h, rows)) => {
            let i = col(&h, "fips")?;
            rows.iter().map(|r| r[i].clone()).collect()
        }
        Err(e) => {
            rep.problems
                .push(format!("canonical county list missing: {e}"));
            return Ok(rep);
        }
    };
    rep.lines
        .push(format!("canonical county list: {} counties", canon.len()));

    // 2. Map counties.
    if let Ok(bytes) = std::fs::read(dir.join(GEO_COUNTIES)) {
        let v: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("{GEO_COUNTIES}: {e}"))?;
        let ids: Vec<String> = v["features"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|f| f["id"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        rep.checks += 1;
        for id in &ids {
            if !canon.contains(id) {
                rep.problems.push(format!(
                    "{GEO_COUNTIES}: feature {id} is not in the canonical county list"
                ));
            }
        }
        rep.lines.push(format!(
            "map features: {} (all in canonical list: {})",
            ids.len(),
            ids.iter().all(|i| canon.contains(i))
        ));
    }

    // Missing lists per job.
    let mut missing_by_job: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (id, job) in &manifest.jobs {
        let set = missing_by_job.entry(id.as_str()).or_default();
        for m in &job.missing {
            set.extend(m.fips.iter().map(String::as_str));
        }
    }

    // 3 & 4. County-keyed files.
    let regions: BTreeSet<&str> = CT_REGIONS.into_iter().collect();
    for f in &files {
        if !f.path.ends_with(".csv") || f.path == CT_CROSSWALK {
            continue;
        }
        let Ok((h, rows)) = read_table(dir, &f.path) else {
            continue;
        };
        let fips_col = ["fips", "county_fips"]
            .iter()
            .find_map(|c| h.iter().position(|x| x == c));
        let Some(fc) = fips_col else { continue };
        rep.checks += 1;
        let present: BTreeSet<&str> = rows.iter().map(|r| r[fc].as_str()).collect();
        let old_ct: Vec<&&str> = present.iter().filter(|p| is_old_ct(p)).collect();
        if !old_ct.is_empty() {
            rep.problems.push(format!(
                "{}: uses old Connecticut county codes {old_ct:?}; apply the planning-region crosswalk",
                f.path
            ));
        }
        let stray: Vec<&&str> = present
            .iter()
            .filter(|p| !canon.contains(**p))
            .take(10)
            .collect();
        if !stray.is_empty() {
            rep.problems.push(format!(
                "{}: county codes not in the canonical list, e.g. {stray:?}",
                f.path
            ));
        }
        // Coverage applies to per-county tables (not the ZIP crosswalk, whose rows are ZIPs).
        if f.key.first().map(String::as_str) == Some("fips") {
            let missing_ok = missing_by_job
                .get(f.job.as_str())
                .cloned()
                .unwrap_or_default();
            let uncovered: Vec<&String> = canon
                .iter()
                .filter(|c| !present.contains(c.as_str()) && !missing_ok.contains(c.as_str()))
                .collect();
            rep.checks += 1;
            if !uncovered.is_empty() {
                rep.problems.push(format!(
                    "{}: {} canonical counties have no row and no stated reason, e.g. {:?}",
                    f.path,
                    uncovered.len(),
                    uncovered.iter().take(8).collect::<Vec<_>>()
                ));
            }
            let listed_but_present: Vec<&&str> = missing_ok
                .iter()
                .filter(|m| present.contains(*m))
                .take(5)
                .collect();
            if !listed_but_present.is_empty() {
                rep.lines.push(format!(
                    "{}: note: counties listed as missing but present: {listed_but_present:?}",
                    f.path
                ));
            }
            let has_regions = regions
                .iter()
                .filter(|r| present.contains(*r) || missing_ok.contains(*r))
                .count();
            rep.checks += 1;
            if has_regions != regions.len() {
                rep.problems.push(format!(
                    "{}: only {has_regions} of 9 Connecticut planning regions present or explained",
                    f.path
                ));
            }
            rep.lines.push(format!(
                "{}: {} counties present, {} explained as missing",
                f.path,
                present.len(),
                missing_ok.iter().filter(|m| canon.contains(**m)).count()
            ));
        }
    }

    // 5. ZIP shares.
    if let Ok((h, rows)) = read_table(dir, ZIP_COUNTY) {
        let (iz, is) = (col(&h, "zip")?, col(&h, "land_share")?);
        let mut sums: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
        for r in &rows {
            let s = r[is].parse::<f64>().unwrap_or(f64::NAN);
            let e = sums.entry(r[iz].as_str()).or_insert((0.0, 0.0));
            e.0 += s;
            e.1 = e.1.max(s);
        }
        rep.checks += 1;
        let bad: Vec<(&&str, &(f64, f64))> = sums
            .iter()
            .filter(|(_, (s, _))| !(*s > 0.0 && *s <= 1.0005))
            .take(5)
            .collect();
        if !bad.is_empty() {
            rep.problems.push(format!(
                "zip_county.csv: ZIP share sums out of range, e.g. {bad:?}"
            ));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(dir: &Path, rel: &str, text: &str, job: &str) -> serde_json::Value {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        serde_json::json!({
            "path": rel,
            "job": job,
            "sha256": sha256_hex(text.as_bytes()),
            "bytes": text.len(),
            "rows": count_rows(rel, text.as_bytes()).unwrap(),
            "key": ["fips"],
        })
    }

    fn fixture(dir: &Path, flood_missing: &[&str], listed_missing: &[&str]) {
        let counties = "fips,name,state_abbr,lat,lon\n01001,Autauga,AL,32.5,-86.6\n09110,Capitol,CT,41.8,-72.6\n09120,Greater Bridgeport,CT,41.2,-73.2\n09130,Lower Connecticut River Valley,CT,41.4,-72.5\n09140,Naugatuck Valley,CT,41.5,-73.1\n09150,Northeastern Connecticut,CT,41.8,-72\n09160,Northwest Hills,CT,41.9,-73.2\n09170,South Central Connecticut,CT,41.3,-72.8\n09180,Southeastern Connecticut,CT,41.5,-72.1\n09190,Western Connecticut,CT,41.3,-73.4\n";
        let mut flood = String::from("fips,sfha_home_share\n");
        for f in ["01001"].iter().chain(CT_REGIONS.iter()) {
            if !flood_missing.contains(f) {
                flood.push_str(&format!("{f},0.01\n"));
            }
        }
        let files = vec![
            entry(dir, COUNTIES, counties, "geography"),
            entry(dir, "core/flood.csv", &flood, "flood"),
        ];
        let manifest = serde_json::json!({
            "schema": 1,
            "pack_version": "test",
            "packs": { "core": { "description": "", "files": files } },
            "jobs": { "flood": { "missing": [ { "reason": "no data", "fips": listed_missing } ] } },
        });
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn verify_passes_a_consistent_pack_and_catches_problems() {
        let dir = std::env::temp_dir().join(format!("rr-data-verify-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        fixture(&dir, &[], &[]);
        let r = verify(&dir).unwrap();
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        assert_eq!(r.files, 2);

        // A county without a row and without a stated reason fails...
        fixture(&dir, &["01001"], &[]);
        assert!(!verify(&dir).unwrap().problems.is_empty());
        // ...and passes once the job lists it as missing with a reason.
        fixture(&dir, &["01001"], &["01001"]);
        assert!(verify(&dir).unwrap().problems.is_empty());

        // A changed byte breaks the checksum, and an old Connecticut code is reported.
        std::fs::write(
            dir.join("core/flood.csv"),
            "fips,sfha_home_share\n09001,0.5\n",
        )
        .unwrap();
        let problems = verify(&dir).unwrap().problems;
        assert!(
            problems.iter().any(|p| p.contains("sha256")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.contains("old Connecticut")),
            "{problems:?}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_repository_pack_verifies() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !dir.join("manifest.json").exists() {
            return;
        }
        let r = verify(&dir).unwrap();
        assert!(r.problems.is_empty(), "{:?}", r.problems);
        // 17 core files and the map (zip_centroids.csv left the pack on 2026-09-26).
        assert!(r.files >= 18, "{} files", r.files);
    }
}
