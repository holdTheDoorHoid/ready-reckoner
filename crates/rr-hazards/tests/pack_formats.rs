//! Records shaped like the `core` data pack that `rr-etl` writes (agent/data, in progress): its
//! climate ratio keys, event types and base-rate ids. These check that rr-hazards reads the pack
//! as it will arrive, not only the richer test fixtures.

mod common;

use common::*;
use rr_types::{BaseRate, ClimateHorizon, EventRate, HazardId as H};

fn event(rate: f32) -> EventRate {
    EventRate {
        rate_per_year: rate,
        share_damaging: None,
        median_days: None,
        p90_days: None,
    }
}

#[test]
fn climate_ratio_keys_from_the_pack() {
    // climate.csv: ratios (future / present) at about +2 °C.
    let mut fixture = county("42101");
    fixture.county.climate = [
        ("hot_days_95f", 4.5f32),
        ("extreme_rain_days", 1.3),
        ("freezing_nights", 0.6),
        ("consecutive_dry_days_mid45", 0.98),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    let mut input = household("philadelphia-renters-4");
    input.dials.climate = ClimateHorizon::Y2050;
    let a = run(&input, &fixture);
    // Heat: the ratio, capped at ×3 (research §5.3).
    assert!(close(
        profile(&a, H::HeatWave).climate_multiplier,
        3.0,
        1e-12
    ));
    // Heavier rain on a ground-floor home with a basement.
    assert!(close(
        profile(&a, H::RiverineFlooding).climate_multiplier,
        1.3,
        1e-6
    ));
    // Cold waves fall back to freezing nights (no very-cold-nights ratio here); both are above
    // the 0.5 floor.
    assert!(close(
        profile(&a, H::ColdWave).climate_multiplier,
        0.6,
        1e-6
    ));
    assert!(close(
        profile(&a, H::WinterWeather).climate_multiplier,
        0.6,
        1e-6
    ));
    // A shorter dry spell never lowers drought below today (FEMA's hazard-multiplier floor).
    assert_eq!(profile(&a, H::Drought).climate_multiplier, 1.0);
    // Ratios from the NCA5 Atlas cite it; the CMRA ones cite CMRA.
    assert!(
        profile(&a, H::HeatWave)
            .sources
            .iter()
            .any(|s| s == "nca5_atlas")
    );
    assert!(
        a.notes
            .iter()
            .any(|n| n.contains("No increase is projected"))
    );
}

#[test]
fn a_high_scenario_column_gives_the_range() {
    let mut fixture = county("42101");
    fixture.county.climate = [("hot_days_95f", 2.0f32), ("hot_days_95f_high", 2.6)]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
    let mut input = household("philadelphia-renters-4");
    input.dials.climate = ClimateHorizon::Y2050;
    let a = run(&input, &fixture);
    let heat = profile(&a, H::HeatWave);
    assert!(close(heat.climate_multiplier, 2.0, 1e-6));
    // Today's range × the scenario range.
    let today = profile(
        &run(&household("philadelphia-renters-4"), &fixture),
        H::HeatWave,
    )
    .clone();
    assert!(heat.rate_range[1] > today.rate_range[1] * 2.0);
    assert!(
        a.notes
            .iter()
            .any(|n| n.contains("heat waves 2 to 2.6 times as often"))
    );
}

#[test]
fn event_types_from_the_pack() {
    let mut fixture = county("42101");
    for (k, v) in [
        ("heat", 2.5f32),
        ("extreme_cold", 0.6),
        ("winter_storm", 4.0),
        ("high_wind", 1.5),
        ("severe_wind_day", 6.0),
        ("ice_storm", 0.8),
    ] {
        fixture.county.events.insert(k.to_owned(), event(v));
    }
    let a = run(&household("philadelphia-renters-4"), &fixture);
    // Episodes come straight from Storm Events: no event-day conversion.
    assert!(close(profile(&a, H::HeatWave).rate_per_year, 2.5, 1e-6));
    assert!(close(profile(&a, H::ColdWave).rate_per_year, 0.6, 1e-6));
    // Winter storms: 4 episodes × the 5 % footprint.
    assert!(close(
        profile(&a, H::WinterWeather).rate_per_year,
        4.0 * 0.05,
        1e-6
    ));
    // Windstorms: high-wind episodes + severe-wind days, × 6 % footprint × 0.5 for a city home.
    assert!(close(
        profile(&a, H::StrongWind).rate_per_year,
        7.5 * 0.06 * 0.5,
        1e-6
    ));
    assert!(close(
        profile(&a, H::IceStorm).rate_per_year,
        0.8 * 0.1 * 0.5,
        1e-6
    ));
    for h in [H::HeatWave, H::ColdWave, H::WinterWeather, H::StrongWind] {
        assert!(
            profile(&a, h)
                .sources
                .iter()
                .any(|s| s == "noaa_storm_events"),
            "{h}"
        );
    }
    // Hail and tornado keep NRI's frequency: their footprint is NRI's loss ratio per NRI event.
    let plain = run(&household("philadelphia-renters-4"), &county("42101"));
    for h in [H::Hail, H::Tornado] {
        assert_eq!(
            profile(&a, h).rate_per_year,
            profile(&plain, h).rate_per_year
        );
    }
}

#[test]
fn hurdat2_passages_give_the_county_major_hurricane_share() {
    // Miami-Dade: passages within 50 nautical miles; half at major strength.
    let mut fixture = county("12086");
    fixture
        .county
        .events
        .insert("hurricane_passage".into(), event(0.20));
    fixture
        .county
        .events
        .insert("major_hurricane_passage".into(), event(0.10));
    let mut input = household("miami-condo-retiree-1");
    input.dials.climate = ClimateHorizon::Today;
    let a = run(&input, &fixture);
    let s = a
        .scenarios
        .iter()
        .find(|s| s.id == "major_hurricane_direct_hit")
        .unwrap();
    // NRI 0.305 hurricanes a year × the HURDAT2 major share 0.5 × the 0.8 footprint.
    assert!(
        close(s.rate_per_year, 0.305 * 0.5 * 0.8, 1e-3),
        "{}",
        s.rate_per_year
    );
    assert!(s.sources.iter().any(|x| x == "noaa_hurdat2"));
}

#[test]
fn base_rate_ids_from_the_pack() {
    let fixture = county("42101");
    let input = household("philadelphia-renters-4");
    // As rr-etl writes them: its own source ids, a pandemic chance with a Poisson interval, and
    // the unintentional-injury death rate.
    let pandemic = 1.0 - rr_types::math::exp(-5.0 / 108.0);
    let rates = vec![
        BaseRate {
            id: "house_fire_per_household_year".into(),
            value: 344_600.0 / 131_434_000.0,
            unit: "residential building fires per household per year".into(),
            low: None,
            high: None,
            source: "usfa_residential_fire_estimates".into(),
            year: 2023,
            note: String::new(),
        },
        BaseRate {
            id: "pandemic_onset_per_year".into(),
            value: pandemic,
            unit: "chance a new pandemic begins in a given year".into(),
            low: Some(0.018),
            high: Some(0.093),
            source: "cdc_pandemic_history".into(),
            year: 2025,
            note: String::new(),
        },
        BaseRate {
            id: "unintentional_injury_death_per_person_year".into(),
            value: 58.1 / 100_000.0,
            unit: "unintentional-injury deaths per person per year".into(),
            low: None,
            high: None,
            source: "nchs_fastats_accidental_injury".into(),
            year: 2024,
            note: String::new(),
        },
    ];
    let a = rr_hazards::assess(&input, &fixture.county, &rates, &fixture.location);
    let fire = profile(&a, H::HouseFire);
    assert!(close(
        fire.rate_per_year,
        2.0 * 344_600.0 / 131_434_000.0,
        1e-9
    ));
    assert!(
        fire.sources
            .iter()
            .any(|s| s == "usfa_residential_fire_estimates")
    );
    // Onsets × the quarter that disrupt daily life; the pack's interval carries through.
    let p = profile(&a, H::Pandemic);
    assert!(close(p.rate_per_year, pandemic * 0.25, 1e-12));
    assert!(p.rate_range[1] > 0.093 * 0.25);
    assert!(
        profile(&a, H::EarnerDeathOrDisability)
            .sources
            .iter()
            .any(|s| s == "nchs_fastats_accidental_injury")
    );
}
