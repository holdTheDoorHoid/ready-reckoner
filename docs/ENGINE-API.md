# Engine API (contract between `rr-wasm` / `rr-cli` and the web app)

`ENGINE_API_VERSION = 1`. Bump it whenever a type, id or function below changes shape; update
`crates/rr-types`, `web/src/engine/types.ts` and `web/src/engine/mock.ts` in the same commit.

All functions take and return JSON strings (wasm-bindgen `String`), so the mock engine in TypeScript
and the WebAssembly engine are interchangeable behind `web/src/engine/index.ts`. Every function
returns an envelope:

```
{ "ok": true,  "value": <result> }
{ "ok": false, "error": { "code": "<snake_case>", "message": "<plain language>", "details"?: any } }
```

Error codes: `bad_input` (schema), `unknown_zip`, `unknown_county`, `pack_missing`, `pack_corrupt`,
`internal`.

## Functions

| Function | Input | Output |
| --- | --- | --- |
| `engine_info()` | — | `EngineInfo { engine_version, api_version, data_pack_version, content_version, packs_loaded: [string] }` |
| `load_pack(name, bytes)` | pack name, bytes (Uint8Array) | `{ name, version, rows }` — the web app fetches packs (same origin) and hands the bytes in; the engine never fetches |
| `county_search(query)` | free text ("phila", "42101", "Cook, IL") | `[LocationResolved]` up to 10 |
| `resolve_location(json)` | `LocationInput` | `LocationResolved` or `unknown_zip` / `unknown_county` with `details.suggestions: [LocationResolved]` |
| `assess(json)` | `PlanInput` | `PlanOutput` |
| `explain(json)` | `{ kind: "hazard" \| "bucket" \| "item" \| "requirement" \| "warning", id, input: PlanInput }` | `Explanation { title, plain: [string], math?: [string], sources: [Citation] }` |
| `catalogue()` | — | `{ items: [Item], citations: [Citation], guidance: [GuidanceMeta] }` |
| `defaults()` | — | a valid `PlanInput` skeleton with every field present (the interview starts from it) |

`assess` must be pure and fast (target < 50 ms in wasm for any fixture): the UI calls it on every
dial change.

## Types

Defined in `crates/rr-types` (Rust, serde) and mirrored by hand in `web/src/engine/types.ts`.
Field names are snake_case in JSON. Enums serialise as their snake_case string ids. Optional fields are
omitted when null. Money is `f32` USD; days are `f32`; dates are ISO `YYYY-MM-DD`.

Input types are those in `docs/DESIGN.md` §4.1 (`PlanInput`, `LocationInput`, `Housing`, `Person`,
`Pets`, `Mobility`, `Finances`, `Owned`, `Dials`).

```
LocationResolved {
  country: "US", county_fips: "42101", county_name: "Philadelphia", state_abbr: "PA", state_name,
  zip?: "19147", zip_county_share?: 1.0,          # share of the ZIP inside this county (crosswalk)
  centroid: { lat, lon }, nca_region: "northeast", coastal: bool, tsunami_zone: bool,
  facility_flags: { nuclear_plant_within_16km: bool, nuclear_plant_within_80km: bool, hazmat_facilities_within_5km: u16 },
  data_note?: string                                  # e.g. "your county; tract-level data not yet loaded"
}

HazardProfile { id, name, tier: natural|societal|personal, annual_probability, probability_range: [lo, hi],
                severity, eal_per_household_usd?, climate_multiplier, confidence, sources: [CitationId],
                frequency_sentence: string, buckets: [BucketId] }

BucketAssessment { id, name, target: Target, covered: Target, tier_enough: TierId,
                   contributions: [{ hazard: HazardId, share: f32 }],
                   frequency_sentences: [string], sources: [CitationId] }
Target = { kind: "days", value } | { kind: "months", value } | { kind: "evacuate", notice_hours, days_away }
       | { kind: "readiness", done: u8, of: u8 }

RequirementLine { id, bucket, item_class, quantity, unit, per: household|person|commuter|pet, rule: string,
                  citations: [CitationId], plain: string }

PlanItem { item_id, name, kind: free_action|purchase|reserve, quantity, unit, est_cost_usd, price_band: {low, high},
           buckets: [BucketId], hazards: [HazardId], why: string, risk_reduction: f32,
           tier: TierId, done?: bool, paid_usd?: f32 }

PlanOutput { engine_version, api_version, data_pack_version, content_version,
             location: LocationResolved, register: [HazardProfile], buckets: [BucketAssessment],
             tier_reached: TierId, tier_recommended: TierId,
             plan: { months: [{ index, budget_usd, items: [PlanItem] }], done_month?: u16, envelopes: [{ item_id, saved_usd, needed_usd }] },
             requirements: [RequirementLine], warnings: [Warning], packet_markdown: string,
             provenance: [Citation] }

Warning { id, severity: note|warn, message, why, related: [string] }
Citation { id, title, publisher, year?, url, retrieved, quote?, license }
Item { id, name, category, unit, buckets, tier, free, spec, look_for: [string], avoid: [string],
       price_band_usd: { low, high, per, note? }, quantity_rule, maintenance?: { rotate_months?, check_months? },
       citations, hazard_extras: [HazardId] }
```

## Ids

Hazard, bucket and tier ids are listed in `docs/DESIGN.md` §4.2, §4.3, §4.5. `rr-types` exposes them
as enums with `as_str()` / `FromStr`, and `catalogue()` returns the human names so the UI never
hardcodes them.

## Mock engine

`web/src/engine/mock.ts` returns deterministic, plausible values for every function, keyed on the
fixture households, so screens can be built and screenshot-tested before the engine exists. A parity
test asserts that the mock and wasm outputs have identical JSON shapes for each fixture.
