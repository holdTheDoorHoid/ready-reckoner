//! Contract v2 behaviour (v0.2.0): rare families stay out of the curves, water-system fragility,
//! the pooled regional tail and its fallback, regional restoration and island curves, the
//! grid emergency in extreme cold, public water in long power cuts, clean air, evacuation causes,
//! displacement, the stress line, the simultaneous-need check, cliff marks and the sentences.

mod support {
    pub mod generic;
    pub mod research;
}

use std::collections::BTreeMap;

use rr_consequence::{
    ConsequenceAssessment, CountyData, OutageModel, RestorationCurve, StressEvent, Survival,
    assess_with_draws,
};
use rr_types::{
    Benefit, BucketId, CitationId, Evidence, HazardId, HouseholdEventRate, PlanInput, Target,
    WaterSource, WaterSystemRecord,
};
use support::{generic, research};

const DRAWS: usize = 64;

fn rate(h: HazardId, r: f64) -> HouseholdEventRate {
    HouseholdEventRate {
        hazard: h,
        rate_per_year: r,
        low: r / 2.0,
        high: r * 2.0,
        evidence: Evidence::Prior,
        sources: vec![CitationId::from("rr_risk_model_priors")],
    }
}

fn days(a: &ConsequenceAssessment, b: BucketId) -> f32 {
    match a.bucket(b).target {
        Target::Days { value, .. } => value,
        _ => 0.0,
    }
}

fn high(a: &ConsequenceAssessment, b: BucketId) -> f32 {
    match a.bucket(b).target {
        Target::Days { high, .. } => high,
        _ => 0.0,
    }
}

fn run(
    input: &PlanInput,
    rates: &[HouseholdEventRate],
    county: CountyData<'_>,
) -> ConsequenceAssessment {
    assess_with_draws(input, rates, county, &[], DRAWS)
}

/// A regional outage model shaped like Linn County, Iowa's (`agent/data-model` at 56f3fea).
fn linn_model() -> OutageModel {
    OutageModel {
        rate: 0.333,
        lam_ge: [0.0541, 0.0132, 0.00129, 0.0000346, 0.00000184],
        z: [0.51, 0.1, 0.02, 0.0041, 0.0013],
        region_counties: 312,
        causes: BTreeMap::from([
            ("wind".to_owned(), 0.47),
            ("unattributed".to_owned(), 0.48),
            ("winter".to_owned(), 0.021),
        ]),
        stress: Some(StressEvent {
            event: "August 2020 Midwest derecho".to_owned(),
            class: "wind".to_owned(),
            cause: "tornado:151777".to_owned(),
            date: "2020-08-10".to_owned(),
            recorded_in: Some("19113".to_owned()),
            distance_km: Some(0.0),
            peak_share: 0.9255,
            share_out_at_days: vec![
                (1.0, 0.9692),
                (3.0, 0.7774),
                (7.0, 0.3695),
                (14.0, 0.01459),
                (30.0, 0.0),
            ],
            source: "eaglei".to_owned(),
        }),
        ..OutageModel::default()
    }
}

fn curve(
    region: &str,
    class: &str,
    events: u32,
    t50: f32,
    t90: f32,
    factor: f32,
) -> RestorationCurve {
    RestorationCurve {
        region: region.to_owned(),
        class: class.to_owned(),
        events,
        share_out_at_days: vec![],
        t50_days: t50,
        t90_days: t90,
        factor: Some(factor),
    }
}

#[test]
fn rare_families_never_enter_a_curve() {
    let input = research::philadelphia_household();
    let with = generic::rates(&input);
    let without: Vec<_> = with
        .iter()
        .filter(|r| !r.hazard.is_rare())
        .cloned()
        .collect();
    assert!(
        with.len() > without.len(),
        "the generic rates carry rare families"
    );
    let a = run(&input, &with, CountyData::default());
    let b = run(&input, &without, CountyData::default());
    assert_eq!(
        serde_json::to_string(&a.buckets).unwrap(),
        serde_json::to_string(&b.buckets).unwrap()
    );
    for d in &a.details {
        assert!(
            d.curve.terms.iter().all(|t| !t.hazard.is_rare()),
            "{}",
            d.bucket
        );
    }
    for bucket in &a.buckets {
        assert!(bucket.contributions.iter().all(|c| !c.hazard.is_rare()));
    }
}

