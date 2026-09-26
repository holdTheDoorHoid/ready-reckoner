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
/// DERIVED from HURDAT2 in the pack (verification, 2026-09-26): the share of NRI hurricane events
/// that are major (Category 3+). NRI's hurricane frequency counts about as many events as
/// HURDAT2's tropical-storm-strength passages within 50 nautical miles (median ratio 0.81 over
/// 1,836 counties; 3.8 against hurricane-strength passages), so the share is taken against
/// tropical-storm passages: 601 major passages in 11,155 tropical-storm passages across the 556
/// counties the major-hurricane scenario can apply to, 5.4 %. The range is a PRIOR. (The older
/// one-third was the share of *landfalling hurricanes* that are major, applied to NRI's
/// tropical-storm-strength count, which inflated major storms about four times.)
pub(crate) const MAJOR_HURRICANE_SHARE: Triple = (0.054, 0.03, 0.10);
/// PRIOR. The weight, in tropical-storm passages, of [`MAJOR_HURRICANE_SHARE`] when a county's
/// own HURDAT2 record sets its share: (majors + 5 × 0.054) ÷ (passages + 5). A short record then
/// neither rules a major storm out nor lets one storm set the rate.
pub(crate) const MAJOR_SHARE_PRIOR_PASSAGES: f64 = 5.0;
/// The years of HURDAT2 track data behind the pack's passage rates (1950–2025, `events.csv`),
/// to turn a rate back into a count.
pub(crate) const HURDAT2_YEARS: f64 = 76.0;
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
/// Ceiling on a county-average household's yearly chance that a landslide damages the home
/// (model review M-05): never above the chance that defines a high-risk flood zone, 1 in 100 a
/// year (`fema_flood_zones`), which the model already uses as its yardstick for "likely to be
/// damaged". Before it, NRI's landslide records gave Utuado, Puerto Rico 21 in 100 a year.
pub(crate) const LANDSLIDE_DAMAGE_CEILING: f64 = 0.01;
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
/// loss ratio; a tenth of it ([`LANDSLIDE_ACCESS`]) damages the home.
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

// ---------------------------------------------------------------------------------------------
// Contract v2 ranked hazards (REVIEW §2.2; hazard-expansion Deliverable A and
// hazard-candidates.csv). Each is listed with its derivation in `docs/RISK_MODEL.md`.
// ---------------------------------------------------------------------------------------------

/// DATA (Insurance Information Institute, from ISO claim data, 2019–2023; read through its
/// published summary): water damage and freezing claims come to about 1 in 67 insured homes a
/// year. The range is PRIOR.
pub(crate) const WATER_DAMAGE: Triple = (0.015, 0.01, 0.02);
/// PRIOR (hazard-expansion H-06): frozen pipes add claims where days that stay below freezing
/// are common.
pub(crate) const WATER_DAMAGE_FREEZE: Triple = (1.3, 1.1, 1.6);
/// Days a year that stay below freezing (CMRA `icing_days_hist`) from which the freeze modifier
/// applies: about a week of hard cold a winter.
pub(crate) const WATER_DAMAGE_FREEZE_DAYS: f64 = 5.0;
/// PRIOR (H-06): a basement (water heater, sump, washing machine and pipes below grade).
pub(crate) const WATER_DAMAGE_BASEMENT: Triple = (1.2, 1.0, 1.5);
/// PRIOR (H-06): renters report fewer water claims of their own; the building's pipes are the
/// landlord's.
pub(crate) const WATER_DAMAGE_RENTER: Triple = (0.8, 0.6, 1.0);
/// DATA (III): the average water-damage and freezing claim, about $15,400; the severity of one
/// event.
pub(crate) const WATER_DAMAGE_LOSS_USD: f64 = 15_400.0;

/// PRIOR (hazard-expansion, `wildfire_smoke`): days in one smoke episode.
pub(crate) const SMOKE_EPISODE_DAYS: Triple = (3.0, 2.0, 5.0);
/// How far a county's smoke-day count may be off, as a factor either way, when it comes from
/// its own air monitors.
pub(crate) const SMOKE_MONITOR_SPREAD: f64 = 1.3;
/// The same when the count is imputed from satellite smoke maps and nearby monitors.
pub(crate) const SMOKE_IMPUTED_SPREAD: f64 = 2.0;

