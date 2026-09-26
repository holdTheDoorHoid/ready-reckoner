//! Societal hazards: national rates from the base-rate pack where present, otherwise the expert
//! priors of research §6.2, each with a "why we think this" range (DESIGN §4.2, §13). The
//! footprint is 1 (they are regional or national events) except where a household or facility
//! modifier applies: setting for unrest and terrorism, TRI facilities for chemical releases,
//! distance for nuclear plants.
//!
//! Base-rate ids read from the base-rate pack (each optional):
//!
//! | id | unit | hazard |
//! | --- | --- | --- |
//! | `pandemic_onset_per_year` | onsets a year (about 0.046) | pandemic, × the share that disrupt daily life |
//! | `grid_failure_per_year` | per year | regional blackout |
//! | `cyber_outage_per_year` | per year | cyberattack on services |
//! | `civil_unrest_per_year` | per year, city household | civil unrest |
//! | `supply_chain_disruption_per_year` | per year | store shortages |
//! | `hazmat_release_per_year` | per year, typical county | chemical spill or release |

use rr_types::{DataConfidence, HazardDisplay, HazardId, math};

use crate::cite;
use crate::ctx::{Ctx, Notes};
use crate::estimate::Estimate;
use crate::params::*;
use crate::rate::HazardRate;
use crate::sentence;

/// A pack base rate if present (its own range, or the prior's relative range), else the prior.
fn base_or_prior(ctx: &Ctx<'_>, id: &str, builtin: Triple, sources: &[&str]) -> Estimate {
    match ctx.base_rate(&[id]) {
        Some(b) => {
            let v = b.value;
            let lo = b.low.map_or(v * builtin.1 / builtin.0, |x| x.min(v));
            let hi = b.high.map_or(v * builtin.2 / builtin.0, |x| x.max(v));
            Estimate::data(v, lo, hi, &[b.source.as_str()])
        }
        None => prior(builtin, sources),
    }
}

fn pandemic(ctx: &Ctx<'_>) -> HazardRate {
    let today = match ctx.base_rate(&["pandemic_onset_per_year"]) {
        // Onsets (about 4.5 % a year) × the share that change daily life as 1918 and 2020 did.
        Some(b) if b.value > 0.0 => {
            let v = b.value;
            let lo = b.low.map_or(v / 1.5, |x| x.min(v));
            let hi = b.high.map_or(v * 1.5, |x| x.max(v));
            Estimate::data(v, lo, hi, &[b.source.as_str()]).times(&prior(
                PANDEMIC_DISRUPTIVE_SHARE,
                &[cite::MARANI_2021, cite::RR_PRIORS],
            ))
        }
        _ => prior(
            PANDEMIC_DISRUPTIVE,
            &[cite::CDC_PANDEMICS, cite::MARANI_2021, cite::RR_PRIORS],
        ),
    };
    HazardRate::new(
        HazardId::Pandemic,
        today,
        "live through a pandemic that changes daily life",
        5_000.0,
    )
}

/// The TRI scaling for chemical releases: ×1 at a typical county, bounded ×0.65 to ×2, rising
/// with the logarithm of the facility count.
pub(crate) fn tri_factor(tri: u16) -> f64 {
    let x = (f64::from(tri) + 1.0) / TRI_REFERENCE_FACILITIES;
    (1.0 + 0.5 * math::ln(x) / core::f64::consts::LN_10).clamp(0.65, 2.0)
}

fn hazmat_release(ctx: &Ctx<'_>) -> HazardRate {
    let mut base = base_or_prior(
        ctx,
        "hazmat_release_per_year",
        CHEMICAL_RELEASE,
        &[cite::RR_PRIORS],
    );
    let (today, avg) = match ctx.county.facilities.as_ref() {
        Some(f) => {
            base = base.cite(&[cite::EPA_TRI]);
            let k = tri_factor(f.tri_facilities);
            let m = Estimate::prior(k, k.min(1.0), k.max(1.0), &[cite::RR_HAZARD_PRIORS]);
            (base.times(&m), base.value)
        }
        None => (base.clone(), base.value),
    };
    HazardRate::new(
        HazardId::HazmatRelease,
        today,
        "be told not to drink the tap water, or to stay inside, after a chemical spill",
        500.0,
    )
    .with_county_average(avg)
}

fn nuclear_plant_incident(ctx: &Ctx<'_>, notes: &mut Notes) -> Option<HazardRate> {
    let flags = &ctx.location.facility_flags;
    let km = ctx
        .county
        .facilities
        .as_ref()
        .and_then(|f| f.nearest_nuclear_km)
        .map(f64::from)
        .filter(|k| k.is_finite() && *k >= 0.0);
    let (epz, ingestion) = match km {
        Some(k) => (k <= 16.0, k <= 80.0),
        None => (
            flags.nuclear_plant_within_16km,
            flags.nuclear_plant_within_80km || flags.nuclear_plant_within_16km,
        ),
    };
    if !ingestion {
        return None;
    }
    let t = if epz {
        NUCLEAR_PLANT_EPZ
    } else {
        NUCLEAR_PLANT_INGESTION
    };
    let zone = if epz {
        "within 10 miles (16 km), the zone where people may be told to shelter or leave"
    } else {
        "within 50 miles (80 km), the zone where food and water may be checked"
    };
    notes.add(format!(
        "A nuclear power plant is {zone}; potassium iodide matters only inside the 10-mile zone \
         and is handed out by the authorities."
    ));
    Some(HazardRate::new(
        HazardId::NuclearPlantIncident,
        prior(t, &[cite::FEMA_NUCLEAR_SITES, cite::RR_HAZARD_PRIORS]),
        "be told to stay inside or leave after a nuclear power plant accident",
        20_000.0,
    ))
}

