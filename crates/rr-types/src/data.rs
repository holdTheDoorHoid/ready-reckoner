//! Data-pack records: what `rr-etl` writes and `rr-data` loads, for `rr-hazards` and
//! `rr-consequence` to read.
//!
//! These are internal to the engine. The web app fetches pack bytes and hands them to
//! `load_pack` without reading them, so they are not mirrored in `web/src/engine/types.ts`.
//! Nearly everything is optional (missing maps load as empty), so a partial pack still loads and
//! the engine can say what it is missing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CitationId, HazardId, LatLon};

/// Everything the core pack knows about one county.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountyRecord {
    /// Five-digit county FIPS code.
    pub fips: String,
    /// County name without the word "County".
    pub name: String,
    /// Two-letter state abbreviation.
    pub state_abbr: String,
    /// State name.
    pub state_name: String,
    /// The county's centre.
    pub centroid: LatLon,
    /// Fifth National Climate Assessment region id.
    pub nca_region: String,
    /// The county touches the coast.
    pub coastal: bool,
    /// The county has a tsunami hazard zone.
    pub tsunami_zone: bool,
    /// Residents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population: Option<u32>,
    /// Households.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub households: Option<u32>,
    /// Total building value in US dollars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building_value_usd: Option<f64>,
    /// Version of the National Risk Index the `nri` fields come from.
    pub nri_version: String,
    /// National Risk Index fields per natural hazard.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub nri: BTreeMap<HazardId, NriHazard>,
    /// Power outage statistics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outages: Option<OutageStats>,
    /// Event rates keyed by event type (for example boil-water notices).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub events: BTreeMap<String, EventRate>,
    /// Earthquake shaking probabilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seismic: Option<Seismic>,
    /// Climate multipliers for the 2050 dial, keyed by hazard or climate variable.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub climate: BTreeMap<String, f32>,
    /// Flood priors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flood: Option<FloodPriors>,
    /// Nearby facilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facilities: Option<Facilities>,
    /// Social vulnerability measures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vulnerability: Option<Vulnerability>,
}

string_enum! {
    /// What a National Risk Index annualised frequency (`AFREQ`) means for a hazard.
    pub enum AfreqKind: "annualised frequency kind" {
        /// Expected events per year (can exceed 1).
        EventsPerYear = "events_per_year",
        /// Chance of at least one event in a year, 0 to 1.
        AnnualProbability = "annual_probability",
    }
}

/// National Risk Index fields for one hazard in one county. Field names follow the NRI column
/// suffixes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NriHazard {
    /// Annualised frequency (`AFREQ`); see `afreq_kind`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub afreq: Option<f32>,
    /// What `afreq` means for this hazard.
    pub afreq_kind: AfreqKind,
    /// Building value exposed, in US dollars (`EXPB`). Not in the core pack (nothing reads it);
    /// only the hand-built sample counties carry it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expb: Option<f32>,
    /// Population exposed (`EXPP`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expp: Option<f32>,
    /// Expected annual loss to buildings, in US dollars (`EALB`). Not in the core pack (nothing
    /// reads it); only the hand-built sample counties carry it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ealb: Option<f32>,
    /// Expected annual loss of population (`EALP`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ealp: Option<f32>,
    /// Expected annual loss, total, in US dollars (`EALT`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ealt: Option<f32>,
    /// Historic loss ratio for buildings (`HLRB`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hlrb: Option<f32>,
    /// Expected annual loss rate for buildings (`ALRB`). Not in the core pack (nothing reads
    /// it); only the hand-built sample counties carry it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alrb: Option<f32>,
    /// Risk score, 0 to 100 (`RISKS`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_score: Option<f32>,
}

