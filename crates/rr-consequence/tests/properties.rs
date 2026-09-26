//! Property tests: monotonicity, invariants, determinism, couplings, overrides, scenarios and the
//! cliff rule, over the seven fixture households.

mod support {
    pub mod generic;
    pub mod research;
}

use std::collections::BTreeMap;

use rr_consequence::{
    CountyData, DURATION_BUCKETS, MONTHS_LADDER, ScenarioCandidate, assess, assess_with_draws,
};
use rr_types::{
    BucketId, EventRate, Heating, HousingKind, IncomeStability, PlanInput, ReturnPeriod,
    TARGET_LADDER_DAYS, Target, WaterSource,
};
use support::{generic, research};

/// Fewer draws keep the property tests quick; the ranges' behaviour does not depend on N.
const TEST_DRAWS: usize = 120;

fn days(t: &Target) -> (f32, f32, f32) {
    match *t {
        Target::Days { value, low, high } => (value, low, high),
        Target::Months { value, low, high } => (value, low, high),
        _ => panic!("not a days/months target: {t:?}"),
    }
}

fn run(input: &PlanInput) -> rr_consequence::ConsequenceAssessment {
    assess_with_draws(
        input,
        &generic::rates(input),
        CountyData::default(),
        &[],
        TEST_DRAWS,
    )
}

fn with_dial(input: &PlanInput, rp: ReturnPeriod) -> PlanInput {
    let mut p = input.clone();
    p.dials.return_period = rp;
    p
}

#[test]
fn every_fixture_gives_well_formed_buckets() {
    for (name, input) in rr_types::fixtures::all() {
        let a = run(&input);
        assert_eq!(a.buckets.len(), 14, "{name}");
        for (b, id) in a.buckets.iter().zip(BucketId::ALL) {
            assert_eq!(b.id, *id, "{name}: bucket order");
            assert_eq!(b.target.kind(), id.target_kind(), "{name} {id}");
            assert_eq!(b.covered.kind(), id.target_kind(), "{name} {id}");
            assert!(!b.frequency_sentences.is_empty(), "{name} {id}");
            for s in &b.frequency_sentences {
                assert!(!s.contains('_'), "{name} {id}: id leaked into {s:?}");
                assert!(
                    !s.contains("NaN") && !s.contains("inf"),
                    "{name} {id}: {s:?}"
                );
                assert!(s.ends_with('.'), "{name} {id}: {s:?}");
            }
            if !b.contributions.is_empty() {
                let total: f32 = b.contributions.iter().map(|c| c.share).sum();
                assert!(
                    (total - 1.0).abs() < 1e-4,
                    "{name} {id}: shares add to {total}"
                );
                assert!(b.contributions.windows(2).all(|w| w[0].share >= w[1].share));
            }
            for s in &b.sources {
                assert!(s.is_well_formed(), "{name} {id}: {s}");
            }
            match b.target {
                Target::Days { value, low, high } => {
                    for v in [value, low, high] {
                        assert!(
                            v == 0.0 || TARGET_LADDER_DAYS.contains(&v),
                            "{name} {id}: {v} is not on the ladder"
                        );
                    }
                    assert!(
                        low <= value && value <= high,
                        "{name} {id}: {low} {value} {high}"
                    );
                    if value > 0.0 {
                        assert!(
                            !b.sources.is_empty(),
                            "{name} {id}: a target without sources"
                        );
                    }
                }
                Target::Months { value, low, high } => {
                    for v in [value, low, high] {
                        assert!(v == 0.0 || MONTHS_LADDER.contains(&v), "{name}: {v} months");
                    }
                    assert!(low <= value && value <= high);
                }
                Target::Evacuate {
                    p_need_10yr,
                    notice_hours_low,
                    notice_hours_high,
                    days_away,
                } => {
                    assert!((0.0..=1.0).contains(&p_need_10yr));
                    assert!(notice_hours_low <= notice_hours_high);
                    assert!(days_away == 0.0 || TARGET_LADDER_DAYS.contains(&days_away));
                }
                Target::Readiness { p_need_10yr, .. } => {
                    assert!((0.0..=1.0).contains(&p_need_10yr), "{name} {id}");
                }
            }
            if let Some(r) = &b.relief {
                assert!(r.help_arrives_days <= r.mostly_restored_days, "{name} {id}");
                assert!(!r.sources.is_empty());
            }
        }
        assert!(!a.statement.is_empty(), "{name}");
    }
}

