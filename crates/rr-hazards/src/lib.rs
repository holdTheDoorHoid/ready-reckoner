//! Ready Reckoner — `rr-hazards`: location + household → a household event rate for every hazard
//! (DESIGN §4.2), the risk-register cards, and the named scenarios that apply.
//!
//! ```text
//! PlanInput + CountyRecord + [BaseRate] + LocationResolved ─► assess ─► HazardAssessment
//!                                                                    ├─ profiles  (register cards)
//!                                                                    ├─ rates     (to rr-consequence)
//!                                                                    ├─ scenarios (named scenarios)
//!                                                                    └─ notes     (plain-language caveats)
//! ```
//!
//! # What a rate means
//!
//! For each hazard, `r_h = λ_h · a_h · m_h` (DESIGN §4.2, research §3.1): the county frequency,
//! times the chance one county event reaches this household (the footprint), times household
//! modifiers. The result is the yearly rate of **household-significant events**: events that
//! reach this household hard enough to trigger at least one consequence bucket, of any
//! duration. `rr-consequence` multiplies it by the chance of each bucket given such an event
//! (`Effect::p_given_event`) and by the duration curve. What one event means for each hazard is
//! stated in `docs/RISK_MODEL.md` § "Hazard rates" (and in the frequency sentence), for example:
//!
//! - heat and cold waves count every episode in the county: they reach every household, and
//!   `rr-consequence` decides what they do given the home's cooling and heating;
//! - windstorms, ice storms and lightning count when they cut this household's power or damage
//!   the home; winter storms when they keep the household in or cut the power;
//! - floods count when water reaches the home (or cuts off an upper-floor flat);
//! - job loss counts spells of unemployment of any earner; medical emergencies count emergency
//!   department visits.
//!
//! # Scenarios
//!
//! A named scenario ([`ScenarioCandidate`]) is a rare, severe version of a hazard. Its parent's
//! entry in [`HazardAssessment::rates`] is the parent's full rate; `rr-consequence` adds the
//! scenario as its own event class when `on` and drops the parent's overlapping class (see the
//! `scenarios` module docs).
//!
//! # Determinism
//!
//! No clock, no randomness, no hash-map iteration; transcendental maths through
//! [`rr_types::math`]. Same inputs, same outputs, byte for byte, on every target.
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(missing_docs))]

mod buckets;
mod cite;
mod climate;
mod ctx;
mod estimate;
mod natural;
mod params;
mod personal;
mod rate;
mod scenarios;
mod sentence;
mod severity;
mod societal;
mod why;

use serde::{Deserialize, Serialize};

use rr_types::{
    BaseRate, CountyRecord, HazardDisplay, HazardId, HazardProfile, HouseholdEventRate,
    LocationResolved, PlanInput, math,
};

pub use scenarios::{AlternativeRate, ScenarioCandidate};
pub use why::why_we_think_this;

use climate::Climate;
use ctx::{Ctx, Notes};
use estimate::Estimate;
use rate::HazardRate;
use scenarios::Detected;

/// Crate name, used by the CLI's `--version` and by the about screen.
pub const CRATE: &str = "rr-hazards";

/// Citation ids this crate can attach to a rate or profile. Each must resolve in
/// `content/citations.toml`; `docs/CITATION_IDS.md` lists them.
pub const CITATION_IDS: &[&str] = cite::ALL;

/// Everything [`assess`] returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardAssessment {
    /// The register: one card per hazard that applies, `ranked` ones first (most likely first,
    /// by today's rate, so the 2050 dial never re-orders the list silently), then the
    /// `rare_catastrophic` ones.
    pub profiles: Vec<HazardProfile>,
    /// Household event rates for `rr-consequence`, one per hazard in the register, in
    /// `HazardId` order: each hazard's full rate (a named scenario is an extra event class).
    pub rates: Vec<HouseholdEventRate>,
    /// Named scenarios that apply to this location, with defaults and the user's overrides.
    pub scenarios: Vec<ScenarioCandidate>,
    /// Plain-language caveats for the packet and the "why" drawers.
    pub notes: Vec<String>,
}

