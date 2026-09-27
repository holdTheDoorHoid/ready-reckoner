# The packet

The printable packet is `PlanOutput.packet_markdown`, written by `crates/rr-plan` (`src/packet/`)
from the plan's numbers and the guidance blocks in `content/guidance/`. The web app renders it with
its sanitising Markdown renderer (with the county map at the start of "Your risks") and prints the
sections one after another, the sources in two small columns and the wallet cards two across on a
page of their own; the CLI prints it as is. `docs/DESIGN.md` §9 and DESIGN-DELTA §3 (packet v2,
v0.2.0) set the sections; this file says what feeds each one and how the household's numbers get
into the guidance text.

## Length and the page budget

Packet v2 has a budget of **24 US Letter pages for the Philadelphia household** by the proxy below
(DESIGN-DELTA §3). The new pages (the family plan, the wallet cards, the shelter plan, the forecast
list, access and functional needs, local help, the recovery page, the long-horizon section) had to
earn their place by trimming elsewhere, so the why behind each number, the requirement lines and
the research stay in the app's Learn and explain views, and the packet keeps what to do.

**Printed pages, the proxy.** Words outside the Sources section count 400 to a printed page and
words in the two-column, 8-point Sources section (with the data credits) 1,000 to a page. A word
is a token with a letter or a digit, citation brackets left out. This is calibrated on the one
measured print: the v0.1.0 Philadelphia packet, 8,054 words outside Sources and 2,055 in it,
printed on 22 US Letter pages, which is 22.19 on this scale. Printed from the app (headless
Chrome, `web/scripts/packet-pages.mjs`) with sections following on (a heading is never left alone
at the foot of a page), tables allowed to run on (rows never split, headers repeat) and 10.5 pt
body text.

The tests hold the line:

- `philadelphia_stays_within_24_printed_pages` (`crates/rr-plan/tests/round2.rs`): Philadelphia at
  most 24.0 pages, and every fixture and backtest household at most 26.0;
- `the_packet_stays_short` (`crates/rr-plan/tests/fixtures.rs`): every fixture at most 12,500
  words, no retired part (the "Your numbers" and "What counts toward it" paragraphs), at most the
  four topic headings, no month after next in detail, no table of later months, checklists only up
  to the step that is enough.

