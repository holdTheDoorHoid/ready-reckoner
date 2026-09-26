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

**Release follow-ups (2026-09-26).** A household on a well with horses or livestock keeps the pump
running on a generator in a power cut: when its no-water target is longer than 3 days and a
generator suits the home, `generator_units` is a need (unless it owns a generator), with the fuel
to keep for it as a note line (`generator_fuel_gallons`), and `livestock_water` stores only the
first 3 days
(`livestock_pump_bridge_days`); two weeks in stock tanks becomes an alternative line (new rule
`livestock_water_stored`). 14 days of animal water remain the need only where no pump power is
planned. Hays: 1,109.5 gallons of stock tank become 237.8.

**Round 2, v0.1.1 (2026-09-26).** Safety and correctness from the round-2 review. Refrigerated
medicine (S1): the one-day discard rule is gone; an insulated bag counts about 1 day
(`cooler_hold_days`, *Prior*); insulin keeps working up to 28 days between 59 °F and 86 °F (FDA);
past a 2-day power target with no backup power a battery power station is a need
(`power_station_units`, item class `power_for_cold_medicine`). Thermometers for the fridge and the
room are needs wherever a power or heat target exists (new rules `fridge_thermometers`,
`room_thermometer`). The well pump (S6): pump power exists only with a generator the household owns
and the interlock that connects it, so animals store 14 days until then; a pump-rated generator is a
need only when a power cut can outlast that water, and comes with an electrician-installed interlock
or transfer switch (new rule `generator_connection_units`, life-safety) and approved fuel cans (new
rule `fuel_cans`); the filter's line names its raw-water source. Smoke alarms: free routes first,
and renters' alarms are the landlord's (a note). The bleeding-control kit gets its own rule
(`bleeding_control_kit`) so it can be life-safety where it matters. New generic switch
`once_if_house_ground_floor` for the outdoor motion light.

