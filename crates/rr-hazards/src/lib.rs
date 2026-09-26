//! Ready Reckoner — `rr-hazards`: location + household → a household event rate for every hazard
//! (DESIGN §4.2), the risk-register cards, and the named scenarios that apply.
//!
//! ```text
//! PlanInput + CountyRecord + [BaseRate] + LocationResolved ─► assess ─► HazardAssessment
//!                                                                    ├─ profiles     (register cards)
//!                                                                    ├─ rates        (to rr-consequence)
//!                                                                    ├─ scenarios    (named scenarios)
//!                                                                    ├─ also_checked (under 1 in 100,000)
//!                                                                    └─ notes        (plain-language caveats)
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
//!   department visits; arrests count arrests (events, not people).
//!
//! # The rare families
//!
//! The nine rare families (contract v2, [`HazardId::RARE`]) are display-only: they are in
//! [`HazardAssessment::profiles`], range only, sorted by how likely they are here, but never in
//! [`HazardAssessment::rates`], so no rare row enters a bucket's Λ, a target or the budget
//! (REVIEW §2.3–§2.4). See the `rare` module.
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
mod exposure;
mod natural;
mod params;
mod personal;
mod rare;
mod rate;
mod scenarios;
mod sentence;
mod severity;
mod societal;
mod strategic;
mod subcauses;
mod why;

use serde::{Deserialize, Serialize};

use rr_types::{
    BaseRate, CitationId, CountyRecord, Evidence, HazardDisplay, HazardId, HazardProfile,
    HouseholdEventRate, LocationResolved, PlanInput, SubCause, math,
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
    /// by today's rate, so the 2050 dial never re-orders the list silently), then the nine
    /// `rare_catastrophic` families, most likely here first (by the middle of their range, which
    /// is never shown).
    pub profiles: Vec<HazardProfile>,
    /// Household event rates for `rr-consequence`, one per **ranked** hazard in the register, in
    /// `HazardId` order: each hazard's full rate (a named scenario is an extra event class). The
    /// rare families are never here: they are shown, not planned for.
    pub rates: Vec<HouseholdEventRate>,
    /// Named scenarios that apply to this location, with defaults and the user's overrides.
    pub scenarios: Vec<ScenarioCandidate>,
    /// Parts of a hazard's rate that `rr-consequence` treats as separate event classes (its
    /// effects rows with a `part`): wildfire warnings to leave ([`WILDFIRE_BURN_PART`]) and
    /// safety power shutoffs ([`WILDFIRE_SHUTOFF_PART`]), which add up to the wildfire rate; and
    /// the landslides that damage the home ([`LANDSLIDE_DAMAGE_PART`]), a part of the landslide
    /// rate (the rest cut off the road).
    #[serde(default)]
    pub parts: Vec<RatePart>,
    /// "Also checked": every hazard and rare sub-row checked for this household and found under
    /// 1 in 100,000 a year, with its rate (REVIEW §2.4, H-12). They have no card; the packet and
    /// the risks screen list them in one line.
    #[serde(default)]
    pub also_checked: Vec<AlsoChecked>,
    /// Plain-language caveats for the packet and the "why" drawers.
    pub notes: Vec<String>,
}

/// A hazard or rare sub-row checked for this household and found under 1 in 100,000 a year.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlsoChecked {
    /// A hazard id (`tornado`) or a rare sub-row id (`asteroid`, `yellowstone`).
    pub id: String,
    /// What it is, in plain words, lower case ("tornadoes", "an asteroid or comet impact").
    pub name: String,
    /// Events a year for this household (0 when none is recorded).
    pub rate_per_year: f64,
    /// A plausible low and high, as `[low, high]`.
    pub rate_range: [f64; 2],
    /// Where the rate comes from.
    pub sources: Vec<CitationId>,
}

/// The wildfire part that counts warnings to leave home (NRI burn probability × residents
/// exposed × households warned per home that burns).
pub const WILDFIRE_BURN_PART: &str = "burn";

/// The wildfire part that counts safety power shutoffs (western states; zero elsewhere).
pub const WILDFIRE_SHUTOFF_PART: &str = "shutoff";

/// The landslide part that counts damage to the home (the rest of the landslide rate counts
/// roads cut off).
pub const LANDSLIDE_DAMAGE_PART: &str = "damage";