/// A rare catastrophe: its rate is the geometric middle of the range (for arithmetic only), its
/// sentence gives only the range (research §6.3), and it rests on expert judgement (published
/// forecasts for nuclear war, an estimate for terrorism).
fn rare(
    hazard: HazardId,
    (low, high): (f64, f64),
    sources: &[&str],
    sentence_text: String,
    severity: f64,
) -> HazardRate {
    let mid = (low * high).sqrt();
    let est = Estimate::prior(mid, low, high, sources);
    let mut r = HazardRate::new(hazard, est, "", 0.0);
    r.display = HazardDisplay::RareCatastrophic;
    r.range_sentence = Some(sentence_text);
    r.fixed_severity = Some(severity);
    r.fixed_confidence = Some(DataConfidence::Prior);
    r
}

fn nuclear_attack() -> HazardRate {
    let (lo, hi) = NUCLEAR_ATTACK_RANGE;
    rare(
        HazardId::NuclearAttack,
        (lo, hi),
        &[cite::FRI_NUCLEAR, cite::READY_NUCLEAR],
        sentence::range_only(
            "Forecasters asked in 2024 put the chance of a nuclear catastrophe anywhere in the \
             world (10 million or more deaths) before 2045 at 1 to 5 in 100. Spread over those \
             years, that is",
            lo,
            hi,
            " That is the chance for the whole world, not for your household: no reliable \
             estimate exists for effects where you live. The first 24 hours of sheltering inside \
             are covered by your basic supplies.",
        ),
        1.0,
    )
}

fn terrorism(ctx: &Ctx<'_>) -> HazardRate {
    let k = unrest_setting(ctx.setting()).value;
    let (lo, hi) = (TERRORISM_RANGE.0 * k, TERRORISM_RANGE.1 * k);
    rare(
        HazardId::Terrorism,
        (lo, hi),
        &[cite::RR_HAZARD_PRIORS],
        sentence::range_only(
            "An attack that shuts down the area where you live for half a day to two days (roads, \
             schools and shops closed) is rare: for a household like yours, expert estimates \
             range from",
            lo,
            hi,
            " This counts the disruption to daily life, not the chance of being hurt.",
        ),
        0.9,
    )
}

/// Every societal hazard that applies to this household.
pub(crate) fn assess(ctx: &Ctx<'_>, notes: &mut Notes) -> Vec<HazardRate> {
    let mut out = vec![
        pandemic(ctx),
        HazardRate::new(
            HazardId::GridFailure,
            base_or_prior(
                ctx,
                "grid_failure_per_year",
                GRID_FAILURE,
                &[cite::RR_PRIORS],
            ),
            "lose power in a regional blackout that is not caused by weather",
            1_000.0,
        ),
        HazardRate::new(
            HazardId::CyberOutage,
            base_or_prior(
                ctx,
                "cyber_outage_per_year",
                CYBER_OUTAGE,
                &[cite::RR_PRIORS],
            ),
            "lose pharmacy, payment or other services to a computer outage or cyberattack",
            300.0,
        ),
        {
            let base = base_or_prior(
                ctx,
                "civil_unrest_per_year",
                CIVIL_UNREST,
                &[cite::RR_PRIORS],
            );
            let today = base.times(&unrest_setting(ctx.setting()));
            HazardRate::new(
                HazardId::CivilUnrest,
                today,
                "live under a curfew because of unrest",
                300.0,
            )
            .with_county_average(base.value)
        },
        HazardRate::new(
            HazardId::SupplyChainDisruption,
            base_or_prior(
                ctx,
                "supply_chain_disruption_per_year",
                SUPPLY_SHORTAGE,
                &[cite::RR_PRIORS],
            ),
            "find store shelves empty of things they need",
            100.0,
        ),
        hazmat_release(ctx),
        nuclear_attack(),
        terrorism(ctx),
    ];
    out.extend(nuclear_plant_incident(ctx, notes));
    out.sort_by_key(|r| r.hazard);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tri_factor_is_bounded_and_one_near_the_typical_county() {
        // 1 + 0.5 · log10(1/5) = 0.6505: a county with no TRI facility.
        assert!((tri_factor(0) - 0.650_515).abs() < 1e-6);
        assert!((tri_factor(4) - 1.0).abs() < 1e-12);
        assert!((tri_factor(49) - 1.5).abs() < 1e-12);
        assert_eq!(tri_factor(5000), 2.0);
        let mut prev = 0.0;
        for n in 0..2000u16 {
            let k = tri_factor(n);
            assert!(k >= prev);
            prev = k;
        }
    }
}
