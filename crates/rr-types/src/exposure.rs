//! Exposure records added with data pack v2 (DESIGN-DELTA §2): per-county and per-ZIP columns
//! for the v0.2.0 hazards (strategic sites, UASI, geomagnetic latitude, wildfire smoke, karst and
//! landslide terrain, levees, dams, drinking-water violations, storm surge, eviction filings).
//!
//! Written by `rr-etl` (one job per source), loaded by `rr-data` into
//! [`CountyRecord::exposure`](crate::CountyRecord) and [`ZipRecord`]. Engine-internal like the
//! rest of [`data`](crate::data): the web app never reads these directly; what reaches the user
//! goes through `LocationResolved.exposure` (DESIGN-DELTA §1.3) and carries a source id.
//!
//! Every field is optional or empty by default, so a pack without a given file still loads and
//! the engine can say what it is missing. Field names are the pack's column names. The
//! user-facing copy, [`Exposure`](crate::Exposure) on `LocationResolved` (DESIGN-DELTA §1.3), is
//! filled from these by `rr_data::DataStore::location`, each value with its citation id.

use serde::{Deserialize, Serialize};

string_enum! {
    /// Strategic-exposure class of a county for the nuclear family (`core/strategic_sites.toml`
    /// holds the rules and the f_S priors). Declared in precedence order, highest exposure
    /// first, so the derived `Ord` puts `A` first.
    pub enum StrategicClass: "strategic class" {
        /// Counterforce and command sites (missile fields, submarine and bomber bases, weapons
        /// storage, command centres, strategic missile defence).
        A = "A",
        /// Largest cities, the National Capital Region and nuclear-weapons plants.
        C1 = "C1",
        /// Downwind (east) of the missile fields: the fallout corridor.
        B = "B",
        /// Other metros of a million or more, big ports and refineries, other major bases.
        C2 = "C2",
        /// Downwind of an A site, a weapons plant or a C1 county.
        D = "D",
        /// Remote from targets and fallout paths.
        E = "E",
    }
}

string_enum! {
    /// How much storm-surge exposure a county has, from the county-level proxy in
    /// `core/surge_proxy.csv` (NRI coastal-flood exposure, hurricane passages and official
    /// evacuation zones). It cannot say whether a given address is in a surge zone: the optional
    /// `surge` pack (NOAA/NHC maps by ZIP) and the official zone lookups do that.
    pub enum SurgeProxyClass: "surge proxy class" {
        /// No coastal storm-surge exposure (inland, Great Lakes, or no hurricane record).
        NoSurge = "none",
        /// A coastal county where surge reaches few people.
        Low = "low",
        /// Surge zones reach a meaningful share of residents, or official evacuation zones exist.
        Moderate = "moderate",
        /// Much of the county lies in coastal flood areas and hurricanes pass often.
        High = "high",
    }
}

/// A site or area behind a county's strategic class, resolved from `core/strategic_sites.toml`
/// so the "Why here" sentence can name it without another lookup.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategicPlace {
    /// Id as in `strategic_site_ids` (a site id, `metro_<cbsa>`, `port_<rank>`,
    /// `refinery_<county>`).
    pub id: String,
    /// Name ("Naval Base Kitsap-Bangor", "Philadelphia-Camden-Wilmington, PA-NJ-DE-MD").
    pub name: String,
    /// Site kind (`icbm_field`, `ssbn_base`, ...) or `metro`, `port`, `refinery`.
    pub kind: String,
    /// What the place is, in plain words (sites only; empty otherwise).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role_plain: String,
    /// State(s) in words (sites), for "the missile fields in {field_states}".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state: String,
    /// Source ids (resolve in `strategic_sites.toml` `[[source]]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// What about this place is not confirmed by a primary source read in full.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unverified: Vec<String>,
}

