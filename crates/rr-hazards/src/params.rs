//! Every number this crate uses that does not come from the county record or the base-rate pack,
//! with its plausible range and where it comes from.
//!
//! Values are `(value, low, high)`. Items marked **PRIOR** are expert judgement, from the
//! risk-model research or added by this crate, each listed with its reasoning in
//! `docs/RISK_MODEL.md` § "Hazard rates" and cited as `rr_risk_model_priors`. The app shows them
//! as estimates. Items marked **DATA** or **DERIVED** cite the source they come from.

use rr_types::{IncomeStability, Setting};

use crate::cite;
use crate::estimate::Estimate;

/// `(value, low, high)`.
pub(crate) type Triple = (f64, f64, f64);

/// An expert estimate with its range.
pub(crate) fn prior(t: Triple, sources: &[&str]) -> Estimate {
    Estimate::prior(t.0, t.1, t.2, sources)
}

/// A data-based number with its range.
pub(crate) fn data(t: Triple, sources: &[&str]) -> Estimate {
    Estimate::data(t.0, t.1, t.2, sources)
}

// ---------------------------------------------------------------------------------------------
// Event-days to episodes. NRI v1.20 counts heat waves, cold waves and winter weather in
// event-days (research §1.1 caveat 1; NRI update documentation §8 for lightning). One household
// "event" is an episode of several days.
// ---------------------------------------------------------------------------------------------

/// PRIOR. Days in a typical heat-wave episode (heat advisories usually run 2–5 days).
pub(crate) const HEAT_EPISODE_DAYS: Triple = (3.0, 2.0, 5.0);
/// PRIOR. Days in a typical cold-wave episode.
pub(crate) const COLD_EPISODE_DAYS: Triple = (2.0, 1.0, 4.0);
/// PRIOR. Days in a typical winter-storm episode.
pub(crate) const WINTER_EPISODE_DAYS: Triple = (1.5, 1.0, 3.0);

// ---------------------------------------------------------------------------------------------
// Footprints: the chance that one county event reaches this household (research §3.1, a_h).
// ---------------------------------------------------------------------------------------------

