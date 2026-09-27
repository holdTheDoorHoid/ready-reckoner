//! Job `outage_model` — the regional power-outage model built from the EAGLE-I events that the
//! `outages` job repaired and recorded (model review M-01, M-02, M-10, M-18; data audit §11).
//!
//! 1. **Cause attribution.** Each county outage event is matched by date and place to what caused
//!    it: a tropical cyclone (a HURDAT2 track point of tropical-storm strength within
//!    [`TRACK_RADIUS_KM`] of the county, from 36 hours before to 24 hours after the outage began),
//!    otherwise a NOAA Storm Events episode in the county or its forecast zone overlapping the
//!    six hours before to two hours after the start (fixed priority, [`PRIORITY`]), otherwise a
//!    DOE OE-417 grid disturbance in the same state, otherwise "cause not recorded". Winter,
//!    ice and cold events that coincide with an OE-417 load-shed or energy-emergency report in
//!    the state become `cold_grid` (the February 2021 Texas blackout is the model case).
//! 2. **Regional pooled tails with credibility weights.** For each county and each length
//!    *d* (1, 3, 7, 14, 30 days) the rate of customer outages lasting at least *d* days per
//!    customer-year from its own record, λ_c(d), is blended with the rate over the county and
//!    its neighbours within [`POOL_RADIUS_KM`] (distance-weighted; 400 km for the short lengths,
//!    800 km for a week or more), λ_R(d): λ̂ = Z·λ_c + (1 − Z)·λ_R with Z = E / (E + k), where E is the number of qualifying
//!    events a county like this one would expect to see in its own record (the region's rate of
//!    events reaching *d* days × the county's years of data) and k = [`CREDIBILITY_K`]. A county's
//!    record gets full weight only where it can be expected to hold many such events, so one
//!    extreme storm (Linn County's 2020 derecho) no longer sets the county's long tail alone.
//!    The pool leaves out outages attributed to hurricanes, wildfires, floods, cold-driven grid
//!    emergencies and other grid failures, which the model carries in rows of their own (M-10's
//!    double count).
//! 3. **Pooled restoration curves** by region and cause: the share of the peak still out 1 to 30
//!    days after the peak, and the days until half and nine in ten are back, over major county
//!    events; Puerto Rico and the US Virgin Islands are regions of their own, and every region's
//!    time to nine in ten restored is compared with the mainland's (the restoration factor).
//! 4. **The worst-event stress table**: per county, the event in its region's record with the
//!    largest share of customers still out after a week, where it was recorded and its curve;
//!    hand-copied pre-2014 events (Hurricane Maria) stand in where EAGLE-I has no record.
//! 5. **A held-out test**: fit on 2014-2019, predict 2020-2025, for the county's own record, the
//!    region alone and the blend (plus an even/odd-year split that is not affected by the lower
//!    coverage of the early years).
//! 6. **Per-county event curves** for events of a day or more, in `core/outage_events.csv` and
//!    `core/outage_holdout.csv` (bundled into the core pack, DESIGN-DELTA-v3 §8, 2026-09-27;
//!    formerly the optional `outage_events` pack, issue #15). Loaded eagerly with the rest of the
//!    core pack; expert views and the validation page are still the only readers.

// The five outage lengths and ten causes are parallel fixed-size arrays; indexing them by
// position reads more plainly than zipped iterators.
#![allow(clippy::needless_range_loop)]

use super::{Ctx, JobOutput};
use crate::csvout::{Table, col, read_table};
use crate::ct::{Crosswalk, is_old_ct};
use crate::geo::haversine_km;
use crate::intermediate as im;
use crate::manifest::SourceRecord;
use crate::num::{sig, sig4};
use crate::{Result, data_err};
use rr_types::math::{exp, ln};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Credibility-weighted regional tails, one row per county.
pub const POOLED: &str = "core/outage_pooled.csv";
/// Share of each county's recorded customer outages by cause.
pub const CAUSES: &str = "core/outage_causes.csv";
/// Pooled restoration curves by region and cause.
pub const CURVES: &str = "core/outage_curves.csv";
/// The worst outage event in each county's region, with its curve.
pub const STRESS: &str = "core/outage_stress.csv";
/// Every county event of a day or more with its restoration curve. A core file, but read only by
/// the expert views and the validation page.
pub const OPT_EVENTS: &str = "core/outage_events.csv";
/// The held-out test results, for the validation page. A core file (see [`OPT_EVENTS`]).
pub const OPT_HOLDOUT: &str = "core/outage_holdout.csv";

/// Radius of a county's region for the stress table.
pub const REGION_RADIUS_KM: f64 = 250.0;
/// Radius of the distance weights for the pooled tails, per outage length (1, 3, 7, 14 and 30
/// days). Longer outages come from rarer, larger storms, so they are pooled more widely. Chosen
/// by comparing 150-1,200 km on the two held-out splits (see [`holdout_test`]).
pub const POOL_RADIUS_KM: [f64; 5] = [400.0, 400.0, 800.0, 800.0, 800.0];
/// The credibility constant k in Z = E / (E + k): the number of qualifying events at which a
/// county's own record and its region's weigh the same. Five is the weight the model already
/// gives five hurricane passages when it shrinks a county's major-hurricane share
/// (`rr-hazards` params, "weight of 5 passages").
pub const CREDIBILITY_K: f64 = 5.0;
/// Outage lengths (days) of the pooled tails; the same as `outages::GE_DAYS`.
pub const MARKS: [u32; 5] = [1, 3, 7, 14, 30];
/// A county event "reaches" a length when at least this share of the county's customers (and
/// at least [`QUALIFY_MIN`]) were out that long.
pub const QUALIFY_SHARE: f64 = 0.0025;
/// See [`QUALIFY_SHARE`].
pub const QUALIFY_MIN: f64 = 5.0;
/// A tropical cyclone causes an outage event when a track point of at least 34 kt passes within
/// this distance of the county's internal point in the attribution window.
pub const TRACK_RADIUS_KM: f64 = 300.0;
/// Attribution window for tropical cyclones: track points from this long before the outage began...
pub const TRACK_BEFORE_S: i64 = 36 * 3600;
/// ...to this long after.
pub const TRACK_AFTER_S: i64 = 24 * 3600;
/// Storm Events episodes count when they overlap from this long before the outage began...
pub const EPISODE_BEFORE_S: i64 = 6 * 3600;
/// ...to this long after.
pub const EPISODE_AFTER_S: i64 = 2 * 3600;
/// OE-417 reports count when they begin within this long of the outage start.
pub const OE417_WINDOW_S: i64 = 24 * 3600;
/// Major county events (for the restoration curves and the stress table): at least this share
/// of the county's customers out at the peak...
pub const MAJOR_SHARE: f64 = 0.10;
/// ...and at least this many customers.
pub const MAJOR_MIN: f64 = 2_000.0;
/// Bounds on the restoration factor (M-10).
pub const FACTOR_BOUNDS: (f64, f64) = (0.5, 5.0);

/// Causes an event can be attributed to.
pub const CLASSES: [&str; 10] = [
    "hurricane",
    "ice",
    "winter",
    "wind",
    "wildfire",
    "heat",
    "cold_grid",
    "flood",
    "grid",
    "unattributed",
];
/// Causes the local pool leaves out: the model carries them in rows of their own (hurricane
/// rows, the wildfire shutoff row, the riverine and coastal flooding rows, the cold-wave row and
/// the regional grid-failure row).
pub const OUTSIDE_POOL: [&str; 5] = ["hurricane", "wildfire", "flood", "cold_grid", "grid"];

/// Storm Events types in priority order, with the cause each stands for. When several episodes
/// overlap an outage, the first in this list wins (tropical types only count when no HURDAT2
/// track matched).
pub const PRIORITY: &[(&str, &str)] = &[
    ("hurricane", "hurricane"),
    ("tropical_storm", "hurricane"),
    ("tropical_depression", "hurricane"),
    ("storm_surge", "hurricane"),
    ("ice_storm", "ice"),
    ("sleet", "ice"),
    ("freezing_fog", "ice"),
    ("tornado", "wind"),
    ("tstm_wind", "wind"),
    ("winter_storm", "winter"),
    ("blizzard", "winter"),
    ("heavy_snow", "winter"),
    ("lake_snow", "winter"),
    ("winter_weather", "winter"),
    ("wildfire", "wildfire"),
    ("dense_smoke", "wildfire"),
    ("high_wind", "wind"),
    ("strong_wind", "wind"),
    ("flood", "flood"),
    ("flash_flood", "flood"),
    ("heavy_rain", "flood"),
    ("debris_flow", "flood"),
    ("coastal_flood", "flood"),
    ("lakeshore_flood", "flood"),
    ("lightning", "wind"),
    ("hail", "wind"),
    ("dust_storm", "wind"),
    ("excessive_heat", "heat"),
    ("heat", "heat"),
    ("extreme_cold", "cold_grid"),
    ("cold", "cold_grid"),
];

/// Hand-copied pre-2014 events and event names (crates/rr-etl/data/).
const HISTORIC: &str = include_str!("../../data/historic_outages.toml");

/// A county from `counties.csv` with its region.
#[derive(Debug, Clone)]
pub struct Place {
    /// FIPS.
    pub fips: String,
    /// Name.
    pub name: String,
    /// State abbreviation.
    pub state: String,
    /// Internal point.
    pub lat: f64,
    /// Internal point.
    pub lon: f64,
    /// Pooling region: the NCA5 region, except that Puerto Rico and the US Virgin Islands (each
    /// its own grid) are regions of their own.
    pub region: String,
}

/// Pooling region of a county.
pub fn region_key(state: &str, nca: &str) -> String {
    match state {
        "PR" => "puerto_rico".into(),
        "VI" => "virgin_islands".into(),
        _ => nca.to_string(),
    }
}

