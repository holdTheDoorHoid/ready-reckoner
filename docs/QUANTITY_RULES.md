# Quantity rules

The shared registry of `quantity_rule` names. Catalogue items (`content/items/*.toml`) name a rule;
`rr-supply` implements it and emits requirement lines under the same name. `rr-content`'s validator
parses the table below: every item's `quantity_rule` must be `once` or appear in the **Rule** column,
so a rule is added here before any item uses it.

## Conventions

- **Units.** A rule returns the household's total quantity in the item's own `unit`, with two
  exceptions: rules whose unit is `gallon` return water volume and rules whose unit is `kcal` return
  food energy. The budget converts those with the item's `volume_l_per_unit` or
  `energy_kcal_per_unit`, so a 7-gallon jug and a gallon of bottled water can both cover
  `water_gallons`.
- **Zero means not needed.** A rule that returns 0 keeps the item out of this household's plan. That
  is how conditional items work (no EV, no `once_if_ev` item).
- **`once` is built in:** 1 per household, for a one-time action or a single purchase.
- **Days** means the target of the named bucket, capped at the tier's horizon (DESIGN §4.5, §4.7).
  Where a rule names two buckets it uses the larger target.
- **People counts** use the age bands in DESIGN §4.1. "13+" means teen, adult and senior.
- **Status.** `requested by content` = named by the content agent for catalogue items and awaiting
  implementation; the supply agent sets `implemented` (or proposes a rename in this file, and the
  content agent follows it). Numbers marked *Prior* are expert estimates and cite `rr_expert_prior`;
  the reasoning is in the Formula column.

## Rules

