//! Job 3 — power outages per county from ORNL EAGLE-I (2014-2025, CC BY 4.0).
//!
//! EAGLE-I records, every 15 minutes, how many electricity customers in each county are without
//! power (scraped from utility outage maps; only snapshots with at least one customer out are
//! listed). The files total 11.6 GB, so each year is streamed straight from figshare, parsed on
//! the fly and never written to disk.
//!
//! This is the empirical duration data for the `power` bucket. The event definition is
//! [`DEFINITION`]; see `docs/DATA_SOURCES.md` for the reasoning. In short:
//! - an **event** is a period when at least 1% of the county's customers (ORNL's modelled
//!   customer count, minimum 10) are out, lasting at least an hour, with dips or missing
//!   snapshots of up to 2 hours bridged;
//! - **durations are per customer**: inside an event, customers are assumed to be restored in the
//!   order they lost power (first out, first restored). That splits the event's customer-hours
//!   into individual outage lengths, so a storm that leaves 5% of customers out for a week and
//!   everyone else for a day is not reported as "a week" for everyone;
//! - **rates are per customer**: customer outages in events, divided by the county's customers
//!   and by the years with data.

use super::{Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::{Table, col, parse_delimited};
use crate::ct::{Crosswalk, is_old_ct};
use crate::manifest::{Attribution, SourceRecord};
use crate::num::{sig4, weighted_quantile};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::io::BufReader;

/// County outage statistics.
pub const OUTAGES: &str = "core/outages.csv";
/// The same statistics pooled by state (for small-sample fallback and cross-checks).
pub const OUTAGES_STATE: &str = "core/outages_state.csv";

const ARTICLE: &str = "https://api.figshare.com/v2/articles/24237376";
/// CDC/ATSDR SVI 2022 county file, used for household counts (the floor on customers).
pub const SVI_COUNTY: &str =
    "https://svi.cdc.gov/Documents/Data/2022/csv/states_counties/SVI_2022_US_county.csv";
const DOI: &str = "https://doi.org/10.6084/m9.figshare.24237376";

/// Share of county customers that must be out for an event to start.
pub const THRESHOLD_SHARE: f64 = 0.01;
/// Minimum customers out for an event to start, however small the county.
pub const THRESHOLD_MIN: f64 = 10.0;
/// Once started, an event continues while at least this share of customers is out (hysteresis,
/// so the slow tail of a restoration is not cut off at the start threshold).
pub const END_SHARE: f64 = 0.0025;
/// Minimum customers for the end threshold.
pub const END_MIN: f64 = 5.0;
/// Snapshots at or above the start threshold an event needs (4 = one hour).
pub const MIN_START_SNAPSHOTS: u32 = 4;
/// Gaps below the end threshold (or missing snapshots) up to this long are bridged.
pub const BRIDGE_S: i64 = 7200;
/// Noise filter: a reversal inside an event counts only when the count moves by more than this
/// share of the current peak or trough (and by at least the start threshold).
pub const REVERSAL_SHARE: f64 = 0.5;
/// EAGLE-I snapshot interval.
pub const SLOT_S: i64 = 900;
/// Counties with fewer events than this use their state's pooled duration distribution.
pub const MIN_EVENTS_FOR_COUNTY_DURATIONS: u32 = 10;
/// State-years (2018-2022, the years ORNL publishes coverage for) with less customer coverage
/// than this are left out.
pub const MIN_STATE_COVERAGE: f64 = 0.5;

/// A gap or dip longer than [`BRIDGE_S`] is bridged up to this long when it looks like a
/// reporting dropout: the count before it was at least the start threshold and the count after
/// it comes back to at least [`DROPOUT_REBOUND`] of that level. EAGLE-I's county count is the sum
/// of each utility's outage map; a utility missing from a scrape reads as everyone restored at
/// once and everyone losing power again when it reappears. Utility outage maps fail most often
/// in the worst storms, for days at a time: Hurricane Michael's restoration in Jackson County,
/// Florida (October 2018) has data gaps of two and three days while about 10,000 customers were
/// still out.
pub const MAX_DROPOUT_S: i64 = 3 * 86_400;
/// See [`MAX_DROPOUT_S`].
pub const DROPOUT_REBOUND: f64 = 0.5;
/// Half-width, in snapshots, of the running median used to cut doubled counts inside a sustained
/// outage (12 = three hours each side).
pub const SPIKE_MEDIAN_HALF: usize = 12;
/// Inside a sustained outage, a count more than this many times the running median is taken to
/// be a report counted twice and is cut to the median. Smaller excursions are left to the
/// reversal filter ([`zigzag`]), and genuine peaks of short outages are kept.
pub const SPIKE_RATIO: f64 = 1.5;
/// Half-width, in snapshots, of the closing that fills dropouts (48 = dips narrower than 24 hours
/// are filled to the level around them).
pub const DROPOUT_CLOSE_HALF: usize = 48;
/// Outage lengths, in days, whose customer counts each event records (first out, first restored).
pub const GE_DAYS: [u32; 5] = [1, 3, 7, 14, 30];
/// Days after the peak at which each event records the share of the peak still out.
pub const CURVE_DAYS: [u32; 11] = [1, 2, 3, 5, 7, 10, 14, 21, 30, 60, 90];

/// One outage event, handed to the `outage_model` job through an intermediate file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventRecord {
    /// First snapshot of the event (UTC seconds).
    pub start: i64,
    /// The last snapshot at or above the end threshold, plus one snapshot interval.
    pub end: i64,
    /// When the repaired count peaked (first time it reached its maximum).
    pub peak_t: i64,
    /// Customers out at the peak (repaired series, see [`repair`]).
    pub peak: f64,
    /// Customer outages in the event (first out, first restored).
    pub arrivals: f64,
    /// Customer-hours without power in the event.
    pub customer_hours: f64,
    /// Customer outages lasting at least [`GE_DAYS`] days.
    pub ge: [f64; 5],
    /// Share of the peak still out [`CURVE_DAYS`] days after the peak: the running minimum of
    /// the repaired count since the peak, so a customer counted at day N has been out at least N
    /// days (a lower bound on the per-customer tail, as the model review's M-02 proposes).
    pub curve: [f64; 11],
}

/// Synthetic unit for Puerto Rico. EAGLE-I files LUMA's outages by utility region, each under one
/// "hub" municipio (the hubs' customer counts in the 2024 file sum to the island's), so the
/// hubs are summed into one island-wide series and every municipio gets the island's figures.
pub const PR_ISLAND: u32 = 72000;

/// The event definition, quoted into the manifest and into every `OutageStats`.
pub const DEFINITION: &str = "EAGLE-I outage event: starts when at least 1% of the county's electricity customers (the larger of ORNL's modelled customer count and the county's households; at least 10 customers) are reported without power, needs at least 1 hour at that level, and lasts until fewer than 0.25% (at least 5) remain out, with dips or missing 15-minute snapshots of up to 2 hours bridged, and up to 72 hours when the count before the gap was at least 1% and comes back to at least half of it (a utility missing from the data for a while). Inside an event the counts are repaired before durations are read: during a sustained outage, counts more than 1.5 times the 3-hour running median (reports counted twice) are cut to the median, and dips shorter than 24 hours are filled to the level around them (a utility missing from some snapshots, which otherwise reads as everyone restored and cut off again); short reversals under half of the current peak or trough, or under the 1% level, are treated as reporting noise. Durations are per customer: customers are assumed to be restored in the order they lost power. Rates are customer outages in such events per customer per year of data.";

#[derive(Debug, Clone, Default)]
struct OpenEvent {
    start: i64,
    /// Last snapshot at or above the end threshold.
    last_active: i64,
    /// Customers out at `last_active`.
    last_active_level: f64,
    /// Snapshots at or above the start threshold.
    start_snapshots: u32,
    rows: Vec<(i64, f64)>,
}

