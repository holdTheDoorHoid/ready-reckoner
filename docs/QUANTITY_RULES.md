# Quantity rules

The shared registry of `quantity_rule` names. A catalogue item (`content/items/*.toml`) names the
rule that sizes it; `rr-supply` implements every rule here and emits requirement lines
(`RequirementLine` in `docs/ENGINE-API.md`) for the ones that size a need per bucket.

**Machine-read.** `rr-content`'s validator reads every table row whose first cell is a rule name in
backticks: an item's `quantity_rule` must be `once` or one of them. `rr-supply`'s own test checks
the Rules table lists exactly the rules the crate implements (`rr_supply::rule_ids()`) and that
every line's unit matches the "unit" column. To ask for a rule that is missing, add a row with
`requested by content` in the last column; the supply workstream implements it or answers in the
row.

This version merges the supply draft with the content workstream's requests (2026-09-25): every
rule name content asked for is kept, with its formula as implemented; where the formula differs
from the request, the status column says how and why.

**Polish round (2026-09-26).** One bottle of bleach whatever the target; the go-bags', pet go-kit's
and get-home bags' water and food are staged from household supplies (alternative lines, never
additive; new rule `get_home_food`); one extinguisher per floor people live on and an escape ladder
only for floors 2–3; blankets and warm layers first (new rule `blankets`, for content's assumed
basics), with `sleeping_bag_or_blanket` a need only beyond a 3-day cold target or for people 65 and
over; and each line cites only the sources behind its own terms. The status column says "Polish
round" where a rule changed. No rule was removed, so every existing `quantity_rule` still
validates.

## Conventions

- **Units.** A rule's quantity is in the item's own unit, with two exceptions: `gallon` rules give
  water volume and `kcal` rules give food energy, which the budget converts with the item's
  `volume_l_per_unit` or `energy_kcal_per_unit` (so a 7-gallon jug and a gallon of bottled water
  both cover `water_gallons`). The "unit" column is the unit of the requirement line.
- **Zero means not needed.** A rule that gives 0 keeps the item out of this household's plan; that
  is how the `once_if_*` switches work.
- **`once` is built in:** 1 per household, for a one-time action or a single purchase.
- **Days** means the target of the named bucket. Lines are sized for the whole target; each line
  also carries its days and a per-day rate (`SizedLine::per_day`), so the budget can size a tier's
  share (`quantity_for_days`). Where a rule names two buckets it uses the longer target.
- **Sizing an item.** `rr_supply::ItemSizer::new(input, buckets, ctx).quantity(rule)` gives the
  quantity for any rule here: generic rules count or switch; line rules give their line's
  quantity (summed over commuters for per-person lines, the largest over buckets otherwise).
- **Coverage is directional.** Where one kind of cover cannot stand in for another, the line's
  `item_class` names the coverage part: `thermal_heat` (fans, cooling) versus `thermal_cold`
  (bedding, layers, a warm room), and `water_stored` (water you keep) versus
  `water_treatment_capacity` (making water safe: filter, bleach, boiling fuel). A battery fan is
  never ice-storm cover, and a filter is not stored water. rr-plan maps these to the allocator's
  coverage parts.
- **Staged supplies are never bought twice.** The go-bags' water and food, the pet go-kit's, and
  each commuter's water and snacks for the walk home come out of the household's own stored water
  and food. They are alternative lines (`bucket.bag.alt.staged_water`,
  `bucket.bag.alt.staged_food`) of the bag they go in, sized by their own standard (Red Cross three
  days, ASPCA a week of water and ten days of food), which can be more than a short target stores:
  they say what to set aside, not what to buy. rr-plan counts an item that uses one of these rules toward the bag's line (an alternative
  meets its need line), so only free staging steps should use them (content's
  `special_pet_go_water` and `special_pet_go_food` do).
- **Each line cites only the sources behind its own amount and words.** A rule that reads a shared
  helper (a day of water, the walk home) cites what its own quantity and sentence use: the
  reused-bottles line at its 6-gallon cap does not cite the pets' water, the boil-water line cites
  the drinking share and not the whole basic gallon, and a line the target's days do not size (the
  bleach bottle) does not cite the target.
- **People counts** use the age bands in DESIGN §4.1; "13+" means teen, adult and senior.
- ***Prior*** marks a planning estimate. It cites `rr_expert_prior`, and the line's sentence says
  "some amounts are estimates". Every other number cites the source in the "citations" column
  (ids from `content/citations.toml`).

## How lines are named

A line's `id` says what kind of line it is, so no consumer adds up the same need twice
(`rr_supply::LineKind::of(id)` reads it):

| Id pattern | Kind | Meaning |
| --- | --- | --- |
| bucket.rule | need | The amount this household needs for that bucket. |
| bucket.rule.person_N | need | The same, for one person (N is the 1-based position in `PlanInput.people`): each commuter's get-home bag is sized to their own trip. |
| bucket.rule.alt.variant | alternative | Another way to meet the need line bucket.rule (the food need's cost as freeze-dried meals, its days beyond the first month as bulk staples, part of the stored water as reused bottles), or a household supply **staged** for it (variant `staged_water` or `staged_food`: the go-bags' water and food, drawn from the stored water and food). Never add it to the need, or to anything else. |
| bucket.rule.alt.variant.person_N | alternative | The same for one person's line bucket.rule.person_N: each commuter's water and snacks for the walk home, staged from home (`get_home.get_home_bag.alt.staged_water.person_1`). `LineKind::alternative_to` gives bucket.rule.person_N. |
| bucket.rule.optional | optional | Worth having for some households, not needed to meet the target (a generator, a solar panel, keeping a fridge running). |
| bucket.rule.note | note | Information for the plan and the packet (what a retail "30-day" kit really lasts); not something to buy. |

