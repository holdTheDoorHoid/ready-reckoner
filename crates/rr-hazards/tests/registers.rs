//! The fixture counties reproduce the research registers (research §8.3 Philadelphia, §9.3 Coos
//! Bay) where the two use the same definitions, and the numbers trace to their sources.

mod common;

use common::*;
use rr_types::{HazardDisplay, HazardId as H, ScenarioToggle};

// ------------------------------------------------------------------------------------------
// Philadelphia: a renting family of four in a rowhouse (research §8).
// ------------------------------------------------------------------------------------------

#[test]
fn philadelphia_job_loss_is_81_in_100_over_ten_years() {
    // Research §8.3 row 1: two typical earners at 0.083 spells a year each (BLS Work Experience
    // 2024) -> 1 − e^(−10 × 0.166) = 81 in 100.
    let a = assess("philadelphia-renters-4", "42101");
    let p = profile(&a, H::JobLoss);
    assert!(
        close(p.rate_per_year, 2.0 * 0.083, 1e-12),
        "{}",
        p.rate_per_year
    );
    assert!((per_100(p.rate_per_year, 10.0) - 81.0).abs() < 0.5);
    assert!(
        p.frequency_sentence
            .contains("about 81 will have someone lose a job")
    );
    assert!(p.sources.iter().any(|s| s == "bls_work_experience_2024"));
}

#[test]
fn philadelphia_house_fire_is_the_usfa_rate_doubled_for_a_rowhouse() {
    // 344,600 fires / 131.434 M households = 0.2622 %/yr (USFA 2023, CPS HH-1), × 2 for an
    // attached home (research §2.7): about 5 in 100 over ten years (research §6.1: "1 in 20").
    let a = assess("philadelphia-renters-4", "42101");
    let p = profile(&a, H::HouseFire);
    assert!(close(p.rate_per_year, 2.0 * 0.002_622, 1e-9));
    assert!((per_100(p.rate_per_year, 10.0) - 5.1).abs() < 0.1);
    assert!(p.frequency_sentence.contains("in their home or next door"));
}

#[test]
fn philadelphia_emergency_care_is_47_visits_per_100_people() {
    // NHAMCS 2022: 47.3 visits per 100 people; four people -> 1.89 a year (research §8.3 row 2,
    // "about 2 visits a year").
    let a = assess("philadelphia-renters-4", "42101");
    let p = profile(&a, H::MedicalEmergency);
    assert!(close(p.rate_per_year, 4.0 * 0.473, 1e-12));
    assert!(p.frequency_sentence.contains("about 1.9 times a year"));
}

#[test]
fn philadelphia_winter_storms_and_windstorms_lead_the_natural_hazards() {
    let a = assess("philadelphia-renters-4", "42101");
    let top: Vec<H> = natural(&a).iter().take(2).map(|p| p.id).collect();
    assert!(
        top.contains(&H::WinterWeather) && top.contains(&H::StrongWind),
        "top natural hazards: {top:?}"
    );
}

#[test]
fn philadelphia_job_loss_and_house_fire_outrank_the_dramatic_natural_hazards() {
    // "Prepare for Tuesday before doomsday": a lost job and a house fire are more likely than an
    // earthquake, a tornado, hail damage, a landslide or coastal flooding reaching the home.
    // (Winter storms, windstorms, ice storms, tropical storms and heat waves with a cooling
    // failure do reach a Philadelphia household more often than a house fire does.)
    let a = assess("philadelphia-renters-4", "42101");
    let job = profile(&a, H::JobLoss).rate_per_year;
    let fire = profile(&a, H::HouseFire).rate_per_year;
    for h in [
        H::Earthquake,
        H::Tornado,
        H::Hail,
        H::Landslide,
        H::CoastalFlooding,
        H::Lightning,
        H::Drought,
    ] {
        let r = profile(&a, h).rate_per_year;
        assert!(fire > r && job > r, "{h}: {r} vs fire {fire}, job {job}");
    }
    // Job loss outranks every natural hazard except the two most common kinds of weather.
    for p in natural(&a).iter().skip(2) {
        assert!(job > p.rate_per_year, "{} {}", p.id, p.rate_per_year);
    }
}

#[test]
fn philadelphia_register_top_five() {
    // The register the planner sanity-checks (see the hazards report). A deliberate change to a
    // prior may reorder it; update this list in the same commit and say why.
    let a = assess("philadelphia-renters-4", "42101");
    let top: Vec<H> = a.profiles.iter().take(5).map(|p| p.id).collect();
    assert_eq!(
        top,
        [
            H::MedicalEmergency,
            H::WinterWeather,
            H::StrongWind,
            H::SupplyChainDisruption,
            H::JobLoss
        ]
    );
}