/// Computes every hazard's household event rate, the register cards and the named scenarios for
/// one household in one county.
///
/// `county` is the core-pack record for `location.county_fips`; `base_rates` are the national
/// rates from the core pack (any may be missing: built-in, cited values are used instead).
/// Pure and deterministic.
pub fn assess(
    input: &PlanInput,
    county: &CountyRecord,
    base_rates: &[BaseRate],
    location: &LocationResolved,
) -> HazardAssessment {
    let ctx = Ctx::new(input, county, base_rates, location);
    let y2050 = ctx.y2050();
    let mut notes = Notes::default();
    notes.add(format!(
        "These chances are for {} as a whole. A home near a river, the coast or a steep slope \
         can face more; the county's average is what the plan uses.",
        ctx.county_label()
    ));
    if county.fips != location.county_fips {
        notes.add(format!(
            "The county record ({}) does not match the resolved location ({}).",
            county.fips, location.county_fips
        ));
    }

    let natural = natural::assess(&ctx, &mut notes);
    let detected = scenarios::detect(&ctx, &natural, &mut notes);

    // Leave out natural hazards too rare to matter, unless a scenario hangs on them.
    let mut rates: Vec<HazardRate> = Vec::new();
    let mut negligible: Vec<&'static str> = Vec::new();
    for r in &natural.rates {
        let parent = detected.iter().any(|d| d.candidate.hazard == r.hazard);
        if r.today.value < params::NEGLIGIBLE_RATE
            && r.future.value < params::NEGLIGIBLE_RATE
            && !parent
        {
            negligible.push(plural(r.hazard));
        } else {
            rates.push(r.clone());
        }
    }
    if !negligible.is_empty() {
        notes.add(format!(
            "Also checked, and too rare here to list (under 1 in 100,000 a year): {}.",
            join_lower(&negligible)
        ));
    }
    if y2050 {
        climate_notes(&rates, &mut notes);
    }
    rates.extend(societal::assess(&ctx, &mut notes));
    rates.extend(personal::assess(&ctx, &mut notes));
    rates.sort_by_key(|r| r.hazard);
    for (h, what) in [
        (HazardId::HeatWave, "Heat waves"),
        (HazardId::ColdWave, "Cold waves"),
    ] {
        if rates.iter().any(|r| r.hazard == h) {
            if let Some(why) = severity::at_risk_reason(input, h) {
                notes.add(format!(
                    "{what} are marked Serious for this household because {why}; they are most \
                     dangerous for households like yours."
                ));
            }
        }
    }

    // rr-consequence gets each hazard's full rate; a named scenario is an extra event class
    // (see the `scenarios` module).
    let household_rates = rates
        .iter()
        .map(|r| household_rate(r.hazard, r.effective(y2050)))
        .collect();

    let mut cards: Vec<(f64, HazardProfile)> = rates
        .iter()
        .map(|r| {
            let (today, effective) = register_rate(r, &detected, y2050);
            (today.value, profile(&ctx, r, &effective))
        })
        .collect();
    cards.sort_by(|(ta, a), (tb, b)| {
        let rare = |p: &HazardProfile| p.display == HazardDisplay::RareCatastrophic;
        rare(a)
            .cmp(&rare(b))
            .then_with(|| {
                if rare(a) {
                    core::cmp::Ordering::Equal
                } else {
                    tb.total_cmp(ta)
                        .then_with(|| b.severity.total_cmp(&a.severity))
                }
            })
            .then_with(|| a.id.cmp(&b.id))
    });

    HazardAssessment {
        profiles: cards.into_iter().map(|(_, p)| p).collect(),
        rates: household_rates,
        scenarios: detected.into_iter().map(|d| d.candidate).collect(),
        notes: notes.0,
    }
}

/// The rate the register card shows (today, and as the plan uses it): the parent without its
/// scenarios' shares plus the scenarios, so the card always shows the full hazard.
fn register_rate(r: &HazardRate, detected: &[Detected], y2050: bool) -> (Estimate, Estimate) {
    let mine: Vec<&Detected> = detected
        .iter()
        .filter(|d| d.candidate.hazard == r.hazard)
        .collect();
    if mine.is_empty() {
        return (r.today.clone(), r.effective(y2050).clone());
    }
    let (mut today, mut future) = mine
        .iter()
        .find_map(|d| d.remainder.clone())
        .unwrap_or_else(|| (r.today.clone(), r.future.clone()));
    for d in mine {
        today = today.plus(&d.today);
        future = future.plus(&d.future);
    }
    let effective = if y2050 { future } else { today.clone() };
    (today, effective)
}

fn household_rate(hazard: HazardId, e: &Estimate) -> HouseholdEventRate {
    HouseholdEventRate {
        hazard,
        rate_per_year: e.value,
        low: e.low,
        high: e.high,
        evidence: e.evidence,
        sources: e.sources.clone(),
    }
}

fn chance(rate: f64) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -math::exp_m1(-rate)
    }
}

