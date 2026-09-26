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
}