| Household | v0.2 before packet v2 (7f6b8c6) | Packet v2 |
| --- | --- | --- |
| Philadelphia (the budget's reference) | 28.07 pages, 13,349 words | 23.77 pages, 11,550 words |
| Chicago, zero budget | 20.97 pages, 10,386 words | 20.24 pages, 10,066 words |
| Coos Bay | 28.28 pages, 13,423 words | 24.70 pages, 11,955 words |
| Hays, Kansas | 28.07 pages, 13,389 words | 24.78 pages, 12,017 words |
| Miami | 27.69 pages, 13,260 words | 23.84 pages, 11,581 words |
| Phoenix | 27.70 pages, 13,258 words | 23.43 pages, 11,409 words |
| Sugar Land | 29.59 pages, 14,091 words | 25.80 pages, 12,492 words |
| Cameron Parish (new) | | 25.22 pages, 12,225 words |
| Detroit (new) | | 24.46 pages, 11,832 words |
| Galveston (new) | | 23.14 pages, 11,274 words |
| Minot (new) | | 23.63 pages, 11,481 words |
| Missoula (new) | | 23.02 pages, 11,208 words |
| Sacramento (new) | | 22.84 pages, 11,173 words |
| San Juan (new) | | 25.81 pages, 12,399 words |

Households with more to say than Philadelphia (insulin and a baby in Sugar Land, a well and
livestock in Cameron Parish, Puerto Rico's long outages in San Juan) run longer; all stay under 26
pages. What pays for the new pages:

- **Risk cards** follow the card rule below; each card's named sub-causes are one "Includes:" line,
  and its footer one "How bad:" line.
- **Rare families:** one collapsed table (how likely, as a range only; if it reaches you; what it
  changes in your plan), sorted by how likely each is here. A family's block prints only where its
  location factor raises the family above the national figure (`packet::family_block_prints`:
  the nuclear family where the strategic class's share is above the population-weighted national
  mean, 0.314), as one "**Name: why here.**" paragraph.
- **"Also checked"** (hazards and rare sub-rows under 1 in 100,000 a year here) is one paragraph
  in `rr-hazards`' words; rows that share a rate ("fewer than 1 in 1,000,000 a year", "none
  recorded here") are listed together with the rate once.
- **Targets:** a bucket whose advice is printed elsewhere points to it ("See Your family plan and
  Your shelter plan") and keeps its "What to avoid" paragraph; see Sections.
- **The plan:** this month and next month in detail (`packet::DETAIL_MONTHS` = 2; six before
  v0.2.0), decisions due that month grouped on one "Decide this month" line, a price band only
  where prices spread more than a quarter either side of the estimate. From month 2 the plan is the
  checklists, each line with the month the plan gets to it ("(month 22)"), so every purchase is
  listed once; the old table of later months is gone.
- **Lists use short names** where a catalogue name carries a long explanation after a colon.
- **Sources:** compact, as before; one access date on the first data credit.

## The card rule

`packet::risks::cards` chooses which ranked hazards get a card (review S3, v0.1.1; packet v2):

- the **likeliest** hazards: at most six (`packet::FREQUENT_CARDS`) by household rate among those
  with at least a 10 in 100 chance in ten years (`packet::CARD_MIN_P10`);
- **house fire**, always;
- every hazard with at least a 1 in 100 chance in ten years (`packet::LIFE_SAFETY_MIN_P10`) that is
  rated **Severe** or worse (`packet::SEVERE`), that **drives a compound event class** behind a
  target (at least 5 in 100 of a target's rate from the grid emergency in extreme cold or the
  blackout during a heat wave), that **strikes fast** (`packet::FAST_HAZARDS`: wildfire, floods,
  tsunami, tornado, earthquake, landslide, avalanche) or that **meets this home** (a basement or a
  floor below street level and floods; a mobile home and wind or hurricanes; someone who needs help
  to move and wildfire, floods or a dam failure);
- the hazard of any **named scenario** the plan includes;
- the household's likeliest **wind** hazard (`packet::WIND_HAZARDS`: tornado, strong wind, hail,
  lightning) from a 10 in 100 chance in ten years, because the shelter-spot advice is on that card,
  unless a wind hazard rated Severe already has a card of its own (it shows the same block);
- **no card for a hazard whose only consequence is lost income** (a lost job, an earner's death or
  disability): what to do about it is the savings goal and the insurance decisions under Documents
  and money.

At most **eight** cards (`packet::HAZARD_CARDS`). Above eight, the less severe cards make room
first, so **a Minor hazard never displaces a Serious or Severe one**; among cards of one severity,
the likeliest-hazard cards go before the fast or exposed ones, the least likely first. House fire,
Severe hazards, compound and scenario hazards always stay, and so does a medical emergency's card,
because the advice it carries would otherwise print in full under Your targets. The wind card
stays against Minor and Moderate rows (so a frequent-but-minor row such as phone outages can no
longer push it out), but a Minor or Moderate wind card yields to a Serious or Severe card that
would otherwise make room. Cards print most likely first. Every other ranked hazard is a
table row, and those under 1 in 100 share one line.

On the fixtures: Sugar Land keeps its heat wave (heat wave, medical emergency, hurricane, cold wave
by the compound rule, burst pipe, flooding, house fire, tornado), and so does Galveston. Chicago
and Coos Bay keep the Strong wind card. In Philadelphia and Miami, Strong wind (Minor there) yields
to a Serious card of the household's own: Philadelphia's wildfire smoke, Serious for its older
adult (heat wave, medical emergency, cold wave, wildfire smoke, burst pipe, flooding, house fire,
earthquake), and Miami's cold wave, Serious for its retiree (heat wave, medical emergency, cold
wave, hurricane, burst pipe, house fire, flooding, coastal flooding). Where to shelter from wind
is in every packet anyway: the shelter plan's Strong wind paragraph prints for every household, so
a household without the wind card still has it, once. `storm_and_heat_cards_stay_where_they_matter` and
`hazard_cards_follow_the_life_safety_rule` (`crates/rr-plan/tests/fixtures.rs`) check the rule.

## Format

- Markdown only: headings, paragraphs, lists, task lists (`- [ ]`), tables, block quotes and bold.
  No HTML, no images, no links other than bare web addresses (the sources and the state's local-help
  pages). Text taken from outputs or from the household's own answers is escaped, so an item name
  or a meeting place can never start emphasis, a link or a table cell.
- One `#` title, then fifteen `##` sections in a fixed order (`packet::SECTION_HEADINGS`), and two
  more that print only when they apply, each after a named section
  (`packet::CONDITIONAL_HEADINGS`). A test checks every packet has each section in order.
  Subsections are `###`; cards and topic blocks inside a section sit under `####`.
- **Citations are numbers in brackets** ("[3]", "[3, 7]") that point into the numbered Sources
  section at the end, so the packet reads the same on paper and on screen. The guidance blocks'
  `[^id]` footnotes become these numbers too, and their own "Sources" lists are left out.
- Numbers follow `web/src/lib/format.ts`: natural frequencies out of 100 (whole numbers to 10,
  the nearest 5 above, "almost all" from 97.5), days on the target ladder in the unit a person
  would use ("2 weeks", "1½ months"), whole dollars with bands as "$30–45", dates as
  "October 1, 2026". Sentences from other crates (frequency sentences, the statement, the "why"
  of each plan item, the allocator's rare-purchase lines) are used as they come.
- **Wallet cards** are one block quote per person under `## Wallet cards`
  (`packet::WALLET_CARDS_HEADING`); the web app finds the section by that heading and prints each
  block quote as a card to cut out. Phone numbers carry non-breaking hyphens (U+2011,
  `packet::NB_HYPHEN`) so a number never breaks across two lines of a card.

## Sections

| # | Heading | What feeds it |
| --- | --- | --- |
| 1 | (title block) and `## Summary` | Who (`people`, `pets`), where (`location`, with the ZIP code), the home (`housing`), the plan date, the dial and water level in words; then the **status line** (`packet::STATUS_LINE`, review C1); a "Sample data" note when the data is the sample counties. **Where you are now** (`tier_reached`), **what is enough for your risks** (`tier_recommended`), and the plan's **two done months**: the bare-minimum kit (`plan.minimum_done_month`: three days of water, light, warmth and medicine) and everything (`plan.done_month`). **The three things that matter most**: first, when leaving home comes first, the decision to leave (see below), with the stay-home amounts then reading "If you are not told to leave, ..."; power and water (the power bucket's first frequency sentence, then both targets); food and medicine; then, if there is room, a cliff warning, a named scenario, the income gap or leaving home. **What the plan assumes you already have**, in one paragraph (`assume_basics`). Leaving comes first (review S2, RR-P02) when the evacuation bucket's ten-year chance is at least 25 in 100 (`packet::LEAVE_FIRST_P10`), the plan includes a major hurricane or a local tsunami (`packet::LEAVE_FIRST_SCENARIOS`), the home is in a storm-surge area (the ZIP code's Category 1–3 surge share, or else the county's surge class, as `rr_budget::is_surge_zone` reads them: "Parts of your area flood in a hurricane's storm surge."), or wildfire or a dam or levee failure (`packet::LEAVE_FIRST_FAST_HAZARDS`) has a ten-year chance of at least 10 in 100, in which case the consequence model's warning by cause follows ("for wildfires, 15 minutes"). A local tsunami adds the shaking rule. |
| 2 | `## Your family plan` | Everything the household wrote on the family-plan screen (`PlanInput.family_plan`), **word for word; nothing in it is computed with**. A table: meeting places near home and outside the neighborhood, the out-of-area contact, who picks up the children (households with children), work and school plans, shelter spots at home and at work or school, where we would go, the first and second way out, neighbors who check on us, who takes the animals (households with animals), the gas, water and electrical shut-offs (gas only where there is gas), roadside assistance (households with a vehicle), the lawyer. **Every empty field is a line to write on** ("__________"), never "not answered", so the page works on paper too. Then the trusted circle (who agreed to help and what each holds: a spare key, copies of documents, medical power of attorney, backup codes; at least two rows) and the numbers we know by heart. **Leaving home**: the evacuation bucket's sentences (the ten-year chance, the warning by cause, the time away; the first is left out when the summary already opens with it). **Staying in touch**: the headed paragraphs of `plan_communication` (four ways to reach each other), the school and child-care paragraph only for households with children. |
| 3 | `## Wallet cards` | One block quote per person, "Wallet card: person 1 (adult)", with the out-of-area contact, the two meeting places, the lawyer, the trusted circle, the numbers to know by heart and a medical-notes line; empty fields are lines to write on, so no card is ever empty. |
| 4 | `## Your risks` | The cards (see The card rule): the card's frequency sentence with its sources, the "What helps" and "What to avoid" paragraphs of the matching `hazard_*` block (each block once; a later card of the same family points to the earlier one), the named sub-causes on one "Includes:" line (a sub-cause whose note says it does not reach here is left out), and "How bad: Severe (mostly data)". The other ranked hazards as a table (chance over the horizon, severity, confidence), those under 1 in 100 in one line, and "Also checked" as one paragraph. **Rare but severe**: the nine families as one table, sorted by how likely each is here, with how likely (a range, never a point), if it reaches you (`if_it_reaches_you`) and what it changes in your plan (`what_it_changes`); then a "why here" paragraph for each family the location factor raises above the national figure. The months-long blackout row reads the household's own power curve at 60 days (`add_power_curve`). `rr-hazards`' notes as a list ("Notes on these numbers"). |
| 5 | `## Your targets` | The dial and what it means (`packet::dial_sentence`, model review M-04): the consequence model's own sentence, as it words it (`ConsequenceAssessment::dial_sentence`: about 1 in 10 households like yours face a longer disruption of any one kind at 1 in 100, and the higher share that meets at least one, from the joint rate, both on one scale, so Coos Bay at 1 in 500 reads "about 2 in 100 ... about 3 in 100"), then "That is why the plan also gives you ways to cope when a target runs out." A table of the duration buckets: target and range, when outside help likely arrives and when service is mostly back (the relief rating for the event behind the target; where there are no restoration records for it, "worst on record: up to N days" from the bucket's worst event on record, with a note), and the step that is enough. **How well do these numbers hold up?**: the frozen backtest's headline tally (`EngineInfo.validation`: 22 events, 6 covered, 9 partial, 6 short, 1 not modelled), cited like any other source to the registry entry for `docs/VALIDATION.md` (`rr_validation_2026`, `rr_plan::validation::CITATION`), so the Sources list carries its title and address. Then one part per bucket with a target: the "What helps" and "What to avoid" paragraphs of its `bucket_*` block, and the consequence model's own lines: the worst event on record (power and both water buckets), the water system's record (once), and the rounded-up note. A bucket whose advice is printed elsewhere points to it and keeps its "What to avoid": to a card showing the same block or whose hazard has that bucket as its only consequence (a medical emergency), to both cards for dangerous heat or cold (not for a wood-heated home), to the family plan and the shelter plan for leaving and getting home, to the forecast list and the family plan for power and phones, to the shelter plan for unhealthy air, to the plan's steps for security, to Special needs for medicine when its medicine lines print; lost income and a damaged home point to Documents and money. Then named scenarios (included or left out, why, what they change). |
| 6 | `## Your plan` | The budget; the free steps to start now (month 0); **safety rules to learn now** (`packet::SAFETY_RULES`, review S3b: fire, gas, the water heater, food in a power cut, the generator, CPR), one line each with sources; this month's purchases (month 0 gets the one-off money) and next month's, each with quantity, estimated cost and, where prices spread widely, the usual band, and next month's decisions on one "Decide this month (see Documents and money)" line; a pointer to the checklists for the months after that; savings toward bigger items; when every need is covered (`done_month`); the guardrail warnings under "Things to watch", including the plan-too-long warning with its deferred list (what lies beyond three years at this budget); and **the rare-event allowance**, when the household opened it, with each rare purchase's line as the allocator words it ("It is for the power out for months row you ticked: ..."). Things the household already has are not steps. |
| 7 | `## Your shelter plan` | The `plan_shelter` block: where to shelter from strong wind (every household) and from each other danger likely enough here (a tornado, hurricane, earthquake, chemical release or smoke paragraph when that hazard has at least a 1 in 100 chance in ten years), and radiation, with its conditional spans for this home. |
| 8 | `## When a storm, freeze or heat wave is forecast` | The `plan_forecast_48h` block (practitioner P-08): the 48-hour list. Its freeze paragraph prints where a cold wave, winter storm or ice storm has at least a 1 in 100 chance in ten years, and its heat-wave paragraph where heat waves do (no freeze steps in Puerto Rico). |
| 9 | `## Checklists` | The free steps from month 2 on, grouped by month; every purchase by step (tier) up to the recommended step, quantities added up and the month the plan buys each ("(month 16)"); the get-home bag per commuter (each person's trip and the water and snacks for the walk; the bag line once, "for each person", when every commuter's is the same); hazard-specific extras the budget does not measure, with price bands; gear for rare catastrophes only when the household opened the rare-event allowance (review S4). |
| 10 | `## Access and functional needs` (only when someone has one) | Who needs what ("person 3 (older adult): deaf or hard of hearing"), then the `topic_access_needs` block (CMIST: communication, maintaining health, independence, support and safety, transportation; review RR-P08), with its `{if:need:…}` spans kept for the needs this household has. |
| 11 | `## Local help` | The household's state row from the registries table (`content/states.toml`, `docs/CONTENT_STANDARDS.md` §10): evacuation zones and the registry for people who may need help, alerts, emergency prescription refills, each with its sources and the date it was checked; web addresses are left as they are so the app links them. A state without a row gets one sentence pointing to the county emergency management office. |
| 12 | `## Documents and money` | FEMA's Emergency Financial First Aid Kit's four parts as a checklist; **Decisions**: each insurance and ID decision with the allocator's reason, home repairs (for owners) on one line; **Cash**: the cash line; **Savings**: the income gap, the savings goal and when the supplies money could turn to it, the first milestone; **If damage forces you out**: the home-loss bucket's months away and what living elsewhere costs. |
| 13 | `## After a disaster: the first 30 days` | "**Your county.** Philadelphia County, Pennsylvania had one federal major-disaster declaration in the last five full years." (OpenFEMA, `PlanOutput.recovery`), then the `after_first_30_days` block (review RR-P06): going home, insurance and records, help from FEMA, scams, in rough order. Every household. |
| 14 | `## If it lasts for months` (only when the plan has a long-horizon section) | The plan's long-horizon items (`plan.long_horizon`), one line each with the quantity and the month each starts (they stay in their months too); **how likely here**: power out for two months or more, and for three months or more, from the household's own power curve (the consequence model's `multi_month` 60- and 90-day rates, as ranges from their 10th and 90th percentiles); then the `topic_long_horizon` block, its wells paragraph only for a household on a well and its fuel paragraph only with a generator or fuel in the plan. |
| 15 | `## Special needs` | Medicine (`medication_days`, `rx_cold_storage`, `epinephrine_check`, `med_list_written`; the cooler advice only without the cold-storage line); antibiotics; powered devices; babies and toddlers; older adults (when nobody has limited mobility, whose advice is under access needs and getting around); getting around; pregnancy and nursing; pets and animals (the pet food, carrier and livestock water lines); stress, mental health and the 988 line (`topic_mental_health`, always, under its own title). A subsection appears only when the household needs it. |
| 16 | `## Maintenance calendar` | Every item in the plan with a rotation or check interval (`Item.maintenance`): every 1 to 3 months as repeating rows; longer intervals as dates counted from the month the item enters the plan; seasonal items anchored to the start of their season (1 March, 1 June, 1 September, 1 December) and checked yearly when they have no interval of their own; "Test" rows for things that must work (lights, jump starters); the yearly review a year after the planning date. |
| 17 | `## Sources` | The citations the packet's brackets point to (the first ones in `PlanOutput.provenance`), numbered in order and run together ten to a paragraph: title, publisher, year, the URL once, expert estimates marked; a count of the provenance's other sources; then the data credits from `EngineInfo.attributions` (the National Risk Index statement exactly as its terms require, with version and access date; the access date once, on the first credit). |

Topic blocks the packet does not print (`topic_the_dial`, `topic_consequences_not_causes`,
`topic_disaster_myths`, `topic_how_numbers_are_made`, `topic_climate_horizon`, `topic_evs`,
`topic_renters`, `topic_antibiotics`, `topic_pets`, `topic_rotation`, `topic_drills`,
`topic_neighbours`, `topic_talking_with_children`, `topic_before_you_need_them`,
`topic_strategic_sites`) belong on the app's Learn screen, as do the `tier_*` blocks. The drills,
neighbours and talking-with-children steps are free steps in the plan; the app shows their how-to.

## Placeholders and conditional text

A guidance block may contain these; the packet fills them for the household. A block without them
is used as written.

| Placeholder | Filled with |
| --- | --- |
| `{frequency}` | In a bucket block, the bucket's first frequency sentence ("Of 100 households like yours, about 40 (35–65) will lose grid power for a day or more in the next 10 years."), cited to the bucket's sources. In a hazard block, the register card's sentence, cited to the hazard's sources. Where there is no sentence (topic and plan blocks), the placeholder and the space after it are dropped. Bucket and hazard blocks must open with it (`docs/CONTENT_STANDARDS.md` §4). |
| `{target}` | The bucket's target in words ("about 5 days (up to 10 days)"), in bucket blocks. |
| `{county}` | "Philadelphia, Pennsylvania". |
| `{horizon}` | "the next 10 years" (from `dials.horizon_years`). |
| `{household}` | "2 adults, 1 older adult and 1 child, with 1 dog". |

Conditional spans (`rr_content::policy::Condition`, `docs/CONTENT_STANDARDS.md` §4) are kept or
dropped for this household, which `packet::Ctx` describes to the renderer
(`rr_content::policy::HouseholdFacts`):

- `{if:<hazard_id>}…{/if}` in a hazard-family block stays only when that hazard's ten-year chance
  is at least 1 in 100 (a Philadelphia rowhouse is not told to carry an avalanche beacon);
- `{if:home:<kind>}…{/if}` and `{if:not_home:<kind>}…{/if}` by the kind of home (`HousingKind`
  strings, several joined with `|`): an inside hallway and no elevators for an apartment building,
  "on or below the 10th floor" for a high-rise, a sturdy building for a mobile home;
- `{if:need:<need>}…{/if}` when someone in the household has that access or functional need;
- `{if:has:<item>}…{/if}` when the plan holds that item (bought, owned or free);
- `{if:benefit:<benefit>}…{/if}` when the household relies on that benefit.

Household-free views (the app's Learn screen) keep every span, so each is written to read well
beside the others.

A test fails if any placeholder is left in a packet, or if a citation marker, a conditional marker
or a `[^id]` reference is left over, or if a bracketed number points past the Sources list.

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
`PlanOutput`, for all fourteen fixtures (`rr_types::fixtures::RAW`), planned from the core pack in
`data/` as the web app plans. `cargo test -p rr-plan --test goldens` compares them and prints a
diff; `RR_UPDATE_GOLDENS=1 cargo test -p rr-plan --test goldens` rewrites them. Explain every
golden change in the commit message.
