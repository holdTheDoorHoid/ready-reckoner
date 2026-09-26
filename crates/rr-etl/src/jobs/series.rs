//! Job `series` — national and regional series for the societal hazards, as small TOML tables
//! under `core/series/` (one file per source).
//!
//! Every file has the same shape: a header (`id`, `title`, `publisher`, `url`, `licence`, the
//! `source` citation id, `retrieved`, `how` = `fetched` or `transcribed`, `notes`), `[[rate]]`
//! entries that `rr-hazards` reads (value, unit, optional range, the figure it came from, the
//! arithmetic, an `unverified` flag for hand-copied figures not yet checked against a primary
//! source), and `[[row]]` entries holding the table the rates come from, for the Learn pages.
//!
//! - `oe417.toml`: DOE OE-417 electric disturbance reports 2014-2023 by cause group and year,
//!   and by state, from PNNL's Event-correlated Outage Dataset in America (CC BY 4.0: the credit
//!   line is recorded as an attribution). The job also hands every report to `outage_model`.
//! - `drug_shortages.toml`: openFDA drug shortages (CC0), counts by status and dosage form on
//!   the day of the refresh.
//! - `fdic_failures.toml`: FDIC failed banks since 1934, failures and failed assets per year.
//! - `funding_gaps.toml`, `fcc_dirs.toml`: hand-copied (see [`super::series_transcribed`]).
//! - `fbi_arrests.toml`: FBI arrests per 100,000 by sex and age (see [`super::series_arrests`]).

use super::{Ctx, JobOutput};
use crate::intermediate as im;
use crate::manifest::Attribution;
use crate::num::sig4;
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

/// DOE OE-417 series.
pub const OE417: &str = "core/series/oe417.toml";
/// openFDA drug shortages.
pub const DRUG_SHORTAGES: &str = "core/series/drug_shortages.toml";
/// FDIC failed banks.
pub const FDIC: &str = "core/series/fdic_failures.toml";

const PNNL_ZIP: &str = "https://data.openei.org/files/6458/Outage_Dataset_R1.zip";
const PNNL_PAGE: &str = "https://data.openei.org/submissions/6458";
const OPENFDA_ZIP: &str =
    "https://download.open.fda.gov/drug/shortages/drug-shortages-0001-of-0001.json.zip";
const FDIC_API: &str = "https://api.fdic.gov/banks/failures?limit=10000&fields=FAILDATE,FAILYR,QBFASSET,RESTYPE&format=csv";

/// Years of OE-417 reports used for the rates (reporting was far less complete before 2019:
/// 32 reports in 2014 against 318 in 2019).
pub const OE417_RATE_YEARS: (u16, u16) = (2019, 2023);

/// A value in a series row.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    /// A number (written with 4 significant figures).
    N(f64),
    /// An integer.
    I(i64),
    /// Text.
    S(String),
    /// A flag.
    B(bool),
}

impl Val {
    fn render(&self) -> String {
        match self {
            Val::N(x) => sig4(*x),
            Val::I(i) => i.to_string(),
            Val::S(s) => toml_str(s),
            Val::B(b) => b.to_string(),
        }
    }
}

/// One `[[rate]]` entry.
#[derive(Debug, Clone, Default)]
pub struct SRate {
    /// Stable id `rr-hazards` asks for.
    pub id: String,
    /// The rate.
    pub value: f64,
    /// Unit in words.
    pub unit: String,
    /// Low end of a plausible range.
    pub low: Option<f64>,
    /// High end of a plausible range.
    pub high: Option<f64>,
    /// Last year the rate describes.
    pub year: u16,
    /// Years the rate is computed over, e.g. `2019-2023`.
    pub period: String,
    /// The published figure (or count) it comes from.
    pub figure: String,
    /// The arithmetic.
    pub derivation: String,
    /// Caveats.
    pub note: String,
    /// True when a hand-copied figure has not been checked against a primary source.
    pub unverified: bool,
}

