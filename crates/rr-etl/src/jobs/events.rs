//! Job 4 — event rates per county from three NOAA records (all public domain):
//!
//! - **HURDAT2** (NHC best tracks, Atlantic and eastern/central Pacific, 1950-2025): storms whose
//!   track passes within 50 nautical miles of the county's internal point at tropical-storm,
//!   hurricane or major-hurricane strength (wind interpolated along the 6-hourly track).
//! - **SPC severe reports** (1996-2025): tornadoes touching the county (state segments, so each
//!   tornado counts once per county), and days with hail of 1 in / 2 in or larger and days with
//!   severe thunderstorm wind reports (any, and 65 kt or more).
//! - **NOAA Storm Events** (1996-2025): county-episodes of winter storms, ice storms, extreme
//!   cold, heat, floods, flash floods, coastal floods, wildfires, high wind, drought and tropical
//!   cyclone impacts. An episode is NOAA's `EPISODE_ID` (one weather system); a zone-based record
//!   applies to every county in the zone (NWS zone-county correlation files). For each county
//!   and type: episodes per year, share with reported damage, share with injuries or deaths, and
//!   the median and 90th-percentile episode length where begin and end times are recorded.
//!
//! A county with no row for a type had no recorded event of that type in the period (a rate of
//! zero) — except where a source does not cover the place at all (listed in the manifest notes).

use super::{Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::Table;
use crate::ct::{Crosswalk, is_old_ct};
use crate::geo::{EARTH_RADIUS_KM, KM_PER_NMI};
use crate::http::zip_entry;
use crate::manifest::SourceRecord;
use crate::num::{sig4, weighted_quantile};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufReader, Read};

/// Event rates per county and type.
pub const EVENTS: &str = "core/events.csv";

const HURDAT_DIR: &str = "https://www.nhc.noaa.gov/data/hurdat/";
const SPC_PAGE: &str = "https://www.spc.noaa.gov/wcm/";
const SPC_DATA: &str = "https://www.spc.noaa.gov/wcm/data/";
const STORM_DIR: &str = "https://www.ncei.noaa.gov/pub/data/swdi/stormevents/csvfiles/";
const ZONE_PAGE: &str = "https://www.weather.gov/gis/ZoneCounty";
const ZONE_DIR: &str = "https://www.weather.gov/source/gis/Shapefiles/County/";

/// First and last year used for SPC and Storm Events.
pub const FIRST_YEAR: i32 = 1996;
/// Track records start (HURDAT2).
pub const HURDAT_FIRST_YEAR: i32 = 1950;
/// Search radius around the county's internal point for storm passages.
pub const PASSAGE_RADIUS_NMI: f64 = 50.0;

/// Storm Events types grouped into our event types.
const STORM_TYPES: &[(&str, &[&str])] = &[
    (
        "winter_storm",
        &["winter storm", "blizzard", "heavy snow", "lake-effect snow"],
    ),
    ("ice_storm", &["ice storm"]),
    (
        "extreme_cold",
        &["extreme cold/wind chill", "cold/wind chill", "extreme cold"],
    ),
    ("heat", &["heat", "excessive heat"]),
    ("flood", &["flood"]),
    ("flash_flood", &["flash flood"]),
    (
        "coastal_flood",
        &["coastal flood", "storm surge/tide", "lakeshore flood"],
    ),
    ("wildfire", &["wildfire"]),
    ("high_wind", &["high wind"]),
    ("drought", &["drought"]),
    (
        "tropical_cyclone_impact",
        &[
            "hurricane (typhoon)",
            "hurricane",
            "tropical storm",
            "tropical depression",
            "typhoon",
        ],
    ),
];

fn storm_type(event_type: &str) -> Option<usize> {
    let t = event_type.trim().to_ascii_lowercase();
    STORM_TYPES
        .iter()
        .position(|(_, names)| names.contains(&t.as_str()))
}

/// One weighted observation for a (county, type).
#[derive(Debug, Clone, Copy)]
struct Obs {
    weight: f64,
    hours: Option<f64>,
    damage: bool,
    injury: bool,
}

#[derive(Debug, Default)]
struct Acc {
    obs: BTreeMap<(String, String), Vec<Obs>>, // (fips, type) -> observations
    source: BTreeMap<String, &'static str>,    // type -> source name
}

impl Acc {
    fn push(&mut self, fips: &str, ty: &str, o: Obs, source: &'static str) {
        self.obs
            .entry((fips.to_string(), ty.to_string()))
            .or_default()
            .push(o);
        self.source.entry(ty.to_string()).or_insert(source);
    }
}

/// Parse a Storm Events damage string ("10.00K", "1.5M", "2B", "0.00K", "").
pub fn parse_damage(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let (num, mult) = match t.chars().last()? {
        'K' | 'k' => (&t[..t.len() - 1], 1e3),
        'M' | 'm' => (&t[..t.len() - 1], 1e6),
        'B' | 'b' => (&t[..t.len() - 1], 1e9),
        'H' | 'h' => (&t[..t.len() - 1], 1e2),
        _ => (t, 1.0),
    };
    if num.is_empty() {
        return Some(0.0);
    }
    num.parse::<f64>().ok().map(|v| v * mult)
}

/// Parse a HURDAT2 coordinate like `28.0N` or `94.8W`.
pub fn parse_coord(s: &str) -> Option<f64> {
    let t = s.trim();
    let (num, hemi) = t.split_at(t.len().checked_sub(1)?);
    let v: f64 = num.trim().parse().ok()?;
    match hemi {
        "N" | "E" => Some(v),
        "S" | "W" => Some(-v),
        _ => None,
    }
}

/// A HURDAT2 fix time (`20170825`, `1800`, UTC) as seconds since the Unix epoch.
pub fn hurdat_time(date: &str, hhmm: &str) -> Option<i64> {
    let d = date.trim();
    let t = hhmm.trim();
    if d.len() != 8 || t.is_empty() || t.len() > 4 {
        return None;
    }
    let y: i64 = d[0..4].parse().ok()?;
    let m: u32 = d[4..6].parse().ok()?;
    let day: u32 = d[6..8].parse().ok()?;
    let hm: i64 = t.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) || hm / 100 > 23 || hm % 100 > 59 {
        return None;
    }
    Some(crate::timefmt::days_from_civil(y, m, day) * 86_400 + (hm / 100) * 3600 + (hm % 100) * 60)
}

