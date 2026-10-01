# Design delta v3: the binder, the consolidated interview and maps (v0.3.0)

Status: **authoritative for v0.3.0**. Where this document and `docs/DESIGN.md`, `docs/PACKET.md` or
`docs/UI.md` disagree, this document wins; those documents are updated by the release-docs
workstream when v0.3.0 ships. Written by the planner from the owner's request and two interview
rounds on 2026-09-27. Section numbers are stable: briefs and code comments cite them as
"DESIGN-DELTA-v3 §n".

## 0. What the owner asked for, and decided

The owner's request, in their words, condensed:

- The Family plan tab asks questions. Consolidate that into the **Your answers** tab, so all the
  information is gathered there. The family-plan questions stay the same and stay **optional**;
  where they are not answered the document shows **blanks or dashes** to fill in by hand later.
- Rename the **Plan** tab to **Prepare**, to make clear these are pre-emptive actions to do before.
- Rework the packet into a document that is **downloaded and referenced during an event**: a table
  of contents and links to jump around; sections for each important piece of information, more like
  an entire page for each person, area or substantial matter; plans for the various incidents that
  might happen, like an **airline checklist** ("go here, do this"), clear, best-practice
  recommendations. It can be much longer than 26 pages. It will be **printed and put in a binder
  with tabs** for quick, easy reference: information grouped together, plans grouped together.
- Include **maps of the area from OpenStreetMap** with the relevant data layers: something
  citywide, a focus on the neighbourhood, and so on.

Interview decisions (2026-09-27):

| # | Question | Decision |
| --- | --- | --- |
| D1 | Where does the preparation material (what to buy each month, budget, decisions, maintenance calendar, the risk and target analysis) live? | **During-event binder only.** Preparation moves to the Prepare and Keep it up tabs, each with its own short printable sheet. The binder keeps a one-page "what you should have and where it is" inventory and a one-page risks-at-a-glance summary. |
| D2 | What does the app ask about each person for their binder page? | **Ask everything**, all optional: name, date of birth, phone, medical conditions, medications and doses, allergies, blood type, doctor and pharmacy, insurance, school or work place. Easy to reprint when something changes. The saved file then holds sensitive information (see §7 for the protection). |
| D3 | Which incidents get a step-by-step checklist? | **Every risk in the household's ranked matrix, plus everyday emergencies every home needs regardless of location, plus the rare families the household opted into.** About 40 to 50 checklist pages. |
| D4 | Which reference pages besides the person pages? | **All four:** the home; the places in your life (work, school, child care); the neighbourhood and getting out; pets, vehicles and documents. |
| D5 | Where do base maps come from? | **OpenStreetMap's own tile servers, fetched only when the user presses the button, cached on the device, with the required credit; the tile address is a setting with a public-domain Census map as the fallback.** No account. |
| D6 | How does the app learn the home point? | **Drop a pin by default; "type an address instead" sends the address to OpenStreetMap's search service after a plain warning.** |

Earlier decisions folded into this release: the county **eviction column** ships with the Eviction
Lab credit line (ODC-BY, approved 2026-09-27); the optional packs (`surge`, `wildfire_places`,
`outage_events`, about 1.1 MB gzipped) are **bundled into the core pack**; the shielded bag stays as
it is; the site stays at `holdthedoorhoid.github.io/ready-reckoner`.

Planner decisions the owner has not been asked about (reversible; each is reported to the owner):
the packet tab is renamed **Binder** (§1); the checklists are grouped into three tabs by onset (§5);
ten tabs with a printable label sheet (§4); the saved file is passphrase-protected by default once it
holds sensitive answers (§7); bank account numbers are not asked, only an optional last four digits
(§2.3); a county hospital list and ZIP centroids join the data pack (§8); the wasm size budget rises
to 1.75 MB gzipped (§10).

## 1. The app after v0.3.0

Tabs, in order: **Start / Your answers** (steps 1–5 required, 6–8 optional), **Risks**, **Prepare**,
**Binder**, **Keep it up**, **Learn**, **About**. The **Family plan** tab is gone: its questions are
steps 6–8 of the interview.

Routes (`web/src/lib/router.svelte.ts`):

| Route id | Path | Title | Note |
| --- | --- | --- | --- |
| `where` … `have` | unchanged | unchanged | steps 1–5, required |
| `people` | `people` | Your people | step 6, optional (§2.1) |
| `places` | `places` | Your places | step 7, optional (§2.2) |
| `contacts` | `contacts` | Contacts, pets, vehicles and documents | step 8, optional (§2.3) |
| `prepare` | `prepare` | Prepare: what to do before | was `plan`; `#/plan` redirects here |
| `binder` | `binder`, `binder/<page-id>` | Your binder | was `packet`; `#/packet` and `#/packet/<section>` redirect to `#/binder` |
| `family` | — | — | removed; `#/family` redirects to `#/places` |

The interview's progress strip reads "Step 6 of 8 · optional" for the optional steps, each of which
has a "Skip for now" control beside "Continue"; the plan computes without them, and every field
they leave empty prints as a blank line in the binder. The Start page's "Continue your plan" and
the Prepare tab's gate are unchanged. `FamilyPlanLink.svelte` points at the optional steps.

## 2. The interview's optional steps

