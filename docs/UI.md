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
  For all needs together: "Across all your needs together, the chance that at least one runs out
  is higher, roughly 1 in 3. That is why the plan also gives you ways to cope when a target runs
  out." The "roughly 1 in 3" is said only at 1-in-100, where it was worked out; the other settings
  say "higher" without a number. It sits under the dial and under "How long to be ready for".
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

### Your family plan (`#/family`)

**One job:** write down the household's own plan, in its own words, so it prints on paper. The screen
sits outside the numbered interview steps. The site navigation lists it right after "Your answers"
("Family plan"), and each plan step whose answer belongs here links to it: "Make a household plan"
("Fill in your household plan here"), "Your trusted circle", "Legal readiness", "Plan how you would
leave" and "Know your shut-offs" (the catalogue ids are listed beside `familySectionFor` in
`web/src/lib/family.ts`).
`#/family/<section>` opens the screen at one part: `contact`, `children`, `shelter`, `leave`, `home`,
`circle`, `lawyer`.

**Privacy.** A box under the lead says the plan is saved only in this browser and in the plan file
the household saves, never sent anywhere, and never used to work out the plan. A smaller line says
the names and numbers are other people's details too, and that until the site has a web address of
its own, other pages at the same address could in principle read what the browser keeps; if that
matters, write the plan on the printed packet instead (warn, don't block; DESIGN §10).

**Fields** (every one optional free text, `PlanInput.family_plan`; `maxlength` is the engine's
limit, 300 characters for notes and 80 for names and numbers, so nothing typed is cut later):

| Section | Asks | Shown when |
| --- | --- | --- |
| Staying in touch | Someone out of the area everyone checks in with (name, phone); where to meet near home; where to meet outside the neighbourhood; numbers to know by heart (add one at a time, up to 5) | always |
| Children, school and work | Who picks up the children, and from where; what each person does at work or school | the first only with a child, toddler, baby or teenager, or when already answered; the heading says "Work and school" otherwise |
| Where to shelter | The safest spot at home; at work or school | always |
| If you have to leave | Where you would go; two ways out; who takes the animals; roadside assistance number | animals only with pets or livestock, roadside only with a vehicle (or when already answered) |
| Around the home | Gas shut-off; main water shut-off; electrical panel; neighbours who check on you | always (the gas help adds "leave it blank if you have no gas" when neither the heat nor the stove is gas) |
| Your trusted circle | Up to 4 people: name, phone, and what they hold (a spare key, copies of our papers, medical power of attorney, backup codes for our accounts) | always; "Add someone" until four |
| A lawyer | Name or office; phone | always |

**Help from the content blocks.** "Staying in touch" and "Where to shelter" each have a folded guide,
"How to plan staying in touch" (`plan_communication`) and "Where to shelter, danger by danger"
(`plan_shelter`), read from `content/guidance/plan_*.md` at build time and trimmed for the household
exactly as the packet trims them (`web/src/lib/conditions.ts` is rr-content's `apply_conditions_for`,
ported): a hazard's paragraph stays only where its ten-year chance is at least 1 in 100, the high-rise
line only in a tall building. Notes are numbered, each linked to its source. A build without the
files shows a one-paragraph fallback instead.

**Saving.** What is typed is saved as it is typed; leaving a field tidies it the way the engine
does (trimmed, capped; a blank field is removed; an empty plan leaves no `family_plan` behind).
List entries (routes, numbers, people) keep their place while being edited; the engine drops empty
ones. "Put it on paper" at the end has **Print wallet cards** (opens `#/packet/wallet-cards`), **See
the whole packet**, and, while the plan's household-plan step is not done, **Mark "…" as done**.

### Wallet cards in the packet

The packet prints the family plan right after the summary, then the wallet cards. The web finds the
cards as the section whose heading names them (its slug contains `wallet`, as in "## Wallet cards"),
or else the family-plan section (`cardsSection` in `web/src/lib/markdown.ts`). `#/packet/wallet-cards`
(or `#/packet/<any section slug>`) scrolls to that section and moves focus to its heading. The
toolbar gains **Print only the wallet cards**, which prints that section alone. Each card is a block
quote in the packet's Markdown: the web draws it with a dashed cut line, two across on paper, and
never splits one across pages.

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
them") when they are on.

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