/// Distance weight of a neighbour: a biweight that falls from 1 at the county to 0 at
/// [`REGION_RADIUS_KM`].
pub fn kernel(km: f64) -> f64 {
    if km >= REGION_RADIUS_KM {
        0.0
    } else {
        let u = km / REGION_RADIUS_KM;
        (1.0 - u * u) * (1.0 - u * u)
    }
}

/// Credibility weight Z = E / (E + k).
pub fn credibility(expected_events: f64) -> f64 {
    if expected_events <= 0.0 {
        0.0
    } else {
        expected_events / (expected_events + CREDIBILITY_K)
    }
}

/// One outage event read back from the `outages` job.
#[derive(Debug, Clone)]
pub struct Ev {
    /// Index into the unit list.
    pub unit: usize,
    /// Start (UTC seconds).
    pub start: i64,
    /// End (UTC seconds).
    pub end: i64,
    /// Peak time.
    pub peak_t: i64,
    /// Customers out at the peak.
    pub peak: f64,
    /// Customer outages.
    pub arrivals: f64,
    /// Customer outages lasting at least 1, 3, 7, 14, 30 days.
    pub ge: [f64; 5],
    /// Share of the peak still out 1, 2, 3, 5, 7, 10, 14, 21, 30, 60, 90 days after the peak.
    pub curve: [f64; 11],
    /// Cause class index into [`CLASSES`].
    pub class: usize,
    /// Storm id, Storm Events type and episode, or OE-417 type.
    pub cause: String,
}

impl Ev {
    /// Curve value at a number of days after the peak (one of `outages::CURVE_DAYS`).
    pub fn at(&self, days: u32) -> f64 {
        super::outages::CURVE_DAYS
            .iter()
            .position(|d| *d == days)
            .map(|i| self.curve[i])
            .unwrap_or(0.0)
    }
}

/// A data unit (an EAGLE-I county code, or the Puerto Rico island series).
#[derive(Debug, Clone)]
pub struct Unit {
    /// Code as EAGLE-I reports it (`37021`, `09001`, `72000`).
    pub code: String,
    /// Customers.
    pub customers: f64,
    /// Months with data, by year.
    pub months: BTreeMap<u16, u32>,
}

impl Unit {
    /// Years of data within an inclusive span of years.
    pub fn years_in(&self, from: u16, to: u16) -> f64 {
        self.months
            .range(from..=to)
            .map(|(_, m)| f64::from(*m))
            .sum::<f64>()
            / 12.0
    }
    /// Qualifying threshold for this unit.
    pub fn qualify(&self) -> f64 {
        (QUALIFY_SHARE * self.customers).max(QUALIFY_MIN)
    }
}

/// A Storm Events county-episode (UTC).
#[derive(Debug, Clone)]
struct Episode {
    begin: i64,
    end: i64,
    code: String,
    id: String,
}

/// An hourly point on a tropical cyclone track.
#[derive(Debug, Clone, Copy)]
struct TrackPoint {
    t: i64,
    lat: f64,
    lon: f64,
    wind: f64,
    storm: usize,
}

/// A DOE OE-417 report.
#[derive(Debug, Clone)]
struct Oe417 {
    state: String,
    begin: i64,
    end: i64,
    kind: String,
}

/// Interpolate HURDAT2 fixes to hourly points.
fn hourly(fixes: &[(i64, f64, f64, f64)], storm: usize) -> Vec<TrackPoint> {
    let mut out = Vec::new();
    for w in fixes.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b.0 <= a.0 {
            continue;
        }
        let mut t = a.0;
        while t < b.0 {
            let f = (t - a.0) as f64 / (b.0 - a.0) as f64;
            let mut dlon = b.2 - a.2;
            if dlon > 180.0 {
                dlon -= 360.0;
            } else if dlon < -180.0 {
                dlon += 360.0;
            }
            out.push(TrackPoint {
                t,
                lat: a.1 + f * (b.1 - a.1),
                lon: a.2 + f * dlon,
                wind: a.3 + f * (b.3 - a.3),
                storm,
            });
            t += 3600;
        }
    }
    if let Some(l) = fixes.last() {
        out.push(TrackPoint {
            t: l.0,
            lat: l.1,
            lon: l.2,
            wind: l.3,
            storm,
        });
    }
    out
}

/// Index of a class name in [`CLASSES`].
pub fn class_index(name: &str) -> usize {
    CLASSES
        .iter()
        .position(|c| *c == name)
        .unwrap_or(CLASSES.len() - 1)
}

/// The cause of the highest-priority Storm Events type among `codes`, with that type.
pub fn storm_events_cause<'a>(
    codes: impl Iterator<Item = &'a str>,
) -> Option<(&'static str, &'static str)> {
    let mut best: Option<usize> = None;
    for c in codes {
        if let Some(i) = PRIORITY.iter().position(|(code, _)| *code == c) {
            best = Some(best.map_or(i, |b| b.min(i)));
        }
    }
    best.map(|i| (PRIORITY[i].1, PRIORITY[i].0))
}

/// OE-417 kinds that mean the grid itself could not keep up (load shed, energy emergency, fuel).
pub fn oe417_is_emergency(kind: &str) -> bool {
    let k = kind.to_ascii_lowercase();
    [
        "load shed",
        "energy emergency",
        "emergency alert",
        "fuel supply",
        "public appeal",
        "voltage reduction",
        "inadequa",
        "load management",
        "system operation",
    ]
    .iter()
    .any(|p| k.contains(p))
}

/// OE-417 kinds that are not the weather: operations, equipment, attacks.
pub fn oe417_is_grid(kind: &str) -> bool {
    let k = kind.to_ascii_lowercase();
    if k.contains("weather") || k.contains("natural disaster") || k.contains("wildfire") {
        return false;
    }
    [
        "system operation",
        "transmission",
        "distribution",
        "vandalism",
        "physical attack",
        "sabotage",
        "suspicious",
        "cyber",
        "fuel supply",
        "equipment",
        "islanding",
        "load shed",
        "energy emergency",
        "generation",
    ]
    .iter()
    .any(|p| k.contains(p))
}

/// Days until a curve falls to `share` of its peak, interpolating in log-time between the curve
/// points (day 0 = 1). `None` when the curve never gets that low within 90 days.
pub fn days_to(curve: &[f64; 11], share: f64) -> Option<f64> {
    let days = super::outages::CURVE_DAYS;
    let mut prev = (0.0f64, 1.0f64);
    for (i, d) in days.iter().enumerate() {
        let (d, s) = (f64::from(*d), curve[i]);
        if s <= share {
            if prev.1 <= share {
                return Some(prev.0);
            }
            // Linear in share between the two points, in log-days when the earlier one is > 0.
            let f = (prev.1 - share) / (prev.1 - s).max(1e-12);
            if f >= 1.0 {
                return Some(d);
            }
            return Some(if prev.0 > 0.0 {
                exp(ln(prev.0) + f * (ln(d) - ln(prev.0)))
            } else {
                f * d
            });
        }
        prev = (d, s);
    }
    None
}

/// A hand-copied historic event (pre-EAGLE-I), from `data/historic_outages.toml`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Historic {
    /// Stable id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Storm id, if a tropical cyclone.
    #[serde(default)]
    pub storm_id: String,
    /// Cause class.
    pub class: String,
    /// Date the outage began (local).
    pub date: String,
    /// State or territory abbreviations it stands in for.
    pub states: Vec<String>,
    /// Share of customers out at the peak.
    pub peak_share: f64,
    /// Days after the peak.
    pub days: Vec<f64>,
    /// Share of the peak still out on those days.
    pub share_out: Vec<f64>,
    /// Sources (citation ids or URLs).
    pub sources: Vec<String>,
    /// True when a figure could not be checked against a primary source.
    #[serde(default)]
    pub unverified: bool,
    /// Notes.
    #[serde(default)]
    pub note: String,
}

impl Historic {
    /// Share of the peak still out after `d` days, interpolated linearly (0 past the last point).
    pub fn share_at(&self, d: f64) -> f64 {
        let mut prev = (0.0, 1.0);
        for (x, y) in self.days.iter().zip(&self.share_out) {
            if d <= *x {
                let f = if *x > prev.0 {
                    (d - prev.0) / (x - prev.0)
                } else {
                    1.0
                };
                return prev.1 + f * (y - prev.1);
            }
            prev = (*x, *y);
        }
        0.0
    }
}

/// A named non-tropical event (for labels in the stress table).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NamedEvent {
    /// Display name.
    pub name: String,
    /// Cause classes it applies to.
    pub classes: Vec<String>,
    /// First date (UTC) an outage may begin.
    pub from: String,
    /// Last date (UTC) an outage may begin.
    pub to: String,
    /// States it applies to.
    pub states: Vec<String>,
    /// Source.
    pub source: String,
}

#[derive(Debug, serde::Deserialize)]
struct HistoricFile {
    #[serde(default)]
    event: Vec<Historic>,
    #[serde(default)]
    name: Vec<NamedEvent>,
}

/// Parse the hand-copied table.
pub fn historic() -> Result<(Vec<Historic>, Vec<NamedEvent>)> {
    let f: HistoricFile = toml::from_str(HISTORIC)
        .map_err(|e| data_err(format!("data/historic_outages.toml: {e}")))?;
    for h in &f.event {
        if h.days.len() != h.share_out.len() || h.days.is_empty() {
            return Err(data_err(format!(
                "historic event {}: days and share_out differ",
                h.id
            )));
        }
        if !CLASSES.contains(&h.class.as_str()) {
            return Err(data_err(format!(
                "historic event {}: unknown class {}",
                h.id, h.class
            )));
        }
    }
    Ok((f.event, f.name))
}

fn date_of(t: i64) -> String {
    crate::timefmt::format_unix(t)[..10].to_string()
}