#[test]
fn the_water_multiplier_follows_the_county_record_and_the_answer() {
    let mut input = research::philadelphia_household();
    let rates = research::philadelphia_rates();
    let with = |share: Option<f64>, rec: Option<WaterSystemRecord>| {
        let mut p = input.clone();
        p.housing.water_system_record = rec;
        run(
            &p,
            &rates,
            CountyData {
                sdwis_violation_share: share,
                ..CountyData::default()
            },
        )
        .fragility
    };
    // x0.5 with no violations, x3 where every customer's system had one, linear between (the
    // population-weighted mean share of about 0.19 gives about x1); x1 when unknown.
    assert!((with(Some(0.0), None).multiplier - 0.5).abs() < 1e-12);
    assert!((with(Some(1.0), None).multiplier - 3.0).abs() < 1e-12);
    assert!((with(Some(0.185), None).multiplier - 0.9625).abs() < 1e-12);
    assert!((with(None, None).multiplier - 1.0).abs() < 1e-12);
    assert!(with(None, None).unknown());
    // The household's answer multiplies: fine x0.5, occasional x1.5, frequent x3, bounded x0.25
    // to x9; "unknown" counts as not asked.
    assert!((with(None, Some(WaterSystemRecord::FrequentProblems)).multiplier - 3.0).abs() < 1e-12);
    assert!(
        (with(Some(1.0), Some(WaterSystemRecord::FrequentProblems)).multiplier - 9.0).abs() < 1e-12
    );
    assert!((with(Some(0.0), Some(WaterSystemRecord::Fine)).multiplier - 0.25).abs() < 1e-12);
    assert!(with(None, Some(WaterSystemRecord::Unknown)).unknown());
    // The major system failure's share scales with it (0.0267 of local water events).
    input.housing.water_system_record = Some(WaterSystemRecord::FrequentProblems);
    let a = run(&input, &rates, CountyData::default());
    let sys = a
        .details
        .iter()
        .find(|d| d.bucket == BucketId::WaterOut)
        .unwrap()
        .terms
        .iter()
        .find(|t| t.label == "a major water system failure")
        .unwrap()
        .events_per_year;
    assert!((sys - 0.15 * 0.0267 * 3.0).abs() < 1e-9, "{sys}");
    // A frequent-problems answer never lowers a water target.
    let base = run(
        &research::philadelphia_household(),
        &rates,
        CountyData::default(),
    );
    for b in [BucketId::WaterOut, BucketId::WaterBoil] {
        assert!(days(&a, b) >= days(&base, b), "{b}");
    }
}

#[test]
fn nothing_known_about_the_water_system_is_said_and_widens_the_range() {
    let input = research::philadelphia_household();
    let rates = research::philadelphia_rates();
    let unknown = run(&input, &rates, CountyData::default());
    let known = run(
        &input,
        &rates,
        CountyData {
            sdwis_violation_share: Some(0.3),
            ..CountyData::default()
        },
    );
    let says = |a: &ConsequenceAssessment, text: &str| {
        a.bucket(BucketId::WaterOut)
            .frequency_sentences
            .iter()
            .any(|s| s.contains(text))
    };
    assert!(says(
        &unknown,
        "We have no record of how your public water system has held up"
    ));
    assert!(says(
        &known,
        "about 30 of 100 public-water customers in your county"
    ));
    assert!(
        known
            .bucket(BucketId::WaterOut)
            .sources
            .iter()
            .any(|s| s == "epa_echo_sdwa")
    );
    // The structural widening: one ladder step above the draws' 90th percentile.
    let d = unknown
        .details
        .iter()
        .find(|d| d.bucket == BucketId::WaterOut)
        .unwrap();
    assert!(d.structural_gap);
    assert!(high(&unknown, BucketId::WaterOut) > days(&unknown, BucketId::WaterOut));
    // A household on a well has no public system: no such sentence.
    let mut well = input.clone();
    well.housing.water = WaterSource::Well;
    let w = run(&well, &rates, CountyData::default());
    assert!(!says(&w, "public water system"));
}

