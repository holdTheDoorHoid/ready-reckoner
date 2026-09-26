//! Invariants over every fixture and dial, monotonicity in the household, determinism, and the
//! preference for local data over national fallbacks.

mod common;

use common::*;
use rr_types::{
    ClimateHorizon, Cooling, EventRate, FloodPriors, HazardDisplay, HazardId as H, HazardTier,
    HousingKind, IncomeStability, ReturnPeriod, Seismic, Vehicle, math,
};

fn all_dial_variants() -> Vec<(String, rr_types::PlanInput, Fixture)> {
    let mut out = Vec::new();
    for (name, fips) in PAIRS {
        let fixture = county(fips);
        for climate in [ClimateHorizon::Today, ClimateHorizon::Y2050] {
            for rp in ReturnPeriod::ALL {
                for years in [1u8, 10, 30] {
                    let mut input = household(name);
                    input.dials.climate = climate;
                    input.dials.return_period = *rp;
                    input.dials.horizon_years = years;
                    out.push((
                        format!("{name} {climate} {rp} {years}y"),
                        input,
                        fixture.clone(),
                    ));
                }
            }
        }
    }
    out
}

#[test]
fn every_profile_and_rate_is_well_formed() {
    let ids = rr_hazards::CITATION_IDS;
    let base_ids: Vec<String> = base_rates().iter().map(|b| b.source.to_string()).collect();
    for (label, input, fixture) in all_dial_variants() {
        let a = run(&input, &fixture);
        assert!(!a.profiles.is_empty(), "{label}");
        let mut seen = std::collections::BTreeSet::new();
        let mut rare_started = false;
        for p in &a.profiles {
            assert!(seen.insert(p.id), "{label}: {} twice", p.id);
            assert!(!p.sources.is_empty(), "{label}: {} has no source", p.id);
            for s in &p.sources {
                assert!(
                    ids.contains(&s.as_str()) || base_ids.contains(&s.to_string()),
                    "{label}: {} cites unknown {s}",
                    p.id
                );
            }
            let [lo, hi] = p.rate_range;
            assert!(
                lo.is_finite() && hi.is_finite() && 0.0 <= lo && lo <= p.rate_per_year,
                "{label}: {} range {lo} {} {hi}",
                p.id,
                p.rate_per_year
            );
            assert!(p.rate_per_year <= hi, "{label}: {}", p.id);
            let [plo, phi] = p.probability_range;
            assert!(
                0.0 <= plo
                    && plo <= p.annual_probability
                    && p.annual_probability <= phi
                    && phi <= 1.0,
                "{label}: {} probability range",
                p.id
            );
            let want = 1.0 - math::exp(-p.rate_per_year);
            assert!(
                (p.annual_probability - want).abs() < 1e-12,
                "{label}: {}",
                p.id
            );
            assert!((0.0..=1.0).contains(&p.severity), "{label}: {}", p.id);
            assert!(p.climate_multiplier.is_finite() && p.climate_multiplier > 0.0);
            if input.dials.climate == ClimateHorizon::Today {
                assert_eq!(p.climate_multiplier, 1.0, "{label}: {}", p.id);
            }
            assert_eq!(p.name, p.id.name());
            assert_eq!(p.tier, p.id.tier());
            assert!(!p.buckets.is_empty());
            assert!(!p.frequency_sentence.is_empty() && !p.frequency_sentence.contains("NaN"));
            assert!(
                p.frequency_sentence.ends_with('.'),
                "{}",
                p.frequency_sentence
            );
            if p.display == HazardDisplay::RareCatastrophic {
                rare_started = true;
                // Research §6.3: never a point estimate.
                assert!(!p.frequency_sentence.contains("households like yours"));
            } else {
                assert!(!rare_started, "{label}: ranked {} after the rare box", p.id);
            }
        }
        // Rates: one per profile, HazardId order, each valid.
        assert_eq!(a.rates.len(), a.profiles.len(), "{label}");
        assert!(
            a.rates.windows(2).all(|w| w[0].hazard < w[1].hazard),
            "{label}"
        );
        for r in &a.rates {
            assert_eq!(r.validate(), Ok(()), "{label}: {}", r.hazard);
            assert!(seen.contains(&r.hazard));
        }
        for s in &a.scenarios {
            assert_eq!(s.household_rate().validate(), Ok(()), "{label}: {}", s.id);
            assert!(!s.sources.is_empty() && !s.applies_because.is_empty());
        }
    }
}

#[test]
fn assess_is_deterministic() {
    for (label, input, fixture) in all_dial_variants().into_iter().step_by(5) {
        let a = serde_json::to_string(&run(&input, &fixture)).unwrap();
        let b = serde_json::to_string(&run(&input, &fixture)).unwrap();
        assert_eq!(a, b, "{label}");
    }
}

