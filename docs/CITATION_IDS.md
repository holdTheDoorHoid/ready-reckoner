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

## Added for the v0.2.0 content on 2026-09-26

The v0.2.0 content brief (new blocks, the state table, the Deviant Ollam lessons) added 88 entries
and re-read 35. Every page below was read on 2026-09-26 and the sentence that cites it was checked
against it. "Quote" means an exact sentence is stored, matched in the page's own text (the HTML, the
PDF's extracted text, or the text of the page as a browser shows it). A page read only through a
summarising fetch stores no quote. How a page was read is stated where it was not a plain download.

### Recovery, the family plan, access needs and clean air

| id | Source | Used in | How it was read and checked |
| --- | --- | --- | --- |
| `ready_gov_recovering` | FEMA / Ready.gov, Recovering from Disaster | after_first_30_days | go back only when officials say it is safe; photos before clean-up. Page fetch (ready.gov refuses some scripted clients), paraphrased |
| `fema_home_inspections` | FEMA, Home Inspections | after_first_30_days | inspectors carry photo ID, never ask for bank details; a FEMA shirt is not ID; keep receipts |
| `fema_disaster_fraud` | FEMA, Disaster Fraud | after_first_30_days | FEMA never charges to apply or inspect |
| `fema_ihp` | FEMA, Individuals and Households Program | after_first_30_days | FEMA help does not replace insurance |
| `disasterassistance_gov` | DHS, DisasterAssistance.gov | after_first_30_days | apply online or at 1-800-621-3362 when a declaration covers the county |
| `floodsmart_start_claim` | NFIP FloodSmart, Start a Claim | after_first_30_days | quote; the adjuster, the estimate, talk before signing a repair contract |
| `ftc_scams_after_disasters` | FTC, scams after weather emergencies | after_first_30_days | quote; never sign an insurance check over to a contractor; no cash up front |
| `healthcare_ready_rx_open` | Healthcare Ready, Rx Open | after_first_30_days | the open-pharmacy map for disaster areas |
| `usa_gov_replace_ids` | USAGov, replace lost or stolen ID cards | after_first_30_days | quote; start with the birth certificate from the state of birth |
| `epa_clean_room` | EPA, Create a Clean Room | bucket_clean_air, hazard_wildfire_smoke, plan_shelter | quote; an air cleaner that does not make ozone |
| `epa_protect_lungs_2026` | EPA fact sheet, Protect Your Lungs From Wildfire Smoke and Ash (June 2026) | bucket_clean_air, hazard_wildfire_smoke | PDF, quote; respirators for children 2 and older, fit caveats |
| `texasready_hurricanes` | Texas DSHS (TexasReady), Hurricanes | plan_forecast_48h | the 48-hour steps (charge, fuel, cash, refills, fill the tub, coldest fridge setting) |
| `nws_aly_cold_safety` | NWS Albany, Cold Weather Safety Tips | plan_forecast_48h, hazard_water_damage | PDF, quote; drip faucets, open sink cabinets, heat at 55 °F or higher |
| `cisa_pace_flyer` | CISA (SAFECOM/NCSWIC), Set Your PACE (2025) | plan_communication | PDF, quote; primary, alternate, contingency, emergency |
| `fema_back_to_school_2026` | FEMA blog, Back to School, Back to Safety | plan_communication | ask the school about its release and reunification plan |
| `aspr_cmist` | HHS ASPR, the CMIST framework | topic_access_needs | the five headings; keeping people with their aids and devices |
| `usfa_fire_safety_disabilities` | USFA, Fire Safety for People with Disabilities | topic_access_needs | quote; vibrating pads and strobe lights for people who are deaf or hard of hearing |
| `ready_gov_your_language` | FEMA / Ready.gov, Ready in Your Language | topic_access_needs | listo.gov and the other languages |
| `aspr_tracie_hha_rule` | HHS ASPR TRACIE, CMS emergency preparedness rule for home health agencies | topic_access_needs | PDF, quote; an individual emergency plan for each patient |

