//! Data pack v2 exposure files: the county columns for the v0.2.0 hazards, the strategic-site
//! table, and the ZIP-level columns (including the optional `surge` and `wildfire_places` packs).
//!
//! Every county file here is keyed by `fips` and sets its own [`CountyExposure`] fields; columns
//! this engine does not know are ignored (a newer pack still loads). The parts are merged into
//! [`CountyRecord::exposure`](rr_types::CountyRecord) whenever the store reassembles its records,
//! so files can arrive in any order.

use crate::table::{Csv, corrupt, f, f32c};
use rr_types::{
    CountyExposure, EngineError, PlaceWildfire, StrategicClass, StrategicPlace, SurgeProxyClass,
    ZipRecord,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::str::FromStr;

/// County-keyed exposure files (each written by one `rr-etl` job).
pub const COUNTY_FILES: &[&str] = &[
    "core/strategic.csv",
    "core/geomag.csv",
    "core/smoke.csv",
    "core/ground.csv",
    "core/levees.csv",
    "core/water_systems.csv",
    "core/surge_proxy.csv",
    "core/eviction.csv",
];

/// The strategic-site table.
pub const STRATEGIC_SITES: &str = "core/strategic_sites.toml";

/// Citation ids for the exposure values `LocationResolved.exposure` carries, by `Exposure`
/// field, plus the county-only columns the hazard crates read from `CountyExposure`
/// (`uasi_area_share`). Ids already in `content/citations.toml`: `usace_nld`,
/// `fema_hsgp_fy2026`. The others are requested from the content workstream in
/// `docs/DATA_SOURCES.md` §13 (title, publisher, URL for each).
pub const EXPOSURE_SOURCES: &[(&str, &str)] = &[
    ("strategic_class", "rr_strategic_sites"),
    ("strategic_km", "rr_strategic_sites"),
    ("surge_cat3_share", "nhc_storm_surge_maps"),
    ("surge_proxy_class", "rr_surge_proxy"),
    ("smoke_days_35", "epa_aqs_daily_pm25"),
    ("leveed_pop_share", "usace_nld"),
    ("dams_high_within_10km", "usace_nid"),
    ("karst_share", "usgs_karst_2014"),
    ("landslide_susceptible_share", "usgs_landslide_2024"),
    ("water_system_flag", "epa_echo_sdwa"),
    ("geomag_factor", "nerc_tpl007_gmd"),
    ("uasi_share", "fema_hsgp_fy2026"),
    ("uasi_area_share", "fema_hsgp_fy2026"),
    ("eviction_rate", "eviction_lab_county_estimates"),
];

/// The citation id for an `Exposure` field (see [`EXPOSURE_SOURCES`]).
pub fn exposure_source(field: &str) -> &'static str {
    EXPOSURE_SOURCES
        .iter()
        .find(|(f, _)| *f == field)
        .map(|(_, id)| *id)
        .unwrap_or("rr_data_pack")
}

/// An `f32` pack value as the `f64` it was written as (0.29, not 0.2899999916553497).
pub fn clean_f64(x: f32) -> f64 {
    x.to_string().parse::<f64>().unwrap_or(f64::from(x))
}
/// Optional pack `surge`: NOAA/NHC surge-area shares by ZIP.
pub const ZIP_SURGE: &str = "opt/surge/zip_surge.csv";
/// Optional pack `wildfire_places`: Wildfire Risk to Communities by Census place.
pub const WILDFIRE_PLACES: &str = "opt/wildfire_places/places.csv";
/// Optional pack `wildfire_places`: which places each ZIP overlaps.
pub const ZIP_PLACES: &str = "opt/wildfire_places/zip_places.csv";

fn u16c(cell: &str) -> Option<u16> {
    f(cell).map(|v| v.round().clamp(0.0, u16::MAX as f64) as u16)
}

