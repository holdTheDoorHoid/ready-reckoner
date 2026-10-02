# The binder

The during-event binder is `PlanOutput.binder`, written by `crates/rr-plan` (`src/binder/`) from
the plan's numbers, the household's own answers and the guidance and checklist blocks in
`content/`. It replaces the v0.2.0 packet (`docs/PACKET.md`, now a pointer to this file):
`docs/DESIGN.md` §9 and `docs/DESIGN-DELTA-v3.md` §4–§6 set the shape; this file says what feeds
each page field by field, the rule for which checklists a household gets, the fit proxy and what
calibrating it against a real print found, the Markdown renderer's conventions, and the goldens.
The **Prepare sheet** (`PlanOutput.prepare_markdown`, §7 below) is the other half of what used to
be one packet: the before-an-event content (budget, purchase checklists, decisions, the
maintenance calendar), printed by the Prepare and Keep it up tabs.

Two renderers share one tree, built once inside `assess`: **Markdown** (`rr-plan`, for the CLI,
the goldens and anyone reading the binder as text, §6 below) and **HTML and PDF** (the web app,
`docs/UI.md`, DESIGN-DELTA-v3 §6), which adds a table of contents with real page numbers, internal
links, headers and footers, a tab-label sheet and embedded maps. `docs/ENGINE-API.md` ("The
binder") is the field-by-field JSON contract; this file explains what the tree means and how it is
built, not its wire format.

## 1. The document model

```
Binder { title, generated_on, household, location, status_line, review_by, parts: [Part],
         sources: [SourceEntry], credits: [string] }
Part   { id, tab: 1..=10, title, short_title (≤ 14 chars, the tab label), pages: [Page] }
Page   { id (unique), title, kind: PageKind, fit: "one" | "two" | "flow", blocks: [Block] }
```

A **page** is the unit a household finds by its tab and its place in the table of contents. Its
`fit` is a promise: `"one"` or `"two"` means the page is meant to print on that many sheets (§5
says how this is kept); `"flow"` (the Sources tab) may run to any length. A **block** is one of:

| Block | Renders as |
| --- | --- |
| `heading` | A page or section heading, level 1 to 3. |
| `para` | A paragraph of inline text. |
| `bullets`, `numbered` | A bulleted or numbered list. |
| `steps` | An airline-style step list; a step marked `memory: true` is a "do first from memory" item and renders bold (§4). |
| `fields` | Label/value rows. A row with no `value` (the household never answered) draws `lines` ruled blanks instead — never "not answered". |
| `table` | A header row and data rows, each cell a run of inline text. |
| `callout` | A boxed `stop`, `warning`, `note` or `decision` block, with its own child blocks; used for the cover's status line, the sample-data warning, and "Leaving comes first here" where it applies. |
| `decision` | A "Leave or stay?" branch: a question, then branches of `when` / `then` text, each optionally pointing (`go_to`) to another page. |
| `map_slot` | A named slot (`neighbourhood`, `area` or `region`) the web app fills with an image it composed (DESIGN-DELTA-v3 §9); the CLI draws a boxed placeholder instead. |
| `cards` | Wallet cards: one `Card` per person, each a title and a few lines, meant to be cut out. |
| `log` | A blank ruled table for the household to fill in by hand after the fact (damage, expenses, contacts, medicines). |
| `page_break` | Forces a new page; used sparingly (never inside a checklist). |

**Inline text** is a run of `{t: "..."}` (plain), `{b: "..."}` (bold), `{cite: [n, ...]}` (one or
more bracket citation numbers) and `{link: {to, text}}` (a cross-reference) spans, plus
`{blank: n}` for a ruled blank about `n` characters wide. The household's own answers
(`fields.value`, and any text pulled from its profile, home or neighbourhood answers) are printed
**exactly as typed** — never read as Markdown or escaped away, and never corrected, checked or
reformatted beyond the trimming and length caps every echo-only field already applies on save
(`docs/ENGINE-API.md`, contract v3).

**Citations** are numbered once, in first-use order, across the whole binder: `cite: [3]` points to
`sources[2]` (`SourceEntry.n` is `3`). `provenance` (the flat citation list on `PlanOutput`) starts
with the binder's sources in that same order, then the Prepare sheet's own (which numbers itself
from 1 in its own Sources section), then everything else — so a citation number means the same
thing in the binder and in `provenance`, but the two documents do not share one numbering.

**Structural rules**, checked by `Binder::check` (shared by rr-plan, the CLI and the tests) before
any binder ships: a part's `tab` runs 1 to 10 and increases; no id (part or page) repeats; a tab
label is at most 14 characters; a `link.to` or `Branch.go_to` names a real page id; a `cite` number
is within range and `sources[i].n == i + 1`; a heading level is 1 to 3. A household never sees a
leftover placeholder, blank marker, `{ref:}`, `{if:}` or footnote mark — every one is resolved or
dropped before the tree is built.

## 2. The ten tabs, page by page

Every tab starts on a new page. Page numbers are **one running count through the whole binder**
("page 37 of 112"), not restarted per tab, so the table of contents and every cross-reference point
to a single number that means the same thing on screen, in the PDF and on paper.

| Tab | Page id(s) | Kind | What feeds it |
| --- | --- | --- | --- |
| 1. Start here | `cover` | `cover` | The household's names, the place, the home address, the plan date, "review by … and whenever something changes" (`review_by`), and the status line (`docs/CONTENT_STANDARDS.md` §6). A sample-data warning callout when the plan was built on the built-in sample counties. |
| | `how_to_use` | `how_to_use` | The `plan_how_to_use_binder` guidance block: how the binder is organized and how to use it (Red Cross and Ready.gov sources). |
| | `quick_start` | `quick_start` | The `plan_quick_start` block: six universal first-five-minutes steps, all "do first from memory" (USFA, Ready.gov home fires, FCC text-to-911, the Red Cross, Ready.gov alerts and evacuation, FCC calling tips). |
| | `index` | `index` | "Which checklist?": every everyday emergency, then the household's ranked hazards, most likely first, each with its tab and page and the warning names people actually hear ("Flash flood warning" → Flooding; "Air quality alert" → Wildfire smoke); rare families not opted into named in one line, and "Also checked" hazards in another. |
| | `contacts` | `contacts` | 911, 988, Poison Help (1-800-222-1222, HRSA) and the Disaster Distress Helpline, each cited; then every number the household gave, grouped (people, trusted circle, out-of-area contact, lawyer, doctors, pharmacies, utilities, insurer, landlord, vet, roadside, hospital, county office), blanks where it gave none. |
| 2. People | `people` (kind `index`) | `index` | "Who is in this binder": one line per person, pointing to their page. |
| | `person_<n>` | `person` | That person's profile exactly as typed (§2.1 of the interview): name, birth date, phone, medical conditions, a medications table, allergies, blood type, insurance, where they spend the day, "from your answers" in plain words. Blanks where the household skipped a field. |
| | `special_needs` (kind `person`) | `person` | Access and functional needs and medical flags gathered across every person, in one place (deaf or hard of hearing; needs help; a service animal; dialysis; home health). |
| | `wallet_cards` | `wallet_cards` | One card per person, named, sized to cut out: the out-of-area contact, the two meeting places, the lawyer, the trusted circle, numbers to know by heart, a medical-notes line. The old `#/packet/wallet-cards` link still lands here. |
| 3. Home and places | `home` | `home` | The home's address, utilities (company and outage number each), insurer, landlord or mortgage company, the gas/water/electrical shut-offs, where the kit/documents/cash/keys are, the safest spot at home, neighbours who check in — plus that home kind's shelter plan (§3.2 of the UI, `plan_shelter`'s span for that housing kind). |
| | `place_<n>` | `place` | One page per distinct place in the household's life (work, school, child care), deduplicated by name: address, phone, its own emergency plan, pick-up rules, the safest spot there. |
| | `neighbourhood` | `neighbourhood` | Meeting places near and outside the neighborhood, hospital, urgent care, pharmacy, shelter, county emergency office, how the household gets alerts; the county's hospitals-with-emergency-rooms table (up to 12, from the `places` data pack, dated, "check before you need it"; one sentence for a county with none listed, nothing printed if the list has not loaded); `map_slot: neighbourhood`. |
| | `getting_out` | `getting_out` | Where the household would go, the two ways out, roadside assistance, the evacuation bucket's warning-by-cause sentence; a "Leaving comes first here" callout where D1's leave-first rule applies (`docs/PACKET.md`'s old rule, carried over: a 25-in-100 ten-year evacuation chance, a major hurricane or local tsunami in the plan, a surge zone, or a fast hazard at 10-in-100 or more); `map_slot: area` and `map_slot: region`. |
| 4. Pets, vehicles and documents | `pets` (only with animals) | `pets` | One block per animal: name, kind, medications, vet, microchip, where its records are, who takes it if the household can't. |
| | `vehicles` (only with vehicles) | `vehicles` | One block per vehicle: description, plate, insurer and policy number, what stays in the car, roadside assistance. |
| | `documents` | `documents` | The Emergency Financial First Aid Kit's four parts as a checklist with blanks; the household's accounts (institution, kind, last four digits only) and insurance policies as tables; where originals, copies and the digital backup are kept; the cash-on-hand line. |
| 5. What you have | `inventory` | `inventory` | The plan's items by consequence bucket: owned, bought or still to buy, quantity, where kept (a blank to fill in), the next rotation or check date; a pointer to the medicine list. |
| | `what_to_expect` (kind `risks_glance`) | `risks_glance` | The dial sentence and what it means for this household (DESIGN-DELTA-v3 §10; the web's Risks screen reads its dial sentence from this page's first paragraph, and its "Also checked" line from the index page), the validation backtest's headline tally, and the hazard engine's "notes on these numbers". |
| | `risks_glance` | `risks_glance` | The ranked matrix as one table (hazard, how likely here over the horizon, how bad, the checklist's tab and page) and the rare-families table with the same columns. |
| 6–8. Checklists | `check_<id>` (§3) | `checklist` | One airline-style page per ranked hazard, per everyday emergency and per opted-in rare family (§3–§4 below); tab 7 opens with `forecast`, the 48-hour list. |
| 9. After | `after` | `after` | The first 30 days: the county's own federal-declaration history, going home, insurance and records, help from FEMA, scams, roughly in that order. |
| | `after_months` (only with a long-horizon part) | `after` | The plan's long-horizon items and how likely a multi-month outage is here, when the plan has a long-horizon section (a target reaches 30 days, or the household asked). |
| | `log_damage`, `log_expenses`, `log_contacts`, `log_medications` | `log` | Four blank logs, ruled for writing by hand during and after an event. |
| 10. Sources | `sources` | `sources` | Every citation the binder uses, numbered as first cited; the data credits (Eviction Lab, CMS, Census, OpenStreetMap contributors, FEMA NFHL, USFS, Nominatim/OSMF, and the rest of `EngineInfo.attributions`); how the binder was made (engine, content and data-pack versions, the review date). |

Three page-kind choices worth knowing if you are reading the tree directly: the People index page
(`people`) uses the generic `index` kind (the same one `cover`'s index page would use, were there
more than one); `what_to_expect` and `risks_glance` both use kind `risks_glance`, because `PageKind`
has no separate kind for a prose overview versus a table; and `special_needs` uses kind `person`,
since it is one more page about people, not a new kind of its own.

## 3. Selection: which checklists a household gets

A binder holds one checklist page for:

1. every hazard in the household's ranked matrix (`Risks.ranked` — the same set the Risks screen
   ranks; "Also checked" hazards are not included, and the index page says so in one line);
2. every everyday emergency in `rr_content::ids::EVENTS` — always, regardless of location: a gas
   leak or carbon-monoxide alarm, a missing person, an evacuation order, a shelter-in-place order, a
   boil-water notice, a power outage at home, and a generic "something else" page;
3. every rare family the household opted into (`Dials.rare_opt_in`), filed under its family's lead
   hazard;

so a typical household ends up with somewhere between 40 and 50 checklist pages. A checklist block
may answer more than one hazard id (`applies_to`); it still prints once, and the index page lists
every hazard name that points to it (strong wind, hail and lightning share "Severe thunderstorm";
flash and river flooding share one page; a dam failure keeps its own page, because its first action
differs). A hazard with no matching checklist block fails a test
(`every_ranked_hazard_has_a_checklist`) — since the content round that shipped 57 pages, every one
of the 60 hazards and everyday emergencies has one, so `REQUIRE_ALL_CHECKLISTS` is `true`.

**Which tab a checklist lands in** is the block's own `onset` front-matter field, not anything
about the household:

- **`now` (tab 6, "happening now"):** house fire, medical emergency, a gas leak or CO alarm, a
  missing person, tornado, earthquake, flash and river flooding, tsunami, wildfire, a chemical spill
  or release, mass violence, an attack or threat closing the area, nuclear attack or EMP (two
  pages), a nuclear power plant accident, a chemical/biological/radiological attack, a dam or levee
  failure, sinkhole, landslide, severe thunderstorm (wind, hail, lightning), a break-in, being
  stranded in a vehicle, a burst pipe or water leak, avalanche, volcanic eruption.
- **`coming` (tab 7, "it is coming"):** hurricane (two pages), coastal flooding, winter storm, ice
  storm, cold wave, heat wave, drought, dust storm, wildfire smoke, a severe solar storm, pandemic,
  severe pandemic, an evacuation order, a shelter-in-place order, a boil-water notice.
- **`ongoing` (tab 8, "it goes on"):** a power outage at home, a regional blackout, power out for
  months, no tap water or a local water/gas outage, a phone or internet outage, a cyberattack on
  services, supply-chain disruption, a medicine shortage, civil unrest, war with attacks on
  infrastructure, job loss, the death or disability of an earner, a long illness, government pay or
  benefits stopping, eviction, a financial crisis with bank closures, arrest or detention, something
  else.

Within a tab, pages are ordered by the ten-year chance of their most likely hazard, highest first;
everyday emergencies come first in tab 6, ahead of any ranked hazard. Tab 7 opens with a `forecast`
page (the 48-hour list) before its checklists.

## 4. The checklist page, and its source format

Every checklist page follows the same airline-checklist order, built by the engine from the
source file's sections:

1. **Title** and the household's own line underneath it, from the matrix ("Here: about 40 in 100
   households like yours over ten years · How bad: Severe"; everyday emergencies have none).
2. **Use this when** — one or two sentences, the trigger as a person would actually notice it.
3. **Do first** (`steps`, `memory: true`, at most six) — the items to know without reading, one
   action each, imperative, bold lead words.
4. **Then** (`steps`, at most ten).
5. **Leave or stay?** (`decision`, at most four branches) — "Leave if …", "Stay if …", "Go to …" or
   "Call …", with `go_to` to the Getting-out or Home page where it applies.
6. **Where and who** (`fields`, at most three of the content's own lines) — the household's own
   answers relevant to this page (a shelter spot, a meeting place, the out-of-area contact, a
   utility, the hospital…), filled or drawn as a blank to write on.
7. **Do not** (`bullets`, at most five).
8. **When it is over** (`bullets`, at most five; the last may link to tab 9).

Every step or bullet that tells someone to do something carries a bracket citation to tab 10.

**Source file**, `content/checklists/<slug>.md` (the file name is the id; ids start with `check_`):

```yaml
---
id: check_house_fire
title: House fire
kind: checklist
onset: now                       # now | coming | ongoing
applies_to: [hazard:house_fire]  # hazard:<id> and/or event:<id> (rr_content::ids::EVENTS)
citations: [usfa_home_fire_escape, nfpa_escape_planning]
pages: 1                         # 1 (default), or 2 for check_hurricane and check_nuclear_attack only
---
```

then exactly these `##` sections, in this order: `Use this when`, `Do first`, `Then`,
`Leave or stay`, `Where and who` (may be left empty — the engine fills it), `Do not`,
`When it is over`, `Sources`. Steps are numbered lines whose first words are bold
(`**Get everyone out.** …`); the other sections are `-` lines. Every instruction cites a `[^id]`
footnote resolving to `content/citations.toml` (`docs/CONTENT_STANDARDS.md` §2).

**Placeholders**, each rendered as the household's own answer or a blank of fitting length:
`{meeting_near}`, `{meeting_far}`, `{shelter_home}`, `{shelter_work}`, `{where_go}`,
`{out_of_area_contact}`, `{gas_shutoff}`, `{water_shutoff}`, `{electric_panel}`,
`{electric_utility}`, `{gas_utility}`, `{water_utility}`, `{hospital}`, `{pharmacy}`, `{alerts}`,
`{county}`, `{shelter}` (the warming/cooling/public shelter the household named in step 7), and
`{county_office}` (the county emergency office's name and phone, from
`Neighbourhood.county_emergency_office`). **Cross-references** — `{ref:home}`, `{ref:getting_out}`,
`{ref:neighbourhood}`, `{ref:contacts}`, `{ref:after}`, `{ref:documents}`, `{ref:people}` — render as
a link ("Turn to Tab 3, Home") with the page number added by the web and PDF renderers.
**Conditional spans** are every one `docs/CONTENT_STANDARDS.md` §4 lists, plus `{if:children}`,
`{if:pets}`, `{if:vehicle}` and `{if:powered_device}` (contract v3): a whole step or bullet may be
conditional (the span opens right after `1. ` or `- ` and closes at the end of the line), and a
step a household's answers empty out is dropped, with the rest renumbered — so a "Leave or stay?"
branch can vanish too (the earthquake page's tsunami branch, for a household nowhere near the
coast, prints nothing).

**Budget:** at most 330 words for a one-page checklist, 620 for a two-page one (Sources not
counted; every conditional span, placeholder and `{ref:…}` counts as one word); reading level at or
below grade 8; the same banned-phrase, dosing and firearms rules as every other block; no brands.
Airline checklists are terse — no explanation inside a step; if a reason is needed, it goes in "Use
this when" or in one "Do not" sentence.

**Everyday emergencies** (`event:` blocks, print for every household regardless of its matrix): a
gas leak or carbon-monoxide alarm; a missing person (a child, or an adult needing supervision); an
evacuation order (any cause); a shelter-in-place order (any cause, including a chemical release);
a boil-water notice; a power outage at home (any cause, hours to days); and a generic "something
else" page (check for danger, get to safety, get information, call, decide leave or stay, keep a
log).

## 5. Fit: the page-budget proxy, and the real print

A page's `fit` ("one" page, "two" pages) is a **promise**, not a suggestion — a household should
never find a checklist has run onto a third sheet mid-emergency. rr-plan checks the promise at
build time with a cheap word-and-row proxy (DESIGN-DELTA-v3 §5.6): a heading counts 6 units, a step
or bullet its words plus 2, a field row 8, a table row 10, a paragraph its words, against a
capacity of **480 units per printed Letter page** (raised from an initial, too-tight estimate once
`content/checklists/` showed that a 330-word page with about 25 list items could not otherwise
reach 1.0). A `fit: one` page failing to stay at or under 1.0, or a `fit: two` page over 2.0, fails
a test. This is a planning-time estimate; the real page count is whatever a browser prints.

**Calibrating the proxy against a real PDF** (web-binder, 2026-10-01) found it holds, but only
because the PDF renderer shrinks a crowded page's type, down to an 8-point floor for ordinary text
and 7 points for the two dense tables (Inventory and Risks at a glance): at full size (9.5 pt) a
sheet holds about 330 units, so the checklists running 330–410 units print at 85–97% size and the
five running 420–445 units (severe thunderstorm, cold wave, ice storm, coastal flooding, medical
emergency) sit at the 8-point floor. **Contacts at a glance is the page closest to breaking its
promise**: Philadelphia's version (402 units) needs the floor and fills its sheet exactly. Real
sheets per tab, against the engine's own proxy, for three households without maps:

| Household | PDF pages | Sheets per tab, real PDF | Sheets per tab, engine proxy |
| --- | --- | --- | --- |
| Philadelphia | 88 | 6/7/8/3/5/14/14/14/5/8 (84) | 6/7/9/3/5/14/14/14/5/10 (87) |
| Detroit | 84 | 6/7/4/1/5/16/13/15/5/8 (80) | 6/7/5/1/5/16/13/15/5/10 (83) |
| Hays | 83 | 6/8/3/4/5/15/12/12/6/8 (79) | 6/8/4/4/5/15/12/12/6/10 (82) |

The two tabs where the proxy and the real print disagree: **tab 3** (the proxy counts Getting
out's two empty map slots as a whole page each; without maps they print as one line each) and
**tab 10** (the sources, set in two 7-point columns, fill 8 real pages, not 10). **Adding maps
costs more, not less, than the proxy assumes**: the proxy counts a map as half a page, but a
readable map with its legend takes a **whole sheet**, never splitting — with all three maps,
Philadelphia's Neighborhood page needs 2 sheets and Getting out needs 3.

**What does not fit at a readable size, even after calibration:** the owner's D1 request was for a
**one-page** inventory and a **one-page** risks summary. Neither reaches one sheet. Every ranked
hazard needs a row in Risks at a glance, and 35 hazards plus 9 rare families already total about
460 proxy units — more than one page can hold even before the household's own items are counted.
As built (the two-sheet promise each page actually carries), both hold: Inventory runs 1.66–1.97
sheets and Risks at a glance 1.87–1.97 across the fixtures tested. Forced to the 7-point floor on a
single sheet, Inventory would still need 1.44–1.77 sheets and Risks at a glance 1.53–1.61; trimming
the "Next check" column, writing "month 7" for "still to get (month 7)", shortening a risk's
checklist pointer to "Tab 6", and folding every hazard under 1-in-100 into one line together bring
it only to 1.17–1.40 sheets. **One sheet needs fewer rows, not smaller type** — a sheet at 7 points
holds about 40–45 one-line rows, and these two pages hold 42–55 rows across 13–14 groups.
**Recommendation (not yet decided): keep the two-sheet promise.** If the owner wants one sheet,
the content needs to be cut — for example showing only items still to buy, or the top 25 risks
with the rest in one line — and that choice is the owner's, not an engineering one.

## 6. The Markdown renderer

The CLI, the goldens and anyone reading the binder as text get a single Markdown document: each
part is a `#` heading, each page a `##` heading, in tab and page order. Blocks follow the v2
packet's Markdown conventions (headings, paragraphs, bulleted and numbered lists, tables, bold);
a `fields` row with no answer prints as a ruled blank (`__________`); a `decision` block prints as
a small table of "When / Then" rows; a `map_slot` with no image prints as a boxed placeholder
("Map: your neighbourhood. Add it in the app, or paste a printed map here."); a `cards` block
prints one block quote per person, with phone numbers carrying non-breaking hyphens so a number
never splits across two lines; a cross-reference (`link` or a resolved `{ref:}`) prints as
"(Tab 3, Home)" — the Markdown renderer has no page numbers to add, unlike the web and PDF
renderers. User text (`fields.value`, and anything pulled from the household's own answers) that
contains Markdown-significant characters (`**`, `|`, `#`, `[`) is escaped, so a typed answer can
never start emphasis, a link or a table cell by accident. Citations print as bracket numbers
("[3]", "[3, 7]"). `rr plan` prints the binder, a horizontal rule, then the Prepare sheet — this
whole combined document is `fixtures/golden/<name>.md`, byte for byte.

## 7. The Prepare sheet

`PlanOutput.prepare_markdown` is the before-an-event half of what the v0.2.0 packet used to be:
the summary (where the household stands, the two done months, the assumed basics), **Your plan**
(the safety rules, the budget, this month and next month's purchases, decisions and savings, the
rare-catastrophe allowance, the guardrail warnings), the purchase checklists by tier and month, the
maintenance calendar, and its own numbered Sources section (numbered separately from the binder's).
It opens with "The three things that matter most" (restored after a brief gap in early builds): the
same three sentences and sources as the v0.2.0 packet's summary, unless leaving comes first, in
which case that decision opens it instead (§2 above). The Prepare and Keep it up tabs print it with
the same Markdown renderer as the binder; the CLI prints it right after the binder, separated by a
rule.

## 8. Goldens

`fixtures/golden/<fixture>.md` holds the binder, a rule, then the Prepare sheet for all 14 fixture
households; `<fixture>.json`'s `binder`, `prepare_markdown` and `provenance` fields hold the same
content as structured data. `cargo test -p rr-plan --test goldens` compares them and prints a diff;
`RR_UPDATE_GOLDENS=1 cargo test -p rr-plan --test goldens` rewrites them — only the planner runs
this, once per merge (`COMMON.md`), and explains what moved and why in the commit message.
