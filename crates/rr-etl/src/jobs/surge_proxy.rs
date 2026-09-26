//! Job — a county-level storm-surge proxy for the core pack (the NOAA/NHC surge maps by ZIP are
//! the optional `surge` pack; see `surge.rs`).
//!
//! It combines what the core pack already holds with one curated table:
//! - the share of residents in NRI's coastal-flood exposure areas (`coastal_flooding` people
//!   exposed ÷ county population; NRI v1.20);
//! - how often tropical storms and hurricanes pass within 50 nautical miles (HURDAT2, 1950 on,
//!   `events.csv`); for Guam, the Northern Mariana Islands and American Samoa, which HURDAT2 does
//!   not cover, NRI's hurricane annualised frequency;
//! - whether officials publish hurricane evacuation zones for the county (statewide "know your
//!   zone" tools in Florida, Georgia, Maryland, North Carolina, South Carolina and Virginia;
//!   county or city tools in coastal Texas, Louisiana's coastal parishes, Baldwin and Mobile
//!   counties in Alabama, Jackson County in Mississippi and New York City; from the state
//!   registry research of 2026-09-26, whose unconfirmed rows are left out).
//!
//! Classes: `none` (not a coastal-flood county, or tropical storms pass less than about once in
//! 50 years); `high` (at least 10% of residents in coastal-flood areas and a hurricane within
//! 50 nmi at least once in 20 years on average); `moderate` (at least 2% of residents and a
//! tropical storm at least once in 10 years, or official evacuation zones and a hurricane at
//! least once in 50 years); `low` (every other coastal county with tropical storms on record).
//! It says whether a county has meaningful surge exposure, never whether an address is in a zone.
//! No elevation is used: no national elevation source is cheap enough to read per ZIP here, and
//! the optional NHC pack answers the ZIP question properly.

use super::{Ctx, JobOutput, load_counties};
use crate::csvout::{Table, col, read_table};
use crate::manifest::Attribution;
use crate::num::sig;
use crate::{Result, data_err};
use std::collections::BTreeMap;

/// Surge proxy per county.
pub const SURGE_PROXY: &str = "core/surge_proxy.csv";

/// Minimum tropical-storm passages a year for any surge class (about once in 50 years).
pub const MIN_TS_RATE: f64 = 0.02;

/// States with a statewide hurricane evacuation-zone lookup (research/state-registries.md,
/// "Statewide" rows), applied to their NRI coastal-flood counties.
pub const STATEWIDE_ZONES: &[(&str, &str)] = &[
    ("FL", "https://www.floridadisaster.org/knowyourzone/"),
    (
        "GA",
        "https://gema.georgia.gov/plan-prepare/hurricane-evacuation-zone",
    ),
    (
        "MD",
        "https://mdem.maryland.gov/action/Pages/know-your-zone-md.aspx",
    ),
    (
        "NC",
        "https://www.ncdps.gov/our-organization/emergency-management/emergency-preparedness/know-your-zone",
    ),
    ("SC", "https://www.scemd.org/prepare/know-your-zone/"),
    ("VA", "https://www.vdem.virginia.gov/know-your-zone/"),
];

/// States where the coastal counties or parishes run their own zone maps (applied to their NRI
/// coastal-flood counties).
pub const COASTAL_COUNTY_ZONES: &[&str] = &["TX", "LA"];

/// Individual counties with confirmed local zone tools.
pub const COUNTY_ZONES: &[(&str, &str)] = &[
    ("01003", "Baldwin County, AL"),
    ("01097", "Mobile County, AL"),
    ("28059", "Jackson County, MS"),
    (
        "36005",
        "Bronx County, NY (NYC Hurricane Evacuation Zone Finder)",
    ),
    ("36047", "Kings County, NY"),
    ("36061", "New York County, NY"),
    ("36081", "Queens County, NY"),
    ("36085", "Richmond County, NY"),
];

/// The inputs for one county.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Inputs {
    /// NRI coastal-flood county.
    pub coastal: bool,
    /// Share of residents in NRI coastal-flood areas.
    pub flood_share: f64,
    /// Tropical-storm passages a year.
    pub ts_rate: f64,
    /// Hurricane passages a year.
    pub hu_rate: f64,
    /// Officials publish evacuation zones.
    pub zones: bool,
}