/// Power outage statistics for a county (for example from EAGLE-I).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutageStats {
    /// Outage events per customer per year.
    pub events_per_customer_year: f32,
    /// Share of outage events lasting at least 1 day, 0 to 1.
    pub p_ge_1d: f32,
    /// Share of outage events lasting at least 3 days, 0 to 1.
    pub p_ge_3d: f32,
    /// Share of outage events lasting at least 7 days, 0 to 1.
    pub p_ge_7d: f32,
    /// Share of outage events lasting at least 14 days, 0 to 1.
    pub p_ge_14d: f32,
    /// Median outage duration, in hours.
    pub median_hours: f32,
    /// 90th-percentile outage duration, in hours.
    pub p90_hours: f32,
    /// The years the statistics cover, for example `"2014-2023"`.
    pub years_covered: String,
    /// What counted as an outage event.
    pub event_definition: String,
}

/// How often an event type happens in a county, and how long it lasts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventRate {
    /// Events per year.
    pub rate_per_year: f32,
    /// Share of events that cause damage or disruption, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_damaging: Option<f32>,
    /// Median duration, in days.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub median_days: Option<f32>,
    /// 90th-percentile duration, in days.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p90_days: Option<f32>,
}

/// Earthquake shaking probabilities for a county (USGS hazard model).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seismic {
    /// Yearly chance of peak ground acceleration of at least 0.1 g.
    pub p_pga_ge_0_1g_per_year: f32,
    /// Yearly chance of peak ground acceleration of at least 0.2 g.
    pub p_pga_ge_0_2g_per_year: f32,
    /// Chance of damaging shaking (intensity VI or more) in 100 years, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mmi6_100yr: Option<f32>,
}

/// Flood priors for a county.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloodPriors {
    /// Share of homes in the Special Flood Hazard Area (the 1% annual chance floodplain), 0 to 1.
    pub sfha_home_share: f32,
    /// Flood insurance claims per 1,000 policies per year.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claims_per_1000_policies_year: Option<f32>,
    /// Mean paid claim, in US dollars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_paid_usd: Option<f32>,
    /// How `sfha_home_share` was computed: `"structures"` (residential structure counts in and
    /// out of the flood zone, the normal case) or `"policies_lower_bound"` (OpenFEMA reports zero
    /// flood-zone structures for this county even though it clearly has some, so the share falls
    /// back to insured flood-zone homes ÷ all homes, which understates the true share, since not
    /// everyone in the zone carries flood insurance). Absent when a county's cell in the pack's
    /// `sfha_share_basis` column is empty. Not read by rr-hazards yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sfha_basis: Option<String>,
}

/// Facilities in or near a county that change hazard chances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facilities {
    /// Distance from the county to the nearest nuclear power plant, in kilometres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nearest_nuclear_km: Option<f32>,
    /// Toxics Release Inventory facilities in the county.
    pub tri_facilities: u16,
    /// High-hazard-potential dams in the county.
    pub high_hazard_dams: u16,
}

/// Social vulnerability measures for a county.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vulnerability {
    /// Share of residents with three or more risk factors (Census Community Resilience
    /// Estimates), 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cre_share_3plus_risk_factors: Option<f32>,
    /// Social Vulnerability Index percentile, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub svi_percentile: Option<f32>,
}