#[test]
fn the_pooled_tail_is_the_regional_rate_at_each_length() {
    let input = research::philadelphia_household();
    let rates = research::philadelphia_rates();
    let m = linn_model();
    let a = run(
        &input,
        &rates,
        CountyData {
            outage_model: Some(&m),
            state_abbr: "IA",
            nca_region: "midwest",
            ..CountyData::default()
        },
    );
    let c = a.curve(BucketId::Power).unwrap();
    // The pool terms together give back the blended rates at 1, 3, 7 and 14 days.
    let pool = |d: f64| -> f64 {
        c.terms
            .iter()
            .filter(|t| t.class == "county")
            .map(|t| t.weight * t.sf(d))
            .sum()
    };
    for (d, want) in [
        (1.0, 0.0541),
        (3.0, 0.0132),
        (7.0, 0.00129),
        (14.0, 0.0000346),
    ] {
        let got = pool(d);
        assert!(
            (got - want).abs() <= 1e-6 * want.max(1e-3) + 1e-12,
            "{d}: {got} vs {want}"
        );
    }
    // Shown under the storm hazards by recorded cause: wind leads.
    let power = a.bucket(BucketId::Power);
    assert!(
        power
            .frequency_sentences
            .iter()
            .any(|s| s.contains("blended with about 310 nearby counties'")),
        "{:?}",
        power.frequency_sentences
    );
    assert!(
        !a.details
            .iter()
            .find(|d| d.bucket == BucketId::Power)
            .unwrap()
            .structural_gap
    );
    // Without it, the county-only fallback says so and widens the range.
    let stats = research::philadelphia_outages();
    let f = run(
        &input,
        &rates,
        CountyData {
            outages: Some(&stats),
            ..CountyData::default()
        },
    );
    assert!(
        f.bucket(BucketId::Power)
            .frequency_sentences
            .iter()
            .any(|s| s.contains("only your county's own outage records"))
    );
    assert!(
        f.details
            .iter()
            .find(|d| d.bucket == BucketId::Power)
            .unwrap()
            .structural_gap
    );
}

#[test]
fn a_thin_county_record_gets_at_least_twice_its_one_day_rate() {
    // Manhattan's underground grid records no outages of its own (rate 0), but the blended tail
    // is not zero: the pool's rate becomes twice the one-day rate.
    let m = OutageModel {
        rate: 0.0,
        lam_ge: [0.00771, 0.00229, 0.000106, 0.000000114, 0.0],
        z: [0.73, 0.17, 0.012, 0.00017, 0.0],
        region_counties: 205,
        ..OutageModel::default()
    };
    let (curve, rate) = rr_consequence::model::pooled_curve(&m, None).expect("a curve");
    assert!(
        (rate - 2.0 * f64::from(0.00771_f32)).abs() < 1e-12,
        "{rate}"
    );
    assert!((curve.sf(1.0, 0.0) - 0.5).abs() < 1e-9);
    assert!((rate * curve.sf(3.0, 0.0) - f64::from(0.00229_f32)).abs() < 1e-9);
}

#[test]
fn regional_restoration_stretches_hurricanes_and_island_grids_use_maria() {
    let input = research::philadelphia_household();
    let rates = vec![rate(HazardId::Hurricane, 0.05)];
    let curves = vec![
        curve(
            "southern_great_plains",
            "hurricane",
            91,
            2.688,
            6.123,
            1.566,
        ),
        curve("midwest", "hurricane", 3, 0.7, 2.4, 0.612),
        RestorationCurve {
            region: "puerto_rico".to_owned(),
            class: "historic:maria_2017_pr".to_owned(),
            events: 1,
            share_out_at_days: vec![
                (1.0, 1.0),
                (3.0, 1.0),
                (7.0, 1.0),
                (14.0, 0.914),
                (30.0, 0.815),
            ],
            t50_days: 90.5,
            t90_days: 171.5,
            factor: Some(5.0),
        },
    ];
    let hurricane_median = |a: &ConsequenceAssessment, label: &str| -> f64 {
        a.details
            .iter()
            .find(|d| d.bucket == BucketId::Power)
            .unwrap()
            .terms
            .iter()
            .find(|t| t.label == label)
            .map(|t| t.median_days)
            .unwrap()
    };
    let at = |state: &'static str, region: &'static str| {
        run(
            &input,
            &rates,
            CountyData {
                state_abbr: state,
                nca_region: region,
                curves: &curves,
                ..CountyData::default()
            },
        )
    };
    let tx = at("TX", "southern_great_plains");
    // Irma's 1.5-day median stretched by the region's factor 1.566.
    let f = f64::from(1.566_f32);
    assert!((hurricane_median(&tx, "hurricanes and tropical storms") - 1.5 * f).abs() < 1e-9);
    assert!(
        tx.overrides
            .iter()
            .any(|o| o.plain.contains("longer to restore here"))
    );
    // Too few events in the region's curve: the national duration stays.
    let ia = at("IA", "midwest");
    assert!((hurricane_median(&ia, "hurricanes and tropical storms") - 1.5).abs() < 1e-9);
    // Puerto Rico: the major-hurricane rows take Maria's curve (half back after about 90 days).
    let pr = at("PR", "caribbean");
    let maria = hurricane_median(&pr, "major hurricanes (category 3 or a direct hit)");
    assert!((maria - 90.5).abs() < 0.5, "{maria}");
    assert!(
        pr.bucket(BucketId::Power)
            .sources
            .iter()
            .any(|s| s == "doe_maria_situation_reports")
    );
    // No curves: nothing changes (the county-only path).
    let none = run(&input, &rates, CountyData::default());
    assert!((hurricane_median(&none, "hurricanes and tropical storms") - 1.5).abs() < 1e-9);
}

