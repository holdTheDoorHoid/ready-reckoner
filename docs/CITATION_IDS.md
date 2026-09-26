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

**Seismic grids (added 2026-09-26).** The data pack reads the original 2023 NSHM grids for the
contiguous US and Alaska and the Hawaii 2021.R2 grid. Cite `usgs_nshm_2023_grid`
(doi:10.5066/P9GNPCOD) and `usgs_nshm_hawaii_2021` (ScienceBase item 6802a4fad4be0210cdcc996b,
part of doi:10.5066/P14VGAV4). `usgs_nshm_2023` stays for the revised 2023.R2 release, for when
its contiguous-US grid can be downloaded (GitHub issue #16).

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

The two ids the data job added itself, `nhtsa_early_estimate_2025` (1.10 deaths per 100 million
vehicle miles in 2025, exact quote stored) and `census_popest_vintage_2025` (the Census population
file used as a denominator), are now registry entries with the same ids. `nhtsa_crashes_2023` now
points at the 2023 overview itself (DOT HS 813 705) rather than the Crash Stats home page, with the
6.14 million crashes quoted.

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

`inquirer_peco_outages` is now in the registry (checked: PECO's second- and fourth-largest outages
were the February 2014 and January 1994 ice storms, 713,802 and 520,016 customers).
`county_boil_water_records` stays a placeholder listed here until the data workstream supplies a
source; it has no URL yet, so it cannot be a registry entry.

| id | Title | Publisher, year | URL | Used for |
| --- | --- | --- | --- | --- |
| `inquirer_peco_outages` | Power outages in Philadelphia history: Peco's biggest storms | The Philadelphia Inquirer, 2025 | https://www.inquirer.com/weather/power-outages-peco-most-history-20250626.html | two of PECO's five largest outages were ice storms (1994, 2014), outside the 2018–2025 records (big ice storm class) |
| `county_boil_water_records` | a county's boil-water notice records, when a data pack provides them (none does yet; the id is a placeholder for the source the data workstream adds) | — | — | replacing the default boil-water duration with the county's median and 90th percentile |

### Added for v0.2.0 (agent/consequence2)

In `content/citations.toml` on this branch (the water failures behind the water buckets' stress
line and the backtest in `docs/VALIDATION.md`, the reports behind the historic restoration
curves, and the two data-pack v2 sources the model reads, with the ids agent/data-hazard chose):

| id | What rr-consequence uses it for |
| --- | --- |
| `avl_watchdog_water_2024` | Asheville after Helene: 75 to 80 % of customers had water on 16 October 2024 (day 19); water began returning to the 80 % who lost it that week |
| `nchn_asheville_water_2024` | Asheville after Helene: 90 to 95 % had water on 18 October 2024 (day 21) |
| `npr_jackson_boil_2022` | Jackson's boil notice lifted after nearly 7 weeks (15 September 2022) |
| `npr_jackson_water_restored_2022` | Jackson: running water restored by 7 September 2022 after several days without service |
| `cbs_long_beach_water_2012` | Long Beach NY after Sandy: water safe and sewers back 13 days after landfall |
| `kut_austin_boil_2021` | Austin's citywide boil notice, 17 to 23 February 2021 |
| `kishore_2018_maria` | Puerto Rico after Maria: households averaged 68 days without water (read from a summary, UNVERIFIED) |
| `cleveland19_blackout_2003` | Cleveland's three-day boil advisories after the August 2003 blackout |
| `doe_maria_situation_reports` | the data pack's historic restoration curves for Puerto Rico (Maria) and the US Virgin Islands (Irma and Maria), used for the island grids' major-hurricane rows and stress line |
| `epa_echo_sdwa` | the county's share of public-water customers on a system with a health-based violation in five years (water-system fragility, model review M-03); requested by agent/data-hazard |
| `epa_aqs_daily_pm25` | the county's smoke days for the clean-air sentence; requested by agent/data-hazard |

The new hazards' rows cite the priors (`rr_risk_model_priors`) and, for dust storms, the Storm
Events record, until agent/content2's topical entries merge (network outages, medicine shortages,
evictions, arrests, sinkholes, dust storms, dam failure, water damage, funding gaps and the 2025
SNAP lapse): each row in `effects.toml` names the id to add in an "awaiting: content" comment.

## Added for the v0.1.1 guidance on 2026-09-26

Life-safety sentences from the round-2 review (S1, S7, RR-P04, RR-P07, RR-P16, RR-P17, P-22). Each
page was read on 2026-09-26 and the claim checked against it. A quote is stored only where the
wording was matched in the page's own text (the HTML or PDF, not a summary).

