//! Job — wildfire-smoke days per county, from two public-domain NOAA and EPA records:
//!
//! - **NOAA Hazard Mapping System (HMS) smoke polygons**: analysts outline the smoke they see on
//!   satellite images every day, with a density class (Light, Medium, Heavy). A county is
//!   *smoke-covered* on a day when its internal point lies inside a Medium or Heavy polygon.
//!   HMS sees the whole column of air, so smoke aloft counts too: that is why the ground
//!   measurements below are needed.
//! - **EPA AQS daily PM2.5** (FRM/FEM monitors, parameter 88101): the daily averages under the
//!   24-hour standard (sample durations `24 HOUR` and `24-HR BLK AVG`), excluding values with
//!   flagged exceptional-event data removed (`Event Type = Excluded`). A county's value for a
//!   day is its highest monitor.
//!
//! A *smoke day of concern* is a smoke-covered day whose 24-hour PM2.5 reaches 35.5 µg/m³ (the
//! AQI's "unhealthy for sensitive groups"); a *severe* one reaches 55.5 (AQI "unhealthy").
//! Monitors do not run every day (filter samplers run every third or sixth day), so each year's
//! count is the share of that year's observed smoke-covered days at or above the threshold,
//! times the year's smoke-covered days. Counties with fewer than ten observed smoke-covered days
//! in the period take the rate of the nearest county with enough within 150 km in the same NCA5
//! region (else the region's pooled rate) applied to their own smoke-covered days, and say so
//! (`smoke_basis = imputed`).
//!
//! Childs et al. (2022) county smoke PM2.5 is not used: its licence (CC BY-SA 4.0) does not fit
//! the packs.

use super::{County, Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::{Table, col, read_table};
use crate::geo::{haversine_km, point_in_ring};
use crate::http::{Sha256Acc, zip_entry};
use crate::manifest::{Attribution, SourceRecord};
use crate::num::sig;
use crate::raster::{CountyGeo, Rings, bbox_of, norm_lon};
use crate::shp::{Shape, read_dbf_field, read_shp};
use crate::timefmt::{civil_from_days, days_from_civil};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufReader, Read};
use std::sync::Mutex;

/// Smoke days per county.
pub const SMOKE: &str = "core/smoke.csv";

/// First and last year counted.
pub const FIRST_YEAR: i64 = 2016;
/// Last year counted (the brief's 2016-2023 window).
pub const LAST_YEAR: i64 = 2023;
/// Thresholds, µg/m³ (24-hour average): AQI "unhealthy for sensitive groups" and "unhealthy".
pub const THRESHOLDS: [f64; 2] = [35.5, 55.5];
/// Observed smoke-covered days a county needs for its own rate.
pub const MIN_OBSERVED: usize = 10;
/// Farthest neighbour an unmonitored county may borrow a rate from, km.
pub const IMPUTE_KM: f64 = 150.0;

const HMS_BASE: &str =
    "https://satepsanone.nesdis.noaa.gov/pub/FIRE/web/HMS/Smoke_Polygons/Shapefile";
const AQS_BASE: &str = "https://aqs.epa.gov/aqsweb/airdata";
const HMS_PAGE: &str = "https://www.ospo.noaa.gov/products/land/hms.html";

/// HMS daily file for a date.
pub fn hms_url(y: i64, m: u32, d: u32) -> String {
    format!("{HMS_BASE}/{y:04}/{m:02}/hms_smoke{y:04}{m:02}{d:02}.zip")
}

/// Whether an HMS density value counts: Medium or Heavy (text), or the numeric classes 16 and 27
/// µg/m³ used in some older files.
pub fn dense(v: &str) -> bool {
    let t = v.trim();
    if t.eq_ignore_ascii_case("medium") || t.eq_ignore_ascii_case("heavy") {
        return true;
    }
    t.parse::<f64>().is_ok_and(|x| x >= 16.0)
}