/// PRIOR. Share of county windstorm event-days that cut this household's power or damage the
/// home, for a suburban home (scaled by [`utility_exposure`]). Calibrated so an urban Philadelphia
/// rowhouse sees about 0.2 windstorm outages a year, in line with PECO's 2024 major-event outage
/// frequency and the EAGLE-I county events in research §8.2.
pub(crate) const STRONG_WIND_FOOTPRINT: Triple = (0.06, 0.02, 0.2);
/// PRIOR. Share of county ice-storm events that cut power or keep this household home.
pub(crate) const ICE_STORM_FOOTPRINT: Triple = (0.1, 0.03, 0.3);
/// PRIOR. Share of lightning days on which lightning damages the home or cuts its power.
/// Lightning insurance claims run at about 5 in 10,000 homes a year nationally; brief local
/// outages from strikes on lines add to that.
pub(crate) const LIGHTNING_FOOTPRINT: Triple = (1.0e-4, 3.0e-5, 3.0e-4);
/// PRIOR. Share of winter-storm episodes that keep this household home or cut its power
/// (research §8 prototype: about one "snowed in" day every two years in Philadelphia).
pub(crate) const WINTER_FOOTPRINT: Triple = (0.05, 0.02, 0.15);
/// PRIOR. Share of hurricane and tropical-storm passages (Category 1–2 class) that cut this
/// household's power or close roads. PECO lost power to 20–53 % of customers in the five
/// tropical storms since 1999 (research S36).
pub(crate) const HURRICANE_FOOTPRINT: Triple = (0.5, 0.3, 0.8);
/// PRIOR. Share of major-hurricane passages (Category 3+) that reach this household.
pub(crate) const MAJOR_HURRICANE_FOOTPRINT: Triple = (0.8, 0.5, 1.0);
/// PRIOR (informed by HURDAT2): share of hurricanes affecting a Gulf or Atlantic county that are
/// major (Category 3+). About a third of US landfalling hurricanes since 1851 were major.
pub(crate) const MAJOR_HURRICANE_SHARE: Triple = (1.0 / 3.0, 0.2, 0.45);
/// PRIOR. Mean damage to a home that hail damages, as a share of its value (a roof and siding
/// claim on a typical home). The footprint is NRI's historic loss ratio divided by this.
pub(crate) const HAIL_DAMAGE_RATIO: Triple = (0.02, 0.01, 0.05);
/// PRIOR. Mean damage to a home in a tornado's damage path, as a share of its value.
pub(crate) const TORNADO_DAMAGE_RATIO: Triple = (0.25, 0.1, 0.5);
/// PRIOR. Homes disrupted (power, debris, closed roads) per home damaged by a tornado.
pub(crate) const TORNADO_DISRUPTION: Triple = (5.0, 2.0, 10.0);
/// PRIOR. Mean damage to a home a landslide reaches, as a share of its value.
pub(crate) const LANDSLIDE_DAMAGE_RATIO: Triple = (0.3, 0.1, 0.6);
/// PRIOR. Homes cut off (road closed) per home damaged by a landslide.
pub(crate) const LANDSLIDE_ACCESS: Triple = (10.0, 3.0, 30.0);
/// PRIOR. Share of recorded tsunami events that bring a warning to leave the inundation zone
/// (most distant-source events are small surges with an advisory only).
pub(crate) const TSUNAMI_WARNING_SHARE: Triple = (0.3, 0.1, 0.6);
/// PRIOR. Share of volcanic events that reach an exposed household with ash or mudflows.
pub(crate) const VOLCANO_SHARE: Triple = (0.5, 0.2, 1.0);
/// PRIOR. Households told to leave (warning or order) per home that burns, near a wildfire.
pub(crate) const WILDFIRE_EVACUATIONS_PER_BURN: Triple = (20.0, 5.0, 50.0);
/// PRIOR (research §8 prototype, Coos Bay): wildfire safety power shutoffs per year for a rural
/// household in a western state whose utility uses them.
pub(crate) const PSPS_RATE: Triple = (0.02, 0.005, 0.1);
/// PRIOR. Fallback footprints when the county record lacks the loss ratio or exposure field.
pub(crate) const HAIL_FALLBACK_FOOTPRINT: Triple = (0.005, 0.001, 0.02);
/// PRIOR. See [`HAIL_FALLBACK_FOOTPRINT`].
pub(crate) const TORNADO_FALLBACK_FOOTPRINT: Triple = (0.005, 0.001, 0.02);
/// PRIOR. Share of a county's residents exposed to a sub-county hazard when NRI gives no
/// exposure (avalanche, landslide, volcano, wildfire, tsunami).
pub(crate) const EXPOSURE_FALLBACK_SHARE: Triple = (0.01, 0.001, 0.05);
/// PRIOR. Landslide footprint (damage or cut-off road) for an exposed home when NRI gives no
/// loss ratio.
pub(crate) const LANDSLIDE_FALLBACK_FOOTPRINT: Triple = (0.001, 0.000_2, 0.005);
/// PRIOR. Share of homes in the Special Flood Hazard Area when neither the NFIP share nor the
/// NRI inland-flood exposure is available.
pub(crate) const SFHA_SHARE_FALLBACK: Triple = (0.05, 0.01, 0.15);
/// PRIOR. Yearly chance of a tsunami from a nearby source for someone in a tsunami zone outside
/// the Cascadia coast (Alaska, Hawaii, California south of Cape Mendocino, the Caribbean).
pub(crate) const LOCAL_TSUNAMI_OTHER: Triple = (0.001, 0.000_3, 0.003);
/// PRIOR. Share of a tsunami-zone county's residents who live in the inundation zone, when NRI
/// gives no tsunami exposure.
pub(crate) const TSUNAMI_ZONE_SHARE_FALLBACK: Triple = (0.1, 0.02, 0.3);

