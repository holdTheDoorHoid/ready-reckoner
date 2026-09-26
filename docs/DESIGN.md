# Ready Reckoner — Design

**Status:** authoritative. Founded 2026-09-25 from an interview with the owner. Sections marked
*(confirm against research)* are to be reconciled with `docs/research/*.md` and the derived documents
`docs/RISK_MODEL.md`, `docs/DATA_SOURCES.md`, `docs/CONTENT_STANDARDS.md` before Phase 2. Decisions
are appended to §14; nothing above §14 is changed silently.

## 1. Purpose

People who decide to "get prepared" reliably prepare for the wrong things: the dramatic, rare event
they saw on screen rather than the ice storm, the lost job, the boil-water notice or the pandemic
that will actually reach them. They spend on gear in the wrong order, burn out, and stop.

Ready Reckoner is the professional emergency manager they could not hire. Given where someone lives
and who is in their household, it:

1. computes a **cited, quantitative risk register** for that location and household;
2. converts it into **consequence targets**: how many days they should be able to go without grid
   power, safe tap water, leaving home, income, medical access or communications, at a stated
   confidence over a stated horizon;
3. sizes the **supplies and actions** that meet those targets, every quantity traceable to a source;
4. **allocates a real budget** month by month, cheapest risk reduction first, and says when enough
   is enough; and
5. hands over a **printable packet** and a **maintenance calendar**.

It is a static website that works offline. Nothing the user enters leaves their browser.

## 2. Decisions from the founding interview (2026-09-25)

| Topic | Decision |
| --- | --- |
| Form | Offline-capable website (PWA) on GitHub Pages; no server, no accounts, no analytics. Desktop wrap later with the same engine. |
| Geography | United States first, county resolution (ZIP accepted); data layer built for country packs later. |
| Audience | Beginners first: guided interview -> packet. Expert dials and every assumption exposed underneath. |
| Hazard universe | Natural (federal data) + societal (grid failure, pandemic, cyber outage, civil unrest, supply chain, hazmat, nuclear) + personal (job loss, house fire, medical emergency, vehicle stranding, local utility outage, burglary). |
| Core idea | Plan by **consequence bucket**, explain by hazard; hazard-specific add-on lists ride on top. |
| Security | Category included (locks, lighting, fire safety, neighbours, awareness). Firearms named as a personal and legal decision with safe-storage and training pointers; never budgeted. |
| Medical | Evidence-cited and prescriber-routed. Antibiotics explained (telehealth-prescriber route, cost, storage, when appropriate vs dangerous); never drug-by-condition dosing. |
| Stack | Rust engine -> WebAssembly; Svelte 5 + TypeScript web app; CLI prints the full packet for fixture households. Decided once; not to be relitigated. |
| Name | Ready Reckoner (`holdTheDoorHoid/ready-reckoner`). |
| Licences | GPLv3 for code; CC BY-SA 4.0 for guidance text and derived tables. |
| Products | Specs, what to look for, what to avoid, price bands. No brands, no store links. The user records what they bought and paid. |
| Session scope | Plan, publish, and start building with parallel Opus agents. |

Planner decisions taken with the owner's delegated design judgement:

- Budget input is monthly **and** one-off; the allocator plans month by month and can save toward a
  lumpy item.
- Climate horizon toggle: **today** vs **2050**, using regional multipliers from the Fifth National
  Climate Assessment / CMRA, labelled as projections.
- Output: interactive plan, printable packet (print stylesheet -> PDF), JSON export/import.
  Persistence is `localStorage` only.
- Location: ZIP or county via a bundled crosswalk; nothing leaves the browser by default; optional
  online lookups (e.g., a flood-zone point query) behind explicit per-use consent. Warn, don't block.
- Data freshness: a scheduled GitHub Action runs the Rust ETL quarterly and opens a pull request.
- Content: original text plus public-domain federal excerpts; every number carries a citation id.
- Social capital (neighbours, CERT, mutual aid) is a first-class plan item.
- UI framework: Svelte 5 (the app is forms and lists); mock engine first behind the same contract.

Additions from the prior-art and behavioural research (`docs/research/prior-art-and-psychology.md`,
2026-09-25):

- **Three water levels.** Official figures range from 2 to 15 L per person per day because they
  measure different things. The app shows survival (about 3 L), basic (about 4 L, one US gallon,
  the default) and comfortable (about 15 L, the Sphere domestic minimum), explains the difference,
  and lets the user choose (`Dials.water_level`).
