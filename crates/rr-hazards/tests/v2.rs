//! Contract v2 (v0.2.0): the ten new ranked hazards, arrests, the nine rare families with their
//! location terms, the new scenarios and the six contract v2 fixture households. Each number is
//! checked against the source it cites (REVIEW §2.3 for the nuclear classes).

mod common;

use common::*;
use rr_types::{Benefit, HazardDisplay, HazardId as H, PlanInput, math};
use serde_json::json;

/// Rounds to one significant figure, as REVIEW §2.3 prints its ranges.
fn sig1(x: f64) -> f64 {
    let e = (math::ln(x) / core::f64::consts::LN_10).floor();
    let scale = math::pow(10.0, e);
    (x / scale).round() * scale
}

fn same1(a: f64, b: f64) -> bool {
    (sig1(a) - b).abs() <= 1e-9 * b
}

fn minot() -> Fixture {
    county_like(
        "20051",
        "38101",
        "Ward",
        "ND",
        "North Dakota",
        json!({
            "strategic_class": "A",
            "strategic_site_ids": ["minot_afb", "minot_field"],
            "strategic_places": [
                place("minot_afb", "Minot Air Force Base", "icbm_host_base",
                      "the Air Force base that runs a nuclear missile field", "ND"),
                place("minot_field", "Minot AFB missile field (91st Missile Wing)", "icbm_field",
                      "a field of underground nuclear missile silos", "ND"),
            ],
            "uasi_share": 0.0,
            "geomag_lat": 56.1, "geomag_factor": 0.63,
            "smoke_days_35": 4.16, "smoke_days_55": 1.47, "smoke_basis": "monitor",
            "karst_share": 0.0, "landslide_susceptible_share": 0.21,
            "leveed_pop_share": 0.145, "levee_risk_high_share": 0.878,
            "sdwis_violation_pop_share": 0.0, "cws_pop_share": 0.939
        }),
    )
}

fn jefferson_tx() -> Fixture {
    county_like(
        "48157",
        "48245",
        "Jefferson",
        "TX",
        "Texas",
        json!({
            "strategic_class": "C2",
            "strategic_site_ids": ["port_7", "refinery_48245"],
            "strategic_places": [
                place("port_7", "Beaumont, TX", "port", "", ""),
                place("refinery_48245",
                      "Motiva Enterprises LLC (Port Arthur), 656400 b/cd; ExxonMobil Refining & \
                       Supply Co (Beaumont), 612000 b/cd; Premcor Refining Group Inc (Port \
                       Arthur), 385000 b/cd; TotalEnergies Petrochem & Refg USA (Port Arthur), \
                       238000 b/cd",
                      "refinery", "", ""),
            ],
            "uasi_share": 0.0,
            "geomag_lat": 38.4, "geomag_factor": 0.1
        }),
    )
}

fn saline_mo() -> Fixture {
    county_like(
        "20051",
        "29195",
        "Saline",
        "MO",
        "Missouri",
        json!({
            "strategic_class": "D",
            "strategic_site_ids": ["kcnsc"],
            "strategic_places": [
                place("kcnsc", "Kansas City National Security Campus", "doe_weapons_complex",
                      "a plant or laboratory that makes or looks after nuclear weapons", "MO"),
            ],
            "strategic_km": 107.8, "strategic_bearing": 264.0,
            "uasi_share": 0.0,
            "geomag_lat": 47.7, "geomag_factor": 0.24
        }),
    )
}

// ------------------------------------------------------------------------------------------
// The nuclear family: REVIEW §2.3's classes within rounding, and hazard-expansion B1.5's
// ten-year ranges word for word.
// ------------------------------------------------------------------------------------------

#[test]
fn every_strategic_class_gives_the_review_range() {
    // (class, household, county, REVIEW §2.3 middle, low, high, B1.5 ten-year words)
    let cases: Vec<(&str, &str, Fixture, f64, f64, f64, &str)> = vec![
        (
            "A",
            "minot-missile-field-3",
            minot(),
            3.6e-4,
            6e-5,
            4e-3,
            "Between 1 in 1,700 and 1 in 26",
        ),
        (
            "B",
            "hays-kansas-farm-5",
            county("20051"),
            2.0e-4,
            2e-5,
            3e-3,
            "Between 1 in 5,000 and 1 in 32",
        ),
        (
            "C1",
            "philadelphia-renters-4",
            county("42101"),
            2.4e-4,
            3e-5,
            4e-3,
            "Between 1 in 3,300 and 1 in 28",
        ),
        (
            "C2",
            "sugar-land-ev-household-3",
            jefferson_tx(),
            1.2e-4,
            1e-5,
            2e-3,
            "Between 1 in 10,000 and 1 in 42",
        ),
        (
            "D",
            "hays-kansas-farm-5",
            saline_mo(),
            6e-5,
            5e-6,
            2e-3,
            "Between 1 in 20,000 and 1 in 63",
        ),
        (
            "E",
            "coos-bay-well-owner-2",
            county("41011"),
            1.2e-5,
            1e-6,
            4e-4,
            "Between 1 in 100,000 and 1 in 250",
        ),
    ];
    for (class, name, fixture, mid, lo, hi, words) in cases {
        let a = run(&household(name), &fixture);
        let p = profile(&a, H::NuclearAttack);
        assert_eq!(p.display, HazardDisplay::RareCatastrophic, "{class}");
        assert!(p.range_only, "{class}");
        assert_eq!(p.location_factor.as_ref().unwrap().class, class);
        // The middle within 1 % (the limited-strike and device-in-a-city terms add less than
        // that), the range within rounding to one figure.
        assert!(
            close(p.rate_per_year, mid, 0.01),
            "{class}: {}",
            p.rate_per_year
        );
        let [l, h] = p.rate_range;
        assert!(same1(l, lo), "{class}: low {l} vs {lo}");
        assert!(same1(h, hi), "{class}: high {h} vs {hi}");
        assert!(
            p.frequency_sentence.starts_with(words),
            "{class}: {}",
            p.frequency_sentence
        );
    }
}

