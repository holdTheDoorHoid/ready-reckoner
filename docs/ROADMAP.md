# Roadmap

Phases are ordered by dependency. Within a phase, workstreams run in parallel as separate agents in
separate worktrees. Each workstream becomes a GitHub issue; the issue is the brief's summary and the
place to report results.

## Phase 0 — Plan and scaffold (2026-09-25)

**Status: done.** Interview, decisions, `docs/`, workspace skeleton, CI, public repository, milestone
issues. Done in the founding session.

## Phase 1 — Foundations (parallel)

**Status: done.** Every crate and directory below exists on `main` and does what it says.

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

**Status: done.** The real engine runs behind the website (no feature gate needed any more), the CLI
prints full packets, and CI diffs every fixture household against its golden packet.

| Workstream | Produces |
| --- | --- |
| plan | `rr-plan`: pipeline orchestration, packet generation (Markdown), explanations |
| wasm | `rr-wasm` bindings implementing `ENGINE-API.md`; size budget |
| cli | `rr-cli`: `plan`, `explain`, `risks`, `data verify`, golden regeneration |
| web-engine | Real engine wired into the site behind a feature gate; parity tests mock vs wasm |
| map | County geometry pack (Census-derived, public domain) + static hazard map thumbnail |
| goldens | Fixture households x golden packets; CI diff |

## Phase 3 — Verification and release

**Status: mostly done**, released as v0.1.0 (September 2026). Of the six items below, five are done;
one is not started.

- Adversarial verifier (numbers vs sources, monotonicity, guardrails, prompt of every dial) — done;
  results and open findings in [VERIFICATION.md](VERIFICATION.md).
- Plain-language pass — done; packets measure at a grade 5.7–6.8 reading level (target: 9 or below).
- README with screenshots — done (this release).
- Pages deploy — done; live at the project's GitHub Pages address.
- Data-refresh Action live — done; runs quarterly, opens a pull request, nothing merges without a
  person reading it.
- Accessibility audit — **automated part done, manual part not.** An axe-core sweep over 38 pages
  at desktop and phone widths found 0 violations (twice: after the web shell and after the real
  engine landed); keyboard use and reduced motion were checked by hand. A screen-reader pass and a
  review with real users have not been done.

## Next

Concrete, near-term follow-ups, mostly from the verification pass
([VERIFICATION.md](VERIFICATION.md)) and from workstream hand-off notes. Day-to-day, these are
tracked as GitHub issues; the list here is the standing summary. See
[CHANGELOG.md](../CHANGELOG.md)'s "Known limitations" for how each of these reads to a user today.

- Do the screen-reader pass and a review with real users.
- Give the 79 counties without their own outage history a state-level fallback instead of today's
  simpler estimate.
- Re-check the earthquake and outage numbers for Pacific coast counties reading unexpectedly high
  (Coos Bay, Oregon is the known example).
- Keep each hazard card's guidance specific to that hazard (stop a cold-wave card from showing
  avalanche advice, and similar cases).
- Add a household-vulnerability factor to heat-wave and cold-wave severity, not just dollar cost, so
  a household with a baby, an older adult, or no cooling/heating is rated correctly.
- Re-size livestock water on a well where a generator is already planned to keep the pump running.
- Fix the printed packet's month-by-month spending so a completed sinking fund and its purchase
  don't both count toward the same month's total.
- Lay out the packet's source list in columns and stop a section splitting across a printed page.
- Replace source links that point to a mirror or a search results page with direct links.
- Recheck price bands that are running high against current prices.
- Show the first several steps of a long checklist with a way to see the rest, instead of every step
  at once.
- Move data loading off the main thread (a Web Worker) so the page does not pause while the ZIP list
  or the first county data batch comes in.
- Add `BucketAssessment.covered_today` so the plan screen can show progress already made, not only
  progress at the end of the plan.
- Trim four unused hazard-data columns and one unused ZIP file from the data pack (roughly 0.65 MB
  smaller first load).
- Confirm offline use (the service worker) across more browsers.
- Decide on a dedicated hosting address, separate from the owner's other projects (see
  [PRIVACY.md](PRIVACY.md)).

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