/// The class for one county (see the module notes).
pub fn classify(i: &Inputs) -> &'static str {
    if !i.coastal || i.ts_rate < MIN_TS_RATE {
        return "none";
    }
    if i.flood_share >= 0.10 && i.hu_rate >= 0.05 {
        return "high";
    }
    if (i.flood_share >= 0.02 && i.ts_rate >= 0.10) || (i.zones && i.hu_rate >= 0.02) {
        return "moderate";
    }
    "low"
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let (h, rows) = read_table(&ctx.data, super::nri::NRI_COUNTIES)?;
    let (i_f, i_p, i_c) = (
        col(&h, "fips")?,
        col(&h, "population")?,
        col(&h, "coastal")?,
    );
    let nri: BTreeMap<String, (f64, bool)> = rows
        .iter()
        .map(|r| {
            (
                r[i_f].clone(),
                (r[i_p].parse::<f64>().unwrap_or(0.0), r[i_c] == "true"),
            )
        })
        .collect();
    let (h, rows) = read_table(&ctx.data, super::nri::NRI_HAZARDS)?;
    let (i_f, i_h, i_e, i_a) = (
        col(&h, "fips")?,
        col(&h, "hazard")?,
        col(&h, "expp")?,
        col(&h, "afreq")?,
    );
    let mut flood_people: BTreeMap<String, f64> = BTreeMap::new();
    let mut nri_hurricane: BTreeMap<String, f64> = BTreeMap::new();
    for r in &rows {
        match r[i_h].as_str() {
            "coastal_flooding" => {
                flood_people.insert(r[i_f].clone(), r[i_e].parse().unwrap_or(0.0));
            }
            "hurricane" => {
                nri_hurricane.insert(r[i_f].clone(), r[i_a].parse().unwrap_or(0.0));
            }
            _ => {}
        }
    }
    let (h, rows) = read_table(&ctx.data, super::events::EVENTS)?;
    let (i_f, i_t, i_r) = (
        col(&h, "fips")?,
        col(&h, "event_type")?,
        col(&h, "rate_per_year")?,
    );
    let mut rates: BTreeMap<(String, String), f64> = BTreeMap::new();
    for r in &rows {
        if r[i_t].ends_with("_passage") {
            rates.insert(
                (r[i_f].clone(), r[i_t].clone()),
                r[i_r].parse().unwrap_or(0.0),
            );
        }
    }
    out.rows_in += nri.len() as u64;

    let mut table = Table::new(&["fips", "surge_proxy_class", "coastal_flood_pop_share"], 1);
    let mut counts: BTreeMap<&str, (u32, f64)> = BTreeMap::new();
    let mut classes: BTreeMap<String, &'static str> = BTreeMap::new();
    for c in &counties {
        let (pop, coastal) = nri.get(&c.fips).copied().unwrap_or((0.0, false));
        let share = if pop > 0.0 {
            (flood_people.get(&c.fips).copied().unwrap_or(0.0) / pop).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let rate = |t: &str| {
            rates
                .get(&(c.fips.clone(), t.to_string()))
                .copied()
                .unwrap_or(0.0)
        };
        let (ts, hu) = if matches!(c.state_abbr.as_str(), "GU" | "MP" | "AS") {
            // No HURDAT2 coverage: NRI's hurricane frequency stands in for both.
            let a = nri_hurricane.get(&c.fips).copied().unwrap_or(0.0);
            (a, a)
        } else {
            (rate("tropical_storm_passage"), rate("hurricane_passage"))
        };
        let zones = coastal
            && (STATEWIDE_ZONES.iter().any(|(s, _)| *s == c.state_abbr)
                || COASTAL_COUNTY_ZONES.contains(&c.state_abbr.as_str())
                || COUNTY_ZONES.iter().any(|(f, _)| *f == c.fips));
        let class = classify(&Inputs {
            coastal,
            flood_share: share,
            ts_rate: ts,
            hu_rate: hu,
            zones,
        });
        let e = counts.entry(class).or_default();
        e.0 += 1;
        e.1 += pop;
        classes.insert(c.fips.clone(), class);
        table.push(vec![c.fips.clone(), class.to_string(), sig(share, 3)]);
    }
    for (fips, want, place) in [
        ("12086", "high", "Miami-Dade County, FL"),
        ("48167", "high", "Galveston County, TX"),
        ("17031", "none", "Cook County, IL (Great Lakes)"),
        ("04013", "none", "Maricopa County, AZ"),
    ] {
        let got = classes.get(fips).copied().unwrap_or("?");
        if got != want {
            return Err(data_err(format!(
                "surge proxy for {place} is {got}, expected {want}"
            )));
        }
    }
    out.table(ctx, SURGE_PROXY, &mut table)?;
    let total_pop: f64 = counts.values().map(|x| x.1).sum();
    out.notes.push(format!(
        "Classes: {}. Inputs: NRI v1.20 coastal-flooding people exposed / population, HURDAT2 tropical-storm and hurricane passages within 50 nmi (events.csv; NRI hurricane frequency for Guam, the Northern Mariana Islands and American Samoa), and published evacuation zones (statewide in {}; coastal counties in {}; {}). Rules: none if not an NRI coastal-flood county or tropical storms under {MIN_TS_RATE} a year; high if at least 10% of residents in coastal-flood areas and at least 0.05 hurricanes a year; moderate if at least 2% and 0.1 tropical storms a year, or evacuation zones and 0.02 hurricanes a year; low otherwise.",
        ["high", "moderate", "low", "none"]
            .iter()
            .map(|k| {
                let (n, p) = counts.get(k).copied().unwrap_or_default();
                format!("{k} {n} counties ({:.1}% of people)", 100.0 * p / total_pop.max(1.0))
            })
            .collect::<Vec<_>>()
            .join(", "),
        STATEWIDE_ZONES.iter().map(|(s, _)| *s).collect::<Vec<_>>().join(", "),
        COASTAL_COUNTY_ZONES.join(", "),
        COUNTY_ZONES.iter().map(|(_, n)| *n).collect::<Vec<_>>().join("; ")
    ));
    out.notes.push("A county proxy: it cannot say that an address is in a surge zone. NRI's coastal-flood areas are the 1% and 0.2% coastal floodplains plus high-tide flooding, smaller than NHC's Category 3 surge extent, so shares understate surge reach. No elevation is used (no national source cheap enough per ZIP); the optional `surge` pack (NOAA/NHC maps by ZIP) is the precise answer. Unconfirmed zone tools in the research (Mississippi statewide, New Jersey, Delaware, Connecticut, Rhode Island) are left out.".into());
    out.definitions.insert("surge_proxy_class".into(), "County storm-surge exposure: none, low, moderate or high, from NRI coastal-flood exposure, hurricane passages and official evacuation zones (a proxy; it does not place an address in or out of a zone).".into());
    out.definitions.insert(
        "coastal_flood_pop_share".into(),
        "People in NRI v1.20 coastal-flood exposure areas / county population.".into(),
    );
    out.attributions.push(Attribution {
        source: "Storm-surge proxy".into(),
        text: "County storm-surge proxy built by Ready Reckoner from the FEMA National Risk Index (coastal flooding exposure), NOAA NHC HURDAT2 storm tracks and state and local hurricane evacuation-zone tools.".into(),
        license: "Derived from US Government works".into(),
        url: "https://www.nhc.noaa.gov/nationalsurge/".into(),
        version: None,
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_follow_the_rules() {
        let miami = Inputs {
            coastal: true,
            flood_share: 0.52,
            ts_rate: 0.30,
            hu_rate: 0.145,
            zones: true,
        };
        assert_eq!(classify(&miami), "high");
        let brooklyn = Inputs {
            coastal: true,
            flood_share: 0.067,
            ts_rate: 0.25,
            hu_rate: 0.039,
            zones: true,
        };
        assert_eq!(classify(&brooklyn), "moderate");
        let philadelphia = Inputs {
            coastal: true,
            flood_share: 0.018,
            ts_rate: 0.17,
            hu_rate: 0.013,
            zones: false,
        };
        assert_eq!(classify(&philadelphia), "low");
        let chicago = Inputs {
            coastal: true,
            flood_share: 0.027,
            ts_rate: 0.0,
            hu_rate: 0.0,
            zones: false,
        };
        assert_eq!(classify(&chicago), "none");
        let inland = Inputs {
            coastal: false,
            ts_rate: 0.5,
            hu_rate: 0.2,
            ..Default::default()
        };
        assert_eq!(classify(&inland), "none");
    }
}