/// How far the recorded frequency may be off, as a factor either way. NRI annualised
/// frequencies come from 20–40 year records (research §1.1 caveat 5).
pub(crate) const NRI_FREQUENCY_SPREAD: f64 = 1.5;
/// The same for per-county Storm Events episode rates.
pub(crate) const STORM_EVENTS_SPREAD: f64 = 1.3;
/// The same for modelled earthquake shaking chances (hazard-model uncertainty).
pub(crate) const SEISMIC_SPREAD: f64 = 2.0;
/// The same for per-county outage-event rates from EAGLE-I.
pub(crate) const OUTAGE_RATE_SPREAD: f64 = 1.3;

/// Heat-wave event-days are capped at the county's historical days over 90 °F, but never below
/// this many a year.
pub(crate) const HEAT_DAYS_CAP_FLOOR: f64 = 0.2;

/// PRIOR. Chance that one household-significant event of each storm type cuts the power, used
/// only to compare the storm rates with recorded outages (the outage floor).
pub(crate) const OUTAGE_SHARE_STRONG_WIND: f64 = 0.9;
/// See [`OUTAGE_SHARE_STRONG_WIND`].
pub(crate) const OUTAGE_SHARE_WINTER: f64 = 0.3;
/// See [`OUTAGE_SHARE_STRONG_WIND`].
pub(crate) const OUTAGE_SHARE_ICE: f64 = 0.8;
/// See [`OUTAGE_SHARE_STRONG_WIND`].
pub(crate) const OUTAGE_SHARE_HURRICANE: f64 = 0.9;
/// See [`OUTAGE_SHARE_STRONG_WIND`].
pub(crate) const OUTAGE_SHARE_TORNADO: f64 = 0.8;
/// See [`OUTAGE_SHARE_STRONG_WIND`].
pub(crate) const OUTAGE_SHARE_LIGHTNING: f64 = 0.8;
/// See [`OUTAGE_SHARE_STRONG_WIND`].
pub(crate) const OUTAGE_SHARE_HAIL: f64 = 0.1;

/// PRIOR. How exposed the household's power lines are, by setting: rural lines are long and
/// overhead, city lines are shorter and partly underground (research §2.7 "utility
/// reliability"; used for windstorms, ice storms and lightning).
pub(crate) fn utility_exposure(setting: Setting) -> Estimate {
    match setting {
        Setting::Urban => prior((0.5, 0.3, 0.8), &[cite::RR_HAZARD_PRIORS]),
        Setting::Suburban => Estimate::exact(1.0),
        Setting::Rural => prior((2.0, 1.3, 3.0), &[cite::RR_HAZARD_PRIORS]),
    }
}

/// PRIOR. How likely the household is to sit in the county's wildfire exposure area, by setting.
pub(crate) fn wildfire_setting(setting: Setting) -> Estimate {
    match setting {
        Setting::Urban => prior((0.3, 0.1, 0.6), &[cite::RR_HAZARD_PRIORS]),
        Setting::Suburban => Estimate::exact(1.0),
        Setting::Rural => prior((2.0, 1.3, 3.0), &[cite::RR_HAZARD_PRIORS]),
    }
}

/// PRIOR. Share of the western-state wildfire shutoff rate that reaches a household, by setting
/// (shutoffs target high-fire-threat districts, mostly rural and edge-of-town).
pub(crate) fn psps_setting(setting: Setting) -> Estimate {
    match setting {
        Setting::Urban => prior((0.1, 0.03, 0.3), &[cite::RR_HAZARD_PRIORS]),
        Setting::Suburban => prior((0.5, 0.2, 0.8), &[cite::RR_HAZARD_PRIORS]),
        Setting::Rural => Estimate::exact(1.0),
    }
}

