//! Intermediate files that one job writes and a later job in the same refresh reads.
//!
//! They are not pack files. They live under `data/raw/intermediate/` (git-ignored) and are
//! deleted at the end of a refresh unless `--keep-raw` is given, so a partial refresh such as
//! `--only outage_model` works only after a `--keep-raw` run of the jobs that write them
//! (`outages`, `events`, `oe417`). Every intermediate is gzip-compressed CSV with a header row;
//! the writers sort their rows so that a rerun on the same input writes the same bytes.

use crate::{Result, data_err};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Directory (relative to the data directory) holding the intermediates.
pub const DIR: &str = "raw/intermediate";

/// Per-event outage records written by `outages` (see `jobs::outages::EventRecord`).
pub const OUTAGE_EVENTS: &str = "outage_events.csv.gz";
/// Per-unit outage exposure (customers, months with data by year) written by `outages`.
pub const OUTAGE_UNITS: &str = "outage_units.csv.gz";
/// Customer-hours inside outage events per unit and local calendar day, written by `outages`.
pub const OUTAGE_DAILY: &str = "outage_daily.csv.gz";
/// Storm Events county-episodes since 2014 in UTC, written by `events`.
pub const STORM_EPISODES: &str = "storm_episodes.csv.gz";
/// HURDAT2 track fixes since 2014 with their UTC times, written by `events`.
pub const HURDAT_TRACKS: &str = "hurdat_tracks.csv.gz";
/// DOE OE-417 disturbance events (from the PNNL linkage), written by `oe417`.
pub const OE417_EVENTS: &str = "oe417_events.csv.gz";

/// Full path of an intermediate file.
pub fn path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir.join(DIR).join(name)
}

/// Write rows (already formatted cells) as a gzip CSV. Rows are written in the order given.
pub fn write(data_dir: &Path, name: &str, header: &[&str], rows: &[Vec<String>]) -> Result<u64> {
    let p = path(data_dir, name);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(&p)?;
    // A fixed compression level and no file name or time in the gzip header keep the bytes stable.
    let gz = flate2::GzBuilder::new().write(
        std::io::BufWriter::with_capacity(1 << 20, file),
        flate2::Compression::new(6),
    );
    let mut w = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(gz);
    w.write_record(header)?;
    for r in rows {
        w.write_record(r)?;
    }
    let gz = w
        .into_inner()
        .map_err(|e| data_err(format!("writing {name}: {e}")))?;
    let mut inner = gz.finish()?;
    inner.flush()?;
    Ok(std::fs::metadata(&p)?.len())
}

/// Read a gzip CSV intermediate into header + rows. The error names the jobs to run first.
pub fn read(
    data_dir: &Path,
    name: &str,
    written_by: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    let p = path(data_dir, name);
    let file = std::fs::File::open(&p).map_err(|e| {
        data_err(format!(
            "{} is needed but could not be read ({e}). It is written by the `{written_by}` job: run that job in the same refresh, or run it once with --keep-raw.",
            p.display()
        ))
    })?;
    let gz = flate2::read::MultiGzDecoder::new(BufReader::with_capacity(1 << 20, file));
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::with_capacity(1 << 20, gz));
    let header: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let mut rows = Vec::new();
    for rec in rdr.records() {
        rows.push(rec?.iter().map(|s| s.to_string()).collect());
    }
    Ok((header, rows))
}

/// Stream a gzip CSV intermediate line by line (for the large ones), calling `f` with each
/// record. Returns the header.
pub fn for_each(
    data_dir: &Path,
    name: &str,
    written_by: &str,
    mut f: impl FnMut(&csv::StringRecord) -> Result<()>,
) -> Result<Vec<String>> {
    let p = path(data_dir, name);
    let file = std::fs::File::open(&p).map_err(|e| {
        data_err(format!(
            "{} is needed but could not be read ({e}). It is written by the `{written_by}` job: run that job in the same refresh, or run it once with --keep-raw.",
            p.display()
        ))
    })?;
    let gz = flate2::read::MultiGzDecoder::new(BufReader::with_capacity(1 << 20, file));
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::with_capacity(1 << 20, gz));
    let header: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let mut rec = csv::StringRecord::new();
    while rdr.read_record(&mut rec)? {
        f(&rec)?;
    }
    Ok(header)
}

/// True when an intermediate exists (jobs use it to say which input is missing).
pub fn exists(data_dir: &Path, name: &str) -> bool {
    path(data_dir, name).exists()
}

/// Remove every intermediate (called at the end of a refresh without `--keep-raw`).
pub fn clear(data_dir: &Path) -> Result<()> {
    crate::http::remove_raw(&data_dir.join(DIR))
}

/// Count the lines of a gzip file (for tests and diagnostics).
pub fn line_count(p: &Path) -> Result<usize> {
    let file = std::fs::File::open(p)?;
    let gz = flate2::read::MultiGzDecoder::new(BufReader::new(file));
    Ok(BufReader::new(gz).lines().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_byte_stable() {
        let dir = std::env::temp_dir().join(format!("rr-etl-int-{}", std::process::id()));
        let rows = vec![
            vec!["a".to_string(), "1".to_string()],
            vec!["b,c".to_string(), "2".to_string()],
        ];
        write(&dir, "t.csv.gz", &["k", "v"], &rows).unwrap();
        let first = std::fs::read(path(&dir, "t.csv.gz")).unwrap();
        write(&dir, "t.csv.gz", &["k", "v"], &rows).unwrap();
        assert_eq!(first, std::fs::read(path(&dir, "t.csv.gz")).unwrap());
        let (h, r) = read(&dir, "t.csv.gz", "test").unwrap();
        assert_eq!(h, vec!["k", "v"]);
        assert_eq!(r, rows);
        let mut n = 0;
        for_each(&dir, "t.csv.gz", "test", |_| {
            n += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(n, 2);
        assert!(
            read(&dir, "missing.csv.gz", "outages")
                .unwrap_err()
                .to_string()
                .contains("outages")
        );
        clear(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }
}