#[test]
fn the_2050_dial_never_reorders_the_register() {
    for (name, fips) in PAIRS {
        let fixture = county(fips);
        let mut input = household(name);
        input.dials.climate = ClimateHorizon::Today;
        let today: Vec<H> = run(&input, &fixture)
            .profiles
            .iter()
            .map(|p| p.id)
            .collect();
        input.dials.climate = ClimateHorizon::Y2050;
        let future: Vec<H> = run(&input, &fixture)
            .profiles
            .iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(today, future, "{name}");
    }
}

#[test]
fn the_2050_dial_leaves_earthquakes_and_people_alone() {
    // Research §5.4: no climate adjustment to earthquakes, tsunamis, volcanoes, or societal and
    // personal hazards.
    for (name, fips) in PAIRS {
        let fixture = county(fips);
        let mut input = household(name);
        input.dials.climate = ClimateHorizon::Today;
        let today = run(&input, &fixture);
        input.dials.climate = ClimateHorizon::Y2050;
        let future = run(&input, &fixture);
        for p in &future.profiles {
            let fixed = matches!(p.id, H::Earthquake | H::Tsunami | H::VolcanicActivity)
                || p.tier != HazardTier::Natural;
            if fixed {
                assert_eq!(p.climate_multiplier, 1.0, "{name}: {}", p.id);
                let before = today.profiles.iter().find(|q| q.id == p.id).unwrap();
                assert_eq!(before.rate_per_year, p.rate_per_year, "{name}: {}", p.id);
            }
        }
    }
}

#[test]
fn heat_multipliers_in_the_south_are_at_least_one_and_capped_at_three() {
    for (name, fips) in [
        ("miami-condo-retiree-1", "12086"),
        ("sugar-land-ev-household-3", "48157"),
        ("phoenix-apartment-cpap-1", "04013"),
    ] {
        let a = assess(name, fips); // these fixtures use the 2050 dial
        let heat = profile(&a, H::HeatWave);
        assert!(
            heat.climate_multiplier >= 1.0 && heat.climate_multiplier <= 3.0 + 1e-12,
            "{name}: {}",
            heat.climate_multiplier
        );
    }
    // Philadelphia (CMRA: days over 95 °F 3.28 -> 19.04): (11.07 + 15.76) / 11.07 = 2.42.
    let mut input = household("philadelphia-renters-4");
    input.dials.climate = ClimateHorizon::Y2050;
    let a = run(&input, &county("42101"));
    let m = profile(&a, H::HeatWave).climate_multiplier;
    assert!((m - (11.07 + 19.04 - 3.277) / 11.07).abs() < 0.01, "{m}");
    // Cold spells and winter storms fall, but not below half (research §5.3).
    assert_eq!(profile(&a, H::ColdWave).climate_multiplier, 0.5);
    assert_eq!(profile(&a, H::WinterWeather).climate_multiplier, 0.5);
    // Heavier rain: days over 2 in. 1.471 -> 2.012 (CMRA).
    let f = profile(&a, H::RiverineFlooding).climate_multiplier;
    assert!((f - 2.012 / 1.471).abs() < 0.001, "{f}");
    // Tornado, hail, ice storms and windstorms are unclear: unchanged.
    for h in [H::Tornado, H::Hail, H::IceStorm, H::StrongWind] {
        assert_eq!(profile(&a, h).climate_multiplier, 1.0, "{h}");
    }
}

#[test]
fn more_earners_and_less_steady_income_mean_more_job_loss() {
    let fixture = county("42101");
    let base = household("philadelphia-renters-4");
    let r0 = profile(&run(&base, &fixture), H::JobLoss).rate_per_year;
    let mut steadier = base.clone();
    steadier.finances.income.stability = IncomeStability::VeryStable;
    let r_steadier = profile(&run(&steadier, &fixture), H::JobLoss).rate_per_year;
    assert!(
        r_steadier < r0 && close(r_steadier / r0, 0.5, 1e-12),
        "very_stable should halve job-loss incidence versus stable (research §2.7): {r_steadier} vs {r0}"
    );
    let mut more = base.clone();
    more.people[2].earner = true;
    more.finances.income.earners = 3;
    let r1 = profile(&run(&more, &fixture), H::JobLoss).rate_per_year;
    assert!(r1 > r0 && close(r1 / r0, 1.5, 1e-12));
    let mut prev = r0;
    for s in [
        IncomeStability::Variable,
        IncomeStability::Seasonal,
        IncomeStability::Gig,
    ] {
        let mut x = base.clone();
        x.finances.income.stability = s;
        let r = profile(&run(&x, &fixture), H::JobLoss).rate_per_year;
        assert!(r >= prev, "{s}");
        prev = r;
    }
}

