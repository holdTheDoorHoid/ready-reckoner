//! One maximal value of every contract type: every optional field is filled in, every enum
//! variant that changes the JSON shape appears at least once. Built with struct literals, so
//! adding a field to a type fails to compile here until the sample sets it.

use rr_types::*;

fn date(y: u16, m: u8, d: u8) -> Date {
    Date::from_ymd(y, m, d).expect("valid sample date")
}

pub fn plan_input() -> PlanInput {
    PlanInput {
        planning_date: date(2026, 10, 1),
        location: LocationInput {
            country: "US".into(),
            zip: Some("19147".into()),
            county_fips: Some("42101".into()),
            setting: Setting::Urban,
        },
        housing: Housing {
            kind: HousingKind::Rowhouse,
            tenure: Tenure::Rent,
            floor: 1,
            basement: true,
            water: WaterSource::Municipal,
            sewer: Wastewater::Sewer,
            heating: Heating::Gas,
            cooling: Cooling::Window,
            backup_power: BackupPower::PowerStation,
            alarms: Alarms {
                smoke: true,
                co: false,
                extinguisher: true,
            },
        },
        people: vec![
            Person {
                age_band: AgeBand::Adult,
                pregnant_or_nursing: true,
                medical: Medical {
                    daily_rx: true,
                    refrigerated_rx: true,
                    powered_device: PoweredDevice::Other { watts: 60.0 },
                    mobility: Mobility::Limited,
                    dietary: vec!["vegetarian".into()],
                    epinephrine: true,
                },
                earner: true,
                commute: Some(Commute {
                    distance_km: 19.5,
                    mode: CommuteMode::Transit,
                    remote_possible: true,
                }),
            },
            Person {
                age_band: AgeBand::Child,
                pregnant_or_nursing: false,
                medical: Medical {
                    daily_rx: false,
                    refrigerated_rx: false,
                    powered_device: PoweredDevice::Cpap,
                    mobility: Mobility::None,
                    dietary: vec![],
                    epinephrine: false,
                },
                earner: false,
                commute: None,
            },
        ],
        pets: Pets {
            dogs: 1,
            cats: 2,
            small: 0,
            large_animals: 3,
        },
        mobility: HouseholdMobility {
            vehicles: vec![Vehicle { fuel: Fuel::Ev }],
        },
        finances: Finances {
            monthly_budget_usd: 60.0,
            one_off_budget_usd: 200.0,
            emergency_fund_months: 0.5,
            monthly_expenses_usd: Some(4200.0),
            income: Income {
                earners: 1,
                stability: IncomeStability::Gig,
            },
            insurance: Insurance {
                home_or_renters: true,
                flood: false,
                earthquake: true,
            },
        },
        existing: vec![Owned {
            item_id: "water_stored".into(),
            qty: 20.0,
            paid_usd: Some(18.5),
        }],
        assume_basics: false,
        dials: Dials {
            return_period: ReturnPeriod::OneIn50,
            climate: ClimateHorizon::Y2050,
            horizon_years: 10,
            water_level: WaterLevel::Comfortable,
            scenario_overrides: vec![ScenarioToggle {
                id: "cascadia_m9".into(),
                on: false,
            }],
            rare_catastrophic_opt_in: true,
        },
        stage: Some(Stage::HaveSomeThings),
        confidence_1to5: Some(3),
    }
}

pub fn location() -> LocationResolved {
    LocationResolved {
        country: "US".into(),
        county_fips: "42101".into(),
        county_name: "Philadelphia".into(),
        state_abbr: "PA".into(),
        state_name: "Pennsylvania".into(),
        zip: Some("19147".into()),
        zip_county_share: Some(1.0),
        centroid: LatLon {
            lat: 40.0094,
            lon: -75.1333,
        },
        nca_region: "northeast".into(),
        coastal: false,
        tsunami_zone: false,
        facility_flags: FacilityFlags {
            nuclear_plant_within_16km: false,
            nuclear_plant_within_80km: true,
            hazmat_facilities_within_5km: 12,
        },
        data_note: Some("your county; tract-level data not yet loaded".into()),
    }
}

pub fn citation() -> Citation {
    Citation {
        id: "ready_gov_water".into(),
        title: "Water".into(),
        publisher: "FEMA / Ready.gov".into(),
        year: Some(2024),
        url: "https://www.ready.gov/water".into(),
        retrieved: date(2026, 9, 25),
        quote: Some("Store at least one gallon of water per person per day.".into()),
        license: "US Government Work (public domain)".into(),
        prior: true,
    }
}