#[test]
fn the_why_here_sentences_name_the_place() {
    let why = |name: &str, f: &Fixture| {
        let a = run(&household(name), f);
        profile(&a, H::NuclearAttack)
            .location_factor
            .clone()
            .unwrap()
            .label
    };
    assert!(why("minot-missile-field-3", &minot()).starts_with(
        "Your county has, or is close to, Minot Air Force Base: the Air Force base that runs a \
         nuclear missile field. In a large nuclear war, places like this are treated as likely \
         targets. That does not mean an attack is likely."
    ));
    assert!(why("hays-kansas-farm-5", &county("20051")).starts_with(
        "You live downwind of the nuclear missile fields in Wyoming, Nebraska and Colorado, about \
         240 miles to your northwest."
    ));
    assert!(
        why("sugar-land-ev-household-3", &county("48157"))
            .starts_with("You live in the Houston metro area.")
    );
    assert!(
        why("sugar-land-ev-household-3", &jefferson_tx()).starts_with(
            "You live in Jefferson County, Texas: a major port and several large oil refineries."
        )
    );
    assert!(
        why("hays-kansas-farm-5", &saline_mo()).starts_with(
            "You live about 70 miles downwind of Kansas City National Security Campus."
        )
    );
    assert!(
        why("coos-bay-well-owner-2", &county("41011"))
            .starts_with("None of the places treated as likely targets")
    );
}

#[test]
fn what_it_changes_and_severity_follow_the_zone() {
    let a = run(&household("minot-missile-field-3"), &minot());
    let p = profile(&a, H::NuclearAttack);
    assert_eq!(p.severity, 1.0);
    assert!(
        p.if_it_reaches_you
            .as_deref()
            .unwrap()
            .starts_with("Life-threatening")
    );
    assert!(
        p.what_it_changes
            .as_deref()
            .unwrap()
            .contains("pick your shelter spot")
    );
    let b = assess("hays-kansas-farm-5", "20051");
    let p = profile(&b, H::NuclearAttack);
    assert_eq!(p.severity, 0.5);
    assert!(
        p.if_it_reaches_you
            .as_deref()
            .unwrap()
            .starts_with("Serious disruption")
    );
    let e = assess("coos-bay-well-owner-2", "41011");
    let p = profile(&e, H::NuclearAttack);
    assert_eq!(p.severity, 0.3);
    assert_eq!(
        p.if_it_reaches_you.as_deref(),
        Some("Shortages, power cuts and lost income, not blast or heavy fallout.")
    );
    assert_eq!(
        p.what_it_changes.as_deref(),
        Some("Nothing beyond your basics.")
    );
}

#[test]
fn an_unknown_class_uses_the_national_average() {
    let mut f = county("42101");
    f.county.exposure = Default::default();
    let a = run(&household("philadelphia-renters-4"), &f);
    let p = profile(&a, H::NuclearAttack);
    let lf = p.location_factor.as_ref().unwrap();
    assert_eq!(lf.class, "unknown");
    assert_eq!(lf.multiplier, [0.01, 0.314, 0.99]);
    // 4e-4 × 0.314, from 1e-4 × 0.01 to 4e-3 × 0.99.
    assert!(close(p.rate_per_year, 4.0e-4 * 0.314, 1e-9));
    assert!(same1(p.rate_range[0], 1e-6) && same1(p.rate_range[1], 4e-3));
    assert!(a.notes.iter().any(|n| n.contains("strategic-site class")));
    assert!(
        a.notes
            .iter()
            .any(|n| n.contains("newer data are not loaded"))
    );
}

// ------------------------------------------------------------------------------------------
// The other rare families.
// ------------------------------------------------------------------------------------------