/// PRIOR (hazard-expansion, `dust_storm`): share of a county's dust-storm episodes (Storm Events
/// counts them by forecast zone, which is larger than a neighbourhood) that reach one household.
pub(crate) const DUST_FOOTPRINT: Triple = (0.3, 0.1, 0.6);

/// PRIOR (hazard-expansion, `sinkhole`: Florida's subsidence reports and sinkhole claims; low
/// confidence): yearly chance that a sinkhole or ground collapse damages a home on karst ground.
pub(crate) const SINKHOLE_ON_KARST: Triple = (2.0e-4, 5.0e-5, 1.0e-3);

/// PRIOR (hazard-expansion H-05; ASDSO counts 173 failures and 587 incidents in 2005 to mid-2013,
/// about 2 in 10,000 failures per dam a year across every class): failures, or incidents that
/// force an evacuation (Oroville 2017), per high-hazard dam a year.
pub(crate) const DAM_EVENT_PER_DAM: Triple = (1.0e-4, 3.0e-5, 5.0e-4);
/// PRIOR (H-05): share of a ZIP code's households told to leave when a dam whose listed
/// downstream town lies in it fails or threatens to.
pub(crate) const DAM_DOWNSTREAM_FOOTPRINT: Triple = (0.3, 0.1, 0.6);
/// PRIOR (H-05): share of a county's households reached by one of its high-hazard dams, used when
/// the ZIP-level downstream count is not known.
pub(crate) const DAM_COUNTY_FOOTPRINT: Triple = (0.01, 0.003, 0.03);
/// PRIOR (H-05): a dam whose latest condition rating is Poor or Unsatisfactory fails or forces an
/// evacuation three times as often.
pub(crate) const DAM_POOR_CONDITION: f64 = 3.0;
/// PRIOR (hazard-expansion, `levee_failure`): yearly chance that a levee is overtopped or fails
/// for a household behind it. Leveed land is mapped outside the high-risk flood zone, so the
/// flood rate does not count it.
pub(crate) const LEVEE_RESIDUAL: Triple = (0.002, 0.0005, 0.01);
/// PRIOR: people behind levees that USACE rates High or Very High risk face twice the residual
/// chance.
pub(crate) const LEVEE_HIGH_RISK: f64 = 2.0;

/// PRIOR (hazard-expansion, `network_outage`; FCC report on the AT&T outage of 22 February 2024:
/// more than 92 million calls and 25,000 calls to 911 blocked for at least 12 hours):
/// carrier-wide outages of several hours come about once a year, and about a third of households
/// are on the carrier that fails.
pub(crate) const NETWORK_OUTAGE: Triple = (0.3, 0.1, 1.0);

/// PRIOR (hazard-expansion, `drug_shortage`; ASHP counted 323 active shortages at the peak in
/// early 2024; openFDA listed 70 medicines as currently short on 2026-09-26): yearly chance that
/// one person's daily prescription is short for days to weeks.
pub(crate) const DRUG_SHORTAGE: Triple = (0.05, 0.02, 0.15);
/// PRIOR: a medicine that must stay cold, usually an injectable, runs short more often (50 of the
/// 70 medicines openFDA listed as short on 2026-09-26 are injectables).
pub(crate) const DRUG_SHORTAGE_COLD: Triple = (1.5, 1.2, 2.0);

