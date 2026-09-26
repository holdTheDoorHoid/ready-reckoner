# Risk model (as implemented)

The derived, maintained description of the model the engine runs. `docs/DESIGN.md` §4 is the
design and `docs/research/risk-model.md` the research behind it. Two workstreams write this file:
`## Hazard rates` (`crates/rr-hazards`) and `## Consequences and targets` (`crates/rr-consequence`).
Every expert estimate listed here is shown in the app as an estimate and cited as
`rr_risk_model_priors`.

Evidence tags: **DATA** (a published figure), **DERIVED** (our calculation from data, method
stated), **PRIOR** (expert judgement, with a range).

## Hazard rates

Owned by `crates/rr-hazards`. Entry point: `rr_hazards::assess(input, county, base_rates,
location) -> HazardAssessment { profiles, rates, scenarios, parts, also_checked, notes }`.
`profiles` holds the ranked hazards, then the nine rare families (contract v2); `rates` holds the
ranked hazards only, so no rare family ever reaches `rr-consequence`, a target or the budget.
`also_checked` lists everything checked and found under 1 in 100,000 a year here, with its rate.
`HazardAssessment::add_power_curve(per_year, years)` finishes the "power out for months" family
once `rr-consequence` has the household's power curve (the plan pipeline calls it).

### What a rate means

For every hazard `h` the engine computes a **household event rate** (DESIGN §4.2, research §3.1):

    r_h = λ_h · a_h · m_h

λ_h is the county frequency, a_h the footprint (the chance one county event reaches this
household) and m_h the household modifier. The result counts **household-significant events**:
events that reach this household hard enough to trigger at least one consequence bucket, of any
length. `rr-consequence` multiplies r_h by the chance of each bucket given such an event
(`p_given_event`) and by a duration curve. What "one event" means for each hazard is in the
tables below and in the hazard card's sentence ("Of 100 households like yours, about N will
*lose power or have damage in a windstorm* in the next ten years").

Three rules keep the two crates from counting anything twice:

1. **Heat and cold waves count every episode in the county.** They reach every household; what
   they do depends on the home's cooling and heating, which `rr-consequence` applies
   (`requires = no_cooling / no_heating`, `heat_share` / `cold_share` on outage rows).
2. **A named scenario is an extra event class.** `rates` carries each hazard's full rate;
   `rr-consequence` adds the scenario's own rate and durations when it is on, and drops the
   parent's overlapping class when the scenario is offered (`replaced_by`, as for major
   hurricanes).
3. **Durations are not computed here.** `county.events` median and 90th-percentile lengths and
   `county.outages` curves pass straight to `rr-consequence`.

Every rate carries a low and a high. Factors multiply with their log-spreads added in quadrature
(`ln(high/value) = √Σ ln(high_i/value_i)²`); parts that add (flood in and outside the zone,
burn plus shutoffs) add their lows and highs. A rate's evidence is PRIOR if any materially
uncertain factor is.

### Inputs read from the core pack

`rr-hazards` reads a `&CountyRecord`, `&[BaseRate]` and the resolved location. Any field may be
missing; a missing field falls back to the next source below, ending at a cited built-in value
or a PRIOR. It never guesses a field's meaning: NRI frequencies follow `afreq_kind`.

