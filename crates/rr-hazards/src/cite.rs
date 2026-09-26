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

/// Ready.gov, Floods.
pub const READY_FLOODS: &str = "ready_gov_floods";
/// Ready.gov, Winter Weather.
pub const READY_WINTER: &str = "ready_gov_winter";
/// Ready.gov, Hurricanes.
pub const READY_HURRICANES: &str = "ready_gov_hurricanes";
/// Ready.gov, Earthquakes.
pub const READY_EARTHQUAKES: &str = "ready_gov_earthquakes";
/// Ready.gov, Tsunamis.
pub const READY_TSUNAMIS: &str = "ready_gov_tsunamis";
/// Ready.gov, Chemical Emergencies.
pub const READY_CHEMICAL: &str = "ready_gov_chemical";
/// Ready.gov, Pandemics.
pub const READY_PANDEMIC: &str = "ready_gov_pandemic";
/// Ready.gov, Cybersecurity.
pub const READY_CYBER: &str = "ready_gov_cybersecurity";
/// Ready.gov, Power Outages.
pub const READY_POWER: &str = "ready_gov_power_outages";
/// Ready.gov, Home Fires.
pub const READY_HOME_FIRES: &str = "ready_gov_home_fires";
/// EPA: Asheville's system-wide boil notice after Helene lasted about seven weeks.
pub const EPA_ASHEVILLE: &str = "epa_asheville_boil_notice_2024";
/// CDC, carbon monoxide poisoning basics.
pub const CDC_CO_BASICS: &str = "cdc_co_basics";
/// FTC, avoiding scams after a disaster.
pub const FTC_DISASTER_SCAMS: &str = "ftc_disaster_scams";
/// CDC, pregnancy and emergencies.
pub const CDC_PREGNANCY: &str = "cdc_pregnancy_emergency";
/// USGS Fact Sheet 2016-3020 (UCERF3): the Bay Area earthquake outlook.
pub const USGS_BAY_AREA_2016: &str = "usgs_bay_area_outlook_2016";

// ---------------------------------------------------------------------------------------------
// Contract v2 (v0.2.0): the data pack's exposure columns, the new ranked hazards, the rare
// families and the new scenarios. Ids not yet in `content/citations.toml` are requested in
// `docs/CITATION_IDS.md` ("Requested by hazards for v0.2.0").
// ---------------------------------------------------------------------------------------------