/// One part of a hazard's household rate that `rr-consequence` keeps as its own event class
/// (model review M-06: warnings to leave and power shutoffs were added together, then re-split
/// 15/85, which undercounted evacuations four- to sevenfold and gave Hawaii shutoffs it does not
/// have).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatePart {
    /// The hazard.
    pub hazard: HazardId,
    /// Which part ([`WILDFIRE_BURN_PART`], [`WILDFIRE_SHUTOFF_PART`]).
    pub part: String,
    /// Household events a year from this part (climate dial applied, as for `rates`).
    pub rate_per_year: f64,
    /// Low end of the plausible range.
    pub low: f64,
    /// High end of the plausible range.
    pub high: f64,
    /// What it rests on.
    pub evidence: Evidence,
    /// Where it comes from.
    pub sources: Vec<CitationId>,
}

impl RatePart {
    fn new(hazard: HazardId, part: &str, e: &Estimate) -> Self {
        RatePart {
            hazard,
            part: part.to_owned(),
            rate_per_year: e.value,
            low: e.low,
            high: e.high,
            evidence: e.evidence,
            sources: e.sources.clone(),
        }
    }
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
    missing_data_note(&ctx, &mut notes);

    // Every ranked hazard: natural, societal and personal. Those too rare to matter here (under
    // 1 in 100,000 a year today and around 2050) go to "Also checked", unless a scenario hangs
    // on them.
    let mut candidates: Vec<HazardRate> = natural.rates.clone();
    candidates.extend(societal::assess(&ctx, &mut notes));
    candidates.extend(personal::assess(&ctx, &mut notes));
    candidates.sort_by_key(|r| r.hazard);
    let mut rates: Vec<HazardRate> = Vec::new();
    let mut also_checked: Vec<AlsoChecked> = Vec::new();
    for r in candidates {
        let parent = detected.iter().any(|d| d.candidate.hazard == r.hazard);
        if r.today.value < params::NEGLIGIBLE_RATE
            && r.future.value < params::NEGLIGIBLE_RATE
            && !parent
        {
            let e = r.effective(y2050);
            also_checked.push(AlsoChecked {
                id: r.hazard.as_str().to_owned(),
                name: plural(r.hazard).to_owned(),
                rate_per_year: e.value,
                rate_range: [e.low, e.high],
                sources: e.sources.clone(),
            });
        } else {
            rates.push(r);
        }
    }
    for (id, name, (v, lo, hi), sources) in rare::also_checked() {
        also_checked.push(AlsoChecked {
            id: id.to_owned(),
            name: name.to_owned(),
            rate_per_year: v,
            rate_range: [lo, hi],
            sources: sources.iter().map(|s| CitationId::from(*s)).collect(),
        });
    }
    if !also_checked.is_empty() {
        let list: Vec<String> = also_checked
            .iter()
            .map(|a| format!("{} ({})", a.name, sentence::per_year_words(a.rate_per_year)))
            .collect();
        notes.add(format!(
            "Also checked, and under 1 in 100,000 a year here: {}.",
            join_plain(&list)
        ));
    }
    if y2050 {
        climate_notes(&rates, &mut notes);
    }
    for r in &mut rates {
        let mut named = subcauses::for_hazard(&ctx, r.hazard);
        r.sub_causes.append(&mut named);
    }
    at_risk_notes(input, &rates, &mut notes);

    // rr-consequence gets each ranked hazard's full rate; a named scenario is an extra event
    // class (see the `scenarios` module). The rare families never reach it.
    let household_rates = rates
        .iter()
        .map(|r| household_rate(r.hazard, r.effective(y2050)))
        .collect();
    // ... and the parts it keeps apart: wildfire warnings to leave and safety shutoffs, and the
    // landslides that damage the home (the rest only cut off the road).
    let kept = |h: HazardId| rates.iter().any(|r| r.hazard == h);
    let mut parts = Vec::new();
    if let Some(w) = natural
        .wildfire
        .as_ref()
        .filter(|_| kept(HazardId::Wildfire))
    {
        let (burn, shutoff) = if y2050 {
            (&w.burn_future, &w.shutoff_future)
        } else {
            (&w.burn_today, &w.shutoff_today)
        };
        parts.push(RatePart::new(HazardId::Wildfire, WILDFIRE_BURN_PART, burn));
        parts.push(RatePart::new(
            HazardId::Wildfire,
            WILDFIRE_SHUTOFF_PART,
            shutoff,
        ));
    }
    if let Some(d) = natural
        .landslide_damage
        .as_ref()
        .filter(|_| kept(HazardId::Landslide))
    {
        parts.push(RatePart::new(HazardId::Landslide, LANDSLIDE_DAMAGE_PART, d));
    }