fn title_case(s: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for c in s.chars() {
        if up {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
        up = !c.is_alphabetic();
    }
    out
}

/// Plain label for a cause class.
pub fn class_label(class: &str) -> &'static str {
    match class {
        "hurricane" => "Hurricane or tropical storm",
        "ice" => "Ice storm",
        "winter" => "Winter storm",
        "wind" => "Wind and thunderstorms",
        "wildfire" => "Wildfire or fire-weather shutoff",
        "heat" => "Heat",
        "cold_grid" => "Cold and a grid emergency",
        "flood" => "Flooding",
        "grid" => "Grid failure",
        _ => "Power cut, cause not recorded",
    }
}

/// Everything the job computes per county.
#[derive(Debug, Clone, Default)]
struct CountyStats {
    /// Years of data (weighted over its units).
    years: f64,
    /// Customer outages per customer-year in the pool (all lengths).
    rate: f64,
    /// Own exceedance rates (pool) per customer-year at each mark.
    own: [f64; 5],
    /// Qualifying events per year of data at each mark (pool).
    qual_rate: [f64; 5],
    /// Customer outages by class (share of all outages) and of outages lasting a day or more.
    by_class: [f64; 10],
    by_class_1d: [f64; 10],
    events_by_class: [f64; 10],
    has_data: bool,
}

/// Per-county sums over a span of years (for the full fit and the held-out test).
fn county_stats(
    places: &[Place],
    targets: &[Vec<(usize, f64)>],
    units: &[Unit],
    events: &[Ev],
    years: (u16, u16),
    pool_only: bool,
) -> Vec<CountyStats> {
    // Per unit first.
    let mut u_ge = vec![[0.0f64; 5]; units.len()];
    let mut u_q = vec![[0.0f64; 5]; units.len()];
    let mut u_arr = vec![0.0f64; units.len()];
    let mut u_class = vec![[0.0f64; 10]; units.len()];
    let mut u_class1 = vec![[0.0f64; 10]; units.len()];
    let mut u_nclass = vec![[0.0f64; 10]; units.len()];
    for e in events {
        let y = crate::timefmt::civil_from_days(e.start.div_euclid(86_400)).0 as u16;
        if y < years.0 || y > years.1 {
            continue;
        }
        u_class[e.unit][e.class] += e.arrivals;
        u_class1[e.unit][e.class] += e.ge[0];
        u_nclass[e.unit][e.class] += 1.0;
        if pool_only && OUTSIDE_POOL.contains(&CLASSES[e.class]) {
            continue;
        }
        u_arr[e.unit] += e.arrivals;
        let q = units[e.unit].qualify();
        for k in 0..5 {
            u_ge[e.unit][k] += e.ge[k];
            if e.ge[k] >= q {
                u_q[e.unit][k] += 1.0;
            }
        }
    }
    let mut out = vec![CountyStats::default(); places.len()];
    for (u, ts) in targets.iter().enumerate() {
        let yrs = units[u].years_in(years.0, years.1);
        if yrs <= 0.0 || units[u].customers <= 0.0 {
            continue;
        }
        let cust = units[u].customers;
        for (c, w) in ts {
            let s = &mut out[*c];
            s.has_data = true;
            s.years += w * yrs;
            s.rate += w * u_arr[u] / cust / yrs;
            for k in 0..5 {
                s.own[k] += w * u_ge[u][k] / cust / yrs;
                s.qual_rate[k] += w * u_q[u][k] / yrs;
            }
            let tot: f64 = u_class[u].iter().sum();
            let tot1: f64 = u_class1[u].iter().sum();
            for k in 0..10 {
                if tot > 0.0 {
                    s.by_class[k] += w * u_class[u][k] / tot;
                }
                if tot1 > 0.0 {
                    s.by_class_1d[k] += w * u_class1[u][k] / tot1;
                }
                s.events_by_class[k] += w * u_nclass[u][k];
            }
        }
    }
    // Weights of a county's units sum to 1 except for Connecticut regions, whose overlapping old
    // counties' land shares also sum to 1: every value above is already a weighted mean.
    out
}

/// How counties are pooled into regions (the constants are the defaults; the held-out test
/// compares alternatives).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolCfg {
    /// Radius of the distance weights at each outage length, km.
    pub radius_km: [f64; 5],
    /// Credibility constant.
    pub k: f64,
    /// Pool only within the county's NCA5 region (otherwise by distance alone; separate island
    /// grids are never pooled together).
    pub same_region: bool,
    /// The county's own record is part of its region's (the collective includes every member, as
    /// in Bühlmann credibility); otherwise it is left out.
    pub include_self: bool,
}

/// The pooling the pack uses.
pub const POOL: PoolCfg = PoolCfg {
    radius_km: POOL_RADIUS_KM,
    k: CREDIBILITY_K,
    same_region: false,
    include_self: true,
};

/// The pooling for this run: [`POOL`], unless a development run overrides it
/// (`RR_OUTAGE_POOL=radius_km,k,same_region,include_self`, for comparing choices on the held-out
/// test; the manifest notes record it).
fn pool_cfg() -> PoolCfg {
    let Ok(spec) = std::env::var("RR_OUTAGE_POOL") else {
        return POOL;
    };
    let p: Vec<&str> = spec.split(',').collect();
    let r = |i: usize| p.get(i).and_then(|x| x.parse().ok());
    let radius_km = match (r(0), r(4)) {
        (Some(a), Some(b)) => [a, a, b, b, b],
        (Some(a), None) => [a; 5],
        _ => POOL.radius_km,
    };
    PoolCfg {
        radius_km,
        k: p.get(1).and_then(|x| x.parse().ok()).unwrap_or(POOL.k),
        same_region: p
            .get(2)
            .map_or(POOL.same_region, |x| *x == "1" || *x == "true"),
        include_self: p
            .get(3)
            .map_or(POOL.include_self, |x| *x == "1" || *x == "true"),
    }
}

fn grid_group(p: &Place, same_region: bool) -> &str {
    match p.region.as_str() {
        "puerto_rico" | "virgin_islands" | "alaska" | "hawaii_pacific" => p.region.as_str(),
        r if same_region => r,
        _ => "mainland",
    }
}

/// Pooling partners of each county: (index, distance weight at each outage length), the
/// county itself first with weight 1 when `cfg.include_self`.
fn neighbours(places: &[Place], cfg: PoolCfg) -> Vec<Vec<(usize, [f64; 5])>> {
    let mut out = vec![Vec::new(); places.len()];
    let rmax = cfg.radius_km.iter().cloned().fold(0.0, f64::max);
    let dlat = rmax / 111.0 + 0.1;
    for i in 0..places.len() {
        if cfg.include_self {
            out[i].push((i, [1.0; 5]));
        }
        for j in 0..places.len() {
            if i == j
                || grid_group(&places[i], cfg.same_region)
                    != grid_group(&places[j], cfg.same_region)
            {
                continue;
            }
            let (a, b) = (&places[i], &places[j]);
            if (a.lat - b.lat).abs() > dlat {
                continue;
            }
            let km = haversine_km(a.lat, a.lon, b.lat, b.lon);
            if km < rmax {
                let mut w = [0.0; 5];
                for (k, r) in cfg.radius_km.iter().enumerate() {
                    if km < *r {
                        let u = km / r;
                        w[k] = (1.0 - u * u) * (1.0 - u * u);
                    }
                }
                out[i].push((j, w));
            }
        }
    }
    out
}

/// The blend for one county: (λ̂, Z, λ_R) at each mark, and the number of neighbours with data.
#[derive(Debug, Clone, Default)]
struct Blend {
    lam: [f64; 5],
    z: [f64; 5],
    region: [f64; 5],
    region_rate: f64,
    region_counties: usize,
    basis: &'static str,
}

fn blend(
    i: usize,
    stats: &[CountyStats],
    nb: &[Vec<(usize, [f64; 5])>],
    customers: &[f64],
    k_cred: f64,
) -> Option<Blend> {
    let own = &stats[i];
    let mut num = [0.0f64; 5];
    let mut den = [0.0f64; 5];
    let mut qnum = [0.0f64; 5];
    let mut qden = [0.0f64; 5];
    let mut rnum = 0.0;
    let mut rden = 0.0;
    let mut n = 0usize;
    for (j, w) in &nb[i] {
        let s = &stats[*j];
        if !s.has_data || s.years <= 0.0 {
            continue;
        }
        if *j != i && w[0] > 0.0 {
            n += 1;
        }
        for k in 0..5 {
            let wc = w[k] * customers[*j].max(1.0) * s.years;
            num[k] += wc * s.own[k];
            den[k] += wc;
            qnum[k] += w[k] * s.years * s.qual_rate[k];
            qden[k] += w[k] * s.years;
        }
        let wc0 = w[0] * customers[*j].max(1.0) * s.years;
        rnum += wc0 * s.rate;
        rden += wc0;
    }
    let mut b = Blend {
        region_counties: n,
        ..Default::default()
    };
    match (own.has_data, n > 0) {
        (false, false) => return None,
        (true, false) => {
            b.basis = "own_only";
            b.lam = own.own;
            b.region = own.own;
            b.z = [1.0; 5];
            b.region_rate = own.rate;
        }
        (false, true) => {
            b.basis = "region_only";
            for k in 0..5 {
                b.region[k] = if den[k] > 0.0 { num[k] / den[k] } else { 0.0 };
                b.lam[k] = b.region[k];
            }
            b.region_rate = rnum / rden;
        }
        (true, true) => {
            b.basis = "blend";
            for k in 0..5 {
                b.region[k] = num[k] / den[k];
                let expected = qnum[k] / qden[k] * own.years;
                b.z[k] = if expected <= 0.0 {
                    0.0
                } else {
                    expected / (expected + k_cred)
                };
                b.lam[k] = b.z[k] * own.own[k] + (1.0 - b.z[k]) * b.region[k];
            }
            b.region_rate = rnum / rden;
        }
    }
    Some(b)
}

