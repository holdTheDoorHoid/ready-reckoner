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
location) -> HazardAssessment { profiles, rates, scenarios, notes }`.

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

**`events` keys** (episodes a year; the pack's Storm Events, SPC and HURDAT2 types, then hazard
ids as aliases): heat wave `heat`; cold wave `extreme_cold`; winter weather `winter_storm`; ice
storm `ice_storm`; windstorm `high_wind` + `severe_wind_day` (added); major-hurricane share
`major_hurricane_passage` ÷ `hurricane_passage`; boil-water notices `boil_water_notice`
(household rate, if a source is ever added). Hail, tornado and landslide stay on NRI's frequency,
because their footprint is NRI's loss ratio per NRI event.

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
`supply_chain_disruption_per_year`, `hazmat_release_per_year`.

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
| Hurricane | cuts the power or damages the home | NRI | Category 1–2: 0.5 (0.3–0.8); major: 0.8 (0.5–1.0); major share from HURDAT2 passages, else 1/3 (0.2–0.45) | none | PRIOR (DERIVED share where the pack has it) |
| Earthquake | shaking strong enough to knock things off shelves (0.1 g, about intensity VI) | USGS yearly chance → −ln(1 − p); else the 100-year intensity-VI chance; else NRI; ×/÷ 2 | 1 | none | DATA (hazard model) |
| Tsunami | a tsunami warning to leave the inundation zone | NRI | residents in the zone (NRI) × 0.3 of events bring a warning (0.1–0.6) | none | DERIVED + PRIOR |
| Inland flooding | flood water reaches the home, or cuts off an upper-floor flat | flood-zone odds (below) | s·p_in + (1 − s)·p_out | basement ×1.5 (1.2–2); second floor or higher ×0.5 (0.3–0.8) | DERIVED + PRIOR |
| Coastal flooding | coastal flood water reaches the home | present when NRI has it | residents in the coastal flood zone (NRI) × 1 %/yr (0.5–3 %) | as inland flooding | DERIVED + DATA |
| Landslide | damages the home or cuts off its road | NRI | exposed share × loss ratio ÷ 0.3 (0.1–0.6) × 10 homes cut off per home damaged (3–30); else 0.001 | none | DERIVED + PRIOR |
| Avalanche | reaches the home or road | NRI | exposed share | none | DERIVED |
| Volcanic activity | ash or mudflows reach the household | NRI | exposed share × 0.5 (0.2–1) | none | DERIVED + PRIOR |
| Wildfire | must leave home, or loses power in a wildfire safety shutoff | NRI burn probability ×/÷ 1.5 | exposed share × 20 households warned per home that burns (5–50); plus shutoffs 0.02/yr (0.005–0.1) in AZ, CA, CO, ID, MT, NM, NV, OR, UT, WA, WY | burn part urban ×0.3, rural ×2; shutoffs urban ×0.1, suburban ×0.5, rural ×1 | PRIOR |
| Drought | a private well runs low; or water limits that change daily life | NRI frequency ÷ national median 13.43, bounded ×0.25–×4 | — | well 1 %/yr (0.3–3 %, research §9); public water 0.2 %/yr (0.05–1 %) | PRIOR |

NRI and Storm Events frequencies carry a ×/÷ 1.5 and ×/÷ 1.3 spread; earthquake models ×/÷ 2.

**Inland flooding.** s is the share of homes in the Special Flood Hazard Area: the NFIP share when
the pack has it, else NRI's inland-flood exposed population ÷ population, else 0.05 (0.01–0.15).
p_in is NFIP claims per policy-year (bounded 0.3–10 %) or the flood-zone definition, at least
1 % a year (1–3 %). p_out, outside the zone, is 0.2 % a year (0.05–0.5 %) × the county's NRI
inland-flood frequency ÷ the national median 0.9643 (bounded ×0.5–×2). Philadelphia with a
basement comes to 0.6 % a year, in line with NRI's expected loss of about $270 a year on a
$300,000 home (data-sources §1.4) at a typical flood claim.

**The outage floor** (research §2.5, §7.5). When the county has EAGLE-I outage records, recorded
outages × 0.7 (0.6–0.9) caused by weather (informed by Do et al. 2023: 62.1 % of long county
outages coincided with extreme weather) must be explained by the storm hazards, each counted with
its chance of cutting power (windstorm 0.9, winter storm 0.3, ice storm 0.8, hurricane 0.9,
tornado 0.8, lightning 0.8, hail 0.1). Any shortfall ÷ 0.9 is added to windstorms, the most
common cause. NRI records only 0.027 windstorms a year for Coos County, but a Coos home is caught
in a county outage 1.09 times a year; the floor lifts Coos windstorms to 0.75 a year.

Natural hazards under 1 in 100,000 a year are left out of the register and named in a note.

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
| Nuclear attack (and EMP) | shown in the rare-catastrophe box | 1 in 2,000 to 1 in 400 a year worldwide (research §6.3, Forecasting Research Institute 2024); never a point estimate; the geometric middle, 1 in 894, is used only for arithmetic | none | PRIOR (published forecasts) |
| Terrorist attack | shown in the rare-catastrophe box: an attack that disrupts daily life where you live | 1 in 10,000 to 1 in 1,000 a year for a city household | setting as unrest | PRIOR |

### Personal hazards

| Hazard | One household event | Base rate | Household modifier | Evidence |
| --- | --- | --- | --- | --- |
| Job loss | a spell of unemployment for any earner | 0.083 per earner-year (BLS: 8.3 % of labour-force participants unemployed at some point in 2024); range 0.06–0.135, the top being the JOLTS 1.117 %/month as a yearly rate, −12·ln(1 − 0.01117) | × earners; stability: stable ×1 (typical salaried job), variable ×1.5 (1–2), seasonal ×1.75 (1.5–2), gig ×1.75 (1.5–2) | DATA + PRIOR (research §2.7, §3.5) |
| House fire | a reported fire in the home (or next door) | 344,600 fires ÷ 131,434,000 households = 0.262 % a year (0.2–0.35 %) | rowhouse or low-rise apartment ×2 (1.5–3; research §2.7), high-rise ×1.5 (1–2) | DERIVED + PRIOR |
| Medical emergency | an emergency department visit | 0.473 per person-year (NHAMCS 2022; 0.35–0.65) | × people; rural addresses get a note on slower ambulances (Mell 2017) | DATA |
| Stranded in a vehicle | a crash or breakdown away from home | 0.15 per vehicle-year (0.05–0.4); no vehicle: 0.03 per non-car commuter (0.01–0.1) | × vehicles | PRIOR (crashes: NHTSA 6.14 million a year) |
| Local water or gas outage | public water: a boil-water notice or a main break; well: a gas leak or local fault | boil notice 5 %/yr (2–10 %) + main break 10 %/yr (5–20 %); well 1 %/yr (0.3–3 %) | water source (the well pump's own failures are an `rr-consequence` coupling) | PRIOR (research §6.1, §8) |
| Break-in | a household burglary | 1 %/yr (0.5–2 %), to be replaced by the BJS victimization survey figure | none | PRIOR |
| Death or disability of an earner | loss of an earner's income | 0.9 % per earner-year (0.5–1.5 %): disability about 0.6 % (SSA: more than 1 in 4 20-year-olds disabled before 67) plus working-age death about 0.3 % | × earners | DERIVED + PRIOR |
| Long illness in the household | someone sick at home for weeks | 1 % per person-year (0.5–3 %) | × people | PRIOR |

Job loss and earner loss are left out (with a note) when no one is marked as earning; vehicle
stranding when there is no vehicle and no commute.

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

The earthquake card shows the county rate less the scenario's long-run share (never below a
quarter of it) plus the scenario's own rate: Coos Bay 0.01981 − 0.0041 + 0.01022 = 0.0259 a
year. The hurricane card shows the full hurricane rate (Category 1–2 plus major); the tsunami card
the county rate plus the local-source scenario. `rates` always carries the parent's full rate.

### Severity, confidence and sentences

**Severity** (0 to 1) is the loss from one household-significant event on a fixed log scale:
$50 or less is 0, $500,000 or more is 1. Natural hazards: NRI expected annual loss per household
÷ the county-average household rate, never below 0.1. Personal and societal hazards use a
per-event loss (PRIOR except house fire, $11.27 billion ÷ 344,600 fires = $32,700): job loss
$12,000; emergency visit $2,000; stranding $300; local outage $100; break-in $2,500; earner loss
$500,000; long illness $5,000; pandemic $5,000; regional blackout $1,000; cyber outage $300;
curfew $300; shortages $100; chemical release $500; nuclear plant accident $20,000. Nuclear attack
is 1 and terrorism 0.9 by definition. The fixed scale means a hazard's severity reads the same in
every county, and likelihood and severity stay separate columns in the rare-catastrophe box.

**Confidence**: data within a factor of 1.6 either way is `high`, within 3 `medium`, wider `low`;
a rate that rests partly on expert judgement is `medium` within a factor of 3, otherwise `low`;
one that rests only on expert judgement is `prior`. The rare catastrophes are `prior`.

**Sentences**: `100 · (1 − e^(−T·r))` households of 100 over `T = dials.horizon_years`, at most
two significant figures, whole numbers from 1 to 10, then "in 1,000" and "1 in N" for rarer
hazards, "nearly every household" from 99.5, with "(about N times a year)" when r ≥ 1. The range
is shown when the rate is an expert estimate. Around 2050 the horizon reads "in a ten-year
stretch around 2050". Rare catastrophes get a range-only sentence.

**Order**: ranked hazards by today's rate, most likely first (so the 2050 dial never reorders
the list), then the rare-catastrophe box.

**Why we think this**: `rr_hazards::why_we_think_this(hazard)` gives one plain-language reason
per hazard (the source, or for an expert estimate the reasoning and its size) for the drawer
behind each card. `HazardAssessment::notes` carries the plain caveats for this household and
county: county scale, missing outage records, capped heat days, the outage floor, hazards too
rare to list, the 2050 changes in words ("heat waves 2.4 to 3 times as often"), ignored scenario
settings, the rural ambulance note and the nuclear planning zone.

### The fixture registers

From `cargo run -p rr-hazards --example register -- <household> <fips>` on the hand-built fixture
counties in `crates/rr-hazards/tests/data` (NRI v1.20 and CMRA 2025 values; research §8–§9
outage rates). Top five by rate, per year and out of 100 households over ten years:

| # | Philadelphia renters (42101) | per year | of 100 | Coos Bay well owners (41011) | per year | of 100 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Heat wave | 3.69 | nearly all | Medical emergency | 0.95 | nearly all |
| 2 | Medical emergency | 1.89 | nearly all | Windstorm | 0.75 | nearly all |
| 3 | Cold wave | 0.40 | 98 | Stranded in a vehicle | 0.30 | 95 |
| 4 | Winter storm | 0.35 | 97 | Winter storm | 0.29 | 94 |
| 5 | Windstorm | 0.21 | 88 | Job loss | 0.25 | 92 |

Philadelphia further down: store shortages 0.20, job loss 0.166 (81 in 100, research §8.3 row 1),
flooding with the basement 0.0061, house fire 0.0052 (5 in 100), earthquake 0.0016. Coos Bay:
earthquake 0.026 (with `cascadia_m9` on by default at 1.02 %/yr), tsunami 0.0092 (with
`local_tsunami`), house fire 0.0026.

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
- **Missing event rows.** The pack writes no row for an event type a county never recorded. A
  missing `major_hurricane_passage` row is read as "unknown" (the national one-third major
  share), not "zero", which errs toward preparing; a recorded share near zero drops the
  major-hurricane scenario.
- **Income stability.** The v1 input has no "tenured or public" choice, so the research's ×0.5
  step cannot be selected; `stable` means the typical salaried job (×1), as in the Philadelphia
  example.
- **Figures to confirm** against the source when the citations are written: UCERF3's 33 % for
  the Hayward fault, USGS's 7–10 % for New Madrid, the SSA "more than 1 in 4" disability figure,
  the one-third major share of landfalling hurricanes, and the 1 % burglary prior.

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
| Water, long outages | Store up to 14 days; beyond that, treat water from a source (filter, bleach, boiling) | DATA (BYU/Church 14 gal plus purification, Oregon, Washington) |
| Boil-water notice | Make the drinking share safe; boil 1 minute (3 above 5,000 ft), bleach by strength (EPA) | DATA |
| Food | kcal by age band, DGA Table A2-2 moderately active, men and women averaged; +400 pregnancy or nursing; costs three ways (USDA Thrifty $8.44, bulk staples $2.15–2.85, freeze-dried $9–39 per 2,000 kcal); bulk staples for days beyond 30 (BYU 2019 list, Ensign child shares) | DATA; DERIVED band averages |
| Medication | Target clamped to 7–30 days (14 when there is none) for daily or refrigerated prescriptions; cold storage for the power target; antibiotics always 0 with the clinician card | DATA (Red Cross, CDC, Florida) |
| Power | CPAP 170 Wh a night; oxygen 300 W and other devices by watts; phones 15 Wh a day; generator 2.8 gal a day capped at the 25-gallon storage limit; December solar by latitude band | DATA (SIL, ENERGY STAR, fire code, PVWatts); PRIOR oxygen watts, phone Wh, band proxy |
| Sanitation | Twin-bucket toilet: 0.45 bags and 1 cup of cover a person-day; soap by person-month (Sphere); 2 cycles of period products for half the adults and teens (sex not asked) | DATA (RDPO, Oregon, Sphere, CDC); PRIOR bag and cover rates |
| Readiness | Go-bags per person 4+ with 3 days of water and food (Red Cross); get-home bags sized to the walk (3 mph, 0.5 L an hour; 0.71 in heat, NIOSH); first-aid kit per 4 people; alarms per level when missing | DATA; PRIOR walking pace and hourly water |

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