- **Housing modifies duration.** Upper floors extend self-sufficiency targets for water and
  evacuation (Tokyo's stockpile guidance uses 7 days above the low floors versus 3), wells and septic
  tie water to power, mobile homes change the evacuate trigger. These are household modifiers in
  `rr-consequence`, cited or tagged `Prior`.
- **Two-tier relief rating** (from the Oregon Resilience Plan): for each bucket the app also shows
  "when outside help plausibly arrives" and "when service is mostly restored" for the design event,
  so "days on your own" has a story behind it (`BucketAssessment.relief`).
- **Stage and confidence, tracked locally.** One optional question at the start ("Where are you
  today?": haven't thought about it / thinking about it / have some things / have a plan / keeping it
  up) tailors copy to the stage; one confidence question (1–5) is asked at the start and again after
  the plan, so the app can show the change. FEMA's 2024 household survey found 60 % believe preparing
  helps but only 43 % feel able to do it; building that confidence is the product's actual job.
- **Drills and pre-commitments count.** If-then triggers ("if the county issues an evacuation
  warning for our zone, we leave within 30 minutes"), a go/stay card, and ten-minute drills are
  readiness items that count toward the `fire`, `evacuate` and `security` targets and appear in the
  maintenance calendar.
- **Calories and litres, never servings.** Food is planned in kcal (default 2,100 per adult-day) and
  water in litres/gallons. The UI shows cost per 2,000 kcal and per gallon for food and water items,
  because "30-day" retail kits were found to supply about 1,700 kcal a day.
- **Neighbours are scored.** The community items (know two neighbours' numbers, agree who checks on
  whom, join or form a block group / CERT) carry real risk-reduction weight in the allocator.
- **Firearms free action** states the safe-storage and suicide-risk evidence in the neutral form
  public-health bodies use, and nothing more.
- **Bundled, versioned data snapshot; no live federal endpoints at runtime.** The National Risk
  Index front end moved into another FEMA tool in 2025 and its Future Risk Index was withdrawn in
  February 2025; other consumer risk sites disappeared the same year.

## 3. The model in five sentences

Where you live gives each hazard a yearly chance; who you are changes what those hazards would do to
you. Each hazard that hits turns into a few plain consequences (no power, no water, stuck at home,
must leave, no income, no help, no phone) that last some number of days. Adding up all the hazards
tells us, for each consequence, how many days you should be ready for if you want to be covered in,
say, nine out of ten ten-year stretches. Those days, times your household, become quantities of
water, food, medicine, fuel and paperwork, each with its source. Your budget then buys the cheapest
risk reduction first, month by month, until each consequence is covered and the app tells you to stop.

## 4. Domain model

All identifiers are stable snake_case strings; the Rust enums in `rr-types` carry them and serialise
to them. Adding an identifier is an `ENGINE_API_VERSION` bump.

### 4.1 Inputs

```
PlanInput {
  planning_date: NaiveDate            # input, never the wall clock
  location: LocationInput             # zip or county_fips; setting
  housing: Housing
  people: [Person]
  pets: Pets
  mobility: Mobility                  # vehicles
  finances: Finances
  existing: [Owned]                   # baseline inventory {item_id, qty, paid_usd?, tested_on?}
  dials: Dials
  stage?: not_thought_about|thinking|have_some_things|have_a_plan|maintaining
  confidence_1to5?: u8                # asked at the start and again after the plan
  family_plan?: FamilyPlan            # v2: the household's own plan; echoed, never computed with
}
LocationInput { country: "US", zip?: "19147", county_fips?: "42101", setting: urban|suburban|rural }
Housing {
  kind: apartment_high_rise|apartment_low_rise|rowhouse|detached|mobile_home|rural_property
  tenure: own|rent
  floor: i8, basement: bool
  water: municipal|well, sewer: sewer|septic
  heating: gas|electric_resistance|heat_pump|oil|propane|wood|district|none
  cooling: central|window|none
  backup_power: none|power_station|generator|solar_battery
  alarms: { smoke: bool, co: bool, extinguisher: bool }
  below_grade_bedroom: bool                                    # v2: someone sleeps below street level
  cooking?: electric|gas|induction|none                        # v2: the main stove; absent = not asked
  raw_water_source?: none|well|surface_nearby|rain_barrel|neighbour_well   # v2: water to filter
  water_system_record?: fine|occasional_notices|frequent_problems|unknown  # v2: with SDWIS
}
Person {
  age_band: infant|toddler|child|teen|adult|senior     # <1, 1-3, 4-12, 13-17, 18-64, 65+
  pregnant_or_nursing: bool
  medical: { daily_rx: bool, refrigerated_rx: bool, powered_device: none|cpap|oxygen|other{watts},
             mobility: none|limited|wheelchair, dietary: [str], epinephrine: bool }
  earner: bool
  commute?: { distance_km: f32, mode: car|transit|walk|bike, remote_possible: bool }
  access_needs: [hearing|vision|limited_english|cognitive|supervision|service_animal|dialysis|home_health]
                                                        # v2: CMIST needs; defaults to []
}
Pets { dogs: u8, cats: u8, small: u8, large_animals: u8 }
Mobility { vehicles: [{ fuel: gas|diesel|hybrid|ev }] }
Finances {
  monthly_budget_usd: f32, one_off_budget_usd: f32
  emergency_fund_months: f32, monthly_expenses_usd?: f32
  income: { earners: u8, stability: very_stable|stable|variable|seasonal|gig }
  insurance: { home_or_renters: bool, flood: bool, earthquake: bool,
               sewer_backup?: bool, life_or_disability?: bool }          # v2: absent = not asked
  benefits: [federal_pay|snap_wic|ssi_ssdi|va|unemployment]              # v2: defaults to []
}
Dials {
  return_period: one_in_10|one_in_50|one_in_100|one_in_500     # default one_in_100; see §4.4
  climate: today|y2050
  horizon_years: u8 = 10                                       # for "in the next ten years" sentences
  water_level: survival|basic|comfortable                      # default basic (≈ 1 US gal/person/day)
  scenario_overrides: [{ id, on }]                             # named scenarios, e.g. cascadia_m9
  rare_catastrophic_opt_in: bool                               # v1 switch; means rare_opt_in = ["all"]
  rare_opt_in: [family id] | ["all"]                           # v2: rare allowance by family
  minimum_kit: bool                                            # v2: bare-minimum mode
  long_horizon: bool                                           # v2: show the long-horizon section
}
FamilyPlan {                          # v2; every field optional free text, trimmed and length-capped
  meeting_place_near, meeting_place_far, out_of_area_contact: {name, phone},
  school_pickup, work_plans, shelter_spot_home, shelter_spot_work, where_we_would_go,
  routes: [2], neighbours_who_check, who_takes_animals, shutoff_gas, shutoff_water, shutoff_electric,
  trusted_circle: [≤ 4 { name, phone, holds: [spare_key|documents|medical_poa|backup_codes] }],
  lawyer: {name, phone}, roadside_assistance, numbers_by_heart: [≤ 5]
}
```

Contract v2 (v0.2.0) adds the fields marked v2; every one is optional or defaults when absent, so a
saved v1 plan still loads. Absent means "not asked", and the engine then assumes nothing: no gas
range, no raw water source, an unknown water-system record, no benefits. `benefits` gates the
benefit-interruption hazard; `access_needs` drive the communication plan, registries and evacuation
help; `family_plan` is captured on a device-only screen and printed after the packet's summary and
on wallet cards (`docs/ENGINE-API.md` § Changes from v1).

### 4.2 Hazards

Natural: the 18 FEMA National Risk Index hazards, `avalanche`, `coastal_flooding`, `cold_wave`,
`drought`, `earthquake`, `hail`, `heat_wave`, `hurricane`, `ice_storm`, `landslide`, `lightning`,
`riverine_flooding` (NRI v1.20 "inland flooding"), `strong_wind`, `tornado`, `tsunami`,
`volcanic_activity`, `wildfire`, `winter_weather`; and, since v0.2.0, five the Index does not cover:
`wildfire_smoke` (county smoke days), `dust_storm` (Storm Events), `sinkhole` (karst share),
`geomagnetic_storm` and `vei7_eruption` (rare).

Societal: `pandemic`, `grid_failure` (regional, multi-day, not weather-caused), `cyber_outage`
(utility, payments, pharmacy/insurer IT), `civil_unrest`, `supply_chain_disruption` (store shortages,
pre-storm runs), `hazmat_release`, `nuclear_plant_incident`, `nuclear_attack` (a family whose
sub-causes include the EMP of a high-altitude burst); since v0.2.0 `dam_failure` (dams and levees),
`network_outage` (phones and internet), `drug_shortage`, `benefit_interruption` (only for households
with a benefit), `attack_disruption` (an attack or threat closes your area), and the rare
`multi_month_blackout`, `war_infrastructure`, `cbrn_attack`, `severe_pandemic`, `financial_crisis`,
`mass_violence`. `terrorism` is retired in v0.2.0: it still parses so saved plans load, but it is
never emitted; its disruption half is `attack_disruption` and its personal-safety half
`mass_violence`.

Personal: `job_loss`, `house_fire`, `medical_emergency`, `vehicle_stranding`, `local_utility_outage`
(water main, boil-water notice, gas leak), `burglary`, `earner_death_or_disability`,
`extended_household_illness`; since v0.2.0 `water_damage` (burst pipes and leaks), `eviction`
(renters) and `arrest_or_detention` ("A household member is arrested or detained"; it counts
arrests, never guilt).

**Rare families.** Nine hazards are rare catastrophes shown as families in their own box:
`nuclear_attack`, `geomagnetic_storm`, `multi_month_blackout`, `war_infrastructure`, `cbrn_attack`,
`severe_pandemic`, `vei7_eruption`, `financial_crisis`, `mass_violence`. A family's id is its
hazard's id (`HazardId::family`); everything else inside it (the nuclear family's limited strike,
city device, use abroad and EMP; Yellowstone; the kinds of CBRN attack) is a sub-cause on the
profile, not an id. Each row shows its likelihood as a range only with one anchor from the
household's own list, "if it reaches you" in zone-conditional words, "why here" from a location
factor (for the nuclear family, the county's strategic-exposure class A–E), and "what it changes in
your plan", usually nothing beyond the basics. Rows are sorted by how likely here, never by expected
loss, and never drive the budget by default (§4.7). Candidates rarer than 1 in 100,000 a year go to
one "Also checked" line.

Named scenarios (decided per location by the engine, toggleable by the user): `cascadia_m9`,
`new_madrid_m7`, `hayward_m7`, `local_tsunami`, `major_hurricane_direct_hit`. A scenario is a hazard
whose single rate sits close to the dial and would otherwise make targets jump (§4.4).

For each hazard the engine computes a **household event rate**

    r_h = λ_h · a_h · m_h

where λ_h is the county event rate (NRI annualised frequency for natural hazards, with per-hazard
semantics from `data/core/nri_semantics.toml`; national base rates for personal and societal
hazards), a_h is the footprint fraction (the chance this household is affected given a county event:
derived from outage peak fractions for power, flood-zone share for floods, 1 for personal events,
`Prior` otherwise), and m_h is the household modifier (§4.3). Every factor carries provenance and a
range.

```
HazardProfile {
  id, name, tier: natural|societal|personal, display: ranked|rare_catastrophic,
  rate_per_year: f64, rate_range: [lo, hi], annual_probability: f64, probability_range: [lo, hi],
  severity: 0..1, eal_per_household_usd?: f64, climate_multiplier: f64,
  confidence: high|medium|low|prior, sources: [CitationId], frequency_sentence, buckets: [BucketId],
  family?, sub_causes: [{ id, name, note, rate_range?, sources }],          # v2
  location_factor?: { class, label, multiplier: [lo, mid, hi], sources },  # v2
  range_only: bool, anchor_sentence?, if_it_reaches_you?, what_it_changes? # v2
}
```

The nine rare families are `rare_catastrophic`: shown in their own box with likelihood (a range
only) and severity as separate columns, never ranked by expected loss (a tiny probability times a
huge loss would otherwise dominate the register).

### 4.3 Consequence buckets

Three kinds. Duration buckets are sized in days; readiness buckets are capabilities you either have or
don't; money buckets are months of income gap or an insurance decision on a separate savings track.
There are 15: `clean_air`, the 15th, joined the readiness buckets in v0.2.0 (owner decision
2026-09-26).

| id | Plain name | Kind | Typical drivers |
| --- | --- | --- | --- |
| `power` | No grid power at home | days | storms, ice, heat, grid failure, wildfire shutoffs |
| `water_boil` | Tap water must be treated (boil notice; taps still run) | days | main breaks, floods, storms |
| `water_out` | No tap water at all (pressure loss, do-not-use, quake) | days | earthquakes, freezes, hurricanes, wells without power |
| `supplies` | Can't get to a store (can't go out, or shelves are empty) | days | winter storms, pandemic, curfews, pre-storm runs |
| `thermal` | Dangerous heat or cold indoors | days | heat wave, cold wave, an outage in season |
| `medication` | Medication and medical-supply continuity | days | any large event; pharmacy IT outage; pandemic |
| `comms` | No phone, internet or card payments | days | storms, grid, cyber |
| `evacuate` | Must leave home quickly | readiness: 10-year need, notice time, days away | wildfire, hurricane, flood, hazmat, tsunami, house fire |
| `get_home` | Stranded away from home | readiness: 10-year need, distance | commute distance and mode |
| `medical_emergency` | Medical emergency when help is slow | readiness | injury, illness; rural response times |
| `fire` | House fire | readiness | cooking, heating, electrical; attached housing |
| `security` | Home and personal security | readiness | burglary, unrest, post-disaster opportunism |
| `clean_air` | Unhealthy air indoors (smoke, dust, ash) | readiness: a checklist (respirators, an air cleaner or DIY filter box, a sealed-room plan) and the smoke or dust days a year | wildfire smoke, dust storms, ash |
| `income` | Loss of income | months of gap after unemployment insurance | job loss, pandemic, disability, displacement |
| `home_loss` | Home damaged or uninhabitable | money + readiness (insurance, documents) | fire, flood, wind, quake |

Each hazard maps to buckets with a conditional probability and a duration distribution:

```
Effect { hazard, bucket, p_given_event: f64,
         duration: LogNormal{median_days, p90_days} | Fixed{days},   # σ = ln(p90/median)/1.2816
         evidence: empirical|prior, sources: [CitationId] }
```

Empirical durations come from outage restoration statistics (EAGLE-I county curves: median = time to
50 % restored, bad case = time to 90 % restored), boil-water-notice records (Texas, Kentucky), Hazus
restoration tables, the Oregon Resilience Plan, displacement surveys and 2020 stay-home durations
(median 45 days). The rest are `Prior`, labelled as such, and the app shows "why we think this" with
a user override. `docs/research/risk-model.md` §2.4 holds the default table.

**Coupling rules** (household modifiers, each one line, each explainable): a private well with an
electric pump makes every power event a `water_out` event; a high-rise above the booster-pump floors
does the same and ties mobility to elevators; a gas furnace needs power for `thermal` cold; a gas
stove covers `water_boil` while gas flows; a wood stove with fuel covers `thermal` cold; refrigerated
medication inherits power events longer than about a day; a powered medical device triples the harm
weight of `power`; age 65+ raises the `thermal` harm weight; infants raise water need by half and add
a formula line; renters cannot install generators and face more permanent displacement; attached
housing doubles neighbour-fire exposure; no vehicle changes evacuation mode and lead time; commute
distance sizes the get-home bag (walking at about 3 mph); income stability scales job-loss incidence
(×0.5 tenured/public, ×1 typical, ×1.5–2 gig/seasonal); rural addresses raise the value of first-aid
capability.

### 4.4 The exceedance curve and the design event

For bucket *b* the central object is

    Λ_b(d) = Σ_h r_h · q_{h,b} · S_{h,b}(d)

the expected number of times per year this household faces a *b*-disruption longer than *d* days.
Because one event feeds several buckets, correlation between buckets is preserved automatically. From
this one curve:

1. **Target.** The dial is a return period; the target is the smallest ladder value *d\** with
   Λ_b(d\*) ≤ Λ\*, where the default `one_in_100` is defined, for any one need, as "90 % sure nothing in the
   next ten years is worse than its target" (across all needs together the chance that at least one
   runs out is higher, roughly 1 in 3; round-2 model review M-04): Λ\* = −ln(0.9)/10 ≈ 0.01054 per year (about 1-in-95, shown as "about 1-in-100"),
   the same one-percent-a-year yardstick behind FEMA flood maps. The other settings are exactly 0.10,
   0.02 and 0.002. A raw target within 3 % above a ladder step counts as that step (3.003 days is
   "3 days", not "5"). In ordinary counties it reproduces official guidance (Philadelphia: about 3 days of
   power and water, 10 days of food, 2 weeks of medication).

   | Dial | Label | Annual rate | Chance in 10 years |
   | --- | --- | --- | --- |
   | one_in_10 | Common disruptions | 0.10 | 65 % |
   | one_in_50 | Serious | 0.02 | 18 % |
   | one_in_100 (default) | Very serious | 0.01 | 10 % |
   | one_in_500 | Rare catastrophes | 0.002 | 2 % |

