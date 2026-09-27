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
            below_grade_bedroom: true,
            cooking: Some(CookingFuel::Gas),
            raw_water_source: Some(RawWaterSource::RainBarrel),
            water_system_record: Some(WaterSystemRecord::OccasionalNotices),
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
                access_needs: vec![AccessNeed::Hearing, AccessNeed::ServiceAnimal],
                profile: Some(person_profile()),
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
                access_needs: vec![AccessNeed::Cognitive],
                profile: None,
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
                sewer_backup: Some(false),
                life_or_disability: Some(true),
            },
            benefits: vec![Benefit::SnapWic, Benefit::FederalPay],
        },
        existing: vec![Owned {
            item_id: "water_stored".into(),
            qty: 20.0,
            paid_usd: Some(18.5),
            tested_on: Some(date(2026, 9, 12)),
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
            rare_opt_in: vec!["nuclear_attack".into(), "all".into()],
            minimum_kit: true,
            long_horizon: true,
            legal_opt_in: true,
        },
        stage: Some(Stage::HaveSomeThings),
        confidence_1to5: Some(3),
        family_plan: Some(family_plan()),
    }
}

pub fn contact(name: &str, phone: &str) -> Contact {
    Contact {
        name: Some(name.into()),
        phone: Some(phone.into()),
        address: None,
    }
}

/// A contact with an address too (contract v3).
pub fn contact_at(name: &str, phone: &str, address: &str) -> Contact {
    Contact {
        address: Some(address.into()),
        ..contact(name, phone)
    }
}

/// A person's profile with every field filled in, already tidy (contract v3). Sample data only:
/// 555-01xx numbers, a made-up street.
pub fn person_profile() -> PersonProfile {
    PersonProfile {
        name: Some("Ana".into()),
        date_of_birth: Some("March 3, 1988".into()),
        phone: Some("555-0110".into()),
        email: Some("ana@example.org".into()),
        place: Some(Place {
            kind: PlaceKind::Work,
            name: Some("Riverside Clinic".into()),
            address: Some("100 Sample Lane, Philadelphia".into()),
            phone: Some("555-0111".into()),
            plan: Some("Staff stay until relieved; the clinic has a generator".into()),
            pickup: Some("Not applicable".into()),
            safest_spot: Some("The records room, ground floor".into()),
        }),
        doctor: Some(contact_at("Dr. Lee", "555-0112", "200 Sample Lane")),
        pharmacy: Some(contact("Corner pharmacy", "555-0113")),
        conditions: Some("Asthma".into()),
        medications: vec![Medication {
            name: Some("Inhaler".into()),
            dose: Some("As prescribed".into()),
            schedule: Some("Morning and night".into()),
            purpose: Some("Asthma".into()),
        }],
        allergies: Some("Penicillin".into()),
        blood_type: Some("O+".into()),
        insurance: Some(HealthInsurance {
            carrier: Some("Sample Health".into()),
            plan_name: Some("Silver".into()),
            member_id: Some("XJ-000-111".into()),
            group_number: Some("G-222".into()),
            phone: Some("555-0114".into()),
        }),
        id_notes: Some("Passport in the document box".into()),
        notes: Some("Keeps a spare inhaler in the car".into()),
    }
}