/// UTC offset in seconds of a Storm Events `CZ_TIMEZONE` value. Recent files write the zone and
/// its offset (`CST-6`, `EST-5`, `AKST-9`, `HST-10`, `GST10`); older ones only the abbreviation
/// (`CST`, `EDT`). Storm Events records begin and end times in that zone's local time, so
/// `UTC = local - offset`. `None` when the value is unknown (callers fall back to the county's
/// longitude).
pub fn tz_offset_s(tz: &str) -> Option<i64> {
    let t = tz.trim().to_ascii_uppercase();
    if t.is_empty() {
        return None;
    }
    // An explicit signed offset after the letters wins.
    let letters: String = t.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let rest = &t[letters.len()..];
    if !rest.is_empty()
        && let Ok(h) = rest.parse::<i64>()
        && (-12..=14).contains(&h)
    {
        return Some(h * 3600);
    }
    let h = match letters.as_str() {
        "EST" => -5,
        "EDT" => -4,
        "CST" | "CSC" | "SCT" => -6,
        "CDT" => -5,
        "MST" => -7,
        "MDT" => -6,
        "PST" => -8,
        "PDT" => -7,
        "AKST" | "AKS" => -9,
        "AKDT" => -8,
        "HST" => -10,
        "AST" => -4,
        "SST" => -11,
        "GST" | "CHST" => 10,
        "UTC" | "GMT" => 0,
        _ => return None,
    };
    Some(h * 3600)
}

/// First year of Storm Events and HURDAT2 records kept for attributing outage events (EAGLE-I
/// starts in 2014).
pub const ATTRIBUTION_FIRST_YEAR: i32 = 2014;

/// Storm Events types kept for attributing outage events to a cause, with a short code.
pub const ATTRIBUTION_TYPES: &[(&str, &str)] = &[
    ("thunderstorm wind", "tstm_wind"),
    ("tornado", "tornado"),
    ("hail", "hail"),
    ("lightning", "lightning"),
    ("high wind", "high_wind"),
    ("strong wind", "strong_wind"),
    ("ice storm", "ice_storm"),
    ("sleet", "sleet"),
    ("freezing fog", "freezing_fog"),
    ("winter storm", "winter_storm"),
    ("blizzard", "blizzard"),
    ("heavy snow", "heavy_snow"),
    ("lake-effect snow", "lake_snow"),
    ("winter weather", "winter_weather"),
    ("extreme cold/wind chill", "extreme_cold"),
    ("cold/wind chill", "cold"),
    ("extreme cold", "extreme_cold"),
    ("heat", "heat"),
    ("excessive heat", "excessive_heat"),
    ("wildfire", "wildfire"),
    ("dense smoke", "dense_smoke"),
    ("flood", "flood"),
    ("flash flood", "flash_flood"),
    ("heavy rain", "heavy_rain"),
    ("debris flow", "debris_flow"),
    ("coastal flood", "coastal_flood"),
    ("storm surge/tide", "storm_surge"),
    ("lakeshore flood", "lakeshore_flood"),
    ("tropical storm", "tropical_storm"),
    ("tropical depression", "tropical_depression"),
    ("hurricane", "hurricane"),
    ("hurricane (typhoon)", "hurricane"),
    ("typhoon", "hurricane"),
    ("dust storm", "dust_storm"),
];

/// Short attribution code for a Storm Events `EVENT_TYPE`, if it is one we keep.
pub fn attribution_type(event_type: &str) -> Option<&'static str> {
    let t = event_type.trim().to_ascii_lowercase();
    ATTRIBUTION_TYPES
        .iter()
        .find(|(n, _)| *n == t)
        .map(|(_, c)| *c)
}

/// A HURDAT2 storm track.
#[derive(Debug, Clone)]
pub struct Track {
    /// Storm id, e.g. `AL092017`.
    pub id: String,
    /// Storm name from the header line (`HARVEY`; `UNNAMED` for storms without one).
    pub name: String,
    /// Year of genesis.
    pub year: i32,
    /// Fixes: (lat, lon, max wind kt).
    pub fixes: Vec<(f64, f64, f64)>,
    /// Time of each fix in seconds since the Unix epoch (HURDAT2 times are UTC), parallel to
    /// `fixes`. Kept so outage events can be matched to the storm by date (until 2026-09-26 the
    /// times were read and thrown away).
    pub times: Vec<i64>,
}

/// Parse a HURDAT2 file.
pub fn parse_hurdat(text: &str) -> Result<Vec<Track>> {
    let mut out = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() < 3 || parts[0].len() != 8 {
            continue;
        }
        let id = parts[0].to_string();
        let year: i32 = id[4..8]
            .parse()
            .map_err(|_| data_err(format!("HURDAT2: bad storm id {id}")))?;
        let name = parts[1].to_string();
        let n: usize = parts[2]
            .parse()
            .map_err(|_| data_err(format!("HURDAT2: bad entry count for {id}")))?;
        let mut fixes = Vec::with_capacity(n);
        let mut times = Vec::with_capacity(n);
        for _ in 0..n {
            let l = lines
                .next()
                .ok_or_else(|| data_err(format!("HURDAT2: {id} ends early")))?;
            let f: Vec<&str> = l.split(',').map(|s| s.trim()).collect();
            if f.len() < 7 {
                continue;
            }
            let (Some(lat), Some(lon)) = (parse_coord(f[4]), parse_coord(f[5])) else {
                continue;
            };
            let Some(t) = hurdat_time(f[0], f[1]) else {
                return Err(data_err(format!(
                    "HURDAT2: bad date or time {} {} in {id}",
                    f[0], f[1]
                )));
            };
            let wind: f64 = f[6].parse().unwrap_or(-999.0);
            fixes.push((lat, lon, wind));
            times.push(t);
        }
        out.push(Track {
            id,
            name,
            year,
            fixes,
            times,
        });
    }
    Ok(out)
}