#[derive(Debug, Clone, Default)]
struct Unit {
    customers: f64,
    threshold: f64,
    end_threshold: f64,
    last_t: i64,
    open: Option<OpenEvent>,
    months: BTreeSet<u32>,
    years: BTreeSet<u16>,
    cust_hours_all: f64,
    cust_hours_events: f64,
    events: u32,
    arrivals: f64,
    /// Customer outage lengths in 15-minute slots -> customers.
    durations: BTreeMap<u32, f64>,
    max_event_s: i64,
    out_of_order: u64,
    duplicates: u64,
    /// Offset of local standard time from UTC (from the county's longitude), for daily totals.
    tz_offset_s: i64,
    /// Every event, in time order.
    records: Vec<EventRecord>,
    /// Customer-hours inside events per local calendar day (days since 1970-01-01).
    daily: BTreeMap<i64, f64>,
    /// Gaps bridged as reporting dropouts.
    dropouts_bridged: u32,
}

#[derive(Debug, Clone, Default)]
struct State {
    units: BTreeMap<u32, Unit>,
    excluded_years: BTreeSet<(String, u16)>,
}

fn year_month(t: i64) -> (u16, u32) {
    let (y, m, _) = crate::timefmt::civil_from_days(t.div_euclid(86_400));
    (y as u16, m)
}

impl Unit {
    fn new(customers: f64, tz_offset_s: i64) -> Self {
        Unit {
            customers,
            threshold: (THRESHOLD_SHARE * customers).max(THRESHOLD_MIN),
            end_threshold: (END_SHARE * customers).max(END_MIN),
            last_t: i64::MIN,
            tz_offset_s,
            ..Default::default()
        }
    }

    fn close(&mut self) {
        let Some(ev) = self.open.take() else { return };
        if ev.start_snapshots < MIN_START_SNAPSHOTS {
            return;
        }
        let dur = ev.last_active - ev.start + SLOT_S;
        let slots = ((ev.last_active - ev.start) / SLOT_S + 1) as usize;
        // Regular 15-minute series, carrying the last value across missing snapshots.
        let mut v = vec![0.0f64; slots];
        let mut filled = vec![false; slots];
        for (t, n) in ev.rows.iter().filter(|(t, _)| *t <= ev.last_active) {
            let i = ((t - ev.start) / SLOT_S) as usize;
            if i < slots {
                v[i] = v[i].max(*n);
                filled[i] = true;
            }
        }
        for i in 1..slots {
            if !filled[i] {
                v[i] = v[i - 1];
            }
        }
        let v = repair(&v, self.threshold);
        // The event curve and daily totals come from the repaired series; durations from its
        // flicker-filtered version.
        let (pk, peak) =
            v.iter().enumerate().fold(
                (0usize, f64::MIN),
                |acc, (i, x)| if *x > acc.1 { (i, *x) } else { acc },
            );
        let mut curve = [0.0f64; 11];
        if peak > 0.0 {
            let mut running = peak;
            let mut at = pk;
            for (k, d) in CURVE_DAYS.iter().enumerate() {
                let j = pk + *d as usize * 96;
                if j >= slots {
                    break;
                }
                while at < j {
                    at += 1;
                    running = running.min(v[at]);
                }
                curve[k] = running / peak;
            }
        }
        let hours = SLOT_S as f64 / 3600.0;
        for (i, x) in v.iter().enumerate() {
            if *x > 0.0 {
                let local = ev.start + i as i64 * SLOT_S + self.tz_offset_s;
                *self.daily.entry(local.div_euclid(86_400)).or_default() += x * hours;
            }
        }
        let v = zigzag(&v, self.threshold, REVERSAL_SHARE);
        let (durations, arrivals) = fifo(&v);
        let mut ge = [0.0f64; 5];
        for (k, d) in GE_DAYS.iter().enumerate() {
            ge[k] = durations.range(d * 96..).map(|(_, w)| *w).sum();
        }
        for (k, w) in &durations {
            *self.durations.entry(*k).or_default() += w;
        }
        let customer_hours = v.iter().sum::<f64>() * hours;
        self.events += 1;
        self.arrivals += arrivals;
        self.cust_hours_events += customer_hours;
        self.max_event_s = self.max_event_s.max(dur);
        self.records.push(EventRecord {
            start: ev.start,
            end: ev.last_active + SLOT_S,
            peak_t: ev.start + pk as i64 * SLOT_S,
            peak,
            arrivals,
            customer_hours,
            ge,
            curve,
        });
    }

    fn row(&mut self, t: i64, n: f64) {
        if t < self.last_t {
            self.out_of_order += 1;
            return;
        }
        if t == self.last_t {
            self.duplicates += 1;
            if let Some(ev) = self.open.as_mut()
                && let Some(last) = ev.rows.last_mut()
                && last.0 == t
            {
                last.1 = last.1.max(n);
                if last.1 >= self.end_threshold {
                    ev.last_active = t;
                    ev.last_active_level = last.1;
                }
            }
            return;
        }
        self.last_t = t;
        self.cust_hours_all += n * (SLOT_S as f64 / 3600.0);
        if let Some(ev) = self.open.as_ref() {
            let gap = t - ev.last_active;
            if gap > BRIDGE_S {
                let high_before = ev.last_active_level >= self.threshold;
                let close = if !high_before || gap > MAX_DROPOUT_S {
                    true
                } else if n >= self.end_threshold {
                    // The data are back: a dropout if the count rebounds to a similar level,
                    // otherwise the outage ended while the data were missing.
                    n < DROPOUT_REBOUND * ev.last_active_level
                } else {
                    // Still low after a high count: wait (up to MAX_DROPOUT_S) to see whether
                    // it comes back.
                    false
                };
                if close {
                    self.close();
                } else if n >= self.end_threshold {
                    self.dropouts_bridged += 1;
                }
            }
        }
        match self.open.as_mut() {
            None => {
                if n >= self.threshold {
                    self.open = Some(OpenEvent {
                        start: t,
                        last_active: t,
                        last_active_level: n,
                        start_snapshots: 1,
                        rows: vec![(t, n)],
                    });
                }
            }
            Some(ev) => {
                ev.rows.push((t, n));
                if n >= self.end_threshold {
                    ev.last_active = t;
                    ev.last_active_level = n;
                }
                if n >= self.threshold {
                    ev.start_snapshots += 1;
                }
            }
        }
    }
}

/// First out, first restored: split an event's (flicker-filtered) customer counts into
/// individual outage lengths. Returns the histogram (15-minute slots -> customers) and the number
/// of customer outages (arrivals).
fn fifo(v: &[f64]) -> (BTreeMap<u32, f64>, f64) {
    let mut durations: BTreeMap<u32, f64> = BTreeMap::new();
    if v.is_empty() {
        return (durations, 0.0);
    }
    let mut queue: VecDeque<(usize, f64)> = VecDeque::new();
    let mut arrivals = v[0];
    queue.push_back((0, v[0]));
    let depart = |queue: &mut VecDeque<(usize, f64)>,
                  mut x: f64,
                  at: usize,
                  durs: &mut BTreeMap<u32, f64>| {
        while x > 1e-9 {
            let Some(front) = queue.front_mut() else {
                break;
            };
            let m = front.1.min(x);
            *durs.entry((at - front.0) as u32).or_default() += m;
            front.1 -= m;
            x -= m;
            if front.1 <= 1e-9 {
                queue.pop_front();
            }
        }
    };
    for i in 1..v.len() {
        let d = v[i] - v[i - 1];
        if d > 0.0 {
            queue.push_back((i, d));
            arrivals += d;
        } else if d < 0.0 {
            depart(&mut queue, -d, i, &mut durations);
        }
    }
    depart(&mut queue, v[v.len() - 1], v.len(), &mut durations);
    (durations, arrivals)
}