/// Medium and Heavy polygons of one HMS day, with their bounding boxes (normalised longitudes).
pub fn hms_polygons(zip: &[u8]) -> Result<Vec<(Rings, [f64; 4])>> {
    let shapes = read_shp(&zip_entry(zip, ".shp")?)?;
    let density = read_dbf_field(&zip_entry(zip, ".dbf")?, "Density")?;
    let mut out = Vec::new();
    for (s, d) in shapes.into_iter().zip(density.iter()) {
        if !dense(d) {
            continue;
        }
        if let Shape::Polygon(rings) = s {
            let rings = crate::raster::norm_rings(rings);
            let b = bbox_of(&rings);
            out.push((rings, b));
        }
    }
    Ok(out)
}

/// Indices of the points (lat, lon with normalised longitudes) inside any of the polygons.
pub fn covered(points: &[(f64, f64)], polys: &[(Rings, [f64; 4])]) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, &(lat, lon)) in points.iter().enumerate() {
        let inside = polys.iter().any(|(rings, b)| {
            lon >= b[0]
                && lon <= b[2]
                && lat >= b[1]
                && lat <= b[3]
                && rings.iter().filter(|r| point_in_ring(lon, lat, r)).count() % 2 == 1
        });
        if inside {
            out.push(i);
        }
    }
    out
}

/// One HMS day: the counties covered, the file's sha256 and its size.
type DayResult = (Vec<usize>, String, u64);

/// One AQS daily value: site location, day, 24-hour mean.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obs {
    /// Site latitude.
    pub lat: f64,
    /// Site longitude.
    pub lon: f64,
    /// Days since 1970-01-01.
    pub day: i64,
    /// 24-hour mean PM2.5, µg/m³.
    pub value: f64,
}

/// Parse an AQS daily PM2.5 CSV (a reader), keeping the 24-hour-standard daily averages.
pub fn parse_aqs(reader: impl Read) -> Result<Vec<Obs>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::with_capacity(1 << 20, reader));
    let h: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let (i_lat, i_lon, i_dur, i_date, i_ev, i_mean) = (
        col(&h, "Latitude")?,
        col(&h, "Longitude")?,
        col(&h, "Sample Duration")?,
        col(&h, "Date Local")?,
        col(&h, "Event Type")?,
        col(&h, "Arithmetic Mean")?,
    );
    let mut out = Vec::new();
    let mut rec = csv::StringRecord::new();
    while rdr.read_record(&mut rec)? {
        let dur = rec[i_dur].trim();
        if dur != "24 HOUR" && dur != "24-HR BLK AVG" {
            continue;
        }
        if rec[i_ev].trim() == "Excluded" {
            continue;
        }
        let date = rec[i_date].trim();
        let (Some(y), Some(m), Some(d)) = (
            date.get(0..4).and_then(|s| s.parse::<i64>().ok()),
            date.get(5..7).and_then(|s| s.parse::<u32>().ok()),
            date.get(8..10).and_then(|s| s.parse::<u32>().ok()),
        ) else {
            continue;
        };
        let (Ok(lat), Ok(lon), Ok(value)) = (
            rec[i_lat].trim().parse::<f64>(),
            rec[i_lon].trim().parse::<f64>(),
            rec[i_mean].trim().parse::<f64>(),
        ) else {
            continue;
        };
        out.push(Obs {
            lat,
            lon,
            day: days_from_civil(y, m, d),
            value,
        });
    }
    Ok(out)
}

/// A county's yearly tallies.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Year {
    /// Smoke-covered days.
    pub hms: u32,
    /// Smoke-covered days with a monitor value.
    pub observed: u32,
    /// Of those, at or above each threshold.
    pub over: [u32; 2],
}