#[test]
fn exceedance_curves_fall_and_frequencies_stay_between_0_and_100() {
    for (name, input) in rr_types::fixtures::all() {
        let a = run(&input);
        for d in &a.details {
            let mut prev = f64::INFINITY;
            for i in 0..400 {
                let x = 0.01 * rr_types::math::exp(f64::from(i) * 0.03);
                let l = d.curve.lambda(x);
                assert!(l <= prev + 1e-15, "{name} {}: Λ rose at {x}", d.bucket);
                assert!(l >= 0.0);
                prev = l;
                for years in [1.0, 10.0, 50.0] {
                    let n = d.curve.natural_frequency(x, years);
                    assert!((0.0..=100.0).contains(&n), "{name}: {n}");
                }
            }
        }
    }
}

#[test]
fn a_more_cautious_dial_never_lowers_a_target() {
    for (name, input) in rr_types::fixtures::all() {
        let runs: Vec<_> = ReturnPeriod::ALL
            .iter()
            .map(|rp| run(&with_dial(&input, *rp)))
            .collect();
        for pair in runs.windows(2) {
            for b in DURATION_BUCKETS.iter().chain([BucketId::Income].iter()) {
                let (v0, l0, h0) = days(&pair[0].bucket(*b).target);
                let (v1, l1, h1) = days(&pair[1].bucket(*b).target);
                assert!(
                    v1 >= v0 && l1 >= l0 && h1 >= h0,
                    "{name} {b}: {v0}/{l0}/{h0} → {v1}/{l1}/{h1}"
                );
            }
            for d in pair[0].details.iter().zip(&pair[1].details) {
                assert!(d.1.target_days >= d.0.target_days, "{name} {}", d.0.bucket);
            }
        }
    }
}

#[test]
fn same_input_same_output() {
    for (name, input) in rr_types::fixtures::all() {
        let a = run(&input);
        let b = run(&input);
        assert_eq!(
            serde_json::to_string(&a.buckets).unwrap(),
            serde_json::to_string(&b.buckets).unwrap(),
            "{name}"
        );
        assert_eq!(a.statement, b.statement);
        assert_eq!(a.warnings, b.warnings);
        assert_eq!(a.scenarios, b.scenarios);
    }
}

#[test]
fn a_well_makes_water_at_least_as_long_as_power() {
    let mut inputs: Vec<PlanInput> = rr_types::fixtures::all()
        .into_iter()
        .map(|(_, mut p)| {
            p.housing.water = WaterSource::Well;
            p
        })
        .collect();
    inputs.push(research::coos_household());
    for input in inputs {
        for rp in ReturnPeriod::ALL {
            let a = run(&with_dial(&input, *rp));
            let (pv, pl, ph) = days(&a.bucket(BucketId::Power).target);
            let (wv, wl, wh) = days(&a.bucket(BucketId::WaterOut).target);
            assert!(
                wv >= pv && wl >= pl && wh >= ph,
                "{rp}: water {wv}/{wl}/{wh} < power {pv}/{pl}/{ph}"
            );
            let pc = a
                .details
                .iter()
                .find(|d| d.bucket == BucketId::Power)
                .unwrap();
            let wc = a
                .details
                .iter()
                .find(|d| d.bucket == BucketId::WaterOut)
                .unwrap();
            assert!(wc.target_days >= pc.target_days);
            for x in [0.1, 1.0, 3.0, 10.0, 60.0] {
                assert!(wc.curve.lambda(x) >= pc.curve.lambda(x));
            }
        }
    }
}

#[test]
fn a_high_rise_above_the_pumps_couples_water_to_power() {
    let mut input = rr_types::fixtures::get("miami-condo-retiree-1").unwrap();
    input.housing.kind = HousingKind::ApartmentHighRise;
    input.housing.floor = 14;
    let a = run(&input);
    assert!(a.couplings.iter().any(|c| c.id == "high_rise_pumps"));
    let (pv, _, _) = days(&a.bucket(BucketId::Power).target);
    let (wv, _, _) = days(&a.bucket(BucketId::WaterOut).target);
    assert!(wv >= pv);
    input.housing.floor = 3;
    let low = run(&input);
    assert!(!low.couplings.iter().any(|c| c.id == "high_rise_pumps"));
}