/// Running median over a window of `half` values on each side (truncated at the ends).
pub fn running_median(v: &[f64], half: usize) -> Vec<f64> {
    let n = v.len();
    let mut out = vec![0.0; n];
    // A sorted copy of the current window, updated one value at a time.
    let mut win: Vec<f64> = Vec::with_capacity(2 * half + 1);
    let (mut lo, mut hi) = (0usize, 0usize); // window is v[lo..hi]
    for (i, o) in out.iter_mut().enumerate() {
        let a = i.saturating_sub(half);
        let b = (i + half + 1).min(n);
        while hi < b {
            let x = v[hi];
            let pos = win.partition_point(|y| *y < x);
            win.insert(pos, x);
            hi += 1;
        }
        while lo < a {
            let x = v[lo];
            let pos = win.partition_point(|y| *y < x);
            win.remove(pos);
            lo += 1;
        }
        let m = win.len();
        *o = if m % 2 == 1 {
            win[m / 2]
        } else {
            0.5 * (win[m / 2 - 1] + win[m / 2])
        };
    }
    out
}

/// Sliding maximum (`max = true`) or minimum over `half` values on each side, in linear time.
fn sliding(v: &[f64], half: usize, max: bool) -> Vec<f64> {
    let n = v.len();
    let mut out = vec![0.0; n];
    let mut dq: VecDeque<usize> = VecDeque::new();
    let better = |a: f64, b: f64| if max { a >= b } else { a <= b };
    let mut next = 0usize;
    for (i, o) in out.iter_mut().enumerate() {
        let b = (i + half).min(n - 1);
        while next <= b {
            while let Some(&back) = dq.back() {
                if better(v[next], v[back]) {
                    dq.pop_back();
                } else {
                    break;
                }
            }
            dq.push_back(next);
            next += 1;
        }
        while let Some(&front) = dq.front() {
            if front + half < i {
                dq.pop_front();
            } else {
                break;
            }
        }
        *o = v[dq[0]];
    }
    out
}

/// Morphological closing (sliding maximum, then sliding minimum) with zero padding at both ends:
/// fills every dip narrower than `2 * half + 1` values to the lower of the levels around it,
/// and leaves rising and falling stretches (a storm's onset, a restoration) unchanged.
pub fn closing(v: &[f64], half: usize) -> Vec<f64> {
    let mut padded = vec![0.0; half];
    padded.extend_from_slice(v);
    padded.extend(std::iter::repeat_n(0.0, half));
    let c = sliding(&sliding(&padded, half, true), half, false);
    c[half..half + v.len()].to_vec()
}

/// Repair an event's counts before durations are read (see [`DEFINITION`]). EAGLE-I's county
/// count is the sum of the utilities' outage maps, scraped every 15 minutes; during big storms a
/// utility is often missing from a scrape (the count drops by most of the outage for 15 minutes
/// to a few hours) or counted twice. Left alone, each drop reads as customers restored and each
/// return as new customers losing power, which chops a two-week outage into hour-long pieces
/// (Buncombe County during Helene: 9.7 million "customer outages" among 181,000 customers).
///
/// 1. Where the 3-hour running median shows a sustained outage (at least `threshold`), counts
///    more than [`SPIKE_RATIO`] times the median are cut to it (reports counted twice).
/// 2. Dips narrower than 24 hours are filled to the level around them (a closing).
pub fn repair(v: &[f64], threshold: f64) -> Vec<f64> {
    if v.len() < 3 {
        return v.to_vec();
    }
    let med = running_median(v, SPIKE_MEDIAN_HALF);
    let clipped: Vec<f64> = v
        .iter()
        .zip(&med)
        .map(|(x, m)| {
            if *m >= threshold && *x > SPIKE_RATIO * m {
                *m
            } else {
                *x
            }
        })
        .collect();
    closing(&clipped, DROPOUT_CLOSE_HALF)
}

/// Reporting-noise filter for an event's customer counts. Finds the significant peaks and troughs
/// (a reversal counts only when the count moves by more than `share` of the current extreme and
/// by more than `floor`), then makes the series monotone between them: running maximum on rising
/// legs, running minimum on falling legs. Genuine second waves survive; flicker does not.
pub fn zigzag(v: &[f64], floor: f64, share: f64) -> Vec<f64> {
    let n = v.len();
    if n < 3 {
        return v.to_vec();
    }
    let mut ext = vec![0usize];
    let mut seeking_max = true;
    let mut cand = 0usize;
    for (i, &x) in v.iter().enumerate().skip(1) {
        if seeking_max {
            if x >= v[cand] {
                cand = i;
            } else if v[cand] - x > (share * v[cand]).max(floor) {
                ext.push(cand);
                seeking_max = false;
                cand = i;
            }
        } else if x <= v[cand] {
            cand = i;
        } else if x - v[cand] > (share * v[cand]).max(floor) {
            ext.push(cand);
            seeking_max = true;
            cand = i;
        }
    }
    ext.push(cand);
    ext.push(n - 1);
    ext.dedup();
    let mut f = v.to_vec();
    for w in ext.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mut m = v[a];
        if v[b] >= v[a] {
            for i in a..=b {
                m = m.max(v[i]);
                f[i] = m;
            }
        } else {
            for i in a..=b {
                m = m.min(v[i]);
                f[i] = m;
            }
        }
    }
    f
}

/// Distribution summary from a duration histogram (slots -> customers).
#[derive(Debug, Clone, Default, PartialEq)]
struct Dist {
    p1: f64,
    p3: f64,
    p7: f64,
    p14: f64,
    p30: f64,
    median_h: f64,
    p90_h: f64,
}

fn summarise(durations: &BTreeMap<u32, f64>) -> Option<Dist> {
    let total: f64 = durations.values().sum();
    if total <= 0.0 {
        return None;
    }
    let ge = |slots: u32| durations.range(slots..).map(|(_, w)| *w).sum::<f64>() / total;
    let sorted: Vec<(f64, f64)> = durations
        .iter()
        .map(|(s, w)| (*s as f64 * SLOT_S as f64 / 3600.0, *w))
        .collect();
    Some(Dist {
        p1: ge(96),
        p3: ge(288),
        p7: ge(672),
        p14: ge(1344),
        p30: ge(2880),
        median_h: weighted_quantile(&sorted, 0.5)?,
        p90_h: weighted_quantile(&sorted, 0.9)?,
    })
}

fn parse_time(b: &[u8]) -> Option<i64> {
    crate::timefmt::parse_datetime(std::str::from_utf8(b).ok()?)
}

/// Per-year diagnostics.
#[derive(Debug, Default, Clone)]
struct YearDiag {
    rows: u64,
    timestamps: usize,
    gaps_over_1h: usize,
    gap_hours: f64,
}

/// UTC offset of local standard time at a longitude (one hour per 15 degrees), in seconds.
pub fn tz_offset_for_lon(lon: f64) -> i64 {
    (lon / 15.0).round() as i64 * 3600
}