2. **Natural frequencies.** Of 100 households like yours, `100 · (1 − exp(−T · Λ_b(d)))` will face a
   disruption longer than *d* in the next *T* years.
3. **Value of the x-th day of supplies** equals Λ_b(x): the marginal value of one more day is the
   annual rate of events that outlast it. Diminishing returns fall out of the mathematics (for
   Philadelphia water the first three days cover roughly 60 times more expected disruption-days than
   days 30–33, and for power hundreds of times; for Coos Bay well water only 10–20 times, which is why
   the coast stores deeper; the exact ratios depend on the priors and are recomputed by the engine).
4. **Consumption** (for rotation and cost): Σ_h r_h q_{h,b} E[D] days per year.

**Ranges.** Every rate and duration carries a stated uncertainty; the engine propagates it (seeded,
deterministic; either an outer loop of parameter draws or one-at-a-time sensitivity) and reports the
target as "about 3 days (2–5)", rounded to the ladder ½, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90,
180, 365. The one or two parameters that drive the range are named.

**The cliff rule.** When a single hazard's rate is within about a factor of three of the dial rate,
targets jump between dial settings (Coos Bay well water: 14 days at one-in-50, 50 at one-in-100, 196
at one-in-500). Then the engine: says so plainly ("your answer depends mostly on one event: a Cascadia
earthquake"); exposes it as a **named scenario** (on by default where state guidance addresses it, as
Oregon's two-weeks-minimum does); shows the plan with and without it; and prefers capabilities over
stockpiles for the long tail (a filter plus a raw water source rather than 120 gallons).

**Readiness buckets** use the 10-year need probability P = 1 − exp(−10 · Σ r): include the capability
when P ≥ 2 % (`Prior` default) or when its value per dollar beats the current tier's best item.
Notice time (minutes for tsunami and fire, hours for flash flood, days for hurricane) decides what
the bag holds and where it lives.

**Money buckets.** Income: each earner's spell rate (about 0.083 per year, scaled by stability) and a
spell length distribution (median 10 weeks, bad case 36) net of unemployment insurance give
Λ_income(months); the target is months of gap at the dial, reported as a savings goal on a separate
track. Home loss: an insurance-and-documents decision plus a displacement-cost estimate, driven by the
NRI loss ratio, flood-zone share, fire rate and displacement surveys. No stockpile target.

**Two-tier relief rating.** For each duration bucket the engine also reports when outside help
plausibly arrives and when service is mostly restored for the design event (Oregon Resilience Plan
style), so "days on your own" has a story behind it.

### 4.5 Tiers

| id | Plain name | Days | Enters the plan when |
| --- | --- | --- | --- |
| `now` | Free actions | 0 | always, first |
| `h72` | Three days | 3 | always |
| `w2` | Two weeks | 14 | any duration target > 3 days (almost everyone) |
| `m1` | One month | 30 | any duration target > 14 days |
| `m3` | Three months | 90 | any duration target > 30 days; mostly the money track |
| `m6` | Six months | 180 | any duration target > 90 days (named scenarios at cautious dials) |
| `y1` | One year | 365 | only when a target exceeds 180 days or the user asks |

No authority defines a one-month tier (Ready.gov says "several days", the Red Cross two weeks at home,
Oregon and Washington two weeks, Germany ten days, the Church three months); `m1` is an interpolation
and the plan says so once. The get-home bag is an item in the `get_home` bucket, unlocked right after
the three-day basics. The
tier a household should reach is the maximum over buckets of the tier that covers each target; the
plan says, per bucket, which tier is enough and stops there.

### 4.6 Items and requirements

`rr-supply` turns bucket targets and the household into **requirement lines**: (bucket, item
class, quantity, unit, citations). `rr-content` holds the **item catalogue**; `rr-budget` covers
requirement lines with catalogue items.

Catalogue entry (`content/items/*.toml`):

```toml
[[item]]
id = "water_stored"
name = "Stored drinking water"
category = "water"
unit = "gallon"
buckets = ["water_out", "water_boil"]
tier = "h72"
free = false
spec = "Commercially bottled water, or tap water in clean food-grade containers with tight lids, kept cool and dark."
look_for = ["Food-grade (HDPE #2 or PET #1) containers", "Sealed bottled water with a date"]
avoid = ["Milk jugs (they leak and grow bacteria)", "Containers that held chemicals"]
price_band_usd = { low = 0.0, high = 1.50, per = "gallon", note = "reused bottles are free; bottled water about $1 per gallon" }
quantity_rule = "water_gallons"         # implemented in rr-supply
volume_l_per_unit = 3.785
maintenance = { rotate_months = 6 }
citations = ["ready_gov_water", "cdc_water_storage"]
hazard_extras = []
```

Citation entry (`content/citations.toml`):

```toml
[[citation]]
id = "ready_gov_water"
title = "Water"
publisher = "FEMA / Ready.gov"
year = 2024
url = "https://www.ready.gov/water"
retrieved = "2026-09-25"
quote = "Store at least one gallon of water per person per day for several days, for drinking and sanitation."
license = "US Government Work (public domain)"
```

Guidance blocks (`content/guidance/<bucket|hazard|tier|topic>.md`) are short Markdown with front
matter (`id`, `applies_to`, `citations`). The packet is assembled from them.

### 4.7 Budget allocation

The value of an item that moves bucket *b*'s coverage from *x* to *x + Δ* days, capped at the
target, over a ten-year horizon is

    V = 10 · w_b · ∫_x^{min(x+Δ, target)} Λ_b(t) dt

expected weighted disruption-days covered per decade. Readiness items use
V = 10 · w · r_need · (day-equivalents of harm avoided). Harm weights *w_b* (`Prior`, documented,
shown in the expert view): water and anyone's daily prescription medication 3; thermal with a
vulnerable member 2; evacuation 2; supplies, power, communications 1; a powered medical device triples
`power`. Because Λ falls with *d*,
the first day of a bucket is always worth more than the fourteenth and cheap early coverage floats to
the top without special cases. Baseline inventory sets the starting *x*.

The allocator, each month: (1) month 0 applies every free action (documents, plan, contacts, alarm
tests, refill-at-seven rule, water-heater reserve, neighbours) and updates coverage; (2) walks the
tiers in order with each bucket's target capped at the tier horizon; (3) orders candidates life-safety
first, then value per dollar, and buys the best affordable one; (4) **promotes** a cheap later-tier
item whose value per dollar is at least five times the current tier's best (an extra week of a
dependent's medication); (5) runs a **split schedule**: when the top-priority item costs more than a month's money, half of
each month's new money is reserved toward it and the rest buys the best affordable items, so a small
budget shows progress every month and an expensive life-safety item (a CPAP battery) still arrives in
bounded time; a fully monotone fixed-order schedule and the research shortcut schedule remain as
options; (6) stops when no tier has a
positive-value candidate and reports "you are done for your risk; here is the maintenance calendar",
routing any surplus to the income savings track or suggesting a more cautious dial. Recorded actual
prices replace band midpoints. A simultaneous-need check runs on the shared event list so a major
hurricane is covered for power, water and food at the same time (this matters for storage space, not
money).

**Rare catastrophic hazards** get a budget cap: specialised items default to $0 and an opt-in allows
at most 10 % of the monthly budget; the packet shows that the three-day and two-week supplies already
cover the official "get inside, stay inside, stay tuned" sheltering phase.

Guardrails (warn, never block): zero budget; a powered medical device with no power plan by month
three; refrigerated medication with no cooling plan; no water at all after month one; an
evacuation-heavy profile with no go-bag; insurance gaps for owners in flood or quake zones; a cliff
(one scenario dominates the answer).

### 4.8 Outputs

```
PlanOutput {
  engine_version, api_version, data_pack_version, content_version,
  location: LocationResolved           # v2: + exposure (each value with its source, for "Why here")
  register: [HazardProfile]            # ranked, with rare_catastrophic items flagged for their own box
  buckets: [BucketAssessment]          # target (with range), covered, tier_enough, contributions, relief,
                                       # sentences; v2: + stress_test (the worst event in the region's record)
  scenarios: [ScenarioInfo]            # named scenarios that apply here, on/off, effect summary
  tier_reached, tier_recommended,
  plan: { months: [{ index, budget, items: [PlanItem] }], done_month?, envelopes: [Envelope],
          savings_track?: { target_months, target_usd, current_months, monthly_suggestion_usd, why },
          first_milestone?: { months, usd, by_month },           # v2
          minimum_kit: bool, long_horizon: [PlanItem] },         # v2
  requirements: [RequirementLine],
  warnings: [Warning],                 # v2 ids: surge_zone_stay_home, cold_chain_power, benefit_lapse,
                                       # plan_too_long, no_raw_water_source, no_cooking_capability
  packet_markdown: String,             # the printable packet, sections in §9
  provenance: [Citation],              # everything referenced above
  recovery: { county_declarations_5yr?, sources }   # v2: facts for the recovery page
}
PlanItem gains requires: [ItemId] and decision: bool; Item gains requires, readiness_share?,
decision, long_horizon, season?, test_interval_months? (v2); EngineInfo gains validation (the
backtest summary for the public validation page).
```

Every v2 output field is left out of the JSON when empty, so v1 outputs read back unchanged;
`docs/ENGINE-API.md` (contract v2) is the field-by-field reference.

## 5. Engine pipeline and crates

```
PlanInput ─► rr-hazards ─► [HazardProfile] ─► rr-consequence ─► [BucketAssessment]
                                                    │
                    rr-content (catalogue, citations, guidance)      │
                                                    ▼
                          rr-supply ─► [RequirementLine] ─► rr-budget ─► Plan
                                                                          │
                                      rr-plan (orchestration, packet) ◄───┘
                                          │                 │
                                      rr-wasm            rr-cli
```

| Crate | Owns | Depends on |
| --- | --- | --- |
| `rr-types` | Every shared type above, ids, `ENGINE_API_VERSION`, seeded RNG (unused unless needed), error type | serde |
| `rr-data` | Pack formats, loaders, ZIP -> county, county record lookup, climate multipliers, base rates | rr-types |
| `rr-hazards` | Location + dials -> HazardProfile per hazard | rr-types, rr-data |
| `rr-consequence` | Effects table, design-event math, frequency sentences | rr-types |
| `rr-supply` | Quantity rules, requirement lines, tier logic | rr-types, rr-content |
| `rr-content` | Catalogue + citations + guidance loading and validation (embedded at build) | rr-types |
| `rr-budget` | Risk function, allocator, guardrails | rr-types, rr-supply, rr-content |
| `rr-plan` | Pipeline, PlanOutput, packet Markdown | all of the above |
| `rr-wasm` | `wasm-bindgen` surface per `docs/ENGINE-API.md` | rr-plan |
| `rr-cli` | `plan`, `risks`, `targets --sweep`, `explain`, `catalogue`, `citations --missing`, `county search/show`, `data verify/info`, `golden [--update]`, `doctor` (`docs/CLI.md`) | rr-plan, rr-data (native) |
| `rr-etl` | Downloads sources, builds packs, writes manifest (native, reqwest); `verify` runs `rr_data::verify` | rr-types, rr-data |

Hard rules (also in `CLAUDE.md`): deterministic; no wall clock; no OS entropy; no `rand`; all engine
crates compile for `wasm32-unknown-unknown`; `#![forbid(unsafe_code)]`.

## 6. Data layer

Grounded in `docs/research/data-sources.md` (2026-09-25, sizes measured). Packs are versioned,
checksummed files under `data/`, produced only by `rr-etl`, described in `data/manifest.json`
(source URL as fetched, version label, retrieval timestamp, sha256, licence and obligations, rows).
Raw downloads are never committed. Output is deterministic so a refresh with unchanged inputs is a
byte-identical pack.

| Pack | Contents | Load | Size target |
| --- | --- | --- | --- |
| `core` | NRI v1.20 counties trimmed to the model's fields (3,232 rows incl. Connecticut's 9 planning regions); ZIP -> county shares (33,791 ZCTAs); county outage statistics derived from ORNL EAGLE-I 2014–2025; per-county event rates from HURDAT2, SPC and NOAA Storm Events; USGS seismic exceedance at county centroids; NCA5 Atlas / CMRA climate ratios; NFIP flood-zone share and claims; nuclear-site distance, TRI and high-hazard-dam counts; Census Community Resilience Estimates and CDC SVI; national base rates with sources | at start | ≤ 5 MB gzipped (research estimate 3.5–4.5) |
| `geo` | County boundaries (Census 2024 cartographic 1:20m, 3-decimal coordinates) for the map thumbnail and click-to-select | lazy | ≤ 0.35 MB gzipped |
| `tract` | Census-tract NRI per state (median 0.3 MB, California 2 MB) | lazy, later | 23 MB for all states |