| id | Source | Used for | Check result |
| --- | --- | --- | --- |
| `fda_insulin_emergency` | FDA, Information Regarding Insulin Storage and Switching Between Products in an Emergency (content current as of 2017-09-19) | insulin in the maker's vial or cartridge keeps working 28 days at 59–86 °F; use warmer insulin in an emergency and replace it; never use frozen insulin; keep it as cool as possible, out of the sun | confirmed, exact quote stored. The 28 days apply to the original vial or cartridge only (insulin moved out of it: two weeks), hence "in its original vial or pen" |
| `ada_insulin_storage` | American Diabetes Association, Insulin Storage and Syringe Safety | do not use insulin with particles, discoloration, frosting, crystals or clumps | confirmed on the page (paraphrased) |
| `aace_diabetes_emergency` | American Association of Clinical Endocrinology, Are You Prepared to Manage Your Diabetes in an Emergency? | the diabetes kit: testing supplies, needles and syringes, glucose tablets | confirmed (paraphrased). CDC's list (`cdc_diabetes_emergencies`) names syringes, the meter, spare batteries, lancets and glucose tablets but not test strips or pen needles, so both are cited |
| `medicare_drugs_disaster` | CMS / Medicare.gov, Getting drugs in a disaster or emergency | ask your drug plan about a 60- to 90-day supply | confirmed, exact quote stored |
| `cdc_heat_related_illness` | CDC, Heat-Related Illnesses: What to Look For, What to Do (CS280226, 2017) | heat stroke: 103 °F or higher; hot, red, dry or damp skin; fast, strong pulse; confusion; passing out; call 911, move to a cooler place, cool cloths or a cool bath, nothing to drink | confirmed from the PDF, exact quote stored. Ready.gov's heat page still says "Red, hot and dry skin with no sweat"; the guidance follows CDC |
| `cpsc_co_information_center` | CPSC, Carbon Monoxide Information Center | symptoms (headache, dizziness, weakness, nausea, vomiting, sleepiness, confusion); get outside to fresh air, then call 911 | confirmed, exact quote stored. `cdc_co_basics` lists the symptoms but not what to do |
| `osha_downed_wires` | OSHA fact sheet, Working Safely Around Downed Electrical Wires (2018) | treat every line as energized; in a vehicle touching a line, stay inside unless it is on fire | confirmed from the PDF, exact quote stored |
| `pa_puc_power_line_safety` | Pennsylvania PUC, Power Line Safety fact sheet (2013) | stay at least 30 feet from a downed line and anything it touches; treat all downed lines as energized; call 911 and the utility | confirmed (paraphrased). The brief asked for 35 feet: that figure comes from ESFI and utility pages; ESFI refuses automated reads, and the one state page giving 35 feet (Texas Department of Insurance) is about an electrocuted worker. The guidance says 30 feet, from the regulator's fact sheet |
| `cpuc_medical_baseline` | California PUC, Medical Baseline | utility medical programs bring a lower rate and advance notice of outages | confirmed (paraphrased); cited in the disability topic and the glossary |
| `psegli_critical_care` | PSEG Long Island, Protect Your Health During Power Outages (Critical Care Program) | the program does not guarantee priority restoration; plan ahead for medical needs; the utility stays in touch in severe weather | confirmed (paraphrased). Replaces Ready.gov's "ask to be put on a list for priority power restoration" (RR-P07) |
| `tdem_stear` | Texas Division of Emergency Management, State of Texas Emergency Assistance Registry | a free state registry; registering does not guarantee a service | confirmed (paraphrased); the example behind "some states" in the disability topic (the state table is v0.2.0) |
| `epa_burnwise_faq` | EPA Burn Wise, Frequent Questions about Wood-Burning Appliances | wood stove, chimney and vents professionally inspected and cleaned each year; creosote build-up causes chimney fires | confirmed, exact quote stored |
| `ready_gov_stay_safe_warm` | FEMA / Ready.gov, Stay Safe and Warm Toolkit (FEMA Advisory, January 2026) | go to a warming center if you cannot keep your home warm; call 2-1-1 to find one | confirmed from the PDF, exact quote stored |
| `pa_puc_gas_emergencies` | Pennsylvania PUC, Gas Emergencies | smell gas: leave at once; do not turn lights on or off or use a phone at home; call 911 and the gas utility from a safe distance | confirmed (paraphrased). PHMSA's leak page refuses automated reads |
| `ready_gov_safety_skills` | FEMA / Ready.gov, Safety Skills | smell gas or hear hissing: get everyone out, call the gas company from a neighbour's home; only a qualified professional turns the gas back on | confirmed through a page fetch; ready.gov refuses plain HTTP clients, so no quote is stored |
| `redcross_sound_the_alarm` | American Red Cross, Sound the Alarm | free smoke alarm installation from your local Red Cross | redcross.org refuses automated requests; read through the Internet Archive capture of the same URL (2026-09-17). Paraphrased |
| `usfa_smoke_alarm_renters` | USFA, Pictograph: Where to put home smoke alarms (renters) | renters without alarms ask the landlord or rental agent to install them | confirmed, exact quote stored. `usfa_smoke_alarms` supplies "some fire departments ... install ... at no cost". No readable national source says landlords must provide alarms "in most states", so the guidance only says to ask |
| `medlineplus_cpr` | MedlinePlus (National Library of Medicine), CPR health topic | hands-only CPR for a teen or adult whose heart has stopped, no training needed; hands on the center of the chest, push hard and fast; call 911; AEDs are in many public places and talk you through each step | confirmed, exact quote stored (NLM-written summary, public domain) |
| `cdc_well_disinfection` | CDC, How to Disinfect Wells After an Emergency (reviewed 2025-05-29) | after flooding, do not drink well water until it is disinfected and tested; ask the health department | confirmed through a page fetch; cdc.gov refuses plain HTTP clients, so no quote is stored |
| `epa_children_wildfire_smoke` | EPA, Protect Children from Wildfires, Smoke, and Volcanic Ash | N95 masks are not made to fit children; use a clean air room | confirmed, exact quote stored |

