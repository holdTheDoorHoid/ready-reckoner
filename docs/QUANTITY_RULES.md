# Quantity rules

The quantity rules in `crates/rr-supply` turn a household and its bucket targets into requirement
lines (`RequirementLine` in `docs/ENGINE-API.md`). A catalogue item names the rule that sizes it in
`quantity_rule`; `rr-content` checks that every name used in `content/items/*.toml` appears in the
table below (or is `once`).

**Machine-read.** `rr-content` parses the table under "## Rules": one row per rule, the first cell
is the rule id in backticks. Only that table is parsed. To ask for a rule that is missing, add a row
with `requested by content` in the last column; the supply workstream implements it or answers in
the row.

## How lines are named

A line's `id` says what kind of line it is, so no consumer adds up the same need twice:

| Id pattern | Kind | Meaning |
| --- | --- | --- |
| `bucket.rule` | need | The amount this household needs for that bucket. |
| `bucket.rule.person_N` | need | The same, for one person (N is the 1-based position in `PlanInput.people`); used for get-home bags, where each commuter's bag is sized to their own trip. |
| `bucket.rule.alt.variant` | alternative | Another way to meet the need line `bucket.rule` (for example, the cost of the food need as freeze-dried meals). Never add it to the need. |
| `bucket.rule.optional` | optional | Worth having for some households, not needed to meet the target (keeping a fridge running). |
| `bucket.rule.note` | note | Information for the plan and the packet (what a power station covers); not something to buy. |

`quantity` is always for the whole household, already multiplied out by `per` (ENGINE-API). Units are
fixed per rule (column "unit"); an item that meets a rule either uses the same unit or gives the
conversion (`volume_l_per_unit` for water containers, `energy_kcal_per_unit` for food). Lines in
watt-hours (`Wh`) are meant for items priced per Wh (power banks, power stations).

Every number in a formula comes from the constants registry (`crates/rr-supply/src/constants.toml`,
exposed as `rr_supply::constants()` for the expert view), and every line cites the sources of the
constants it used. `Prior` marks a planning estimate rather than a published figure.

Inputs: `people`, `pets`, `housing`, `mobility`, `finances`, `dials.water_level` come from `PlanInput`;
`target(b)` is the bucket's design target from `BucketAssessment` (days, months, or the evacuation /
readiness shape); `hot` is true when the county has at least 30 days a year at or above 95 °F under the
chosen climate dial (optional context; temperate when absent); `lat` is the county centroid latitude
(optional context).

## Rules