All new fields are **echo-only**: the engine trims them, caps their length and prints them; it never
computes with them (as `FamilyPlan` today, `docs/ENGINE-API.md` §1.1). User text is never subject
to content checks (dosing, banned phrases, reading level); renderers escape it. Every field is
optional; a group left entirely empty is omitted from the JSON.

### 2.1 Step 6, Your people (`#/people`)

One card per person from step 2, in the same order, headed "Person 1 (adult)" until a name is
given. Fields map to `Person.profile` (§3.1): name or nickname; date of birth; phone; email; where
they spend the day (kind: work, school, child care, other; the place's name, address, phone, its
own emergency plan, pick-up rules, the safest spot there); doctor (name, phone); pharmacy (name,
phone); conditions; medications (up to 12 rows: name, dose, when taken, what for); allergies;
blood type; insurance (carrier, plan name, member ID, group number, phone); ID notes ("passport
number, or where it is kept"); anything else a helper should know.

Above the card list, one paragraph says what this is for (a page per person in the binder, and the
wallet cards) and that it is kept only on this device and in files the household saves (§7).

### 2.2 Step 7, Your places (`#/places`)

Four groups, each a card:

- **Your home** → `FamilyPlan.home` (§3.2): street address; the electric, gas and water companies
  (name and outage number each); insurer (name, phone) and policy number; landlord or mortgage
  company (name, phone); where the kit is; where the documents are; where the cash is; where the
  spare keys are. Plus the v2 fields, unchanged: the gas, water and electrical shut-offs; the
  safest spot at home; the neighbours who check on you.
- **Meeting places and staying in touch** (v2 fields, unchanged): where to meet near home; where to
  meet outside the neighbourhood; the out-of-area contact; numbers to know by heart; what each
  person does at work or school and who picks up the children (the v2 free-text fields, kept as
  the general answer; per-person places live in step 6).
- **Your neighbourhood** → `FamilyPlan.neighbourhood`: nearest hospital with an emergency room
  (name, address, phone); urgent care; the pharmacy the household uses; the shelter or place the
  community opens in an emergency; the county emergency management office (name, phone); how you
  get local alerts (the county's alert service, radio station).
- **Getting out** (v2 fields, unchanged): where you would go; two ways out; roadside assistance.
  With maps enabled (§9), this card also carries "Set your home point and meeting places on a map".

### 2.3 Step 8, Contacts, pets, vehicles and documents (`#/contacts`)

- **Trusted circle** and **lawyer** (v2 fields, unchanged, same limits).
- **Pets and animals** → `FamilyPlan.pets`: one row per animal (up to 8): name, kind, description,
  medications, vet (name, phone), microchip or tag number, where the records are; plus the v2 field
  "who takes the animals if you can't".
- **Vehicles** → `FamilyPlan.vehicles`: up to 4: description ("blue 2016 hatchback"), plate,
  insurer (name, phone) and policy number, what stays in the car.
- **Documents and money** → `FamilyPlan.documents`: accounts (up to 12: institution, kind, phone,
  **last four digits only**; the app never asks for a full account number), insurance policies not
  already given (up to 8: insurer, kind, policy number, phone), where the originals are, where the
  copies are, where the digital backup is.

### 2.4 Persistence and migration (web)

`SavedPlan.version` becomes 2. A version-1 file or storage entry loads unchanged: every v2 field
keeps its name and place, and the new groups are simply absent. `progress.completed` may hold the
new step ids. `STEP_IDS` gains `people`, `places`, `contacts` and `ROUTES` marks them
`optional: true`. Map state is web-only and lives outside `input` (§9.5).

## 3. Contract v3: inputs

`ENGINE_API_VERSION = 3`. `docs/ENGINE-API.md` gets a v3 section; `web/src/engine/types.ts`
mirrors every type. Field names below are final; types3 implements them verbatim and reports any
change it has to make before other workstreams code against them. Length caps are characters after
trimming; `tidy()` cuts and drops empties as `FamilyPlan::tidy` does today.

As implemented by types3 (2026-09-27), two differences from the first draft, both recorded below:
the person's insurance type is **`HealthInsurance`** (the JSON field stays `insurance`), because
`Insurance` already names the household's insurance answers in `Finances` in Rust and TypeScript;
and every list is **optional** in the JSON (`medications?`, `pets?`, `vehicles?`, `accounts?`,
`policies?`), because an empty list is omitted like an empty group. The caps are exported by name
(`SHORT_TEXT_MAX` 40, `MEDIUM_TEXT_MAX` 60, `LABEL_TEXT_MAX` 80, `DESCRIPTION_MAX` 120,
`LONG_TEXT_MAX` 200, `NOTE_MAX` 400, `BLOOD_TYPE_MAX` 8, `PLATE_MAX` 20, `LAST4_LEN` 4, and the
list lengths); `docs/ENGINE-API.md` lists which fields use which.

### 3.1 `Person.profile`

```
Person { …v2 fields unchanged…, profile?: PersonProfile }      # omitted when empty

PersonProfile {
  name?: string(60), date_of_birth?: string(40), phone?: string(40), email?: string(80),
  place?: Place, doctor?: Contact, pharmacy?: Contact,
  conditions?: string(400), medications?: [Medication] (≤ 12), allergies?: string(200),
  blood_type?: string(8), insurance?: HealthInsurance, id_notes?: string(200), notes?: string(400) }

Place { kind: "work" | "school" | "childcare" | "other", name?: string(120), address?: string(200),
        phone?: string(40), plan?: string(400), pickup?: string(200), safest_spot?: string(200) }
                                                  # dropped by tidy when only `kind` is set
Medication { name?: string(80), dose?: string(60), schedule?: string(80), purpose?: string(80) }
HealthInsurance { carrier?: string(80), plan_name?: string(80), member_id?: string(60),
                  group_number?: string(60), phone?: string(40) }   # `Insurance` in the first draft
Contact { name?, phone?, address?: string(200) }                  # v3 adds the optional address
```

### 3.2 `FamilyPlan` additions (v2 fields unchanged)

```
FamilyPlan { …v2…, home?: HomeInfo, neighbourhood?: Neighbourhood, pets?: [PetInfo] (≤ 8),
             vehicles?: [VehicleInfo] (≤ 4), documents?: DocumentsInfo }

HomeInfo { address?: string(200), electric_utility?: Contact, gas_utility?: Contact,
           water_utility?: Contact, insurer?: Contact, policy_number?: string(60),
           landlord_or_mortgage?: Contact, where_kit?: string(200), where_documents?: string(200),
           where_cash?: string(200), where_keys?: string(200) }
Neighbourhood { hospital?: Contact, urgent_care?: Contact, pharmacy?: Contact, shelter?: Contact,
                county_emergency_office?: Contact, alerts?: string(200) }
PetInfo { name?: string(60), kind?: string(40), description?: string(120), medications?: string(200),
          vet?: Contact, microchip?: string(60), records_where?: string(200) }
VehicleInfo { description?: string(120), plate?: string(20), insurer?: Contact,
              policy_number?: string(60), kept_in_car?: string(200) }
DocumentsInfo { accounts?: [AccountInfo] (≤ 12), policies?: [PolicyInfo] (≤ 8),
                where_originals?: string(200), where_copies?: string(200), digital_backup?: string(200) }
AccountInfo { institution?: string(80), kind?: string(40), phone?: string(40), last4?: string(4) }
PolicyInfo { insurer?: string(80), kind?: string(40), policy_number?: string(60), phone?: string(40) }
```

`last4` accepts only digits (a longer value is cut to its last four digits by `tidy`, so a pasted
full number never reaches the file).

### 3.3 Other input changes

- `LocationResolved.zip_centroid?: LatLon` (output type, but set at resolve time): the ZIP code's
  centre from the new `core/zip_centroids.csv` (§8), omitted when unknown. Maps start there.
- `rr_content::policy::HouseholdFacts` gains `has_children()`, `has_pets()`, `has_vehicle()`,
  `has_powered_device()` for the new conditional spans `{if:children}`, `{if:pets}`,
  `{if:vehicle}`, `{if:powered_device}` (§5.4).
- `rr_content::ids::EVENTS`: the everyday emergencies that are not hazards, for `applies_to`:
  `gas_leak_or_co`, `missing_person`, `evacuation_order`, `shelter_in_place`, `boil_water_notice`,
  `power_outage`, `something_else`.
- `GuidanceKind` (and `GuidanceMeta.kind`, `GUIDANCE_KINDS` in `types.ts`) gains `checklist`, so
  `catalogue().guidance` can list the checklist blocks (types3, 2026-09-27).

## 4. Contract v3: the binder replaces the packet

`PlanOutput.packet_markdown` is removed. In its place:

```
PlanOutput { …, binder: Binder, prepare_markdown: string, … }
```

`prepare_markdown` is the **Prepare sheet**: a short Markdown document (the same conventions as the
v2 packet's "Your plan", "Checklists", "Documents and money: Decisions" and "Maintenance calendar"
sections, trimmed to what is not already on screen) that the Prepare and Keep it up tabs print.
It is the v2 packet's preparation content, kept cited and deterministic, and the CLI prints it
with `rr plan`.

### 4.1 The document model

```
Binder { title: string, generated_on: IsoDate, household: string, location: string,
         status_line: string, review_by: IsoDate, parts: [Part], sources: [SourceEntry],
         credits: [string] }
Part   { id: string, tab: u8 (1..=10), title: string, short_title: string (≤ 14 chars, the tab label),
         pages: [Page] }
Page   { id: string (unique in the binder), title: string, kind: PageKind, fit: "one" | "two" | "flow",
         blocks: [Block] }
PageKind = "cover" | "how_to_use" | "quick_start" | "index" | "contacts" | "person" | "wallet_cards"
         | "home" | "place" | "neighbourhood" | "getting_out" | "pets" | "vehicles" | "documents"
         | "inventory" | "risks_glance" | "checklist" | "after" | "log" | "sources"

Block  = { heading: { level: 1|2|3, text: string } }
       | { para: [Inline] }
       | { bullets: [[Inline]] }
       | { numbered: [[Inline]] }
       | { steps: [Step] }                       # airline-style: Step { text: [Inline], memory: bool }
       | { fields: [FieldRow] }                  # FieldRow { label: string, value?: string, lines: u8 }
       | { table: { header: [string], rows: [[[Inline]]] } }
       | { callout: { kind: "stop"|"warning"|"note"|"decision", title?: string, blocks: [Block] } }
       | { decision: { question: string, branches: [Branch] } }
                                                 # Branch { when: [Inline], then: [Inline], go_to?: string (page id) }
       | { map_slot: { id: string, kind: "region"|"area"|"neighbourhood", caption: string } }
       | { cards: [Card] }                       # Card { title: string, lines: [[Inline]] }  (wallet cards)
       | { log: { columns: [string], rows: u8 } }
       | { page_break: true }

Inline = { t: string } | { b: string } | { cite: [u32] } | { link: { to: string, text: string } }
       | { blank: u8 }                           # a ruled blank of about n characters
SourceEntry { n: u32, title: string, publisher: string, year?: u16, url?: string, expert: bool }
```

Names in code (types3, 2026-09-27): in Rust (`rr_types::binder`) and `types.ts` the anonymous
payloads above are the types `Heading`, `Table`, `Callout`, `Decision`, `MapSlot`, `Log` and
`Link`, with the enums `PageKind`, `Fit`, `CalloutKind` and `MapSlotKind`; in `types.ts` each
block and inline variant is its own interface (`HeadingBlock { heading: Heading }`,
`TextInline { t: string }`, …) in the unions `Block` and `Inline`. `Binder::check` returns the
structural problems below and those it can see for itself (part ids used twice, a tab label over
14 characters, sources numbered out of place, heading levels outside 1 to 3).

Rules: every `link.to` and `Branch.go_to` names an existing `Page.id` (`Binder::check`; a
`FieldRow` holds no page reference in this model); every `cite` number is in `1..=sources.len()`; `fields.value` is user text,
printed as written, or absent, in which case the renderer draws `lines` ruled lines; `steps` with
`memory: true` are the "do first from memory" items and render bold; `map_slot` is filled by the
web app (§9) and rendered by the CLI as a boxed placeholder ("Map: your neighbourhood. Add it in the
app, or paste a printed map here."). `fit: "one"` is a promise the page fits one printed page,
enforced by a word-and-row proxy in rr-plan (§5.6) and by verify3's real print.

Two renderers ship: **Markdown** (rr-plan, for the CLI, the goldens and humans: parts as `#`, pages
as `##`, blocks in the v2 packet's Markdown conventions, blanks as `__________`, cross-references as
"(Tab 3, Home)") and **HTML + PDF** (web, §6). The goldens in `fixtures/golden/` stay Markdown.

### 4.2 The parts (tabs), fixed order

| Tab | `Part.id` | Title | Pages |
| --- | --- | --- | --- |
| 1 | `start` | Start here | Cover (household, place, date, status line, `review_by`); How to use this binder; **Quick start: the first five minutes of any emergency** (universal memory items, cited); **Which checklist?** index: every ranked hazard name and everyday emergency → tab and page, the rare families not opted into listed as "not in this binder; tick the family under Your settings to add it"; **Contacts at a glance**: every number in the binder on one page (people, trusted circle, out-of-area contact, lawyer, doctors, pharmacies, utilities, insurer, landlord, vet, roadside, hospital, county office, 911/988/poison control). |
| 2 | `people` | People | One page per person (`kind: person`; the profile as `fields`, medications as a table, blanks where empty; access needs and medical flags from step 2 in words), then the wallet cards page(s) (`cards`), one card per person, names on them. |
| 3 | `home_places` | Home and places | Home (address, shut-offs with tools, utilities and outage numbers, where things are, insurer, landlord, safest spot, neighbours); one page per distinct place from the profiles (people who go there, address, phone, its plan, pick-up, safest spot; deduplicated by name); Neighbourhood (meeting places near and far, hospital, urgent care, pharmacy, shelter, alerts, county office, the county hospital list from the pack (§8), `map_slot: neighbourhood`); Getting out (where we would go, two ways out, roadside, the evacuation bucket's warning-by-cause sentence, `map_slot: area`, `map_slot: region`). |
| 4 | `pets_vehicles_documents` | Pets, vehicles and documents | Pets (one block per animal; who takes them; the pet lines the plan holds); Vehicles (each vehicle; roadside; what stays in the car); Documents and money (the Emergency Financial First Aid Kit's four parts as a checklist with blanks; accounts and policies tables; where originals, copies and backup are; cash on hand line). |
| 5 | `have` | What you have | Inventory: the plan's items by consequence bucket (owned, bought, still to buy marked), quantity, where kept (blank), next rotation or check date; the medicine list pointer; **Risks at a glance**: the ranked matrix as one table (hazard, how likely here over the horizon, how bad, the checklist's tab and page) and the rare-families table with the same columns. |
| 6 | `check_now` | Checklists: happening now | Checklist pages for fast-onset hazards and everyday emergencies (§5.2), ordered by local likelihood, everyday ones first. |
| 7 | `check_coming` | Checklists: it is coming | Forecast and warned hazards (§5.2). |
| 8 | `check_ongoing` | Checklists: it goes on | Outages and slow crises (§5.2). |
| 9 | `after` | After | The first 30 days (the v2 recovery page with the county's declarations); insurance and FEMA steps; If it lasts for months (only when the plan has it); blank logs (`log`): damage, expenses, people contacted, medications given. |
| 10 | `sources` | Sources | Sources numbered as cited; data credits; how this binder was made (versions, the data pack, the content version, the review date). |

Every part starts on a new page. Page headers carry the tab number and short title; page numbers
are global ("page 37 of 112": a single running count keeps the table of contents and every
cross-reference simple to follow on paper). The binder's TOC is generated by each renderer from the
parts and pages.

## 5. Checklists

### 5.1 Selection rule (rr-plan)

A household's binder holds one checklist page for:

1. every hazard in its ranked matrix (`Risks.ranked`, the same set the Risks screen ranks; the
   "Also checked" hazards are not included and the index says so in one line);
2. every everyday emergency in `rr_content::ids::EVENTS` (always);
3. every rare family the household opted into (`Dials.rare_opt_in`), by its family's lead hazard;
4. `something_else`: a generic page for anything not covered.

A checklist block may apply to several hazards (`applies_to`); the page prints once and the index
lists every hazard name that points to it. Within a tab, pages are ordered by the ten-year chance of
their most likely hazard, highest first; everyday emergencies come first in tab 6. A hazard with no
checklist block fails a test (`every_ranked_hazard_has_a_checklist`).

### 5.2 Which checklist goes in which tab

`onset` in the block's front matter decides the tab:

- **now** (tab 6): house fire, medical emergency, gas leak or carbon-monoxide alarm, missing
  person, tornado, earthquake, flash and river flooding, tsunami, wildfire, chemical spill or
  release, mass shooting or bombing, terrorist attack, attack or threat closing the area, nuclear
  attack or EMP (two pages), nuclear power plant accident, chemical, biological or radiological
  attack, dam or levee failure, sinkhole, landslide, lightning, strong wind, hail, break-in,
  stranded in a vehicle, burst pipe or water leak, avalanche, volcanic eruption.
- **coming** (tab 7): hurricane (two pages), coastal flooding, winter storm, ice storm, cold wave,
  heat wave, drought, dust storm, wildfire smoke, severe solar storm, pandemic, severe pandemic,
  evacuation order, shelter-in-place order, boil-water notice.
- **ongoing** (tab 8): power outage at home, regional blackout, power out for months, no tap
  water or local water or gas outage, phone or internet outage, cyberattack on services, supply
  chain disruption, medicine shortage, civil unrest, war with attacks on infrastructure, job loss,
  death or disability of an earner, long illness, government pay or benefits stop, eviction,
  financial crisis with bank closures, arrest or detention, something else.

Content authors may merge hazards whose first actions are the same (strong wind, hail and
lightning as one "severe thunderstorm" page; flash and river flooding as one page) and must keep
apart those whose first actions differ (dam failure is its own page). Every hazard name still
appears in the index.

### 5.3 The page, in airline-checklist form

Each checklist page has these blocks, in this order (the engine builds them from the block's
sections; §5.4 gives the source format):

1. **Title** and the household's own line: "Here: about 40 in 100 households like yours over ten
   years · How bad: Severe" from the matrix (everyday emergencies have none).
2. **Use this when**: one or two sentences, the trigger as a person would notice it.
3. **Do first** (`steps`, `memory: true`, at most six): one action per step, imperative, bold lead
   words. These are the items to know without reading.
4. **Then** (`steps`, at most ten).
5. **Leave or stay?** (`decision`): at most four branches, each "Leave if …", "Stay if …", "Go to
   …" or "Call …", with `go_to` to the Getting out or Home page where it applies.
6. **Where and who** (`fields`): the household's own answers relevant to this page (shelter spot,
   meeting places, out-of-area contact, utilities, hospital…) as filled fields or blanks; the
   content may add one or two lines of its own.
7. **Do not** (`bullets`, at most five).
8. **When it is over** (`bullets`, at most five; the last may link to tab 9).
9. Citations as bracket numbers at the end of each step or bullet; they resolve to tab 10.

### 5.4 Source format: `content/checklists/<slug>.md`

```yaml
---
id: check_house_fire            # file name = id; ids start with check_
title: House fire
kind: checklist
onset: now                      # now | coming | ongoing
applies_to: [hazard:house_fire] # hazard:<id> and/or event:<id> (rr_content::ids::EVENTS)
citations: [usfa_home_fire_escape, nfpa_escape_planning]
pages: 1                        # 1 (default) or 2; 2 only for hurricane and nuclear_attack
---
```

Then exactly these `##` sections in this order: `Use this when`, `Do first`, `Then`,
`Leave or stay`, `Where and who` (may be empty: the engine fills it), `Do not`, `When it is over`,
`Sources`. Steps are numbered lines (`1.`) whose first words are bold (`**Get everyone out.** …`);
"Leave or stay" and the two bullet sections are `-` lines. Every sentence that tells someone to do
something carries a `[^id]` footnote to an authority (FEMA / Ready.gov, USFA, NFPA, the Red Cross,
the National Weather Service, CDC, EPA, CPSC, DHS / CISA, NRC, FDA, a state emergency agency).
Footnotes are defined in `Sources` as today and every id is in `content/citations.toml`.

Placeholders, each rendered as the household's answer or a blank of fitting length:
`{meeting_near}`, `{meeting_far}`, `{shelter_home}`, `{shelter_work}`, `{where_go}`,
`{out_of_area_contact}`, `{gas_shutoff}`, `{water_shutoff}`, `{electric_panel}`,
`{electric_utility}`, `{gas_utility}`, `{water_utility}`, `{hospital}`, `{pharmacy}`,
`{alerts}`, `{county}`, `{shelter}`, `{county_office}`. Cross-references: `{ref:home}`, `{ref:getting_out}`, `{ref:neighbourhood}`,
`{ref:contacts}`, `{ref:after}`, `{ref:documents}`, `{ref:people}` (page ids from §4.2), rendered by each renderer
as a link or "(Tab 3, Home)".

Conditional spans: all of `docs/CONTENT_STANDARDS.md` §4's, plus `{if:children}`, `{if:pets}`,
`{if:vehicle}`, `{if:powered_device}` (§3.3). Spans never nest.

Budget and voice: at most **330 words** for a one-page checklist and **620** for a two-page one,
Sources not counted; reading level at or below grade 8 (the validator computes it); the banned
pressure phrases, the dosing rule and the firearms rule apply as to every block; no brands. Say
what to do and when. Airline checklists are terse: no explanations inside a step; the reason, if
one is needed, goes in "Use this when" or in one sentence under "Do not".

As the validator implements it (types3, 2026-09-27; `check_checklists` in
`crates/rr-content/src/validate.rs`, `docs/CONTENT_STANDARDS.md` §4): "Where and who" holds at
most three lines of the block's own, written `Label: {placeholder}` or `Label:` alone for a line to
write on; a citation is required on every step and on every "Do not" and "When it is over" bullet,
except a bullet that only points to a page with `{ref:…}`, and not on "Leave or stay" branches or
"Where and who" lines; the word count keeps every conditional span, counts each placeholder and
`{ref:…}` as one word, and leaves out the `##` headings; reading level above grade 8 is a warning;
a whole step or bullet may be conditional (the span opens right after `1. ` or `- ` and closes at
the end of the line; a step left empty is dropped and the renderer numbers the steps); a span in a
checklist may name any hazard; each hazard and event has one checklist.

### 5.5 Everyday emergencies

`event:` blocks print for every household: gas leak or carbon-monoxide alarm; missing person (a
child, or an adult who needs supervision); evacuation order (any cause: what to take, where to
go, what to shut off, how to leave word); shelter-in-place order (any cause, including a chemical
release: rooms, sealing, air, when it lifts); boil-water notice; power outage at home (any cause,
hours to days: what to switch off, food, heat and cold, medical devices, when to leave); something else (a generic page:
check for danger, get to safety, get information, call, decide leave or stay, keep a log).

### 5.6 Fit

rr-plan estimates a page's load in word units (a heading counts 6, a step or bullet its words
plus 2, a field row 8, a table row 10, a paragraph its words) against a capacity of **480 units per
printed Letter page** (the content budget of 330 words leaves room for the eight headings, about
twenty-five list items and a few household fields), and fails a test when a `fit: one` page exceeds
1.0 or a `fit: two` page 2.0. The capacity is a starting estimate: the web-binder workstream
calibrates it against the real PDF and verify3 checks every golden binder's real page counts against
`fit`; a calibration changes the constant, not the content.

## 6. The web binder: on screen, as PDF, on paper

- **On screen** (`#/binder`, `#/binder/<page-id>`): the Binder rendered to HTML from the tree (not
  from Markdown), with a sticky table of contents (parts and pages) that jumps by anchor, the
  household's maps in their slots (§9), and every cross-reference a link. The old
  `#/packet/wallet-cards` request lands on the wallet cards page.
- **Download PDF**: built in the browser from the same tree (pdfmake or an equivalent MIT/BSD
  library, loaded as a lazy chunk on first use and precached by the service worker so it works
  offline). Letter or A4. A cover, "How to use this binder", a **table of contents with page
  numbers**, PDF internal links from the TOC and every cross-reference, headers with the tab number
  and short title, footers with the running page number, the app version, the generation date and
  `review_by`. Every part starts on a new page; a "double-sided" option starts each part on a
  right-hand page. A **tab label sheet** (ten labels sized for standard ten-tab dividers, plus a
  hand-writing fallback) is the last page. Blanks are ruled lines; wallet cards are cut-out boxes
  with cut marks; maps are embedded images with a legend table, or a dashed placeholder box. One
  embedded open-licence font family (Latin and Latin Extended subsets; the lazy chunk with fonts at
  most 500 KB gzipped). File name `ready-reckoner-binder-<county>-<date>.pdf`.
- **Black and white**: no meaning carried by colour alone; severity as words; hatching for map
  overlays; a test renders a page in greyscale and checks contrast of the callout kinds.
- **Print** (the browser's own print) stays as the fallback and prints the HTML with the same page
  breaks where the browser can honour them.
- **Prepare sheet** and **Keep it up sheet**: the Prepare tab prints `prepare_markdown` with the
  existing Markdown renderer under a "Print your preparation plan" button; the Keep it up tab
  prints its calendar.

## 7. Sensitive answers: storage, export and the privacy promise

Browser storage is unchanged (one `localStorage` entry, the app's own origin, cleared by "Forget
everything", which now also clears the maps store, §9.5). The **export file** changes:

- When the saved plan holds any sensitive answer (anything in `Person.profile` beyond `name` and
  `phone`; `FamilyPlan.home.address`; `documents`; `vehicles.plate`; `pets.microchip`), "Save to a
  file" offers **"Protect this file with a passphrase"**, on by default, with the plain-file choice
  behind a one-sentence warning. Protection is WebCrypto: PBKDF2-SHA-256 (600,000 iterations,
  16-byte salt) to an AES-GCM-256 key; the file is
  `{ format: "ready-reckoner-plan-encrypted", version: 1, kdf: {…}, iv, ciphertext }`; import asks
  for the passphrase and explains that a forgotten passphrase cannot be recovered (the printed
  binder is the backup).
- The file-saved warning names what the file holds in plain words ("names, phone numbers, medical
  details, insurance IDs and your address").
- `docs/PRIVACY.md` is rewritten for v0.3.0: what the device holds, what the file holds and how it
  is protected, and the first optional lookups (§9) with exactly who sees what, replacing the
  "Optional lookups that do not exist yet" section. The end-to-end check that a first visit makes
  no external request stays and must pass.

## 8. Data pack changes (data3)

1. **Eviction column**: enable the Eviction Lab job in rr-etl, add the county eviction rate to
   `core/counties.csv` (or its own core file), the ODC-BY attribution in `data/manifest.json`
   (`attributions`) and on About ("Eviction data from the Eviction Lab at Princeton University,
   evictionlab.org, under the Open Data Commons Attribution License"), and wire `Eviction`'s county
   rate in rr-hazards where the national base rate is used today (goldens move; say by how much).
2. **Bundle the optional packs**: `surge`, `wildfire_places` and `outage_events` become core files
   (manifest `packs.core.files`; the loader's file groups; the service worker's start-up list; the
   wasm source's lazy ZIP group keeps the ZIP-keyed tables lazy; About's sizes; `docs/DATA_SOURCES.md`;
   `rr-etl verify`). The CLI's core-only default then includes them.
3. **ZIP centroids**: `core/zip_centroids.csv` (`zip,lat,lon`) from the Census Bureau's 2020 ZCTA
   Gazetteer (public domain), loaded lazily with the ZIP tables, surfaced as
   `LocationResolved.zip_centroid` (§3.3).
4. **Hospitals**: `core/hospitals.csv` from CMS's Hospital General Information dataset
   (data.cms.gov, public domain): `fips,name,city,state,zip,phone,emergency_services,type`, filtered
   to hospitals with emergency services, loaded lazily (a `places` group, like `geo`), read by
   rr-plan for the Neighbourhood page's county hospital table (§4.2, tab 3) with the dataset's
   release date and "check before you need it".
5. Pack version and `rr-etl manifest --rehash`; every new source in `docs/DATA_SOURCES.md` with URL,
   retrieval date, licence.

## 9. Maps (web only; the engine never sees them)

### 9.1 Principle

The engine's binder has `map_slot` blocks; the web app fills them with images the household chose
to fetch. Nothing is fetched without a **consent screen on every press** that names each recipient
and what it receives (`docs/DESIGN.md` §10, hard rule 3: warn, do not block). A first visit and a
household that never presses "Add maps" make no external request (the e2e check).

### 9.2 The three maps

| Slot | Centre and scale | Layers |
| --- | --- | --- |
| `neighbourhood` | the home pin, about 1.5 km across (zoom 15–16) | home; meeting place near home; pharmacies, grocery stores, fuel, urgent care and clinics, schools and child care (when the household has children), fire station, police (OpenStreetMap places); FEMA flood zones hatched where a flood hazard is in the matrix at 1 in 100 or more, or the household ticks it |
| `area` | the home pin, the city or county scale (zoom 11–12), the county outline from `geo/counties.json` | hospitals with emergency rooms, fire stations, police (OpenStreetMap places), the meeting place outside the neighbourhood; storm-surge zones where the home is in a surge area or hurricane is in the matrix; wildfire hazard where wildfire is in the matrix at 1 in 100 or more |
| `region` | the home and "where we would go" (zoom 8–10; scaled to fit both) | the two ways out as the household drew them, the destination pin, main roads from the base map |

Each map prints as an image with a numbered legend table under it (name, kind, address or phone
when the source has it), the credit line ("© OpenStreetMap contributors" and each layer's source
with its retrieval date), and a scale bar. Images are composed in the browser on a canvas from
tiles and overlays, desaturated and contrast-raised so they read in black and white; overlays are
hatched, not coloured; markers are numbered discs.

### 9.3 Sources and their policies

| Data | Endpoint (a setting in one module, `web/src/lib/maps/sources.ts`) | What is sent | Policy to follow |
| --- | --- | --- | --- |
| Base tiles | `https://tile.openstreetmap.org/{z}/{x}/{y}.png`; fallback: the Census TIGERweb map export (public domain) | tile coordinates (the map area) | OSMF tile usage policy: fetched only on a user's press, cached (HTTP cache and the maps store), no more than ~250 tiles per press, the browser's Referer identifies the site, attribution shown |
| Places | Overpass API (`https://overpass-api.de/api/interpreter`; fallback `https://overpass.kumi.systems/api/interpreter`) | one bounding-box query per map | at most two queries per press, 25 s timeout, attribution |
| Flood zones | FEMA National Flood Hazard Layer map service export (hazards.fema.gov, public) | the map's bounding box | federal, public; note the layer's date |
| Storm surge | NOAA National Hurricane Center storm-surge risk maps (public) if a stable image service exists; otherwise omit and print the state's "know your zone" link | bounding box | federal, public |
| Wildfire hazard | USDA Forest Service Wildfire Hazard Potential image service (public) | bounding box | federal, public |
| Address search | Nominatim (`https://nominatim.openstreetmap.org/search`), only after the D6 warning, only on an explicit "Search" press | the typed address | Nominatim usage policy: one request a second, no autocomplete, Referer |

Every origin is added to the content security policy's `connect-src` and `img-src` (the plugin in
`web/vite-plugins/pwa`), and `docs/PRIVACY.md` lists them all.

### 9.4 Interaction

- Step 7's Getting out card and the Binder tab both offer **"Add maps"**. The consent screen lists
  the recipients above with a checkbox per layer (base map and places on by default; flood, surge
  and wildfire on when their rule in §9.2 applies), says what each receives, and has "Fetch maps"
  and "Not now".
- The **pin map** (an interactive map, Leaflet or equivalent, loaded lazily, base tiles only) opens
  centred on the ZIP centroid (or the county centroid). The household drags pins for home, the two
  meeting places and "where we would go", and may draw the two ways out as click-to-add lines.
  "Type an address instead" (D6) warns, then searches Nominatim on press and offers up to three
  matches to place the pin.
- **Fetch** composes the three images, stores them, and shows them on the Binder tab with "Refresh
  maps" and "Remove maps". A failure of one source leaves that layer out and says so on the map's
  legend ("Flood zones could not be fetched on 1 October 2026").

### 9.5 Storage

`SavedPlan.maps` (web-only, outside `input`): `{ home?: LatLon, meeting_near?: LatLon,
meeting_far?: LatLon, where_go?: LatLon, routes: [[LatLon]], layers: { places, flood, surge,
wildfire }: bool, fetched_on?: IsoDate }`. Images live in an IndexedDB store `rr-maps` (keyed by
slot, with the fetch date and the legend rows), not in the export file; an imported plan with pins
but no images shows "Maps need refreshing" on the Binder tab. "Forget everything" clears the store.

## 10. Versions, budgets and release

- `ENGINE_API_VERSION = 3`; engine, CLI and app version **0.3.0**; content version bumps with the
  checklists; the data pack version bumps with §8.
- Wasm budget **1.75 MB gzipped** (v0.2.0: 1.40 MB of 1.5; about fifty checklist blocks and the
  binder code are new); `assess()` target stays under 50 ms for every fixture; the binder is built
  inside `assess` like the packet was, so its cost counts.
- Goldens: `fixtures/golden/<name>.md` is the Markdown rendering of the binder followed by the
  Prepare sheet; regenerated once per merge by the planner as in v0.2.0. Three fixture households
  gain sample profiles and family-plan groups so the goldens exercise every page kind
  (Philadelphia: full; Minot: partial; Chicago: none).
- Tests the release must pass: every fixture's binder has the ten parts in order; every page id is
  unique and every reference resolves; every citation resolves; no placeholder or conditional
  marker remains; every ranked hazard has a checklist; every `fit: one` page fits the proxy; user
  text is never altered beyond trimming and caps; the mock engine produces a shape-identical binder;
  wasm parity to the last digit on all fixtures; web suite, axe, e2e (no external request on a first
  visit; the consent screen appears before any map request); `rr validate` 88/88.

## 11. Workstreams and tiers

| Tier | Workstream | Model | Scope (brief in `~/Desktop/ready-reckoner-briefs/round3/`) |
| --- | --- | --- | --- |
| 0 | `types3` | Opus | §3, §4.1, §3.3 in rr-types, rr-content (checklist parser, validator, `EVENTS`, new facts), `docs/ENGINE-API.md` v3, `web/src/engine/types.ts`, fixture profiles; two exemplar checklists (house fire, tornado) |
| 1 | `checklists-a1`, `checklists-a2`, `checklists-b`, `checklists-c` | Opus | §5 content: tab 6 everyday and small hazards; tab 6 large fast hazards; tab 7; tab 8 |
| 1 | `data3` | Sonnet | §8 |
| 1 | `web-interview3` | Opus | §1, §2, §7 (steps 6–8, renames, redirects, persistence v2, passphrase, mock input) |
| 1 | `web-maps` | Opus | §9 (independent of the engine; a `MapSlot` fixture until the binder lands) |
| 2 | `binder` | Opus | §4, §5.1–5.3, §5.6 in rr-plan: assembly, Markdown renderer, `prepare_markdown`, CLI, goldens |
| 2 | `web-binder` | Opus | §6: HTML from the tree, PDF, tab labels, Prepare and Keep it up sheets |
| 2 | `checklist-review` | Opus | practitioner review of every checklist against the authorities, consistency, the standards; fixes in place |
| 3 | `wasm3` | Sonnet | envelope v3, sizes, parity, timing |
| 4 | `verify3` | Opus | full verification, real PDF page counts, adversarial households, a11y, e2e with the consent flow, tile counts per press |
| 4 | `release-docs3` | Sonnet | DESIGN/PACKET/UI/PRIVACY/DATA_SOURCES/ROADMAP/CHANGELOG/README, version bump |

Integration: branch `v0.3` (draft PR for CI), agents branch from `v0.3`, never push; the planner
merges, regenerates goldens once per merge, runs the suites (never Rust and web at once), and
tags v0.3.0 from `main` after verify3's go.