Rules learned from the research:

- **The National Risk Index has terms of use beyond public domain.** They forbid reverse
  engineering or deriving the underlying datasets, require a citation naming the dataset version and
  access date plus a "uses NRI data but is not endorsed by FEMA" statement, forbid presenting
  modified data as FEMA's, and let FEMA rescind use. We comply: the app ships only the trimmed
  per-county fields the model needs (never the raw tables), shows the disclaimer with version and date
  on the About screen and in the packet's sources, and states plainly which numbers are ours. This was
  flagged to the owner on 2026-09-25.
- **NRI v1.20 changed meaning.** Riverine flooding became inland flooding (`IFLD`), social
  vulnerability now comes from Census Community Resilience Estimates, and `AFREQ` is an event count
  for some hazards and a probability for wildfire. Per-hazard semantics ship in
  `data/core/nri_semantics.toml`; `rr-hazards` never guesses.
- **ZIP codes are ambiguous**: 30 % of ZCTAs span more than one county. When no county holds 80 % of
  a ZIP the engine returns `ambiguous_zip` with the candidates and the UI asks.
- **Connecticut** uses planning-region FIPS (091xx) in NRI and Census 2024 but old counties (090xx) in
  the ZIP relationship file and CMRA; the ETL applies a crosswalk so every pack joins.