/// Exposure columns for one county. Each group comes from one pack file (named per field).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountyExposure {
    /// Strategic-exposure class (`strategic.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategic_class: Option<StrategicClass>,
    /// Sites or areas that put the county in its class, membership first, then nearest first
    /// (`strategic.csv`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strategic_site_ids: Vec<String>,
    /// Those places, resolved (`strategic_sites.toml`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strategic_places: Vec<StrategicPlace>,
    /// Distance from the county's centre of population to the first distance-based reason, km
    /// (`strategic.csv`; absent for membership reasons and class E).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategic_km: Option<f32>,
    /// Compass bearing from the county to that place, degrees clockwise from north.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategic_bearing: Option<f32>,
    /// The county's share of the national FY2026 UASI total, 0 to 1: its urban area's share
    /// split among the area's counties by population (`strategic.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uasi_share: Option<f32>,
    /// The urban area's own share of the national FY2026 UASI total, 0 to 1, the same for every
    /// county in the area; 0 outside every funded urban area (`strategic.csv`). Source id:
    /// `fema_hsgp_fy2026` (`rr_data::exposure_source("uasi_area_share")`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uasi_area_share: Option<f32>,
    /// The FEMA urban area the county is funded under, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uasi_area: Option<String>,
    /// Geomagnetic latitude of the county's internal point, degrees (IGRF-14; `geomag.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geomag_lat: Option<f32>,
    /// NERC TPL-007 geomagnetic scaling factor alpha, 0.1 to 1 (`geomag.csv`); earth
    /// conductivity (beta) is not included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geomag_factor: Option<f32>,
    /// Smoke days a year with 24-hour PM2.5 of 35.5 µg/m³ or more (unhealthy for sensitive
    /// groups), 2016-2023 mean (`smoke.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoke_days_35: Option<f32>,
    /// Smoke days a year with 55.5 µg/m³ or more (unhealthy), 2016-2023 mean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoke_days_55: Option<f32>,
    /// Least-squares trend of the yearly `smoke_days_35` count, days per year per year.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoke_trend: Option<f32>,
    /// Days a year under medium or heavy satellite-mapped smoke, 2016-2023 mean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hms_smoke_days: Option<f32>,
    /// `monitor`, `imputed` or `hms_only`: how the smoke-day counts were made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoke_basis: Option<String>,
    /// Share of the county's land on karst-prone rock (carbonates and evaporites), 0 to 1
    /// (`ground.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub karst_share: Option<f32>,
    /// Share of the county's area that is landslide-susceptible terrain, 0 to 1 (`ground.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landslide_susceptible_share: Option<f32>,
    /// Share of residents living behind levees, 0 to 1 (`levees.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leveed_pop_share: Option<f32>,
    /// Of those, the share behind levees USACE rates High or Very High risk, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levee_risk_high_share: Option<f32>,
    /// High-hazard-potential dams in the county (`facilities.csv`, `high_hazard_dams`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dams_high_total: Option<u16>,
    /// Of those, dams whose latest condition assessment is Poor or Unsatisfactory
    /// (`facilities.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dams_high_poor_condition: Option<u16>,
    /// Share of people on community water systems served by a system with a health-based
    /// violation in the last five years, 0 to 1 (`water_systems.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdwis_violation_pop_share: Option<f32>,
    /// Community water systems' population ÷ county population, capped at 1 (the rest are
    /// mostly on private wells, which the violation data never see).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cws_pop_share: Option<f32>,
    /// County-level storm-surge proxy (`surge_proxy.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surge_proxy_class: Option<SurgeProxyClass>,
    /// Share of residents in NRI's coastal-flood exposure areas, 0 to 1 (`surge_proxy.csv`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coastal_flood_pop_share: Option<f32>,
    /// Eviction filings per renter household per year, 2014-2018 mean (`eviction.csv`, present
    /// only once the owner has approved the ODC-BY attribution licence).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eviction_filing_rate: Option<f32>,
}

impl CountyExposure {
    /// True when no exposure file contributed anything.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Wildfire exposure of one Census place overlapping a ZIP code (optional pack
/// `wildfire_places`, USFS Wildfire Risk to Communities).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceWildfire {
    /// Census place GEOID (seven digits).
    pub place: String,
    /// Place name.
    pub name: String,
    /// Share of the ZIP code's land inside the place, 0 to 1.
    pub zip_land_share: f32,
    /// Share of the place's buildings directly exposed to wildfire (in burnable land), 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buildings_direct: Option<f32>,
    /// Share indirectly exposed (embers and home-to-home spread), 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buildings_indirect: Option<f32>,
    /// National percentile rank of risk to homes, 0 to 1 (1 = highest).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_national_rank: Option<f32>,
}

/// What the packs know about one ZIP code (ZCTA) beyond the counties it spans.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZipRecord {
    /// Five-digit ZIP code (ZCTA).
    pub zip: String,
    /// Distance from the ZIP's centre to the nearest operating nuclear plant, km.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nearest_nuclear_km: Option<f32>,
    /// Toxics Release Inventory facilities within 5 km of the ZIP's centre.
    #[serde(default)]
    pub tri_within_5km: u16,
    /// High-hazard dams within 10 km of the ZIP's centre whose inventory entry names a town
    /// the ZIP overlaps as the place a failure would flood.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dams_high_within_10km_naming_town: Option<u16>,
    /// Distance to the nearest class A or C1 strategic point site within 150 km, km.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategic_km: Option<f32>,
    /// Compass bearing from the ZIP to that site, degrees clockwise from north.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategic_bearing: Option<f32>,
    /// That site's id (resolves in `strategic_sites.toml`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategic_site: Option<String>,
    /// Share of the ZIP's land in NOAA/NHC's Category 1 storm-surge area (optional pack `surge`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surge_cat1_share: Option<f32>,
    /// Share in the Category 3 area (which contains the Category 1 and 2 areas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surge_cat3_share: Option<f32>,
    /// Census places the ZIP overlaps, with their wildfire exposure (optional pack
    /// `wildfire_places`), largest overlap first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wildfire_places: Vec<PlaceWildfire>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn classes_parse_and_order_by_precedence() {
        assert_eq!(StrategicClass::from_str("C1").unwrap(), StrategicClass::C1);
        assert!(StrategicClass::A < StrategicClass::C1 && StrategicClass::C1 < StrategicClass::B);
        assert!(StrategicClass::from_str("F").is_err());
        assert_eq!(
            SurgeProxyClass::from_str("none").unwrap(),
            SurgeProxyClass::NoSurge
        );
    }

    #[test]
    fn empty_exposure_round_trips_to_nothing() {
        let e = CountyExposure::default();
        assert!(e.is_empty());
        assert_eq!(serde_json::to_value(&e).unwrap(), serde_json::json!({}));
        let full = CountyExposure {
            strategic_class: Some(StrategicClass::B),
            strategic_site_ids: vec!["warren_field".into()],
            strategic_km: Some(381.1),
            smoke_days_35: Some(2.5),
            surge_proxy_class: Some(SurgeProxyClass::High),
            ..Default::default()
        };
        let back: CountyExposure =
            serde_json::from_str(&serde_json::to_string(&full).unwrap()).unwrap();
        assert_eq!(back, full);
        assert!(serde_json::from_str::<CountyExposure>(r#"{"smoke_days_36": 1}"#).is_err());
    }
}
