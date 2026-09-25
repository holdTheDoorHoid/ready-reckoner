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
  mobility: Mobility                  # vehicles, commutes
  finances: Finances
  existing: [Owned]                   # optional inventory {item_id, qty}
  dials: Dials
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
}
Person {
  age_band: infant|toddler|child|teen|adult|senior     # <1, 1-3, 4-12, 13-17, 18-64, 65+
  pregnant_or_nursing: bool
  medical: { daily_rx: bool, refrigerated_rx: bool, powered_device: none|cpap|oxygen|other{watts},
             mobility: none|limited|wheelchair, dietary: [str], epinephrine: bool }
  earner: bool
  commute?: { distance_km: f32, mode: car|transit|walk|bike, remote_possible: bool }
}
Pets { dogs: u8, cats: u8, small: u8, large_animals: u8 }
Mobility { vehicles: [{ fuel: gas|diesel|hybrid|ev }] }
Finances {
  monthly_budget_usd: f32, one_off_budget_usd: f32
  emergency_fund_months: f32, monthly_expenses_usd?: f32
  income: { earners: u8, stability: stable|variable|seasonal|gig }
  insurance: { home_or_renters: bool, flood: bool, earthquake: bool }
}
Dials {
  confidence: nine_in_ten|nineteen_in_twenty|ninety_nine_in_hundred    # design-event quantile
  climate: today|y2050
  horizon_years: u8 = 10
}
```

### 4.2 Hazards

Natural (the 18 FEMA National Risk Index hazards): `avalanche`, `coastal_flooding`, `cold_wave`,
`drought`, `earthquake`, `hail`, `heat_wave`, `hurricane`, `ice_storm`, `landslide`, `lightning`,
`riverine_flooding`, `strong_wind`, `tornado`, `tsunami`, `volcanic_activity`, `wildfire`,
`winter_weather`.

Societal: `pandemic`, `grid_failure` (regional, multi-day, not weather-caused), `cyber_outage`
(utility, payments, telecom), `civil_unrest`, `supply_chain_disruption`, `hazmat_release`,
`nuclear_plant_incident`, `nuclear_attack` (includes EMP), `terrorism`.

Personal: `job_loss`, `house_fire`, `medical_emergency`, `vehicle_stranding`, `local_utility_outage`
(water main, boil-water notice, gas leak), `burglary`, `earner_death_or_disability`,
`extended_household_illness`.

Each hazard has, per location and dial setting:

```
HazardProfile {
  id, annual_probability: f64,       # P(at least one household-significant event in a year)
  severity: 0..1,                    # relative, for ranking and copy
  eal_per_household_usd?: f64,       # where NRI gives it
  climate_multiplier: f64,           # 1.0 for today
  confidence: high|medium|low|prior,
  sources: [CitationId]
}
```

Natural hazard probabilities come from the NRI annualised frequency and exposure fields (county
level; tract later), cross-checked against USGS seismic, USFS wildfire and NOAA severe-weather
climatologies *(confirm against research)*. Societal and personal probabilities come from national
base rates adjusted by household and location factors (income stability for `job_loss`, housing type
and alarms for `house_fire`, facility proximity for `hazmat_release` and `nuclear_plant_incident`,
setting for `burglary`), each adjustment cited or labelled `prior`.

### 4.3 Consequence buckets

| id | Plain name | Target type | Typical drivers |
| --- | --- | --- | --- |
| `power` | No grid electricity | days | storms, ice, heat, grid failure, wildfire PSPS |
| `water` | No safe tap water | days | floods, earthquakes, freezes, main breaks, boil-water notices |
| `shelter_in_place` | Can't leave home; stores and roads closed | days | winter storms, pandemic, unrest, hazmat |
| `evacuate` | Must leave home | notice hours + days away | wildfire, hurricane, flood, hazmat, tsunami |
| `thermal` | Dangerous heat or cold indoors | days | heat wave, cold wave, outage in season |
| `medical` | No EMS, pharmacy or hospital access | days | any large event; pandemic |
| `income` | Loss of household income | months | job loss, pandemic, disability, disaster displacement |
| `comms` | No phone or internet | days | storms, grid, cyber |
| `supply_chain` | Shortages of everyday goods | weeks | pandemic, port/rail/fuel disruption, regional disaster |
| `home_loss` | Home damaged or uninhabitable | one-off | fire, flood, tornado, earthquake, hurricane |
| `fire` | House fire | readiness | cooking, heating, electrical |
| `security` | Personal and home security | readiness | burglary, unrest, post-disaster opportunism |

Each hazard maps to buckets with a conditional probability and a duration distribution:

```
Effect { hazard, bucket, p_given_event: f64, duration: LogNormal{median_days, p90_days} | Fixed,
         evidence: empirical|prior, sources: [CitationId] }