fn text(cell: &str) -> Option<String> {
    let t = cell.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Set one known column on a county's exposure. Returns `Ok(false)` for a column this engine
/// does not know (ignored) and an error for a malformed value in a known one.
pub(crate) fn apply_column(
    e: &mut CountyExposure,
    column: &str,
    cell: &str,
) -> Result<bool, String> {
    match column {
        "strategic_class" => {
            e.strategic_class = match text(cell) {
                Some(t) => Some(StrategicClass::from_str(&t).map_err(|x| x.to_string())?),
                None => None,
            }
        }
        "strategic_site_ids" => {
            e.strategic_site_ids = cell
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        }
        "strategic_km" => e.strategic_km = f32c(cell),
        "strategic_bearing" => e.strategic_bearing = f32c(cell),
        "uasi_share" => e.uasi_share = f32c(cell),
        "uasi_area_share" => e.uasi_area_share = f32c(cell),
        "geomag_lat" => e.geomag_lat = f32c(cell),
        "geomag_factor" => e.geomag_factor = f32c(cell),
        "smoke_days_35" => e.smoke_days_35 = f32c(cell),
        "smoke_days_55" => e.smoke_days_55 = f32c(cell),
        "smoke_trend" => e.smoke_trend = f32c(cell),
        "hms_smoke_days" => e.hms_smoke_days = f32c(cell),
        "smoke_basis" => e.smoke_basis = text(cell),
        "karst_share" => e.karst_share = f32c(cell),
        "landslide_susceptible_share" => e.landslide_susceptible_share = f32c(cell),
        "leveed_pop_share" => e.leveed_pop_share = f32c(cell),
        "levee_risk_high_share" => e.levee_risk_high_share = f32c(cell),
        "sdwis_violation_pop_share" => e.sdwis_violation_pop_share = f32c(cell),
        "cws_pop_share" => e.cws_pop_share = f32c(cell),
        "surge_proxy_class" => {
            e.surge_proxy_class = match text(cell) {
                Some(t) => Some(SurgeProxyClass::from_str(&t).map_err(|x| x.to_string())?),
                None => None,
            }
        }
        "coastal_flood_pop_share" => e.coastal_flood_pop_share = f32c(cell),
        "eviction_filing_rate" => e.eviction_filing_rate = f32c(cell),
        _ => return Ok(false),
    }
    Ok(true)
}

/// Fill every field `part` sets into `into` (the files set disjoint fields).
pub(crate) fn merge(into: &mut CountyExposure, part: &CountyExposure) {
    macro_rules! take {
        ($($field:ident),+ $(,)?) => {
            $( if part.$field.is_some() { into.$field = part.$field.clone(); } )+
        };
    }
    take!(
        strategic_class,
        strategic_km,
        strategic_bearing,
        uasi_share,
        uasi_area_share,
        uasi_area,
        geomag_lat,
        geomag_factor,
        smoke_days_35,
        smoke_days_55,
        smoke_trend,
        hms_smoke_days,
        smoke_basis,
        karst_share,
        landslide_susceptible_share,
        leveed_pop_share,
        levee_risk_high_share,
        dams_high_total,
        dams_high_poor_condition,
        sdwis_violation_pop_share,
        cws_pop_share,
        surge_proxy_class,
        coastal_flood_pop_share,
        eviction_filing_rate,
    );
    if !part.strategic_site_ids.is_empty() {
        into.strategic_site_ids = part.strategic_site_ids.clone();
    }
    if !part.strategic_places.is_empty() {
        into.strategic_places = part.strategic_places.clone();
    }
}

/// A county file parsed into partial exposures, plus the raw UASI area ranks (resolved to names
/// against the strategic table when the records are assembled).
#[derive(Debug, Clone, Default)]
pub(crate) struct CountyPart {
    pub rows: BTreeMap<String, CountyExposure>,
    pub uasi_rank: BTreeMap<String, u32>,
}

/// Parse one county exposure file.
pub(crate) fn parse_county_file(name: &str, t: &Csv) -> Result<CountyPart, EngineError> {
    let i_f = t.col("fips")?;
    let i_rank = t.opt_col("uasi_area");
    let mut part = CountyPart::default();
    for r in &t.rows {
        let mut e = CountyExposure::default();
        for (i, h) in t.header.iter().enumerate() {
            if i == i_f {
                continue;
            }
            apply_column(&mut e, h, &r[i])
                .map_err(|m| corrupt(name, format!("{} for {}: {m}", h, r[i_f])))?;
        }
        if let Some(i) = i_rank
            && let Some(rank) = f(&r[i])
        {
            part.uasi_rank.insert(r[i_f].clone(), rank as u32);
        }
        part.rows.insert(r[i_f].clone(), e);
    }
    Ok(part)
}

/// A site in `strategic_sites.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrategicSite {
    /// Stable id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Main kind.
    pub kind: String,
    /// Other kinds.
    #[serde(default)]
    pub also: Vec<String>,
    /// `A`, `C1` or `C2`.
    pub class: String,
    /// State(s) in words.
    #[serde(default)]
    pub state: String,
    /// Latitude.
    #[serde(default)]
    pub lat: Option<f64>,
    /// Longitude.
    #[serde(default)]
    pub lon: Option<f64>,
    /// `point_30km` or `county_list`.
    pub rule: String,
    /// Counties that hold it.
    #[serde(default)]
    pub counties: Vec<String>,
    /// Plain-language role.
    #[serde(default)]
    pub role_plain: String,
    /// Role in detail.
    #[serde(default)]
    pub role: String,
    /// How well the sources support it.
    #[serde(default)]
    pub support: String,
    /// Source ids.
    #[serde(default)]
    pub sources: Vec<String>,
    /// What is not confirmed by a primary source read in full.
    #[serde(default)]
    pub unverified: Vec<String>,
    /// Notes.
    #[serde(default)]
    pub note: String,
}