- **No live federal endpoints at runtime.** fema.gov blocks scripted clients, the Census API needs a
  key, several sources were withdrawn in 2025. Everything is precomputed; the only optional online
  lookups (flood zone by snapped grid cell, NWS alerts by county) are consented, and none is needed
  for the plan.
- **Do not bundle** GEM (non-commercial share-alike), rmpmap (share-alike), First Street or
  PowerOutage.us. Credit lines are required for EAGLE-I and the NCA5 Atlas (CC BY 4.0); the app's
  About screen lists every attribution from `EngineInfo.attributions`.

Refresh: `.github/workflows/data-refresh.yml` runs quarterly and opens a PR; `rr-etl verify` checks
checksums, row counts and that every county FIPS joins across packs; the About screen shows pack
versions and dates so a stale snapshot is obvious.

## 7. Content layer

`content/` is CC BY-SA 4.0. It holds `items/*.toml`, `citations.toml`, `guidance/*.md`, and
`glossary.toml`. `rr-content` embeds and validates it at build time: every item cites, every
citation has a URL and licence, every `quantity_rule` exists in `rr-supply`, every guidance block's
`applies_to` resolves. Standards for writing it are in `docs/CONTENT_STANDARDS.md`.

Sensitive-content policy is in `docs/PRINCIPLES.md` §9 and is enforced by the content validator where
it can be (no brand tokens from a denylist, no dosing patterns, no firearm items with a price band).