```

Empirical durations come from outage restoration statistics (EAGLE-I, utility reliability reports),
boil-water-notice records, displacement studies and pandemic stay-home data where they exist; the
rest are labelled priors *(confirm against research)*.

### 4.4 The design event

For bucket *b*, horizon *H* years, and hazard *h* with annual probability *p_h*, the expected number
of *b*-consequences from *h* in *H* years is λ_h = H · p_h · q_{h,b}. Treating events as a Poisson
process with independent durations drawn from F_{h,b}, the chance that no event in *H* years lasts
longer than *d* days is

    P(max ≤ d) = exp( − Σ_h λ_h · (1 − F_{h,b}(d)) )

The **target** for bucket *b* is the smallest *d* with P(max ≤ d) ≥ c, where *c* is the confidence
dial (0.90, 0.95, 0.99). This is closed-form apart from a monotone root find, so it is deterministic
and fast enough to recompute on every dial change. Natural-frequency copy falls out of the same
expression: "of 100 households like yours, about `100·(1 − exp(−Σ λ_h (1 − F_h(3))))` will face an
outage longer than three days in the next ten years."

Targets are rounded up to the plan's tier boundaries for presentation and the raw value is kept for
the expert view. `income` uses months and is driven mainly by `job_loss`, `pandemic` and
`earner_death_or_disability`; `evacuate` reports notice time (minutes to hours) and days away;
`fire` and `security` are readiness checklists, not durations.

### 4.5 Tiers

| id | Plain name | Days | Enters the plan when |
| --- | --- | --- | --- |
| `now` | Free actions | 0 | always, first |
| `h72` | Three days | 3 | always |
| `get_home` | Get-home bag | per commuter | any commute > 3 km or by transit |
| `w2` | Two weeks | 14 | any bucket target > 3 days (almost everyone) |
| `m1` | One month | 30 | any bucket target > 14 days, or `income` ≥ 1 month |
| `m3` | Three months | 90 | `income` target ≥ 3 months or `supply_chain` ≥ 6 weeks |
| `m6` | Six months | 180 | `income` target ≥ 6 months |
| `y1` | One year | 365 | only when the user asks, or income/pandemic dials justify it |

The tier a household should reach is the maximum over buckets of the tier that covers each target.
The plan says, per bucket, which tier is "enough" and stops there.

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
buckets = ["water"]
tier = "h72"
free = false
spec = "Commercially bottled water, or tap water in clean food-grade containers with tight lids, kept cool and dark."
look_for = ["Food-grade (HDPE #2 or PET #1) containers", "Sealed bottled water with a date"]
avoid = ["Milk jugs (they leak and grow bacteria)", "Containers that held chemicals"]
price_band_usd = { low = 0.0, high = 1.50, per = "gallon", note = "reused bottles are free; bottled water about $1 per gallon" }
quantity_rule = "water_gallons"         # implemented in rr-supply
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

Uncovered risk for the household is

    R = Σ_b w_b · Σ_{d = covered_b + 1}^{target_b} P_b(need ≥ d days within H)

where *w_b* is a documented severity weight per bucket (water and medical highest, comms lowest)
and P_b(need ≥ d) = 1 − exp(−Σ_h λ_h (1 − F_{h,b}(d))). Because P falls with *d*, the first day of a
bucket is always worth more than the fourteenth: cheap early coverage floats to the top without
special cases.

Each month the allocator: (1) applies every `free = true` action not yet done, ordered by ΔR;
(2) repeatedly buys the (item, increment) with the highest ΔR per dollar that fits the remaining
budget; (3) if the best item costs more than the month's budget, reserves toward it (an envelope) and
says so. It stops when every bucket is covered to its target and reports "you are done for your
risk; here is the maintenance calendar". The user's recorded actual prices replace the band midpoint.

Guardrails (warn, never block): zero budget; a powered medical device with no power plan by month
three; refrigerated medication with no cooling plan; no water at all after month one; a household
with an evacuation-heavy profile and no go-bag; insurance gaps for owners in flood or quake zones.

### 4.8 Outputs

```
PlanOutput {
  engine_version, data_pack_version, content_version,
  location: LocationResolved,
  register: [HazardProfile]            # ranked
  buckets: [BucketAssessment]          # target, covered, contributions, frequency sentences
  tier_reached, tier_recommended,
  plan: { months: [{ index, budget, actions: [PlanItem] }], done: bool, reserve: [Envelope] },
  requirements: [RequirementLine],
  warnings: [Warning],
  packet_markdown: String,             # the printable packet, sections in §9
  provenance: [Citation]               # everything referenced above
}
```

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
| `rr-cli` | `plan`, `risks`, `explain`, `catalogue`, `data verify`, `golden` | rr-plan (native) |
| `rr-etl` | Downloads sources, builds packs, writes manifest (native, reqwest) | rr-types |

Hard rules (also in `CLAUDE.md`): deterministic; no wall clock; no OS entropy; no `rand`; all engine
crates compile for `wasm32-unknown-unknown`; `#![forbid(unsafe_code)]`.