#[test]
fn solar_storms_scale_with_geomagnetic_latitude() {
    // A Carrington-class storm 3 in 1,000 a year × P(multi-day outage) 0.09 × α / 0.2285.
    // Minot, α 0.63: 7.4 in 10,000 a year (REVIEW §2.3: "about 7 in 10,000 in Minot").
    let at_minot = profile(
        &run(&household("minot-missile-field-3"), &minot()),
        H::GeomagneticStorm,
    )
    .clone();
    // The pack stores factors as 32-bit floats (about 7 digits), hence the 1e-6 tolerances.
    let want = 3.0e-3 * 0.09 * 0.63 / 0.2285;
    assert!(
        close(at_minot.rate_per_year, want, 1e-6),
        "{}",
        at_minot.rate_per_year
    );
    assert!(close(at_minot.rate_per_year, 7.0e-4, 0.07));
    // Low: 5 in 10,000 storms × 0.06; high: 1.3 in 100 × 0.12.
    assert!(close(
        at_minot.rate_range[0],
        5.0e-4 * 0.06 * 0.63 / 0.2285,
        1e-6
    ));
    assert!(close(
        at_minot.rate_range[1],
        1.3e-2 * 0.12 * 0.63 / 0.2285,
        1e-6
    ));
    let phl = profile(
        &assess("philadelphia-renters-4", "42101"),
        H::GeomagneticStorm,
    )
    .clone();
    let miami = profile(
        &assess("miami-condo-retiree-1", "12086"),
        H::GeomagneticStorm,
    )
    .clone();
    assert!(at_minot.rate_per_year > phl.rate_per_year && phl.rate_per_year > miami.rate_per_year);
    // Miami sits at NERC's floor, α 0.1 (REVIEW §2.3's 6 in 100,000 used 0.06, below the floor).
    assert!(close(
        miami.rate_per_year,
        3.0e-3 * 0.09 * 0.1 / 0.2285,
        1e-6
    ));
    assert_eq!(at_minot.location_factor.as_ref().unwrap().class, "high");
    assert_eq!(miami.location_factor.as_ref().unwrap().class, "low");
    // The chance one storm cuts the power for days never passes one half.
    let mut anchorage = county("41011");
    anchorage.county.exposure.geomag_factor = Some(1.0);
    let p = profile(
        &run(&household("coos-bay-well-owner-2"), &anchorage),
        H::GeomagneticStorm,
    )
    .clone();
    assert!(close(p.rate_range[1], 1.3e-2 * 0.5, 1e-9));
}

#[test]
fn war_cbrn_and_the_worldwide_families() {
    // War: 0.5 % × 0.5 near military sites and infrastructure (Philadelphia, C1), × 0.3 far
    // (Hays, B): REVIEW §2.3 "2.5 in 1,000 (4 in 10,000 to 8 in 1,000)".
    let phl = assess("philadelphia-renters-4", "42101");
    let war = profile(&phl, H::WarInfrastructure);
    assert!(close(war.rate_per_year, 2.5e-3, 1e-9));
    assert!(close(war.rate_range[0], 4.0e-4, 1e-9) && close(war.rate_range[1], 8.0e-3, 1e-9));
    let hays = assess("hays-kansas-farm-5", "20051");
    assert!(close(
        profile(&hays, H::WarInfrastructure).rate_per_year,
        2.5e-3 * 0.3,
        1e-9
    ));
    // CBRN: 3 in 100 a year × the Philadelphia area's 2.842 % × 5 %.
    let cbrn = profile(&phl, H::CbrnAttack);
    assert!(close(cbrn.rate_per_year, 0.03 * 0.02842 * 0.05, 1e-9));
    assert_eq!(cbrn.sub_causes.len(), 3);
    // Worldwide families carry the published ranges unchanged.
    let sp = profile(&phl, H::SeverePandemic);
    assert_eq!(sp.rate_range, [5.0e-4, 5.0e-3]);
    let vei = profile(&phl, H::Vei7Eruption);
    assert_eq!(vei.rate_range, [8.0e-4, 4.0e-3]);
    assert!(vei.sub_causes.iter().any(|s| s.id == "yellowstone"));
    let fin = profile(&phl, H::FinancialCrisis);
    assert_eq!(fin.rate_range, [5.0e-4, 1.0e-2]);
    assert!(fin.sub_causes.iter().any(|s| s.id == "bank_failure"));
    // Mass violence: 3 in 10 million per person a year (FBI 2024), four people.
    let mv = profile(&phl, H::MassViolence);
    assert!(close(mv.rate_per_year, 4.0 * 3.0e-7, 1e-9));
    assert!(
        mv.what_it_changes
            .as_deref()
            .unwrap()
            .contains("run, hide, fight")
    );
}

#[test]
fn the_months_long_blackout_takes_the_power_curve_later() {
    let mut a = assess("philadelphia-renters-4", "42101");
    let before = profile(&a, H::MultiMonthBlackout).clone();
    let ids: Vec<&str> = before.sub_causes.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["solar_storm", "emp", "war"]);
    a.add_power_curve((1.0e-4, 5.0e-5, 2.0e-4), 10);
    let after = profile(&a, H::MultiMonthBlackout).clone();
    assert!(close(
        after.rate_per_year,
        before.rate_per_year + 1.0e-4,
        1e-12
    ));
    assert!(close(
        after.rate_range[1],
        before.rate_range[1] + 2.0e-4,
        1e-12
    ));
    assert!(after.sub_causes.iter().any(|s| s.id == "own_record"));
    assert!(after.frequency_sentence.contains("two months or more"));
    // A second call changes nothing.
    a.add_power_curve((1.0e-4, 5.0e-5, 2.0e-4), 10);
    assert_eq!(profile(&a, H::MultiMonthBlackout), &after);
    // Territories have no EMP part (a burst over the lower 48 states).
    let mut pr = county("12086");
    pr.county.state_abbr = "PR".into();
    let a = run(&household("san-juan-2"), &pr);
    let emp = profile(&a, H::MultiMonthBlackout)
        .sub_causes
        .iter()
        .find(|s| s.id == "emp")
        .unwrap()
        .rate_range
        .unwrap();
    assert_eq!(emp, [0.0, 0.0]);
}

