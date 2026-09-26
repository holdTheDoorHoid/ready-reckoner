# The packet

The printable packet is `PlanOutput.packet_markdown`, written by `crates/rr-plan` (`src/packet/`)
from the plan's numbers and the guidance blocks in `content/guidance/`. The web app renders it with
its sanitising Markdown renderer and prints each `##` section on its own page; the CLI prints it
as is. `docs/DESIGN.md` §9 sets the sections; this file says what feeds each one and how the
household's numbers get into the guidance text.

## Format

- Markdown only: headings, paragraphs, lists, task lists (`- [ ]`), tables, block quotes and bold.
  No HTML, no images, no links other than bare source URLs. Text taken from outputs is escaped, so
  an item name can never start emphasis, a link or a table cell.
- One `#` title, then ten `##` sections in a fixed order (a test checks every packet has each
  one, in order). Subsections are `###`; topic blocks inside a section sit under `####`.
- **Citations are numbers in brackets** ("[3]", "[3, 7]") that point into the numbered Sources
  section at the end, so the packet reads the same on paper and on screen. The guidance blocks'
  `[^id]` footnotes become these numbers too, and their own "Sources" lists are left out.
- Numbers follow `web/src/lib/format.ts`: natural frequencies out of 100 (whole numbers to 10,
  the nearest 5 above, "almost all" from 97.5), days on the target ladder in the unit a person
  would use ("2 weeks", "1½ months"), whole dollars with bands as "$30–45", dates as
  "October 1, 2026". Sentences from other crates (frequency sentences, the statement, the "why"
  of each plan item) are used as they come.

## Sections