## 6. Data layer *(confirm against research)*

Packs are versioned, checksummed files under `data/`, produced only by `rr-etl`, described in
`data/manifest.json` (source URL, retrieval date, sha256, licence, row counts). Raw downloads are never
committed.

| Pack | Contents | Load | Size target |
| --- | --- | --- | --- |
| `core` | County table (3,1xx rows: NRI per-hazard annualised frequency, exposure, EAL, SVI, resilience; state; NCA region; coastal/tsunami flags), ZIP -> county crosswalk, climate multipliers, national base rates, facility proximity flags | at start | ≤ 3 MB gzipped |
| `geo` | County boundaries (Census-derived TopoJSON) for the map thumbnail | lazy | ≤ 1 MB gzipped |
| `tract` | Census-tract NRI for finer location | lazy, later | tens of MB, chunked by state |

Refresh: `.github/workflows/data-refresh.yml` runs quarterly and opens a PR; `rr-cli data verify`
checks checksums and row counts; the About screen shows pack versions.

Privacy: ZIP -> county is a bundled table; no geocoder is called. County boundaries are bundled.
An optional online flood-zone point lookup, if added, is behind consent that names FEMA as the
recipient.

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

- 2026-09-25 — Founding interview decisions recorded in §2. Planner decisions recorded in §2.
- 2026-09-25 — Prior-art and behavioural research folded in (§2 additions): three water levels,
  housing duration modifiers, two-tier relief rating, stage/confidence questions, drills as readiness
  items, kcal/litre planning, scored community items, firearm free-action wording, bundled snapshot.
  Contract additions (optional fields): `Dials.water_level`, `PlanInput.stage`, `PlanInput.confidence_1to5`,
  `BucketAssessment.relief`, `Item.energy_kcal_per_unit`, `Item.volume_l_per_unit`.