/// DATA (CRS RS20348, Table 1, via the data-model series `funding_gaps`): federal funding gaps
/// of 14 full days or more came in 4 of the 45 fiscal years 1982–2026 (FY1996, FY2014, FY2019,
/// FY2026): 0.089 a year, exact Poisson 90 % interval 0.030–0.203. Federal pay stops until the
/// gap ends.
pub(crate) const FUNDING_GAP_14D: Triple = (0.08889, 0.03036, 0.2034);
/// PRIOR: share of those long gaps that stop SNAP or WIC payments (one of four so far, November
/// 2025, the first lapse in SNAP's history).
pub(crate) const SNAP_LAPSE_GIVEN_GAP: Triple = (0.25, 0.1, 0.6);
/// PRIOR: Social Security, SSI, SSDI and VA payments continued through every shutdown; a delay
/// needs something that has not happened (the debt limit breached).
pub(crate) const MANDATORY_BENEFIT_DELAY: Triple = (0.005, 0.001, 0.02);
/// PRIOR: unemployment benefits are run by the states and kept paying through shutdowns; a
/// delay comes from a state system failing or a federal funding lapse.
pub(crate) const UNEMPLOYMENT_DELAY: Triple = (0.01, 0.002, 0.04);
/// PRIOR: a month of lost pay or benefits, the severity of one interruption.
pub(crate) const BENEFIT_LOSS_USD: f64 = 1_500.0;

/// PRIOR (Eviction Lab, national rate for 2016: about 2.3 eviction judgments per 100 renter
/// households; UNVERIFIED beyond summaries): judgments per renter household a year, used when
/// the county's filing rate is not in the pack.
pub(crate) const EVICTION_NATIONAL: Triple = (0.023, 0.01, 0.05);
/// PRIOR (Eviction Lab 2016: about 0.9 million judgments from 2.3 million filings; UNVERIFIED):
/// share of eviction filings that end in an order to leave.
pub(crate) const EVICTION_JUDGMENT_SHARE: Triple = (0.4, 0.3, 0.55);
/// PRIOR (hazard-expansion, `eviction`): three months or more of savings halves the rate.
pub(crate) const EVICTION_SAVINGS: Triple = (0.5, 0.3, 0.8);
/// Months of savings from which [`EVICTION_SAVINGS`] applies.
pub(crate) const EVICTION_SAVINGS_MONTHS: f32 = 3.0;

/// PRIOR (CSIS terrorism dataset 1994–2025; hazard-expansion B3): attacks or credible threats a
/// year that put a metro area under an order covering 100,000 people or more for 12 hours or
/// more (Oklahoma City 1995, 11 September 2001, the anthrax letters of 2001, Boston 2013).
pub(crate) const ATTACK_US: Triple = (0.1, 0.04, 0.25);
/// PRIOR (B3): share of the metro area's households under the order.
pub(crate) const ATTACK_METRO_SHARE: Triple = (0.3, 0.1, 0.8);
/// DERIVED + PRIOR: a household outside every funded urban area. The 5 % of attacks held back
/// from the funded areas, over the roughly 64 million households outside them (48 % of people,
/// data-hazard's UASI table and the NRI population), with an order covering 40,000 households:
/// 0.1 × 0.05 × 40,000 ÷ 64,000,000 ≈ 3 in a million a year.
pub(crate) const ATTACK_NON_UASI: Triple = (3.0e-6, 1.0e-6, 3.0e-5);
/// PRIOR: what one closure costs a household (a day or two of lost work and school); severity
/// 0.3, "a few days' disruption" (hazard-expansion H-10).
pub(crate) const ATTACK_LOSS_USD: f64 = 800.0;