/// A class definition (the f_s factors are expert priors).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrategicClassDef {
    /// Class id.
    pub id: String,
    /// Label.
    pub label: String,
    /// Central factor (prior).
    pub f_s: f64,
    /// Low end (prior).
    pub f_s_low: f64,
    /// High end (prior).
    pub f_s_high: f64,
    /// True: expert prior.
    #[serde(default)]
    pub prior: bool,
    /// Rule in words.
    pub rule: String,
    /// "Why here" template.
    pub why_here: String,
}

/// A named area (metro, port, refinery county).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrategicArea {
    /// Id (`metro_<cbsa>`, `port_<rank>`, `refinery_<county>`).
    pub id: String,
    /// Title or name (metros: `title`; ports: `name`; refineries: the sites joined).
    #[serde(default, alias = "title")]
    pub name: String,
    /// Refinery sites (refineries only).
    #[serde(default)]
    pub sites: Vec<String>,
}

/// A UASI urban area.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UasiArea {
    /// Rank by allocation.
    pub rank: u32,
    /// FEMA's name.
    pub urban_area: String,
    /// FY2026 allocation, dollars.
    pub usd_fy2026: u64,
    /// Share of the national total.
    pub share: f64,
    /// Proposed county footprint.
    #[serde(default)]
    pub counties: Vec<String>,
}

/// A citation in the strategic table.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrategicSource {
    /// Citation id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Publisher.
    pub publisher: String,
    /// Year.
    pub year: u32,
    /// URL.
    pub url: String,
    /// `fetched`, `search_summary` or `not_retrieved`.
    pub access: String,
    /// Agency, statute or EIS.
    #[serde(default)]
    pub primary: bool,
}

/// `core/strategic_sites.toml`, as the engine reads it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrategicSites {
    /// Schema label.
    pub schema: String,
    /// Class precedence.
    pub precedence: Vec<String>,
    /// Classes with their priors and sentence templates.
    pub class: Vec<StrategicClassDef>,
    /// Plain role phrases.
    #[serde(default)]
    pub role_plain: BTreeMap<String, String>,
    /// How template placeholders are filled.
    #[serde(default)]
    pub template_rules: BTreeMap<String, String>,
    /// Sites.
    pub site: Vec<StrategicSite>,
    /// Metros.
    #[serde(default)]
    pub metro: Vec<StrategicArea>,
    /// Ports.
    #[serde(default)]
    pub port: Vec<StrategicArea>,
    /// Refinery counties.
    #[serde(default)]
    pub refinery: Vec<StrategicArea>,
    /// UASI areas.
    #[serde(default)]
    pub uasi: Vec<UasiArea>,
    /// Sources.
    #[serde(default)]
    pub source: Vec<StrategicSource>,
}

impl StrategicSites {
    /// Resolve an id from `strategic_site_ids` to a place.
    pub fn place(&self, id: &str) -> Option<StrategicPlace> {
        if let Some(s) = self.site.iter().find(|s| s.id == id) {
            return Some(StrategicPlace {
                id: s.id.clone(),
                name: s.name.clone(),
                kind: s.kind.clone(),
                role_plain: s.role_plain.clone(),
                state: s.state.clone(),
                sources: s.sources.clone(),
                unverified: s.unverified.clone(),
            });
        }
        let (list, kind, sources): (&[StrategicArea], &str, &[&str]) = if id.starts_with("metro_") {
            (
                &self.metro,
                "metro",
                &["census_cbsa_pop_2024", "census_delineation_2023"],
            )
        } else if id.starts_with("port_") {
            (&self.port, "port", &["bts_ports_2025"])
        } else if id.starts_with("refinery_") {
            (&self.refinery, "refinery", &["eia_refcap_2026"])
        } else {
            return None;
        };
        let a = list.iter().find(|a| a.id == id)?;
        let name = if a.name.is_empty() {
            a.sites.join("; ")
        } else {
            a.name.clone()
        };
        Some(StrategicPlace {
            id: a.id.clone(),
            name,
            kind: kind.to_string(),
            role_plain: String::new(),
            state: String::new(),
            sources: sources.iter().map(|s| s.to_string()).collect(),
            unverified: if kind == "port" {
                vec!["principal port counties: assigned by the research agent, not checked against USACE port limits".into()]
            } else {
                Vec::new()
            },
        })
    }

    /// The UASI area with this rank.
    pub fn uasi_area(&self, rank: u32) -> Option<&UasiArea> {
        self.uasi.iter().find(|u| u.rank == rank)
    }
}

