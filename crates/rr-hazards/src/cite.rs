//! Citation ids this crate attaches to rates and profiles.
//!
//! Each id must resolve to an entry in `content/citations.toml` (owned by the content
//! workstream). The ids and what they stand for are listed in `docs/CITATION_IDS.md` with the
//! status "requested by hazards"; a test checks that every id here appears in that table.

/// FEMA National Risk Index v1.20 (December 2025), county table: annualised frequencies,
/// exposure, historic loss ratios and expected annual losses.
pub const NRI: &str = "fema_nri_v120";
/// USGS National Seismic Hazard Model 2023 (revised 2026): yearly chance of shaking at the county
/// centre.
pub const USGS_NSHM: &str = "usgs_nshm_2023";
/// ORNL EAGLE-I county outage records 2014–2025 (Brelsford et al. 2024; CC BY 4.0).
pub const EAGLE_I: &str = "ornl_eagle_i";
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
pub const NCA5: &str = "nca5_2023";
/// Goldfinger et al. (2012), USGS Professional Paper 1661-F, as reported by Oregon State
/// University: 40 % chance of a major Cascadia earthquake near Coos Bay in 50 years; 19
/// full-margin and 22 southern-only ruptures in 10,000 years.
pub const CASCADIA_2012: &str = "goldfinger_2012_cascadia";
/// The Oregon Resilience Plan (2013): prepare for a minimum of two weeks.
pub const ORP_2013: &str = "orp_2013";
/// Washington Emergency Management Division, "2 Weeks Ready".
pub const WA_TWO_WEEKS: &str = "wa_emd_2_weeks_ready";
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
pub const USFA_FIRES: &str = "usfa_residential_fires_2023";
/// US Census Bureau, Current Population Survey table HH-1: 131,434,000 households in 2023.
pub const CENSUS_HH1: &str = "census_cps_hh1";
/// BLS, Work Experience of the Population, 2024: 8.3 % of people who worked or looked for work
/// were unemployed at some point in the year.
pub const BLS_WORK_EXPERIENCE: &str = "bls_work_experience_2024";
/// BLS JOLTS layoffs and discharges rate: 1.117 % of workers per month on average in 2025.
pub const BLS_JOLTS: &str = "bls_jolts_2025";
/// CDC NCHS FastStats, emergency department visits (NHAMCS 2022): 47.3 per 100 people a year.
pub const NHAMCS_ED: &str = "cdc_nhamcs_ed_2022";
/// NHTSA Traffic Safety Facts, 2023: 6.14 million police-reported crashes, 2.44 million injured.
pub const NHTSA_CRASHES: &str = "nhtsa_crashes_2023";
/// CDC NCHS FastStats, accidental injury deaths: 58.1 per 100,000 people (2024).
pub const NCHS_ACCIDENTS: &str = "nchs_accidental_injury_2024";
/// Social Security Administration disability facts: more than 1 in 4 of today's 20-year-olds will
/// become disabled before reaching full retirement age.
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
pub const EPA_BWA: &str = "epa_bwa_rtc_2024";
/// Shaffer et al. (2026), Environmental Science & Technology: Texas boil water notices 2010–2022.
pub const TEXAS_BWN: &str = "shaffer_2026_texas_bwn";
/// Mell et al. (2017), JAMA Surgery: emergency medical services take longer to reach rural
/// addresses.
pub const MELL_2017_EMS: &str = "mell_2017_ems";
/// Ready Reckoner risk-model research report (2026), expert estimates tagged PRIOR (§6.2 societal
/// rates, §2.7 household modifiers). A `prior = true` citation.
pub const RR_PRIORS: &str = "rr_priors";
/// Ready Reckoner `docs/RISK_MODEL.md` § "Hazard rates": the footprint, episode-length and
/// household-modifier estimates this crate adds. A `prior = true` citation.
pub const RR_HAZARD_PRIORS: &str = "rr_hazard_priors";

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
    CASCADIA_2012,
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
    RR_HAZARD_PRIORS,
];
