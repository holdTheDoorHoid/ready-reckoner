//! Data-pack v2 calibration records (the `data-model` workstream): the regional outage model,
//! the worst-event stress line, pooled restoration curves, temperature shares, utility
//! reliability and disaster declarations.
//!
//! Like everything in [`crate::data`], these are internal to the engine: `rr-etl` writes them,
//! `rr-data` loads them, `rr-hazards` and `rr-consequence` read them. Every field is optional
//! or defaulted so a pack without the new files still loads.
//!
//! The output types they fill, [`crate::StressTest`] and [`crate::RecoveryInfo`], are the
//! contract's; these are the pack records behind them. Field names follow DESIGN-DELTA §1.3.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Outage lengths, in days, of the pooled tails ([`OutageModel::lam_ge`] and friends).
pub const OUTAGE_MARKS_DAYS: [f32; 5] = [1.0, 3.0, 7.0, 14.0, 30.0];

/// How a county's pooled tail was formed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PoolBasis {
    /// The county's own record blended with its region's (the normal case).
    #[default]
    Blend,
    /// No usable record of its own: the region's rates.
    RegionOnly,
    /// No neighbour with records within the pooling radius: the county's own rates.
    OwnOnly,
}

/// The regional outage model for one county (`core/outage_pooled.csv`, `core/outage_causes.csv`,
/// `core/outage_stress.csv`).
///
/// Rates count customer outages lasting at least N days per customer per year, for the outages
/// the model does not carry in rows of their own: outages attributed to hurricanes, wildfires,
/// floods, cold-driven grid emergencies and other grid failures are left out (their shares are in
/// `causes`). The blend is
/// `lam_ge = z × own_ge + (1 − z) × region`, so the region's rate is
/// `(lam_ge − z × own_ge) / (1 − z)` when `z < 1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct OutageModel {
    /// How the pooled tail was formed.
    #[serde(default)]
    pub basis: PoolBasis,
    /// Years of outage records the county's own rates rest on. Not in the core pack (the
    /// county's `outages.years_covered` says the same); kept for fixtures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub years: Option<f32>,
    /// Customer outages per customer per year in the pool, all lengths (the county's own; the
    /// region's when `basis` is `region_only`).
    pub rate: f32,
    /// Blended rate of outages lasting at least 1, 3, 7, 14 and 30 days.
    pub lam_ge: [f32; 5],
    /// The county's own rates at the same lengths. Not in the core pack (it would add 40 KB to
    /// every first visit); kept for hand-built fixtures and tests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_ge: Option<[f32; 5]>,
    /// Credibility weight of the county's own record at each length, 0 to 1:
    /// `E / (E + 5)`, E = events of that length the county would expect to record.
    pub z: [f32; 5],
    /// Neighbouring counties with records inside the pooling radius.
    #[serde(default)]
    pub region_counties: u32,
    /// Share of the county's recorded customer outages by cause (`hurricane`, `ice`, `winter`,
    /// `wind`, `wildfire`, `heat`, `cold_grid`, `flood`, `grid`, `unattributed`), all lengths.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub causes: BTreeMap<String, f32>,
    /// Share of the county's customer outages lasting a day or more that came from each cause
    /// outside the pool (`hurricane`, `wildfire`, `flood`, `cold_grid`, `grid`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub causes_ge_1d: BTreeMap<String, f32>,
    /// The worst outage event in the county's region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stress: Option<StressEvent>,
}

/// The worst outage event in a county's region (`core/outage_stress.csv`): the event with the
/// largest share of customers still out a week after the peak, as recorded where it was worst.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StressEvent {
    /// Display name ("Hurricane Helene", "August 2020 Midwest derecho").
    pub event: String,
    /// Cause class.
    pub class: String,
    /// HURDAT2 storm id or Storm Events type (empty when not recorded).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cause: String,
    /// Date the outage began (`YYYY-MM-DD`, UTC).
    pub date: String,
    /// County where the curve was recorded (FIPS); absent for hand-copied events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_in: Option<String>,
    /// Distance from this county to `recorded_in`, km.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_km: Option<f32>,
    /// Share of the recording county's customers out at the peak.
    pub peak_share: f32,
    /// (days after the peak, share of the peak still out) at 1, 3, 7, 14 and 30 days.
    pub share_out_at_days: Vec<(f32, f32)>,
    /// `eaglei` or `historic:<id>` (hand-copied, pre-2014).
    pub source: String,
}

