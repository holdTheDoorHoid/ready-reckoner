//! Loaders for the data-pack v2 calibration files (the `data-model` workstream): the regional
//! outage model, the stress table, pooled restoration curves, temperature shares, utility
//! reliability, disaster declarations, the national series under `core/series/`, and the
//! per-event outage tables (`core/outage_events.csv`, `core/outage_holdout.csv`; bundled into the
//! core pack, DESIGN-DELTA-v3 §8, 2026-09-27; formerly the optional `outage_events` pack).
//!
//! Kept apart from `lib.rs` so the county-record assembly there only asks this module to fill
//! four fields ([`Calibration::fill`]).

use std::collections::BTreeMap;

use rr_types::{
    CountyRecord, Declarations, EngineError, OutageModel, PoolBasis, Reliability, RestorationCurve,
    StressEvent, TemperatureProfile,
};

use crate::table::{Csv, corrupt, f32c};

/// Calibration pack files this module reads.
pub const FILES: &[&str] = &[
    "core/outage_pooled.csv",
    "core/outage_causes.csv",
    "core/outage_curves.csv",
    "core/outage_stress.csv",
    "core/temperature.csv",
    "core/reliability.csv",
    "core/declarations.csv",
    "core/series/oe417.toml",
    "core/series/drug_shortages.toml",
    "core/series/fdic_failures.toml",
    "core/series/funding_gaps.toml",
    "core/series/fcc_dirs.toml",
    "core/series/fbi_arrests.toml",
    "core/series/ihp_displacement.toml",
    "core/outage_events.csv",
    "core/outage_holdout.csv",
];

/// Outage lengths (days) of the curve columns `s_1d` ... `s_30d`.
const CURVE_DAYS: [(f32, &str); 5] = [
    (1.0, "s_1d"),
    (3.0, "s_3d"),
    (7.0, "s_7d"),
    (14.0, "s_14d"),
    (30.0, "s_30d"),
];

/// A value in a series table row.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(untagged)]
pub enum SeriesValue {
    /// A flag.
    Bool(bool),
    /// A number.
    Number(f64),
    /// Text.
    Text(String),
}

/// One `[[rate]]` of a series file: a national or regional rate with the figure it comes from.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeriesRate {
    /// Stable id.
    pub id: String,
    /// The rate.
    pub value: f64,
    /// Unit in words.
    pub unit: String,
    /// Low end of a plausible range.
    #[serde(default)]
    pub low: Option<f64>,
    /// High end of a plausible range.
    #[serde(default)]
    pub high: Option<f64>,
    /// Last year the rate describes.
    pub year: u16,
    /// Years it is computed over.
    #[serde(default)]
    pub period: String,
    /// The published figure or count.
    pub figure: String,
    /// The arithmetic.
    pub derivation: String,
    /// Caveats.
    #[serde(default)]
    pub note: String,
    /// True when a hand-copied figure has not been checked against a primary source.
    #[serde(default)]
    pub unverified: bool,
}

/// A national series file (`core/series/<id>.toml`).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeriesFile {
    /// Stable id (the file stem).
    pub id: String,
    /// Title.
    pub title: String,
    /// Publisher.
    pub publisher: String,
    /// URL.
    pub url: String,
    /// Licence.
    pub licence: String,
    /// Credit line, when the licence asks for one.
    #[serde(default)]
    pub attribution: String,
    /// Citation id for `content/citations.toml`.
    pub source: String,
    /// When read, or "each refresh".
    pub retrieved: String,
    /// `fetched` or `transcribed`.
    pub how: String,
    /// Notes.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Rates.
    #[serde(default)]
    pub rate: Vec<SeriesRate>,
    /// The table the rates come from.
    #[serde(default)]
    pub row: Vec<BTreeMap<String, SeriesValue>>,
}

impl SeriesFile {
    /// One rate by id.
    pub fn rate(&self, id: &str) -> Option<&SeriesRate> {
        self.rate.iter().find(|r| r.id == id)
    }
}