/// DATA (FBI Crime in the United States 2023–2025, Tables 29, 39 and 40, via the data-model
/// series `fbi_arrests`): arrests per 100,000 people a year by age band, as `(band, first age,
/// last age, male (value, low, high), female (value, low, high))`. The value is the mean of the
/// three years; low and high are the lowest and highest year. The FBI counts arrests, not people
/// or convictions: one person arrested twice counts twice.
pub(crate) const ARRESTS_PER_100K: &[(&str, u8, u8, Triple, Triple)] = &[
    (
        "10_17",
        10,
        17,
        (1992.0, 1894.0, 2079.0),
        (948.5, 880.8, 1006.0),
    ),
    (
        "18_24",
        18,
        24,
        (5390.0, 5243.0, 5603.0),
        (2122.0, 2051.0, 2202.0),
    ),
    (
        "25_34",
        25,
        34,
        (6816.0, 6500.0, 7168.0),
        (2647.0, 2534.0, 2758.0),
    ),
    (
        "35_44",
        35,
        44,
        (6198.0, 6150.0, 6247.0),
        (2400.0, 2353.0, 2450.0),
    ),
    (
        "45_54",
        45,
        54,
        (3784.0, 3643.0, 3964.0),
        (1295.0, 1221.0, 1375.0),
    ),
    (
        "55_64",
        55,
        64,
        (2093.0, 1989.0, 2221.0),
        (584.0, 538.0, 630.8),
    ),
    (
        "65_plus",
        65,
        84,
        (495.3, 445.3, 549.9),
        (115.3, 99.29, 132.0),
    ),
];
/// PRIOR: what one arrest costs the household (bail, a lawyer, lost pay, care for children or
/// pets), for severity only.
pub(crate) const ARREST_LOSS_USD: f64 = 5_000.0;

// ---------------------------------------------------------------------------------------------
// The rare families (REVIEW §2.3; hazard-expansion Deliverable B). Every factor is expert
// judgement stacked on expert judgement: `prior`, shown only as a range. The ranges multiply
// low by low and high by high, so they span every combination of the factors (the review's own
// arithmetic), rather than adding log-spreads in quadrature as the ranked rates do.
// ---------------------------------------------------------------------------------------------

/// PRIOR (B1.3): a large nuclear attack on the US homeland, a year. FRI 2024 (a catastrophe of 10
/// million deaths or more by 2045: experts 5 %, superforecasters 1%, i.e. 0.049–0.25 % a year)
/// × the chance it reaches US soil, 0.33 (0.23–0.46) from FRI's dyad shares; the high end
/// reaches Rethink Priorities' 0.38 % a year for a US–Russia exchange.
pub(crate) const NUCLEAR_STRATEGIC_US: Triple = (4.0e-4, 1.0e-4, 4.0e-3);
/// PRIOR (B1.3): a limited strike (one or a few weapons) on US territory: use anywhere × 2 %
/// (0.5–5 %).
pub(crate) const NUCLEAR_LIMITED_US: Triple = (1.0e-4, 2.0e-5, 5.0e-4);
/// PRIOR (B1.3): a crude nuclear device in a US city: FRI's non-state acquisition forecasts
/// (0.05–0.17 % a year) × 0.3 detonated × 0.25 in a US city; the low end respects 80 years of none.
pub(crate) const NUCLEAR_IND_US: Triple = (5.0e-5, 1.0e-6, 5.0e-4);
/// PRIOR (B1.3): a nuclear weapon used anywhere in the world (XPT 0.48–0.54 %, FRI 0.11–0.55 %,
/// Good Judgment 0.40 %, Rethink Priorities about 1.1 %, the 80-year record 0.62 % a year).
pub(crate) const NUCLEAR_USE_WORLD: Triple = (5.0e-3, 1.0e-3, 1.5e-2);
/// PRIOR (B2): chance a large attack includes a high-altitude burst (EMP).
pub(crate) const HEMP_GIVEN_STRATEGIC: Triple = (0.5, 0.2, 0.8);
/// PRIOR (B2): the same for a limited strike.
pub(crate) const HEMP_GIVEN_LIMITED: Triple = (0.3, 0.1, 0.5);
/// PRIOR (B1.1): share of a targeted county's households seriously affected by one weapon.
pub(crate) const LIMITED_STRIKE_HOUSEHOLD_SHARE: f64 = 0.3;
/// PRIOR (B1.4): a limited strike's weight on one of the most plausible target counties (the
/// counterforce sites of class A, and the military bases of Hawaii and Guam): a flat 1 in 20.
pub(crate) const LIMITED_STRIKE_WEIGHT: f64 = 0.05;
/// PRIOR (B1.1): share of a metro area's households in the damage zone or the dangerous fallout
/// zone of a crude device.
pub(crate) const IND_METRO_HOUSEHOLD_SHARE: f64 = 0.1;
/// DERIVED: the population-weighted mean of f_S over every county in the first cut (A 3.5 %,
/// B 4.4 %, C1 26.0 %, C2 28.9 %, D 5.8 %, E 31.5 % of people), for a county whose class is not
/// known; its range runs from class E's low to class A's high.
pub(crate) const STRATEGIC_FACTOR_UNKNOWN: Triple = (0.314, 0.01, 0.99);

