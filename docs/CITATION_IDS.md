# Citation ids

Ids that engine crates attach to numbers. Each must resolve to an entry in
`content/citations.toml` (owned by the content workstream; rules in `docs/CONTENT_STANDARDS.md`
§2). Rows marked **requested by hazards** are ids `crates/rr-hazards` uses that the registry does
not define yet; the details given are what the entry should hold. Figures marked "confirm" were
taken from memory or a summary and must be checked against the source when the entry is written.
`cargo test -p rr-hazards` checks that every id the crate can emit is either in
`content/citations.toml` or listed in this file.

## Used by hazards

### Already in `content/citations.toml`

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

### Requested by hazards

| id | Title | Publisher, year | URL | Used for |
| --- | --- | --- | --- | --- |
| `usgs_nshm_2023` | 2023 National Seismic Hazard Model for the conterminous United States (revised release 2023.R2) | U.S. Geological Survey, 2023; revised 2026 | https://doi.org/10.5066/P14VGAV4 | yearly chance of shaking of 0.1 g or more at the county centre (earthquake rate), when the pack has `seismic` |
| `noaa_storm_events` | Storm Events Database | NOAA National Centers for Environmental Information | https://www.ncei.noaa.gov/stormevents/ | county episode rates (heat, extreme cold, winter storms, ice storms, high wind) when the pack has `events` |
| `openfema_nfip` | OpenFEMA: NFIP Residential Penetration Rates (v1) and FIMA NFIP Redacted Claims (v3) | FEMA | https://www.fema.gov/about/openfema/data-sets | share of homes in the Special Flood Hazard Area; flood claims per policy (inland-flood rate) |
| `fema_flood_zones` | Flood Zones (glossary: the Special Flood Hazard Area is the land with at least a 1 % annual chance of flooding) | FEMA | https://www.fema.gov/about/glossary/flood-zones | the 1 %-a-year chance for homes in a mapped flood zone (confirm the wording and URL) |
| `usgs_ucerf3_2015` | UCERF3: A New Earthquake Forecast for California's Complex Fault System (Fact Sheet 2015–3009) | U.S. Geological Survey, 2015 | https://pubs.usgs.gov/fs/2015/3009/ | 33 % chance of magnitude 6.7 or more on the Hayward–Rodgers Creek fault in 30 years (`hayward_m7`; confirm) |
| `usgs_new_madrid` | New Madrid Seismic Zone | U.S. Geological Survey, Earthquake Hazards Program | https://www.usgs.gov/programs/earthquake-hazards/new-madrid-seismic-zone | 7–10 % chance of a repeat of the 1811–1812 earthquakes in 50 years (`new_madrid_m7`; confirm figure and URL) |
| `noaa_hurdat2` | HURDAT2 Atlantic best-track database, and Continental United States Hurricane Impacts/Landfalls 1851–2023 | NOAA National Hurricane Center; AOML Hurricane Research Division | https://www.nhc.noaa.gov/data/hurdat/ ; https://www.aoml.noaa.gov/hrd/hurdat/All_U.S._Hurricanes.html | share of hurricanes that are major (about one third of US landfalls; confirm), and county passage counts in the pack |
| `nchs_accidental_injury_2024` | FastStats: Accidents or Unintentional Injuries | CDC National Center for Health Statistics, 2024 data | https://www.cdc.gov/nchs/fastats/accidental-injury.htm | 58.1 unintentional-injury deaths per 100,000 (death or disability of an earner) |
| `ssa_disability_facts` | Disability benefits facts ("more than 1 in 4 of today's 20-year-olds will become disabled before reaching full retirement age") | Social Security Administration | https://www.ssa.gov/disabilityfacts/facts.html | disability onset about 0.6 % a year (death or disability of an earner; confirm) |
| `cdc_pandemic_history` | Past pandemics (1918, 1957, 1968, 2009) | Centers for Disease Control and Prevention (archived pages) | https://archive.cdc.gov/www_cdc_gov/flu/pandemic-resources/1918-pandemic-h1n1.html | with COVID-19, five pandemic onsets in 108 years (pandemic rate) |
| `fema_nuclear_sites` | Operating Nuclear Power Plant Sites, with the 10-mile and 50-mile planning zones | FEMA | https://gis.fema.gov/arcgis/rest/services/Partner/Operating_Nuclear_Power_Plant_Sites/FeatureServer/0 | whether a plant is within 16 km or 80 km (nuclear plant accident) |
| `epa_tri_2024` | Toxics Release Inventory, 2024 national data | U.S. Environmental Protection Agency | https://www.epa.gov/toxics-release-inventory-tri-program | TRI facilities in the county (chemical spill or release) |

The data pack's own base-rate source ids (`usfa_residential_fire_estimates`,
`nchs_fastats_emergency_department`, `nchs_fastats_accidental_injury`, `census_cps_hh1_households`
and others, written by `rr-etl` into `base_rates`) are passed through unchanged to the cards they
feed. They duplicate registry entries above under different names; the data and content
workstreams should settle on one id per source.

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