#[test]
fn attached_homes_have_more_house_fires() {
    let fixture = county("42101");
    let mut input = household("philadelphia-renters-4");
    input.housing.kind = HousingKind::Detached;
    let detached = profile(&run(&input, &fixture), H::HouseFire).rate_per_year;
    for kind in [
        HousingKind::Rowhouse,
        HousingKind::ApartmentLowRise,
        HousingKind::ApartmentHighRise,
    ] {
        input.housing.kind = kind;
        let r = profile(&run(&input, &fixture), H::HouseFire).rate_per_year;
        assert!(r > detached, "{kind}");
    }
    input.housing.kind = HousingKind::Rowhouse;
    let row = profile(&run(&input, &fixture), H::HouseFire).rate_per_year;
    assert!(close(row, 2.0 * detached, 1e-12));
}

#[test]
fn more_people_and_vehicles_mean_more_personal_events() {
    let fixture = county("42101");
    let base = household("philadelphia-renters-4");
    let a0 = run(&base, &fixture);
    let mut more = base.clone();
    more.people.push(more.people[2].clone());
    let a1 = run(&more, &fixture);
    for h in [H::MedicalEmergency, H::ExtendedHouseholdIllness] {
        assert!(
            profile(&a1, h).rate_per_year > profile(&a0, h).rate_per_year,
            "{h}"
        );
    }
    let mut cars = base.clone();
    cars.mobility.vehicles.push(Vehicle {
        fuel: rr_types::Fuel::Ev,
    });
    let a2 = run(&cars, &fixture);
    let (v0, v2) = (
        profile(&a0, H::VehicleStranding).rate_per_year,
        profile(&a2, H::VehicleStranding).rate_per_year,
    );
    assert!(close(v2, 2.0 * v0, 1e-12));
}

#[test]
fn floods_follow_the_floor_and_basement() {
    let fixture = county("42101");
    let mut input = household("philadelphia-renters-4");
    let with_basement = profile(&run(&input, &fixture), H::RiverineFlooding).rate_per_year;
    input.housing.basement = false;
    let slab = profile(&run(&input, &fixture), H::RiverineFlooding).rate_per_year;
    input.housing.floor = 5;
    let upper = profile(&run(&input, &fixture), H::RiverineFlooding).rate_per_year;
    assert!(with_basement > slab && slab > upper);
    assert!(close(with_basement, 1.5 * slab, 1e-12));
    assert!(close(upper, 0.5 * slab, 1e-12));
}

#[test]
fn heat_waves_reach_every_household_whatever_its_cooling() {
    // NRI: 11.07 heat-wave days a year in Philadelphia, 3 days per episode. Whether a heat wave is
    // dangerous indoors depends on the home's cooling, which rr-consequence applies; the rate of
    // heat waves reaching the household does not.
    let fixture = county("42101");
    let mut input = household("philadelphia-renters-4");
    let cooled = profile(&run(&input, &fixture), H::HeatWave).rate_per_year;
    input.housing.cooling = Cooling::None;
    let hot = profile(&run(&input, &fixture), H::HeatWave).rate_per_year;
    assert_eq!(cooled, hot);
    assert!(close(hot, 11.07 / 3.0, 1e-6));
}

#[test]
fn longer_horizons_never_lower_the_counts() {
    // The natural-frequency count 100·(1 − e^(−T·r)) grows with T; check the sentence numbers
    // through the probability they are built from.
    let fixture = county("42101");
    let mut input = household("philadelphia-renters-4");
    let mut prev = vec![0.0; 64];
    for years in [1u8, 5, 10, 20, 50] {
        input.dials.horizon_years = years;
        let a = run(&input, &fixture);
        for (i, p) in a.profiles.iter().enumerate() {
            let c = per_100(p.rate_per_year, f64::from(years));
            assert!(c >= prev[i] - 1e-12, "{} at {years}", p.id);
            prev[i] = c;
        }
    }
}

// ------------------------------------------------------------------------------------------
// Local data wins over national or county-average fallbacks.
// ------------------------------------------------------------------------------------------

#[test]
fn usgs_shaking_is_preferred_to_nri_for_earthquakes() {
    let mut fixture = county("42101");
    fixture.county.seismic = Some(Seismic {
        p_pga_ge_0_1g_per_year: 0.003,
        p_pga_ge_0_2g_per_year: 0.001,
        mmi6_100yr: None,
    });
    let a = run(&household("philadelphia-renters-4"), &fixture);
    let p = profile(&a, H::Earthquake);
    assert!(close(
        p.rate_per_year,
        -math::ln_1p(-f64::from(0.003f32)),
        1e-12
    ));
    assert!(p.sources.iter().any(|s| s == "usgs_nshm_2023"));
    assert!(p.sources.iter().all(|s| s != "fema_nri_v120"));
}