/// A national base rate (for societal and personal hazards), with its source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseRate {
    /// Stable id, for example `house_fire_per_household_year`.
    pub id: String,
    /// The rate.
    pub value: f64,
    /// Unit of `value`, for example `"per household per year"`.
    pub unit: String,
    /// Low end of a plausible range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<f64>,
    /// High end of a plausible range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<f64>,
    /// Where it comes from.
    pub source: CitationId,
    /// The year the rate describes.
    pub year: u16,
    /// How it was derived or what it covers.
    pub note: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_json() -> serde_json::Value {
        serde_json::json!({
            "fips": "42101", "name": "Philadelphia", "state_abbr": "PA", "state_name": "Pennsylvania",
            "centroid": { "lat": 40.0, "lon": -75.1 }, "nca_region": "northeast",
            "coastal": false, "tsunami_zone": false, "nri_version": "1.20"
        })
    }

    #[test]
    fn a_partial_record_loads() {
        let r: CountyRecord = serde_json::from_value(minimal_json()).unwrap();
        assert!(r.nri.is_empty() && r.events.is_empty() && r.climate.is_empty());
        assert!(r.outages.is_none() && r.seismic.is_none() && r.population.is_none());
        // Empty maps and absent options are left out again.
        assert_eq!(serde_json::to_value(&r).unwrap(), minimal_json());
    }

    #[test]
    fn a_full_record_round_trips() {
        let mut v = minimal_json();
        v["population"] = 1_550_000.into();
        v["households"] = 640_000.into();
        v["building_value_usd"] = 2.1e11.into();
        v["nri"] = serde_json::json!({
            "heat_wave": { "afreq": 1.5, "afreq_kind": "events_per_year", "expb": 1.0e11, "expp": 1.5e6,
                           "ealb": 2.0e6, "ealp": 0.5, "ealt": 5.0e6, "hlrb": 0.0001, "alrb": 0.00002, "risk_score": 95.5 },
            "earthquake": { "afreq": 0.002, "afreq_kind": "annual_probability" }
        });
        v["outages"] = serde_json::json!({
            "events_per_customer_year": 1.2, "p_ge_1d": 0.1, "p_ge_3d": 0.03, "p_ge_7d": 0.01, "p_ge_14d": 0.002,
            "median_hours": 3.5, "p90_hours": 20.0, "years_covered": "2014-2023", "event_definition": "customers out"
        });
        v["events"] = serde_json::json!({ "boil_water_notice": { "rate_per_year": 0.05, "share_damaging": 1.0, "median_days": 2.0, "p90_days": 6.0 } });
        v["seismic"] = serde_json::json!({ "p_pga_ge_0_1g_per_year": 0.001, "p_pga_ge_0_2g_per_year": 0.0003, "mmi6_100yr": 0.05 });
        v["climate"] = serde_json::json!({ "heat_wave": 1.4 });
        v["flood"] = serde_json::json!({ "sfha_home_share": 0.03, "claims_per_1000_policies_year": 12.0, "mean_paid_usd": 40000.0 });
        v["facilities"] = serde_json::json!({ "nearest_nuclear_km": 45.0, "tri_facilities": 60, "high_hazard_dams": 2 });
        v["vulnerability"] =
            serde_json::json!({ "cre_share_3plus_risk_factors": 0.3, "svi_percentile": 0.9 });
        let r: CountyRecord = serde_json::from_value(v).unwrap();
        assert_eq!(
            r.nri[&HazardId::Earthquake].afreq_kind,
            AfreqKind::AnnualProbability
        );
        assert_eq!(r.nri[&HazardId::HeatWave].risk_score, Some(95.5));
        let again: CountyRecord =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(again, r);
    }

    #[test]
    fn typos_and_unknown_hazards_fail() {
        let mut v = minimal_json();
        v["populaton"] = 5.into();
        assert!(serde_json::from_value::<CountyRecord>(v).is_err());
        let mut v = minimal_json();
        v["nri"] = serde_json::json!({ "hurrican": { "afreq_kind": "events_per_year" } });
        assert!(serde_json::from_value::<CountyRecord>(v).is_err());
        let mut v = minimal_json();
        v["nri"] = serde_json::json!({ "hurricane": { "afreq": 0.1 } });
        assert!(
            serde_json::from_value::<CountyRecord>(v).is_err(),
            "afreq_kind is required"
        );
    }

    #[test]
    fn base_rate_round_trips() {
        let b = BaseRate {
            id: "house_fire_per_household_year".into(),
            value: 0.003,
            unit: "per household per year".into(),
            low: Some(0.002),
            high: None,
            source: "nfpa_home_fires".into(),
            year: 2023,
            note: "Reported home structure fires divided by occupied housing units.".into(),
        };
        let json = serde_json::to_value(&b).unwrap();
        assert!(json.get("high").is_none());
        assert_eq!(serde_json::from_value::<BaseRate>(json).unwrap(), b);
    }
}
