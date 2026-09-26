//! Ready Reckoner — `rr-data`: loads the data packs written by `rr-etl` and answers lookups.
//!
//! The web app fetches each pack file listed in `data/manifest.json` from its own origin and
//! hands the bytes to [`DataStore::load_pack`] with the file's manifest path (for example
//! `core/nri_hazards.csv`). The engine never fetches anything. Load `manifest.json` first: every
//! later file is checked against the sha256 recorded there (files loaded before the manifest are
//! checked when it arrives). Files can be loaded in any order; county records are reassembled
//! after each load, and anything not loaded yet is simply absent (`None` or an empty map).
//!
//! Lookups: [`DataStore::county`], [`DataStore::resolve_zip`], [`DataStore::resolve`] (a
//! [`LocationInput`] to a [`LocationResolved`], with the `ambiguous_zip` rule),
//! [`DataStore::search`], [`DataStore::climate_multiplier`], [`DataStore::base_rate`],
//! [`DataStore::attributions`] and [`DataStore::manifest`].
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

mod climate;
mod location;
pub mod manifest;
mod search;
mod table;

pub use climate::{CLIMATE_CLAMP, variables_for};
pub use location::AMBIGUOUS_ZIP_SHARE;
pub use manifest::Manifest;
pub use search::MAX_RESULTS;

use rr_types::{
    AfreqKind, Attribution, BaseRate, CitationId, CountyRecord, Date, EngineError, ErrorCode,
    EventRate, Facilities, FloodPriors, HazardId, LatLon, NriHazard, OutageStats, PackInfo,
    Seismic, Vulnerability,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use table::{Csv, b, corrupt, f, f32c};

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-data";

/// Every pack file the engine understands, by manifest path.
pub const PACK_FILES: &[&str] = &[
    "manifest.json",
    "core/counties.csv",
    "core/states.csv",
    "core/ct_crosswalk.csv",
    "core/zip_county.csv",
    "core/zip_centroids.csv",
    "core/nri_counties.csv",
    "core/nri_hazards.csv",
    "core/nri_semantics.toml",
    "core/outages.csv",
    "core/outages_state.csv",
    "core/events.csv",
    "core/seismic.csv",
    "core/climate.csv",
    "core/flood.csv",
    "core/facilities.csv",
    "core/zip_facilities.csv",
    "core/vulnerability.csv",
    "core/base_rates.toml",
    "geo/counties.json",
];

/// What the store keeps per county from `counties.csv`.
#[derive(Debug, Clone)]
struct Base {
    name: String,
    state_abbr: String,
    state_name: String,
    centroid: LatLon,
    nca_region: String,
}

#[derive(Debug, Clone, Default)]
struct NriCounty {
    population: Option<u32>,
    building_value_usd: Option<f64>,
    coastal: bool,
    tsunami_zone: bool,
}

#[derive(Debug, Clone)]
struct RawNri {
    hazard: HazardId,
    afreq: Option<f32>,
    expb: Option<f32>,
    expp: Option<f32>,
    ealb: Option<f32>,
    ealp: Option<f32>,
    ealt: Option<f32>,
    hlrb: Option<f32>,
    alrb: Option<f32>,
    risk_score: Option<f32>,
}

#[derive(Debug, Clone)]
struct RawOutage {
    events_per_customer_year: f32,
    p: [f32; 4],
    median_hours: f32,
    p90_hours: f32,
    years_covered: String,
}

/// Facility data per county.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CountyFacilityFlags {
    /// Any part of the county within 16 km (10 miles) of a nuclear plant.
    pub nuclear_within_16km: bool,
    /// Any part of the county within 80 km (50 miles) of a nuclear plant.
    pub nuclear_within_80km: bool,
}

/// Facility data per ZIP.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ZipFacilities {
    /// Distance from the ZIP centroid to the nearest nuclear plant, km.
    pub nearest_nuclear_km: Option<f32>,
    /// Toxics Release Inventory facilities within 5 km of the ZIP centroid.
    pub tri_within_5km: u16,
}

