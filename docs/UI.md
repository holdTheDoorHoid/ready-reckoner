# UI specification (v1)

The interface is a guided interview that produces a plan and a printable packet, with an expert
layer underneath. Stack: Svelte 5 + Vite + TypeScript; engine via WebAssembly behind
`docs/ENGINE-API.md`; a JavaScript mock of the same contract lets the UI develop before the engine
lands. Works offline (service worker), installable (PWA). State lives in `localStorage` and in an
exportable JSON file; there is no server.

## Screens

| # | Screen | One job | Notes |
| --- | --- | --- | --- |
| 0 | Start | Explain what this is and the privacy promise; start new or import a saved plan | Import via file picker (JSON). Big, calm. |
| 1 | Where you live | ZIP or county, setting (urban/suburban/rural), housing type, own/rent, water/sewer, heating/cooling, backup power | ZIP resolves to county from a bundled crosswalk. Nothing sent anywhere; say so on the screen. Optional "improve with online lookups" toggle explains exactly what would be sent to whom. |
| 2 | Who is in your household | People by age band; medical needs (daily meds, refrigerated meds, powered devices, mobility, dietary); pregnancy; pets | Add-a-person cards. Medical fields collapsed until "anyone with medical needs?" is yes. |
| 3 | How you get around | Vehicles and fuel type; each adult's commute distance and mode; can they work remotely; school distance | Drives the get-home bag and evacuation logic. EVs get their own guidance. |
| 4 | Money | Monthly prep budget; one-off amount if any; months of expenses saved; earners and income stability; insurance held | Budget slider with "typical" marker. Reassure: $0 produces a plan of free actions. |
| 5 | What you already have | Optional inventory: quick checklist of common items with quantities | Skippable. Reduces the plan by what is owned. |
| 6 | Your risks | The register: hazard cards ranked, natural-frequency sentences, a county map thumbnail, the consequence buckets with "days you should be able to manage" | Dials live here in a drawer: confidence (9 of 10 / 19 of 20 / 99 of 100 ten-year periods), climate horizon (today / 2050), planning horizon. Changing a dial re-renders live. Every number has a source link. |
| 7 | Your plan | Phased purchases and actions by month; "this month" first; each item shows spec, what to look for, avoid, price band, and *why* (which buckets, which hazards); check-off and record what you paid | Free actions first, always. Progress bar per bucket ("power: 3 of 9 days covered"). Guardrail warnings, never blocks. |
| 8 | Your packet | Print view: summary, risks, targets, checklists per tier, evacuation and family plan, documents list, maintenance calendar, special needs, sources | Print stylesheet -> browser PDF. Black-and-white friendly. Page breaks per section. |
| 9 | Maintain | Rotation and check calendar, drills, review reminders (local only), export/import | Rotation dates computed from purchase dates the user recorded. |
| 10 | Learn | Why consequences not causes; disaster myths; how the numbers are made; talking with children; community | Short articles from `content/`. |
| 11 | About and method | Versions of engine, data packs and content; every source; licences; how to report a wrong number | |

## Copy rules

- One idea per sentence. Eighth-grade level. Natural frequencies before percentages.
- Threat and action always together: "Outages of three days or more hit about 12 of 100 households
  like yours per decade. Two weeks of stored water costs about $20 in reused bottles."
- Name the thing: "two weeks of water", not "Tier 2".
- Never a countdown, never scarcity language, never imagery of suffering.

## Components

- `HazardCard`: name, plain description, frequency sentence, severity band, contributing data
  sources, climate delta chip, "what it does to you" (buckets).
- `BucketGauge`: bucket name, target days, covered days, contributing hazards.
- `ItemCard`: spec, look-for, avoid, price band, quantity for this household, why, check-off,
  paid-amount field.
- `Dial`: labelled, with the plain phrase and the jargon in brackets.
- `SourceLink`: citation id -> title, publisher, year, URL, retrieved date.
- `Warning`: guardrail message with the reason and a "keep anyway" affordance.

## Accessibility

WCAG 2.2 AA. Keyboard-complete. Colour never the only signal (severity bands have text labels and
patterns). Reduced-motion respected. Print view tested at 100% greyscale.

## Theming

Light and dark, following system preference with a manual override. Colours defined as tokens.
Severity palette is colour-blind safe and paired with labels.

## Persistence

`localStorage` key `rr.plan.v1` holding the household, dials, check-offs and purchases. Export and
import as JSON (`ready-reckoner-plan.json`). A "forget everything" button clears storage and says so.

## Non-goals for v1

Accounts, sync, sharing links that carry household data, push notifications, native app stores.