/// PRIOR (B2): a Carrington-class geomagnetic storm, a year: the geometric middle of five
/// published estimates (Riley 2012 about 1.3 %, Love about 1.1 %, Riley and Love 2017 0.3–1.1 %,
/// Moriña 2019 0.05–0.19 %, Lloyd's 2013 about 0.7 %).
pub(crate) const CARRINGTON_STORM: Triple = (3.0e-3, 5.0e-4, 1.3e-2);
/// PRIOR (Lloyd's 2013: 20–40 million people at risk of a long outage out of about 330 million):
/// chance that such a storm cuts a household's power for days, at the population-average
/// geomagnetic latitude.
pub(crate) const GMD_OUTAGE_GIVEN_STORM: Triple = (0.09, 0.06, 0.12);
/// DERIVED (data-hazard's `geomag.csv` × NRI 2020 population): the population-weighted mean NERC
/// scaling factor α over every county, 0.2285. A county's factor over this is its location
/// multiplier.
pub(crate) const GMD_ALPHA_POP_MEAN: f64 = 0.2285;
/// The chance that one storm cuts a household's power for days never exceeds this.
pub(crate) const GMD_OUTAGE_CAP: f64 = 0.5;

/// PRIOR (Lloyd's 2013: the worst-hit areas out for 16 days to a year or two): share of the
/// multi-day outages from a severe solar storm that last two months or more.
pub(crate) const GMD_SHARE_GE_60D: Triple = (0.1, 0.02, 0.3);
/// PRIOR (B2: power 3 days median, 60 days at the 90th percentile; EPRI 2019 does not support
/// months-long nationwide blackouts): share of EMP outages that last two months or more.
pub(crate) const HEMP_SHARE_GE_60D: Triple = (0.1, 0.02, 0.3);
/// PRIOR (B5: wartime outages 1 day median, 7 days at the 90th percentile): share lasting two
/// months or more.
pub(crate) const WAR_SHARE_GE_60D: Triple = (0.02, 0.005, 0.1);

/// PRIOR (B5; FRI 2024: violent Russia–USA conflict by 2030, experts 5 %, superforecasters
/// 1.8 %): a great-power war, a year.
pub(crate) const GREAT_POWER_WAR: Triple = (5.0e-3, 2.0e-3, 1.0e-2);
/// PRIOR (B5): chance such a war brings attacks (cyber, sabotage or missiles) on power, water or
/// communications in the US that reach a household near military sites and infrastructure.
pub(crate) const WAR_HOMELAND_ATTACKED: Triple = (0.5, 0.2, 0.8);
/// PRIOR (B5): the same far from military sites, big cities, ports and refineries (classes B, D
/// and E), relative to near them.
pub(crate) const WAR_FAR_FROM_TARGETS: f64 = 0.3;

/// PRIOR (B4; START POICN counts 517 CBRN events worldwide in 1990–2017, about 76 % chemical):
/// chemical, biological or radiological attacks a year that disrupt daily life in a US metro.
pub(crate) const CBRN_US: Triple = (0.03, 0.01, 0.1);
/// PRIOR (B4): share of the metro area's households under an order (buildings and blocks, not
/// the whole metro, as with the anthrax letters).
pub(crate) const CBRN_METRO_SHARE: Triple = (0.05, 0.01, 0.2);
/// DERIVED + PRIOR, as [`ATTACK_NON_UASI`]: 0.03 × 0.05 × 5,000 households ÷ 64 million.
pub(crate) const CBRN_NON_UASI: Triple = (1.5e-7, 1.0e-8, 2.0e-6);