**`fema_nhs_2024` repointed (RR-P17).** It cited FEMA's 2024 National Household Survey findings deck
through a copy on a private website. FEMA has removed the deck from fema.gov (the URL now answers 404),
so the entry now points at the Internet Archive capture of FEMA's own file
(`fema_icpd_2024-national-household-survey-on-disaster-preparedness-findings_05072025.pdf`, captured
2025-05-07), publisher "FEMA (via Internet Archive)", with the cost-barrier sentence as its quote. The
text of FEMA's file and the old copy is identical, so every figure the guidance cites still matches
(checked: 88, 34, 18 and 8 of 100 with supplies past three days, two weeks, a month and three months;
26 cost; 25 "don't know what else to do"; 16 health or disability; 71 expect help from family; 18 heard
how to help neighbours; renters 43 and owners 87 insured; 63/22 power outage and 47/5 active shooter,
risk and experience). The DataLumos archive the review suggested (project 218642) holds FEMA's
2017–2023 survey data, not the 2024 findings deck.

## Requested by hazards for v0.2.0 (2026-09-26)

The v0.2.0 hazard rows (REVIEW §2, DESIGN-DELTA §1.2) cite these. None is in the registry yet;
the content workstream writes each entry and checks the figure. "Confirm" marks a figure read only
through a search summary or a secondary copy (hazard-expansion report, "UNVERIFIED items"). As
with `county_boil_water_records`, each requested id is also listed in rr-content's
`NOT_CITATIONS` (tests/citation_ids.rs) and rr-plan's `AWAITING_CONTENT` until its entry exists;
rr-plan's test then says to remove it from both. Ids a data job already uses in its files keep
that id (fbi_cde_arrests, crs_rs20348_funding_gaps, fdic_failed_banks, openfda_drug_shortages,
pnnl_oe417_linkage: the data-model series files).