    let mut ranked: Vec<(f64, HazardProfile)> = rates
        .iter()
        .map(|r| {
            let (today, effective) = register_rate(r, &detected, y2050);
            (today.value, profile(&ctx, r, &effective))
        })
        .collect();
    ranked.sort_by(|(ta, a), (tb, b)| {
        tb.total_cmp(ta)
            .then_with(|| b.severity.total_cmp(&a.severity))
            .then_with(|| a.id.cmp(&b.id))
    });
    let anchors: Vec<(HazardId, f64)> = ranked
        .iter()
        .map(|(_, p)| (p.id, p.rate_per_year))
        .collect();
    let mut rare_cards: Vec<HazardProfile> = rare::assess(&ctx, &mut notes)
        .iter()
        .map(|r| {
            let mut p = profile(&ctx, r, &r.today);
            p.anchor_sentence = anchor_for(p.rate_range[1], &anchors, ctx.years());
            p
        })
        .collect();
    rare_cards.sort_by(|a, b| {
        b.rate_per_year
            .total_cmp(&a.rate_per_year)
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut profiles: Vec<HazardProfile> = ranked.into_iter().map(|(_, p)| p).collect();
    profiles.extend(rare_cards);
    HazardAssessment {
        profiles,
        rates: household_rates,
        scenarios: detected.into_iter().map(|d| d.candidate).collect(),
        parts,
        also_checked,
        notes: notes.0,
    }
}

impl HazardAssessment {
    /// Finishes the "power out for months" family with the household's own power curve: `per_year`
    /// is the yearly rate of power cuts lasting 60 days or more that `rr-consequence` reads off
    /// its exceedance curve for this household, as `(value, low, high)`. `rr-hazards` runs
    /// before `rr-consequence`, so the family first holds only the solar-storm, EMP and war
    /// parts; the plan pipeline calls this once the curve exists. awaiting: plan — `rr-plan`
    /// calls it after `rr_consequence::assess_with_parts`; awaiting: consequence — the curve
    /// value at 60 days.
    pub fn add_power_curve(&mut self, per_year: (f64, f64, f64), years: u8) {
        let (v, lo, hi) = per_year;
        if !(v.is_finite() && lo.is_finite() && hi.is_finite()) || v < 0.0 {
            return;
        }
        let (lo, hi) = (lo.clamp(0.0, v), hi.max(v));
        let anchors: Vec<(HazardId, f64)> = self
            .profiles
            .iter()
            .filter(|p| p.display == HazardDisplay::Ranked)
            .map(|p| (p.id, p.rate_per_year))
            .collect();
        let Some(p) = self
            .profiles
            .iter_mut()
            .find(|p| p.id == HazardId::MultiMonthBlackout)
        else {
            return;
        };
        if p.sub_causes.iter().any(|s| s.id == "own_record") {
            return;
        }
        p.rate_per_year += v;
        p.rate_range = [p.rate_range[0] + lo, p.rate_range[1] + hi];
        p.annual_probability = chance(p.rate_per_year);
        p.probability_range = [chance(p.rate_range[0]), chance(p.rate_range[1])];
        p.frequency_sentence = sentence::range_sentence(
            "be without power for two months or more",
            p.rate_range[0],
            p.rate_range[1],
            years,
        );
        p.sub_causes.push(SubCause {
            id: "own_record".to_owned(),
            name: "Your county's own outage record".to_owned(),
            note: "Storms and failures like the ones in your county's outage records, with the \
                   long tail the records cannot show yet."
                .to_owned(),
            rate_range: Some([lo, hi]),
            sources: vec![CitationId::from(cite::EAGLE_I)],
        });
        let source = CitationId::from(cite::EAGLE_I);
        if !p.sources.contains(&source) {
            p.sources.push(source);
        }
        p.anchor_sentence = anchor_for(p.rate_range[1], &anchors, years);
    }
}

/// The anchor of a rare row (REVIEW §2.4): the household's own ranked hazard with the smallest
/// rate still above the row's upper bound.
fn anchor_for(high: f64, ranked: &[(HazardId, f64)], years: u8) -> Option<String> {
    ranked
        .iter()
        .filter(|(_, r)| *r > high)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(h, r)| sentence::anchor(anchor_phrase(*h), *r, years))
}

/// One note naming the v2 data columns the pack does not have for this county, so a reader knows
/// which rows rest on national averages or are left out, and one when the urban area's UASI share
/// is missing (the attack, CBRN and crude-device terms then fall back to a share of 0).
fn missing_data_note(ctx: &Ctx<'_>, notes: &mut Notes) {
    let e = ctx.exposure();
    let mut missing = Vec::new();
    if e.smoke().is_none() {
        missing.push("smoke days (wildfire smoke is left out)");
    }
    if e.karst_share().is_none() {
        missing.push("karst maps (sinkholes are left out)");
    }
    if e.levees().is_none() {
        missing.push("levees");
    }
    if e.geomag().is_none() {
        missing.push("geomagnetic latitude");
    }
    if !missing.is_empty() {
        notes.add(format!(
            "Some of the newer data are not loaded for {}: {}. The rows that need them use \
             national averages or are left out.",
            ctx.county_label(),
            join_plain(&missing.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        ));
    }
    // The metro weight falls back to 0 rather than to an average, so it has a note of its own.
    if matches!(e.uasi(), exposure::Uasi::Absent) {
        notes.add(format!(
            "FEMA's urban-area funding shares are not loaded for {}, so the estimates for an \
             attack that closes your area, a chemical, biological or radiological attack, and a \
             crude nuclear device count it as outside the 44 funded urban areas. In a big city \
             those estimates are too low.",
            ctx.county_label()
        ));
    }
}

/// Heat waves, cold waves, smoke and dust marked Serious for a household at risk: one note when
/// the reason is the same, otherwise one each.
fn at_risk_notes(input: &PlanInput, rates: &[HazardRate], notes: &mut Notes) {
    let reason = |h: HazardId| {
        rates
            .iter()
            .any(|r| r.hazard == h)
            .then(|| severity::at_risk_reason(input, h))
            .flatten()
    };
    let (heat, cold) = (reason(HazardId::HeatWave), reason(HazardId::ColdWave));
    let marked = match (heat, cold) {
        (Some(h), Some(c)) if h == c => vec![("Heat and cold waves", h)],
        (h, c) => [("Heat waves", h), ("Cold waves", c)]
            .into_iter()
            .filter_map(|(what, why)| why.map(|w| (what, w)))
            .collect(),
    };
    for (what, why) in marked {
        notes.add(format!(
            "{what} are marked Serious for this household because {why}; they are most \
             dangerous for households like yours."
        ));
    }
    let smoke = rates
        .iter()
        .any(|r| r.hazard == HazardId::WildfireSmoke && r.today.value >= params::NEGLIGIBLE_RATE)
        .then(|| severity::at_risk_reason(input, HazardId::WildfireSmoke))
        .flatten();
    if let Some(why) = smoke {
        notes.add(format!(
            "Wildfire smoke is marked Serious for this household because {why}; smoke harms them \
             most."
        ));
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
    let (mut today, mut future) = match mine.iter().find_map(|d| d.remainder.clone()) {
        Some(split) => split,
        None => {
            let (s_today, s_future) = mine
                .iter()
                .filter_map(|d| d.share)
                .fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
            (
                scenarios::without(&r.today, s_today),
                scenarios::without(&r.future, s_future),
            )
        }
    };
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
    let around_2050 = y2050 && (climate_multiplier - 1.0).abs() > 1e-9;
    let sentence_for = |e: &Estimate, verb: &str| {
        if r.range_only {
            sentence::range_sentence(verb, e.low, e.high, ctx.years())
        } else {
            sentence::natural_frequency(
                sentence::Frequency {
                    rate: e.value,
                    low: e.low,
                    high: e.high,
                    show_range: e.evidence == rr_types::Evidence::Prior,
                    years: ctx.years(),
                    around_2050,
                },
                verb,
            )
        }
    };
    let mut frequency_sentence = match &r.range_sentence {
        Some(s) => s.clone(),
        None => sentence_for(e, &r.verb),
    };
    if let Some((today, future, verb)) = &r.part_sentence {
        let part = if y2050 { future } else { today };
        frequency_sentence.push(' ');
        frequency_sentence.push_str(&sentence::part_frequency(
            sentence::Frequency {
                rate: part.value,
                low: part.low,
                high: part.high,
                show_range: part.evidence == rr_types::Evidence::Prior,
                years: ctx.years(),
                around_2050,
            },
            verb,
        ));
    }
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
        // Contract v2: a rare row names the family it heads (a family's id is its hazard's id).
        family: r.hazard.family().map(str::to_owned),
        sub_causes: r.sub_causes.clone(),
        location_factor: r.location_factor.clone(),
        range_only: r.range_only,
        anchor_sentence: None,
        if_it_reaches_you: r.if_it_reaches_you.clone(),
        what_it_changes: r.what_it_changes.clone(),
    }
}

/// How the anchor names a ranked hazard: "less likely than {phrase}".
fn anchor_phrase(hazard: HazardId) -> &'static str {
    use HazardId::*;
    match hazard {
        Avalanche => "an avalanche reaching your home or road",
        CoastalFlooding => "coastal flooding reaching your home",
        ColdWave => "a cold wave",
        Drought => "a drought that limits your water",
        Earthquake => "an earthquake strong enough to knock things off shelves",
        Hail => "hail damage",
        HeatWave => "a heat wave",
        Hurricane => "a hurricane or tropical storm",
        IceStorm => "an ice storm",
        Landslide => "a landslide",
        Lightning => "lightning damaging your home",
        RiverineFlooding => "flood water reaching your home",
        StrongWind => "a windstorm",
        Tornado => "a tornado",
        Tsunami => "a tsunami warning",
        VolcanicActivity => "ash or mudflows from a volcano",
        Wildfire => "a wildfire",
        WinterWeather => "a winter storm",
        WildfireSmoke => "days of wildfire smoke",
        DustStorm => "a dust storm",
        Sinkhole => "a sinkhole",
        Pandemic => "a pandemic that changes daily life",
        GridFailure => "a regional blackout",
        CyberOutage => "a computer outage that stops services",
        CivilUnrest => "a curfew",
        SupplyChainDisruption => "empty store shelves",
        HazmatRelease => "a chemical spill order",
        NuclearPlantIncident => "a nuclear plant accident",
        DamFailure => "a dam or levee failure",
        NetworkOutage => "a phone or internet outage",
        DrugShortage => "a medicine shortage",
        BenefitInterruption => "pay or benefits stopping",
        AttackDisruption => "an attack or threat closing your area",
        JobLoss => "a job loss",
        HouseFire => "a house fire",
        MedicalEmergency => "a medical emergency",
        VehicleStranding => "being stranded in a vehicle",
        LocalUtilityOutage => "a water main break or boil-water notice",
        Burglary => "a break-in",
        EarnerDeathOrDisability => "the death or disability of an earner",
        ExtendedHouseholdIllness => "a long illness at home",
        WaterDamage => "a burst pipe or leak",
        Eviction => "an eviction",
        ArrestOrDetention => "an arrest in the household",
        other => other.name(),
    }
}

/// "a, b and c" (items kept as written).
fn join_plain(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

/// A hazard's plural name for lists in notes ("heat waves", "tornadoes"), lower case.
pub(crate) fn plural(hazard: HazardId) -> &'static str {
    use HazardId::*;
    match hazard {
        WildfireSmoke => "days of unhealthy wildfire smoke",
        DustStorm => "dust storms",
        Sinkhole => "sinkholes",
        DamFailure => "dam or levee failures",
        AttackDisruption => "an attack or threat closing your area",
        NetworkOutage => "phone or internet outages",
        DrugShortage => "medicine shortages",
        BenefitInterruption => "pay or benefits stopping",
        NuclearPlantIncident => "a nuclear plant accident",
        WaterDamage => "burst pipes or leaks",
        Eviction => "eviction",
        ArrestOrDetention => "an arrest in the household",
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

pub(crate) fn join_lower(names: &[&str]) -> String {
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
