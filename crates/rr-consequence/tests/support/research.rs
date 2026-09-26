//! The research worked examples (docs/research/risk-model.md §8 Philadelphia, §9 Coos Bay) as
//! consequence-model inputs: households, household event rates, county outage statistics and
//! scenario candidates.
//!
//! The rates are built so that rate × share in `effects.toml` reproduces the research prototype's
//! event classes (`rm_proto/params.py`); hazards the prototype did not use in its bucket model are
//! left out, so the calibration checks this crate's maths and table against the research numbers,
//! not rr-hazards' rates. Each rate carries a plausible 10th–90th percentile range: a factor of 2
//! for expert estimates, tighter for measured rates.

#![allow(dead_code)]

use rr_consequence::{ScenarioCandidate, Survival};
use rr_hazards::AlternativeRate;
use rr_types::{
    CitationId, Evidence, HazardId, HouseholdEventRate, IncomeStability, OutageStats, PlanInput,
};

/// Research §3.3: "the 90th percentile of the worst disruption in 10 years", Λ = −ln(0.9)/10.
pub const RESEARCH_DEFAULT_RATE: f64 = 0.010_536_051_565_782_63;

fn rate(h: HazardId, r: f64, factor: f64, evidence: Evidence, src: &str) -> HouseholdEventRate {
    HouseholdEventRate {
        hazard: h,
        rate_per_year: r,
        low: r / factor,
        high: r * factor,
        evidence,
        sources: vec![CitationId::from(src)],
    }
}

fn prior(h: HazardId, r: f64) -> HouseholdEventRate {
    rate(h, r, 2.0, Evidence::Prior, "rr_risk_model_priors")
}

/// County outage statistics whose duration shares follow a log-normal with this median and 90th
/// percentile (the research fitted one per county).
pub fn outage_stats(rate: f32, median_h: f64, p90_h: f64, years: &str) -> OutageStats {
    let s = Survival::log_normal(median_h / 24.0, p90_h / 24.0);
    OutageStats {
        events_per_customer_year: rate,
        p_ge_1d: s.sf(1.0, 0.0) as f32,
        p_ge_3d: s.sf(3.0, 0.0) as f32,
        p_ge_7d: s.sf(7.0, 0.0) as f32,
        p_ge_14d: s.sf(14.0, 0.0) as f32,
        median_hours: median_h as f32,
        p90_hours: p90_h as f32,
        years_covered: years.to_owned(),
        event_definition: "county events with at least 0.5% of customers out (research prototype)"
            .to_owned(),
    }
}

/// The Philadelphia family of four renting a rowhouse (research §8.1). The fixture matches the
/// research household: two W-2 earners (regular salary), a child, a 70-year-old on daily oral
/// medication, gas furnace, window air conditioning, one 12-mile car commute.
pub fn philadelphia_household() -> PlanInput {
    rr_types::fixtures::get("philadelphia-renters-4").expect("fixture")
}

/// Household event rates for Philadelphia, reproducing the research prototype's classes with the
/// effects table's shares (rr-hazards' event definitions):
/// power (storm days 0.2/yr, county events 0.065, major regional 0.024, catastrophic 0.003, grid
/// 0.005), no tap water (main breaks 0.10, chemical 0.02, system failure 0.004, grid pumping
/// 0.002), boil notices 0.05, food (snowed in 0.5, shortages 0.2, curfews 0.03, pandemic 0.01,
/// catastrophic 0.003), medicine (pharmacy access 0.1, IT outage 0.02, displacement 0.008,
/// pandemic 0.01, catastrophic 0.003), communications (storms 0.05, catastrophic 0.003).
pub fn philadelphia_rates() -> Vec<HouseholdEventRate> {
    use HazardId::*;
    vec![
        prior(StrongWind, 0.133),
        prior(WinterWeather, 0.5),
        prior(IceStorm, 0.091),
        prior(Hurricane, 0.0242),
        prior(GridFailure, 0.005),
        prior(LocalUtilityOutage, 0.15),
        prior(HazmatRelease, 0.0333),
        prior(SupplyChainDisruption, 0.2),
        prior(CivilUnrest, 0.03),
        prior(CyberOutage, 0.02),
        // Pandemics that change daily life: a quarter of the 4.6 %/yr onsets (as rr-hazards).
        rate(
            Pandemic,
            0.0115,
            1.5,
            Evidence::Empirical,
            "cdc_pandemic_history",
        ),
        // 0.26 % of households a year, doubled for an attached rowhouse.
        rate(
            HouseFire,
            0.0052,
            1.3,
            Evidence::Empirical,
            "usfa_residential_fires",
        ),
        // Two earners at 0.083 spells a year (regular salary).
        rate(
            JobLoss,
            0.166,
            1.3,
            Evidence::Empirical,
            "bls_work_experience_2024",
        ),
        // 47.3 emergency visits per 100 people a year, four people.
        rate(
            MedicalEmergency,
            1.892,
            1.2,
            Evidence::Empirical,
            "cdc_nchs_ed_visits",
        ),
        prior(VehicleStranding, 0.1),
        prior(Burglary, 0.015),
    ]
}