/// A national base rate with the figure it came from (`base_rates.toml`).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct BaseRateEntry {
    /// Stable id.
    pub id: String,
    /// The rate.
    pub value: f64,
    /// Unit of `value`.
    pub unit: String,
    /// Low end of a plausible range.
    #[serde(default)]
    pub low: Option<f64>,
    /// High end of a plausible range.
    #[serde(default)]
    pub high: Option<f64>,
    /// Citation id (for `content/citations.toml`).
    pub source: String,
    /// Year the rate describes.
    pub year: u16,
    /// The published figure.
    pub figure: String,
    /// How the value was computed from the figure.
    pub derivation: String,
    /// Caveats.
    #[serde(default)]
    pub note: String,
}

/// A publication cited by the base rates.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Publication {
    /// Citation id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Publisher.
    pub publisher: String,
    /// URL.
    pub url: String,
    /// When it was read (or "each refresh").
    #[serde(default)]
    pub retrieved: String,
}

#[derive(serde::Deserialize)]
struct BaseRatesFile {
    #[serde(default)]
    rate: Vec<BaseRateEntry>,
    #[serde(default)]
    publication: Vec<Publication>,
}

#[derive(serde::Deserialize)]
struct SemanticsFile {
    #[serde(default)]
    nri_version: String,
    #[serde(default)]
    nri_version_date: String,
    #[serde(default)]
    hazard: Vec<SemanticsEntry>,
}

/// What a National Risk Index hazard's annualised frequency means (`nri_semantics.toml`).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SemanticsEntry {
    /// Hazard id.
    pub id: String,
    /// NRI field prefix.
    pub nri_code: String,
    /// Events per year or yearly probability.
    pub afreq_kind: String,
    /// `distinct_events`, `event_days` or `modelled`.
    pub basis: String,
    /// Unit in words.
    pub unit: String,
    /// Period of record.
    pub period_of_record: String,
    /// Whether `1 - exp(-afreq)` is a sound yearly chance.
    pub poisson_ok: bool,
    /// Technical documentation section.
    pub section: String,
    /// Notes for modellers.
    pub notes: String,
}

/// Loads pack files and answers lookups. Cheap to create; load files as they arrive.
#[derive(Debug, Default)]
pub struct DataStore {
    manifest: Option<Manifest>,
    loaded: BTreeMap<String, u32>,
    shas: BTreeMap<String, String>,
    base: BTreeMap<String, Base>,
    nri_county: BTreeMap<String, NriCounty>,
    nri_rows: BTreeMap<String, Vec<RawNri>>,
    semantics: BTreeMap<HazardId, SemanticsEntry>,
    nri_version: String,
    outages: BTreeMap<String, RawOutage>,
    events: BTreeMap<String, BTreeMap<String, EventRate>>,
    seismic: BTreeMap<String, Seismic>,
    climate: BTreeMap<String, BTreeMap<String, f32>>,
    flood: BTreeMap<String, FloodPriors>,
    facilities: BTreeMap<String, (Facilities, CountyFacilityFlags)>,
    vulnerability: BTreeMap<String, (Vulnerability, Option<u32>)>,
    zip_county: BTreeMap<String, Vec<(String, f32)>>,
    zip_points: BTreeMap<String, LatLon>,
    zip_facilities: BTreeMap<String, ZipFacilities>,
    base_rates: Vec<BaseRate>,
    base_rate_entries: Vec<BaseRateEntry>,
    publications: Vec<Publication>,
    map_ids: BTreeSet<String>,
    counties: BTreeMap<String, CountyRecord>,
    defer_rebuild: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn bad_name(name: &str) -> EngineError {
    EngineError::new(
        ErrorCode::BadInput,
        format!(
            "{name} is not a data pack file this engine knows. Load the files listed in data/manifest.json."
        ),
    )
}

impl DataStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load one pack file. `name` is its path in the manifest (a leading `data/` is ignored).
    /// Returns the pack version and the number of rows (or entries) read.
    pub fn load_pack(&mut self, name: &str, bytes: &[u8]) -> Result<PackInfo, EngineError> {
        let name = name.trim_start_matches("./").trim_start_matches("data/");
        if !PACK_FILES.contains(&name) {
            return Err(bad_name(name));
        }
        let sha = sha256_hex(bytes);
        if name == "manifest.json" {
            let m: Manifest = serde_json::from_slice(bytes).map_err(|e| corrupt(name, e))?;
            // Check files that arrived before the manifest.
            for (path, got) in &self.shas {
                if let Some(entry) = m.file(path)
                    && &entry.sha256 != got
                {
                    return Err(EngineError::new(
                        ErrorCode::PackCorrupt,
                        format!(
                            "The data file {path} does not match its checksum in the manifest. Reload the page to fetch it again."
                        ),
                    ));
                }
            }
            let files = m.packs.values().map(|p| p.files.len()).sum::<usize>() as u32;
            self.manifest = Some(m);
            self.loaded.insert(name.to_string(), files);
            self.rebuild();
            return Ok(self.info(name, files));
        }
        if let Some(m) = &self.manifest {
            match m.file(name) {
                Some(entry) if entry.sha256 != sha => {
                    return Err(EngineError::new(
                        ErrorCode::PackCorrupt,
                        format!(
                            "The data file {name} does not match its checksum in the manifest. Reload the page to fetch it again."
                        ),
                    ));
                }
                Some(_) => {}
                None => {
                    return Err(EngineError::new(
                        ErrorCode::PackCorrupt,
                        format!(
                            "The data file {name} is not listed in the loaded manifest (the files may be from different refreshes)."
                        ),
                    ));
                }
            }
        }
        let rows = self.parse(name, bytes)?;
        self.shas.insert(name.to_string(), sha);
        self.loaded.insert(name.to_string(), rows);
        self.rebuild();
        Ok(self.info(name, rows))
    }

