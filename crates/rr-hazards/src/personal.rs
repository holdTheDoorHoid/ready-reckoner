//! Personal hazards: national base rates (research §6.1, data-sources §8) scaled by the household
//! (DESIGN §4.3, research §2.7). The footprint is 1: these rates are already per household,
//! person, earner or vehicle.
//!
//! Base-rate ids read from the base-rate pack (each optional; the built-in value in `params`,
//! with its citation, is used when absent):
//!
//! | id | unit | hazard |
//! | --- | --- | --- |
//! | `house_fire_per_household_year` | per household-year | house fire |
//! | `unemployment_spell_per_worker_year` | per worker-year | job loss |
//! | `layoff_per_worker_month` | per worker-month (JOLTS) | job loss (upper end of the range) |
//! | `ed_visits_per_person_year` (or `ed_visits_per_100_persons_year`) | per person-year | medical emergency |
//! | `accidental_death_per_person_year` | per person-year | death or disability of an earner (cited) |

use rr_types::{BaseRate, CommuteMode, HazardId, HousingKind, WaterSource, math};

use crate::cite;
use crate::ctx::{Ctx, Notes};
use crate::estimate::Estimate;
use crate::params::*;
use crate::rate::HazardRate;

/// A base rate from the pack, with the built-in range (relative to its value) when the pack
/// gives none; otherwise the built-in value.
fn base_or(ctx: &Ctx<'_>, ids: &[&str], per: f64, builtin: Triple, sources: &[&str]) -> Estimate {
    match ctx.base_rate(ids) {
        Some(b) => from_base(b, per, builtin),
        None => data(builtin, sources),
    }
}

/// A pack base rate divided by `per` (for example 100 for "per 100 people").
fn from_base(b: &BaseRate, per: f64, builtin: Triple) -> Estimate {
    let v = b.value / per;
    let lo = b
        .low
        .map_or(v * builtin.1 / builtin.0, |x| (x / per).min(v));
    let hi = b
        .high
        .map_or(v * builtin.2 / builtin.0, |x| (x / per).max(v));
    Estimate::data(v, lo, hi, &[b.source.as_str()])
}

fn job_loss(ctx: &Ctx<'_>, notes: &mut Notes) -> Option<HazardRate> {
    let earners = ctx.earners();
    if earners == 0 {
        notes.add("Job loss is left out: no one in the household is marked as earning.");
        return None;
    }
    let mut spell = base_or(
        ctx,
        &["unemployment_spell_per_worker_year"],
        1.0,
        JOB_LOSS_SPELL,
        &[cite::BLS_WORK_EXPERIENCE],
    );
    match ctx.base_rate(&["layoff_per_worker_month"]) {
        // The JOLTS monthly layoff rate as a yearly rate is the upper end of the range.
        Some(b) if b.value > 0.0 && b.value < 1.0 => {
            spell.high = spell.high.max(-12.0 * math::ln_1p(-b.value));
            spell = spell.cite(&[b.source.as_str()]);
        }
        _ => spell = spell.cite(&[cite::BLS_JOLTS]),
    }
    let stability = income_stability(ctx.input.finances.income.stability);
    let today = spell.times(&stability).scaled(earners as f64);
    Some(HazardRate::new(
        HazardId::JobLoss,
        today,
        "have someone lose a job",
        12_000.0,
    ))
}

fn house_fire(ctx: &Ctx<'_>) -> HazardRate {
    let base = base_or(
        ctx,
        &["house_fire_per_household_year"],
        1.0,
        HOUSE_FIRE,
        &[cite::USFA_FIRES, cite::CENSUS_HH1],
    );
    let (m, verb) = match ctx.input.housing.kind {
        HousingKind::Rowhouse | HousingKind::ApartmentLowRise => (
            prior(FIRE_ATTACHED, &[cite::RR_PRIORS]),
            "have a house fire, in their home or next door",
        ),
        HousingKind::ApartmentHighRise => (
            prior(FIRE_HIGH_RISE, &[cite::RR_HAZARD_PRIORS]),
            "have a fire in their home or building",
        ),
        _ => (Estimate::exact(1.0), "have a house fire"),
    };
    HazardRate::new(HazardId::HouseFire, base.times(&m), verb, 32_700.0)
        .with_county_average(base.value)
}

