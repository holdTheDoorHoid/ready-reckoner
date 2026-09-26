//! Named scenarios (DESIGN §4.2, §4.4 "the cliff rule"; research §3.3).
//!
//! A scenario is a rare, severe version of a hazard whose single rate sits close to the dial and
//! would otherwise make targets jump between dial settings. The engine decides which apply to a
//! location, sets a default (on where state guidance addresses it or the rate reaches the
//! one-in-100 yardstick), and the user can override it with `dials.scenario_overrides`.
//!
//! **How a scenario relates to its parent hazard.** The parent's entry in
//! [`crate::HazardAssessment::rates`] is the parent's *full* rate. `rr-consequence` adds the
//! scenario as its own event class, with its own durations, when `on` is true, and drops the
//! parent's overlapping event class when the scenario is offered (its effects table marks those
//! rows `replaced_by`, as for major hurricanes), so the same event is not planned for twice.
//!
//! The parent's register card (`HazardProfile`) shows the de-duplicated total whether the
//! scenario is on or off, because the register describes the place, not the plan: for
//! hurricanes the Category 1–2 part plus the major part (which is the full rate); for
//! earthquakes the county rate less the scenario's long-run share (never below a quarter of it)
//! plus the scenario's own rate; for tsunamis the county rate plus the local-source scenario.

use serde::{Deserialize, Serialize};

use rr_types::{CitationId, Evidence, HazardId, HouseholdEventRate, math};

use crate::cite;
use crate::ctx::{Ctx, Notes};
use crate::estimate::Estimate;
use crate::natural::Natural;
use crate::params::*;

/// Another published reading of a scenario's rate, shown beside the one used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlternativeRate {
    /// Plain words for what this reading is.
    pub label: String,
    /// Household events a year under this reading.
    pub rate_per_year: f64,
    /// Where it comes from.
    pub sources: Vec<CitationId>,
}

/// A named scenario that applies to this location (for example `cascadia_m9`).
///
/// The fields `id`, `name`, `hazard`, `rate_per_year`, `low`, `high`, `on`, `applies_because`,
/// `variant` and `sources` have the same names and meaning as `rr-consequence`'s own
/// `ScenarioCandidate`, so the plan crate can copy them across; the rest are extra.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioCandidate {
    /// Stable id (`cascadia_m9`, `new_madrid_m7`, `hayward_m7`, `local_tsunami`,
    /// `major_hurricane_direct_hit`), as used in `dials.scenario_overrides` and
    /// `ScenarioInfo::id`.
    pub id: String,
    /// Plain name.
    pub name: String,
    /// The hazard it is a severe version of (the family it is counted under).
    pub hazard: HazardId,
    /// Household events a year when the scenario is on (climate multiplier applied).
    pub rate_per_year: f64,
    /// Low end of the plausible rate.
    pub low: f64,
    /// High end of the plausible rate.
    pub high: f64,
    /// What the rate rests on.
    pub evidence: Evidence,
    /// On in this plan: the default, or the user's override from `dials.scenario_overrides`.
    pub on: bool,
    /// On by default here (state guidance addresses it, or its rate reaches half the one-in-100
    /// yardstick).
    pub default_on: bool,
    /// The user's override decided `on`.
    pub overridden: bool,
    /// Why it applies here and why it is on or off by default, in plain language.
    pub applies_because: String,
    /// Which set of consequences applies: `coast` or `valley` for Cascadia (the Oregon Resilience
    /// Plan gives different restoration times for each); absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// Other published readings of the rate (for Cascadia near Coos Bay: the long-run
    /// recurrence, about 0.4 % a year, beside the time-dependent 1.0 % a year that is used).
    pub alternatives: Vec<AlternativeRate>,
    /// Where the scenario, its rate and its default come from.
    pub sources: Vec<CitationId>,
}

impl ScenarioCandidate {
    /// The scenario's rate as a [`HouseholdEventRate`] of its parent hazard.
    pub fn household_rate(&self) -> HouseholdEventRate {
        HouseholdEventRate {
            hazard: self.hazard,
            rate_per_year: self.rate_per_year,
            low: self.low,
            high: self.high,
            evidence: self.evidence,
            sources: self.sources.clone(),
        }
    }
}