#[test]
fn also_checked_lists_what_is_too_rare_here() {
    let a = assess("philadelphia-renters-4", "42101");
    let ids: Vec<&str> = a.also_checked.iter().map(|c| c.id.as_str()).collect();
    for id in ["asteroid", "yellowstone", "sinkhole", "dust_storm"] {
        assert!(ids.contains(&id), "{id}: {ids:?}");
    }
    let asteroid = a.also_checked.iter().find(|c| c.id == "asteroid").unwrap();
    assert_eq!(asteroid.rate_range, [1.0e-9, 6.0e-9]);
    let y = a
        .also_checked
        .iter()
        .find(|c| c.id == "yellowstone")
        .unwrap();
    assert!(close(y.rate_per_year, 1.0 / 730_000.0, 1e-12));
    let note = a
        .notes
        .iter()
        .find(|n| n.starts_with("Also checked"))
        .unwrap();
    assert!(
        note.contains("a Yellowstone super-eruption (about 1 in 730,000 a year)"),
        "{note}"
    );
    assert!(note.contains("dust storms (none recorded here)"), "{note}");
    // Outside every funded urban area, an attack that closes the area is under 1 in 100,000.
    let hays = assess("hays-kansas-farm-5", "20051");
    let attack = hays
        .also_checked
        .iter()
        .find(|c| c.id == "attack_disruption")
        .unwrap();
    assert!(close(attack.rate_per_year, 3.0e-6, 1e-9));
}

// ------------------------------------------------------------------------------------------
// The new ranked hazards.
// ------------------------------------------------------------------------------------------

#[test]
fn water_damage_is_the_claim_rate_with_climate_basement_and_tenure() {
    // III/ISO: 1.5 in 100 homes a year; Philadelphia has 14 days a year below freezing (×1.3),
    // a basement (×1.2) and rents (×0.8).
    let a = assess("philadelphia-renters-4", "42101");
    let p = profile(&a, H::WaterDamage);
    assert!(close(p.rate_per_year, 0.015 * 1.3 * 1.2 * 0.8, 1e-12));
    assert!(p.sub_causes.iter().any(|s| s.id == "sewer_backup"));
    // Miami: no freezing days, no basement, an owner: the national rate.
    let m = assess("miami-condo-retiree-1", "12086");
    assert!(close(
        profile(&m, H::WaterDamage).rate_per_year,
        0.015,
        1e-12
    ));
    // It outranks a house fire (1 in 67 against 1 in 380 homes a year).
    assert!(profile(&m, H::WaterDamage).rate_per_year > profile(&m, H::HouseFire).rate_per_year);
}

#[test]
fn smoke_days_become_episodes_and_raise_severity_for_the_vulnerable() {
    // Philadelphia: 1.12 smoke days a year at 35.5 µg/m³ or more ÷ 3 days an episode.
    let a = assess("philadelphia-renters-4", "42101");
    let p = profile(&a, H::WildfireSmoke);
    assert!(close(p.rate_per_year, 1.12 / 3.0, 1e-6));
    assert!(p.severity >= 0.4, "a senior lives there");
    assert!(p.buckets.contains(&rr_types::BucketId::CleanAir));
    // Coos: 7.17 days, imputed from satellite maps (a wider range).
    let c = assess("coos-bay-well-owner-2", "41011");
    let q = profile(&c, H::WildfireSmoke);
    assert!(close(q.rate_per_year, 7.17 / 3.0, 1e-6));
    assert!(q.rate_range[1] / q.rate_per_year > p.rate_range[1] / p.rate_per_year);
    // Phoenix: no such days recorded: "Also checked".
    let ph = assess("phoenix-apartment-cpap-1", "04013");
    assert!(!has_profile(&ph, H::WildfireSmoke));
    assert!(
        ph.also_checked
            .iter()
            .any(|x| x.id == "wildfire_smoke" && x.rate_per_year == 0.0)
    );
}

#[test]
fn dust_storms_and_sinkholes() {
    // Maricopa's dust-storm episodes (a hand-set test value, 2 a year) × 0.3 reaching a home.
    let ph = assess("phoenix-apartment-cpap-1", "04013");
    assert!(close(profile(&ph, H::DustStorm).rate_per_year, 0.6, 1e-9));
    // Miami-Dade: 97.5 % karst × 2 in 10,000 a year.
    let m = assess("miami-condo-retiree-1", "12086");
    let s = profile(&m, H::Sinkhole);
    assert!(close(s.rate_per_year, 0.975 * 2.0e-4, 1e-6));
    assert!(s.location_factor.is_some());
}