## 8. Web application

Svelte 5 + Vite + TypeScript, see `docs/UI.md`. The engine is loaded as WebAssembly behind the
contract in `docs/ENGINE-API.md`; `web/src/engine/mock.ts` implements the same contract with
plausible fixed data so the interface can be built and tested before the engine lands, and parity
tests compare mock and wasm shapes. State: `localStorage` key `rr.plan.v1`; export/import JSON.
Service worker for offline; installable. Print stylesheet renders `packet_markdown` as the packet.
No third-party scripts, fonts or analytics.

## 9. The packet

1. Summary page: who this is for, date, tier reached and tier recommended, the three sentences that
   matter most.
2. Your risks: ranked hazard cards with natural-frequency sentences and a county map.
3. Your targets: the buckets with days, the dial settings, and what drives each.
4. Your plan: this month, next month, and the full phased list with prices and "why".
5. Checklists per tier (three days, get-home bag per commuter, two weeks, one month...).
6. Family plan: meeting places, out-of-area contact, school and work plans, evacuation routes and
   triggers, pets.
7. Documents and money: the Emergency Financial First Aid Kit list, insurance questions, cash.
8. Special needs: medication continuity, powered devices, infants, seniors, disability, pets.
9. Maintenance calendar: rotation, checks, drills, annual review.
10. Sources: every citation used, with URLs and retrieval dates.

## 10. Privacy and security

Static site; no server, no accounts, no analytics, no third-party requests. Engine has no network
access. Household data is in `localStorage` and in files the user exports. A "forget everything"
control clears storage. The content security policy forbids inline scripts and external origins
except for the optional, consented lookups. The repo publishes a threat model note in
`docs/PRIVACY.md` (Phase 3).

**Open decision (raised 2026-09-26 by the web workstream): the hosting origin.** Browser storage
belongs to the whole origin, and `holdthedoorhoid.github.io` is shared by every project the owner
publishes there, so any of those pages could read a household's saved plan. Before real use the site
should live on its own origin: a dedicated GitHub organisation (free; `<org>.github.io`) or a custom
domain. Until then the site carries the "sample numbers" banner and the About screen states the
limitation.

## 11. Determinism and testing

