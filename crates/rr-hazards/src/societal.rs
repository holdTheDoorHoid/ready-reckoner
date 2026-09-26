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

use rr_types::{Benefit, HazardId, LocationFactor, math};

use crate::cite;
use crate::ctx::{Ctx, Notes};
use crate::estimate::Estimate;
use crate::params::*;
use crate::rare::metro_weight;
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
    notes.add(if epz {
        "A nuclear power plant is within 10 miles (16 km), the zone where people may be told to \
         shelter or leave; potassium iodide matters only inside this zone and is handed out by \
         the authorities."
            .to_owned()
    } else {
        "A nuclear power plant is within 50 miles (80 km), the zone where food and water may be \
         checked; potassium iodide is only for the 10-mile zone."
            .to_owned()
    });
    Some(HazardRate::new(
        HazardId::NuclearPlantIncident,
        prior(t, &[cite::FEMA_NUCLEAR_SITES, cite::RR_HAZARD_PRIORS]),
        "be told to stay inside or leave after a nuclear power plant accident",
        20_000.0,
    ))
}

/// Dam or levee failure (REVIEW H4): high-hazard dams whose listed downstream town lies in the
/// household's ZIP code (else the county's high-hazard dams, reaching a small share of its
/// households), weighted up for dams in poor condition, plus the residual chance for the share of
/// the county's people behind levees. Every factor is expert judgement on top of the inventories,
/// so the card shows the range only. `None` when the pack knows neither dams nor levees.
fn dam_failure(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let e = ctx.exposure();
    let dams = e.dams();
    let levees = e.levees();
    let per_dam = prior(DAM_EVENT_PER_DAM, &[cite::ASDSO, cite::RR_HAZARD_PRIORS]);
    let poor = match (dams.county_poor, dams.county_total) {
        (Some(p), Some(t)) if t > 0 => {
            1.0 + (DAM_POOR_CONDITION - 1.0) * (f64::from(p.min(t)) / f64::from(t))
        }
        _ => 1.0,
    };
    let (dam_part, dam_label, footprint) = match (dams.zip_downstream, dams.county_total) {
        (Some(n), _) => (
            per_dam
                .scaled(f64::from(n) * poor)
                .times(&prior(DAM_DOWNSTREAM_FOOTPRINT, &[cite::RR_HAZARD_PRIORS]))
                .cite(&[cite::USACE_NID]),
            if n > 0 {
                format!(
                    "Your ZIP code is within 10 km of {n} high-hazard dam{} that list{} a town in \
                     it as the place a failure would flood. High hazard describes what a failure \
                     would do, not how likely it is.",
                    if n == 1 { "" } else { "s" },
                    if n == 1 { "s" } else { "" }
                )
            } else {
                "No high-hazard dam within 10 km lists your ZIP code's towns as the place a failure \
                 would flood."
                    .to_owned()
            },
            Some(DAM_DOWNSTREAM_FOOTPRINT),
        ),
        (None, Some(n)) => (
            per_dam
                .scaled(f64::from(n) * poor)
                .times(&prior(DAM_COUNTY_FOOTPRINT, &[cite::RR_HAZARD_PRIORS]))
                .cite(&[cite::USACE_NID]),
            format!(
                "{} has {n} high-hazard dam{}; without a ZIP code we cannot tell whether you live \
                 downstream of one, so this assumes each could reach about 1 in 100 homes in the \
                 county.",
                ctx.county_label(),
                if n == 1 { "" } else { "s" }
            ),
            Some(DAM_COUNTY_FOOTPRINT),
        ),
        (None, None) => (
            Estimate::data(0.0, 0.0, 0.0, &[cite::USACE_NID]),
            String::new(),
            None,
        ),
    };
    let levee_part = levees.map(|l| {
        let k = 1.0 + (LEVEE_HIGH_RISK - 1.0) * l.high_share;
        Estimate::data(l.pop_share, l.pop_share, l.pop_share, &[cite::USACE_NLD])
            .times(&prior(LEVEE_RESIDUAL, &[cite::RR_HAZARD_PRIORS]))
            .scaled(k)
    });
    if dams.zip_downstream.is_none() && dams.county_total.is_none() && levees.is_none() {
        return None;
    }
    let today = match &levee_part {
        Some(l) => dam_part.plus(l),
        None => dam_part.clone(),
    };
    let mut r = HazardRate::new(
        HazardId::DamFailure,
        today,
        "be told to leave because a dam or levee fails or threatens to",
        40_000.0,
    );
    r.range_only = true;
    let mut label = dam_label;
    if let Some(l) = levees.filter(|l| l.pop_share > 0.0) {
        if !label.is_empty() {
            label.push(' ');
        }
        label.push_str(&format!(
            "About {} in 100 people in {} live behind a levee.",
            sentence::sig2((l.pop_share * 100.0).max(0.1)),
            ctx.county_label()
        ));
    }
    if let Some(fp) = footprint {
        r.location_factor = Some(LocationFactor {
            class: if dams.zip_downstream.is_some() {
                "zip_downstream".to_owned()
            } else {
                "county_dams".to_owned()
            },
            label,
            multiplier: [fp.1, fp.0, fp.2],
            sources: vec![cite::USACE_NID.into(), cite::USACE_NLD.into()],
        });
    }
    if let Some(l) = levee_part {
        r.sub_causes.push(rr_types::SubCause {
            id: "levee_failure".to_owned(),
            name: "Levee failure or overtopping".to_owned(),
            note: "Land behind a levee is mapped outside the high-risk flood zone, so the flood \
                   card does not count it; a levee that is overtopped or breaks floods it deep and \
                   fast. Counted here."
                .to_owned(),
            rate_range: Some([l.low, l.high]),
            sources: vec![cite::USACE_NLD.into(), cite::RR_HAZARD_PRIORS.into()],
        });
    }
    Some(r)
}