fn plan_item(done: bool) -> PlanItem {
    PlanItem {
        item_id: "water_stored".into(),
        name: "Stored drinking water".into(),
        kind: if done {
            PlanItemKind::Purchase
        } else {
            PlanItemKind::FreeAction
        },
        quantity: 12.0,
        unit: "gallon".into(),
        est_cost_usd: 9.0,
        price_band: CostRange {
            low: 0.0,
            high: 18.0,
        },
        buckets: vec![BucketId::WaterOut, BucketId::WaterBoil],
        hazards: vec![HazardId::WinterWeather],
        why: "Three days of water for four people.".into(),
        risk_reduction: 4.5,
        tier: TierId::H72,
        done,
        paid_usd: done.then_some(8.0),
    }
}

fn bucket(id: BucketId, target: Target, relief: Option<Relief>) -> BucketAssessment {
    BucketAssessment {
        id,
        name: id.name().into(),
        target,
        covered: target,
        covered_today: target,
        tier_enough: TierId::W2,
        contributions: vec![Contribution {
            hazard: HazardId::IceStorm,
            share: 1.0,
        }],
        frequency_sentences: vec!["About 12 of 100 households like yours ...".into()],
        sources: vec!["eagle_i_outages".into()],
        relief,
    }
}

fn hazard(id: HazardId, display: HazardDisplay) -> HazardProfile {
    HazardProfile {
        id,
        name: id.name().into(),
        tier: id.tier(),
        display,
        rate_per_year: 0.3,
        rate_range: [0.2, 0.45],
        annual_probability: 0.26,
        probability_range: [0.18, 0.36],
        severity: 0.4,
        eal_per_household_usd: Some(120.0),
        climate_multiplier: 1.1,
        confidence: DataConfidence::Medium,
        sources: vec!["fema_nri".into()],
        frequency_sentence: "About 26 of 100 households like yours each year.".into(),
        buckets: vec![BucketId::Power, BucketId::Thermal],
    }
}

pub fn plan_output() -> PlanOutput {
    PlanOutput {
        engine_version: "0.1.0".into(),
        api_version: ENGINE_API_VERSION,
        data_pack_version: "core-2026.09".into(),
        content_version: "content-2026.09".into(),
        location: location(),
        register: vec![
            hazard(HazardId::IceStorm, HazardDisplay::Ranked),
            hazard(HazardId::NuclearAttack, HazardDisplay::RareCatastrophic),
        ],
        buckets: vec![
            bucket(
                BucketId::Power,
                Target::Days {
                    value: 3.0,
                    low: 2.0,
                    high: 5.0,
                },
                Some(Relief {
                    help_arrives_days: 3.0,
                    mostly_restored_days: 7.0,
                    sources: vec!["oregon_resilience_plan".into()],
                }),
            ),
            bucket(
                BucketId::Income,
                Target::Months {
                    value: 3.0,
                    low: 2.0,
                    high: 6.0,
                },
                None,
            ),
            bucket(
                BucketId::Evacuate,
                Target::Evacuate {
                    p_need_10yr: 0.05,
                    notice_hours_low: 0.25,
                    notice_hours_high: 48.0,
                    days_away: 5.0,
                },
                None,
            ),
            bucket(
                BucketId::Fire,
                Target::Readiness {
                    p_need_10yr: 0.03,
                    done: 2,
                    of: 5,
                },
                None,
            ),
        ],
        scenarios: vec![ScenarioInfo {
            id: "cascadia_m9".into(),
            name: "A magnitude 9 Cascadia earthquake".into(),
            applies_because: "Your county is on the Oregon coast.".into(),
            on: true,
            effect_summary: "Raises no-water days from 14 to 50.".into(),
            sources: vec!["oregon_resilience_plan".into()],
        }],
        tier_reached: TierId::H72,
        tier_recommended: TierId::W2,
        plan: Plan {
            months: vec![PlanMonth {
                index: 0,
                budget_usd: 260.0,
                items: vec![plan_item(true), plan_item(false)],
            }],
            done_month: Some(7),
            envelopes: vec![SavingsEnvelope {
                item_id: "power_station_small".into(),
                saved_usd: 60.0,
                needed_usd: 250.0,
            }],
            savings_track: Some(SavingsTrack {
                target_months: 3.0,
                target_usd: 12600.0,
                current_months: 0.5,
                monthly_suggestion_usd: 100.0,
                why: "Most job losses last about ten weeks.".into(),
            }),
        },
        requirements: vec![RequirementLine {
            id: "water_out.water_stored".into(),
            bucket: BucketId::WaterOut,
            item_class: "water_stored".into(),
            quantity: 12.0,
            unit: "gallon".into(),
            per: Per::Person,
            rule: "water_gallons".into(),
            citations: vec!["ready_gov_water".into()],
            plain: "12 gallons: 1 gallon per person per day for 3 days.".into(),
        }],
        warnings: vec![Warning {
            id: "no_renters_insurance".into(),
            severity: WarningSeverity::Warn,
            message: "You have no renters insurance.".into(),
            why: "It pays for a place to stay if a fire makes your home unlivable.".into(),
            related: vec!["home_loss".into()],
        }],
        packet_markdown: "# Your plan\n".into(),
        provenance: vec![citation()],
    }
}