/// Closest approach of a track to a point: (distance km, interpolated wind kt at that point).
fn closest_along(lat: f64, lon: f64, a: (f64, f64, f64), b: (f64, f64, f64)) -> (f64, f64) {
    let kx = lat.to_radians().cos() * EARTH_RADIUS_KM.to_radians();
    let ky = EARTH_RADIUS_KM.to_radians();
    let unwrap = |x: f64| {
        let mut d = x - lon;
        while d > 180.0 {
            d -= 360.0;
        }
        while d < -180.0 {
            d += 360.0;
        }
        d
    };
    let (ax, ay) = (unwrap(a.1) * kx, (a.0 - lat) * ky);
    let (bx, by) = (unwrap(b.1) * kx, (b.0 - lat) * ky);
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (-(ax * dx + ay * dy) / len2).clamp(0.0, 1.0)
    };
    let (px, py) = (ax + t * dx, ay + t * dy);
    ((px * px + py * py).sqrt(), a.2 + t * (b.2 - a.2))
}

fn latest_by_suffix(names: &[String], prefix: &str) -> Option<String> {
    // Names like hurdat2-1851-2025-091226.txt (MMDDYY) or ...-02272026.txt (MMDDYYYY).
    let key = |n: &str| -> Option<(u32, u32)> {
        let rest = n.strip_prefix(prefix)?.strip_suffix(".txt")?;
        let (end, date) = rest.split_once('-')?;
        let end: u32 = end.parse().ok()?;
        let date = date.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let (m, d, y) = match date.len() {
            6 => (&date[0..2], &date[2..4], format!("20{}", &date[4..6])),
            8 => (&date[0..2], &date[2..4], date[4..8].to_string()),
            _ => return None,
        };
        let stamp = y.parse::<u32>().ok()? * 10_000
            + m.parse::<u32>().ok()? * 100
            + d.parse::<u32>().ok()?;
        Some((end, stamp))
    };
    names
        .iter()
        .filter_map(|n| key(n).map(|k| (k, n.clone())))
        .max()
        .map(|(_, n)| n)
}

fn links(html: &str, pattern_start: &str, pattern_end: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("href=\"") {
        rest = &rest[i + 6..];
        let Some(j) = rest.find('"') else { break };
        let href = &rest[..j];
        let name = href.rsplit('/').next().unwrap_or(href);
        if name.starts_with(pattern_start) && name.ends_with(pattern_end) {
            out.push(name.to_string());
        }
        rest = &rest[j..];
    }
    out.sort();
    out.dedup();
    out
}

fn fips_str(state: u32, county: u32) -> String {
    format!("{state:02}{county:03}")
}

/// Target counties and weights for a county code as reported by a source (handles Connecticut's
/// old counties and a few renamed codes).
fn targets(code: &str, canon: &BTreeSet<String>, cw: &Crosswalk) -> Vec<(String, f64)> {
    if canon.contains(code) {
        return vec![(code.to_string(), 1.0)];
    }
    if is_old_ct(code) {
        return cw
            .overlaps
            .iter()
            .filter(|o| o.old == code)
            .map(|o| (o.region.clone(), o.share_of_region))
            .collect();
    }
    crate::ct::successors(code)
        .map(|v| v.iter().map(|n| (n.to_string(), 1.0)).collect())
        .unwrap_or_default()
}

/// Words that qualify a forecast-zone name without naming a county ("Northern Fairfield").
const ZONE_QUALIFIERS: &[&str] = &[
    "northern",
    "southern",
    "eastern",
    "western",
    "central",
    "northeast",
    "northwest",
    "southeast",
    "southwest",
    "north",
    "south",
    "east",
    "west",
    "coastal",
    "inland",
    "upper",
    "lower",
    "interior",
    "county",
    "parish",
    "borough",
    "mountains",
    "lowlands",
    "highlands",
    "higher",
    "elevations",
    "the",
    "and",
    "of",
];