#[test]
fn a_grid_emergency_in_extreme_cold_where_the_grid_has_failed() {
    let mut input = research::philadelphia_household();
    input.housing.heating = rr_types::Heating::HeatPump;
    let rates = vec![
        rate(HazardId::ColdWave, 0.1),
        rate(HazardId::LocalUtilityOutage, 0.15),
    ];
    let has = |a: &ConsequenceAssessment, b: BucketId| {
        a.details
            .iter()
            .find(|d| d.bucket == b)
            .unwrap()
            .terms
            .iter()
            .any(|t| t.label.contains("grid emergency in extreme cold"))
    };
    let texas = run(
        &input,
        &rates,
        CountyData {
            state_abbr: "TX",
            ..CountyData::default()
        },
    );
    for b in [
        BucketId::Power,
        BucketId::WaterBoil,
        BucketId::WaterOut,
        BucketId::Supplies,
        BucketId::Thermal,
    ] {
        assert!(has(&texas, b), "{b}");
    }
    // Its rate is the class's own (1 in 30 a year), not the cold waves'.
    let power = texas
        .details
        .iter()
        .find(|d| d.bucket == BucketId::Power)
        .unwrap()
        .terms
        .iter()
        .find(|t| t.label.contains("grid emergency"))
        .unwrap()
        .events_per_year;
    assert!((power - 0.033).abs() < 1e-12, "{power}");
    let pa = run(
        &input,
        &rates,
        CountyData {
            state_abbr: "PA",
            ..CountyData::default()
        },
    );
    assert!(!has(&pa, BucketId::Power));
    // With the regional records, the county's own cause record decides, wherever it is.
    let m = OutageModel {
        rate: 0.8,
        lam_ge: [0.03, 0.0025, 0.0003, 0.00001, 0.000001],
        z: [0.7, 0.2, 0.02, 0.003, 0.0002],
        causes: BTreeMap::from([("cold_grid".to_owned(), 0.011), ("wind".to_owned(), 0.5)]),
        ..OutageModel::default()
    };
    let nc = run(
        &input,
        &rates,
        CountyData {
            state_abbr: "NC",
            outage_model: Some(&m),
            ..CountyData::default()
        },
    );
    assert!(has(&nc, BucketId::Power));
}

#[test]
fn public_water_fails_in_power_cuts_past_three_days() {
    let input = research::philadelphia_household();
    let rates = vec![rate(HazardId::IceStorm, 0.1)];
    let a = run(&input, &rates, CountyData::default());
    let water = a
        .details
        .iter()
        .find(|d| d.bucket == BucketId::WaterOut)
        .unwrap();
    let t = water
        .terms
        .iter()
        .find(|t| {
            t.origin == "coupling rule: public_water_power" && t.label == "an ice storm of record"
        })
        .expect("the long ice-storm cuts reach the water");
    // 0.1 of the power cuts (fragility unknown: x1), counted past 3 days.
    assert!(
        (t.events_per_year - 0.1 * 0.017 * 0.1).abs() < 1e-12,
        "{}",
        t.events_per_year
    );
    assert!((t.threshold_days - 3.0).abs() < 1e-12);
    assert!(a.couplings.iter().any(|c| c.id == "public_water_power"));
    // A well household has its own, stronger coupling instead.
    let mut well = input.clone();
    well.housing.water = WaterSource::Well;
    let w = run(&well, &rates, CountyData::default());
    assert!(!w.couplings.iter().any(|c| c.id == "public_water_power"));
}