| id | Title | Publisher, year | URL | Used for |
| --- | --- | --- | --- | --- |
| `rr_strategic_sites` | Strategic sites and county strategic-exposure classes (data/core/strategic_sites.toml) | Ready Reckoner, compiled 2026-09-26 from DoD MIRTA, the Sentinel EIS, NNSA, 10 U.S.C. 2674, Census, EIA and BTS, one public source per site | https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/data/core/strategic_sites.toml | the county class A–E of the nuclear family, the sites named in "Why here", and the near/far split of the war row. Nine sites' roles rest on secondary sources (research strategic-sites §1) |
| `fema_protection_nuclear_age_1985` | Protection in the Nuclear Age (H-20) | FEMA, 1985 | https://www.nukepills.com/docs/FEMA_Nuclear_War_Survival.pdf (private copy; **confirm** with a FEMA, NARA or HathiTrust copy) | the public precedent for the class A and C1 sentences: a risk area "does not mean that it will be attacked" (p. 12) |
| `fema_napb90` | Nuclear Attack Planning Base – 1990, Executive Summary | FEMA, 1987 (released 2005) | https://nuke.fas.org/guide/usa/napb-90/execsum.html | the method precedent for county blast and fallout classes (not a public precedent: it was restricted until 2005) |
| `philippe_2023_icbm_fallout` | Who Would Take the Brunt of an Attack on U.S. Nuclear Missile Silos? | Scientific American (S. Philippe), 2023; Princeton, The Missiles on our Land | https://www.scientificamerican.com/article/who-would-take-the-brunt-of-an-attack-on-u-s-nuclear-missile-silos/ | the calibration of class B (fallout downwind of the missile fields) |
| `fema_hsgp_fy2026` | Fiscal Year 2026 Homeland Security Grant Program NOFO, Appendix I: HSGP Allocations | FEMA, 2026 | https://www.fema.gov/sites/default/files/documents/fema_gpd_hsgp-nofo-fy2026.pdf | the metro weight of the attack, CBRN and nuclear-terrorism rows: each urban area's share of the $584,250,000 (New York-White Plains 24.39 %, Philadelphia 2.84 %) |
| `nerc_tpl007_gmd` | Benchmark Geomagnetic Disturbance Event Description | NERC, 2014 | https://www.nerc.com/globalassets/standards/projects/2013-03/benchmark_gmd_event_aug27_clean.pdf | the scaling factor α = 0.001·e^(0.115·λ), bounded 0.1–1, by geomagnetic latitude |
| `igrf14_coefficients` | International Geomagnetic Reference Field, 14th generation (coefficients) | IAGA; NOAA NCEI copy, 2024 | https://www.ngdc.noaa.gov/IAGA/vmod/coeffs/igrf14coeffs.txt | each county's geomagnetic latitude |
| `noaa_hms_smoke` | Hazard Mapping System Fire and Smoke Product | NOAA OSPO | https://www.ospo.noaa.gov/products/land/hms.html | which days are smoke days |
| `epa_aqs_daily_pm25` | AirData pre-generated files: daily PM2.5 and AQI by county | EPA | https://aqs.epa.gov/aqsweb/airdata/download_files.html | smoke days at 35.5 µg/m³ or more (2016–2023 mean): the wildfire-smoke rate |
| `usgs_karst_2014` | Karst in the United States: A Digital Map Compilation and Database (Open-File Report 2014-1156) | USGS (Weary and Doctor), 2014 | https://pubs.usgs.gov/of/2014/1156 | the share of a county on karst: the sinkhole rate |
| `usace_nid` | National Inventory of Dams | USACE | https://nid.sec.usace.army.mil/ | high-hazard dams, their condition and their listed downstream town |
| `usace_nld` | National Levee Database | USACE | https://levees.sec.usace.army.mil/ | people behind levees and USACE's levee risk rating |
| `asdso_dam_failures` | Estimated Rates of Failure of Dams in the United States | Association of State Dam Safety Officials | https://damsafety.org/reference/estimated-rates-failure-dams-united-states | 173 failures and 587 incidents, January 2005 to June 2013: about 2 in 10,000 failures per dam a year |
| `eviction_lab_county_estimates` | Eviction Lab: national and county estimates, 2000–2018 | Princeton University Eviction Lab (ODC-BY 1.0: owner sign-off on the attribution licence pending, data audit §7) | https://evictionlab.org/map/ | about 2.3 eviction judgments per 100 renter households (2016; **confirm**), and about 0.9 million judgments from 2.3 million filings (**confirm**) |
| `iii_water_damage` | Facts + Statistics: Homeowners and renters insurance | Insurance Information Institute (ISO data) | https://www.iii.org/fact-statistic/facts-statistics-homeowners-and-renters-insurance | water damage and freezing: about 1 in 67 insured homes a year (2019–2023), average claim about $15,400 (**confirm**: read through search summaries) |
| `fcc_att_outage_2024` | February 22, 2024 AT&T Mobility Network Outage Report | FCC Public Safety and Homeland Security Bureau, 2024 | https://docs.fcc.gov/public/attachments/DOC-404150A1.pdf | more than 92 million calls and 25,000 calls to 911 blocked for at least 12 hours |
| `ashp_shortages` | Drug Shortages Statistics | ASHP | https://www.ashp.org/drug-shortages/shortage-resources/drug-shortages-statistics | 323 active shortages in the first quarter of 2024 (headline only; the data behind it are proprietary) |
| `openfda_drug_shortages` | openFDA drug shortages endpoint | US FDA (CC0) | https://open.fda.gov/apis/drug/drugshortages/ | 70 medicines listed as currently short on 2026-09-26, 50 of them injectables |
| `crs_rs20348_funding_gaps` | Federal Funding Gaps: A Brief Overview (RS20348, version 48) | Congressional Research Service, 2026 | https://www.congress.gov/crs-product/RS20348 | funding gaps of 14 days or more in 4 of the 45 fiscal years 1982–2026 |
| `snap_lapse_2025` | SNAP benefits and the government shutdown | CNBC, 12 November 2025 (with Axios, 14 November 2025) | https://www.cnbc.com/2025/11/12/snap-benefits-government-shutdown-negotiations.html | November 2025: the first lapse in SNAP payments, about 42 million people |
| `csis_terrorism_2025` | CSIS US terrorism dataset: methodology | Center for Strategic and International Studies, 2024 | https://csis-website-prod.s3.amazonaws.com/s3fs-public/2024-10/241021_McCabe_Domestic_Methodology.pdf | 750 attacks and plots, 1994 to July 2025; three or four metro-wide closures in 31 years (**confirm**) |
| `fbi_cde_arrests` | Crime in the United States: arrests by age and sex (Tables 29, 39 and 40) | FBI Uniform Crime Reporting, Crime Data Explorer, 2023–2025 | https://cde.ucr.cjis.gov/LATEST/webapp/#/pages/downloads | arrests per 100,000 people a year by age band and sex (data-model series fbi_arrests) |
| `fbi_active_shooter_2024` | Active Shooter Incidents in the United States in 2024 | FBI, 2025 | https://www.fbi.gov/file-repository/reports-and-publications/2024-active-shooter-report | 24 incidents, 23 killed, 83 wounded |
| `start_poicn` | Profiles of Incidents involving CBRN and Non-state Actors (POICN) | START, University of Maryland | https://www.start.umd.edu/research-projects/profiles-incidents-involving-cbrn-and-non-state-actors-poicn-database | 517 CBRN events worldwide 1990–2017, about 76 % chemical |
| `xpt_2023_karger` | Forecasting Existential Risk: Evidence from a Long-Run Forecasting Tournament | Karger et al., Forecasting Research Institute, 2023 | https://forecastingresearch.org/pdf/existential-risk-persuasion-tournament.pdf | nuclear use killing more than 1,000 by 2030: experts 4.5 %, superforecasters 4 % |
| `rp_2019_nuclear` | How likely is a nuclear exchange between the US and Russia? | Rethink Priorities (L. Rodriguez), 2019 | https://rethinkpriorities.org/research-area/how-likely-is-a-nuclear-exchange-between-the-us-and-russia/ | the 0.38 % a year aggregate for a US–Russia exchange |
| `barrett_2013_inadvertent` | Analyzing and Reducing the Risks of Inadvertent Nuclear War Between the United States and Russia | Barrett, Baum and Hostetler, Science & Global Security 21(2), 2013 | https://scienceandglobalsecurity.org/archive/sgs21barrett.pdf | median 0.9 % a year (90 % interval 0.02–7 %) |
| `fema_nuclear_72h_2023` | Nuclear Detonation Response Guidance: Planning for the First 72 Hours | FEMA, 2023 | https://www.fema.gov/sites/default/files/documents/fema_oet-72-hour-nuclear-detonation-response-guidance.pdf | the 50-mile shelter message and at least 24 hours inside |
| `epri_2019_hemp` | High-Altitude Electromagnetic Pulse and the Bulk Power System (3002014979) | EPRI, 2019 | https://www.epri.com/research/products/000000003002014979 | large-transformer failures unlikely; no support for months-long nationwide blackouts |
| `riley_2012_carrington` | On the probability of occurrence of extreme space weather events | Riley, Space Weather 10, 2012 | https://doi.org/10.1029/2011SW000734 | about 12 % a decade for a Carrington-class storm (**confirm**: read through Moriña 2019) |
| `morina_2019_carrington` | Probability estimation of a Carrington-like geomagnetic storm | Moriña et al., Scientific Reports 9, 2019 | https://www.nature.com/articles/s41598-019-38918-8 | 0.46–1.88 % a decade |
| `love_carrington` | Lognormality of historical magnetic-storm intensity statistics: implications for extreme-event probabilities | Love, USGS | https://www.usgs.gov/publications/lognormality-historical-magnetic-storm-intensity-statistics-implications-extreme-event | 1.13 Carrington-class storms per century (0.42–2.41) |
| `lloyds_2013_solar` | Solar Storm Risk to the North American Electric Grid | Lloyd's and AER, 2013 | https://assets.lloyds.com/assets/pdf-solar-storm-risk-to-the-north-american-electric-grid/1/pdf-Solar-Storm-Risk-to-the-North-American-Electric-Grid.pdf | 20–40 million people at risk of a long outage; 16 days to a year or two for the worst hit; a return period of about 150 years |
| `cassidy_mani_2022` | Huge volcanic eruptions: time to prepare | Cassidy and Mani, Nature 608, 2022 | https://doi.org/10.1038/d41586-022-02177-x | a one-in-six chance of a VEI 7 eruption this century |
| `usgs_yvo` | Questions about supervolcanoes | USGS Yellowstone Volcano Observatory | https://www.usgs.gov/volcanoes/yellowstone/questions-about-supervolcanoes | about 1 in 730,000 a year for a caldera-forming eruption |
| `nasa_tunguska_2019` | Tunguska Revisited: 111-year-old mystery impact inspires new, more optimistic asteroid predictions | NASA, 2019 | https://www.nasa.gov/solar-system/tunguska-revisited-111-year-old-mystery-impact-inspires-new-more-optimistic-asteroid-predictions/ | Tunguska-class impacts "on the order of millennia" |
| `fdic_failed_banks` | Bank failures and assistance transactions (BankFind Suite) | FDIC | https://banks.data.fdic.gov/bankfind-suite/failures | 583 failures in 2001–2025, about 23 a year (data-model series fdic_failures) |
| `npr_maria_2018` | 11 months after Hurricane Maria hit Puerto Rico, officials say all power is restored | NPR, 2018 | https://www.npr.org/2018/08/15/639001372/11-months-after-hurricane-maria-hit-puerto-rico-officials-say-all-power-is-resto | 328 days until every customer had power |
| `utah_wguep_2016` | Earthquake Probabilities for the Wasatch Front Region in Utah, Idaho, and Wyoming (Miscellaneous Publication 16-3) | Working Group on Utah Earthquake Probabilities, Utah Geological Survey, 2016 | https://geology.utah.gov/hazards/earthquakes/earthquake-probabilities/ (**confirm** URL) | 43 % chance of a magnitude 6.75 or larger earthquake in 50 years (**confirm**) |
| `usgs_seattle_fault` | The Seattle fault zone | USGS Earthquake Hazards Program with Washington DNR (**confirm** the page) | https://www.usgs.gov/programs/earthquake-hazards (**confirm**) | about 5 % chance of a magnitude 6.5 or larger earthquake in 50 years (**confirm**) |
| `pnnl_oe417_linkage` | Event-correlated Outage Dataset in America | Pacific Northwest National Laboratory, OpenEI submission 6458 (CC BY 4.0: credit line required) | https://data.openei.org/submissions/6458 | OE-417 reports 2019–2023: 78.4 a year of physical attack, vandalism, sabotage or theft; 7.4 cyber (data-model series oe417) |
| `cdc_co_quickstats` | QuickStats: Number of deaths resulting from unintentional carbon monoxide poisoning, 2010–2015 | CDC, MMWR 66(8), 2017 | https://www.cdc.gov/mmwr/volumes/66/wr/mm6608a9.htm | about 374 deaths a year |
| `ftc_sentinel_2024` | Consumer Sentinel Network Data Book 2024 | FTC, 2025 | https://www.ftc.gov/reports/consumer-sentinel-network-data-book-2024 | 1.1 million identity-theft reports |
| `usgs_barry_arm` | Potential landslide-generated tsunami in Prince William Sound's Barry Arm | USGS | https://www.usgs.gov/news/state-news-release/potential-landslide-generated-tsunami-prince-william-sounds-barry-arm | the fjord-landslide tsunami note |
| `cdc_h5n1_situation` | H5 Bird Flu: Current Situation | CDC | https://www.cdc.gov/bird-flu/situation-summary/index.html | 70 US human cases since April 2024, none spread between people |
| `iv_fluids_helene_2024` | IV fluid shortage after Hurricane Helene (commentary) | PubMed Central, 2024 | https://pmc.ncbi.nlm.nih.gov/articles/PMC11627566/ | the Baxter North Cove plant, about 60 % of US IV fluids |