/// PRIOR (B5; Marani et al. 2021: a COVID-intensity pandemic about 0.5 % a year): a pandemic far
/// deadlier than COVID-19, natural or engineered, anywhere, a year.
pub(crate) const SEVERE_PANDEMIC: Triple = (1.5e-3, 5.0e-4, 5.0e-3);
/// DATA-based PRIOR (Cassidy and Mani 2022: "one-in-six" chance of a VEI 7 eruption this century):
/// a very large eruption anywhere, a year.
pub(crate) const VEI7_WORLD: Triple = (1.8e-3, 8.0e-4, 4.0e-3);
/// DATA (USGS Yellowstone Volcano Observatory: about 1 in 730,000 a year): a caldera-forming
/// eruption at Yellowstone.
pub(crate) const YELLOWSTONE: Triple = (1.0 / 730_000.0, 5.0e-7, 3.0e-6);
/// DERIVED (NASA 2019: Tunguska-class impacts "on the order of millennia"; a 2,000–3,000 km²
/// damage area over the Earth's surface): an asteroid or comet damaging the area where a
/// household lives, a year.
pub(crate) const ASTEROID: Triple = (3.0e-9, 1.0e-9, 6.0e-9);
/// PRIOR (B5; one national bank holiday, 1933, in about a century; deposit insurance since):
/// banks closed for three days or more across the country, a year.
pub(crate) const FINANCIAL_CRISIS: Triple = (2.0e-3, 5.0e-4, 1.0e-2);
/// DATA (FBI, Active Shooter Incidents in the United States in 2024: 23 killed and 83 wounded
/// among about 336 million people): being hurt or killed in a mass shooting or bombing, per person
/// a year; the range covers broader definitions.
pub(crate) const MASS_VIOLENCE_PER_PERSON: Triple = (3.0e-7, 1.0e-7, 1.0e-6);

// ---------------------------------------------------------------------------------------------
// Named scenarios added in v0.2.0 (REVIEW H10; hazard-expansion H-13).
// ---------------------------------------------------------------------------------------------