| Field | Used for |
| --- | --- |
| `nri.<hazard>.afreq`, `afreq_kind` | λ. An annual probability p becomes −ln(1 − p); for wildfire the burn probability is used directly. A value marked as a probability but at or above 1 (NRI v1.20's landslide field: Coos County 12.77) is read as events a year. |
| `nri.<hazard>.expp` ÷ `population` | share of residents exposed to sub-county hazards (tsunami, coastal and inland flooding, landslide, avalanche, volcano, wildfire) |
| `nri.<hazard>.hlrb` | footprint of damage-type hazards (hail, tornado, landslide) |
| `nri.<hazard>.ealt` ÷ `households` | expected annual loss per household (`eal_per_household_usd`) and severity |
| `seismic.p_pga_ge_0_1g_per_year`, `mmi6_100yr` | earthquake rate, in preference to NRI |
| `outages.events_per_customer_year`, `years_covered` | the windstorm outage floor |
| `flood.sfha_home_share`, `claims_per_1000_policies_year` | inland-flood rate |
| `facilities.nearest_nuclear_km`, `tri_facilities` | nuclear plant hazard; chemical releases |
| `tsunami_zone`, `state_abbr`, `fips` | scenario detection; western-state shutoffs; hurricane states |
| `climate.*` | the 2050 dial (keys below) |
| `events.*` | episode rates (keys below) |

**v2 exposure columns** (`CountyRecord::exposure`, data-hazard's `CountyExposure`; the ZIP-level
dam count from `LocationResolved::exposure`). Absent means unknown: the row falls back as listed,
and one note names the missing columns.

| Column | Used for | Without it |
| --- | --- | --- |
| `strategic_class`, `strategic_places`, `strategic_km`, `strategic_bearing` | the nuclear family's class and "Why here"; the war row's near/far split | the population-weighted f_S (0.314, range 0.01–0.99); war at ×0.71 (0.3–1) |
| `uasi_share`, `uasi_area` | the metro weight w_m of the attack, CBRN and nuclear-terrorism terms: the FEMA urban area's own share, looked up by `uasi_area` in the FY2026 table (`params::UASI_FY2026`) | a range from a small town to a 5 % metro |
| `geomag_factor`, `geomag_lat` | the solar-storm location multiplier α ÷ 0.2285 | the national average (×1) |
| `smoke_days_35`, `smoke_basis` | wildfire smoke (imputed counties get a ×/÷ 2 spread, monitored ×/÷ 1.3) | wildfire smoke left out |
| `karst_share` | sinkholes | sinkholes left out |
| `leveed_pop_share`, `levee_risk_high_share` | the levee part of dam or levee failure | no levee part |
| `dams_high_total` (else `facilities.high_hazard_dams`), `dams_high_poor_condition` | the county part of dam failure; the poor-condition weight | ×1 for condition |
| `dams_high_within_10km` (ZIP, `LocationResolved::exposure`) | the downstream part of dam failure | the county part |
| `eviction_filing_rate` | eviction (only once the owner approves Eviction Lab's ODC-BY licence) | the national judgment rate |

**`events` keys** (episodes a year; the pack's Storm Events, SPC and HURDAT2 types, then hazard
ids as aliases): heat wave `heat`; cold wave `extreme_cold`; winter weather `winter_storm`; ice
storm `ice_storm`; windstorm `high_wind` + `severe_wind_day` (added); major-hurricane share
`major_hurricane_passage` ÷ `hurricane_passage`; boil-water notices `boil_water_notice`
(household rate, if a source is ever added); dust storms `dust_storm` (v2; a county with no row
recorded none); flash floods `flash_flood` and tropical cyclones `tropical_cyclone_impact` (their
sub-cause notes only). Hail, tornado and landslide stay on NRI's frequency, because their
footprint is NRI's loss ratio per NRI event.

**`climate` keys**, in order of preference: a finished multiplier `<hazard_id>` (and
`<hazard_id>_high`); county counts `<variable>_hist`, `_2050`, `_2050_high` for
`days_over_95f`, `days_over_2in`, `icing_days`, `dry_spell_days` (these allow the research's
additive rule for hot days and show the emissions range), plus `days_over_90f_hist` for the heat
cap; then the pack's ratio variables (future ÷ present at about +2 °C, optional `<key>_high`):
`hot_days_95f`, `hot_days_90f_mid45`, `extreme_rain_days`, `heavy_rain_days_1in_mid45`,
`very_cold_nights_0f`, `freezing_nights`, `consecutive_dry_days_mid45`, `dry_days_mid45`.

**Base-rate ids** (value, low, high and source id are passed through): `house_fire_per_household_year`,
`unemployment_spell_per_worker_year`, `layoff_per_worker_month`, `ed_visits_per_person_year` (or
`ed_visits_per_100_persons_year`), `unintentional_injury_death_per_person_year` (or
`accidental_death_per_person_year`), `pandemic_onset_per_year`, and optional overrides
`grid_failure_per_year`, `cyber_outage_per_year`, `civil_unrest_per_year`,
`supply_chain_disruption_per_year`, `hazmat_release_per_year`. v2: `arrests_per_100k_{male,female}_<band>`
(bands `10_17`, `18_24`, `25_34`, `35_44`, `45_54`, `55_64`, `65_plus`: the data-model series
`fbi_arrests` passed as base rates) replace the built-in FBI table.

### Natural hazards

| Hazard | One household event | λ, county frequency | a, footprint | m, household | Evidence |
| --- | --- | --- | --- | --- | --- |
| Heat wave | a heat wave in the county | Storm Events `heat` episodes; else NRI event-days ÷ 3 days per episode (2–5), capped at the county's historical days over 90 °F (floor 0.2) | 1 | none | DATA + PRIOR |
| Cold wave | a cold wave in the county | `extreme_cold` episodes; else NRI event-days ÷ 2 (1–4) | 1 | none | DATA + PRIOR |
| Winter weather | a winter storm keeps the household home or cuts its power | `winter_storm` episodes; else NRI event-days ÷ 1.5 (1–3) | 0.05 (0.02–0.15) | none | PRIOR |
| Strong wind | a windstorm cuts the power or damages the home | `high_wind` + `severe_wind_day`; else NRI | 0.06 (0.02–0.2), raised by the outage floor | power-line exposure: urban ×0.5 (0.3–0.8), suburban ×1, rural ×2 (1.3–3) | PRIOR; DERIVED where the floor applies |
| Ice storm | cuts the power or keeps the household home | `ice_storm` episodes; else NRI | 0.1 (0.03–0.3) | power-line exposure | PRIOR |
| Lightning | damages the home or cuts its power | NRI (days with strikes) | 1 in 10,000 (3 in 100,000 – 3 in 10,000) | power-line exposure | PRIOR |
| Hail | damages the home or car | NRI | NRI loss ratio ÷ 0.02 mean damage (0.01–0.05); else 0.005 | none | DERIVED + PRIOR |
| Tornado | damages or cuts off the neighborhood | NRI | loss ratio ÷ 0.25 (0.1–0.5) × 5 homes disrupted per home damaged (2–10), at most 1; else 0.005 | none | DERIVED + PRIOR |
| Hurricane | cuts the power or damages the home | NRI (its frequency counts about as many events as HURDAT2's tropical-storm-strength passages) | Category 1–2 and tropical storms: 0.5 (0.3–0.8); major: 0.8 (0.5–1.0); major share = (HURDAT2 major passages + 5 × 5.4 %) ÷ (tropical-storm passages + 5), else 5.4 % (3–10 %) | none | PRIOR (DERIVED share where the pack has it) |
| Earthquake | shaking strong enough to knock things off shelves (0.1 g, about intensity VI) | USGS yearly chance → −ln(1 − p); else the 100-year intensity-VI chance; else NRI; ×/÷ 2 | 1 | none | DATA (hazard model) |
| Tsunami | a tsunami warning to leave the inundation zone | NRI | residents in the zone (NRI) × 0.3 of events bring a warning (0.1–0.6) | none | DERIVED + PRIOR |
| Inland flooding | flood water reaches the home, or cuts off an upper-floor flat | flood-zone odds (below) | s·p_in + (1 − s)·p_out | basement ×1.5 (1.2–2); second floor or higher ×0.5 (0.3–0.8) | DERIVED + PRIOR |
| Coastal flooding | coastal flood water reaches the home | present when NRI has it | residents in the coastal flood zone (NRI) × 1 %/yr (0.5–3 %) | as inland flooding | DERIVED + DATA |
| Landslide | cuts off the road or damages the home (the card gives both) | NRI | home damage: exposed share × loss ratio ÷ 0.3 (0.1–0.6), bounded by NRI's expected loss and at most 1 in 100 a year (below); roads cut off: 10 homes per home damaged (3–30); no loss ratio: 0.001 of which a tenth is damage | none | DERIVED + PRIOR |
| Avalanche | reaches the home or road | NRI | exposed share | none | DERIVED |
| Volcanic activity | ash or mudflows reach the household | NRI | exposed share × 0.5 (0.2–1) | none | DERIVED + PRIOR |
| Wildfire | must leave home, or loses power in a wildfire safety shutoff (two parts, kept apart: below) | NRI burn probability ×/÷ 1.5 | warnings to leave: exposed share × 20 households warned per home that burns (5–50); plus shutoffs 0.02/yr (0.005–0.1) in AZ, CA, CO, ID, MT, NM, NV, OR, UT, WA, WY | burn part urban ×0.3, rural ×2; shutoffs urban ×0.1, suburban ×0.5, rural ×1 | PRIOR |
| Drought | a private well runs low; or water limits that change daily life | NRI frequency ÷ national median 13.43, bounded ×0.25–×4 | — | well 1 %/yr (0.3–3 %, research §9); public water 0.2 %/yr (0.05–1 %) | PRIOR |
| Wildfire smoke (v2) | days of unhealthy smoke at home | the county's smoke days a year at 24-hour PM2.5 of 35.5 µg/m³ or more (NOAA HMS smoke maps with EPA AirData, 2016–2023 mean; data audit §3.2) ÷ 3 days an episode (2–5); ×/÷ 1.3, ×/÷ 2 when imputed | 1 (distant smoke reaches every home) | none; severity at least Serious for a child, someone 65+, pregnancy or a breathing device | DATA + PRIOR |
| Dust storm (v2) | caught in a dust storm on the road or at home | Storm Events "Dust Storm" episodes (`dust_storm`) ×/÷ 1.3 | 0.3 (0.1–0.6) of a zone's episodes | none; the same severity floor as smoke | DATA + PRIOR |
| Sinkhole (v2) | ground collapse damages the home | the county's share on karst (USGS OFR 2014-1156) | × 2 in 10,000 a year for a home on karst (5 in 100,000 – 1 in 1,000; Florida's reports and claims, hazard-expansion) | none | DATA + PRIOR (low confidence: not all karst has sinkholes) |

NRI and Storm Events frequencies carry a ×/÷ 1.5 and ×/÷ 1.3 spread; earthquake models ×/÷ 2.

**Inland flooding.** s is the share of homes in the Special Flood Hazard Area: the NFIP share when
the pack has it, else NRI's inland-flood exposed population ÷ population, else 0.05 (0.01–0.15).
p_in is NFIP claims per policy-year (bounded 0.3–10 %) or the flood-zone definition, at least
1 % a year (1–3 %). p_out, outside the zone, is 0.2 % a year (0.05–0.5 %) × the county's NRI
inland-flood frequency ÷ the national median 0.9643 (bounded ×0.5–×2). Philadelphia with a
basement comes to 0.6 % a year, in line with NRI's expected loss of about $270 a year on a
$300,000 home (data-sources §1.4) at a typical flood claim.

**Landslides: home damage bounded, roads cut off counted apart** (v0.1.1, model review M-05).
The home-damage part is exposed residents × NRI's historic loss ratio ÷ a 0.3 damage ratio, as
before, but it may not exceed what NRI's own expected annual loss supports: EAL ÷ (0.3 × the
county's building value), the share of the county's homes a landslide damages each year if NRI's
loss figure is right. The exposure share counts residents, and in landslide country it runs far
ahead of the buildings NRI counts as exposed (Santa Barbara: 3 in 100 residents, 1 in 1,000 of the
building value). Nor may it exceed 1 in 100 a year, the chance that defines a high-risk flood zone
(`fema_flood_zones`), the yardstick the model uses for "likely to be damaged": no hazard makes a
county-average home more likely to be damaged than a home in a mapped floodplain without a named
source. When that ceiling binds, a note says so. Roads cut off stay 10 per home damaged (3–30,
PRIOR), now per the bounded damage; `rr-hazards` passes the damage part to `rr-consequence`
(`RatePart` `damage`), where only it forces the household out or damages the home, and the card
gives both numbers: "Of 100 households like yours, about 63 will have a landslide cut off their
road or damage their home in the next ten years. About 10 of 100 will have one damage their
home." **Range:** before, 33 counties had a home-damage rate above 1 in 100 a year and NRI's
landslide records gave Utuado 2.1 events a year with 90 in 100 homes damaged in ten years; with
the expected-loss bound 6 counties are still above 1 in 100 (Maricao, Utuado, Las Marías, Jayuya,
Adjuntas and Orocovis, all in Puerto Rico's central mountains) and are held at it, 13 counties are
above 5 in 100 over ten years, and Santa Barbara comes to 2 in 100 damaged (18 cut off or
damaged), Buncombe (Asheville) to about 1 in 1,300. `no_county_lets_landslides_damage_more_homes_than_the_ceiling`
(`crates/rr-plan/tests/round2.rs`) checks every county in the pack.

**Wildfire: warnings to leave and shutoffs are separate event classes** (v0.1.1, model review
M-06). The two parts in the table above used to be added into one rate that `rr-consequence`
re-split 15 % warnings / 85 % shutoffs, which gave Butte County 0.0047 evacuations a year against
the 0.021 warnings the burn part counts (4.5 times too few) and put 85 % of Maui's warnings into
power shutoffs Hawaii does not have. `rr-hazards` now passes each part (`HazardAssessment::parts`,
`RatePart` `burn` and `shutoff`; they add up to the wildfire rate), and every warning is an
evacuation, every shutoff a power cut. The register card still shows the whole rate.

**The outage floor** (research §2.5, §7.5). When the county has EAGLE-I outage records, recorded
outages × 0.7 (0.6–0.9) caused by weather (informed by Do et al. 2023: 62.1 % of long county
outages coincided with extreme weather) must be explained by the storm hazards, each counted with
its chance of cutting power (windstorm 0.9, winter storm 0.3, ice storm 0.8, hurricane 0.9,
tornado 0.8, lightning 0.8, hail 0.1). Any shortfall ÷ 0.9 is added to windstorms, the most
common cause. NRI records only 0.027 windstorms a year for Coos County, but a Coos home is caught
in a county outage 1.09 times a year; the floor lifts Coos windstorms to 0.75 a year.

Any ranked hazard under 1 in 100,000 a year today and around 2050 (natural, societal or personal)
is left out of the register and listed in `also_checked` with its rate, unless a scenario hangs on
it (see "Sub-causes and Also checked").

### Societal hazards

| Hazard | One household event | Rate | Modifier | Evidence |
| --- | --- | --- | --- | --- |
| Pandemic | a pandemic that changes daily life (as in 1918 and 2020) | `pandemic_onset_per_year` (5 onsets in 108 years, about 4.5 %) × 0.25 of onsets that disrupt daily life (0.1–0.4); else 1 %/yr (0.5–2 %, research §6.2) | none | DATA × PRIOR |
| Regional blackout | multi-day, not caused by weather | 0.5 %/yr (0.15–1.5 %) | none | PRIOR, research §6.2 |
| Cyberattack on services | pharmacy, insurer, payment or utility systems down | 2 %/yr (0.7–5 %) | none | PRIOR, research §6.2 |
| Civil unrest | a curfew | 3 %/yr for a city household (1–6 %) | suburban ×0.5 (0.3–0.8), rural ×0.2 (0.1–0.4) | PRIOR, research §6.2 |
| Supply chain disruption | store shelves empty of what the household needs | 20 %/yr (10–40 %) | none | PRIOR, research §6.2 |
| Chemical spill or release | a do-not-drink order or an order to stay inside | 2 %/yr (0.7–5 %) × TRI factor 1 + 0.5·log10((n + 1)/5), bounded ×0.65–×2 | TRI facilities in the county | PRIOR |
| Nuclear plant accident | an order to shelter or leave | plant within 16 km: 2 in 10,000 a year (0.2 to 5 in 10,000); 16–80 km: 5 in 100,000 (1 to 20 in 100,000); beyond 80 km: not listed | distance | PRIOR (one US accident needing off-site action, Three Mile Island, in several thousand reactor-years) |
| Dam or levee failure (v2) | told to leave because a dam or levee fails or threatens to | high-hazard dams whose listed downstream town is in the ZIP code × 1 in 10,000 a year per dam (3 in 100,000 – 5 in 10,000; ASDSO: about 2 in 10,000 failures per dam a year, all classes, and incidents such as Oroville 2017) × 0.3 of the ZIP's households (0.1–0.6); without a ZIP, the county's high-hazard dams × 0.01 of its households (0.003–0.03); dams in Poor or Unsatisfactory condition count ×3; plus the share of the county behind levees × 0.2 % a year (0.05–1 %; leveed land is mapped outside the flood zone, so the flood rate misses it), ×2 behind levees USACE rates High or Very High | footprints as stated | PRIOR stacked on the inventories: range only |
| Phone or internet outage (v2) | phones and internet down for hours, 911 included | 0.3 a year (0.1–1): carrier-wide outages of several hours about once a year (FCC, AT&T 22 February 2024: 92 million calls, 25,000 calls to 911 blocked) × about a third of households on the failing carrier | none | PRIOR |
| Medicine shortage (v2) | a daily medicine cannot be filled for days to weeks | 5 % a year per person on a daily prescription (2–15 %; ASHP: 323 active shortages at the start of 2024; openFDA: 70 medicines short on 2026-09-26) | ×1.5 (1.2–2) for a medicine that must stay cold (50 of openFDA's 70 are injectables); left out with no daily prescription | PRIOR |
| Pay or benefits stop (v2) | federal pay or a benefit stops for weeks | federal pay: funding gaps of 14 days or more, 4 in fiscal years 1982–2026 (CRS RS20348): 0.089 a year (exact Poisson 90 %: 0.030–0.203); SNAP or WIC: × 0.25 (0.1–0.6) of those gaps (one of four, November 2025); SSI, SSDI and VA: 0.5 % a year (0.1–2 %), since they were paid through every shutdown; unemployment 1 % (0.2–4 %) | only for households with `finances.benefits`; the largest of their benefits' rates (one lapse stops them all) | DATA (federal pay) + PRIOR |
| Attack or threat closes your area (v2; replaces the disruption half of terrorism) | a shelter order, closure or transit shutdown for half a day or more | 0.1 metro-wide attacks or threats a year (0.04–0.25; CSIS 1994–2025: Oklahoma City, 11 September, the anthrax letters, Boston) × the metro area's share of FEMA's FY2026 UASI money (New York-White Plains 24.39 %, Chicago 5.30 %, Philadelphia 2.84 %) × 0.3 of its households (0.1–0.8); outside every funded area 3 in a million a year (1 in a million – 3 in 100,000: 5 % of attacks over the 64 million households there, 40,000 households an order) | the metro weight | PRIOR: range only |

The old nuclear-attack row (a worldwide catastrophe forecast shown to every household, H-01) and
the terrorism row (retired in contract v2, H-02) are replaced by the rare families below.

### Personal hazards

| Hazard | One household event | Base rate | Household modifier | Evidence |
| --- | --- | --- | --- | --- |
| Job loss | a spell of unemployment for any earner | 0.083 per earner-year (BLS: 8.3 % of labour-force participants unemployed at some point in 2024); range 0.06–0.135, the top being the JOLTS 1.117 %/month as a yearly rate, −12·ln(1 − 0.01117) | × earners; stability: very_stable ×0.5 (0.3–0.7; tenured, public sector, pension), stable ×1 (typical salaried job), variable ×1.5 (1–2), seasonal ×1.75 (1.5–2), gig ×1.75 (1.5–2) | DATA + PRIOR (research §2.7, §3.5) |
| House fire | a reported fire in the home (or next door) | 344,600 fires ÷ 131,434,000 households = 0.262 % a year (0.2–0.35 %) | rowhouse or low-rise apartment ×2 (1.5–3; research §2.7), high-rise ×1.5 (1–2) | DERIVED + PRIOR |
| Medical emergency | an emergency department visit | 0.473 per person-year (NHAMCS 2022; 0.35–0.65) | × people; rural addresses get a note on slower ambulances (Mell 2017) | DATA |
| Stranded in a vehicle | a crash or breakdown away from home | 0.15 per vehicle-year (0.05–0.4); no vehicle: 0.03 per non-car commuter (0.01–0.1) | × vehicles | PRIOR (crashes: NHTSA 6.14 million a year) |
| Local water or gas outage | public water: a boil-water notice or a main break; well: a gas leak or local fault | boil notice 5 %/yr (2–10 %) + main break 10 %/yr (5–20 %); well 1 %/yr (0.3–3 %) | water source (the well pump's own failures are an `rr-consequence` coupling) | PRIOR (research §6.1, §8) |
| Break-in | a household burglary | 1 %/yr (0.5–2 %), to be replaced by the BJS victimization survey figure | none | PRIOR |
| Death or disability of an earner | loss of an earner's income | 0.9 % per earner-year (0.5–1.5 %): disability about 0.6 % (SSA: 1 in 4 20-year-olds disabled before full retirement age) plus working-age death about 0.3 % | × earners | DERIVED + PRIOR |
| Long illness in the household | someone sick at home for weeks | 1 % per person-year (0.5–3 %) | × people | PRIOR |
| Burst pipe or water leak (v2) | a burst, frozen or leaking pipe or appliance floods part of the home | 1.5 % per home-year (1–2 %; III/ISO water damage and freezing, about 1 in 67 insured homes a year, 2019–2023) | ×1.3 (1.1–1.6) where 5 or more days a year stay below freezing (CMRA `icing_days_hist`); basement ×1.2 (1–1.5); renters ×0.8 (0.6–1); no housing-age column exists yet, so age is not a modifier | DATA + PRIOR (confidence medium: read through the publisher's summary) |
| Eviction (v2) | a renting household is taken to court and ordered to leave | the county's eviction filings per renter household × 0.4 (0.3–0.55) that end in a judgment, when the pack has the column; otherwise 2.3 per 100 renter households a year (1–5; Eviction Lab 2016, confirm) | renters only; income stability as for job loss; ×0.5 (0.3–0.8) with three months of savings | PRIOR |
| Arrest or detention (v2, owner decision 2026-09-26) | a household member is arrested | FBI arrests per 100,000 a year by age band, 2023–2025 (Crime Data Explorer, Tables 29, 39 and 40; the data-model series `fbi_arrests`): the men's and women's rates averaged, since the form does not ask sex. Adults 18–64 are the five FBI bands weighted by the years each covers (7, 10, 10, 10, 10): 3.31 per 100 a year; teens the 10–17 band, 1.47; 65 and over 0.31; children under 13 are not counted | summed over the household (events, not people); the range runs from the women's lowest year to the men's highest | DATA (confidence medium) |

Job loss and earner loss are left out (with a note) when no one is marked as earning; vehicle
stranding when there is no vehicle and no commute; medicine shortages when no one takes a daily
prescription; eviction for owners; pay or benefits stopping without `finances.benefits`.

The arrest sentence counts events: "For households with people the ages of yours, the FBI's
counts come to about 7 arrests for every 100 households a year (2023–2025). This counts arrests,
not guilt or convictions, and one person arrested twice counts twice." Its buckets are income and
home loss (the legal-readiness checklist); `rr-consequence` owns the effects.

### The rare families

The nine rare families (REVIEW §2.3–§2.4, hazard-expansion Deliverable B) are shown in their own
box, range only, sorted by the middle of their range (never shown), never by expected loss, and
never handed to `rr-consequence`. Their factors are published forecasts and expert judgement
stacked together, so their ranges multiply low by low and high by high (`Estimate::times_span`,
the review's own arithmetic), not in quadrature. Each has `family` (its own id), `sub_causes`,
`location_factor` where there is a location term, `range_only`, `if_it_reaches_you`,
`what_it_changes`, and an `anchor_sentence`: the household's ranked hazard with the smallest rate
above the row's upper bound ("Less likely than a regional blackout (about 5 in 100 for you in the
next ten years)."). The sentence gives the range as natural frequencies over the horizon ("Between
1 in 3,300 and 1 in 28 households like yours would …"); a range wider than a thousandfold is said
in words ("At most about 1 in 58 …").

**Nuclear attack.** Serious local effects (blast or dangerous fallout):
r = λ_S·f_S(class) + λ_L·w_L·0.3 + λ_I·s_UASI·0.1, with λ_S = 4 in 10,000 a year (1 in 10,000 – 4
in 1,000: FRI 2024's catastrophe forecasts × 0.33 reaching US soil; XPT 2023; Rethink Priorities
2019; Barrett 2013), λ_L = 1 in 10,000 (2 in 100,000 – 5 in 10,000), λ_I = 5 in 100,000 (1 in a
million – 5 in 10,000), w_L = 0.05 for class A counties and Hawaii and Guam, and s_UASI the metro
weight. National disruption (λ_S + 0.5·λ_L), use abroad (λ_U = 5 in 1,000, 1–15 in 1,000) and the
EMP of a high-altitude burst (λ_S × 0.5 + λ_L × 0.3; lower 48 states) are sub-causes that never
enter the local rate. The class is the county's strategic-exposure class from
`data/core/strategic_sites.toml` (research `strategic-sites.md`):

| Class | Rule | f_S | Serious local effects a year | Ten years | Severity; if it reaches you |
| --- | --- | --- | --- | --- | --- |
| A | counterforce and command sites: missile fields, submarine and bomber bases, weapons storage, command and missile-defence sites | 0.9 (0.6–0.99) | 3.6 in 10,000 (6 in 100,000 – 4 in 1,000) | between 1 in 1,700 and 1 in 26 | 1.0; life-threatening |
| B | downwind of the missile fields (bearing 45–135°, 800 km, or within 75 km) | 0.5 (0.2–0.8) | 2.0 in 10,000 (2 in 100,000 – 3 in 1,000) | between 1 in 5,000 and 1 in 32 | 0.5; serious disruption |
| C1 | the ten largest metros, the National Capital Region, NNSA sites | 0.6 (0.3–0.9) | 2.4 in 10,000 (3 in 100,000 – 4 in 1,000) | between 1 in 3,300 and 1 in 28 | 1.0; life-threatening |
| C2 | other metros of a million or more, big ports, large refineries, other major bases | 0.3 (0.1–0.6) | 1.2 in 10,000 (1 in 100,000 – 2 in 1,000) | between 1 in 10,000 and 1 in 42 | 0.5; serious disruption |
| D | downwind (45–135°, 150 km) of an A site, an NNSA site or a C1 county | 0.15 (0.05–0.4) | 6 in 100,000 (5 in a million – 2 in 1,000) | between 1 in 20,000 and 1 in 63 | 0.5; serious disruption |
| E | everything else | 0.03 (0.01–0.1) | 1.2 in 100,000 (1 in a million – 4 in 10,000) | between 1 in 100,000 and 1 in 250 | 0.3; shortages, power cuts and lost income |
| unknown | the pack has no class | 0.314 (0.01–0.99), the population-weighted mean over the first cut | 1.3 in 10,000 | — | 0.5 |

`location_factor.label` is the research's "Why here" template for the class (research
`strategic-sites.md` §7), filled from the pack's resolved places, the distance in miles (to the
nearest 10) and the 16-point bearing: "You live downwind of the nuclear missile fields in
Wyoming, Nebraska and Colorado, about 240 miles to your northwest." A placeholder the data cannot
fill falls back to plainer words, never a guess. What it changes: one free step, pick a shelter
spot at home and at work, in classes A–D (FEMA's 72-hour guidance); nothing beyond the basics in
class E.

**Severe solar storm.** A Carrington-class storm, 3 in 1,000 a year (5 in 10,000 – 1.3 in 100:
Riley 2012, Love, Riley and Love 2017, Moriña 2019, Lloyd's 2013) × the chance it cuts a
household's power for days, 0.09 (0.06–0.12: Lloyd's 20–40 million of about 330 million people) ×
α ÷ 0.2285, capped at one half. α is NERC TPL-007's factor by geomagnetic latitude (IGRF-14, 0.1 to
1); 0.2285 is its population-weighted mean over every county (DERIVED from the pack's `geomag.csv`
and NRI population). Minot (α 0.63): 7.4 in 10,000 a year; Philadelphia (0.29): 3.4 in 10,000;
Miami (NERC's floor, 0.1): 1.2 in 10,000 (the review's 6 in 100,000 used α 0.06, below the floor).
Ground conductivity (β) is not in the pack, and the "Why here" sentence says so. The asteroid
moves to Also checked (3 in a billion a year).

**Power out for months (any cause).** The solar-storm row × 0.1 of those outages lasting two
months or more (0.02–0.3; Lloyd's worst case 16 days to a year or two), the EMP sub-cause × 0.1
(0.02–0.3; EPRI 2019 found months-long nationwide blackouts unsupported; lower 48 states only), the
war row × 0.02 (0.005–0.1), plus the household's own power curve at 60 days once
`add_power_curve` runs (awaiting: plan and consequence). Confidence medium.

**War with attacks on US infrastructure.** A great-power war, 0.5 % a year (0.2–1 %; FRI 2024's
Russia–US conflict forecasts) × 0.5 (0.2–0.8) that attacks reach US power, water or phones near
military sites, big cities, ports and refineries (classes A, C1, C2); ×0.3 far from them (B, D,
E): 2.5 in 1,000 (4 in 10,000 – 8 in 1,000) near, 7.5 in 10,000 far.

**Chemical, biological or radiological attack.** 0.03 disruptive attacks a year (0.01–0.1; START
POICN: 517 CBRN events worldwide 1990–2017, about 76 % chemical) × the metro weight × 0.05
(0.01–0.2) of its households under an order (buildings and blocks, as with the anthrax letters);
outside the funded areas 1.5 in 10 million (1 in 100 million – 2 in a million). Sub-causes:
chemical, biological, radiological.

**The worldwide families.** Severe pandemic, 1.5 in 1,000 a year (5 in 10,000 – 5 in 1,000;
Marani 2021); very large eruption anywhere, 1.8 in 1,000 (8 in 10,000 – 4 in 1,000; Cassidy and
Mani 2022), with Yellowstone (1 in 730,000 a year, USGS) as a sub-cause and in Also checked;
financial crisis with bank closures, 2 in 1,000 (5 in 10,000 – 1 in 100; one national bank holiday
in about a century), with a single bank failing (about 23 a year 2001–2025, FDIC) as a sub-cause;
mass shooting or bombing, 3 in 10 million per person a year (1–10 in 10 million; FBI 2024: 23
killed and 83 wounded), × the people in the household, location not modelled, free actions only.
Confidence is `prior` except the eruption, mass violence and the months-long blackout (`medium`).

### Sub-causes and Also checked

The hazard-candidates CSV names 52 sub-causes (its rows "c"). 51 are notes on ranked cards
(`crates/rr-hazards/src/subcauses.rs`); the 52nd, a single bank failing, is on the financial-crisis
family. A sub-cause has the same consequences as its parent, so it is named, not rated again: the
parent already counts it. Where the pack or the CSV gives a number, the note carries its own range:
flash floods (the county's Storm Events episodes × 0.3–5 %, else 0.1–5 % a year), basement flooding
(0.05–0.5 %), carbon monoxide beside house fires (7–30 in 100,000 households a year, CDC; shown, not
added), inland tropical flooding (the county's tropical-cyclone passages × 0.3–0.8). Three are
placed elsewhere and say so: levee failure is on the dam and levee card, the heat-and-blackout
compound is a named scenario in desert counties, and the new earthquake scenarios are with the
scenarios. None re-rates its parent in this release: the pack has no column yet that measures
rail, pipeline or plant releases, gas curtailment or dam releases.

`also_checked` and its note list every ranked hazard under 1 in 100,000 a year here and the rare
sub-rows too small to show (the asteroid, Yellowstone), each with its rate in words ("dust storms
(none recorded here)", "a Yellowstone super-eruption (about 1 in 730,000 a year)").

### Climate: "around 2050"

Applied only when `dials.climate = y2050`. Frequencies change, never restoration times; no
multipliers are stacked (research §5.1, §5.4). The register keeps today's order; the card's
`climate_multiplier` drives the arrows.

| Hazard | Multiplier | Bounds | Evidence |
| --- | --- | --- | --- |
| Heat wave | (event-days + change in days over 95 °F) ÷ event-days (research §5.3); from the pack's ratios, the `hot_days_95f` ratio | ×1 to ×3 | DATA (CMRA, NCA5 Atlas) |
| Inland flooding | ratio of days with more than 2 in. of rain (or the pack's extreme-rain ratio); homes at ground level only | ×0.5 to ×3 | DATA |
| Cold wave, winter weather | ratio of days that stay below freezing (or the pack's very-cold and freezing-night ratios) | ×0.5 ("fewer, not none") to ×1.5 | DATA |
| Drought, wildfire | ratio of the longest dry spell | ×1 to ×2 | DATA |
| Hurricane | the share of major storms × 1.2 (1.1–1.3); how often hurricanes come does not change | — | PRIOR (NCA5) |
| Coastal flooding | only a pack-supplied multiplier (sea-level rise needs its own projection) | ×0.5 to ×3 | DATA |
| Tornado, hail, ice storm, windstorm, lightning, landslide, avalanche | unchanged: "unclear" (research §5.4) | ×1 | — |
| Earthquake, tsunami, volcano, all societal and personal hazards | never changed (research §5.4) | ×1 | — |

A baseline of fewer than half a day a year (Miami's freezing days) is too small for a ratio and
is left as today, with a note. Philadelphia around 2050: heat waves ×2.42 (central) to ×3.00
(high); heavy-rain flooding ×1.37 to ×1.49; cold spells and winter storms ×0.5.

### Named scenarios

A scenario is a rare, severe version of a hazard whose rate sits near the dial (DESIGN §4.4, the
cliff rule). `ScenarioCandidate` uses `rr-consequence`'s field names (`id`, `name`, `hazard`,
`rate_per_year`, `low`, `high`, `on`, `applies_because`, `variant`, `sources`) plus `evidence`,
`default_on`, `overridden` and `alternatives`. `dials.scenario_overrides` set `on`; an override
for a scenario that does not apply is ignored with a note.

| Scenario | Where | Household rate | Alternative | Default | Variant |
| --- | --- | --- | --- | --- | --- |
| `cascadia_m9` | 39 counties: Oregon and Washington coast and inland valleys, California's north coast | southern margin (Coos, Curry, Douglas, Jackson, Josephine, Lane; Del Norte, Humboldt, Mendocino): 40 % in 50 years → 1.02 %/yr (0.41–1.28 %); northern margin: 19 full-margin ruptures in 10,000 years → 0.19 %/yr (0.13–0.30 %) | southern margin: the long-run 41 ruptures in 10,000 years, 0.41 %/yr (research §3.3) | on in Oregon (2 Weeks Ready, Oregon Resilience Plan) and Washington (Prepare in a Year); elsewhere on when the rate is at least 0.5 %/yr | `coast` or `valley` |
| `hayward_m7` | 9 Bay Area counties | 33 % chance of magnitude 6.7+ in 30 years → 1.33 %/yr (0.74–1.99 %) | — | on (above the yardstick) | — |
| `new_madrid_m7` | 29 counties in AR, IL, KY, MO, TN | 7–10 % in 50 years → 0.18 %/yr (0.15–0.21 %) | — | off (rarer than the yardstick) | — |
| `local_tsunami` | counties with a tsunami zone | the Cascadia rate (on the Cascadia coast) or 0.1 %/yr (0.03–0.3 %) elsewhere, × residents in the zone (NRI; else 10 %) | the long-run Cascadia rate × the same share | on (knowing the route costs nothing) | — |
| `major_hurricane_direct_hit` | Gulf and Atlantic states with NRI hurricane frequency ≥ 0.1 a year | the major part of the hurricane rate: frequency × major share × 0.8 | — | on | — |
| `wasatch_m7` (v2) | 9 Wasatch Front counties (Box Elder, Davis, Morgan, Salt Lake, Summit, Tooele, Utah, Wasatch, Weber) | 43 % chance of magnitude 6.75+ in 50 years (Working Group on Utah Earthquake Probabilities 2016; confirm) → 1.12 %/yr (0.71–1.69 %) | — | on (above half the yardstick) | — |
| `san_andreas_south_m78` (v2) | 7 counties of the ShakeOut area (Imperial, Kern, Los Angeles, Orange, Riverside, San Bernardino, Ventura) | 19 % chance of magnitude 6.7+ on the southern San Andreas in 30 years (UCERF3; confirm) → 0.70 %/yr (0.43–1.09 %) | — | on | — |
| `seattle_fault_m7` (v2) | King, Kitsap, Pierce and Snohomish | about 5 % chance of magnitude 6.5+ in 50 years (USGS and Washington DNR; confirm) → 0.10 %/yr (0.04–0.21 %) | — | on in Washington (Prepare in a Year), though rarer than the yardstick | — |
| `heat_blackout` (v2) | counties with 60 or more days a year over 95 °F (CMRA baseline: 44 counties, the desert Southwest, South Texas, southwest Oklahoma; Maricopa 123, Pima 89, Clark 74) | heat episodes × power cuts of a day or more a year (the county's EAGLE-I record × its share over a day, else 2 %/yr, 0.5–6 %) × 3 days ÷ 365 (Stone et al. 2023) | — | on (the most dangerous combination; the answer is a cool place to go) | — |

The earthquake card shows the county rate less the long-run shares of every earthquake scenario
that applies (never below a quarter of it) plus the scenarios' own rates: Coos Bay 0.01981 − 0.0041
+ 0.01022 = 0.0259 a year; King County takes out both Cascadia's and the Seattle fault's shares.
The heat card shows the heat waves (the blackout scenario is one of them, taken out and added
back). The hurricane card shows the full hurricane rate (Category 1–2 plus major); the tsunami card
the county rate plus the local-source scenario. `rates` always carries the parent's full rate.

### Severity, confidence and sentences

**Severity** (0 to 1) is the loss from one household-significant event on a fixed log scale:
$50 or less is 0, $500,000 or more is 1. Natural hazards: NRI expected annual loss per household
÷ the county-average household rate, never below 0.1. Personal and societal hazards use a
per-event loss (PRIOR except house fire, $11.27 billion ÷ 344,600 fires = $32,700): job loss
$12,000; emergency visit $2,000; stranding $300; local outage $100; break-in $2,500; earner loss
$500,000; long illness $5,000; pandemic $5,000; regional blackout $1,000; cyber outage $300;
curfew $300; shortages $100; chemical release $500; nuclear plant accident $20,000; v2: burst pipe
or leak $15,400 (III's average claim); smoke $300; dust storm $200; sinkhole $30,000; dam or levee
failure $40,000; phone or internet outage $100; medicine shortage $500; pay or benefits stopping
$1,500 (a month); eviction $5,000; an attack closing the area $800 (0.3, "a few days'
disruption", H-10); an arrest $5,000. The rare families have a fixed severity by zone: nuclear 1.0
in classes A and C1, 0.5 in B, C2 and D, 0.3 in E (0.5 when the class is unknown); solar storm,
war and CBRN 0.5; months-long blackout and severe pandemic 0.8; eruption and financial crisis 0.3;
mass violence 0.9. The fixed scale means a hazard's severity reads the same in every county, and
likelihood and severity stay separate columns in the rare-catastrophe box.
**Heat and cold for households at risk** (verification, 2026-09-26): NRI's expected loss already
counts deaths and injuries, valued per statistical life, but spreading it over every household
and every county episode makes heat read "Minor" (Philadelphia: about $85 an episode). A heat or
cold wave therefore shows at least 0.4, "Serious" (an emergency visit on the same scale), when the
household has someone 65 or older, a baby, someone on a powered medical device, someone pregnant
(heat), or no air conditioning (heat) or heating (cold) — the groups CDC names and the Chicago
1995 findings (`cdc_heat_health`, `cdc_winter_safety`, `semenza_1996_heat_deaths`; PRIOR). A note
says why. Smoke and dust get the same floor for a child, someone 65 or older, pregnancy or a
breathing machine (EPA: children breathe more air for their size and N95s do not fit them; asthma
and COPD are not asked, so age stands in). Severity is shown, not planned with: the targets do not
change.

**Confidence**: data within a factor of 1.6 either way is `high`, within 3 `medium`, wider `low`;
a rate that rests partly on expert judgement is `medium` within a factor of 3, otherwise `low`;
one that rests only on expert judgement is `prior`. The rare families carry a fixed confidence
(`prior`, or `medium` for the eruption, mass violence and the months-long blackout); water damage,
arrests and the dust-storm test value are fixed at `medium` because their source was read through a
summary or their sex mix is unknown.

**Sentences**: `100 · (1 − e^(−T·r))` households of 100 over `T = dials.horizon_years`, at most
two significant figures, whole numbers from 1 to 10, then "in 1,000" and "1 in N" for rarer
hazards, "nearly every household" from 99.5, with "(about N times a year)" when r ≥ 1. The range
is shown when the rate is an expert estimate. Around 2050 the horizon reads "in a ten-year
stretch around 2050". Rare catastrophes and ranked rates built from stacked expert judgement (dam
or levee failure, an attack closing the area; `range_only`) get a range-only sentence ("Between 1 in
490 and 1 in 31 households like yours would …"); arrests get an events sentence (see Personal
hazards).

**Order**: ranked hazards by today's rate, most likely first (so the 2050 dial never reorders
the list), then the nine rare families, most likely here first by the middle of their range.

**Why we think this**: `rr_hazards::why_we_think_this(hazard)` gives one plain-language reason
per hazard (the source, or for an expert estimate the reasoning and its size) for the drawer
behind each card. `HazardAssessment::notes` carries the plain caveats for this household and
county: county scale, missing outage records, capped heat days, the outage floor, hazards too
rare to list with their rates ("Also checked"), the v2 data columns missing for the county, the
2050 changes in words ("heat waves 2.4 to 3 times as often"), ignored scenario settings, the rural
ambulance note, the nuclear planning zone, and the Serious floors for heat, cold and smoke.

### The fixture registers

From `cargo run -p rr-hazards --example register -- <household> <fips>` on the hand-built fixture
counties in `crates/rr-hazards/tests/data` (NRI v1.20 and CMRA 2025 values; research §8–§9
outage rates; v2 exposure values from data-hazard's own pack rows). Top five by rate, per year
and out of 100 households over ten years:

| # | Philadelphia renters (42101) | per year | of 100 | Coos Bay well owners (41011) | per year | of 100 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Heat wave | 3.69 | nearly all | Wildfire smoke | 2.39 | nearly all |
| 2 | Medical emergency | 1.89 | nearly all | Medical emergency | 0.95 | nearly all |
| 3 | Cold wave | 0.40 | 98 | Windstorm | 0.75 | nearly all |
| 4 | Wildfire smoke | 0.37 | 98 | Stranded in a vehicle | 0.30 | 95 |
| 5 | Winter storm | 0.35 | 97 | Phone or internet outage | 0.30 | 95 |

Philadelphia further down: phone or internet outage 0.30, windstorm 0.21, store shortages 0.20,
job loss 0.166 (81 in 100, research §8.3 row 1), arrests 0.069 (about 7 for every 100 households a
year), medicine shortage 0.05, eviction 0.023, burst pipe or leak 0.019, flooding with the basement
0.0061, house fire 0.0052 (5 in 100), earthquake 0.0016, an attack closing the area 0.00085 (range
only). Its rare box: war 2.5 in 1,000, financial crisis 2 in 1,000, eruption 1.8 in 1,000, severe
pandemic 1.5 in 1,000, solar storm 3.4 in 10,000, nuclear 2.4 in 10,000 (class C1, "between 1 in
3,300 and 1 in 28"), months-long blackout 1.1 in 10,000, CBRN 4 in 100,000, mass violence 1.2 in a
million (range middles, never shown). Coos Bay: earthquake 0.026 (with `cascadia_m9` on by default
at 1.02 %/yr), tsunami 0.0092 (with `local_tsunami`), house fire 0.0026; nuclear class E, "between
1 in 100,000 and 1 in 250".

### Decisions and known gaps

- **Common weather outranks a house fire.** Heat and cold waves, winter storms, windstorms, ice
  storms and tropical storms each reach a Philadelphia household more often than a house fire
  (0.52 % a year even doubled for a rowhouse). What holds is that job loss and house fire outrank
  every dramatic natural hazard (earthquake, tornado, hail, landslide, coastal flooding).
- **Heat cap needs a column.** The pack's `climate.csv` has ratios only. Without the historical
  count of days over 90 °F (`days_over_90f_hist`, CMRA `HISTORIC_MEAN_TMAX90F`) the heat-wave cap
  does not apply, and a mild coastal county such as Coos keeps NRI's county-wide heat-wave count
  (about 0.8 a year instead of 0.08). A `<key>_high` (+3 °C) column would add the emissions
  range; without it the 2050 multipliers show the central projection only.
- **Earthquake overlap.** `rr-consequence` has `replaced_by` rows for major hurricanes but not
  for earthquakes, so with Cascadia on, the part of the county earthquake rate that is Cascadia's
  own long-run share (0.41 %/yr near Coos Bay) is planned both as a typical damaging earthquake
  and inside the scenario. The effect is small beside the scenario's own 1.02 %/yr.
- **Missing event rows and the major-hurricane share** (changed in the verification pass,
  2026-09-26). The pack writes no row for an event type a county never recorded, and HURDAT2
  covers every county, so a missing `major_hurricane_passage` row next to passage rows means none
  was recorded. The share of NRI's hurricane events that are major is taken against HURDAT2's
  *tropical-storm-strength* passages, because NRI's frequency counts about as many events as those
  (median ratio 0.81 over 1,836 counties; 3.8 against hurricane-strength passages), and it is
  shrunk toward the pooled 5.4 % (601 majors in 11,155 tropical-storm passages in the 556 counties
  the scenario can apply to) with the weight of 5 passages. The earlier rule (major ÷
  hurricane-strength passages, a missing row read as the national one-third) put a direct hit by a
  major hurricane on Hartford, Connecticut at 1 in 25 years (HURDAT2: 1 major passage in 76
  years) and on Sagadahoc County, Maine at 1 in 36 (none recorded). Now the scenario rate is about
  HURDAT2's own major-passage rate × the 0.8 direct-hit footprint (Miami-Dade 0.037 a year,
  against 4 majors in 76 years × 0.8 = 0.042).
- **Income stability.** The research's ×0.5 step is `IncomeStability::very_stable` (tenured, public
  sector, pension); `stable` still means the typical salaried job (×1), as in the Philadelphia
  example. The research report gives only the ×0.5 point estimate for that step; its 0.3–0.7 range
  is this crate's own prior (`docs/RISK_MODEL.md` §"Hazard rates" / `rr_risk_model_priors`), not a
  cited figure.
- **Figures to confirm** against the source when the citations are written: UCERF3's 33 % for
  the Hayward fault, USGS's 7–10 % for New Madrid, the SSA "1 in 4" disability figure (checked 2026-09-26: the source says 1 in 4, not "more than"),
  the one-third major share of landfalling hurricanes, and the 1 % burglary prior.

**v0.2.0 (contract v2).**

- **Rare rows never enter Λ.** `rates` holds the ranked hazards only; the nine families are
  display rows. The old nuclear effect row in `rr-consequence` (supplies, 1 day / 3 days) no longer
  receives a rate. This drops the nuclear share from the supplies bucket (it was 1.1 in 1,000 a year
  against a one-in-100 dial) and makes "never drive the budget" hold by construction.
- **Ranges of the rare families span every combination.** The review computes its class ranges
  as low × low to high × high, so the rare families do too (`Estimate::times_span`); the ranked
  rates keep quadrature. The class examples reproduce REVIEW §2.3 within rounding and B1.5's ten-year
  ranges word for word (`tests/v2.rs`).
- **The UASI share is the urban area's, not the county's.** The attack, CBRN and nuclear-terrorism
  formulas weigh the metro area (λ × w_m × the share of its households under an order). The pack's
  `uasi_share` is the county's population split of its area's share, so the engine looks the area's
  own share up by `uasi_area` in the FY2026 table. awaiting: data-hazard — a `uasi_area_share` column
  would retire that table, and `LocationResolved::exposure.uasi_share` (the contract calls it the
  metro area's share) should carry the area's share, not the county split.
- **The solar-storm location term uses NERC's floor.** α never falls below 0.1, so Miami comes to
  1.2 in 10,000 a year, not the review's 6 in 100,000 (which used α 0.06). The population mean of α
  (0.2285) replaces the review's assumed 0.25. Ground conductivity (β) is not in the pack.
- **Levee residual behind high-risk levees.** 0.2 % a year (0.05–1 %) ×2 behind levees rated High or
  Very High puts the most exposed parishes (Concordia, Louisiana) at 4 in 1,000 a year (up to 2 in
  100): a prior with a range-only card, flagged for the owner.
- **Arrests are sex-averaged.** The household form does not ask sex; the range runs from the women's
  lowest year to the men's highest, and confidence is fixed at `medium`. Arrests are events: the
  chance that anyone in a household is arrested at least once is lower than the event rate implies,
  which the sentence says.
- **Eviction uses the national rate** until the owner approves Eviction Lab's ODC-BY licence; the
  county column then takes over with filings × 0.4 to judgments. Income stability (the job-loss
  modifier) and savings scale it, as the hazard-expansion CSV proposed.
- **Medicine shortages and phone outages are national priors**; phone and internet outages (0.3 a
  year) now rank near the top of most registers with a tiny severity ($100 an event).
- **Recalibrations from the series.** The data-model series (OE-417, FCC DIRS, funding gaps, FDIC,
  FBI arrests, openFDA) are used where a series measures the row: funding gaps (benefits), FBI
  arrests, FDIC and openFDA (notes). OE-417 counts reports, not household outages, so the regional
  blackout keeps its prior and gains the OE-417 physical-attack and cyber counts as sub-cause notes;
  no FCC or CSB series measures cyber outages or chemical releases, so those priors stand.
- **Outages by cause (M-18, M-10).** The outage floor can top up each storm hazard by its share of
  recorded outages matched by date (`natural::shortfall_by_cause`), instead of counting every
  shortfall as windstorms; it reads data-model's `OutageModel::causes` (awaiting: data-model), so
  the v0.1 rule runs until then. The hurricane double count M-10 is `rr-consequence`'s (county curve
  plus hurricane rows); `rr-hazards` already subtracts modelled hurricane outages in the floor.
- **New scenarios need effects rows.** `wasatch_m7`, `san_andreas_south_m78`, `seattle_fault_m7`
  and `heat_blackout` are offered with their rates and defaults; `rr-consequence` plans them once it
  has effects rows (awaiting: consequence). `heat_blackout` overlaps the compound heat-and-outage
  class `rr-consequence` is building; one of the two should own it.
- **Figures to confirm** (hazard-expansion "UNVERIFIED items"): III's 1 in 67 and $15,400, Eviction
  Lab's 2.3 in 100 and the 0.4 judgment share, the CSIS count of metro-wide closures, Riley 2012's
  12 % a decade, the Wasatch 43 %, southern San Andreas 19 % and Seattle fault 5 % figures, and the
  dust-storm rate in the Maricopa fixture (a test value until the Storm Events dust job lands).

## Consequences and targets

Owned by `crates/rr-consequence`. Entry point: `rr_consequence::assess(input, rates, county,
scenarios) -> ConsequenceAssessment`. The numbers are in `crates/rr-consequence/src/effects.toml`;
`cargo test -p rr-consequence` writes the research calibration to `target/calibration.md` and the
end-to-end run on the fixture households (with rr-hazards' rates) to `target/pipeline.md`.

### What it does, in plain words

Each hazard reaches the household some number of times a year (rr-hazards). The effects table
says what one event does: with some chance it cuts the power, stops the tap water, keeps the
household from shopping, interrupts medicine and so on, for a number of days that varies from event
to event. Adding every hazard's contribution gives, for each consequence, how often a disruption
longer than *d* days happens to a household like this one. The target is the number of days that
only the rarest disruptions (rarer than the dial) outlast, rounded up to the day ladder. The same
curve gives the natural-frequency sentences and the value of each extra day of supplies.

### Interface

```rust
rr_consequence::assess(
    input: &PlanInput,
    rates: &[HouseholdEventRate],        // HazardAssessment::rates (each hazard's full rate)
    county: CountyData<'_>,              // CountyData::from_record(&county_record)
    scenarios: &[ScenarioCandidate],     // rr_hazards::ScenarioCandidate (HazardAssessment::scenarios)
) -> ConsequenceAssessment
```

`ConsequenceAssessment` carries the 14 `BucketAssessment`s (in `BucketId::ALL` order), the
`ScenarioInfo`s, cliff `Warning`s, the self-sufficiency statement, and the numbers behind every
target: per duration bucket an `ExceedanceCurve` (`lambda`, `unmet_days`, `value_between`,
`consumption_days_per_year`, `natural_frequency`, `sample`, `target_at`, `ladder_at`), the raw
design duration, the dial table, the drivers and every term; the income curve; evacuation,
get-home and home-loss details; the coupling rules that fired; the county overrides applied; notes.
For rr-budget, `assessment.value_between(bucket, x0, x1)` = ∫ Λ_b over [x0, x1] and
`assessment.unmet_days(bucket, x)` = ∫ₓ^∞ Λ_b, exact (closed form, or numerical for the few terms
with a floor); `ExceedanceCurve::sample(n, max_days)` gives `BucketCurve`'s `(days, lambda)`.

What rr-plan still fills: `covered` holds only what the home already provides (a gas stove, item
`gas_stove` in `existing`, covers boil-water notices; savings cover income months); the plan's
coverage and the readiness checklists (`done`/`of`, left at 0) come from rr-supply and rr-budget.
Cliff warnings have ids `cliff_<bucket>` (rr-plan drops rr-budget's duplicate).

### How the shares read rr-hazards' events

A share in the effects table is the chance that **one event as rr-hazards defines it** (see "What a
rate means" above) causes the consequence. The shares were set so that rr-hazards' definitions
reproduce the research prototype's event classes:

- **Heat and cold waves** count every county episode, so per-episode shares are small: a cold wave
  overloads the grid 1 time in 200, brings a boil-water notice 1 in 300, freezes pipes 1 in 200.
  Homes without heating or cooling face dangerous indoor temperatures in every episode
  (`requires = no_heating / no_cooling`); air-conditioned homes and homes whose heating needs
  electricity face them only when a power cut coincides (each power row's `heat_share` /
  `cold_share`).
- **Windstorms, ice storms, winter storms, lightning** are events that cut the power or keep the
  household home: most of their outages are short and local (3 h, bad case 10 h); county-wide
  outages come from the county's EAGLE-I records; big windstorms beyond the records are 3 % of
  windstorms (Coos Bay's research class, 0.03 a year, over its windstorm rate).
- **Hurricanes** (rr-hazards: "cuts the power or damages the home"): 45 % bring a multi-day power
  cut (Irma restoration), 6 % are Category 3+ direct hits (Michael restoration) unless
  `major_hurricane_direct_hit` is offered, which replaces that class.
- **Earthquakes** are 0.1 g shaking (things off shelves), about three times as frequent as damaging
  shaking, so damage shares are about a third of a damaging quake's.
- **Pandemics** change daily life (a quarter of onsets): 9 in 10 disrupt shopping for about two
  weeks, and about 1 in 9 workers loses a job. **Chemical releases** bring a do-not-drink order (6
  in 10) or an order to stay inside. **Wildfire** comes in two parts that `rr-hazards` passes
  separately (v0.1.1): every warning to leave is an evacuation (and 1 in 20 of them a burned home),
  every safety shutoff a power cut; the table's rows name their `part`, and a caller with whole
  rates only gets the old 15 / 85 split as a fallback. **Landslides** likewise: the `damage` part
  forces the household out and damages the home, the whole rate (roads cut off included) closes
  roads and cuts power. **Drought** for a well household *is* the well running low.
- **Named scenarios** are extra event classes. rr-hazards passes each parent hazard's full rate;
  when a scenario is offered (on or off) its long-run share leaves the parent's ordinary rows:
  `[[overlap]]` rows scale the parent's rate by 1 − share/rate, never below a quarter (the same
  floor as the hazard card), where the share is the scenario's lowest published rate (Cascadia near
  Coos Bay: 0.41 %/yr, its long-run recurrence). Major hurricanes use `replaced_by` instead (a
  class, not a share). "Off" therefore means the plan leaves the scenario's event out entirely.

The calibration households use these rates (a year; `tests/support/research.rs`), set so that the
shares reproduce the research's event classes. Compare them with rr-hazards' rates in
`target/pipeline.md` to see where the two workstreams' numbers differ.

| Hazard | Philadelphia | Coos Bay |
|---|---|---|
| `strong_wind` | 0.133 | 0.99 (with the county's 1.09 recorded outages a year) |
| `winter_weather` | 0.5 | 0.2 |
| `ice_storm` | 0.091 | — |
| `hurricane` | 0.0242 | — |
| `wildfire`, `drought` | — | 0.0235, 0.01 |
| `grid_failure` | 0.005 | — |
| `local_utility_outage` | 0.15 | — (well) |
| `hazmat_release` | 0.0333 | — |
| `supply_chain_disruption` | 0.2 | 0.2 |
| `civil_unrest`, `cyber_outage` | 0.03, 0.02 | —, 0.02 |
| `pandemic` | 0.0115 | 0.0115 |
| `house_fire` | 0.0052 (attached) | 0.0026 |
| `job_loss` (household total) | 0.166 | 0.166 |
| `medical_emergency` (household total) | 1.892 | 0.946 |
| scenario `cascadia_m9` (coast) | — | 0.0102 (alternative 0.0041) |
| scenario `local_tsunami` | — | 0.0028 (27 % of the Cascadia rate: one adult works in the zone) |

### The effects table

Every row is an **event class** of a hazard or named scenario: rows with the same class are the same
event hitting several buckets, which keeps the buckets correlated. *Share* is the chance that one
event is of this class and causes this consequence; durations are log-normal with the median and
90th percentile shown (σ = ln(p90/median)/z₀.₉). Evidence "data" means both the share and the
duration rest on records (or the share is 1 by definition); "prior" is an expert estimate, cited as
`rr_risk_model_priors` and shown as such. Sources: research §2.4 (hazard-to-bucket map), §2.5
(empirical durations), §6.2 (societal priors) and the §7.5 EAGLE-I backtest, which replaced the
first-pass power priors with county outage records for ordinary storms and kept explicit priors
only for classes the 2018–2025 records cannot show (Sandy-class storms and ice storms of record,
big windstorms, grid failure, Cascadia).

<!-- effects-table:start (generated from crates/rr-consequence/src/effects.toml) -->
| Hazard or *scenario* | Bucket | Class | Event class in words | Share | Median | Bad case (p90) | Evidence | Applies / notes | Sources |
|---|---|---|---|---|---|---|---|---|---|
| avalanche | supplies | slide | avalanches closing roads | 1 | 1 d | 4 d | prior |  | rr_risk_model_priors |
| avalanche | get_home | slide | avalanches closing roads | 0.3 | — | — | prior | households with a commuter | rr_risk_model_priors |
| avalanche | evacuate | slide | avalanches closing roads | 0.05 | 2 d | 7 d | prior | warning 0.5–12 h | rr_risk_model_priors |
| avalanche | home_loss | slide | avalanches closing roads | 0.01 | — | — | prior |  | rr_risk_model_priors |
| coastal_flooding | evacuate | surge | flooding from the sea | 0.5 | 3 d | 30 d | prior | warning 12–72 h | rr_risk_model_priors |
| coastal_flooding | home_loss | surge | flooding from the sea | 0.2 | — | — | prior |  | fema_nri_v120, rr_risk_model_priors |
| coastal_flooding | water_boil | surge | flooding from the sea | 0.1 | 3 d | 10 d | prior | homes on public water | epa_boil_water_report_2024, rr_risk_model_priors |
| coastal_flooding | power | surge | flooding from the sea | 0.3 | 1 d | 5 d | prior | in heat 0.2; in cold 0.3 | rr_risk_model_priors |
| coastal_flooding | supplies | surge | flooding from the sea | 0.3 | 1 d | 5 d | prior |  | rr_risk_model_priors |
| coastal_flooding | medication | surge | flooding from the sea | 0.1 | 5 d | 30 d | prior |  | rr_risk_model_priors |
| cold_wave | power | extreme_cold | extreme cold straining the grid | 0.005 | 1 d | 3 d | prior | in cold 1 | rr_risk_model_priors |
| cold_wave | water_boil | extreme_cold | extreme cold straining the grid | 0.003 | 7 d | 21 d | prior | homes on public water | shaffer_2026_texas_boil_notices, rr_risk_model_priors |
| cold_wave | water_out | extreme_cold | extreme cold straining the grid | 0.005 | 1 d | 5 d | prior |  | rr_risk_model_priors |
| cold_wave | thermal | no_heating | cold waves in a home with no heating | 1 | 2 d | 5 d | prior | homes without heating; county events: cold_wave, extreme_cold, extreme_cold_wind_chill, cold_wind_chill | rr_risk_model_priors |
| cold_wave | supplies | extreme_cold | extreme cold straining the grid | 0.1 | 1 d | 2 d | prior |  | rr_risk_model_priors |
| drought | water_out | low_yield | drought lowering a well | 1 | 30 d | 90 d | prior | homes on a private well; county events: drought | rr_risk_model_priors |
| earthquake | power | shaking | earthquake shaking that knocks things off shelves | 0.3 | 12 h | 3 d | prior | in heat 0.1; in cold 0.4 | fema_hazus_eq_restoration, oregon_resilience_plan_2013, rr_risk_model_priors |
| earthquake | water_out | shaking | earthquake shaking that knocks things off shelves | 0.1 | 3 d | 14 d | prior | homes on public water | fema_hazus_eq_restoration, rr_risk_model_priors |
| earthquake | water_out | shaking | earthquake shaking that knocks things off shelves | 0.07 | 8 d | 20 d | prior | homes on a private well | fema_hazus_eq_restoration, rr_risk_model_priors |
| earthquake | supplies | shaking | earthquake shaking that knocks things off shelves | 0.1 | 1 d | 5 d | prior |  | rr_risk_model_priors |
| earthquake | medication | shaking | earthquake shaking that knocks things off shelves | 0.07 | 2 d | 7 d | prior |  | rr_risk_model_priors |
| earthquake | comms | shaking | earthquake shaking that knocks things off shelves | 0.3 | 12 h | 3 d | prior |  | rr_risk_model_priors |
| earthquake | evacuate | shaking | earthquake shaking that knocks things off shelves | 0.02 | 3 d | 30 d | prior | warning 0–0.05 h | rr_risk_model_priors |
| earthquake | home_loss | shaking | earthquake shaking that knocks things off shelves | 0.02 | — | — | prior |  | fema_nri_v120, rr_risk_model_priors |
| earthquake | get_home | shaking | earthquake shaking that knocks things off shelves | 0.1 | — | — | prior | households with a commuter | rr_risk_model_priors |
| hail | power | local | short storm outages | 0.1 | 3 h | 10 h | prior | county outage records replace the county-wide part; in heat 0.3 | rr_risk_model_priors |
| hail | home_loss | local | short storm outages | 0.002 | — | — | prior |  | rr_risk_model_priors |
| heat_wave | power | heat_outage | heat waves straining the grid | 0.02 | 4 h | 20 h | prior | county outage records replace the county-wide part; in heat 1 | stone_2023_heat_blackout, rr_risk_model_priors |
| heat_wave | thermal | no_cooling | heat waves in a home without air conditioning | 1 | 3 d | 7 d | prior | homes without air conditioning; county events: heat_wave, excessive_heat, heat | rr_risk_model_priors |
| hurricane | power | cat12 | hurricanes and tropical storms | 0.45 | 1.5 d | 5 d | duration data, share prior | in heat 0.5 | ornl_repowrd_2022, rr_risk_model_priors |
| hurricane | power | cat3plus | major hurricanes (category 3 or a direct hit) | 0.06 | 5 d | 16 d | duration data, share prior | in heat 0.5; dropped when *major_hurricane_direct_hit* is offered | ornl_repowrd_2022, rr_risk_model_priors |
| hurricane | water_boil | cat12 | hurricanes and tropical storms | 0.15 | 6 d | 18 d | prior | homes on public water | shaffer_2026_texas_boil_notices, rr_risk_model_priors |
| hurricane | supplies | cat12 | hurricanes and tropical storms | 0.1 | 2 d | 5 d | prior |  | rr_risk_model_priors |
| hurricane | supplies | cat3plus | major hurricanes (category 3 or a direct hit) | 0.06 | 5 d | 14 d | prior | dropped when *major_hurricane_direct_hit* is offered | rr_risk_model_priors |
| hurricane | medication | cat3plus | major hurricanes (category 3 or a direct hit) | 0.06 | 7 d | 21 d | prior | dropped when *major_hurricane_direct_hit* is offered | rr_risk_model_priors |
| hurricane | comms | cat12 | hurricanes and tropical storms | 0.1 | 12 h | 2 d | prior |  | rr_risk_model_priors |
| hurricane | comms | cat3plus | major hurricanes (category 3 or a direct hit) | 0.06 | 3 d | 10 d | prior | dropped when *major_hurricane_direct_hit* is offered | rr_risk_model_priors |
| hurricane | evacuate | cat12 | hurricanes and tropical storms | 0.1 | 3 d | 14 d | prior | warning 24–72 h | rr_risk_model_priors |
| hurricane | evacuate | mobile_home | wind warnings for mobile homes | 0.2 | 2 d | 7 d | prior | mobile homes; warning 24–72 h | rr_risk_model_priors |
| hurricane | home_loss | cat12 | hurricanes and tropical storms | 0.01 | — | — | prior |  | rr_risk_model_priors |
| hurricane | home_loss | cat3plus | major hurricanes (category 3 or a direct hit) | 0.02 | — | — | prior | dropped when *major_hurricane_direct_hit* is offered | rr_risk_model_priors |
| ice_storm | power | local | short storm outages | 0.65 | 3 h | 10 h | prior | county outage records replace the county-wide part; in cold 1 | rr_risk_model_priors |
| ice_storm | power | major | big ice storms | 0.1 | 1.5 d | 5 d | prior | in cold 1 | inquirer_peco_outages, rr_risk_model_priors |
| ice_storm | power | record | an ice storm of record | 0.017 | 5 d | 14 d | prior | in cold 1 | rr_risk_model_priors |
| ice_storm | supplies | icy_roads | icy roads | 0.5 | 1 d | 3 d | prior |  | rr_risk_model_priors |
| ice_storm | supplies | record | an ice storm of record | 0.017 | 5 d | 14 d | prior |  | rr_risk_model_priors |
| ice_storm | medication | record | an ice storm of record | 0.017 | 7 d | 21 d | prior |  | rr_risk_model_priors |
| ice_storm | comms | record | an ice storm of record | 0.017 | 3 d | 10 d | prior |  | rr_risk_model_priors |
| ice_storm | get_home | icy_roads | icy roads | 0.05 | — | — | prior | households with a commuter | rr_risk_model_priors |
| landslide | supplies | slide | landslides closing roads | 0.5 | 1 d | 5 d | prior |  | rr_risk_model_priors |
| landslide | power | slide | landslides closing roads | 0.1 | 12 h | 2 d | prior | in cold 0.5 | rr_risk_model_priors |
| landslide | evacuate | damage | landslides damaging the home | 1 | 5 d | 60 d | prior | share of the landslides that damage the home (part `damage`); warning 0–2 h | rr_risk_model_priors |
| landslide | home_loss | damage | landslides damaging the home | 1 | — | — | prior | share of the landslides that damage the home (part `damage`) | rr_risk_model_priors |
| landslide | get_home | slide | landslides closing roads | 0.1 | — | — | prior | households with a commuter | rr_risk_model_priors |
| lightning | power | local | short storm outages | 0.5 | 3 h | 10 h | prior | county outage records replace the county-wide part; in heat 0.3 | rr_risk_model_priors |
| riverine_flooding | evacuate | flood | floods at or near the home | 0.3 | 3 d | 30 d | prior | warning 1–12 h | rr_risk_model_priors |
| riverine_flooding | home_loss | flood | floods at or near the home | 0.15 | — | — | prior |  | fema_nri_v120, rr_risk_model_priors |
| riverine_flooding | water_boil | flood | floods at or near the home | 0.1 | 3 d | 10 d | prior | homes on public water | epa_boil_water_report_2024, rr_risk_model_priors |
| riverine_flooding | power | flood | floods at or near the home | 0.1 | 12 h | 3 d | prior | in heat 0.2; in cold 0.3 | rr_risk_model_priors |
| riverine_flooding | supplies | flood | floods at or near the home | 0.2 | 1 d | 4 d | prior |  | rr_risk_model_priors |
| riverine_flooding | medication | flood | floods at or near the home | 0.15 | 5 d | 30 d | prior |  | rr_risk_model_priors |
| riverine_flooding | get_home | flood | floods at or near the home | 0.05 | — | — | prior | households with a commuter | rr_risk_model_priors |
| strong_wind | power | local | short storm outages | 0.85 | 3 h | 10 h | prior | county outage records replace the county-wide part; in heat 0.15; in cold 0.4 | eia_861_reliability_2024, rr_risk_model_priors |
| strong_wind | power | major | a windstorm bigger than any in the recent records | 0.03 | 1.5 d | 5 d | prior | in cold 0.8 | rr_risk_model_priors |
| strong_wind | comms | local | short storm outages | 0.3 | 10 h | 1.8 d | prior |  | rr_risk_model_priors |
| tornado | power | path | tornadoes | 0.8 | 1 d | 5 d | prior | in heat 0.2; in cold 0.1 | rr_risk_model_priors |
| tornado | supplies | path | tornadoes | 0.3 | 1 d | 3 d | prior |  | rr_risk_model_priors |
| tornado | comms | path | tornadoes | 0.3 | 12 h | 2 d | prior |  | rr_risk_model_priors |
| tornado | medication | path | tornadoes | 0.1 | 2 d | 7 d | prior |  | rr_risk_model_priors |
| tornado | home_loss | path | tornadoes | 0.3 | — | — | prior |  | rr_risk_model_priors |
| tornado | evacuate | mobile_home | wind warnings for mobile homes | 0.5 | 1 d | 7 d | prior | mobile homes; warning 0.1–0.5 h | rr_risk_model_priors |
| tsunami | evacuate | distant | tsunamis from distant earthquakes | 1 | 1 d | 7 d | prior | warning 2–12 h | rr_risk_model_priors |
| tsunami | home_loss | distant | tsunamis from distant earthquakes | 0.1 | — | — | prior |  | rr_risk_model_priors |
| volcanic_activity | supplies | ash | volcanic ash | 0.5 | 2 d | 7 d | prior |  | rr_risk_model_priors |
| volcanic_activity | evacuate | ash | volcanic ash | 0.2 | 5 d | 30 d | prior | warning 1–48 h | rr_risk_model_priors |
| volcanic_activity | power | ash | volcanic ash | 0.2 | 1 d | 5 d | prior | in cold 0.4 | rr_risk_model_priors |
| volcanic_activity | water_boil | ash | volcanic ash | 0.2 | 2 d | 7 d | prior | homes on public water | rr_risk_model_priors |
| volcanic_activity | comms | ash | volcanic ash | 0.1 | 1 d | 3 d | prior |  | rr_risk_model_priors |
| volcanic_activity | home_loss | ash | volcanic ash | 0.02 | — | — | prior |  | rr_risk_model_priors |
| wildfire | evacuate | threat | wildfires | 1 | 3 d | 30 d | prior | share of the wildfire warnings to leave (part `burn`); warning 0.25–12 h | rr_risk_model_priors |
| wildfire | power | shutoff | wildfire safety power shutoffs | 1 | 1 d | 3 d | prior | share of the wildfire safety power shutoffs (part `shutoff`); in heat 0.3 | rr_risk_model_priors |
| wildfire | supplies | smoke | wildfire smoke | 0.3 | 2 d | 7 d | prior |  | rr_risk_model_priors |
| wildfire | home_loss | threat | wildfires | 0.05 | — | — | prior | share of the wildfire warnings to leave (part `burn`) | rr_risk_model_priors |
| winter_weather | supplies | snowed_in | snow and ice storms | 1 | 1 d | 2 d | prior | county events: winter_weather, winter_storm, blizzard, heavy_snow | rr_risk_model_priors |
| winter_weather | power | local | short storm outages | 0.3 | 3 h | 10 h | prior | county outage records replace the county-wide part; in cold 1 | rr_risk_model_priors |
| winter_weather | comms | local | short storm outages | 0.02 | 10 h | 1.8 d | prior |  | rr_risk_model_priors |
| winter_weather | get_home | snowed_in | snow and ice storms | 0.05 | — | — | prior | households with a commuter | rr_risk_model_priors |
| pandemic | supplies | stay_home | a pandemic that disrupts shopping | 0.9 | 14 d | 45 d | prior |  | cdc_mmwr_stay_at_home_2020, cdc_pandemic_history, rr_risk_model_priors |
| pandemic | medication | stay_home | a pandemic that disrupts shopping | 0.9 | 7 d | 30 d | prior |  | cdc_pandemic_history, rr_risk_model_priors |
| grid_failure | power | regional | a regional blackout | 1 | 1 d | 3 d | prior | in heat 0.3; in cold 0.4 | rr_risk_model_priors |
| grid_failure | water_out | regional | a regional blackout | 0.4 | 1 d | 3 d | prior | homes on public water | rr_risk_model_priors |
| grid_failure | comms | regional | a regional blackout | 0.5 | 1 d | 3 d | prior |  | rr_risk_model_priors |
| grid_failure | get_home | regional | a regional blackout | 0.27 | — | — | prior | households with a commuter | rr_risk_model_priors |
| cyber_outage | medication | pharmacy_it | pharmacy or insurer computer outages | 1 | 5 d | 21 d | prior |  | rr_risk_model_priors |
| cyber_outage | comms | payments | card-payment outages | 0.25 | 1 d | 3 d | prior |  | rr_risk_model_priors |
| civil_unrest | supplies | curfew | curfews | 1 | 2 d | 5 d | prior |  | rr_risk_model_priors |
| civil_unrest | security | curfew | curfews | 1 | — | — | prior |  | rr_risk_model_priors |
| civil_unrest | get_home | curfew | curfews | 0.1 | — | — | prior | households with a commuter | rr_risk_model_priors |
| supply_chain_disruption | supplies | shortage | store shortages and runs on stores | 1 | 2 d | 5 d | prior |  | rr_risk_model_priors |
| supply_chain_disruption | medication | pharmacy_access | pharmacy closures and refill delays | 0.5 | 1.5 d | 4 d | prior |  | rr_risk_model_priors |
| hazmat_release | water_out | release | chemical releases | 0.6 | 1.5 d | 7 d | prior | homes on public water | rr_risk_model_priors |
| hazmat_release | supplies | release | chemical releases | 0.4 | 6 h | 1 d | prior |  | rr_risk_model_priors |
| hazmat_release | evacuate | release | chemical releases | 0.1 | 1 d | 3 d | prior | warning 0.1–2 h | rr_risk_model_priors |
| nuclear_plant_incident | evacuate | release | a nuclear plant accident | 1 | 7 d | 60 d | prior | warning 1–12 h | rr_risk_model_priors |
| nuclear_plant_incident | supplies | release | a nuclear plant accident | 1 | 1 d | 3 d | prior |  | rr_risk_model_priors |
| nuclear_plant_incident | home_loss | release | a nuclear plant accident | 0.05 | — | — | prior |  | rr_risk_model_priors |
| nuclear_attack | supplies | shelter | a nuclear attack | 1 | 1 d | 3 d | prior |  | ready_gov_nuclear, rr_risk_model_priors |
| terrorism | supplies | lockdown | a terrorist attack | 0.3 | 12 h | 2 d | prior |  | rr_risk_model_priors |
| terrorism | comms | lockdown | a terrorist attack | 0.2 | 12 h | 2 d | prior |  | rr_risk_model_priors |
| terrorism | evacuate | lockdown | a terrorist attack | 0.05 | 1 d | 3 d | prior | warning 0–1 h | rr_risk_model_priors |
| terrorism | security | lockdown | a terrorist attack | 0.5 | — | — | prior |  | rr_risk_model_priors |
| house_fire | evacuate | fire | a fire at home or next door | 1 | 3 d | 60 d | prior | warning 0.02–0.1 h | usfa_residential_fires, rr_risk_model_priors |
| house_fire | fire | fire | a fire at home or next door | 1 | — | — | data |  | usfa_residential_fires |
| house_fire | home_loss | fire | a fire at home or next door | 0.5 | — | — | prior |  | usfa_residential_fires, census_pulse_displacement, rr_risk_model_priors |
| house_fire | medication | fire | a fire at home or next door | 1 | 5 d | 30 d | prior |  | rr_risk_model_priors |
| medical_emergency | medical_emergency | emergency | medical emergencies | 1 | — | — | data |  | cdc_nchs_ed_visits |
| vehicle_stranding | get_home | stranded | being stranded by a breakdown or closed roads | 1 | — | — | prior |  | rr_risk_model_priors |
| local_utility_outage | water_out | pressure_loss | water main breaks | 0.667 | 6 h | 1 d | prior | homes on public water | epa_boil_water_report_2024, rr_risk_model_priors |
| local_utility_outage | water_boil | notice | local water problems | 0.333 | 2 d | 6 d | prior | homes on public water; county events: boil_water_notice, boil_water | shaffer_2026_texas_boil_notices, water_2024_kentucky_advisories, rr_risk_model_priors |
| local_utility_outage | water_out | system_failure | a major water system failure | 0.0267 | 7 d | 30 d | prior | homes on public water | epa_asheville_boil_notice_2024, rr_risk_model_priors |
| local_utility_outage | evacuate | gas_leak | gas leaks and building emergencies | 0.02 | 1 d | 5 d | prior | warning 0.1–1 h | rr_risk_model_priors |
| local_utility_outage | medication | gas_leak | gas leaks and building emergencies | 0.02 | 5 d | 30 d | prior |  | rr_risk_model_priors |
| burglary | security | break_in | break-ins | 1 | — | — | prior |  | rr_risk_model_priors |
| extended_household_illness | medical_emergency | illness | a long illness at home | 0.5 | — | — | prior |  | rr_risk_model_priors |
| *cascadia_m9* (coast) | power | event | a magnitude 9 Cascadia earthquake | 1 | 90 d | 180 d | prior | in cold 0.4; relief: help 14 d, mostly restored 180 d | oregon_resilience_plan_2013 |
| *cascadia_m9* (coast) | water_out | event | a magnitude 9 Cascadia earthquake | 1 | 365 d | 1095 d | prior | homes on public water; relief: help 14 d, mostly restored 1095 d | oregon_resilience_plan_2013 |
| *cascadia_m9* (coast) | water_out | event | a magnitude 9 Cascadia earthquake | 1 | 90 d | 270 d | prior | homes on a private well; relief: help 14 d, mostly restored 270 d | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (coast) | supplies | event | a magnitude 9 Cascadia earthquake | 1 | 21 d | 60 d | prior | relief: help 14 d, mostly restored 1095 d | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (coast) | medication | event | a magnitude 9 Cascadia earthquake | 1 | 30 d | 90 d | prior | relief: help 14 d, mostly restored 1095 d | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (coast) | comms | event | a magnitude 9 Cascadia earthquake | 1 | 14 d | 60 d | prior |  | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (coast) | home_loss | event | a magnitude 9 Cascadia earthquake | 0.1 | — | — | prior |  | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (coast) | get_home | event | a magnitude 9 Cascadia earthquake | 0.27 | — | — | prior | households with a commuter | rr_risk_model_priors |
| *cascadia_m9* (valley) | power | event | a magnitude 9 Cascadia earthquake | 1 | 30 d | 90 d | prior | in cold 0.4; relief: help 3 d, mostly restored 90 d | oregon_resilience_plan_2013 |
| *cascadia_m9* (valley) | water_out | event | a magnitude 9 Cascadia earthquake | 1 | 30 d | 365 d | prior | homes on public water; relief: help 3 d, mostly restored 365 d | oregon_resilience_plan_2013 |
| *cascadia_m9* (valley) | water_out | event | a magnitude 9 Cascadia earthquake | 1 | 30 d | 90 d | prior | homes on a private well; relief: help 3 d, mostly restored 90 d | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (valley) | supplies | event | a magnitude 9 Cascadia earthquake | 1 | 7 d | 21 d | prior | relief: help 3 d, mostly restored 365 d | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (valley) | medication | event | a magnitude 9 Cascadia earthquake | 1 | 14 d | 60 d | prior | relief: help 3 d, mostly restored 540 d | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (valley) | comms | event | a magnitude 9 Cascadia earthquake | 1 | 7 d | 30 d | prior |  | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (valley) | home_loss | event | a magnitude 9 Cascadia earthquake | 0.05 | — | — | prior |  | oregon_resilience_plan_2013, rr_risk_model_priors |
| *cascadia_m9* (valley) | get_home | event | a magnitude 9 Cascadia earthquake | 0.27 | — | — | prior | households with a commuter | rr_risk_model_priors |
| *local_tsunami* | evacuate | event | a tsunami from a nearby earthquake | 1 | 3 d | 90 d | prior | warning 0.25–0.33 h | dogami_tsunami_faq, rr_risk_model_priors |
| *local_tsunami* | home_loss | event | a tsunami from a nearby earthquake | 0.5 | — | — | prior |  | rr_risk_model_priors |
| *major_hurricane_direct_hit* | power | event | a direct hit by a major hurricane | 1 | 5 d | 16 d | data | in heat 0.5 | ornl_repowrd_2022 |
| *major_hurricane_direct_hit* | water_out | event | a direct hit by a major hurricane | 0.5 | 7 d | 49 d | prior | homes on public water | epa_asheville_boil_notice_2024, rr_risk_model_priors |
| *major_hurricane_direct_hit* | water_boil | event | a direct hit by a major hurricane | 0.5 | 6 d | 18 d | prior | homes on public water | shaffer_2026_texas_boil_notices, rr_risk_model_priors |
| *major_hurricane_direct_hit* | supplies | event | a direct hit by a major hurricane | 1 | 5 d | 14 d | prior |  | rr_risk_model_priors |
| *major_hurricane_direct_hit* | medication | event | a direct hit by a major hurricane | 1 | 7 d | 21 d | prior |  | rr_risk_model_priors |
| *major_hurricane_direct_hit* | comms | event | a direct hit by a major hurricane | 1 | 2 d | 7 d | prior |  | rr_risk_model_priors |
| *major_hurricane_direct_hit* | evacuate | event | a direct hit by a major hurricane | 0.8 | 5 d | 30 d | prior | warning 24–72 h | rr_risk_model_priors |
| *major_hurricane_direct_hit* | home_loss | event | a direct hit by a major hurricane | 0.3 | — | — | prior |  | rr_risk_model_priors |
| *new_madrid_m7* | power | event | a magnitude 7 New Madrid earthquake | 1 | 3 d | 14 d | prior | in heat 0.1; in cold 0.4 | fema_hazus_eq_restoration, oregon_resilience_plan_2013, rr_risk_model_priors |
| *new_madrid_m7* | water_out | event | a magnitude 7 New Madrid earthquake | 0.7 | 14 d | 60 d | prior | homes on public water | fema_hazus_eq_restoration, rr_risk_model_priors |
| *new_madrid_m7* | water_out | event | a magnitude 7 New Madrid earthquake | 0.5 | 8 d | 20 d | prior | homes on a private well | fema_hazus_eq_restoration |
| *new_madrid_m7* | supplies | event | a magnitude 7 New Madrid earthquake | 1 | 7 d | 21 d | prior |  | rr_risk_model_priors |
| *new_madrid_m7* | medication | event | a magnitude 7 New Madrid earthquake | 1 | 7 d | 30 d | prior |  | rr_risk_model_priors |
| *new_madrid_m7* | comms | event | a magnitude 7 New Madrid earthquake | 1 | 3 d | 14 d | prior |  | oregon_resilience_plan_2013, rr_risk_model_priors |
| *new_madrid_m7* | home_loss | event | a magnitude 7 New Madrid earthquake | 0.1 | — | — | prior |  | rr_risk_model_priors |
| *new_madrid_m7* | evacuate | event | a magnitude 7 New Madrid earthquake | 0.05 | 3 d | 30 d | prior | warning 0–0.05 h | rr_risk_model_priors |
| *new_madrid_m7* | get_home | event | a magnitude 7 New Madrid earthquake | 0.27 | — | — | prior | households with a commuter | rr_risk_model_priors |
| *hayward_m7* | power | event | a magnitude 7 Hayward fault earthquake | 1 | 3 d | 14 d | prior | in heat 0.1; in cold 0.3 | fema_hazus_eq_restoration, oregon_resilience_plan_2013, rr_risk_model_priors |
| *hayward_m7* | water_out | event | a magnitude 7 Hayward fault earthquake | 0.7 | 14 d | 60 d | prior | homes on public water | fema_hazus_eq_restoration, rr_risk_model_priors |
| *hayward_m7* | water_out | event | a magnitude 7 Hayward fault earthquake | 0.5 | 8 d | 20 d | prior | homes on a private well | fema_hazus_eq_restoration |
| *hayward_m7* | supplies | event | a magnitude 7 Hayward fault earthquake | 1 | 7 d | 21 d | prior |  | rr_risk_model_priors |
| *hayward_m7* | medication | event | a magnitude 7 Hayward fault earthquake | 1 | 7 d | 30 d | prior |  | rr_risk_model_priors |
| *hayward_m7* | comms | event | a magnitude 7 Hayward fault earthquake | 1 | 3 d | 14 d | prior |  | oregon_resilience_plan_2013, rr_risk_model_priors |
| *hayward_m7* | home_loss | event | a magnitude 7 Hayward fault earthquake | 0.1 | — | — | prior |  | rr_risk_model_priors |
| *hayward_m7* | evacuate | event | a magnitude 7 Hayward fault earthquake | 0.05 | 3 d | 30 d | prior | warning 0–0.05 h | rr_risk_model_priors |
| *hayward_m7* | get_home | event | a magnitude 7 Hayward fault earthquake | 0.27 | — | — | prior | households with a commuter | rr_risk_model_priors |

| Income stream | Per earner | Spell median | Spell bad case | Unemployment insurance | Evidence | Sources |
|---|---|---|---|---|---|---|
| job_loss: losing a job | 1 | 10 wk | 36 wk | yes | prior | bls_work_experience_2024, bls_unemployment_duration, rr_risk_model_priors |
| pandemic: job losses in a pandemic | 0.11 | 10 wk | 36 wk | yes | prior | cdc_pandemic_history, rr_risk_model_priors |
| *cascadia_m9*: the regional economy after a Cascadia earthquake | 0.3 | 26 wk | 78 wk | yes | prior | oregon_resilience_plan_2013, rr_risk_model_priors |
| *major_hurricane_direct_hit*: the local economy after a major hurricane | 0.1 | 10 wk | 36 wk | yes | prior | rr_risk_model_priors |
| *new_madrid_m7*: the regional economy after a New Madrid earthquake | 0.1 | 10 wk | 36 wk | yes | prior | rr_risk_model_priors |
| *hayward_m7*: the regional economy after a Hayward fault earthquake | 0.1 | 10 wk | 36 wk | yes | prior | rr_risk_model_priors |

| Hazard | Part | Events it counts | Share when no split is passed | Why |
|---|---|---|---|---|
| wildfire | `burn` | wildfire warnings to leave | 0.15 | rr-hazards: NRI burn probability x residents exposed x 20 households warned per home that burns (model review M-06: these used to be added to the shutoffs and re-split 15/85, which undercounted evacuations 4 to 7 times). |
| wildfire | `shutoff` | wildfire safety power shutoffs | 0.85 | rr-hazards: 0.02 a year in the western shutoff states (less in cities), none elsewhere. |
| landslide | `damage` | landslides that damage the home | 0.1 | rr-hazards: exposed residents x NRI loss ratio / damage ratio 0.3, bounded by NRI's expected annual loss over the county's building value and by 1 in 100 a year (the high-risk flood-zone yardstick); roads cut off are 10 times as many (3 to 30). |

| Scenario | Parent hazard whose ordinary rows give up the scenario's long-run share | Why |
|---|---|---|
| *cascadia_m9* | earthquake | The county earthquake rate includes Cascadia's own shaking (0.41 %/yr near Coos Bay, the long-run recurrence). |
| *hayward_m7* | earthquake | The Bay Area earthquake rate includes Hayward fault ruptures. |
| *new_madrid_m7* | earthquake | The county earthquake rate includes New Madrid ruptures. |
<!-- effects-table:end -->

### County overrides

- **Power (EAGLE-I).** With `OutageStats`, county-wide storm outages happen at the county's
  measured rate (`events_per_customer_year`) with its measured duration curve: a curve through the
  median, the 90th percentile and `p_ge_1d/3d/7d/14d`, linear in (ln d, normal score), so a
  log-normal fit is reproduced exactly and the tail follows the county's own shares. Two bounds
  (verification V-01, 2026-09-26): the first zero share past the last positive one is an upper
  bound of 0.5 % at that length (no customer outage that long was recorded), not a missing point;
  and beyond the last point the curve decays at least as fast as a log-normal with σ = 2. Without
  them a small county's three points ran on to a year (Eddy County, North Dakota: longest recorded
  event 62 hours, power target 365 days; now 5), and 88 counties were told to prepare for a year
  without power. The rate is
  split between the storm hazards in proportion to their short-outage rates, for the
  contributions. A county with no outage records of its own uses its state's pooled series
  (`outages_state.csv`; `OutageStats.state_series` names the state, and the notes and the override
  say whose records they are; verification V-15, 2026-09-26: Juneau went from ½ day of power and no
  heat-or-cold target to 3 days of each). Only where there is no state row either (American Samoa,
  Guam, the Northern Mariana Islands) are county-wide outages a third of short storm outages at
  2 h / 20 h (the Philadelphia fit, prior). Recorded in `overrides`; `ornl_eagle_i_outages` joins
  the power bucket's sources.
- **County event records.** A `county.events` entry whose key is in a row's `event_keys` and that
  has `median_days` replaces the row's duration (its `p90_days` too, or the row's ratio): snow-ins
  (`winter_storm` …), heat and cold spells for homes without cooling or heating (`heat`,
  `extreme_cold` …), well droughts, boil-water notices. Source `noaa_storm_events` (weather) or
  `county_boil_water_records`.

### Coupling rules

One explainable line each (`ConsequenceAssessment::couplings`); a rule appears only when it changes
something for the household.

| Rule | When | Effect in the model | Applied in |
|---|---|---|---|
| `well_pump` | private well | every power cut is also a water cut; where the table has its own water row for the same event (Cascadia "power plus possible well damage", earthquake well damage), the water outage lasts at least as long as the power cut (co-monotone: P(max(D₁,D₂) > d) = max(S₁,S₂)), so water is never shorter than power in any draw; plus pump failure 0.07/yr, 2 d / 7 d (prior) | rr-consequence |
| `high_rise_pumps` | `apartment_high_rise` at floor 7 or above on public water | as the well rule, without pump failure or drought | rr-consequence |
| `heating_needs_power` | gas, oil, propane, electric or district heat | power cuts in cold weather (each row's `cold_share`) longer than half a day become dangerous-cold days | rr-consequence |
| `wood_heat` | wood heat | no heating coupling; a residual 0.01/yr, 1 d / 3 d, for outages that outrun the stove (prior) | rr-consequence |
| `no_heating` | no heating | cold waves themselves are dangerous-cold days (2 d / 5 d, or county records) | rr-consequence |
| `cooling_needs_power` | central or window air conditioning | power cuts during dangerous heat (each row's `heat_share`) become dangerous-heat days | rr-consequence |
| `no_cooling` | no air conditioning | heat waves themselves are dangerous-heat days (3 d / 7 d, or county records) | rr-consequence |
| `refrigerated_medicine` | anyone with refrigerated medicine | power cuts longer than a day interrupt medicine (with the same floor where the table has a medicine row for the event) | rr-consequence |
| `mobile_home` | mobile home | evacuation for hurricane winds and tornado warnings | rr-consequence |
| `commute` | anyone who commutes | walk home at 3 mph, half a litre of water an hour in heat; get-home events count | rr-consequence |
| `rural_ems` | rural setting | a sentence on slower ambulances (`mell_2017_ems_response`) | rr-consequence |
| `renter` | renting | a sentence on renters insurance; 1 in 4 displaced renters never return (1 in 10 owners) | rr-consequence |
| `infant`, `pets` | an infant; any pets | water +50 % and formula; pet water, food and carriers | note for rr-supply |
| `powered_device`, `senior`, `renter_no_generator` | a powered device; someone 65+; renting | harm weight ×3 on power; thermal weight; no generator or transfer switch | note for rr-budget |
| `attached_housing` | rowhouse or apartment | fire rate ×2 | note for rr-hazards |

### The dial and the targets

For every duration bucket, Λ_b(d) = Σ r_h · q_{h,b} · S_{h,b}(d) (thresholds and floors applied).

- **Dial rates** (planner decision, 2026-09-25): `one_in_10` = 0.1, `one_in_50` = 0.02,
  `one_in_100` = −ln(0.9)/10 = **0.010536051565782628** a year (the research default: for any one need, "90 % sure
  nothing in the next ten years is worse than its target"; across all needs together the chance
  that at least one runs out is roughly 1 in 3, M-04; shown as "about 1 in 100"), `one_in_500` = 0.002
  (`rr_consequence::dial_rate`, `ONE_IN_100_RATE`).
- **Ladder rule** (planner decision): the raw design duration (the smallest d with Λ_b(d) ≤ the dial
  rate) is rounded **up** to the ladder ½, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90, 180, 365
  days, and **a raw value within 3 % above a step counts as that step** (3.003 days is 3 days, not
  5). 0 when disruptions of any length are rarer than the dial; 365 when even a year is not enough.
  The same rule applies to the months ladder (½ … 36) for income and to the days away from home. The
  raw value stays in `BucketDetail::target_days` and `IncomeDetail::target_months`.
- **Natural frequencies**: out of 100 households like this one, 100·(1 − e^(−T·Λ_b(d))) face a
  disruption longer than d in the next T = `horizon_years` years, rounded per research §7.1 (under
  1 → "fewer than 1", 1–10 whole numbers, above 10 the nearest 5) with the 10th–90th percentile.
- **Value of supplies**: U_b(x) = ∫ₓ^∞ Λ_b(t) dt, so covering days x₀ to x₁ is worth U(x₀) −
  U(x₁) disruption-days a year (log-normal terms in closed form, E[(D − x)⁺]).
- **Consumption**: U_b(0) = Σ r·q·E[D] days a year.
- **Income** (research §3.5): each earner holds s = 1/earners of income; a spell of D weeks costs
  [s(1−ρ)·min(D, 26) + s·max(D − 26, 0)] / 4.345 months, ρ = 0.5 (prior); spells median 10 weeks,
  bad case 36 (prior, consistent with BLS). Streams: job loss (rr-hazards' household rate), job
  losses in a pandemic (0.11 per earner), the regional economy after Cascadia (0.3 per earner,
  spells median 26 weeks, bad case 78) or another named scenario (0.1 per earner). The death or
  disability of an earner is left to insurance (a sentence says so), not added to the savings
  target.
- **Readiness**: P_need = 1 − e^(−10·Σ r·q). `evacuate` adds the warning band (least and most
  warning among causes with at least 5 % of the rate; since v0.1.1 the short warning of a fast
  hazard, `FAST_WARNING_HAZARDS`: wildfire, flash floods under floods from rivers or heavy rain,
  tsunami and chemical releases, counts whenever it contributes at all, model review M-08:
  Lahaina read "as short as 2 hours" and now reads 6 minutes, the chemical-release band) and the
  typical days away (median of the time-away mixture, on the ladder). `get_home` gives each commuter's walk and water. Tier `h72`
  when P_need ≥ 2 % (prior), else `now`. `home_loss`: the ten-year displacement chance and the
  Household Pulse shares (most back within a month; 1 in 4 renters and
  1 in 10 owners never back).
- **Tier enough**: the smallest tier whose days cover the ladder target; income `m3`.
- **All needs together** (v0.1.1, model review M-04): each target is outlasted at about the dial's
  rate for its own need, so the chance that *at least one* need runs past its target is higher.
  `ConsequenceAssessment::joint_rate` counts each event class once, at the need it runs past most
  (co-monotone within an event, the well coupling's convention), and sums over classes. At the
  1-in-100 setting the ten-year chance is 26 to 39 in 100 for ten of the twelve fixture and
  backtest households (Philadelphia 36; the review's hand calculation gave 32) and about 20 in 100
  for Coos Bay and Miami, where one named scenario drives most targets at once. The packet, the
  CLI and the web app say "roughly 1 in 3" (the canonical dial sentence).

### Ranges

Every uncertain input is a named parameter: each hazard's rate (its 10th–90th range from
rr-hazards, split log-normal), each share (×/÷1.5 for priors, 1.2 for data), each duration scale
(×/÷2 for priors, 1.3 for data), county outage rates (×/÷1.3), the coupling rates. Draws use Latin
hypercube sampling: 256 standard-normal scores, shuffled per parameter by `SplitMix64` seeded from
a fixed constant and the parameter's name. So the ranges are deterministic, adding an unrelated
row never reshuffles another parameter's draws, a coupled term shares its source's draws, and the
10th and 90th percentiles move with the dial in the same direction as the target. `low`/`high`
are the 10th and 90th percentiles of the ladder target over the draws, widened to include the
central value. The one or two inputs that move the target most (one at a time, each at its 10th
and 90th percentile) are named in the bucket's last frequency sentence.

Timing of one `assess` call (`cargo run -p rr-consequence --example calibrate --release` for
native; a WebAssembly build with the workspace release profile under Node 24 on the same laptop,
with other builds running): the seven fixture households with rr-hazards' rates and scenarios
take 9–17 ms in WebAssembly (Miami, with the major-hurricane scenario, is the slowest); with all 35
hazards active, 9–14 ms native and 12–18 ms in WebAssembly; the research inputs 4–5 ms native and
5–6 ms in WebAssembly. The budget is about 40 ms; 400 draws took up to 27 ms, hence 256. A slow
phone may take two to three times as long.

### The cliff rule

A warning `cliff_<bucket>` ("Your answer depends mostly on one event: …") when one hazard or
scenario (1) supplies half or more of Λ at the design duration, (2) has a rate for that bucket
within a factor of 3 of the dial rate, and (3) makes the target jump between the chosen setting
and a neighbouring one: it grows at least as fast as the square of the dial's return period
(elasticity ln(t₂/t₁)/ln(rate₁/rate₂) ≥ 2) and by 3 days or more. The third condition keeps ordinary heavy
tails (chemical do-not-drink notices, a store shortage at 1-in-10) from being called cliffs; a
log-normal tail gives an elasticity near 1. The warning gives the targets at the chosen setting and
its two neighbours (1 in 50, 100 and 500 at the default; 1 in 10, 50 and 100 at 1 in 50) and, for a
scenario, says it can be turned off and that long tails are better met with capabilities
than stockpiles. The statement names each event once.

### Named scenarios

Rows keyed by scenario id (and `coast`/`valley` for Cascadia, from the candidate or the county's
`coastal` flag): `cascadia_m9` (Oregon Resilience Plan restoration times), `major_hurricane_direct_hit`
(ORNL Michael restoration; Asheville water), `new_madrid_m7` and `hayward_m7` (Hazus- and
Tohoku-based priors; no regional restoration study was at hand), `local_tsunami` (15–20 minutes'
warning; rr-hazards' rate already counts only households in the zone). The user's toggle in
`dials.scenario_overrides` wins. `ScenarioInfo.effect_summary` compares the ladder targets with and
without the scenario (Coos Bay: "Power: 3 days → 14 days; Tap water: 14 days → 60 days …") and, for
evacuation-only scenarios, the ten-year chance of leaving.

### Relief rating

For the event class that dominates each duration bucket's design event: its own rating when the
table has one (Cascadia coast: help in 14 days, the Oregon Resilience Plan's 1–2 weeks; power
mostly restored at 180 days, water 1,095 days on public water and 270 on a well; valley: help in 3
days); otherwise, when its duration comes from measured restoration records (EAGLE-I county curves,
ORNL hurricane restoration, county event records), help within the 72-hour standard (or sooner if
service is back sooner) and "mostly restored" when 90 % of that class's outages **that last at
least a day** are over (`RELIEF_FROM_DAYS`, v0.1.1). Over all outages, most of them an hour or
two, the 90th percentile described ordinary outages, not the design event: Asheville read "mostly
back in about half a day" beside a two-week target (model review M-13). Now Asheville's power is
mostly back in about 6 days, Philadelphia's in 6 (the hurricane curve: median 1.5 days, 5 days to
90 %), Utuado's in 11 beside a month; Cascadia's own rating is unchanged. A rating whose "mostly
restored" still comes out under a third of its target describes some smaller event, so it is
dropped (`None`, "not known"): no screen may print a relief time that short without naming the
event. `None` for expert-estimated durations.

### Sentences

Per duration bucket: the natural frequency of a disruption of a day or more (three days for
shopping and medicine, as in the research register), the share left facing a longer one at the
target, the drivers, and the coupling lines that apply. Readiness buckets: the ten-year chance, the
warning band and days away, each commuter's walk home, the average emergency-room visits. The
statement (research §3.7 style) is one sentence per entry for rr-plan: power and water, the well
coupling, the long-tail capability note when water is a month or more, food, everyday medicine (and
a cold plan for refrigerated medicine), heat or cold, the get-home trip, the go-bag, each cliff
event with its relief, and income ("Your biggest long disruption is losing a paycheck" when the
income target is the longest).

### Calibration against the research

Research households with rates that reproduce the prototype's event classes through the table's
shares (`tests/support/research.rs`), compared at the same dial rates. 48 of the 54 design
durations in research §8.4 and §9.4 are within 25 % (or 6 hours below a day); the six others are
explained and listed in `tests/calibration.rs` (Coos Bay income at 1 in 50, 95 and 500: the research gave the 55- and
58-year-old earners longer spells, and PlanInput has age bands, not ages; Philadelphia heat or cold
at 1 in 50, 95 and 500: more coupled classes than the prototype, county-wide winter outages and the
ice storm of record also stop the furnace). Natural frequencies match the research register
(Philadelphia power for 1, 3 and 7 days or more: 26, 9 and 3 in 100 in the research, 27, 9 and 3
here).

| Check | Research | Ours, raw | Ours on the ladder |
|---|---|---|---|
| Philadelphia power | 2.8 d | 2.8 d | **3 d** |
| Philadelphia no tap water | 2.8 d | 2.8 d | **3 d** |
| Philadelphia food | 9.6 d | 9.8 d | **10 d** |
| Philadelphia medicine | 12 d | 12.4 d | **14 d** |
| Philadelphia income | 3.8 mo | 3.8 mo | **4 mo** |
| Coos Bay power (Cascadia on) | 13 d | 13.5 d | **14 d** |
| Coos Bay well water (Cascadia on) | 50 d | 56.6 d | **60 d** |
| Coos Bay well water at `one_in_50` | 14 d | 14.5 d | **21 d** (3.4 % above the 14-day step, past the 3 % tolerance) |

Coos Bay water is about 57 d rather than 50 because the well coupling holds the water outage at least as
long as the power cut (the research's single "power plus damage" row, 90/270 d, sat below the
90/180-day power cut for the first three months). Without Cascadia, Coos Bay gives power 3.0 d,
food 9.3 d, medicine 10 d, water 13 d (research: about 3, 9, 9 and 14); with Cascadia at its
long-run recurrence (0.41 %/yr) the water target is 24 d (research 23). Two looser checks: the
diminishing-returns ratio of research §3.2 (days 0–3 against days 30–33) is 62× for Philadelphia
water (research about 60×) and 191× for food (190×), but 488× for Philadelphia power (about
1,600×: the major-hurricane and ice-storm-of-record rows keep more weight past a month) and 14× for
Coos Bay well water (19×).

**End to end with rr-hazards' rates** (`target/pipeline.md`, fixture county records): Philadelphia
at the default dial gets power 5 d, boil-water 7 d, no tap water 3 d, food 10 d, heat or cold 3 d,
medicine 14 d, phone 2 d, income 4 months. Power is 5 rather than 3 because rr-hazards' hurricane
rate for Philadelphia (0.055 a year, with the national one-third major share, as the fixture has no
HURDAT2 passages) is about twice the rate the research's classes imply (the real pack's passage
counts should lower it). The Coos Bay fixture (well, Cascadia on; its own dial is 1 in 500) at
1 in 100 gets power 14 d, water 90 d (its drought rate and ordinary 0.1 g earthquakes add to the
research's classes), food 21 d, medicine 30 d. Miami's targets (3 weeks of power, a month of
water) come from the major-hurricane scenario at about 0.1 a year; Hays, Kansas (well) gets 2
months of water from the well-drought prior. Each household's rates are listed in the report. *(Verification, 2026-09-26: this paragraph describes the earlier one-third major share. With the
share taken against tropical-storm passages, Philadelphia's power target is 3 days on the data
pack, as the research gives, and Miami's major-hurricane scenario runs at 0.037 a year: power 2
weeks, tap water 3 weeks.)*

### Decisions and open questions

1. **Decided by the planner** (2026-09-25): the dial rates above, the 3 % ladder tolerance, and
   `cliff_<bucket>` warning ids.
2. **Earner death or disability** feeds no bucket: savings cannot bridge a permanent loss, so the
   income bucket says life and disability insurance are the answer.
3. **Routine non-storm power outages** (equipment faults, 1–4 hours) are not in the hazard list and
   are left out; they never reach the half-day ladder step.
4. **Heat waves for homes without air conditioning** use a 3 d / 7 d spell length (prior) until the
   pack has Storm Events heat episode lengths (`county.events["heat_wave"]` with `median_days`).
   With every county episode counted and no county lengths, a home without air conditioning gets
   long targets from the log-normal tail: 3 weeks in Philadelphia (3.7 episodes a year), 2 weeks in
   Chicago, a month in Phoenix (17 a year), against 3 days with air conditioning.
5. **Gas stove** coverage waits for a catalogue id (`gas_stove` assumed until rr-content has one).
6. **Storm surge and evacuation zones** (v0.1.1): the packet leads with the decision to leave when
   the ten-year chance of leaving home is at least 25 in 100 or the plan includes a major
   hurricane or a local tsunami, and shelter advice follows the kind of home. Surge exposure
   itself is not in the data yet: the county-average hurricane and coastal-flood rates dilute a
   barrier island. Per-ZIP surge shares from NOAA NHC's National Storm Surge Hazard Maps (a flag,
   never the official zone) come in v0.2.0 (review S2, RR-P02).

### Citations

Every id this section's numbers carry is listed under "Used by consequence" in
`docs/CITATION_IDS.md` (ids already in `content/citations.toml`, two shared with hazards, two new
requests). Expert estimates cite `rr_risk_model_priors`.

## Supply sizing

Owned by `crates/rr-supply`. Entry points: `rr_supply::requirements(input, buckets)` (and
`requirements_with` with a `SupplyContext`: days a year at or above 95 °F, the county latitude,
the nuclear-plant flag) returns the `RequirementLine`s; `sized_requirements` adds each line's
kind, tier, days, per-day rate, formula, estimate tag and life-safety flag;
`ItemSizer::new(input, buckets, ctx).quantity(rule)` sizes any catalogue item by its
`quantity_rule`; `constants()` is the registry the expert view shows. Every rule, formula and unit
is in `docs/QUANTITY_RULES.md`.

### What the numbers are

Each bucket's design target (days, or the evacuation and readiness shapes) becomes quantities.
Every number comes from `crates/rr-supply/src/constants.toml` (about 150 values with sources, ranges,
alternatives and the research §13 disagreement notes), and each line cites exactly the sources of
the numbers it used plus the bucket target's own sources. Estimates cite `rr_expert_prior` and the
line says "some amounts are estimates".

| Need | Rule | Evidence |
| --- | --- | --- |
| Water, basic | 1 gal a person a day (survival 0.8, comfortable 4.0); drinking share doubled in a hot county (basic becomes 1.75); +0.29 pregnant or nursing, +0.25 formula baby; pets by weight | DATA (Ready.gov, CDC, Sphere, DRI); DERIVED heat split; PRIOR pet weights and the 30-day hot threshold |
| Water, long outages | Store up to 14 days; beyond that, treat water from a source (filter, bleach, boiling). One bottle of bleach whatever the target (at ½ mL a gallon it treats thousands of gallons), a fresh one every 6 months | DATA (BYU/Church 14 gal plus purification, Oregon, Washington; CDC bottle and dose); PRIOR the 6-month replacement |
| Boil-water notice | Make the drinking share safe; boil 1 minute (3 above 5,000 ft), bleach by strength (EPA) | DATA |
| Food | kcal by age band, DGA Table A2-2 moderately active, men and women averaged; +400 pregnancy or nursing; costs three ways (USDA Thrifty $8.44, bulk staples $2.15–2.85, freeze-dried $9–39 per 2,000 kcal); bulk staples for days beyond 30 (BYU 2019 list, Ensign child shares) | DATA; DERIVED band averages |
| Medication | Target clamped to 7–30 days (14 when there is none) for daily or refrigerated prescriptions; cold storage for the power target; antibiotics always 0 with the clinician card | DATA (Red Cross, CDC, Florida) |
| Power | CPAP 170 Wh a night; oxygen 300 W and other devices by watts; phones 15 Wh a day; generator 2.8 gal a day capped at the 25-gallon storage limit; December solar by latitude band | DATA (SIL, ENERGY STAR, fire code, PVWatts); PRIOR oxygen watts, phone Wh, band proxy |
| Sanitation | Twin-bucket toilet: 0.45 bags and 1 cup of cover a person-day; soap by person-month (Sphere); 2 cycles of period products for half the adults and teens (sex not asked) | DATA (RDPO, Oregon, Sphere, CDC); PRIOR bag and cover rates |
| Heat and cold | Fans only below 90 °F indoors and a cooling plan; for cold, a blanket and warm layers for everyone first (most homes have them), then a sleeping bag or extra heavy blanket only beyond 3 days of cold or for people 65 and over (an optional upgrade with a wood stove) | DATA (CDC, Ready.gov, Sphere); PRIOR the 3-day threshold |
| Readiness | Go-bags per person 4+ (a bag already owned will do) whose 3 days of water and food are staged from the household's own supplies, never bought twice (Red Cross); get-home bags sized to the walk (3 mph, 0.5 L an hour; 0.71 in heat, NIOSH) with water and snacks from home; first-aid kit per 4 people; alarms per level when missing; one extinguisher per floor people live on; an escape ladder only for floors 2–3 | DATA; PRIOR walking pace, hourly water, floor counts |

### Tiers

`tier_for_days(d)`: 0 is `now`; up to 3 days `h72`, 14 `w2`, 30 `m1`, 90 `m3`, 180 `m6`, then `y1`.
`tier_enough` gives that tier for duration buckets, `m3` for income (the savings track), `now` for
home loss, and `h72` for the other readiness buckets once the ten-year chance of need reaches 2 %
(PRIOR). `tier_recommended` is the highest over duration and readiness buckets, never below `h72`.
The first line that reaches the one-month tier says once that no agency sets a one-month amount.

### The reference households

At the research targets (Philadelphia: power 3 days, water 3 without and 4 to boil, food 10,
medication 14; Coos Bay: power 13, no-water 50, food 17, medication 21):

| Need | Philadelphia renters (4 people, a dog) | Coos Bay well owners (2 people, 2 dogs, a cat) |
| --- | --- | --- |
| Stored water | 12.9 gal (3 days) | 37.6 gal (the first 14 of 50 days) |
| Water to make safe | 13.3 gal over a 4-day boil notice | 96.8 gal over days 15–50, one filter and a source |
| Food | 82,000 kcal ($338 as groceries) | 76,900 kcal ($316) |
| Medication on hand | 14 person-days | 21 person-days |
| Generator fuel | none (no generator) | 25 gal stored (13 days needs 36.4) |
| Phone power | 140 Wh | 390 Wh |
| Toilet bags / cover | 6 bags, 12 cups | 45 bags, 100 cups |
| Bleach | 1 bottle (was 1) | 1 bottle (was 4) |
| Extinguishers / escape ladder | 2 / none (was 3 / 1) | owns an extinguisher / no ladder (was 1) |
| Cold bedding | 4 blankets and 4 sets of layers (assumed basics), 1 sleeping bag or extra heavy blanket for the senior (was 4 sleeping bags or blankets) | none (wood stove, no cold target) |
| Go-bag water and food | 12 gal and 24,600 kcal, staged from the stored water and food | 6 gal and 13,600 kcal, staged |
| Tier recommended | two weeks | three months |

### Decisions and known gaps

- **Pregnant or nursing** is one box, so water uses the larger breastfeeding figure (+0.29 gal)
  and food +400 kcal.
- **Hot climate** needs the county's days at or above 95 °F from the plan crate; without it the
  household is sized as temperate (Philadelphia's 3.3 days a year is temperate).
- **Formula** is detected from the word "formula" in a baby's dietary notes.
- **Rule names** follow the content workstream's requests; where a formula differs (water from the
  no-water target only, insulin counts as a prescription, masks from age 4, two cycles up to a
  month) `docs/QUANTITY_RULES.md` says why.
- **Heat and cold are separate cover** (`thermal_heat` and `thermal_cold` item classes), as are
  stored water and treatment (`water_stored`, `water_treatment_capacity`).
- **Readiness lines are emitted whatever the chance of need**; the allocator decides with value
  per dollar, and `tier_enough` applies the 2 % threshold.
- **Polish round (2026-09-26), from the first end-to-end plans:**
  - *Bleach* is one bottle per household whatever the target (CDC says to store a bottle; at
    ½ mL a gallon it treats thousands of gallons), replaced every six months because it weakens
    (an estimate: no agency gives a shelf life). A one-year target no longer buys 27 bottles.
  - *Staged supplies.* The go-bags' water and food, the pet go-kit's, and each commuter's water and
    snacks for the walk home come out of the household's stored water and food. They are
    alternative lines of the bag they go in (`evacuate.go_bag.alt.staged_water`,
    `get_home.get_home_bag.alt.staged_food.person_1`), never added to a purchase; their amounts
    follow their own standards (Red Cross three days, ASPCA a week) and say what to set aside.
  - *Fire.* One extinguisher per floor people live on (apartment 1, house 2, not the basement), the
    kitchen assumed to be on the floor with the way out; an escape ladder only when the household
    lives on floor 2 or 3. The form's floor stands for where people sleep, so a house at street
    level is not assumed to sleep upstairs: its escape plan asks for a second way out of any
    upstairs bedroom instead.
  - *Cold.* Blankets and warm layers first (content flags them as assumed basics); a sleeping bag
    or extra heavy blanket is a need only beyond a 3-day cold target (everyone aged 1 and over) or
    for people 65 and over (CDC: most at risk in the cold). A wood stove heats without power, so
    there it is an optional upgrade; a gas furnace is not assumed to work without power. Babies
    get a sleep sack, never loose bedding.
  - *Per-line citations.* Each line cites only the sources behind its own amount and words: the
    reused-bottles line at its cap no longer carries the pet-water sources, the boil-water line
    cites the drinking share alone, and lines the target's days do not size do not cite the
    target.