#[test]
fn dams_count_downstream_zips_poor_condition_and_levees() {
    // A ZIP code with two high-hazard dams listing its town: 2 × 1 in 10,000 × 0.3.
    let mut f = county("42101");
    f.location.exposure.dams_high_within_10km = Some(rr_types::Sourced {
        value: 2,
        source: "usace_nid".into(),
    });
    let input = household("philadelphia-renters-4");
    let p = profile(&run(&input, &f), H::DamFailure).clone();
    assert!(close(p.rate_per_year, 2.0 * 1.0e-4 * 0.3, 1e-9));
    assert!(p.range_only);
    assert_eq!(p.location_factor.as_ref().unwrap().class, "zip_downstream");
    // The county's one high-hazard dam in poor condition: three times as likely.
    f.county.exposure.dams_high_poor_condition = Some(1);
    let q = profile(&run(&input, &f), H::DamFailure).clone();
    assert!(close(q.rate_per_year, 3.0 * p.rate_per_year, 1e-9));
    // Hays: no ZIP; its one dam reaches 1 in 100 county homes; 28.2 % live behind levees.
    let h = assess("hays-kansas-farm-5", "20051");
    let d = profile(&h, H::DamFailure);
    assert!(close(d.rate_per_year, 1.0e-4 * 0.01 + 0.282 * 0.002, 1e-6));
    let levee = d
        .sub_causes
        .iter()
        .find(|s| s.id == "levee_failure")
        .unwrap();
    assert!(close(levee.rate_range.unwrap()[1], 0.282 * 0.01, 1e-6));
}

#[test]
fn medicine_shortages_count_daily_prescriptions() {
    // Miami: one senior on a refrigerated daily medicine: 5 in 100 × 1.5.
    let m = assess("miami-condo-retiree-1", "12086");
    assert!(close(
        profile(&m, H::DrugShortage).rate_per_year,
        0.075,
        1e-9
    ));
    // Hays: no one takes a daily prescription.
    let h = assess("hays-kansas-farm-5", "20051");
    assert!(!has_profile(&h, H::DrugShortage));
    assert!(
        h.notes
            .iter()
            .any(|n| n.contains("Medicine shortages are left out"))
    );
}

#[test]
fn benefits_stop_only_for_households_that_rely_on_them() {
    let base = county("42101");
    let mut input = household("philadelphia-renters-4");
    assert!(!has_profile(&run(&input, &base), H::BenefitInterruption));
    // Federal pay: gaps of 14 days or more in 4 of 45 fiscal years (CRS).
    input.finances.benefits = vec![Benefit::FederalPay];
    let p = profile(&run(&input, &base), H::BenefitInterruption).clone();
    assert!(close(p.rate_per_year, 4.0 / 45.0, 1e-4));
    assert!(p.frequency_sentence.contains("federal pay stop"));
    // SNAP or WIC: a quarter of those gaps (November 2025).
    input.finances.benefits = vec![Benefit::SnapWic];
    let s = profile(&run(&input, &base), H::BenefitInterruption).clone();
    assert!(close(s.rate_per_year, 0.08889 * 0.25, 1e-9));
    // One lapse stops every benefit it reaches: the largest rate, not the sum.
    input.finances.benefits = vec![Benefit::SnapWic, Benefit::FederalPay, Benefit::SsiSsdi];
    let all = profile(&run(&input, &base), H::BenefitInterruption).clone();
    assert!(close(all.rate_per_year, p.rate_per_year, 1e-12));
    input.finances.benefits = vec![Benefit::SsiSsdi];
    let ssi = profile(&run(&input, &base), H::BenefitInterruption).clone();
    assert!(close(ssi.rate_per_year, 0.005, 1e-12));
}

#[test]
fn eviction_is_for_renters_and_savings_halve_it() {
    let base = county("42101");
    let mut input = household("philadelphia-renters-4");
    let p = profile(&run(&input, &base), H::Eviction).rate_per_year;
    assert!(
        close(p, 0.023, 1e-12),
        "national judgment rate, stable income"
    );
    input.finances.emergency_fund_months = 3.0;
    let q = profile(&run(&input, &base), H::Eviction).rate_per_year;
    assert!(close(q, 0.023 * 0.5, 1e-12));
    // With the county's filing rate (when the licence allows it): filings × 0.4 to judgments.
    let mut f = county("42101");
    f.county.exposure.eviction_filing_rate = Some(0.06);
    input.finances.emergency_fund_months = 0.5;
    let r = profile(&run(&input, &f), H::Eviction).rate_per_year;
    assert!(close(r, 0.06 * 0.4, 1e-6));
    // Owners never see it.
    assert!(!has_profile(
        &assess("coos-bay-well-owner-2", "41011"),
        H::Eviction
    ));
}

