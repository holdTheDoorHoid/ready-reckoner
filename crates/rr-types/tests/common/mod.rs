//! Shared helpers for the integration tests: maximal samples of every contract type and a parser
//! for the TypeScript mirror.
#![allow(dead_code)]

pub mod samples;
pub mod ts;

use std::path::PathBuf;

/// Path of a file relative to the repository root.
pub fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// Reads a file relative to the repository root.
pub fn read_repo_file(relative: &str) -> String {
    let path = repo_path(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}
