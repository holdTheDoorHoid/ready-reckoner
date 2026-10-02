# UI specification (v1)

The interface is a guided interview that produces a plan and a printable packet, with an expert
layer underneath. Stack: Svelte 5 + Vite + TypeScript; engine via WebAssembly behind
`docs/ENGINE-API.md`; a JavaScript mock of the same contract lets the UI develop before the engine
lands. Works offline (service worker), installable (PWA). State lives in `localStorage` and in an
exportable JSON file; there is no server.

## Screens

| # | Screen | One job | Notes |
| --- | --- | --- | --- |
| 0 | Start | Explain what this is and the privacy promise; start new or import a saved plan | Import via file picker (JSON). Big, calm. The status line (see Copy rules) sits directly under the "Private by design" box. |
| 1 | Where you live | ZIP or county, setting (urban/suburban/rural), housing type, own/rent, water/sewer, heating/cooling, backup power | ZIP resolves to county from a bundled crosswalk (the ZIP code list loads when someone starts typing one). A ZIP code that spans counties lists each with its share of the ZIP code's land and a numbered map. A county map confirms the result. Nothing sent anywhere; say so on the screen. Optional "improve with online lookups" toggle explains exactly what would be sent to whom. |
| 2 | Who is in your household | People by age band; medical needs (daily meds, refrigerated meds, powered devices, mobility, dietary); pregnancy; pets | Add-a-person cards. Medical fields collapsed until "anyone with medical needs?" is yes. |
| 3 | How you get around | Vehicles and fuel type; each adult's commute distance and mode; can they work remotely; school distance | Drives the get-home bag and evacuation logic. EVs get their own guidance. |
| 4 | Money | Monthly prep budget; one-off amount if any; months of expenses saved; earners and income stability; insurance held | Budget slider with "typical" marker. Reassure: $0 produces a plan of free actions. |
| 5 | What you already have | Optional inventory: quick checklist of common items with quantities | Skippable. Reduces the plan by what is owned. |
| 6 | Your risks | The register: the risk matrix, hazard cards ranked, natural-frequency sentences, a county map thumbnail, the consequence buckets with "days you should be able to manage" | Dials live here in a drawer: return period (Common 1-in-10 / Serious 1-in-50 / Very serious 1-in-100, default / Rare catastrophes 1-in-500, each with what its targets promise for any one need), climate horizon (today / around 2050), water level (survival / basic / comfortable), and named-scenario toggles when the engine offers them (e.g. Cascadia). Directly under the settings, the **risk matrix** (below) lists every hazard in one table; then the top three cards and "All N risks". Rare catastrophic hazards sit in their own two-column box (how likely / how bad), how likely shown as a range only. Duration buckets show target with range and the relief rating, under the dial sentence; readiness buckets are have/don't-have cards; money buckets are a separate savings track. Changing a dial re-renders live. Every number has a source link. |
| 7 | Your plan | Phased purchases and actions by month; "this month" first; each item shows spec, what to look for, avoid, price band, and *why* (which buckets, which hazards); check-off and record what you paid | Free actions first, always. Progress bar per bucket ("power: 3 of 9 days covered"). Guardrail warnings, never blocks. "Done so far" counts only steps checked off on the plan ("0 of 74 steps"); what the household already had (its "What you already have" list and the assumed everyday basics) shows beside it as "Already have: 8" and in its own "Already have" list, never as steps done. Money (cash in small bills) is "set aside", never bought. |
| 8 | Your packet | Print view: summary, risks (with the county map), targets, checklists per tier, evacuation and family plan, documents list, maintenance calendar, special needs, sources | Print stylesheet -> browser PDF. Black-and-white friendly. Sections follow on (no forced page break; a heading never ends a page); sources in two columns of small type. |
| 9 | Maintain | Rotation and check calendar, drills, review reminders (local only), export/import | Rotation dates computed from purchase dates the user recorded. |
| 10 | Learn | Why consequences not causes; disaster myths; how the numbers are made; talking with children; community | The five reviewed, cited topic blocks in `content/guidance/topic_*.md` (consequences_not_causes, disaster_myths, how_numbers_are_made, talking_with_children, neighbours), imported at build time; each note links to its source. Addresses stay `#/learn/consequences`, `myths`, `numbers`, `children`, `community`. No draft label; to show "Draft text, still being reviewed" on one again, add its slug to `IN_REVIEW` in `web/src/learn/articles.ts`. |
| 11 | About and method | Versions of engine, data packs and content; every source; licences; how to report a wrong number | The status line near the top. |

