//! `data/manifest.json`: provenance for every pack file.
//!
//! The manifest is rewritten on every refresh. Everything in it is deterministic except the
//! timestamps (`generated`, `finished`, `retrieved`), so a refresh with unchanged inputs changes
//! only those fields. `rr-data` reads the parts it needs (versions, attributions, disclaimer,
//! checksums) for the About screen and for its own integrity checks.

use crate::{Result, data_err};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Manifest schema version.
pub const SCHEMA: u32 = 1;

/// The whole manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// Schema version of this file.
    pub schema: u32,
    /// Content hash (first 12 hex digits of a sha256 over every file's path and sha256). Changes
    /// only when a pack's bytes change.
    pub pack_version: String,
    /// UTC timestamp of the most recent refresh that wrote this manifest.
    pub generated: String,
    /// Packs by name (`core`, `geo`).
    pub packs: BTreeMap<String, Pack>,
    /// One record per ETL job, by job id.
    pub jobs: BTreeMap<String, JobRecord>,
    /// Credit lines the app must show (About screen and packet sources).
    pub attributions: Vec<Attribution>,
    /// Owner decisions a job must see before it may ship a source, by key (for example
    /// `eviction_lab_odc_by`: an attribution licence needs the owner's approval under CLAUDE.md
    /// rule 5). Refreshes keep this section as it is; only a person sets `approved`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sign_offs: BTreeMap<String, SignOff>,
    /// Attributions that belong to an optional pack, by attribution `source` -> pack name. The
    /// engine shows those credit lines only once the pack is loaded (a packet should not credit
    /// data it never read).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attribution_packs: BTreeMap<String, String>,
}

/// One owner decision (see [`Manifest::sign_offs`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SignOff {
    /// True once the owner has approved; jobs gated on this key write their output only then.
    pub approved: bool,
    /// What is being approved, in words.
    #[serde(default)]
    pub what: String,
    /// Who approved it and when (for example "owner, 2026-10-01"); empty until approved.
    #[serde(default)]
    pub by: String,
}

/// One pack: a set of files loaded together.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Pack {
    /// What the pack is for and when the app loads it.
    pub description: String,
    /// Files in the pack, sorted by path.
    pub files: Vec<FileEntry>,
}

/// One pack file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    /// Path relative to the data directory, with forward slashes (`core/nri_hazards.csv`).
    pub path: String,
    /// Job that writes it.
    pub job: String,
    /// sha256 of the file bytes.
    pub sha256: String,
    /// File size in bytes.
    pub bytes: u64,
    /// Data rows (CSV rows after the header, GeoJSON features, TOML entries).
    pub rows: u64,
    /// Key columns (CSV only) that identify a row; used by `verify` and by change summaries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key: Vec<String>,
}

/// What one job fetched, produced and noted.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct JobRecord {
    /// Plain-language title.
    pub title: String,
    /// UTC timestamp when the job finished.
    pub finished: String,
    /// Every raw input the job used.
    pub sources: Vec<SourceRecord>,
    /// Rows read from the sources (after filtering to the relevant records).
    pub rows_in: u64,
    /// Rows written across the job's outputs.
    pub rows_out: u64,
    /// Output files (paths relative to the data directory).
    pub outputs: Vec<String>,
    /// Methodology notes, in plain language.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// Definitions that downstream code must quote exactly (e.g. the outage event definition).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub definitions: BTreeMap<String, String>,
    /// Counties in the canonical list that this job has no data for, grouped by reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<Missing>,
}

/// One raw input.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SourceRecord {
    /// Dataset name.
    pub name: String,
    /// URL actually fetched (after redirects). For paged APIs, the query URL without paging.
    pub url: String,
    /// Version or label reported by the source itself.
    pub version: String,
    /// UTC timestamp of retrieval.
    pub retrieved: String,
    /// sha256 of the raw bytes (for paged APIs: of all pages concatenated in request order).
    pub sha256: String,
    /// Raw size in bytes.
    pub bytes: u64,
    /// Licence or terms.
    pub license: String,
    /// What the licence obliges us to do (credit line, disclaimer, ...). Empty for public domain.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub obligations: String,
}

