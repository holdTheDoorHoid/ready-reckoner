//! Writing pack files deterministically and summarising what changed.

use crate::http::sha256_hex;
use crate::{Result, data_err};
use std::collections::BTreeMap;
use std::path::Path;

/// An in-memory CSV table. Cells are already formatted strings (see [`crate::num`]).
#[derive(Debug, Clone, Default)]
pub struct Table {
    /// Column names (snake_case).
    pub header: Vec<String>,
    /// Rows; each has `header.len()` cells.
    pub rows: Vec<Vec<String>>,
    /// Number of leading columns that together identify a row.
    pub key_cols: usize,
}

impl Table {
    /// New empty table with the given header; the first `key_cols` columns form the key.
    pub fn new(header: &[&str], key_cols: usize) -> Self {
        Self {
            header: header.iter().map(|s| s.to_string()).collect(),
            rows: Vec::new(),
            key_cols,
        }
    }

    /// New table from owned column names.
    pub fn with_header(header: Vec<String>, key_cols: usize) -> Self {
        Self {
            header,
            rows: Vec::new(),
            key_cols,
        }
    }

    /// Append a row. Panics in debug builds if the width is wrong.
    pub fn push(&mut self, row: Vec<String>) {
        debug_assert_eq!(row.len(), self.header.len(), "row width mismatch");
        self.rows.push(row);
    }

    /// Sort rows by the key columns (then by the full row) and fail on duplicate keys.
    pub fn sort_and_check(&mut self) -> Result<()> {
        let k = self.key_cols.max(1);
        self.rows
            .sort_by(|a, b| a[..k].cmp(&b[..k]).then_with(|| a.cmp(b)));
        for w in self.rows.windows(2) {
            if w[0][..k] == w[1][..k] {
                return Err(data_err(format!(
                    "duplicate key {:?} in table with columns {:?}",
                    &w[0][..k],
                    self.header
                )));
            }
        }
        Ok(())
    }

    /// Key column names.
    pub fn key_names(&self) -> Vec<String> {
        self.header[..self.key_cols.min(self.header.len())].to_vec()
    }

    /// Serialise as CSV bytes: comma separated, `\n` line ends, quotes only where needed.
    pub fn to_csv(&self) -> Result<Vec<u8>> {
        let mut w = csv::WriterBuilder::new()
            .terminator(csv::Terminator::Any(b'\n'))
            .quote_style(csv::QuoteStyle::Necessary)
            .from_writer(Vec::new());
        w.write_record(&self.header)?;
        for r in &self.rows {
            w.write_record(r)?;
        }
        w.into_inner()
            .map_err(|e| data_err(format!("CSV writer: {e}")))
    }
}

/// How a pack file changed compared with the previous version on disk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffSummary {
    /// Rows in the previous file (0 if there was none).
    pub previous_rows: u64,
    /// Rows now.
    pub rows: u64,
    /// Keys that are new.
    pub added: u64,
    /// Keys that disappeared.
    pub removed: u64,
    /// Keys whose row content changed.
    pub changed: u64,
    /// True when the bytes are identical.
    pub identical: bool,
    /// True when there was no previous file.
    pub new_file: bool,
    /// True when the job no longer writes this file (it was deleted).
    pub removed_file: bool,
}

/// What was written.
#[derive(Debug, Clone)]
pub struct Written {
    /// Path relative to the data directory.
    pub path: String,
    /// Data rows.
    pub rows: u64,
    /// Size in bytes.
    pub bytes: u64,
    /// sha256 of the bytes.
    pub sha256: String,
    /// Key column names (CSV).
    pub key: Vec<String>,
    /// Change summary against the previous file.
    pub diff: DiffSummary,
}

fn keyed_rows(bytes: &[u8], key_cols: usize) -> Option<BTreeMap<Vec<String>, Vec<String>>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(bytes);
    let mut out = BTreeMap::new();
    for rec in rdr.records() {
        let rec = rec.ok()?;
        let row: Vec<String> = rec.iter().map(|s| s.to_string()).collect();
        let k = key_cols.max(1).min(row.len());
        out.insert(row[..k].to_vec(), row);
    }
    Some(out)
}