## The risk matrix (Risks screen, v0.1.1)

The owner's request (2026-09-26): the whole register at a glance, before any card.

- **Where.** Directly under "Your settings" (and before the warnings, scenarios and cards), so it is
  the first thing a screen reader meets after the settings. Heading "Your risks at a glance", a
  one-line intro, a table with a caption.
- **Rows.** Every ranked hazard in the engine's order (most likely first; the same order as the
  cards, so the rank matches), then, under a divider row "Rare but severe", the rare catastrophic
  hazards, never ranked.
- **Four columns at every width.** *Risk*: the rank and the name, a link to the hazard's card.
  *How likely* (over the "Show chances over" years): the chance worded exactly as the card's
  sentence words it (two significant figures; "about 86 of 100", "about 9 in 1,000", "about 1 in
  2,000", "nearly every household"), with expert estimates (`confidence: prior`) carrying their
  range ("about 86 (63–98) of 100"), and how often a year on a second, muted line ("about 4.5 times
  a year", "about once a year", "about 1 in 190 a year"). Rare rows show the range only (as the
  rare box does). *How bad*: the severity swatch and word. *How sure*: the confidence label. On a
  phone the names and cells wrap with every word whole, the severity swatch sits above its word,
  cells are tighter, and at 360 px and below the type is a step smaller; "1 in 2,000" and short
  ranges like "(63–98)" stay on one line. Checked at 390, 360 and 320 px: the table never scrolls
  sideways.
- **Jumping.** Each card has the id `hazard-<id>`. A name in the matrix scrolls to its card and
  moves focus to the card's heading; a card inside the folded "All N risks" list opens it first.
  The address (`#/risks`) never changes. Every card and the rare box end with "Back to the table",
  which returns focus to that hazard's row. Rare rows jump to the rare box.
- **The nuclear note.** When the nuclear row is present, the matrix and the rare box both say: "The
  nuclear figure is the chance of a nuclear catastrophe anywhere in the world, not for your county;
  a location-aware version is coming."
- **Print.** A plain black-and-white table; links print as text; the back links do not print.

## What the cards and boxes show (v0.1.1)

- **Rare box, how likely (H-02).** A range only, from `rate_range` over the chosen years: "between
  1 in 200 and 1 in 41", or "very unlikely: less than 1 in B" when the range spans more than 1,000
  times. The column header is "How likely (in the next N years)". It no longer says "households
  like yours", because the nuclear figure is worldwide.
- **What helps on a hazard card.** The free steps and purchases that answer the hazard, never a
  savings deposit. Items made for other hazards are left out (their catalogue `hazard_extras`, and
  the heat or cold requirement class `thermal_heat` / `thermal_cold`): a heat-wave card never offers
  the warm-room step, a cold-wave card never offers the fan. Items made for the hazard are offered
  even when the engine lists other hazards first. For hazards whose main consequence is a readiness
  checklist (a medical emergency, a fire, a break-in, being stranded, having to leave) life-saving
  items come first: the catalogue's `life_safety` flag plus the first-aid kit, the bleeding-control
  kit and the extinguisher. Names keep a leading acronym ("N95 respirators"). Each shows "free",
  its price, or "have it" / "done" when the household already has it (an assumed basic is never
  priced).
- **Evacuate card.** "Notice could be 1 minute to 3 days": the number as shown decides singular or
  plural.
- **Savings track.** Before the full goal, the next nearer one: "First goal: one month of
  expenses, about $4,200, by October 2031", or "Next goal: three months" once a month is saved.
  Dollars from the engine's `target_usd / target_months`; the date from its suggested monthly amount,
  which starts once the supplies plan is done (`done_month`); no date when either is missing. The
  full goal and the engine's own sentence about how long it takes stay.

## Copy rules

- The **status line**, word for word, on Start (under the privacy box) and on About; the packet
  prints it from the engine: "Ready Reckoner is an independent, open-source planning aid. It is not
  official emergency guidance, and not medical, legal or financial advice. Follow instructions from
  your local officials first."
- The **dial sentence** says what a target promises for one need, never for all at once. Each
  setting: "For any one need, something worse than its target comes in about 1 of every 10 ten-year
  stretches" (6 of every 10, 2 of every 10, 1 of every 10, 2 of every 100 for the four settings).
  For all needs together, the engine's own sentence for the household, as the packet prints it
  ("At this setting, about 1 in 10 households like yours will face a longer disruption of any one
  kind in the next 10 years; about 3 in 10 will face at least one kind that runs past its target.
  That is why the plan also gives you ways to cope when a target runs out."), read from the first
  paragraph of the packet's "Your targets" section, since the contract carries it only there. The
  figure differs by household (2 to 5 in 10 across the fixtures at 1-in-100), so when the engine's
  sentence is not at hand (the stand-in engine, or a moment after the dial is turned) the site says
  "Across all your needs together, the chance that at least one runs out is higher" with no
  number (since v0.2.0; v0.1.1 gave one fixed figure). It sits under the dial and under "How long
  to be ready for".
- **Plan months are numbered as the packet numbers them** (the packet is the oracle): month 0 is the
  plan date's month and reads "This month (October 2026)", then "Month 1 (November 2026)", "Month 4
  (February 2027)", so a purchase has the same month on the Plan screen as in the packet's
  checklists ("(month 4)"). Back later, the month the household is in keeps its number ("This
  month: month 2 (December 2026)"); the others never move. Envelopes ("ready to buy in month 12
  (October 2027)") and the done line ("After month 40 (February 2030)") use the same numbers. Keep it
  up and the summary cards give calendar dates only.
- One idea per sentence. Eighth-grade level. Natural frequencies before percentages.
- Threat and action always together: "Outages of three days or more hit about 12 of 100 households
  like yours per decade. Two weeks of stored water costs about $20 in reused bottles."
- Name the thing: "two weeks of water", not "Tier 2".
- Never a countdown, never scarcity language, never imagery of suffering.

## Components

- `RiskMatrix`: the risk matrix above: every hazard, most likely first, four columns, rare rows
  under a divider, each name a jump to its card.
- `HazardCard`: name, plain description, frequency sentence, severity band, contributing data
  sources, climate delta chip, "what it does to you" (buckets), what helps. Id `hazard-<id>`;
  "Back to the table" on the Risks screen.
- `BucketGauge`: bucket name, target days, covered days, contributing hazards.
- `ItemCard`: spec, look-for, avoid, price band, quantity for this household, why, check-off,
  paid-amount field.
- `Dial`: labelled, with the plain phrase and the jargon in brackets, each setting's per-need line,
  and the all-needs sentence for the setting chosen.
- `SourceLink` (`Sources.svelte`): citation id -> title, publisher, year, URL (with the site's name),
  retrieved date and the quoted passage, "Expert estimate" for priors; from `PlanOutput.provenance`.
  A card-footer disclosure ("Sources (3)") and a small inline one after a single number.
- `CountyMap`: inline SVG from the lazy `geo` pack; the county striped and outlined inside its
  state (a ring when it is tiny), or the counties of an ambiguous ZIP code numbered as in the list.
- `DataProgress`: the one calm line under the header while the county data loads (only after half a
  second), with the reason and "Try again" if it fails.
- `Warning`: guardrail message with the reason and a "keep anyway" affordance.

## Accessibility

WCAG 2.2 AA. Keyboard-complete. Colour never the only signal (severity bands have text labels and
patterns). Reduced-motion respected. Print view tested at 100% greyscale.

## Theming

Light and dark, following system preference with a manual override. Colours defined as tokens.
Severity palette is colour-blind safe and paired with labels.

## Persistence

`localStorage` key `rr.plan.v1` holding the household, dials, check-offs and purchases. Export and
import as JSON (`ready-reckoner-plan.json`). Display preferences (theme, expert view) are kept
separately under `rr.prefs.v1`; they hold no household data and are not part of export/import. A
"forget everything" button clears both keys (and anything else under `rr.`) and says so.

## Non-goals for v1

Accounts, sync, sharing links that carry household data, push notifications, native app stores.

## v0.2.0: the family plan and the new questions (web-interview)

### Your family plan (`#/family`) — removed in v0.3.0

This screen (its own place in the navigation, outside the numbered interview steps, with
`#/family/<section>` opening one part: `contact`, `children`, `shelter`, `leave`, `home`, `circle`,
`lawyer`) is gone. Its questions are now the interview's optional steps 6–8 (the "v0.3.0" section
below); `#/family` and `#/family/<section>` redirect to the matching step or card with no new
history entry, and the fields themselves kept their names and limits (`PlanInput.family_plan`) so a
plan saved under v0.2.0 still loads and still prints the same answers. The privacy note this screen
used to show is now `docs/PRIVACY.md`'s own "A caution about the address" section.

### Wallet cards in the packet — removed in v0.3.0

The packet no longer exists as a single Markdown document, so the wallet cards are no longer found
by matching a heading's text. They are the binder's own `wallet_cards` page (`docs/BINDER.md` §2;
`PageKind::WalletCards`), reached directly by page id at
`#/binder/wallet-cards`, with their own "Print this page" button like any other binder page.

### New questions in the interview

All optional; a question never answered stays absent ("not asked") and the engine assumes nothing.

- **Who is in your household.** Under each person's medical details, a second fold: **Help in an
  emergency for person N**, grouped as planners group access and functional needs (CMIST): *Getting
  warnings* (deaf or hard of hearing; blind or low vision; limited English), *Support and safety*
  (memory, thinking or understanding; needs someone with them; has a service animal), *Care that
  can't wait* (needs dialysis; gets home health care). The yes/no question that reveals the details
  now names "extra help in an emergency"; answering No clears both folds and says so.
- **Where you live.** "Someone sleeps below street level", only with a basement or a flat below
  ground (a hidden question keeps no answer); "Has your water system had problems?", thinking of the
  last 10 years (no notices or outages we remember / occasional problems: a boil notice or an outage
  of a few days, now and then / frequent problems: out of water, or under a boil notice, for more than
  a week in the last 10 years / not sure; the engine scales the water-outage rows by this answer, so
  "frequent" carries that definition), public water only and cleared on switching to a well;
  "Is there water nearby you could filter if the taps stopped?" (none nearby, a well, a river, lake,
  pond or creek, a rain barrel or cistern, a neighbour's well); "What do you cook on?" (gas or propane
  stove, electric stove, induction cooktop, no stove).
- **Money.** "Show me the bare minimum first" under the one-off amount (also in the settings);
  **Pay and benefits**: "Does your household rely on any of these?" (federal pay, SNAP or WIC, SSI or
  SSDI, VA benefits, unemployment benefits) with "Ticking one only adds one risk to your list: these
  payments can pause during a government shutdown or a funding gap. It stays on this device.";
  insurance gains sewer or water backup cover and, when someone earns, life or disability insurance.
- **What you already have.** An item that needs trying now and then (`Item.test_interval_months`)
  asks **When did you last try it?** once the household has some: a date field (never after today)
  and a **Tried it today** button, then "Last tried <date>". The assumed-basics box is unchanged.

### Settings on the Risks screen

The single "Allow up to 10% of my budget for rare catastrophes" box becomes a list: **All of them**
(written as `["all"]`, which also covers families added later; half-ticked when only some are) and
one box per rare family, named from the catalogue. A saved v1 plan's single switch reads as all of
them and is retired on the first change; ticking every family also writes `["all"]`. Two switches
join it: **Show me the bare minimum first** and **Show the long-horizon part of the plan**. The
settings summary adds "bare minimum first" and "rare-catastrophe allowance: 2 of 9" (or "all of
them") when they are on. Under the rare list (verify2): **Also save toward a legal emergency (bail, a
lawyer's retainer)** (`Dials.legal_opt_in`), off by default and left out of the saved plan when off;
turned on, the engine adds the bail sentence to the savings track, the savings card lists its source
(`bjs_felony_defendants_2009`), and the summary adds "also saving toward a legal emergency".

### Persistence (v2)

Every new answer lives in `input` (the household), so it is saved, exported and imported with no
change to the file's version. The one exception is when an item was last tried (`Owned.tested_on`):
kept on its entry in `input.existing` when the household had it before the plan, otherwise on its
latest check-off (`Purchase.tested_on`); the engine is given the latest date for each item.

### Shared pieces other screens use

- `web/src/lib/dials.ts`: the rare allowance read as the engine reads it (`rareFamilies`,
  `allowsRare`, `allowsEveryRareFamily`) and changed (`setRareFamily`, `setEveryRareFamily`), plus
  bare minimum and the long horizon. `RareOptIn.svelte` is the family list; the rare box may use it.
- `web/src/lib/persistence.ts`: `heldQuantity`, `testedOn`, `setTestedOn` (one tested-on date for the
  Have screen and Keep it up).
- `web/src/lib/family.ts`: `tidyFamilyPlan` (the engine's tidy), the form's edits, and
  `familySectionFor(itemId)`. `web/src/lib/conditions.ts` and `web/src/lib/guidance.ts` show any
  reviewed block trimmed for the household.

## v0.2.0: rare families, targets, validation, plan and upkeep (web-risks)

Contract v2 (`docs/ENGINE-API.md`). This section supersedes the v0.1.1 notes above where they
disagree: the rare box is no longer two columns, rare rows in the matrix jump to their own row, and
the nuclear note shows only while the nuclear row has no location term.

### Rare but severe (Risks screen; REVIEW §2.4, hazard-expansion Deliverable C)

- **Nine families, one row each**, sorted by how likely here (the engine's order: the middle of
  the range, never shown), never by how bad. The intro says so, and that each chance is a range.
- **Five columns**: *What* (the family name, a button that opens the row); *How likely for you (in
  the next N years)* (the range only, as the matrix words it, then the engine's
  `anchor_sentence`, muted: "Less likely than a house fire (about 6 in 100 for you in the next ten
  years)"); *If it reaches you* (`if_it_reaches_you`); *Why here* (the first sentence of
  `location_factor.label`, or "The same everywhere: this chance does not depend on where you
  live."); *What it changes in your plan* (`what_it_changes`, with a check mark and in green when
  it starts "Nothing").
- **Opening a row** shows *What it includes* (each `sub_cause` with its own range a year where the
  engine gives one, and its note) and a **"How this number is made"** drawer: the range over the
  chosen years and in any one year; why it is a range; *Where you live* (the full location label;
  for the nuclear family also the chance the county would be in a blast or dangerous-fallout zone
  if a large attack happened, from the class factor: "about 6 in 10 (3 to 9 in 10)"); what goes
  into it; every source (the row's, the location factor's and each sub-cause's). The expert view
  adds the location group and factor. Then "What to do if it happens" and "Back to the table".
- **What the plan spends on these**, under the table: "Nothing" until the household allows it; or
  "Your rare-event allowance (up to $X a month) buys a radiation meter ($N, around Month YYYY) for
  the nuclear attack row"; or that its basics already cover the rows it chose (the money stays in
  the main plan). "Choose what the plan may spend on" opens Your settings at the family list (the
  opt-in itself, `Dials.rare_opt_in`, belongs to the settings; web-interview's `RareOptIn`).
- **Phones and narrow windows** (48 rem and below): each family is a bordered block, each cell
  labelled; the table keeps explicit roles so screen readers still read a table.
- **Print**: the collapsed rows only (no buttons, no sub-rows); the packet prints its own
  collapsed table.

### The matrix, cards and "Also checked"

- Rare rows follow the box's order under "Rare but severe: sorted by how likely here, shown as a
  range, never ranked"; each name jumps to its row in the box and focuses its button.
- A ranked row whose chance rests on stacked expert estimates (`range_only`, such as an attack
  closing the area) shows the range over the years and the range a year, never one number; its card
  draws no picture of 100 households.
- A ranked row with named causes has a small "What it includes (N)" disclosure; the card has the
  same list with each note and its sources.
- **Also checked**: under the matrix, folded: "Also checked: N more, too rare here to list", then
  each hazard or sub-row with its rate in words, read from the engine's note in the packet ("Also
  checked, and under 1 in 100,000 a year here: …"; the v0.1 wording is read too).

### Targets (model review 3.3–3.4)

- **Badge** beside each duration target: "From records", "Partly estimates" or "Estimates", by how
  much of the target's drivers (`contributions`) rest on hazards whose chance is an expert estimate
  (`confidence: prior`): under a quarter, a quarter to three quarters, over three quarters. The
  border style differs too (solid, dashed, dotted), so it reads without colour.
- **Stress line** under the relief line, from `stress_test`: "In the worst power cut in your
  region's records (Hurricane Helene, 2024), some homes were without power for up to 1 month. It
  was worst about 40 miles away. A target of 2 weeks would have left about 20 in 100 homes there
  still waiting." (or "would have covered at least 9 in 10 homes there", with a check). The share
  still out at the target is read off the event's points exactly as rr-consequence reads them.
  Water events say "it lasted about 7 days for most homes and up to 10 days for some". Its source
  follows inline.
- **Driver bars** at the top of the target's "Why?" drawer: each hazard, its share ("65 in 100"),
  and "from records" or "an estimate"; estimates are striped. Every "Why?" drawer ends with a link
  to the validation page.

### How well do these numbers hold up? (`#/validation`; model review 3.1)

- Route `#/validation`, linked from every "Why?" drawer and from About (with the tally).
- **The tally first**: "We checked 22 real disasters. This version covered 6, partly covered 9 and
  fell short on 6. We cannot model one yet." with four boxes, and the earlier tally (6, 5, 10, 1)
  beside it. Counts come from the engine (`EngineInfo.validation`); a note appears if they ever
  disagree with the rows.
- **What the results mean**: the four words of the scoring rule, and what "In our records" means.
- **Every event**: one row each (event, place and date; who we planned for; what happened, with
  sources; what the planner says today; the result, and the result before this round's changes
  where it differs; what we changed). "In our records" marks the 11 events inside the data the
  model learned from. On narrow screens each event is a labelled block.
- **What the misses need**, **How we keep this honest**, and a link to the full test
  (`docs/VALIDATION.md`). The rows are the web's copy of that file (`web/src/lib/validation.ts`);
  a test checks them against it verdict for verdict once it is in the repository.

### Plan screen

- **Bare minimum first** (`Plan.minimum_kit`): a banner above the warnings: why (the household asked,
  or the full plan would take more than three years), that the smallest three-day kit comes first,
  and a folded list of what falls beyond three years (the `plan_too_long` warning's `related`
  items). The warning itself is not repeated below the banner.
- **Decisions** (`PlanItem.decision`): a "Decision" chip, no price, and the check-off says
  "Decided"; the details say it is never paid from the supplies budget. Grouped as the packet
  groups them (verify2): this month's in one "Decide this month (N)" block that names them on one
  line ("ID for every person (…); flood insurance, even outside a flood zone; …", the item names
  without "Decide:") with the cards folded below it; one "Decide (N): …" line in next month's
  preview and in each month of the whole plan; "Still to decide" under earlier months.
- **"With: …"** under an item that needs another first (`requires`), by name.
- **First savings goal**: the engine's `first_milestone` when sent ("First goal: $500 in savings,
  by February 2028."); otherwise the v0.1.1 nearer goal. The full goal stays beside it.
- **If it lasts for months** (`Plan.long_horizon`): after the whole plan, when a target is a month or
  more or the household asked (`Dials.long_horizon`): "Worth having" and "Worth learning", outside
  the monthly schedule.

### Keep it up

- **Tests** (`Item.test_interval_months`): "Test: jump starter pack", due from the last day it was
  tried (`Owned.tested_on`, on the household's own entry or else its latest check-off) or else from
  when it was bought; "Not tested yet: try it and record the day"; "Tested today" records the date.
- **Seasonal anchors** (`Item.season`): "Before summer: check battery fans", due on the first day of
  the season (meteorological: March, June, September, December), yearly; and a **Through the year**
  view: one card per season with its month and what to have ready.
- The **calendar file** (`.ics`) carries the tests and the seasonal checks with the rest.

### Components (v0.2.0)

- `RareBox`: as above. `RiskMatrix`: gains `alsoChecked`. `BucketGauge`: badge, stress line,
  driver bars. `ExplainButton`: takes content to show first, and ends with the validation link.
  `ItemCard`: decisions and "With:". `SavingsTrack`: the engine's first goal.
- `web/src/lib/rare.ts`, `targets.ts`, `validation.ts`: the wording and reading rules, with tests.

## v0.3.0: the consolidated interview, Prepare, the Binder and maps

Contract v3 (`docs/ENGINE-API.md`, `docs/DESIGN-DELTA-v3.md`). This section supersedes the Screens
table and the v0.2.0 notes above where they disagree: the interview is now 8 steps (1–5 required,
6–8 optional); screen 7 is **Prepare** (was Plan, `#/plan` redirects); screen 8 is the **Binder**
(was Your packet, `#/packet[/*]` redirects); the standalone **Family plan** screen (`#/family`) is
gone — its questions are now the optional steps below, and `#/family` redirects to Your places.

### The interview's three optional steps

The progress strip reads **"Step 6 of 8 · optional"** for steps 6–8, each with dashed step circles
and a **"Skip for now"** control beside Continue; the plan computes with or without them, and a
question left unanswered prints as a blank line in the binder, never an error. The Start screen's
button now reads "Add your people and places for the binder"; "Continue your plan" still looks only
at the required steps (1–5) to decide whether a plan exists.

- **Step 6, Your people (`#/people`).** One card per person from step 2, in the same order, headed
  "Person 1 (adult)" until a name is typed. Every field is optional: name or nickname; date of
  birth; phone; email; where they spend the day (work, school, child care or other — guessed from
  the person's age band when a new place card is started empty — with its name, address, phone, its
  own emergency plan, pick-up rules, the safest spot there); doctor; pharmacy; conditions;
  medications (up to 12 rows: name, dose, when taken, what for); allergies; blood type; insurance
  (carrier, plan name, member ID, group number, phone); ID notes; anything else a helper should
  know. A paragraph above the cards says what this is for (a page per person in the binder, and the
  wallet cards) and that it stays on this device and in files the household itself saves.
- **Step 7, Your places (`#/places`).** Four cards: **Your home** (street address; electric, gas
  and water companies with their outage numbers; insurer and policy number; landlord or mortgage
  company; where the kit, documents, cash and spare keys are; plus the v2 fields — gas/water/
  electrical shut-offs, the safest spot at home, neighbors who check in); **Meeting places and
  staying in touch** (the v2 fields unchanged, including "the safest spot at work or school");
  **Your neighborhood** (nearest hospital with an emergency room, urgent care, the household's
  pharmacy, the shelter the community opens, the county emergency management office, how the
  household gets local alerts); **Getting out** (the v2 fields, plus — once maps are turned on,
  §"Maps" below — "Set your home point and meeting places on a map").
- **Step 8, Contacts, pets, vehicles and documents (`#/contacts`).** The v2 trusted circle and
  lawyer fields, unchanged; **Pets and animals** (up to 8: name, kind, description, medications,
  vet, microchip or tag number, where the records are, who takes them if the household can't);
  **Vehicles** (up to 4: description, plate, insurer and policy number, what stays in the car);
  **Documents and money** (accounts, up to 12 — institution, kind, phone, and **only the last four
  digits** of the account number: a pasted full number is cut down to its last four digits as it is
  typed, and the app never asks for a full one; insurance policies not already given, up to 8; where
  the originals, the copies and the digital backup are kept). A "Put it on paper" card closes the
  step, as it did on the old Family plan screen.

### Prepare (`#/prepare`)

One job: explain the tab (these are the things to do *before* something happens) and print the
plan. **"Print your preparation plan"** prints the engine's `prepare_markdown` with the same
Markdown renderer the Binder tab's Markdown fallback uses, opening with "The three things that
matter most" (or, where it applies, the leave-first decision in its place).

### Saving a copy: the protected export

Once the saved plan holds any sensitive answer (anything in a person's profile beyond their name
and phone; the home address; the documents group; a vehicle's plate; a pet's microchip number; or a
map's home pin or a drawn route, which say where the household lives as plainly as the address),
"Save to a file" offers **"Protect this file with a passphrase"**, ticked by default, with a
passphrase field and a confirmation field (at least 8 characters). Unticking it shows a one-sentence
warning about what the plain file would expose. Opening a protected file asks for the passphrase,
says plainly when it is wrong, and — the first time — explains that a forgotten passphrase cannot be
recovered (the printed binder is the only backup at that point). **"Forget everything"** now also
deletes the `rr-maps` IndexedDB database (below), not only the two `localStorage` keys.

### The Binder screen (`#/binder`, `#/binder/<page-id>`)

**One job:** show the engine's binder (`docs/BINDER.md`) on screen, print it, or save it as a PDF
that works without the engine. A contents list stays in view beside the pages (folded by default on
a phone); opening `#/binder/<page-id>` jumps straight to that page, and the old
`#/packet/wallet-cards` address still lands on the wallet cards. Every cross-reference in the text
is a link, and every citation number jumps to its numbered source. The household's own answers
print exactly as typed — nothing in them is read as Markdown or formatting — and an answer left
blank becomes a line to write on. "Do first" steps sit bold on a grey band; "Leave or stay?" is an
If/Then table whose "Go to" column links to the page it names ("Turn to Tab 3, Home"); the four logs
are blank ruled tables; wallet cards are drawn in boxes. Each page has its own "Print this page"
button.

**Toolbar:** Download PDF (a Letter/A4 choice and a "printing on both sides" box); Print, the
browser's own, kept as a fallback; Add maps / Refresh maps (the maps panel below, unchanged from
where it is reached); a map slot with nothing in it shows one line — "No map added yet" and a
button — never an empty frame. **Hospitals:** the first time the Binder tab is opened, the browser
fetches the small county hospital list (the lazy `places` pack) and works the plan out again, so the
Neighborhood page gains its hospital table; before that, or on a site built with no such pack, the
page simply has no table and no gap. A household that never opens the Binder tab never downloads
that pack.

**The PDF**, built in the browser (works offline): a cover, "How to use this binder", and a table of
contents with real page numbers, every line a link; each tab starts a new page, and with "both
sides" ticked each tab starts on a right-hand page (a blank left page says "This page is blank on
purpose."); every page has a header (the tab number and a short title) and a footer ("page 37 of 88
· version · made … · review by …"); a cross-reference prints as an internal link that also shows the
page number, "(Tab 9, After a disaster: the first 30 days, page 74)", and a table of links gains a
"Page" column. The two dense pages — Inventory and Risks at a glance — print as one compact table
each, with column names repeated at the top of every sheet. A stored map prints with its legend,
keys, notes and credits all on the one sheet it takes. **Black and white:** a callout names its own
kind in a word (STOP, WARNING, DECIDE, NOTE) and is also set apart by border weight and shading, not
colour alone. The file is named `ready-reckoner-binder-<county>-<date>.pdf`. The Prepare sheet and
Keep it up print as their own single sheets from the same toolbar area on their own tabs; About
credits the PDF library and its one embedded font.

### Maps (step 7's Getting out card, and the Binder tab)

Both places offer **"Add maps."** Every press — never only the first — opens a **consent screen**
that names each outside service in plain words and says in one sentence what it would receive, with
a checkbox per layer (the street map and nearby places ticked by default; flood zones and wildfire
hazard ticked only where the household's own flood or wildfire chance warrants it), and two buttons,
**"Fetch maps"** and **"Not now."** Nothing is requested before that press, and the screen remembers
no answer from last time. Step 7's card shows a shorter version naming only the street map.

**The pin map** (after consent, or "Edit pins") opens an interactive OpenStreetMap street map
centred on the household's ZIP code. The household drags pins for home, the two meeting places and
"where we would go," and may draw the two ways out as click-to-add lines ("Undo last point," "Clear
this line"); **"Put it at the cross"** places a pin for anyone who cannot drag. **"Type an address
instead"** shows a warning first, then sends one request per "Search" press and offers up to three
matches to place the pin from.

**After "Fetch maps,"** three printable images appear — the neighborhood, the area and the region —
each about 7 by 5 inches on paper: the base map turned grey, with hatched flood-zone and
wildfire-hazard overlays where they apply; lettered squares for the household's own points; numbered
discs for nearby places (with leader lines when they crowd); the county line, both ways out, scale
bars in miles and kilometres, a north arrow and the map credit. Under each map sits a legend table
(name, kind, address or phone), the overlay pattern key, a dated line for anything that could not be
fetched ("Flood zones could not be fetched on October 1, 2026"), and dated credits. The panel offers
**"Refresh maps," "Edit pins"** and **"Remove maps"** (which can also forget the pins).

**Storage:** the pins and drawn routes are saved with the plan, like any other answer. The three map
images live only in this browser, in an IndexedDB database named `rr-maps` — never in the exported
file. A plan imported with pins but no images shows "Maps need refreshing." "Forget everything"
deletes the `rr-maps` database along with everything else.

### Persistence (v3)

`SavedPlan.version` is 2 (the `localStorage` key stays `rr.plan.v1`); a v1 file loads unchanged and
is written back as v2. `STEP_IDS` gains `people`, `places` and `contacts`, each marked
`optional: true` in `ROUTES`. `SavedPlan.maps` (web-only, outside `input`, §"Maps" above) holds the
pins, the drawn routes and which layers were last ticked; the map images themselves are not part of
the saved plan at all.

### Redirects (no new history entry)

`#/plan[/*]` → `#/prepare`; `#/packet` → `#/binder`; `#/packet/wallet-cards` →
`#/binder/wallet-cards`; any other `#/packet/*` → `#/binder`; `#/family` → `#/places`.