/// EAGLE-I county events for Philadelphia, 2018–2025 (research §8.2): a home sits inside one
/// about 0.065 times a year; fitted median 2 h, bad case 20 h.
pub fn philadelphia_outages() -> OutageStats {
    outage_stats(0.065, 2.0, 20.0, "2018-2025")
}

/// The Coos Bay couple with a well (research §9.1). The fixture is used with two changes to match
/// the research household: income from regular W-2 jobs (the fixture says "variable") and the
/// one-in-100 dial (the fixture uses one-in-500). The fixture's generator does not change targets
/// (it is coverage, not exposure).
pub fn coos_household() -> PlanInput {
    let mut p = rr_types::fixtures::get("coos-bay-well-owner-2").expect("fixture");
    p.finances.income.stability = IncomeStability::Stable;
    p.dials.return_period = rr_types::ReturnPeriod::OneIn100;
    p
}

/// Household event rates for Coos Bay, reproducing the prototype's classes: county outage events
/// 1.09/yr, big windstorms and ice beyond the record 0.03, wildfire shutoffs 0.02, road closures
/// 0.2, store shortages 0.2, pandemic 0.01, pharmacy access 0.1, IT outage 0.02, storms cutting
/// communications 0.3, drought lowering the well 0.01, wildfire evacuation 0.003, home fire
/// 0.0026. Cascadia and the local tsunami come as scenario candidates.
pub fn coos_rates() -> Vec<HouseholdEventRate> {
    use HazardId::*;
    vec![
        // Windstorms that cut the power: 0.99 a year carries the prototype's storm phone
        // outages (0.3 a year) and its big windstorms beyond the record (0.03 a year); ice
        // storms are rare on the Oregon coast.
        prior(StrongWind, 0.99),
        prior(WinterWeather, 0.2),
        // Safety shutoffs 0.02 and warnings to leave 0.0035 a year.
        prior(Wildfire, 0.0235),
        prior(Drought, 0.01),
        prior(SupplyChainDisruption, 0.2),
        prior(CyberOutage, 0.02),
        rate(
            Pandemic,
            0.0115,
            1.5,
            Evidence::Empirical,
            "cdc_pandemic_history",
        ),
        rate(
            HouseFire,
            0.0026,
            1.3,
            Evidence::Empirical,
            "usfa_residential_fires",
        ),
        rate(
            JobLoss,
            0.166,
            1.3,
            Evidence::Empirical,
            "bls_work_experience_2024",
        ),
        rate(
            MedicalEmergency,
            0.946,
            1.2,
            Evidence::Empirical,
            "cdc_nchs_ed_visits",
        ),
        prior(VehicleStranding, 0.1),
        prior(Burglary, 0.01),
    ]
}

/// EAGLE-I county events for Coos County, 2018–2025 (research §9.2): about 1.09 a year for a
/// home; fitted median 1.8 h, bad case 8.4 h.
pub fn coos_outages() -> OutageStats {
    outage_stats(1.09, 1.8, 8.4, "2018-2025")
}