/// One county outage event of a day or more (`core/outage_events.csv`).
#[derive(Debug, Clone, PartialEq)]
pub struct CountyOutageEvent {
    /// Start, `YYYY-MM-DD HH:MM` UTC.
    pub start: String,
    /// Cause class.
    pub class: String,
    /// Storm id, Storm Events type and episode, or OE-417 kind.
    pub cause: String,
    /// Share of the county's customers out at the peak.
    pub peak_share: f32,
    /// (days after the peak, share of the peak still out).
    pub share_out_at_days: Vec<(f32, f32)>,
    /// Customer outages lasting a day or more, per county customer.
    pub ge_1d_share: f32,
    /// Customer outages lasting a week or more, per county customer.
    pub ge_7d_share: f32,
}

/// One row of the outage model's held-out test (`core/outage_holdout.csv`).
#[derive(Debug, Clone, PartialEq)]
pub struct HoldoutRow {
    /// Split ("2014-2019 -> 2020-2025", "even years -> odd years").
    pub split: String,
    /// Outage length, days.
    pub mark_days: u32,
    /// "county only", "region only" or "blend".
    pub estimator: String,
    /// `customer_outages`, `mean_poisson_deviance` or `reliability`.
    pub measure: String,
    /// Reliability bin label (empty otherwise).
    pub bin: String,
    /// Predicted value.
    pub predicted: Option<f64>,
    /// Observed value.
    pub observed: Option<f64>,
    /// Counties or county-years.
    pub n: u32,
}

/// Cause shares of one county: all recorded outages, and those lasting a day or more.
type CauseShares = (BTreeMap<String, f32>, BTreeMap<String, f32>);

/// Everything the calibration files hold.
#[derive(Debug, Default, Clone)]
pub struct Calibration {
    pooled: BTreeMap<String, OutageModel>,
    causes: BTreeMap<String, CauseShares>,
    stress: BTreeMap<String, StressEvent>,
    curves: Vec<RestorationCurve>,
    temperature: BTreeMap<String, TemperatureProfile>,
    reliability: BTreeMap<String, Reliability>,
    declarations: BTreeMap<String, Declarations>,
    series: BTreeMap<String, SeriesFile>,
    events: BTreeMap<String, Vec<CountyOutageEvent>>,
    holdout: Vec<HoldoutRow>,
}

fn u(cell: &str) -> u32 {
    cell.trim()
        .parse::<f64>()
        .map_or(0, |v| v.round().max(0.0) as u32)
}

fn curve(t: &Csv, r: &[String]) -> Vec<(f32, f32)> {
    CURVE_DAYS
        .iter()
        .filter_map(|(d, c)| Some((*d, f32c(&r[t.opt_col(c)?])?)))
        .collect()
}

impl Calibration {
    /// Whether this module reads `name`.
    pub fn handles(name: &str) -> bool {
        FILES.contains(&name)
    }