#[test]
fn philadelphia_flooding_with_a_basement_matches_the_nri_loss_rate() {
    // NRI v1.20: 0.67 % of Philadelphia's residents are in the inland-flood hazard area
    // (EXPP 10,790 / 1,602,305); in-zone homes flood at 1 %/yr, others at 0.2 %/yr scaled ×2
    // for a county with 5.8 flood events a year (national median 0.96); a basement ×1.5.
    // About 0.6 %/yr, in line with the research's NRI-implied $270/yr expected loss on a
    // $300k home (research §1.4 of data-sources).
    let a = assess("philadelphia-renters-4", "42101");
    let s = 10_790.0 / 1_602_305.0;
    let want = (s * 0.01 + (1.0 - s) * 0.002 * 2.0) * 1.5;
    let p = profile(&a, H::RiverineFlooding);
    assert!(
        close(p.rate_per_year, want, 1e-3),
        "{} vs {want}",
        p.rate_per_year
    );
}

#[test]
fn philadelphia_has_no_named_scenarios_and_shows_rare_catastrophes_last() {
    let a = assess("philadelphia-renters-4", "42101");
    assert!(a.scenarios.is_empty(), "{:?}", a.scenarios);
    let n = a.profiles.len();
    let rare: Vec<H> = a.profiles[n - 2..].iter().map(|p| p.id).collect();
    assert_eq!(rare, [H::NuclearAttack, H::Terrorism]);
    for p in &a.profiles[n - 2..] {
        assert_eq!(p.display, HazardDisplay::RareCatastrophic);
        assert!(!p.frequency_sentence.contains("Of 100"));
    }
    let nuke = profile(&a, H::NuclearAttack);
    // Research §6.3: "about 1 in 2,000 to about 1 in 400 per year", never a point estimate.
    assert!(
        nuke.frequency_sentence
            .contains("about 1 in 2,000 to about 1 in 400 a year")
    );
    assert_eq!(nuke.rate_range, [0.0005, 0.0025]);
    // A plant within 80 km (Limerick): the incident appears, ranked, with a tiny rate.
    let plant = profile(&a, H::NuclearPlantIncident);
    assert_eq!(plant.display, HazardDisplay::Ranked);
    assert!(plant.rate_per_year < 1e-3);
}

// ------------------------------------------------------------------------------------------
// Coos Bay: a well-owning couple on the Oregon coast (research §9).
// ------------------------------------------------------------------------------------------

#[test]
fn coos_bay_has_earthquake_tsunami_and_cascadia_on_by_default() {
    let a = assess("coos-bay-well-owner-2", "41011");
    assert!(has_profile(&a, H::Earthquake) && has_profile(&a, H::Tsunami));
    let c = a
        .scenarios
        .iter()
        .find(|s| s.id == "cascadia_m9")
        .expect("cascadia_m9");
    assert!(c.default_on && c.on && !c.overridden);
    assert_eq!(c.hazard, H::Earthquake);
    assert_eq!(c.zone, Some(rr_hazards::ScenarioZone::Coast));
    // Research S31: 40 % in 50 years near Coos Bay -> 1.02 %/yr (research §3.3: 1.0 %/yr) ...
    let want = -rr_types::math::ln_1p(-0.40) / 50.0;
    assert!(
        close(c.rate.rate_per_year, want, 1e-9),
        "{}",
        c.rate.rate_per_year
    );
    assert!((c.rate.rate_per_year - 0.0102).abs() < 0.0001);
    // ... against the time-independent recurrence, 41 ruptures in 10,000 years (≈ 0.4 %/yr).
    assert_eq!(c.alternatives.len(), 1);
    assert!((c.alternatives[0].rate_per_year - 0.0041).abs() < 1e-12);
    assert!(c.applies_because.contains("Oregon asks every household"));
    assert!(c.sources.iter().any(|s| s == "orp_2013"));
}

#[test]
fn coos_bay_earthquake_rate_to_consequence_excludes_cascadia_but_the_card_shows_it() {
    // NRI v1.20: damaging shaking 0.01962 a year as a probability -> rate 0.01981. The
    // hazard model already contains Cascadia at its recurrence (0.0041), so the rate handed to
    // rr-consequence is 0.01572 and the scenario adds 0.01022 when on. The register card shows
    // both: 0.02594.
    let a = assess("coos-bay-well-owner-2", "41011");
    let nri = -rr_types::math::ln_1p(-0.01962);
    let base = rate(&a, H::Earthquake).rate_per_year;
    // The record stores f32 values (about 7 significant digits), hence the 1e-6 tolerances.
    assert!(close(base, nri - 0.0041, 1e-6), "{base}");
    let cascadia = &a.scenarios[0].rate.rate_per_year;
    let card = profile(&a, H::Earthquake).rate_per_year;
    assert!(close(card, base + cascadia, 1e-12), "{card}");
    assert!(
        profile(&a, H::Earthquake)
            .sources
            .iter()
            .any(|s| s == "goldfinger_2012_cascadia")
    );
}