/// States whose utilities use wildfire safety power shutoffs.
pub(crate) const PSPS_STATES: &[&str] = &[
    "AZ", "CA", "CO", "ID", "MT", "NM", "NV", "OR", "UT", "WA", "WY",
];

// ---------------------------------------------------------------------------------------------
// Household modifiers (m_h; DESIGN §4.3, research §2.7).
// ---------------------------------------------------------------------------------------------

/// PRIOR. A basement takes on water in heavy rain and sewer backups outside the mapped flood
/// zone.
pub(crate) const FLOOD_BASEMENT: Triple = (1.5, 1.2, 2.0);
/// PRIOR. A home on the second floor or higher stays dry; the household is still cut off or
/// loses building services when the ground floor floods.
pub(crate) const FLOOD_UPPER_FLOOR: Triple = (0.5, 0.3, 0.8);
/// PRIOR (research §2.7): a rowhouse or other attached home also burns when a neighbour's does.
pub(crate) const FIRE_ATTACHED: Triple = (2.0, 1.5, 3.0);
/// PRIOR. A high-rise apartment shares a building, but fire-rated construction and sprinklers
/// limit spread from other units.
pub(crate) const FIRE_HIGH_RISE: Triple = (1.5, 1.0, 2.0);

/// PRIOR (research §2.7; DESIGN §4.3): how income stability scales job-loss incidence. `stable` (a
/// regular salary) is the typical W-2 job at ×1, as in the research's Philadelphia example;
/// `very_stable` (tenured, public sector, a pension) is the research's ×0.5 step. The research
/// gives only the point estimate for `very_stable`; the range here is this crate's own symmetric
/// spread (±40%), the same shape used for the other stability priors below.
pub(crate) fn income_stability(stability: IncomeStability) -> Estimate {
    match stability {
        IncomeStability::VeryStable => prior((0.5, 0.3, 0.7), &[cite::RR_PRIORS]),
        IncomeStability::Stable => Estimate::exact(1.0),
        IncomeStability::Variable => prior((1.5, 1.0, 2.0), &[cite::RR_PRIORS]),
        IncomeStability::Seasonal => prior((1.75, 1.5, 2.0), &[cite::RR_PRIORS]),
        IncomeStability::Gig => prior((1.75, 1.5, 2.0), &[cite::RR_PRIORS]),
    }
}

/// PRIOR. Curfews and unrest concentrate in cities (research §6.2's 3 %/yr is a Philadelphia
/// figure); the same scaling is used for terrorism.
pub(crate) fn unrest_setting(setting: Setting) -> Estimate {
    match setting {
        Setting::Urban => Estimate::exact(1.0),
        Setting::Suburban => prior((0.5, 0.3, 0.8), &[cite::RR_HAZARD_PRIORS]),
        Setting::Rural => prior((0.2, 0.1, 0.4), &[cite::RR_HAZARD_PRIORS]),
    }
}

// ---------------------------------------------------------------------------------------------
// Floods.
// ---------------------------------------------------------------------------------------------

/// DATA (definition). A home in the Special Flood Hazard Area has at least a 1 % chance of
/// flooding each year; the high end allows for zones that flood more often than mapped.
pub(crate) const SFHA_ANNUAL: Triple = (0.01, 0.01, 0.03);
/// PRIOR. Yearly chance that flood water reaches a home outside the mapped flood zone (heavy
/// rain, overwhelmed drains), for a county with the national median inland-flood frequency.
pub(crate) const OUTSIDE_SFHA_ANNUAL: Triple = (0.002, 0.0005, 0.005);
/// DERIVED (NRI v1.20 county table): national median inland-flood annualised frequency,
/// 0.9643 events a year. A county's frequency relative to this scales the outside-zone chance
/// (bounded ×0.5 to ×2).
pub(crate) const INLAND_FLOOD_AFREQ_MEDIAN: f64 = 0.9643;
/// DATA (definition). Yearly chance of flooding for a home in a coastal flood zone.
pub(crate) const COASTAL_ZONE_ANNUAL: Triple = (0.01, 0.005, 0.03);
/// Bounds on the NFIP claims rate used as the in-zone chance (per policy-year).
pub(crate) const NFIP_CLAIMS_BOUNDS: (f64, f64) = (0.003, 0.1);