/// A detected scenario plus what it does to its parent hazard's register card.
#[derive(Debug, Clone)]
pub(crate) struct Detected {
    pub candidate: ScenarioCandidate,
    /// The scenario's household rate today and around 2050.
    pub today: Estimate,
    pub future: Estimate,
    /// The parent hazard's rate without the part this scenario stands for (today, around 2050),
    /// when the parent's rate already contains it. Used only for the register card, which shows
    /// remainder + scenario so nothing is counted twice there; the rate handed to
    /// `rr-consequence` stays the parent's full rate.
    pub remainder: Option<(Estimate, Estimate)>,
}

/// Zones for Cascadia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    Coast,
    Valley,
}

impl Zone {
    fn variant(self) -> &'static str {
        match self {
            Zone::Coast => "coast",
            Zone::Valley => "valley",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Margin {
    /// The southern margin (Coos Bay to Cape Mendocino), which also ruptures on its own.
    South,
    /// Full-margin ruptures only.
    North,
}

use Zone::{Coast, Valley};

/// Counties shaken hard by a Cascadia subduction earthquake, with their zone and margin.
const CASCADIA: &[(&str, Zone, Margin)] = &[
    ("41007", Coast, Margin::North),  // Clatsop, OR
    ("41057", Coast, Margin::North),  // Tillamook, OR
    ("41041", Coast, Margin::North),  // Lincoln, OR
    ("41011", Coast, Margin::South),  // Coos, OR
    ("41015", Coast, Margin::South),  // Curry, OR
    ("41003", Valley, Margin::North), // Benton, OR
    ("41005", Valley, Margin::North), // Clackamas, OR
    ("41009", Valley, Margin::North), // Columbia, OR
    ("41043", Valley, Margin::North), // Linn, OR
    ("41047", Valley, Margin::North), // Marion, OR
    ("41051", Valley, Margin::North), // Multnomah, OR
    ("41053", Valley, Margin::North), // Polk, OR
    ("41067", Valley, Margin::North), // Washington, OR
    ("41071", Valley, Margin::North), // Yamhill, OR
    ("41019", Valley, Margin::South), // Douglas, OR
    ("41029", Valley, Margin::South), // Jackson, OR
    ("41033", Valley, Margin::South), // Josephine, OR
    ("41039", Valley, Margin::South), // Lane, OR
    ("53009", Coast, Margin::North),  // Clallam, WA
    ("53027", Coast, Margin::North),  // Grays Harbor, WA
    ("53031", Coast, Margin::North),  // Jefferson, WA
    ("53049", Coast, Margin::North),  // Pacific, WA
    ("53011", Valley, Margin::North), // Clark, WA
    ("53015", Valley, Margin::North), // Cowlitz, WA
    ("53029", Valley, Margin::North), // Island, WA
    ("53033", Valley, Margin::North), // King, WA
    ("53035", Valley, Margin::North), // Kitsap, WA
    ("53041", Valley, Margin::North), // Lewis, WA
    ("53045", Valley, Margin::North), // Mason, WA
    ("53053", Valley, Margin::North), // Pierce, WA
    ("53055", Valley, Margin::North), // San Juan, WA
    ("53057", Valley, Margin::North), // Skagit, WA
    ("53061", Valley, Margin::North), // Snohomish, WA
    ("53067", Valley, Margin::North), // Thurston, WA
    ("53069", Valley, Margin::North), // Wahkiakum, WA
    ("53073", Valley, Margin::North), // Whatcom, WA
    ("06015", Coast, Margin::South),  // Del Norte, CA
    ("06023", Coast, Margin::South),  // Humboldt, CA
    ("06045", Coast, Margin::South),  // Mendocino, CA
];

/// Counties near the Hayward and Rodgers Creek faults.
const HAYWARD: &[&str] = &[
    "06001", // Alameda
    "06013", // Contra Costa
    "06041", // Marin
    "06055", // Napa
    "06075", // San Francisco
    "06081", // San Mateo
    "06085", // Santa Clara
    "06095", // Solano
    "06097", // Sonoma
];

/// Counties in and around the New Madrid Seismic Zone.
const NEW_MADRID: &[&str] = &[
    "05021", "05031", "05035", "05055", "05093", "05111", // Arkansas
    "17003", "17127", "17153", // Illinois
    "21007", "21039", "21075", "21083", "21105", "21145", // Kentucky
    "29023", "29031", "29069", "29133", "29143", "29155", "29201", "29207", // Missouri
    "47045", "47095", "47097", "47131", "47157", "47167", // Tennessee
];

/// Rate a year from "a chance of `p` in `years` years".
fn rate_from_chance(p: f64, years: f64) -> f64 {
    -math::ln_1p(-p) / years
}

/// SRC (research S31): 40 % chance of a major Cascadia earthquake near Coos Bay in 50 years.
fn cascadia_south_time_dependent() -> f64 {
    rate_from_chance(0.40, 50.0)
}
/// SRC (research S31): 19 full-margin and 22 southern-only ruptures in 10,000 years.
const CASCADIA_SOUTH_RECURRENCE: f64 = 41.0 / 10_000.0;
/// SRC (research S31): 19 full-margin ruptures in 10,000 years.
const CASCADIA_NORTH_RECURRENCE: f64 = 19.0 / 10_000.0;
/// The parent's rate never falls below this share of itself when a scenario is taken out.
const REMAINDER_FLOOR: f64 = 0.25;

/// The one-in-100 yardstick of the default dial.
const DEFAULT_DIAL_RATE: f64 = 0.01;

/// The parent's estimate with `share` a year taken out, never below a quarter of itself.
fn without(parent: &Estimate, share: f64) -> Estimate {
    let f = |x: f64| (x - share).max(REMAINDER_FLOOR * x);
    let mut out = parent.clone();
    out.value = f(parent.value);
    out.low = f(parent.low);
    out.high = f(parent.high);
    out
}

fn ids(list: &[&str]) -> Vec<CitationId> {
    list.iter().map(|s| CitationId::from(*s)).collect()
}

/// Share of residents in the tsunami zone, and whether it came from data.
fn tsunami_share(ctx: &Ctx<'_>) -> (Estimate, bool) {
    match ctx.exposure_share(HazardId::Tsunami) {
        Some(s) if s > 0.0 => (Estimate::data(s, s, s, &[cite::NRI]), true),
        _ => (
            prior(TSUNAMI_ZONE_SHARE_FALLBACK, &[cite::RR_HAZARD_PRIORS]),
            false,
        ),
    }
}

struct Draft {
    id: &'static str,
    name: &'static str,
    hazard: HazardId,
    variant: Option<&'static str>,
    today: Estimate,
    future: Estimate,
    alternatives: Vec<AlternativeRate>,
    default_on: bool,
    applies_because: String,
    sources: Vec<CitationId>,
    remainder: Option<(Estimate, Estimate)>,
}

fn cascadia(ctx: &Ctx<'_>, natural: &Natural) -> Option<(Draft, Margin, Zone)> {
    let &(_, zone, margin) = CASCADIA.iter().find(|(f, _, _)| *f == ctx.county.fips)?;
    let county = ctx.county_label();
    let state = ctx.county.state_abbr.as_str();
    let (today, recurrence, alternatives) = match margin {
        Margin::South => {
            let r = cascadia_south_time_dependent();
            (
                Estimate::data(
                    r,
                    CASCADIA_SOUTH_RECURRENCE,
                    r * 1.25,
                    &[cite::CASCADIA_2012],
                ),
                CASCADIA_SOUTH_RECURRENCE,
                vec![AlternativeRate {
                    label: "Long-run average from the 10,000-year record (about 1 in 240 years)"
                        .to_owned(),
                    rate_per_year: CASCADIA_SOUTH_RECURRENCE,
                    sources: ids(&[cite::CASCADIA_2012]),
                }],
            )
        }
        Margin::North => (
            Estimate::data(
                CASCADIA_NORTH_RECURRENCE,
                0.0013,
                0.003,
                &[cite::CASCADIA_2012],
            ),
            CASCADIA_NORTH_RECURRENCE,
            Vec::new(),
        ),
    };
    let chance = match margin {
        Margin::South => {
            "Scientists put the chance of a major Cascadia earthquake near Coos Bay at about 40 \
             in 100 over the next 50 years."
        }
        Margin::North => {
            "Scientists count 19 full-length Cascadia earthquakes in the last 10,000 years, about \
             one every 500 years."
        }
    };
    let place = match zone {
        Coast => format!("{county} is on the Cascadia coast, next to the fault."),
        _ => format!(
            "{county} is inland from the Cascadia fault, where a magnitude 9 earthquake would \
             still shake hard and cut power and water for weeks to months."
        ),
    };
    let (default_on, why_default, mut sources) = match state {
        "OR" => (
            true,
            " Oregon asks every household to be ready for at least two weeks, so the plan \
             includes it."
                .to_owned(),
            ids(&[cite::CASCADIA_2012, cite::OREGON_TWO_WEEKS, cite::ORP_2013]),
        ),
        "WA" => (
            true,
            " Washington asks every household to be two weeks ready, so the plan includes it."
                .to_owned(),
            ids(&[cite::CASCADIA_2012, cite::WA_TWO_WEEKS]),
        ),
        _ => {
            let on = today.value >= 0.5 * DEFAULT_DIAL_RATE;
            let why = if on {
                " That is close to the one-in-100-a-year yardstick, so the plan includes it."
            } else {
                " That is rarer than the one-in-100-a-year yardstick, so it is off unless you \
                 turn it on."
            };
            (on, why.to_owned(), ids(&[cite::CASCADIA_2012]))
        }
    };
    for id in [cite::CASCADIA_PP1661F, cite::ORP_2013] {
        let id = CitationId::from(id);
        if !sources.contains(&id) {
            sources.push(id);
        }
    }
    let remainder = natural
        .rates
        .iter()
        .find(|r| r.hazard == HazardId::Earthquake)
        .map(|r| {
            (
                without(&r.today, recurrence),
                without(&r.future, recurrence),
            )
        });
    let draft = Draft {
        id: "cascadia_m9",
        name: "Magnitude 9 Cascadia earthquake",
        hazard: HazardId::Earthquake,
        variant: Some(zone.variant()),
        future: today.clone(),
        today,
        alternatives,
        default_on,
        applies_because: format!("{place} {chance}{why_default}"),
        sources,
        remainder,
    };
    Some((draft, margin, zone))
}

fn named_quake(
    ctx: &Ctx<'_>,
    natural: &Natural,
    id: &'static str,
    name: &'static str,
    today: Estimate,
    lead: &str,
) -> Draft {
    let county = ctx.county_label();
    let default_on = today.value >= 0.5 * DEFAULT_DIAL_RATE;
    let why = if default_on {
        "That is more than the one-in-100-a-year yardstick, so the plan includes it."
    } else {
        "That is rarer than the one-in-100-a-year yardstick, so it is off unless you turn it on."
    };
    let share = today.value;
    let remainder = natural
        .rates
        .iter()
        .find(|r| r.hazard == HazardId::Earthquake)
        .map(|r| (without(&r.today, share), without(&r.future, share)));
    Draft {
        id,
        name,
        hazard: HazardId::Earthquake,
        variant: None,
        sources: today.sources.clone(),
        future: today.clone(),
        today,
        alternatives: Vec::new(),
        default_on,
        applies_because: format!("{county} {lead} {why}"),
        remainder,
    }
}

fn local_tsunami(ctx: &Ctx<'_>, cascadia: Option<(Margin, Zone)>) -> Option<Draft> {
    if !(ctx.county.tsunami_zone || ctx.location.tsunami_zone) {
        return None;
    }
    let county = ctx.county_label();
    let (share, from_data) = tsunami_share(ctx);
    let (source_rate, alternatives, minutes, mut sources) = match cascadia {
        Some((margin, _)) => {
            let (r, alt) = match margin {
                Margin::South => (
                    Estimate::data(
                        cascadia_south_time_dependent(),
                        CASCADIA_SOUTH_RECURRENCE,
                        cascadia_south_time_dependent() * 1.25,
                        &[cite::CASCADIA_2012],
                    ),
                    Some(CASCADIA_SOUTH_RECURRENCE),
                ),
                Margin::North => (
                    Estimate::data(
                        CASCADIA_NORTH_RECURRENCE,
                        0.0013,
                        0.003,
                        &[cite::CASCADIA_2012],
                    ),
                    None,
                ),
            };
            let alternatives = alt
                .map(|a| AlternativeRate {
                    label: "Long-run average from the 10,000-year record".to_owned(),
                    rate_per_year: a * share.value,
                    sources: ids(&[cite::CASCADIA_2012]),
                })
                .into_iter()
                .collect();
            (
                r,
                alternatives,
                "the first waves arrive in 15 to 20 minutes",
                ids(&[cite::DOGAMI_TSUNAMI, cite::CASCADIA_2012]),
            )
        }
        None => (
            prior(LOCAL_TSUNAMI_OTHER, &[cite::RR_HAZARD_PRIORS]),
            Vec::new(),
            "the first waves can arrive within minutes",
            ids(&[cite::RR_HAZARD_PRIORS]),
        ),
    };
    if from_data {
        sources.push(CitationId::from(cite::NRI));
    }
    let today = source_rate.times(&share);
    let who = if from_data {
        let n = (share.value * 100.0).round().max(1.0);
        format!(
            "Part of {county} is in the tsunami zone: about {n:.0} in 100 residents live there."
        )
    } else {
        format!("Part of {county} is in the tsunami zone.")
    };
    Some(Draft {
        id: "local_tsunami",
        name: "Tsunami from a nearby earthquake",
        hazard: HazardId::Tsunami,
        variant: None,
        future: today.clone(),
        today,
        alternatives,
        default_on: true,
        applies_because: format!(
            "{who} After a nearby earthquake {minutes}, so anyone who lives, works or goes to \
             school in the zone should walk to high ground as soon as the shaking stops. \
             Knowing the route costs nothing, so the plan includes it."
        ),
        sources,
        remainder: None,
    })
}

fn major_hurricane(ctx: &Ctx<'_>, natural: &Natural) -> Option<Draft> {
    let split = natural.hurricane.as_ref()?;
    let state = ctx.county.state_abbr.as_str();
    if split.county_rate < MAJOR_HURRICANE_AFREQ_THRESHOLD || !HURRICANE_STATES.contains(&state) {
        return None;
    }
    // A county whose HURDAT2 record shows no major-hurricane passages has nothing to offer.
    if split.major_today.value < NEGLIGIBLE_RATE && split.major_future.value < NEGLIGIBLE_RATE {
        return None;
    }
    let county = ctx.county_label();
    let state_name = ctx.county.state_name.as_str();
    let years = (1.0 / split.county_rate).round().max(1.0);
    let one_in = (1.0 / split.major_share.max(1e-6)).round().max(1.0);
    let mut sources = split.major_today.sources.clone();
    for id in [cite::NRI, cite::HURDAT2] {
        let id = CitationId::from(id);
        if !sources.contains(&id) {
            sources.push(id);
        }
    }
    Some(Draft {
        id: "major_hurricane_direct_hit",
        name: "Direct hit by a major hurricane",
        hazard: HazardId::Hurricane,
        variant: None,
        today: split.major_today.clone(),
        future: split.major_future.clone(),
        alternatives: Vec::new(),
        default_on: true,
        applies_because: format!(
            "Tropical storms and hurricanes reach {county} about once every {years:.0} years, \
             and about 1 in {one_in:.0} of them is a major storm (Category 3 or stronger) that \
             can cut power and water for weeks. Hurricane guidance in {state_name} covers it, so \
             the plan includes it."
        ),
        sources,
        remainder: Some((split.cat12_today.clone(), split.cat12_future.clone())),
    })
}

/// The scenarios that apply to this location, in a fixed order, with the user's overrides
/// applied.
pub(crate) fn detect(ctx: &Ctx<'_>, natural: &Natural, notes: &mut Notes) -> Vec<Detected> {
    let mut drafts = Vec::new();
    let mut cascadia_where = None;
    if let Some((d, margin, zone)) = cascadia(ctx, natural) {
        cascadia_where = Some((margin, zone));
        drafts.push(d);
    }
    let fips = ctx.county.fips.as_str();
    if NEW_MADRID.contains(&fips) {
        // SRC (USGS): 7–10 % chance of a repeat of the 1811–12 earthquakes in 50 years.
        let today = Estimate::data(
            rate_from_chance(0.085, 50.0),
            rate_from_chance(0.07, 50.0),
            rate_from_chance(0.10, 50.0),
            &[cite::NEW_MADRID],
        );
        drafts.push(named_quake(
            ctx,
            natural,
            "new_madrid_m7",
            "Magnitude 7 New Madrid earthquake",
            today,
            "is near the New Madrid fault zone. The USGS puts the chance of a repeat of the \
             1811–1812 earthquakes at 7 to 10 in 100 over the next 50 years.",
        ));
    }
    if HAYWARD.contains(&fips) {
        // SRC (USGS Fact Sheet 2015-3009, UCERF3): 33 % chance of magnitude 6.7+ on the
        // Hayward–Rodgers Creek fault in 30 years; the range is a PRIOR spread.
        let today = Estimate::data(
            rate_from_chance(0.33, 30.0),
            rate_from_chance(0.20, 30.0),
            rate_from_chance(0.45, 30.0),
            &[cite::UCERF3],
        );
        drafts.push(named_quake(
            ctx,
            natural,
            "hayward_m7",
            "Magnitude 7 Hayward fault earthquake",
            today,
            "is near the Hayward and Rodgers Creek faults. The USGS puts the chance of a \
             magnitude 6.7 or larger earthquake on them at about 33 in 100 over the next 30 \
             years.",
        ));
    }
    drafts.extend(local_tsunami(ctx, cascadia_where));
    drafts.extend(major_hurricane(ctx, natural));

    let overrides = &ctx.input.dials.scenario_overrides;
    for o in overrides {
        if !drafts.iter().any(|d| d.id == o.id) {
            notes.add(format!(
                "The scenario setting \"{}\" does not apply to {}, so it was ignored.",
                o.id,
                ctx.county_label()
            ));
        }
    }
    let y2050 = ctx.y2050();
    drafts
        .into_iter()
        .map(|d| {
            let toggle = overrides.iter().find(|o| o.id == d.id);
            let effective = if y2050 { &d.future } else { &d.today };
            let mut sources = d.sources;
            for id in &effective.sources {
                if !sources.contains(id) {
                    sources.push(id.clone());
                }
            }
            Detected {
                candidate: ScenarioCandidate {
                    id: d.id.to_owned(),
                    name: d.name.to_owned(),
                    hazard: d.hazard,
                    rate_per_year: effective.value,
                    low: effective.low,
                    high: effective.high,
                    evidence: effective.evidence,
                    on: toggle.map_or(d.default_on, |t| t.on),
                    default_on: d.default_on,
                    overridden: toggle.is_some(),
                    applies_because: d.applies_because,
                    variant: d.variant.map(str::to_owned),
                    alternatives: d.alternatives,
                    sources,
                },
                today: d.today,
                future: d.future,
                remainder: d.remainder,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_chances_become_rates() {
        // 40 % in 50 years -> 1.02 % a year (research §3.3 uses 1.0 %/yr; the prototype 0.0102).
        assert!((cascadia_south_time_dependent() - 0.010_216).abs() < 1e-6);
        // "Every 240 years or so" -> about 0.4 % a year.
        assert!((CASCADIA_SOUTH_RECURRENCE - 0.0041).abs() < 1e-12);
        // 33 % in 30 years -> 1.33 % a year.
        assert!((rate_from_chance(0.33, 30.0) - 0.013_349).abs() < 1e-6);
        // 7–10 % in 50 years -> 0.145–0.211 % a year.
        assert!((rate_from_chance(0.07, 50.0) - 0.001_451).abs() < 1e-6);
        assert!((rate_from_chance(0.10, 50.0) - 0.002_107).abs() < 1e-6);
    }

    #[test]
    fn county_lists_are_valid_fips_without_repeats() {
        let mut all: Vec<&str> = CASCADIA.iter().map(|(f, _, _)| *f).collect();
        all.extend(HAYWARD);
        all.extend(NEW_MADRID);
        for f in &all {
            assert!(f.len() == 5 && f.bytes().all(|b| b.is_ascii_digit()), "{f}");
        }
        let mut sorted = all.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "a county is listed twice");
    }

    #[test]
    fn removing_a_scenario_never_takes_the_parent_below_a_quarter() {
        let parent = Estimate::data(0.0196, 0.0098, 0.0392, &["x"]);
        let r = without(&parent, 0.0041);
        assert!((r.value - 0.0155).abs() < 1e-9);
        assert!(r.low <= r.value && r.value <= r.high);
        let r = without(&parent, 0.1);
        assert!((r.value - 0.25 * 0.0196).abs() < 1e-12);
    }
}