- Fixture households in `fixtures/households/*.json` cover: a renting family of four in
  Philadelphia; a well-water homeowner on the Oregon coast (Cascadia + tsunami); a Miami condo
  retiree on daily medication; a rural Kansas family with livestock; a Phoenix apartment single with
  a CPAP; a Houston suburban household with an EV; a zero-budget student in Chicago.
- Golden packets in `fixtures/golden/` regenerated by `rr-cli golden`; CI diffs them.
- Property tests: more people -> no less water; higher confidence -> no fewer days; more budget ->
  no less coverage; total spend ≤ budget; every item and number cites; targets monotone in horizon.
- Parity test: mock engine and wasm engine produce structurally identical outputs for the fixtures.
- The CLI is the oracle for humans and agents.

## 12. Non-goals for v1

Accounts, sync, sharing links carrying household data, native app stores, push notifications,
non-US data, translations, product recommendations, a marketplace, forums, medical dosing, firearms
guidance beyond safe storage and training pointers.

## 13. Risks and open questions

- **Duration data is thin** for several hazard -> bucket pairs; the app must be honest about priors
  and the verifier must confirm each is labelled. *(research)*
- **NRI is county-scale.** A ZIP in a large county can sit in a very different flood or wildfire
  regime; the tract pack is the fix, and copy must say "your county" until then.
- **Base rates for societal hazards** (grid failure, cyber, unrest) are genuinely uncertain; they are
  shown with wide ranges and low confidence, and the consequence-bucket design keeps their effect on
  the plan bounded.
- **Price bands date.** They carry a retrieval date and the user's own prices override them.
- **Federal risk data is moving and being withdrawn.** The NRI Future Risk Index is gone, so climate
  multipliers come from NCA5 / CMRA, not from NRI; the ETL must tolerate URL changes and the manifest
  must make a stale snapshot obvious. *(research)*
- **Scope creep** toward a general prepping encyclopedia. The packet is the product; guidance blocks
  stay short and attached to a bucket, hazard or tier.

## 14. Decision log (append only)

- 2026-09-26 — Round-2 review and the v0.2.0 contract (agent/types2; `~/Desktop/ready-reckoner-briefs/round2/REVIEW.md`,
  `round2/phase2/DESIGN-DELTA.md`). The review (a site walkthrough, four panels, a 22-event backtest:
  covered 6, partial 5, short 10, not modelled 1) found safety fixes (shipped as v0.1.1), a thin
  hazard list, model misses in water-system failure and power-outage tails, and a packet that is a
  shopping list rather than a plan. Decisions taken in the owner interview, as REVIEW §8 records
  them: **Hazard scope:** full taxonomy — 10 new ranked hazards, the 9-family severe box with
  location logic, 52 named sub-causes, and an "Also checked" line for everything else. **Nuclear
  wording:** say it plainly — the strategic-exposure class in words, a "how this number is made"
  drawer, a public source per site, and an anchor comparison with the household's own list.
  **Sensitive rows:** include all four — benefit interruption (gated by an optional input),
  eviction (renters), mass shooting or bombing (rare personal-safety row, free actions only),
  financial crisis. **Clean air:** a new `clean_air` readiness bucket (15th bucket), sized from
  county smoke days, with two new item kinds (air cleaner, respirators); ENGINE_API_VERSION 2.
  **Round scope:** everything — plan features, kit and allocator overhaul, model robustness with the
  public validation page, and the long-horizon module. **Execution:** v0.1.1 safety and correctness
  release first, then one parallel agent build released as v0.2.0 when verified. **Hosting:** keep
  the current address for now; the privacy page keeps its warning and agency outreach waits.
  **Owner request (mid-review):** the Risks page gets a plain ranked matrix at the top — every
  hazard, most likely first, with how likely and how bad, each name a jump link to its card. Added
  the same day after the Deviant Ollam talk: a ranked personal hazard `arrest_or_detention`, a
  trusted circle, lawyer, roadside number and numbers by heart on the family plan, and "tested?"
  dates for items that need testing. Contract v2 as merged: 19 new hazard ids (`HazardId::ACTIVE`
  holds the 53 the engine may emit); `terrorism` retired (parses, `#[deprecated]`, never emitted);
  a rare family's id is its hazard's id; the natural tier now includes five non-NRI hazards
  (`HazardId::is_nri` marks the 18); every new input optional or defaulted and every new output
  field left out when empty, so v1 plans and outputs load unchanged; the family plan is trimmed and
  length-capped, never required; the six v2 fixture households are staged in
  `fixtures/households/pending/` until their goldens and sample counties exist.
- 2026-09-26 — Release follow-ups (agent/followups; `~/Desktop/ready-reckoner-briefs/brief-release-followups.md`).
  `BucketAssessment.covered_today` (additive; API 1): coverage from `existing` and the assumed
  basics before the plan buys anything; `covered` stays the plan's end, and the plan and risks
  screens show both ("you have now" solid, "your plan covers" striped). Core pack trimmed:
  `zip_centroids.csv` and the NRI columns `expb`, `ealb`, `alrb` dropped (2.99 → 2.37 MB gzipped;
  a first visit fetches 1.99 MB of it); `ealp` kept for a health-based heat and cold severity
  although nothing reads it yet. A household on a well with large animals and a no-water target
  over 3 days gets a generator as a need for the pump (its fuel as a note line, so fuel is never
  bought before the generator) and stores 3 days of animal water, with two weeks in stock tanks
  as the alternative line; 14 stored days remain only where no pump power is planned (Hays: done
  month 43 → 34, purchases $3,410 → $2,664). Hays' 60-day no-water target is drought (a dry
  well), which neither a generator nor tanks solve; the line says to haul water in. Counties
  without outage records use their state's series (V-15); the water guardrail asks for stored
  water beyond refilled bottles by month 3 (V-16, replacing "no water at all after month one" in
  §4.7); the packet's "Spend" is the month's money out (V-18); "do not wait" is a pressure phrase
  (V-17).
- 2026-09-26 — Verification pass (agent/verify; `docs/VERIFICATION.md`). Model decisions taken to fix
  implausible real-pack numbers: a county outage curve treats the first zero share past its last
  positive one as an upper bound (0.5 %) and never decays more slowly than a log-normal with σ = 2
  beyond its last point (88 counties had a 365-day power target; none now exceeds 90 days); the
  major-hurricane share is taken against HURDAT2 tropical-storm passages, which NRI's frequency
  counts, shrunk toward the pooled 5.4 % with the weight of 5 passages (a missing major row now
  means none recorded, replacing the one-third fallback; Philadelphia power 5 → 3 days, as the
  research gives); heat and cold waves show at least "Serious" for households with someone 65+,
  a baby, a powered device, pregnancy (heat) or no cooling/heating (display only); family guidance
  marks one hazard's sentences `{if:<hazard>}…{/if}` and the packet keeps them only where that
  hazard's ten-year chance is at least 1 in 100. Housekeeping: `rr_data::verify` replaces
  `rr_etl::verify` (rr-cli drops rr-etl), and rr-data returns the NRI statement first. Open for the
  web workstream: the plan screen's list keys (V-10). Proposed, not done: pool counties without
  (or with few) outage records with the state series (V-15).