fn profile(ctx: &Ctx<'_>, r: &HazardRate, e: &Estimate) -> HazardProfile {
    let y2050 = ctx.y2050();
    let climate_multiplier = match &r.climate {
        Climate::Projected { multiplier, .. } if y2050 => multiplier.value,
        _ => 1.0,
    };
    let frequency_sentence = match &r.range_sentence {
        Some(s) => s.clone(),
        None => sentence::natural_frequency(
            sentence::Frequency {
                rate: e.value,
                low: e.low,
                high: e.high,
                show_range: e.evidence == rr_types::Evidence::Prior,
                years: ctx.years(),
                around_2050: y2050 && (climate_multiplier - 1.0).abs() > 1e-9,
            },
            &r.verb,
        ),
    };
    HazardProfile {
        id: r.hazard,
        name: r.hazard.name().to_owned(),
        tier: r.hazard.tier(),
        display: r.display,
        rate_per_year: e.value,
        rate_range: [e.low, e.high],
        annual_probability: chance(e.value),
        probability_range: [chance(e.low), chance(e.high)],
        severity: severity::for_household(r, ctx.input),
        eal_per_household_usd: r.eal_per_household,
        climate_multiplier,
        confidence: r
            .fixed_confidence
            .unwrap_or_else(|| severity::confidence(e)),
        sources: e.sources.clone(),
        frequency_sentence,
        buckets: buckets::for_hazard(r.hazard),
    }
}

/// A natural hazard's plural name for lists in notes ("heat waves", "tornadoes").
fn plural(hazard: HazardId) -> &'static str {
    use HazardId::*;
    match hazard {
        Avalanche => "avalanches",
        CoastalFlooding => "coastal floods",
        ColdWave => "cold waves",
        Drought => "droughts",
        Earthquake => "earthquakes",
        Hail => "hailstorms",
        HeatWave => "heat waves",
        Hurricane => "hurricanes",
        IceStorm => "ice storms",
        Landslide => "landslides",
        Lightning => "lightning strikes",
        RiverineFlooding => "floods from rivers or heavy rain",
        StrongWind => "windstorms",
        Tornado => "tornadoes",
        Tsunami => "tsunamis",
        VolcanicActivity => "volcanic eruptions",
        Wildfire => "wildfires",
        WinterWeather => "winter storms",
        other => other.name(),
    }
}

/// What a county has too few of to project a change, for the note.
fn too_few_of(hazard: HazardId) -> &'static str {
    match hazard {
        HazardId::ColdWave | HazardId::WinterWeather => "freezing days",
        HazardId::RiverineFlooding => "days of very heavy rain",
        _ => "dry spells",
    }
}

fn join_lower(names: &[&str]) -> String {
    let lower: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    match lower.len() {
        0 => String::new(),
        1 => lower[0].clone(),
        n => format!("{} and {}", lower[..n - 1].join(", "), lower[n - 1]),
    }
}

/// The 2050 note: what changes, what does not, and what is unclear.
fn climate_notes(rates: &[HazardRate], notes: &mut Notes) {
    let mut projected = Vec::new();
    let mut no_increase = Vec::new();
    let mut unclear = Vec::new();
    let mut missing = Vec::new();
    let mut too_few: Vec<(&'static str, &'static str)> = Vec::new();
    let mut not_exposed = Vec::new();
    for r in rates {
        let name = plural(r.hazard);
        match &r.climate {
            Climate::Projected { multiplier, .. }
                if multiplier.low == 1.0 && multiplier.high == 1.0 =>
            {
                no_increase.push(name)
            }
            Climate::Projected { what, .. } => projected.push(what.clone()),
            Climate::Unclear => unclear.push(name),
            Climate::NoData => missing.push(name),
            Climate::TooFew => too_few.push((too_few_of(r.hazard), name)),
            Climate::NotExposed => not_exposed.push(name),
            Climate::NotApplicable => {}
        }
    }
    if !projected.is_empty() {
        notes.add(format!(
            "Around 2050 (climate projections; where there is a range, it runs from middle to \
             high emissions): {}.",
            projected.join("; ")
        ));
    }
    if !no_increase.is_empty() {
        notes.add(format!(
            "No increase is projected here by 2050 for {}.",
            join_lower(&no_increase)
        ));
    }
    if !not_exposed.is_empty() {
        notes.add(format!(
            "Heavier rain by 2050 is not added to {} because your home is above the ground \
             floor.",
            join_lower(&not_exposed)
        ));
    }
    let mut kinds: Vec<&str> = too_few.iter().map(|(k, _)| *k).collect();
    kinds.dedup();
    for kind in kinds {
        let names: Vec<&str> = too_few
            .iter()
            .filter(|(k, _)| *k == kind)
            .map(|(_, n)| *n)
            .collect();
        notes.add(format!(
            "This county has too few {kind} to project a change for {}, so they are left as \
             today.",
            join_lower(&names)
        ));
    }
    notes.add(
        "Earthquakes, tsunamis and volcanoes, and risks such as job loss, house fires and \
         pandemics, are not changed for 2050.",
    );
    if !unclear.is_empty() {
        notes.add(format!(
            "How often these will happen by 2050 is unclear, so they are left as today: {}.",
            join_lower(&unclear)
        ));
    }
    if !missing.is_empty() {
        notes.add(format!(
            "No climate projection for this county covers {}, so they are left as today.",
            join_lower(&missing)
        ));
    }
}
