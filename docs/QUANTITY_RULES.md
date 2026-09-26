# Quantity rules

The quantity rules in `crates/rr-supply` turn a household and its bucket targets into requirement
lines (`RequirementLine` in `docs/ENGINE-API.md`). A catalogue item names the rule that sizes it in
`quantity_rule`; `rr-content` checks that every name used in `content/items/*.toml` appears in the
table below (or is `once`).

**Machine-read.** `rr-content` parses the one table in the Rules section below: one row per rule,
the first cell is the rule id in backticks. `rr-supply`'s own test checks the table lists exactly the rules the
crate implements (`rr_supply::RULE_IDS`) and that every line's unit matches the "unit" column. To
ask for a rule that is missing, add a row with `requested by content` in the last column; the
supply workstream implements it or answers in the row.

## How lines are named

A line's `id` says what kind of line it is, so no consumer adds up the same need twice
(`rr_supply::LineKind::of(id)` reads it):

| Id pattern | Kind | Meaning |
| --- | --- | --- |
| `bucket.rule` | need | The amount this household needs for that bucket. |
| `bucket.rule.person_N` | need | The same, for one person (N is the 1-based position in `PlanInput.people`): each commuter's get-home bag is sized to their own trip. |
| `bucket.rule.alt.variant` | alternative | Another way to meet the need line `bucket.rule` (for example the food need's cost as freeze-dried meals, or its days beyond the first month as bulk staples). Never add it to the need. |
| `bucket.rule.optional` | optional | Worth having for some households, not needed to meet the target (keeping a fridge running). |
| `bucket.rule.note` | note | Information for the plan and the packet (what a retail "30-day" kit really lasts); not something to buy. |

`quantity` is always for the whole household, already multiplied out by `per` (ENGINE-API). Each rule
has one unit (column "unit"); an item that meets a rule either uses the same unit or gives the
conversion (`volume_l_per_unit` for water containers and for a filter's rated capacity,
`energy_kcal_per_unit` for food). Lines in watt-hours (`Wh`) suit items priced per Wh (power banks,
power stations). `person_day` is one person's supply for one day; `pet_day` the same for a pet.

Every number in a formula comes from the constants registry (`crates/rr-supply/src/constants.toml`,
exposed as `rr_supply::constants()` for the expert view). Each line cites the sources of the
constants it used plus its bucket target's own sources; `Prior` marks a planning estimate
(citation `prior_supply_estimate`), and the line's sentence says "some amounts are estimates".

Inputs: `people`, `pets`, `housing`, `mobility`, `finances`, `dials.water_level` come from `PlanInput`;
`target(b)` is the bucket's design target from `BucketAssessment` (days, months, or the evacuation /
readiness shape). Two optional facts come from `rr_supply::SupplyContext`: `hot` is true when the
county has at least 30 days a year at or above 95 °F under the chosen climate dial (temperate when
absent), and `lat` is the county centroid latitude (no solar note when absent). Buckets missing from
the input get no lines, and a duration bucket with a target of 0 days gets none either.

## Rules

