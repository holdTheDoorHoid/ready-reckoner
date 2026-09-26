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
  in 10) or an order to stay inside. **Wildfire** events are safety shutoffs (85 %) or warnings to
  leave (15 %). **Drought** for a well household *is* the well running low.
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
| landslide | evacuate | slide | landslides closing roads | 0.1 | 5 d | 60 d | prior | warning 0–2 h | rr_risk_model_priors |
| landslide | home_loss | slide | landslides closing roads | 0.1 | — | — | prior |  | rr_risk_model_priors |
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
| wildfire | evacuate | threat | wildfires | 0.15 | 3 d | 30 d | prior | warning 0.25–12 h | rr_risk_model_priors |
| wildfire | power | shutoff | wildfire safety power shutoffs | 0.85 | 1 d | 3 d | prior | in heat 0.3 | rr_risk_model_priors |
| wildfire | supplies | smoke | wildfire smoke | 0.3 | 2 d | 7 d | prior |  | rr_risk_model_priors |
| wildfire | home_loss | threat | wildfires | 0.02 | — | — | prior |  | rr_risk_model_priors |
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
  log-normal fit is reproduced exactly and the tail follows the county's own shares. The rate is
  split between the storm hazards in proportion to their short-outage rates, for the
  contributions. Without records, county-wide outages are a third of short storm outages at 2 h /
  20 h (the Philadelphia fit, prior). Recorded in `overrides`; `ornl_eagle_i_outages` joins the
  power bucket's sources.
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
  `one_in_100` = −ln(0.9)/10 = **0.010536051565782628** a year (the research default, "90 % sure
  nothing in the next ten years is worse", shown as "about 1 in 100"), `one_in_500` = 0.002
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
  spells median 26 weeks, bad case 78) or another named scenario (0.1 per earner). The death or disability of an earner is left
  to insurance (a sentence says so), not added to the savings target.
- **Readiness**: P_need = 1 − e^(−10·Σ r·q). `evacuate` adds the warning band (least and most
  warning among causes with at least 5 % of the rate) and the typical days away (median of the
  time-away mixture, on the ladder). `get_home` gives each commuter's walk and water. Tier `h72`
  when P_need ≥ 2 % (prior), else `now`. `home_loss`: the ten-year displacement chance and the
  Household Pulse shares (a third back within a week, 12 % out over six months, 1 in 4 renters and
  1 in 10 owners never back).
- **Tier enough**: the smallest tier whose days cover the ladder target; income `m3`.

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

Timing (`cargo run -p rr-consequence --example calibrate --release`, and a WebAssembly build with
the workspace release profile under Node 24, same laptop): the seven fixtures with all 35 hazards
active take 9–13 ms native and 10–17 ms in WebAssembly; the research inputs 4 ms native and 5–6 ms
in WebAssembly. 400 draws took up to 27 ms in WebAssembly, hence 256.

### The cliff rule

A warning `cliff_<bucket>` ("Your answer depends mostly on one event: …") when one hazard or
scenario (1) supplies half or more of Λ at the design duration, (2) has a rate for that bucket
within a factor of 3 of the dial rate, and (3) makes the target jump between the chosen setting
and a neighbouring one: it grows at least as fast as the square of the dial's return period (elasticity
ln(t₂/t₁)/ln(rate₁/rate₂) ≥ 2) and by 3 days or more. The third condition keeps ordinary heavy
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
without the scenario ("Power: 5 days → 14 days; Tap water: 21 days → 60 days …") and, for
evacuation-only scenarios, the ten-year chance of leaving.

### Relief rating

For the event class that dominates each duration bucket's design event: its own rating when the
table has one (Cascadia coast: help in 14 days, the Oregon Resilience Plan's 1–2 weeks; power
mostly restored at 180 days, water 1,095 days on public water and 270 on a well; valley: help in 3
days); otherwise, when its duration comes from measured restoration records (EAGLE-I county curves,
ORNL hurricane restoration, county event records), help within the 72-hour standard (or sooner if
service is back sooner) and "mostly restored" at the time to 90 % restored. `None` for
expert-estimated durations.

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
explained and listed in
`tests/calibration.rs` (Coos Bay income at 1 in 50, 95 and 500: the research gave the 55- and
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

Coos Bay water is 57 d rather than 50 because the well coupling holds the water outage at least as
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
months of water from the well-drought prior. Each household's rates are listed in the report.

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
5. **Gas stove** coverage waits for a catalogue id (`gas_stove` assumed). // awaiting: rr-content

### Citations

Every id this section's numbers carry is listed under "Used by consequence" in
`docs/CITATION_IDS.md` (ids already in `content/citations.toml`, two shared with hazards, two new
requests). Expert estimates cite `rr_risk_model_priors`.
