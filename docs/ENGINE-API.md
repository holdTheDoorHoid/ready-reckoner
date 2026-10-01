# Engine API (contract between `rr-wasm` / `rr-cli` and the web app)

`ENGINE_API_VERSION = 3`. Bump it whenever a type, id or function below changes shape; update
`crates/rr-types`, `web/src/engine/types.ts` and `web/src/engine/mock.ts` in the same commit.
`cargo test -p rr-types` fails if `types.ts` drifts from the Rust types (fields, optional markers,
id lists, limits) or if this file stops naming a function or error code.

Version 3 is the v0.3.0 contract (`docs/DESIGN-DELTA-v3.md` §3, §4). Every input addition is
optional, so a saved v1 or v2 plan still loads unchanged; the one breaking change is on the output
side: `PlanOutput.packet_markdown` is gone, replaced by the during-event [binder](#the-binder)
and the Prepare sheet (`prepare_markdown`). See [Changes from v2](#changes-from-v2).

Version 2 was the v0.2.0 contract (`~/Desktop/ready-reckoner-briefs/round2/phase2/DESIGN-DELTA.md`
§1): every addition optional or defaulted, the one breaking change the retired hazard id
`terrorism`. See [Changes from v1](#changes-from-v1).

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
| `engine_info()` | — | `EngineInfo { engine_version, api_version, data_pack_version?, content_version, packs_loaded: [string], attributions: [Attribution], validation?: ValidationSummary }` |
| `load_pack(name, bytes)` | one data file: its path as `data/manifest.json` lists it (`manifest.json`, `core/nri_hazards.csv`, `geo/counties.json`), bytes (Uint8Array) | `PackInfo { name, version, rows }`: the path, the manifest's `pack_version`, the rows read. The web app fetches the files (same origin) and hands the bytes in, manifest first; the engine never fetches. See [Loading](#loading) |
| `county_search(query)` | free text ("phila", "42101", "Cook, IL") | `[LocationResolved]`, up to 10 |
| `resolve_location(json)` | `LocationInput` | `LocationResolved`, or `unknown_zip` / `unknown_county` / `ambiguous_zip` with suggestions |
| `assess(json)` | `PlanInput` | `PlanOutput`, with the [binder](#the-binder) (`PlanOutput.binder`) and the Prepare sheet (`prepare_markdown`) built inside the call |
| `explain(json)` | `ExplainRequest { kind: "hazard" \| "bucket" \| "item" \| "requirement" \| "warning", id, input: PlanInput }` | `Explanation { title, plain: [string], math?: [string], sources: [Citation] }` |
| `catalogue()` | — | `Catalogue { items: [Item], citations: [Citation], guidance: [GuidanceMeta], hazards: [HazardInfo], buckets: [BucketInfo], tiers: [TierInfo] }` |
| `defaults()` | — | a valid `PlanInput` skeleton with every section filled in (the interview starts from it) |

`assess` must be pure and fast (target < 50 ms in wasm for any fixture, the binder included): the
UI calls it on every dial change. `rr_types::PlanInput::from_json` is the single parse-and-validate entry point, so
`rr-wasm` and `rr-cli` reject bad input identically.

`defaults()` sets two placeholders the app must replace: `planning_date` (2026-10-01; the engine
never reads the clock) and `location.zip` `"00000"`, which is well formed but not a real ZIP code,
so a forgotten placeholder fails with `unknown_zip` instead of planning for somewhere else.
Safety equipment defaults to absent, so a skipped question leads to a recommendation.

`attributions` lists the credit lines and disclaimers the About screen and the binder must show:
the FEMA National Risk Index terms require the dataset version, the access date and a statement that
FEMA does not endorse the app; CC BY sources (EAGLE-I, the NCA5 Atlas) need credit lines. A
source that only feeds an optional pack (the surge and wildfire-places packs) is listed only once
a file of that pack has been loaded.

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
- Old files keep working. Input fields added in v2 and v3 are optional (absent means "not
  asked") or default when absent, so every v1 and v2 input parses unchanged. Output fields added
  in v2 are left out when empty (absent, `[]` or `false`). Contract v3 removed
  `PlanOutput.packet_markdown` and made `binder` and `prepare_markdown` required, so an output
  stored under v1 or v2 no longer parses as a v3 `PlanOutput`: the app recomputes outputs on load
  and never reads a stored one, so a stored output's ids never outlive the engine. Retired ids
  still parse (`HazardId::is_retired`); the engine never emits them, and engine crates build
  output from `HazardId::ACTIVE`, never from `ALL`.
- Echo-only text (the family plan and, since v3, the people's profiles and the family-plan
  groups) is trimmed and length-capped by `PlanInput::tidy` and otherwise printed exactly as
  written: never checked, corrected or used in a computation.

## Validation

`bad_input` from validation lists every problem, in a fixed order:
`Problem { code, field, message }`, where `field` is the JSON path (`people[1].commute.distance_km`;
empty for the input as a whole) and `message` is plain language fit to show next to the field.
Codes: `schema` (the JSON does not match the types), `unsupported_country` (not `"US"`),
`location_missing` (neither ZIP nor county), `zip_format` and `county_fips_format` (not exactly five
digits), `no_people`, `negative_value`, `not_finite`, `out_of_range` (horizon outside 1–50 years,
device watts not above 0, `confidence_1to5` outside 1–5), `earners_mismatch`
(`finances.income.earners` differs from the people marked `earner`), `id_format` (an item,
scenario or family id that is not snake_case), `duplicate_id` (a scenario toggled twice),
`unknown_id` (a `dials.rare_opt_in` entry that names no rare family and is not `all`; v2).

Unusual but meaningful choices (a zero budget, no insurance, qty 0) are not problems. The plan's
guardrail warnings handle those, and they never block.

`PlanInput::from_json` tidies before it validates: the family plan's free text is trimmed, cut to
`FAMILY_PLAN_TEXT_MAX` (300) characters for notes and `FAMILY_PLAN_SHORT_MAX` (80) for names and
phone numbers, its lists to `ROUTES_MAX` (2), `TRUSTED_CIRCLE_MAX` (4) and `NUMBERS_BY_HEART_MAX`
(5) entries; blank text becomes absent, empty entries are dropped, and an empty plan disappears.
Since v3 the same goes for each person's `profile` and the family plan's `home`, `neighbourhood`,
`pets`, `vehicles` and `documents`: every string is cut at the cap its field names (characters
after trimming; the table under [Types](#types)), every list at its length, and an empty entry or
group is dropped (a `Place` with nothing but its `kind` counts as empty). `AccountInfo.last4`
keeps only the last four digits of whatever was typed, and is dropped when it holds no digit, so
a pasted account number never reaches the file. Nothing in any of it is ever required or
reported as a problem: a household with none of it is complete. The limits are exported in
`types.ts` for the form's `maxlength`.

## Types

Defined in `crates/rr-types` (Rust, serde) and mirrored by hand in `web/src/engine/types.ts`.
`?` marks an optional field.

### Inputs (`docs/DESIGN.md` §4.1)

```
PlanInput { planning_date, location: LocationInput, housing: Housing, people: [Person], pets: Pets,
            mobility: HouseholdMobility, finances: Finances, existing: [Owned],
            assume_basics?: bool,                                  # defaults to true when absent
            dials: Dials,
            stage?: not_thought_about|thinking|have_some_things|have_a_plan|maintaining,
            confidence_1to5?: u8,
            family_plan?: FamilyPlan }                             # v2; never computed with
LocationInput { country: "US", zip?, county_fips?, setting: urban|suburban|rural }
Housing { kind, tenure, floor: i8, basement, water: municipal|well, sewer: sewer|septic, heating,
          cooling, backup_power, alarms: { smoke, co, extinguisher },
          below_grade_bedroom: bool,                               # v2; defaults to false
          cooking?: electric|gas|induction|none,                   # v2; absent = not asked
          raw_water_source?: none|well|surface_nearby|rain_barrel|neighbour_well,   # v2
          water_system_record?: fine|occasional_notices|frequent_problems|unknown } # v2
Person { age_band, pregnant_or_nursing, medical: Medical, earner, commute?: Commute,
         access_needs: [hearing|vision|limited_english|cognitive|supervision|service_animal
                        |dialysis|home_health],                    # v2; defaults to []
         profile?: PersonProfile }                                 # v3; omitted when empty
Medical { daily_rx, refrigerated_rx, powered_device, mobility: none|limited|wheelchair,
          dietary: [string], epinephrine }
  powered_device: "none" | "cpap" | "oxygen" | { "other": { "watts": f32 } }
Commute { distance_km: f32, mode: car|transit|walk|bike, remote_possible }
Pets { dogs, cats, small, large_animals }                      # u8 counts
HouseholdMobility { vehicles: [{ fuel: gas|diesel|hybrid|ev }] }   # JSON field `mobility`
Finances { monthly_budget_usd, one_off_budget_usd, emergency_fund_months, monthly_expenses_usd?,
           income: { earners: u8, stability: very_stable|stable|variable|seasonal|gig },
           insurance: { home_or_renters, flood, earthquake,
                        sewer_backup?: bool, life_or_disability?: bool },   # v2; absent = not asked
           benefits: [federal_pay|snap_wic|ssi_ssdi|va|unemployment] }       # v2; defaults to []
Owned { item_id, qty: f32, paid_usd?, tested_on?: date }       # tested_on: v2
Dials { return_period: one_in_10|one_in_50|one_in_100|one_in_500,
        climate: today|y2050, horizon_years: u8,
        water_level: survival|basic|comfortable,               # defaults to basic when absent
        scenario_overrides: [{ id, on }],                      # defaults to [] when absent
        rare_catastrophic_opt_in: bool,                        # defaults to false when absent
        rare_opt_in: [string],                                 # v2; family ids or ["all"]; defaults to []
        minimum_kit: bool,                                     # v2; defaults to false
        long_horizon: bool,                                    # v2; defaults to false
        legal_opt_in?: bool }                                  # v2; absent = false
FamilyPlan { meeting_place_near?, meeting_place_far?, out_of_area_contact?: Contact,
             school_pickup?, work_plans?, shelter_spot_home?, shelter_spot_work?,
             where_we_would_go?, routes?: [string], neighbours_who_check?, who_takes_animals?,
             shutoff_gas?, shutoff_water?, shutoff_electric?,
             trusted_circle?: [TrustedPerson], lawyer?: Contact, roadside_assistance?,
             numbers_by_heart?: [string],                      # v2; every field free text
             home?: HomeInfo, neighbourhood?: Neighbourhood, pets?: [PetInfo] (≤ 8),
             vehicles?: [VehicleInfo] (≤ 4), documents?: DocumentsInfo }   # v3
Contact { name?, phone?, address? }                            # address: v3
TrustedPerson { name?, phone?, holds?: [spare_key|documents|medical_poa|backup_codes] }
```

The contract v3 groups (DESIGN-DELTA-v3 §3.1, §3.2), with each string's cap in characters after
trimming. Every field is optional; a list or group left empty is omitted from the JSON, which is
why the lists carry `?` here (the delta writes them without it).

```
PersonProfile { name?(60), date_of_birth?(40), phone?(40), email?(80), place?: Place,
                doctor?: Contact, pharmacy?: Contact, conditions?(400),
                medications?: [Medication] (≤ 12), allergies?(200), blood_type?(8),
                insurance?: HealthInsurance, id_notes?(200), notes?(400) }
Place { kind: work|school|childcare|other, name?(120), address?(200), phone?(40), plan?(400),
        pickup?(200), safest_spot?(200) }                     # dropped when only `kind` is set
Medication { name?(80), dose?(60), schedule?(80), purpose?(80) }
HealthInsurance { carrier?(80), plan_name?(80), member_id?(60), group_number?(60), phone?(40) }
Contact { name?(80), phone?(80), address?(200) }
HomeInfo { address?(200), electric_utility?: Contact, gas_utility?: Contact,
           water_utility?: Contact, insurer?: Contact, policy_number?(60),
           landlord_or_mortgage?: Contact, where_kit?(200), where_documents?(200),
           where_cash?(200), where_keys?(200) }
Neighbourhood { hospital?: Contact, urgent_care?: Contact, pharmacy?: Contact, shelter?: Contact,
                county_emergency_office?: Contact, alerts?(200) }
PetInfo { name?(60), kind?(40), description?(120), medications?(200), vet?: Contact,
          microchip?(60), records_where?(200) }
VehicleInfo { description?(120), plate?(20), insurer?: Contact, policy_number?(60),
              kept_in_car?(200) }
DocumentsInfo { accounts?: [AccountInfo] (≤ 12), policies?: [PolicyInfo] (≤ 8),
                where_originals?(200), where_copies?(200), digital_backup?(200) }
AccountInfo { institution?(80), kind?(40), phone?(40), last4?(4, digits only) }
PolicyInfo { insurer?(80), kind?(40), policy_number?(60), phone?(40) }
```

The caps are exported by name in Rust and `types.ts`: `LAST4_LEN` (4), `BLOOD_TYPE_MAX` (8),
`PLATE_MAX` (20), `SHORT_TEXT_MAX` (40: dates of birth, the v3 phone fields, kinds),
`MEDIUM_TEXT_MAX` (60: names of people and animals, doses, member, group, policy and microchip
numbers), `LABEL_TEXT_MAX` (80: emails, institutions, insurers, carriers and plan names, a
medication's name, schedule and purpose), `DESCRIPTION_MAX` (120), `LONG_TEXT_MAX` (200:
addresses and where-it-is answers), `NOTE_MAX` (400), and the list lengths `MEDICATIONS_MAX` (12),
`PETS_MAX` (8), `VEHICLES_MAX` (4), `ACCOUNTS_MAX` (12), `POLICIES_MAX` (8). A `Contact`'s name
and phone keep their v2 cap, `FAMILY_PLAN_SHORT_MAX` (80). Each field's cap is named in its doc
comment in `crates/rr-types/src/input.rs` and `types.ts`.

`existing` is the baseline inventory. A free action counts as done when it appears there with `qty`
1 or more. `paid_usd` is what the household paid for that quantity in total; recorded prices replace
price-band midpoints. `assume_basics` (on by default) credits the household with everyday basics
that almost every home has — blankets and warm layers, a cooking pot and can opener, a phone, a bag
per person, three days of ordinary food — unless the Have screen says otherwise; the packet lists
what was assumed. Those items are the ones with `Item.assumed_basic` set. `defaults()` uses
`return_period` `one_in_100` and `horizon_years` 10.

v2 inputs, in plain terms:

- `access_needs` (CMIST: communication, maintaining health, independence, support and safety,
  transportation) drive the communication plan, the registries and evacuation help; mobility and
  powered devices keep their own fields in `medical`.
- `below_grade_bedroom`: someone sleeps below street level (flash-flood card, water alarm).
  `cooking`: the main stove; a gas range can boil water through a power cut. `raw_water_source`:
  water the household could filter; a filter's days count only with a source. `water_system_record`
  is combined with the county's EPA drinking-water violations. Absent means "not asked": the engine
  assumes no gas range, no raw source and an unknown record.
- `benefits`: only households that tick one see the benefit-interruption hazard.
  `insurance.sewer_backup` and `insurance.life_or_disability` feed the insurance decisions.
- `Owned.tested_on`: when the household last tried an item that needs testing (items with
  `Item.test_interval_months`).
- `rare_opt_in` names the rare families the rare allowance may buy for (a family id is the id of
  the rare hazard that heads it; see [Ids](#ids)), or `["all"]`. `rare_catastrophic_opt_in` stays
  and means `["all"]`; engine crates read both through `Dials::rare_families()`. `minimum_kit`
  turns on bare-minimum mode (the engine also switches it on, and warns, when a plan would run past
  36 months). `long_horizon` shows the long-horizon section even when no target passes 30 days.
  `legal_opt_in` (turned on from the arrest row of the register) adds a legal-emergency amount
  (bail and a lawyer, cited, with its range) to the savings track, shown apart from the months of
  income the goal protects; it is left out of the JSON when false.
- `family_plan` is the household's own plan from a device-only screen: echoed into the packet and
  the wallet cards, stored in the saved plan file (it is the household's own file), never used for
  computation, and never sent anywhere.
- v3: `Person.profile` and the family plan's `home`, `neighbourhood`, `pets`, `vehicles` and
  `documents` are the answers to the interview's optional steps 6–8 (DESIGN-DELTA-v3 §2): echoed
  into the binder (a page per person, the wallet cards, the home, places, neighbourhood, pets,
  vehicles and documents pages; blank lines where nothing was answered) and never computed with.
  The plan's own numbers still come from the v1 and v2 fields (`pets` counts, `mobility`
  vehicles, `medical`). The saved file may now hold sensitive answers (DESIGN-DELTA-v3 §7).

### Outputs

```
LocationResolved { country, county_fips, county_name, state_abbr, state_name, zip?,
                   zip_county_share?: f32, centroid: { lat, lon },
                   zip_centroid?: { lat, lon },                            # v3
                   nca_region, coastal, tsunami_zone,
                   facility_flags: { nuclear_plant_within_16km, nuclear_plant_within_80km,
                                     hazmat_facilities_within_5km: u16 },
                   data_note?,
                   exposure?: Exposure }                                   # v2
Exposure { strategic_class?, strategic_km?, surge_cat3_share?, surge_proxy_class?, smoke_days_35?, leveed_pop_share?,
           dams_high_within_10km?, karst_share?, landslide_susceptible_share?, water_system_flag?,
           geomag_factor?, uasi_share?, eviction_rate? }    # v2; each a Sourced { value, source }

HazardProfile { id, name, tier: natural|societal|personal, display: ranked|rare_catastrophic,
                rate_per_year, rate_range: [lo, hi], annual_probability, probability_range: [lo, hi],
                severity, eal_per_household_usd?, climate_multiplier,
                confidence: high|medium|low|prior, sources: [CitationId], frequency_sentence,
                buckets: [BucketId],
                family?, sub_causes?: [SubCause], location_factor?: LocationFactor,  # v2
                range_only?: bool, anchor_sentence?, if_it_reaches_you?, what_it_changes? } # v2
SubCause { id, name, note, rate_range?: [lo, hi], sources: [CitationId] }            # v2
LocationFactor { class, label, multiplier: [lo, mid, hi], sources: [CitationId] }    # v2

BucketAssessment { id, name, target: Target, covered: Target, covered_today: Target,
                   tier_enough: TierId,
                   contributions: [{ hazard, share }], frequency_sentences: [string],
                   sources: [CitationId],
                   relief?: { help_arrives_days, mostly_restored_days, sources },
                   stress_test?: StressTest }                              # v2
StressTest { event, date, region, share_out_at_days: [[days, share], …], covered_by_target,
             sources: [CitationId] }                                        # v2
Target = { kind: "days", value, low, high }                                # duration buckets
       | { kind: "months", value, low, high }                              # income
       | { kind: "evacuate", p_need_10yr, notice_hours_low, notice_hours_high, days_away }
       | { kind: "readiness", p_need_10yr, done: u8, of: u8 }              # other readiness buckets, home_loss

RequirementLine { id, bucket, item_class, quantity, unit, per: household|person|commuter|pet, rule,
                  citations: [CitationId], plain }

PlanItem { item_id, name, kind: free_action|purchase|reserve, quantity, unit, est_cost_usd,
           price_band: { low, high }, buckets: [BucketId], hazards: [HazardId], why,
           risk_reduction, tier: TierId, done?, paid_usd?,
           requires?: [ItemId], decision?: bool }                               # v2

PlanOutput { engine_version, api_version, data_pack_version, content_version,
             location: LocationResolved, register: [HazardProfile], buckets: [BucketAssessment],
             scenarios: [ScenarioInfo], tier_reached: TierId, tier_recommended: TierId,
             plan: Plan, requirements: [RequirementLine], warnings: [Warning],
             binder: Binder, prepare_markdown,                                   # v3
             provenance: [Citation],
             recovery?: RecoveryInfo }                                           # v2
RecoveryInfo { county_declarations_5yr?: u16, sources: [CitationId] }            # v2
Plan { months: [{ index, budget_usd, items: [PlanItem] }], done_month?,
       minimum_done_month?,                                                      # v2
       envelopes: [{ item_id, saved_usd, needed_usd }],
       savings_track?: { target_months, target_usd, current_months, monthly_suggestion_usd, why },
       first_milestone?: { months, usd, by_month },                              # v2
       minimum_kit?: bool, long_horizon?: [PlanItem] }                           # v2
ScenarioInfo { id, name, applies_because, on, effect_summary, sources: [CitationId] }
Warning { id, severity: note|warn, message, why, related: [string] }
```

What the numbers mean:

- `register`: `ranked` hazards first, most important first; then the `rare_catastrophic` ones (the
  nine rare families), which the app shows in their own box, sorted by how likely here and never
  by expected loss: likelihood as a range only, "if it reaches you" in zone-conditional words,
  "why here" from `location_factor`, and "what it changes in your plan". `rate_per_year` is the
  household event rate r_h; `annual_probability` is 1 − e^(−r_h). With `range_only` set (every
  rare row, and any rate built on stacked expert judgement) the app shows only the range, never a
  point estimate. `family` names the family a rare row heads; `sub_causes` are the named causes
  inside a hazard (the EMP of a high-altitude burst, Yellowstone, a dam release), each a note with
  sources and, where known, its own rate range; they are data, not ids. `anchor_sentence` compares
  the row with the household's own list ("less likely than a house fire, about 5 in 100 for you").
- `location.zip_centroid` (v3): the ZIP code's centre from the Census 2020 ZCTA Gazetteer
  (`core/zip_centroids.csv`, DESIGN-DELTA-v3 §8), set at resolve time; the maps start there.
  Absent without a ZIP code or when the pack does not know it (and on the built-in sample
  counties).
- `binder` (v3): the during-event binder, a tree of parts, pages and blocks; see
  [The binder](#the-binder). `prepare_markdown` (v3): the Prepare sheet, a short Markdown
  document of the preparation plan (the v2 packet's plan, checklists, decisions and maintenance
  calendar conventions), cited and deterministic, printed by the Prepare and Keep it up tabs and
  by `rr plan`. Both are built inside `assess` by rr-plan (`crates/rr-plan/src/binder/` and
  `crates/rr-plan/src/packet/`); the binder's `cite` numbers are its own, numbered in the order
  it first cites them, and `provenance` starts with those sources in the same order, so `cite` n
  is `binder.sources[n - 1]` and `provenance[n - 1]`. The Prepare sheet numbers its own sources
  from 1 in its Sources section.
- `location.exposure`: what the data pack knows about the place's exposure to the v2 hazards
  (strategic class A–E, distance to a strategic site, storm-surge share, smoke days, leveed
  population, high-hazard dams, karst, landslide susceptibility, drinking-water violations,
  geomagnetic factor, UASI share, eviction filings), each value with its source, for the "Why
  here" drawers and the About data page. Absent fields are unknown, not zero.
- `buckets` come in `BucketId` order. The target's kind follows the bucket: days for the duration
  buckets, months for `income`, `evacuate` for `evacuate`, readiness for the other readiness buckets
  and for `home_loss` (an insurance-and-documents decision with no stockpile target).
  `value` is rounded to the day ladder ½, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90, 180, 365
  (`TARGET_LADDER_DAYS` in both Rust and TypeScript); `low` and `high` are the 10th and 90th
  percentiles under parameter uncertainty. `covered` is where the plan takes the household once
  every step in it is done; `covered_today` is what the household has now, before the plan buys
  anything: `existing` (what it owns and has checked off) plus the assumed basics when
  `assume_basics` is on. Both have the target's kind, days never exceed the target's days, and
  `covered_today` never exceeds `covered`; in both, `low` and `high` equal `value` and
  `p_need_10yr` repeats the target's. The plan screen's progress bars and the risks screen's
  gauges show `covered_today` as "you have now" and `covered` as "your plan covers".
  `tier_enough` is where the plan stops adding to that bucket.
- `relief` (duration buckets, where known): when outside help plausibly arrives and when service is
  mostly restored for the design event (the Oregon Resilience Plan's two-tier rating).
- `stress_test` (v2; power and water buckets where the pack has the record): the worst event in the
  region's record, how many customers were still out after so many days, and whether the target
  would have outlasted it.
- `scenarios`: named scenarios (for example `cascadia_m9`) that apply to this location, with the
  engine's default or the user's override from `dials.scenario_overrides`.
- `RequirementLine.quantity` is for the whole household, already multiplied out by `per`.
- `PlanItem.price_band` is the cost range of the whole quantity; `est_cost_usd` is the recorded price
  if there is one, otherwise the middle of the band. `done` is omitted when false.
- `Plan.months` are counted from 0: month 0 begins on the planning date, holds the free actions
  first, and receives the one-off budget. `done_month` is the month by which every bucket is covered.
  `minimum_done_month` (v2) is the month the bare-minimum kit (three days of water, light, warmth
  and medicine, as `rr-supply` marks it) is complete, for every plan: the plan's two done months are
  that one and `done_month`, and the packet's summary line uses the first.
  `envelopes` are sinking funds for items that cost more than a month's budget. `savings_track` is the
  emergency-fund goal for `income`, never funded from the supplies budget.
- `first_milestone` (v2): the first savings step, one month of expenses or $500, whichever is
  smaller, and the plan month the supplies budget reaches it once the supplies plan is done; left
  out when it is already saved or the plan never frees the budget. `minimum_kit` (v2): the plan is
  in bare-minimum mode (the kit, with the three-day life-safety items, comes before the tiers).
  `long_horizon` (v2): the long-horizon section (rain catchment, fuel storage, sanitation for months),
  present when a target reaches 30 days or `dials.long_horizon` is on; its items stay in the months
  too (the section only groups them, one line per item with its quantities added up).
- Free steps: at most eight ordinary free steps a month (month 0 lists the first eight whatever
  their kind). Decisions (`PlanItem.decision`), the long-horizon pointer, the clean-room plan and
  the 90-day-fills step sit outside that count and are all scheduled by month 1.
- `PlanItem.requires` (v2): items it needs first; the allocator never schedules it before them.
  `PlanItem.decision` (v2): an insurance or home-repair decision, not a purchase.
- `recovery` (v2): facts for the "After a disaster: the first 30 days" page, such as how many federal
  disaster declarations covered the county in the last five years.
- `warnings`: guardrails, identified by stable ids; see [Warnings](#warnings).

### Content (`docs/DESIGN.md` §4.6)

```
Citation { id, title, publisher, year?, url, retrieved, quote?, license,
           prior }                                             # defaults to false when absent
Item { id, name, category, unit, buckets: [BucketId], tier: TierId, free,
       life_safety, rare_catastrophic, assumed_basic,          # default to false when absent
       spec, look_for: [string], avoid: [string],
       price_band_usd: { low, high, per, note? }, retrieved?,  # when the price band was observed
       quantity_rule, maintenance?: { rotate_months?, check_months? }, citations: [CitationId],
       hazard_extras: [HazardId], energy_kcal_per_unit?, volume_l_per_unit?,
       requires: [ItemId], decision, long_horizon,             # v2; default to [] / false when absent
       readiness_share?: f32, alternative_group?: string,      # v2
       season?: spring|summer|fall|winter,
       test_interval_months?: u16 }                            # v2
GuidanceMeta { id, title, applies_to: [string], citations: [CitationId],
               kind?: after|plan|hazard|bucket|tier|topic|family|checklist }   # v2; checklist: v3
```

- `Citation.prior`: the source is an expert judgement, not data. Every number that cites it is shown
  as an estimate ("tagged Prior").
- `Item.life_safety`: the allocator orders it first within its tier (smoke and CO alarms, water, a
  dependent's medication, powered-device backup). `Item.rare_catastrophic`: the allocator gives it
  $0 by default and, when the household turns on `Dials.rare_catastrophic_opt_in`, at most 10% of
  the monthly budget (a radiation meter, potassium iodide only on official instruction, Faraday
  storage); off by default. `Item.assumed_basic`: credited to every household as already owned
  when `PlanInput.assume_basics` is on (see above); off by default.
- `energy_kcal_per_unit` and `volume_l_per_unit` let the app show cost per 2,000 kcal and per litre or
  gallon.
- v2: `Item.requires` lists the items an accessory needs first (batteries need the light, fuel the
  can). `readiness_share` (0 to 1) is the share of its readiness bucket's value the item carries, so
  a whistle no longer outranks a headlamp. `decision` marks an insurance or mitigation decision that
  costs the supplies budget nothing. `long_horizon` puts the item in the long-horizon section.
  `season` is its maintenance anchor: have it before the season starts, or check it then
  (meteorological seasons; summer starts 1 June with the hurricane season). `test_interval_months`
  says how often to try it; `Owned.tested_on` records the last time.
- v2: `GuidanceMeta.kind` says where a block belongs: `after` (the recovery page), `plan` (shelter
  plan, 48-hour list, communication plan), `hazard`, `bucket`, `tier`, `topic`, or `family` (a rare
  family, with its "what it changes in your plan" paragraph; `applies_to` holds the family id).
  Blocks not yet classified leave it out.
- v2 renderer conditions (implemented in `rr-content`): besides `{if:<hazard>}…{/if}`, a block may
  keep a span only for a housing kind (`{if:home:<kind>}`, `{if:not_home:<kind>}`), an access need
  (`{if:need:<access_need>}`), an item the plan holds (`{if:has:<item_id>}`) or a benefit
  (`{if:benefit:<benefit>}`). Spans do not nest.
- v3: `GuidanceMeta.kind` `checklist` marks an incident checklist (`content/checklists/`,
  DESIGN-DELTA-v3 §5.4); its `applies_to` names `hazard:<id>` and `event:<id>` targets, the events
  being the everyday emergencies in `rr_content::ids::EVENTS` (`gas_leak_or_co`,
  `missing_person`, `evacuation_order`, `shelter_in_place`, `boil_water_notice`, `power_outage`,
  `something_else`). `catalogue().guidance` lists the checklists with the other blocks. Four more
  renderer conditions, from the household's own answers: `{if:children}` (anyone under 18),
  `{if:pets}` (a pet counted in `pets`), `{if:vehicle}` (a vehicle in `mobility`) and
  `{if:powered_device}` (anyone with a powered medical device).

### Function arguments and results

```
EngineInfo { engine_version, api_version, data_pack_version?, content_version,
             packs_loaded: [string], attributions: [Attribution],
             validation?: ValidationSummary }                              # v2
ValidationSummary { events_tested, covered, partial, short, not_modelled: u16,
                    data_pack, url_anchor }                                # v2
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

`data_pack_version` is absent from `EngineInfo` until a pack is loaded. `validation` (v2) summarises
the backtest against the frozen set of past disasters in `docs/VALIDATION.md` (how many
event-and-household pairs the target covered, partly covered, fell short on or could not model),
from a table bundled with the engine, for the public `#/validation` page; absent when no table is
bundled. `catalogue().hazards` lists every hazard the engine may emit (`HazardId::ACTIVE`), so it
leaves out the retired `terrorism`.

### Engine-internal types (Rust only)

`rr-types` also defines the data contract between the engine crates, which never crosses into
JavaScript: `CountyRecord` (with `NriHazard`, `OutageStats`, `EventRate`, `Seismic`, `FloodPriors`,
`Facilities`, `Vulnerability`, and the data pack v2 `CountyExposure`), `ZipRecord` (per-ZIP
extras: dams naming the town, nearest strategic site, the optional surge and wildfire-places
packs), `BaseRate`, `HouseholdEventRate` (the `rr-hazards` →
`rr-consequence` interface), and `Effect` with `DurationDist`
(`{ kind: "log_normal", median_days, p90_days }` or `{ kind: "fixed", days }`;
σ = ln(p90 / median) / z₀.₉ with z₀.₉ = 1.2815515655446004). `Effect` and `DurationDist` are also
mirrored in `types.ts` for an expert view.

`FloodPriors.sfha_basis` names how `sfha_home_share` was computed: `"structures"` (residential
structure counts, the normal case) or `"policies_lower_bound"` (OpenFEMA reports zero flood-zone
structures for some counties that clearly have them, so the share falls back to insured flood-zone
homes ÷ all homes — a lower bound, since not everyone in the zone carries flood insurance). Absent
when the pack's `sfha_share_basis` cell is empty for that county. Not read by `rr-hazards` yet, and
not mirrored in `types.ts`: like the rest of `FloodPriors`, it never crosses into JavaScript.

## The binder

`PlanOutput.binder` (contract v3): the during-event document the household prints and puts in a
binder with ten tabs. This section copies `docs/DESIGN-DELTA-v3.md` §4.1; keep the two in step.
rr-plan renders the tree to Markdown (the CLI, the goldens, humans) and the web app to HTML and
PDF, never from Markdown.

```
Binder { title: string, generated_on: IsoDate, household: string, location: string,
         status_line: string, review_by: IsoDate, parts: [Part], sources: [SourceEntry],
         credits: [string] }
Part   { id: string, tab: u8 (1..=10), title: string, short_title: string (≤ 14 chars, the tab label),
         pages: [Page] }
Page   { id: string (unique in the binder), title: string, kind: PageKind, fit: "one" | "two" | "flow",
         blocks: [Block] }
PageKind = "cover" | "how_to_use" | "quick_start" | "index" | "contacts" | "person" | "wallet_cards"
         | "home" | "place" | "neighbourhood" | "getting_out" | "pets" | "vehicles" | "documents"
         | "inventory" | "risks_glance" | "checklist" | "after" | "log" | "sources"

Block  = { heading: { level: 1|2|3, text: string } }
       | { para: [Inline] }
       | { bullets: [[Inline]] }
       | { numbered: [[Inline]] }
       | { steps: [Step] }                       # airline-style: Step { text: [Inline], memory: bool }
       | { fields: [FieldRow] }                  # FieldRow { label: string, value?: string, lines: u8 }
       | { table: { header: [string], rows: [[[Inline]]] } }
       | { callout: { kind: "stop"|"warning"|"note"|"decision", title?: string, blocks: [Block] } }
       | { decision: { question: string, branches: [Branch] } }
                                                 # Branch { when: [Inline], then: [Inline], go_to?: string (page id) }
       | { map_slot: { id: string, kind: "region"|"area"|"neighbourhood", caption: string } }
       | { cards: [Card] }                       # Card { title: string, lines: [[Inline]] }  (wallet cards)
       | { log: { columns: [string], rows: u8 } }
       | { page_break: true }

Inline = { t: string } | { b: string } | { cite: [u32] } | { link: { to: string, text: string } }
       | { blank: u8 }                           # a ruled blank of about n characters
SourceEntry { n: u32, title: string, publisher: string, year?: u16, url?: string, expert: bool }
```

**JSON.** A block and an inline are objects with exactly one key, the variant's name:
`{"heading": {"level": 2, "text": "Do first"}}`, `{"para": [{"t": "Stay low. "}, {"cite": [3]}]}`,
`{"steps": [{"text": [{"b": "Get out."}], "memory": true}]}`, `{"blank": 24}`,
`{"page_break": true}` (never `false`). Unknown keys are rejected at every level, as everywhere
in the contract. In Rust (`rr_types::binder`), `Block` and `Inline` are enums whose variants
carry the payloads as structs named `Heading`, `Table`, `Callout`, `Decision`, `MapSlot`, `Log` and
`Link`, with the enums `PageKind`, `Fit`, `CalloutKind` and `MapSlotKind`; `page_break`'s
payload is the marker `True` (`Block::page_break()`). In `types.ts` each variant is an interface
(`HeadingBlock { heading: Heading }` … `PageBreakBlock { page_break: true }`, `TextInline { t }`,
`BoldInline { b }`, `CiteInline { cite }`, `LinkInline { link }`, `BlankInline { blank }`)
in the unions `Block` and `Inline`; narrow them with `'steps' in block`.

**Rules.** `Binder::check` (Rust; rr-plan, the CLI and the tests share it) returns every
structural problem, in a fixed order: a part whose `tab` is outside 1 to 10 or not above the one
before (`MAX_TABS`); a part id used twice; a part with no pages; a tab label over 14 characters
(`SHORT_TITLE_MAX`); a page id used twice anywhere; a `link.to` or `Branch.go_to` that names no
page; a `cite` number outside 1 to `sources.len()`; a source whose `n` is not its place in the list
(`sources[i].n == i + 1`, so `cite` n is `sources[n - 1]`); a heading level outside 1 to 3.
`fields.value` is user text, printed as written, or absent, in which case the renderer draws
`lines` ruled lines; `steps` with `memory: true` are the "do first from memory" items and render
bold; `map_slot` is filled by the web app (DESIGN-DELTA-v3 §9) and rendered by the CLI as a
boxed placeholder ("Map: your neighbourhood. Add it in the app, or paste a printed map here.");
`fit: "one"` is a promise the page fits one printed page, enforced by a word-and-row proxy in
rr-plan (DESIGN-DELTA-v3 §5.6) and by a real print in verification.

**Renderers.** Markdown (rr-plan, for the CLI, the goldens and humans): parts as `#`, pages as
`##`, blocks in the v2 packet's Markdown conventions, blanks as `__________`, cross-references as
"(Tab 3, Home)". HTML and PDF in the web app (DESIGN-DELTA-v3 §6). The goldens in
`fixtures/golden/` stay Markdown.

**Parts.** Ten, in a fixed order (DESIGN-DELTA-v3 §4.2): 1 `start` (Start here), 2 `people`
(People), 3 `home_places` (Home and places), 4 `pets_vehicles_documents` (Pets, vehicles and
documents), 5 `have` (What you have), 6 `check_now` (Checklists: happening now), 7 `check_coming`
(Checklists: it is coming), 8 `check_ongoing` (Checklists: it goes on), 9 `after` (After), 10
`sources` (Sources). Every part starts on a new page; page numbers are one running count
through the whole binder ("page 37 of 112"), so the table of contents and every cross-reference
point to a single number.

## Ids

Ids are stable snake_case strings. `rr-types` exposes them as enums with `ALL`, `as_str()`,
`FromStr` and `Display`; `types.ts` exposes each as an `as const` list and a union type.
`catalogue()` returns the plain names, so the UI never hard-codes them.

- **Hazards** (54 ids, 53 active; `docs/DESIGN.md` §4.2): 23 natural (the 18 NRI hazards, then
  `wildfire_smoke`, `dust_storm`, `sinkhole`, `geomagnetic_storm`, `vei7_eruption`), 20 societal
  (the nine of v1, then `dam_failure`, `network_outage`, `drug_shortage`, `benefit_interruption`,
  `attack_disruption`, `multi_month_blackout`, `war_infrastructure`, `cbrn_attack`,
  `severe_pandemic`, `financial_crisis`, `mass_violence`), 11 personal (the eight of v1, then
  `water_damage`, `eviction`, `arrest_or_detention`). `HazardId::is_nri` marks the 18 whose county
  rates come from the National Risk Index.
- **Retired hazard ids**: `terrorism` (retired in v2; `#[deprecated]` in Rust, `RETIRED_HAZARD_IDS`
  in `types.ts`). It still parses so v1 plans load; it is never emitted and is left out of
  `HazardId::ACTIVE` and `catalogue().hazards`. Its two halves are `attack_disruption` (ranked: an
  attack or threat closes your area) and `mass_violence` (rare: being caught up in a shooting or
  bombing).
- **Rare families** (9): each rare hazard heads one family, and the family id is that hazard's id
  (`HazardId::family`; `RARE_HAZARD_IDS` in `types.ts`). Everything else in a family is a
  sub-cause. `Dials.rare_opt_in` takes these ids or `"all"`.

  | Family id | Shown as | Sub-causes (examples) |
  | --- | --- | --- |
  | `geomagnetic_storm` | Severe solar storm | asteroid or comet |
  | `vei7_eruption` | Very large volcanic eruption | Yellowstone |
  | `nuclear_attack` | Nuclear attack | limited strike, device in a city, use abroad, EMP |
  | `multi_month_blackout` | Power out for months (any cause) | computed from the power curve and the rows above |
  | `war_infrastructure` | War with attacks on US infrastructure | — |
  | `cbrn_attack` | Chemical, biological or radiological attack | chemical, biological, radiological |
  | `severe_pandemic` | Severe pandemic | 1918-class, engineered |
  | `financial_crisis` | Financial crisis with bank closures | — |
  | `mass_violence` | Mass shooting or bombing | — |

- **Buckets** (15): duration `power`, `water_boil`, `water_out`, `supplies`, `thermal`,
  `medication`, `comms`; readiness `evacuate`, `get_home`, `medical_emergency`, `fire`, `security`,
  `clean_air` (v2: "Unhealthy air indoors", a checklist sized by smoke and dust days); money
  `income`, `home_loss`.
- **Tiers** (7): `now` (0 days), `h72` (3), `w2` (14), `m1` (30), `m3` (90), `m6` (180), `y1` (365).
  The get-home bag is an item in the `get_home` bucket, not a tier.
- **Return periods**: `one_in_10` "Common disruptions", `one_in_50` "Serious", `one_in_100` "Very
  serious" (default), `one_in_500` "Rare catastrophes".
- **Seasons** (v2): `spring`, `summer`, `fall`, `winter`. **Guidance kinds** (v2): `after`, `plan`,
  `hazard`, `bucket`, `tier`, `topic`, `family`, and `checklist` (v3).
- **Places** (v3, `Place.kind`; `PLACE_KINDS`): `work`, `school`, `childcare`, `other`.
- **Binder** (v3): page kinds (`PAGE_KINDS`), page fits `one`, `two`, `flow` (`PAGE_FITS`), callout
  kinds `stop`, `warning`, `note`, `decision` (`CALLOUT_KINDS`), map slots `region`, `area`,
  `neighbourhood` (`MAP_SLOT_KINDS`); see [The binder](#the-binder).

## Warnings

`Warning.id` is a stable string; the app can react to an id, never to the message. v2 adds the six
marked below (`rr_types::Warning::V2_IDS`) and the `simultaneous_need` note.

| Id | Emitted by | When |
| --- | --- | --- |
| `zero_budget` | rr-budget | no monthly and no one-off money |
| `device_power_plan` | rr-budget | a powered medical device and no backup power by month 3 |
| `cold_chain_plan` | rr-budget | refrigerated medicine and no way to keep it cool by month 3 |
| `no_stored_water_by_month_3` | rr-budget | no stored water beyond refilled bottles by month 3 |
| `smoke_alarms_landlord` | rr-budget | renters with no working smoke alarms |
| `evacuation_no_go_bag` | rr-budget | a 10 % ten-year chance of leaving and no go-bag by month 6 |
| `insurance_flood`, `insurance_quake` | rr-budget | an owner in a flood- or quake-prone area without that policy |
| `uncovered_<bucket>` | rr-budget | the plan ran out of things to buy before the bucket's goal |
| `cliff_<bucket>` | rr-consequence | one event near the dial drives the bucket's target |
| `assumed_basics`, `unknown_existing_items` | rr-plan | basics were credited; unknown item ids were ignored |
| `citation_missing` | rr-plan | a number points to a source still being added |
| `surge_zone_stay_home` (v2) | rr-budget | a surge zone or a likely evacuation, and a plan that never says to leave (REVIEW S2) |
| `cold_chain_power` (v2) | rr-budget | refrigerated medicine that needs a power source for a power target of 2 days or more, and none planned (S1) |
| `benefit_lapse` (v2) | rr-budget | a household relying on federal pay or a benefit with no food buffer by month 3 (H7) |
| `plan_too_long` (v2) | rr-budget | the full plan would run past 36 months; bare-minimum mode takes over (R6). `related` lists the items that fall beyond three years even so (the plan screen's deferred list) |
| `no_raw_water_source` (v2) | rr-budget | a water filter in the plan and no raw water source named (S6) |
| `no_cooking_capability` (v2) | rr-budget | no way to cook or boil water without power (K1) |
| `simultaneous_need` (v2) | rr-budget | a note: the plan is done, but one event that sets a target would need more stored water, food or power at once than the plan holds (DESIGN §4.7's simultaneous-need check, from rr-consequence) |

## Loading

How the web app gets the engine and its data (`crates/rr-wasm`, `web/src/engine/wasm.ts`,
`web/src/engine/loader.ts`).

**Build.** `bash crates/rr-wasm/build-web.sh` compiles `rr-wasm` with wasm-pack (`--target web`,
the release profile at opt-level `s` unless `CARGO_PROFILE_RELEASE_OPT_LEVEL` says otherwise, then
`wasm-opt -Os`; never `--profile`) into `web/public/pkg/` (`rr_wasm.js`, `rr_wasm_bg.wasm`) and
copies `data/manifest.json`, `data/core/`, `data/geo/` and `data/places/` into `web/public/data/`
(all git-ignored). It prints the raw and gzipped size of the `.wasm`; the budget is 1.5 MB gzipped,
content included (DESIGN-DELTA-v3 §10 raises the shipped budget to 1.75 MB; this script's constant
is unchanged). The Pages workflow runs it before `npm run build`. The site talks to the
WebAssembly engine when `web/public/pkg/rr_wasm.js` exists at build time or `VITE_ENGINE=wasm`,
otherwise to the mock (`VITE_ENGINE=mock` forces it). v0.3.0 raises the budget to 1.75 MB gzipped
(DESIGN-DELTA-v3 §10: the checklist blocks and the binder code are new).

**Start.** `getEngine()` imports `pkg/rr_wasm.js` from the site's own origin, instantiates
`pkg/rr_wasm_bg.wasm`, checks that `engine_info().api_version` equals the app's
`ENGINE_API_VERSION`, and resolves at once: the catalogue and the defaults are built into the
engine, so the first screens work while the data loads behind them (a progress line under the
header says how far it has got). Calls that need data wait for the part they need
(`gateEngine` in `web/src/engine/wasm.ts`), so no screen ever sees a plan from the built-in
sample counties while real data is on its way; an answer that never depends on data (`bad_input`)
comes back at once.

**Data, in this order.** `load_pack(name, bytes)` takes one file per call, named by its path in
`data/manifest.json` (`web/src/engine/loader.ts`; the file groups are in
`web/src/engine/data-files.ts`).

1. `manifest.json` first. The engine checks every later file against the sha256 it records (a file
   loaded before the manifest is checked when the manifest arrives).
2. Every file listed under `packs.core.files` except the ZIP tables, fetched at once and
   loaded one by one with `core/counties.csv` last. The engine reassembles the county records
   after each file, which is only real work once the county list is in, so this order is the
   fastest; any order gives the same answers. Since v0.3.0 this includes the eviction column and
   the wildfire-by-place and per-event outage tables bundled from their optional packs
   (DESIGN-DELTA-v3 §8); a first visit is correspondingly larger (about 3.5 MB gzipped, up from
   2.4 MB in v0.2.0).
3. The ZIP tables (`core/zip_county.csv`, `core/zip_facilities.csv`, `core/zip_surge.csv`,
   `core/zip_centroids.csv`, `core/zip_wildfire_places.csv`; `ZIP_FILES` in `crates/rr-wasm/src/source.rs`) when a location has a
   real ZIP code: the app starts fetching them when someone starts typing one, or when a saved
   plan has one. Only ZIP lookups read them, so a first visit is about 0.76 MB lighter, and someone
   who finds their county by name never downloads them.
4. `geo/counties.json` (county outlines for the map) only when a map is shown; nothing else needs
   it.
5. `places/hospitals.csv` (county hospitals with emergency services) only when the binder's
   Neighbourhood page is shown (`PackLoader.places()`); nothing else needs it. Like `geo`, it is
   its own manifest pack, not part of `packs.core`.

File URLs carry `?v=<pack_version>`, so a cached manifest is always paired with the files it
describes. The service worker precaches `data/manifest.json` with the app (each installed version
reads the data it was built with; new data arrives with the next version) and the files of step 2
by their versioned address, in a data cache that survives app updates; the ZIP tables and the map
are cached the first time they are fetched. After one visit the app works offline.

**Which data answers.** `engine_info()` tells the app which of four states the engine is in:

| State | `packs_loaded` | `data_pack_version` | `assess`, `resolve_location`, `county_search` |
| --- | --- | --- | --- |
| No pack file loaded | `[]` | absent | The seven built-in sample counties, the fixture households' counties; any other place is `unknown_zip` or `unknown_county`. Plans carry `data_pack_version` `fixtures+…`, as the goldens do. The app shows "Sample counties only". |
| Part-way: the manifest and some core files | the loaded files' paths | the manifest's `pack_version` | `pack_missing`, naming how many of the county files are in: the engine never plans from part of the core pack |
| The manifest and every core file except the ZIP tables | the loaded files' paths | the manifest's `pack_version` | Counties answer from the national data, exactly as with the whole pack (`county_search`, a location or plan by county code). Any location with a ZIP code answers `pack_missing` ("The list of ZIP codes has not loaded yet") until the ZIP tables are in: a ZIP code decides the county and the facility distances. |
| The manifest and every core file | `["core"]`, then `["core", "geo"]` and/or `["core", "places"]` as those load | the manifest's `pack_version` | The national data. Attributions come from the manifest, the National Risk Index statement first. |

Once any pack file is loaded the packs decide, and the sample counties no longer answer.

**Errors.**

- `load_pack` answers `bad_input` (one `schema` problem on `name`) for a file name the engine does
  not know, and `pack_corrupt` for a file whose sha256 differs from the manifest's, a file the
  loaded manifest does not list, or one that cannot be decoded. A refused file changes nothing;
  files loaded before it stay loaded.
- The loader treats a missing `data/manifest.json` (HTTP 404, or the dev server's HTML page in its
  place) as a site built without data, and the engine keeps its sample counties.
- If the engine itself cannot start (the module fails to load, or its contract version does not
  match), `getEngine()` falls back to the mock engine, with the reason on the About screen.
- If data fails once the engine runs (a download that fails, a file the engine refuses), the calls
  that need that data answer `pack_missing` with the reason, the progress line says what happened
  and offers "Try again", and nothing else changes. The mock never stands in for the real engine
  at that point: its placeholder numbers must not replace real ones mid-visit.
- A panic inside the engine is a bug. The call traps, the adapter turns the trap into `internal`,
  and every later call answers `internal` with the panic message rather than trapping again; the
  page should be reloaded.

## Mock engine

`web/src/engine/mock.ts` returns deterministic, plausible values for every function, keyed on the
fixture households (`web/src/engine/fixtures.ts`), so screens can be built and screenshot-tested
before the engine exists. A parity test asserts that the mock and wasm outputs have identical JSON
shapes for each fixture.

The engine's binder is the real one (rr-plan's `binder` module, merged for v0.3.0): the ten
parts of DESIGN-DELTA-v3 §4.2 with every block kind in use (headings, paragraphs, bullets,
numbered lists, steps, fields, tables, callouts, decisions, map slots, cards, logs), and
`prepare_markdown` is the Prepare sheet. The transitional shim (`packet/shim.rs`, a page of
`para` blocks per v2 packet section) is gone from the engine. The mock's `assess` must give a
binder of the same shape (the parity test above); its transitional builder,
`web/src/engine/mock/binder-shim.ts`, still makes the shim's one-paragraph pages from the mock's
own packet until the web binder workstream replaces it, so until then the two binders do not
have the same shape (the mock's pages hold only `para` blocks).

## Changes from v2

Contract v3 ships with v0.3.0. It comes from `docs/DESIGN-DELTA-v3.md` §3 and §4, which turn the
owner's request and the interview decisions of 2026-09-27 (D1–D6 in the delta's §0) into contract
changes. Every input addition is optional; the one breaking change is the removal of
`PlanOutput.packet_markdown`.

**Loading older files.** A v1 or v2 plan parses unchanged: every new input field is optional and
a group left empty is omitted from the JSON (tested on every v1 and v2 household in the
repository). An output stored under v1 or v2 no longer parses as a v3 `PlanOutput` (it has
`packet_markdown` and no `binder`); the app recomputes outputs on load and never reads a stored
one, and the goldens are regenerated under v3.

### Inputs

| Field | Type | When absent | Why |
| --- | --- | --- | --- |
| `Person.profile` | `PersonProfile` (with `Place`, `PlaceKind`, `Medication`, `HealthInsurance`) | absent (not answered) | step 6, "Your people": the person's binder page and wallet card (§2.1, §3.1; D2) |
| `Contact.address` | string, 200 characters | absent | addresses of the hospital, the pharmacy, a doctor, a shelter (§3.1) |
| `FamilyPlan.home` | `HomeInfo` | absent | step 7, "Your home": address, utilities and outage numbers, insurer, landlord, where things are (§2.2, §3.2; D4) |
| `FamilyPlan.neighbourhood` | `Neighbourhood` | absent | step 7: hospital, urgent care, pharmacy, shelter, county emergency office, alerts (§2.2) |
| `FamilyPlan.pets` | `[PetInfo]`, at most 8 | `[]`, left out | step 8: one entry per animal (§2.3) |
| `FamilyPlan.vehicles` | `[VehicleInfo]`, at most 4 | `[]`, left out | step 8: one entry per vehicle (§2.3) |
| `FamilyPlan.documents` | `DocumentsInfo` (with `AccountInfo`, `PolicyInfo`) | absent | step 8: accounts with their last four digits only, policies, where the papers are (§2.3; planner decision: no full account numbers) |

All of it is echo-only, like the v2 family plan: `PlanInput::tidy` trims every string and cuts it
at its cap, keeps lists to their length, drops empty entries and groups, and keeps only the last
four digits of `AccountInfo.last4`; nothing is computed with it, and nothing in it is ever
required or a problem. The caps are exported in Rust and `types.ts` (see [Types](#types)).

### Outputs

| Field | Type | When empty | Why |
| --- | --- | --- | --- |
| **`PlanOutput.packet_markdown`** | **removed (breaking)** | — | the packet becomes the during-event binder and the Prepare sheet (§4; owner request) |
| `PlanOutput.binder` | `Binder` (with `Part`, `Page`, `PageKind`, `Fit`, `Block`, `Heading`, `Step`, `FieldRow`, `Table`, `Callout`, `CalloutKind`, `Decision`, `Branch`, `MapSlot`, `MapSlotKind`, `Card`, `Log`, `Inline`, `Link`, `SourceEntry`) | always present | the binder: ten tabs, a page per person, place and matter, airline-style checklists (§4.1, §4.2; D1, D3, D4) |
| `PlanOutput.prepare_markdown` | string (Markdown) | always present | the Prepare sheet the Prepare and Keep it up tabs print (§4; D1) |
| `LocationResolved.zip_centroid` | `LatLon` | left out | the ZIP code's centre, where the maps start (§3.3, §8; D5) |

### Content and ids

| Change | Ids | Why |
| --- | --- | --- |
| New guidance kind | `checklist`: incident checklists in `content/checklists/`, listed in `catalogue().guidance` | §5.4; D3 |
| New enums | `PlaceKind` (`work`, `school`, `childcare`, `other`), `PageKind` (20), `Fit` (`one`, `two`, `flow`), `CalloutKind` (`stop`, `warning`, `note`, `decision`), `MapSlotKind` (`region`, `area`, `neighbourhood`) | §3.1, §4.1 |
| New content targets | `event:<id>` in a checklist's `applies_to`, the ids in `rr_content::ids::EVENTS`: `gas_leak_or_co`, `missing_person`, `evacuation_order`, `shelter_in_place`, `boil_water_notice`, `power_outage`, `something_else` | §3.3, §5.5 |
| New renderer conditions | `{if:children}`, `{if:pets}`, `{if:vehicle}`, `{if:powered_device}` (`rr_content::policy::HouseholdFacts::has_children`, `has_pets`, `has_vehicle`, `has_powered_device`) | §3.3, §5.4 |

`types.ts` gains the matching lists (`PLACE_KINDS`, `PAGE_KINDS`, `PAGE_FITS`, `CALLOUT_KINDS`,
`MAP_SLOT_KINDS`, and `checklist` in `GUIDANCE_KINDS`), the caps (`LAST4_LEN`, `BLOOD_TYPE_MAX`,
`PLATE_MAX`, `SHORT_TEXT_MAX`, `MEDIUM_TEXT_MAX`, `LABEL_TEXT_MAX`, `DESCRIPTION_MAX`,
`LONG_TEXT_MAX`, `NOTE_MAX`, `MEDICATIONS_MAX`, `PETS_MAX`, `VEHICLES_MAX`, `ACCOUNTS_MAX`,
`POLICIES_MAX`) and the binder's `MAX_TABS` and `SHORT_TITLE_MAX`; the mirror test checks all of
them.

**Where the names differ from the delta's first draft.** The person's health insurance is the type
`HealthInsurance` (JSON field `insurance`, as the delta says): `Insurance` already names the
household's insurance answers in `Finances`, in Rust and in `types.ts`. The delta's anonymous
payloads are named `Heading`, `Table`, `Callout`, `Decision`, `MapSlot`, `Log` and `Link`, and its
lists that are omitted when empty carry `?` here (`pets?`, `vehicles?`, `medications?`,
`accounts?`, `policies?`). `docs/DESIGN-DELTA-v3.md` records the same.

## Changes from v1

Contract v2 ships with v0.2.0. It comes from `DESIGN-DELTA.md` §1 (in
`~/Desktop/ready-reckoner-briefs/round2/phase2/`), which turns the round-2 review
(`../REVIEW.md`; finding ids below) and the owner's decisions of 2026-09-26 into contract changes.
Everything is additive except the retired `terrorism` id.

**Loading v1 files.** A v1 plan (`rr.plan.v1`) parses unchanged: every new input field is optional
or has a default, and `terrorism` still parses. A v1 `PlanOutput` parses and re-serialises byte for
byte (tested against the goldens), because every new output field is left out when empty. The app
recomputes outputs on load and ignores ids it does not know in a stored output.

### Inputs

| Field | Type | When absent | Why |
| --- | --- | --- | --- |
| `Person.access_needs` | `[AccessNeed]`: `hearing`, `vision`, `limited_english`, `cognitive`, `supervision`, `service_animal`, `dialysis`, `home_health` | `[]` | CMIST needs for the communication plan, registries and evacuation help (§1.1; REVIEW N3) |
| `Housing.below_grade_bedroom` | bool | `false` | someone sleeps below street level: flash-flood card and water alarm (§1.1; R3, backtest M-09) |
| `Housing.cooking` | `electric` \| `gas` \| `induction` \| `none` | absent (not asked) | cooking capability; a gas range activates the boil-water coupling (§1.1; K1) |
| `Housing.raw_water_source` | `none` \| `well` \| `surface_nearby` \| `rain_barrel` \| `neighbour_well` | absent (counts as none) | a filter's coverage counts only with a source (§1.1; S6) |
| `Housing.water_system_record` | `fine` \| `occasional_notices` \| `frequent_problems` \| `unknown` | absent (counts as unknown) | self-report combined with SDWIS for water-system fragility (§1.1; R2) |
| `Finances.benefits` | `[Benefit]`: `federal_pay`, `snap_wic`, `ssi_ssdi`, `va`, `unemployment` | `[]` | the benefit-interruption hazard, shown only to households that tick one (§1.1; H7) |
| `Insurance.sewer_backup` | bool | absent (not asked) | insurance decisions (§1.1; N4) |
| `Insurance.life_or_disability` | bool | absent (not asked) | insurance decisions; the earner-death row gets an action (§1.1; N4) |
| `Owned.tested_on` | date | absent | "tested?" on the Have screen for items with `Item.test_interval_months` (§4a, Deviant Ollam lessons) |
| `Dials.rare_opt_in` | `[string]`: family ids, or `["all"]` | `[]` | rare allowance by family (§1.1; H8, REVIEW §2.4). `rare_catastrophic_opt_in` stays and maps to `["all"]` (`Dials::rare_families()`) |
| `Dials.minimum_kit` | bool | `false` | bare-minimum mode (§1.1; R6); the engine also warns past 36 months |
| `Dials.long_horizon` | bool | `false` | show the long-horizon section below 30-day targets (§1.1) |
| `Dials.legal_opt_in` | bool, left out when false | `false` | a legal-emergency amount (bail and a lawyer) beside the savings goal, turned on from the arrest row of the register (owner decision 2026-09-26; budget2) |
| `PlanInput.family_plan` | `FamilyPlan` (with `Contact`, `TrustedPerson`, `Holds`) | absent | the device-only family-plan screen, printed in the packet and on wallet cards; never computed with; trimmed and length-capped, never required (§1.1; N1; trusted circle, lawyer, roadside number and numbers by heart from §4a) |

### Ids

| Change | Ids | Why |
| --- | --- | --- |
| 10 new ranked hazards | `water_damage` (personal), `wildfire_smoke`, `dust_storm`, `sinkhole` (natural), `dam_failure`, `network_outage`, `drug_shortage`, `benefit_interruption`, `attack_disruption` (societal), `eviction` (personal) | §1.2; REVIEW §2.2, H4–H7 |
| 1 new ranked personal hazard | `arrest_or_detention` | §1.2, owner decision 2026-09-26 (Deviant Ollam talk) |
| 8 new rare families | `geomagnetic_storm`, `vei7_eruption` (natural), `multi_month_blackout`, `war_infrastructure`, `cbrn_attack`, `severe_pandemic`, `financial_crisis`, `mass_violence` (societal) | §1.2; REVIEW §2.3, H1–H3 |
| Kept, now a family | `nuclear_attack` (its EMP, limited-strike, nuclear-terrorism and use-abroad rows are sub-causes) | §1.2; H1, H3 |
| **Retired (breaking)** | `terrorism`: parses, never emitted, not in `ACTIVE` or the catalogue | §1; H2 |
| New bucket | `clean_air` ("Unhealthy air indoors"), readiness; the 15th bucket | §1.2; H6, owner decision |
| New enums | `AccessNeed`, `CookingFuel`, `RawWaterSource`, `WaterSystemRecord`, `Benefit`, `Holds`, `Season`, `GuidanceKind` | as above |
| New problem code | `unknown_id` | a `rare_opt_in` entry that names no family |

The natural tier now means weather, geology, fire and space weather: the 18 NRI hazards
(`HazardId::is_nri`) plus five the Index does not cover. Engine crates iterate `HazardId::ACTIVE`
when they build output and `HazardId::RARE` for the families.

### Outputs

| Field | Type | When empty | Why |
| --- | --- | --- | --- |
| `HazardProfile.family` | string (the family id) | left out (ranked rows) | rare rows shown as families (§1.3; REVIEW §2.4) |
| `HazardProfile.sub_causes` | `[SubCause { id, name, note, rate_range?, sources }]` | left out | the 52 named sub-causes; data, not ids (§1.3; H9) |
| `HazardProfile.location_factor` | `LocationFactor { class, label, multiplier: [lo, mid, hi], sources }` | left out | "Why here": strategic class, metro tier, magnetic latitude (§1.3; H1, REVIEW §2.3) |
| `HazardProfile.range_only` | bool | left out when false | range-only display for priors (§1.3; H1) |
| `HazardProfile.anchor_sentence` | string | left out | one comparison with the household's own list (§1.3; REVIEW §2.4) |
| `HazardProfile.if_it_reaches_you` | string | left out | zone-conditional severity words (§1.3; H9) |
| `HazardProfile.what_it_changes` | string | left out | "what it changes in your plan" (§1.3; REVIEW §2.4) |
| `LocationResolved.exposure` | `Exposure` of optional `Sourced { value, source }` fields | left out when every field is unknown | "Why here" drawers and the About data page (§1.3, §2) |
| `BucketAssessment.stress_test` | `StressTest { event, date, region, share_out_at_days, covered_by_target, sources }` | left out | the worst event in the region's record (§1.3; R10) |
| `Plan.first_milestone` | `SavingsMilestone { months, usd, by_month }` | left out | a first savings step within reach (§1.3; N4) |
| `Plan.minimum_kit` | bool | left out when false | bare-minimum mode is on (§1.3; R6) |
| `Plan.minimum_done_month` | u16 | left out | the month the bare-minimum kit is complete, for every plan; the first of the two done months (R6, M-12; budget2) |
| `Plan.long_horizon` | `[PlanItem]` | left out | the long-horizon section (§1.3) |
| `PlanItem.requires` | `[ItemId]` | left out | accessories never before their device (§1.3; K4) |
| `PlanItem.decision` | bool | left out when false | insurance and mitigation decisions, no purchase (§1.3; N4, N5) |
| `PlanOutput.recovery` | `RecoveryInfo { county_declarations_5yr?, sources }` | left out | the recovery page (§1.3; N2) |
| `EngineInfo.validation` | `ValidationSummary { events_tested, covered, partial, short, not_modelled, data_pack, url_anchor }` | left out | the public validation page (§1.3; R10, C3) |
| `Warning.id` | six new ids (see [Warnings](#warnings)) | — | §1.3; S1, S2, S6, H7, R6, K1 |

### Content and catalogue

| Field | Type | When absent | Why |
| --- | --- | --- | --- |
| `Item.requires` | `[ItemId]` | `[]` | §1.3; K4 |
| `Item.readiness_share` | f32, 0 to 1 | absent | per-item readiness value (§1.3; K3) |
| `Item.alternative_group` | string | absent | items that do the same readiness job share one credit: once one is owned or planned, the others earn no readiness value (K3; budget2) |
| `Item.decision` | bool | `false` | decision items (§1.3; N4, N5) |
| `Item.long_horizon` | bool | `false` | long-horizon section (§1.3) |
| `Item.season` | `spring` \| `summer` \| `fall` \| `winter` | absent | seasonal maintenance anchors and season-aware ordering (§1.3; N8) |
| `Item.test_interval_months` | u16 | absent | "Test" maintenance items (§4a) |
| `GuidanceMeta.kind` | `after` \| `plan` \| `hazard` \| `bucket` \| `tier` \| `topic` \| `family` | absent (not yet classified) | where a block prints (§1.3) |
| renderer conditions | `{if:home:…}`, `{if:not_home:…}`, `{if:need:…}`, `{if:has:…}`, `{if:benefit:…}` | — | §1.3; implemented in `rr-content` |
| `catalogue().hazards` | the 53 active hazards | — | the retired id is left out |

`types.ts` gains the matching lists (`RARE_HAZARD_IDS`, `RETIRED_HAZARD_IDS`, `ACCESS_NEEDS`,
`COOKING_FUELS`, `RAW_WATER_SOURCES`, `WATER_SYSTEM_RECORDS`, `BENEFITS`, `HOLDS`, `SEASONS`,
`GUIDANCE_KINDS`) and the family-plan limits (`FAMILY_PLAN_TEXT_MAX`, `FAMILY_PLAN_SHORT_MAX`,
`TRUSTED_CIRCLE_MAX`, `ROUTES_MAX`, `NUMBERS_BY_HEART_MAX`); the mirror test checks all of them.