pub fn item() -> Item {
    Item {
        id: "water_stored".into(),
        name: "Stored drinking water".into(),
        category: "water".into(),
        unit: "gallon".into(),
        buckets: vec![BucketId::WaterOut, BucketId::WaterBoil],
        tier: TierId::H72,
        free: false,
        life_safety: true,
        rare_catastrophic: false,
        assumed_basic: true,
        spec: "Bottled water, or tap water in clean food-grade containers.".into(),
        look_for: vec!["Food-grade containers".into()],
        avoid: vec!["Milk jugs".into()],
        price_band_usd: PriceBand {
            low: 0.0,
            high: 1.5,
            per: "gallon".into(),
            note: Some("reused bottles are free".into()),
        },
        retrieved: Some(date(2026, 9, 25)),
        quantity_rule: "water_gallons".into(),
        maintenance: Some(Maintenance {
            rotate_months: Some(6),
            check_months: Some(3),
        }),
        citations: vec!["ready_gov_water".into()],
        hazard_extras: vec![HazardId::Hurricane],
        energy_kcal_per_unit: Some(0.0),
        volume_l_per_unit: Some(3.785),
    }
}

pub fn catalogue() -> Catalogue {
    Catalogue {
        items: vec![item()],
        citations: vec![citation()],
        guidance: vec![GuidanceMeta {
            id: "water".into(),
            title: "Water".into(),
            applies_to: vec!["water_out".into(), "water_boil".into()],
            citations: vec!["ready_gov_water".into()],
        }],
        hazards: HazardInfo::all(),
        buckets: BucketInfo::all(),
        tiers: TierInfo::all(),
    }
}

pub fn engine_info() -> EngineInfo {
    EngineInfo {
        engine_version: "0.1.0".into(),
        api_version: ENGINE_API_VERSION,
        data_pack_version: Some("core-2026.09".into()),
        content_version: "content-2026.09".into(),
        packs_loaded: vec!["core".into()],
        attributions: vec![Attribution {
            source: "FEMA National Risk Index".into(),
            text: "Uses National Risk Index data; not endorsed by FEMA.".into(),
            url: "https://hazards.fema.gov/nri/".into(),
            version: Some("1.20".into()),
            accessed: date(2026, 9, 25),
        }],
    }
}

pub fn pack_info() -> PackInfo {
    PackInfo {
        name: "core".into(),
        version: "2026.09".into(),
        rows: 3232,
    }
}

pub fn explain_request() -> ExplainRequest {
    ExplainRequest {
        kind: ExplainKind::Bucket,
        id: "power".into(),
        input: plan_input(),
    }
}

pub fn explanation() -> Explanation {
    Explanation {
        title: "Why three days of power".into(),
        plain: vec!["Outages longer than three days are rare here.".into()],
        math: Some(vec!["Λ(3) = 0.009 ≤ 1/100".into()]),
        sources: vec![citation()],
    }
}

pub fn problem() -> Problem {
    Problem::new(
        ProblemCode::ZipFormat,
        "location.zip",
        "A ZIP code is 5 digits, like 19147.",
    )
}

pub fn bad_input_details() -> BadInputDetails {
    BadInputDetails {
        problems: vec![problem()],
    }
}

pub fn location_suggestions() -> LocationSuggestions {
    LocationSuggestions {
        suggestions: vec![location()],
    }
}

pub fn engine_error() -> EngineError {
    EngineError::bad_input(vec![problem()])
}

pub fn effect(duration: DurationDist) -> Effect {
    Effect {
        hazard: HazardId::IceStorm,
        bucket: BucketId::Power,
        p_given_event: 0.4,
        duration,
        evidence: Evidence::Empirical,
        sources: vec!["eagle_i_outages".into()],
    }
}

pub fn log_normal() -> DurationDist {
    DurationDist::LogNormal {
        median_days: 2.0,
        p90_days: 7.0,
    }
}

pub fn fixed() -> DurationDist {
    DurationDist::Fixed { days: 1.0 }
}