    /// Load several pack files and reassemble the county records once at the end (faster at
    /// start-up than calling [`Self::load_pack`] for each). Stops at the first error; files
    /// loaded before it stay loaded. A `manifest.json` among them is loaded first.
    pub fn load_many(&mut self, files: &[(&str, &[u8])]) -> Result<Vec<PackInfo>, EngineError> {
        let mut out = Vec::with_capacity(files.len());
        let norm = |n: &str| {
            n.trim_start_matches("./")
                .trim_start_matches("data/")
                .to_string()
        };
        if let Some((n, b)) = files.iter().find(|(n, _)| norm(n) == "manifest.json") {
            out.push(self.load_pack(n, b)?);
        }
        self.defer_rebuild = true;
        let result = files
            .iter()
            .filter(|(n, _)| norm(n) != "manifest.json")
            .try_for_each(|(n, b)| {
                out.push(self.load_pack(n, b)?);
                Ok(())
            });
        self.defer_rebuild = false;
        self.rebuild();
        result.map(|_| out)
    }

    fn info(&self, name: &str, rows: u32) -> PackInfo {
        PackInfo {
            name: name.to_string(),
            version: self
                .manifest
                .as_ref()
                .map(|m| m.pack_version.clone())
                .unwrap_or_else(|| "unversioned".to_string()),
            rows,
        }
    }

