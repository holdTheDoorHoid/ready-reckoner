# The packet

The printable packet is `PlanOutput.packet_markdown`, written by `crates/rr-plan` (`src/packet/`)
from the plan's numbers and the guidance blocks in `content/guidance/`. The web app renders it with
its sanitising Markdown renderer (with the county map at the start of "Your risks") and prints the
sections one after another, the sources in two small columns; the CLI prints it
as is. `docs/DESIGN.md` §9 sets the sections; this file says what feeds each one and how the
household's numbers get into the guidance text.

## Length

The packet is the short, printable version: the goal is about 20 printed pages (about 8,000
words; POLISH_ROUND, 2026-09-26). The why behind each number, the requirement lines and the
research live in the app's Learn and explain views; the packet keeps what to do. So:

- hazard cards only for the likeliest hazards: at most six (`packet::HAZARD_CARDS`), by household
  rate, among those with at least a 10 in 100 chance in ten years (`packet::CARD_MIN_P10`), plus
  the hazard of any named scenario the plan includes; every other hazard is a table row, and those
  under 1 in 100 share one line;
- each bucket shows what to do: its block's "What helps" and "What to avoid" paragraphs, not the
  opening why, the numbers behind the target or the requirement lines. (The warnings stay because
  they carry the carbon monoxide, floodwater and do-not-drink rules.)
- tier checklists only up to the recommended step, without the tier blocks' prose; the free steps
  are ticked off in the plan itself;
- four topic blocks: talking with children, neighbours, drills and mental health, without their
  opening paragraph;
- the first 12 months of the plan in detail (`packet::DETAIL_MONTHS`), the rest as one table;
- compact sources: only the ones the packet's brackets point to, run together ten to a paragraph,
  each with title, publisher, year and the URL once.

On the seven fixtures this gives 7,900 to 11,100 words (down from 16,800 to 24,500), a fifth of
it the Sources section and data credits. `the_packet_stays_short`
(`crates/rr-plan/tests/fixtures.rs`) fails above 11,500 words, so the packet cannot grow back
unnoticed.