The v0.2.0 hazard rows also cite registry entries not listed above for hazards before:
`ready_gov_floods`, `ready_gov_winter`, `ready_gov_hurricanes`, `ready_gov_earthquakes`,
`ready_gov_tsunamis`, `ready_gov_chemical`, `ready_gov_pandemic`, `ready_gov_cybersecurity`,
`ready_gov_power_outages`, `ready_gov_home_fires`, `epa_asheville_boil_notice_2024`,
`cdc_co_basics`, `ftc_disaster_scams`, `cdc_pregnancy_emergency`, `usgs_bay_area_outlook_2016`
and `stone_2023_heat_blackout` (sub-cause notes and the heat-and-blackout scenario).

## Registry index for the other workstreams

Every id below is in `content/citations.toml`. Federal entries carry an exact quote where one was
checked; "prior" marks an expert estimate.

| Topic | Ids |
| --- | --- |
| Screens and credits (About) | `fema_nri_disclaimer`, `openfema_disclaimer`, `fema_nri_v120`, `ornl_eagle_i_outages` (CC BY 4.0), `nca5_atlas` (CC BY 4.0), `cmra_2025` |
| Expert estimates (prior = true) | `rr_expert_prior` (supply sizing and upkeep, `docs/QUANTITY_RULES.md`), `rr_risk_model_priors` (hazard rates and durations, `docs/RISK_MODEL.md`), `prior_harm_weights` (allocator harm weights) |
| Research compilations and prices | `rr_research_supply_standards`, `rr_research_risk_model`, `rr_research_prior_art`, `rr_research_data_sources`, `rr_price_observations_2026_09` |
| Water quantity and storage | `ready_gov_water`, `ready_gov_kit`, `cdc_water_storage`, `sphere_2018`, `who_wedc_tn9`, `iom_dri_water_2005`, `church_emergency_prep_manual`, `washington_prepare_in_a_year`, `oregon_b2wr_toolkit`, `doe_water_heaters`, `cdc_find_clean_water`, `bbk_vorsorgen_2025`, `dema_prepared_for_crises` |
| Water treatment and advisories | `cdc_water_disinfection`, `epa_emergency_disinfection`, `cdc_water_advisories`, `cdc_bleach_disinfecting`, `cdc_well_disinfection` (private wells after a flood) |
| Water outage durations | `shaffer_2026_texas_boil_notices`, `water_2024_kentucky_advisories`, `epa_boil_water_report_2024`, `epa_asheville_boil_notice_2024`, `fema_hazus_eq_restoration`, `oregon_resilience_plan_2013` |
| Food and energy needs | `usda_dga_2020_2025`, `ready_gov_food`, `fsis_shelf_stable`, `usda_fooddata_central`, `church_home_storage_2007`, `church_hsc_order_form_2026`, `byu_longer_term_storage_2019`, `usu_food_storage_booklet`, `ensign_2006_year_supply` |
| Food costs | `usda_tfp_aug2026`, `usda_food_plans_aug2026`, `bls_average_prices` |
| Infants | `aap_formula_amounts`, `cdc_infant_feeding_disaster`, `cdc_infant_checklist`, `sutter_diapers` |
| Medicine continuity | `cdc_pregnancy_emergency`, `redcross_survival_kit`, `florida_dem_medication`, `cdc_diabetes_emergencies`, `cdc_insulin_emergency`, `ready_gov_disability`, `healthcare_ready_refill_laws`, `hhs_aspr_epap`, `fda_expired_medicines`, `fda_shelf_life_extension`, `kff_ehbs_2025`, `fda_insulin_emergency`, `ada_insulin_storage`, `aace_diabetes_emergency`, `medicare_drugs_disaster` |
| Antibiotics (quantity 0) | `cdc_antibiotic_use`, `usc_21_353`, `cdc_yellow_book_travel_kits`, `fda_fish_antibiotics_warning_2023`, `bishop_2020_fish_antibiotics`, `mo_med_2026_antibiotic_kits`, `wms_wound_2014` |
| First aid and masks | `redcross_first_aid_kit`, `dhs_stop_the_bleed`, `medlineplus_cpr`, `cdc_masks`, `epa_children_wildfire_smoke`, `cdc_cholera_treatment`, `cdc_nchs_ed_visits`, `mell_2017_ems_response` |
| Sanitation and hygiene | `rdpo_emergency_toilet`, `oregon_b2wr_toolkit`, `cdc_hygiene_emergency`, `cdc_period_factsheet`, `sphere_2018` |
| Heat and cold | `cdc_heat_health`, `ready_gov_heat`, `cdc_niosh_heat_hydration`, `cdc_yellow_book_heat_cold`, `cdc_winter_safety`, `ready_gov_winter`, `semenza_1996_heat_deaths`, `stone_2023_heat_blackout`, `cdc_co_basics`, `cdc_heat_related_illness`, `cpsc_co_information_center`, `ready_gov_stay_safe_warm`, `epa_burnwise_faq` |
| Power and fuel | `ready_gov_power_outages`, `eia_outage_hours_2024`, `eia_861_reliability_2024`, `ornl_eagle_i_outages`, `ornl_repowrd_2022`, `do_2023_outages`, `epa_energy_star_refrigerators`, `doe_appliance_energy`, `sil_cpap_power`, `nlr_pvwatts_v8`, `eia_btu`, `lehi_fuel_storage`, `ecfr_49_180_209`, `martell_2020_restoration_data`, `honda_eu2200i_spec` (generator fuel use), `osha_downed_wires`, `pa_puc_power_line_safety`, `cpuc_medical_baseline`, `psegli_critical_care` |
| Communications | `ready_gov_alerts`, `fcc_wea`, `nws_weather_radio`, `fcc_frs`, `fcc_gmrs`, `ecfr_47_1_1102`, `fcc_text_911`, `ready_gov_low_cost`, `ready_gov_plan`, `ready_gov_family_comm_card` |
| Pets | `ready_gov_pets`, `aspca_disaster_prep`, `petmd_dog_water`, `merck_vet_maintenance_fluids` |
| Evacuation and getting home | `ready_gov_evacuation`, `cdc_evacuation_psa`, `ready_gov_kit_2020`, `fhwa_mutcd_walking_speed`, `nws_tsunami_safety`, `dogami_tsunami_faq`, `hcfl_ev_safety`, `doe_afdc_stations`, `doe_fueleconomy_ev`, `ready_gov_pets`, `aspca_disaster_prep` |
| Hazard pages (Ready.gov) | `ready_gov_earthquakes`, `ready_gov_tsunamis`, `ready_gov_floods`, `ready_gov_hurricanes`, `ready_gov_tornadoes`, `ready_gov_wildfires`, `ready_gov_winter`, `ready_gov_heat`, `ready_gov_drought`, `ready_gov_volcanoes`, `ready_gov_landslides`, `ready_gov_avalanche`, `ready_gov_severe_weather`, `ready_gov_chemical`, `ready_gov_shelter`, `ready_gov_radiation`, `ready_gov_nuclear`, `ready_gov_pandemic`, `ready_gov_public_spaces`, `ready_gov_cybersecurity`, `ready_gov_home_fires` |
| Stay-home and pandemic | `cdc_mmwr_stay_at_home_2020`, `marani_2021_pandemics`, `cdc_pandemic_history` |
| Money and income | `fema_effak`, `ready_gov_financial`, `cfpb_emergency_fund`, `finra_financial_foundations`, `stlouisfed_emergency_fund_2025`, `fed_shed_2024`, `bls_work_experience_2024`, `bls_unemployment_duration`, `bls_displaced_workers_2026`, `bls_jolts_layoffs`, `dol_unemployment_insurance`, `ssa_disability_facts`, `nchs_accidental_injury_2024` |
| Home loss and insurance | `fema_nfip_flood_insurance`, `floodsmart_buy_policy`, `fema_flood_zones`, `openfema_nfip`, `aung_2025_displacement`, `census_pulse_displacement`, `usfa_residential_fires` |
| Home fire and gas | `usfa_residential_fires`, `usfa_smoke_alarms`, `usfa_smoke_alarm_renters`, `redcross_sound_the_alarm`, `ready_gov_home_fires`, `usfa_heating_fires`, `usfa_cooking_fires`, `usfa_extinguishers`, `pa_puc_gas_emergencies`, `ready_gov_safety_skills` |
| Security | `ncpc_home_safety`, `bjs_criminal_victimization_2023`, `cisa_secure_our_world`, `cisa_deescalation`, `ftc_disaster_scams`, `anglemyer_2014_firearm_access` (the firearms free action only) |
| Nuclear and radiation | `ready_gov_nuclear`, `ready_gov_radiation`, `cdc_potassium_iodide`, `nrc_potassium_iodide`, `fema_nuclear_sites`, `fri_nuclear_risk_2024` |
| Seismic and geologic | `usgs_nshm_2023`, `usgs_ucerf3_2015`, `usgs_bay_area_outlook_2016`, `usgs_new_madrid`, `usgs_pp1661f_cascadia`, `osu_cascadia_2012`, `oregon_resilience_plan_2013`, `oregon_2_weeks_ready` |
| Weather and climate data | `noaa_storm_events`, `noaa_hurdat2`, `cmra_2025`, `nca5_climate_trends`, `nca5_atlas` |
| Community and behaviour | `aldrich_sawada_2015`, `ye_aldrich_2019`, `fema_nhs_2024` (FEMA's own file via the Internet Archive), `tdem_stear`, `listos_california`, `ready_gov_cert`, `clarke_2002_panic`, `tierney_2006_disaster_myths`, `drury_2009_shared_identity`, `wood_2018_milling`, `vinnell_2020_shakeout`, `gargano_2017_wtc_training`, `gollwitzer_sheeran_2006`, `lally_2010_habits` |
| Risk communication | `gigerenzer_2007_statistics`, `akl_2011_cochrane_frequencies`, `witte_allen_2000_eppm`, `tannenbaum_2015_fear_appeals` |
| Mental health | `samhsa_988`, `samhsa_disaster_distress` |
| Hazmat and planning | `epa_tri_2024`, `cdc_water_advisories`, `fema_cpg201_thira`, `tokyo_bichiku_navi` |

## Requested

One placeholder is open: `county_boil_water_records` (see "Requested by consequence"), which
waits for the data workstream to name a source. Add new rows here as described at the top.