/// A family plan with every field filled in, already tidy.
pub fn family_plan() -> FamilyPlan {
    FamilyPlan {
        meeting_place_near: Some("The mailbox at the corner of 9th and Pine".into()),
        meeting_place_far: Some("The public library on Main Street".into()),
        out_of_area_contact: Some(contact("Aunt Rosa", "555-0100")),
        school_pickup: Some("Grandpa picks up from Lincoln Elementary; the school holds children until a listed adult arrives".into()),
        work_plans: Some("Sam stays at the hospital until relieved; Ana walks home".into()),
        shelter_spot_home: Some("The inner hallway on the ground floor".into()),
        shelter_spot_work: Some("The stairwell on floor 3".into()),
        where_we_would_go: Some("Rosa's house in Harrisburg".into()),
        routes: vec!["I-76 west to the turnpike".into(), "US-322 west".into()],
        neighbours_who_check: Some("Mr. Lee next door checks on us; we check on Mrs. Park".into()),
        who_takes_animals: Some("Rosa takes the dog".into()),
        shutoff_gas: Some("Beside the meter at the back; wrench on the hook".into()),
        shutoff_water: Some("Basement, front wall, blue handle".into()),
        shutoff_electric: Some("Panel in the kitchen closet".into()),
        trusted_circle: vec![
            TrustedPerson {
                name: Some("Rosa".into()),
                phone: Some("555-0100".into()),
                holds: vec![Holds::SpareKey, Holds::Documents],
            },
            TrustedPerson {
                name: Some("Dev".into()),
                phone: Some("555-0101".into()),
                holds: vec![Holds::MedicalPoa, Holds::BackupCodes],
            },
        ],
        lawyer: Some(contact_at("J. Ortiz", "555-0102", "1 Court Street")),
        roadside_assistance: Some("555-0103".into()),
        numbers_by_heart: vec!["555-0100".into(), "555-0104".into()],
        home: Some(HomeInfo {
            address: Some("12 Sample Street, Philadelphia".into()),
            electric_utility: Some(contact("City Electric", "555-0120")),
            gas_utility: Some(contact("City Gas", "555-0121")),
            water_utility: Some(contact("Water Department", "555-0122")),
            insurer: Some(contact("Sample Mutual", "555-0123")),
            policy_number: Some("HO-12345".into()),
            landlord_or_mortgage: Some(contact("Pine Street Rentals", "555-0124")),
            where_kit: Some("Hall closet".into()),
            where_documents: Some("Fireproof box under the bed".into()),
            where_cash: Some("Envelope in the document box".into()),
            where_keys: Some("With Rosa".into()),
        }),
        neighbourhood: Some(Neighbourhood {
            hospital: Some(contact_at("General Hospital", "555-0130", "300 Sample Avenue")),
            urgent_care: Some(contact("Walk-in clinic", "555-0131")),
            pharmacy: Some(contact("Corner pharmacy", "555-0113")),
            shelter: Some(contact_at("Rec center", "555-0132", "400 Sample Avenue")),
            county_emergency_office: Some(contact("Office of Emergency Management", "555-0133")),
            alerts: Some("The city's text alerts; the news radio station".into()),
        }),
        pets: vec![PetInfo {
            name: Some("Biscuit".into()),
            kind: Some("Dog".into()),
            description: Some("Brown terrier, red collar".into()),
            medications: Some("Ear drops".into()),
            vet: Some(contact("Sample Vet", "555-0140")),
            microchip: Some("985-000-000".into()),
            records_where: Some("Document box".into()),
        }],
        vehicles: vec![VehicleInfo {
            description: Some("Blue 2016 hatchback".into()),
            plate: Some("ABC-1234".into()),
            insurer: Some(contact("Sample Auto", "555-0150")),
            policy_number: Some("AU-555".into()),
            kept_in_car: Some("Blanket, water, a phone charger".into()),
        }],
        documents: Some(DocumentsInfo {
            accounts: vec![AccountInfo {
                institution: Some("Sample Credit Union".into()),
                kind: Some("Checking".into()),
                phone: Some("555-0160".into()),
                last4: Some("4821".into()),
            }],
            policies: vec![PolicyInfo {
                insurer: Some("Sample Life".into()),
                kind: Some("Life".into()),
                policy_number: Some("LF-777".into()),
                phone: Some("555-0161".into()),
            }],
            where_originals: Some("Fireproof box under the bed".into()),
            where_copies: Some("With Rosa".into()),
            digital_backup: Some("Encrypted drive in the go-bag".into()),
        }),
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
        zip_centroid: Some(LatLon {
            lat: 39.9364,
            lon: -75.1525,
        }),
        nca_region: "northeast".into(),
        coastal: false,
        tsunami_zone: false,
        facility_flags: FacilityFlags {
            nuclear_plant_within_16km: false,
            nuclear_plant_within_80km: true,
            hazmat_facilities_within_5km: 12,
        },
        data_note: Some("your county; tract-level data not yet loaded".into()),
        exposure: exposure(),
    }
}

