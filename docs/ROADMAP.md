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

## Round 2 — Review, v0.1.1 and the v0.2.0 programme (2026-09-26)

**Status: done, shipped as v0.2.0 (September 2026).** See [CHANGELOG.md](../CHANGELOG.md) for the
full release notes and `docs/DESIGN.md` §14 for the decision log of how the programme below was
built (many parallel workstreams, merged and verified together against contract v2).

A walk through the live site and four review panels (emergency management, practitioner,
catastrophe model with a 22-event backtest, hazard data) produced a merged review (kept in the
project's briefs folder, `round2/REVIEW.md`, with the panel reports beside it). **v0.1.1** shipped
the safety and correctness fixes (see [CHANGELOG.md](../CHANGELOG.md)). The owner's decisions for
**v0.2.0**: the full hazard taxonomy (ten new ranked hazards including burst pipes, wildfire
smoke, dam and levee failure, benefit interruption, eviction, arrest or detention; a nine-family
rare-but-severe box with a county strategic-exposure class for nuclear risk, solar storms, CBRN,
war, severe pandemic, very large eruptions, financial crisis, mass violence; sub-causes on every
card); a `clean_air` bucket; plan features (family-plan screen and wallet cards, shelter plan,
48-hour checklist, recovery page, access-and-functional-needs questions, insurance and mitigation
decisions, trusted circle, legal readiness, lockout plan); the kit and allocator overhaul (22 items,
28 rule changes, cooking capability, prerequisites, bare-minimum mode); model robustness (regional
outage pooling, event restoration curves, water-system fragility, compound events) with a public
validation page; and the long-horizon module. Engine contract v2; hosting stays where it is for now.

## Next

Concrete, near-term follow-ups, mostly from the verification pass
([VERIFICATION.md](VERIFICATION.md)) and from workstream hand-off notes. Day-to-day, these are
tracked as GitHub issues; the list here is the standing summary. See
[CHANGELOG.md](../CHANGELOG.md)'s "Known limitations" for how each of these reads to a user today.

- Do the screen-reader pass and a review with real users.
- Re-check the earthquake and outage numbers for Pacific coast counties reading unexpectedly high
  (Coos Bay, Oregon is the known example; its numbers moved higher still in v0.2.0 as the outage
  model improved, so this is more pressing, not less).
- Add a household-vulnerability factor to heat-wave and cold-wave severity, not just dollar cost, so
  a household with a baby, an older adult, or no cooling/heating is rated correctly. (v0.2.0 made sure
  such a household's severity label never reads better than "Serious"; the underlying budget priority
  still doesn't weigh who is in the household, only typical dollar cost.)
- Re-size livestock water on a well where a generator is already planned to keep the pump running
  (today the household must already own the generator and its interlock; a generator the plan intends
  to buy doesn't count yet).
- Fix the printed packet's month-by-month spending so a completed sinking fund and its purchase
  don't both count toward the same month's total.
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
- Decide on a dedicated hosting address, separate from the owner's other projects — more pressing
  now that a saved plan can hold a family's trusted circle, names and phone numbers (see
  [PRIVACY.md](PRIVACY.md)).
- **From the v0.2.0 build**, carried into this list:
  - Close the gap in how the events job matches storm records to counties: 77 counties, mostly in
    western Washington and coastal Alaska, run their windstorm number on a thin-record fallback until
    it's fixed.
  - Recalibrate the National Risk Index base rates that still sit well above their own county's
    episode record for ice storms, winter storms and hurricanes (491, 31 and 472 counties
    respectively) — a later, dedicated calibration pass.
  - Put `Item.alternative_group` (built for v0.2.0 but not yet used by any catalogue item) to work —
    two different ways to recharge a battery is the obvious first candidate.
  - Add an interview question about owning a gas grill, so the spare-propane-tank accessory (built,
    but never offered today) has a way to be recommended.
  - Give the website's stand-in engine (used before the real data loads) real sample data for the
    seven newer fixture counties; it currently falls back to a generic per-state entry for them,
    which is fine for development but not a place to add more.
  - Keep an eye on the two figures closest to their ceilings: the WebAssembly download (1.40 MB of a
    1.5 MB compressed budget) and the time to work out a full plan (about 46 ms in a command-line
    JavaScript engine, 38 ms in a real browser, against a 50 ms target). Both are comfortable today,
    and both have been trending toward their limits as hazards and data are added each round.

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