/// The manifest definition of the pooled tail, for the pooling actually used.
fn pooled_tail_definition(cfg: PoolCfg) -> String {
    format!(
        "For each county and each length d of 1, 3, 7, 14 and 30 days: lam_ge_Nd = Z x own_ge_Nd + (1 - Z) x region_Nd. region_Nd is the same rate over {} every county within R(d) of it ({:.0} km for 1 and 3 days, {:.0} km for 7, 14 and 30 days; {}), each weighted by customers x years of data x (1 - (distance/R)^2)^2. Z = E / (E + {}), where E is the number of events reaching d days that a county with this county's years of data would record at the pooled rate (an event reaches d days when at least 0.25% of the county's customers, and at least 5, were out that long). Rates are customer outages lasting at least d days per customer per year, counting only outages the model does not carry in rows of their own: outages attributed to hurricanes, wildfires, floods, cold-driven grid emergencies and other grid failures are left out.",
        if cfg.include_self {
            "the county and"
        } else {
            "(leaving the county out)"
        },
        cfg.radius_km[0],
        cfg.radius_km[2],
        if cfg.same_region {
            "only counties in the same NCA5 region"
        } else {
            "any NCA5 region, but Puerto Rico, the US Virgin Islands, Alaska and Hawaii with the Pacific islands pool only among themselves"
        },
        cfg.k
    )
}

/// The manifest definition of cause attribution (the windows `run` applies).
fn attribution_definition() -> String {
    format!(
        "An outage event is attributed, in order, to a tropical cyclone when a HURDAT2 track point of at least 34 kt (fixes interpolated hourly) passes within {TRACK_RADIUS_KM:.0} km of the county's internal point from {} hours before the outage began to {} hours after it reached its peak (the peak taken at most 48 hours after the start); else to the highest-priority NOAA Storm Events episode in the county or its forecast zone overlapping the {} hours before the start to {} hours after the peak (tropical, ice, tornado and thunderstorm wind, winter, wildfire, high wind, flood, lightning and hail, heat, cold); winter, ice and cold events that coincide (same state, report span within {} hours of the start and the peak) with a DOE OE-417 load-shed, energy-emergency, public-appeal, voltage-reduction, fuel-supply or system-operations report are cold_grid; else to an OE-417 grid disturbance that is not weather in the same state, within the same window; else 'unattributed' (cause not recorded).",
        TRACK_BEFORE_S / 3600,
        TRACK_AFTER_S / 3600,
        EPISODE_BEFORE_S / 3600,
        EPISODE_AFTER_S / 3600,
        OE417_WINDOW_S / 3600
    )
}

