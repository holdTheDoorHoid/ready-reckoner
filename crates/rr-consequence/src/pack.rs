//! Data-pack v2 records this crate reads: the regional outage model, the worst-event stress line,
//! the pooled restoration curves and the temperature shares (the `data-model` workstream), and the
//! drinking-water and smoke columns of the exposure record (the `data-hazard` workstream).
//!
//! awaiting: data-model — these structs mirror `rr_types::calibration` on `agent/data-model`
//! (`OutageModel`, `PoolBasis`, `StressEvent`, `RestorationCurve`, `TemperatureProfile`) field for
//! field, so the JSON is the same and the code that reads them does not change. When that branch
//! merges, replace the definitions below with
//!
//! ```text
//! pub use rr_types::{OutageModel, PoolBasis, RestorationCurve, StressEvent, TemperatureProfile};
//! ```
//!
//! and fill [`crate::CountyData::from_record`] from `CountyRecord::outage_model` and
//! `CountyRecord::temperature` (the lines are written there as comments). The restoration curves
//! are not per county: `rr_data::DataStore::restoration_curves()` serves them, and `rr-plan` hands
//! them in with [`crate::CountyData::with_curves`].

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
/// `causes`). The blend is `lam_ge = z × own_ge + (1 − z) × region`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct OutageModel {
    /// How the pooled tail was formed.
    #[serde(default)]
    pub basis: PoolBasis,
    /// Years of outage records the county's own rates rest on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub years: Option<f32>,
    /// Customer outages per customer per year in the pool, all lengths (the county's own; the
    /// region's when `basis` is `region_only`).
    pub rate: f32,
    /// Blended rate of outages lasting at least 1, 3, 7, 14 and 30 days.
    pub lam_ge: [f32; 5],
    /// The county's own rates at the same lengths (absent when it has no record).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_ge: Option<[f32; 5]>,
    /// Credibility weight of the county's own record at each length, 0 to 1.
    pub z: [f32; 5],
    /// Neighbouring counties with records inside the pooling radius.
    #[serde(default)]
    pub region_counties: u32,
    /// Share of the county's recorded customer outages by cause (`hurricane`, `ice`, `winter`,
    /// `wind`, `wildfire`, `heat`, `cold_grid`, `flood`, `grid`, `unattributed`), all lengths.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub causes: BTreeMap<String, f32>,
    /// Share of the county's customer outages lasting a day or more that came from the causes
    /// outside the pool.
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
    /// The same two shares over the county's region (customer-hour weighted, within 250 km),
    /// steadier for small counties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_outage_hot_share: Option<f32>,
    /// See `region_outage_hot_share`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_outage_cold_share: Option<f32>,
}

/// The outage-pooling region of a county, as `rr_data::outage_region` names it: its NCA5 region,
/// except that Puerto Rico and the US Virgin Islands (separate grids) are regions of their own.
pub fn outage_region(state_abbr: &str, nca_region: &str) -> String {
    match state_abbr {
        "PR" => "puerto_rico".to_owned(),
        "VI" => "virgin_islands".to_owned(),
        _ => nca_region.to_owned(),
    }
}

/// The pooled restoration curve for `region` and `class`, if the pack has it.
pub fn curve<'c>(
    curves: &'c [RestorationCurve],
    region: &str,
    class: &str,
) -> Option<&'c RestorationCurve> {
    curves
        .iter()
        .find(|c| c.region == region && c.class == class)
}

/// A hand-copied historic curve (`historic:<id>`) for `region`, if the pack has one: Hurricane
/// Maria for Puerto Rico, Irma and Maria for the US Virgin Islands (island grids, M-10).
pub fn historic_curve<'c>(
    curves: &'c [RestorationCurve],
    region: &str,
) -> Option<&'c RestorationCurve> {
    curves
        .iter()
        .find(|c| c.region == region && c.class.starts_with("historic:"))
}