### The eleven new ranked hazards

| id | Source | Used in | How it was read and checked |
| --- | --- | --- | --- |
| `iii_homeowners_losses`, `iii_water_damage_protect` | Insurance Information Institute | hazard_water_damage | water and freezing claims are filed about six times as often as fire claims (ISO data, 2019–2023); shut-off valve, hoses, what policies cover. The institute is the insurance industry's information body; no agency publishes claim frequency by cause, so it is the source (paraphrased) |
| `fema_dam_residual_risk_2018` | FEMA fact sheet, Risk Exposure and Residual Risk Related to Dams | hazard_dam_levee | PDF (fema.gov refuses scripted clients, so read through a page fetch and its text extracted), quote |
| `fema_living_with_levees` | FEMA, Living with Levees | hazard_dam_levee | levees lower the risk from some floods but not all; ask the local government; the levee database |
| `fema_nfip_levees_2021` | FEMA fact sheet, NFIP and Levees | hazard_dam_levee | PDF as above, quote; accreditation does not guarantee protection |
| `usace_nld` | USACE, National Levee Database | hazard_dam_levee | the public levee lookup |
| `mo_sema_dam_failure` | Missouri SEMA, Dam Failures | hazard_dam_levee | ask about a high-hazard dam upstream and its emergency action plan; know the route; get to higher ground |
| `fcc_network_outage_2024` | FCC PSHSB report on the 2024-02-22 wireless outage | hazard_network_outage | PDF, quote; more than 25,000 failed 911 calls |
| `fda_drug_shortages`, `fda_drug_shortages_faq`, `fda_besaferx` | FDA | hazard_drug_shortage | quotes on the first two; why shortages happen; try other pharmacies and ask about alternatives; FDA's shortage list; state-licensed online pharmacies |
| `me_dhhs_snap_2025` | Maine DHHS news release (2025-10-21) | hazard_benefit_interruption | in October 2025 USDA told every state that November SNAP benefits would not be issued; about 42 million people get SNAP |
| `usda_hunger_hotline` | USDA National Hunger Hotline | hazard_benefit_interruption | the hotline and its hours |
| `dol_ucfe_furlough_2023` | DOL, Federal Furloughs: UCFE Fact Sheet | hazard_benefit_interruption | PDF; furloughed federal workers may file for UCFE |
| `cfpb_shutdown_2013`, `cfpb_payday_loans` | CFPB | hazard_benefit_interruption | quotes; call lenders early; the cost of a two-week payday loan |
| `eviction_lab_national` | Eviction Lab, Princeton University | hazard_eviction | national filing estimates (paraphrased) |
| `cfpb_rent_help`, `cfpb_facing_eviction` | CFPB | hazard_eviction | rent help; quote: many renters give up before court |
| `lsc_get_legal_help` | Legal Services Corporation, I Need Legal Help | hazard_eviction, docs_legal_readiness, topic_before_you_need_them | LSC funds civil legal aid for people with low incomes; the office finder. Civil only, so the text says "civil problems" |
| `fema_hsgp_fy2026` | FEMA, FY2026 Homeland Security Grant Program NOFO | hazard_attack_disruption | PDF; the UASI urban areas and allocations |
| `nws_dust_storms` | NWS, Dust Storms and Haboobs | hazard_dust_storm | quote; pull off, lights off, parking brake on, foot off the brake pedal; never stop in a lane |
| `cdc_valley_fever` | CDC, About Valley Fever | hazard_dust_storm | page fetch (cdc.gov refuses scripted clients), paraphrased |
| `usgs_sinkholes` | USGS Water Science School, Sinkholes | hazard_sinkhole | quote; the states with the most damage |
| `fl_dep_sinkhole_faq` | Florida Geological Survey, Sinkhole FAQ | hazard_sinkhole | call the insurer; only a licensed geologist or engineer can tell a true sinkhole; mark it off; edges keep slumping for about a day |
| `fbi_cde_arrests_2024` | FBI UCR, Crime Data Explorer | hazard_arrest_or_detention | the national arrest count for January–December 2024 (about 7.1 million), read from the Crime Data Explorer's own data endpoint (`/LATEST/arrest/national/all`, totals and counts); the web app needs a script-capable browser. The block says it counts arrests, not people |
| `aclu_stopped_by_police` | ACLU, Know Your Rights: Stopped by Police | hazard_arrest_or_detention, docs_legal_readiness | stay calm, hands visible, say you want to stay silent and want a lawyer; calls from custody may be heard except calls to a lawyer (paraphrased) |
| `nlg_mass_defense` | National Lawyers Guild, Mass Defense Resources | hazard_arrest_or_detention | chapters run legal hotlines (paraphrased) |
| `nia_affairs_checklist` | National Institute on Aging, Getting Your Affairs in Order Checklist | arrest block, bucket_medical_emergency, the documents and legal steps, the circle, the Learn article | quote; will, powers of attorney for money and for health care, advance directives, tell someone where the papers are |

