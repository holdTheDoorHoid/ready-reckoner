# Roadmap

Phases are ordered by dependency. Within a phase, workstreams run in parallel as separate agents in
separate worktrees. Each workstream becomes a GitHub issue; the issue is the brief's summary and the
place to report results.

## Phase 0 — Plan and scaffold (2026-09-25)

Interview, decisions, `docs/`, workspace skeleton, CI, public repository, milestone issues. Done in
the founding session.

## Phase 1 — Foundations (parallel)

Tier 0 (gates everything): `rr-types` shared contract + `docs/ENGINE-API.md` + fixture households.

Tier 1 workstreams:

| Workstream | Crate / dir | Produces |
| --- | --- | --- |
| data | `rr-etl`, `rr-data`, `data/` | ETL that downloads the federal sources, builds the core county pack + ZIP crosswalk + climate multipliers + base rates, writes `data/manifest.json`; loaders |
| hazards | `rr-hazards` | Location + dials -> per-hazard annual probability / severity with provenance, incl. societal and personal base rates and climate adjustment |
| consequence | `rr-consequence` | Hazard -> bucket mapping with duration distributions; the design-event calculation; natural-frequency sentences |
| supply | `rr-supply` | Bucket targets + household -> quantities per item with citations; tier definitions |
| budget | `rr-budget` | Item risk-reduction curves, greedy allocation by month, free actions first, guardrails |
| content | `content/`, `rr-content` | Item catalogue v1 (specs, look-for, avoid, price bands), citations registry, guidance blocks per bucket/hazard/tier; validator |
| web-shell | `web/` | Svelte app, all screens against the mock engine, persistence, print packet, PWA, a11y baseline |
| docs | `docs/` | RISK_MODEL, DATA_SOURCES, CONTENT_STANDARDS finalised from the research reports; glossary |

## Phase 2 — Integration

| Workstream | Produces |
| --- | --- |
| plan | `rr-plan`: pipeline orchestration, packet generation (Markdown), explanations |
| wasm | `rr-wasm` bindings implementing `ENGINE-API.md`; size budget |
| cli | `rr-cli`: `plan`, `explain`, `risks`, `data verify`, golden regeneration |
| web-engine | Real engine wired into the site behind a feature gate; parity tests mock vs wasm |
| map | County geometry pack (Census-derived, public domain) + static hazard map thumbnail |
| goldens | Fixture households x golden packets; CI diff |

## Phase 3 — Verification and release

Adversarial verifier (numbers vs sources, monotonicity, guardrails, prompt of every dial), accessibility
audit, plain-language pass, README with screenshots, Pages deploy, data-refresh Action live.

## Later

- Census-tract pack for finer location (lazy pack).
- Country packs: Canada, UK, Australia, New Zealand.
- Spanish translation first; then others.
- Desktop wrap (Tauri) with the same engine.
- Inventory tracking with rotation reminders (local notifications).
- Household sharing by file; caregiver view.
- Insurance gap calculators (NFIP, renters, earthquake).
- Community layer: CERT / mutual-aid locator (needs a data source that respects privacy).
- Import of a FEMA National Household Survey-style self-assessment to benchmark against peers.