/// A pooled restoration curve for one region and cause (`core/outage_curves.csv`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestorationCurve {
    /// Pooling region (NCA5 region id, `puerto_rico`, `virgin_islands` or `mainland`).
    pub region: String,
    /// Cause class, `all`, or `historic:<id>` for a hand-copied event.
    pub class: String,
    /// Major county events pooled.
    pub events: u32,
    /// (days after the peak, peak-weighted mean share of the peak still out).
    pub share_out_at_days: Vec<(f32, f32)>,
    /// Peak-weighted median days until half of the peak is back.
    pub t50_days: f32,
    /// Peak-weighted median days until nine in ten are back.
    pub t90_days: f32,
    /// `t90_days` over the mainland's for the same cause, bounded 0.5 to 5 (M-10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub factor: Option<f32>,
}

/// Heat and cold day shares by month (NOAA nClimGrid-Daily, 1991-2020) and during recorded
/// outages (2014-2025) for one county (`core/temperature.csv`). Contiguous US only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TemperatureProfile {
    /// Share of days in each month (January first) with a high of 90 °F or more.
    pub tmax_ge_90f: [f32; 12],
    /// Share of days in each month with a high of 100 °F or more.
    pub tmax_ge_100f: [f32; 12],
    /// Share of days in each month with a low of 20 °F or less.
    pub tmin_le_20f: [f32; 12],
    /// Share of days in each month with a low of 0 °F or less.
    pub tmin_le_0f: [f32; 12],
    /// Share of the county's outage customer-hours (2014-2025) on days with a high of 95 °F or
    /// more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outage_hot_share: Option<f32>,
    /// Share of the county's outage customer-hours on days with a low of 20 °F or less.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outage_cold_share: Option<f32>,
    /// The same two shares over the county's region (customer-hour weighted, within 400 km),
    /// steadier for small counties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_outage_hot_share: Option<f32>,
    /// See `region_outage_hot_share`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_outage_cold_share: Option<f32>,
}

/// Utility reliability indices for the utilities serving a county (EIA-861), customer-weighted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Reliability {
    /// Minutes without power per customer per year, including major event days (SAIDI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saidi_with_med_min: Option<f32>,
    /// Interruptions per customer per year, including major event days (SAIFI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saifi_with_med: Option<f32>,
    /// SAIDI without major event days, minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saidi_without_med_min: Option<f32>,
    /// SAIFI without major event days.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saifi_without_med: Option<f32>,
    /// Utility-years averaged.
    #[serde(default)]
    pub utility_years: u32,
}

/// Federal disaster declarations covering a county (OpenFEMA).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Declarations {
    /// Major-disaster declarations in the five full years before the pack was built.
    pub last_5yr: u16,
    /// Major-disaster declarations since 2000.
    pub since_2000: u16,
    /// Of those, with Individual Assistance (help to households).
    pub with_individual_assistance: u16,
    /// Of those, for hurricanes, tropical storms, coastal storms or floods.
    pub hurricane_or_flood: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_and_default() {
        let m = OutageModel {
            basis: PoolBasis::Blend,
            years: Some(11.5),
            rate: 1.2,
            lam_ge: [0.1, 0.03, 0.01, 0.002, 0.0001],
            own_ge: Some([0.12, 0.02, 0.0, 0.0, 0.0]),
            z: [0.9, 0.6, 0.2, 0.05, 0.01],
            region_counties: 87,
            causes: BTreeMap::from([("wind".to_string(), 0.6), ("ice".to_string(), 0.4)]),
            causes_ge_1d: BTreeMap::new(),
            stress: Some(StressEvent {
                event: "Hurricane Helene".into(),
                class: "hurricane".into(),
                cause: "AL092024".into(),
                date: "2024-09-27".into(),
                recorded_in: Some("37021".into()),
                distance_km: Some(0.0),
                peak_share: 0.64,
                share_out_at_days: vec![(1.0, 0.92), (7.0, 0.69)],
                source: "eaglei".into(),
            }),
        };
        let j = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<OutageModel>(&j).unwrap(), m);
        assert!(j.contains("\"basis\":\"blend\""));
        let t = TemperatureProfile::default();
        assert_eq!(
            serde_json::from_str::<TemperatureProfile>(&serde_json::to_string(&t).unwrap())
                .unwrap(),
            t
        );
        assert!(serde_json::from_str::<Declarations>("{\"last_5yr\":1}").is_err());
    }
}
