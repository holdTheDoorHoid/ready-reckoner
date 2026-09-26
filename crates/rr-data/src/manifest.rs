//! The parts of `data/manifest.json` the engine uses: pack version, per-file checksums and row
//! counts, job records (for the About screen) and attributions. Unknown fields are ignored so the
//! ETL can add provenance without breaking old engines.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `data/manifest.json`, as the engine reads it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest schema version.
    #[serde(default)]
    pub schema: u32,
    /// Content hash of every pack file; the data pack version shown on the About screen.
    #[serde(default)]
    pub pack_version: String,
    /// When the packs were last refreshed (UTC).
    #[serde(default)]
    pub generated: String,
    /// Packs by name.
    #[serde(default)]
    pub packs: BTreeMap<String, Pack>,
    /// ETL job records by job id.
    #[serde(default)]
    pub jobs: BTreeMap<String, Job>,
    /// Credit lines the app must show.
    #[serde(default)]
    pub attributions: Vec<ManifestAttribution>,
    /// Attributions that belong to an optional pack (attribution source -> pack name): shown only
    /// once that pack is loaded.
    #[serde(default)]
    pub attribution_packs: BTreeMap<String, String>,
}

/// A pack.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    /// What it is for.
    #[serde(default)]
    pub description: String,
    /// Its files.
    #[serde(default)]
    pub files: Vec<FileEntry>,
}

/// A pack file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Path relative to the data directory, e.g. `core/nri_hazards.csv`.
    pub path: String,
    /// Job that writes it.
    #[serde(default)]
    pub job: String,
    /// sha256 of the bytes (lower-case hex).
    pub sha256: String,
    /// Size in bytes.
    #[serde(default)]
    pub bytes: u64,
    /// Data rows.
    #[serde(default)]
    pub rows: u64,
    /// Key columns (CSV only) that identify a row; `verify` checks county coverage of files
    /// keyed by `fips`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key: Vec<String>,
}

/// A job record (abridged).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Job {
    /// Title.
    #[serde(default)]
    pub title: String,
    /// When it finished (UTC).
    #[serde(default)]
    pub finished: String,
    /// Sources used.
    #[serde(default)]
    pub sources: Vec<Source>,
    /// Methodology notes.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Exact definitions (for example the outage event definition, the NRI disclaimer).
    #[serde(default)]
    pub definitions: BTreeMap<String, String>,
    /// Counties without data, by reason.
    #[serde(default)]
    pub missing: Vec<Missing>,
}

/// A source (abridged).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// Dataset name.
    #[serde(default)]
    pub name: String,
    /// URL fetched.
    #[serde(default)]
    pub url: String,
    /// Version label.
    #[serde(default)]
    pub version: String,
    /// When retrieved (UTC).
    #[serde(default)]
    pub retrieved: String,
    /// Licence.
    #[serde(default)]
    pub license: String,
}

/// Counties a job has no data for.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Missing {
    /// Why.
    #[serde(default)]
    pub reason: String,
    /// Which counties.
    #[serde(default)]
    pub fips: Vec<String>,
}

/// An attribution as written by the ETL.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ManifestAttribution {
    /// Source name.
    #[serde(default)]
    pub source: String,
    /// Text to display.
    #[serde(default)]
    pub text: String,
    /// Licence.
    #[serde(default)]
    pub license: String,
    /// URL.
    #[serde(default)]
    pub url: String,
    /// Dataset version.
    #[serde(default)]
    pub version: Option<String>,
    /// Access date (`YYYY-MM-DD`).
    #[serde(default)]
    pub accessed: String,
}

impl Manifest {
    /// The entry for a pack file.
    pub fn file(&self, path: &str) -> Option<&FileEntry> {
        self.packs
            .values()
            .flat_map(|p| p.files.iter())
            .find(|f| f.path == path)
    }

    /// A definition recorded by a job.
    pub fn definition(&self, job: &str, key: &str) -> Option<&str> {
        self.jobs.get(job)?.definitions.get(key).map(|s| s.as_str())
    }

    /// Counties a job has no data for, with the reason.
    pub fn missing(&self, job: &str) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        if let Some(j) = self.jobs.get(job) {
            for m in &j.missing {
                for f in &m.fips {
                    out.insert(f.clone(), m.reason.clone());
                }
            }
        }
        out
    }
}
