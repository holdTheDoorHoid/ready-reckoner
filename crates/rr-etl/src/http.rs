//! HTTP downloads with retries, sha256 of every raw input, and optional raw copies.
//!
//! The client identifies itself honestly (project name and repository URL). It never imitates a
//! browser: hosts that refuse scripted clients are reached through the APIs they publish for
//! that purpose (ArcGIS REST, ArcGIS Hub, OpenFEMA), as `docs/research/data-sources.md` advises.

use crate::{EtlError, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// User-Agent sent with every request.
pub const USER_AGENT: &str =
    "ready-reckoner-etl/0.1 (+https://github.com/holdTheDoorHoid/ready-reckoner; build-time data refresh)";

/// A fetched document held in memory.
#[derive(Debug, Clone)]
pub struct Fetched {
    /// The URL that was requested.
    pub url: String,
    /// The URL that finally answered (after redirects).
    pub final_url: String,
    /// Body bytes.
    pub bytes: Vec<u8>,
    /// Lower-case hex sha256 of `bytes`.
    pub sha256: String,
    /// UTC timestamp of retrieval.
    pub retrieved: String,
    /// The `Last-Modified` header, when the server sent one.
    pub last_modified: Option<String>,
}

impl Fetched {
    /// Body as UTF-8 text (invalid sequences replaced), with a leading byte-order mark removed.
    pub fn text(&self) -> String {
        let s = String::from_utf8_lossy(&self.bytes);
        s.trim_start_matches('\u{feff}').to_string()
    }
}

/// Result of consuming a streamed download.
#[derive(Debug, Clone)]
pub struct Streamed {
    /// The URL that finally answered.
    pub final_url: String,
    /// sha256 of every byte read from the stream.
    pub sha256: String,
    /// Number of bytes read.
    pub bytes: u64,
    /// UTC timestamp when the stream started.
    pub retrieved: String,
}

/// Hex-encode a digest.
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// sha256 of a byte slice as lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Incremental sha256 over several pages of one logical source (for paged APIs).
#[derive(Debug, Default, Clone)]
pub struct Sha256Acc {
    hasher: Sha256,
    bytes: u64,
}

impl Sha256Acc {
    /// Start an empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }
    /// Feed bytes.
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
        self.bytes += data.len() as u64;
    }
    /// Total bytes fed so far.
    pub fn len(&self) -> u64 {
        self.bytes
    }
    /// True when nothing has been fed.
    pub fn is_empty(&self) -> bool {
        self.bytes == 0
    }
    /// Finish and return the hex digest.
    pub fn finish(self) -> String {
        hex(&self.hasher.finalize())
    }
}

/// A reader adapter that hashes and counts everything read through it and can tee the bytes to
/// a raw copy on disk.
pub struct HashingReader<R: Read> {
    inner: R,
    hasher: Sha256,
    count: u64,
    tee: Option<BufWriter<File>>,
}

impl<R: Read> HashingReader<R> {
    /// Wrap `inner`; if `tee` is given, every byte read is also written there.
    pub fn new(inner: R, tee: Option<File>) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            count: 0,
            tee: tee.map(|f| BufWriter::with_capacity(1 << 20, f)),
        }
    }
    /// Bytes read so far.
    pub fn count(&self) -> u64 {
        self.count
    }
    /// Finish: flush the tee and return (sha256 hex, byte count).
    pub fn finish(mut self) -> Result<(String, u64)> {
        if let Some(t) = self.tee.as_mut() {
            t.flush()?;
        }
        Ok((hex(&self.hasher.finalize()), self.count))
    }
}

impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.hasher.update(&buf[..n]);
            self.count += n as u64;
            if let Some(t) = self.tee.as_mut() {
                t.write_all(&buf[..n])?;
            }
        }
        Ok(n)
    }
}

/// Blocking HTTP client shared by all jobs.
pub struct Http {
    client: reqwest::blocking::Client,
    /// Directory for raw copies (`data/raw`).
    pub raw_dir: PathBuf,
    /// Keep raw copies of every download under `raw_dir`.
    pub keep_raw: bool,
    /// Number of attempts per request.
    pub attempts: u32,
}