/// ZIP-level columns from `zip_facilities.csv` beyond the two the v1 loader reads.
pub(crate) fn zip_extras(t: &Csv) -> Result<BTreeMap<String, ZipRecord>, EngineError> {
    let i_z = t.col("zip")?;
    let (i_d, i_k, i_b, i_s) = (
        t.opt_col("dams_high_within_10km_naming_town"),
        t.opt_col("strategic_km"),
        t.opt_col("strategic_bearing"),
        t.opt_col("strategic_site"),
    );
    let mut out = BTreeMap::new();
    for r in &t.rows {
        let rec = ZipRecord {
            zip: r[i_z].clone(),
            dams_high_within_10km_naming_town: i_d.and_then(|i| u16c(&r[i])),
            strategic_km: i_k.and_then(|i| f32c(&r[i])),
            strategic_bearing: i_b.and_then(|i| f32c(&r[i])),
            strategic_site: i_s.and_then(|i| text(&r[i])),
            ..Default::default()
        };
        out.insert(r[i_z].clone(), rec);
    }
    Ok(out)
}

/// Category 1 and Category 3 surge-area shares of a ZIP code.
pub(crate) type SurgeShares = (Option<f32>, Option<f32>);

/// `opt/surge/zip_surge.csv`: (Category 1 share, Category 3 share) by ZIP.
pub(crate) fn zip_surge(t: &Csv) -> Result<BTreeMap<String, SurgeShares>, EngineError> {
    let (i_z, i_1, i_3) = (
        t.col("zip")?,
        t.col("surge_cat1_share")?,
        t.col("surge_cat3_share")?,
    );
    Ok(t.rows
        .iter()
        .map(|r| (r[i_z].clone(), (f32c(&r[i_1]), f32c(&r[i_3]))))
        .collect())
}

/// `opt/wildfire_places/places.csv`: exposure by Census place (land share left at 0; filled per
/// ZIP from `zip_places.csv`).
pub(crate) fn places(t: &Csv) -> Result<BTreeMap<String, PlaceWildfire>, EngineError> {
    let (i_p, i_n) = (t.col("place")?, t.col("name")?);
    let (i_d, i_i, i_r) = (
        t.opt_col("buildings_direct"),
        t.opt_col("buildings_indirect"),
        t.opt_col("risk_national_rank"),
    );
    Ok(t.rows
        .iter()
        .map(|r| {
            (
                r[i_p].clone(),
                PlaceWildfire {
                    place: r[i_p].clone(),
                    name: r[i_n].clone(),
                    zip_land_share: 0.0,
                    buildings_direct: i_d.and_then(|i| f32c(&r[i])),
                    buildings_indirect: i_i.and_then(|i| f32c(&r[i])),
                    risk_national_rank: i_r.and_then(|i| f32c(&r[i])),
                },
            )
        })
        .collect())
}

/// `opt/wildfire_places/zip_places.csv`: places per ZIP with the ZIP's land share, largest first.
pub(crate) fn zip_places(t: &Csv) -> Result<BTreeMap<String, Vec<(String, f32)>>, EngineError> {
    let (i_z, i_p, i_s) = (t.col("zip")?, t.col("place")?, t.col("zip_land_share")?);
    let mut out: BTreeMap<String, Vec<(String, f32)>> = BTreeMap::new();
    for r in &t.rows {
        out.entry(r[i_z].clone())
            .or_default()
            .push((r[i_p].clone(), f32c(&r[i_s]).unwrap_or(0.0)));
    }
    for v in out.values_mut() {
        v.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_columns_set_fields_and_unknown_ones_are_ignored() {
        let mut e = CountyExposure::default();
        assert!(apply_column(&mut e, "strategic_class", "C1").unwrap());
        assert!(apply_column(&mut e, "strategic_site_ids", "metro_37980;y12").unwrap());
        assert!(apply_column(&mut e, "smoke_days_35", "0.125").unwrap());
        assert!(apply_column(&mut e, "surge_proxy_class", "none").unwrap());
        assert!(!apply_column(&mut e, "something_new", "5").unwrap());
        assert!(apply_column(&mut e, "strategic_class", "Z").is_err());
        assert_eq!(e.strategic_class, Some(StrategicClass::C1));
        assert_eq!(e.strategic_site_ids, vec!["metro_37980", "y12"]);
        assert_eq!(e.smoke_days_35, Some(0.125));
        assert_eq!(e.surge_proxy_class, Some(SurgeProxyClass::NoSurge));
        let mut into = CountyExposure {
            karst_share: Some(0.4),
            ..Default::default()
        };
        merge(&mut into, &e);
        assert_eq!(into.karst_share, Some(0.4));
        assert_eq!(into.strategic_class, Some(StrategicClass::C1));
    }
}