**Round 2, v0.2.0 (2026-09-26).** Capabilities before consumables (practitioner review P-01 to
P-23; the 28 rule changes are listed with their tests under "Round 2 rule changes, as implemented"
below). Batteries stop at two weeks (`battery_pack_cap_days`) and a way to recharge takes over (new
rule `recharge_capability`: a car charger, or a solar panel for a household with no vehicle); toilet
paper follows the store target only; generator fuel is counted in run-hours; the medicine reserve
claims at most 30 days and 60- to 90-day fills cover the rest (new rule `medication_fills`); a
12-volt fridge keeps refrigerated medicine cold in hot counties (new rule `rx_fridge_units`); every
long store target and every formula-fed baby under a boil notice gets a way to cook without power
(new rule `cooking_capability`, propane where winters freeze, with `propane_cylinders` and
`grill_propane_tank`); ready-to-feed formula covers a baby's first three days (new rule
`infant_formula_rtf`); cash follows the household's expenses; an escape ladder wherever people sleep
upstairs; bleach sits in the three-day tier and is replaced every six months; fans and cooling
towels are optional in hot counties, where the cooling plan is the cover; the bleeding-control kit
is life-safety for rural homes, with a wound-care add-on (new rule `wound_care_addon`); a walk-home
filter only for long walks (new rule `get_home_filter`); toilet bags for the first month unless an
earthquake drives the target. The clean-air bucket (contract v2) gets respirators for teens and
adults by the county's smoke days, an air cleaner sized by EPA's table or the cheaper filter box,
and a clean-room plan. The long-horizon section gets rain barrels by the state's rainfall and rules,
water carriers, a hauling tote for animals, a well hand pump option, household kits and a free
pointer (new switch `once_if_long_horizon`). Decisions (insurance, ID, home repairs) are free in the
budget: four new insurance rules and four owner switches. Every line carries its share of the
bare-minimum kit (`SizedLine::minimum`), and `rr_supply::storage_by_tier` gives the stored supplies'
space and weight. The sections after "Which bucket's days size which line" explain the new item
fields: `requires`, `readiness_share`, `season` and `test_interval_months`.

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
| `water_reused_bottles` | water | water_out (alternative of `water_gallons`) | water_stored | people, pets, water_level, hot, target(water_out) | min(stored days × the per-day water, 6 gal), stored days = min(target, 14); *Prior* cap: the clean drink bottles a household can gather. At the cap the line cites only the cap's sources; under it (small households) the amount is all the stored days' water and it cites the `water_gallons` sources instead | gallon | household | rr_expert_prior, rr_research_risk_model, cdc_water_storage, ready_gov_water, church_emergency_prep_manual (under the cap: the `water_gallons` sources instead of the first two) | implemented (requested by content). Polish round: cites only the sources behind its own amount. Round 2, v0.2.0 (review P-13): the whole stored target, not 3 days, so the Chicago student's 5-gallon target is met by bottles filled for free |
| `water_treatment_capacity` | water | water_boil, water_out | water_treatment_capacity | people, pets, hot, target(water_boil), target(water_out), housing.water, housing.raw_water_source, the `rain_catchment_units` line | Lines, in gallons of water to make safe: water_boil = (people × 0.75 gal drinking share, × 2 when hot, + pregnancy/nursing + formula + pets) × target days, whatever the water level; water_out = the full per-day need × (target − 14) days, only when the target is longer than 14 days ("a filter plus a water source beats storing more"). The water_out line names the raw-water source: the household's well (with a way to run its pump in a power cut: a generator through an electrician-installed interlock or transfer switch, or a hand pump), the source it named (`Housing.raw_water_source`: a stream or pond, a neighbour's well, its own rain barrel), or the rain barrels the plan makes a need; with none, it says a filter adds nothing yet and points to the carriers line. Item quantity: 1 filter when the home has a well, or when the water_out target is over 14 days and a source exists; else 0 | gallon | household | cdc_water_disinfection, epa_emergency_disinfection, byu_longer_term_storage_2019, oregon_b2wr_toolkit | implemented. Round 2 (v0.1.1): the water_out line names the raw-water source. Round 2, v0.2.0 (review P-03): a filter counts only with a source; with none the item is 0 and the line says so |
| `bleach_bottles` | water | water_boil (water_out when there is no boil target) | water_treatment_capacity | — | 1 bottle of plain unscented 5–9 % bleach per household, whatever the target: at ½ mL a gallon (CDC) each fluid ounce treats about 60 gallons of clear water (30 cloudy), so one bottle treats thousands of gallons. A fresh bottle every 6 months, when stored water is rotated (*Prior*: bleach weakens in storage and no agency gives a shelf life; up to 12) | bottle | household | cdc_water_storage, epa_emergency_disinfection, cdc_water_disinfection, cdc_bleach_disinfecting, rr_expert_prior | implemented (requested by content). Polish round: one bottle, not one per 14 days of the longer water target (27 bottles for a year); the days no longer size it, so the line does not cite the target |
| `boil_fuel` | water | water_boil (optional) | water_treatment_capacity | treatment gallons | litres to boil ÷ 27.5 L per lb of propane (*Prior*, 25–30); outdoors only; no more than 2 one-pound cylinders indoors | lb | household | eia_btu, lehi_fuel_storage, cdc_co_basics, rr_expert_prior | rr-supply |
| `rain_catchment_units` | water | water_out (a need; `.optional` for washing and flushing only; `.note` where rain cannot help) | raw_water_source | housing (a house, `raw_water_source`, well), target(water_out), drought, the per-day water, `SupplyContext.state_fips` | Only for a house on town water with a no-water target over 14 days, no drought behind it and no raw-water source named. Barrels (50 gal each, `rain_barrel_gal`) = clamp(ceil(monthly water ÷ a barrel's dry-month catch), 1, 4 or the state's limit), where a barrel's catch = 250 sq ft of roof (`rain_catchment_sqft_per_barrel`, *Prior*, 150–500) × 0.623 gal per sq ft per inch × the state's driest three months between April and October ÷ 3 (NOAA nClimDiv 1991–2020 normals; barrels freeze in winter). If even the allowed barrels would bring in less than a quarter of the monthly water (`rain_dry_season_min_share`, *Prior*), the line is a note that points to hauling. The state's rule is NCSL's map (September 2025): where it does not let a household drink rainwater the line is optional, for washing and flushing, and says the rule; limits (Colorado 110 gal in two barrels, California under 360 gal, Utah two containers unregistered) cap the count. No state known: 1 barrel, optional | barrel | household | rr_expert_prior, rr_price_observations_2026_09, cdc_water_disinfection, noaa_nclimdiv, ncsl_rainwater | implemented (Round 2, v0.2.0, review P-03, item N-04) |
| `water_carriers` | water | water_out (a need over 14 days; optional for a well or a high-rise) | water_carrier | target(water_out), people, housing.water, housing.kind | 2 carriers of about 5 gal (`water_carrier_gal`), 4 for a household of 4 or more (*Prior*); a full one weighs about 42 lb (8.34 lb a gallon). A need when the no-water target is longer than the 14 stored days (the rest is hauled from a distribution point, a neighbour's well or a spring, as in Asheville after Helene); an option for a well or a tall building with a shorter target | carrier | household | rr_expert_prior, epa_asheville_boil_notice_2024, rr_price_observations_2026_09 | implemented (Round 2, v0.2.0, review P-03, item N-05) |
| `well_hand_pump` | water | water_out (optional) | well_hand_pump | housing.water = well, target(water_out) | 1 hand pump for the well when the no-water target is over 30 days (`hand_pump_min_days`, *Prior*; research risk-model §9.6), where the water level is not too deep; deep wells need a costlier pump | pump | household | rr_expert_prior, rr_research_risk_model | implemented (Round 2, v0.2.0, review P-02/P-03, item N-07; a long-horizon item) |
| `livestock_water` | water | water_out | livestock_water | pets.large_animals, target(water_out), housing.water, backup_power, existing (`power_transfer_interlock`) | 25 L (20–30) per large animal per day × stored days ÷ 3.785. Only where pump power actually exists (a well, a generator the household owns, and the interlock or transfer switch that connects it, listed as owned), stored days = min(target, 3) (`livestock_pump_bridge_days`, *Prior*: until the generator runs the pump), and the line says to let the generator keep the pump going in a longer power cut, and to haul water in when a drought drives the target; `livestock_water_stored` is then the alternative. Otherwise (a generator merely planned counts for nothing yet) stored days = min(target, 14): no more than the 14 days of water a household stores (`water_stored_cap_days`, *Prior* for animals: the same cap as people's stored water), and beyond 14 days the line says a longer power cut needs a pump-rated generator connected through an interlock or transfer switch, and a drought needs water hauled in, instead of buying more tank space | gallon | pet | sphere_2018, aspca_disaster_prep (with pump power also rr_expert_prior; above 14 days without it also byu_longer_term_storage_2019, ensign_2006_year_supply, oregon_b2wr_toolkit, washington_prepare_in_a_year, rr_expert_prior) | rr-supply. Capped at 14 days (2026-09-26): a 60-day target for 12 animals was 4,755 gal; it is now 1,109.5. Round 2 (v0.1.1, review P-02): 3 days only once pump power exists, not as soon as a generator is planned; Hays 237.8 → 1,109.5 gal again |
| `livestock_water_stored` | water | water_out (alternative of `livestock_water`, only when it stores 3 days) | livestock_water | pets.large_animals, target(water_out) | 25 L per large animal per day × min(target, 14) days ÷ 3.785: two weeks in stock tanks instead of relying on the generator for the pump (a stock tank also holds water hauled in). No catalogue item uses this rule; it says what storing instead of pumping would take. Only where pump power exists (see `livestock_water`) | gallon | pet | sphere_2018, aspca_disaster_prep (above 14 days also byu_longer_term_storage_2019, ensign_2006_year_supply, oregon_b2wr_toolkit, washington_prepare_in_a_year, rr_expert_prior) | implemented (release follow-ups, 2026-09-26) |
| `livestock_haul_tank` | water | water_out | livestock_water | pets.large_animals, target(water_out), drought | 1 food-grade tote of about 275 gal (`livestock_haul_tank_gal`) that fits a pickup, when there are horses or livestock and a drought drives the no-water target or it runs past 14 days; the line says how many days a load lasts (25 L an animal a day, Sphere). Its item class is the animals' water, so it is part of the same cover as the stock tank | tote | household | rr_price_observations_2026_09, sphere_2018, aspca_disaster_prep | implemented (Round 2, v0.2.0, review P-02/P-03, item N-06) |
| `food_kcal` | food | supplies | food | people, target(supplies) | Σ people: DGA 2020–2025 Table A2-2, moderately active, men's and women's rows averaged year by year over the band (toddler 1,067, child 1,667, teen 2,280, adult 2,262, 65+ 2,009; babies 0, see `infant_formula_oz`) + 400 per pregnant or nursing person (the form does not say trimester or month); × days | kcal | person | usda_dga_2020_2025 | implemented (in the supply brief) |
| `food_cost_estimate` | food | supplies (alternatives of `food_kcal`) | food_pantry, food_bulk_staples, food_freeze_dried | people, target(supplies) | pantry: person-days × $8.44 × USDA household-size factor (+20 % for 1 person … −10 % for 7+, babies not counted); bulk staples: person-days × $2.50 ($2.15–2.85); freeze-dried: kcal ÷ 2,000 × $24 ($9–39). Cost comparison only; no item uses this rule | usd | household | usda_tfp_aug2026, church_hsc_order_form_2026, byu_longer_term_storage_2019, rr_price_observations_2026_09 | rr-supply |
| `food_kit_check` | food | supplies (note) | food_kit | people | days a retail "30-day" kit really lasts = 30 × 1,290 kcal ÷ household average kcal per person-day (kits supply 1,290–1,730 kcal a day) | day | person | rr_price_observations_2026_09, usda_dga_2020_2025 | rr-supply |
| `long_term_staples` | food | supplies (alternatives of `food_kcal`, target over 30 days) | staples_grains, staples_legumes, staples_dry_milk, staples_sugar, staples_fruit_veg, staples_salt_leavening, staples_oil, staples_vitamin | people, target(supplies) | for the days beyond 30: BYU 2019 per-adult-year amount × (days − 30) ÷ 365 × Σ adult-equivalents (Ensign 2006 child shares with one year added: 50 % to age 3, 70 % 4–6, 90 % 7–10, 100 % 11+; babies 0) | lb, gallon, tablet | person | byu_longer_term_storage_2019, ensign_2006_year_supply, church_hsc_order_form_2026, usu_food_storage_booklet | rr-supply |
| `cooking_capability` | food | supplies (a need; `.note` when the home already cooks without power) | cooking | target(supplies), target(water_boil), formula-fed babies, housing.heating, housing.cooking, heat or cold hazards | 1 way to cook and boil water without power when the store target is at least 14 days (`cooking_capability_min_days`, *Prior*; Oregon's 2 Weeks Ready asks households to know how to prepare two weeks of food without electricity or gas) or a formula-fed baby's powder must be mixed during a boil-water target. A wood stove or a gas range (`Housing.cooking = gas`) already covers it: a note, nothing to buy. Otherwise a one-burner camp stove, burning propane where a cold hazard drives the heat-or-cold bucket (butane stops turning to gas near 32 °F, propane near −44 °F; NIST), outdoors only (CDC). Tier: three days for the formula case, two weeks otherwise | stove | household | rr_expert_prior, oregon_2_weeks_ready, cdc_infant_feeding_disaster, cdc_co_basics, ready_gov_winter, nist_butane, nist_propane | implemented (Round 2, v0.2.0, review P-07; REVIEW K1) |
| `cooking_fuel_canisters` | food | supplies (a need when `cooking_capability` is; otherwise optional) | cooking | people, target(supplies), target(water_boil), heat or cold hazards | Where no cold hazard drives the heat-or-cold bucket: max(2, ceil(fuel lb ÷ 0.5 lb)) 8-ounce butane canisters, where fuel lb = people × the longer of the store and boil targets × 0.2 lb (*Prior*: boils drinking water and heats one meal) + the dry staples beyond the first month ÷ 2,000 kcal × 0.3 lb (`staples_fuel_lb_per_2000kcal`, *Prior*). Where a cold hazard drives it (or none is named), `propane_cylinders` instead | canister | household | rr_research_supply_standards, eia_btu, cdc_co_basics, rr_price_observations_2026_09, rr_expert_prior | implemented (requested by content). Round 2, v0.2.0 (review P-07): fuel for staples, a need with the stove, butane only where winter does not come |
| `propane_cylinders` | food | supplies (a need when `cooking_capability` is; otherwise optional) | cooking | as `cooking_fuel_canisters` | Where a cold hazard drives the heat-or-cold bucket or none is named: max(2, ceil(fuel lb ÷ 1 lb)) one-pound propane cylinders (`propane_cylinder_lb`), the fuel lb of `cooking_fuel_canisters`; stored outside the living space, no more than 2 inside (`propane_small_cylinders_indoor_max`, Lehi) | cylinder | household | as `cooking_fuel_canisters`, lehi_fuel_storage | implemented (Round 2, v0.2.0, review P-07, item N-08) |
| `grill_propane_tank` | food | supplies (optional) | cooking | housing.kind | 1 spare filled 20-lb cylinder for a house with an outdoor gas grill: the cheapest boiling and cooking fuel, outdoors only. The interview does not ask about a grill, so the line stays optional until it does | cylinder | household | lehi_fuel_storage, cdc_co_basics | implemented (Round 2, v0.2.0, item N-09; a grill question would make it an alternative of `cooking_capability`) |
| `infant_formula_rtf` | food | supplies | infant_formula | formula-fed babies | babies on formula × 32 fl oz (`infant_formula_oz_day`, AAP's most) × 3 days (`baby_supply_min_days`) of ready-to-feed formula: it needs no water, so it is the safest formula in an emergency (CDC). Life-safety, three-day tier | fl oz | person | aap_formula_amounts, cdc_infant_checklist, cdc_infant_feeding_disaster | implemented (Round 2, v0.2.0, review P-17, item N-11) |
| `infant_formula_oz` | food | supplies | infant_formula | formula-fed babies (`dietary` mentions formula), target(supplies) | babies on formula × (days − 3) × 5 oz of powder, only when the target is longer than the 3 days of ready-to-feed formula (`infant_formula_rtf`); *Prior*: 32 oz of prepared formula a day (the AAP most) takes about 5 oz of powder, labels differ. Life-safety | oz | person | aap_formula_amounts, cdc_infant_feeding_disaster, cdc_infant_checklist, rr_expert_prior | implemented (requested by content). Round 2, v0.2.0 (review P-17): powder only after the first 3 days |
| `nursing_supplies` | special needs | supplies | nursing_supplies | babies not on formula | 1 manual pump + 1–2 boxes of nursing pads per breastfed baby | kit | person | cdc_infant_checklist | rr-supply |
| `pet_food_lb` | food | supplies | pet_food | dogs, cats, small pets, target(supplies) | (dogs × 0.7 + cats × 0.15 + small pets × 0.05) lb a day × max(days, 7); *Prior* feeding amounts for a 40 lb dog and a 10 lb cat (the sizes the water rule assumes); ASPCA advises 7–10 days | lb | pet | aspca_disaster_prep, ready_gov_pets, rr_expert_prior | implemented (requested by content) |
| `pet_food_days` | food | supplies (alternative of `pet_food_lb`) | pet_food | dogs, cats, small pets, target(supplies) | (dogs + cats + small pets) × max(days, 7) pet-days (ASPCA 7–10 days), for items counted in days rather than pounds | pet_day | pet | aspca_disaster_prep, ready_gov_pets | implemented (requested by content) |
| `medication_days` | medication | medication | prescription_medicine | people with `daily_rx` or `refrigerated_rx`, target(medication) | people on prescriptions × reserve days (the target clamped to 7–30; 14 when there is no target) | person_day | person | florida_dem_medication, cdc_diabetes_emergencies, cdc_pregnancy_emergency, redcross_survival_kit, healthcare_ready_refill_laws | implemented. Differs from the request: refrigerated prescriptions (insulin) count too |
| `medication_fills` | medication | medication | prescription_medicine | people with `daily_rx` or `refrigerated_rx`, target(medication) | people on prescriptions × (target − 30) person-days, only when the target is longer than the 30-day reserve `medication_days` keeps: kept up by 60- to 90-day fills (`rx_fill_days`, Medicare) and early refills, never a bigger stockpile. Content's free step `med_90_day_fills` meets it | person_day | person | medicare_drugs_disaster, florida_dem_medication, cdc_diabetes_emergencies, redcross_survival_kit, healthcare_ready_refill_laws | implemented (Round 2, v0.2.0, review P-04: "30 days of medicine adds 90") |
| `rx_cold_storage` | medication | medication | medicine_cooler | people with `refrigerated_rx`, target(power), housing.backup_power | When a power source is needed (someone takes refrigerated medicine, the power target is 2 days or more (`rx_power_min_days`, *Prior*) and the household has no generator, power station or solar battery of its own): days of cold storage = the power target, of which an insulated bag with fresh cold packs covers about 1 day (`cooler_hold_days`, *Prior*, 0.5–2) and a battery power station the rest (`power_station_units` is then a need). Otherwise min(target, 1 day): the bag's day (a short target, or backup power the household has keeps the fridge running; for up to 2 days the line says to replace the cold packs with ice). The medication target stands in when there is no power target. The line says insulin in its vial or pen keeps working up to 28 days between 59 °F and 86 °F (FDA), so the job is shade and below 86 °F, never frozen; for any other refrigerated medicine, ask the pharmacist now | day | household | fda_insulin_emergency, cdc_insulin_emergency, rr_expert_prior | Round 2 (v0.1.1, review S1): the one-day discard rule (Ready.gov) is gone, and the bag counts 1 day, never the whole target. rr-plan's coverage table must count the bag as 1 day of the line (and the power station as the rest) for the plan's numbers to follow; see the p1-supply report |
| `rx_fridge_units` | medication | medication (life-safety) | medicine_cooler | people with `refrigerated_rx`, target(power), hot, housing.backup_power | 1 small 12-volt compressor fridge when someone takes refrigerated medicine, the county is hot, the power target is at least 3 days (`rx_fridge_min_days`, *Prior*) and no generator or solar battery keeps the house fridge running: insulin keeps working up to 28 days between 59 °F and 86 °F (FDA), which a home without power in a hot county passes. It runs from the power station or the car (engine outdoors). Its class is the cold-storage part, with the bag and the station | fridge | household | rr_expert_prior, fda_insulin_emergency, nca5_atlas, cdc_co_basics | implemented (Round 2, v0.2.0, review P-01, item N-15) |
| `antibiotics_none` | medication | medication | antibiotics_clinician_card | — | always 0: no antibiotic quantity is ever sized; the line points to the clinician card | course | household | cdc_antibiotic_use, fda_fish_antibiotics_warning_2023, fda_expired_medicines, cdc_yellow_book_travel_kits | rr-supply |
| `epinephrine_check` | medication | medication | epinephrine_auto_injector | people with `epinephrine` | 1 per person who carries an auto-injector (check dates; ask the prescriber how many to keep) | person | person | ready_gov_disability | rr-supply |
| `medical_device_wh` | power | power | medical_device_power | people's `powered_device`, target(power) | CPAP 170 Wh a night (96 with the humidifier off); oxygen concentrator 300 W × 24 h (*Prior*; use the label); other device watts × 24 h (*Prior*); × days | Wh | person | sil_cpap_power, ready_gov_disability, ready_gov_power_outages, rr_expert_prior | rr-supply |
| `device_battery_units` | power | power | device_battery | people's `powered_device` | people with a powered medical device | battery | person | sil_cpap_power, ready_gov_disability | implemented (requested by content) |
| `lights` | power | power | light | people | 1 battery light per person aged 4 and over, at least 1 (*Prior*) | light | person | ready_gov_kit, ready_gov_power_outages, rr_expert_prior | rr-supply |
| `battery_packs` | power | power | battery_pack | target(power) | max(1, ceil(min(days, 14) ÷ 7)) packs of AA/AAA cells; *Prior*: one pack runs a household's lights and radio for a week, and past two weeks (`battery_pack_cap_days`, *Prior*) a way to recharge (`recharge_capability`) beats more batteries | pack | household | ready_gov_kit, rr_expert_prior, oregon_2_weeks_ready | implemented (requested by content). Round 2, v0.2.0 (review P-04): capped at 14 days (Coos Bay's 180 days was 26 packs, now 2) |
| `recharge_capability` | power | power | recharge | target(power), people's `powered_device`, housing.backup_power, mobility.vehicles | 1 way to recharge from the car (a 12-volt charger or a small inverter, about 150 W) when the power target is longer than the 14 days of batteries and phone power the plan stores, or someone uses a powered medical device, and the household has a vehicle and no generator or solar battery of its own (either can recharge; a power station cannot recharge itself). Engine outdoors only (CDC). A household with no vehicle gets `solar_panel_units` as a need instead, in the same `recharge` class. Tier: the power target's past 14 days, else three days | charger | household | rr_expert_prior, oregon_2_weeks_ready, cdc_co_basics, sil_cpap_power | implemented (Round 2, v0.2.0, review P-04/P-09, item N-14) |
| `power_station_units` | power | power (a need with a medical device or with refrigerated medicine that needs a power source; otherwise optional) | power_station (`power_for_cold_medicine` when it is the cold-medicine need) | devices, `refrigerated_rx`, target(power), housing.backup_power | with a powered medical device: ceil(device Wh a day × days ÷ 909 usable Wh), 1 to 2 (*Prior* cap); with refrigerated medicine, a power target of 2 days or more (*Prior*) and no backup power: 1, a need of its own part (a cooler bag lasts about a day; it runs a small 12-volt cooler or the fridge in spells, recharged from a car with the engine outdoors or a solar panel); with refrigerated medicine otherwise: 1, optional; otherwise 1 when the power target is 3 days or more (an optional upgrade), else 0. A station is 1,070 Wh, 85 % usable (*Prior*). An electric car is not counted as a power source: the engine cannot tell whether it can power a cooler | power station | household | sil_cpap_power, epa_energy_star_refrigerators, rr_price_observations_2026_09, rr_expert_prior (the cold-medicine need also cdc_co_basics) | implemented (requested by content). Round 2 (v0.1.1, review S1): a need for refrigerated medicine past 2 days |
| `generator_units` | power | power (optional; a need on a well with large animals when a power cut can outlast their stored water) | generator | target(power), housing.kind, housing.water, pets.large_animals, target(water_out), backup_power | 0 for apartments (no safe spot 20 ft from openings); for houses, 1 when the power target is 3 days or more, or 1 day or more on a well; else 0 (*Prior* thresholds). On a well the optional line says a generator runs the pump only if it supplies the pump's starting watts (2–3 × running, 240 V if the pump needs it) and only through an electrician-installed interlock or transfer switch, never a wall outlet. The line is a need (`power.generator_units`) for a household on a well with large animals that owns no generator and whose power target is longer than the animals' stored water (min(no-water target, 14) days): a pump-rated generator, with `generator_connection_units` and `fuel_cans` | generator | household | cdc_co_basics, ready_gov_power_outages, rr_research_supply_standards, rr_expert_prior (on a well also cpsc_generator_safety_alert, osha_portable_generators; the need line: cdc_co_basics, ready_gov_power_outages, aspca_disaster_prep, rr_expert_prior, cpsc_generator_safety_alert, osha_portable_generators) | implemented (requested by content). Round 2 (v0.1.1, review S6/P-02): pump-rated, connected only through an interlock or transfer switch; a need only when a power cut can outlast the stored water (Hays: power target 3 days, 14 days stored: optional) |
| `generator_fuel_gallons` | power | power (a note for a generator the plan buys) | generator_fuel | `backup_power` = generator, target(power), the `generator_units` need | 0 without a generator; else min(2.8 gal a day × days, 25 gal) (a 2.2 kW inverter unit at quarter load, 7.1 at full load; 25 gal is the fire-code limit, 10 in an attached garage, none indoors). The line's days are the days the fuel lasts at light load (gallons ÷ 2.8), never the target, and the sentence gives run-hours: 25 gal is about 214 h at light load (85 at full load), 8.9 days nonstop or 21 to 54 days at 4 hours a day (`generator_rationed_hours_per_day`, *Prior*) | gallon | household | honda_eu2200i_spec, lehi_fuel_storage, cdc_co_basics, ready_gov_power_outages, rr_expert_prior | implemented (requested by content): a need for a generator the household owns; a note for the pump generator its plan buys; an optional generator gets no fuel line. Round 2, v0.2.0 (review P-04): fuel in run-hours, and the line's days are the fuel's, so a gallon is never read as a week |
| `generator_connection_units` | power | power (a life-safety need) | generator_connection | housing.kind, housing.water, housing.tenure, backup_power, the `generator_units` need | 1 inlet with an interlock or transfer switch, installed by a licensed electrician, for a house on a well that its household owns, with a generator it owns or the pump generator its plan buys: a well pump is wired into the house, and OSHA says to connect a generator to a building only through a transfer switch a qualified electrician installed; CPSC warns that plugging one into a wall outlet (backfeeding) can electrocute utility workers and neighbours. Life-safety, so it never comes after the generator. Renters ask the landlord (no line) | installed kit | household | osha_portable_generators, cpsc_generator_safety_alert | implemented (round 2, v0.1.1, review S6/P-02) |
| `fuel_cans` | power | power | generator_fuel (an owned generator's fuel), generator (the pump generator the plan buys) | the `generator_fuel_gallons` line or note | ceil(fuel gallons ÷ 5) approved 5-gallon gasoline cans (`fuel_can_gal`), with fuel stabilizer; kept outside the living space, away from a gas water heater (CPSC), fuel used within 6 months | can | household | rr_price_observations_2026_09, lehi_fuel_storage, cpsc_generator_safety_alert | implemented (round 2, v0.1.1, review S6/P-02) |
| `solar_panel_units` | power | power (optional; a need as the way to recharge for a household with no vehicle) | solar_panel (`recharge` as the need) | target(power), lat, devices, people aged 13+, mobility.vehicles | 1 when the power target is 7 days or more, else 0 (*Prior*); the line gives December output for the latitude band (NLR PVWatts cities: under 36° N 382, 36–42° 261, 42–46° 219, 46° and north 121 Wh a day per 100 W; *Prior* proxy) and the panel watts the essential load needs. A need, whatever the target, when `recharge_capability` would be needed but the household has no vehicle | panel | household | nlr_pvwatts_v8, epa_energy_star_refrigerators, rr_expert_prior (as the recharge need also oregon_2_weeks_ready) | implemented (requested by content). Round 2, v0.2.0 (review P-04): the way to recharge without a car |
| `fridge_wh` | power | power (optional) | fridge_power | target(power) | 995 Wh a day (ENERGY STAR median top-freezer; chest freezer 597) × days; a closed fridge holds about 4 hours, a full freezer about 48 | Wh | household | epa_energy_star_refrigerators, ready_gov_food, ready_gov_power_outages | rr-supply |
| `fridge_thermometers` | power | power | fridge_thermometer | target(power) | 1 pair of appliance thermometers, one in the fridge and one in the freezer, whenever there is a power target: after a power cut, keep refrigerated food only if it stayed at 40 °F or below, and throw out perishable food above 40 °F for 2 hours or more | pair | household | ready_gov_food | implemented (round 2, v0.1.1, review RR-P11): the plan's own 40 °F rule depends on it |
| `well_pump_wh` | power | power (optional) | well_pump_power | `water` = well, target(power) | 1,250 W (1,000–1,500) × 1 h a day × days; starting takes 2–3 × the running watts (*Prior*; check the nameplate) | Wh | household | rr_expert_prior | rr-supply |
| `wheelchair_battery` | special needs | power | wheelchair_battery | people with `mobility` = wheelchair | 1 spare battery per wheelchair user (if the chair is powered) | battery | person | ready_gov_disability | rr-supply |
| `noaa_radio` | comms | comms | weather_radio | — | 1 battery or hand-crank radio with NOAA Weather Radio per household | radio | household | ready_gov_kit, nws_weather_radio | rr-supply |
| `phone_power_wh` | comms | comms | power_bank | people aged 13+, target(power) | phones (1 per person aged 13+) × 15 Wh a day (*Prior*, 10–20) × min(days without power, 14) (the comms target when there is no power target); past 14 days the line points to the recharge line | Wh | person | ready_gov_kit, ready_gov_earthquakes, rr_expert_prior, oregon_2_weeks_ready | rr-supply. Round 2, v0.2.0 (review P-04): capped at the 14 days of phone power the plan stores |
| `two_way_radios` | comms | comms | two_way_radio | people aged 13+ | 1 per person aged 13+ when there are at least 2 (*Prior*); FRS needs no licence, GMRS $35 for 10 years | radio | person | fcc_frs, fcc_gmrs, ecfr_47_1_1102, rr_expert_prior | rr-supply |
| `contact_cards` | comms | comms | contact_card | people aged 4+ | 1 written contact card per person aged 4 and over | card | person | ready_gov_plan, ready_gov_low_cost | rr-supply |
| `local_map` | comms | comms | paper_map | — | 1 paper map of the area per household | map | household | ready_gov_kit | rr-supply |
| `cash_reserve_usd` | comms | comms | cash | finances.monthly_expenses_usd, target(comms) | max($100, floor(0.5 × 3 days × monthly expenses ÷ 30 ÷ $20) × $20) (`cash_essential_share`, `cash_expense_days`, `cash_round_usd`, all *Prior*: no agency gives a figure); $100 until the household gives its expenses, with the days of basics that should cover (the comms target clamped to 3–14). Philadelphia ($4,200) $200, Sugar Land ($7,000) $340, Chicago ($1,400) $100 | usd | household | ready_gov_financial, fema_effak, rr_expert_prior | implemented (requested by content). Round 2, v0.2.0 (review P-18): from the household's own expenses |
| `toilet_buckets` | sanitation | water_out | toilet_bucket | — | 2 buckets (pee and poo) with a seat per household | bucket | household | rdpo_emergency_toilet, oregon_b2wr_toolkit | rr-supply |
| `toilet_bags` | sanitation | water_out | toilet_bags | people over 1, target(water_out), earthquake among the no-water target's hazards | ceil(people × days × 0.45) bags (0.3–0.6, *Prior*), counting both bags of a double-bagged load, where days = min(target, 30) (`toilet_supplies_cap_days`, *Prior*) unless an earthquake drives the target (it can break the sewer itself); past the cap the line says a working sewer or septic system is the better toilet | bag | person | rdpo_emergency_toilet, oregon_b2wr_toolkit, rr_expert_prior | implemented (requested by content); babies in diapers are not counted. Round 2, v0.2.0 (review P-16): capped at 30 days (a year without an earthquake: 27 bags for two, not 329) |
| `toilet_cover_material` | sanitation | water_out | toilet_cover_material | people over 1, target(water_out), earthquake | 1 cup (0.5–1.5, *Prior*) of sawdust, shredded paper or similar per person-day, for the days `toilet_bags` counts | cup | person | rdpo_emergency_toilet, rr_expert_prior | rr-supply. Round 2, v0.2.0: capped with the bags |
| `toilet_paper_rolls` | sanitation | supplies | toilet_paper | people over 1, target(supplies) | ceil(people × store target ÷ 5) rolls; *Prior*: about a roll a person every 5 days (Oregon: measure a week's use and double it) | roll | person | oregon_b2wr_toolkit, rr_expert_prior | implemented (requested by content); babies in diapers are not counted. Round 2, v0.2.0 (review P-04): the store target only; how much paper you use has nothing to do with the tap (Coos Bay: 24 rolls, not 146) |
| `household_ops_kits` | sanitation | water_out | household_ops | target(water_out) | ceil(min(days, 30) ÷ 14) kits of paper plates, cups and utensils, garbage bags with ties and waterproof matches (`household_ops_kit_days`, `household_ops_cap_days`, *Prior*; all on Ready.gov's kit list), only when the no-water target is longer than the 14 stored days: paper plates save the water washing dishes takes, and after the first month dishes are washed with treated water (at most 3 kits) | kit | household | rr_expert_prior, ready_gov_kit | implemented (Round 2, v0.2.0, review P-23, item N-16) |
| `soap_person_months` | sanitation | supplies | soap | people, target(supplies) | people × ceil(days ÷ 30) person-months, each 250 g of bathing soap and 200 g of laundry soap | person_month | person | sphere_2018 | implemented (requested by content) |
| `menstrual_cycles` | sanitation | supplies | menstrual_products | adults and teens not pregnant or nursing, target(supplies) | ceil(half of them × cycles each), cycles each = 2, + 1 per 28 days beyond a month; about 20 products a cycle (15–30); *Prior*: sex is not asked | cycle | person | cdc_period_factsheet, sphere_2018, oregon_b2wr_toolkit, rr_expert_prior | implemented. Differs from the request: 2 cycles cover plans up to a month, as the research reads the CDC "2 cycles" advice (the request's 2 + floor(days ÷ 28) gives 3 at 28 days) |
| `diapers` | sanitation | supplies | diapers | babies and toddlers, target(supplies) | (babies × 8 + toddlers × 6) × max(days, 3); *Prior* from newborns' 8–12 changes a day | diaper | person | sutter_diapers, cdc_infant_checklist, rr_expert_prior | implemented (requested by content) |
| `baby_wipes` | sanitation | supplies | baby_wipes | babies and toddlers, target(supplies) | 2 packs per child per 2 weeks (at least 2); rounded up | pack | person | cdc_infant_checklist | rr-supply |
| `first_aid_kit` | first aid | medical_emergency | first_aid_kit | people | ceil(people ÷ 4) kits (the Red Cross list is for four people), at least 1 | kit | household | redcross_first_aid_kit | implemented (requested by content) |
| `otc_medicines` | first aid | medical_emergency | otc_medicines | people, target(supplies) | 4 kinds (pain reliever, anti-diarrhea, antacid, allergy medicine) × packages of each = max(1, days ÷ 14 × people ÷ 4) rounded up (*Prior* scaling; 14 days when there is no supplies target) | package | household | ready_gov_kit, cdc_yellow_book_travel_kits, redcross_first_aid_kit, rr_expert_prior | rr-supply. Round 2, v0.2.0 (review P-22): an allergy medicine in place of the laxative, matching the catalogue item |
| `n95_masks` | clean air | clean_air | respirator | people aged 13+, `SupplyContext.smoke_days` | people aged 13+ × 1 a day × days, days = the county's days a year of unhealthy smoke (PM2.5 at or above 35.5 µg/m³), at least 5 (`n95_days`) and at most 30 (`respirator_days_max`), 5 when not known (*Prior*); NIOSH respirators come in adult sizes, and for children the clean room is the protection (EPA) | mask | person | cdc_masks, cdc_wildfire_smoke, epa_children_wildfire_smoke, rr_expert_prior | implemented. Round 2, v0.2.0 (review P-22, P-05 and the clean-air bucket): teens and adults only, sized by smoke days, and moved from medical emergencies to clean air |
| `thermometer` | first aid | medical_emergency | thermometer | babies | 1 oral thermometer + 1 infant thermometer when there is a baby | thermometer | household | redcross_first_aid_kit, cdc_infant_checklist | rr-supply |
| `ors_packets` | first aid | medical_emergency | oral_rehydration_salts | people, target(supplies) | 3 packets per person per 2 weeks (*Prior*), at least 2 weeks' worth, each into 1 L of safe water | packet | person | cdc_cholera_treatment, cdc_yellow_book_travel_kits, rr_expert_prior | rr-supply |
| `bleeding_control_kit` | first aid | medical_emergency | bleeding_control_kit | location.setting, target(medical_emergency) | 1 kit (a tourniquet and a pressure bandage) with a Stop the Bleed class, for every household. Life-safety where the setting is rural (ambulances take longer, Mell 2017) or the medical-emergency ten-year chance is at least 0.5 (`bleeding_kit_life_safety_p10`, *Prior*, standing in for "the medical-emergency card ranks in the top three": the rules do not see the register). The threshold orders the plan and is not cited on the line | kit | household | dhs_stop_the_bleed (rural: also mell_2017_ems_response) | implemented (round 2, v0.1.1, review P-11) |
| `wound_care_addon` | first aid | medical_emergency | wound_care | location.setting, target(supplies) | 1 wound-care and splint add-on (irrigation syringe, closure strips, elastic wrap, padded splint, shears, blister dressings, packing gauze; Wilderness Medical Society) for a rural home (three-day tier; ambulances take longer, Mell 2017) or a store target of at least 14 days (`wound_addon_min_days`, *Prior*; two-week tier) | kit | household | wms_wound_2014, redcross_first_aid_kit (rural: also mell_2017_ems_response; otherwise also rr_expert_prior) | implemented (Round 2, v0.2.0, review P-11, item N-12) |
| `air_cleaner_units` | clean air | clean_air | air_cleaner | — | 1 portable air cleaner with a HEPA filter for the clean room, with a clean air delivery rate of at least 0.65 cfm per square foot (EPA's sizing table, 8-foot ceiling) × a 200 sq ft bedroom (`clean_room_sqft`, *Prior*) = 130 | air cleaner | household | epa_air_cleaner_guide, epa_wildfire_indoor_air, rr_expert_prior | implemented (Round 2, v0.2.0, the clean-air bucket) |
| `diy_filter_box` | clean air | clean_air (alternative of `air_cleaner_units`) | air_cleaner | — | 1 box fan with a MERV 13 filter taped to it, which EPA calls a cost-effective way to cut smoke indoors: the cheaper way to meet the air-cleaner line, never an addition | filter box | household | epa_diy_air_cleaners | implemented (Round 2, v0.2.0, the clean-air bucket); the catalogue prices it inside the one clean-room air cleaner item (`fire_clean_air_room`), so the plan never buys both |
| `clean_room_plan` | clean air | clean_air | clean_room_plan | people | 1 plan: one room with few windows and doors, kept closed, the heating or cooling set to recirculate, the air cleaner running, no frying, candles or vacuuming on smoky days; children stay in it | plan | household | epa_wildfire_indoor_air, cdc_wildfire_smoke | implemented (Round 2, v0.2.0, the clean-air bucket) |
| `battery_fan` | thermal | thermal (heat; `.optional` in a hot county) | thermal_heat | people, hot | 1 per household + 1 per person 65+, baby or pregnant (*Prior*); fans help only below 90 °F indoors. In a hot county (30+ days a year at 95 °F) a home without power soon passes 90 °F, so the fans are an optional comfort there and do not count as heat cover | fan | household | cdc_heat_health, rr_expert_prior (hot: also nca5_atlas, stone_2023_heat_blackout) | rr-supply. Round 2, v0.2.0 (review P-09): optional in hot counties |
| `cooling_towel` | thermal | thermal (heat; `.optional` in a hot county) | thermal_heat | people, hot | 1 per person (*Prior*); optional in a hot county, like the fans | towel | person | rr_expert_prior | rr-supply. Round 2, v0.2.0 (review P-09): optional in hot counties |
| `cooling_plan` | thermal | thermal (heat) | thermal_heat | hot, target(power) | 1 plan: nearest cooling centre (dial 2-1-1), coolest room, check-ins. In a hot county it is the heat cover itself, with a written trigger ("if the power is off and it is 90 °F inside, we go to ___") and a ride, and it is life-safety when there is a power target of a day or more | plan | household | cdc_heat_health, ready_gov_heat (hot: also nca5_atlas, rr_expert_prior) | rr-supply. Round 2, v0.2.0 (review P-09): the place to go is the cover in hot counties |
| `room_thermometer` | thermal | thermal (heat) | thermal_heat | — | 1 room thermometer, in the room where the most vulnerable person sleeps, wherever the heat lines appear: fans help only while it is below 90 °F indoors (CDC), so it tells you when to go somewhere cooler | thermometer | household | cdc_heat_health | implemented (round 2, v0.1.1, review RR-P11) |
| `blankets` | thermal | thermal (cold) | thermal_cold | people | 1 warm blanket per person aged 1 and over (babies: warm sleepwear or a sleep sack instead of loose blankets, CDC). Most homes have them: content's assumed-basic blanket item meets this line when it uses `quantity_rule = "blankets"` | blanket | person | ready_gov_kit, sphere_2018, cdc_winter_safety | rr-supply (polish round, for content's assumed basics) |
| `sleeping_bag_or_blanket` | thermal | thermal (cold; a need, or `.optional`) | thermal_cold | people, target(thermal), housing.heating | The extra layer beyond `blankets` and `warm_layers`: a sleeping bag or an extra heavy blanket. A need for everyone aged 1 and over when the target is longer than 3 days (*Prior*), otherwise for each person 65 or over (CDC: most at risk in the cold). An optional upgrade (`thermal.sleeping_bag_or_blanket.optional`, 1 per person aged 1+) with a wood stove (it heats without power; a gas furnace is not assumed to) or for a short target with nobody 65+. Babies get a sleep sack, never loose bedding | item | person | ready_gov_kit, sphere_2018, rr_expert_prior, cdc_winter_safety | rr-supply. Polish round: it was 1 per person for any cold target (4 sleeping bags for Philadelphia's 1.7 days; now 1, for the senior). rr-plan's `thermal_sleeping_bag` → `thermal.sleeping_bag_or_blanket` entry still holds (an optional line matches nothing, so no bag is bought); an item may also use this rule directly |
| `warm_layers` | thermal | thermal (cold) | thermal_cold | people | 1 set of warm layers (coat, hat, gloves) per person. Most homes have them: content's assumed-basic warm-layers item meets this line when it uses `quantity_rule = "warm_layers"` | set | person | ready_gov_kit, cdc_winter_safety | rr-supply |
| `warm_room_plan` | thermal | thermal (cold) | thermal_cold | housing.heating, people | 1 plan: one warm room, towels under doors, blankets over windows, the nearest warming centre, never unvented combustion indoors; wording follows the heating | plan | household | cdc_winter_safety, cdc_co_basics, ready_gov_stay_safe_warm | rr-supply. Round 2, v0.2.0 (review P-22): names the warming centre |
| `fire_escape_plan` | fire | fire | fire_escape_plan | housing.kind, housing.floor, housing.below_grade_bedroom, alarms.smoke | 1 plan for every household: two ways out of every room, a meeting spot outside, a practice run; stairways in a building; a house at street level: upstairs bedrooms need a second way out (the ladder line); a bedroom below ground needs its own second way out; monthly alarm tests when there are alarms | plan | household | ready_gov_home_fires | rr-supply. Round 2, v0.2.0: points to the ladder, and names below-grade bedrooms (contract v2) |
| `smoke_alarm_count` | fire | fire (a need for owners; a note for renters) | smoke_alarm | housing.kind, housing.tenure, basement, alarms.smoke, people | 0 when the household has smoke alarms; else levels (apartment or mobile home 1, house 2, +1 basement) + bedrooms (ceil(people ÷ 2)); *Prior* level and bedroom counts. Free routes first: some fire departments install alarms at no cost (USFA) and the Red Cross installs them free where it runs home visits. Renters ask the landlord first, so for them the line is a note (`fire.smoke_alarm_count.note`) and the plan buys no alarms; owners ask the fire department or the Red Cross first and buy only if they cannot | alarm | household | usfa_smoke_alarms, ready_gov_home_fires, redcross_sound_the_alarm, rr_expert_prior | implemented (requested by content); 0 when `alarms.smoke` is true, so the plan never buys alarms a home already has. Round 2 (v0.1.1, review RR-P16): free routes first; renters' alarms are the landlord's |
| `co_alarm_count` | fire | fire | co_alarm | housing.kind, alarms.co | 0 when the household has CO alarms; else one per level where people sleep (apartment or mobile home 1, house 2); *Prior* level count | alarm | household | cdc_co_basics, ready_gov_power_outages, rr_expert_prior | implemented (requested by content); 0 when `alarms.co` is true |
| `extinguisher_count` | fire | fire | fire_extinguisher | housing.kind, alarms.extinguisher | 0 when the household has one; else 1 multipurpose (A-B-C) extinguisher per floor people live on (apartment or mobile home 1, house 2; not the basement; *Prior*), the ground-floor one near the kitchen. The form does not say where the kitchen is, so it is assumed to be on the floor with the way out; the line says to add one where it is not, and that small home fires are common (CPSC: 6.6 a year per 100 households, about 3.4 % attended by fire departments) | extinguisher | household | usfa_extinguishers, cpsc_unreported_fires_2009, rr_expert_prior | implemented (requested by content); 0 when `alarms.extinguisher` is true. Polish round: the basement no longer adds one (Philadelphia 3 → 2). Round 2 (v0.1.1, review P-06): the catalogue item is life-safety and the budget values it on CPSC's all-fires rate |
| `escape_ladder_count` | fire | fire | escape_ladder | housing.kind, housing.floor | 1 two-storey ladder for a house (rowhouse, detached, rural property) at street level, since the form does not ask and a house is assumed to have bedrooms upstairs (*Prior*, from the 2 levels it is counted with; the line says to skip it if every bedroom is on the ground floor); 1 sized to the floor for any home but a mobile home whose household lives on floor 2 or 3; else 0 (a ladder reaches no higher; the stairs are the way out) | ladder | household | ready_gov_home_fires, rr_expert_prior | implemented (requested by content). Polish round: a house at street level no longer got a ladder. Round 2, v0.2.0 (review P-06): it does again, because rowhouse and house bedrooms are usually upstairs |
| `neighbour_contacts` | security | security | neighbour_contacts | — | 2 neighbours' numbers swapped, and who checks on whom (*Prior*) | contact | household | rr_expert_prior | rr-supply |
| `document_kit` | documents | home_loss | document_kit | — | 1 Emergency Financial First Aid Kit (4 parts: ID, financial and legal, medical, contacts), copies kept safe and updated yearly | kit | household | fema_effak | rr-supply |
| `insurance_home_or_renters` | documents | home_loss | insurance_home_or_renters | finances.insurance.home_or_renters = false, housing.tenure, housing.kind | 1 decision: renters, homeowners or (for an owned apartment) condominium unit-owners insurance | decision | household | fema_effak, ready_gov_financial (owned apartment: also naic_home_insurance_guide) | rr-supply. Round 2, v0.2.0: names the unit-owners form for condo owners |
| `insurance_flood` | documents | home_loss | insurance_flood | finances.insurance.flood = false, `SupplyContext.sfha_home_share` | 1 decision for every household without flood insurance: standard policies exclude floods, a new policy usually waits 30 days, NFIP pays up to $250,000 building / $100,000 belongings, and about 3 in 10 flood insurance claims (29 percent, 2014–2024) come from outside high-risk flood areas (FloodSmart); where most of the county's homes are outside the mapped zone the line says so | decision | household | fema_nfip_flood_insurance, floodsmart_buy_policy, ready_gov_financial, floodsmart_flood_risk (low zone share: also openfema_nfip) | rr-supply. Round 2, v0.2.0 (review RR-P09): the outside-the-zone share, for every household without a policy |
| `insurance_earthquake` | documents | home_loss | insurance_earthquake | finances.insurance.earthquake = false, earthquake among home_loss's contributing hazards | 1 decision; standard policies exclude earthquake damage | decision | household | ready_gov_earthquakes | rr-supply |
| `insurance_wind_deductible` | documents | home_loss | insurance_wind_deductible | finances.insurance.home_or_renters, hurricane among home_loss's contributing hazards | 1 decision: check the policy for a separate hurricane or windstorm deductible, which in some places is a percentage of the insured value, and whether wind is covered at all on the coast (NAIC); add it to the savings goal | decision | household | naic_home_insurance_guide | implemented (Round 2, v0.2.0, review RR-P09) |
| `insurance_sewer_backup` | documents | home_loss | insurance_sewer_backup | housing.basement, finances.insurance (home_or_renters, sewer_backup) | 1 decision for a home with a basement and a policy, unless the household holds the cover: most policies pay little or nothing for water that backs up through drains or a sump pump that overflows; ask about a sewer and drain backup endorsement (NAIC) | decision | household | naic_home_insurance_guide | implemented (Round 2, v0.2.0, review RR-P09) |
| `insurance_condo_unit` | documents | home_loss | insurance_condo_unit | housing (an owned apartment), finances.insurance.home_or_renters | 1 decision for an owned apartment with a policy: check it is a condominium unit-owners policy, which covers belongings and the unit's walls, floors and ceilings (NAIC), and what the association's policy leaves to the owner | decision | household | naic_home_insurance_guide | implemented (Round 2, v0.2.0, review RR-P09) |
| `insurance_life_disability` | documents | income | insurance_life_disability | people's `earner`, finances.insurance.life_or_disability | 1 decision when someone earns money and the household has not said it holds the cover: disability insurance through work, and life insurance when others rely on the income; about 1 in 4 workers who start at 20 become disabled before retirement age (SSA) | decision | household | ssa_disability_facts, naic_life_insurance_guide | implemented (Round 2, v0.2.0, review RR-P09: the register's "death or disability of an earner" gets its paired action) |
| `emergency_fund_months` | documents | income | emergency_fund | target(income), finances | months = income target (3 when absent; planners suggest 3–6, the CFPB says it depends); dollars in the text when monthly expenses are given. Savings track, never the supplies budget | month | household | finra_financial_foundations, stlouisfed_emergency_fund_2025, cfpb_emergency_fund | rr-supply |
| `get_home_bag` | get home | get_home | get_home_bag | each person's `commute` (distance above 0) | 1 bag per commuter (a line per person): walking shoes, a light, map, cash, a weather layer, and water and snacks from home (that person's staged lines, below); kept in the car (car commuters) or at work; the walk home's hours at 3 mph (*Prior*); be ready to shelter at work 24 h | bag | commuter | ready_gov_kit_2020, ready_gov_kit, fhwa_mutcd_walking_speed, rr_expert_prior | rr-supply. Polish round: the water and food amounts moved to the staged lines |
| `get_home_water` | get home | get_home (staged: `get_home.get_home_bag.alt.staged_water.person_N`) | walking_water | commute distance, hot | hours = miles ÷ 3 mph (*Prior*; 2–2.4 loaded); litres = hours × 0.5 L an hour (*Prior*; 0.71, NIOSH, when hot); past 2.5 L carry a filter (the filter line), or in a hot county more water. Filled from home, never bought extra | litre | commuter | cdc_niosh_heat_hydration, fhwa_mutcd_walking_speed, rr_expert_prior | rr-supply. Polish round: staged from household supplies. Round 2, v0.2.0 (review P-15): carry more water in a hot county |
| `get_home_food` | get home | get_home (staged: `get_home.get_home_bag.alt.staged_food.person_N`) | walking_food | commute distance | hours = miles ÷ 3 mph (*Prior*); kcal = hours ÷ 12 × 1,250 (*Prior*, 1,000–1,500), rounded to 100 kcal, at least 100. From the household's food, never bought extra | kcal | commuter | fhwa_mutcd_walking_speed, usda_dga_2020_2025, rr_expert_prior | rr-supply (polish round; the amount was in the bag line's text) |
| `get_home_filter` | get home | get_home (`get_home.get_home_filter.person_N`) | walking_filter | commute distance, hot | 1 small filter (0.3 micron or smaller, CDC) or purification tablets for each commuter whose walk home needs more than the 2.5 L of water worth carrying (`walk_water_carry_max_l`, *Prior*), outside hot counties (there may be no water to filter; the staged water line says to carry more) | filter | commuter | fhwa_mutcd_walking_speed, rr_expert_prior, cdc_water_disinfection | implemented (Round 2, v0.2.0, review P-15: Philadelphia bought filters for walks needing 2 L and 0.6 L) |
| `car_kit` | get home | get_home | car_kit | mobility.vehicles | 1 car emergency kit per vehicle (jumper cables or a charged jump pack you have tried, a light, warm clothes, a blanket, water, snacks, the roadside-assistance number on paper) | kit | household | ready_gov_winter | rr-supply. Round 2, v0.2.0: the jump pack and roadside number (the Deviant Ollam lessons) |
| `go_bag` | evacuate | evacuate | go_bag | people aged 4+, target(evacuate) notice and days away | 1 bag per person aged 4 and over (babies' things go in a parent's bag); a bag already owned will do; documents on paper and a flash drive, an old pair of prescription glasses; water, food and medicines come from the household's supplies (the staged lines, below); where it lives follows the notice band (under 1 h minutes, under 24 h hours, else days; *Prior* band edges) | bag | person | washington_prepare_in_a_year, ready_gov_kit, cdc_evacuation_psa, rr_expert_prior | rr-supply. Round 2, v0.2.0: the document drive and old glasses (the Deviant Ollam lessons) |
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
| `once_if_house_ground_floor` | generic | any | (the item) | housing.kind, housing.floor | 1 for a house (detached, rowhouse, mobile home or rural property) whose household lives at floor 1 or below, else 0: an outdoor light at the door or path | item | household | (the item's own) | implemented (round 2, v0.1.1, review P-14) |
| `once_if_near_nuclear_plant` | generic | any | (the item) | nuclear_16km (`SupplyContext`) | 1 if a nuclear plant is within 16 km (10 miles, the emergency planning zone), else 0 | item | household | (the item's own) | implemented (requested by content): the flag comes from `LocationResolved.facility_flags` through `SupplyContext.nuclear_plant_within_16km` |
| `once_if_generator` | generic | any | (the item) | housing.backup_power | 1 if the household already has a generator, else 0 (for generator upkeep items) | item | household | (the item's own) | rr-supply |
| `once_if_homeowner` | generic | any | (the item) | housing.tenure | 1 if the household owns its home, else 0 | item | household | (the item's own) | implemented (Round 2, v0.2.0, for owners' decisions) |
| `once_if_owned_house` | generic | any | (the item) | housing.tenure, housing.kind | 1 for a house (detached, rowhouse, mobile home or rural property) the household owns, else 0: roof, safe-room and wildfire decisions, and the key safe (renters keep a spare key with a neighbour, the free lockout plan) | item | household | (the item's own) | implemented (Round 2, v0.2.0, review RR-P10) |
| `once_if_owned_detached` | generic | any | (the item) | housing.tenure, housing.kind | 1 for a detached house or one on rural land that the household owns, else 0: bolting a house to its foundation applies to a house on its own foundation | item | household | (the item's own) | implemented (Round 2, v0.2.0, review RR-P10) |
| `once_if_owned_basement` | generic | any | (the item) | housing.tenure, housing.kind, housing.basement | 1 for an owned house with a basement, else 0: a backflow valve and a sump pump with battery backup | item | household | (the item's own) | implemented (Round 2, v0.2.0, review RR-P10) |
| `once_if_long_horizon` | generic | any | (the item) | the bucket targets, dials.long_horizon | 1 when the plan has a long-horizon section: some duration target is at least 30 days (`long_horizon_min_days`, *Prior*, the design threshold) or the household turned the section on; else 0. Switches on the section's free pointers (`food_going_further`) | item | household | rr_expert_prior | implemented (Round 2, v0.2.0, the long-horizon section) |

## Which bucket's days size which line

Duration lines use their own bucket's target, with these exceptions: `phone_power_wh`,
`rx_cold_storage` and `rx_fridge_units` use the **power** target (phones die and medicine warms when
the power is out); `otc_medicines`, `ors_packets` and `wound_care_addon` use the **supplies** target
(how long you can't reach a store; 14 days when there is none); `cooking_fuel_canisters` and
`propane_cylinders` use the longer of **supplies** and **water_boil**. Since v0.2.0
`toilet_paper_rolls` follows **supplies** only (review P-04), `household_ops_kits` covers at most the
first 30 days of **water_out**, and `toilet_bags` and `toilet_cover_material` the first 30 unless an
earthquake drives it. `n95_masks` counts the county's **smoke days** (`SupplyContext::smoke_days`),
not a target. `bleach_bottles` uses no days at all (one bottle whatever the target), and neither do
`fridge_thermometers`, `room_thermometer`, `generator_connection_units`, `fuel_cans` (sized by the
fuel line), `bleeding_control_kit`, `infant_formula_rtf` (always three days), the clean-air lines
(`air_cleaner_units`, `diy_filter_box`, `clean_room_plan`), `grill_propane_tank` or the insurance
decisions. Heat lines (`battery_fan`, `cooling_towel`, `cooling_plan`, `room_thermometer`) appear
when `heat_wave` drives the thermal bucket, cold lines (`blankets`, `warm_layers`,
`sleeping_bag_or_blanket`, `warm_room_plan`) when `cold_wave`, `winter_weather` or `ice_storm` does,
and both when the assessment names neither; the thermal target decides whether
`sleeping_bag_or_blanket` is a need (over 3 days) and its tier. The clean-air lines appear only when
the bucket's ten-year chance is at least 2 in 100 (`clean_air_min_p10`, DESIGN §4.4's bar for
readiness capabilities).

## Prerequisites: `requires`

`Item.requires` (contract v2) lists what an accessory needs first, so the allocator never buys the
batteries before a light or the fuel before its can (review P-12). **The list names alternatives:
the item may be bought once any one of them is owned or bought the same month or earlier.** A single
entry is therefore a plain prerequisite, and a chain says the order: the generator, then its fuel
cans, then the fuel. Decisions are never required.

| Item | Requires (any one) | Why |
| --- | --- | --- |
| Spare batteries (`power_batteries`) | `power_headlamp`, `power_lantern`, `comms_noaa_radio` | spare batteries for a light or the radio |
| Generator fuel (`power_generator_fuel`) | `power_fuel_cans` | gasoline only in approved cans (CPSC) |
| Fuel cans and the interlock (`power_fuel_cans`, `power_transfer_interlock`) | `power_generator` | the cans and the connection serve a generator |
| Stove fuel (`food_cooking_fuel`, `food_propane_cylinders`) | `food_camp_stove` | fuel for the stove |
| Toilet bags (`san_toilet_bags`) | `san_twin_bucket_toilet` | bags line the buckets |
| Baby wipes (`san_baby_wipes`) | `san_diapers` | wipes go with diapers |
| Wound-care add-on (`med_wound_splint_addon`) | `med_first_aid_kit` | an add-on to the kit |
| Whistle (`evac_whistle`) | `evac_go_bag` | it goes in the bag |
| Foil blankets (`thermal_emergency_blankets`) | `evac_go_bag`, `gethome_bag`, `gethome_car_kit` | foil blankets go in a bag or the car |
| Walk-home filter (`water_personal_filter`) | `gethome_bag` | it goes in the walk-home bag |
| Chlorine dioxide (`water_chlorine_dioxide`) | `gethome_bag`, `evac_go_bag`, `water_filter_gravity` | for a bag, or after a filter |

## Readiness share

Readiness buckets have no days: an item that meets one of their lines, or lists one first among
its buckets, avoids the bucket's harm each time it is needed (rr-plan's `READINESS_HARM`). Before
v0.2.0 every such item earned the whole of it, so a whistle outranked a headlamp and a HEPA cleaner
counted as medical-emergency care (review P-05). `Item.readiness_share` (contract v2) is the share
of that harm the item averts on its own, and every item that lists a readiness bucket carries one
(a catalogue test checks it). The values are estimates, set on one scale:

- **1.0, the capability itself:** the thing that makes the household ready, with nothing else on the
  checklist doing its job: the go-bag, the get-home bag, the first-aid kit, working smoke alarms and
  the escape plan, emergency alerts, the evacuation plan, a ride out for a household with no car,
  the evacuation assistance plan, knowing two neighbours, and the clean room's air cleaner (one item:
  a HEPA unit or the cheaper box fan with a MERV 13 filter, since readiness credit is per item and
  two items would both be bought).
- **0.5, a second core piece:** needed for the capability to work in a common case, or the core for
  part of the household: the contact card, drills, the 48-hour list, pet and infant plans and the
  pet go-kit, the older-adult and livestock plans, the car kit, the bleeding-control kit, the device
  power plan and epinephrine, the carbon monoxide alarm, an extinguisher, an escape ladder, home
  security basics, respirators, and the clean-room plan.
- **0.2 to 0.3, supporting items:** they help a capability without being it: the weather radio, the
  documents step and pouch, cash, the half tank, a headlamp for the go-bag, the jump starter, the
  medicine list, the wound-care add-on, the pregnancy plan, over-the-counter medicines, shut-offs
  and the shut-off wrench, water-heater straps, the motion light, the key safe, the shelter-in-place
  kit and cleanup gear.
- **0.05 to 0.1, accessories:** worth having, but no one is ready because of them: the whistle,
  foil blankets, chlorine dioxide, the walk-home filter, warm layers for the walk, staged pet food
  and water, the ID decision, gloves, the thermometer, the crisis line step and firearm storage
  (which is about safety at home, not security).
- **0.0:** the antibiotics conversation, which is not medical-emergency care (review P-05).

| Bucket credited | Share | Items |
| --- | --- | --- |
| evacuate | 1.0 | `comms_wea_alerts_on`, `evac_know_zone`, `evac_go_bag`, `evac_ride_plan`, `special_access_needs_plan` |
| evacuate | 0.5 | `comms_contact_card`, `evac_ten_minute_drills`, `evac_forecast_48h_checklist`, `special_pet_plan`, `special_pet_kit`, `special_infant_go_kit`, `special_older_adult_plan`, `special_livestock_plan` |
| evacuate | 0.2–0.3 | `comms_noaa_radio`, `docs_effak`, `evac_half_tank` (0.3); `docs_document_pouch`, `docs_cash_reserve`, `power_headlamp` (0.2) |
| evacuate | 0.05–0.1 | `decide_id_for_every_person`, `special_pet_go_water`, `special_pet_go_food` (0.1); `evac_whistle`, `special_pet_food`, `thermal_emergency_blankets` (0.05) |
| get_home | 1.0, 0.5, 0.3 | `gethome_bag`; `gethome_car_kit`; `gethome_jump_pack` |
| get_home | 0.05–0.1 | `thermal_warm_layers`, `water_personal_filter` (0.1); `water_chlorine_dioxide` (0.05) |
| medical_emergency | 1.0 | `med_first_aid_kit` |
| medical_emergency | 0.5 | `med_device_power_plan`, `med_bleeding_control_kit`, `med_epinephrine_plan` |
| medical_emergency | 0.2–0.3 | `med_list_written`, `med_wound_splint_addon`, `special_pregnancy_plan` (0.3); `med_otc_basics` (0.2) |
| medical_emergency | 0–0.1 | `med_988_saved`, `med_thermometer`, `docs_legal_readiness` (0.1); `san_nitrile_gloves` (0.05); `med_antibiotics_clinician_card` (0) |
| fire | 1.0 | `fire_test_alarms`, `fire_smoke_alarm` |
| fire | 0.5 | `fire_co_alarm`, `fire_extinguisher`, `fire_escape_ladder` |
| fire | 0.2–0.3 | `fire_learn_shutoffs` (0.3); `fire_utility_wrench`, `water_heater_strap_kit` (0.2) |
| security | 1.0, 0.5 | `community_know_two_neighbours`; `security_home_basics`, `community_trusted_circle` |
| security | 0.1–0.3 | `security_lockout_plan` (0.3); `security_motion_light`, `security_key_safe` (0.2); `security_firearms_safe_storage` (0.1) |
| clean_air | 1.0 | `fire_clean_air_room` (a HEPA air cleaner or the cheaper filter box) |
| clean_air | 0.5 | `fire_clean_room_plan`, `med_n95_respirators` |
| clean_air | 0.2–0.3 | `evac_shelter_in_place_kit` (0.3); `med_cleanup_ppe` (0.2) |

The allocator (rr-budget) multiplies an item's readiness value by its share; an item with no share
keeps the whole value, as before. rr-plan's `READINESS_HARM` gains a clean-air row (0.5
day-equivalents, like security's, an estimate), since the research priced no clean-air harm.

## Seasons and test intervals

`Item.season` (contract v2) is the season to have the item by, or to check it in (meteorological
seasons; summer starts 1 June, with the Atlantic hurricane season, `ready_gov_hurricanes`). The
anchors: **spring** for stored water and bleach (with the six-month rotation, the other check falls
in the fall), the weather radio (tornado season), rain barrels, the backflow valve and sump pump,
and wildfire hardening; **summer** for fans, cooling towels, the room thermometer, the air cleaner
and filter box (wildfire season), insect repellent and the tarp kit (hurricane season); **fall** for
the generator and its upkeep, the carbon monoxide alarm (heating season), sleeping bags, blankets
and warm layers, the camp stove and propane, the car kit and the jump starter (cold weakens
batteries). The budget crate uses the anchors to buy a fan before summer when the cost order allows:
in the default schedule an item is due while its season is under way or starts within two months,
and a due item one month's money can buy moves ahead of the ordinary items of the current tier. It
never moves an item into an earlier tier, ahead of a life-safety item or capability, or into a
sinking fund (`docs/RISK_MODEL.md`, "Budget allocation").

`Item.test_interval_months` says how often to try the thing the way it would be used, because a
backup nobody has tried may not work (the Deviant Ollam lessons). The maintenance calendar prints
these as **Test** rows beside Check and Use-and-restock: switch on each light (every 6 months), start
the generator and run it with something plugged in (every 3), charge the jump starter and try it
(every 3), and open the key safe with its code and try the key in the door (every 6). The intervals
are estimates (`rr_expert_prior`); the generator's matches its existing three-month check, and the
lights' the lantern's six-month check. Only these five items carry one (a catalogue test checks it).

## The bare-minimum kit

`Dials::minimum_kit` (or a plan that would run past 36 months) asks the budget crate to schedule the
smallest set that covers **three days** (`minimum::MINIMUM_KIT_DAYS`) of water, light, warmth and
medicine continuity first. rr-supply marks each line's share of that kit in `SizedLine::minimum`
(`rr_supply::minimum_kit` lists them):

| Function | Line | Share in the kit |
| --- | --- | --- |
| Water | `water_out.water_gallons` | three days of the household's water (or the target, if shorter) |
| Safe water | `water_boil.bleach_bottles` or `water_out.bleach_bottles` | the one bottle |
| Light | `power.lights`, `power.battery_packs` | one light and one pack of batteries |
| Warmth | `thermal.blankets`, `thermal.warm_layers`, `thermal.cooling_plan` | all of them: most homes have them, and the heat plan is free |
| Medicine | `medication.medication_days`, `power.medical_device_wh` | three days (or the target) |
| Medicine | `medication.rx_cold_storage` | the cooler bag's day |
| Medicine | `power.device_battery_units` | every spare battery |
| A baby's food | `supplies.infant_formula_rtf` | the three days of ready-to-feed formula (water and food at once) |

Smoke and carbon monoxide alarms are not in it: they are life-safety lines, which the allocator
orders first anyway. The budget crate buys the kit (with the three-day life-safety items) before the
tiers in bare-minimum mode, in just the amount each line needs (one headlamp of four), and reports
the month it is complete for every plan (`Plan.minimum_done_month`).

## Storage space and weight

`rr_supply::storage_by_tier(lines)` gives, for each tier from three days to the highest tier the
stored supplies reach, the space and weight of the bulky consumables: water (a litre weighs a
kilogram), food (1.9 L and 0.8 kg per 2,000 kcal, packaging included), dry pet food (1 L a pound)
and toilet paper (1.3 L and 0.1 kg a roll), all estimates (`food_storage_l_per_2000kcal` and the
rest). A line that grows with days counts its share up to the tier's days. Gear (lights, radios, the
first-aid kit) is not counted. The plan prints these as the "Where it lives" rows (review P-20).

## The long-horizon section

`Item.long_horizon` (contract v2) puts an item in the long-horizon section, which the plan shows
when any duration target reaches 30 days (`long_horizon_min_days`) or the household turns it on
(`rr_supply::long_horizon`). The flag groups items; it does not change how they are sized or
bought: each still has its own rule (a rain barrel only for a house on town water with a no-water
target over 14 days, carriers past the 14 stored days), so a 20-day target can still buy carriers.
Flagged: the rain barrel, water carriers, the hauling tote for animals, the well hand pump (an
option), household kits, the children's activity kit (morale), bulk staples, and the free pointer
`food_going_further` (`once_if_long_horizon`). Canning, gardens and wells are pointers in the
content workstream's "If it lasts for months" block, not a manual.

## Decisions

`Item.decision` (contract v2) marks a decision rather than a purchase: an insurance policy, ID, a
home repair to weigh. Decisions are free items in tier now with a $0 band, so they never draw on the
supplies budget; their notes say the cost is a quote, a premium or a fee (the passport fees quoted
from 22 CFR 22.1). The insurance decisions meet the insurance lines (`insurance_home_or_renters`,
`insurance_flood`, `insurance_earthquake`, `insurance_wind_deductible`, `insurance_sewer_backup`,
`insurance_condo_unit`, `insurance_life_disability`); the earthquake and wind ones also need the
hazard at 2 in 100 in ten years. The home repairs use the owner switches and a hazard: a FORTIFIED
roof (`once_if_owned_house`; hurricane, strong wind, hail, tornado), a safe room (tornado,
hurricane), a backflow valve and battery-backup sump pump (`once_if_owned_basement`; flooding, water
damage), bolting a house to its foundation (`once_if_owned_detached`; earthquake) and wildfire
hardening (wildfire), each with a grant or discount pointer. Water-heater straps stay a purchase
(about $20, life-safety in earthquake country).

## Round 2 rule changes, as implemented

The practitioner's quantity-rule table (practitioner-review.md, "Quantity-rule changes"), with where
each change lives and the test that reads its source (`crates/rr-supply` unless noted).

| Change | As implemented | Test |
| --- | --- | --- |
| `rx_cold_storage` coverage | the bag counts 1 day; the rest needs power (v0.1.1), plus a 12-volt fridge in hot counties (`rx_fridge_units`) | `sugar_land_keeps_insulin_cold_and_feeds_the_baby_without_water` |
| `power_station_units` | a need with refrigerated medicine, a power target of 2 days and no backup power (v0.1.1) | `requirements.rs` `the_pending_cold_chain_and_well_fixture_exercises_the_round_two_rules`, `sugar_land_...` |
| `generator_units` (well) | pump-sized with 240 V, plus `generator_connection_units` (v0.1.1) | `requirements.rs` `livestock_on_a_well_store_two_weeks_until_pump_power_exists` |
| `livestock_water` | 3 days only with pump power, 14 until then (v0.1.1); a hauling tote in a drought | `livestock_on_a_well_store_two_weeks_until_pump_power_exists`, `hays_hauls_water_in_a_drought` |
| `water_treatment_capacity` coverage | the filter counts only with a raw-water source (well, a named source, or a rain barrel the plan makes a need) | `rain_barrels_are_the_source_the_filter_needs` |
| new `rain_catchment_units` | barrels from the state's driest-months rain (NOAA nClimDiv) where NCSL's map lets households drink rainwater; optional otherwise | `rain_barrels_are_the_source_the_filter_needs` |
| `battery_packs` | ceil(min(days, 14) ÷ 7), then `recharge_capability` | `coos_bay_stores_two_weeks_of_batteries_and_paper_for_the_store_target`, `a_long_power_target_asks_for_a_way_to_recharge` |
| `toilet_paper_rolls` | the supplies target only | `coos_bay_...` |
| `generator_fuel_gallons` coverage | run-hours at light and full load; the line's days are the days the fuel lasts | `coos_bay_...` |
| `medication_days` coverage | at most 30 days; `medication_fills` covers the rest | `coos_bay_...` |
| readiness value | × `readiness_share` (content; rr-budget multiplies) | rr-content `every_readiness_item_carries_a_share` |
| `bleach_bottles` tier | always three days, one bottle | `rural_wound_care_and_the_long_walk_filter` |
| extinguisher need rate | CPSC's all-fires rate in the line; the item is life-safety | `fire.rs` `the_extinguisher_line_counts_the_small_fires_nobody_reports`; rr-content `the_practitioner_items_exist_with_their_rules` |
| `escape_ladder_count` | a house is assumed to sleep upstairs unless every bedroom is on the ground floor | `fire.rs` `apartments_count_one_level_and_ladders_follow_the_floor` |
| new `cooking_capability` | a need with a store target of 14 days or a formula-fed baby under a boil notice; covered by a wood stove or gas range | `food.rs` `a_way_to_cook_without_power`, `sugar_land_...` |
| `cooking_fuel_canisters` | + 0.3 lb per 2,000 kcal of staples; `propane_cylinders` where winters freeze | `food.rs` `camp_stove_canisters`, `a_way_to_cook_without_power` |
| `battery_fan`, `cooling_towel` in hot counties | optional; the cooling plan is the cover and is life-safety with a power target | `hot_counties_count_a_place_to_go_not_a_fan` |
| bleeding-control tier | three days; life-safety for rural homes or a likely medical emergency | `requirements.rs` `life_safety_lines_are_marked`; the add-on: `first_aid.rs` `the_wound_add_on_is_for_homes_far_from_help` |
| `requires` (new item field) | content, with any-one-of semantics (above) | rr-content `accessories_require_their_device` |
| `water_reused_bottles` | min(target days, 14) of water, at most 6 gallons | `chicago_meets_its_water_target_with_free_bottles` |
| motion light, utility wrench | `once_if_house_ground_floor`, `once_if_house` | rr-content `the_practitioner_items_exist_with_their_rules` |
| thermal plans | `warm_room_plan` and `cooling_plan` | rr-content, the same |
| new `get_home_filter` | only where the walk's water passes 2.5 L outside hot counties | `rural_wound_care_and_the_long_walk_filter` |
| `toilet_bags` | the first 30 days unless an earthquake drives the no-water target | `coos_bay_...` (earthquake: all 365) |
| `infant_formula_oz` | three days of ready-to-feed (`infant_formula_rtf`), then powder | `sugar_land_...` |
| `cash_reserve_usd` | max($100, half of 3 days of monthly expenses rounded down to $20) | `comms.rs` `philadelphia_comms` |
| `n95_masks` | teens and adults, by the county's smoke days (5 when unknown, at most 30) | `clean_air_lines_follow_the_smoke_days` |
| bleach rotation | 6 months, rule and item | rr-content `water_items_match_their_sources` |

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