/// A series file being built.
#[derive(Debug, Clone, Default)]
pub struct SeriesDoc {
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
    pub attribution: String,
    /// Citation id for `content/citations.toml`.
    pub source: String,
    /// When read (`YYYY-MM-DD`), or `each refresh`.
    pub retrieved: String,
    /// `fetched` or `transcribed`.
    pub how: String,
    /// Notes.
    pub notes: Vec<String>,
    /// Rates.
    pub rates: Vec<SRate>,
    /// The table.
    pub rows: Vec<Vec<(String, Val)>>,
}

/// TOML string literal.
pub fn toml_str(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

impl SeriesDoc {
    /// Render deterministically.
    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "# {}. Written by rr-etl (job `series`); see docs/DATA_SOURCES.md.\n\n",
            self.title
        ));
        for (k, v) in [
            ("id", &self.id),
            ("title", &self.title),
            ("publisher", &self.publisher),
            ("url", &self.url),
            ("licence", &self.licence),
            ("attribution", &self.attribution),
            ("source", &self.source),
            ("retrieved", &self.retrieved),
            ("how", &self.how),
        ] {
            s.push_str(&format!("{k} = {}\n", toml_str(v)));
        }
        s.push_str("notes = [\n");
        for n in &self.notes {
            s.push_str(&format!("  {},\n", toml_str(n)));
        }
        s.push_str("]\n\n");
        for r in &self.rates {
            s.push_str("[[rate]]\n");
            s.push_str(&format!("id = {}\n", toml_str(&r.id)));
            s.push_str(&format!("value = {}\n", sig4(r.value)));
            s.push_str(&format!("unit = {}\n", toml_str(&r.unit)));
            if let Some(l) = r.low {
                s.push_str(&format!("low = {}\n", sig4(l)));
            }
            if let Some(h) = r.high {
                s.push_str(&format!("high = {}\n", sig4(h)));
            }
            s.push_str(&format!("year = {}\n", r.year));
            s.push_str(&format!("period = {}\n", toml_str(&r.period)));
            s.push_str(&format!("figure = {}\n", toml_str(&r.figure)));
            s.push_str(&format!("derivation = {}\n", toml_str(&r.derivation)));
            s.push_str(&format!("note = {}\n", toml_str(&r.note)));
            s.push_str(&format!("unverified = {}\n\n", r.unverified));
        }
        for row in &self.rows {
            s.push_str("[[row]]\n");
            for (k, v) in row {
                s.push_str(&format!("{k} = {}\n", v.render()));
            }
            s.push('\n');
        }
        s
    }

    /// Check the rendering parses and write it.
    pub fn write(&self, ctx: &Ctx, out: &mut JobOutput, rel: &str) -> Result<()> {
        let text = self.render();
        let parsed: toml::Table = text
            .parse()
            .map_err(|e| data_err(format!("{rel} does not parse: {e}")))?;
        // Every top-level array counts, as `verify` counts TOML entries (rates, rows, notes).
        let n: usize = parsed
            .values()
            .map(|v| v.as_array().map_or(0, |a| a.len()))
            .sum();
        out.text(ctx, rel, &text, n as u64)
    }
}

/// Exact Poisson 90% interval for a count `n` (the Garwood interval): the rates at which seeing
/// `n` or more events, or `n` or fewer, has a 5% chance. Solved by bisection on the Poisson
/// distribution function.
pub fn poisson90(n: u64) -> (f64, f64) {
    if n > 300 {
        // exp(-mu) underflows long before this; the normal approximation is good here.
        let x = n as f64;
        return (x - 1.645 * x.sqrt(), x + 1.645 * x.sqrt() + 1.0);
    }
    // P(X <= k | mu).
    let cdf = |k: u64, mu: f64| -> f64 {
        let mut term = rr_types::math::exp(-mu);
        let mut sum = term;
        for i in 1..=k {
            term *= mu / i as f64;
            sum += term;
        }
        sum
    };
    let solve = |f: &dyn Fn(f64) -> f64, mut lo: f64, mut hi: f64| {
        // f is decreasing in mu; find f(mu) = 0.05.
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if f(mid) > 0.05 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    };
    let top = 10.0 * (n as f64 + 10.0);
    let upper = solve(&|mu| cdf(n, mu), 0.0, top);
    let lower = if n == 0 {
        0.0
    } else {
        // P(X >= n | mu) = 1 - P(X <= n-1 | mu) increases with mu; solve P(X <= n-1) = 0.95.
        solve(&|mu| cdf(n - 1, mu) - 0.90, 0.0, top)
    };
    (lower, upper)
}