#[test]
fn clean_air_counts_smoke_dust_ash_and_fumes() {
    let input = research::philadelphia_household();
    let smoke = 0.5;
    let rates = vec![
        rate(HazardId::WildfireSmoke, smoke),
        rate(HazardId::HazmatRelease, 0.02),
    ];
    let a = run(&input, &rates, CountyData::default());
    let Target::Readiness { p_need_10yr, .. } = a.bucket(BucketId::CleanAir).target else {
        panic!("readiness");
    };
    let r = smoke + 0.02 * 0.4;
    assert!((p_need_10yr - (1.0 - rr_types::math::exp(-10.0 * r))).abs() < 1e-12);
    // Days a year from the episodes' typical length (2 days, bad case 7: mean about 3.2 days).
    let mean = Survival::log_normal(2.0, 7.0).mean_days();
    let fumes = 0.02 * 0.4 * Survival::log_normal(0.25, 1.0).mean_days();
    assert!((a.clean_air.days_per_year - (smoke * mean + fumes)).abs() < 1e-9);
    assert!(!a.clean_air.smoke_days_from_record);
    // With the county's record, its smoke days replace the model's.
    let b = run(
        &input,
        &rates,
        CountyData {
            smoke_days: Some(8.27),
            ..CountyData::default()
        },
    );
    assert!((b.clean_air.days_per_year - (8.27 + fumes)).abs() < 1e-9);
    assert!(b.clean_air.smoke_days_from_record);
    assert!(
        b.bucket(BucketId::CleanAir)
            .frequency_sentences
            .iter()
            .any(|s| s.contains("about 8 days a year here (smoke days"))
    );
    assert!(
        b.bucket(BucketId::CleanAir)
            .sources
            .iter()
            .any(|s| s == "epa_aqs_daily_pm25")
    );
    assert_eq!(
        a.bucket(BucketId::CleanAir).tier_enough,
        rr_types::TierId::H72
    );
}

#[test]
fn the_warning_names_its_cause_and_the_fastest_one_outside_the_home() {
    let mut input = research::philadelphia_household();
    input.housing.below_grade_bedroom = true;
    let rates = vec![
        rate(HazardId::HouseFire, 0.0052),
        rate(HazardId::RiverineFlooding, 0.007),
        rate(HazardId::Hurricane, 0.03),
        rate(HazardId::DamFailure, 0.0002),
    ];
    let a = run(&input, &rates, CountyData::default());
    assert_eq!(a.evacuate.fastest_cause, Some(HazardId::HouseFire));
    // Flash flooding below street level: 3 minutes; the dam: 15 minutes.
    let (h, n) = a
        .evacuate
        .fastest_outside
        .expect("a fast cause outside the home");
    assert_eq!(h, HazardId::RiverineFlooding);
    assert!((n - 0.05).abs() < 1e-12, "{n}");
    let s = &a.bucket(BucketId::Evacuate).frequency_sentences;
    assert!(
        s.iter()
            .any(|x| x.contains("as short as a few minutes (a home fire)")),
        "{s:?}"
    );
    assert!(
        s.iter().any(|x| x.contains(
            "For floods from rivers and heavy rain, plan for as little as a few minutes of warning"
        )),
        "{s:?}"
    );
    // A cause rarer than 1 in 1,000 in ten years does not set the short end.
    let rare = vec![
        rate(HazardId::Hurricane, 0.03),
        rate(HazardId::DamFailure, 0.00005),
    ];
    let b = run(
        &research::philadelphia_household(),
        &rare,
        CountyData::default(),
    );
    assert_eq!(b.evacuate.fastest_cause, Some(HazardId::Hurricane));
    assert!((b.evacuate.notice_hours[0] - 24.0).abs() < 1e-12);
}