/// A phone or internet network outage (split from cyber outages): a national prior.
fn network_outage() -> HazardRate {
    HazardRate::new(
        HazardId::NetworkOutage,
        prior(
            NETWORK_OUTAGE,
            &[cite::FCC_ATT_2024, cite::RR_HAZARD_PRIORS],
        ),
        "lose phone and internet service for hours in a network outage",
        100.0,
    )
}

/// A daily prescription that cannot be filled (REVIEW H7): per person who takes one, more for a
/// medicine that must stay cold. `None` (with a note) when no one takes a daily prescription.
fn drug_shortage(ctx: &Ctx<'_>, notes: &mut Notes) -> Option<HazardRate> {
    let base = prior(
        DRUG_SHORTAGE,
        &[
            cite::ASHP_SHORTAGES,
            cite::OPENFDA_SHORTAGES,
            cite::RR_HAZARD_PRIORS,
        ],
    );
    let cold = prior(
        DRUG_SHORTAGE_COLD,
        &[cite::OPENFDA_SHORTAGES, cite::RR_HAZARD_PRIORS],
    );
    let mut total: Option<Estimate> = None;
    for p in &ctx.input.people {
        let m = &p.medical;
        if !(m.daily_rx || m.refrigerated_rx) {
            continue;
        }
        let e = if m.refrigerated_rx {
            base.times(&cold)
        } else {
            base.clone()
        };
        total = Some(match total {
            Some(t) => t.plus(&e),
            None => e,
        });
    }
    let Some(today) = total else {
        notes.add("Medicine shortages are left out: no one takes a daily prescription.");
        return None;
    };
    Some(HazardRate::new(
        HazardId::DrugShortage,
        today,
        "have a daily medicine they cannot fill for days or weeks because of a shortage",
        500.0,
    ))
}

/// Pay or benefits stopping (REVIEW H7): only for households that rely on one. One lapse stops
/// every benefit it reaches, so the household rate is the largest of its benefits' rates, not
/// their sum.
fn benefit_interruption(ctx: &Ctx<'_>) -> Option<HazardRate> {
    let benefits = &ctx.input.finances.benefits;
    let gap = data(FUNDING_GAP_14D, &[cite::CRS_FUNDING_GAPS]);
    let mut best: Option<(Estimate, &'static str)> = None;
    for b in benefits {
        let (e, verb) = match b {
            Benefit::FederalPay => (
                gap.clone(),
                "have federal pay stop for two weeks or more in a government shutdown",
            ),
            Benefit::SnapWic => (
                gap.times(&prior(
                    SNAP_LAPSE_GIVEN_GAP,
                    &[cite::SNAP_LAPSE_2025, cite::RR_HAZARD_PRIORS],
                )),
                "have SNAP or WIC payments stop in a government shutdown",
            ),
            Benefit::SsiSsdi | Benefit::Va => (
                prior(
                    MANDATORY_BENEFIT_DELAY,
                    &[cite::CRS_FUNDING_GAPS, cite::RR_HAZARD_PRIORS],
                ),
                "have a Social Security, SSI, SSDI or VA payment delayed",
            ),
            Benefit::Unemployment => (
                prior(UNEMPLOYMENT_DELAY, &[cite::RR_HAZARD_PRIORS]),
                "have unemployment benefits delayed",
            ),
        };
        if best.as_ref().is_none_or(|(x, _)| e.value > x.value) {
            best = Some((e, verb));
        }
    }
    let (today, verb) = best?;
    Some(HazardRate::new(
        HazardId::BenefitInterruption,
        today,
        verb,
        BENEFIT_LOSS_USD,
    ))
}

/// An attack or credible threat closes the household's area (REVIEW H2, B3): national attacks ×
/// the metro area's UASI share × the share of the metro's households under the order. Outside the
/// funded urban areas, a small residual. Stacked expert judgement: range only.
fn attack_disruption(ctx: &Ctx<'_>) -> HazardRate {
    let mw = metro_weight(ctx);
    let sources = [
        cite::CSIS_TERRORISM,
        cite::FEMA_UASI_FY2026,
        cite::RR_HAZARD_PRIORS,
    ];
    let lambda = prior(ATTACK_US, &sources);
    let share = prior(ATTACK_METRO_SHARE, &[cite::RR_HAZARD_PRIORS]);
    // Outside every funded urban area, and the fallback of 0 when the pack does not say.
    let today = match mw.w {
        Some(w) => lambda.scaled(w).times(&share),
        None => prior(ATTACK_NON_UASI, &sources),
    };
    let base = ATTACK_US.0 * ATTACK_METRO_SHARE.0;
    let multiplier = match mw.w {
        Some(w) => [w, w, w],
        None => [today.low / base, today.value / base, today.high / base],
    };
    let mut r = HazardRate::new(
        HazardId::AttackDisruption,
        today,
        "have an attack or threat close their area or its transit for half a day or more",
        ATTACK_LOSS_USD,
    );
    r.range_only = true;
    r.location_factor = Some(LocationFactor {
        class: mw.class.to_owned(),
        label: mw.label,
        multiplier,
        sources: vec![cite::FEMA_UASI_FY2026.into(), cite::RR_HAZARD_PRIORS.into()],
    });
    r
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
        network_outage(),
        attack_disruption(ctx),
    ];
    out.extend(nuclear_plant_incident(ctx, notes));
    out.extend(dam_failure(ctx));
    out.extend(drug_shortage(ctx, notes));
    out.extend(benefit_interruption(ctx));
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