#[test]
fn attacks_that_close_the_area_follow_the_metro_share() {
    // 1 a year in 10 nationally × the Philadelphia area's 2.842 % × 30 % of its households.
    let a = assess("philadelphia-renters-4", "42101");
    let p = profile(&a, H::AttackDisruption);
    assert!(close(p.rate_per_year, 0.1 * 0.02842 * 0.3, 1e-9));
    assert!(p.range_only);
    assert!(p.frequency_sentence.starts_with("Between 1 in"));
    assert_eq!(p.location_factor.as_ref().unwrap().class, "uasi_large");
    // Chicago's area gets 5.304 %: nearly twice Philadelphia's.
    let c = assess("chicago-student-zero-budget-1", "17031");
    assert!(close(
        profile(&c, H::AttackDisruption).rate_per_year,
        0.1 * 0.05304 * 0.3,
        1e-9
    ));
    assert_eq!(
        profile(&c, H::AttackDisruption)
            .location_factor
            .as_ref()
            .unwrap()
            .class,
        "uasi_top"
    );
}

#[test]
fn arrests_are_summed_over_the_household_by_age() {
    // FBI 2023–2025, per 100,000 a year, men's and women's rates averaged; adults 18–64 weighted
    // by the years each band covers (7, 10, 10, 10, 10).
    let adult = (7.0 * (5390.0 + 2122.0) / 2.0
        + 10.0 * (6816.0 + 2647.0) / 2.0
        + 10.0 * (6198.0 + 2400.0) / 2.0
        + 10.0 * (3784.0 + 1295.0) / 2.0
        + 10.0 * (2093.0 + 584.0) / 2.0)
        / 47.0
        / 1e5;
    let senior = (495.3 + 115.3) / 2.0 / 1e5;
    let teen = (1992.0 + 948.5) / 2.0 / 1e5;
    // Philadelphia: two adults, a child (not counted) and a senior.
    let a = assess("philadelphia-renters-4", "42101");
    let p = profile(&a, H::ArrestOrDetention);
    assert!(close(p.rate_per_year, 2.0 * adult + senior, 1e-9));
    assert!(
        p.frequency_sentence
            .contains("about 7 arrests for every 100 households a year")
    );
    assert!(p.frequency_sentence.contains("not guilt"));
    assert!(
        p.frequency_sentence
            .contains("one person arrested twice counts twice")
    );
    assert_eq!(p.confidence, rr_types::DataConfidence::Medium);
    assert_eq!(
        p.buckets,
        [rr_types::BucketId::Income, rr_types::BucketId::HomeLoss]
    );
    // Hays: two adults, a teen, a child and a toddler.
    let h = assess("hays-kansas-farm-5", "20051");
    assert!(close(
        profile(&h, H::ArrestOrDetention).rate_per_year,
        2.0 * adult + teen,
        1e-9
    ));
    // The range runs from women's lowest year to men's highest.
    let [lo, hi] = p.rate_range;
    assert!(lo < p.rate_per_year && hi > p.rate_per_year);
}

#[test]
fn a_pack_base_rate_replaces_the_built_in_arrest_table() {
    let f = county("42101");
    let input = household("miami-condo-retiree-1");
    let mut rates = base_rates();
    for sex in ["male", "female"] {
        rates.push(rr_types::BaseRate {
            id: format!("arrests_per_100k_{sex}_65_plus"),
            value: 1000.0,
            unit: "arrests per 100,000 a year".into(),
            low: None,
            high: None,
            source: "fbi_cde_arrests".into(),
            year: 2025,
            note: "test".into(),
        });
    }
    let a = rr_hazards::assess(&input, &f.county, &rates, &f.location);
    assert!(close(
        profile(&a, H::ArrestOrDetention).rate_per_year,
        0.01,
        1e-12
    ));
}

// ------------------------------------------------------------------------------------------
// Named scenarios added in v0.2.0.
// ------------------------------------------------------------------------------------------

fn scenario_ids(a: &rr_hazards::HazardAssessment) -> Vec<&str> {
    a.scenarios.iter().map(|s| s.id.as_str()).collect()
}

#[test]
fn wasatch_san_andreas_and_seattle() {
    let input = household("philadelphia-renters-4");
    let slc = county_like("42101", "49035", "Salt Lake", "UT", "Utah", json!({}));
    let a = run(&input, &slc);
    let w = a.scenarios.iter().find(|s| s.id == "wasatch_m7").unwrap();
    // 43 in 100 in 50 years: 1.12 % a year, on (above half the one-in-100 yardstick).
    assert!(close(w.rate_per_year, -math::ln_1p(-0.43) / 50.0, 1e-9));
    assert!(w.on && w.default_on);
    let la = county_like(
        "42101",
        "06037",
        "Los Angeles",
        "CA",
        "California",
        json!({}),
    );
    let a = run(&input, &la);
    let s = a
        .scenarios
        .iter()
        .find(|s| s.id == "san_andreas_south_m78")
        .unwrap();
    assert!(close(s.rate_per_year, -math::ln_1p(-0.19) / 30.0, 1e-9));
    assert!(s.on);
    // King County: the Seattle fault (on by Washington's guidance, though rarer than the
    // yardstick) and Cascadia both apply; the earthquake card takes out both shares.
    let king = county_like("42101", "53033", "King", "WA", "Washington", json!({}));
    let a = run(&input, &king);
    let ids = scenario_ids(&a);
    assert!(
        ids.contains(&"cascadia_m9") && ids.contains(&"seattle_fault_m7"),
        "{ids:?}"
    );
    let sf = a
        .scenarios
        .iter()
        .find(|s| s.id == "seattle_fault_m7")
        .unwrap();
    assert!(sf.default_on && sf.rate_per_year < 0.005);
    assert!(
        sf.applies_because
            .contains("Washington asks every household")
    );
    let card = profile(&a, H::Earthquake).rate_per_year;
    let parent = rate(&a, H::Earthquake).rate_per_year;
    let scen: f64 = a
        .scenarios
        .iter()
        .filter(|s| s.hazard == H::Earthquake)
        .map(|s| s.rate_per_year)
        .sum();
    assert!(
        close(card, 0.25 * parent + scen, 1e-9),
        "{card} {parent} {scen}"
    );
}

