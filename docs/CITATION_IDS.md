# Citation ids

Ids that engine crates attach to numbers. Each must resolve to an entry in
`content/citations.toml` (owned by the content workstream; rules in `docs/CONTENT_STANDARDS.md`
§2). `cargo test -p rr-hazards` checks that every id the hazards crate can emit is in the registry
or listed here; `cargo test -p rr-content` checks that every id named in this file is in the
registry.

**Need an id that is not here?** Add a row under "Requested" at the bottom with the title,
publisher, year and URL and what the number is used for, and mark figures you took from memory with
"confirm". The content workstream writes the entry and checks the figure against the source.

## Used by hazards

### From the content registry

| id | What rr-hazards uses it for |
| --- | --- |
| `fema_nri_v120` | county annualised frequencies, exposure, loss ratios, expected annual losses |
| `ornl_eagle_i_outages` | the windstorm outage floor (household outage-event rate) |
| `do_2023_outages` | share of county outages that coincide with extreme weather (62.1 %) |
| `cmra_2025` | 2050 climate ratios from CMRA variables and county counts |
| `nca5_climate_trends` | stronger tropical cyclones (hurricane intensity multiplier) |
| `nca5_atlas` | 2050 climate ratios from the NCA5 Atlas variables in the data pack |
| `osu_cascadia_2012` | 40 % chance of a major Cascadia earthquake near Coos Bay in 50 years; 19 full-margin and 22 southern-only ruptures in 10,000 years |
| `usgs_pp1661f_cascadia` | the Cascadia turbidite record behind those figures |
| `oregon_2_weeks_ready` | why `cascadia_m9` is on by default in Oregon |
| `oregon_resilience_plan_2013` | two weeks minimum; coast and valley Cascadia consequences |
| `washington_prepare_in_a_year` | why `cascadia_m9` is on by default in Washington (two weeks ready) |
| `dogami_tsunami_faq` | a local tsunami arrives 15–20 minutes after the earthquake |
| `usfa_residential_fires` | 344,600 residential building fires in 2023 (house fire rate); $11.27 billion loss (severity) |
| `census_households_cps` | 131,434,000 households in 2023 (house fire denominator) |
| `bls_work_experience_2024` | 8.3 % of labour-force participants unemployed at some point in 2024 (job-loss spell rate) |
| `bls_jolts_layoffs` | layoffs and discharges 1.117 % a month (upper end of the job-loss range) |
| `cdc_nchs_ed_visits` | 47.3 emergency department visits per 100 people (medical emergency) |
| `nhtsa_crashes_2023` | 6.14 million police-reported crashes (vehicle stranding) |
| `marani_2021_pandemics` | a COVID-19-intensity pandemic about every 209 years (pandemic range) |
| `fri_nuclear_risk_2024` | 1 in 2,000 to 1 in 400 a year, worldwide nuclear catastrophe |
| `ready_gov_nuclear` | get inside, stay inside, stay tuned (the nuclear card's sentence) |
| `epa_boil_water_report_2024` | no national boil-water tracking; causes of advisories (local utility outage) |
| `shaffer_2026_texas_boil_notices` | Texas boil-water notice rates, when a county boil-notice rate is used |
| `mell_2017_ems_response` | ambulances take longer to reach rural addresses (the rural note) |
| `rr_risk_model_priors` | every expert estimate in `docs/RISK_MODEL.md` § "Hazard rates" (prior = true) |

### Added for hazards on 2026-09-25, with the flagged figures checked

| id | Source | Used for | Check result |
| --- | --- | --- | --- |
| `usgs_nshm_2023` | USGS 2023 National Seismic Hazard Model (2023.R2) | yearly chance of shaking of 0.1 g or more at the county centre | dataset DOI; no figure to check |
| `noaa_storm_events` | NOAA NCEI Storm Events Database | county episode rates | dataset; no figure to check |
| `openfema_nfip` | OpenFEMA NFIP penetration rates and redacted claims | homes in the flood zone; claims per policy | dataset. **Its terms require a statement like NRI's:** use `openfema_disclaimer` on the About screen |
| `fema_flood_zones` | FEMA glossary, Flood Zones | the 1 %-a-year chance for mapped flood zones | confirmed: the glossary defines the Special Flood Hazard Area by the flood with a 1-percent chance in any given year (read through a summarizing fetch, so no quote is stored) |
| `usgs_ucerf3_2015` | UCERF3 fact sheet 2015–3009 | the California forecast model | the fact sheet page does not state the Hayward figure |
| `usgs_bay_area_outlook_2016` | USGS fact sheet 2016–3020, based on UCERF3 | **cite this for the 33 %** | confirmed, exact quote stored: 33 % chance of magnitude 6.7 or larger on the Hayward or Rodgers Creek faults in 2014–2043 (22 % on the San Andreas) |
| `usgs_new_madrid` | USGS Earthquake Hazards Program, New Madrid Seismic Zone | `new_madrid_m7` | confirmed, exact quote stored: about 7–10 % chance of a repeat of the 1811–1812 earthquakes in 50 years (25–40 % for magnitude 6.0 or larger) |
| `noaa_hurdat2` | NOAA AOML, Continental US Hurricane Impacts/Landfalls 1851–2025 (from HURDAT2) | share of hurricanes that are major | confirmed from the table: 95 of 299 US landfalls in 1851–2023 were category 3 or higher (32 %); 71 of 206 since 1900 (34 %) |
| `nchs_accidental_injury_2024` | CDC NCHS FastStats | 58.1 unintentional-injury deaths per 100,000 | confirmed, exact quote stored (197,449 deaths in 2024) |
| `ssa_disability_facts` | Social Security Administration, Disability Benefits (publication 05-10029) | disability onset | confirmed with a different wording and URL than requested: "a 20-year-old worker has a 1-in-4 chance of developing a disability before reaching full retirement age" (exact quote stored) |
| `cdc_pandemic_history` | CDC archive, 1918 pandemic and later pandemics | five onsets in 108 years | the 1918 page's figures confirmed (about 675,000 US deaths); the onset count is the hazards derivation |
| `fema_nuclear_sites` | FEMA Operating Nuclear Power Plant Sites | plant within 16 or 80 km | dataset |
| `epa_tri_2024` | EPA Toxics Release Inventory | TRI facilities in the county | dataset |
| `bjs_criminal_victimization_2023` | BJS, Criminal Victimization 2023 | **can replace the burglary prior** | new: 1.01 % of households were victims of burglary or trespassing in 2023 (exact quote stored); burglary alone was 9.0 per 1,000 households. The 1 %-a-year prior in `docs/RISK_MODEL.md` is consistent with it |

## Used by consequence

`cargo test -p rr-consequence --test docs` checks that every id `crates/rr-consequence` can emit
(`rr_consequence::citation_ids()`) is in `content/citations.toml` or listed here.

### Already in `content/citations.toml`

| id | What rr-consequence uses it for |
| --- | --- |
| `rr_risk_model_priors` | every expert estimate in `docs/RISK_MODEL.md` § "Consequences and targets": event shares, durations without records, coupling rules, income assumptions, uncertainty factors (prior = true) |
| `oregon_resilience_plan_2013` | Cascadia restoration times (coast and valley), relief in 1–2 weeks on the coast and 72 hours inland; Tohoku and Maule restoration comparisons |
| `ornl_eagle_i_outages` | county-wide storm outage rates and duration curves (the county override); the no-records fallback |
| `ornl_repowrd_2022` | hurricane power restoration (Irma: half restored in 35 h, 90 % in 115 h; Michael: 116 h and 384 h) |
| `eia_861_reliability_2024` | split of storm-day outages into short local ones (PECO major-event SAIFI) |
| `epa_boil_water_report_2024` | most boil-water advisories come from main breaks and pressure loss (80 %); flood-caused notices |
| `shaffer_2026_texas_boil_notices` | Texas boil-water notices: hurricane median 6 days, winter median 7 days, mode 3–4 days |
| `water_2024_kentucky_advisories` | Kentucky advisories averaged 5 days |
| `epa_asheville_boil_notice_2024` | Asheville's system-wide boil notice lasted about 7 weeks after Helene |
| `fema_hazus_eq_restoration` | earthquake water and power restoration (wells extensive 10.5 days; treatment plant 32; storage tanks 93) |
| `fema_nri_v120` | loss ratios as a screening check for home damage from floods and earthquakes |
| `cdc_mmwr_stay_at_home_2020` | 2020 stay-home orders (median 45 days) behind the pandemic's two-week shopping disruption |
| `census_pulse_displacement` | after a disaster a third are home within a week, 12 % are out over six months, 1 in 4 renters and 1 in 10 owners never return |
| `aung_2025_displacement` | 1.5 % of adults displaced by a disaster in a year |
| `usfa_residential_fires` | home fires: leaving home, fire readiness, displacement |
| `cdc_nchs_ed_visits` | 47.3 emergency visits per 100 people (medical emergency readiness) |
| `bls_work_experience_2024` | 8.3 % unemployed at some point in 2024 (job-loss base rate; spell lengths consistent with 21.5 % looking 27+ weeks) |
| `bls_unemployment_duration` | current spells median 11.4 weeks (completed spells are shorter) |
| `mell_2017_ems_response` | ambulances take longer to reach rural homes (the rural note) |
| `stone_2023_heat_blackout` | a blackout during a heat wave is the most dangerous combination |
| `ready_gov_nuclear` | get inside, stay inside at least 24 hours, stay tuned |
| `dogami_tsunami_faq` | a local tsunami arrives in 15–20 minutes |

### Also requested by hazards (same entry serves both)

| id | What rr-consequence uses it for |
| --- | --- |
| `cdc_pandemic_history` | one of the five pandemics since 1918 (COVID-19) disrupted shopping and caused mass job loss |
| `noaa_storm_events` | county episode lengths (snow-ins, heat and cold spells, drought) replacing default durations |

### Requested by consequence

| id | Title | Publisher, year | URL | Used for |
| --- | --- | --- | --- | --- |
| `inquirer_peco_outages` | Power outages in Philadelphia history: Peco's biggest storms | The Philadelphia Inquirer, 2025 | https://www.inquirer.com/weather/power-outages-peco-most-history-20250626.html | two of PECO's five largest outages were ice storms (1994, 2014), outside the 2018–2025 records (big ice storm class) |
| `county_boil_water_records` | a county's boil-water notice records, when a data pack provides them (none does yet; the id is a placeholder for the source the data workstream adds) | — | — | replacing the default boil-water duration with the county's median and 90th percentile |

## Added for supply on 2026-09-25

Supply cited these through `rr_research_supply_standards` until they had their own entries; its
constants can now point at them directly. Only the CDC page is federal, so only it stores a quote;
the others were read on the retrieval date and are paraphrased. A maker's specification sheet is a
legitimate source: its URL names the maker, but its title and publisher are neutral so the packet's
source list stays brand-free. Item and guidance text never names a maker either (the validator
checks citation titles, publishers and quotes for brand names; only URLs are exempt).

| id | Source | Supply number it backs | Check result |
| --- | --- | --- | --- |
| `petmd_dog_water` | PetMD, How Much Water Should a Dog Drink? (2020) | dogs: 1 oz of water per lb of body weight a day | confirmed on the page |
| `merck_vet_maintenance_fluids` | Merck Veterinary Manual, Maintenance Fluid Plan in Animals (updated Nov 2025) | 132 × kg^0.75 mL a day for dogs, 80 × kg^0.75 for cats; the cats' 0.8 oz per lb (a 10 lb cat about 250 mL a day) is derived from it | both formulas confirmed on the page |
| `bbk_vorsorgen_2025` | Germany, BBK, *Vorsorgen für Krisen und Katastrophen*, 2nd edition (11/2025) | 2 L a person a day, 0.5 L of it for cooking; households able to manage 10 days | both confirmed in the PDF |
| `dema_prepared_for_crises` | Denmark, Danish Emergency Management Agency, Prepared for crises | 3 L a person a day for drinking and food preparation (9 L for three days); manage for three days | confirmed on the page |
| `cdc_yellow_book_heat_cold` | CDC Yellow Book 2026, Heat and Cold Illness in Travelers | sweat can reach 1 L an hour (the hot-weather drinking share) | confirmed, exact quote stored. The same chapter says forcing water on someone who is not thirsty raises the risk of hyponatremia, which supports drinking to thirst on the walk home |
| `honda_eu2200i_spec` | Maker's specification sheet for a 2,200 W inverter generator (archived 2026-09-14) | 0.95 gal tank; 3.2 h at rated load and 8.1 h at a quarter load, so about 2.8 gal a day at a quarter load and 7.1 at full load | confirmed on the archived page |

## Data pack source ids

The data workstream writes source ids into `data/core/base_rates.toml` and the manifest. Use these
registry ids, so each source has exactly one id:

| Instead of | Use |
| --- | --- |
| `usfa_residential_fire_estimates` | `usfa_residential_fires` |
| `nchs_fastats_emergency_department` | `cdc_nchs_ed_visits` |
| `nchs_fastats_accidental_injury` | `nchs_accidental_injury_2024` |
| `census_cps_hh1_households` | `census_households_cps` |
| an EIA reliability id | `eia_861_reliability_2024` (data) or `eia_outage_hours_2024` (the 11-hour headline) |
| an EAGLE-I id | `ornl_eagle_i_outages` (CC BY 4.0: credit line required) |
| an OpenFEMA id | `openfema_nfip` for the data and `openfema_disclaimer` for the required statement |
| a pandemic base-rate id | `cdc_pandemic_history` (onsets) or `marani_2021_pandemics` (recurrence) |

## Registry index for the other workstreams

Every id below is in `content/citations.toml`. Federal entries carry an exact quote where one was
checked; "prior" marks an expert estimate.

| Topic | Ids |
| --- | --- |
| Screens and credits (About) | `fema_nri_disclaimer`, `openfema_disclaimer`, `fema_nri_v120`, `ornl_eagle_i_outages` (CC BY 4.0), `nca5_atlas` (CC BY 4.0), `cmra_2025` |
| Expert estimates (prior = true) | `rr_expert_prior` (supply sizing and upkeep, `docs/QUANTITY_RULES.md`), `rr_risk_model_priors` (hazard rates and durations, `docs/RISK_MODEL.md`), `prior_harm_weights` (allocator harm weights) |
| Research compilations and prices | `rr_research_supply_standards`, `rr_research_risk_model`, `rr_research_prior_art`, `rr_research_data_sources`, `rr_price_observations_2026_09` |
| Water quantity and storage | `ready_gov_water`, `ready_gov_kit`, `cdc_water_storage`, `sphere_2018`, `who_wedc_tn9`, `iom_dri_water_2005`, `church_emergency_prep_manual`, `washington_prepare_in_a_year`, `oregon_b2wr_toolkit`, `doe_water_heaters`, `cdc_find_clean_water`, `bbk_vorsorgen_2025`, `dema_prepared_for_crises` |
| Water treatment and advisories | `cdc_water_disinfection`, `epa_emergency_disinfection`, `cdc_water_advisories`, `cdc_bleach_disinfecting` |
| Water outage durations | `shaffer_2026_texas_boil_notices`, `water_2024_kentucky_advisories`, `epa_boil_water_report_2024`, `epa_asheville_boil_notice_2024`, `fema_hazus_eq_restoration`, `oregon_resilience_plan_2013` |
| Food and energy needs | `usda_dga_2020_2025`, `ready_gov_food`, `fsis_shelf_stable`, `usda_fooddata_central`, `church_home_storage_2007`, `church_hsc_order_form_2026`, `byu_longer_term_storage_2019`, `usu_food_storage_booklet`, `ensign_2006_year_supply` |
| Food costs | `usda_tfp_aug2026`, `usda_food_plans_aug2026`, `bls_average_prices` |
| Infants | `aap_formula_amounts`, `cdc_infant_feeding_disaster`, `cdc_infant_checklist`, `sutter_diapers` |
| Medicine continuity | `cdc_pregnancy_emergency`, `redcross_survival_kit`, `florida_dem_medication`, `cdc_diabetes_emergencies`, `cdc_insulin_emergency`, `ready_gov_disability`, `healthcare_ready_refill_laws`, `hhs_aspr_epap`, `fda_expired_medicines`, `fda_shelf_life_extension`, `kff_ehbs_2025` |
| Antibiotics (quantity 0) | `cdc_antibiotic_use`, `usc_21_353`, `cdc_yellow_book_travel_kits`, `fda_fish_antibiotics_warning_2023`, `bishop_2020_fish_antibiotics`, `mo_med_2026_antibiotic_kits`, `wms_wound_2014` |
| First aid and masks | `redcross_first_aid_kit`, `dhs_stop_the_bleed`, `cdc_masks`, `cdc_cholera_treatment`, `cdc_nchs_ed_visits`, `mell_2017_ems_response` |
| Sanitation and hygiene | `rdpo_emergency_toilet`, `oregon_b2wr_toolkit`, `cdc_hygiene_emergency`, `cdc_period_factsheet`, `sphere_2018` |
| Heat and cold | `cdc_heat_health`, `ready_gov_heat`, `cdc_niosh_heat_hydration`, `cdc_yellow_book_heat_cold`, `cdc_winter_safety`, `ready_gov_winter`, `semenza_1996_heat_deaths`, `stone_2023_heat_blackout`, `cdc_co_basics` |
| Power and fuel | `ready_gov_power_outages`, `eia_outage_hours_2024`, `eia_861_reliability_2024`, `ornl_eagle_i_outages`, `ornl_repowrd_2022`, `do_2023_outages`, `epa_energy_star_refrigerators`, `doe_appliance_energy`, `sil_cpap_power`, `nlr_pvwatts_v8`, `eia_btu`, `lehi_fuel_storage`, `ecfr_49_180_209`, `martell_2020_restoration_data`, `honda_eu2200i_spec` (generator fuel use) |
| Communications | `ready_gov_alerts`, `fcc_wea`, `nws_weather_radio`, `fcc_frs`, `fcc_gmrs`, `ecfr_47_1_1102`, `fcc_text_911`, `ready_gov_low_cost`, `ready_gov_plan`, `ready_gov_family_comm_card` |
| Pets | `ready_gov_pets`, `aspca_disaster_prep`, `petmd_dog_water`, `merck_vet_maintenance_fluids` |
| Evacuation and getting home | `ready_gov_evacuation`, `cdc_evacuation_psa`, `ready_gov_kit_2020`, `fhwa_mutcd_walking_speed`, `nws_tsunami_safety`, `dogami_tsunami_faq`, `hcfl_ev_safety`, `doe_afdc_stations`, `doe_fueleconomy_ev`, `ready_gov_pets`, `aspca_disaster_prep` |
| Hazard pages (Ready.gov) | `ready_gov_earthquakes`, `ready_gov_tsunamis`, `ready_gov_floods`, `ready_gov_hurricanes`, `ready_gov_tornadoes`, `ready_gov_wildfires`, `ready_gov_winter`, `ready_gov_heat`, `ready_gov_drought`, `ready_gov_volcanoes`, `ready_gov_landslides`, `ready_gov_avalanche`, `ready_gov_severe_weather`, `ready_gov_chemical`, `ready_gov_shelter`, `ready_gov_radiation`, `ready_gov_nuclear`, `ready_gov_pandemic`, `ready_gov_public_spaces`, `ready_gov_cybersecurity`, `ready_gov_home_fires` |
| Stay-home and pandemic | `cdc_mmwr_stay_at_home_2020`, `marani_2021_pandemics`, `cdc_pandemic_history` |
| Money and income | `fema_effak`, `ready_gov_financial`, `cfpb_emergency_fund`, `finra_financial_foundations`, `stlouisfed_emergency_fund_2025`, `fed_shed_2024`, `bls_work_experience_2024`, `bls_unemployment_duration`, `bls_displaced_workers_2026`, `bls_jolts_layoffs`, `dol_unemployment_insurance`, `ssa_disability_facts`, `nchs_accidental_injury_2024` |
| Home loss and insurance | `fema_nfip_flood_insurance`, `floodsmart_buy_policy`, `fema_flood_zones`, `openfema_nfip`, `aung_2025_displacement`, `census_pulse_displacement`, `usfa_residential_fires` |
| Security | `ncpc_home_safety`, `bjs_criminal_victimization_2023`, `cisa_secure_our_world`, `cisa_deescalation`, `ftc_disaster_scams`, `anglemyer_2014_firearm_access` (the firearms free action only) |
| Nuclear and radiation | `ready_gov_nuclear`, `ready_gov_radiation`, `cdc_potassium_iodide`, `nrc_potassium_iodide`, `fema_nuclear_sites`, `fri_nuclear_risk_2024` |
| Seismic and geologic | `usgs_nshm_2023`, `usgs_ucerf3_2015`, `usgs_bay_area_outlook_2016`, `usgs_new_madrid`, `usgs_pp1661f_cascadia`, `osu_cascadia_2012`, `oregon_resilience_plan_2013`, `oregon_2_weeks_ready` |
| Weather and climate data | `noaa_storm_events`, `noaa_hurdat2`, `cmra_2025`, `nca5_climate_trends`, `nca5_atlas` |
| Community and behaviour | `aldrich_sawada_2015`, `ye_aldrich_2019`, `fema_nhs_2024`, `listos_california`, `ready_gov_cert`, `clarke_2002_panic`, `tierney_2006_disaster_myths`, `drury_2009_shared_identity`, `wood_2018_milling`, `vinnell_2020_shakeout`, `gargano_2017_wtc_training`, `gollwitzer_sheeran_2006`, `lally_2010_habits` |
| Risk communication | `gigerenzer_2007_statistics`, `akl_2011_cochrane_frequencies`, `witte_allen_2000_eppm`, `tannenbaum_2015_fear_appeals` |
| Mental health | `samhsa_988`, `samhsa_disaster_distress` |
| Hazmat and planning | `epa_tri_2024`, `cdc_water_advisories`, `fema_cpg201_thira`, `tokyo_bichiku_navi` |

## Requested

Two open requests from consequence (`inquirer_peco_outages`, `county_boil_water_records`) are
listed under "Requested by consequence" above. Add new rows here as described at the top.