| rule | family | bucket | item_class | inputs | formula | unit | per | citations | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `water_gallons` | water | water_out | stored_water | people, pets, water_level, hot, target(water_out) | days = min(target, 14); per day = Σ people × level allowance (survival 0.8, basic 1.0, comfortable 4.0 gal; drinking share × 2 when hot, so basic is 1.75) + 0.29 per pregnant or nursing person + 0.25 per formula-fed infant + dogs 40 lb × 1 oz/lb ÷ 128 + cats 10 lb × 0.8 oz/lb ÷ 128 + small pets 8 oz ÷ 128; × days. Beyond 14 days see `water_treatment_capacity` | gallon | person | ready_gov_water, cdc_water_storage, sphere_2018, iom_dri_water_2005, aap_formula_amounts, petmd_pet_water | rr-supply |
| `water_treatment_capacity` | water | water_boil, water_out | water_treatment | people, pets, hot, target | water_boil: (people × 0.75 gal drinking share, × 2 when hot, + pregnancy/nursing + formula + pets) × target days, whatever the water level; water_out: the full per-day need × (target − 14) days, only when the target is longer than 14 days ("a filter plus a water source beats storing more") | gallon | household | cdc_water_treatment, epa_emergency_disinfection, byu_longer_term_storage_2019, oregon_b2wr_toolkit | rr-supply |
| `boil_fuel` | water | water_boil (optional) | stove_fuel | treatment gallons | litres to boil ÷ 27.5 L per lb of propane (Prior, 25–30); outdoors only | lb | household | eia_btu, prior_supply_estimate, cdc_co_poisoning | rr-supply |
| `livestock_water` | water | water_out | livestock_water | pets.large_animals, target(water_out) | 25 L (20–30) per large animal per day × days ÷ 3.785 | gallon | pet | sphere_2018, aspca_disaster_prep | rr-supply |
| `food_kcal` | food | supplies | food | people, target(supplies) | Σ people: DGA 2020–2025 Table A2-2, moderately active, men's and women's rows averaged year by year over the band (toddler 1,067, child 1,667, teen 2,280, adult 2,262, 65+ 2,009; babies 0, see `infant_formula`) + 400 per pregnant or nursing person; × days | kcal | person | dga_2020_2025 | rr-supply |
| `food_cost_estimate` | food | supplies (alternatives of `food_kcal`) | food_pantry, food_bulk_staples, food_freeze_dried | people, target(supplies) | pantry: person-days × $8.44 × USDA household-size factor (+20 % for 1 person … −10 % for 7+, babies not counted); bulk staples: person-days × $2.50 ($2.15–2.85); freeze-dried: kcal ÷ 2,000 × $24 ($9–39). Cost comparison only; no item uses this rule | usd | household | usda_tfp_aug_2026, church_hsc_order_form_2026, byu_longer_term_storage_2019, retail_prices_2026_09 | rr-supply |
| `food_kit_check` | food | supplies (note) | food_kit | people | days a retail "30-day" kit really lasts = 30 × 1,290 kcal ÷ household average kcal per person-day (kits supply 1,290–1,730 kcal a day) | day | person | retail_prices_2026_09, dga_2020_2025 | rr-supply |
| `long_term_staples` | food | supplies (alternatives of `food_kcal`, target over 30 days) | staples_grains, staples_legumes, staples_dry_milk, staples_sugar, staples_fruit_veg, staples_salt_leavening, staples_oil, staples_vitamin | people, target(supplies) | for the days beyond 30: BYU 2019 per-adult-year amount × (days − 30) ÷ 365 × Σ adult-equivalents (Ensign 2006 child shares with one year added: 50 % to age 3, 70 % 4–6, 90 % 7–10, 100 % 11+; babies 0) | lb, gallon, tablet | person | byu_longer_term_storage_2019, ensign_2006_year_supply, church_hsc_order_form_2026, usu_food_storage | rr-supply |
| `infant_formula` | food | supplies | infant_formula | formula-fed babies (`dietary` mentions formula), target(supplies) | 32 oz of prepared formula per baby per day (AAP maximum) × days | oz | person | aap_formula_amounts, cdc_infant_feeding_disaster, cdc_infant_emergency_checklist | rr-supply |
| `nursing_supplies` | special needs | supplies | nursing_supplies | babies not on formula | 1 manual pump + 1–2 boxes of nursing pads per breastfed baby | kit | person | cdc_infant_emergency_checklist | rr-supply |
| `pet_food` | food | supplies | pet_food | dogs, cats, small pets, target(supplies) | pets × days (no per-pet calorie figure is published; feed what the pet eats now) | pet_day | pet | ready_gov_pets, aspca_disaster_prep | rr-supply |
| `medication_days` | medication | medication | prescription_medicine | people with `daily_rx` or `refrigerated_rx`, target(medication) | days = target clamped to 7–30 (14 when there is no target); × people on prescriptions | person_day | person | florida_dem_medication, cdc_diabetes_emergency, cdc_pregnancy_emergency, red_cross_survival_kit, healthcare_ready_refill_laws | rr-supply |
| `rx_cold_storage` | medication | medication | medicine_cooler | people with `refrigerated_rx`, target(power) | days of cold storage = power target (the medication target when there is no power target) | day | household | ready_gov_power_outages, cdc_insulin_emergency | rr-supply |
| `antibiotics_none` | medication | medication | antibiotics_clinician_card | — | always 0: no antibiotic quantity is ever sized; the line points to the clinician card | course | household | cdc_antibiotics_aware, fda_fish_antibiotics, fda_expired_medicines, cdc_yellow_book_travel_kits | rr-supply |
| `epinephrine_check` | medication | medication | epinephrine_auto_injector | people with `epinephrine` | 1 per person who carries an auto-injector (check dates; ask the prescriber how many to keep) | person | person | ready_gov_disability | rr-supply |
| `medical_device_wh` | power | power | medical_device_power | people's `powered_device`, target(power) | CPAP 170 Wh a night (96 with the humidifier off); oxygen concentrator 300 W × 24 h (Prior; use the label); other device watts × 24 h (Prior); × days | Wh | person | sil_cpap_power, ready_gov_disability, ready_gov_power_outages, prior_supply_estimate | rr-supply |
| `lights` | power | power | light | people | 1 battery light per person aged 4 and over, at least 1 (Prior) | light | person | ready_gov_kit, prior_supply_estimate | rr-supply |
| `generator_fuel` | power | power | generator_fuel | `backup_power` = generator, target(power) | 2.8 gal a day (¼ load; 7.1 at full load) × days, capped at the 25 gal a home may store (10 in an attached garage, none indoors) | gallon | household | generator_2200w_spec, lehi_fuel_storage, cdc_co_poisoning, ready_gov_power_outages | rr-supply |
| `power_station_wh` | power | power (optional) | power_station | devices, people aged 13+, target(power) | (device Wh a day + phones × 15 Wh) × days; a 1,070 Wh station gives about 910 usable Wh (85 %, Prior) | Wh | household | sil_cpap_power, retail_prices_2026_09, energy_star_refrigerators, prior_supply_estimate | rr-supply |
| `fridge_wh` | power | power (optional) | fridge_power | target(power) | 995 Wh a day (ENERGY STAR median top-freezer; chest freezer 597) × days; a closed fridge holds about 4 hours, a full freezer about 48 | Wh | household | energy_star_refrigerators, energy_star_freezers, ready_gov_food, ready_gov_power_outages | rr-supply |
| `well_pump_wh` | power | power (optional) | well_pump_power | `water` = well, target(power) | 1,250 W (1,000–1,500) × 1 h a day × days; starting takes 2–3 × the running watts (Prior; check the nameplate) | Wh | household | prior_supply_estimate | rr-supply |
| `solar_panel_watts` | power | power (note) | solar_panel | lat, devices, people aged 13+ | panel W = essential Wh a day ÷ December Wh per 100 W for the latitude band (NREL PVWatts cities: under 36° N 382, 36–42° 261, 42–46° 219, 46° and north 121; Prior proxy) × 100 | watt | household | nrel_pvwatts_v8, energy_star_refrigerators, prior_supply_estimate | rr-supply |
| `wheelchair_battery` | special needs | power | wheelchair_battery | people with `mobility` = wheelchair | 1 spare battery per wheelchair user (if the chair is powered) | battery | person | ready_gov_disability | rr-supply |
| `noaa_radio` | comms | comms | weather_radio | — | 1 battery or hand-crank radio with NOAA Weather Radio per household | radio | household | ready_gov_kit, noaa_nwr | rr-supply |
| `phone_power_wh` | comms | comms | power_bank | people aged 13+, target(power) | phones (1 per person aged 13+) × 15 Wh a day (Prior, 10–20) × days without power (the comms target when there is no power target) | Wh | person | prior_supply_estimate, ready_gov_kit, ready_gov_earthquakes | rr-supply |
| `two_way_radios` | comms | comms | two_way_radio | people aged 13+ | 1 per person aged 13+ when there are at least 2; FRS needs no licence, GMRS $35 for 10 years | radio | person | fcc_frs, fcc_gmrs, ecfr_47_1_1102, prior_supply_estimate | rr-supply |
| `contact_cards` | comms | comms | contact_card | people aged 4+ | 1 written contact card per person aged 4 and over | card | person | ready_gov_plan, ready_gov_low_cost | rr-supply |
| `local_map` | comms | comms | paper_map | — | 1 paper map of the area per household | map | household | ready_gov_kit | rr-supply |
| `cash_days` | comms | comms | cash | target(comms) | days of cash for basics = target clamped to 3–14; the household chooses the daily amount (no agency gives one) | day | household | ready_gov_financial_preparedness, fema_effak, prior_supply_estimate | rr-supply |
| `toilet_buckets` | sanitation | water_out | toilet_bucket | — | 2 buckets (pee and poo) with a seat per household | bucket | household | rdpo_emergency_toilet, oregon_b2wr_toolkit | rr-supply |
| `toilet_bags` | sanitation | water_out | toilet_bags | people over 1, target(water_out) | 0.45 bags (0.3–0.6, Prior) per person-day, counting both bags of a double-bagged load; rounded up | bag | person | rdpo_emergency_toilet, oregon_b2wr_toolkit, prior_supply_estimate | rr-supply |
| `toilet_cover_material` | sanitation | water_out | toilet_cover_material | people over 1, target(water_out) | 1 cup (0.5–1.5, Prior) of sawdust, shredded paper or similar per person-day | cup | person | rdpo_emergency_toilet, prior_supply_estimate | rr-supply |
| `soap_grams` | sanitation | supplies | soap | people, target(supplies) | (250 g bathing + 200 g laundry) per person-month × days ÷ 30 | gram | person | sphere_2018 | rr-supply |
| `menstrual_products` | sanitation | supplies | menstrual_products | adults and teens not pregnant or nursing, target(supplies) | 50 % of them (sex is not asked) × 20 products (15–30) per cycle × (2 cycles, + 1 per 28 days beyond a month); rounded up | product | person | cdc_period_disaster, sphere_2018, oregon_b2wr_toolkit, prior_supply_estimate | rr-supply |
| `diapers` | sanitation | supplies | diapers | babies and toddlers, target(supplies) | baby 10 a day (6–12), toddler 6 a day (Prior) × days | diaper | person | sutter_health_diapers, cdc_infant_emergency_checklist, prior_supply_estimate | rr-supply |
| `baby_wipes` | sanitation | supplies | baby_wipes | babies and toddlers, target(supplies) | 2 packs per child per 2 weeks (at least 2); rounded up | pack | person | cdc_infant_emergency_checklist | rr-supply |
| `first_aid_kit` | first aid | medical_emergency | first_aid_kit | people | kits = people ÷ 4 rounded up (the Red Cross family-of-four list), at least 1 | kit | household | red_cross_first_aid_kit | rr-supply |
| `otc_medicines` | first aid | medical_emergency | otc_medicines | people, target(supplies) | 4 kinds (pain reliever, anti-diarrhea, antacid, laxative) × packages of each = max(1, days ÷ 14 × people ÷ 4) rounded up (Prior scaling; 14 days when there is no supplies target) | package | household | ready_gov_kit, red_cross_first_aid_kit, prior_supply_estimate | rr-supply |
| `n95_masks` | first aid | medical_emergency | n95_mask | people aged 4+ | 1 per person per day × 5 days of smoke or outbreak (Prior) | mask | person | cdc_masks, ready_gov_kit, prior_supply_estimate | rr-supply |
| `thermometer` | first aid | medical_emergency | thermometer | babies | 1 oral thermometer + 1 infant thermometer when there is a baby | thermometer | household | red_cross_first_aid_kit, cdc_infant_emergency_checklist | rr-supply |
| `ors_packets` | first aid | medical_emergency | oral_rehydration_salts | people, target(supplies) | 3 packets per person per 2 weeks (Prior), at least 2 weeks' worth, each into 1 L of safe water | packet | person | cdc_cholera_ors, cdc_yellow_book_travel_kits, prior_supply_estimate | rr-supply |
| `battery_fan` | thermal | thermal (heat) | battery_fan | people | 1 per household + 1 per person 65+, baby or pregnant (Prior); fans help only below 90 °F indoors | fan | household | cdc_heat_health, prior_supply_estimate | rr-supply |
| `cooling_towel` | thermal | thermal (heat) | cooling_towel | people | 1 per person (Prior) | towel | person | prior_supply_estimate | rr-supply |
| `cooling_plan` | thermal | thermal (heat) | cooling_plan | — | 1 plan: nearest cooling centre (dial 2-1-1), coolest room, check-ins | plan | household | cdc_heat_health, ready_gov_heat | rr-supply |
| `sleeping_bag_or_blanket` | thermal | thermal (cold) | sleeping_bag | people | 1 per person, rated for the coldest nights | item | person | ready_gov_kit, sphere_2018 | rr-supply |
| `warm_layers` | thermal | thermal (cold) | warm_layers | people | 1 set of warm layers (coat, hat, gloves) per person | set | person | ready_gov_kit, cdc_winter_storm_safety | rr-supply |
| `warm_room_plan` | thermal | thermal (cold) | warm_room_plan | housing.heating, people | 1 plan: one warm room, towels under doors, blankets over windows, never unvented combustion indoors; wording follows the heating | plan | household | cdc_winter_storm_safety, cdc_co_poisoning | rr-supply |
| `smoke_alarm` | fire | fire | smoke_alarm | housing.kind, basement, alarms.smoke = false | 1 per level: apartments and mobile homes 1, houses 2, + 1 for a basement (Prior level count) | alarm | household | ready_gov_home_fires, prior_supply_estimate | rr-supply |
| `co_alarm` | fire | fire | co_alarm | housing.kind, basement, alarms.co = false | 1 per level (same level count) | alarm | household | cdc_co_poisoning, ready_gov_power_outages, prior_supply_estimate | rr-supply |
| `fire_extinguisher` | fire | fire | fire_extinguisher | alarms.extinguisher = false | 1 per household (Prior) | extinguisher | household | prior_supply_estimate | rr-supply |
| `fire_escape_plan` | fire | fire | fire_escape_plan | housing.kind, alarms.smoke | 1 plan for every household: two ways out of every room, a meeting spot outside, a practice run; stairways in a building; monthly alarm tests when there are alarms | plan | household | ready_gov_home_fires | rr-supply |
| `neighbour_contacts` | security | security | neighbour_contacts | — | 2 neighbours' numbers swapped, and who checks on whom (Prior) | contact | household | prior_supply_estimate | rr-supply |
| `document_kit` | documents | home_loss | document_kit | — | 1 Emergency Financial First Aid Kit (4 parts: ID, financial and legal, medical, contacts), copies kept safe and updated yearly | kit | household | fema_effak | rr-supply |
| `insurance_home_or_renters` | documents | home_loss | insurance_home_or_renters | finances.insurance.home_or_renters = false, housing.tenure | 1 decision: renters or homeowners insurance | decision | household | fema_effak, ready_gov_financial_preparedness | rr-supply |
| `insurance_flood` | documents | home_loss | insurance_flood | finances.insurance.flood = false | 1 decision; standard policies exclude floods, a new policy usually waits 30 days, NFIP pays up to $250,000 building / $100,000 belongings | decision | household | fema_nfip, floodsmart_limits, ready_gov_financial_preparedness | rr-supply |
| `insurance_earthquake` | documents | home_loss | insurance_earthquake | finances.insurance.earthquake = false, earthquake among home_loss's contributing hazards | 1 decision; standard policies exclude earthquake damage | decision | household | ready_gov_earthquakes | rr-supply |
| `emergency_fund_months` | documents | income | emergency_fund | target(income), finances | months = income target (3 when absent; planners suggest 3–6, the CFPB says it depends); dollars in the text when monthly expenses are given. Savings track, never the supplies budget | month | household | finra_emergency_fund, stlouisfed_emergency_fund, cfpb_emergency_fund | rr-supply |
| `get_home_bag` | get home | get_home | get_home_bag | each person's `commute` (distance above 0), hot | 1 bag per commuter (line per person): walking shoes, light, map, cash, weather layer, water and food for the walk (1,250 kcal per 12 h, Prior); kept in the car (car commuters) or at work; be ready to shelter at work 24 h | bag | commuter | ready_gov_kit_2020, ready_gov_kit, prior_supply_estimate, mutcd_walking_speed | rr-supply |
| `get_home_water` | get home | get_home | walking_water | commute distance, hot | hours = miles ÷ 3 mph (Prior; 2–2.4 loaded); litres = hours × 0.5 L an hour (Prior; 0.71, NIOSH, when hot); past 2.5 L carry a filter | litre | commuter | prior_supply_estimate, niosh_heat_stress, mutcd_walking_speed | rr-supply |
| `car_kit` | get home | get_home | car_kit | mobility.vehicles | 1 car emergency kit per vehicle (jumper cables, light, warm clothes, blanket, water, snacks) | kit | household | ready_gov_winter_weather | rr-supply |
| `go_bag` | evacuate | evacuate | go_bag | people aged 4+, target(evacuate) notice and days away | 1 bag per person aged 4 and over (babies' things go in a parent's bag); where it lives follows the notice band (under 1 h minutes, under 24 h hours, else days; Prior band edges) | bag | person | wa_emd_prepare_in_a_year, ready_gov_kit, cdc_evacuation_psa, prior_supply_estimate | rr-supply |
| `go_bag_water` | evacuate | evacuate | go_bag_water | people, water_level, hot, target(evacuate).days_away | people's per-day water × min(days away, 3) days (Red Cross three-day evacuation supply); pets have their own kit | gallon | person | red_cross_survival_kit, wa_emd_prepare_in_a_year, ready_gov_water | rr-supply |
| `go_bag_food` | evacuate | evacuate | go_bag_food | people, target(evacuate).days_away | DGA kcal a day × min(days away, 3); food that needs no cooking | kcal | person | red_cross_survival_kit, dga_2020_2025, wa_emd_prepare_in_a_year | rr-supply |
| `pet_carrier` | evacuate | evacuate | pet_carrier | dogs, cats, small pets | 1 carrier per pet | carrier | pet | aspca_disaster_prep, ready_gov_evacuation | rr-supply |
| `pet_go_water` | evacuate | evacuate | pet_water | dogs, cats, small pets | pets' water per day × 7 days (ASPCA); replaced every 2 months | gallon | pet | aspca_disaster_prep, petmd_pet_water | rr-supply |
| `pet_go_food` | evacuate | evacuate | pet_food | dogs, cats, small pets | pets × 10 days (ASPCA 7–10); replaced every 2 months | pet_day | pet | aspca_disaster_prep | rr-supply |
| `fuel_half_tank` | evacuate | evacuate | fuel_half_tank | mobility.vehicles | vehicles kept at least half full (an EV half charged, Prior); full when leaving looks likely | vehicle | household | ready_gov_evacuation | rr-supply |
| `evacuation_ride_plan` | evacuate | evacuate | evacuation_ride_plan | no vehicle | 1 plan: who drives you, or which bus or train leads out; leave early | plan | household | ready_gov_evacuation | rr-supply |
| `evacuation_assistance_plan` | special needs | evacuate | evacuation_assistance_plan | people with `mobility` limited or wheelchair | 1 transport plan per person who needs help to leave | person | person | ready_gov_older_adults, ready_gov_disability | rr-supply |
| `once` | generic | any | (the item) | — | 1 per household (free actions and one-off items); no line is emitted, the plan sizes the item | item | household | (the item's own) | built in |
| `per_person` | generic | any | (the item) | people | 1 per person; no line is emitted, the plan sizes the item with `rr_supply::generic_quantity` | item | person | (the item's own) | rr-supply |
| `per_vehicle` | generic | any | (the item) | mobility.vehicles | 1 per vehicle; no line is emitted | item | household | (the item's own) | rr-supply |
| `per_pet` | generic | any | (the item) | dogs, cats, small pets | 1 per pet; no line is emitted | item | pet | (the item's own) | rr-supply |
| `per_commuter` | generic | any | (the item) | people with a commute | 1 per commuter; no line is emitted | item | commuter | (the item's own) | rr-supply |

## Which bucket's days size which line

Duration lines use their own bucket's target, with three exceptions: `phone_power_wh` and
`rx_cold_storage` use the **power** target (phones die and medicine warms when the power is out), and
`otc_medicines` / `ors_packets` use the **supplies** target (how long you can't reach a store; 14 days
when there is none). Heat lines (`battery_fan`, `cooling_towel`, `cooling_plan`) appear when
`heat_wave` drives the thermal bucket, cold lines (`sleeping_bag_or_blanket`, `warm_layers`,
`warm_room_plan`) when `cold_wave`, `winter_weather` or `ice_storm` does, and both when the
assessment names neither.

## Citation ids rr-supply uses

The `[[source]]` table at the end of `crates/rr-supply/src/constants.toml` lists every citation id the
supply lines can emit, with title, publisher and URL (`rr_supply::citations_used()` returns them).
The content workstream owns `content/citations.toml`; where its id for a source differs, change the
id in `constants.toml` (the registry test keeps every use consistent). `prior_supply_estimate` is a
citation with `prior = true`: "Ready Reckoner planning estimate (supply)". `retail_prices_2026_09`
stands for the retail price and label observations in `docs/research/supply-standards.md` §12.