/// Cause group of an OE-417 event type as PNNL transcribed it (labels changed over the years).
pub fn oe417_group(kind: &str) -> &'static str {
    let t = kind.to_ascii_lowercase();
    if t.contains("cyber") {
        "cyber"
    } else if t.contains("weather")
        || t.contains("natural disaster")
        || t.contains("winter")
        || t.contains("high wind")
        || t.contains("wind")
    {
        "weather"
    } else if t.contains("fuel") {
        "fuel"
    } else if ["suspected", "potential", "threat"]
        .iter()
        .any(|p| t.contains(p))
    {
        "suspicious"
    } else if ["physical", "vandalism", "sabotage", "theft"]
        .iter()
        .any(|p| t.contains(p))
    {
        "physical"
    } else if t.contains("suspicious") {
        "suspicious"
    } else {
        "operations"
    }
}

/// Clean an OE-417 type label (no-break spaces, stray spaces, a leading dash).
pub fn clean_kind(kind: &str) -> String {
    kind.replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_start_matches("- ")
        .to_string()
}

fn state_abbr(name: &str) -> Option<&'static str> {
    const STATES: &[(&str, &str)] = &[
        ("Alabama", "AL"),
        ("Alaska", "AK"),
        ("Arizona", "AZ"),
        ("Arkansas", "AR"),
        ("California", "CA"),
        ("Colorado", "CO"),
        ("Connecticut", "CT"),
        ("Delaware", "DE"),
        ("District of Columbia", "DC"),
        ("Florida", "FL"),
        ("Georgia", "GA"),
        ("Hawaii", "HI"),
        ("Idaho", "ID"),
        ("Illinois", "IL"),
        ("Indiana", "IN"),
        ("Iowa", "IA"),
        ("Kansas", "KS"),
        ("Kentucky", "KY"),
        ("Louisiana", "LA"),
        ("Maine", "ME"),
        ("Maryland", "MD"),
        ("Massachusetts", "MA"),
        ("Michigan", "MI"),
        ("Minnesota", "MN"),
        ("Mississippi", "MS"),
        ("Missouri", "MO"),
        ("Montana", "MT"),
        ("Nebraska", "NE"),
        ("Nevada", "NV"),
        ("New Hampshire", "NH"),
        ("New Jersey", "NJ"),
        ("New Mexico", "NM"),
        ("New York", "NY"),
        ("North Carolina", "NC"),
        ("North Dakota", "ND"),
        ("Ohio", "OH"),
        ("Oklahoma", "OK"),
        ("Oregon", "OR"),
        ("Pennsylvania", "PA"),
        ("Rhode Island", "RI"),
        ("South Carolina", "SC"),
        ("South Dakota", "SD"),
        ("Tennessee", "TN"),
        ("Texas", "TX"),
        ("Utah", "UT"),
        ("Vermont", "VT"),
        ("Virginia", "VA"),
        ("Washington", "WA"),
        ("West Virginia", "WV"),
        ("Wisconsin", "WI"),
        ("Wyoming", "WY"),
        ("Puerto Rico", "PR"),
        ("U.S. Virgin Islands", "VI"),
        ("Guam", "GU"),
    ];
    STATES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name.trim()))
        .map(|(_, a)| *a)
}