fn sourced<T>(value: T, source: &str) -> Option<Sourced<T>> {
    Some(Sourced {
        value,
        source: source.into(),
    })
}

/// Every exposure field filled in.
pub fn exposure() -> Exposure {
    Exposure {
        strategic_class: sourced("C1".to_owned(), "rr_strategic_sites"),
        strategic_km: sourced(42.5, "rr_strategic_sites"),
        surge_cat3_share: sourced(0.0, "nhc_surge_maps"),
        surge_proxy_class: sourced("moderate".to_owned(), "nhc_surge_maps"),
        smoke_days_35: sourced(1.4, "epa_aqs_pm25"),
        leveed_pop_share: sourced(0.02, "usace_nld"),
        dams_high_within_10km: sourced(1, "usace_nid"),
        karst_share: sourced(0.1, "usgs_karst"),
        landslide_susceptible_share: sourced(0.05, "usgs_landslide_susceptibility"),
        water_system_flag: sourced(0.0, "epa_sdwis"),
        geomag_factor: sourced(0.7, "nerc_tpl007_benchmark"),
        uasi_share: sourced(0.03, "fema_uasi_fy2025"),
        eviction_rate: sourced(0.06, "eviction_lab"),
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
        requires: vec!["water_container".into()],
        decision: !done,
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
        stress_test: relief.as_ref().map(|_| StressTest {
            event: "Hurricane Isaias".into(),
            date: date(2020, 8, 4),
            region: "the Northeast".into(),
            share_out_at_days: vec![(1.0, 0.4), (3.0, 0.1), (7.0, 0.01)],
            covered_by_target: true,
            sources: vec!["eagle_i_outages".into()],
        }),
        relief,
    }
}

fn hazard(id: HazardId, display: HazardDisplay) -> HazardProfile {
    let rare = display == HazardDisplay::RareCatastrophic;
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
        family: id.family().map(str::to_owned),
        sub_causes: vec![SubCause {
            id: if rare { "hemp" } else { "freezing_rain" }.into(),
            name: if rare {
                "Electrical effects (EMP)"
            } else {
                "Freezing rain"
            }
            .into(),
            note: "A named cause inside this hazard.".into(),
            rate_range: rare.then_some([1.0e-5, 2.0e-3]),
            sources: vec!["epri_2019_hemp".into()],
        }],
        location_factor: rare.then(|| LocationFactor {
            class: "C1".into(),
            label: "in one of the ten largest metro areas".into(),
            multiplier: [0.3, 0.6, 0.9],
            sources: vec!["rr_strategic_sites".into()],
        }),
        range_only: rare,
        anchor_sentence: rare
            .then(|| "Less likely than a house fire (about 5 in 100 for you).".into()),
        if_it_reaches_you: rare.then(|| "Life-threatening.".into()),
        what_it_changes: rare.then(|| "One free step: pick your shelter spot.".into()),
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
            minimum_done_month: Some(2),
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
            first_milestone: Some(SavingsMilestone {
                months: 0.12,
                usd: 500.0,
                by_month: 5,
            }),
            minimum_kit: true,
            long_horizon: vec![plan_item(false)],
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
        binder: binder(),
        prepare_markdown: "# Prepare\n".into(),
        provenance: vec![citation()],
        recovery: RecoveryInfo {
            county_declarations_5yr: Some(3),
            sources: vec!["openfema_declarations".into()],
        },
    }
}

fn t(text: &str) -> binder::Inline {
    binder::Inline::T(text.into())
}