#[test]
fn storm_events_episodes_are_preferred_to_nri_event_days() {
    let mut fixture = county("42101");
    fixture.county.events.insert(
        "heat_wave".into(),
        EventRate {
            rate_per_year: 2.5,
            share_damaging: None,
            median_days: Some(3.0),
            p90_days: Some(6.0),
        },
    );
    let a = run(&household("philadelphia-renters-4"), &fixture);
    let p = profile(&a, H::HeatWave);
    // 2.5 heat-wave episodes a year from Storm Events, not NRI's 11.07 event-days ÷ 3.
    assert!(close(p.rate_per_year, 2.5, 1e-12));
    assert!(p.sources.iter().any(|s| s == "noaa_storm_events"));
}

#[test]
fn nfip_flood_zone_share_and_claims_are_used_when_present() {
    let mut fixture = county("42101");
    fixture.county.flood = Some(FloodPriors {
        sfha_home_share: 0.10,
        claims_per_1000_policies_year: Some(20.0),
        mean_paid_usd: None,
    });
    let a = run(&household("philadelphia-renters-4"), &fixture);
    let p = profile(&a, H::RiverineFlooding);
    // 10 % of homes in the zone flooding at 2 %/yr (20 claims per 1,000 policies), the rest at
    // 0.2 %/yr × 2; a basement × 1.5.
    let want = (0.10f64 * 0.02 + 0.9 * 0.004) * 1.5;
    assert!(
        close(p.rate_per_year, want, 1e-6),
        "{} vs {want}",
        p.rate_per_year
    );
    assert!(p.sources.iter().any(|s| s == "openfema_nfip"));
}

#[test]
fn base_rates_from_the_pack_are_used_and_built_ins_fill_gaps() {
    let fixture = county("42101");
    let input = household("philadelphia-renters-4");
    let with = run(&input, &fixture);
    let without = rr_hazards::assess(&input, &fixture.county, &[], &fixture.location);
    // Same numbers either way (the pack carries the research figures) ...
    for h in [H::HouseFire, H::JobLoss, H::MedicalEmergency] {
        assert!(close(
            profile(&with, h).rate_per_year,
            profile(&without, h).rate_per_year,
            1e-9
        ));
    }
    // ... and the pandemic rate from the onset base rate (5 in 108 years ≈ 0.046) × the share
    // that disrupt daily life (0.25) matches the research's 1 %/yr prior within its range.
    let p = profile(&with, H::Pandemic).rate_per_year;
    assert!(close(p, 0.046 * 0.25, 1e-12));
    assert!(profile(&without, H::Pandemic).rate_per_year == 0.01);
}

#[test]
fn a_county_without_nri_data_still_gets_personal_and_societal_hazards() {
    let mut fixture = county("42101");
    fixture.county.nri.clear();
    fixture.county.outages = None;
    fixture.county.climate.clear();
    let a = run(&household("philadelphia-renters-4"), &fixture);
    assert!(a.profiles.iter().all(|p| p.tier != HazardTier::Natural));
    assert!(has_profile(&a, H::JobLoss) && has_profile(&a, H::Pandemic));
}

#[test]
fn a_probability_above_one_is_read_as_a_count() {
    // NRI v1.20 labels landslide frequency an annualised probability, yet Coos County's is 12.77.
    let a = assess("coos-bay-well-owner-2", "41011");
    let p = profile(&a, H::Landslide);
    assert!(p.rate_per_year.is_finite() && p.rate_per_year > 0.0);
}

#[test]
fn the_nuclear_plant_hazard_follows_distance() {
    let mut fixture = county("42101");
    let input = household("philadelphia-renters-4");
    let fac = fixture.county.facilities.as_mut().unwrap();
    fac.nearest_nuclear_km = Some(10.0);
    let near = profile(&run(&input, &fixture), H::NuclearPlantIncident).rate_per_year;
    fixture
        .county
        .facilities
        .as_mut()
        .unwrap()
        .nearest_nuclear_km = Some(50.0);
    let mid = profile(&run(&input, &fixture), H::NuclearPlantIncident).rate_per_year;
    fixture
        .county
        .facilities
        .as_mut()
        .unwrap()
        .nearest_nuclear_km = Some(120.0);
    let a = run(&input, &fixture);
    assert!(near > mid);
    assert!(!has_profile(&a, H::NuclearPlantIncident));
}

#[test]
fn chemical_releases_scale_with_tri_facilities() {
    let mut fixture = county("42101");
    let input = household("philadelphia-renters-4");
    let mut prev = 0.0;
    for n in [0u16, 5, 50, 500] {
        fixture.county.facilities.as_mut().unwrap().tri_facilities = n;
        let r = profile(&run(&input, &fixture), H::HazmatRelease).rate_per_year;
        assert!(r > prev, "{n}");
        prev = r;
    }
}
