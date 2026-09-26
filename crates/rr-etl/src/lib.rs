//! Ready Reckoner — `rr-etl`: the build-time ETL (native only, never built for wasm32).
//!
//! `rr-etl refresh --out data` downloads the public sources listed in `docs/DATA_SOURCES.md`,
//! turns them into the small, deterministic pack files under `data/`, and records provenance in
//! `data/manifest.json` (URL actually fetched, version label, retrieval time, sha256 of the raw
//! input, licence and obligations, rows in and out). `rr-etl verify --data data` re-checks the
//! checksums, row counts and that every county joins across every pack.
//!
//! Rules this crate follows (see `docs/DESIGN.md` §6):
//! - Raw downloads are streamed or held in memory; nothing raw is written to disk unless
//!   `--keep-raw` is given, and then only under `data/raw/` (git-ignored).
//! - Output is deterministic: stable row order, numbers rounded to 4 significant figures, so an
//!   unchanged input produces a byte-identical pack. Only the manifest's timestamps change.
//! - Every county code in every pack uses the Census 2024 county list, including Connecticut's
//!   nine planning regions (`091xx`); older sources are converted with [`ct`].
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod changes;
pub mod cli;
pub mod csvout;
pub mod ct;
pub mod geo;
pub mod http;
pub mod jobs;
pub mod manifest;
pub mod num;
pub mod raster;
pub mod shp;
pub mod timefmt;
pub mod verify;
pub mod xlsx;

/// Everything that can go wrong in the ETL. Messages are written for the person running the
/// refresh, so they name the source and what to check.
#[derive(Debug, thiserror::Error)]
pub enum EtlError {
    /// A download failed after retries.
    #[error("download failed: {0}")]
    Http(String),
    /// Local file problem.
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    /// A CSV input or output could not be parsed or written.
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    /// A JSON input could not be parsed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// A ZIP archive could not be read.
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    /// The source's content was not what the job expected (format change, missing field, ...).
    #[error("unexpected source data: {0}")]
    Data(String),
    /// `verify` found a problem.
    #[error("verification failed: {0}")]
    Verify(String),
}

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, EtlError>;

/// Build an [`EtlError::Data`] from anything printable.
pub fn data_err(msg: impl std::fmt::Display) -> EtlError {
    EtlError::Data(msg.to_string())
}