#[test]
fn desert_heat_and_blackout() {
    // Maricopa (123 days a year over 95 °F): heat episodes × outages of a day or more (national
    // 2 in 100 a year; the county has no outage record here) × 3 days ÷ 365.
    let a = assess("phoenix-apartment-cpap-1", "04013");
    let s = a
        .scenarios
        .iter()
        .find(|s| s.id == "heat_blackout")
        .unwrap();
    assert!(s.on && s.default_on);
    assert_eq!(s.hazard, H::HeatWave);
    let heat = rate(&a, H::HeatWave).rate_per_year;
    assert!(close(s.rate_per_year, heat * 0.02 * 3.0 / 365.0, 1e-9));
    // The heat card still shows the heat waves, not heat waves plus the scenario.
    assert!(close(profile(&a, H::HeatWave).rate_per_year, heat, 1e-9));
    // Not in Philadelphia (3 days a year over 95 °F).
    let p = assess("philadelphia-renters-4", "42101");
    assert!(!scenario_ids(&p).contains(&"heat_blackout"));
}

// ------------------------------------------------------------------------------------------
// The six contract v2 fixture households (fixtures/households/pending), each in a county built
// from the data pack's own exposure rows.
// ------------------------------------------------------------------------------------------

fn v2_households() -> Vec<(&'static str, Fixture)> {
    vec![
        ("minot-missile-field-3", minot()),
        (
            "sacramento-leveed-2",
            county_like(
                "48157",
                "06067",
                "Sacramento",
                "CA",
                "California",
                json!({
                    "strategic_class": "C2",
                    "strategic_places": [place("metro_40900", "Sacramento-Roseville-Folsom, CA", "metro", "", "")],
                    "uasi_share": 0.004521, "uasi_area": "Sacramento-Roseville-Folsom, CA",
                    "geomag_lat": 44.2, "geomag_factor": 0.16,
                    "smoke_days_35": 6.12, "smoke_basis": "monitor",
                    "karst_share": 0.0, "leveed_pop_share": 0.454, "levee_risk_high_share": 0.944
                }),
            ),
        ),
        (
            "missoula-smoke-2",
            county_like(
                "41011",
                "30063",
                "Missoula",
                "MT",
                "Montana",
                json!({
                    "strategic_class": "E", "uasi_share": 0.0,
                    "geomag_lat": 53.6, "geomag_factor": 0.47,
                    "smoke_days_35": 7.62, "smoke_days_55": 3.12, "smoke_basis": "monitor",
                    "karst_share": 0.114, "leveed_pop_share": 0.0184, "levee_risk_high_share": 0.0
                }),
            ),
        ),
        (
            "detroit-snap-3",
            county_like(
                "17031",
                "26163",
                "Wayne",
                "MI",
                "Michigan",
                json!({
                    "strategic_class": "C2",
                    "strategic_places": [place("metro_19820", "Detroit-Warren-Dearborn, MI", "metro", "", "")],
                    "uasi_share": 0.005037, "uasi_area": "Detroit-Warren-Dearborn, MI",
                    "geomag_lat": 51.3, "geomag_factor": 0.37,
                    "smoke_days_35": 1.75, "smoke_basis": "monitor",
                    "karst_share": 0.11, "leveed_pop_share": 0.00187, "levee_risk_high_share": 0.0
                }),
            ),
        ),
        (
            "galveston-highrise-1",
            county_like(
                "48157",
                "48167",
                "Galveston",
                "TX",
                "Texas",
                json!({
                    "strategic_class": "C1",
                    "strategic_places": [place("metro_26420", "Houston-Pasadena-The Woodlands, TX", "metro", "", "")],
                    "uasi_share": 0.001961, "uasi_area": "Houston-The Woodlands-Sugar Land, TX",
                    "geomag_lat": 37.7, "geomag_factor": 0.1,
                    "smoke_days_35": 0.0, "smoke_basis": "monitor",
                    "karst_share": 0.0, "leveed_pop_share": 0.0738, "levee_risk_high_share": 0.0
                }),
            ),
        ),
        (
            "san-juan-2",
            county_like(
                "12086",
                "72127",
                "San Juan",
                "PR",
                "Puerto Rico",
                json!({
                    "strategic_class": "C2",
                    "strategic_places": [place("metro_41980", "San Juan-Bayamón-Caguas, PR", "metro", "", "")],
                    "uasi_share": 0.0,
                    "geomag_lat": 27.6, "geomag_factor": 0.1,
                    "smoke_days_35": 0.0, "smoke_basis": "hms_only",
                    "karst_share": 0.00781, "leveed_pop_share": 0.0, "levee_risk_high_share": 0.0
                }),
            ),
        ),
    ]
}