| rule | family | bucket | inputs | formula | unit | per | citations | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `water_gallons` | water | water_out | people, pets, water_level, hot, target(water_out) | days = min(target, 14); per day = Σ people (level allowance: survival 0.8, basic 1.0, comfortable 4.0 gal; drinking share ×2 when hot) + 0.29 per pregnant or nursing person + 0.25 per formula-fed infant + dogs 40 lb × 1 oz/lb/128 + cats 10 lb × 0.8 oz/lb/128 + small pets 8 oz/128; quantity = per day × days. Beyond 14 days see `water_treatment_capacity` | gallon | person | ready_gov_water, cdc_water_storage, sphere_2018, iom_dri_water_2005, aap_formula_amounts, petmd_pet_water, byu_longer_term_storage_2019 | rr-supply |
| `water_treatment_capacity` | water | water_boil, water_out | as `water_gallons` | water_boil: drinking share per day × target days (the water that must be made safe to swallow); water_out: per-day need × (target − 14) days, only when the target is longer than 14 days ("a filter plus a water source beats storing more") | gallon | household | cdc_water_treatment, epa_emergency_disinfection, byu_longer_term_storage_2019, oregon_b2wr_toolkit | rr-supply |
| `boil_fuel` | water | water_boil (optional line) | treatment gallons | litres to boil ÷ 27.5 L per lb of propane (Prior 25–30); outdoors only | lb | household | eia_btu, prior_supply_estimate, cdc_co_poisoning | rr-supply |
| `livestock_water` | water | water_out | pets.large_animals, target(water_out) | 25 L (20–30) per large animal per day × days ÷ 3.785 | gallon | pet | sphere_2018 | rr-supply |
| `food_kcal` | food | supplies | people, target(supplies) | Σ people DGA 2020–2025 Table A2-2 moderately active, average of men's and women's rows over the band's ages (toddler 1,067, child 1,667, teen 2,280, adult 2,262, senior 2,000; infants 0, see `infant_formula`) + 400 per pregnant or nursing person; × days | kcal | person | dga_2020_2025 | rr-supply |
| `food_cost_estimate` | food | supplies (alternative lines of `food_kcal`) | kcal, people | pantry: person-days × $8.44 × USDA household-size factor; bulk staples: person-days × $2.50 ($2.15–2.85); freeze-dried: kcal ÷ 2,000 × $24 ($9–39). Cost comparison only; no item uses this rule | usd | household | usda_tfp_aug_2026, church_hsc_order_form_2026, byu_longer_term_storage_2019, retail_prices_2026_09 | rr-supply |
| `food_kit_check` | food | supplies (note) | people | days a retail "30-day" kit really lasts = 30 × 1,290 kcal ÷ household average kcal per person-day (kits supply 1,290–1,730 kcal/day) | day | person | retail_prices_2026_09, dga_2020_2025 | rr-supply |
| `long_term_staples` | food | supplies (alternative lines of `food_kcal`, target over 30 days) | people, target(supplies) | for days beyond 30: BYU 2019 per-adult-year list × (days − 30)/365 × Σ child shares (50 % age ≤ 3, 70 % 4–6, 90 % 7–10, 100 % 11+, age + 1 year); one line each for grains, legumes, dry milk, sugar, fruit and vegetables, salt and leavening (lb) and oil (gal) | lb (oil: gallon) | person | byu_longer_term_storage_2019, ensign_2006_year_supply, usu_food_storage | rr-supply |
| `infant_formula` | food | supplies | formula-fed infants (`dietary` mentions formula), target(supplies) | 32 oz prepared formula per infant per day (AAP maximum) × days | oz | person | aap_formula_amounts, cdc_infant_feeding_disaster | rr-supply |
| `nursing_supplies` | special needs | supplies | infants not on formula | 1 manual pump + 2 boxes of nursing pads per breastfed infant | kit | person | cdc_infant_emergency_checklist | rr-supply |
| `pet_food` | food | supplies | dogs, cats, small pets, target(supplies) | pets × days (no per-pet calorie figure is published; feed what the pet eats now) | pet_day | pet | ready_gov_pets, aspca_disaster_prep | rr-supply |
| `medication_days` | medication | medication | people with `daily_rx` or `refrigerated_rx`, target(medication) | days = target clamped to 7–30 (14 when no target); quantity = days × people on prescriptions | person_day | person | cdc_pregnancy_emergency, cdc_diabetes_emergency, red_cross_survival_kit, florida_dem_medication, healthcare_ready_refill_laws | rr-supply |
| `rx_cold_storage` | medication | medication | people with `refrigerated_rx`, target(power) | days of cold storage = power target (medication target when no power target) | day | household | ready_gov_power_outages, cdc_insulin_emergency | rr-supply |
| `antibiotics_none` | medication | medication | — | always 0: no antibiotic quantity is ever sized; the line points to the clinician card | course | household | cdc_antibiotics_aware, fda_fish_antibiotics, fda_expired_medicines, cdc_yellow_book_travel_kits | rr-supply |
| `epinephrine_check` | medication | medication | people with `epinephrine` | 1 per person who carries an auto-injector (check dates; ask the prescriber how many to keep) | person | person | ready_gov_disability | rr-supply |
| `medical_device_wh` | power | power | people's `powered_device`, target(power) | CPAP 170 Wh per night (96 without the humidifier); oxygen concentrator 300 W × 24 h (Prior; use the label); other device watts × 24 h (Prior); × days | Wh | person | sil_cpap_power, ready_gov_disability, prior_supply_estimate | rr-supply |
| `lights` | power | power | people | 1 light per person aged 4 and over, at least 1 (Prior) | light | person | ready_gov_kit, prior_supply_estimate | rr-supply |
| `generator_fuel` | power | power | `backup_power` = generator, target(power) | 2.8 gal/day (¼ load; 7.1 at full load) × days, capped at 25 gal stored (10 in an attached garage, none indoors) | gallon | household | generator_2200w_spec, lehi_fuel_storage, cdc_co_poisoning | rr-supply |
| `power_station_wh` | power | power (optional) | devices, phones, target(power) | (device Wh/day + phones × 15 Wh/day) × days; a ~1,000 Wh station gives about 910 usable Wh (85 %, Prior) | Wh | household | sil_cpap_power, retail_prices_2026_09, prior_supply_estimate | rr-supply |
| `fridge_wh` | power | power (optional) | target(power) | 995 Wh/day (ENERGY STAR median top-freezer; chest freezer 597) × days; a closed fridge holds about 4 hours, a full freezer about 48 | Wh | household | energy_star_refrigerators, energy_star_freezers, ready_gov_food, ready_gov_power_outages | rr-supply |
| `well_pump_wh` | power | power (optional) | `water` = well, target(power) | 1,250 W (1,000–1,500) × 1 h/day × days; starting needs 2–3 × the running watts (Prior; check the nameplate) | Wh | household | prior_supply_estimate | rr-supply |
| `solar_panel_watts` | power | power (note) | lat, essential load | panel W = essential Wh/day ÷ December Wh per 100 W for the latitude band (NREL PVWatts cities; Prior proxy) × 100 | watt | household | nrel_pvwatts_v8, prior_supply_estimate | rr-supply |
| `wheelchair_battery` | special needs | power | people with `mobility` = wheelchair | 1 spare battery per wheelchair user (if the chair is powered) | battery | person | ready_gov_disability | rr-supply |
| `noaa_radio` | comms | comms | — | 1 per household | radio | household | ready_gov_kit, noaa_nwr | rr-supply |
| `phone_power_wh` | comms | comms | people aged 13+, target(power) | phones (1 per person 13+) × 15 Wh/day (Prior, 10–20) × days without power | Wh | person | prior_supply_estimate, ready_gov_kit | rr-supply |
| `two_way_radios` | comms | comms | people aged 13+ | 1 per person 13+ when there are at least 2; FRS needs no licence, GMRS $35 for 10 years | radio | person | fcc_frs, fcc_gmrs, ecfr_47_1_1102 | rr-supply |
| `contact_cards` | comms | comms | people aged 4+ | 1 written contact card per person aged 4 and over | card | person | ready_gov_plan, ready_gov_low_cost | rr-supply |
| `local_map` | comms | comms | — | 1 paper map of the area per household | map | household | ready_gov_kit | rr-supply |
| `cash_days` | comms | comms | target(comms) | days of cash for basics = target clamped to 3–14; the household chooses the daily amount (no agency gives one) | day | household | ready_gov_financial_preparedness, fema_effak | rr-supply |
| `toilet_buckets` | sanitation | water_out | — | 2 buckets (pee and poo) with a seat per household | bucket | household | rdpo_emergency_toilet, oregon_b2wr_toolkit | rr-supply |
| `toilet_bags` | sanitation | water_out | people over 1, target(water_out) | 0.45 bags (0.3–0.6, Prior) per person-day, counting both bags of a double-bagged load | bag | person | rdpo_emergency_toilet, oregon_b2wr_toolkit, prior_supply_estimate | rr-supply |
| `toilet_cover_material` | sanitation | water_out | people over 1, target(water_out) | 1 cup (0.5–1.5, Prior) of sawdust, shredded paper or similar per person-day | cup | person | rdpo_emergency_toilet, prior_supply_estimate | rr-supply |
| `soap_grams` | sanitation | supplies | people, target(supplies) | (250 g bathing + 200 g laundry) per person-month × days ÷ 30 | gram | person | sphere_2018 | rr-supply |
| `menstrual_products` | sanitation | supplies | adults and teens not pregnant or nursing, target(supplies) | 50 % of them (sex is not asked) × 20 products (15–30) per cycle × (2 cycles, + 1 per 28 days beyond a month) | product | person | cdc_period_disaster, sphere_2018, prior_supply_estimate | rr-supply |
| `diapers` | sanitation | supplies | infants and toddlers, target(supplies) | infant 10/day (6–12), toddler 6/day (Prior) × days | diaper | person | cdc_infant_emergency_checklist, sutter_health_diapers, prior_supply_estimate | rr-supply |
| `baby_wipes` | sanitation | supplies | infants and toddlers, target(supplies) | 2 packs per child per 2 weeks (at least 2) | pack | person | cdc_infant_emergency_checklist | rr-supply |
| `first_aid_kit` | first aid | medical_emergency | people | kits = max(1, people ÷ 4): the Red Cross family-of-four list, consumables scaled per extra person | kit | household | red_cross_first_aid_kit | rr-supply |
| `otc_medicines` | first aid | medical_emergency | people, target(supplies) | 4 kinds (pain reliever, anti-diarrhea, antacid, laxative) × max(1, days ÷ 14 × people ÷ 4) packages (Prior scaling) | package | household | ready_gov_kit, prior_supply_estimate | rr-supply |
| `n95_masks` | first aid | medical_emergency | people aged 4+ | 1 per person per day × 5 days of smoke or outbreak (Prior) | mask | person | cdc_masks, ready_gov_kit, prior_supply_estimate | rr-supply |
| `thermometer` | first aid | medical_emergency | infants | 1 oral thermometer + 1 infant thermometer when there is a baby | thermometer | household | red_cross_first_aid_kit, cdc_infant_emergency_checklist | rr-supply |
| `ors_packets` | first aid | medical_emergency | people, target(supplies) | 3 packets per person per 2 weeks (Prior), each mixed into 1 L of safe water | packet | person | cdc_cholera_ors, cdc_yellow_book_travel_kits, prior_supply_estimate | rr-supply |
| `battery_fan` | thermal | thermal (heat) | people | 1 per household + 1 per person 65+, infant or pregnant (Prior); fans help only below 90 °F indoors | fan | household | cdc_heat_health, ready_gov_heat, prior_supply_estimate | rr-supply |
| `cooling_towel` | thermal | thermal (heat) | people | 1 per person (Prior) | towel | person | cdc_heat_health, prior_supply_estimate | rr-supply |
| `cooling_plan` | thermal | thermal (heat) | — | 1 plan: nearest cooling centre (dial 2-1-1), coolest room, check-ins | plan | household | cdc_heat_health, ready_gov_heat | rr-supply |
| `sleeping_bag_or_blanket` | thermal | thermal (cold) | people | 1 per person, rated for the coldest nights | item | person | ready_gov_kit, sphere_2018 | rr-supply |
| `warm_layers` | thermal | thermal (cold) | people | 1 set of warm layers (coat, hat, gloves) per person | set | person | ready_gov_kit, cdc_winter_storm_safety | rr-supply |
| `warm_room_plan` | thermal | thermal (cold) | housing.heating | 1 plan: one warm room, towels under doors, blankets over windows, never unvented combustion indoors | plan | household | cdc_winter_storm_safety, cdc_co_poisoning | rr-supply |
| `smoke_alarm` | fire | fire | housing.kind, basement, alarms.smoke = false | 1 per level: apartments and mobile homes 1, houses 2, + 1 for a basement (Prior level count) | alarm | household | ready_gov_home_fires, prior_supply_estimate | rr-supply |
| `co_alarm` | fire | fire | housing.kind, basement, alarms.co = false | 1 per level (same level count) | alarm | household | ready_gov_power_outages, cdc_co_poisoning, prior_supply_estimate | rr-supply |
| `fire_extinguisher` | fire | fire | alarms.extinguisher = false | 1 per household (Prior) | extinguisher | household | prior_supply_estimate | rr-supply |
| `neighbour_contacts` | security | security | — | 2 neighbours' numbers swapped, and who checks on whom (Prior) | contact | household | prior_supply_estimate | rr-supply |
| `document_kit` | documents | home_loss | — | 1 Emergency Financial First Aid Kit (4 parts: ID, financial and legal, medical, contacts), copies kept safe and updated yearly | kit | household | fema_effak | rr-supply |
| `insurance_review` | documents | home_loss | finances.insurance, contributing hazards | 1 decision each for home or renters, flood (30-day wait; up to $250,000 building / $100,000 contents), earthquake, when not held | decision | household | fema_effak, fema_nfip, floodsmart_limits, ready_gov_financial_preparedness, ready_gov_earthquakes | rr-supply |
| `emergency_fund_months` | documents | income | target(income), finances | months = income target (3 when absent; planners suggest 3–6, the CFPB says it depends); dollars in the text when monthly expenses are given. Savings track, not the supplies budget | month | household | finra_emergency_fund, stlouisfed_emergency_fund, cfpb_emergency_fund | rr-supply |
| `get_home_bag` | get home | get_home | each person's `commute` (distance > 0) | 1 bag per commuter: walking shoes, light, map, cash, weather layer, water and food for the walk; kept in the car (car commuters) or at work | bag | commuter | ready_gov_kit_2020, ready_gov_kit | rr-supply |
| `get_home_water` | get home | get_home | commute distance, hot | hours = miles ÷ 3 mph (Prior; 2–2.4 when loaded); litres = hours × 0.5 L/h (Prior; 0.71 L/h, NIOSH, when hot); past 2.5 L carry a filter | litre | commuter | prior_supply_estimate, niosh_heat_stress, mutcd_walking_speed | rr-supply |
| `car_kit` | get home | get_home | mobility.vehicles | 1 car emergency kit per vehicle (jumper cables, light, warm clothes, blanket, water, snacks) | kit | household | ready_gov_winter_weather | rr-supply |
| `go_bag` | evacuate | evacuate | people aged 4+ | 1 bag per person aged 4 and over (babies' things go in a parent's bag) | bag | person | wa_emd_prepare_in_a_year, ready_gov_kit, cdc_evacuation_psa | rr-supply |
| `go_bag_water` | evacuate | evacuate | people, water_level, hot, target(evacuate).days_away | per-day allowance × min(days away, 3) days (Red Cross three-day evacuation supply) | gallon | person | red_cross_survival_kit, ready_gov_water | rr-supply |
| `go_bag_food` | evacuate | evacuate | people, target(evacuate).days_away | DGA kcal per day × min(days away, 3); food that needs no cooking | kcal | person | red_cross_survival_kit, dga_2020_2025, wa_emd_prepare_in_a_year | rr-supply |
| `pet_carrier` | evacuate | evacuate | dogs, cats, small pets | 1 carrier per pet | carrier | pet | aspca_disaster_prep, ready_gov_pets | rr-supply |
| `pet_go_water` | evacuate | evacuate | dogs, cats, small pets | pet water per day × 7 days (ASPCA); rotate every 2 months | gallon | pet | aspca_disaster_prep, petmd_pet_water | rr-supply |
| `pet_go_food` | evacuate | evacuate | dogs, cats, small pets | pets × 10 days (ASPCA 7–10); rotate every 2 months | pet_day | pet | aspca_disaster_prep | rr-supply |
| `fuel_half_tank` | evacuate | evacuate | mobility.vehicles | vehicles kept at least half full (half charged for an EV); full when leaving looks likely | vehicle | household | ready_gov_evacuation | rr-supply |
| `evacuation_assistance_plan` | special needs | evacuate | people with `mobility` limited or wheelchair | 1 transport plan per person who needs help to leave | person | person | ready_gov_older_adults, ready_gov_disability | rr-supply |
| `once` | generic | any | — | 1 per household (free actions and one-off items); no line is emitted, the plan sizes it | item | household | (the item's own) | built in |
| `per_person` | generic | any | people | 1 per person; no line is emitted, the plan sizes it with `rr_supply::generic_quantity` | item | person | (the item's own) | rr-supply |
| `per_vehicle` | generic | any | mobility.vehicles | 1 per vehicle; no line is emitted | item | household | (the item's own) | rr-supply |
| `per_pet` | generic | any | dogs, cats, small pets | 1 per pet; no line is emitted | item | pet | (the item's own) | rr-supply |
| `per_commuter` | generic | any | people with a commute | 1 per commuter; no line is emitted | item | commuter | (the item's own) | rr-supply |

## Citation ids rr-supply uses

These ids appear in the lines above. The content workstream owns `content/citations.toml`; where its id
for a source differs, tell the supply workstream (or add the row here) and the constants file follows.
`prior_supply_estimate` is a citation with `prior = true`: "Ready Reckoner planning estimate (supply)".
The full list with titles and URLs is the `[[source]]` table at the end of
`crates/rr-supply/src/constants.toml`.