/// Normalise a county or zone name for the retired-zone fallback.
pub fn norm_name(s: &str) -> String {
    s.to_ascii_lowercase()
        .replace("saint ", "st ")
        .replace("st. ", "st ")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .filter(|w| !ZONE_QUALIFIERS.contains(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let cw = Crosswalk::load(&ctx.data)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let last_year: i32 = crate::timefmt::today_utc()[..4]
        .parse::<i32>()
        .unwrap_or(2026)
        - 1;
    let years_recent = (last_year - FIRST_YEAR + 1) as f64;
    let years_hurdat = (last_year - HURDAT_FIRST_YEAR + 1) as f64;
    let abbr_by_fips: BTreeMap<u32, &str> = crate::jobs::geography::STATE_FACTS
        .iter()
        .filter_map(|(f, a, _, _)| Some((f.parse::<u32>().ok()?, *a)))
        .collect();
    let mut acc = Acc::default();
    // Retired forecast zones: fall back to the county whose name the zone carries, when exactly one
    // county in the state has that name.
    let mut by_name: HashMap<(String, String), Vec<String>> = HashMap::new();
    for c in &counties {
        by_name
            .entry((c.state_abbr.clone(), norm_name(&c.name)))
            .or_default()
            .push(c.fips.clone());
    }

    // --- HURDAT2 ---------------------------------------------------------------------------
    let dir = ctx.http.get(HURDAT_DIR, None)?;
    let names = links(&dir.text(), "hurdat2-", ".txt");
    let atl = latest_by_suffix(&names, "hurdat2-1851-")
        .ok_or_else(|| data_err("no Atlantic HURDAT2 file found"))?;
    let pac = latest_by_suffix(&names, "hurdat2-nepac-1949-")
        .ok_or_else(|| data_err("no Pacific HURDAT2 file found"))?;
    let mut tracks = Vec::new();
    for (name, basin) in [(&atl, "Atlantic"), (&pac, "eastern and central Pacific")] {
        let f = ctx.http.get(
            &format!("{HURDAT_DIR}{name}"),
            Some(&format!("events/{name}")),
        )?;
        out.source(super::source_from(
            &format!("NOAA NHC HURDAT2 best tracks, {basin}"),
            &f,
            name.clone(),
            super::PUBLIC_DOMAIN,
            "",
        ));
        let t = parse_hurdat(&f.text())?;
        out.rows_in += t.len() as u64;
        tracks.extend(
            t.into_iter()
                .filter(|t| t.year >= HURDAT_FIRST_YEAR && t.year <= last_year),
        );
    }
    // Fixes since 2014 with their times, for attributing outage events (job `outage_model`).
    let mut track_rows: Vec<Vec<String>> = Vec::new();
    for t in tracks.iter().filter(|t| t.year >= ATTRIBUTION_FIRST_YEAR) {
        for (f, time) in t.fixes.iter().zip(&t.times) {
            track_rows.push(vec![
                t.id.clone(),
                t.name.clone(),
                time.to_string(),
                format!("{:.1}", f.0),
                format!("{:.1}", f.1),
                format!("{:.0}", f.2),
            ]);
        }
    }
    track_rows.sort();
    crate::intermediate::write(
        &ctx.data,
        crate::intermediate::HURDAT_TRACKS,
        &["storm_id", "name", "time", "lat", "lon", "wind_kt"],
        &track_rows,
    )?;
    let radius = PASSAGE_RADIUS_NMI * KM_PER_NMI;
    let classes: [(&str, f64); 3] = [
        ("tropical_storm_passage", 34.0),
        ("hurricane_passage", 64.0),
        ("major_hurricane_passage", 96.0),
    ];
    for c in &counties {
        let mut counts = [0u32; 3];
        for t in &tracks {
            let mut best = [false; 3];
            for w in t.fixes.windows(2) {
                let (a, b) = (w[0], w[1]);
                // Cheap reject: both fixes more than 3 degrees of latitude away.
                if (a.0 - c.lat).abs() > 3.0
                    && (b.0 - c.lat).abs() > 3.0
                    && (a.0 - c.lat).signum() == (b.0 - c.lat).signum()
                {
                    continue;
                }
                let (d, wind) = closest_along(c.lat, c.lon, a, b);
                if d <= radius {
                    for (k, (_, thr)) in classes.iter().enumerate() {
                        if wind >= *thr {
                            best[k] = true;
                        }
                    }
                }
            }
            for k in 0..3 {
                counts[k] += best[k] as u32;
            }
        }
        for (k, (ty, _)) in classes.iter().enumerate() {
            for _ in 0..counts[k] {
                acc.push(
                    &c.fips,
                    ty,
                    Obs {
                        weight: 1.0,
                        hours: None,
                        damage: false,
                        injury: false,
                    },
                    "hurdat2",
                );
            }
        }
    }

    // --- SPC severe reports ------------------------------------------------------------------
    let page = ctx.http.get(SPC_PAGE, None)?.text();
    let all_torn = links(&page, "1950-", "_all_tornadoes.csv")
        .pop()
        .ok_or_else(|| data_err("SPC: no all_tornadoes file linked"))?;
    let hail_zip = links(&page, "1955-", "_hail.csv.zip")
        .pop()
        .ok_or_else(|| data_err("SPC: no hail zip linked"))?;
    let wind_zip = links(&page, "1955-", "_wind.csv.zip")
        .pop()
        .ok_or_else(|| data_err("SPC: no wind zip linked"))?;
    let spc_version = |name: &str, f: &crate::http::Fetched| match &f.last_modified {
        Some(lm) => format!("{name}; Last-Modified {lm}"),
        None => name.to_string(),
    };
    let torn = ctx.http.get(
        &format!("{SPC_DATA}{all_torn}"),
        Some(&format!("events/{all_torn}")),
    )?;
    out.source(super::source_from(
        "NOAA SPC severe weather database: tornadoes (all segments)",
        &torn,
        spc_version(&all_torn, &torn),
        super::PUBLIC_DOMAIN,
        "",
    ));
    {
        let (h, rows) = crate::csvout::parse_delimited(&torn.text(), b',')?;
        let ix = |n: &str| crate::csvout::col(&h, n);
        let (iy, iom, isn, istf, imag, iinj, ifat, iloss) = (
            ix("yr")?,
            ix("om")?,
            ix("sn")?,
            ix("stf")?,
            ix("mag")?,
            ix("inj")?,
            ix("fat")?,
            ix("loss")?,
        );
        let ifs = [ix("f1")?, ix("f2")?, ix("f3")?, ix("f4")?];
        let mut seen: BTreeSet<(String, String, String)> = BTreeSet::new();
        for r in &rows {
            let y: i32 = r[iy].parse().unwrap_or(0);
            if y < FIRST_YEAR || y > last_year || r[isn] != "1" {
                continue;
            }
            out.rows_in += 1;
            let st: u32 = r[istf].parse().unwrap_or(0);
            let mag: i32 = r[imag].parse().unwrap_or(-9);
            let damage = r[iloss].parse::<f64>().unwrap_or(0.0) > 0.0;
            let injury = r[iinj].parse::<f64>().unwrap_or(0.0) > 0.0
                || r[ifat].parse::<f64>().unwrap_or(0.0) > 0.0;
            for i in ifs {
                let cty: u32 = r[i].parse().unwrap_or(0);
                if cty == 0 {
                    continue;
                }
                for (target, w) in targets(&fips_str(st, cty), &canon, &cw) {
                    if !seen.insert((target.clone(), r[iy].clone(), r[iom].clone())) {
                        continue;
                    }
                    let o = Obs {
                        weight: w,
                        hours: None,
                        damage,
                        injury,
                    };
                    acc.push(&target, "tornado", o, "spc");
                    if mag >= 2 {
                        acc.push(&target, "tornado_ef2plus", o, "spc");
                    }
                }
            }
        }
    }
    for (zipname, kind) in [(&hail_zip, "hail"), (&wind_zip, "wind")] {
        let z = ctx.http.get(
            &format!("{SPC_DATA}{zipname}"),
            Some(&format!("events/{zipname}")),
        )?;
        out.source(super::source_from(
            &format!("NOAA SPC severe weather database: {kind} reports"),
            &z,
            spc_version(zipname, &z),
            super::PUBLIC_DOMAIN,
            "",
        ));
        let text = String::from_utf8_lossy(&zip_entry(&z.bytes, ".csv")?).to_string();
        let (h, rows) = crate::csvout::parse_delimited(&text, b',')?;
        let ix = |n: &str| crate::csvout::col(&h, n);
        let (iy, idate, istf, imag, if1, iinj, ifat, iloss) = (
            ix("yr")?,
            ix("date")?,
            ix("stf")?,
            ix("mag")?,
            ix("f1")?,
            ix("inj")?,
            ix("fat")?,
            ix("loss")?,
        );
        // Days with a report, per county and class: (fips, class) -> date -> (damage, injury, weight).
        type DayFlags = BTreeMap<String, (bool, bool, f64)>;
        let mut days: BTreeMap<(String, &str), DayFlags> = BTreeMap::new();
        for r in &rows {
            let y: i32 = r[iy].parse().unwrap_or(0);
            if y < FIRST_YEAR || y > last_year {
                continue;
            }
            out.rows_in += 1;
            let st: u32 = r[istf].parse().unwrap_or(0);
            let cty: u32 = r[if1].parse().unwrap_or(0);
            if cty == 0 {
                continue;
            }
            let mag: f64 = r[imag].parse().unwrap_or(0.0);
            let damage = r[iloss].parse::<f64>().unwrap_or(0.0) > 0.0;
            let injury = r[iinj].parse::<f64>().unwrap_or(0.0) > 0.0
                || r[ifat].parse::<f64>().unwrap_or(0.0) > 0.0;
            let classes: Vec<&str> = match kind {
                "hail" if mag >= 2.0 => vec!["hail_1in_day", "hail_2in_day"],
                "hail" if mag >= 1.0 => vec!["hail_1in_day"],
                "wind" if mag >= 65.0 => vec!["severe_wind_day", "severe_wind_65kt_day"],
                "wind" => vec!["severe_wind_day"],
                _ => vec![],
            };
            for (target, w) in targets(&fips_str(st, cty), &canon, &cw) {
                for cl in &classes {
                    let e = days
                        .entry((target.clone(), cl))
                        .or_default()
                        .entry(r[idate].clone())
                        .or_insert((false, false, 0.0));
                    e.0 |= damage;
                    e.1 |= injury;
                    e.2 = e.2.max(w);
                }
            }
        }
        for ((fips, cl), dates) in days {
            // The weight is 1 except for Connecticut reports placed in a retired county, which
            // count for each planning region by the share of the region's land in that county.
            for (_, (damage, injury, weight)) in dates {
                acc.push(
                    &fips,
                    cl,
                    Obs {
                        weight,
                        hours: None,
                        damage,
                        injury,
                    },
                    "spc",
                );
            }
        }
    }

    // --- NWS zone-county correlation -----------------------------------------------------------
    let zpage = ctx.http.get(ZONE_PAGE, None)?.text();
    let mut zone_files = links(&zpage, "bp", ".dbx");
    zone_files.sort_by_key(|n| {
        // bpDDmmYY.dbx -> YYMMDD for newest-first ordering
        let s = n.trim_start_matches("bp").trim_end_matches(".dbx");
        let months = [
            "ja", "fe", "mr", "ap", "my", "jn", "jl", "au", "se", "oc", "no", "de",
        ];
        let (d, m, y) = (
            &s[0..2.min(s.len())],
            s.get(2..4).unwrap_or(""),
            s.get(4..6).unwrap_or(""),
        );
        let mi = months.iter().position(|x| *x == m).unwrap_or(0);
        std::cmp::Reverse(format!("{y}{mi:02}{d}"))
    });
    let mut zones: HashMap<(String, String), Vec<String>> = HashMap::new();
    for zf in &zone_files {
        let f = ctx
            .http
            .get(&format!("{ZONE_DIR}{zf}"), Some(&format!("events/{zf}")))?;
        out.source(super::source_from(
            "NWS zone-county correlation file",
            &f,
            zf.clone(),
            super::PUBLIC_DOMAIN,
            "",
        ));
        let mut this: HashMap<(String, String), Vec<String>> = HashMap::new();
        for line in f.text().lines() {
            let p: Vec<&str> = line.split('|').collect();
            if p.len() < 7 || p[6].len() != 5 {
                continue;
            }
            this.entry((p[0].to_string(), p[1].to_string()))
                .or_default()
                .push(p[6].to_string());
        }
        for (k, v) in this {
            zones.entry(k).or_insert(v); // newest file wins
        }
    }

    // --- Storm Events -----------------------------------------------------------------------
    let listing = ctx.http.get(STORM_DIR, None)?.text();
    let detail_files = links(&listing, "StormEvents_details-ftp_v1.0_d", ".csv.gz");
    let mut per_year: BTreeMap<i32, String> = BTreeMap::new();
    for n in &detail_files {
        let Some(y) = n.get(30..34).and_then(|s| s.parse::<i32>().ok()) else {
            continue;
        };
        if (FIRST_YEAR..=last_year).contains(&y) {
            let e = per_year.entry(y).or_default();
            if n > e {
                *e = n.clone();
            }
        }
    }
    if per_year.len() as i32 != last_year - FIRST_YEAR + 1 {
        return Err(data_err(format!(
            "Storm Events: expected files for {FIRST_YEAR}-{last_year}, found {:?}",
            per_year.keys()
        )));
    }
    let mut unmapped_zone_records = 0u64;
    let mut zone_name_fallbacks = 0u64;
    let mut unmapped_zones: BTreeSet<String> = BTreeSet::new();
    let mut used_records = 0u64;
    let mut tz_fallbacks = 0u64;
    let lon_by_fips: HashMap<String, f64> =
        counties.iter().map(|c| (c.fips.clone(), c.lon)).collect();
    // County-episodes for outage attribution: (county, episode, type) -> (weight, begin, end, damage, injury).
    type AttrKey = (String, String, &'static str);
    let mut attr_eps: BTreeMap<AttrKey, (f64, i64, i64, bool, bool)> = BTreeMap::new();
    for (y, name) in &per_year {
        let url = format!("{STORM_DIR}{name}");
        eprintln!("  Storm Events {y}");
        // county-episode accumulation for this file: (target, type, episode) -> (source unit -> weight), begin, end, damage, injury
        type Key = (String, usize, String);
        #[derive(Default)]
        struct Ep {
            units: BTreeMap<String, f64>,
            begin: Option<i64>,
            end: Option<i64>,
            damage: bool,
            injury: bool,
        }
        let (eps, streamed) = ctx.http.stream(
            &url,
            Some(&format!("events/{name}")),
            |reader: &mut dyn Read| {
                let gz =
                    flate2::read::MultiGzDecoder::new(BufReader::with_capacity(1 << 20, reader));
                let mut rdr = csv::ReaderBuilder::new()
                    .has_headers(true)
                    .from_reader(BufReader::with_capacity(1 << 20, gz));
                let h: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
                let ix = |n: &str| {
                    h.iter()
                        .position(|x| x == n)
                        .ok_or_else(|| data_err(format!("Storm Events: no column {n}")))
                };
                let (i_ep, i_ty, i_czt, i_czf, i_stf, i_czn) = (
                    ix("EPISODE_ID")?,
                    ix("EVENT_TYPE")?,
                    ix("CZ_TYPE")?,
                    ix("CZ_FIPS")?,
                    ix("STATE_FIPS")?,
                    ix("CZ_NAME")?,
                );
                let (i_bym, i_bd, i_bt, i_eym, i_ed, i_et) = (
                    ix("BEGIN_YEARMONTH")?,
                    ix("BEGIN_DAY")?,
                    ix("BEGIN_TIME")?,
                    ix("END_YEARMONTH")?,
                    ix("END_DAY")?,
                    ix("END_TIME")?,
                );
                let (i_dp, i_dc, i_ind, i_ini, i_dd, i_di) = (
                    ix("DAMAGE_PROPERTY")?,
                    ix("DAMAGE_CROPS")?,
                    ix("INJURIES_DIRECT")?,
                    ix("INJURIES_INDIRECT")?,
                    ix("DEATHS_DIRECT")?,
                    ix("DEATHS_INDIRECT")?,
                );
                let i_tz = ix("CZ_TIMEZONE")?;
                let mut eps: HashMap<Key, Ep> = HashMap::new();
                let mut attr: HashMap<AttrKey, (f64, i64, i64, bool, bool)> = HashMap::new();
                let mut unmapped = 0u64;
                let mut by_zone_name = 0u64;
                let mut tz_fallback = 0u64;
                let mut unmapped_names: BTreeSet<String> = BTreeSet::new();
                let mut used = 0u64;
                let mut rec = csv::StringRecord::new();
                while rdr.read_record(&mut rec)? {
                    let group = storm_type(&rec[i_ty]);
                    let attr_code = if *y >= ATTRIBUTION_FIRST_YEAR {
                        attribution_type(&rec[i_ty])
                    } else {
                        None
                    };
                    if group.is_none() && attr_code.is_none() {
                        continue;
                    }
                    let stf: u32 = rec[i_stf].trim().parse().unwrap_or(0);
                    let czf: u32 = rec[i_czf].trim().parse().unwrap_or(0);
                    let codes: Vec<String> = match rec[i_czt].trim() {
                        "C" => vec![fips_str(stf, czf)],
                        "Z" => {
                            let Some(abbr) = abbr_by_fips.get(&stf) else {
                                continue;
                            };
                            match zones.get(&(abbr.to_string(), format!("{czf:03}"))) {
                                Some(v) => v.clone(),
                                None => {
                                    match by_name.get(&(abbr.to_string(), norm_name(&rec[i_czn]))) {
                                        Some(v) if v.len() == 1 => {
                                            by_zone_name += 1;
                                            v.clone()
                                        }
                                        _ => {
                                            unmapped += 1;
                                            unmapped_names.insert(format!("{abbr}Z{czf:03}"));
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        _ => continue,
                    };
                    // Local time of the record's zone -> UTC (the zone's offset from
                    // CZ_TIMEZONE; the county's longitude when the value is unknown).
                    let offset = match tz_offset_s(&rec[i_tz]) {
                        Some(o) => o,
                        None => {
                            tz_fallback += 1;
                            codes
                                .first()
                                .and_then(|c| lon_by_fips.get(c))
                                .map(|lon| (lon / 15.0).round() as i64 * 3600)
                                .unwrap_or(-6 * 3600)
                        }
                    };
                    let time = |ym: &str, d: &str, hm: &str| -> Option<i64> {
                        let ym: i64 = ym.trim().parse().ok()?;
                        let d: u32 = d.trim().parse().ok()?;
                        let hm: i64 = hm.trim().parse().ok()?;
                        Some(
                            crate::timefmt::days_from_civil(ym / 100, (ym % 100) as u32, d)
                                * 86_400
                                + (hm / 100) * 3600
                                + (hm % 100) * 60
                                - offset,
                        )
                    };
                    let b = time(&rec[i_bym], &rec[i_bd], &rec[i_bt]);
                    let e = time(&rec[i_eym], &rec[i_ed], &rec[i_et]);
                    let dmg = parse_damage(&rec[i_dp]).unwrap_or(0.0)
                        + parse_damage(&rec[i_dc]).unwrap_or(0.0)
                        > 0.0;
                    let inj = [i_ind, i_ini, i_dd, i_di]
                        .iter()
                        .any(|i| rec[*i].trim().parse::<f64>().unwrap_or(0.0) > 0.0);
                    if group.is_some() {
                        used += 1;
                    }
                    let episode = rec[i_ep].trim().to_string();
                    for code in &codes {
                        for (target, w) in targets(code, &canon, &cw) {
                            if let (Some(code_a), Some(b), Some(e)) = (attr_code, b, e) {
                                let a = attr
                                    .entry((target.clone(), episode.clone(), code_a))
                                    .or_insert((0.0, b, e, false, false));
                                a.0 = a.0.max(w);
                                a.1 = a.1.min(b);
                                a.2 = a.2.max(e);
                                a.3 |= dmg;
                                a.4 |= inj;
                            }
                            let Some(ty) = group else { continue };
                            let ep = eps.entry((target, ty, episode.clone())).or_default();
                            let u = ep.units.entry(code.clone()).or_insert(0.0);
                            *u = u.max(w);
                            if let Some(b) = b {
                                ep.begin = Some(ep.begin.map_or(b, |x| x.min(b)));
                            }
                            if let Some(e) = e {
                                ep.end = Some(ep.end.map_or(e, |x| x.max(e)));
                            }
                            ep.damage |= dmg;
                            ep.injury |= inj;
                        }
                    }
                }
                Ok((
                    eps,
                    attr,
                    unmapped,
                    unmapped_names,
                    used,
                    by_zone_name,
                    tz_fallback,
                ))
            },
        )?;
        let (eps, attr, unmapped, names, used, by_name_n, tz_fb) = eps;
        unmapped_zone_records += unmapped;
        zone_name_fallbacks += by_name_n;
        tz_fallbacks += tz_fb;
        unmapped_zones.extend(names);
        used_records += used;
        for (k, v) in attr {
            let a = attr_eps.entry(k).or_insert(v);
            a.0 = a.0.max(v.0);
            a.1 = a.1.min(v.1);
            a.2 = a.2.max(v.2);
            a.3 |= v.3;
            a.4 |= v.4;
        }
        out.source(SourceRecord {
            name: format!("NOAA NCEI Storm Events details {y}"),
            url: streamed.final_url.clone(),
            version: name.clone(),
            retrieved: streamed.retrieved.clone(),
            sha256: streamed.sha256.clone(),
            bytes: streamed.bytes,
            license: super::PUBLIC_DOMAIN.into(),
            obligations: String::new(),
        });
        for ((target, ty, _), ep) in eps {
            let weight = ep.units.values().sum::<f64>().min(1.0);
            let hours = match (ep.begin, ep.end) {
                (Some(b), Some(e)) if e >= b => Some((e - b) as f64 / 3600.0),
                _ => None,
            };
            acc.push(
                &target,
                STORM_TYPES[ty].0,
                Obs {
                    weight,
                    hours,
                    damage: ep.damage,
                    injury: ep.injury,
                },
                "storm_events",
            );
        }
    }
    out.rows_in += used_records;
    let attr_rows: Vec<Vec<String>> = attr_eps
        .iter()
        .map(|((fips, episode, code), (w, b, e, dmg, inj))| {
            vec![
                fips.clone(),
                episode.clone(),
                code.to_string(),
                b.to_string(),
                e.to_string(),
                sig4(*w),
                (*dmg as u8).to_string(),
                (*inj as u8).to_string(),
            ]
        })
        .collect();
    crate::intermediate::write(
        &ctx.data,
        crate::intermediate::STORM_EPISODES,
        &[
            "fips", "episode", "type", "begin", "end", "weight", "damage", "injury",
        ],
        &attr_rows,
    )?;
    eprintln!(
        "  kept {} county-episodes since {ATTRIBUTION_FIRST_YEAR} for outage attribution",
        attr_rows.len()
    );

    // --- Output -------------------------------------------------------------------------------
    let mut table = Table::new(
        &[
            "fips",
            "event_type",
            "rate_per_year",
            "share_damaging",
            "median_days",
            "p90_days",
            "events",
            "share_injury",
            "source",
            "years",
        ],
        2,
    );
    let mut covered = BTreeSet::new();
    for ((fips, ty), obs) in &acc.obs {
        let n: f64 = obs.iter().map(|o| o.weight).sum();
        if n <= 0.0 {
            continue;
        }
        let source = acc.source.get(ty).copied().unwrap_or("");
        let years = if source == "hurdat2" {
            years_hurdat
        } else {
            years_recent
        };
        let period = if source == "hurdat2" {
            format!("{HURDAT_FIRST_YEAR}-{last_year}")
        } else {
            format!("{FIRST_YEAR}-{last_year}")
        };
        let (share_dmg, share_inj) = if source == "hurdat2" {
            (String::new(), String::new())
        } else {
            let d = obs
                .iter()
                .filter(|o| o.damage)
                .map(|o| o.weight)
                .sum::<f64>()
                / n;
            let i = obs
                .iter()
                .filter(|o| o.injury)
                .map(|o| o.weight)
                .sum::<f64>()
                / n;
            (sig4(d), sig4(i))
        };
        let mut lengths: Vec<(f64, f64)> = obs
            .iter()
            .filter_map(|o| o.hours.map(|h| (h / 24.0, o.weight)))
            .collect();
        lengths.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (med, p90) = if lengths.is_empty() {
            (String::new(), String::new())
        } else {
            (
                weighted_quantile(&lengths, 0.5)
                    .map(sig4)
                    .unwrap_or_default(),
                weighted_quantile(&lengths, 0.9)
                    .map(sig4)
                    .unwrap_or_default(),
            )
        };
        table.push(vec![
            fips.clone(),
            ty.clone(),
            sig4(n / years),
            share_dmg,
            med,
            p90,
            sig4(n),
            share_inj,
            source.to_string(),
            period,
        ]);
        covered.insert(fips.clone());
    }
    out.table(ctx, EVENTS, &mut table)?;

    out.missing = missing_groups(&counties, &covered, |_| {
        "No recorded events of any tracked type".to_string()
    });
    let hurricane_gap: Vec<String> = counties
        .iter()
        .filter(|c| matches!(c.state_abbr.as_str(), "GU" | "MP" | "AS"))
        .map(|c| c.fips.clone())
        .collect();
    out.notes.push(format!(
        "Periods: HURDAT2 {HURDAT_FIRST_YEAR}-{last_year} ({years_hurdat:.0} years); SPC and Storm Events {FIRST_YEAR}-{last_year} ({years_recent:.0} years; Storm Events began recording all event types in 1996). A missing row means no recorded event of that type (rate 0), except that HURDAT2 does not cover the western or southern Pacific: typhoon passages for {} are not counted (use NRI hurricane fields or tropical_cyclone_impact).",
        hurricane_gap.join(", ")
    ));
    out.notes.push(format!(
        "Storm passages: a storm counts when its best track passes within {PASSAGE_RADIUS_NMI} nautical miles of the county's internal point with interpolated maximum sustained wind of at least 34 kt (tropical_storm_passage), 64 kt (hurricane_passage) or 96 kt (major_hurricane_passage), whatever its tropical or post-tropical status."
    ));
    out.notes.push("SPC: tornadoes are counted once per county per tornado using the per-state segment records (sn = 1) and their county list (f1-f4); tornado_ef2plus uses the rating (mag >= 2). Hail and wind are counted as days with at least one report in the county (hail 1 in and 2 in or larger by size; all severe-wind reports, and 65 kt or more). share_damaging is the share with a reported property loss above zero; loss units changed in 2016 but only 'above zero' is used.".into());
    out.notes.push("Storm Events: one observation per county per NOAA episode and type. Zone records apply to every county the NWS zone-county correlation file lists for that zone (newest file first, older files as fallback). Episode length runs from the earliest begin to the latest end of the episode's records in the county, with every time converted to UTC from the record's CZ_TIMEZONE (until 2026-09-26 the zone was ignored, so an episode whose records sat in two time zones was off by an hour). share_damaging counts reported property or crop damage above zero (many records leave damage blank, so it is a lower bound); share_injury counts direct or indirect injuries or deaths.".into());
    out.notes.push(format!(
        "Retired forecast zones: {zone_name_fallbacks} zone records whose code is not in the correlation files were placed in the one county of that state whose name the zone carries (after dropping words like Northern or Coastal); {unmapped_zone_records} records ({} distinct zones) could not be placed and were skipped ({:.1}% of the records used).",
        unmapped_zones.len(),
        100.0 * unmapped_zone_records as f64 / (used_records + unmapped_zone_records).max(1) as f64
    ));
    out.notes.push(format!(
        "Time zones: {tz_fallbacks} Storm Events records had no recognisable CZ_TIMEZONE; their times were converted with the county's longitude (one hour per 15 degrees). The {ATTRIBUTION_FIRST_YEAR}+ county-episodes (UTC begin and end, every outage-relevant type) and the HURDAT2 fixes with their UTC times are handed to the outage_model job; they are not pack files."
    ));
    out.notes.push("Connecticut: sources that report the retired counties are apportioned to planning regions by land-area share (each region's weight is the share of its land inside the reporting county).".into());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_strings() {
        assert_eq!(parse_damage("10.00K"), Some(10_000.0));
        assert_eq!(parse_damage("1.5M"), Some(1_500_000.0));
        assert_eq!(parse_damage("0.00K"), Some(0.0));
        assert_eq!(parse_damage(""), None);
        assert_eq!(parse_damage("K"), Some(0.0));
    }

    #[test]
    fn hurdat_parsing_and_passage() {
        let text = "AL092017,             HARVEY,     3,\n\
20170825, 1800,  , HU, 27.6N,  96.8W, 115,  941,\n\
20170826, 0300, L, HU, 28.0N,  97.1W, 115,  937,\n\
20170826, 1200,  , TS, 28.8N,  97.6W,  60,  965,\n";
        let t = parse_hurdat(text).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].year, 2017);
        assert_eq!(t[0].fixes[0], (27.6, -96.8, 115.0));
        // A point on the track at landfall: distance ~0, wind 115.
        let (d, w) = closest_along(28.0, -97.1, t[0].fixes[0], t[0].fixes[1]);
        assert!(d < 0.5 && (w - 115.0).abs() < 1e-9);
        // Halfway along the weakening segment the interpolated wind is ~87.5 kt.
        let (d, w) = closest_along(28.4, -97.35, t[0].fixes[1], t[0].fixes[2]);
        assert!(d < 1.0 && (w - 87.5).abs() < 1.0, "{d} {w}");
    }

    #[test]
    fn latest_file_by_date_suffix() {
        let names = vec![
            "hurdat2-1851-2024-040425.txt".to_string(),
            "hurdat2-1851-2025-02272026.txt".to_string(),
            "hurdat2-1851-2025-091226.txt".to_string(),
        ];
        assert_eq!(
            latest_by_suffix(&names, "hurdat2-1851-").unwrap(),
            "hurdat2-1851-2025-091226.txt"
        );
    }

    #[test]
    fn zone_names_reduce_to_county_names() {
        assert_eq!(norm_name("NORTHERN FAIRFIELD"), "fairfield");
        assert_eq!(norm_name("Harris"), "harris");
        assert_eq!(norm_name("COASTAL ST. JOHNS"), "st johns");
        assert_eq!(norm_name("St. Johns"), "st johns");
    }

    #[test]
    fn hurdat_keeps_names_and_fix_times() {
        let text = "AL092024,             HELENE,     2,\n\
20240927, 0310, L, HU, 30.0N,  83.9W, 120,  938,\n\
20240927, 1200,  , TS, 34.4N,  84.0W,  50,  970,\n";
        let t = parse_hurdat(text).unwrap();
        assert_eq!(t[0].name, "HELENE");
        // 2024-09-27 03:10 UTC and 12:00 UTC.
        assert_eq!(t[0].times[0], 1_727_406_600);
        assert_eq!(t[0].times[1] - t[0].times[0], 8 * 3600 + 50 * 60);
        assert!(parse_hurdat("AL012020, X, 1,\n2020060, 1200, , TS, 20.0N, 90.0W, 40,\n").is_err());
    }

    #[test]
    fn storm_events_time_zones() {
        assert_eq!(tz_offset_s("CST-6"), Some(-6 * 3600));
        assert_eq!(tz_offset_s("EST-5"), Some(-5 * 3600));
        assert_eq!(tz_offset_s("AKST-9"), Some(-9 * 3600));
        assert_eq!(tz_offset_s("HST-10"), Some(-10 * 3600));
        assert_eq!(tz_offset_s("GST10"), Some(10 * 3600));
        assert_eq!(tz_offset_s("PST"), Some(-8 * 3600));
        assert_eq!(tz_offset_s("cdt"), Some(-5 * 3600));
        assert_eq!(tz_offset_s("UNK"), None);
        assert_eq!(tz_offset_s(""), None);
        // A derecho report at 12:40 CST in Linn County, Iowa is 18:40 UTC.
        let local = crate::timefmt::days_from_civil(2020, 8, 10) * 86_400 + 12 * 3600 + 40 * 60;
        let utc = local - tz_offset_s("CST-6").unwrap();
        assert_eq!(crate::timefmt::format_unix(utc), "2020-08-10T18:40:00Z");
    }

    #[test]
    fn attribution_types_cover_outage_causes() {
        assert_eq!(attribution_type("Thunderstorm Wind"), Some("tstm_wind"));
        assert_eq!(attribution_type("Ice Storm"), Some("ice_storm"));
        assert_eq!(attribution_type("Hurricane (Typhoon)"), Some("hurricane"));
        assert_eq!(attribution_type("Rip Current"), None);
    }

    #[test]
    fn storm_type_groups() {
        assert_eq!(
            STORM_TYPES[storm_type("Blizzard").unwrap()].0,
            "winter_storm"
        );
        assert_eq!(STORM_TYPES[storm_type("Excessive Heat").unwrap()].0, "heat");
        assert!(storm_type("Thunderstorm Wind").is_none());
    }
}