#[test]
fn less_stable_income_never_lowers_the_income_target() {
    // With no job-loss rate supplied, stability enters through the national base rate.
    for (name, input) in rr_types::fixtures::all() {
        if input.finances.income.earners == 0 {
            continue;
        }
        let mut last = (0.0f32, 0.0f32, 0.0f32);
        for s in [
            IncomeStability::Stable,
            IncomeStability::Variable,
            IncomeStability::Seasonal,
            IncomeStability::Gig,
        ] {
            let mut p = input.clone();
            p.finances.income.stability = s;
            let rates: Vec<_> = generic::rates(&p)
                .into_iter()
                .filter(|r| r.hazard != rr_types::HazardId::JobLoss)
                .collect();
            let a = assess_with_draws(&p, &rates, CountyData::default(), &[], TEST_DRAWS);
            assert!(a.income.base_rate_used);
            let t = days(&a.bucket(BucketId::Income).target);
            assert!(
                t.0 >= last.0 && t.1 >= last.1 && t.2 >= last.2,
                "{name} {s}: {t:?} < {last:?}"
            );
            last = t;
        }
    }
    // With a supplied rate: a higher job-loss rate never lowers the target.
    let input = research::philadelphia_household();
    let mut prev = 0.0;
    for k in [0.5, 1.0, 1.5, 2.0, 3.0] {
        let rates: Vec<_> = research::philadelphia_rates()
            .into_iter()
            .map(|mut r| {
                if r.hazard == rr_types::HazardId::JobLoss {
                    r.rate_per_year *= k;
                    r.low *= k;
                    r.high *= k;
                }
                r
            })
            .collect();
        let a = assess_with_draws(&input, &rates, CountyData::default(), &[], TEST_DRAWS);
        assert!(a.income.target_months >= prev);
        prev = a.income.target_months;
    }
}

#[test]
fn no_earners_means_no_income_target() {
    let mut input = rr_types::fixtures::get("miami-condo-retiree-1").unwrap();
    for p in &mut input.people {
        p.earner = false;
    }
    input.finances.income.earners = 0;
    let a = run(&input);
    assert_eq!(days(&a.bucket(BucketId::Income).target).0, 0.0);
    assert!(a.bucket(BucketId::Income).frequency_sentences[0].contains("No one"));
}

#[test]
fn refrigerated_medicine_inherits_long_power_cuts() {
    let base = research::philadelphia_household();
    let mut cold = base.clone();
    cold.people[3].medical.refrigerated_rx = true;
    let rates = research::philadelphia_rates();
    let a = assess_with_draws(&base, &rates, CountyData::default(), &[], TEST_DRAWS);
    let b = assess_with_draws(&cold, &rates, CountyData::default(), &[], TEST_DRAWS);
    let ma = a
        .details
        .iter()
        .find(|d| d.bucket == BucketId::Medication)
        .unwrap();
    let mb = b
        .details
        .iter()
        .find(|d| d.bucket == BucketId::Medication)
        .unwrap();
    assert!(mb.target_days >= ma.target_days);
    // Power cuts shorter than a day do not touch the medicine: the curves agree below a day…
    let short = mb.curve.lambda(1.0) - ma.curve.lambda(1.0);
    assert!(short > 0.0);
    assert!((mb.curve.lambda(0.5) - ma.curve.lambda(0.5) - short).abs() < 1e-12);
    assert!(b.couplings.iter().any(|c| c.id == "refrigerated_medicine"));
}

#[test]
fn heating_that_needs_power_turns_winter_outages_into_cold() {
    let rates = research::philadelphia_rates();
    let gas = research::philadelphia_household();
    let mut wood = gas.clone();
    wood.housing.heating = Heating::Wood;
    let a = assess_with_draws(&gas, &rates, CountyData::default(), &[], TEST_DRAWS);
    let b = assess_with_draws(&wood, &rates, CountyData::default(), &[], TEST_DRAWS);
    let ta = a
        .details
        .iter()
        .find(|d| d.bucket == BucketId::Thermal)
        .unwrap();
    let tb = b
        .details
        .iter()
        .find(|d| d.bucket == BucketId::Thermal)
        .unwrap();
    assert!(ta.curve.lambda(2.0) > tb.curve.lambda(2.0));
    assert!(a.couplings.iter().any(|c| c.id == "heating_needs_power"));
    assert!(b.couplings.iter().any(|c| c.id == "wood_heat"));
}