// ---------------------------------------------------------------------------------------------
// Drought.
// ---------------------------------------------------------------------------------------------

/// PRIOR (research §9, Coos Bay): yearly chance a drought lowers a private well's yield enough
/// to matter, for a county with the national median drought frequency.
pub(crate) const DROUGHT_WELL: Triple = (0.01, 0.003, 0.03);
/// PRIOR. Yearly chance a drought brings water limits that change daily life for a household on
/// a public water system.
pub(crate) const DROUGHT_MUNICIPAL: Triple = (0.002, 0.0005, 0.01);
/// DERIVED (NRI v1.20 county table): national median drought annualised frequency, 13.43.
pub(crate) const DROUGHT_AFREQ_MEDIAN: f64 = 13.43;

// ---------------------------------------------------------------------------------------------
// Societal hazards (research §6.2; all PRIOR unless the base-rate pack has a number).
// ---------------------------------------------------------------------------------------------

/// PRIOR (research §6.2): multi-day regional grid failure not caused by weather.
pub(crate) const GRID_FAILURE: Triple = (0.005, 0.0015, 0.015);
/// PRIOR (research §6.2): pharmacy or insurer computer outage (also payments, utilities).
pub(crate) const CYBER_OUTAGE: Triple = (0.02, 0.007, 0.05);
/// PRIOR (research §6.2): a civil-unrest curfew, for a city household.
pub(crate) const CIVIL_UNREST: Triple = (0.03, 0.01, 0.06);
/// PRIOR (research §6.2): a chemical do-not-drink order; the base of the chemical-release rate.
pub(crate) const CHEMICAL_RELEASE: Triple = (0.02, 0.007, 0.05);
/// PRIOR. TRI facility count at which the chemical-release rate is ×1 (roughly the typical
/// county).
pub(crate) const TRI_REFERENCE_FACILITIES: f64 = 5.0;
/// PRIOR (research §6.2): store shortages and pre-storm runs.
pub(crate) const SUPPLY_SHORTAGE: Triple = (0.2, 0.1, 0.4);
/// PRIOR (research §6.2): a pandemic that disrupts food access and daily life. Used when the
/// base-rate pack has no pandemic onset rate.
pub(crate) const PANDEMIC_DISRUPTIVE: Triple = (0.01, 0.005, 0.02);
/// PRIOR. Share of pandemic onsets that disrupt daily life as 1918 and 2020 did (two of the five
/// since 1918); multiplies the onset rate (about 4.5 % a year) when the pack has it.
pub(crate) const PANDEMIC_DISRUPTIVE_SHARE: Triple = (0.25, 0.1, 0.4);
/// PRIOR. Yearly chance of an accident at a plant within 16 km (10 miles) that brings orders to
/// shelter or leave. One US accident needing off-site protective action (Three Mile Island,
/// 1979) in several thousand reactor-years.
pub(crate) const NUCLEAR_PLANT_EPZ: Triple = (2.0e-4, 2.0e-5, 5.0e-4);
/// PRIOR. Same, for a plant 16–80 km away (food and water advisories in the ingestion zone).
pub(crate) const NUCLEAR_PLANT_INGESTION: Triple = (5.0e-5, 1.0e-5, 2.0e-4);
/// SRC (research §6.3, from the Forecasting Research Institute): superforecasters' and experts'
/// medians of 1 % and 5 % for a catastrophe killing 10 million or more before 2045, spread over
/// the years to 2045: about 1 in 2,000 to about 1 in 400 a year (our arithmetic). Never
/// shown as a point estimate; the value used for arithmetic is the geometric middle.
pub(crate) const NUCLEAR_ATTACK_RANGE: (f64, f64) = (1.0 / 2000.0, 1.0 / 400.0);
/// PRIOR. An attack that disrupts daily life where a city household lives: 1 in 10,000 to 1 in
/// 1,000 a year. Never shown as a point estimate.
pub(crate) const TERRORISM_RANGE: (f64, f64) = (1.0e-4, 1.0e-3);

