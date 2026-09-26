# Engine API (contract between `rr-wasm` / `rr-cli` and the web app)

`ENGINE_API_VERSION = 1`. Bump it whenever a type, id or function below changes shape; update
`crates/rr-types`, `web/src/engine/types.ts` and `web/src/engine/mock.ts` in the same commit.
`cargo test -p rr-types` fails if `types.ts` drifts from the Rust types (fields, optional markers,
id lists) or if this file stops naming a function or error code.

All functions take and return JSON strings (wasm-bindgen `String`), so the mock engine in TypeScript
and the WebAssembly engine are interchangeable behind `web/src/engine/index.ts` (`interface Engine`;
`RawEngine` is the string-level surface `rr-wasm` exports). Every function returns an envelope:

```
{ "ok": true,  "value": <result> }
{ "ok": false, "error": { "code": "<snake_case>", "message": "<plain language>", "details"?: any } }
```

Error codes and their `details`:

| Code | When | `details` |
| --- | --- | --- |
| `bad_input` | The JSON does not match the schema, or `PlanInput::validate` finds problems | `{ problems: [Problem] }` |
| `unknown_zip` | No such ZIP code | `{ suggestions: [LocationResolved] }` |
| `unknown_county` | No such county | `{ suggestions: [LocationResolved] }` |
| `ambiguous_zip` | The ZIP code spans several counties and none holds at least 80% of it (about 30% of ZIP codes) | `{ suggestions: [LocationResolved] }`, one per county, largest share first |
| `pack_missing` | A data pack the call needs is not loaded | absent |
| `pack_corrupt` | A pack failed its checksum or could not be decoded | absent |
| `internal` | A bug in the engine | absent |

After `ambiguous_zip` the app asks the user to pick a county and stores it in
`location.county_fips`, keeping the ZIP code: when both are given, `county_fips` decides the county.
`assess` returns the same location errors as `resolve_location`.

## Functions

| Function | Input | Output |
| --- | --- | --- |
| `engine_info()` | — | `EngineInfo { engine_version, api_version, data_pack_version?, content_version, packs_loaded: [string], attributions: [Attribution] }` |
| `load_pack(name, bytes)` | pack name, bytes (Uint8Array) | `PackInfo { name, version, rows }`. The web app fetches packs (same origin) and hands the bytes in; the engine never fetches |
| `county_search(query)` | free text ("phila", "42101", "Cook, IL") | `[LocationResolved]`, up to 10 |
| `resolve_location(json)` | `LocationInput` | `LocationResolved`, or `unknown_zip` / `unknown_county` / `ambiguous_zip` with suggestions |
| `assess(json)` | `PlanInput` | `PlanOutput` |
| `explain(json)` | `ExplainRequest { kind: "hazard" \| "bucket" \| "item" \| "requirement" \| "warning", id, input: PlanInput }` | `Explanation { title, plain: [string], math?: [string], sources: [Citation] }` |
| `catalogue()` | — | `Catalogue { items: [Item], citations: [Citation], guidance: [GuidanceMeta], hazards: [HazardInfo], buckets: [BucketInfo], tiers: [TierInfo] }` |
| `defaults()` | — | a valid `PlanInput` skeleton with every section filled in (the interview starts from it) |

`assess` must be pure and fast (target < 50 ms in wasm for any fixture): the UI calls it on every
dial change. `rr_types::PlanInput::from_json` is the single parse-and-validate entry point, so
`rr-wasm` and `rr-cli` reject bad input identically.

`defaults()` sets two placeholders the app must replace: `planning_date` (2026-10-01; the engine
never reads the clock) and `location.zip` `"00000"`, which is well formed but not a real ZIP code,
so a forgotten placeholder fails with `unknown_zip` instead of planning for somewhere else.
Safety equipment defaults to absent, so a skipped question leads to a recommendation.

`attributions` lists the credit lines and disclaimers the About screen and the packet must show:
the FEMA National Risk Index terms require the dataset version, the access date and a statement that
FEMA does not endorse the app; CC BY sources (EAGLE-I, the NCA5 Atlas) need credit lines.

## Conventions

- Field names are snake_case. Enums serialise as their snake_case string ids. Optional fields are
  omitted when absent, never `null`. Dates are ISO `YYYY-MM-DD`.