/// A binder that uses every block and inline variant and fills every optional field, and passes
/// [`Binder::check`] (contract v3).
pub fn binder() -> Binder {
    use binder::*;
    let home = Page {
        id: "home".into(),
        title: "Home".into(),
        kind: PageKind::Home,
        fit: Fit::One,
        blocks: vec![
            Block::Heading(Heading {
                level: 2,
                text: "Shut-offs".into(),
            }),
            Block::Fields(vec![
                FieldRow {
                    label: "Gas shut-off".into(),
                    value: Some("Beside the meter at the back".into()),
                    lines: 1,
                },
                FieldRow {
                    label: "Water shut-off".into(),
                    value: None,
                    lines: 2,
                },
            ]),
            Block::MapSlot(MapSlot {
                id: "neighbourhood".into(),
                kind: MapSlotKind::Neighbourhood,
                caption: "Your neighbourhood".into(),
            }),
        ],
    };
    let fire = Page {
        id: "check_house_fire".into(),
        title: "House fire".into(),
        kind: PageKind::Checklist,
        fit: Fit::One,
        blocks: vec![
            Block::Para(vec![
                t("Here: about 5 in 100 households like yours over ten years."),
                Inline::Cite(vec![1]),
            ]),
            Block::Steps(vec![Step {
                text: vec![
                    Inline::B("Get out.".into()),
                    t(" Leave by the nearest safe way."),
                    Inline::Cite(vec![1]),
                ],
                memory: true,
            }]),
            Block::Decision(Decision {
                question: "Leave or stay?".into(),
                branches: vec![Branch {
                    when: vec![t("Leave if the way out is clear.")],
                    then: vec![
                        t("Meet at "),
                        Inline::Blank(24),
                        t(". "),
                        Inline::Link(Link {
                            to: "home".into(),
                            text: "Tab 3, Home".into(),
                        }),
                    ],
                    go_to: Some("home".into()),
                }],
            }),
            Block::Bullets(vec![vec![t("Do not go back inside.")]]),
            Block::Numbered(vec![vec![t("Call 911 from outside.")]]),
            Block::Callout(Callout {
                kind: CalloutKind::Stop,
                title: Some("Never".into()),
                blocks: vec![Block::Para(vec![t("Do not hide from firefighters.")])],
            }),
            Block::page_break(),
        ],
    };
    let cards = Page {
        id: "wallet_cards".into(),
        title: "Wallet cards".into(),
        kind: PageKind::WalletCards,
        fit: Fit::Flow,
        blocks: vec![
            Block::Cards(vec![Card {
                title: "Ana".into(),
                lines: vec![vec![t("Out-of-area contact: Rosa, 555-0100")]],
            }]),
            Block::Table(Table {
                header: vec!["Name".into(), "Phone".into()],
                rows: vec![vec![vec![t("Rosa")], vec![t("555-0100")]]],
            }),
            Block::Log(Log {
                columns: vec!["Date".into(), "Who".into()],
                rows: 12,
            }),
        ],
    };
    Binder {
        title: "Your emergency binder".into(),
        generated_on: date(2026, 10, 1),
        household: "2 adults".into(),
        location: "Philadelphia County, Pennsylvania (ZIP code 19147)".into(),
        status_line: "Ready Reckoner is an independent, open-source planning aid.".into(),
        review_by: date(2027, 10, 1),
        parts: vec![
            Part {
                id: "people".into(),
                tab: 2,
                title: "People".into(),
                short_title: "People".into(),
                pages: vec![cards],
            },
            Part {
                id: "home_places".into(),
                tab: 3,
                title: "Home and places".into(),
                short_title: "Home, places".into(),
                pages: vec![home],
            },
            Part {
                id: "check_now".into(),
                tab: 6,
                title: "Checklists: happening now".into(),
                short_title: "Now".into(),
                pages: vec![fire],
            },
        ],
        sources: vec![SourceEntry {
            n: 1,
            title: "Home Fires".into(),
            publisher: "FEMA / Ready.gov".into(),
            year: Some(2026),
            url: Some("https://www.ready.gov/home-fires".into()),
            expert: false,
        }],
        credits: vec!["Uses National Risk Index data; not endorsed by FEMA.".into()],
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
        requires: vec!["water_container".into()],
        readiness_share: Some(0.25),
        alternative_group: Some("stored_water".into()),
        decision: true,
        long_horizon: true,
        season: Some(Season::Summer),
        test_interval_months: Some(6),
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
            kind: Some(GuidanceKind::Bucket),
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
        validation: ValidationSummary {
            events_tested: 22,
            covered: 6,
            partial: 5,
            short: 10,
            not_modelled: 1,
            data_pack: "core-2026.09".into(),
            url_anchor: "#/validation".into(),
        },
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
