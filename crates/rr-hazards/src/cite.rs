//! Citation ids this crate attaches to rates and profiles.
//!
//! Each id must resolve to an entry in `content/citations.toml` (owned by the content
//! workstream). Most are already defined there; the ones this crate adds are listed in
//! `docs/CITATION_IDS.md` as "requested by hazards". A test checks that every id here is either
//! in `content/citations.toml` or in that table.

/// FEMA National Risk Index v1.20 (December 2025), county table: annualised frequencies,
/// exposure, historic loss ratios and expected annual losses.
pub const NRI: &str = "fema_nri_v120";
/// USGS National Seismic Hazard Model 2023 (revised 2026): yearly chance of shaking at the county
/// centre.
pub const USGS_NSHM: &str = "usgs_nshm_2023";
/// ORNL EAGLE-I county outage records 2014–2025 (Brelsford et al. 2024; CC BY 4.0).
pub const EAGLE_I: &str = "ornl_eagle_i_outages";
/// Do et al. (2023), Nature Communications 14:2470: 62.1 % of county outages of 8 hours or more
/// coincided with extreme weather.
pub const DO_2023: &str = "do_2023_outages";
/// NOAA NCEI Storm Events Database (per-county episode rates, when the data pack has them).
pub const STORM_EVENTS: &str = "noaa_storm_events";
/// OpenFEMA NFIP residential penetration rates and claims (v3): share of homes in the Special
/// Flood Hazard Area, claims per policy.
pub const NFIP: &str = "openfema_nfip";
/// FEMA flood zone definitions: the Special Flood Hazard Area is the land with a 1 % or greater
/// chance of flooding in any year.
pub const FEMA_FLOOD_ZONES: &str = "fema_flood_zones";
/// Climate Mapping for Resilience and Adaptation (CMRA), 2025 county projections (LOCA2,
/// SSP2-4.5 and SSP5-8.5, 2036–2065).
pub const CMRA: &str = "cmra_2025";
/// Fifth National Climate Assessment (2023), chapter 2: stronger tropical cyclones.
pub const NCA5: &str = "nca5_climate_trends";
/// Fifth National Climate Assessment Interactive Atlas (CC BY 4.0): county changes at global
/// warming levels (hot days, cold nights, extreme rain), the source of the data pack's ratios.
pub const NCA5_ATLAS: &str = "nca5_atlas";
/// Goldfinger et al. (2012), USGS Professional Paper 1661-F, as reported by Oregon State
/// University: 40 % chance of a major Cascadia earthquake near Coos Bay in 50 years; 19
/// full-margin and 22 southern-only ruptures in 10,000 years.
pub const CASCADIA_2012: &str = "osu_cascadia_2012";
/// Goldfinger et al. (2012), USGS Professional Paper 1661-F: the Cascadia turbidite record.
pub const CASCADIA_PP1661F: &str = "usgs_pp1661f_cascadia";
/// Oregon Department of Emergency Management, "2 Weeks Ready": a plan and supplies to get through
/// at least two weeks after a disaster.
pub const OREGON_TWO_WEEKS: &str = "oregon_2_weeks_ready";
/// The Oregon Resilience Plan (2013): prepare for a minimum of two weeks; coast and valley
/// restoration times after a Cascadia earthquake.
pub const ORP_2013: &str = "oregon_resilience_plan_2013";
/// Washington Emergency Management Division, "Prepare in a Year": one hour a month for a year to
/// become two weeks ready.
pub const WA_TWO_WEEKS: &str = "washington_prepare_in_a_year";
/// DOGAMI Oregon Tsunami Clearinghouse: waves arrive 15–20 minutes after a local earthquake.
pub const DOGAMI_TSUNAMI: &str = "dogami_tsunami_faq";
/// USGS Fact Sheet 2015-3009 (UCERF3): 33 % chance of magnitude 6.7 or more on the
/// Hayward–Rodgers Creek fault in 30 years.
pub const UCERF3: &str = "usgs_ucerf3_2015";
/// USGS New Madrid Seismic Zone: 7–10 % chance of a repeat of the 1811–12 earthquakes in 50 years.
pub const NEW_MADRID: &str = "usgs_new_madrid";
/// NOAA hurricane database (HURDAT2) and the list of US landfalling hurricanes: about a third of
/// landfalling hurricanes are major (Category 3 or stronger).
pub const HURDAT2: &str = "noaa_hurdat2";
/// US Fire Administration, residential fire statistics, 2023: 344,600 fires, $11.27 billion loss.
pub const USFA_FIRES: &str = "usfa_residential_fires";
/// US Census Bureau, Current Population Survey table HH-1: 131,434,000 households in 2023.
pub const CENSUS_HH1: &str = "census_households_cps";
/// BLS, Work Experience of the Population, 2024: 8.3 % of people who worked or looked for work
/// were unemployed at some point in the year.
pub const BLS_WORK_EXPERIENCE: &str = "bls_work_experience_2024";
/// BLS JOLTS layoffs and discharges rate: 1.117 % of workers per month on average in 2025.
pub const BLS_JOLTS: &str = "bls_jolts_layoffs";
/// CDC NCHS FastStats, emergency department visits (NHAMCS 2022): 47.3 per 100 people a year.
pub const NHAMCS_ED: &str = "cdc_nchs_ed_visits";
/// NHTSA Traffic Safety Facts, 2023: 6.14 million police-reported crashes, 2.44 million injured.
pub const NHTSA_CRASHES: &str = "nhtsa_crashes_2023";
/// CDC NCHS FastStats, accidental injury deaths: 58.1 per 100,000 people (2024).
pub const NCHS_ACCIDENTS: &str = "nchs_accidental_injury_2024";
/// Social Security Administration disability facts: a 20-year-old worker has a 1-in-4 chance of
/// becoming disabled before reaching full retirement age.
pub const SSA_DISABILITY: &str = "ssa_disability_facts";
/// CDC pandemic history (1918, 1957, 1968, 2009) plus COVID-19: five pandemics in 108 years.
pub const CDC_PANDEMICS: &str = "cdc_pandemic_history";
/// Marani et al. (2021), PNAS: a COVID-19-intensity pandemic recurs about every 209 years.
pub const MARANI_2021: &str = "marani_2021_pandemics";
/// Forecasting Research Institute (2024): chance of a nuclear catastrophe by 2045, experts 5 %,
/// superforecasters 1 %.
pub const FRI_NUCLEAR: &str = "fri_nuclear_risk_2024";
/// Ready.gov, Nuclear Explosion: get inside, stay inside, stay tuned.
pub const READY_NUCLEAR: &str = "ready_gov_nuclear";
/// FEMA Operating Nuclear Power Plant Sites and the 10-mile and 50-mile planning zones.
pub const FEMA_NUCLEAR_SITES: &str = "fema_nuclear_sites";
/// EPA Toxics Release Inventory, 2024 facilities.
pub const EPA_TRI: &str = "epa_tri_2024";
/// EPA (2024), Report to Congress on boil water advisories: no national tracking; 80 % of the
/// advisories located were for main breaks and pressure loss.
pub const EPA_BWA: &str = "epa_boil_water_report_2024";
/// Shaffer et al. (2026), Environmental Science & Technology: Texas boil water notices 2010–2022.
pub const TEXAS_BWN: &str = "shaffer_2026_texas_boil_notices";
/// Mell et al. (2017), JAMA Surgery: emergency medical services take longer to reach rural
/// addresses.
pub const MELL_2017_EMS: &str = "mell_2017_ems_response";
/// Ready Reckoner expert estimates for hazard rates, each listed with its reasoning in
/// `docs/RISK_MODEL.md` § "Hazard rates": the research priors (risk-model §6.2, §2.7) and the
/// footprint, episode-length and household-modifier estimates this crate adds. A `prior = true`
/// citation.
pub const RR_PRIORS: &str = "rr_risk_model_priors";
/// The same citation as [`RR_PRIORS`], named for the estimates this crate adds.
pub const RR_HAZARD_PRIORS: &str = RR_PRIORS;

/// Every id above, for tests.
pub const ALL: &[&str] = &[
    NRI,
    USGS_NSHM,
    EAGLE_I,
    DO_2023,
    STORM_EVENTS,
    NFIP,
    FEMA_FLOOD_ZONES,
    CMRA,
    NCA5,
    NCA5_ATLAS,
    CASCADIA_2012,
    CASCADIA_PP1661F,
    OREGON_TWO_WEEKS,
    ORP_2013,
    WA_TWO_WEEKS,
    DOGAMI_TSUNAMI,
    UCERF3,
    NEW_MADRID,
    HURDAT2,
    USFA_FIRES,
    CENSUS_HH1,
    BLS_WORK_EXPERIENCE,
    BLS_JOLTS,
    NHAMCS_ED,
    NHTSA_CRASHES,
    NCHS_ACCIDENTS,
    SSA_DISABILITY,
    CDC_PANDEMICS,
    MARANI_2021,
    FRI_NUCLEAR,
    READY_NUCLEAR,
    FEMA_NUCLEAR_SITES,
    EPA_TRI,
    EPA_BWA,
    TEXAS_BWN,
    MELL_2017_EMS,
    RR_PRIORS,
];
