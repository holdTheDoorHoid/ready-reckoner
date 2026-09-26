//! `rr-etl verify`: re-check the packs against the manifest and check that counties join.
//!
//! The checks live in [`rr_data::verify`] (moved there so the `rr` command line can run them
//! without this crate's network stack); this module keeps the ETL's entry points and error type.
//! Checks:
//! 1. every file in the manifest exists, and its sha256 and row count match;
//! 2. every county in `geo/counties.json` is in the canonical list (`core/counties.csv`);
//! 3. every county-keyed core file covers every canonical county, or lists it under the job's
//!    `missing` reasons, and contains no county code outside the canonical list;
//! 4. no pack uses an old Connecticut county code (`090xx`) except the crosswalk itself, and all
//!    nine planning regions are present;
//! 5. `zip_county.csv` points only at canonical counties and each ZIP's shares sum to at most 1.

use crate::{Result, data_err};
use std::path::Path;

pub use rr_data::verify::Report;

/// Count data rows in a pack file the same way the ETL does.
pub fn count_rows(path: &str, bytes: &[u8]) -> Result<u64> {
    rr_data::verify::count_rows(path, bytes).map_err(data_err)
}

/// Run all checks.
pub fn verify(data_dir: &Path) -> Result<Report> {
    rr_data::verify::verify(data_dir).map_err(data_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::sha256_hex;
    use crate::manifest::{FileEntry, JobRecord, Manifest, Missing, Pack};

    fn write(dir: &Path, rel: &str, text: &str) -> FileEntry {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        FileEntry {
            path: rel.into(),
            job: if rel.contains("counties") {
                "geography".into()
            } else {
                "flood".into()
            },
            sha256: sha256_hex(text.as_bytes()),
            bytes: text.len() as u64,
            rows: count_rows(rel, text.as_bytes()).unwrap(),
            key: vec!["fips".into()],
        }
    }

    fn fixture(dir: &Path, flood_missing: &[&str]) {
        let counties = "fips,name,state_abbr,lat,lon\n01001,Autauga,AL,32.5,-86.6\n09110,Capitol,CT,41.8,-72.6\n09120,Greater Bridgeport,CT,41.2,-73.2\n09130,Lower Connecticut River Valley,CT,41.4,-72.5\n09140,Naugatuck Valley,CT,41.5,-73.1\n09150,Northeastern Connecticut,CT,41.8,-72\n09160,Northwest Hills,CT,41.9,-73.2\n09170,South Central Connecticut,CT,41.3,-72.8\n09180,Southeastern Connecticut,CT,41.5,-72.1\n09190,Western Connecticut,CT,41.3,-73.4\n";
        let mut flood = String::from("fips,sfha_home_share\n");
        for f in [
            "01001", "09110", "09120", "09130", "09140", "09150", "09160", "09170", "09180",
            "09190",
        ] {
            if !flood_missing.contains(&f) {
                flood.push_str(&format!("{f},0.01\n"));
            }
        }
        let e1 = write(dir, "core/counties.csv", counties);
        let e2 = write(dir, "core/flood.csv", &flood);
        let mut m = Manifest::default();
        m.packs.insert(
            "core".into(),
            Pack {
                description: String::new(),
                files: vec![e1, e2],
            },
        );
        m.jobs.insert("flood".into(), JobRecord::default());
        m.save(dir).unwrap();
    }

    #[test]
    fn verify_passes_a_consistent_pack_and_catches_problems() {
        let dir = std::env::temp_dir().join(format!("rr-etl-verify-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        fixture(&dir, &[]);
        let r = verify(&dir).unwrap();
        assert!(r.problems.is_empty(), "{:?}", r.problems);

        // A county without a row and without a stated reason fails...
        fixture(&dir, &["01001"]);
        assert!(!verify(&dir).unwrap().problems.is_empty());
        // ...and passes once the job lists it as missing with a reason.
        let mut m = Manifest::load(&dir).unwrap();
        m.jobs.get_mut("flood").unwrap().missing = vec![Missing {
            reason: "no data".into(),
            fips: vec!["01001".into()],
        }];
        m.save(&dir).unwrap();
        assert!(verify(&dir).unwrap().problems.is_empty());

        // A changed byte breaks the checksum.
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
        // ...and the old Connecticut code is reported.
        assert!(
            problems.iter().any(|p| p.contains("old Connecticut")),
            "{problems:?}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