    /// Parse one calibration file; returns the rows (or entries) read.
    pub fn parse(&mut self, name: &str, bytes: &[u8]) -> Result<u32, EngineError> {
        if name.ends_with(".toml") {
            let text = std::str::from_utf8(bytes).map_err(|e| corrupt(name, e))?;
            let s: SeriesFile = toml::from_str(text).map_err(|e| corrupt(name, e))?;
            // Every top-level array counts, as the manifest counts TOML entries.
            let n = (s.rate.len() + s.row.len() + s.notes.len()) as u32;
            self.series.insert(s.id.clone(), s);
            return Ok(n);
        }
        let t = Csv::parse(name, bytes)?;
        let n = t.rows.len() as u32;
        match name {
            "core/outage_pooled.csv" => {
                let i_f = t.col("fips")?;
                let i_b = t.col("basis")?;
                let i_r = t.col("rate")?;
                let lam: Vec<usize> = [
                    "lam_ge_1d",
                    "lam_ge_3d",
                    "lam_ge_7d",
                    "lam_ge_14d",
                    "lam_ge_30d",
                ]
                .iter()
                .map(|c| t.col(c))
                .collect::<Result<_, _>>()?;
                let z: Vec<usize> = ["z_1d", "z_3d", "z_7d", "z_14d", "z_30d"]
                    .iter()
                    .map(|c| t.col(c))
                    .collect::<Result<_, _>>()?;
                let i_n = t.col("region_counties")?;
                self.pooled.clear();
                for r in &t.rows {
                    let basis = match r[i_b].as_str() {
                        "region_only" => PoolBasis::RegionOnly,
                        "own_only" => PoolBasis::OwnOnly,
                        _ => PoolBasis::Blend,
                    };
                    let mut m = OutageModel {
                        basis,
                        rate: f32c(&r[i_r]).unwrap_or(0.0),
                        region_counties: u(&r[i_n]),
                        ..Default::default()
                    };
                    for k in 0..5 {
                        m.lam_ge[k] = f32c(&r[lam[k]])
                            .ok_or_else(|| corrupt(name, format!("missing rate for {}", r[i_f])))?;
                        m.z[k] = f32c(&r[z[k]]).unwrap_or(0.0);
                    }
                    self.pooled.insert(r[i_f].clone(), m);
                }
            }
            "core/outage_causes.csv" => {
                let i_f = t.col("fips")?;
                self.causes.clear();
                for r in &t.rows {
                    let mut all = BTreeMap::new();
                    let mut long = BTreeMap::new();
                    for (i, h) in t.header.iter().enumerate() {
                        let Some(v) = f32c(&r[i]) else { continue };
                        if v <= 0.0 {
                            continue;
                        }
                        if let Some(c) = h.strip_suffix("_share_ge_1d") {
                            long.insert(c.to_string(), v);
                        } else if let Some(c) = h.strip_suffix("_share") {
                            all.insert(c.to_string(), v);
                        }
                    }
                    self.causes.insert(r[i_f].clone(), (all, long));
                }
            }
            "core/outage_stress.csv" => {
                let ix = |c: &str| t.col(c);
                let (i_f, i_e, i_c, i_ca, i_d, i_r, i_k, i_p, i_s) = (
                    ix("fips")?,
                    ix("event")?,
                    ix("class")?,
                    ix("cause")?,
                    ix("date")?,
                    ix("recorded_in")?,
                    ix("distance_km")?,
                    ix("peak_share")?,
                    ix("source")?,
                );
                self.stress.clear();
                for r in &t.rows {
                    self.stress.insert(
                        r[i_f].clone(),
                        StressEvent {
                            event: r[i_e].clone(),
                            class: r[i_c].clone(),
                            cause: r[i_ca].clone(),
                            date: r[i_d].clone(),
                            recorded_in: (!r[i_r].is_empty()).then(|| r[i_r].clone()),
                            distance_km: f32c(&r[i_k]),
                            peak_share: f32c(&r[i_p]).unwrap_or(0.0),
                            share_out_at_days: curve(&t, r),
                            source: r[i_s].clone(),
                        },
                    );
                }
            }
            "core/outage_curves.csv" => {
                let ix = |c: &str| t.col(c);
                let (i_r, i_c, i_e, i_50, i_90, i_x) = (
                    ix("region")?,
                    ix("class")?,
                    ix("events")?,
                    ix("t50_days")?,
                    ix("t90_days")?,
                    ix("factor")?,
                );
                self.curves = t
                    .rows
                    .iter()
                    .map(|r| RestorationCurve {
                        region: r[i_r].clone(),
                        class: r[i_c].clone(),
                        events: u(&r[i_e]),
                        share_out_at_days: curve(&t, r),
                        t50_days: f32c(&r[i_50]).unwrap_or(0.0),
                        t90_days: f32c(&r[i_90]).unwrap_or(0.0),
                        factor: f32c(&r[i_x]),
                    })
                    .collect();
            }
            "core/temperature.csv" => {
                let i_f = t.col("fips")?;
                let months = |prefix: &str| -> Result<Vec<usize>, EngineError> {
                    (1..=12)
                        .map(|m| t.col(&format!("{prefix}_{m:02}")))
                        .collect()
                };
                let (h90, h100, c20, c0) = (
                    months("tmax_ge_90f")?,
                    months("tmax_ge_100f")?,
                    months("tmin_le_20f")?,
                    months("tmin_le_0f")?,
                );
                let opt = |c: &str| t.opt_col(c);
                let (o_h, o_c, r_h, r_c) = (
                    opt("outage_hot_share"),
                    opt("outage_cold_share"),
                    opt("region_outage_hot_share"),
                    opt("region_outage_cold_share"),
                );
                self.temperature.clear();
                for r in &t.rows {
                    let arr = |cols: &[usize]| {
                        let mut a = [0.0f32; 12];
                        for (k, c) in cols.iter().enumerate() {
                            a[k] = f32c(&r[*c]).unwrap_or(0.0);
                        }
                        a
                    };
                    let get = |c: Option<usize>| c.and_then(|i| f32c(&r[i]));
                    self.temperature.insert(
                        r[i_f].clone(),
                        TemperatureProfile {
                            tmax_ge_90f: arr(&h90),
                            tmax_ge_100f: arr(&h100),
                            tmin_le_20f: arr(&c20),
                            tmin_le_0f: arr(&c0),
                            outage_hot_share: get(o_h),
                            outage_cold_share: get(o_c),
                            region_outage_hot_share: get(r_h),
                            region_outage_cold_share: get(r_c),
                        },
                    );
                }
            }
            "core/reliability.csv" => {
                let ix = |c: &str| t.col(c);
                let (i_f, i_a, i_b, i_c, i_d, i_n) = (
                    ix("fips")?,
                    ix("saidi_with_med_min")?,
                    ix("saifi_with_med")?,
                    ix("saidi_without_med_min")?,
                    ix("saifi_without_med")?,
                    ix("utility_years")?,
                );
                self.reliability = t
                    .rows
                    .iter()
                    .map(|r| {
                        (
                            r[i_f].clone(),
                            Reliability {
                                saidi_with_med_min: f32c(&r[i_a]),
                                saifi_with_med: f32c(&r[i_b]),
                                saidi_without_med_min: f32c(&r[i_c]),
                                saifi_without_med: f32c(&r[i_d]),
                                utility_years: u(&r[i_n]),
                            },
                        )
                    })
                    .collect();
            }
            "core/declarations.csv" => {
                let ix = |c: &str| t.col(c);
                let (i_f, i_5, i_a, i_ia, i_h) = (
                    ix("fips")?,
                    ix("last_5yr")?,
                    ix("since_2000")?,
                    ix("with_individual_assistance")?,
                    ix("hurricane_or_flood")?,
                );
                let u16c = |c: &str| u(c).min(u32::from(u16::MAX)) as u16;
                self.declarations = t
                    .rows
                    .iter()
                    .map(|r| {
                        (
                            r[i_f].clone(),
                            Declarations {
                                last_5yr: u16c(&r[i_5]),
                                since_2000: u16c(&r[i_a]),
                                with_individual_assistance: u16c(&r[i_ia]),
                                hurricane_or_flood: u16c(&r[i_h]),
                            },
                        )
                    })
                    .collect();
            }
            "core/outage_events.csv" => {
                let ix = |c: &str| t.col(c);
                let (i_f, i_s, i_c, i_ca, i_p, i_1, i_7) = (
                    ix("county_fips")?,
                    ix("start")?,
                    ix("class")?,
                    ix("cause")?,
                    ix("peak_share")?,
                    ix("ge_1d_share")?,
                    ix("ge_7d_share")?,
                );
                self.events.clear();
                for r in &t.rows {
                    self.events
                        .entry(r[i_f].clone())
                        .or_default()
                        .push(CountyOutageEvent {
                            start: r[i_s].clone(),
                            class: r[i_c].clone(),
                            cause: r[i_ca].clone(),
                            peak_share: f32c(&r[i_p]).unwrap_or(0.0),
                            share_out_at_days: curve(&t, r),
                            ge_1d_share: f32c(&r[i_1]).unwrap_or(0.0),
                            ge_7d_share: f32c(&r[i_7]).unwrap_or(0.0),
                        });
                }
            }
            "core/outage_holdout.csv" => {
                let ix = |c: &str| t.col(c);
                let (i_s, i_m, i_e, i_me, i_b, i_p, i_o, i_n) = (
                    ix("split")?,
                    ix("mark_days")?,
                    ix("estimator")?,
                    ix("measure")?,
                    ix("bin")?,
                    ix("predicted")?,
                    ix("observed")?,
                    ix("n")?,
                );
                let fl = |c: &str| crate::table::f(c);
                self.holdout = t
                    .rows
                    .iter()
                    .map(|r| HoldoutRow {
                        split: r[i_s].clone(),
                        mark_days: u(&r[i_m]),
                        estimator: r[i_e].clone(),
                        measure: r[i_me].clone(),
                        bin: r[i_b].clone(),
                        predicted: fl(&r[i_p]),
                        observed: fl(&r[i_o]),
                        n: u(&r[i_n]),
                    })
                    .collect();
            }
            _ => return Err(corrupt(name, "not a calibration file")),
        }
        Ok(n)
    }