#[test]
fn displacement_months_and_their_cost() {
    let input = research::philadelphia_household();
    let rates = vec![rate(HazardId::HouseFire, 0.0052)];
    let a = run(&input, &rates, CountyData::default());
    // A home fire's displacement: 60 days, bad case a year; nine in ten back within the 90th
    // percentile, 365 days = 12 months.
    let months = a.home_loss.months_away_p90;
    assert!((months - 365.0 / 30.44).abs() < 0.05, "{months}");
    let expenses = f64::from(input.finances.monthly_expenses_usd.unwrap());
    let cost = a.home_loss.displacement_cost_usd.unwrap();
    assert!((cost - months * 0.3 * expenses).abs() < 1e-6, "{cost}");
    assert!(
        a.bucket(BucketId::HomeLoss)
            .frequency_sentences
            .iter()
            .any(|s| s.contains("home again within about 12 months")),
        "{:?}",
        a.bucket(BucketId::HomeLoss).frequency_sentences
    );
    assert_eq!(a.home_loss.months_away_at_dial, 0.0);
}

#[test]
fn the_stress_line_says_whether_the_target_outlasts_the_worst_event() {
    let input = research::philadelphia_household();
    let rates = research::philadelphia_rates();
    let m = linn_model();
    let a = run(
        &input,
        &rates,
        CountyData {
            outage_model: Some(&m),
            state_abbr: "IA",
            nca_region: "midwest",
            ..CountyData::default()
        },
    );
    let st = a
        .bucket(BucketId::Power)
        .stress_test
        .clone()
        .expect("a stress line");
    assert_eq!(st.event, "August 2020 Midwest derecho");
    assert_eq!(st.date.to_string(), "2020-08-10");
    // Shares of all customers: the peak (92.55 %) times the share of the peak still out.
    let week = st
        .share_out_at_days
        .iter()
        .find(|(d, _)| *d == 7.0)
        .unwrap()
        .1;
    assert!((f64::from(week) - 0.9255 * 0.3695).abs() < 1e-6);
    // Covered only if nine in ten of those who lost power were back by the target: between the
    // week (37 % still out) and two weeks (1.5 %), interpolated on log days.
    let target = days(&a, BucketId::Power);
    let t = f64::from(target);
    let share = if t <= 7.0 {
        1.0
    } else {
        let f = rr_types::math::ln(t / 7.0) / rr_types::math::ln(2.0);
        0.3695 + (0.01459 - 0.3695) * f.min(1.0)
    };
    assert_eq!(
        st.covered_by_target,
        share <= 0.1,
        "target {target}, share {share}"
    );
    assert!(
        a.bucket(BucketId::Power)
            .sources
            .iter()
            .any(|s| s == "ornl_eagle_i_outages")
    );
    // Water: the state's worst documented failure, for homes on public water.
    let nc = run(
        &input,
        &rates,
        CountyData {
            state_abbr: "NC",
            ..CountyData::default()
        },
    );
    let w = nc
        .bucket(BucketId::WaterOut)
        .stress_test
        .clone()
        .expect("Helene");
    assert_eq!(w.event, "Hurricane Helene");
    assert_eq!(w.covered_by_target, days(&nc, BucketId::WaterOut) >= 21.0);
    let mut well = input.clone();
    well.housing.water = WaterSource::Well;
    let wl = run(
        &well,
        &rates,
        CountyData {
            state_abbr: "NC",
            ..CountyData::default()
        },
    );
    assert!(wl.bucket(BucketId::WaterOut).stress_test.is_none());
    // No record, no line.
    let none = run(&input, &rates, CountyData::default());
    assert!(none.buckets.iter().all(|b| b.stress_test.is_none()));
}