impl Http {
    /// Build the client.
    pub fn new(raw_dir: PathBuf, keep_raw: bool) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(30))
            .timeout(None::<Duration>)
            .build()
            .map_err(|e| EtlError::Http(format!("could not build HTTP client: {e}")))?;
        Ok(Self { client, raw_dir, keep_raw, attempts: 4 })
    }

    fn backoff(attempt: u32) {
        std::thread::sleep(Duration::from_secs(2u64.pow(attempt.min(5))));
    }

    fn check(resp: reqwest::blocking::Response, url: &str) -> Result<reqwest::blocking::Response> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp)
        } else {
            Err(EtlError::Http(format!("{url} answered HTTP {status}")))
        }
    }

    /// GET a document into memory, with retries. `raw_name` names the raw copy if `--keep-raw`.
    pub fn get(&self, url: &str, raw_name: Option<&str>) -> Result<Fetched> {
        self.fetch_with(url, raw_name, |c| c.get(url))
    }

    /// POST a form (used for ArcGIS queries with long field lists), with retries.
    pub fn post_form(&self, url: &str, form: &[(&str, String)]) -> Result<Fetched> {
        self.fetch_with(url, None, |c| c.post(url).form(form))
    }

    fn fetch_with(
        &self,
        url: &str,
        raw_name: Option<&str>,
        build: impl Fn(&reqwest::blocking::Client) -> reqwest::blocking::RequestBuilder,
    ) -> Result<Fetched> {
        let mut last = String::new();
        for attempt in 0..self.attempts {
            if attempt > 0 {
                Self::backoff(attempt);
            }
            let retrieved = crate::timefmt::now_utc();
            let resp = match build(&self.client).timeout(Duration::from_secs(600)).send() {
                Ok(r) => r,
                Err(e) => {
                    last = format!("{url}: {e}");
                    continue;
                }
            };
            let status = resp.status();
            if status.as_u16() == 404 || status.as_u16() == 403 || status.as_u16() == 401 {
                // Not transient: report at once.
                return Err(EtlError::Http(format!("{url} answered HTTP {status}")));
            }
            let resp = match Self::check(resp, url) {
                Ok(r) => r,
                Err(e) => {
                    last = e.to_string();
                    continue;
                }
            };
            let final_url = resp.url().to_string();
            let last_modified = resp
                .headers()
                .get(reqwest::header::LAST_MODIFIED)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            match resp.bytes() {
                Ok(b) => {
                    let bytes = b.to_vec();
                    let sha256 = sha256_hex(&bytes);
                    if let (true, Some(name)) = (self.keep_raw, raw_name) {
                        self.save_raw(name, &bytes)?;
                    }
                    return Ok(Fetched { url: url.to_string(), final_url, bytes, sha256, retrieved, last_modified });
                }
                Err(e) => last = format!("{url}: body read failed: {e}"),
            }
        }
        Err(EtlError::Http(last))
    }

    /// Stream a large download through `consume` without holding it in memory or on disk
    /// (unless `--keep-raw` and `raw_name` is given, which tees it to `raw_dir/raw_name`). The whole stream is retried
    /// from the start if the connection fails, so `consume` must be restartable: it is called
    /// once per attempt and must reset its own state.
    pub fn stream<T>(
        &self,
        url: &str,
        raw_name: Option<&str>,
        mut consume: impl FnMut(&mut dyn Read) -> Result<T>,
    ) -> Result<(T, Streamed)> {
        let mut last = String::new();
        for attempt in 0..self.attempts {
            if attempt > 0 {
                eprintln!("  retrying {url} (attempt {})", attempt + 1);
                Self::backoff(attempt);
            }
            let retrieved = crate::timefmt::now_utc();
            let resp = match self.client.get(url).send() {
                Ok(r) => r,
                Err(e) => {
                    last = format!("{url}: {e}");
                    continue;
                }
            };
            let resp = match Self::check(resp, url) {
                Ok(r) => r,
                Err(e) => {
                    last = e.to_string();
                    continue;
                }
            };
            let final_url = resp.url().to_string();
            // `raw_name: None` means never keep a copy (used for multi-gigabyte inputs, which
            // would break the disk budget even with --keep-raw).
            let tee = match (self.keep_raw, raw_name) {
                (true, Some(name)) => Some(self.raw_file(name)?),
                _ => None,
            };
            let mut reader = HashingReader::new(resp, tee);
            match consume(&mut reader) {
                Ok(v) => {
                    // Drain anything the consumer did not read so the hash covers the whole body.
                    std::io::copy(&mut reader, &mut std::io::sink())?;
                    let (sha256, bytes) = reader.finish()?;
                    return Ok((v, Streamed { final_url, sha256, bytes, retrieved }));
                }
                Err(e) => {
                    last = format!("{url}: {e}");
                    if matches!(e, EtlError::Data(_)) {
                        return Err(e);
                    }
                }
            }
        }
        Err(EtlError::Http(last))
    }

    fn raw_file(&self, name: &str) -> Result<File> {
        let path = self.raw_dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(File::create(path)?)
    }

    /// Save a raw copy (only called when `--keep-raw`).
    pub fn save_raw(&self, name: &str, bytes: &[u8]) -> Result<PathBuf> {
        let path = self.raw_dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        Ok(path)
    }
}

/// Read one named entry of a ZIP archive held in memory.
pub fn zip_entry(zip_bytes: &[u8], name_suffix: &str) -> Result<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))?;
    let mut found = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        if entry.name().to_ascii_lowercase().ends_with(&name_suffix.to_ascii_lowercase()) {
            found = Some(i);
            break;
        }
    }
    let idx = found.ok_or_else(|| crate::data_err(format!("ZIP has no entry ending in {name_suffix}")))?;
    let mut entry = archive.by_index(idx)?;
    let mut out = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut out)?;
    Ok(out)
}

/// Delete a raw file or directory if it exists (used when a job finishes and `--keep-raw` is off).
pub fn remove_raw(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