    /// Fill a county record's calibration fields from what is loaded.
    pub fn fill(&self, rec: &mut CountyRecord) {
        let fips = rec.fips.as_str();
        let mut model = self.pooled.get(fips).cloned();
        if model.is_none() && (self.causes.contains_key(fips) || self.stress.contains_key(fips)) {
            model = Some(OutageModel::default());
        }
        if let Some(m) = model.as_mut() {
            if let Some((all, long)) = self.causes.get(fips) {
                m.causes = all.clone();
                m.causes_ge_1d = long.clone();
            }
            m.stress = self.stress.get(fips).cloned();
        }
        rec.outage_model = model;
        rec.temperature = self.temperature.get(fips).cloned();
        rec.reliability = self.reliability.get(fips).cloned();
        rec.declarations = self.declarations.get(fips).cloned();
    }

    /// Pooled restoration curves by region and cause.
    pub fn curves(&self) -> &[RestorationCurve] {
        &self.curves
    }

    /// A national series file by id (`oe417`, `drug_shortages`, ...).
    pub fn series(&self, id: &str) -> Option<&SeriesFile> {
        self.series.get(id)
    }

    /// Every loaded series file, by id.
    pub fn all_series(&self) -> &BTreeMap<String, SeriesFile> {
        &self.series
    }

    /// A county's outage events of a day or more (`core/outage_events.csv`; empty until it is
    /// loaded). Puerto Rico's record is one island-wide series, filed under San Juan (72127) and
    /// served for every municipio.
    pub fn county_events(&self, fips: &str) -> &[CountyOutageEvent] {
        let key = if fips.starts_with("72") {
            "72127"
        } else {
            fips
        };
        self.events.get(key).map_or(&[], |v| v.as_slice())
    }

    /// The outage model's held-out test (`core/outage_holdout.csv`).
    pub fn holdout(&self) -> &[HoldoutRow] {
        &self.holdout
    }
}

/// The outage-pooling region of a county: its NCA5 region, except that Puerto Rico and the US
/// Virgin Islands (separate grids) are regions of their own. Use it to pick a row of
/// [`crate::DataStore::restoration_curves`].
pub fn outage_region(county: &CountyRecord) -> String {
    match county.state_abbr.as_str() {
        "PR" => "puerto_rico".into(),
        "VI" => "virgin_islands".into(),
        _ => county.nca_region.clone(),
    }
}
