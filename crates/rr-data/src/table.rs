//! A small typed view over a CSV pack file: header lookup and parsed cells.

use rr_types::{EngineError, ErrorCode};

/// A parsed CSV file: header plus rows of string cells.
pub(crate) struct Csv {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
    name: String,
}

pub(crate) fn corrupt(name: &str, msg: impl std::fmt::Display) -> EngineError {
    EngineError::new(
        ErrorCode::PackCorrupt,
        format!("The data file {name} could not be read: {msg}"),
    )
}

impl Csv {
    /// Parse CSV bytes.
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Self, EngineError> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(bytes);
        let header: Vec<String> = rdr
            .headers()
            .map_err(|e| corrupt(name, e))?
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut rows = Vec::new();
        for rec in rdr.records() {
            let rec = rec.map_err(|e| corrupt(name, e))?;
            rows.push(rec.iter().map(|s| s.to_string()).collect());
        }
        Ok(Self {
            header,
            rows,
            name: name.to_string(),
        })
    }

    /// Index of a required column.
    pub fn col(&self, name: &str) -> Result<usize, EngineError> {
        self.header
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| corrupt(&self.name, format!("missing column {name}")))
    }

    /// Index of an optional column.
    pub fn opt_col(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }
}

/// Parse a cell as `f64`; empty or invalid is `None`.
pub(crate) fn f(cell: &str) -> Option<f64> {
    let t = cell.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok().filter(|v| v.is_finite())
    }
}

/// Parse a cell as `f32`.
pub(crate) fn f32c(cell: &str) -> Option<f32> {
    f(cell).map(|v| v as f32)
}

/// Parse a boolean cell (`true`/`false`).
pub(crate) fn b(cell: &str) -> bool {
    cell.trim().eq_ignore_ascii_case("true")
}