/// Counties with no data from a job, and why.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Missing {
    /// Plain-language reason.
    pub reason: String,
    /// County FIPS codes, sorted.
    pub fips: Vec<String>,
}

/// A file entry that [`Manifest::rehash`] changed: its row count and sha256 before and after.
#[derive(Debug, Clone, PartialEq)]
pub struct Rehashed {
    /// Path relative to the data directory.
    pub path: String,
    /// Row count the manifest recorded.
    pub old_rows: u64,
    /// Row count of the file on disk.
    pub rows: u64,
    /// sha256 the manifest recorded.
    pub old_sha256: String,
    /// sha256 of the file on disk.
    pub sha256: String,
}

/// A credit line the app must display.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Attribution {
    /// Short source name.
    pub source: String,
    /// Exact text to display.
    pub text: String,
    /// Licence name.
    pub license: String,
    /// Where the source lives.
    pub url: String,
    /// Dataset version, where the source has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Date the data was accessed (`YYYY-MM-DD`, UTC).
    #[serde(default)]
    pub accessed: String,
}

impl Manifest {
    /// Read `data/manifest.json`, or return an empty manifest if it does not exist yet.
    pub fn load_or_default(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join("manifest.json");
        if !path.exists() {
            return Ok(Manifest {
                schema: SCHEMA,
                ..Default::default()
            });
        }
        let text = std::fs::read_to_string(&path)?;
        let m: Manifest = serde_json::from_str(&text)
            .map_err(|e| data_err(format!("{} is not a valid manifest: {e}", path.display())))?;
        Ok(m)
    }

    /// Read `data/manifest.json`; error if absent.
    pub fn load(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join("manifest.json");
        if !path.exists() {
            return Err(data_err(format!(
                "{} does not exist; run `rr-etl refresh` first",
                path.display()
            )));
        }
        Self::load_or_default(data_dir)
    }

    /// Recompute `pack_version` from the file entries.
    pub fn recompute_version(&mut self) {
        let mut acc = crate::http::Sha256Acc::new();
        for pack in self.packs.values() {
            for f in &pack.files {
                acc.update(format!("{}:{}\n", f.path, f.sha256).as_bytes());
            }
        }
        self.pack_version = acc.finish()[..12].to_string();
    }

    /// Recompute every file entry's sha256, size and row count from the files under `data_dir`,
    /// each job's `rows_out` (the rows of its outputs) and `pack_version`, without downloading
    /// anything or running a job (`rr-etl manifest --rehash`). For merges of data branches, where
    /// a file both branches rebuilt matches neither side's entry, and for hand edits of a pack
    /// file. `generated` and every job's sources, notes and timestamps stay as they are: they
    /// describe the refreshes that fetched the data. Returns the entries that changed.
    ///
    /// # Errors
    ///
    /// A listed file that cannot be read or whose rows cannot be counted; the manifest is then
    /// left unchanged.
    pub fn rehash(&mut self, data_dir: &Path) -> Result<Vec<Rehashed>> {
        let mut next = self.clone();
        let mut changed = Vec::new();
        for pack in next.packs.values_mut() {
            for f in &mut pack.files {
                let bytes = std::fs::read(data_dir.join(&f.path)).map_err(|e| {
                    data_err(format!(
                        "{} is listed in the manifest but cannot be read: {e}",
                        f.path
                    ))
                })?;
                let sha256 = crate::http::sha256_hex(&bytes);
                let rows = crate::verify::count_rows(&f.path, &bytes)?;
                let size = bytes.len() as u64;
                if sha256 != f.sha256 || rows != f.rows || size != f.bytes {
                    changed.push(Rehashed {
                        path: f.path.clone(),
                        old_rows: f.rows,
                        rows,
                        old_sha256: f.sha256.clone(),
                        sha256: sha256.clone(),
                    });
                }
                f.sha256 = sha256;
                f.rows = rows;
                f.bytes = size;
            }
        }
        let rows: BTreeMap<String, u64> =
            next.all_files().map(|f| (f.path.clone(), f.rows)).collect();
        for job in next.jobs.values_mut() {
            // A job's outputs are its pack files; an output the packs no longer list keeps the
            // recorded total.
            if let Some(total) = job.outputs.iter().map(|o| rows.get(o)).sum::<Option<u64>>() {
                job.rows_out = total;
            }
        }
        next.recompute_version();
        *self = next;
        Ok(changed)
    }