| Rule | Inputs | Formula | Unit | Citations | Status |
|---|---|---|---|---|---|
| `once` | none | 1 per household | item unit | — | built in |
| `per_person` | people | number of people | item unit | ready_gov_kit | requested by content |
| `per_person_13_plus` | people | teens + adults + seniors (people who carry a phone and a bag) | item unit | ready_gov_kit | requested by content |
| `per_commuter` | people.commute | people with a commute | item unit | ready_gov_kit_2020 | requested by content |
| `per_vehicle` | mobility.vehicles | number of vehicles | item unit | ready_gov_winter | requested by content |
| `per_pet` | pets | dogs + cats + small pets (large animals have their own plan) | item unit | ready_gov_pets, aspca_disaster_prep | requested by content |
| `per_infant_or_toddler` | people | infants + toddlers (under 4) | item unit | cdc_infant_checklist | requested by content |
| `once_if_daily_rx` | people.medical | 1 if anyone takes a daily prescription, else 0 | item unit | ready_gov_disability | requested by content |
| `once_if_refrigerated_rx` | people.medical | 1 if anyone takes a refrigerated prescription, else 0 | item unit | ready_gov_power_outages | requested by content |
| `once_if_powered_device` | people.medical | 1 if anyone uses a powered medical device, else 0 | item unit | ready_gov_disability | requested by content |
| `once_if_infant` | people | 1 if there is an infant (under 1), else 0 | item unit | cdc_infant_checklist | requested by content |
| `once_if_children` | people | 1 if anyone is under 18, else 0 | item unit | ready_gov_plan | requested by content |
| `once_if_pets` | pets | 1 if there is a dog, cat or small pet, else 0 | item unit | ready_gov_pets | requested by content |
| `once_if_large_animals` | pets.large_animals | 1 if there are horses or livestock, else 0 | item unit | aspca_disaster_prep | requested by content |
| `once_if_vehicle` | mobility.vehicles | 1 if the household has a vehicle, else 0 | item unit | ready_gov_evacuation | requested by content |
| `once_if_ev` | mobility.vehicles | 1 if any vehicle is fully electric, else 0 | item unit | doe_fueleconomy_ev | requested by content |
| `once_if_commuter` | people.commute | 1 if anyone commutes, else 0 | item unit | ready_gov_kit_2020 | requested by content |
| `once_if_senior` | people | 1 if anyone is 65 or older, else 0 | item unit | ready_gov_older_adults | requested by content |
| `once_if_mobility_needs` | people.medical.mobility | 1 if anyone has limited mobility or uses a wheelchair, else 0 | item unit | ready_gov_disability | requested by content |
| `once_if_wheelchair` | people.medical.mobility | 1 if anyone uses a wheelchair, else 0 | item unit | ready_gov_disability | requested by content |
| `once_if_pregnant_or_nursing` | people | 1 if anyone is pregnant or nursing, else 0 | item unit | cdc_pregnancy_emergency | requested by content |
| `once_if_renter` | housing.tenure | 1 if renting, else 0 | item unit | fema_nhs_2024 | requested by content |
| `once_if_well` | housing.water | 1 if the home has a private well, else 0 | item unit | cdc_find_clean_water | requested by content |
| `once_if_earner` | people.earner | 1 if anyone earns income, else 0 | item unit | dol_unemployment_insurance | requested by content |
| `once_if_gas_service` | housing.heating | 1 if heating is natural gas or propane, else 0 | item unit | phmsa_gas_safety | requested by content |
| `once_if_house` | housing.kind | 1 for detached, rowhouse, mobile home or rural property; 0 for apartments | item unit | ready_gov_wildfires | requested by content |
| `once_if_near_nuclear_plant` | location.facility_flags | 1 if a nuclear plant is within 16 km (10 miles, the emergency planning zone), else 0 | item unit | nrc_potassium_iodide, cdc_potassium_iodide | requested by content |
| `water_gallons` | water_out and water_boil days, people, pets, dials.water_level, hot climate, pregnancy, nursing, formula infants, livestock, dehydrated-food person-days | research supply §1.6: Σ people × days × 1.0 gal (survival 0.8, comfortable 4.0) × climate (1.75–2.0 hot) + nursing 0.29 + pregnant 0.08 + formula infant 0.25 gal/day + pets 1 oz/lb/day + livestock 25 L/day + 0.6 gal per dehydrated-food person-day | gallon | ready_gov_water, cdc_water_storage, sphere_2018, iom_dri_water_2005, aap_formula_amounts | in supply brief |
| `water_reused_bottles` | water_out days, people | min(3 days of `water_gallons`, 6 gal). *Prior* cap: the clean drink bottles a household can collect in a month | gallon | cdc_water_storage, ready_gov_water, rr_expert_prior | requested by content |
| `water_treatment_capacity` | water_out days, housing.water | 1 when the water_out target is over 14 days or the home has a well (a filter plus a raw water source beats stockpiling for the long tail), else 0. *Prior* threshold | item unit | cdc_water_disinfection, rr_research_risk_model, rr_expert_prior | in supply brief |
| `bleach_bottles` | water_out and water_boil days | max(1, ceil(days ÷ 14)) bottles of 64–121 oz. *Prior*: one bottle covers a household's water treatment and cleaning for two weeks | item unit | cdc_bleach_disinfecting, epa_emergency_disinfection, rr_expert_prior | requested by content |
| `food_kcal` | supplies days, people (age band, pregnancy, nursing) | Σ person kcal/day (DGA 2020–2025 Table A2-2, moderately active; pregnancy +340/+452, use +400 when the trimester is unknown; nursing +330/+400) × days | kcal | usda_dga_2020_2025, sphere_2018 | in supply brief |
| `cooking_fuel_canisters` | water_boil and supplies days, people | max(2, ceil(people × days × 0.4)) 8-ounce butane canisters. *Prior*: about 0.2 lb of fuel per person-day boils drinking water and heats one meal (1 lb of propane boils about 25–30 L, research supply §2.6); an 8-ounce canister holds 0.5 lb | item unit | eia_btu, lehi_fuel_storage, rr_research_supply_standards, rr_expert_prior | requested by content |
| `infant_formula_oz` | supplies days, infants | formula-fed infants × max(days, 3) × 5 oz of powder. *Prior*: 32 fl oz of prepared formula a day (the AAP ceiling) takes about 5 oz of powder; labels differ | item unit | aap_formula_amounts, cdc_infant_feeding_disaster, cdc_infant_checklist, rr_expert_prior | requested by content |
| `pet_food_days` | supplies days, pets | (dogs + cats + small pets) × max(days, 7) pet-days | item unit | aspca_disaster_prep, ready_gov_pets | requested by content |
| `medication_days` | medication days, people.medical.daily_rx | people with a daily prescription × reserve days (default 14; 7–30) | item unit | cdc_pregnancy_emergency, redcross_survival_kit, florida_dem_medication, cdc_diabetes_emergencies | in supply brief |
| `first_aid_kit` | people | ceil(people ÷ 4) kits (the Red Cross list is for four people) | item unit | redcross_first_aid_kit | requested by content |
| `n95_masks` | people | people aged 2 and older × 5 masks per smoke or illness episode. *Prior*: five days of exposure, one mask a day | item unit | cdc_masks, rr_expert_prior | requested by content |
| `toilet_bags` | water_out days, people | ceil(people × days × 0.45) bags (range 0.3–0.6). *Prior* from Oregon's 5 gallons of waste per person per week and the fill-half-then-double-bag rule | item unit | oregon_b2wr_toolkit, rdpo_emergency_toilet, rr_expert_prior | requested by content |
| `toilet_paper_rolls` | supplies and water_out days, people | ceil(people × days ÷ 5) rolls. *Prior*: about one roll per person per 5 days; Oregon advises doubling your own weekly use | item unit | oregon_b2wr_toolkit, rr_expert_prior | requested by content |
| `soap_person_months` | supplies days, people | people × ceil(days ÷ 30) person-months (250 g bath soap and 200 g laundry soap each) | item unit | sphere_2018 | requested by content |
| `menstrual_cycles` | supplies days, people | people who menstruate × (2 + floor(days ÷ 28)) cycles. *Prior*: sex is not asked, so half of teens and adults under 65 | item unit | cdc_period_factsheet, sphere_2018, rr_expert_prior | requested by content |
| `diapers` | supplies days, people | (infants × 8 + toddlers × 6) × max(days, 3). *Prior* from newborns' 8–12 changes a day and at least six wet diapers a day after the first week | item unit | cdc_infant_checklist, sutter_diapers, rr_expert_prior | requested by content |
| `battery_packs` | power days | max(1, ceil(days ÷ 7)) packs of AA/AAA cells. *Prior*: one pack runs a household's lights and radio for a week | item unit | ready_gov_kit, rr_expert_prior | requested by content |
| `power_station_units` | power days, people.medical | with a powered medical device or refrigerated medicine: ceil(device Wh per night × nights ÷ 900 usable Wh), capped at 2; otherwise 1 when the power target is 3 days or more (an optional upgrade), else 0. CPAP 96–170 Wh a night; a top-freezer fridge about 995 Wh a day | item unit | sil_cpap_power, epa_energy_star_refrigerators, rr_research_supply_standards, rr_expert_prior | requested by content |
| `generator_units` | power days, housing, water | 0 for apartments (no safe spot 20 ft from openings); for houses, 1 when the power target is 3 days or more, or 1 day or more with a well; else 0 | item unit | cdc_co_basics, ready_gov_power_outages, rr_research_supply_standards | requested by content |
| `generator_fuel_cans` | power days, generator | 0 without a generator; else ceil(min(2.8 gal a day × days, 25 gal) ÷ 5) five-gallon cans (a 2.2 kW inverter unit at quarter load; the 25-gallon cap is the fire-code limit, 10 gal in an attached garage) | item unit | rr_research_supply_standards, lehi_fuel_storage | requested by content |
| `solar_panel_units` | power days | 1 when the power target is 7 days or more, else 0. *Prior* threshold | item unit | nlr_pvwatts_v8, rr_expert_prior | requested by content |
| `device_battery_units` | people.medical.powered_device | people with a powered medical device | item unit | sil_cpap_power, ready_gov_disability | requested by content |
| `cash_reserve_usd` | finances, household | the user's own days × daily cash spend; default $100 per household until they enter theirs. *Prior* (no authority gives a figure) | item unit | ready_gov_financial, fema_effak, rr_expert_prior | requested by content |
| `smoke_alarm_count` | housing, people | levels (apartment or mobile home 1, house 2, +1 basement) + sleeping rooms (ceil(people ÷ 2)). *Prior* for the level and bedroom counts, which the interview does not ask | item unit | usfa_smoke_alarms, ready_gov_home_fires, rr_expert_prior | requested by content |
| `co_alarm_count` | housing | one per level with sleeping areas (apartment or mobile home 1, house 2). *Prior* for the level count | item unit | cdc_co_basics, ready_gov_power_outages, rr_expert_prior | requested by content |
| `extinguisher_count` | housing | one per level (apartment or mobile home 1, house 2, +1 basement). *Prior* for the level count | item unit | usfa_extinguishers, rr_expert_prior | requested by content |
| `escape_ladder_count` | housing | 1 for a house with bedrooms upstairs (detached, rowhouse, rural) or an apartment on floor 2–3; else 0. *Prior* | item unit | ready_gov_home_fires, rr_expert_prior | requested by content |