#[test]
fn coos_bay_local_tsunami_uses_the_share_of_residents_in_the_zone() {
    // NRI v1.20 tsunami exposure: 11,060 of 64,845 residents (17 %). A local tsunami comes with
    // the Cascadia earthquake: 0.01022 × 0.1706.
    let a = assess("coos-bay-well-owner-2", "41011");
    let t = a
        .scenarios
        .iter()
        .find(|s| s.id == "local_tsunami")
        .expect("local_tsunami");
    assert!(t.on && t.default_on);
    let share = 11_060.0 / 64_845.0;
    assert!(close(
        t.rate.rate_per_year,
        a.scenarios[0].rate.rate_per_year * share,
        1e-9
    ));
    assert!(t.applies_because.contains("about 17 in 100 residents"));
    assert!(t.applies_because.contains("15 to 20 minutes"));
    assert!(t.sources.iter().any(|s| s == "dogami_tsunami_faq"));
}

#[test]
fn coos_bay_cascadia_can_be_turned_off() {
    let mut input = household("coos-bay-well-owner-2");
    input.dials.scenario_overrides = vec![ScenarioToggle {
        id: "cascadia_m9".into(),
        on: false,
    }];
    let fixture = county("41011");
    let a = run(&input, &fixture);
    let c = a.scenarios.iter().find(|s| s.id == "cascadia_m9").unwrap();
    assert!(!c.on && c.default_on && c.overridden);
    // The rate to rr-consequence already excludes Cascadia, so it does not change.
    let on = assess("coos-bay-well-owner-2", "41011");
    assert_eq!(rate(&a, H::Earthquake), rate(&on, H::Earthquake));
    // The tsunami scenario keeps its own default.
    assert!(
        a.scenarios
            .iter()
            .find(|s| s.id == "local_tsunami")
            .unwrap()
            .on
    );
}

#[test]
fn coos_bay_windstorms_are_lifted_to_the_recorded_outage_rate() {
    // EAGLE-I 2018–2025 (research §9.2): a Coos home sits inside a county outage event about
    // 1.09 times a year. With 70 % of those weather-caused, the storm hazards must explain
    // 0.763 power cuts a year; NRI's windstorm count for Coos (0.027 a year) cannot, so the gap
    // is counted as windstorms.
    let a = assess("coos-bay-well-owner-2", "41011");
    let share = |h: H| match h {
        H::StrongWind => 0.9,
        H::WinterWeather => 0.3,
        H::IceStorm | H::Tornado | H::Lightning => 0.8,
        H::Hurricane => 0.9,
        H::Hail => 0.1,
        _ => 0.0,
    };
    let explained: f64 = a
        .rates
        .iter()
        .map(|r| r.rate_per_year * share(r.hazard))
        .sum();
    // Tornado (9e-6 a year here) is left out of the register as negligible, hence 1e-4.
    assert!(close(explained, 0.7 * 1.09, 1e-4), "{explained}");
    let wind = profile(&a, H::StrongWind);
    assert!(wind.rate_per_year > 0.7, "{}", wind.rate_per_year);
    assert!(wind.sources.iter().any(|s| s == "ornl_eagle_i"));
}

#[test]
fn coos_bay_heat_waves_are_capped_by_the_hot_days_in_its_climate() {
    // NRI counts 2.36 heat-wave days a year county-wide, but CMRA's historical baseline has only
    // 0.2289 days over 90 °F; with no air conditioning every such episode (3 days) counts.
    let a = assess("coos-bay-well-owner-2", "41011");
    let p = profile(&a, H::HeatWave);
    assert!(
        close(p.rate_per_year, 0.2289 / 3.0, 1e-6),
        "{}",
        p.rate_per_year
    );
    assert!(a.notes.iter().any(|n| n.contains("days a year over 90 °F")));
}

#[test]
fn coos_bay_well_and_income_modifiers() {
    let a = assess("coos-bay-well-owner-2", "41011");
    // A private well: drought prior 1 %/yr at the national median drought frequency (13.43),
    // scaled by Coos County's 24.56.
    assert!(close(
        profile(&a, H::Drought).rate_per_year,
        0.01 * 24.56 / 13.43,
        1e-6
    ));
    // Two earners with variable income: 2 × 0.083 × 1.5.
    assert!(close(
        profile(&a, H::JobLoss).rate_per_year,
        2.0 * 0.083 * 1.5,
        1e-12
    ));
    // Rural: the ambulance note.
    assert!(a.notes.iter().any(|n| n.contains("Ambulances take longer")));
}