| # | Heading | What feeds it |
| --- | --- | --- |
| 1 | (title block) and `## Summary` | Who (`people`, `pets`), where (`location`, with the ZIP code), the home (`housing`), the plan date, the dial and water level in words; the step reached (`tier_reached`) and the step that is enough (`tier_recommended`) with the month the plan gets there (`plan.done_month`); **the three things that matter most**: power and water (the power bucket's first frequency sentence, then both targets), food and medicine (the supplies bucket's sentence, then both targets), then one of: a cliff warning, a named scenario the plan includes that changes the targets, the income gap and savings goal, or leaving home; **what your household should be able to handle** (`rr-consequence`'s statement); the first three free steps of month 0. When the data is the fixture counties, a "Sample data" note. |
| 2 | `## Your risks` | The five most likely ranked hazards as cards: the matching `hazard:*` guidance block with the card's frequency sentence (and its sources) in place of `{frequency}`, what the hazard can do (its buckets), severity and confidence in words. Each hazard block appears once. The other ranked hazards as a table (ten-year chance, severity, confidence). The rare-and-catastrophic box: likelihood and severity as separate columns, the "get inside, stay inside, stay tuned" sentence, and the nuclear block. `rr-hazards`' notes as a list. |
| 3 | `## Your targets` | The dial in words with the share of ten-year stretches worse than the targets; a table of the duration buckets (target and range, relief rating, the step that is enough); the climate block when the dial is "around 2050". Then one subsection per bucket with a target: the `bucket:*` guidance block with the bucket's first frequency sentence in place of `{frequency}`, the bucket's other sentences, the relief rating, the savings goal (income), the need lines that count toward it and the alternatives, optional upgrades and notes (`rr-supply`). Then named scenarios (included or left out, why, what they change) and cliff warnings. |
| 4 | `## Your plan` | The budget; the free steps to start now (month 0) with their "why"; this month's purchases (month 0 gets the one-off money); next month; the whole plan month by month with quantity, estimated cost, price band and "why" for each purchase and savings deposit (free steps after month 0 by name). Funds still open at the end, when the plan is done (`done_month`) or what a zero budget means, the income savings goal, and the guardrail warnings (not the cliff warnings, which sit under targets). |
| 5 | `## Checklists` | Every plan step by step (tier), quantities added up across months, with the `tier:*` guidance block for each step present; the get-home bag per commuter (the `get_home` lines of each person and their walk home); hazard-specific extras the budget does not measure (a shelter-in-place kit), with price bands. |
| 6 | `## Family plan` | A table to fill in (meeting places, out-of-area contact, children, work and school, where to stay, routes or a ride, neighbours, animals); the specs of the relevant free actions as advice (contacts, alerts, triggers, the go-or-stay card, drills, hazard-specific steps, school and work, neighbours, pets); the evacuation numbers (ten-year chance, warning, days away); topic blocks `topic:drills`, `topic:evs` (an electric vehicle), `topic:talking_with_children` (children), `topic:neighbours`. |
| 7 | `## Documents and money` | The Emergency Financial First Aid Kit's four parts as a checklist; the document, insurance and savings free actions; the insurance lines (`insurance_*`); `topic:renters` when renting; the cash line; the savings track. |
| 8 | `## Special needs` | Medicine (`medication_days`, `rx_cold_storage`, `epinephrine_check` and the medicine free actions); antibiotics (the `antibiotics_none` line and `topic:antibiotics`, always); powered devices; babies and toddlers; older adults; getting around (`topic:disability_access`); pregnancy and nursing; pets and animals (`topic:pets`); stress and mental health (`topic:mental_health`, always). A subsection appears only when the household needs it. |
| 9 | `## Maintenance calendar` | For every item in the plan with a rotation or check interval (`Item.maintenance`): every 1 to 3 months as repeating rows; longer intervals as dates counted from the planning date and the month the item enters the plan; the yearly review a year after the planning date; `topic:rotation`. |
| 10 | `## Sources` | Every citation in `PlanOutput.provenance`, numbered as the brackets refer to them, with publisher, year, URL and retrieval date, expert estimates marked; then the data credits from `EngineInfo.attributions` (the National Risk Index statement exactly as its terms require, with version and access date). |

Topic blocks the packet does not use (`topic:the_dial`, `topic:consequences_not_causes`,
`topic:disaster_myths`, `topic:how_numbers_are_made`) belong on the app's Learn screen.

## Placeholders

A guidance block may contain these; the packet fills them for the household. A block without them
is used as written.

| Placeholder | Filled with |
| --- | --- |
| `{frequency}` | In a bucket block, the bucket's first frequency sentence ("Of 100 households like yours, about 40 (35–65) will lose grid power for a day or more in the next 10 years."), cited to the bucket's sources. In a hazard block, the register card's sentence, cited to the hazard's sources. Where there is no sentence (topic and tier blocks), the placeholder and the space after it are dropped. Bucket and hazard blocks must open with it (`docs/CONTENT_STANDARDS.md` §4). |
| `{target}` | The bucket's target in words ("about 5 days (up to 10 days)"), in bucket blocks. |
| `{county}` | "Philadelphia, Pennsylvania". |
| `{horizon}` | "the next 10 years" (from `dials.horizon_years`). |
| `{household}` | "2 adults, 1 older adult and 1 child, with 1 dog". |

A test fails if any of these is left in a packet, or if a citation marker or `[^id]` reference is
left over, or if a bracketed number points past the Sources list.

## Where the covered days come from

The packet's "covered" numbers and the plan's order come from `crates/rr-plan/src/coverage.rs`
(its module documentation has the whole model): each catalogue item meets one or more of
`rr-supply`'s need lines; a bucket's need lines are grouped into parts by kind of cover (stored
water, the toilet, food, heat, cold, lights ...), and the bucket is covered for as many days as
its weakest part. The allocator in `rr-budget` values each part with an equal share of the
bucket (a powered medical device's part gets two thirds of the power bucket), and readiness items
by the harm they avoid each time they are needed.

## Goldens

`fixtures/golden/<fixture>.md` holds each fixture household's packet and `<fixture>.json` its whole
`PlanOutput`. `cargo test -p rr-plan --test goldens` compares them and prints a diff;
`RR_UPDATE_GOLDENS=1 cargo test -p rr-plan --test goldens` rewrites them. Explain every golden
change in the commit message.
