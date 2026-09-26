//! Default mapping from hazards to the climate variables in `climate.csv`.
//!
//! The data layer publishes ratios for climate variables (projected / present); which variable
//! stands for which hazard is a modelling choice that belongs to `rr-hazards`. This table is a
//! documented default so every consumer starts from the same place: the first variable that has a
//! value for the county is used, the result is clamped to [`CLIMATE_CLAMP`], and hazards with no
//! physical analogue in these sources get 1 (no change). `rr-hazards` may override it.

use crate::DataStore;
use rr_types::{ClimateHorizon, HazardId};

/// Multipliers are kept within this range so a ratio built on a tiny baseline cannot dominate.
pub const CLIMATE_CLAMP: (f32, f32) = (0.2, 5.0);

/// Climate variables that stand for a hazard, in order of preference (empty = no multiplier).
pub fn variables_for(hazard: HazardId) -> &'static [&'static str] {
    use HazardId::*;
    match hazard {
        HeatWave => &["hot_days_95f", "hot_days_90f_mid45"],
        ColdWave => &["very_cold_nights_0f", "freezing_nights"],
        RiverineFlooding => &["extreme_rain_days", "heavy_rain_days_1in_mid45"],
        Drought => &["consecutive_dry_days_mid45", "dry_days_mid45"],
        Wildfire => &["consecutive_dry_days_mid45", "hot_days_90f_mid45"],
        WinterWeather | IceStorm => &["freezing_nights"],
        _ => &[],
    }
}

impl DataStore {
    /// Climate multiplier for a hazard in a county: 1 for today's climate; for 2050 the first
    /// available ratio from [`variables_for`], clamped to [`CLIMATE_CLAMP`]; 1 when there is no
    /// analogue or no data (for example outside the contiguous US).
    pub fn climate_multiplier(&self, fips: &str, hazard: HazardId, horizon: ClimateHorizon) -> f32 {
        if horizon == ClimateHorizon::Today {
            return 1.0;
        }
        let Some(rec) = self.county(fips) else {
            return 1.0;
        };
        variables_for(hazard)
            .iter()
            .find_map(|v| rec.climate.get(*v).copied())
            .map(|r| r.clamp(CLIMATE_CLAMP.0, CLIMATE_CLAMP.1))
            .unwrap_or(1.0)
    }
}