/// Cascadia (40 % in 50 years near Coos Bay, 1.02 %/yr; the time-independent recurrence of the
/// same record is 0.41 %/yr, given as an alternative) and the local tsunami it would send. The
/// research household lives outside the inundation zone and one adult works inside it, so the
/// local tsunami reaches a household member at about 27 % of the Cascadia rate (the share of the
/// week spent at work; research §9.3).
pub fn coos_scenarios(cascadia_on: bool) -> Vec<ScenarioCandidate> {
    let r = -rr_types::math::ln(0.6) / 50.0;
    let long_run = 0.0041;
    let alt = |rate: f64| {
        vec![AlternativeRate {
            label: "the long-run recurrence (41 ruptures in 10,000 years)".into(),
            rate_per_year: rate,
            sources: vec![CitationId::from("osu_cascadia_2012")],
        }]
    };
    vec![
        ScenarioCandidate {
            id: "cascadia_m9".into(),
            name: "A magnitude 9 Cascadia earthquake".into(),
            hazard: HazardId::Earthquake,
            rate_per_year: r,
            low: long_run,
            high: r,
            evidence: Evidence::Prior,
            on: cascadia_on,
            default_on: true,
            overridden: !cascadia_on,
            applies_because: "Coos County is on the Oregon coast above the Cascadia fault; Oregon asks households to be ready for at least two weeks.".into(),
            variant: Some("coast".into()),
            alternatives: alt(long_run),
            sources: vec![
                CitationId::from("osu_cascadia_2012"),
                CitationId::from("oregon_resilience_plan_2013"),
            ],
        },
        ScenarioCandidate {
            id: "local_tsunami".into(),
            name: "A tsunami from a nearby earthquake".into(),
            hazard: HazardId::Tsunami,
            rate_per_year: r * 0.27,
            low: long_run * 0.27,
            high: r * 0.27,
            evidence: Evidence::Prior,
            on: true,
            default_on: true,
            overridden: false,
            applies_because: "Coos County has a tsunami inundation zone, and one adult works in it.".into(),
            variant: None,
            alternatives: alt(long_run * 0.27),
            sources: vec![
                CitationId::from("osu_cascadia_2012"),
                CitationId::from("dogami_tsunami_faq"),
            ],
        },
    ]
}

/// Research §8.4 and §9.4 targets at 1-in-10, 1-in-50, ≈1-in-95 and 1-in-500 (days; months for
/// income). `None` where the research shows "—".
pub struct ResearchRow {
    pub bucket: rr_types::BucketId,
    pub values: [Option<f64>; 4],
}

pub const RESEARCH_RATES: [f64; 4] = [0.1, 0.02, RESEARCH_DEFAULT_RATE, 0.002];

pub fn philadelphia_research() -> Vec<ResearchRow> {
    use rr_types::BucketId::*;
    let h = 1.0 / 24.0;
    vec![
        ResearchRow {
            bucket: Power,
            values: [Some(7.0 * h), Some(1.6), Some(2.8), Some(8.0)],
        },
        ResearchRow {
            bucket: WaterOut,
            values: [Some(3.0 * h), Some(1.4), Some(2.8), Some(13.0)],
        },
        ResearchRow {
            bucket: WaterBoil,
            values: [None, Some(2.5), Some(4.0), Some(9.0)],
        },
        ResearchRow {
            bucket: Supplies,
            values: [Some(2.8), Some(6.6), Some(9.6), Some(31.0)],
        },
        ResearchRow {
            bucket: Medication,
            values: [Some(1.2), Some(6.9), Some(12.0), Some(39.0)],
        },
        ResearchRow {
            bucket: Thermal,
            values: [None, Some(19.0 * h), Some(1.7), Some(5.0)],
        },
        ResearchRow {
            bucket: Comms,
            values: [None, Some(19.0 * h), Some(1.4), Some(4.6)],
        },
        ResearchRow {
            bucket: Income,
            values: [Some(0.4), Some(2.2), Some(3.8), Some(9.5)],
        },
    ]
}

pub fn coos_research() -> Vec<ResearchRow> {
    use rr_types::BucketId::*;
    let h = 1.0 / 24.0;
    vec![
        ResearchRow {
            bucket: Power,
            values: [Some(14.0 * h), Some(3.1), Some(13.0), Some(143.0)],
        },
        ResearchRow {
            bucket: WaterOut,
            values: [Some(1.3), Some(14.0), Some(50.0), Some(196.0)],
        },
        ResearchRow {
            bucket: Supplies,
            values: [Some(2.9), Some(9.0), Some(17.0), Some(54.0)],
        },
        ResearchRow {
            bucket: Medication,
            values: [Some(1.2), Some(9.3), Some(21.0), Some(71.0)],
        },
        ResearchRow {
            bucket: Comms,
            values: [Some(14.0 * h), Some(2.9), Some(6.0), Some(37.0)],
        },
        ResearchRow {
            bucket: Thermal,
            values: [None, None, None, Some(2.0)],
        },
        ResearchRow {
            bucket: Income,
            values: [Some(0.6), Some(3.5), Some(5.8), Some(15.5)],
        },
    ]
}