### Rare families, strategic sites and the EMP paragraph

| id | Source | Used in | How it was read and checked |
| --- | --- | --- | --- |
| `missilesonourland_2023` | Princeton SGS, The Missiles on our Land | family_nuclear, topic_strategic_sites | the Air Force expects the missiles to be targets; fallout depends on the winds; the public map |
| `dod_mirta_points` | DoD, MIRTA installation points | family_nuclear, topic_strategic_sites | the public list of military sites (ArcGIS feature service) |
| `census_cbsa_pop_2024` | Census Bureau, metro population estimates 2024 | family_nuclear, topic_strategic_sites | the largest metros and those over a million |
| `usc_10_2674` | 10 U.S.C. § 2674(f)(2) (Cornell LII) | family_nuclear | the definition of the National Capital Region |
| `nnsa_locations` | NNSA, Locations | family_nuclear, topic_strategic_sites | the national nuclear labs and plants |
| `eia_refinery_capacity_2026` | EIA Refinery Capacity Report, Table 3 | family_nuclear | PDF; large refineries by state |
| `fema_nuclear_72h_2023` | FEMA, Planning for the First 72 Hours (March 2023) | family_nuclear, topic_strategic_sites | PDF, quote; get inside, stay inside at least 24 hours, tune in; fallout is most dangerous in the first hours and travels downwind |
| `swpc_power_grid` | NOAA SWPC, Electric Power Transmission | family_solar_storm | the March 1989 storm's nine-hour blackout in Quebec |
| `morina_2019_carrington` | Moriña et al. 2019, Scientific Reports (CC BY 4.0) | family_solar_storm | the range of Carrington-class estimates |
| `nerc_tpl007_benchmark` | NERC TPL-007 benchmark event | family_solar_storm | the geomagnetic-latitude scaling (stronger toward the poles) |
| `nasa_tunguska_2019` | NASA, Tunguska Revisited | family_solar_storm | quote; regional-scale impacts every millennia, not centuries |
| `eia_maria_2017`, `npr_maria_2018` | EIA Today in Energy; NPR | family_long_blackout, topic_validation | quote (EIA): all 1.57 million PREPA customers out; about 11 months to restore every home |
| `powermag_epri_2019_hemp` | POWER magazine on EPRI's 2019 HEMP report | family_long_blackout | EPRI's own report page returned only its home page to a scripted read, so the finding (little harm to large transformers; no months-long nationwide blackout) is cited through POWER's report of it |
| `pry_2015_emp_testimony` | Statement for the record, House Oversight, 2015-05-13 | family_long_blackout | the "9 of 10 Americans" figure rests on an assumed nationwide year-long blackout; no published model is cited for it, which is what the block says |
| `cisa_volt_typhoon_2024` | CISA advisory AA24-038A | family_war_infrastructure | actors working for China's government got into US communications, energy, transport and water systems, to be ready to disrupt them in a conflict |
| `start_poicn` | START, POICN database | family_cbrn | 517 incidents worldwide in 1990–2016, failed attempts and plots included (the review's "1990–2017" is corrected) |
| `ready_gov_biohazard` | FEMA / Ready.gov, Biohazard Exposure | family_cbrn | follow doctors and public health officials; avoid crowds (paraphrased) |
| `duke_2021_pandemics` | Duke Global Health Institute on Marani et al. 2021 | family_severe_pandemic | the study's article on PMC shows a CAPTCHA to scripted clients, so the family block cites Duke's release of the same study; `marani_2021_pandemics` is unchanged |
| `cam_2022_volcano_risk` | University of Cambridge news (CC BY-NC-SA 4.0) | family_large_eruption | about 1 in 6 for a magnitude-7 eruption somewhere in the next 100 years; 1815 and the year without a summer. Paraphrased only |
| `usgs_yvo_supervolcano` | USGS Yellowstone Volcano Observatory | family_large_eruption | about 1 in 730,000 a year for a Yellowstone super-eruption |
| `fdic_deposit_insurance`, `fdic_history_1930s` | FDIC | family_financial_crisis | quotes; $250,000 per depositor per bank; the 1933 bank holiday |
| `fbi_active_shooter_2024` | FBI 2024 report (via Internet Archive) | family_mass_violence | read from the Internet Archive capture of the FBI's own report page (2026-09-04; an earlier capture of 2026-03-24 agrees); the registry points at the capture. Neutral title, since the report's own title uses a word the validator reserves for the one firearm item |

### Long horizon and the two Learn articles

| id | Source | Used in | How it was read and checked |
| --- | --- | --- | --- |
| `pnnl_2015_rainwater` | PNNL-24347 (2015), for DOE | topic_long_horizon | PDF, quote; states set the rules on collecting rain, and they vary widely. Its 2015 state-by-state details are out of date and are not used |
| `cdc_rainwater_collection` | CDC, Collecting Rainwater and Your Health (2024-07-23) | topic_long_horizon | read in a browser page (cdc.gov refuses scripted clients); quote matched in the page text; rainwater for plants you do not eat |
| `vdh_storm_wells` | Virginia Department of Health, Before and After the Storm (2024-10-01) | topic_long_horizon | quote; the well pump stops without power; a licensed electrician connects any generator |
| `wsc_wellcare_help_2025` | Water Systems Council (wellcare), Emergency Preparedness for Homeowners with Water Wells (April 2025) | topic_long_horizon | a trade body's sheet, used because it is the only readable source found that names a hand pump. The Virginia, Ohio and Michigan health pages, NEHA's fact sheet and the EPA-funded PrivateWellClass answer name generators and stored water, not hand pumps. The site rate-limits scripted clients; the PDF was read through a page fetch and its text extracted. Paraphrased |
| `lehi_fuel_storage` (re-read) | Lehi City Fire Department | topic_long_horizon | International Fire Code limits (25 gallons, 10 in an attached garage, none in a basement), rotation, "consult your local fire department" |
| `rdpo_emergency_toilet` (re-read) | RDPO, Emergency Toilet project | topic_long_horizon | weeks or months without flushing after a strong earthquake; the twin-bucket system |
| `cdc_botulism_home_canning` | CDC, Home-Canned Foods (2024-04-25) | topic_long_horizon | browser page, quote matched; low-acid foods need a pressure canner; tested recipes; Extension services help with growing and preserving food |
| `nchfp_home` | National Center for Home Food Preservation (UGA Extension) | topic_long_horizon | research-based home canning recommendations |
| `cdc_managing_stress` | CDC, Managing Stress (2026-05-12) | topic_long_horizon | browser page, quote matched; regular sleep times, breaks from the news, talk with people you trust |
| `rr_design_decision_log_2026` | Ready Reckoner, `docs/DESIGN.md` §14 decision log | topic_validation, topic_strategic_sites | the 22-event backtest (covered 6, partial 5, short 10, not modelled 1) and "a public source per site". Read on the `v0.2` branch (pushed to GitHub); the entry reaches main with the v0.2.0 merge, so the registry points at main and stores no quote |

### The Deviant Ollam lessons

| id | Source | Used in | How it was read and checked |
| --- | --- | --- | --- |
| `ollam_2022_lawyer_passport_locksmith_gun` | Deviant Ollam, SAINTCON 2022 keynote (video and slides) | the three free steps, docs_effak, bucket_comms, topic_mental_health, plan_communication, hazard_arrest_or_detention, topic_before_you_need_them | the transcript read from the video's captions in a browser page, and the slide PDF from deviating.net. Principles and checklist steps only, never numbers; paraphrased. The registry title leaves out the talk's own title, which ends with a word the validator reserves for the one firearm item; the id keeps it (footnote ids are not checked as words). **The supply worktree adds the same id with an identical record: keep one when merging** |
| `state_dept_passport_card` | State Department, Get a Passport Card | docs_effak, topic_before_you_need_them | travel.state.gov refuses scripted clients; read from the Internet Archive capture of the same URL (2026-05-15). Quote: proof of citizenship and identity; TSA accepts it on domestic flights; a certified birth certificate carries the issuing office's seal |
| `state_dept_child_passport` | State Department, Apply for a Child's Passport Under 16 | docs_effak, topic_before_you_need_them | read from the Internet Archive capture of the same URL (2026-05-23): children may get a book, a card or both, with both parents' approval |
| `ftc_2008_locksmith` | FTC press release, 2008-05-30 | security_lockout_plan, topic_before_you_need_them | research a locksmith before you need one and save the number; some who advertise are not local or trained. The FTC's consumer article it links to has been withdrawn, so it is not cited |
| `cisa_data_backup_2012` | US-CERT (now CISA), Data Backup Options | docs_effak, topic_before_you_need_them | PDF, quote; the 3-2-1 rule (three copies, two kinds of media, one away from home) |
| `ready_gov_cybersecurity` (re-read) | FEMA / Ready.gov, Cybersecurity | docs_effak, topic_before_you_need_them | quote re-matched: use a password manager and two methods of verification; back up to encrypted storage |
| `dhs_stop_the_bleed` (re-read) | DHS, Stop the Bleed | bucket_medical_emergency, topic_before_you_need_them | ask the health department, hospitals, EMS, fire or police about training. It does not say classes are "free or low-cost", so bucket_medical_emergency no longer does |

No agency page found today says to keep account backup codes on paper or with a trusted person
(CISA's "Turn On MFA" and the FTC's two-factor article do not mention backup codes), so that step
cites the talk as a principle.

### The state table

`content/tables/state_registries.toml` (rules: `docs/CONTENT_STANDARDS.md` §10). Each row's
agency address was opened on 2026-09-26. Where the agency site refused scripted clients, the
Internet Archive's latest capture of the same address was read: Arizona DEMA (2026-08-31),
Massachusetts MEMA (2026-08-31), New York DHSES (2026-09-12), Rhode Island EMA (2026-08-31),
Tennessee TEMA (2026-08-16), Kentucky EM (2026-09-15) and ReadyNH (2026-08-31), plus the Rhode
Island registry page (2026-05-08) and the NY-Alert sign-up page (2026-08-11). Every other address
was read directly, as a download or a page fetch.

- **Confirmed state-level tools** (named with their address): zone lookups in Florida, Georgia,
  North Carolina, South Carolina, Virginia and Maryland, and Hawaii's tsunami evacuation zones;
  registries in Florida, Texas (STEAR), New Jersey (Register Ready) and Rhode Island; alert
  sign-ups in New York (NY-Alert), DC (AlertDC), New Hampshire (through ReadyNH), Washington (the
  state's alert sign-up list) and Michigan (MI Ready). Vendor-run alert portals are linked through
  the state's own page.
- **Dropped after checking:** Utah's old special-needs registry domain now hosts an unrelated
  gambling site; North Dakota's registry page answers 404; Delaware's registry domain no longer
  resolves. Those rows send the household to its local office.
- **Refill rules:** Healthcare Ready's review (`healthcare_ready_refill_laws`, re-read) for every
  row. Florida's Board of Pharmacy page and Healthcare Ready agree once both are read (72 hours day
  to day; up to 30 days in a declared emergency), as do DC's two sources, so neither says "rules
  differ". Virginia and North Carolina's sources disagree, so their rows say so and cite both. South
  Carolina's second source (NACDS) gives a shorter limit, which the row notes. Minnesota's and
  Missouri's categories were corrected against the review. Puerto Rico has no confirmed registry,
  alert system or refill rule, and its row says so, pointing to Medicare's disaster drug rule.

### Re-read and re-dated

Read again on 2026-09-26 for sentences written today, with the retrieved date moved: 20 Ready.gov
pages (`ready_gov_kit`, `_food`, `_power_outages`, `_disability`, `_older_adults`, `_evacuation`,
`_shelter`, `_heat`, `_plan`, `_family_comm_card`, `_pandemic`, `_alerts`, `_low_cost`,
`_earthquakes`, `_radiation`, `_public_spaces`, `_tornadoes`, `_hurricanes`, `_chemical`,
`_cybersecurity`), `cdc_co_basics` and `cdc_potassium_iodide` (quotes re-matched in a browser
page), `epa_diy_air_cleaners`, `epa_wildfire_indoor_air`, `epa_asheville_boil_notice_2024`,
`ornl_eagle_i_outages`, `nws_turn_around_dont_drown`, `dhs_stop_the_bleed`,
`healthcare_ready_refill_laws`, `hcfl_ev_safety`, `florida_dem_medication`, `rdpo_emergency_toilet`,
`lehi_fuel_storage`, `fri_nuclear_risk_2024` and `rr_research_risk_model`. Every stored quote with
the new date on an entry this branch added or re-dated was matched again: in the page as
downloaded, in the extracted PDF text (FEMA, ASPR TRACIE, PNNL, US-CERT), in the page as a browser
shows it (cdc.gov) or in the Internet Archive capture (travel.state.gov).
`ready_gov_public_spaces` is retitled "Mass Gathering Incidents (Run. Hide. Fight.)", the page's
current title.

### Not cited, and why

- **FEMA, *Protection in the Nuclear Age* (1985).** The strategic-sites research found it the
  closest public precedent for "Why here", but the only copy found is on a private website, so it
  is not cited; the strategic-sites article rests on current sources instead.
- **Flex Your Rights.** The brief names it for know-your-rights text; the organisation closed on
  2023-12-31, so the arrest block cites the ACLU and the National Lawyers Guild.
- **Pages that could not be read:** EPRI's HEMP report page (cited through POWER magazine instead),
  the ASHP, NCPC and ABA lawyer-referral pages (blocked), the PNAS article on PMC (a CAPTCHA; Duke's
  release is cited instead) and travel.state.gov live (read through captures). The NCSL rainwater
  map answered 404 at the address tried; the supply worktree has since found its current address
  (ncsl_rainwater), which this branch does not add, to avoid a second id for one source.

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
| Recovery and fraud (v0.2.0) | `ready_gov_recovering`, `fema_home_inspections`, `fema_disaster_fraud`, `fema_ihp`, `disasterassistance_gov`, `floodsmart_start_claim`, `ftc_scams_after_disasters`, `healthcare_ready_rx_open`, `usa_gov_replace_ids` |
| Family plan and forecasts (v0.2.0) | `cisa_pace_flyer`, `fema_back_to_school_2026`, `texasready_hurricanes`, `nws_aly_cold_safety`, `epa_clean_room` |
| Access and functional needs (v0.2.0) | `aspr_cmist`, `usfa_fire_safety_disabilities`, `ready_gov_your_language`, `aspr_tracie_hha_rule`, `tdem_stear`, `psegli_critical_care` |
| Clean air (v0.2.0) | `epa_clean_room`, `epa_protect_lungs_2026`, `epa_diy_air_cleaners`, `epa_wildfire_indoor_air`, `epa_children_wildfire_smoke` |
| New ranked hazards (v0.2.0) | `iii_homeowners_losses`, `iii_water_damage_protect`, `fema_dam_residual_risk_2018`, `fema_living_with_levees`, `fema_nfip_levees_2021`, `usace_nld`, `mo_sema_dam_failure`, `fcc_network_outage_2024`, `fda_drug_shortages`, `fda_drug_shortages_faq`, `fda_besaferx`, `fema_hsgp_fy2026`, `nws_dust_storms`, `cdc_valley_fever`, `usgs_sinkholes`, `fl_dep_sinkhole_faq` |
| Benefits, eviction and legal help (v0.2.0) | `me_dhhs_snap_2025`, `usda_hunger_hotline`, `dol_ucfe_furlough_2023`, `cfpb_shutdown_2013`, `cfpb_payday_loans`, `eviction_lab_national`, `cfpb_rent_help`, `cfpb_facing_eviction`, `lsc_get_legal_help`, `aclu_stopped_by_police`, `nlg_mass_defense`, `nia_affairs_checklist`, `fbi_cde_arrests_2024` |
| Rare families and strategic sites (v0.2.0) | `missilesonourland_2023`, `dod_mirta_points`, `census_cbsa_pop_2024`, `usc_10_2674`, `nnsa_locations`, `eia_refinery_capacity_2026`, `fema_nuclear_72h_2023`, `swpc_power_grid`, `morina_2019_carrington`, `nerc_tpl007_benchmark`, `nasa_tunguska_2019`, `eia_maria_2017`, `npr_maria_2018`, `powermag_epri_2019_hemp`, `pry_2015_emp_testimony`, `cisa_volt_typhoon_2024`, `start_poicn`, `ready_gov_biohazard`, `duke_2021_pandemics`, `cam_2022_volcano_risk`, `usgs_yvo_supervolcano`, `fdic_deposit_insurance`, `fdic_history_1930s`, `fbi_active_shooter_2024` |
| Long horizon (v0.2.0) | `pnnl_2015_rainwater`, `cdc_rainwater_collection`, `vdh_storm_wells`, `wsc_wellcare_help_2025`, `lehi_fuel_storage`, `rdpo_emergency_toilet`, `cdc_botulism_home_canning`, `nchfp_home`, `cdc_managing_stress` |
| Documents, identity and accounts (v0.2.0) | `state_dept_passport_card`, `state_dept_child_passport`, `cisa_data_backup_2012`, `ready_gov_cybersecurity`, `ftc_2008_locksmith`, `nia_affairs_checklist`, `ollam_2022_lawyer_passport_locksmith_gun` (principles only) |
| Emergency refills by state (v0.2.0) | `healthcare_ready_refill_laws`, `nacds_2018_emergency_refills`, `fl_bop_emergency_refills`, `tx_pharmacy_disaster_2024`, `medicare_drugs_disaster` |
| Our own documents | `rr_design_decision_log_2026` (the backtest and the nuclear-wording decisions) |

## Requested

One placeholder is open: `county_boil_water_records` (see "Requested by consequence"), which
waits for the data workstream to name a source. Add new rows here as described at the top.