/// The manifest note on regions: the stress table's, and the pooling actually used (so a
/// `RR_OUTAGE_POOL` override is on record).
fn regions_note(cfg: PoolCfg) -> String {
    let radius: Vec<String> = cfg.radius_km.iter().map(|r| format!("{r:.0}")).collect();
    format!(
        "Stress table: major events in the county's NCA5 region within {REGION_RADIUS_KM:.0} km, where Puerto Rico and the US Virgin Islands are regions of their own (separate grids). Pooled tails: {}, {}; radius {} km for outages of 1, 3, 7, 14 and 30 days; k = {}. Major county events (restoration curves and stress table): at least {:.0}% of the county's customers and at least {MAJOR_MIN:.0} customers out at the peak. Curves are shares of the peak still out N days after the peak, the running minimum of the repaired count (a lower bound on how long customers were out). Restoration factor = the region's median days until 9 in 10 are back / the mainland's for the same cause, bounded {}-{}.",
        if cfg.same_region {
            "within the county's NCA5 region"
        } else {
            "by distance across NCA5 regions (the island grids apart)"
        },
        if cfg.include_self {
            "the county included"
        } else {
            "the county left out"
        },
        radius.join("/"),
        cfg.k,
        MAJOR_SHARE * 100.0,
        FACTOR_BOUNDS.0,
        FACTOR_BOUNDS.1
    )
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let (h, rows) = read_table(&ctx.data, super::geography::COUNTIES)?;
    let (i_f, i_n, i_s, i_lat, i_lon, i_r) = (
        col(&h, "fips")?,
        col(&h, "name")?,
        col(&h, "state_abbr")?,
        col(&h, "lat")?,
        col(&h, "lon")?,
        col(&h, "nca_region")?,
    );
    let places: Vec<Place> = rows
        .iter()
        .map(|r| {
            Ok(Place {
                fips: r[i_f].clone(),
                name: r[i_n].clone(),
                state: r[i_s].clone(),
                lat: r[i_lat].parse().map_err(|_| data_err("bad lat"))?,
                lon: r[i_lon].parse().map_err(|_| data_err("bad lon"))?,
                region: region_key(&r[i_s], &r[i_r]),
            })
        })
        .collect::<Result<_>>()?;
    let index: HashMap<String, usize> = places
        .iter()
        .enumerate()
        .map(|(i, p)| (p.fips.clone(), i))
        .collect();
    let cw = Crosswalk::load(&ctx.data)?;
    // Per-county customer counts (the municipio's own count in Puerto Rico) for weighting.
    let (oh, orows) = read_table(&ctx.data, super::outages::OUTAGES)?;
    let (oi_f, oi_c) = (col(&oh, "fips")?, col(&oh, "customers")?);
    let mut customers = vec![0.0f64; places.len()];
    for r in &orows {
        if let (Some(i), Ok(c)) = (index.get(&r[oi_f]), r[oi_c].parse::<f64>()) {
            customers[*i] = c;
        }
    }

    // --- Units and events from the outages job ---------------------------------------------
    let (_, urows) = im::read(&ctx.data, im::OUTAGE_UNITS, "outages")?;
    let mut units = Vec::new();
    let mut unit_index: HashMap<String, usize> = HashMap::new();
    for r in &urows {
        let months: BTreeMap<u16, u32> = r[2]
            .split_whitespace()
            .filter_map(|p| {
                let (y, m) = p.split_once(':')?;
                Some((y.parse().ok()?, m.parse().ok()?))
            })
            .collect();
        unit_index.insert(r[0].clone(), units.len());
        units.push(Unit {
            code: r[0].clone(),
            customers: r[1].parse().unwrap_or(0.0),
            months,
        });
    }
    // Unit -> canonical counties (weights), as in the outages job.
    let mut targets: Vec<Vec<(usize, f64)>> = vec![Vec::new(); units.len()];
    for (u, unit) in units.iter().enumerate() {
        let code = unit.code.as_str();
        if code == format!("{:05}", super::outages::PR_ISLAND) {
            for (i, p) in places.iter().enumerate() {
                if p.state == "PR" {
                    targets[u].push((i, 1.0));
                }
            }
        } else if let Some(i) = index.get(code) {
            targets[u].push((*i, 1.0));
        } else if is_old_ct(code) {
            for o in cw.overlaps.iter().filter(|o| o.old == code) {
                if let Some(i) = index.get(&o.region) {
                    targets[u].push((*i, o.share_of_region));
                }
            }
        } else if let Some(new) = crate::ct::successors(code) {
            for n in new {
                if let Some(i) = index.get(*n) {
                    targets[u].push((*i, 1.0));
                }
            }
        }
    }
    let mut events: Vec<Ev> = Vec::new();
    im::for_each(&ctx.data, im::OUTAGE_EVENTS, "outages", |r| {
        let Some(&unit) = unit_index.get(&r[0]) else {
            return Ok(());
        };
        let g = |i: usize| r[i].parse::<f64>().unwrap_or(0.0);
        let mut ge = [0.0; 5];
        for (k, x) in ge.iter_mut().enumerate() {
            *x = g(7 + k);
        }
        let mut curve = [0.0; 11];
        for (k, x) in curve.iter_mut().enumerate() {
            *x = g(12 + k);
        }
        events.push(Ev {
            unit,
            start: r[1].parse().unwrap_or(0),
            end: r[2].parse().unwrap_or(0),
            peak_t: r[3].parse().unwrap_or(0),
            peak: g(4),
            arrivals: g(5),
            ge,
            curve,
            class: CLASSES.len() - 1,
            cause: String::new(),
        });
        Ok(())
    })?;
    out.rows_in = events.len() as u64;
    eprintln!("  {} events from {} units", events.len(), units.len());

    // --- Attribution inputs -----------------------------------------------------------------
    let mut episodes: HashMap<usize, Vec<Episode>> = HashMap::new();
    let mut n_episodes = 0usize;
    im::for_each(&ctx.data, im::STORM_EPISODES, "events", |r| {
        if let Some(&i) = index.get(&r[0]) {
            episodes.entry(i).or_default().push(Episode {
                begin: r[3].parse().unwrap_or(0),
                end: r[4].parse().unwrap_or(0),
                code: r[2].to_string(),
                id: r[1].to_string(),
            });
            n_episodes += 1;
        }
        Ok(())
    })?;
    for v in episodes.values_mut() {
        v.sort_by(|a, b| (a.begin, &a.code, &a.id).cmp(&(b.begin, &b.code, &b.id)));
    }
    // Puerto Rico is one series: its episodes are the union over the municipios.
    let pr_idx: Vec<usize> = places
        .iter()
        .enumerate()
        .filter(|(_, p)| p.state == "PR")
        .map(|(i, _)| i)
        .collect();
    let mut pr_eps: Vec<Episode> = Vec::new();
    let mut seen = BTreeSet::new();
    for i in &pr_idx {
        for e in episodes.get(i).into_iter().flatten() {
            if seen.insert((e.id.clone(), e.code.clone())) {
                pr_eps.push(e.clone());
            }
        }
    }
    pr_eps.sort_by(|a, b| (a.begin, &a.code, &a.id).cmp(&(b.begin, &b.code, &b.id)));

    let mut storm_names: Vec<(String, String, f64)> = Vec::new(); // id, name, lifetime max wind
    let mut fixes: BTreeMap<String, Vec<(i64, f64, f64, f64)>> = BTreeMap::new();
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    im::for_each(&ctx.data, im::HURDAT_TRACKS, "events", |r| {
        let id = r[0].to_string();
        names.entry(id.clone()).or_insert_with(|| r[1].to_string());
        fixes.entry(id).or_default().push((
            r[2].parse().unwrap_or(0),
            r[3].parse().unwrap_or(0.0),
            r[4].parse().unwrap_or(0.0),
            r[5].parse().unwrap_or(0.0),
        ));
        Ok(())
    })?;
    let mut points: Vec<TrackPoint> = Vec::new();
    for (id, f) in &mut fixes {
        f.sort_by_key(|x| x.0);
        let max_wind = f.iter().map(|x| x.3).fold(0.0, f64::max);
        let storm = storm_names.len();
        storm_names.push((
            id.clone(),
            names.get(id).cloned().unwrap_or_default(),
            max_wind,
        ));
        points.extend(hourly(f, storm).into_iter().filter(|p| p.wind >= 34.0));
    }
    points.sort_by_key(|p| (p.t, p.storm));

    let oe417: Vec<Oe417> = if im::exists(&ctx.data, im::OE417_EVENTS) {
        let (_, r) = im::read(&ctx.data, im::OE417_EVENTS, "oe417")?;
        let mut v: Vec<Oe417> = r
            .iter()
            .map(|x| {
                let begin = x[2].parse().unwrap_or(0);
                Oe417 {
                    state: x[1].clone(),
                    begin,
                    end: x[3].parse().unwrap_or(begin),
                    kind: x[4].clone(),
                }
            })
            .collect();
        v.sort_by_key(|x| x.begin);
        v
    } else {
        out.notes.push("OE-417 reports were not available to this run (the oe417 job did not run first), so no event was attributed to a grid disturbance and winter events were not split into cold_grid by load-shed reports.".into());
        Vec::new()
    };

    // --- Attribute every event ---------------------------------------------------------------
    let mut n_track = 0usize;
    let mut n_se = 0usize;
    let mut n_oe = 0usize;
    for e in &mut events {
        let tg = &targets[e.unit];
        let Some(&(ci, _)) = tg
            .iter()
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        else {
            continue;
        };
        let is_pr = units[e.unit].code == format!("{:05}", super::outages::PR_ISLAND);
        let (lat, lon) = if is_pr {
            (18.22, -66.43)
        } else {
            (places[ci].lat, places[ci].lon)
        };
        let state = places[ci].state.clone();
        // A long event is attributed by what brought it to its peak: rain ahead of a hurricane
        // can start an outage a day or two before the storm arrives (Helene in Buncombe County
        // began on 25 September 2024 and peaked on the 27th).
        let rise_end = e.peak_t.clamp(e.start, e.start + 48 * 3600);
        // 1. Tropical cyclones.
        let lo = points.partition_point(|p| p.t < e.start - TRACK_BEFORE_S);
        let mut best: Option<(f64, usize)> = None;
        for p in points[lo..]
            .iter()
            .take_while(|p| p.t <= rise_end + TRACK_AFTER_S)
        {
            if (p.lat - lat).abs() > 3.0 {
                continue;
            }
            let d = haversine_km(lat, lon, p.lat, p.lon);
            if d <= TRACK_RADIUS_KM && best.is_none_or(|b| d < b.0) {
                best = Some((d, p.storm));
            }
        }
        if let Some((_, s)) = best {
            e.class = class_index("hurricane");
            e.cause = storm_names[s].0.clone();
            n_track += 1;
            continue;
        }
        // 2. Storm Events episodes overlapping the start.
        let eps: &[Episode] = if is_pr {
            &pr_eps
        } else {
            episodes.get(&ci).map(|v| v.as_slice()).unwrap_or(&[])
        };
        let hi = eps.partition_point(|x| x.begin <= rise_end + EPISODE_AFTER_S);
        let from = eps.partition_point(|x| x.begin < e.start - 45 * 86_400);
        let overlapping: Vec<&Episode> = eps[from..hi]
            .iter()
            .filter(|x| x.end >= e.start - EPISODE_BEFORE_S)
            .collect();
        let oe_near = |pred: fn(&str) -> bool| {
            // Reports whose span overlaps the event's rise, give or take a day.
            let hi = oe417.partition_point(|x| x.begin <= rise_end + OE417_WINDOW_S);
            oe417[..hi]
                .iter()
                .rev()
                .take_while(|x| x.begin >= e.start - 30 * 86_400)
                .filter(|x| x.end >= e.start - OE417_WINDOW_S)
                .filter(|x| x.state == state && pred(&x.kind))
                .map(|x| x.kind.clone())
                .min()
        };
        if let Some((class, code)) = storm_events_cause(overlapping.iter().map(|x| x.code.as_str()))
        {
            let id = overlapping
                .iter()
                .find(|x| x.code == code)
                .map(|x| x.id.clone())
                .unwrap_or_default();
            let mut class = class;
            if matches!(class, "winter" | "ice" | "cold_grid") {
                if let Some(kind) = oe_near(oe417_is_emergency) {
                    class = "cold_grid";
                    e.cause = format!("{code}:{id}; oe417:{kind}");
                } else {
                    e.cause = format!("{code}:{id}");
                }
            } else {
                e.cause = format!("{code}:{id}");
            }
            e.class = class_index(class);
            n_se += 1;
            continue;
        }
        // 3. A grid disturbance reported to DOE.
        if let Some(kind) = oe_near(oe417_is_grid) {
            e.class = class_index("grid");
            e.cause = format!("oe417:{kind}");
            n_oe += 1;
        }
    }
    let long: Vec<&Ev> = events
        .iter()
        .filter(|e| e.at(7) > 0.0 || e.ge[2] > 0.0)
        .collect();
    let long_attr = long
        .iter()
        .filter(|e| CLASSES[e.class] != "unattributed")
        .count();
    out.notes.push(format!(
        "Attribution: {} county events read; {n_track} matched to a tropical cyclone track, {n_se} to Storm Events episodes, {n_oe} to an OE-417 grid disturbance, the rest are 'cause not recorded'. Of the {} events with customers still out a week after the peak, {long_attr} ({:.0}%) are attributed. {n_episodes} Storm Events county-episodes and {} tropical cyclones (2014 onward) were available.",
        events.len(),
        long.len(),
        100.0 * long_attr as f64 / long.len().max(1) as f64,
        storm_names.len()
    ));

    // --- County statistics, pooled tails -------------------------------------------------------
    let first_year = units
        .iter()
        .filter_map(|u| u.months.keys().next().copied())
        .min()
        .unwrap_or(2014);
    let last_year = units
        .iter()
        .filter_map(|u| u.months.keys().next_back().copied())
        .max()
        .unwrap_or(2025);
    let pool = county_stats(
        &places,
        &targets,
        &units,
        &events,
        (first_year, last_year),
        true,
    );
    let cfg = pool_cfg();
    let nb = neighbours(&places, cfg);
    let blends: Vec<Option<Blend>> = (0..places.len())
        .map(|i| blend(i, &pool, &nb, &customers, cfg.k))
        .collect();

    // Three significant figures for rates and two for weights: the estimates are far less
    // certain than that, and the core pack is fetched on every first visit.
    let mut pooled = Table::new(
        &[
            "fips",
            "basis",
            "rate",
            "lam_ge_1d",
            "lam_ge_3d",
            "lam_ge_7d",
            "lam_ge_14d",
            "lam_ge_30d",
            "z_1d",
            "z_3d",
            "z_7d",
            "z_14d",
            "z_30d",
            "region_counties",
        ],
        1,
    );
    for (i, b) in blends.iter().enumerate() {
        let Some(b) = b else { continue };
        let s = &pool[i];
        let mut row = vec![
            places[i].fips.clone(),
            b.basis.to_string(),
            sig(if s.has_data { s.rate } else { b.region_rate }, 3),
        ];
        row.extend(b.lam.iter().map(|x| sig(*x, 3)));
        row.extend(b.z.iter().map(|x| sig(*x, 2)));
        row.push(b.region_counties.to_string());
        pooled.push(row);
    }
    out.table(ctx, POOLED, &mut pooled)?;

    // --- Causes (all recorded outages, including those outside the pool) ---------------------
    let all = county_stats(
        &places,
        &targets,
        &units,
        &events,
        (first_year, last_year),
        false,
    );
    let mut header: Vec<String> = vec!["fips".into()];
    header.extend(CLASSES.iter().map(|c| format!("{c}_share")));
    header.extend(OUTSIDE_POOL.iter().map(|c| format!("{c}_share_ge_1d")));
    let mut causes = Table::with_header(header, 1);
    for (i, s) in all.iter().enumerate() {
        if !s.has_data {
            continue;
        }
        let mut row = vec![places[i].fips.clone()];
        // Two significant figures, and shares under half a percent written as 0.
        let share = |x: f64| {
            if x < 0.005 {
                "0".to_string()
            } else {
                sig(x, 2)
            }
        };
        row.extend(s.by_class.iter().map(|x| share(*x)));
        for c in OUTSIDE_POOL {
            row.push(share(s.by_class_1d[class_index(c)]));
        }
        causes.push(row);
    }
    out.table(ctx, CAUSES, &mut causes)?;

    // --- Pooled restoration curves by region and cause -------------------------------------
    let major = |e: &Ev| {
        let cust = units[e.unit].customers;
        e.peak >= MAJOR_MIN && e.peak >= MAJOR_SHARE * cust
    };
    let region_of_unit: Vec<Option<String>> = targets
        .iter()
        .map(|t| {
            t.iter()
                .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
                .map(|(i, _)| places[*i].region.clone())
        })
        .collect();
    let mut groups: BTreeMap<(String, String), Vec<&Ev>> = BTreeMap::new();
    for e in events.iter().filter(|e| major(e)) {
        let Some(region) = &region_of_unit[e.unit] else {
            continue;
        };
        let class = CLASSES[e.class].to_string();
        for key in [
            (region.clone(), class.clone()),
            (region.clone(), "all".to_string()),
        ] {
            groups.entry(key).or_default().push(e);
        }
        if region != "puerto_rico"
            && region != "virgin_islands"
            && region != "hawaii_pacific"
            && region != "alaska"
        {
            groups
                .entry(("mainland".into(), class))
                .or_default()
                .push(e);
            groups
                .entry(("mainland".into(), "all".into()))
                .or_default()
                .push(e);
        }
    }
    let summarise = |evs: &[&Ev]| {
        let w: f64 = evs.iter().map(|e| e.peak).sum();
        let mean = |d: u32| evs.iter().map(|e| e.peak * e.at(d)).sum::<f64>() / w;
        let wq = |f: &dyn Fn(&Ev) -> f64| {
            let mut v: Vec<(f64, f64)> = evs.iter().map(|e| (f(e), e.peak)).collect();
            v.sort_by(|a, b| a.0.total_cmp(&b.0));
            crate::num::weighted_quantile(&v, 0.5).unwrap_or(0.0)
        };
        let t50 = wq(&|e: &Ev| days_to(&e.curve, 0.5).unwrap_or(90.0));
        let t90 = wq(&|e: &Ev| days_to(&e.curve, 0.1).unwrap_or(90.0));
        (w, [mean(1), mean(3), mean(7), mean(14), mean(30)], t50, t90)
    };
    let mainland_t90: BTreeMap<String, f64> = groups
        .iter()
        .filter(|((r, _), _)| r == "mainland")
        .map(|((_, c), v)| (c.clone(), summarise(v).3))
        .collect();
    let mut curves = Table::new(
        &[
            "region",
            "class",
            "events",
            "customers",
            "s_1d",
            "s_3d",
            "s_7d",
            "s_14d",
            "s_30d",
            "t50_days",
            "t90_days",
            "factor",
        ],
        2,
    );
    for ((region, class), evs) in &groups {
        let (w, s, t50, t90) = summarise(evs);
        let factor = mainland_t90
            .get(class)
            .filter(|m| **m > 0.0)
            .map(|m| (t90 / m).clamp(FACTOR_BOUNDS.0, FACTOR_BOUNDS.1));
        let mut row = vec![
            region.clone(),
            class.clone(),
            evs.len().to_string(),
            sig4(w),
        ];
        row.extend(s.iter().map(|x| sig4(*x)));
        row.push(sig4(t50));
        row.push(sig4(t90));
        row.push(factor.map(sig4).unwrap_or_default());
        curves.push(row);
    }
    // Hand-copied island-grid scenario (Maria) as its own row, so the model can use it where
    // EAGLE-I has no record of a storm that size.
    let (historic, named) = historic()?;
    for h in &historic {
        let region = if h.states.iter().any(|s| s == "PR") {
            "puerto_rico"
        } else if h.states.iter().any(|s| s == "VI") {
            "virgin_islands"
        } else {
            continue;
        };
        let at = |d: f64| h.share_at(d);
        let t = |share: f64| {
            let mut d = 0.0;
            while d < 400.0 && at(d) > share {
                d += 0.5;
            }
            d
        };
        let mut row = vec![
            region.to_string(),
            format!("historic:{}", h.id),
            "1".into(),
            String::new(),
        ];
        row.extend([1.0, 3.0, 7.0, 14.0, 30.0].iter().map(|d| sig4(at(*d))));
        row.push(sig4(t(0.5)));
        row.push(sig4(t(0.1)));
        let factor = mainland_t90
            .get("hurricane")
            .filter(|m| **m > 0.0)
            .map(|m| (t(0.1) / m).clamp(FACTOR_BOUNDS.0, FACTOR_BOUNDS.1));
        row.push(factor.map(sig4).unwrap_or_default());
        curves.push(row);
    }
    out.table(ctx, CURVES, &mut curves)?;

    // --- Stress table --------------------------------------------------------------------------
    // Candidate events: major county events, with the county (index) they were recorded in.
    let mut cands: Vec<(usize, &Ev)> = Vec::new();
    for e in events.iter().filter(|e| major(e) && e.at(1) > 0.0) {
        let tg = &targets[e.unit];
        if units[e.unit].code == format!("{:05}", super::outages::PR_ISLAND) {
            // One island series: record it once, under San Juan (the largest municipio).
            if let Some(i) = index.get("72127") {
                cands.push((*i, e));
            }
        } else if let Some(&(ci, _)) = tg
            .iter()
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        {
            cands.push((ci, e));
        }
    }
    let label = |e: &Ev, recorded: usize| -> String {
        let class = CLASSES[e.class];
        if class == "hurricane" {
            if let Some(s) = storm_names.iter().find(|s| s.0 == e.cause) {
                let kind = if s.2 >= 64.0 {
                    "Hurricane"
                } else {
                    "Tropical Storm"
                };
                return format!("{kind} {}", title_case(&s.1));
            }
        }
        let d = date_of(e.start);
        for n in &named {
            if n.classes.iter().any(|c| c == class)
                && n.states.contains(&places[recorded].state)
                && d.as_str() >= n.from.as_str()
                && d.as_str() <= n.to.as_str()
            {
                return n.name.clone();
            }
        }
        let (y, m, _) = crate::timefmt::civil_from_days(e.start.div_euclid(86_400));
        const MONTHS: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        format!("{}, {} {y}", class_label(class), MONTHS[(m - 1) as usize])
    };
    let score = |e: &Ev| (e.at(7), e.at(14), e.at(3), e.peak);
    let mut stress = Table::new(
        &[
            "fips",
            "event",
            "class",
            "cause",
            "date",
            "recorded_in",
            "distance_km",
            "peak_share",
            "s_1d",
            "s_3d",
            "s_7d",
            "s_14d",
            "s_30d",
            "source",
        ],
        1,
    );
    for (i, p) in places.iter().enumerate() {
        let mut best: Option<(usize, &Ev, f64)> = None;
        for (ci, e) in &cands {
            let same = *ci == i;
            if !same && places[*ci].region != p.region {
                continue;
            }
            let d = if same {
                0.0
            } else {
                haversine_km(p.lat, p.lon, places[*ci].lat, places[*ci].lon)
            };
            if d >= REGION_RADIUS_KM {
                continue;
            }
            let better = match &best {
                None => true,
                Some((_, b, _)) => {
                    let (x, y) = (score(e), score(b));
                    x.0.total_cmp(&y.0)
                        .then(x.1.total_cmp(&y.1))
                        .then(x.2.total_cmp(&y.2))
                        .then(x.3.total_cmp(&y.3))
                        .then(b.start.cmp(&e.start))
                        .is_gt()
                }
            };
            if better {
                best = Some((*ci, e, d));
            }
        }
        // Hand-copied events stand in where they are worse than anything recorded.
        let hist = historic
            .iter()
            .filter(|h| h.states.contains(&p.state))
            .max_by(|a, b| a.share_at(7.0).total_cmp(&b.share_at(7.0)));
        if let Some(h) = hist
            && best
                .as_ref()
                .is_none_or(|(_, e, _)| h.share_at(7.0) > e.at(7))
        {
            let mut row = vec![
                p.fips.clone(),
                h.name.clone(),
                h.class.clone(),
                h.storm_id.clone(),
                h.date.clone(),
                String::new(),
                String::new(),
                sig4(h.peak_share),
            ];
            row.extend(
                [1.0, 3.0, 7.0, 14.0, 30.0]
                    .iter()
                    .map(|d| sig4(h.share_at(*d))),
            );
            row.push(format!("historic:{}", h.id));
            stress.push(row);
            continue;
        }
        let Some((ci, e, d)) = best else { continue };
        let cust = units[e.unit].customers;
        let mut row = vec![
            p.fips.clone(),
            label(e, ci),
            CLASSES[e.class].to_string(),
            e.cause.split(';').next().unwrap_or("").to_string(),
            date_of(e.start),
            places[ci].fips.clone(),
            format!("{:.0}", d),
            sig4((e.peak / cust).min(1.0)),
        ];
        row.extend([1u32, 3, 7, 14, 30].iter().map(|dd| sig4(e.at(*dd))));
        row.push("eaglei".into());
        stress.push(row);
    }
    out.table(ctx, STRESS, &mut stress)?;

    // --- Optional pack: county event curves ----------------------------------------------
    // Keyed `county_fips`, not `fips`: most counties have no outage of a day or more, and the
    // coverage check in `verify` applies to per-county tables only.
    let mut opt = Table::new(
        &[
            "county_fips",
            "start",
            "class",
            "cause",
            "peak_share",
            "s_1d",
            "s_3d",
            "s_7d",
            "s_14d",
            "s_30d",
            "ge_1d_share",
            "ge_7d_share",
        ],
        2,
    );
    // Events that left a real share of the county out for a day or more (the qualifying test);
    // Puerto Rico's island-wide series is written once, under San Juan (rr-data serves it for
    // every municipio). Two significant figures.
    let pr_code = format!("{:05}", super::outages::PR_ISLAND);
    for e in events.iter().filter(|e| e.ge[0] >= units[e.unit].qualify()) {
        let cust = units[e.unit].customers;
        let start = crate::timefmt::format_unix(e.start)[..16].replace('T', " ");
        let places_of: Vec<usize> = if units[e.unit].code == pr_code {
            index.get("72127").copied().into_iter().collect()
        } else {
            targets[e.unit].iter().map(|(ci, _)| *ci).collect()
        };
        for ci in places_of {
            // The cause without its episode number (the storm id stays): the id adds entropy
            // the web does not use.
            let cause = e
                .cause
                .split(';')
                .next()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("")
                .to_string();
            let mut row = vec![
                places[ci].fips.clone(),
                start.clone(),
                CLASSES[e.class].to_string(),
                cause,
                sig((e.peak / cust).min(1.0), 2),
            ];
            row.extend([1u32, 3, 7, 14, 30].iter().map(|d| sig(e.at(*d), 2)));
            row.push(sig(e.ge[0] / cust, 2));
            row.push(sig(e.ge[2] / cust, 2));
            opt.push(row);
        }
    }
    // Two units can map to one Connecticut region at the same start minute; keep the larger.
    opt.rows.sort();
    opt.rows.dedup_by(|a, b| a[0] == b[0] && a[1] == b[1]);
    out.table(ctx, OPT_EVENTS, &mut opt)?;

    // --- Held-out test ---------------------------------------------------------------------------
    let (holdout, lines) = holdout_test(&places, &targets, &units, &events, &customers, cfg);
    for l in &lines {
        eprintln!("  {l}");
    }
    let mut ht = holdout;
    out.table(ctx, OPT_HOLDOUT, &mut ht)?;
    out.notes.extend(lines);

    // --- Counties without rows, by reason (verify checks every county-keyed file) -------------
    let covered_pooled: BTreeSet<String> = pooled.rows.iter().map(|r| r[0].clone()).collect();
    let covered_own: BTreeSet<String> = causes.rows.iter().map(|r| r[0].clone()).collect();
    let covered_stress: BTreeSet<String> = stress.rows.iter().map(|r| r[0].clone()).collect();
    let mut missing: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for p in &places {
        let reason = if !covered_pooled.contains(&p.fips) {
            "No EAGLE-I record for the county or any county near it (island areas outside the EAGLE-I coverage)"
        } else if !covered_own.contains(&p.fips) {
            "No EAGLE-I record of its own: the pooled tail is its region's, and there are no causes to report"
        } else if !covered_stress.contains(&p.fips) {
            "No major outage event recorded in the county's region"
        } else {
            continue;
        };
        missing.entry(reason).or_default().push(p.fips.clone());
    }
    out.missing = missing
        .into_iter()
        .map(|(reason, fips)| crate::manifest::Missing {
            reason: reason.to_string(),
            fips,
        })
        .collect();

    // --- Provenance and notes ---------------------------------------------------------------
    out.source(SourceRecord {
        name: "Hand-copied pre-2014 outage events and event names (crates/rr-etl/data/historic_outages.toml)".into(),
        url: "crates/rr-etl/data/historic_outages.toml".into(),
        version: format!("{} events, {} names", historic.len(), named.len()),
        retrieved: String::new(),
        sha256: crate::http::sha256_hex(HISTORIC.as_bytes()),
        bytes: HISTORIC.len() as u64,
        license: "figures from the sources listed per event (US Government works)".into(),
        obligations: String::new(),
    });
    out.definitions
        .insert("pooled_tail".into(), pooled_tail_definition(cfg));
    out.definitions
        .insert("attribution".into(), attribution_definition());
    out.notes.push(regions_note(cfg));
    for h in &historic {
        out.notes.push(format!(
            "Hand-copied event {} ({}, {}): share of the peak still out at {:?} days = {:?}; sources: {}{}.",
            h.id,
            h.name,
            h.date,
            h.days,
            h.share_out,
            h.sources.join("; "),
            if h.unverified { "; UNVERIFIED figures" } else { "" }
        ));
    }
    Ok(out)
}