    /// Write the manifest as pretty JSON with a trailing newline.
    pub fn save(&self, data_dir: &Path) -> Result<()> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        std::fs::write(data_dir.join("manifest.json"), text)?;
        Ok(())
    }

    /// Find a file entry by path.
    pub fn file(&self, path: &str) -> Option<&FileEntry> {
        self.packs
            .values()
            .flat_map(|p| p.files.iter())
            .find(|f| f.path == path)
    }

    /// Every file entry across packs.
    pub fn all_files(&self) -> impl Iterator<Item = &FileEntry> {
        self.packs.values().flat_map(|p| p.files.iter())
    }

    /// Whether the owner has approved the decision `key`.
    pub fn signed_off(&self, key: &str) -> bool {
        self.sign_offs.get(key).is_some_and(|s| s.approved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(dir: &Path, rel: &str, text: &str, job: &str) -> FileEntry {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        FileEntry {
            path: rel.into(),
            job: job.into(),
            sha256: crate::http::sha256_hex(text.as_bytes()),
            bytes: text.len() as u64,
            rows: crate::verify::count_rows(rel, text.as_bytes()).unwrap(),
            key: vec!["fips".into()],
        }
    }

    #[test]
    fn rehash_recomputes_entries_rows_out_and_the_version() {
        let dir = std::env::temp_dir().join(format!("rr-etl-rehash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let events = entry(&dir, "core/events.csv", "fips,x\n01001,1\n", "events");
        let pooled = entry(
            &dir,
            "opt/p/rows.csv",
            "fips,y\n01001,2\n01003,3\n",
            "model",
        );
        let mut m = Manifest::default();
        m.packs.insert(
            "core".into(),
            Pack {
                description: String::new(),
                files: vec![events],
            },
        );
        m.packs.insert(
            "p".into(),
            Pack {
                description: String::new(),
                files: vec![pooled],
            },
        );
        for (id, out, rows) in [
            ("events", "core/events.csv", 1),
            ("model", "opt/p/rows.csv", 2),
        ] {
            m.jobs.insert(
                id.into(),
                JobRecord {
                    finished: "2026-09-26T19:00:00Z".into(),
                    outputs: vec![out.into()],
                    rows_out: rows,
                    ..Default::default()
                },
            );
        }
        m.generated = "2026-09-26T19:00:00Z".into();
        m.recompute_version();
        // Nothing changed on disk: nothing to do.
        let before = m.clone();
        assert!(m.rehash(&dir).unwrap().is_empty());
        assert_eq!(m, before);

        // A merge adds rows to one file: its entry, its job's rows_out and the version move.
        std::fs::write(dir.join("core/events.csv"), "fips,x\n01001,1\n01003,4\n").unwrap();
        let changed = m.rehash(&dir).unwrap();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].path, "core/events.csv");
        assert_eq!((changed[0].old_rows, changed[0].rows), (1, 2));
        let f = m.file("core/events.csv").unwrap();
        assert_eq!((f.rows, f.bytes), (2, 23));
        assert_eq!(m.jobs["events"].rows_out, 2);
        assert_eq!(m.jobs["model"].rows_out, 2);
        assert_ne!(m.pack_version, before.pack_version);
        // Timestamps describe the refreshes, not the rehash.
        assert_eq!(m.generated, before.generated);
        assert_eq!(m.jobs["events"].finished, "2026-09-26T19:00:00Z");

        // A listed file that is gone is an error, and the manifest is left as it was.
        std::fs::remove_file(dir.join("opt/p/rows.csv")).unwrap();
        let kept = m.clone();
        let e = m.rehash(&dir).unwrap_err().to_string();
        assert!(e.contains("opt/p/rows.csv"), "{e}");
        assert_eq!(m, kept);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
