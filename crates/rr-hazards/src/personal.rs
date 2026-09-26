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
//! | `unintentional_injury_death_per_person_year` (or `accidental_death_per_person_year`) | per person-year | death or disability of an earner (cited) |

use rr_types::{
    AgeBand, BaseRate, CommuteMode, DataConfidence, HazardId, HousingKind, Tenure, WaterSource,
    math,
};

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
    let mut today = per_person.scaled(ctx.people() as f64);
    if ctx.setting() == rr_types::Setting::Rural {
        notes.add(
            "Ambulances take longer to reach rural homes, so first-aid skills and supplies count \
             for more here.",
        );
        today = today.cite(&[cite::MELL_2017_EMS]);
    }
    HazardRate::new(
        HazardId::MedicalEmergency,
        today,
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
    if let Some(b) = ctx.base_rate(&[
        "unintentional_injury_death_per_person_year",
        "accidental_death_per_person_year",
    ]) {
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

/// Burst, frozen or leaking pipes and appliances (REVIEW H5): the national claim frequency ×
/// freezing climate × basement × tenure. No housing-age column exists yet, so age is not a
/// modifier (see `docs/RISK_MODEL.md`).
fn water_damage(ctx: &Ctx<'_>) -> HazardRate {
    let base = data(WATER_DAMAGE, &[cite::III_WATER]);
    let mut m = Estimate::exact(1.0);
    let icing = ctx
        .county
        .climate
        .get("icing_days_hist")
        .map(|v| f64::from(*v))
        .filter(|v| v.is_finite());
    if icing.is_some_and(|d| d >= WATER_DAMAGE_FREEZE_DAYS) {
        m = m.times(&prior(
            WATER_DAMAGE_FREEZE,
            &[cite::CMRA, cite::RR_HAZARD_PRIORS],
        ));
    }
    if ctx.input.housing.basement {
        m = m.times(&prior(WATER_DAMAGE_BASEMENT, &[cite::RR_HAZARD_PRIORS]));
    }
    if ctx.input.housing.tenure == Tenure::Rent {
        m = m.times(&prior(WATER_DAMAGE_RENTER, &[cite::RR_HAZARD_PRIORS]));
    }
    let mut r = HazardRate::new(
        HazardId::WaterDamage,
        base.times(&m),
        "have a burst pipe or leak flood part of the home",
        WATER_DAMAGE_LOSS_USD,
    )
    .with_county_average(base.value);
    // The national figure comes from insured homes and was read through the publisher's summary.
    r.fixed_confidence = Some(DataConfidence::Medium);
    r
}

/// Eviction (REVIEW H7), renters only: the county's filing rate × the share that end in an order
/// to leave when the pack has it, otherwise the national judgment rate; × income stability; ×0.5
/// with three months of savings.
fn eviction(ctx: &Ctx<'_>) -> Option<HazardRate> {
    if ctx.input.housing.tenure != Tenure::Rent {
        return None;
    }
    let base = match ctx.exposure().eviction_filing_rate() {
        Some(f) if f > 0.0 => {
            Estimate::data(f, f / 1.3, f * 1.3, &[cite::EVICTION_LAB]).times(&prior(
                EVICTION_JUDGMENT_SHARE,
                &[cite::EVICTION_LAB, cite::RR_PRIORS],
            ))
        }
        _ => prior(EVICTION_NATIONAL, &[cite::EVICTION_LAB, cite::RR_PRIORS]),
    };
    let mut m = income_stability(ctx.input.finances.income.stability);
    if ctx.input.finances.emergency_fund_months >= EVICTION_SAVINGS_MONTHS {
        m = m.times(&prior(EVICTION_SAVINGS, &[cite::RR_PRIORS]));
    }
    Some(
        HazardRate::new(
            HazardId::Eviction,
            base.times(&m),
            "be taken to court and ordered to leave their rented home",
            5_000.0,
        )
        .with_county_average(base.value),
    )
}

/// Arrests per person a year for one age band, as (value, low, high): the mean of the men's and
/// women's rates (the household form does not ask sex); the range runs from the women's lowest
/// year to the men's highest. Base rates `arrests_per_100k_{male,female}_<band>` from the pack
/// replace the built-in table.
fn arrest_band(
    ctx: &Ctx<'_>,
    band: &str,
    male: Triple,
    female: Triple,
    pack_sources: &mut Vec<String>,
) -> Triple {
    let mut read = |sex: &str, builtin: Triple| {
        let id = format!("arrests_per_100k_{sex}_{band}");
        match ctx.base_rate(&[id.as_str()]) {
            Some(b) => {
                let src = b.source.to_string();
                if !pack_sources.contains(&src) {
                    pack_sources.push(src);
                }
                let v = b.value;
                (v, b.low.unwrap_or(v).min(v), b.high.unwrap_or(v).max(v))
            }
            None => builtin,
        }
    };
    let m = read("male", male);
    let f = read("female", female);
    ((m.0 + f.0) / 2.0 / 1e5, f.1 / 1e5, m.2 / 1e5)
}

/// A household member is arrested or detained (owner decision 2026-09-26): FBI arrest counts per
/// person-year by age band, summed over the household. Arrests are events, not people: one
/// person arrested twice counts twice, and the sentence says so. Children under 13 are not
/// counted (arrests of children under 10 are almost nil, and the FBI's youngest band is 10–17).
fn arrest_or_detention(ctx: &Ctx<'_>) -> Option<HazardRate> {
    // Adults: the five FBI bands from 18 to 64, weighted by the years each covers.
    let mut adult = (0.0, 0.0, 0.0);
    let mut weight = 0.0;
    let mut teen = (0.0, 0.0, 0.0);
    let mut senior = (0.0, 0.0, 0.0);
    let mut pack_sources: Vec<String> = Vec::new();
    for &(band, first, last, male, female) in ARRESTS_PER_100K {
        let t = arrest_band(ctx, band, male, female, &mut pack_sources);
        match band {
            "10_17" => teen = t,
            "65_plus" => senior = t,
            _ => {
                let w = f64::from(last - first + 1);
                adult = (adult.0 + w * t.0, adult.1 + w * t.1, adult.2 + w * t.2);
                weight += w;
            }
        }
    }
    let adult = (adult.0 / weight, adult.1 / weight, adult.2 / weight);
    let sources: &[&str] = &[cite::FBI_ARRESTS];
    let mut total: Option<Estimate> = None;
    for p in &ctx.input.people {
        let t = match p.age_band {
            AgeBand::Teen => teen,
            AgeBand::Adult => adult,
            AgeBand::Senior => senior,
            AgeBand::Infant | AgeBand::Toddler | AgeBand::Child => continue,
        };
        let e = Estimate::data(t.0, t.1, t.2, sources);
        total = Some(match total {
            Some(sum) => sum.plus(&e),
            None => e,
        });
    }
    let pack: Vec<&str> = pack_sources.iter().map(String::as_str).collect();
    let today = total?.cite(&pack);
    let sentence = format!(
        "For households with people the ages of yours, the FBI's counts come to {} a year \
         (2023–2025). This counts arrests, not guilt or convictions, and one person arrested \
         twice counts twice.",
        crate::sentence::arrests_per_households(today.value)
    );
    let mut r = HazardRate::new(
        HazardId::ArrestOrDetention,
        today,
        "have a member arrested or detained",
        ARREST_LOSS_USD,
    );
    r.range_sentence = Some(sentence);
    r.fixed_confidence = Some(DataConfidence::Medium);
    Some(r)
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
    out.push(water_damage(ctx));
    out.extend(eviction(ctx));
    out.extend(arrest_or_detention(ctx));
    out.sort_by_key(|r| r.hazard);
    out
}