#[test]
fn county_outage_records_replace_the_county_wide_storm_class() {
    let input = research::philadelphia_household();
    let rates = research::philadelphia_rates();
    let stats = research::philadelphia_outages();
    let with = assess_with_draws(
        &input,
        &rates,
        CountyData {
            outages: Some(&stats),
            ..CountyData::default()
        },
        &[],
        TEST_DRAWS,
    );
    let without = assess_with_draws(&input, &rates, CountyData::default(), &[], TEST_DRAWS);
    let o = with
        .overrides
        .iter()
        .find(|o| o.bucket == BucketId::Power)
        .expect("override recorded");
    assert!(o.plain.contains("2018-2025"));
    assert!(
        with.bucket(BucketId::Power)
            .sources
            .iter()
            .any(|s| s == "eagle_i_outages")
    );
    assert!(without.overrides.is_empty());
    // The county-wide class runs at the records' rate…
    let county_rate = |a: &rr_consequence::ConsequenceAssessment| -> f64 {
        a.curve(BucketId::Power)
            .unwrap()
            .terms
            .iter()
            .filter(|t| matches!(t.survival, rr_consequence::Survival::Empirical(_)))
            .map(|t| t.weight)
            .sum()
    };
    assert!((county_rate(&with) - 0.065).abs() < 1e-7);
    assert_eq!(county_rate(&without), 0.0);
    // …and a county with longer, more frequent outages gets a longer power target.
    let rough = research::outage_stats(0.6, 6.0, 60.0, "2014-2025");
    let r = assess_with_draws(
        &input,
        &rates,
        CountyData {
            outages: Some(&rough),
            ..CountyData::default()
        },
        &[],
        TEST_DRAWS,
    );
    assert!((county_rate(&r) - 0.6).abs() < 1e-7);
    let tw = with
        .details
        .iter()
        .find(|d| d.bucket == BucketId::Power)
        .unwrap()
        .target_days;
    let tr = r
        .details
        .iter()
        .find(|d| d.bucket == BucketId::Power)
        .unwrap()
        .target_days;
    assert!(tr > tw, "{tr} <= {tw}");
}

#[test]
fn county_event_records_replace_matching_durations() {
    let input = research::philadelphia_household();
    let rates = research::philadelphia_rates();
    let mut events = BTreeMap::new();
    events.insert(
        "boil_water_notice".to_owned(),
        EventRate {
            rate_per_year: 1.0,
            share_damaging: None,
            median_days: Some(4.0),
            p90_days: Some(12.0),
        },
    );
    let a = assess_with_draws(
        &input,
        &rates,
        CountyData {
            events: Some(&events),
            ..CountyData::default()
        },
        &[],
        TEST_DRAWS,
    );
    let o = a
        .overrides
        .iter()
        .find(|o| o.bucket == BucketId::WaterBoil)
        .expect("boil-water override");
    assert!(
        o.plain.contains("4 days") && o.plain.contains("2 weeks"),
        "{}",
        o.plain
    );
    let term = a
        .details
        .iter()
        .find(|d| d.bucket == BucketId::WaterBoil)
        .unwrap()
        .terms
        .iter()
        .find(|t| t.label == "local water problems")
        .unwrap()
        .clone();
    assert!((term.median_days - 4.0).abs() < 1e-9 && (term.p90_days - 12.0).abs() < 1e-9);
    assert!(
        a.bucket(BucketId::WaterBoil)
            .sources
            .iter()
            .any(|s| s == "county_boil_water_records")
    );
}