fn process_year(
    reader: &mut dyn std::io::Read,
    state: &mut State,
    mcc: &BTreeMap<u32, f64>,
    state_of: &dyn Fn(u32) -> String,
    coverage: &BTreeMap<(String, u16), f64>,
    lon_of: &dyn Fn(u32) -> f64,
) -> Result<YearDiag> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .buffer_capacity(1 << 20)
        .from_reader(BufReader::with_capacity(1 << 20, reader));
    let headers = rdr.byte_headers()?.clone();
    let find = |names: &[&str]| {
        headers
            .iter()
            .position(|h| names.iter().any(|n| h.eq_ignore_ascii_case(n.as_bytes())))
    };
    let i_f =
        find(&["fips_code", "fips"]).ok_or_else(|| data_err("EAGLE-I: no fips_code column"))?;
    let i_n = find(&["customers_out", "sum"])
        .ok_or_else(|| data_err("EAGLE-I: no customers_out/sum column"))?;
    let i_t =
        find(&["run_start_time"]).ok_or_else(|| data_err("EAGLE-I: no run_start_time column"))?;
    let mut rec = csv::ByteRecord::new();
    let mut diag = YearDiag::default();
    let mut stamps: HashSet<i64> = HashSet::with_capacity(40_000);
    let mut pr_sums: BTreeMap<i64, f64> = BTreeMap::new();
    let pr_customers: f64 = mcc.range(72001..73000).map(|(_, c)| *c).sum();
    while rdr.read_byte_record(&mut rec)? {
        diag.rows += 1;
        let (Some(f), Some(n), Some(t)) = (rec.get(i_f), rec.get(i_n), rec.get(i_t)) else {
            continue;
        };
        let Some(fips) = std::str::from_utf8(f)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
        else {
            continue;
        };
        let Some(n) = std::str::from_utf8(n)
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
        else {
            continue;
        };
        let Some(t) = parse_time(t) else { continue };
        stamps.insert(t);
        let st = state_of(fips);
        let (y, m) = year_month(t);
        if coverage
            .get(&(st.clone(), y))
            .is_some_and(|c| *c < MIN_STATE_COVERAGE)
        {
            state.excluded_years.insert((st, y));
            continue;
        }
        if fips / 1000 == 72 {
            *pr_sums.entry(t).or_default() += n;
            let island = state
                .units
                .entry(PR_ISLAND)
                .or_insert_with(|| Unit::new(pr_customers, tz_offset_for_lon(lon_of(PR_ISLAND))));
            island.months.insert(y as u32 * 12 + m);
            island.years.insert(y);
            continue;
        }
        let unit = state.units.entry(fips).or_insert_with(|| {
            Unit::new(
                mcc.get(&fips).copied().unwrap_or(0.0),
                tz_offset_for_lon(lon_of(fips)),
            )
        });
        if unit.customers <= 0.0 {
            continue;
        }
        unit.months.insert(y as u32 * 12 + m);
        unit.years.insert(y);
        unit.row(t, n);
    }
    // Puerto Rico: feed the island-wide sums in time order (works for either file sort order).
    if let Some(island) = state.units.get_mut(&PR_ISLAND) {
        for (t, n) in pr_sums {
            island.row(t, n);
        }
    }
    let mut ts: Vec<i64> = stamps.into_iter().collect();
    ts.sort_unstable();
    diag.timestamps = ts.len();
    for w in ts.windows(2) {
        let g = w[1] - w[0];
        if g > 3600 {
            diag.gaps_over_1h += 1;
            diag.gap_hours += (g - SLOT_S) as f64 / 3600.0;
        }
    }
    Ok(diag)
}

/// Columns of the per-event intermediate ([`crate::intermediate::OUTAGE_EVENTS`]).
pub const EVENT_COLUMNS: &[&str] = &[
    "unit",
    "start",
    "end",
    "peak_t",
    "peak",
    "arrivals",
    "customer_hours",
    "ge1",
    "ge3",
    "ge7",
    "ge14",
    "ge30",
    "c1",
    "c2",
    "c3",
    "c5",
    "c7",
    "c10",
    "c14",
    "c21",
    "c30",
    "c60",
    "c90",
];