- Money is `f32` US dollars and days are `f32`; probabilities, rates, severity and coordinates are
  `f64`. Counts are unsigned integers.
- Every struct rejects unknown fields, so a typo in the app, a fixture, an imported plan or a content
  file is an error rather than a silently ignored value. Fields marked "defaults when absent" below
  may be left out of input and are always present in the engine's output.
- `PlanInput.existing` holds at most one entry per item id: the UI merges check-offs into the one
  entry rather than appending a new one. In `BucketAssessment.covered`, `Target::days.value` (and
  the other target kinds' `value`) may be 0, meaning the household is not on the ladder for that
  bucket at all, not that the target itself is 0. `Plan.envelopes` holds one `SavingsEnvelope` per
  item id. `Plan.months[0]` may list items that are already done (`PlanItem.kind = "free_action"`,
  `done: true`), not only ones still to do.
- Deterministic: same input, same output, byte for byte, on every target. Engine crates compute
  exp, ln, pow and the normal distribution with `rr_types::math` (pure-Rust libm), because the
  platform maths library differs between native builds and WebAssembly in the last bit.

## Validation

`bad_input` from validation lists every problem, in a fixed order:
`Problem { code, field, message }`, where `field` is the JSON path (`people[1].commute.distance_km`;
empty for the input as a whole) and `message` is plain language fit to show next to the field.
Codes: `schema` (the JSON does not match the types), `unsupported_country` (not `"US"`),
`location_missing` (neither ZIP nor county), `zip_format` and `county_fips_format` (not exactly five
digits), `no_people`, `negative_value`, `not_finite`, `out_of_range` (horizon outside 1–50 years,
device watts not above 0, `confidence_1to5` outside 1–5), `earners_mismatch`
(`finances.income.earners` differs from the people marked `earner`), `id_format` (an item or
scenario id that is not snake_case), `duplicate_id` (a scenario toggled twice).

Unusual but meaningful choices (a zero budget, no insurance, qty 0) are not problems. The plan's
guardrail warnings handle those, and they never block.

## Types

Defined in `crates/rr-types` (Rust, serde) and mirrored by hand in `web/src/engine/types.ts`.
`?` marks an optional field.

### Inputs (`docs/DESIGN.md` §4.1)

```
PlanInput { planning_date, location: LocationInput, housing: Housing, people: [Person], pets: Pets,
            mobility: HouseholdMobility, finances: Finances, existing: [Owned], dials: Dials,
            stage?: not_thought_about|thinking|have_some_things|have_a_plan|maintaining,
            confidence_1to5?: u8 }
LocationInput { country: "US", zip?, county_fips?, setting: urban|suburban|rural }
Housing { kind, tenure, floor: i8, basement, water: municipal|well, sewer: sewer|septic, heating,
          cooling, backup_power, alarms: { smoke, co, extinguisher } }
Person { age_band, pregnant_or_nursing, medical: Medical, earner, commute?: Commute }
Medical { daily_rx, refrigerated_rx, powered_device, mobility: none|limited|wheelchair,
          dietary: [string], epinephrine }
  powered_device: "none" | "cpap" | "oxygen" | { "other": { "watts": f32 } }
Commute { distance_km: f32, mode: car|transit|walk|bike, remote_possible }
Pets { dogs, cats, small, large_animals }                      # u8 counts
HouseholdMobility { vehicles: [{ fuel: gas|diesel|hybrid|ev }] }   # JSON field `mobility`
Finances { monthly_budget_usd, one_off_budget_usd, emergency_fund_months, monthly_expenses_usd?,
           income: { earners: u8, stability: very_stable|stable|variable|seasonal|gig },
           insurance: { home_or_renters, flood, earthquake } }
Owned { item_id, qty: f32, paid_usd? }
Dials { return_period: one_in_10|one_in_50|one_in_100|one_in_500,
        climate: today|y2050, horizon_years: u8,
        water_level: survival|basic|comfortable,               # defaults to basic when absent
        scenario_overrides: [{ id, on }],                      # defaults to [] when absent
        rare_catastrophic_opt_in: bool }                       # defaults to false when absent
```

`existing` is the baseline inventory. A free action counts as done when it appears there with `qty`
1 or more. `paid_usd` is what the household paid for that quantity in total; recorded prices replace
price-band midpoints. `defaults()` uses `return_period` `one_in_100` and `horizon_years` 10.

### Outputs

```
LocationResolved { country, county_fips, county_name, state_abbr, state_name, zip?,
                   zip_county_share?: f32, centroid: { lat, lon }, nca_region, coastal, tsunami_zone,
                   facility_flags: { nuclear_plant_within_16km, nuclear_plant_within_80km,
                                     hazmat_facilities_within_5km: u16 },
                   data_note? }

HazardProfile { id, name, tier: natural|societal|personal, display: ranked|rare_catastrophic,
                rate_per_year, rate_range: [lo, hi], annual_probability, probability_range: [lo, hi],
                severity, eal_per_household_usd?, climate_multiplier,
                confidence: high|medium|low|prior, sources: [CitationId], frequency_sentence,
                buckets: [BucketId] }

BucketAssessment { id, name, target: Target, covered: Target, tier_enough: TierId,
                   contributions: [{ hazard, share }], frequency_sentences: [string],
                   sources: [CitationId],
                   relief?: { help_arrives_days, mostly_restored_days, sources } }
Target = { kind: "days", value, low, high }                                # duration buckets
       | { kind: "months", value, low, high }                              # income
       | { kind: "evacuate", p_need_10yr, notice_hours_low, notice_hours_high, days_away }
       | { kind: "readiness", p_need_10yr, done: u8, of: u8 }              # other readiness buckets, home_loss

RequirementLine { id, bucket, item_class, quantity, unit, per: household|person|commuter|pet, rule,
                  citations: [CitationId], plain }

PlanItem { item_id, name, kind: free_action|purchase|reserve, quantity, unit, est_cost_usd,
           price_band: { low, high }, buckets: [BucketId], hazards: [HazardId], why,
           risk_reduction, tier: TierId, done?, paid_usd? }

PlanOutput { engine_version, api_version, data_pack_version, content_version,
             location: LocationResolved, register: [HazardProfile], buckets: [BucketAssessment],
             scenarios: [ScenarioInfo], tier_reached: TierId, tier_recommended: TierId,
             plan: Plan, requirements: [RequirementLine], warnings: [Warning],
             packet_markdown, provenance: [Citation] }
Plan { months: [{ index, budget_usd, items: [PlanItem] }], done_month?,
       envelopes: [{ item_id, saved_usd, needed_usd }],
       savings_track?: { target_months, target_usd, current_months, monthly_suggestion_usd, why } }
ScenarioInfo { id, name, applies_because, on, effect_summary, sources: [CitationId] }
Warning { id, severity: note|warn, message, why, related: [string] }
```

What the numbers mean:

- `register`: `ranked` hazards first, most important first; then the `rare_catastrophic` ones
  (nuclear attack and EMP, war, terrorism), which the app shows in their own box with likelihood
  and severity as two columns, never ranked by expected loss. `rate_per_year` is the household event
  rate r_h; `annual_probability` is 1 − e^(−r_h).
- `buckets` come in `BucketId` order. The target's kind follows the bucket: days for the duration
  buckets, months for `income`, `evacuate` for `evacuate`, readiness for the other readiness buckets
  and for `home_loss` (an insurance-and-documents decision with no stockpile target).
  `value` is rounded to the day ladder ½, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90, 180, 365
  (`TARGET_LADDER_DAYS` in both Rust and TypeScript); `low` and `high` are the 10th and 90th
  percentiles under parameter uncertainty. In `covered`, `low` and `high` equal `value` and
  `p_need_10yr` repeats the target's. `tier_enough` is where the plan stops adding to that bucket.
- `relief` (duration buckets, where known): when outside help plausibly arrives and when service is
  mostly restored for the design event (the Oregon Resilience Plan's two-tier rating).
- `scenarios`: named scenarios (for example `cascadia_m9`) that apply to this location, with the
  engine's default or the user's override from `dials.scenario_overrides`.
- `RequirementLine.quantity` is for the whole household, already multiplied out by `per`.
- `PlanItem.price_band` is the cost range of the whole quantity; `est_cost_usd` is the recorded price
  if there is one, otherwise the middle of the band. `done` is omitted when false.
- `Plan.months` are counted from 0: month 0 begins on the planning date, holds the free actions
  first, and receives the one-off budget. `done_month` is the month by which every bucket is covered.
  `envelopes` are sinking funds for items that cost more than a month's budget. `savings_track` is the
  emergency-fund goal for `income`, never funded from the supplies budget.

### Content (`docs/DESIGN.md` §4.6)

```
Citation { id, title, publisher, year?, url, retrieved, quote?, license,
           prior }                                             # defaults to false when absent
Item { id, name, category, unit, buckets: [BucketId], tier: TierId, free,
       life_safety, rare_catastrophic,                         # default to false when absent
       spec, look_for: [string], avoid: [string],
       price_band_usd: { low, high, per, note? }, retrieved?,  # when the price band was observed
       quantity_rule, maintenance?: { rotate_months?, check_months? }, citations: [CitationId],
       hazard_extras: [HazardId], energy_kcal_per_unit?, volume_l_per_unit? }
GuidanceMeta { id, title, applies_to: [string], citations: [CitationId] }
```

- `Citation.prior`: the source is an expert judgement, not data. Every number that cites it is shown
  as an estimate ("tagged Prior").
- `Item.life_safety`: the allocator orders it first within its tier (smoke and CO alarms, water, a
  dependent's medication, powered-device backup). `Item.rare_catastrophic`: the allocator gives it
  $0 by default and, when the household turns on `Dials.rare_catastrophic_opt_in`, at most 10% of
  the monthly budget (a radiation meter, potassium iodide only on official instruction, Faraday
  storage); off by default.
- `energy_kcal_per_unit` and `volume_l_per_unit` let the app show cost per 2,000 kcal and per litre or
  gallon.

### Function arguments and results

```
EngineInfo { engine_version, api_version, data_pack_version?, content_version,
             packs_loaded: [string], attributions: [Attribution] }
Attribution { source, text, url, version?, accessed }         # show `text` exactly
PackInfo { name, version, rows: u32 }
Catalogue { items, citations, guidance, hazards: [HazardInfo], buckets: [BucketInfo],
            tiers: [TierInfo] }
HazardInfo { id, name, tier }  BucketInfo { id, name, kind, target_kind }  TierInfo { id, name, days }
ExplainRequest { kind, id, input: PlanInput }
Explanation { title, plain: [string], math?: [string], sources: [Citation] }
EngineError { code, message, details? }
Problem { code, field, message }
```

`data_pack_version` is absent from `EngineInfo` until a pack is loaded.

### Engine-internal types (Rust only)

`rr-types` also defines the data contract between the engine crates, which never crosses into
JavaScript: `CountyRecord` (with `NriHazard`, `OutageStats`, `EventRate`, `Seismic`, `FloodPriors`,
`Facilities`, `Vulnerability`), `BaseRate`, `HouseholdEventRate` (the `rr-hazards` →
`rr-consequence` interface), and `Effect` with `DurationDist`
(`{ kind: "log_normal", median_days, p90_days }` or `{ kind: "fixed", days }`;
σ = ln(p90 / median) / z₀.₉ with z₀.₉ = 1.2815515655446004). `Effect` and `DurationDist` are also
mirrored in `types.ts` for an expert view.

## Ids

Ids are stable snake_case strings. `rr-types` exposes them as enums with `ALL`, `as_str()`,
`FromStr` and `Display`; `types.ts` exposes each as an `as const` list and a union type.
`catalogue()` returns the plain names, so the UI never hard-codes them.

- **Hazards** (35; `docs/DESIGN.md` §4.2): 18 natural (the NRI hazards), 9 societal, 8 personal.
- **Buckets** (14): duration `power`, `water_boil`, `water_out`, `supplies`, `thermal`,
  `medication`, `comms`; readiness `evacuate`, `get_home`, `medical_emergency`, `fire`, `security`;
  money `income`, `home_loss`.
- **Tiers** (7): `now` (0 days), `h72` (3), `w2` (14), `m1` (30), `m3` (90), `m6` (180), `y1` (365).
  The get-home bag is an item in the `get_home` bucket, not a tier.
- **Return periods**: `one_in_10` "Common disruptions", `one_in_50` "Serious", `one_in_100` "Very
  serious" (default), `one_in_500` "Rare catastrophes".

## Mock engine

`web/src/engine/mock.ts` returns deterministic, plausible values for every function, keyed on the
fixture households (`web/src/engine/fixtures.ts`), so screens can be built and screenshot-tested
before the engine exists. A parity test asserts that the mock and wasm outputs have identical JSON
shapes for each fixture.