/// DATA (Working Group on Utah Earthquake Probabilities 2016; confirm): 43 % chance of a
/// magnitude 6.75 or larger earthquake on the Wasatch Front in 50 years. Low and high are PRIOR.
pub(crate) const WASATCH_50YR: Triple = (0.43, 0.30, 0.57);
/// DATA (UCERF3, USGS Fact Sheet 2015–3009; confirm): 19 % chance of magnitude 6.7 or larger on
/// the southern San Andreas fault in 30 years. Low and high are PRIOR.
pub(crate) const SAN_ANDREAS_SOUTH_30YR: Triple = (0.19, 0.12, 0.28);
/// DATA (USGS and Washington DNR on the Seattle fault zone; confirm): about 5 % chance of a
/// magnitude 6.5 or larger earthquake on the Seattle fault in 50 years. Low and high are PRIOR.
pub(crate) const SEATTLE_FAULT_50YR: Triple = (0.05, 0.02, 0.10);
/// Days a year over 95 °F (CMRA historical baseline) from which a county counts as a desert
/// heat county for the heat-and-blackout scenario (Maricopa about 140, Clark about 125).
pub(crate) const DESERT_HEAT_DAYS_95F: f64 = 60.0;
/// PRIOR (EAGLE-I national pattern; research §2.5): power cuts of a day or more a year for a
/// household, used for the heat-and-blackout scenario when the county has no outage record.
pub(crate) const OUTAGE_GE_1D_FALLBACK: Triple = (0.02, 0.005, 0.06);

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
            ("water damage", WATER_DAMAGE),
            ("water damage freeze", WATER_DAMAGE_FREEZE),
            ("water damage basement", WATER_DAMAGE_BASEMENT),
            ("water damage renter", WATER_DAMAGE_RENTER),
            ("smoke episode", SMOKE_EPISODE_DAYS),
            ("dust footprint", DUST_FOOTPRINT),
            ("sinkhole", SINKHOLE_ON_KARST),
            ("dam event", DAM_EVENT_PER_DAM),
            ("dam downstream", DAM_DOWNSTREAM_FOOTPRINT),
            ("dam county", DAM_COUNTY_FOOTPRINT),
            ("levee", LEVEE_RESIDUAL),
            ("network", NETWORK_OUTAGE),
            ("drug shortage", DRUG_SHORTAGE),
            ("drug shortage cold", DRUG_SHORTAGE_COLD),
            ("funding gap", FUNDING_GAP_14D),
            ("snap lapse", SNAP_LAPSE_GIVEN_GAP),
            ("mandatory benefit", MANDATORY_BENEFIT_DELAY),
            ("unemployment", UNEMPLOYMENT_DELAY),
            ("eviction", EVICTION_NATIONAL),
            ("eviction judgment", EVICTION_JUDGMENT_SHARE),
            ("eviction savings", EVICTION_SAVINGS),
            ("attack", ATTACK_US),
            ("attack share", ATTACK_METRO_SHARE),
            ("attack non-uasi", ATTACK_NON_UASI),
            ("strategic", NUCLEAR_STRATEGIC_US),
            ("limited", NUCLEAR_LIMITED_US),
            ("ind", NUCLEAR_IND_US),
            ("use abroad", NUCLEAR_USE_WORLD),
            ("hemp strategic", HEMP_GIVEN_STRATEGIC),
            ("hemp limited", HEMP_GIVEN_LIMITED),
            ("unknown class", STRATEGIC_FACTOR_UNKNOWN),
            ("carrington", CARRINGTON_STORM),
            ("gmd outage", GMD_OUTAGE_GIVEN_STORM),
            ("gmd 60 d", GMD_SHARE_GE_60D),
            ("hemp 60 d", HEMP_SHARE_GE_60D),
            ("war 60 d", WAR_SHARE_GE_60D),
            ("war", GREAT_POWER_WAR),
            ("war homeland", WAR_HOMELAND_ATTACKED),
            ("cbrn", CBRN_US),
            ("cbrn share", CBRN_METRO_SHARE),
            ("cbrn non-uasi", CBRN_NON_UASI),
            ("severe pandemic", SEVERE_PANDEMIC),
            ("vei 7", VEI7_WORLD),
            ("yellowstone", YELLOWSTONE),
            ("asteroid", ASTEROID),
            ("financial", FINANCIAL_CRISIS),
            ("mass violence", MASS_VIOLENCE_PER_PERSON),
            ("outage 1 d fallback", OUTAGE_GE_1D_FALLBACK),
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
    fn rare_family_priors_are_the_review_values() {
        // REVIEW §2.3 and hazard-expansion B1.3.
        assert_eq!(NUCLEAR_STRATEGIC_US, (4.0e-4, 1.0e-4, 4.0e-3));
        assert_eq!(NUCLEAR_LIMITED_US, (1.0e-4, 2.0e-5, 5.0e-4));
        assert_eq!(NUCLEAR_IND_US, (5.0e-5, 1.0e-6, 5.0e-4));
        assert_eq!(NUCLEAR_USE_WORLD, (5.0e-3, 1.0e-3, 1.5e-2));
        // The unknown-class factor lies between the classes it averages.
        assert!(STRATEGIC_FACTOR_UNKNOWN.0 > 0.03 && STRATEGIC_FACTOR_UNKNOWN.0 < 0.9);
    }

    #[test]
    fn arrest_bands_are_ordered_and_men_are_arrested_more_often() {
        let mut last = 0;
        for (band, first, age_last, m, f) in ARRESTS_PER_100K {
            assert!(*first > last || last == 0, "{band}");
            assert!(first <= age_last, "{band}");
            last = *age_last;
            for (v, lo, hi) in [m, f] {
                assert!(lo <= v && v <= hi && *lo > 0.0, "{band}");
            }
            assert!(m.0 > f.0, "{band}");
        }
    }

    #[test]
    fn a_long_funding_gap_is_four_in_forty_five_years() {
        // CRS RS20348: FY1996, FY2014, FY2019 and FY2026 in fiscal years 1982-2026.
        assert!((FUNDING_GAP_14D.0 - 4.0 / 45.0).abs() < 1e-5);
    }
}