    fn parse(&mut self, name: &str, bytes: &[u8]) -> Result<u32, EngineError> {
        if name.ends_with(".toml") {
            let text = std::str::from_utf8(bytes).map_err(|e| corrupt(name, e))?;
            return match name {
                "core/nri_semantics.toml" => {
                    let s: SemanticsFile = toml::from_str(text).map_err(|e| corrupt(name, e))?;
                    self.nri_version = format!("{} ({})", s.nri_version, s.nri_version_date);
                    self.semantics.clear();
                    for h in &s.hazard {
                        let id = HazardId::from_str(&h.id)
                            .map_err(|_| corrupt(name, format!("unknown hazard id {}", h.id)))?;
                        AfreqKind::from_str(&h.afreq_kind).map_err(|_| {
                            corrupt(name, format!("unknown afreq_kind {}", h.afreq_kind))
                        })?;
                        self.semantics.insert(id, h.clone());
                    }
                    Ok(s.hazard.len() as u32)
                }
                "core/base_rates.toml" => {
                    let s: BaseRatesFile = toml::from_str(text).map_err(|e| corrupt(name, e))?;
                    self.base_rates = s
                        .rate
                        .iter()
                        .map(|r| BaseRate {
                            id: r.id.clone(),
                            value: r.value,
                            unit: r.unit.clone(),
                            low: r.low,
                            high: r.high,
                            source: CitationId::from(r.source.clone()),
                            year: r.year,
                            note: if r.note.is_empty() {
                                format!("{} ({}).", r.figure, r.derivation)
                            } else {
                                format!("{} ({}). {}", r.figure, r.derivation, r.note)
                            },
                        })
                        .collect();
                    let n = (s.rate.len() + s.publication.len()) as u32;
                    self.base_rate_entries = s.rate;
                    self.publications = s.publication;
                    Ok(n)
                }
                _ => Err(bad_name(name)),
            };
        }
        if name == "geo/counties.json" {
            let v: serde_json::Value =
                serde_json::from_slice(bytes).map_err(|e| corrupt(name, e))?;
            let feats = v["features"]
                .as_array()
                .ok_or_else(|| corrupt(name, "no features"))?;
            self.map_ids = feats
                .iter()
                .filter_map(|f| f["id"].as_str().map(|s| s.to_string()))
                .collect();
            return Ok(feats.len() as u32);
        }
        let t = Csv::parse(name, bytes)?;
        let n = t.rows.len() as u32;
        match name {
            "core/counties.csv" => {
                let (i_f, i_n, i_s, i_sn, i_lat, i_lon, i_r) = (
                    t.col("fips")?,
                    t.col("name")?,
                    t.col("state_abbr")?,
                    t.col("state_name")?,
                    t.col("lat")?,
                    t.col("lon")?,
                    t.col("nca_region")?,
                );
                self.base.clear();
                for r in &t.rows {
                    let (Some(lat), Some(lon)) = (f(&r[i_lat]), f(&r[i_lon])) else {
                        return Err(corrupt(name, format!("bad coordinates for {}", r[i_f])));
                    };
                    self.base.insert(
                        r[i_f].clone(),
                        Base {
                            name: r[i_n].clone(),
                            state_abbr: r[i_s].clone(),
                            state_name: r[i_sn].clone(),
                            centroid: LatLon { lat, lon },
                            nca_region: r[i_r].clone(),
                        },
                    );
                }
            }
            "core/states.csv" | "core/ct_crosswalk.csv" | "core/outages_state.csv" => {
                // Loaded for completeness and checksums; the engine reads county-level values.
            }
            "core/zip_county.csv" => {
                let (i_z, i_c, i_s) = (t.col("zip")?, t.col("county_fips")?, t.col("land_share")?);
                self.zip_county.clear();
                for r in &t.rows {
                    let share = f32c(&r[i_s])
                        .ok_or_else(|| corrupt(name, format!("bad share for ZIP {}", r[i_z])))?;
                    self.zip_county
                        .entry(r[i_z].clone())
                        .or_default()
                        .push((r[i_c].clone(), share));
                }
                for v in self.zip_county.values_mut() {
                    v.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                }
            }
            "core/zip_centroids.csv" => {
                let (i_z, i_lat, i_lon) = (t.col("zip")?, t.col("lat")?, t.col("lon")?);
                self.zip_points = t
                    .rows
                    .iter()
                    .filter_map(|r| {
                        Some((
                            r[i_z].clone(),
                            LatLon {
                                lat: f(&r[i_lat])?,
                                lon: f(&r[i_lon])?,
                            },
                        ))
                    })
                    .collect();
            }
            "core/nri_counties.csv" => {
                let (i_f, i_p, i_b, i_c, i_t) = (
                    t.col("fips")?,
                    t.col("population")?,
                    t.col("building_value_usd")?,
                    t.col("coastal")?,
                    t.col("tsunami_zone")?,
                );
                self.nri_county = t
                    .rows
                    .iter()
                    .map(|r| {
                        (
                            r[i_f].clone(),
                            NriCounty {
                                population: f(&r[i_p])
                                    .map(|v| v.round().clamp(0.0, u32::MAX as f64) as u32),
                                building_value_usd: f(&r[i_b]),
                                coastal: b(&r[i_c]),
                                tsunami_zone: b(&r[i_t]),
                            },
                        )
                    })
                    .collect();
            }
            "core/nri_hazards.csv" => {
                let ix = |c: &str| t.col(c);
                let (i_f, i_h) = (ix("fips")?, ix("hazard")?);
                let cols = [
                    ix("afreq")?,
                    ix("expb")?,
                    ix("expp")?,
                    ix("ealb")?,
                    ix("ealp")?,
                    ix("ealt")?,
                    ix("hlrb")?,
                    ix("alrb")?,
                    ix("risk_score")?,
                ];
                self.nri_rows.clear();
                for r in &t.rows {
                    let hazard = HazardId::from_str(&r[i_h])
                        .map_err(|_| corrupt(name, format!("unknown hazard {}", r[i_h])))?;
                    let v: Vec<Option<f32>> = cols.iter().map(|i| f32c(&r[*i])).collect();
                    self.nri_rows
                        .entry(r[i_f].clone())
                        .or_default()
                        .push(RawNri {
                            hazard,
                            afreq: v[0],
                            expb: v[1],
                            expp: v[2],
                            ealb: v[3],
                            ealp: v[4],
                            ealt: v[5],
                            hlrb: v[6],
                            alrb: v[7],
                            risk_score: v[8],
                        });
                }
            }
            "core/outages.csv" => {
                let ix = |c: &str| t.col(c);
                let (i_f, i_e, i_1, i_3, i_7, i_14, i_m, i_9, i_y) = (
                    ix("fips")?,
                    ix("events_per_customer_year")?,
                    ix("p_ge_1d")?,
                    ix("p_ge_3d")?,
                    ix("p_ge_7d")?,
                    ix("p_ge_14d")?,
                    ix("median_hours")?,
                    ix("p90_hours")?,
                    ix("years_covered")?,
                );
                self.outages.clear();
                for r in &t.rows {
                    let g = |i: usize| {
                        f32c(&r[i])
                            .ok_or_else(|| corrupt(name, format!("missing value for {}", r[i_f])))
                    };
                    self.outages.insert(
                        r[i_f].clone(),
                        RawOutage {
                            events_per_customer_year: g(i_e)?,
                            p: [g(i_1)?, g(i_3)?, g(i_7)?, g(i_14)?],
                            median_hours: g(i_m)?,
                            p90_hours: g(i_9)?,
                            years_covered: r[i_y].clone(),
                        },
                    );
                }
            }
            "core/events.csv" => {
                let (i_f, i_t, i_r, i_d, i_m, i_9) = (
                    t.col("fips")?,
                    t.col("event_type")?,
                    t.col("rate_per_year")?,
                    t.col("share_damaging")?,
                    t.col("median_days")?,
                    t.col("p90_days")?,
                );
                self.events.clear();
                for r in &t.rows {
                    let rate = f32c(&r[i_r]).ok_or_else(|| {
                        corrupt(name, format!("missing rate for {} {}", r[i_f], r[i_t]))
                    })?;
                    self.events.entry(r[i_f].clone()).or_default().insert(
                        r[i_t].clone(),
                        EventRate {
                            rate_per_year: rate,
                            share_damaging: f32c(&r[i_d]),
                            median_days: f32c(&r[i_m]),
                            p90_days: f32c(&r[i_9]),
                        },
                    );
                }
            }
            "core/seismic.csv" => {
                let (i_f, i_1, i_2, i_m) = (
                    t.col("fips")?,
                    t.col("p_pga_ge_0_1g_per_year")?,
                    t.col("p_pga_ge_0_2g_per_year")?,
                    t.col("mmi6_100yr")?,
                );
                self.seismic = t
                    .rows
                    .iter()
                    .filter_map(|r| {
                        Some((
                            r[i_f].clone(),
                            Seismic {
                                p_pga_ge_0_1g_per_year: f32c(&r[i_1])?,
                                p_pga_ge_0_2g_per_year: f32c(&r[i_2])?,
                                mmi6_100yr: f32c(&r[i_m]),
                            },
                        ))
                    })
                    .collect();
            }
            "core/climate.csv" => {
                let i_f = t.col("fips")?;
                self.climate = t
                    .rows
                    .iter()
                    .map(|r| {
                        let m: BTreeMap<String, f32> = t
                            .header
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != i_f)
                            .filter_map(|(i, h)| Some((h.clone(), f32c(&r[i])?)))
                            .collect();
                        (r[i_f].clone(), m)
                    })
                    .collect();
            }
            "core/flood.csv" => {
                let (i_f, i_s, i_c, i_m) = (
                    t.col("fips")?,
                    t.col("sfha_home_share")?,
                    t.col("claims_per_1000_policies_year")?,
                    t.col("mean_paid_usd")?,
                );
                self.flood = t
                    .rows
                    .iter()
                    .filter_map(|r| {
                        Some((
                            r[i_f].clone(),
                            FloodPriors {
                                sfha_home_share: f32c(&r[i_s])?,
                                claims_per_1000_policies_year: f32c(&r[i_c]),
                                mean_paid_usd: f32c(&r[i_m]),
                            },
                        ))
                    })
                    .collect();
            }
            "core/facilities.csv" => {
                let (i_f, i_n, i_t, i_h) = (
                    t.col("fips")?,
                    t.col("nearest_nuclear_km")?,
                    t.col("tri_facilities")?,
                    t.col("high_hazard_dams")?,
                );
                let (i_16, i_80) = (
                    t.opt_col("nuclear_within_16km"),
                    t.opt_col("nuclear_within_80km"),
                );
                let u16c = |s: &str| {
                    f(s).map(|v| v.round().clamp(0.0, u16::MAX as f64) as u16)
                        .unwrap_or(0)
                };
                self.facilities = t
                    .rows
                    .iter()
                    .map(|r| {
                        (
                            r[i_f].clone(),
                            (
                                Facilities {
                                    nearest_nuclear_km: f32c(&r[i_n]),
                                    tri_facilities: u16c(&r[i_t]),
                                    high_hazard_dams: u16c(&r[i_h]),
                                },
                                CountyFacilityFlags {
                                    nuclear_within_16km: i_16.is_some_and(|i| b(&r[i])),
                                    nuclear_within_80km: i_80.is_some_and(|i| b(&r[i])),
                                },
                            ),
                        )
                    })
                    .collect();
            }
            "core/zip_facilities.csv" => {
                let (i_z, i_n, i_t) = (
                    t.col("zip")?,
                    t.col("nearest_nuclear_km")?,
                    t.col("tri_within_5km")?,
                );
                self.zip_facilities = t
                    .rows
                    .iter()
                    .map(|r| {
                        let tri = f(&r[i_t])
                            .map(|v| v.round().clamp(0.0, u16::MAX as f64) as u16)
                            .unwrap_or(0);
                        (
                            r[i_z].clone(),
                            ZipFacilities {
                                nearest_nuclear_km: f32c(&r[i_n]),
                                tri_within_5km: tri,
                            },
                        )
                    })
                    .collect();
            }
            "core/vulnerability.csv" => {
                let (i_f, i_c, i_s, i_h) = (
                    t.col("fips")?,
                    t.col("cre_share_3plus_risk_factors")?,
                    t.col("svi_percentile")?,
                    t.col("households")?,
                );
                self.vulnerability = t
                    .rows
                    .iter()
                    .map(|r| {
                        let hh = f(&r[i_h]).map(|v| v.round().clamp(0.0, u32::MAX as f64) as u32);
                        (
                            r[i_f].clone(),
                            (
                                Vulnerability {
                                    cre_share_3plus_risk_factors: f32c(&r[i_c]),
                                    svi_percentile: f32c(&r[i_s]),
                                },
                                hh,
                            ),
                        )
                    })
                    .collect();
            }
            _ => return Err(bad_name(name)),
        }
        Ok(n)
    }

    /// Reassemble every county record from what is loaded (skipped inside [`Self::load_many`]).
    fn rebuild(&mut self) {
        if self.defer_rebuild {
            return;
        }
        let event_definition = self
            .manifest
            .as_ref()
            .and_then(|m| m.definition("outages", "event_definition"))
            .unwrap_or("EAGLE-I outage event (definition in data/manifest.json)")
            .to_string();
        let mut out = BTreeMap::new();
        for (fips, base) in &self.base {
            let nc = self.nri_county.get(fips).cloned().unwrap_or_default();
            let nri: BTreeMap<HazardId, NriHazard> = self
                .nri_rows
                .get(fips)
                .into_iter()
                .flatten()
                .filter_map(|r| {
                    let kind =
                        AfreqKind::from_str(&self.semantics.get(&r.hazard)?.afreq_kind).ok()?;
                    Some((
                        r.hazard,
                        NriHazard {
                            afreq: r.afreq,
                            afreq_kind: kind,
                            expb: r.expb,
                            expp: r.expp,
                            ealb: r.ealb,
                            ealp: r.ealp,
                            ealt: r.ealt,
                            hlrb: r.hlrb,
                            alrb: r.alrb,
                            risk_score: r.risk_score,
                        },
                    ))
                })
                .collect();
            let outages = self.outages.get(fips).map(|o| OutageStats {
                events_per_customer_year: o.events_per_customer_year,
                p_ge_1d: o.p[0],
                p_ge_3d: o.p[1],
                p_ge_7d: o.p[2],
                p_ge_14d: o.p[3],
                median_hours: o.median_hours,
                p90_hours: o.p90_hours,
                years_covered: o.years_covered.clone(),
                event_definition: event_definition.clone(),
            });
            let (vulnerability, households) = match self.vulnerability.get(fips) {
                Some((v, h)) => (Some(v.clone()), *h),
                None => (None, None),
            };
            out.insert(
                fips.clone(),
                CountyRecord {
                    fips: fips.clone(),
                    name: base.name.clone(),
                    state_abbr: base.state_abbr.clone(),
                    state_name: base.state_name.clone(),
                    centroid: base.centroid,
                    nca_region: base.nca_region.clone(),
                    coastal: nc.coastal,
                    tsunami_zone: nc.tsunami_zone,
                    population: nc.population,
                    households,
                    building_value_usd: nc.building_value_usd,
                    nri_version: self.nri_version.clone(),
                    nri,
                    outages,
                    events: self.events.get(fips).cloned().unwrap_or_default(),
                    seismic: self.seismic.get(fips).cloned(),
                    climate: self.climate.get(fips).cloned().unwrap_or_default(),
                    flood: self.flood.get(fips).cloned(),
                    facilities: self.facilities.get(fips).map(|x| x.0.clone()),
                    vulnerability,
                },
            );
        }
        self.counties = out;
    }

    /// The record for a county (five-digit FIPS), if `counties.csv` is loaded and it exists.
    ///
    /// `climate` holds every column of `climate.csv` by its header name: ratios to today (central
    /// and `_high`) and CMRA day counts (`*_hist`, `*_2050`, `*_2050_high`, which are counts, not
    /// multipliers). See `docs/DATA_SOURCES.md`.
    pub fn county(&self, fips: &str) -> Option<&CountyRecord> {
        self.counties.get(fips)
    }

    /// Every county record, in FIPS order.
    pub fn counties(&self) -> impl Iterator<Item = &CountyRecord> {
        self.counties.values()
    }

    /// The counties a ZIP code (ZCTA) covers, with the share of the ZIP's land in each, largest
    /// first. Empty when the ZIP is unknown or `zip_county.csv` is not loaded.
    pub fn resolve_zip(&self, zip: &str) -> Vec<(String, f32)> {
        self.zip_county.get(zip.trim()).cloned().unwrap_or_default()
    }

    pub(crate) fn zip_county_iter(&self) -> impl Iterator<Item = (&String, &Vec<(String, f32)>)> {
        self.zip_county.iter()
    }

    /// The ZIP centroid, if known.
    pub fn zip_centroid(&self, zip: &str) -> Option<LatLon> {
        self.zip_points.get(zip.trim()).copied()
    }

    /// Facility data for a ZIP, if known.
    pub fn zip_facilities(&self, zip: &str) -> Option<ZipFacilities> {
        self.zip_facilities.get(zip.trim()).copied()
    }

    /// County-level nuclear proximity flags, if `facilities.csv` is loaded.
    pub fn county_facility_flags(&self, fips: &str) -> Option<&CountyFacilityFlags> {
        self.facilities.get(fips).map(|x| &x.1)
    }

    /// A raw climate ratio (future / present) for a county and variable, as published in
    /// `climate.csv` (`Today` always gives 1).
    pub fn climate_ratio(
        &self,
        fips: &str,
        variable: &str,
        horizon: rr_types::ClimateHorizon,
    ) -> Option<f32> {
        match horizon {
            rr_types::ClimateHorizon::Today => Some(1.0),
            rr_types::ClimateHorizon::Y2050 => self.climate.get(fips)?.get(variable).copied(),
        }
    }

    /// National base rates, in file order.
    pub fn base_rates(&self) -> &[BaseRate] {
        &self.base_rates
    }

    /// One base rate by id.
    pub fn base_rate(&self, id: &str) -> Option<&BaseRate> {
        self.base_rates.iter().find(|r| r.id == id)
    }

    /// Base rates with their source figures and derivations.
    pub fn base_rate_entries(&self) -> &[BaseRateEntry] {
        &self.base_rate_entries
    }

    /// Publications the base rates cite (title, publisher, URL), for `content/citations.toml`.
    pub fn publications(&self) -> &[Publication] {
        &self.publications
    }

    /// What each NRI hazard's `AFREQ` means, if `nri_semantics.toml` is loaded.
    pub fn nri_semantics(&self, hazard: HazardId) -> Option<&SemanticsEntry> {
        self.semantics.get(&hazard)
    }

    /// The NRI version the county records carry, e.g. `1.20.0 (December 2025)`.
    pub fn nri_version(&self) -> &str {
        &self.nri_version
    }

    /// The loaded manifest.
    pub fn manifest(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }

    /// The data pack version (content hash), once the manifest is loaded.
    pub fn pack_version(&self) -> Option<&str> {
        self.manifest.as_ref().map(|m| m.pack_version.as_str())
    }

    /// Pack files loaded so far, with their row counts.
    pub fn loaded(&self) -> &BTreeMap<String, u32> {
        &self.loaded
    }

    /// County ids present in the map file, once `geo/counties.json` is loaded.
    pub fn map_ids(&self) -> &BTreeSet<String> {
        &self.map_ids
    }

    /// Credit lines and disclaimers the app must show (from the manifest), sorted by source.
    pub fn attributions(&self) -> Vec<Attribution> {
        let Some(m) = &self.manifest else {
            return Vec::new();
        };
        let fallback = Date::parse(m.generated.get(..10).unwrap_or("")).ok();
        m.attributions
            .iter()
            .filter_map(|a| {
                let accessed = Date::parse(&a.accessed).ok().or(fallback)?;
                Some(Attribution {
                    source: a.source.clone(),
                    text: a.text.clone(),
                    url: a.url.clone(),
                    version: a.version.clone(),
                    accessed,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_pack_names_are_rejected() {
        let mut s = DataStore::new();
        let e = s.load_pack("core/secrets.csv", b"a,b\n").unwrap_err();
        assert_eq!(e.code, ErrorCode::BadInput);
    }

    #[test]
    fn corrupt_csv_is_reported() {
        let mut s = DataStore::new();
        let e = s
            .load_pack("core/counties.csv", b"fips,name\n42101,Philadelphia\n")
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::PackCorrupt);
    }

    #[test]
    fn checksum_mismatch_is_pack_corrupt() {
        let mut s = DataStore::new();
        let manifest = serde_json::json!({
            "pack_version": "abc",
            "packs": { "core": { "files": [ { "path": "core/states.csv", "sha256": "00", "rows": 0 } ] } }
        });
        s.load_pack("manifest.json", manifest.to_string().as_bytes())
            .unwrap();
        let e = s.load_pack("core/states.csv", b"state_fips\n").unwrap_err();
        assert_eq!(e.code, ErrorCode::PackCorrupt);
    }

    #[test]
    fn a_small_store_assembles_records() {
        let mut s = DataStore::new();
        s.load_pack(
            "core/counties.csv",
            b"fips,name,name_full,state_abbr,state_name,lat,lon,land_sqmi,nca_region\n42101,Philadelphia,Philadelphia County,PA,Pennsylvania,40.0094,-75.1333,134.3,northeast\n",
        )
        .unwrap();
        assert_eq!(s.county("42101").unwrap().name, "Philadelphia");
        assert!(s.county("42101").unwrap().nri.is_empty());
        // NRI rows need the semantics file before they appear.
        s.load_pack("core/nri_hazards.csv", b"fips,hazard,afreq,expb,expp,ealb,ealp,ealt,hlrb,alrb,risk_score\n42101,heat_wave,11.07,1,2,3,4,5,6,7,99.97\n").unwrap();
        assert!(s.county("42101").unwrap().nri.is_empty());
        s.load_pack(
            "core/nri_semantics.toml",
            b"nri_version = \"1.20.0\"\nnri_version_date = \"December 2025\"\n[[hazard]]\nid = \"heat_wave\"\nnri_code = \"HWAV\"\nafreq_kind = \"events_per_year\"\nbasis = \"event_days\"\nunit = \"days\"\nperiod_of_record = \"19 years\"\npoisson_ok = false\nsection = \"12.5\"\nnotes = \"\"\n",
        )
        .unwrap();
        let rec = s.county("42101").unwrap();
        assert_eq!(rec.nri[&HazardId::HeatWave].afreq, Some(11.07));
        assert_eq!(
            rec.nri[&HazardId::HeatWave].afreq_kind,
            AfreqKind::EventsPerYear
        );
        assert_eq!(rec.nri_version, "1.20.0 (December 2025)");
    }
}