#[test]
fn the_contract_v2_households() {
    let find = |name: &str| -> (PlanInput, Fixture) {
        let (_, f) = v2_households()
            .into_iter()
            .find(|(n, _)| *n == name)
            .unwrap();
        (household(name), f)
    };
    for (name, fixture) in v2_households() {
        let a = run(&household(name), &fixture);
        // Every family, never in the rates; every rate valid.
        let rare = a
            .profiles
            .iter()
            .filter(|p| p.display == HazardDisplay::RareCatastrophic)
            .count();
        assert_eq!(rare, 9, "{name}");
        assert_eq!(a.rates.len(), a.profiles.len() - 9, "{name}");
        for r in &a.rates {
            assert_eq!(r.validate(), Ok(()), "{name}: {}", r.hazard);
        }
    }
    // Minot: a federal employee in a missile-field county.
    let (i, f) = find("minot-missile-field-3");
    let a = run(&i, &f);
    assert_eq!(
        profile(&a, H::NuclearAttack)
            .location_factor
            .as_ref()
            .unwrap()
            .class,
        "A"
    );
    assert!(close(
        profile(&a, H::BenefitInterruption).rate_per_year,
        0.08889,
        1e-9
    ));
    // Sacramento (Natomas): 45 % of the county behind levees, 94 % of them High or Very High
    // risk: the levee part is 0.454 × 0.2 % × 1.944.
    let (i, f) = find("sacramento-leveed-2");
    let a = run(&i, &f);
    let d = profile(&a, H::DamFailure);
    let levee = d
        .sub_causes
        .iter()
        .find(|s| s.id == "levee_failure")
        .unwrap();
    let [lo, hi] = levee.rate_range.unwrap();
    assert!(lo <= 0.454 * 0.002 * 1.944 && 0.454 * 0.002 * 1.944 <= hi);
    // Missoula: 7.62 smoke days, a senior at home: smoke is Serious.
    let (i, f) = find("missoula-smoke-2");
    let a = run(&i, &f);
    let s = profile(&a, H::WildfireSmoke);
    assert!(close(s.rate_per_year, 7.62 / 3.0, 1e-6) && s.severity >= 0.4);
    assert!(has_profile(&a, H::Eviction), "renters");
    // Detroit: SNAP, renting on gig income with no savings.
    let (i, f) = find("detroit-snap-3");
    let a = run(&i, &f);
    assert!(close(
        profile(&a, H::BenefitInterruption).rate_per_year,
        0.08889 * 0.25,
        1e-9
    ));
    assert!(close(
        profile(&a, H::Eviction).rate_per_year,
        0.023 * 1.75,
        1e-9
    ));
    // Galveston: SSI keeps coming through shutdowns; the Houston area's UASI share.
    let (i, f) = find("galveston-highrise-1");
    let a = run(&i, &f);
    assert!(close(
        profile(&a, H::BenefitInterruption).rate_per_year,
        0.005,
        1e-12
    ));
    assert!(close(
        profile(&a, H::AttackDisruption).rate_per_year,
        0.1 * 0.03996 * 0.3,
        1e-9
    ));
    // San Juan: outside every funded urban area; no EMP part on the months-long blackout.
    let (i, f) = find("san-juan-2");
    let a = run(&i, &f);
    assert!(a.also_checked.iter().any(|c| c.id == "attack_disruption"));
    let mm = profile(&a, H::MultiMonthBlackout);
    let emp = mm.sub_causes.iter().find(|s| s.id == "emp").unwrap();
    assert_eq!(emp.rate_range, Some([0.0, 0.0]));
}

#[test]
fn the_sub_cause_table_covers_the_csv() {
    // 51 sub-causes on ranked cards plus bank failure on the financial row: the CSV's 52.
    let a = assess("philadelphia-renters-4", "42101");
    let flood = profile(&a, H::RiverineFlooding);
    let ids: Vec<&str> = flood.sub_causes.iter().map(|s| s.id.as_str()).collect();
    for id in [
        "flash_flood",
        "urban_basement_flood",
        "levee_failure",
        "ice_jam_flood",
    ] {
        assert!(ids.contains(&id), "{id}: {ids:?}");
    }
    assert!(!ids.contains(&"glacial_outburst_flood"), "Juneau only");
    let hazmat = profile(&a, H::HazmatRelease);
    assert_eq!(hazmat.sub_causes.len(), 6);
    // Carbon monoxide carries its own rate (CDC: 1.5 in 10,000 households a year), not added
    // to the house-fire rate.
    let fire = profile(&a, H::HouseFire);
    let co = fire
        .sub_causes
        .iter()
        .find(|s| s.id == "co_poisoning")
        .unwrap();
    assert_eq!(co.rate_range, Some([7.0e-5, 3.0e-4]));
    assert!(close(fire.rate_per_year, 2.0 * 0.002_622, 1e-9));
}