#[test]
fn cascadia_scenario_on_off_and_user_override() {
    let input = research::coos_household();
    let rates = research::coos_rates();
    let stats = research::coos_outages();
    let county = CountyData {
        outages: Some(&stats),
        coastal: true,
        tsunami_zone: true,
        ..CountyData::default()
    };
    let on = assess_with_draws(
        &input,
        &rates,
        county,
        &research::coos_scenarios(true),
        TEST_DRAWS,
    );
    let off = assess_with_draws(
        &input,
        &rates,
        county,
        &research::coos_scenarios(false),
        TEST_DRAWS,
    );
    assert!(on.scenarios[0].on && !off.scenarios[0].on);
    for b in DURATION_BUCKETS {
        let (a, _, _) = days(&on.bucket(b).target);
        let (z, _, _) = days(&off.bucket(b).target);
        assert!(a >= z, "{b}: on {a} < off {z}");
    }
    let summary = &on.scenarios[0].effect_summary;
    assert!(
        summary.contains("Power") && summary.contains("Tap water"),
        "{summary}"
    );
    // The same summary whether the scenario is on or off: it compares with and without.
    assert_eq!(
        on.scenarios[0].effect_summary,
        off.scenarios[0].effect_summary
    );
    // The user's toggle wins over the engine's default.
    let mut user_off = input.clone();
    user_off
        .dials
        .scenario_overrides
        .push(rr_types::ScenarioToggle {
            id: "cascadia_m9".into(),
            on: false,
        });
    let u = assess_with_draws(
        &user_off,
        &rates,
        county,
        &research::coos_scenarios(true),
        TEST_DRAWS,
    );
    assert!(!u.scenarios[0].on);
    assert_eq!(
        serde_json::to_string(&u.buckets).unwrap(),
        serde_json::to_string(&off.buckets).unwrap()
    );
}

#[test]
fn the_cliff_rule_names_cascadia_on_the_coast_and_nothing_in_philadelphia() {
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
        TEST_DRAWS,
    );
    for bucket in ["power", "water_out"] {
        let w = coos
            .warnings
            .iter()
            .find(|w| w.id == format!("cliff_{bucket}"))
            .unwrap_or_else(|| panic!("cliff warning for {bucket}: {:?}", coos.warnings));
        assert!(w.message.contains("Cascadia"), "{}", w.message);
        assert_eq!(w.related, vec!["cascadia_m9".to_owned(), bucket.to_owned()]);
        assert_eq!(w.severity, rr_types::WarningSeverity::Warn);
    }
    // The statement names the event once, not once per bucket.
    let named = coos
        .statement
        .iter()
        .filter(|s| s.contains("depends mostly on"))
        .count();
    assert_eq!(named, 1, "{:?}", coos.statement);
    // Far from the dial (1 in 500 against about 1 in 100 a year), no cliff.
    let rare = assess_with_draws(
        &with_dial(&research::coos_household(), ReturnPeriod::OneIn500),
        &research::coos_rates(),
        county,
        &research::coos_scenarios(true),
        TEST_DRAWS,
    );
    assert!(rare.warnings.is_empty());
    let phl_stats = research::philadelphia_outages();
    for rp in ReturnPeriod::ALL {
        let phl = assess_with_draws(
            &with_dial(&research::philadelphia_household(), *rp),
            &research::philadelphia_rates(),
            CountyData {
                outages: Some(&phl_stats),
                ..CountyData::default()
            },
            &[],
            TEST_DRAWS,
        );
        assert!(phl.warnings.is_empty(), "{rp}: {:?}", phl.warnings);
    }
}

#[test]
fn relief_follows_the_design_event() {
    let stats = research::coos_outages();
    let coos = assess_with_draws(
        &research::coos_household(),
        &research::coos_rates(),
        CountyData {
            outages: Some(&stats),
            coastal: true,
            tsunami_zone: true,
            ..CountyData::default()
        },
        &research::coos_scenarios(true),
        TEST_DRAWS,
    );
    let r = coos.bucket(BucketId::Power).relief.clone().expect("relief");
    assert_eq!((r.help_arrives_days, r.mostly_restored_days), (14.0, 180.0));
    assert_eq!(
        r.sources,
        vec![rr_types::CitationId::from("oregon_resilience_plan_2013")]
    );
    // Philadelphia's power design event is a hurricane with a measured restoration curve: help
    // within the 72-hour standard, mostly restored at the time to 90 % restored (5 days).
    let phl_stats = research::philadelphia_outages();
    let phl = assess_with_draws(
        &research::philadelphia_household(),
        &research::philadelphia_rates(),
        CountyData {
            outages: Some(&phl_stats),
            ..CountyData::default()
        },
        &[],
        TEST_DRAWS,
    );
    let r = phl.bucket(BucketId::Power).relief.clone().expect("relief");
    assert!(
        (r.mostly_restored_days - 5.0).abs() < 1e-4 && r.help_arrives_days == 3.0,
        "{r:?}"
    );
    // Water outages from expert estimates only: no relief rating.
    assert!(phl.bucket(BucketId::WaterOut).relief.is_none());
}