/// Estimated yearly smoke days at each threshold from a county's tallies and an exceedance
/// fraction to use when a year has no observed smoke-covered day.
pub fn yearly_estimates(years: &[Year], fallback: [f64; 2]) -> Vec<[f64; 2]> {
    years
        .iter()
        .map(|y| {
            let mut e = [0.0; 2];
            for (k, slot) in e.iter_mut().enumerate() {
                *slot = if y.observed > 0 {
                    y.over[k] as f64 / y.observed as f64 * y.hms as f64
                } else {
                    fallback[k] * y.hms as f64
                };
            }
            e
        })
        .collect()
}

/// Least-squares slope of `v` against 0, 1, 2, ... (per step).
pub fn slope(v: &[f64]) -> f64 {
    let n = v.len() as f64;
    if v.len() < 2 {
        return 0.0;
    }
    let mx = (n - 1.0) / 2.0;
    let my = v.iter().sum::<f64>() / n;
    let (mut sxy, mut sxx) = (0.0, 0.0);
    for (i, y) in v.iter().enumerate() {
        let dx = i as f64 - mx;
        sxy += dx * (y - my);
        sxx += dx * dx;
    }
    if sxx == 0.0 { 0.0 } else { sxy / sxx }
}

fn fraction(years: &[Year]) -> Option<[f64; 2]> {
    let obs: u32 = years.iter().map(|y| y.observed).sum();
    if (obs as usize) < MIN_OBSERVED {
        return None;
    }
    let mut f = [0.0; 2];
    for (k, slot) in f.iter_mut().enumerate() {
        *slot = years.iter().map(|y| y.over[k]).sum::<u32>() as f64 / obs as f64;
    }
    Some(f)
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties: Vec<County> = load_counties(ctx)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let points: Vec<(f64, f64)> = counties.iter().map(|c| (c.lat, norm_lon(c.lon))).collect();
    let n_years = (LAST_YEAR - FIRST_YEAR + 1) as usize;
    let first_day = days_from_civil(FIRST_YEAR, 1, 1);
    let last_day = days_from_civil(LAST_YEAR, 12, 31);
    let n_days = (last_day - first_day + 1) as usize;

    // --- HMS: which counties are smoke-covered each day -------------------------------------
    // covered_days[county] = sorted day offsets. Four workers fetch days in parallel; every
    // result is keyed by day, so the output does not depend on the order they finish in.
    let per_day: Mutex<BTreeMap<usize, DayResult>> = Mutex::new(BTreeMap::new());
    let missing_days: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let failure: Mutex<Option<String>> = Mutex::new(None);
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if k >= n_days || failure.lock().map(|f| f.is_some()).unwrap_or(true) {
                        break;
                    }
                    let (y, m, d) = civil_from_days(first_day + k as i64);
                    let url = hms_url(y, m, d);
                    match ctx.http.get(&url, None) {
                        Ok(f) => match hms_polygons(&f.bytes) {
                            Ok(polys) => {
                                let hit = covered(&points, &polys);
                                if let Ok(mut map) = per_day.lock() {
                                    map.insert(k, (hit, f.sha256.clone(), f.bytes.len() as u64));
                                }
                            }
                            Err(e) => {
                                if let Ok(mut f) = failure.lock() {
                                    *f = Some(format!("{url}: {e}"));
                                }
                            }
                        },
                        Err(e) => {
                            let msg = e.to_string();
                            if msg.contains("404") {
                                if let Ok(mut v) = missing_days.lock() {
                                    v.push(format!("{y:04}-{m:02}-{d:02}"));
                                }
                            } else if let Ok(mut f) = failure.lock() {
                                *f = Some(msg);
                            }
                        }
                    }
                }
            });
        }
    });
    if let Some(e) = failure.into_inner().unwrap_or(None) {
        return Err(data_err(format!("HMS smoke polygons: {e}")));
    }
    let per_day = per_day.into_inner().unwrap_or_default();
    let mut missing_days = missing_days.into_inner().unwrap_or_default();
    missing_days.sort();
    if missing_days.len() > n_days / 20 {
        return Err(data_err(format!(
            "HMS: {} of {n_days} daily files missing; expected a handful",
            missing_days.len()
        )));
    }
    let mut acc = Sha256Acc::new();
    let mut hms_bytes = 0u64;
    let mut covered_days: Vec<Vec<usize>> = vec![Vec::new(); counties.len()];
    for (k, (hit, sha, bytes)) in &per_day {
        acc.update(sha.as_bytes());
        hms_bytes += bytes;
        for &c in hit {
            covered_days[c].push(*k);
        }
    }
    out.rows_in += per_day.len() as u64;
    out.source(SourceRecord {
        name: format!("NOAA NESDIS Hazard Mapping System smoke polygons, daily shapefiles {FIRST_YEAR}-{LAST_YEAR}"),
        url: format!("{HMS_BASE}/YYYY/MM/hms_smokeYYYYMMDD.zip"),
        version: format!(
            "{} daily files ({} missing); sha256 is of the files' sha256 values in date order",
            per_day.len(),
            missing_days.len()
        ),
        retrieved: crate::timefmt::now_utc(),
        sha256: acc.finish(),
        bytes: hms_bytes,
        license: super::PUBLIC_DOMAIN.into(),
        obligations: String::new(),
    });

    // --- AQS: each county's highest 24-hour PM2.5 per day -------------------------------------
    let (geo, cb_source) = CountyGeo::download(ctx, &canon, "smoke/cb_2024_us_county_500k.zip")?;
    out.source(cb_source);
    let index_of: BTreeMap<&str, usize> = counties
        .iter()
        .enumerate()
        .map(|(i, c)| (c.fips.as_str(), i))
        .collect();
    let mut county_day_max: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    let mut site_county: BTreeMap<(i64, i64), Option<usize>> = BTreeMap::new();
    for y in FIRST_YEAR..=LAST_YEAR {
        let url = format!("{AQS_BASE}/daily_88101_{y}.zip");
        let f = ctx
            .http
            .get(&url, Some(&format!("smoke/daily_88101_{y}.zip")))?;
        out.source(super::source_from(
            &format!("EPA AQS daily PM2.5 (FRM/FEM, parameter 88101), {y}"),
            &f,
            match &f.last_modified {
                Some(lm) => format!("daily_88101_{y}.zip; Last-Modified {lm}"),
                None => format!("daily_88101_{y}.zip"),
            },
            super::PUBLIC_DOMAIN,
            "",
        ));
        let obs = {
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&f.bytes))?;
            let idx = (0..zip.len())
                .find(|&i| zip.by_index(i).is_ok_and(|e| e.name().ends_with(".csv")))
                .ok_or_else(|| data_err(format!("{url}: no CSV in the zip")))?;
            parse_aqs(zip.by_index(idx)?)?
        };
        out.rows_in += obs.len() as u64;
        for o in obs {
            if o.day < first_day || o.day > last_day {
                continue;
            }
            // Sites are placed by their coordinates (AQS still files Connecticut under the
            // retired counties), rounded to about 10 m for the cache key.
            let key = ((o.lat * 1e4).round() as i64, (o.lon * 1e4).round() as i64);
            let county = *site_county.entry(key).or_insert_with(|| {
                geo.locate(o.lat, o.lon)
                    .and_then(|f| index_of.get(f).copied())
            });
            let Some(c) = county else { continue };
            let e = county_day_max
                .entry((c, (o.day - first_day) as usize))
                .or_insert(f64::NEG_INFINITY);
            if o.value > *e {
                *e = o.value;
            }
        }
    }

    // --- Tallies per county and year ---------------------------------------------------------
    let year_of = |k: usize| (civil_from_days(first_day + k as i64).0 - FIRST_YEAR) as usize;
    let mut tallies: Vec<Vec<Year>> = vec![vec![Year::default(); n_years]; counties.len()];
    for (c, days) in covered_days.iter().enumerate() {
        for &k in days {
            let t = &mut tallies[c][year_of(k)];
            t.hms += 1;
            if let Some(v) = county_day_max.get(&(c, k)) {
                t.observed += 1;
                for (i, thr) in THRESHOLDS.iter().enumerate() {
                    if *v >= *thr {
                        t.over[i] += 1;
                    }
                }
            }
        }
    }
    let (h, rows) = read_table(&ctx.data, super::geography::COUNTIES)?;
    let (i_f, i_r) = (col(&h, "fips")?, col(&h, "nca_region")?);
    let region: BTreeMap<String, String> = rows
        .iter()
        .map(|r| (r[i_f].clone(), r[i_r].clone()))
        .collect();
    let own: Vec<Option<[f64; 2]>> = tallies.iter().map(|t| fraction(t)).collect();
    // Pooled rates by NCA5 region, from the counties with enough observations.
    let mut pooled: BTreeMap<&str, (u32, [u32; 2])> = BTreeMap::new();
    for (c, t) in tallies.iter().enumerate() {
        if own[c].is_none() {
            continue;
        }
        let r = region
            .get(&counties[c].fips)
            .map(String::as_str)
            .unwrap_or("");
        let e = pooled.entry(r).or_default();
        for y in t {
            e.0 += y.observed;
            e.1[0] += y.over[0];
            e.1[1] += y.over[1];
        }
    }
    let mut table = Table::new(
        &[
            "fips",
            "smoke_days_35",
            "smoke_days_55",
            "smoke_trend",
            "hms_smoke_days",
            "smoke_basis",
        ],
        1,
    );
    let mut covered_set = BTreeSet::new();
    let mut basis_count: BTreeMap<&str, usize> = BTreeMap::new();
    let mut values: BTreeMap<String, f64> = BTreeMap::new();
    let outside_hms = |c: &County| matches!(c.state_abbr.as_str(), "GU" | "AS" | "MP");
    for (c, county) in counties.iter().enumerate() {
        if outside_hms(county) {
            continue;
        }
        let t = &tallies[c];
        let hms_mean = t.iter().map(|y| y.hms as f64).sum::<f64>() / n_years as f64;
        let (fallback, basis) = match own[c] {
            Some(f) => (Some(f), "monitor"),
            None => {
                let r = region.get(&county.fips).map(String::as_str).unwrap_or("");
                let neighbour = counties
                    .iter()
                    .enumerate()
                    .filter(|(j, o)| {
                        own[*j].is_some() && region.get(&o.fips).map(String::as_str) == Some(r)
                    })
                    .map(|(j, o)| (haversine_km(county.lat, county.lon, o.lat, o.lon), j))
                    .filter(|(d, _)| *d <= IMPUTE_KM)
                    .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                match neighbour {
                    Some((_, j)) => (own[j], "imputed"),
                    None => match pooled.get(r) {
                        Some((obs, over)) if *obs > 0 => (
                            Some([over[0] as f64 / *obs as f64, over[1] as f64 / *obs as f64]),
                            "imputed",
                        ),
                        _ => (None, "hms_only"),
                    },
                }
            }
        };
        *basis_count.entry(basis).or_default() += 1;
        let row = match fallback {
            Some(f) => {
                // A county without its own rate uses the borrowed rate every year.
                let est = if basis == "monitor" {
                    yearly_estimates(t, f)
                } else {
                    yearly_estimates(
                        &t.iter()
                            .map(|y| Year {
                                hms: y.hms,
                                ..Default::default()
                            })
                            .collect::<Vec<_>>(),
                        f,
                    )
                };
                let mean = |k: usize| est.iter().map(|e| e[k]).sum::<f64>() / n_years as f64;
                let series: Vec<f64> = est.iter().map(|e| e[0]).collect();
                values.insert(county.fips.clone(), mean(0));
                vec![
                    county.fips.clone(),
                    sig(mean(0), 3),
                    sig(mean(1), 3),
                    sig(slope(&series), 3),
                    sig(hms_mean, 3),
                    basis.to_string(),
                ]
            }
            // No rate to borrow: with no smoke-covered day at all the counts are 0; otherwise
            // they stay unknown.
            None if hms_mean == 0.0 => vec![
                county.fips.clone(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                basis.to_string(),
            ],
            None => vec![
                county.fips.clone(),
                String::new(),
                String::new(),
                String::new(),
                sig(hms_mean, 3),
                basis.to_string(),
            ],
        };
        table.push(row);
        covered_set.insert(county.fips.clone());
    }

    // Sanity anchors: the brief's Philadelphia (almost none) and Missoula (many); June 2023
    // put New York County over the line at least once; Butte, CA among the smokiest.
    let v = |f: &str| values.get(f).copied().unwrap_or(-1.0);
    let rank_top_decile = |f: &str| {
        let x = v(f);
        let above = values.values().filter(|y| **y > x).count();
        above <= values.len() / 10
    };
    let ny_2023 = tallies[index_of["36061"]][(2023 - FIRST_YEAR) as usize].over[0];
    if !(v("42101") >= 0.0 && v("42101") < 1.5) {
        return Err(data_err(format!(
            "Philadelphia smoke_days_35 {:.2}, expected under 1.5",
            v("42101")
        )));
    }
    if v("30063") < 2.0 || !rank_top_decile("30063") {
        return Err(data_err(format!(
            "Missoula smoke_days_35 {:.2}, expected 2+ and top decile",
            v("30063")
        )));
    }
    if ny_2023 < 1 {
        return Err(data_err(
            "New York County shows no smoke day of concern in 2023 (June 2023 Canadian smoke)",
        ));
    }
    if !rank_top_decile("06007") {
        return Err(data_err(format!(
            "Butte County, CA ({:.2}) is not in the top decile",
            v("06007")
        )));
    }
    out.table(ctx, SMOKE, &mut table)?;
    out.missing = missing_groups(&counties, &covered_set, |_| {
        "Outside the NOAA HMS smoke analysis area (Guam, American Samoa, Northern Mariana Islands)"
            .into()
    });
    out.notes.push(format!(
        "Smoke-covered: the county's internal point inside a Medium or Heavy HMS polygon ({} daily files {FIRST_YEAR}-{LAST_YEAR}, {} missing{}). PM2.5: EPA AQS daily 24-hour averages (sample durations 24 HOUR and 24-HR BLK AVG, 'Excluded' exceptional-event rows left out), the county's highest monitor per day, sites placed by coordinates.",
        per_day.len(),
        missing_days.len(),
        if missing_days.is_empty() { String::new() } else { format!(": {}", missing_days.join(", ")) }
    ));
    out.notes.push(format!(
        "smoke_days_35 / smoke_days_55: mean days a year {FIRST_YEAR}-{LAST_YEAR} that are smoke-covered with 24-hour PM2.5 of at least 35.5 / 55.5 µg/m³, estimated each year as (observed smoke-covered days over the threshold / observed smoke-covered days) x smoke-covered days, so monitors that run every third or sixth day are not undercounted. smoke_trend: least-squares slope of the yearly smoke_days_35 estimate, days per year per year. hms_smoke_days: mean smoke-covered days a year. Basis: monitor {} (at least {MIN_OBSERVED} observed smoke-covered days), imputed {} (rate of the nearest such county within {IMPUTE_KM} km in the same NCA5 region, else the region's pooled rate, applied to the county's own smoke-covered days), hms_only {} (no rate to borrow: counts are 0 where HMS never showed smoke, blank otherwise).",
        basis_count.get("monitor").unwrap_or(&0),
        basis_count.get("imputed").unwrap_or(&0),
        basis_count.get("hms_only").unwrap_or(&0)
    ));
    out.notes.push("HMS outlines smoke in the whole air column; the PM2.5 test keeps only days when it reached the ground. Winter inversion pollution (Utah, the San Joaquin Valley) counts only on days HMS also shows smoke. Childs et al. 2022 (CC BY-SA) is not bundled.".into());
    out.definitions.insert("smoke_days_35".into(), format!("Days a year ({FIRST_YEAR}-{LAST_YEAR} mean) when NOAA HMS shows medium or heavy smoke over the county and the county's 24-hour PM2.5 reaches 35.5 µg/m³ (AQI unhealthy for sensitive groups)."));
    out.definitions.insert(
        "smoke_days_55".into(),
        format!("The same at 55.5 µg/m³ (AQI unhealthy), {FIRST_YEAR}-{LAST_YEAR} mean."),
    );
    out.attributions.push(Attribution {
        source: "NOAA HMS and EPA AQS".into(),
        text: "Smoke days from NOAA NESDIS Hazard Mapping System smoke analysis polygons and U.S. EPA Air Quality System daily PM2.5 data.".into(),
        license: "US Government works (public domain)".into(),
        url: HMS_PAGE.into(),
        version: Some(format!("{FIRST_YEAR}-{LAST_YEAR}")),
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_classes() {
        assert!(dense("Medium") && dense("Heavy") && dense("27.000") && dense("16"));
        assert!(!dense("Light") && !dense("5.000") && !dense(""));
    }

    #[test]
    fn points_inside_polygons() {
        let square = vec![vec![
            [-100.0, 40.0],
            [-99.0, 40.0],
            [-99.0, 41.0],
            [-100.0, 41.0],
            [-100.0, 40.0],
        ]];
        let b = bbox_of(&square);
        let pts = [(40.5, -99.5), (42.0, -99.5)];
        assert_eq!(covered(&pts, &[(square, b)]), vec![0]);
    }

    #[test]
    fn aqs_rows_filter_to_daily_standard_values() {
        let csv = "\"State Code\",\"County Code\",\"Latitude\",\"Longitude\",\"Sample Duration\",\"Date Local\",\"Event Type\",\"Arithmetic Mean\"\n\
\"01\",\"003\",30.49,-87.88,\"1 HOUR\",\"2023-06-07\",\"None\",60.0\n\
\"01\",\"003\",30.49,-87.88,\"24-HR BLK AVG\",\"2023-06-07\",\"Included\",48.2\n\
\"01\",\"003\",30.49,-87.88,\"24-HR BLK AVG\",\"2023-06-07\",\"Excluded\",12.0\n\
\"01\",\"003\",30.49,-87.88,\"24 HOUR\",\"2023-06-08\",\"None\",9.5\n";
        let o = parse_aqs(csv.as_bytes()).unwrap();
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].value, 48.2);
        assert_eq!(o[0].day, days_from_civil(2023, 6, 7));
    }

    #[test]
    fn yearly_estimates_scale_sparse_monitors() {
        // A filter sampler saw 2 of 12 smoke-covered days; 1 was over 35.5: 6 days estimated.
        let y = Year {
            hms: 12,
            observed: 2,
            over: [1, 0],
        };
        let e = yearly_estimates(&[y], [0.0, 0.0]);
        assert!((e[0][0] - 6.0).abs() < 1e-12 && e[0][1] == 0.0);
        // No observation that year: the fallback rate times the smoke-covered days.
        let e = yearly_estimates(
            &[Year {
                hms: 10,
                ..Default::default()
            }],
            [0.2, 0.05],
        );
        assert!((e[0][0] - 2.0).abs() < 1e-12 && (e[0][1] - 0.5).abs() < 1e-12);
        assert!((slope(&[1.0, 2.0, 3.0, 4.0]) - 1.0).abs() < 1e-12);
        assert_eq!(slope(&[5.0]), 0.0);
    }
}