// ---------------------------------------------------------------------------------------------
// Personal hazards (research §6.1, data-sources §8).
// ---------------------------------------------------------------------------------------------

/// DERIVED: 344,600 reported residential fires (USFA 2023) / 131,434,000 households (CPS HH-1
/// 2023) = 0.2622 % per household-year. The range is PRIOR.
pub(crate) const HOUSE_FIRE: Triple = (0.002_622, 0.002, 0.0035);
/// SRC (BLS Work Experience 2024): 8.3 % of labour-force participants had at least one spell of
/// unemployment in the year, used as the per-earner spell rate (research §3.5). The low end is
/// PRIOR; the high end is the JOLTS layoff rate of 1.117 % a month as a yearly rate,
/// −12 · ln(1 − 0.01117) = 0.135 (an upper bound, because layoffs cluster in high-churn jobs).
pub(crate) const JOB_LOSS_SPELL: Triple = (0.083, 0.06, 0.135);
/// SRC (NHAMCS 2022): 47.3 emergency department visits per 100 people a year. The range is
/// PRIOR (age and health vary by household).
pub(crate) const ED_VISITS: Triple = (0.473, 0.35, 0.65);
/// PRIOR. Crashes (NHTSA: about 6.1 million police-reported a year) and breakdowns that leave a
/// driver stuck away from home, per vehicle-year.
pub(crate) const VEHICLE_STRANDING: Triple = (0.15, 0.05, 0.4);
/// PRIOR. A transit shutdown or storm that strands a commuter without a car, per commuter-year.
pub(crate) const TRANSIT_STRANDING: Triple = (0.03, 0.01, 0.1);
/// PRIOR (research §6.1: 2–10 % a year per household): a boil-water notice.
pub(crate) const BOIL_NOTICE: Triple = (0.05, 0.02, 0.1);
/// PRIOR (research §8 prototype): a water main break or pressure loss that cuts the tap for
/// hours.
pub(crate) const MAIN_BREAK: Triple = (0.10, 0.05, 0.2);
/// PRIOR. A gas leak or other local utility problem for a household on a private well (the well
/// pump's own failures are handled with the well coupling in `rr-consequence`).
pub(crate) const WELL_LOCAL_OUTAGE: Triple = (0.01, 0.003, 0.03);
/// PRIOR. A household burglary, about 1 in 100 homes a year (to be replaced by the BJS National
/// Crime Victimization Survey figure).
pub(crate) const BURGLARY: Triple = (0.01, 0.005, 0.02);
/// DERIVED + PRIOR, per earner-year: disability onset ≈ 0.0061 (SSA: 1 in 4 20-year-olds
/// disabled before full retirement age, 67, spread over 47 years) plus death at working age ≈ 0.003
/// (accidental deaths alone are 58.1 per 100,000, NCHS 2024).
pub(crate) const EARNER_LOSS: Triple = (0.009, 0.005, 0.015);
/// PRIOR. A long illness (weeks) that keeps someone sick at home, per person-year.
pub(crate) const LONG_ILLNESS: Triple = (0.01, 0.005, 0.03);

// ---------------------------------------------------------------------------------------------
// Power-outage floor (research §2.5, §7.5: EAGLE-I county records).
// ---------------------------------------------------------------------------------------------