#[test]
fn value_between_integrates_the_curve() {
    let input = research::philadelphia_household();
    let stats = research::philadelphia_outages();
    let a = assess(
        &input,
        &research::philadelphia_rates(),
        CountyData {
            outages: Some(&stats),
            ..CountyData::default()
        },
        &[],
    );
    for b in DURATION_BUCKETS {
        let c = a.curve(b).unwrap();
        for (x0, x1) in [(0.0f64, 3.0f64), (3.0, 14.0), (30.0, 33.0), (0.5, 0.6)] {
            // Trapezoid rule on a fine grid.
            let n = 20_000;
            let mut num = 0.0;
            let mut prev = c.lambda(x0.max(1e-9));
            for i in 1..=n {
                let t = x0 + (x1 - x0) * f64::from(i) / f64::from(n);
                let l = c.lambda(t);
                num += 0.5 * (l + prev) * (x1 - x0) / f64::from(n);
                prev = l;
            }
            let exact = a.value_between(b, x0, x1);
            assert!(
                (exact - num).abs() <= 1e-6 + 1e-3 * num,
                "{b} [{x0}, {x1}]: {exact} vs {num}"
            );
        }
        assert_eq!(a.value_between(b, 3.0, 3.0), 0.0);
        assert_eq!(a.value_between(b, 5.0, 3.0), 0.0);
        let total = a.value_between(b, 0.0, 1e6);
        assert!((total - c.consumption_days_per_year()).abs() < 1e-6);
        assert!((a.unmet_days(b, 0.0) - c.consumption_days_per_year()).abs() < 1e-12);
    }
    // Diminishing returns (research §3.2 point 4): the first three days of Philadelphia water
    // cover many times more disruption-days than days 30 to 33.
    let ratio = a.value_between(BucketId::WaterOut, 0.0, 3.0)
        / a.value_between(BucketId::WaterOut, 30.0, 33.0);
    assert!(ratio > 20.0, "{ratio}");
}

#[test]
fn horizon_changes_the_sentences_not_the_targets() {
    let input = research::philadelphia_household();
    let mut long = input.clone();
    long.dials.horizon_years = 30;
    let rates = research::philadelphia_rates();
    let a = assess_with_draws(&input, &rates, CountyData::default(), &[], TEST_DRAWS);
    let b = assess_with_draws(&long, &rates, CountyData::default(), &[], TEST_DRAWS);
    for (x, y) in a.buckets.iter().zip(&b.buckets) {
        assert_eq!(x.target, y.target);
    }
    assert!(b.bucket(BucketId::Power).frequency_sentences[0].contains("the next 30 years"));
}

#[test]
fn households_without_commuters_have_no_get_home_walks() {
    let input = rr_types::fixtures::get("chicago-student-zero-budget-1").unwrap();
    let mut p = input.clone();
    for person in &mut p.people {
        person.commute = None;
    }
    let a = run(&p);
    assert!(a.get_home.commuters.is_empty());
    assert_eq!(
        a.bucket(BucketId::GetHome).tier_enough,
        rr_types::TierId::Now
    );
    let phl = run(&research::philadelphia_household());
    let walk = &phl.get_home.commuters[0];
    // 19 km at 3 mph (4.83 km/h) is 3.9 hours; half a litre an hour in heat.
    assert!((walk.walk_hours - 19.0 / (3.0 * 1.609_344)).abs() < 1e-12);
    assert!((walk.water_litres_in_heat - 0.5 * walk.walk_hours).abs() < 1e-12);
}

#[test]
fn a_scenario_with_no_consequence_rows_is_reported_but_changes_nothing() {
    let input = research::philadelphia_household();
    let odd = ScenarioCandidate {
        id: "example_scenario".into(),
        name: "An example scenario".into(),
        hazard: rr_types::HazardId::Earthquake,
        rate_per_year: 0.01,
        low: 0.005,
        high: 0.02,
        on: true,
        applies_because: "Test.".into(),
        variant: None,
        sources: vec!["prior_rr_event_shares".into()],
    };
    let a = assess_with_draws(
        &input,
        &research::philadelphia_rates(),
        CountyData::default(),
        &[odd],
        TEST_DRAWS,
    );
    assert_eq!(a.scenarios.len(), 1);
    assert!(a.scenarios[0].effect_summary.starts_with("Does not change"));
}
