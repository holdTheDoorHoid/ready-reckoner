//! Climate multipliers for the "around 2050" dial (research §5.3).
//!
//! Principles (research §5.1, §5.4): change how often hazards happen, never how long restoration
//! takes; use county projections with the scenario range (SSP2-4.5 as the central case, SSP5-8.5
//! as the high case, or +2 °C / +3 °C warming levels); threshold counts such as "days over 95 °F"
//! change additively, not as ratios; no adjustment at all for earthquakes, tsunamis, volcanoes, or
//! societal and personal hazards; tornado, hail, ice-storm and windstorm frequency changes are
//! "unclear" and left at ×1; never stack multipliers on one hazard.
//!
//! # Keys read from `CountyRecord::climate`
//!
//! A pack may give a finished multiplier per hazard, which wins:
//!
//! | key | meaning |
//! | --- | --- |
//! | `<hazard_id>` (for example `heat_wave`) | multiplier, central scenario |
//! | `<hazard_id>_high` | multiplier, high scenario (optional) |
//!
//! Otherwise the multiplier is computed from county climate variables, each given as
//! `<variable>_hist` (the model's historical baseline), `<variable>_2050` (central scenario,
//! 2036–2065) and optionally `<variable>_2050_high` (high scenario):
//!
//! | variable | unit | used for |
//! | --- | --- | --- |
//! | `days_over_95f` | days a year with a high above 95 °F | heat waves: event-days + change, capped ×3 |
//! | `days_over_90f` (`_hist` only) | days a year above 90 °F | caps today's heat-wave days (see `natural`) |
//! | `days_over_2in` | days a year with more than 2 in. of rain | inland flooding, ratio, for homes at ground level |
//! | `icing_days` | days a year that stay below freezing | cold waves and winter storms, ratio, floor ×0.5 |
//! | `dry_spell_days` | longest run of dry days | drought and wildfire, ratio, ×1 to ×2 |

use rr_types::{CountyRecord, HazardId};

use crate::cite;
use crate::estimate::Estimate;
use crate::params::{
    COLD_MULTIPLIER_FLOOR, DRY_SPELL_MULTIPLIER_BOUNDS, HEAT_MULTIPLIER_CAP,
    PRECIP_MULTIPLIER_BOUNDS,
};

/// Ratios of counts smaller than this (days a year) are too unstable to use.
const MIN_BASELINE_DAYS: f64 = 0.5;

/// What the 2050 dial does to one hazard.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Climate {
    /// A projection applies: multiply the rate by `multiplier` (range = scenario range).
    Projected {
        /// Central value, with low and high across scenarios.
        multiplier: Estimate,
        /// Plain words for the note, for example "heat waves ×2.4 to ×3.0".
        what: String,
    },
    /// The research finds a change unclear or not defensible: ×1.
    Unclear,
    /// Never adjusted (earthquakes, tsunamis, volcanoes, societal and personal hazards).
    NotApplicable,
    /// A projection would apply but the county record does not have it: ×1.
    NoData,
    /// The baseline count is too small for a stable ratio (for example almost no freezing days
    /// in Miami): ×1.
    TooFew,
    /// The household is not exposed to the part that changes (heavier rain on a home above the
    /// ground floor): ×1.
    NotExposed,
}

impl Climate {
    /// The multiplier to apply (1 unless projected).
    pub fn multiplier(&self) -> Estimate {
        match self {
            Climate::Projected { multiplier, .. } => multiplier.clone(),
            _ => Estimate::exact(1.0),
        }
    }
}

fn get(county: &CountyRecord, key: &str) -> Option<f64> {
    county
        .climate
        .get(key)
        .map(|v| f64::from(*v))
        .filter(|v| v.is_finite())
}

/// `(central, high)` values of a variable at 2050, and its baseline.
fn variable(county: &CountyRecord, var: &str) -> Option<(f64, f64, f64)> {
    let hist = get(county, &format!("{var}_hist"))?;
    let mid = get(county, &format!("{var}_2050"))?;
    let high = get(county, &format!("{var}_2050_high")).unwrap_or(mid);
    Some((hist, mid, high))
}

/// A multiplier estimate from central and high values, clamped.
fn multiplier(mid: f64, high: f64, lo_bound: f64, hi_bound: f64) -> Estimate {
    let c = |x: f64| x.clamp(lo_bound, hi_bound);
    let (m, h) = (c(mid), c(high));
    Estimate::data(m, m.min(h), m.max(h), &[cite::CMRA])
}

fn describe(label: &str, m: &Estimate) -> String {
    if (m.high - m.low).abs() < 0.005 {
        format!("{label} ×{:.2}", m.value)
    } else {
        format!("{label} ×{:.2} to ×{:.2}", m.low, m.high)
    }
}