fn oe417(ctx: &Ctx, out: &mut JobOutput) -> Result<()> {
    let f = ctx
        .http
        .get(PNNL_ZIP, Some("series/Outage_Dataset_R1.zip"))?;
    out.source(super::source_from(
        "PNNL Event-correlated Outage Dataset in America (EAGLE-I 2014-2023 x DOE OE-417), OpenEI submission 6458",
        &f,
        "Outage_Dataset_R1.zip (release 1)",
        "CC BY 4.0",
        "Credit PNNL (CC BY 4.0).",
    ));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&f.bytes))?;
    // event key -> (state abbr, begin, end, kind); linked counties per event.
    let mut events: BTreeMap<(String, String), (String, i64, i64, String)> = BTreeMap::new();
    let mut linked: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .filter(|n| {
            n.contains("eaglei_outages_with_events_") && n.ends_with(".csv") && !n.contains("_lag")
        })
        .collect();
    if names.len() < 10 {
        return Err(data_err(format!(
            "PNNL outage dataset: expected ten yearly event files, found {}",
            names.len()
        )));
    }
    let mut unknown_states = BTreeSet::new();
    for name in &names {
        let mut text = String::new();
        archive.by_name(name)?.read_to_string(&mut text)?;
        let (h, rows) = crate::csvout::parse_delimited(&text, b',')?;
        let ix = |c: &str| crate::csvout::col(&h, c);
        let (i_id, i_st, i_b, i_r, i_t, i_f) = (
            ix("event_id")?,
            ix("state_event")?,
            ix("Datetime Event Began")?,
            ix("Datetime Restoration")?,
            ix("Event Type")?,
            ix("fips")?,
        );
        for r in &rows {
            if r[i_id] == "event_id" {
                continue;
            }
            let key = (r[i_id].clone(), r[i_b].clone());
            let Some(st) = state_abbr(&r[i_st]) else {
                unknown_states.insert(r[i_st].clone());
                continue;
            };
            let begin = crate::timefmt::parse_datetime(&r[i_b]).unwrap_or(0);
            let end = crate::timefmt::parse_datetime(&r[i_r])
                .unwrap_or(begin)
                .max(begin);
            events
                .entry(key.clone())
                .or_insert_with(|| (st.to_string(), begin, end, clean_kind(&r[i_t])));
            linked.entry(key).or_default().insert(r[i_f].clone());
        }
    }
    let total = events.len();
    // Intermediate for outage_model.
    let mut rows: Vec<Vec<String>> = events
        .iter()
        .map(|((id, _), (st, b, e, k))| {
            vec![
                id.clone(),
                st.clone(),
                b.to_string(),
                e.to_string(),
                k.clone(),
                oe417_group(k).to_string(),
            ]
        })
        .collect();
    rows.sort_by(|a, b| (a[2].as_str(), a[0].as_str()).cmp(&(b[2].as_str(), b[0].as_str())));
    im::write(
        &ctx.data,
        im::OE417_EVENTS,
        &["event_id", "state", "begin", "end", "kind", "group"],
        &rows,
    )?;
    // Tables.
    let groups = [
        "weather",
        "operations",
        "physical",
        "suspicious",
        "cyber",
        "fuel",
    ];
    let mut by_year: BTreeMap<(u16, &str), u64> = BTreeMap::new();
    let mut by_state: BTreeMap<(String, &str), u64> = BTreeMap::new();
    for (st, b, _, k) in events.values() {
        let y = crate::timefmt::civil_from_days(b.div_euclid(86_400)).0 as u16;
        let g = oe417_group(k);
        *by_year.entry((y, g)).or_default() += 1;
        *by_state.entry((st.clone(), g)).or_default() += 1;
    }
    let years: BTreeSet<u16> = by_year.keys().map(|(y, _)| *y).collect();
    let mut doc = SeriesDoc {
        id: "oe417".into(),
        title: "DOE OE-417 electric disturbance reports by cause, 2014-2023".into(),
        publisher: "Pacific Northwest National Laboratory (from DOE OE-417 reports and ORNL EAGLE-I)".into(),
        url: PNNL_PAGE.into(),
        licence: "CC BY 4.0".into(),
        attribution: "Grid disturbance counts derived by Ready Reckoner from the Event-correlated Outage Dataset in America, Pacific Northwest National Laboratory, OpenEI submission 6458 (CC BY 4.0), which links U.S. Department of Energy Form OE-417 electric emergency and disturbance reports to ORNL EAGLE-I outages.".into(),
        source: "pnnl_oe417_linkage".into(),
        retrieved: f.retrieved[..10].to_string(),
        how: "fetched".into(),
        ..Default::default()
    };
    doc.notes.push(format!("{total} distinct OE-417 reports in 2014-2023 (the file's ten yearly event files; the 8- and 24-hour lag variants are not used)."));
    doc.notes.push("OE-417 reports are filed for large events only (a threshold on load or customers lost, or any physical or cyber attack), the form's type labels changed over the years (grouped here into weather, operations, physical attack or vandalism, suspicious activity, cyber and fuel supply), and reporting grew sharply (32 reports in 2014, 503 in 2020). Rates therefore use 2019-2023 only.".into());
    doc.notes.push("PNNL links reports to EAGLE-I county outages by state and time window, which over-attributes in large states: the number of linked counties is context, not the size of the event.".into());
    let (ry0, ry1) = OE417_RATE_YEARS;
    let n_years = f64::from(ry1 - ry0 + 1);
    for g in groups {
        let n: u64 = by_year
            .iter()
            .filter(|((y, gg), _)| *gg == g && *y >= ry0 && *y <= ry1)
            .map(|(_, c)| *c)
            .sum();
        let (lo, hi) = poisson90(n);
        let (id, unit) = match g {
            "weather" => (
                "grid_weather_reports_per_year",
                "OE-417 reports of weather-caused disturbances per year, United States",
            ),
            "operations" => (
                "grid_operations_reports_per_year",
                "OE-417 reports of system-operations, transmission, distribution or generation disturbances per year, United States",
            ),
            "physical" => (
                "grid_physical_attack_reports_per_year",
                "OE-417 reports of physical attack, vandalism, sabotage or theft per year, United States",
            ),
            "suspicious" => (
                "grid_suspicious_activity_reports_per_year",
                "OE-417 reports of suspicious activity or a suspected or threatened attack per year, United States",
            ),
            "cyber" => (
                "grid_cyber_reports_per_year",
                "OE-417 reports of a cyber event per year, United States",
            ),
            _ => (
                "grid_fuel_supply_reports_per_year",
                "OE-417 reports of a fuel supply deficiency or emergency per year, United States",
            ),
        };
        doc.rates.push(SRate {
            id: id.into(),
            value: n as f64 / n_years,
            unit: unit.into(),
            low: Some(lo / n_years),
            high: Some(hi / n_years),
            year: ry1,
            period: format!("{ry0}-{ry1}"),
            figure: format!("{n} reports in {ry0}-{ry1}"),
            derivation: format!("{n} / {n_years:.0} years; range = exact Poisson 90% interval"),
            note: "Counts reports, not outages: most reports of attacks and suspicious activity involve no customer outage.".into(),
            unverified: false,
        });
    }
    for y in &years {
        for g in groups {
            doc.rows.push(vec![
                ("table".into(), Val::S("year".into())),
                ("year".into(), Val::I(i64::from(*y))),
                ("group".into(), Val::S(g.into())),
                (
                    "reports".into(),
                    Val::I(*by_year.get(&(*y, g)).unwrap_or(&0) as i64),
                ),
            ]);
        }
    }
    let states: BTreeSet<String> = by_state.keys().map(|(s, _)| s.clone()).collect();
    for s in &states {
        for g in groups {
            let n = *by_state.get(&(s.clone(), g)).unwrap_or(&0);
            if n > 0 {
                doc.rows.push(vec![
                    ("table".into(), Val::S("state".into())),
                    ("state".into(), Val::S(s.clone())),
                    ("group".into(), Val::S(g.into())),
                    ("reports".into(), Val::I(n as i64)),
                ]);
            }
        }
    }
    if !unknown_states.is_empty() {
        doc.notes.push(format!(
            "Reports with an unrecognised state were skipped: {}.",
            unknown_states.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    // A changed total means the source changed (the audit counted 2,922 for this release).
    if total < 2_500 {
        return Err(data_err(format!(
            "PNNL outage dataset: only {total} OE-417 reports (release 1 has 2,922)"
        )));
    }
    doc.write(ctx, out, OE417)?;
    out.attributions.push(Attribution {
        source: "PNNL Event-correlated Outage Dataset".into(),
        text: doc.attribution.clone(),
        license: "CC BY 4.0".into(),
        url: PNNL_PAGE.into(),
        version: Some("release 1 (Outage_Dataset_R1.zip)".into()),
        accessed: f.retrieved[..10].to_string(),
    });
    out.rows_in += total as u64;
    Ok(())
}

fn drug_shortages(ctx: &Ctx, out: &mut JobOutput) -> Result<()> {
    let f = ctx
        .http
        .get(OPENFDA_ZIP, Some("series/drug-shortages.json.zip"))?;
    let json = crate::http::zip_entry(&f.bytes, ".json")?;
    let v: serde_json::Value = serde_json::from_slice(&json)?;
    let updated = v["meta"]["last_updated"].as_str().unwrap_or("").to_string();
    let licence = v["meta"]["license"].as_str().unwrap_or("").to_string();
    out.source(super::source_from(
        "openFDA drug shortages (bulk download)",
        &f,
        format!("last_updated {updated}"),
        "CC0 1.0 (openFDA content)",
        "",
    ));
    let results = v["results"]
        .as_array()
        .ok_or_else(|| data_err("openFDA shortages: no results array"))?;
    // Records are per package; count distinct generic names.
    let mut by_status: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut by_form: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for r in results {
        let status = r["status"].as_str().unwrap_or("Unknown").to_string();
        let name = r["generic_name"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if name.is_empty() {
            continue;
        }
        let form = r["dosage_form"].as_str().unwrap_or("").to_lowercase();
        let group = if form.contains("inject") || form.contains("infusion") {
            "injectable"
        } else if [
            "tablet",
            "capsule",
            "oral",
            "solution",
            "suspension",
            "syrup",
            "chewable",
        ]
        .iter()
        .any(|p| form.contains(p))
        {
            "oral or liquid"
        } else {
            "other"
        };
        by_status
            .entry(status.clone())
            .or_default()
            .insert(name.clone());
        by_form
            .entry((status, group.to_string()))
            .or_default()
            .insert(name);
    }
    let current = by_status.get("Current").map_or(0, |s| s.len());
    if current == 0 || results.len() < 200 {
        return Err(data_err(format!(
            "openFDA shortages: {} records and {current} current generic names looks wrong",
            results.len()
        )));
    }
    let mut doc = SeriesDoc {
        id: "drug_shortages".into(),
        title: "FDA drug shortages on the day of the refresh (openFDA)".into(),
        publisher: "U.S. Food and Drug Administration (openFDA)".into(),
        url: "https://open.fda.gov/apis/drug/drugshortages/".into(),
        licence: format!("CC0 1.0 ({licence})"),
        source: "openfda_drug_shortages".into(),
        retrieved: f.retrieved[..10].to_string(),
        how: "fetched".into(),
        ..Default::default()
    };
    doc.notes.push(format!("{} package-level records on {updated}; counts are distinct generic names, so one medicine in short supply in several strengths counts once.", results.len()));
    doc.notes.push("A snapshot, not a history: the openFDA file lists the shortages FDA currently tracks (and a few recently resolved), so it gives how many medicines are short now, not how often a given medicine runs short. The ASHP/University of Utah series (about 270 active shortages in 2024) counts more broadly and is proprietary; it is not used.".into());
    doc.rates.push(SRate {
        id: "drug_shortages_current".into(),
        value: current as f64,
        unit: "distinct medicines (generic names) FDA lists as currently in shortage".into(),
        year: updated
            .get(..4)
            .and_then(|y| y.parse().ok())
            .unwrap_or(2026),
        period: updated.clone(),
        figure: format!("{current} generic names with status Current"),
        derivation: "count of distinct generic_name where status = Current".into(),
        note: "Openly licensed snapshot; see notes.".into(),
        ..Default::default()
    });
    for (status, names) in &by_status {
        doc.rows.push(vec![
            ("table".into(), Val::S("status".into())),
            ("status".into(), Val::S(status.clone())),
            ("medicines".into(), Val::I(names.len() as i64)),
        ]);
    }
    for ((status, group), names) in &by_form {
        doc.rows.push(vec![
            ("table".into(), Val::S("status_by_form".into())),
            ("status".into(), Val::S(status.clone())),
            ("form".into(), Val::S(group.clone())),
            ("medicines".into(), Val::I(names.len() as i64)),
        ]);
    }
    doc.write(ctx, out, DRUG_SHORTAGES)?;
    out.rows_in += results.len() as u64;
    Ok(())
}

fn fdic(ctx: &Ctx, out: &mut JobOutput) -> Result<()> {
    let f = ctx.http.get(FDIC_API, Some("series/fdic_failures.csv"))?;
    out.source(super::source_from(
        "FDIC BankFind Suite: failed banks (API, CSV)",
        &f,
        "all failures since 1934",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let (h, rows) = crate::csvout::parse_delimited(&f.text(), b',')?;
    let (i_y, i_a) = (
        crate::csvout::col(&h, "FAILYR")?,
        crate::csvout::col(&h, "QBFASSET")?,
    );
    let mut per_year: BTreeMap<u16, (u64, f64)> = BTreeMap::new();
    for r in &rows {
        let Ok(y) = r[i_y].parse::<u16>() else {
            continue;
        };
        let e = per_year.entry(y).or_default();
        e.0 += 1;
        e.1 += r[i_a].parse::<f64>().unwrap_or(0.0) * 1000.0;
    }
    if rows.len() < 4_117 {
        return Err(data_err(format!(
            "FDIC failures: {} records (at least 4,117 expected)",
            rows.len()
        )));
    }
    let last_full = crate::timefmt::today_utc()[..4]
        .parse::<u16>()
        .unwrap_or(2026)
        - 1;
    let span = |a: u16, b: u16| {
        let n: u64 = (a..=b).map(|y| per_year.get(&y).map_or(0, |x| x.0)).sum();
        (n, f64::from(b - a + 1))
    };
    let mut doc = SeriesDoc {
        id: "fdic_failures".into(),
        title: "FDIC-insured bank failures per year since 1934".into(),
        publisher: "Federal Deposit Insurance Corporation".into(),
        url: "https://banks.data.fdic.gov/bankfind-suite/failures".into(),
        licence: super::PUBLIC_DOMAIN.into(),
        source: "fdic_failed_banks".into(),
        retrieved: f.retrieved[..10].to_string(),
        how: "fetched".into(),
        ..Default::default()
    };
    doc.notes.push(format!("{} failures and assistance transactions, 1934 to the refresh date; assets are total assets at failure in nominal dollars (QBFASSET, thousands, x 1,000).", rows.len()));
    doc.notes.push("Insured deposits (up to $250,000 per depositor per bank) were paid in every failure; for a household the disruption is days without access to an account, not lost savings.".into());
    let (n1, _) = span(1934, last_full);
    let (n2, y2) = span(2001, last_full);
    let cluster_years = (1934..=last_full)
        .filter(|y| per_year.get(y).is_some_and(|x| x.0 >= 25))
        .count();
    let all_years = f64::from(last_full - 1934 + 1);
    doc.rates.push(SRate {
        id: "bank_failures_per_year".into(),
        value: n2 as f64 / y2,
        unit: "FDIC-insured bank failures per year, United States".into(),
        low: None,
        high: None,
        year: last_full,
        period: format!("2001-{last_full}"),
        figure: format!("{n2} failures in 2001-{last_full} ({n1} in 1934-{last_full})"),
        derivation: format!("{n2} / {y2:.0} years"),
        note: "Most failures are small banks; the count swings from none to 157 (2010).".into(),
        unverified: false,
    });
    doc.rates.push(SRate {
        id: "bank_failure_cluster_year_share".into(),
        value: cluster_years as f64 / all_years,
        unit: "share of years with 25 or more bank failures".into(),
        low: None,
        high: None,
        year: last_full,
        period: format!("1934-{last_full}"),
        figure: format!("{cluster_years} of {all_years:.0} years"),
        derivation: format!("{cluster_years} / {all_years:.0}"),
        note: "Cluster years mark the banking crises of the 1930s, the late 1980s and early 1990s, and 2009-2012.".into(),
        unverified: false,
    });
    for (y, (n, a)) in &per_year {
        doc.rows.push(vec![
            ("year".into(), Val::I(i64::from(*y))),
            ("failures".into(), Val::I(*n as i64)),
            ("failed_assets_usd".into(), Val::N(*a)),
        ]);
    }
    doc.write(ctx, out, FDIC)?;
    out.rows_in += rows.len() as u64;
    Ok(())
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    oe417(ctx, &mut out)?;
    drug_shortages(ctx, &mut out)?;
    fdic(ctx, &mut out)?;
    super::series_transcribed::build(ctx, &mut out)?;
    super::series_arrests::build(ctx, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oe417_groups() {
        assert_eq!(
            oe417_group("Severe Weather/Transmission Interruption"),
            "weather"
        );
        assert_eq!(oe417_group("- Weather or natural disaster"), "weather");
        assert_eq!(oe417_group("Actual Physical Attack/Vandalism"), "physical");
        assert_eq!(
            oe417_group("Suspected Physical Attack - Vandalism"),
            "suspicious"
        );
        assert_eq!(oe417_group("Suspicious Activity"), "suspicious");
        assert_eq!(oe417_group("Cyber Event"), "cyber");
        assert_eq!(oe417_group("Fuel Supply Deficiency"), "fuel");
        assert_eq!(oe417_group("System Operations"), "operations");
        assert_eq!(oe417_group("Generation Inadequacy"), "operations");
        assert_eq!(clean_kind("- Vandalism\u{a0}- Theft"), "Vandalism - Theft");
    }

    #[test]
    fn poisson_interval_brackets_the_count() {
        for n in [0u64, 1, 5, 20, 45, 200] {
            let (lo, hi) = poisson90(n);
            assert!(lo <= n as f64 && hi >= n as f64, "{n}: {lo} {hi}");
        }
        assert!((poisson90(0).1 - 2.9957).abs() < 1e-3);
        assert!((poisson90(20).0 - 13.2547).abs() < 1e-3);
        assert!(poisson90(900).0 > 850.0);
        // The base_rates pandemic interval uses the same table row.
        assert!((poisson90(5).0 - 3.940_299 / 2.0).abs() < 1e-3);
        assert!((poisson90(5).1 - 21.026_07 / 2.0).abs() < 1e-3);
    }

    #[test]
    fn a_series_document_renders_and_parses() {
        let mut d = SeriesDoc {
            id: "t".into(),
            title: "Test \"quoted\"".into(),
            ..Default::default()
        };
        d.rates.push(SRate {
            id: "r".into(),
            value: 0.123456,
            unit: "u".into(),
            low: Some(0.1),
            ..Default::default()
        });
        d.rows.push(vec![
            ("year".into(), Val::I(2020)),
            ("x".into(), Val::N(1.5)),
        ]);
        let t: toml::Table = d.render().parse().unwrap();
        assert_eq!(t["rate"][0]["value"].as_float(), Some(0.1235));
        assert_eq!(t["row"][0]["year"].as_integer(), Some(2020));
        assert_eq!(t["title"].as_str(), Some("Test \"quoted\""));
    }
}