fn medical_emergency(ctx: &Ctx<'_>, notes: &mut Notes) -> HazardRate {
    let per_person = match ctx.base_rate(&["ed_visits_per_person_year"]) {
        Some(b) => from_base(b, 1.0, ED_VISITS),
        None => match ctx.base_rate(&["ed_visits_per_100_persons_year"]) {
            Some(b) => from_base(b, 100.0, ED_VISITS),
            None => data(ED_VISITS, &[cite::NHAMCS_ED]),
        },
    };
    if ctx.setting() == rr_types::Setting::Rural {
        notes.add(
            "Ambulances take longer to reach rural homes, so first-aid skills and supplies count \
             for more here.",
        );
    }
    HazardRate::new(
        HazardId::MedicalEmergency,
        per_person.scaled(ctx.people() as f64),
        "have someone need emergency care",
        2_000.0,
    )
}

fn vehicle_stranding(ctx: &Ctx<'_>, notes: &mut Notes) -> Option<HazardRate> {
    let vehicles = ctx.vehicles();
    let today = if vehicles > 0 {
        prior(
            VEHICLE_STRANDING,
            &[cite::NHTSA_CRASHES, cite::RR_HAZARD_PRIORS],
        )
        .scaled(vehicles as f64)
    } else {
        let commuters = ctx
            .input
            .people
            .iter()
            .filter(|p| {
                p.commute
                    .as_ref()
                    .is_some_and(|c| c.mode != CommuteMode::Car)
            })
            .count();
        if commuters == 0 {
            notes.add(
                "Being stranded on the road is left out: the household has no vehicle and no \
                 one commutes.",
            );
            return None;
        }
        prior(TRANSIT_STRANDING, &[cite::RR_HAZARD_PRIORS]).scaled(commuters as f64)
    };
    Some(HazardRate::new(
        HazardId::VehicleStranding,
        today,
        "be stranded away from home by a crash, breakdown or shutdown",
        300.0,
    ))
}

fn local_utility_outage(ctx: &Ctx<'_>) -> HazardRate {
    let (today, verb) = if ctx.input.housing.water == WaterSource::Municipal {
        let boil = match ctx.event("boil_water_notice") {
            Some(e) => {
                let r = f64::from(e.rate_per_year);
                Estimate::data(r, r / 1.5, r * 1.5, &[cite::TEXAS_BWN])
            }
            None => prior(BOIL_NOTICE, &[cite::EPA_BWA, cite::RR_PRIORS]),
        };
        (
            boil.plus(&prior(MAIN_BREAK, &[cite::RR_PRIORS])),
            "have a water main break or boil-water notice",
        )
    } else {
        (
            prior(WELL_LOCAL_OUTAGE, &[cite::RR_HAZARD_PRIORS]),
            "have a gas leak or other local utility problem",
        )
    };
    HazardRate::new(HazardId::LocalUtilityOutage, today, verb, 100.0)
}

fn burglary() -> HazardRate {
    HazardRate::new(
        HazardId::Burglary,
        prior(BURGLARY, &[cite::RR_HAZARD_PRIORS]),
        "have a break-in",
        2_500.0,
    )
}

fn earner_death_or_disability(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let earners = ctx.earners();
    if earners == 0 {
        return None;
    }
    let mut per_earner = prior(
        EARNER_LOSS,
        &[
            cite::SSA_DISABILITY,
            cite::NCHS_ACCIDENTS,
            cite::RR_HAZARD_PRIORS,
        ],
    );
    if let Some(b) = ctx.base_rate(&["accidental_death_per_person_year"]) {
        per_earner = per_earner.cite(&[b.source.as_str()]);
    }
    Some(HazardRate::new(
        HazardId::EarnerDeathOrDisability,
        per_earner.scaled(earners as f64),
        "lose an earner's income to death or disability",
        500_000.0,
    ))
}

fn extended_household_illness(ctx: &Ctx<'_>) -> HazardRate {
    HazardRate::new(
        HazardId::ExtendedHouseholdIllness,
        prior(LONG_ILLNESS, &[cite::RR_HAZARD_PRIORS]).scaled(ctx.people() as f64),
        "have someone sick at home for weeks",
        5_000.0,
    )
}

/// Every personal hazard that applies to this household.
pub(crate) fn assess(ctx: &Ctx<'_>, notes: &mut Notes) -> Vec<HazardRate> {
    let mut out = Vec::new();
    out.extend(job_loss(ctx, notes));
    out.push(house_fire(ctx));
    out.push(medical_emergency(ctx, notes));
    out.extend(vehicle_stranding(ctx, notes));
    out.push(local_utility_outage(ctx));
    out.push(burglary());
    out.extend(earner_death_or_disability(ctx));
    out.push(extended_household_illness(ctx));
    out.sort_by_key(|r| r.hazard);
    out
}