/// Write a table to `data_dir/rel`, returning its checksum and a change summary.
pub fn write_table(data_dir: &Path, rel: &str, table: &Table) -> Result<Written> {
    let bytes = table.to_csv()?;
    let path = data_dir.join(rel);
    let diff = match std::fs::read(&path) {
        Ok(old) if old == bytes => DiffSummary {
            previous_rows: table.rows.len() as u64,
            rows: table.rows.len() as u64,
            identical: true,
            ..Default::default()
        },
        Ok(old) => {
            let new_map = keyed_rows(&bytes, table.key_cols).unwrap_or_default();
            match keyed_rows(&old, table.key_cols) {
                Some(old_map) => {
                    let added = new_map.keys().filter(|k| !old_map.contains_key(*k)).count() as u64;
                    let removed =
                        old_map.keys().filter(|k| !new_map.contains_key(*k)).count() as u64;
                    let changed = new_map
                        .iter()
                        .filter(|(k, v)| old_map.get(*k).is_some_and(|o| o != *v))
                        .count() as u64;
                    DiffSummary {
                        previous_rows: old_map.len() as u64,
                        rows: new_map.len() as u64,
                        added,
                        removed,
                        changed,
                        ..Default::default()
                    }
                }
                None => DiffSummary {
                    rows: table.rows.len() as u64,
                    ..Default::default()
                },
            }
        }
        Err(_) => DiffSummary {
            rows: table.rows.len() as u64,
            new_file: true,
            ..Default::default()
        },
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &bytes)?;
    Ok(Written {
        path: rel.to_string(),
        rows: table.rows.len() as u64,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        key: table.key_names(),
        diff,
    })
}

/// Write a non-CSV pack file (JSON, TOML). `rows` is the number of logical entries.
pub fn write_text(data_dir: &Path, rel: &str, text: &str, rows: u64) -> Result<Written> {
    let bytes = text.as_bytes();
    let path = data_dir.join(rel);
    let diff = match std::fs::read(&path) {
        Ok(old) if old == bytes => DiffSummary {
            previous_rows: rows,
            rows,
            identical: true,
            ..Default::default()
        },
        Ok(_) => DiffSummary {
            rows,
            changed: rows,
            ..Default::default()
        },
        Err(_) => DiffSummary {
            rows,
            new_file: true,
            ..Default::default()
        },
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, bytes)?;
    Ok(Written {
        path: rel.to_string(),
        rows,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(bytes),
        key: Vec::new(),
        diff,
    })
}

/// Read a CSV pack file from the data directory into header + rows (for jobs that build on
/// earlier jobs' outputs, and for `verify`).
pub fn read_table(data_dir: &Path, rel: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let path = data_dir.join(rel);
    let bytes = std::fs::read(&path).map_err(|e| {
        data_err(format!(
            "{} is needed but could not be read ({e}); run the job that builds it first",
            path.display()
        ))
    })?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(bytes.as_slice());
    let header: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let mut rows = Vec::new();
    for rec in rdr.records() {
        rows.push(rec?.iter().map(|s| s.to_string()).collect());
    }
    Ok((header, rows))
}

/// Index of a column by name.
pub fn col(header: &[String], name: &str) -> Result<usize> {
    header
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| data_err(format!("column {name} not found in {header:?}")))
}

/// Parse delimited text (CSV, pipe or tab separated) into trimmed header + rows. A leading
/// byte-order mark is removed; quoted fields (including embedded newlines) are handled.
pub fn parse_delimited(text: &str, delimiter: u8) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let text = text.trim_start_matches('\u{feff}');
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true)
        .from_reader(text.as_bytes());
    let header: Vec<String> = rdr
        .headers()?
        .iter()
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        let mut row: Vec<String> = rec.iter().map(|s| s.trim().to_string()).collect();
        row.resize(header.len(), String::new());
        rows.push(row);
    }
    Ok((header, rows))
}

/// Index of the first column whose name starts with `prefix` (for headers like
/// `"STATEFP (INCITS38)"`).
pub fn col_prefix(header: &[String], prefix: &str) -> Result<usize> {
    header
        .iter()
        .position(|h| h.starts_with(prefix))
        .ok_or_else(|| data_err(format!("no column starting with {prefix} in {header:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_counts_rows() {
        let dir = std::env::temp_dir().join(format!("rr-etl-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut t = Table::new(&["fips", "v"], 1);
        t.push(vec!["01001".into(), "1".into()]);
        t.push(vec!["01003".into(), "2".into()]);
        t.sort_and_check().unwrap();
        let w1 = write_table(&dir, "t.csv", &t).unwrap();
        assert!(w1.diff.new_file);
        let w2 = write_table(&dir, "t.csv", &t).unwrap();
        assert!(w2.diff.identical);
        assert_eq!(w1.sha256, w2.sha256);
        let mut t2 = Table::new(&["fips", "v"], 1);
        t2.push(vec!["01001".into(), "1".into()]);
        t2.push(vec!["01003".into(), "3".into()]);
        t2.push(vec!["01005".into(), "4".into()]);
        t2.sort_and_check().unwrap();
        let w3 = write_table(&dir, "t.csv", &t2).unwrap();
        assert_eq!((w3.diff.added, w3.diff.removed, w3.diff.changed), (1, 0, 1));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn duplicate_keys_fail() {
        let mut t = Table::new(&["fips", "v"], 1);
        t.push(vec!["01001".into(), "1".into()]);
        t.push(vec!["01001".into(), "2".into()]);
        assert!(t.sort_and_check().is_err());
    }
}