/// Hand every event, each unit's exposure and the daily customer-hours to the jobs that model
/// outages (`outage_model`) and join them to temperatures (`climate_daily`).
fn write_intermediates(ctx: &Ctx, state: &State) -> Result<()> {
    let mut ev_rows = Vec::new();
    let mut unit_rows = Vec::new();
    let mut day_rows = Vec::new();
    for (fips, u) in &state.units {
        if u.months.is_empty() {
            continue;
        }
        let code = format!("{fips:05}");
        for r in &u.records {
            let mut row = vec![
                code.clone(),
                r.start.to_string(),
                r.end.to_string(),
                r.peak_t.to_string(),
                sig4(r.peak),
                sig4(r.arrivals),
                sig4(r.customer_hours),
            ];
            row.extend(r.ge.iter().map(|x| sig4(*x)));
            row.extend(r.curve.iter().map(|x| sig4(*x)));
            ev_rows.push(row);
        }
        let mut by_year: BTreeMap<u32, u32> = BTreeMap::new();
        for m in &u.months {
            *by_year.entry((m - 1) / 12).or_default() += 1;
        }
        unit_rows.push(vec![
            code.clone(),
            sig4(u.customers),
            by_year
                .iter()
                .map(|(y, n)| format!("{y}:{n}"))
                .collect::<Vec<_>>()
                .join(" "),
            u.dropouts_bridged.to_string(),
        ]);
        for (d, h) in &u.daily {
            day_rows.push(vec![code.clone(), d.to_string(), sig4(*h)]);
        }
    }
    let b1 = crate::intermediate::write(
        &ctx.data,
        crate::intermediate::OUTAGE_EVENTS,
        EVENT_COLUMNS,
        &ev_rows,
    )?;
    let b2 = crate::intermediate::write(
        &ctx.data,
        crate::intermediate::OUTAGE_UNITS,
        &["unit", "customers", "months_by_year", "dropouts_bridged"],
        &unit_rows,
    )?;
    let b3 = crate::intermediate::write(
        &ctx.data,
        crate::intermediate::OUTAGE_DAILY,
        &["unit", "day", "customer_hours"],
        &day_rows,
    )?;
    eprintln!(
        "  intermediates: {} events ({:.1} MB), {} units, {} unit-days ({:.1} MB)",
        ev_rows.len(),
        b1 as f64 / 1e6,
        unit_rows.len(),
        day_rows.len(),
        (b2 + b3) as f64 / 1e6
    );
    Ok(())
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let cw = Crosswalk::load(&ctx.data)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();

    let article = ctx.http.get(ARTICLE, Some("outages/article.json"))?;
    let meta: serde_json::Value = serde_json::from_slice(&article.bytes)?;
    let version = format!(
        "figshare version {} published {} ({})",
        meta["version"],
        meta["published_date"].as_str().unwrap_or("?"),
        meta["doi"].as_str().unwrap_or("?")
    );
    let license = meta["license"]["name"]
        .as_str()
        .unwrap_or("CC BY 4.0")
        .to_string();
    if !license.contains("CC BY") {
        return Err(data_err(format!(
            "EAGLE-I licence is now {license}; review before redistributing derived data"
        )));
    }
    out.source(super::source_from(
        "ORNL EAGLE-I recorded outages: figshare article metadata",
        &article,
        version.clone(),
        &license,
        "",
    ));
    let files: Vec<(String, String)> = meta["files"]
        .as_array()
        .ok_or_else(|| data_err("EAGLE-I article has no files list"))?
        .iter()
        .filter_map(|f| {
            Some((
                f["name"].as_str()?.to_string(),
                f["download_url"].as_str()?.to_string(),
            ))
        })
        .collect();
    let url_of = |name: &str| {
        files
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, u)| u.clone())
    };

    // Modelled customers per county.
    let mcc_url = url_of("MCC.csv").ok_or_else(|| data_err("EAGLE-I: MCC.csv not in article"))?;
    let mcc_doc = ctx.http.get(&mcc_url, Some("outages/MCC.csv"))?;
    out.source(super::source_from(
        "ORNL EAGLE-I modelled county customers (MCC.csv)",
        &mcc_doc,
        version.clone(),
        &license,
        "Credit ORNL EAGLE-I (CC BY 4.0).",
    ));
    let (mh, mrows) = parse_delimited(&mcc_doc.text(), b',')?;
    let (mi_f, mi_c) = (col(&mh, "County_FIPS")?, col(&mh, "Customers")?);
    let mut mcc: BTreeMap<u32, f64> = BTreeMap::new();
    for r in &mrows {
        if let (Ok(f), Ok(c)) = (r[mi_f].parse::<u32>(), r[mi_c].parse::<f64>()) {
            mcc.insert(f, c);
        }
    }

    // Households: every household is at least one electricity customer, so the customer count is
    // the larger of MCC and households (MCC is far below the household count in some counties).
    let svi = ctx
        .http
        .get(SVI_COUNTY, Some("outages/SVI_2022_US_county.csv"))?;
    out.source(super::source_from(
        "CDC/ATSDR Social Vulnerability Index 2022, counties (households E_HH)",
        &svi,
        "SVI 2022",
        super::PUBLIC_DOMAIN,
        "Cite CDC/ATSDR SVI 2022.",
    ));
    let (sh, srows) = parse_delimited(&svi.text(), b',')?;
    let (si_f, si_h) = (col(&sh, "FIPS")?, col(&sh, "E_HH")?);
    let mut households: BTreeMap<String, f64> = BTreeMap::new();
    for r in &srows {
        if let Ok(h) = r[si_h].parse::<f64>()
            && h > 0.0
        {
            households.insert(r[si_f].clone(), h);
        }
    }
    // Old Connecticut counties: households apportioned from the planning regions by land share.
    for o in &cw.overlaps {
        if let Some(h) = households.get(&o.region).copied() {
            *households.entry(o.old.clone()).or_default() += h * o.share_of_region;
        }
    }
    let mut raised = Vec::new();
    for (f, c) in mcc.iter_mut() {
        if let Some(h) = households.get(&format!("{f:05}")).copied()
            && h > *c
        {
            raised.push((format!("{f:05}"), *c, h));
            *c = h;
        }
    }
    for (code, h) in &households {
        if let Ok(f) = code.parse::<u32>() {
            mcc.entry(f).or_insert(*h);
        }
    }
    raised.sort_by(|a, b| (a.1 / a.2).total_cmp(&(b.1 / b.2)));
    out.notes.push(format!(
        "Customer counts: ORNL's modelled customers (MCC.csv) are below the county's household count in {} counties, sometimes far below (e.g. {}), which would inflate per-customer rates; the customer count used is the larger of MCC and households (CDC SVI 2022, ACS 2018-2022). Thresholds scale with it.",
        raised.len(),
        raised.iter().take(5).map(|(f, c, h)| format!("{f}: MCC {c:.0} vs {h:.0} households")).collect::<Vec<_>>().join("; ")
    ));

    // State coverage 2018-2022.
    let cov_url = url_of("coverage_history.csv")
        .ok_or_else(|| data_err("EAGLE-I: coverage_history.csv not in article"))?;
    let cov_doc = ctx
        .http
        .get(&cov_url, Some("outages/coverage_history.csv"))?;
    out.source(super::source_from(
        "ORNL EAGLE-I state coverage history",
        &cov_doc,
        version.clone(),
        &license,
        "Credit ORNL EAGLE-I (CC BY 4.0).",
    ));
    let (ch, crows) = parse_delimited(&cov_doc.text(), b',')?;
    let (ci_y, ci_s, ci_max) = (
        col(&ch, "year")?,
        col(&ch, "state")?,
        col(&ch, "max_pct_covered")?,
    );
    let mut coverage: BTreeMap<(String, u16), f64> = BTreeMap::new();
    for r in &crows {
        let year = r[ci_y]
            .rsplit('/')
            .next()
            .and_then(|y| y.parse::<u16>().ok())
            .map(|y| if y < 100 { 2000 + y } else { y });
        if let (Some(y), Ok(c)) = (year, r[ci_max].parse::<f64>()) {
            coverage.insert((r[ci_s].clone(), y), c);
        }
    }

    let abbr_by_fips: BTreeMap<u32, &str> = crate::jobs::geography::STATE_FACTS
        .iter()
        .filter_map(|(f, a, _, _)| Some((f.parse::<u32>().ok()?, *a)))
        .collect();
    let state_of = |fips: u32| {
        abbr_by_fips
            .get(&(fips / 1000))
            .map(|s| s.to_string())
            .unwrap_or_default()
    };

    // Longitudes for local calendar days: the county's internal point; codes not in the county
    // list (Connecticut's old counties, renamed Alaska areas) take their state's mean.
    let mut lon_by_unit: BTreeMap<u32, f64> = BTreeMap::new();
    let mut state_lon: BTreeMap<u32, (f64, f64)> = BTreeMap::new();
    for c in &counties {
        if let Ok(f) = c.fips.parse::<u32>() {
            lon_by_unit.insert(f, c.lon);
            let e = state_lon.entry(f / 1000).or_default();
            e.0 += c.lon;
            e.1 += 1.0;
        }
    }
    let lon_of = |f: u32| -> f64 {
        if f == PR_ISLAND {
            return -66.4;
        }
        lon_by_unit.get(&f).copied().unwrap_or_else(|| {
            state_lon
                .get(&(f / 1000))
                .map(|(s, n)| s / n)
                .unwrap_or(-90.0)
        })
    };

    // Years to process (all, unless RR_ETL_EAGLEI_YEARS limits them for a development run).
    let mut years: Vec<u16> = files
        .iter()
        .filter_map(|(n, _)| {
            n.strip_prefix("eaglei_outages_")?
                .strip_suffix(".csv")?
                .parse::<u16>()
                .ok()
        })
        .collect();
    years.sort_unstable();
    let dev_years = std::env::var("RR_ETL_EAGLEI_YEARS").ok();
    if let Some(spec) = &dev_years {
        let (a, b) = spec.split_once('-').unwrap_or((spec, spec));
        let (a, b): (u16, u16) = (a.parse().unwrap_or(0), b.parse().unwrap_or(9999));
        years.retain(|y| *y >= a && *y <= b);
        out.notes.push(format!("DEVELOPMENT RUN: RR_ETL_EAGLEI_YEARS={spec} limited the years to {years:?}. Do not ship."));
        eprintln!("  development run: years {years:?}");
    }
    let mut state = State::default();
    let mut diags = Vec::new();
    for y in &years {
        let name = format!("eaglei_outages_{y}.csv");
        let url =
            url_of(&name).ok_or_else(|| data_err(format!("EAGLE-I: {name} not in article")))?;
        eprintln!("  streaming {name}");
        let snapshot = state.clone();
        let (diag, streamed) = ctx.http.stream(&url, None, |reader| {
            state = snapshot.clone();
            process_year(reader, &mut state, &mcc, &state_of, &coverage, &lon_of)
        })?;
        eprintln!(
            "    {} rows, {} snapshots, {} collection gaps over 1 h ({:.0} h), {:.2} GB",
            diag.rows,
            diag.timestamps,
            diag.gaps_over_1h,
            diag.gap_hours,
            streamed.bytes as f64 / 1e9
        );
        out.rows_in += diag.rows;
        out.source(SourceRecord {
            name: format!("ORNL EAGLE-I recorded outages {y}"),
            url: streamed.final_url.clone(),
            version: version.clone(),
            retrieved: streamed.retrieved.clone(),
            sha256: streamed.sha256.clone(),
            bytes: streamed.bytes,
            license: license.clone(),
            obligations: "Credit ORNL EAGLE-I (CC BY 4.0); streamed, never stored.".into(),
        });
        diags.push((*y, diag));
    }
    for u in state.units.values_mut() {
        u.close();
    }
    write_intermediates(ctx, &state)?;

    // Map data units to canonical counties.
    let mut by_county: BTreeMap<String, Vec<(u32, f64)>> = BTreeMap::new(); // county -> (unit fips, weight)
    let mut unknown = BTreeSet::new();
    for fips in state.units.keys() {
        let code = format!("{fips:05}");
        if *fips == PR_ISLAND {
            for c in counties.iter().filter(|c| c.fips.starts_with("72")) {
                by_county
                    .entry(c.fips.clone())
                    .or_default()
                    .push((*fips, 1.0));
            }
        } else if canon.contains(&code) {
            by_county.entry(code).or_default().push((*fips, 1.0));
        } else if is_old_ct(&code) {
            for o in cw.overlaps.iter().filter(|o| o.old == code) {
                by_county
                    .entry(o.region.clone())
                    .or_default()
                    .push((*fips, o.share_of_region));
            }
        } else if let Some(new) = crate::ct::successors(&code) {
            for n in new {
                by_county
                    .entry(n.to_string())
                    .or_default()
                    .push((*fips, 1.0));
            }
        } else {
            unknown.insert(code);
        }
    }

    // State pools (customer-weighted by construction).
    let mut pools: BTreeMap<String, (Unit, u32)> = BTreeMap::new();
    for (fips, u) in &state.units {
        if u.months.is_empty() {
            continue;
        }
        let st = state_of(*fips);
        let e = pools.entry(st).or_default();
        for (k, w) in &u.durations {
            *e.0.durations.entry(*k).or_default() += w;
        }
        e.0.arrivals += u.arrivals;
        e.0.customers += u.customers * (u.months.len() as f64 / 12.0); // customer-years
        e.0.events += u.events;
        e.1 += 1;
    }
    let mut st_table = Table::new(
        &[
            "state_abbr",
            "events_per_customer_year",
            "p_ge_1d",
            "p_ge_3d",
            "p_ge_7d",
            "p_ge_14d",
            "p_ge_30d",
            "median_hours",
            "p90_hours",
            "units_with_data",
            "events",
        ],
        1,
    );
    let mut pool_dist: BTreeMap<String, Dist> = BTreeMap::new();
    for (st, (u, n)) in &pools {
        let Some(d) = summarise(&u.durations) else {
            continue;
        };
        let epcy = if u.customers > 0.0 {
            u.arrivals / u.customers
        } else {
            0.0
        };
        st_table.push(vec![
            st.clone(),
            sig4(epcy),
            sig4(d.p1),
            sig4(d.p3),
            sig4(d.p7),
            sig4(d.p14),
            sig4(d.p30),
            sig4(d.median_h),
            sig4(d.p90_h),
            n.to_string(),
            u.events.to_string(),
        ]);
        pool_dist.insert(st.clone(), d);
    }

    let mut table = Table::new(
        &[
            "fips",
            "events_per_customer_year",
            "p_ge_1d",
            "p_ge_3d",
            "p_ge_7d",
            "p_ge_14d",
            "p_ge_30d",
            "median_hours",
            "p90_hours",
            "years_covered",
            "years_of_data",
            "duration_basis",
            "events_per_year",
            "events",
            "customers",
            "customer_hours_per_customer_year",
            "share_customer_hours_in_events",
            "longest_event_hours",
        ],
        1,
    );
    let mut covered = BTreeSet::new();
    let mut state_basis = 0;
    for (county, parts) in &by_county {
        // Per-unit metrics, then weighted average across units (only CT and legacy codes have
        // more than one unit or fractional weights).
        let mut acc: BTreeMap<&str, f64> = BTreeMap::new();
        let mut wsum = 0.0;
        let mut years_all: BTreeSet<u16> = BTreeSet::new();
        let mut events_total = 0.0;
        let mut customers_total = 0.0;
        let mut longest: i64 = 0;
        let mut merged: BTreeMap<u32, f64> = BTreeMap::new();
        let mut n_events = 0u32;
        for (fips, w) in parts {
            let u = &state.units[fips];
            if u.months.is_empty() {
                continue;
            }
            let yrs = u.months.len() as f64 / 12.0;
            let add = |acc: &mut BTreeMap<&str, f64>, k: &'static str, v: f64| {
                *acc.entry(k).or_default() += w * v
            };
            add(&mut acc, "epcy", u.arrivals / u.customers / yrs);
            add(&mut acc, "epy", u.events as f64 / yrs);
            add(&mut acc, "yrs", yrs);
            add(&mut acc, "saidi", u.cust_hours_all / u.customers / yrs);
            add(
                &mut acc,
                "share",
                if u.cust_hours_all > 0.0 {
                    u.cust_hours_events / u.cust_hours_all
                } else {
                    0.0
                },
            );
            wsum += w;
            years_all.extend(u.years.iter().copied());
            events_total += w * u.events as f64;
            customers_total += w * u.customers;
            longest = longest.max(u.max_event_s);
            n_events += u.events;
            for (k, c) in &u.durations {
                *merged.entry(*k).or_default() += w * c;
            }
        }
        if wsum <= 0.0 {
            continue;
        }
        let get = |k: &str| acc.get(k).copied().unwrap_or(0.0) / wsum;
        let st = counties
            .iter()
            .find(|c| &c.fips == county)
            .map(|c| c.state_abbr.clone())
            .unwrap_or_default();
        let (dist, basis) = match (
            n_events >= MIN_EVENTS_FOR_COUNTY_DURATIONS,
            summarise(&merged),
            pool_dist.get(&st),
        ) {
            (true, Some(d), _) => (Some(d), "county"),
            (_, _, Some(p)) => {
                state_basis += 1;
                (Some(p.clone()), "state")
            }
            (_, Some(d), None) => (Some(d), "county"),
            _ => (None, ""),
        };
        let Some(d) = dist else { continue };
        let island = parts.iter().any(|(f, _)| *f == PR_ISLAND);
        let basis = if island { "island" } else { basis };
        // Island rows carry the island's rates, but `customers` is the municipio's own count so
        // that weighting rows by customers does not count the island 78 times.
        let customers_col = if island {
            county
                .parse::<u32>()
                .ok()
                .and_then(|f| mcc.get(&f).copied())
                .unwrap_or(0.0)
        } else {
            customers_total
        };
        let years_covered = match (years_all.first(), years_all.last()) {
            (Some(a), Some(b)) => format!("{a}-{b}"),
            _ => String::new(),
        };
        table.push(vec![
            county.clone(),
            sig4(get("epcy")),
            sig4(d.p1),
            sig4(d.p3),
            sig4(d.p7),
            sig4(d.p14),
            sig4(d.p30),
            sig4(d.median_h),
            sig4(d.p90_h),
            years_covered,
            sig4(get("yrs")),
            basis.to_string(),
            sig4(get("epy")),
            sig4(events_total),
            sig4(customers_col),
            sig4(get("saidi")),
            sig4(get("share")),
            sig4(longest as f64 / 3600.0),
        ]);
        covered.insert(county.clone());
    }
    out.table(ctx, OUTAGES, &mut table)?;
    out.table(ctx, OUTAGES_STATE, &mut st_table)?;

    out.missing = missing_groups(&counties, &covered, |c| {
        if super::is_island_territory(&c.state_abbr) {
            "EAGLE-I does not cover this island area".to_string()
        } else if c.state_abbr == "PR" {
            "No usable EAGLE-I records for Puerto Rico".to_string()
        } else {
            "No usable EAGLE-I records for this county (utility not tracked, or no modelled customer count)".to_string()
        }
    });
    out.definitions
        .insert("event_definition".into(), DEFINITION.into());
    out.notes.push(format!(
        "Event definition: {DEFINITION} Counties with fewer than {MIN_EVENTS_FOR_COUNTY_DURATIONS} events use their state's pooled duration distribution (duration_basis = state; {state_basis} counties); the event rate is always the county's own."
    ));
    out.notes.push("Puerto Rico: EAGLE-I files LUMA's outages by utility region, each under one hub municipio (the hubs' customer counts in the 2024 file sum to the island's 1.49 million). The hubs are summed into one island-wide series (customers = the sum of ORNL's modelled customers for all 78 municipios), and every municipio carries the island's rates, durations and event counts (duration_basis = island); its customers column is its own modelled customer count, so weighting rows by customers counts the island once. Data start in 2021.".into());
    out.notes.push("Years of data: a month counts when the county has at least one record in it (EAGLE-I lists only snapshots with customers out), so years_of_data is months with data divided by 12. State-years in 2018-2022 where ORNL reports under 50% customer coverage are left out; ORNL publishes no coverage figures for other years, so partial utility coverage there biases rates low.".into());
    out.notes.push("Customer outages at the start of an event are counted from the event's start (they may have begun below the threshold) and customers still out when the county drops below the threshold are counted as restored then, so durations for the first and last customers in an event are slightly understated.".into());
    if !state.excluded_years.is_empty() {
        let list: Vec<String> = state
            .excluded_years
            .iter()
            .map(|(s, y)| format!("{s} {y}"))
            .collect();
        out.notes
            .push(format!("Excluded for low coverage: {}.", list.join(", ")));
    }
    let oo: u64 = state.units.values().map(|u| u.out_of_order).sum();
    let dups: u64 = state.units.values().map(|u| u.duplicates).sum();
    out.notes.push(format!("Data checks: {oo} out-of-order rows skipped, {dups} duplicate snapshots merged (maximum kept)."));
    for (y, d) in &diags {
        out.notes.push(format!(
            "{y}: {} rows, {} distinct snapshots, {} collection gaps longer than 1 hour totalling {:.0} hours.",
            d.rows, d.timestamps, d.gaps_over_1h, d.gap_hours
        ));
    }
    if !unknown.is_empty() {
        out.notes.push(format!("County codes in EAGLE-I that are not in the 2024 county list and have no known successor (dropped): {}.", unknown.into_iter().collect::<Vec<_>>().join(", ")));
    }
    out.attributions.push(Attribution {
        source: "ORNL EAGLE-I".into(),
        text: "Power outage statistics derived by Ready Reckoner from: Brelsford, C., Tennille, S., Myers, A., et al., The Environment for Analysis of Geo-Located Energy Information's Recorded Electricity Outages 2014-2025, Oak Ridge National Laboratory, figshare, doi:10.6084/m9.figshare.24237376 (CC BY 4.0).".into(),
        license,
        url: DOI.into(),
        version: Some(version),
        accessed: article.retrieved[..10].to_string(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(customers: f64) -> Unit {
        Unit::new(customers, 0)
    }

    #[test]
    fn fifo_conserves_customer_hours() {
        // 1,000 customers: 100 out for 4 slots, then 50 for 4 more, then 0.
        let mut u = unit(1000.0);
        for i in 0..4 {
            u.row(i * SLOT_S, 100.0);
        }
        for i in 4..8 {
            u.row(i * SLOT_S, 50.0);
        }
        u.row(20 * SLOT_S, 5.0); // below threshold, far later: closes the event
        u.close();
        assert_eq!(u.events, 1);
        let total_slots: f64 = u.durations.iter().map(|(k, w)| *k as f64 * w).sum();
        // customer-slots inside the event = 100*4 + 50*4 = 600
        assert!((total_slots - 600.0).abs() < 1e-9, "{total_slots}");
        // 50 customers out 4 slots (restored first), 50 out 8 slots
        assert_eq!(u.durations.get(&4), Some(&50.0));
        assert_eq!(u.durations.get(&8), Some(&50.0));
        assert!((u.arrivals - 100.0).abs() < 1e-9);
    }

    #[test]
    fn short_or_small_outages_are_not_events() {
        let mut u = unit(1000.0);
        // 3 slots (45 minutes) above threshold: shorter than one hour.
        for i in 0..3 {
            u.row(i * SLOT_S, 500.0);
        }
        u.row(100 * SLOT_S, 1.0);
        u.close();
        assert_eq!(u.events, 0);
        // Below the 1% threshold for a long time: no event.
        let mut u = unit(100_000.0);
        for i in 0..40 {
            u.row(i * SLOT_S, 900.0);
        }
        u.close();
        assert_eq!(u.events, 0);
    }

    #[test]
    fn gaps_up_to_two_hours_are_bridged() {
        let mut u = unit(1000.0);
        for i in 0..4 {
            u.row(i * SLOT_S, 100.0);
        }
        // Missing snapshots for 1.75 hours, then back above threshold.
        for i in 11..15 {
            u.row(i * SLOT_S, 100.0);
        }
        u.close();
        assert_eq!(u.events, 1);
        // Carry-forward: 100 customers out for all 15 slots.
        assert_eq!(u.durations.get(&15), Some(&100.0));
        // A gap longer than 2 hours after a small count (under the 1% start level) splits
        // events: the outage was over.
        let mut u = unit(1000.0);
        for i in 0..4 {
            u.row(i * SLOT_S, 100.0);
        }
        for i in 4..8 {
            u.row(i * SLOT_S, 8.0);
        }
        for i in 18..22 {
            u.row(i * SLOT_S, 100.0);
        }
        u.close();
        assert_eq!(u.events, 2);
    }

    #[test]
    fn reporting_dropouts_are_bridged_up_to_three_days() {
        // 1,000 customers, 200 out; the data vanish for 6 hours and come back at the same level:
        // a utility missing from the scrape, not a restoration.
        let mut u = unit(1000.0);
        for i in 0..8 {
            u.row(i * SLOT_S, 200.0);
        }
        for i in 32..40 {
            u.row(i * SLOT_S, 200.0);
        }
        u.close();
        assert_eq!(u.events, 1);
        assert_eq!(u.dropouts_bridged, 1);
        assert_eq!(u.records[0].arrivals, 200.0);
        // Coming back at under half the level: the outage ended while the data were missing.
        let mut u = unit(1000.0);
        for i in 0..8 {
            u.row(i * SLOT_S, 200.0);
        }
        for i in 32..40 {
            u.row(i * SLOT_S, 60.0);
        }
        u.close();
        assert_eq!(u.events, 2);
        // Two and a half days: still a dropout.
        let mut u = unit(1000.0);
        for i in 0..8 {
            u.row(i * SLOT_S, 200.0);
        }
        for i in 250..258 {
            u.row(i * SLOT_S, 200.0);
        }
        u.close();
        assert_eq!(u.events, 1);
        // Longer than three days: two events.
        let mut u = unit(1000.0);
        for i in 0..8 {
            u.row(i * SLOT_S, 200.0);
        }
        for i in 300..308 {
            u.row(i * SLOT_S, 200.0);
        }
        u.close();
        assert_eq!(u.events, 2);
    }

    #[test]
    fn closing_fills_dips_and_keeps_restorations() {
        // A restoration curve is left alone.
        let down: Vec<f64> = (0..300).map(|i| 1000.0 - 3.0 * i as f64).collect();
        assert_eq!(closing(&down, DROPOUT_CLOSE_HALF), down);
        let up: Vec<f64> = (0..300).map(|i| 3.0 * i as f64).collect();
        assert_eq!(closing(&up, DROPOUT_CLOSE_HALF), up);
        // A 2-hour dip on a plateau is filled; a dip longer than a day is not.
        let mut v = vec![500.0; 200];
        for x in v.iter_mut().skip(100).take(8) {
            *x = 20.0;
        }
        assert!(closing(&v, DROPOUT_CLOSE_HALF).iter().all(|x| *x == 500.0));
        let mut v = vec![500.0; 400];
        for x in v.iter_mut().skip(100).take(150) {
            *x = 20.0;
        }
        assert_eq!(closing(&v, DROPOUT_CLOSE_HALF)[175], 20.0);
    }

    #[test]
    fn running_median_window() {
        let v = [1.0, 9.0, 2.0, 8.0, 3.0];
        assert_eq!(running_median(&v, 1), vec![5.0, 2.0, 8.0, 3.0, 5.5]);
        assert_eq!(running_median(&v, 0), v.to_vec());
    }

    /// Hurricane Helene in Buncombe County, North Carolina, as EAGLE-I recorded it: about 116,000
    /// customers out, falling to about 80,000 after a week, 25,000 after two weeks and a few
    /// hundred after three; one utility missing from a scrape every hour or two (the count drops
    /// to about 9,700 for 15 to 45 minutes) and the occasional report counted twice.
    fn helene_like() -> Vec<f64> {
        let mut v = Vec::new();
        for i in 0..(21 * 96) {
            let d = i as f64 / 96.0;
            let level = if d < 7.0 {
                116_000.0 - (116_000.0 - 80_000.0) * d / 7.0
            } else if d < 14.0 {
                80_000.0 - (80_000.0 - 25_000.0) * (d - 7.0) / 7.0
            } else {
                25_000.0 - (25_000.0 - 400.0) * (d - 14.0) / 7.0
            };
            let x = if i % 6 == 3 || (i % 11 == 5 && i < 800) {
                (0.084 * level).min(9_700.0) // a utility missing from the scrape
            } else if i % 37 == 20 {
                level * 1.95 // counted twice
            } else {
                level
            };
            v.push(x);
        }
        v
    }

    #[test]
    fn helene_like_series_keeps_its_multi_day_tail() {
        let customers = 181_010.0;
        let threshold = THRESHOLD_SHARE * customers;
        let raw = helene_like();
        // Unrepaired, the dropouts chop the outage into pieces: tens of "outages" per customer
        // and nothing lasting a week.
        let (d_raw, a_raw) = fifo(&zigzag(&raw, threshold, REVERSAL_SHARE));
        assert!(a_raw > 20.0 * 116_000.0, "{a_raw}");
        assert_eq!(d_raw.range(7 * 96..).map(|(_, w)| *w).sum::<f64>(), 0.0);
        // Repaired: one outage per affected customer, and the week-long tail is back.
        let fixed = repair(&raw, threshold);
        let (d, a) = fifo(&zigzag(&fixed, threshold, REVERSAL_SHARE));
        assert!((a - 116_000.0).abs() < 1_000.0, "{a}");
        let share = |days: u32| d.range(days * 96..).map(|(_, w)| *w).sum::<f64>() / a;
        assert!(share(7) > 0.6 && share(7) < 0.75, "{}", share(7));
        assert!(share(14) > 0.15 && share(14) < 0.3, "{}", share(14));
        assert!(fixed.iter().cloned().fold(0.0, f64::max) < 117_000.0);
        // Through the unit: the event record carries the curve from the peak.
        let mut u = unit(customers);
        for (i, x) in raw.iter().enumerate() {
            u.row(i as i64 * SLOT_S, *x);
        }
        u.row(30 * 96 * SLOT_S, 1.0);
        u.close();
        assert_eq!(u.events, 1);
        let r = &u.records[0];
        let c7 = r.curve[CURVE_DAYS.iter().position(|d| *d == 7).unwrap()];
        let c14 = r.curve[CURVE_DAYS.iter().position(|d| *d == 14).unwrap()];
        assert!((c7 - 80_000.0 / 116_000.0).abs() < 0.02, "{c7}");
        assert!((c14 - 25_000.0 / 116_000.0).abs() < 0.02, "{c14}");
        assert!(r.ge[2] > 0.6 * r.arrivals);
    }

    #[test]
    fn zigzag_removes_flicker_but_keeps_waves() {
        // Flicker of 10% during a restoration is removed; the fill is monotone.
        let v = [0.0, 100.0, 90.0, 99.0, 80.0, 88.0, 50.0, 55.0, 20.0, 0.0];
        let f = zigzag(&v, 5.0, 0.5);
        assert_eq!(
            f,
            vec![0.0, 100.0, 90.0, 90.0, 80.0, 80.0, 50.0, 50.0, 20.0, 0.0]
        );
        // A genuine second storm (trough 10 -> 90) is kept as a new rise.
        let v = [0.0, 100.0, 40.0, 10.0, 90.0, 30.0, 0.0];
        let f = zigzag(&v, 5.0, 0.5);
        assert_eq!(f, v.to_vec());
    }

    #[test]
    fn hysteresis_keeps_the_slow_tail() {
        // 1,000 customers: start at 1% (10), end below 0.25% (5, the minimum).
        let mut u = unit(1000.0);
        for i in 0..8 {
            u.row(i * SLOT_S, 100.0);
        }
        // Tail between the end and start thresholds for 4 more hours.
        for i in 8..24 {
            u.row(i * SLOT_S, 8.0);
        }
        u.row(40 * SLOT_S, 1.0);
        u.close();
        assert_eq!(u.events, 1);
        // 92 customers restored at slot 8, 8 customers at slot 24.
        assert_eq!(u.durations.get(&8), Some(&92.0));
        assert_eq!(u.durations.get(&24), Some(&8.0));
    }

    #[test]
    fn puerto_rico_hubs_are_summed_into_one_island_series() {
        let mut mcc = BTreeMap::new();
        mcc.insert(72001u32, 600.0);
        mcc.insert(72003u32, 400.0);
        let mut csv = String::from("fips_code,county,state,customers_out,run_start_time\n");
        for q in 0..5 {
            let t = format!("2024-01-01 {:02}:{:02}:00", q / 4, (q % 4) * 15);
            csv.push_str(&format!(
                "72013,Arecibo,Puerto Rico,30,{t}\n72021,Bayamon,Puerto Rico,20,{t}\n"
            ));
        }
        let mut state = State::default();
        let state_of = |f: u32| {
            if f / 1000 == 72 {
                "PR".to_string()
            } else {
                String::new()
            }
        };
        process_year(
            &mut csv.as_bytes(),
            &mut state,
            &mcc,
            &state_of,
            &BTreeMap::new(),
            &|_| -66.4,
        )
        .unwrap();
        assert_eq!(
            state.units.len(),
            1,
            "hubs do not become units of their own"
        );
        let island = state.units.get_mut(&PR_ISLAND).unwrap();
        assert_eq!(island.customers, 1000.0);
        island.close();
        assert_eq!(island.events, 1);
        // 30 + 20 customers out at every snapshot, all restored together at the end.
        assert!((island.arrivals - 50.0).abs() < 1e-9);
        assert_eq!(island.durations.get(&5), Some(&50.0));
    }

    #[test]
    fn distribution_summary() {
        let mut d = BTreeMap::new();
        d.insert(4u32, 90.0); // one hour
        d.insert(96 * 3, 10.0); // three days
        let s = summarise(&d).unwrap();
        assert!((s.p1 - 0.1).abs() < 1e-12 && (s.p3 - 0.1).abs() < 1e-12 && s.p7 == 0.0);
        assert_eq!(s.median_h, 1.0);
        assert_eq!(s.p90_h, 1.0);
    }
}