Printed from the app (headless Chrome, `web/scripts/packet-pages.mjs`), the Philadelphia packet
was 30 US Letter pages (28 A4) while every `##` section started a new page; with sections
following on (a heading is never left alone at the foot of a page), the sources in two columns of
8 pt type, tables allowed to run on (rows never split, headers repeat) and tighter paragraph
spacing it is 22 Letter pages (21 A4) at the same 10.5 pt body text. The rest of the way to 20
is the words themselves.

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
| 1 | (title block) and `## Summary` | Who (`people`, `pets`), where (`location`, with the ZIP code), the home (`housing`), the plan date, the dial and water level in words; the step reached (`tier_reached`) and the step that is enough (`tier_recommended`) with the month the plan gets there (`plan.done_month`); **the three things that matter most**: power and water (the power bucket's first frequency sentence, then both targets), food and medicine (the supplies bucket's sentence, then both targets), then one of: a cliff warning, a named scenario the plan includes that changes the targets, the income gap and savings goal, or leaving home; the first three free steps of month 0; **what the plan assumes you already have** (the everyday basics credited when `assume_basics` is on, with how to untick them on the Have screen, or a line saying the household unticked it). When the data is the fixture counties, a "Sample data" note. |
| 2 | `## Your risks` | Cards for at most six ranked hazards (see Length) plus named-scenario hazards: the card's frequency sentence (with its sources), then the "What helps" and "What to avoid" paragraphs of the matching `hazard:*` block, then what the hazard can do (its buckets), severity and confidence in words. Each block appears once. The other ranked hazards as a table (chance over the horizon, severity, confidence), those under 1 in 100 in one line. The rare-and-catastrophic box: likelihood and severity as separate columns, and the advice paragraphs of the nuclear block. `rr-hazards`' notes as a list. |
| 3 | `## Your targets` | The dial setting with the share of ten-year stretches worse than the targets; a table of the duration buckets (target and range, relief rating, the step that is enough). Then one subsection per bucket with a target: the "What helps" and "What to avoid" paragraphs of its `bucket:*` block. A bucket whose block a risk card already showed, or whose card hazard has that bucket as its only consequence (a medical emergency), points to the card; leaving home and getting home point to the family plan's steps for what helps and keep their warnings; a damaged home and lost income point to Documents and money. Then named scenarios (included or left out, why, what they change) and cliff warnings. |
| 4 | `## Your plan` | The budget; the free steps to start now (month 0); this month's purchases (month 0 gets the one-off money) and next month's, each with what it adds (the first sentence of its "why"); months 2 to 11 with quantity, estimated cost and price band; the months after the first year as one table (what to buy or save toward, and the month's spend: the money that leaves that month's budget, so a deposit counts once and a purchase paid from savings, marked "paid from savings" or "$90 of it from savings", counts only for the rest; never more than the month's budget plus what earlier months left). Things the household already has (listed or assumed) are not steps. Funds still open at the end, when the plan is done (`done_month`) or what a zero budget means, and the guardrail warnings (not the cliff warnings, which sit under targets, nor the assumed-basics note, which is in the summary). |
| 5 | `## Checklists` | Every purchase by step (tier) up to the recommended step, quantities added up across months (free steps are ticked in the plan); the get-home bag per commuter (the bag line without the trip, which the person's heading gives, and the water and snacks for the walk in one line); hazard-specific extras the budget does not measure (a shelter-in-place kit), with price bands. |
| 6 | `## Family plan` | A table to fill in (meeting places, out-of-area contact, children, work and school, where to stay, routes or a ride, neighbours, animals); the specs of the relevant free actions (each a parent action since content2: `comms_contact_card`, `comms_wea_alerts_on`, `evac_know_zone`, `evac_ride_plan`, `evac_half_tank`, `special_pet_plan`, `special_livestock_plan`); the evacuation numbers (ten-year chance, warning, days away); topic blocks `topic:drills`, `topic:talking_with_children` (children) and `topic:neighbours`, with the steps they cover (drills, neighbours) listed by name. |
| 7 | `## Documents and money` | The Emergency Financial First Aid Kit's four parts as a checklist; `docs_effak` (documents, inventory and insurance cover) and the document pouch; the insurance lines (`insurance_*`); the cash line; the savings track and `docs_start_emergency_fund`. |
| 8 | `## Special needs` | Medicine (`medication_days`, `rx_cold_storage`, `epinephrine_check`, `med_list_written`, the cooler, `med_epinephrine_plan`); antibiotics (the `antibiotics_none` line, always); powered devices; babies and toddlers; older adults; getting around; pregnancy and nursing; pets and animals (the pet lines, with the go-kit's water and food staged from household stock next to the carrier); stress and mental health (`topic:mental_health`, always). A subsection appears only when the household needs it. |
| 9 | `## Maintenance calendar` | For every item in the plan with a rotation or check interval (`Item.maintenance`): every 1 to 3 months as repeating rows; longer intervals as dates counted from the planning date and the month the item enters the plan; the yearly review a year after the planning date. |
| 10 | `## Sources` | The citations the packet's brackets point to (the first ones in `PlanOutput.provenance`), numbered in order and run together ten to a paragraph: title (without a trailing journal parenthetical), publisher, year, the URL once (in full, so the app links it), expert estimates marked; a count of the provenance's other sources; then the data credits from `EngineInfo.attributions` (the National Risk Index statement exactly as its terms require, with version and access date). |

Topic blocks the packet does not use (`topic:the_dial`, `topic:consequences_not_causes`,
`topic:disaster_myths`, `topic:how_numbers_are_made`, `topic:climate_horizon`, `topic:evs`,
`topic:renters`, `topic:antibiotics`, `topic:disability_access`, `topic:pets`, `topic:rotation`)
belong on the app's Learn screen, as do the `tier:*` blocks.

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

A hazard-family block (one block for several hazards, such as winter storms, ice, cold waves and
avalanches) marks a span about one of its hazards as `{if:<hazard_id>}…{/if}`. The packet keeps
the span only when that hazard's ten-year chance for the household is at least 1 in 100 (the cut
the risk section uses to list it), so a Philadelphia rowhouse is not told to carry an avalanche
beacon. The hazard must be one the block `applies_to` (the content validator checks this, and
that spans close, do not nest and stay in one paragraph). Household-free views keep every span.

A test fails if any of these is left in a packet, or if a citation marker, a conditional marker
or `[^id]` reference is left over, or if a bracketed number points past the Sources list.

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