/// Ready Reckoner's strategic-site table (`data/core/strategic_sites.toml`): the class rules,
/// every site with one public source, the metros, ports and refineries (compiled 2026-09-26).
pub const STRATEGIC_SITES: &str = "rr_strategic_sites";
/// FEMA, *Protection in the Nuclear Age* (1985): a place designated a risk area "does not mean
/// that it will be attacked"; the public precedent for the "Why here" sentence.
pub const FEMA_PNA_1985: &str = "fema_protection_nuclear_age_1985";
/// FEMA, Nuclear Attack Planning Base 1990 (1987, released 2005): county blast and fallout
/// classes, the method precedent.
pub const FEMA_NAPB90: &str = "fema_napb90";
/// Philippe (2023), Scientific American and Princeton's *The Missiles on our Land*: fallout from
/// an attack on the missile silos, the calibration of the downwind class.
pub const PHILIPPE_2023: &str = "philippe_2023_icbm_fallout";
/// FEMA FY2026 Homeland Security Grant Program notice, Appendix I.B: UASI allocations by urban
/// area.
pub const FEMA_UASI_FY2026: &str = "fema_hsgp_fy2026";
/// NERC TPL-007 benchmark geomagnetic disturbance event: the scaling factor by geomagnetic
/// latitude.
pub const NERC_TPL007: &str = "nerc_tpl007_gmd";
/// IGRF-14 (IAGA, NOAA NCEI copy): the dipole behind each county's geomagnetic latitude.
pub const IGRF14: &str = "igrf14_coefficients";
/// NOAA Hazard Mapping System smoke polygons: which days count as smoke days.
pub const NOAA_HMS: &str = "noaa_hms_smoke";
/// EPA AirData daily PM2.5 by county: smoke days at 35.5 µg/m³ or more.
pub const EPA_AQS: &str = "epa_aqs_daily_pm25";
/// USGS Open-File Report 2014-1156, Karst in the United States: the share of a county on karst.
pub const USGS_KARST: &str = "usgs_karst_2014";
/// USACE National Inventory of Dams: high-hazard dams, their condition and downstream town.
pub const USACE_NID: &str = "usace_nid";
/// USACE National Levee Database: people behind levees and USACE's levee risk rating.
pub const USACE_NLD: &str = "usace_nld";
/// Association of State Dam Safety Officials: estimated rates of dam failure (173 failures and
/// 587 incidents, 2005 to mid-2013).
pub const ASDSO: &str = "asdso_dam_failures";
/// Eviction Lab (Princeton): eviction filings and judgments by county (ODC-BY).
pub const EVICTION_LAB: &str = "eviction_lab_county_estimates";
/// Insurance Information Institute, facts and statistics on homeowners claims (ISO data): water
/// damage and freezing, about 1 in 67 insured homes a year.
pub const III_WATER: &str = "iii_water_damage";
/// FCC, report on the AT&T wireless outage of 22 February 2024.
pub const FCC_ATT_2024: &str = "fcc_att_outage_2024";
/// ASHP drug shortage statistics: 323 active shortages at the start of 2024.
pub const ASHP_SHORTAGES: &str = "ashp_shortages";
/// openFDA drug shortages: medicines FDA lists as currently short (CC0).
pub const OPENFDA_SHORTAGES: &str = "openfda_drug_shortages";
/// Congressional Research Service RS20348, Federal Funding Gaps: A Brief Overview.
pub const CRS_FUNDING_GAPS: &str = "crs_rs20348_funding_gaps";
/// Reporting on the November 2025 lapse in SNAP benefits during the 43-day shutdown.
pub const SNAP_LAPSE_2025: &str = "snap_lapse_2025";
/// CSIS, the US terrorism dataset (attacks and plots, 1994–2025).
pub const CSIS_TERRORISM: &str = "csis_terrorism_2025";
/// FBI, Crime in the United States: arrests by age and sex (Crime Data Explorer).
pub const FBI_ARRESTS: &str = "fbi_cde_arrests";
/// FBI, Active Shooter Incidents in the United States in 2024.
pub const FBI_ACTIVE_SHOOTER: &str = "fbi_active_shooter_2024";
/// START, Profiles of Incidents involving CBRN and Non-state Actors (POICN).
pub const START_POICN: &str = "start_poicn";
/// Karger et al. (2023), the Existential Risk Persuasion Tournament (XPT).
pub const XPT_2023: &str = "xpt_2023_karger";
/// Rethink Priorities (Rodriguez 2019): how likely is a nuclear exchange between the US and
/// Russia.
pub const RP_2019: &str = "rp_2019_nuclear";
/// Barrett, Baum and Hostetler (2013): inadvertent nuclear war between the US and Russia.
pub const BARRETT_2013: &str = "barrett_2013_inadvertent";
/// FEMA, Nuclear Detonation Response Guidance: Planning for the First 72 Hours (2023).
pub const FEMA_NUCLEAR_72H: &str = "fema_nuclear_72h_2023";
/// EPRI (2019), High-Altitude Electromagnetic Pulse and the Bulk Power System.
pub const EPRI_HEMP: &str = "epri_2019_hemp";
/// Riley (2012), Space Weather: the chance of another Carrington event.
pub const RILEY_2012: &str = "riley_2012_carrington";
/// Moriña et al. (2019), Scientific Reports: the probability of a Carrington-like event.
pub const MORINA_2019: &str = "morina_2019_carrington";
/// Love (USGS): lognormality of historical magnetic-storm intensity.
pub const LOVE_CARRINGTON: &str = "love_carrington";
/// Lloyd's and AER (2013), Solar storm risk to the North American electric grid.
pub const LLOYDS_2013: &str = "lloyds_2013_solar";
/// Cassidy and Mani (2022), Nature: huge volcanic eruptions, a one-in-six chance this century.
pub const CASSIDY_MANI_2022: &str = "cassidy_mani_2022";
/// USGS Yellowstone Volcano Observatory: about 1 in 730,000 a year.
pub const USGS_YVO: &str = "usgs_yvo";
/// NASA (2019), Tunguska Revisited: Tunguska-class impacts on the order of millennia.
pub const NASA_TUNGUSKA: &str = "nasa_tunguska_2019";
/// FDIC failed-bank list, 1934 to date.
pub const FDIC_FAILURES: &str = "fdic_failed_banks";
/// NPR (2018): 11 months after Hurricane Maria, every customer's power was back (328 days).
pub const NPR_MARIA: &str = "npr_maria_2018";
/// Stone et al. (2023), Environmental Science & Technology: a blackout during a heat wave in
/// Atlanta, Detroit and Phoenix.
pub const STONE_2023: &str = "stone_2023_heat_blackout";
/// Working Group on Utah Earthquake Probabilities (2016): earthquake forecast for the Wasatch
/// Front.
pub const UTAH_WGUEP: &str = "utah_wguep_2016";
/// USGS, the Seattle fault zone: its earthquakes and their chance.
pub const USGS_SEATTLE_FAULT: &str = "usgs_seattle_fault";
/// PNNL event-correlated outage dataset (DOE OE-417 reports linked to EAGLE-I outages; CC BY 4.0).
pub const PNNL_OE417: &str = "pnnl_oe417_linkage";
/// CDC MMWR QuickStats: about 374 unintentional carbon monoxide deaths a year (2010–2015).
pub const CDC_CO: &str = "cdc_co_quickstats";
/// FTC Consumer Sentinel Network Data Book 2024: identity-theft reports by state.
pub const FTC_SENTINEL: &str = "ftc_sentinel_2024";
/// USGS, the Barry Arm landslide and tsunami hazard in Prince William Sound.
pub const USGS_BARRY_ARM: &str = "usgs_barry_arm";
/// CDC, H5N1 bird flu: current situation summary.
pub const CDC_H5N1: &str = "cdc_h5n1_situation";
/// Mid-Atlantic IV-fluid shortage after Hurricane Helene (PMC commentary, 2024).
pub const IV_FLUIDS_2024: &str = "iv_fluids_helene_2024";

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
    READY_FLOODS,
    READY_WINTER,
    READY_HURRICANES,
    READY_EARTHQUAKES,
    READY_TSUNAMIS,
    READY_CHEMICAL,
    READY_PANDEMIC,
    READY_CYBER,
    READY_POWER,
    READY_HOME_FIRES,
    EPA_ASHEVILLE,
    CDC_CO_BASICS,
    FTC_DISASTER_SCAMS,
    CDC_PREGNANCY,
    USGS_BAY_AREA_2016,
    STRATEGIC_SITES,
    FEMA_PNA_1985,
    FEMA_NAPB90,
    PHILIPPE_2023,
    FEMA_UASI_FY2026,
    NERC_TPL007,
    IGRF14,
    NOAA_HMS,
    EPA_AQS,
    USGS_KARST,
    USACE_NID,
    USACE_NLD,
    ASDSO,
    EVICTION_LAB,
    III_WATER,
    FCC_ATT_2024,
    ASHP_SHORTAGES,
    OPENFDA_SHORTAGES,
    CRS_FUNDING_GAPS,
    SNAP_LAPSE_2025,
    CSIS_TERRORISM,
    FBI_ARRESTS,
    FBI_ACTIVE_SHOOTER,
    START_POICN,
    XPT_2023,
    RP_2019,
    BARRETT_2013,
    FEMA_NUCLEAR_72H,
    EPRI_HEMP,
    RILEY_2012,
    MORINA_2019,
    LOVE_CARRINGTON,
    LLOYDS_2013,
    CASSIDY_MANI_2022,
    USGS_YVO,
    NASA_TUNGUSKA,
    FDIC_FAILURES,
    NPR_MARIA,
    STONE_2023,
    UTAH_WGUEP,
    USGS_SEATTLE_FAULT,
    PNNL_OE417,
    CDC_CO,
    FTC_SENTINEL,
    USGS_BARRY_ARM,
    CDC_H5N1,
    IV_FLUIDS_2024,
];