- 2026-09-25 — Founding interview decisions recorded in §2. Planner decisions recorded in §2.
- 2026-09-26 — Polish round merged (supply2 296939c, budget2 c4e0452, content2 17a6334, supply3 c9db968,
  plan-2 24aec38; plus rr-cli 0be9046 and rr-wasm 7efef62). Goldens now come from the REAL data pack and
  the wasm engine matches them number for number. Philadelphia: done in month 25 for $1,492 (was 44 /
  $2,585); Phoenix CPAP battery month 3 (was 10); Hays month 43; packets ≈ 10,000 words / ≈ 30 printed
  pages (target 20; the remaining lever is the print stylesheet: no page break before every section,
  sources in two columns). Decisions: livestock water capped at 14 stored days; 8 assumed basics credited
  in month 0 and listed in the packet; free actions grouped to 28 parents; kits are containers; each
  bucket keeps its "what to avoid" paragraph (CO, floodwater, do-not-drink warnings).
- 2026-09-26 — First end-to-end plan (rr-plan merged, 217f394) exposed presentation and catalogue
  problems, not model problems; polish round decided (`~/Desktop/ready-reckoner-briefs/POLISH_ROUND.md`):
  assumed household basics (`PlanInput.assume_basics`, `Item.assumed_basic`, listed in the packet);
  free actions grouped to ≤ 30 parents; go-bag, get-home bag and pet kit modelled as containers plus
  staged household supplies, never additive; bleach capped at one bottle per six months; one
  extinguisher per floor; sleeping bags only beyond a 3-day cold target or with a vulnerable member;
  one-off money goes to the top life-safety item; packet ≤ 20 printed pages with hazard blocks only for
  the top six likely hazards plus named scenarios; price-band sanity pass on the 30 most expensive items.
  Contract tweaks merged (agent/tweaks 342c39e): `Dials.rare_catastrophic_opt_in`,
  `IncomeStability::very_stable` (×0.5 job loss), ENGINE-API conventions; goldens regenerated for the
  citation repoint (source names only).
- 2026-09-26 — Web shell merged (agent/web-shell f8f1afe). Decisions: interview "Continue anyway"
  on validation problems (warn, don't block); no online-lookup switch in v1 (explanatory text instead);
  month-0 free actions capped at 8 with the rest rolled into months 1–3 (behavioural research; budget
  follow-up); `existing` holds one entry per item id (check-offs merge); `covered` may be 0; envelopes
  are one per item id; done items may appear in month 0. Open decision: hosting origin (§10).
- 2026-09-26 — Budget allocator decisions (agent/budget): default schedule is Split (reserve half toward
  an item costing more than a month's money, spend the rest); any person's daily prescription has harm
  weight 3; guardrail thresholds: evacuation-heavy = 10 % ten-year chance, go-bag expected by month 6,
  cold chain by month 3; one-off money lands in month 0 and the monthly budget starts in month 1;
  `one_in_100` ⇒ Λ* = −ln(0.9)/10; ladder rounding tolerates 3 % above a step; the consequence crate's
  cliff warning is canonical; heat and cold coverage are separate parts (`thermal_heat` / `thermal_cold`
  requirement classes); pending contract tweak `Dials.rare_catastrophic_opt_in`.
- 2026-09-25 — `rr-types` merged (agent/types 7cf13e0). Decisions made there: deterministic
  transcendental math via `rr_types::math` on pure-Rust `libm` (browser and glibc round `exp`/`ln`
  differently; `clippy.toml` now forbids the std methods in the workspace); z₀.₉ = 1.2815515655446004;
  type names `SavingsEnvelope`, `DataConfidence`, `HouseholdMobility` (JSON unchanged); optional
  additions `Owned.paid_usd`, `Citation.prior`, `Item.{retrieved, life_safety, rare_catastrophic}`, name
  tables in `Catalogue`, `TARGET_LADDER_DAYS`; `bad_input` details carry `problems[]`; county wins when
  both ZIP and county are given (how the UI records an `ambiguous_zip` pick); plan months count from 0 and
  the one-off budget lands in month 0; `home_loss` has a readiness target; `EngineInfo.attributions` is
  required so every engine carries the FEMA disclaimer; horizon 1–50 years; `income.earners` must equal
  the people marked earners. Confirmed: `OutageStats.p_ge_Nd` is the share of outages lasting ≥ N days
  (conditional on an outage), and `get_home` distance comes from the commute input.
- 2026-09-25 — Supply-standards research folded in: constants registry with sources and disagreement notes (`docs/research/supply-standards.md` §13–§14); water 1 gal/person-day basic (¾ drinking), survival ≈ 3 L, comfortable 15 L, heat ×1.75–2; food in kcal by DGA age band with cost per person-day (pantry $8.44 USDA TFP Aug 2026; staples $2.15–2.85; freeze-dried $9–39 per 2,000 kcal); medication reserve 14 days (7–30); **antibiotics quantity 0 with a clinician card**; one-month tier is an interpolation.
- 2026-09-25 — Risk-model research folded into §4: 14 buckets in three kinds (duration / readiness / money), water split into boil vs no-water, `supplies` replaces shelter_in_place + supply_chain, `get_home` and `medical_emergency` as readiness buckets; return-period dial (default one-in-100) with named-scenario toggles and the cliff rule; ranges and the day ladder; savings track for income; rare-catastrophic box and budget cap; harm weights; allocator promotion and sinking fund. `CountyRecord`, `BaseRate`, `HouseholdEventRate` added to the shared types as the data contract.
- 2026-09-25 — Data-source research folded into §6: NRI terms and v1.20 semantics, 5 MB core budget, ZIP ambiguity rule (`ambiguous_zip`), Connecticut crosswalk, no runtime federal calls, `EngineInfo.attributions`.
- 2026-09-25 — Prior-art and behavioural research folded in (§2 additions): three water levels,
  housing duration modifiers, two-tier relief rating, stage/confidence questions, drills as readiness
  items, kcal/litre planning, scored community items, firearm free-action wording, bundled snapshot.
  Contract additions (optional fields): `Dials.water_level`, `PlanInput.stage`, `PlanInput.confidence_1to5`,
  `BucketAssessment.relief`, `Item.energy_kcal_per_unit`, `Item.volume_l_per_unit`.