/// PRIOR (informed by Do et al. 2023, 62.1 % of long county outages coincided with extreme
/// weather): share of recorded county outage events caused by weather.
pub(crate) const OUTAGE_WEATHER_SHARE: Triple = (0.7, 0.6, 0.9);

// ---------------------------------------------------------------------------------------------
// Climate (research §5.3).
// ---------------------------------------------------------------------------------------------

/// Heat multipliers are capped at ×3 (research §5.3).
pub(crate) const HEAT_MULTIPLIER_CAP: f64 = 3.0;
/// Cold and winter multipliers may fall below 1 but not below this ("fewer, not none").
pub(crate) const COLD_MULTIPLIER_FLOOR: f64 = 0.5;
/// Heavy-precipitation multipliers on inland flooding are bounded to this range.
pub(crate) const PRECIP_MULTIPLIER_BOUNDS: (f64, f64) = (0.5, 3.0);
/// Dry-spell multipliers on drought and wildfire are bounded to this range (never below 1, as in
/// FEMA's Future Risk Index hazard multiplier).
pub(crate) const DRY_SPELL_MULTIPLIER_BOUNDS: (f64, f64) = (1.0, 2.0);
/// PRIOR (research §5.3, NCA5): the share of hurricanes that are major rises ×1.1–1.3 by 2050;
/// hurricane frequency is not changed.
pub(crate) const HURRICANE_INTENSITY: Triple = (1.2, 1.1, 1.3);

// ---------------------------------------------------------------------------------------------
// Scenarios, severity and display thresholds.
// ---------------------------------------------------------------------------------------------