#[test]
fn a_design_event_lists_the_needs_it_brings_at_once() {
    let input = research::philadelphia_household();
    let rates = research::philadelphia_rates();
    let a = run(&input, &rates, CountyData::default());
    assert!(!a.simultaneous.is_empty());
    for n in &a.simultaneous {
        assert!(!n.sets_target_of.is_empty());
        for b in &n.sets_target_of {
            let need = n
                .needs
                .iter()
                .find(|(x, _, _)| x == b)
                .expect("its own bucket");
            assert_eq!(need.1, days(&a, *b), "{b}");
            assert!((need.2 - 1.0).abs() < 1e-12);
        }
        for (_, d, chance) in &n.needs {
            assert!(*d >= 0.0 && (0.0..=1.0).contains(chance));
        }
    }
    // Philadelphia's power target is set by a hurricane class that also brings supplies needs.
    let power = a
        .simultaneous
        .iter()
        .find(|n| n.sets_target_of.contains(&BucketId::Power))
        .expect("power's design event");
    assert!(power.needs.len() >= 2, "{power:?}");
}

#[test]
fn income_streams_for_benefits_and_arrests() {
    let mut input = research::philadelphia_household();
    let rates = vec![
        rate(HazardId::BenefitInterruption, 0.077),
        rate(HazardId::ArrestOrDetention, 0.05),
    ];
    let a = run(&input, &rates, CountyData::default());
    // No benefit in the household: the lapse stream does not apply; arrests do, at the
    // household's rate (not times the earners).
    let streams: Vec<_> = a.income.streams.iter().map(|(h, _, r)| (*h, *r)).collect();
    assert!(
        !streams
            .iter()
            .any(|(h, _)| *h == HazardId::BenefitInterruption)
    );
    let arrests = streams
        .iter()
        .find(|(h, _)| *h == HazardId::ArrestOrDetention)
        .unwrap()
        .1;
    assert!((arrests - 0.05).abs() < 1e-12, "{arrests}");
    input.finances.benefits = vec![Benefit::FederalPay];
    let b = run(&input, &rates, CountyData::default());
    assert!(
        b.income
            .streams
            .iter()
            .any(|(h, _, r)| *h == HazardId::BenefitInterruption && (*r - 0.077).abs() < 1e-12)
    );
    // SNAP: the food row instead.
    input.finances.benefits = vec![Benefit::SnapWic];
    let c = run(&input, &rates, CountyData::default());
    assert!(
        c.bucket(BucketId::Supplies)
            .contributions
            .iter()
            .any(|x| x.hazard == HazardId::BenefitInterruption)
    );
    assert!(
        !c.income
            .streams
            .iter()
            .any(|(h, _, _)| *h == HazardId::BenefitInterruption)
    );
}

#[test]
fn the_dial_sentence_and_the_multi_month_power_curve() {
    let input = research::philadelphia_household();
    let rates = research::philadelphia_rates();
    let a = run(&input, &rates, CountyData::default());
    let s = a.dial_sentence();
    assert!(
        s.starts_with("At this setting, about 1 in 10 households like yours"),
        "{s}"
    );
    let joint = -rr_types::math::exp_m1(-10.0 * a.joint_rate());
    let k = ((joint * 10.0) + 0.5).floor() as i64;
    assert!(
        s.contains(&format!("about {k} in 10 will face at least one kind")),
        "{s}"
    );
    let m = a.multi_month_blackout();
    assert!((m.rate_60_days - a.curve(BucketId::Power).unwrap().lambda(60.0)).abs() < 1e-15);
    assert!(m.rate_90_days <= m.rate_60_days);
    assert!((0.0..=1.0).contains(&m.p10_60_days));
}

#[test]
fn only_true_cliffs_are_marked_on_the_dial_table() {
    let stats = research::coos_outages();
    let county = CountyData {
        outages: Some(&stats),
        coastal: true,
        tsunami_zone: true,
        ..CountyData::default()
    };
    let coos = assess_with_draws(
        &research::coos_household(),
        &research::coos_rates(),
        county,
        &research::coos_scenarios(true),
        DRAWS,
    );
    let water = coos
        .details
        .iter()
        .find(|d| d.bucket == BucketId::WaterOut)
        .unwrap();
    assert!(
        water.dial_table.iter().any(|p| p.cliff),
        "{:?}",
        water.dial_table
    );
    let phl_stats = research::philadelphia_outages();
    let phl = run(
        &research::philadelphia_household(),
        &research::philadelphia_rates(),
        CountyData {
            outages: Some(&phl_stats),
            ..CountyData::default()
        },
    );
    for d in &phl.details {
        assert!(
            d.dial_table.iter().all(|p| !p.cliff),
            "{}: {:?}",
            d.bucket,
            d.dial_table
        );
    }
}