/// A pack-supplied multiplier for `hazard`, if any.
fn direct(county: &CountyRecord, hazard: HazardId, label: &str) -> Option<Climate> {
    let mid = get(county, hazard.as_str())?;
    let high = get(county, &format!("{}_high", hazard.as_str())).unwrap_or(mid);
    let (lo_bound, hi_bound) = match hazard {
        HazardId::HeatWave => (1.0, HEAT_MULTIPLIER_CAP),
        HazardId::ColdWave | HazardId::WinterWeather => (COLD_MULTIPLIER_FLOOR, 1.5),
        HazardId::Drought | HazardId::Wildfire => DRY_SPELL_MULTIPLIER_BOUNDS,
        _ => PRECIP_MULTIPLIER_BOUNDS,
    };
    let m = multiplier(mid, high, lo_bound, hi_bound);
    Some(Climate::Projected {
        what: describe(label, &m),
        multiplier: m,
    })
}

/// The climate treatment of `hazard` in `county`.
///
/// `heat_event_days` is today's heat-wave event-days a year (after the cap in `natural`); it is
/// the base the additive change in hot days is added to. `ground_level` is false for homes on
/// the second floor or higher, which do not get the heavy-rain flood multiplier.
pub(crate) fn treatment(
    county: &CountyRecord,
    hazard: HazardId,
    heat_event_days: f64,
    ground_level: bool,
) -> Climate {
    use HazardId::*;
    match hazard {
        HeatWave => direct(county, hazard, "heat waves").unwrap_or_else(|| {
            match variable(county, "days_over_95f") {
                Some((hist, mid, high)) if heat_event_days > 0.0 => {
                    let base = heat_event_days.max(MIN_BASELINE_DAYS);
                    let m = multiplier(
                        1.0 + (mid - hist).max(0.0) / base,
                        1.0 + (high - hist).max(0.0) / base,
                        1.0,
                        HEAT_MULTIPLIER_CAP,
                    );
                    Climate::Projected {
                        what: describe("heat waves", &m),
                        multiplier: m,
                    }
                }
                _ => Climate::NoData,
            }
        }),
        RiverineFlooding => {
            if !ground_level {
                return Climate::NotExposed;
            }
            direct(county, hazard, "flooding from heavy rain").unwrap_or_else(|| {
                match variable(county, "days_over_2in") {
                    Some((hist, _, _)) if hist < 0.05 => Climate::TooFew,
                    Some((hist, mid, high)) => {
                        let (lo, hi) = PRECIP_MULTIPLIER_BOUNDS;
                        let m = multiplier(mid / hist, high / hist, lo, hi);
                        Climate::Projected {
                            what: describe("flooding from heavy rain", &m),
                            multiplier: m,
                        }
                    }
                    _ => Climate::NoData,
                }
            })
        }
        ColdWave | WinterWeather => {
            let label = if hazard == ColdWave {
                "cold waves"
            } else {
                "winter storms"
            };
            direct(county, hazard, label).unwrap_or_else(|| match variable(county, "icing_days") {
                Some((hist, _, _)) if hist < MIN_BASELINE_DAYS => Climate::TooFew,
                Some((hist, mid, high)) => {
                    let m = multiplier(mid / hist, high / hist, COLD_MULTIPLIER_FLOOR, 1.5);
                    Climate::Projected {
                        what: describe(label, &m),
                        multiplier: m,
                    }
                }
                _ => Climate::NoData,
            })
        }
        Drought | Wildfire => {
            let label = if hazard == Drought {
                "drought"
            } else {
                "wildfire"
            };
            direct(county, hazard, label).unwrap_or_else(|| {
                match variable(county, "dry_spell_days") {
                    Some((hist, _, _)) if hist < 1.0 => Climate::TooFew,
                    Some((hist, mid, high)) => {
                        let (lo, hi) = DRY_SPELL_MULTIPLIER_BOUNDS;
                        let m = multiplier(mid / hist, high / hist, lo, hi);
                        Climate::Projected {
                            what: describe(label, &m),
                            multiplier: m,
                        }
                    }
                    _ => Climate::NoData,
                }
            })
        }
        // Sea-level rise needs its own projection (research §5.3); only a pack-supplied
        // multiplier is used.
        CoastalFlooding => {
            direct(county, hazard, "flooding from the sea").unwrap_or(Climate::NoData)
        }
        // Hurricanes: the share of major storms rises; frequency does not change. Handled where
        // the hurricane rate is split (see `natural`).
        Hurricane => Climate::NotApplicable,
        Tornado | Hail | IceStorm | StrongWind | Lightning | Landslide | Avalanche => {
            Climate::Unclear
        }
        Earthquake | Tsunami | VolcanicActivity => Climate::NotApplicable,
        _ => Climate::NotApplicable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn county(climate: &[(&str, f32)]) -> CountyRecord {
        let mut c: CountyRecord = serde_json_minimal();
        c.climate = climate
            .iter()
            .map(|(k, v)| ((*k).to_owned(), *v))
            .collect::<BTreeMap<_, _>>();
        c
    }

    fn serde_json_minimal() -> CountyRecord {
        CountyRecord {
            fips: "42101".into(),
            name: "Philadelphia".into(),
            state_abbr: "PA".into(),
            state_name: "Pennsylvania".into(),
            centroid: rr_types::LatLon {
                lat: 40.0,
                lon: -75.1,
            },
            nca_region: "northeast".into(),
            coastal: false,
            tsunami_zone: false,
            population: None,
            households: None,
            building_value_usd: None,
            nri_version: "1.20.0".into(),
            nri: BTreeMap::new(),
            outages: None,
            events: BTreeMap::new(),
            seismic: None,
            climate: BTreeMap::new(),
            flood: None,
            facilities: None,
            vulnerability: None,
        }
    }

    #[test]
    fn philadelphia_heat_adds_the_change_in_hot_days() {
        // CMRA 2025: days above 95 °F 3.28 -> 19.04 (SSP2-4.5) -> 26.58 (SSP5-8.5); NRI heat
        // event-days 11.07. (11.07 + 15.77) / 11.07 = 2.42; the high case caps at 3.
        let c = county(&[
            ("days_over_95f_hist", 3.277),
            ("days_over_95f_2050", 19.04),
            ("days_over_95f_2050_high", 26.58),
        ]);
        let Climate::Projected { multiplier: m, .. } =
            treatment(&c, HazardId::HeatWave, 11.07, true)
        else {
            panic!("expected a projection")
        };
        assert!(
            (m.value - (11.07 + 19.04 - 3.277) / 11.07).abs() < 1e-3,
            "{}",
            m.value
        );
        assert_eq!(m.high, 3.0);
        assert!(m.low >= 1.0);
    }

    #[test]
    fn philadelphia_heavy_rain_ratio_matches_research() {
        // CMRA: days over 2 in. 1.471 -> 2.012 -> 2.191, a ratio of 1.37 to 1.49 (research
        // §5.3 quotes 1.33–1.47 from rounded inputs).
        let c = county(&[
            ("days_over_2in_hist", 1.471),
            ("days_over_2in_2050", 2.012),
            ("days_over_2in_2050_high", 2.191),
        ]);
        let m = treatment(&c, HazardId::RiverineFlooding, 0.0, true).multiplier();
        assert!((m.value - 1.368).abs() < 0.002, "{}", m.value);
        assert!((m.high - 1.489).abs() < 0.002, "{}", m.high);
        // An upper-floor flat is not flood-exposed: no multiplier.
        assert_eq!(
            treatment(&c, HazardId::RiverineFlooding, 0.0, false),
            Climate::NotExposed
        );
    }

    #[test]
    fn cold_multipliers_fall_but_not_below_the_floor() {
        // Philadelphia icing days 14.23 -> 3.89 -> 2.42: ratios 0.27 and 0.17, floored at 0.5.
        let c = county(&[
            ("icing_days_hist", 14.23),
            ("icing_days_2050", 3.885),
            ("icing_days_2050_high", 2.418),
        ]);
        for h in [HazardId::ColdWave, HazardId::WinterWeather] {
            let m = treatment(&c, h, 0.0, true).multiplier();
            assert_eq!((m.value, m.low, m.high), (0.5, 0.5, 0.5));
        }
    }

    #[test]
    fn dry_spell_scales_drought_and_wildfire_never_below_one() {
        // Coos: longest dry spell 52.96 -> 60.57 -> 61.91 days.
        let c = county(&[
            ("dry_spell_days_hist", 52.96),
            ("dry_spell_days_2050", 60.57),
            ("dry_spell_days_2050_high", 61.91),
        ]);
        let m = treatment(&c, HazardId::Drought, 0.0, true).multiplier();
        assert!((m.value - 60.57 / 52.96).abs() < 1e-3);
        let shorter = county(&[("dry_spell_days_hist", 33.3), ("dry_spell_days_2050", 31.6)]);
        assert_eq!(
            treatment(&shorter, HazardId::Wildfire, 0.0, true)
                .multiplier()
                .value,
            1.0
        );
    }

    #[test]
    fn a_direct_multiplier_wins_and_is_clamped() {
        let c = county(&[("heat_wave", 4.0), ("days_over_95f_hist", 1.0)]);
        let m = treatment(&c, HazardId::HeatWave, 5.0, true).multiplier();
        assert_eq!(m.value, 3.0);
    }

    #[test]
    fn never_adjusted_and_unclear_hazards() {
        let c = county(&[("earthquake", 2.0), ("tornado", 2.0)]);
        for h in [
            HazardId::Earthquake,
            HazardId::Tsunami,
            HazardId::JobLoss,
            HazardId::Pandemic,
        ] {
            assert_eq!(treatment(&c, h, 0.0, true), Climate::NotApplicable, "{h}");
        }
        for h in [HazardId::Tornado, HazardId::Hail, HazardId::IceStorm] {
            assert_eq!(treatment(&c, h, 0.0, true), Climate::Unclear, "{h}");
        }
        // Missing data is reported, not guessed.
        assert_eq!(
            treatment(&county(&[]), HazardId::HeatWave, 5.0, true),
            Climate::NoData
        );
        // Miami has no freezing days: too few to project a ratio.
        let warm = county(&[("icing_days_hist", 0.0), ("icing_days_2050", 0.0)]);
        assert_eq!(
            treatment(&warm, HazardId::ColdWave, 0.0, true),
            Climate::TooFew
        );
    }
}