/// Hurricane annualised frequency at or above which a Gulf or Atlantic county gets the
/// `major_hurricane_direct_hit` scenario (research §3.8: 27 % of Americans live where it is at
/// least 0.1 a year).
pub(crate) const MAJOR_HURRICANE_AFREQ_THRESHOLD: f64 = 0.1;
/// Gulf and Atlantic states and territories.
pub(crate) const HURRICANE_STATES: &[&str] = &[
    "AL", "CT", "DC", "DE", "FL", "GA", "LA", "MA", "MD", "ME", "MS", "NC", "NH", "NJ", "NY", "PA",
    "PR", "RI", "SC", "TX", "VA", "VI",
];
/// Natural hazards whose household rate is below this (1 in 100,000 a year) are left out of the
/// register and listed in a note instead.
pub(crate) const NEGLIGIBLE_RATE: f64 = 1.0e-5;
/// Severity scale: a per-event loss of this many dollars (or less) is severity 0 ...
pub(crate) const SEVERITY_LOSS_ZERO_USD: f64 = 50.0;
/// ... and this many dollars (or more) is severity 1, on a log scale in between.
pub(crate) const SEVERITY_LOSS_ONE_USD: f64 = 500_000.0;
/// Natural hazards never show a severity below this: even a windstorm that only cuts the power
/// is more than nothing.
pub(crate) const NATURAL_SEVERITY_FLOOR: f64 = 0.1;
/// PRIOR. The severity a heat wave or cold wave shows at least, for a household with someone at
/// higher risk from it: 0.4, "Serious", the severity of an emergency-department visit ($2,000 on
/// the fixed scale). NRI's expected loss does count deaths and injuries (valued per statistical
/// life), but spread over every household and every episode in the county, so the per-event
/// loss reads "Minor" (Philadelphia heat: about $85 an episode) although heat is most dangerous
/// for exactly these households (CDC; Semenza et al. 1996, Chicago 1995).
pub(crate) const AT_RISK_TEMPERATURE_SEVERITY: f64 = 0.4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_triple_is_ordered_and_positive() {
        let all: &[(&str, Triple)] = &[
            ("heat episode", HEAT_EPISODE_DAYS),
            ("cold episode", COLD_EPISODE_DAYS),
            ("winter episode", WINTER_EPISODE_DAYS),
            ("wind", STRONG_WIND_FOOTPRINT),
            ("ice", ICE_STORM_FOOTPRINT),
            ("lightning", LIGHTNING_FOOTPRINT),
            ("winter", WINTER_FOOTPRINT),
            ("hurricane", HURRICANE_FOOTPRINT),
            ("major hurricane", MAJOR_HURRICANE_FOOTPRINT),
            ("major share", MAJOR_HURRICANE_SHARE),
            ("hail ratio", HAIL_DAMAGE_RATIO),
            ("tornado ratio", TORNADO_DAMAGE_RATIO),
            ("tornado disruption", TORNADO_DISRUPTION),
            ("landslide ratio", LANDSLIDE_DAMAGE_RATIO),
            ("landslide access", LANDSLIDE_ACCESS),
            ("tsunami warning", TSUNAMI_WARNING_SHARE),
            ("volcano", VOLCANO_SHARE),
            ("wildfire evacuations", WILDFIRE_EVACUATIONS_PER_BURN),
            ("psps", PSPS_RATE),
            ("hail fallback", HAIL_FALLBACK_FOOTPRINT),
            ("tornado fallback", TORNADO_FALLBACK_FOOTPRINT),
            ("exposure fallback", EXPOSURE_FALLBACK_SHARE),
            ("landslide fallback", LANDSLIDE_FALLBACK_FOOTPRINT),
            ("sfha fallback", SFHA_SHARE_FALLBACK),
            ("local tsunami", LOCAL_TSUNAMI_OTHER),
            ("tsunami zone share", TSUNAMI_ZONE_SHARE_FALLBACK),
            ("basement", FLOOD_BASEMENT),
            ("upper floor", FLOOD_UPPER_FLOOR),
            ("attached", FIRE_ATTACHED),
            ("high rise", FIRE_HIGH_RISE),
            ("sfha", SFHA_ANNUAL),
            ("outside sfha", OUTSIDE_SFHA_ANNUAL),
            ("coastal zone", COASTAL_ZONE_ANNUAL),
            ("drought well", DROUGHT_WELL),
            ("drought municipal", DROUGHT_MUNICIPAL),
            ("grid", GRID_FAILURE),
            ("cyber", CYBER_OUTAGE),
            ("unrest", CIVIL_UNREST),
            ("chemical", CHEMICAL_RELEASE),
            ("shortage", SUPPLY_SHORTAGE),
            ("pandemic", PANDEMIC_DISRUPTIVE),
            ("pandemic share", PANDEMIC_DISRUPTIVE_SHARE),
            ("epz", NUCLEAR_PLANT_EPZ),
            ("ingestion", NUCLEAR_PLANT_INGESTION),
            ("fire", HOUSE_FIRE),
            ("job", JOB_LOSS_SPELL),
            ("ed", ED_VISITS),
            ("vehicle", VEHICLE_STRANDING),
            ("transit", TRANSIT_STRANDING),
            ("boil", BOIL_NOTICE),
            ("main break", MAIN_BREAK),
            ("well local", WELL_LOCAL_OUTAGE),
            ("burglary", BURGLARY),
            ("earner", EARNER_LOSS),
            ("illness", LONG_ILLNESS),
            ("outage weather", OUTAGE_WEATHER_SHARE),
            ("hurricane intensity", HURRICANE_INTENSITY),
        ];
        for (name, (v, lo, hi)) in all {
            assert!(*lo > 0.0 && lo <= v && v <= hi, "{name}: {lo} {v} {hi}");
        }
    }

    #[test]
    fn house_fire_rate_is_usfa_fires_over_cps_households() {
        // 344,600 reported residential building fires (USFA 2023) / 131,434,000 households.
        assert!((HOUSE_FIRE.0 - 344_600.0 / 131_434_000.0).abs() < 1e-6);
    }

    #[test]
    fn nuclear_range_is_research_section_6_3() {
        assert_eq!(NUCLEAR_ATTACK_RANGE, (0.0005, 0.0025));
    }
}