Inputs: `people`, `pets`, `housing`, `mobility`, `finances`, `dials.water_level` come from `PlanInput`;
`target(b)` is the bucket's design target from `BucketAssessment` (days, months, or the evacuation /
readiness shape). Three optional facts come from `rr_supply::SupplyContext`: `hot` is true when the
county has at least 30 days a year at or above 95 °F under the chosen climate dial (temperate when
absent), `lat` is the county centroid latitude, and `nuclear_16km` is the facility flag for a plant
within 16 km. Buckets missing from the input get no lines, and a duration bucket with a target of 0
days gets none either.

## Rules

| rule | family | bucket | item_class | inputs | formula | unit | per | citations | status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `water_gallons` | water | water_out | water_stored | people, pets, water_level, hot, target(water_out) | days = min(target, 14); per day = people × level allowance (survival 0.8, basic 1.0, comfortable 4.0 gal; the drinking share × 2 when hot, so basic becomes 1.75) + 0.29 per pregnant or nursing person + 0.25 per formula-fed infant + dogs 40 lb × 1 oz/lb ÷ 128 + cats 10 lb × 0.8 oz/lb ÷ 128 + small pets 8 oz ÷ 128 (weights and small pets *Prior*); × days. Beyond 14 days see `water_treatment_capacity` | gallon | person | ready_gov_water, cdc_water_storage, sphere_2018, iom_dri_water_2005, aap_formula_amounts, petmd_dog_water, merck_vet_maintenance_fluids | implemented. Differs from the request: uses the water_out target only (boil notices are met by treatment, see below; Philadelphia's 3 days stay 12.9 gal); livestock have their own rule `livestock_water`; the 0.6 gal per dehydrated-food person-day is added only when the food plan uses dried food (`water_gallons(…, dehydrated_food_person_days)`), and the freeze-dried alternative line says so |
| `water_reused_bottles` | water | water_out (alternative of `water_gallons`) | water_stored | people, pets, water_level, hot | min(3 days of the per-day water, 6 gal); *Prior* cap: the clean drink bottles a household can gather. At the cap the line cites only the cap's sources; under it (small households) the amount is 3 days of water and it cites the `water_gallons` sources instead | gallon | household | rr_expert_prior, rr_research_risk_model, cdc_water_storage, ready_gov_water, church_emergency_prep_manual (under the cap: the `water_gallons` sources instead of the first two) | implemented (requested by content). Polish round: cites only the sources behind its own amount (it used to inherit the pet-water sources) |
| `water_treatment_capacity` | water | water_boil, water_out | water_treatment_capacity | people, pets, hot, target(water_boil), target(water_out), housing.water | Lines, in gallons of water to make safe: water_boil = (people × 0.75 gal drinking share, × 2 when hot, + pregnancy/nursing + formula + pets) × target days, whatever the water level; water_out = the full per-day need × (target − 14) days, only when the target is longer than 14 days ("a filter plus a water source beats storing more"). Item quantity: 1 filter when the water_out target is over 14 days or the home has a well, else 0 | gallon | household | cdc_water_disinfection, epa_emergency_disinfection, byu_longer_term_storage_2019, oregon_b2wr_toolkit | implemented. The item quantity follows the content request (1 or 0); the lines state the gallons, which one family filter far exceeds |
| `bleach_bottles` | water | water_boil (water_out when there is no boil target) | water_treatment_capacity | — | 1 bottle of plain unscented 5–9 % bleach per household, whatever the target: at ½ mL a gallon (CDC) each fluid ounce treats about 60 gallons of clear water (30 cloudy), so one bottle treats thousands of gallons. A fresh bottle every 6 months, when stored water is rotated (*Prior*: bleach weakens in storage and no agency gives a shelf life; up to 12) | bottle | household | cdc_water_storage, epa_emergency_disinfection, cdc_water_disinfection, cdc_bleach_disinfecting, rr_expert_prior | implemented (requested by content). Polish round: one bottle, not one per 14 days of the longer water target (27 bottles for a year); the days no longer size it, so the line does not cite the target |
| `boil_fuel` | water | water_boil (optional) | water_treatment_capacity | treatment gallons | litres to boil ÷ 27.5 L per lb of propane (*Prior*, 25–30); outdoors only; no more than 2 one-pound cylinders indoors | lb | household | eia_btu, lehi_fuel_storage, cdc_co_basics, rr_expert_prior | rr-supply |
| `livestock_water` | water | water_out | livestock_water | pets.large_animals, target(water_out) | 25 L (20–30) per large animal per day × days ÷ 3.785 | gallon | pet | sphere_2018, aspca_disaster_prep | rr-supply |
| `food_kcal` | food | supplies | food | people, target(supplies) | Σ people: DGA 2020–2025 Table A2-2, moderately active, men's and women's rows averaged year by year over the band (toddler 1,067, child 1,667, teen 2,280, adult 2,262, 65+ 2,009; babies 0, see `infant_formula_oz`) + 400 per pregnant or nursing person (the form does not say trimester or month); × days | kcal | person | usda_dga_2020_2025 | implemented (in the supply brief) |
| `food_cost_estimate` | food | supplies (alternatives of `food_kcal`) | food_pantry, food_bulk_staples, food_freeze_dried | people, target(supplies) | pantry: person-days × $8.44 × USDA household-size factor (+20 % for 1 person … −10 % for 7+, babies not counted); bulk staples: person-days × $2.50 ($2.15–2.85); freeze-dried: kcal ÷ 2,000 × $24 ($9–39). Cost comparison only; no item uses this rule | usd | household | usda_tfp_aug2026, church_hsc_order_form_2026, byu_longer_term_storage_2019, rr_price_observations_2026_09 | rr-supply |
| `food_kit_check` | food | supplies (note) | food_kit | people | days a retail "30-day" kit really lasts = 30 × 1,290 kcal ÷ household average kcal per person-day (kits supply 1,290–1,730 kcal a day) | day | person | rr_price_observations_2026_09, usda_dga_2020_2025 | rr-supply |
| `long_term_staples` | food | supplies (alternatives of `food_kcal`, target over 30 days) | staples_grains, staples_legumes, staples_dry_milk, staples_sugar, staples_fruit_veg, staples_salt_leavening, staples_oil, staples_vitamin | people, target(supplies) | for the days beyond 30: BYU 2019 per-adult-year amount × (days − 30) ÷ 365 × Σ adult-equivalents (Ensign 2006 child shares with one year added: 50 % to age 3, 70 % 4–6, 90 % 7–10, 100 % 11+; babies 0) | lb, gallon, tablet | person | byu_longer_term_storage_2019, ensign_2006_year_supply, church_hsc_order_form_2026, usu_food_storage_booklet | rr-supply |
| `cooking_fuel_canisters` | food | supplies (optional) | stove_fuel | people, target(supplies), target(water_boil) | max(2, ceil(people × longer target × 0.2 lb ÷ 0.5 lb)) 8-ounce canisters; *Prior*: 0.2 lb a person-day boils drinking water and heats one meal (1 lb boils about 25–30 L) | canister | household | rr_research_supply_standards, eia_btu, cdc_co_basics, rr_expert_prior | implemented (requested by content) |
| `infant_formula_oz` | food | supplies | infant_formula | formula-fed babies (`dietary` mentions formula), target(supplies) | babies on formula × max(days, 3) × 5 oz of powder; *Prior*: 32 oz of prepared formula a day (the AAP most) takes about 5 oz of powder, labels differ | oz | person | aap_formula_amounts, cdc_infant_feeding_disaster, cdc_infant_checklist, rr_expert_prior | implemented (requested by content) |
| `nursing_supplies` | special needs | supplies | nursing_supplies | babies not on formula | 1 manual pump + 1–2 boxes of nursing pads per breastfed baby | kit | person | cdc_infant_checklist | rr-supply |
| `pet_food_lb` | food | supplies | pet_food | dogs, cats, small pets, target(supplies) | (dogs × 0.7 + cats × 0.15 + small pets × 0.05) lb a day × max(days, 7); *Prior* feeding amounts for a 40 lb dog and a 10 lb cat (the sizes the water rule assumes); ASPCA advises 7–10 days | lb | pet | aspca_disaster_prep, ready_gov_pets, rr_expert_prior | implemented (requested by content) |
| `pet_food_days` | food | supplies (alternative of `pet_food_lb`) | pet_food | dogs, cats, small pets, target(supplies) | (dogs + cats + small pets) × max(days, 7) pet-days (ASPCA 7–10 days), for items counted in days rather than pounds | pet_day | pet | aspca_disaster_prep, ready_gov_pets | implemented (requested by content) |
| `medication_days` | medication | medication | prescription_medicine | people with `daily_rx` or `refrigerated_rx`, target(medication) | people on prescriptions × reserve days (the target clamped to 7–30; 14 when there is no target) | person_day | person | florida_dem_medication, cdc_diabetes_emergencies, cdc_pregnancy_emergency, redcross_survival_kit, healthcare_ready_refill_laws | implemented. Differs from the request: refrigerated prescriptions (insulin) count too |
| `rx_cold_storage` | medication | medication | medicine_cooler | people with `refrigerated_rx`, target(power) | days of cold storage = power target (the medication target when there is no power target) | day | household | ready_gov_power_outages, cdc_insulin_emergency | rr-supply |
| `antibiotics_none` | medication | medication | antibiotics_clinician_card | — | always 0: no antibiotic quantity is ever sized; the line points to the clinician card | course | household | cdc_antibiotic_use, fda_fish_antibiotics_warning_2023, fda_expired_medicines, cdc_yellow_book_travel_kits | rr-supply |
| `epinephrine_check` | medication | medication | epinephrine_auto_injector | people with `epinephrine` | 1 per person who carries an auto-injector (check dates; ask the prescriber how many to keep) | person | person | ready_gov_disability | rr-supply |
| `medical_device_wh` | power | power | medical_device_power | people's `powered_device`, target(power) | CPAP 170 Wh a night (96 with the humidifier off); oxygen concentrator 300 W × 24 h (*Prior*; use the label); other device watts × 24 h (*Prior*); × days | Wh | person | sil_cpap_power, ready_gov_disability, ready_gov_power_outages, rr_expert_prior | rr-supply |
| `device_battery_units` | power | power | device_battery | people's `powered_device` | people with a powered medical device | battery | person | sil_cpap_power, ready_gov_disability | implemented (requested by content) |
| `lights` | power | power | light | people | 1 battery light per person aged 4 and over, at least 1 (*Prior*) | light | person | ready_gov_kit, ready_gov_power_outages, rr_expert_prior | rr-supply |
| `battery_packs` | power | power | battery_pack | target(power) | max(1, ceil(days ÷ 7)) packs of AA/AAA cells; *Prior*: one pack runs a household's lights and radio for a week | pack | household | ready_gov_kit, rr_expert_prior | implemented (requested by content) |
| `power_station_units` | power | power (a need with a medical device, otherwise optional) | power_station | devices, `refrigerated_rx`, target(power) | with a powered medical device: ceil(device Wh a day × days ÷ 909 usable Wh), 1 to 2 (*Prior* cap); with refrigerated medicine: 1; otherwise 1 when the power target is 3 days or more (an optional upgrade), else 0. A station is 1,070 Wh, 85 % usable (*Prior*) | power station | household | sil_cpap_power, epa_energy_star_refrigerators, rr_price_observations_2026_09, rr_expert_prior | implemented (requested by content) |
| `generator_units` | power | power (optional) | generator | target(power), housing.kind, housing.water | 0 for apartments (no safe spot 20 ft from openings); for houses, 1 when the power target is 3 days or more, or 1 day or more on a well; else 0 (*Prior* thresholds) | generator | household | cdc_co_basics, ready_gov_power_outages, rr_research_supply_standards, rr_expert_prior | implemented (requested by content) |
| `generator_fuel_gallons` | power | power | generator_fuel | `backup_power` = generator, target(power) | 0 without a generator; else min(2.8 gal a day × days, 25 gal) (a 2.2 kW inverter unit at quarter load, 7.1 at full load; 25 gal is the fire-code limit, 10 in an attached garage, none indoors) | gallon | household | honda_eu2200i_spec, lehi_fuel_storage, cdc_co_basics, ready_gov_power_outages | implemented (requested by content): "a generator" means one the household owns (`backup_power`); the budget sizes fuel for a generator it buys with the same formula |
| `solar_panel_units` | power | power (optional) | solar_panel | target(power), lat, devices, people aged 13+ | 1 when the power target is 7 days or more, else 0 (*Prior*); the line gives December output for the latitude band (NLR PVWatts cities: under 36° N 382, 36–42° 261, 42–46° 219, 46° and north 121 Wh a day per 100 W; *Prior* proxy) and the panel watts the essential load needs | panel | household | nlr_pvwatts_v8, epa_energy_star_refrigerators, rr_expert_prior | implemented (requested by content) |
| `fridge_wh` | power | power (optional) | fridge_power | target(power) | 995 Wh a day (ENERGY STAR median top-freezer; chest freezer 597) × days; a closed fridge holds about 4 hours, a full freezer about 48 | Wh | household | epa_energy_star_refrigerators, ready_gov_food, ready_gov_power_outages | rr-supply |
| `well_pump_wh` | power | power (optional) | well_pump_power | `water` = well, target(power) | 1,250 W (1,000–1,500) × 1 h a day × days; starting takes 2–3 × the running watts (*Prior*; check the nameplate) | Wh | household | rr_expert_prior | rr-supply |
| `wheelchair_battery` | special needs | power | wheelchair_battery | people with `mobility` = wheelchair | 1 spare battery per wheelchair user (if the chair is powered) | battery | person | ready_gov_disability | rr-supply |
| `noaa_radio` | comms | comms | weather_radio | — | 1 battery or hand-crank radio with NOAA Weather Radio per household | radio | household | ready_gov_kit, nws_weather_radio | rr-supply |
| `phone_power_wh` | comms | comms | power_bank | people aged 13+, target(power) | phones (1 per person aged 13+) × 15 Wh a day (*Prior*, 10–20) × days without power (the comms target when there is no power target) | Wh | person | ready_gov_kit, ready_gov_earthquakes, rr_expert_prior | rr-supply |
| `two_way_radios` | comms | comms | two_way_radio | people aged 13+ | 1 per person aged 13+ when there are at least 2 (*Prior*); FRS needs no licence, GMRS $35 for 10 years | radio | person | fcc_frs, fcc_gmrs, ecfr_47_1_1102, rr_expert_prior | rr-supply |
| `contact_cards` | comms | comms | contact_card | people aged 4+ | 1 written contact card per person aged 4 and over | card | person | ready_gov_plan, ready_gov_low_cost | rr-supply |
| `local_map` | comms | comms | paper_map | — | 1 paper map of the area per household | map | household | ready_gov_kit | rr-supply |
| `cash_reserve_usd` | comms | comms | cash | target(comms) | $100 per household until the household sets its own daily amount (*Prior*: no agency gives a figure); the line also says how many days of basics that should cover (the comms target clamped to 3–14) | usd | household | ready_gov_financial, fema_effak, rr_expert_prior | implemented (requested by content) |
| `toilet_buckets` | sanitation | water_out | toilet_bucket | — | 2 buckets (pee and poo) with a seat per household | bucket | household | rdpo_emergency_toilet, oregon_b2wr_toolkit | rr-supply |
| `toilet_bags` | sanitation | water_out | toilet_bags | people over 1, target(water_out) | ceil(people × days × 0.45) bags (0.3–0.6, *Prior*), counting both bags of a double-bagged load | bag | person | rdpo_emergency_toilet, oregon_b2wr_toolkit, rr_expert_prior | implemented (requested by content); babies in diapers are not counted |
| `toilet_cover_material` | sanitation | water_out | toilet_cover_material | people over 1, target(water_out) | 1 cup (0.5–1.5, *Prior*) of sawdust, shredded paper or similar per person-day | cup | person | rdpo_emergency_toilet, rr_expert_prior | rr-supply |
| `toilet_paper_rolls` | sanitation | supplies | toilet_paper | people over 1, target(supplies), target(water_out) | ceil(people × longer target ÷ 5) rolls; *Prior*: about a roll a person every 5 days (Oregon: measure a week's use and double it) | roll | person | oregon_b2wr_toolkit, rr_expert_prior | implemented (requested by content); babies in diapers are not counted |
| `soap_person_months` | sanitation | supplies | soap | people, target(supplies) | people × ceil(days ÷ 30) person-months, each 250 g of bathing soap and 200 g of laundry soap | person_month | person | sphere_2018 | implemented (requested by content) |
| `menstrual_cycles` | sanitation | supplies | menstrual_products | adults and teens not pregnant or nursing, target(supplies) | ceil(half of them × cycles each), cycles each = 2, + 1 per 28 days beyond a month; about 20 products a cycle (15–30); *Prior*: sex is not asked | cycle | person | cdc_period_factsheet, sphere_2018, oregon_b2wr_toolkit, rr_expert_prior | implemented. Differs from the request: 2 cycles cover plans up to a month, as the research reads the CDC "2 cycles" advice (the request's 2 + floor(days ÷ 28) gives 3 at 28 days) |
| `diapers` | sanitation | supplies | diapers | babies and toddlers, target(supplies) | (babies × 8 + toddlers × 6) × max(days, 3); *Prior* from newborns' 8–12 changes a day | diaper | person | sutter_diapers, cdc_infant_checklist, rr_expert_prior | implemented (requested by content) |
| `baby_wipes` | sanitation | supplies | baby_wipes | babies and toddlers, target(supplies) | 2 packs per child per 2 weeks (at least 2); rounded up | pack | person | cdc_infant_checklist | rr-supply |
| `first_aid_kit` | first aid | medical_emergency | first_aid_kit | people | ceil(people ÷ 4) kits (the Red Cross list is for four people), at least 1 | kit | household | redcross_first_aid_kit | implemented (requested by content) |
| `otc_medicines` | first aid | medical_emergency | otc_medicines | people, target(supplies) | 4 kinds (pain reliever, anti-diarrhea, antacid, laxative) × packages of each = max(1, days ÷ 14 × people ÷ 4) rounded up (*Prior* scaling; 14 days when there is no supplies target) | package | household | ready_gov_kit, redcross_first_aid_kit, rr_expert_prior | rr-supply |
| `n95_masks` | first aid | medical_emergency | n95_mask | people aged 4+ | people aged 4+ × 1 a day × 5 days of smoke or outbreak (*Prior*) | mask | person | cdc_masks, ready_gov_kit, rr_expert_prior | implemented. Differs from the request: counts people aged 4 and over, because the toddler band (1–3) includes babies under 2, who should not wear masks |
| `thermometer` | first aid | medical_emergency | thermometer | babies | 1 oral thermometer + 1 infant thermometer when there is a baby | thermometer | household | redcross_first_aid_kit, cdc_infant_checklist | rr-supply |
| `ors_packets` | first aid | medical_emergency | oral_rehydration_salts | people, target(supplies) | 3 packets per person per 2 weeks (*Prior*), at least 2 weeks' worth, each into 1 L of safe water | packet | person | cdc_cholera_treatment, cdc_yellow_book_travel_kits, rr_expert_prior | rr-supply |
| `battery_fan` | thermal | thermal (heat) | thermal_heat | people | 1 per household + 1 per person 65+, baby or pregnant (*Prior*); fans help only below 90 °F indoors | fan | household | cdc_heat_health, rr_expert_prior | rr-supply |
| `cooling_towel` | thermal | thermal (heat) | thermal_heat | people | 1 per person (*Prior*) | towel | person | rr_expert_prior | rr-supply |
| `cooling_plan` | thermal | thermal (heat) | thermal_heat | — | 1 plan: nearest cooling centre (dial 2-1-1), coolest room, check-ins | plan | household | cdc_heat_health, ready_gov_heat | rr-supply |
| `blankets` | thermal | thermal (cold) | thermal_cold | people | 1 warm blanket per person aged 1 and over (babies: warm sleepwear or a sleep sack instead of loose blankets, CDC). Most homes have them: content's assumed-basic blanket item meets this line when it uses `quantity_rule = "blankets"` | blanket | person | ready_gov_kit, sphere_2018, cdc_winter_safety | rr-supply (polish round, for content's assumed basics) |
| `sleeping_bag_or_blanket` | thermal | thermal (cold; a need, or `.optional`) | thermal_cold | people, target(thermal), housing.heating | The extra layer beyond `blankets` and `warm_layers`: a sleeping bag or an extra heavy blanket. A need for everyone aged 1 and over when the target is longer than 3 days (*Prior*), otherwise for each person 65 or over (CDC: most at risk in the cold). An optional upgrade (`thermal.sleeping_bag_or_blanket.optional`, 1 per person aged 1+) with a wood stove (it heats without power; a gas furnace is not assumed to) or for a short target with nobody 65+. Babies get a sleep sack, never loose bedding | item | person | ready_gov_kit, sphere_2018, rr_expert_prior, cdc_winter_safety | rr-supply. Polish round: it was 1 per person for any cold target (4 sleeping bags for Philadelphia's 1.7 days; now 1, for the senior). rr-plan's `thermal_sleeping_bag` → `thermal.sleeping_bag_or_blanket` entry still holds (an optional line matches nothing, so no bag is bought); an item may also use this rule directly |
| `warm_layers` | thermal | thermal (cold) | thermal_cold | people | 1 set of warm layers (coat, hat, gloves) per person. Most homes have them: content's assumed-basic warm-layers item meets this line when it uses `quantity_rule = "warm_layers"` | set | person | ready_gov_kit, cdc_winter_safety | rr-supply |
| `warm_room_plan` | thermal | thermal (cold) | thermal_cold | housing.heating, people | 1 plan: one warm room, towels under doors, blankets over windows, never unvented combustion indoors; wording follows the heating | plan | household | cdc_winter_safety, cdc_co_basics | rr-supply |
| `fire_escape_plan` | fire | fire | fire_escape_plan | housing.kind, housing.floor, alarms.smoke | 1 plan for every household: two ways out of every room, a meeting spot outside, a practice run; stairways in a building; for a house at street level (floor 1 or below), check a second way out of any upstairs bedroom (a window onto a porch roof, or an escape ladder); monthly alarm tests when there are alarms | plan | household | ready_gov_home_fires | rr-supply |
| `smoke_alarm_count` | fire | fire | smoke_alarm | housing.kind, basement, alarms.smoke, people | 0 when the household has smoke alarms; else levels (apartment or mobile home 1, house 2, +1 basement) + bedrooms (ceil(people ÷ 2)); *Prior* level and bedroom counts | alarm | household | usfa_smoke_alarms, ready_gov_home_fires, rr_expert_prior | implemented (requested by content); 0 when `alarms.smoke` is true, so the plan never buys alarms a home already has |
| `co_alarm_count` | fire | fire | co_alarm | housing.kind, alarms.co | 0 when the household has CO alarms; else one per level where people sleep (apartment or mobile home 1, house 2); *Prior* level count | alarm | household | cdc_co_basics, ready_gov_power_outages, rr_expert_prior | implemented (requested by content); 0 when `alarms.co` is true |
| `extinguisher_count` | fire | fire | fire_extinguisher | housing.kind, alarms.extinguisher | 0 when the household has one; else 1 multipurpose (A-B-C) extinguisher per floor people live on (apartment or mobile home 1, house 2; not the basement; *Prior*), the ground-floor one near the kitchen. The form does not say where the kitchen is, so it is assumed to be on the floor with the way out; the line says to add one where it is not | extinguisher | household | usfa_extinguishers, rr_expert_prior | implemented (requested by content); 0 when `alarms.extinguisher` is true. Polish round: the basement no longer adds one (Philadelphia 3 → 2) |
| `escape_ladder_count` | fire | fire | escape_ladder | housing.kind, housing.floor | 1 when the household lives on floor 2 or 3, any kind of home but a mobile home (*Prior*: a ladder reaches no higher, where the stairs are the way out; the form does not ask about a second exit, so none is assumed); else 0. `floor` is where the household lives (1 is street level): a house at floor 1 is not assumed to sleep upstairs, and its escape plan asks about upstairs bedrooms instead | ladder | household | ready_gov_home_fires, rr_expert_prior | implemented (requested by content). Polish round: a house at street level no longer gets a ladder by default |
| `neighbour_contacts` | security | security | neighbour_contacts | — | 2 neighbours' numbers swapped, and who checks on whom (*Prior*) | contact | household | rr_expert_prior | rr-supply |
| `document_kit` | documents | home_loss | document_kit | — | 1 Emergency Financial First Aid Kit (4 parts: ID, financial and legal, medical, contacts), copies kept safe and updated yearly | kit | household | fema_effak | rr-supply |
| `insurance_home_or_renters` | documents | home_loss | insurance_home_or_renters | finances.insurance.home_or_renters = false, housing.tenure | 1 decision: renters or homeowners insurance | decision | household | fema_effak, ready_gov_financial | rr-supply |
| `insurance_flood` | documents | home_loss | insurance_flood | finances.insurance.flood = false | 1 decision; standard policies exclude floods, a new policy usually waits 30 days, NFIP pays up to $250,000 building / $100,000 belongings | decision | household | fema_nfip_flood_insurance, floodsmart_buy_policy, ready_gov_financial | rr-supply |
| `insurance_earthquake` | documents | home_loss | insurance_earthquake | finances.insurance.earthquake = false, earthquake among home_loss's contributing hazards | 1 decision; standard policies exclude earthquake damage | decision | household | ready_gov_earthquakes | rr-supply |
| `emergency_fund_months` | documents | income | emergency_fund | target(income), finances | months = income target (3 when absent; planners suggest 3–6, the CFPB says it depends); dollars in the text when monthly expenses are given. Savings track, never the supplies budget | month | household | finra_financial_foundations, stlouisfed_emergency_fund_2025, cfpb_emergency_fund | rr-supply |
| `get_home_bag` | get home | get_home | get_home_bag | each person's `commute` (distance above 0) | 1 bag per commuter (a line per person): walking shoes, a light, map, cash, a weather layer, and water and snacks from home (that person's staged lines, below); kept in the car (car commuters) or at work; the walk home's hours at 3 mph (*Prior*); be ready to shelter at work 24 h | bag | commuter | ready_gov_kit_2020, ready_gov_kit, fhwa_mutcd_walking_speed, rr_expert_prior | rr-supply. Polish round: the water and food amounts moved to the staged lines |
| `get_home_water` | get home | get_home (staged: `get_home.get_home_bag.alt.staged_water.person_N`) | walking_water | commute distance, hot | hours = miles ÷ 3 mph (*Prior*; 2–2.4 loaded); litres = hours × 0.5 L an hour (*Prior*; 0.71, NIOSH, when hot); past 2.5 L carry a filter. Filled from home, never bought extra | litre | commuter | cdc_niosh_heat_hydration, fhwa_mutcd_walking_speed, rr_expert_prior | rr-supply. Polish round: staged from household supplies (it was the need line `get_home.get_home_water.person_N`); rr-plan's `water_personal_filter` → `get_home.get_home_water` entry no longer matches a need line |
| `get_home_food` | get home | get_home (staged: `get_home.get_home_bag.alt.staged_food.person_N`) | walking_food | commute distance | hours = miles ÷ 3 mph (*Prior*); kcal = hours ÷ 12 × 1,250 (*Prior*, 1,000–1,500), rounded to 100 kcal, at least 100. From the household's food, never bought extra | kcal | commuter | fhwa_mutcd_walking_speed, usda_dga_2020_2025, rr_expert_prior | rr-supply (polish round; the amount was in the bag line's text) |
| `car_kit` | get home | get_home | car_kit | mobility.vehicles | 1 car emergency kit per vehicle (jumper cables, light, warm clothes, blanket, water, snacks) | kit | household | ready_gov_winter | rr-supply |
| `go_bag` | evacuate | evacuate | go_bag | people aged 4+, target(evacuate) notice and days away | 1 bag per person aged 4 and over (babies' things go in a parent's bag); a bag already owned will do; water, food and medicines come from the household's supplies (the staged lines, below); where it lives follows the notice band (under 1 h minutes, under 24 h hours, else days; *Prior* band edges) | bag | person | washington_prepare_in_a_year, ready_gov_kit, cdc_evacuation_psa, rr_expert_prior | rr-supply |
| `go_bag_water` | evacuate | evacuate (staged: `evacuate.go_bag.alt.staged_water`) | go_bag_water | people, water_level, hot, target(evacuate).days_away | people's per-day water × min(days away, 3) days (Red Cross three-day evacuation supply), drawn from the stored water (`water_gallons`), never bought extra; pets have their own kit | gallon | person | redcross_survival_kit, washington_prepare_in_a_year, ready_gov_water, rr_expert_prior | rr-supply. Polish round: staged (it was the need line `evacuate.go_bag_water`) |
| `go_bag_food` | evacuate | evacuate (staged: `evacuate.go_bag.alt.staged_food`) | go_bag_food | people, target(evacuate).days_away | DGA kcal a day × min(days away, 3); food that needs no cooking, drawn from the household's food, never bought extra | kcal | person | redcross_survival_kit, usda_dga_2020_2025, washington_prepare_in_a_year | rr-supply. Polish round: staged (it was the need line `evacuate.go_bag_food`) |
| `pet_carrier` | evacuate | evacuate | pet_carrier | dogs, cats, small pets | 1 carrier per pet, with a pet go-kit packed from household supplies: water and food (the staged lines, below), medicine, records and a photo | carrier | pet | aspca_disaster_prep, ready_gov_evacuation | rr-supply |
| `pet_go_water` | evacuate | evacuate (staged: `evacuate.pet_carrier.alt.staged_water`) | pet_water | dogs, cats, small pets | pets' water per day × 7 days (ASPCA), set aside from the stored water; replaced every 2 months | gallon | pet | aspca_disaster_prep, petmd_dog_water, merck_vet_maintenance_fluids, rr_expert_prior | rr-supply. Polish round: staged (it was the need line `evacuate.pet_go_water`). Content's free action `special_pet_go_water` uses this rule; rr-plan counts it toward the pet carrier line, as an alternative |
| `pet_go_food` | evacuate | evacuate (staged: `evacuate.pet_carrier.alt.staged_food`) | pet_food | dogs, cats, small pets | pets × 10 days (ASPCA 7–10), set aside from the household's pet food; replaced every 2 months | pet_day | pet | aspca_disaster_prep | rr-supply. Polish round: staged (it was the need line `evacuate.pet_go_food`). Content's free action `special_pet_go_food` uses this rule |
| `fuel_half_tank` | evacuate | evacuate | fuel_half_tank | mobility.vehicles | vehicles kept at least half full (an EV half charged, *Prior*); full when leaving looks likely | vehicle | household | ready_gov_evacuation | rr-supply |
| `evacuation_ride_plan` | evacuate | evacuate | evacuation_ride_plan | no vehicle | 1 plan: who drives you, or which bus or train leads out; leave early | plan | household | ready_gov_evacuation | rr-supply |
| `evacuation_assistance_plan` | special needs | evacuate | evacuation_assistance_plan | people with `mobility` limited or wheelchair | 1 transport plan per person who needs help to leave | person | person | ready_gov_older_adults, ready_gov_disability | rr-supply |
| `once` | generic | any | (the item) | — | 1 per household | item | household | (the item's own) | built in |
| `per_person` | generic | any | (the item) | people | number of people | item | person | (the item's own) | implemented (requested by content) |
| `per_person_13_plus` | generic | any | (the item) | people | teens + adults + seniors | item | person | (the item's own) | implemented (requested by content) |
| `per_commuter` | generic | any | (the item) | people.commute | people with a commute of more than 0 km | item | commuter | (the item's own) | implemented (requested by content) |
| `per_vehicle` | generic | any | (the item) | mobility.vehicles | number of vehicles | item | household | (the item's own) | implemented (requested by content) |
| `per_pet` | generic | any | (the item) | pets | dogs + cats + small pets (large animals have their own rules) | item | pet | (the item's own) | implemented (requested by content) |
| `per_infant_or_toddler` | generic | any | (the item) | people | babies + toddlers (under 4) | item | person | (the item's own) | implemented (requested by content) |
| `once_if_daily_rx` | generic | any | (the item) | people.medical | 1 if anyone takes a daily prescription, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_refrigerated_rx` | generic | any | (the item) | people.medical | 1 if anyone takes a refrigerated prescription, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_powered_device` | generic | any | (the item) | people.medical | 1 if anyone uses a powered medical device, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_infant` | generic | any | (the item) | people | 1 if there is a baby under 1, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_children` | generic | any | (the item) | people | 1 if anyone is under 18, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_pets` | generic | any | (the item) | pets | 1 if there is a dog, cat or small pet, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_large_animals` | generic | any | (the item) | pets.large_animals | 1 if there are horses or livestock, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_vehicle` | generic | any | (the item) | mobility.vehicles | 1 if the household has a vehicle, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_ev` | generic | any | (the item) | mobility.vehicles | 1 if any vehicle is fully electric, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_commuter` | generic | any | (the item) | people.commute | 1 if anyone commutes, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_senior` | generic | any | (the item) | people | 1 if anyone is 65 or older, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_mobility_needs` | generic | any | (the item) | people.medical.mobility | 1 if anyone has limited mobility or uses a wheelchair, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_wheelchair` | generic | any | (the item) | people.medical.mobility | 1 if anyone uses a wheelchair, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_pregnant_or_nursing` | generic | any | (the item) | people | 1 if anyone is pregnant or nursing, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_renter` | generic | any | (the item) | housing.tenure | 1 if renting, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_well` | generic | any | (the item) | housing.water | 1 if the home has a private well, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_earner` | generic | any | (the item) | people.earner | 1 if anyone earns income, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_gas_service` | generic | any | (the item) | housing.heating | 1 if heating is natural gas or propane, else 0 | item | household | (the item's own) | implemented (requested by content) |
| `once_if_house` | generic | any | (the item) | housing.kind | 1 for detached, rowhouse, mobile home or rural property; 0 for apartments | item | household | (the item's own) | implemented (requested by content) |
| `once_if_near_nuclear_plant` | generic | any | (the item) | nuclear_16km (`SupplyContext`) | 1 if a nuclear plant is within 16 km (10 miles, the emergency planning zone), else 0 | item | household | (the item's own) | implemented (requested by content): the flag comes from `LocationResolved.facility_flags` through `SupplyContext.nuclear_plant_within_16km` |
| `once_if_generator` | generic | any | (the item) | housing.backup_power | 1 if the household already has a generator, else 0 (for generator upkeep items) | item | household | (the item's own) | rr-supply |

## Which bucket's days size which line

Duration lines use their own bucket's target, with these exceptions: `phone_power_wh` and
`rx_cold_storage` use the **power** target (phones die and medicine warms when the power is out);
`otc_medicines` and `ors_packets` use the **supplies** target (how long you can't reach a store; 14
days when there is none); `cooking_fuel_canisters` uses the longer of **supplies** and
**water_boil**, and `toilet_paper_rolls` the longer of **supplies** and **water_out**.
`bleach_bottles` uses no days at all (one bottle whatever the target). Heat lines (`battery_fan`,
`cooling_towel`, `cooling_plan`) appear when `heat_wave` drives the thermal bucket, cold lines
(`blankets`, `warm_layers`, `sleeping_bag_or_blanket`, `warm_room_plan`) when `cold_wave`,
`winter_weather` or `ice_storm` does, and both when the assessment names neither; the thermal
target decides whether `sleeping_bag_or_blanket` is a need (over 3 days) and its tier.

## Citation ids

Every citation id above is an entry in `content/citations.toml`. The `[[source]]` table at the end
of `crates/rr-supply/src/constants.toml` lists the ones the supply lines use (title, publisher and
URL copied from the registry); `rr_supply::citations_used()` returns them, and a test checks each
against `content/citations.toml`. `rr_expert_prior` is the registry's planning-estimate entry (with
`prior = true`); its URL points here, and the reasoning for each estimate is in the formula column.
PetMD and the Merck Veterinary Manual (pet water), Germany's and Denmark's water guides
(`bbk_vorsorgen_2025`, `dema_prepared_for_crises`), the CDC Yellow Book heat chapter
(`cdc_yellow_book_heat_cold`) and the generator maker's fuel figures (`honda_eu2200i_spec`) now have
their own registry entries, and the constants that use them cite them directly.
`rr_research_supply_standards` remains only where a number is genuinely this crate's own derivation
from the research report with no primary source of its own: the camp-stove fuel-per-person-day
assumption (`cooking_fuel_lb_per_person_day`) and the outage-length thresholds at which a generator
is offered (`generator_min_days`, `generator_min_days_well`). The bleach bottle is now one per
household (`bleach_bottles_per_household`, CDC) at CDC's ½ mL a gallon (`bleach_ml_per_gal`),
replaced every six months (`bleach_replace_months`, an estimate); the old two-week bottle life is
gone.