#[test]
fn coos_bay_register_top_five() {
    let a = assess("coos-bay-well-owner-2", "41011");
    let top: Vec<H> = a.profiles.iter().take(5).map(|p| p.id).collect();
    assert_eq!(
        top,
        [
            H::MedicalEmergency,
            H::StrongWind,
            H::VehicleStranding,
            H::WinterWeather,
            H::JobLoss
        ]
    );
}

// ------------------------------------------------------------------------------------------
// The other fixtures.
// ------------------------------------------------------------------------------------------

#[test]
fn gulf_and_atlantic_counties_get_the_major_hurricane_scenario() {
    for (name, fips) in [
        ("miami-condo-retiree-1", "12086"),
        ("sugar-land-ev-household-3", "48157"),
    ] {
        let a = assess(name, fips);
        let s = a
            .scenarios
            .iter()
            .find(|s| s.id == "major_hurricane_direct_hit")
            .unwrap_or_else(|| panic!("{name}"));
        assert!(s.on && s.default_on, "{name}");
        // The card shows the whole hurricane rate; rr-consequence gets the Category 1–2 part
        // and the scenario the major part.
        let card = profile(&a, H::Hurricane).rate_per_year;
        let parent = rate(&a, H::Hurricane).rate_per_year;
        assert!(close(card, parent + s.rate.rate_per_year, 1e-12), "{name}");
    }
    // Philadelphia's hurricane frequency (0.092 a year) is under the 0.1 threshold; Phoenix is
    // not on the Gulf or Atlantic.
    for (name, fips) in [
        ("philadelphia-renters-4", "42101"),
        ("phoenix-apartment-cpap-1", "04013"),
    ] {
        let a = assess(name, fips);
        assert!(
            a.scenarios
                .iter()
                .all(|s| s.id != "major_hurricane_direct_hit"),
            "{name}"
        );
    }
}

#[test]
fn miami_2050_raises_major_hurricanes_not_hurricane_frequency() {
    // Miami's fixture uses the 2050 dial. NRI hurricane frequency 0.305 a year stays; the major
    // share (1/3) rises ×1.2, so the major part rises ×1.2 and the card total only slightly.
    let a = assess("miami-condo-retiree-1", "12086");
    let card = profile(&a, H::Hurricane);
    assert!(card.climate_multiplier > 1.0 && card.climate_multiplier < 1.1);
    let major = &a
        .scenarios
        .iter()
        .find(|s| s.id == "major_hurricane_direct_hit")
        .unwrap()
        .rate;
    let today = 0.305 * (1.0 / 3.0) * 0.8;
    assert!(
        close(major.rate_per_year, today * 1.2, 1e-3),
        "{}",
        major.rate_per_year
    );
}

#[test]
fn households_without_earners_have_no_income_hazards() {
    for (name, fips) in [
        ("chicago-student-zero-budget-1", "17031"),
        ("miami-condo-retiree-1", "12086"),
    ] {
        let a = assess(name, fips);
        assert!(!has_profile(&a, H::JobLoss), "{name}");
        assert!(!has_profile(&a, H::EarnerDeathOrDisability), "{name}");
        assert!(a.notes.iter().any(|n| n.contains("Job loss is left out")));
    }
    // The student bikes to class: stranding comes from the commute, not a car.
    let a = assess("chicago-student-zero-budget-1", "17031");
    assert!(close(
        profile(&a, H::VehicleStranding).rate_per_year,
        0.03,
        1e-12
    ));
}

#[test]
fn hays_works_from_a_county_code_and_scales_seasonal_income() {
    let a = assess("hays-kansas-farm-5", "20051");
    assert!(close(
        profile(&a, H::JobLoss).rate_per_year,
        2.0 * 0.083 * 1.75,
        1e-12
    ));
    // Five people: 5 × 0.473 emergency-care visits.
    assert!(close(
        profile(&a, H::MedicalEmergency).rate_per_year,
        5.0 * 0.473,
        1e-12
    ));
    // Hail alley: hail damage derived from NRI's loss ratio (1.666e-5 ÷ 0.02 per event, 9.394
    // events a year) is far above Philadelphia's.
    let hays = profile(&a, H::Hail).rate_per_year;
    assert!(close(hays, 9.394 * 1.666e-5 / 0.02, 1e-3), "{hays}");
    let phl = profile(&assess("philadelphia-renters-4", "42101"), H::Hail).rate_per_year;
    assert!(hays > 10.0 * phl);
}