/// Bins of the predicted yearly chance of a qualifying event, for the reliability rows.
const RELIABILITY_BINS: [(f64, f64, &str); 5] = [
    (0.0, 0.01, "under 1 in 100"),
    (0.01, 0.03, "1 to 3 in 100"),
    (0.03, 0.1, "3 to 10 in 100"),
    (0.1, 0.3, "10 to 30 in 100"),
    (0.3, 1.01, "30 in 100 or more"),
];

/// The held-out test: fit 2014-2019, predict 2020-2025 (and an even/odd-year split that the
/// lower coverage of the early years does not bias). For each outage length and each estimator
/// (the county's own record, the region alone, the blend) it reports: predicted and observed
/// customer outages lasting that long (calibration in the large); the mean Poisson deviance of
/// each county's count of qualifying events (lower is better; a county-only estimate that saw
/// none predicts none and is penalised when one comes); and a reliability table: the predicted
/// yearly chance that a county records a qualifying event against the share of county-years
/// that did.
fn holdout_test(
    places: &[Place],
    targets: &[Vec<(usize, f64)>],
    units: &[Unit],
    events: &[Ev],
    customers: &[f64],
    cfg: PoolCfg,
) -> (Table, Vec<String>) {
    let nb = &neighbours(places, cfg);
    let mut table = Table::new(
        &[
            "split",
            "mark_days",
            "estimator",
            "measure",
            "bin",
            "predicted",
            "observed",
            "n",
        ],
        5,
    );
    let mut lines = Vec::new();
    type Filter = Box<dyn Fn(u16) -> bool>;
    let splits: Vec<(&str, Filter, Filter)> = vec![
        (
            "2014-2019 -> 2020-2025",
            Box::new(|y| y <= 2019),
            Box::new(|y| y >= 2020),
        ),
        (
            "even years -> odd years",
            Box::new(|y| y % 2 == 0),
            Box::new(|y| y % 2 == 1),
        ),
    ];
    let year_of = |t: i64| crate::timefmt::civil_from_days(t.div_euclid(86_400)).0 as u16;
    for (name, fit, test) in &splits {
        // Unit-level sums for a year filter, and qualifying events per unit and year.
        let sums = |f: &dyn Fn(u16) -> bool| {
            let mut ge = vec![[0.0f64; 5]; units.len()];
            let mut q = vec![[0.0f64; 5]; units.len()];
            let mut qy: Vec<BTreeMap<u16, [f64; 5]>> = vec![BTreeMap::new(); units.len()];
            for e in events {
                if OUTSIDE_POOL.contains(&CLASSES[e.class]) {
                    continue;
                }
                let y = year_of(e.start);
                if !f(y) {
                    continue;
                }
                let thr = units[e.unit].qualify();
                for k in 0..5 {
                    ge[e.unit][k] += e.ge[k];
                    if e.ge[k] >= thr {
                        q[e.unit][k] += 1.0;
                        qy[e.unit].entry(y).or_default()[k] += 1.0;
                    }
                }
            }
            let years: Vec<f64> = units
                .iter()
                .map(|u| {
                    u.months
                        .iter()
                        .filter(|(y, _)| f(**y))
                        .map(|(_, m)| f64::from(*m))
                        .sum::<f64>()
                        / 12.0
                })
                .collect();
            (ge, q, years, qy)
        };
        let (fge, fq, fyears, _) = sums(fit.as_ref());
        let (tge, tq, tyears, tqy) = sums(test.as_ref());
        let to_county = |ge: &[[f64; 5]], q: &[[f64; 5]], years: &[f64]| {
            let mut s = vec![CountyStats::default(); places.len()];
            for (u, ts) in targets.iter().enumerate() {
                if years[u] <= 0.0 || units[u].customers <= 0.0 {
                    continue;
                }
                for (c, w) in ts {
                    let x = &mut s[*c];
                    x.has_data = true;
                    x.years += w * years[u];
                    for k in 0..5 {
                        x.own[k] += w * ge[u][k] / units[u].customers / years[u];
                        x.qual_rate[k] += w * q[u][k] / years[u];
                    }
                }
            }
            s
        };
        let fit_s = to_county(&fge, &fq, &fyears);
        let test_s = to_county(&tge, &tq, &tyears);
        // Test years with data per county, and qualifying events per county-year (the county's
        // main unit).
        let main_unit: Vec<Option<usize>> = {
            let mut m: Vec<Option<(usize, f64)>> = vec![None; places.len()];
            for (u, ts) in targets.iter().enumerate() {
                for (c, w) in ts {
                    if m[*c].is_none_or(|(_, bw)| *w > bw) {
                        m[*c] = Some((u, *w));
                    }
                }
            }
            m.into_iter().map(|x| x.map(|(u, _)| u)).collect()
        };
        for (k, mark) in MARKS.iter().enumerate().take(4) {
            let mut acc: BTreeMap<&str, (f64, f64, f64, usize)> = BTreeMap::new();
            let mut rel: BTreeMap<(&str, usize), (f64, f64, usize)> = BTreeMap::new();
            for i in 0..places.len() {
                if !fit_s[i].has_data || !test_s[i].has_data || test_s[i].years <= 0.0 {
                    continue;
                }
                let Some(b) = blend(i, &fit_s, nb, customers, cfg.k) else {
                    continue;
                };
                if b.basis != "blend" {
                    continue;
                }
                let exposure = customers[i].max(1.0) * test_s[i].years;
                let obs = test_s[i].own[k] * exposure;
                let n_obs = test_s[i].qual_rate[k] * test_s[i].years;
                let own_q = fit_s[i].qual_rate[k];
                let mut qn = 0.0;
                let mut qd = 0.0;
                for (j, w) in &nb[i] {
                    if fit_s[*j].has_data {
                        qn += w[k] * fit_s[*j].years * fit_s[*j].qual_rate[k];
                        qd += w[k] * fit_s[*j].years;
                    }
                }
                let reg_q = if qd > 0.0 { qn / qd } else { own_q };
                let z = b.z[k];
                let preds = [
                    ("county only", fit_s[i].own[k], own_q),
                    ("region only", b.region[k], reg_q),
                    ("blend", b.lam[k], z * own_q + (1.0 - z) * reg_q),
                ];
                // County-years of the test: the main unit's years with data.
                let county_years: Vec<(u16, bool)> = main_unit[i]
                    .map(|u| {
                        units[u]
                            .months
                            .iter()
                            .filter(|(y, m)| test(**y) && **m >= 6)
                            .map(|(y, _)| (*y, tqy[u].get(y).is_some_and(|c| c[k] >= 1.0)))
                            .collect()
                    })
                    .unwrap_or_default();
                for (est, lam, q) in preds {
                    let mu = (q * test_s[i].years).max(1e-6);
                    let dev = if n_obs > 0.0 {
                        2.0 * (n_obs * ln(n_obs / mu) - (n_obs - mu))
                    } else {
                        2.0 * mu
                    };
                    let e = acc.entry(est).or_default();
                    e.0 += lam * exposure;
                    e.1 += obs;
                    e.2 += dev;
                    e.3 += 1;
                    let p = 1.0 - exp(-q);
                    let bin = RELIABILITY_BINS
                        .iter()
                        .position(|(lo, hi, _)| p >= *lo && p < *hi)
                        .unwrap_or(RELIABILITY_BINS.len() - 1);
                    let r = rel.entry((est, bin)).or_default();
                    for (_, hit) in &county_years {
                        r.0 += p;
                        r.1 += f64::from(u8::from(*hit));
                        r.2 += 1;
                    }
                }
            }
            for (est, (pred, obs, dev, n)) in &acc {
                let ratio = if *pred > 0.0 { obs / pred } else { f64::NAN };
                table.push(vec![
                    name.to_string(),
                    mark.to_string(),
                    est.to_string(),
                    "customer_outages".into(),
                    String::new(),
                    sig4(*pred),
                    sig4(*obs),
                    n.to_string(),
                ]);
                table.push(vec![
                    name.to_string(),
                    mark.to_string(),
                    est.to_string(),
                    "mean_poisson_deviance".into(),
                    String::new(),
                    sig4(dev / *n as f64),
                    String::new(),
                    n.to_string(),
                ]);
                lines.push(format!(
                    "Held-out test ({name}), outages of {mark}+ days, {est}: predicted {pred:.0} customer outages, observed {obs:.0} (observed/predicted {ratio:.2}); mean Poisson deviance of qualifying-event counts {:.3} over {n} counties.",
                    dev / *n as f64
                ));
            }
            for ((est, bin), (p, hits, n)) in &rel {
                if *n == 0 {
                    continue;
                }
                table.push(vec![
                    name.to_string(),
                    mark.to_string(),
                    est.to_string(),
                    "reliability".into(),
                    format!("{bin}: {}", RELIABILITY_BINS[*bin].2),
                    sig4(p / *n as f64),
                    sig4(hits / *n as f64),
                    n.to_string(),
                ]);
                if *est == "blend" {
                    lines.push(format!(
                        "Held-out reliability ({name}), {mark}+ days, blend, predicted {}: mean predicted yearly chance {:.3}, observed share of county-years {:.3} ({n} county-years).",
                        RELIABILITY_BINS[*bin].2,
                        p / *n as f64,
                        hits / *n as f64
                    ));
                }
            }
        }
    }
    (table, lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_and_credibility() {
        assert_eq!(kernel(0.0), 1.0);
        assert_eq!(kernel(REGION_RADIUS_KM), 0.0);
        assert!((kernel(125.0) - 0.5625).abs() < 1e-12);
        assert_eq!(credibility(0.0), 0.0);
        assert!((credibility(5.0) - 0.5).abs() < 1e-12);
        assert!((credibility(45.0) - 0.9).abs() < 1e-12);
    }

    #[test]
    fn storm_events_priority() {
        let c = |v: &[&str]| storm_events_cause(v.iter().copied());
        assert_eq!(c(&["heat", "tstm_wind"]), Some(("wind", "tstm_wind")));
        assert_eq!(
            c(&["high_wind", "winter_storm"]),
            Some(("winter", "winter_storm"))
        );
        assert_eq!(c(&["tstm_wind", "ice_storm"]), Some(("ice", "ice_storm")));
        assert_eq!(c(&["flood", "wildfire"]), Some(("wildfire", "wildfire")));
        assert_eq!(c(&["extreme_cold"]), Some(("cold_grid", "extreme_cold")));
        assert_eq!(c(&["rip_current"]), None);
    }

    #[test]
    fn oe417_kinds() {
        assert!(oe417_is_emergency(
            "Load Shed of 100+ MW Under Emergency Operational Policy"
        ));
        assert!(oe417_is_emergency("Energy Emergency Alert Level 3"));
        assert!(!oe417_is_emergency("Severe Weather"));
        assert!(oe417_is_grid("Transmission Interruption"));
        assert!(oe417_is_grid("Vandalism"));
        assert!(!oe417_is_grid("Severe Weather - Transmission Interruption"));
    }

    #[test]
    fn days_to_interpolates_in_log_time() {
        // 1, 2, 3, 5, 7, 10, 14, 21, 30, 60, 90 days.
        let c = [0.9, 0.8, 0.7, 0.5, 0.3, 0.1, 0.05, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(days_to(&c, 0.5), Some(5.0));
        assert_eq!(days_to(&c, 0.1), Some(10.0));
        let t = days_to(&c, 0.4).unwrap();
        assert!(t > 5.0 && t < 7.0, "{t}");
        assert_eq!(days_to(&[0.99; 11], 0.5), None);
    }

    #[test]
    fn hourly_track_interpolation() {
        let f = [(0i64, 20.0, -80.0, 40.0), (6 * 3600, 26.0, -86.0, 100.0)];
        let p = hourly(&f, 0);
        assert_eq!(p.len(), 7);
        assert!((p[3].lat - 23.0).abs() < 1e-9 && (p[3].lon + 83.0).abs() < 1e-9);
        assert!((p[3].wind - 70.0).abs() < 1e-9);
    }

    #[test]
    fn historic_table_parses_and_interpolates() {
        let (h, names) = historic().unwrap();
        let maria = h.iter().find(|x| x.id.starts_with("maria")).unwrap();
        assert!(maria.states.contains(&"PR".to_string()));
        assert_eq!(maria.share_at(0.0), 1.0);
        assert!(
            maria.share_at(30.0) > 0.5,
            "most of Puerto Rico was still dark a month after Maria"
        );
        assert!(!names.is_empty());
    }

    #[test]
    fn blend_shrinks_a_one_storm_county_toward_its_region() {
        // Three counties in one region; county 0 recorded one long outage, its neighbours none.
        let mk = |own7: f64, q7: f64| CountyStats {
            years: 10.0,
            rate: 1.0,
            own: [0.1, 0.05, own7, 0.0, 0.0],
            qual_rate: [2.0, 0.5, q7, 0.0, 0.0],
            has_data: true,
            ..Default::default()
        };
        let stats = vec![mk(0.08, 0.1), mk(0.0, 0.0), mk(0.0, 0.0)];
        let all = vec![(0, [1.0; 5]), (1, [1.0; 5]), (2, [1.0; 5])];
        let nb = vec![all.clone(), all.clone(), all];
        let cust = vec![1000.0; 3];
        let b0 = blend(0, &stats, &nb, &cust, CREDIBILITY_K).unwrap();
        let b1 = blend(1, &stats, &nb, &cust, CREDIBILITY_K).unwrap();
        // At 1 day the region records 2 qualifying events a year: E = 20 over 10 years, Z = 0.8.
        assert!((b0.z[0] - 0.8).abs() < 1e-12);
        // At 7 days: region rate 0.08 / 3, E = (1 event / 30 county-years) x 10 = 1/3.
        let e = 1.0 / 3.0;
        let z = e / (e + CREDIBILITY_K);
        assert!((b0.z[2] - z).abs() < 1e-12);
        assert!((b0.region[2] - 0.08 / 3.0).abs() < 1e-12);
        assert!((b0.lam[2] - (z * 0.08 + (1.0 - z) * 0.08 / 3.0)).abs() < 1e-12);
        // The storm's county keeps more of it than its neighbours, and both share it.
        assert!(b0.lam[2] > b1.lam[2] && b1.lam[2] > 0.0);
        assert_eq!(b0.region_counties, 2);
        // Leaving the county out of its own region (the old way) gives the neighbour more than
        // the county the storm hit.
        let w = [1.0; 5];
        let loo = vec![
            vec![(1, w), (2, w)],
            vec![(0, w), (2, w)],
            vec![(0, w), (1, w)],
        ];
        let l0 = blend(0, &stats, &loo, &cust, CREDIBILITY_K).unwrap();
        let l1 = blend(1, &stats, &loo, &cust, CREDIBILITY_K).unwrap();
        assert!(l1.lam[2] > l0.lam[2] * 0.5);
        assert_eq!(l0.z[2], 0.0);
    }

    #[test]
    fn the_manifest_text_describes_the_pooling_and_windows_in_use() {
        // The pack's pooling: by distance, the county in its own collective, 400/800 km.
        let note = regions_note(POOL);
        assert!(note.contains("by distance across NCA5 regions"), "{note}");
        assert!(note.contains("the county included"), "{note}");
        assert!(note.contains("radius 400/400/800/800/800 km"), "{note}");
        assert!(note.contains("within 250 km"), "{note}");
        let def = pooled_tail_definition(POOL);
        assert!(
            def.contains("over the county and every county within R(d)"),
            "{def}"
        );
        assert!(
            def.contains("400 km for 1 and 3 days, 800 km for 7, 14 and 30 days"),
            "{def}"
        );
        assert!(def.contains("Z = E / (E + 5)"), "{def}");
        // An override is on record.
        let other = PoolCfg {
            radius_km: [250.0; 5],
            k: 5.0,
            same_region: true,
            include_self: false,
        };
        assert!(regions_note(other).contains(
            "within the county's NCA5 region, the county left out; radius 250/250/250/250/250 km"
        ));
        assert!(pooled_tail_definition(other).contains("(leaving the county out)"));
        // Attribution windows are anchored on the peak, as `run` applies them.
        let a = attribution_definition();
        assert!(
            a.contains(
                "from 36 hours before the outage began to 24 hours after it reached its peak"
            ),
            "{a}"
        );
        assert!(
            a.contains("the 6 hours before the start to 2 hours after the peak"),
            "{a}"
        );
        assert!(a.contains("within 300 km"), "{a}");
    }
}
