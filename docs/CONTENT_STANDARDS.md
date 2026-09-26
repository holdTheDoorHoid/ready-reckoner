# Content standards

How to write anything a user reads: catalogue items, guidance blocks, packet text, UI copy, and the
numbers behind them. `docs/PRINCIPLES.md` is the policy; this is the craft. `rr-content`'s validator
enforces the mechanical parts.

## 1. Every number has a home

| Kind of number | Where it lives | Must have |
| --- | --- | --- |
| Per-person or per-household quantity (water, calories, medication days) | a `quantity_rule` in `rr-supply`, referenced by an item | citation ids on the rule and on the item |
| Hazard frequency, duration, base rate | `data/` packs or `rr-consequence` effects table | source in `data/manifest.json` or a citation id; `Prior` tag if expert judgement |
| Price band | the item's `price_band_usd` | a `note` saying what the band reflects and a `retrieved` date on the item |
| Claim in prose ("outages of 3+ days are the most common reason...") | a guidance block | citation id in the block's front matter and inline `[^id]` |

If it has no citation, it does not ship. If the best available source is weak, say so in the text
("one utility's data", "an expert estimate"), tag it `Prior`, and open an issue to find better.

## 2. Citations

`content/citations.toml`. One entry per source document. Fields: `id` (snake_case, stable),
`title`, `publisher`, `year`, `url`, `retrieved` (ISO date), `quote` (the exact sentence relied on,
under 50 words), `license` (`US Government Work (public domain)`, `CC BY 4.0`, `All rights reserved
(paraphrased, not quoted)`, etc.), `prior`. Prefer, in order: federal agencies (FEMA, Ready.gov, CDC,
NOAA, USGS, USFA, EIA, Census), standards bodies (Sphere, WHO, NFPA), peer-reviewed research, state
emergency management agencies, reputable non-profits (Red Cross, ASPCA). Never: retailers (except as
a price observation), forums, influencer content, product marketing.

- **Quotes only where checked.** A `quote` is stored only when its wording was matched against the
  source on the `retrieved` date. Non-federal sources are paraphrased and carry no quote; their
  licence says "(paraphrased, not quoted)". A superscript in a quote is written with `^`.
- **Priors.** `prior = true` marks an expert estimate, not data. Its `url` points at the document
  that lists each estimate with its reasoning (`docs/QUANTITY_RULES.md`, `docs/RISK_MODEL.md`), and
  every number citing it is shown as an estimate.
- **Makers as sources.** A maker's own specification sheet can be the best source for a number (a
  generator's fuel use). Keep its exact URL, which may name the maker, but give it a neutral `title`
  and `publisher` ("Portable inverter generator (2.2 kW class): manufacturer specification";
  "Manufacturer specification sheet, archived"), because titles appear in the packet's source list.
  Brand names stay out of item text, guidance prose, the glossary and UI copy too. The validator
  checks every citation's title, publisher and quote against its brand list; only URLs are exempt.
- **Firearm words** may appear in a citation only when the permitted free action (§5) is the only
  thing that cites it.
- **Prices.** Every priced item cites the price-observation log (`rr_price_observations_…`), and its
  band is the lowest and highest unit price logged for it in `docs/PRICE_OBSERVATIONS.md` (a test
  checks this). Each row there gives the source, listing, price, units and URL.
- **Ids other crates use** are listed in `docs/CITATION_IDS.md`; a test checks each one resolves.
  Need a new one? Add it under "Requested" there. A placeholder with no source yet may be listed
  there (and exempted in the test) but never enters the registry, since every entry needs a URL.
- **Check the claim, not just the quote.** Before a guidance sentence cites a page, read the page
  and confirm it says that. If the page cannot be read, or says something weaker, drop or soften
  the sentence. A URL must point at the document itself, not a landing page.
- **Removed federal documents.** When an agency takes a page or file down, cite the Internet
  Archive capture of the agency's own URL, with the publisher "<agency> (via Internet Archive)"
  (`fema_nhs_2024`, `doe_water_heaters`). Never cite a copy on a third-party site.
- **Sites that refuse automated reads.** Some sites (cdc.gov, ready.gov, redcross.org, some state
  pages) answer plain HTTP clients with "access denied". Read the page with a normal page fetch or
  through a web-archive capture of the same URL, say how it was read in `docs/CITATION_IDS.md`, and
  store a `quote` only when the wording was matched in the page's own text. Never work around bot
  protection or a CAPTCHA; if a page cannot be read, it cannot back a sentence.
- **When federal pages disagree,** follow the agency whose job it is (CDC for first aid, FDA for
  medicines) and note the other wording in `docs/CITATION_IDS.md`. Ready.gov's heat page still
  says heat-stroke skin is "dry with no sweat"; CDC says dry or damp, and the guidance follows CDC.
- **How it was read.** When a page was not read as a plain download, say how in
  `docs/CITATION_IDS.md`: a browser page (cdc.gov and travel.state.gov refuse scripted clients),
  a summarising fetch, a site's own data endpoint, or an Internet Archive capture of the same URL
  with its date. A quote read in a browser page counts as matched when it is found in the page's
  own text; a summarising fetch never supports a quote. A live address that refuses scripted
  clients keeps its own URL in the registry (a person can open it), with "(via Internet Archive)"
  in the publisher when a capture was what was read (`state_dept_passport_card`).
- **Principles, not numbers.** A talk, essay or other personal source may back an idea or a
  checklist step ("find a lawyer before you need one"), never a number or a claim about the
  world. Wherever such a sentence also states a fact, cite an agency, legal-aid or research source
  beside it (`ollam_2022_lawyer_passport_locksmith_gun`, whose licence line says so).
- **Neutral titles.** Titles, publishers and quotes are checked like prose (brands, firearm
  words), so a source whose own title carries one gets a neutral registry title and keeps its
  URL. Footnote ids are not checked as words: an id may keep a source's own name.
- **Our own documents** (`rr_…`) point at the file on `main`. When the text is not on `main` yet
  (a decision-log entry on a release branch), store no quote and say in `docs/CITATION_IDS.md`
  where it was read.
- **A trade body is a last resort.** An industry association's consumer sheet may back a
  sentence only when no agency, university or professional-body page says the same thing, and
  `docs/CITATION_IDS.md` records the search (`wsc_wellcare_help_2025`, the hand-pump sentence).

## 3. Items

One `[[item]]` per distinct thing a household would acquire or do. Fields are in `docs/DESIGN.md`
§4.6. Rules:

- `name` is what a person would call it. "Stored drinking water", not "H2O reserve".
- `spec` is one or two sentences saying what the thing must do. It never names a brand or model.
  The validator rejects a denylist of brand tokens; if a generic term is also a brand (Thermos,
  Band-Aid), use the generic word (vacuum flask, adhesive bandage).
- `look_for` and `avoid` are 2–5 bullets each, concrete and checkable in a store aisle. For a free
  action, `look_for` is its list of steps, and for a kit you assemble (unit `bag` or `kit`) it is the
  contents; either may run to 8 entries.
- `price_band_usd` is a range for the spec, from at least two current retail observations, with a
  `note` ("DIY with reused bottles is free; bottled water about $1 per gallon") and a `retrieved`
  date on the item. Bands are reviewed at each data refresh.
- **Money reserves.** An amount of money set aside (unit `dollar`, priced at exactly $1 per dollar)
  is not a purchase, so it needs no observation; its note says how the amount is chosen.
- `free = true` for actions that cost nothing (documents, plans, phone numbers, testing alarms).
  Free items have a $0 band, sit in tier `now`, and still have a `quantity_rule` (usually `once`).
  Purchases never sit in `now`.
- **Free actions are grouped.** At most 30 free parent actions (one "household plan", one
  "documents", one "alarms", one "know your shut-offs", and so on); a parent's `look_for` lists its
  steps. A new free step joins the closest parent. It stands alone only when the plan must count it
  on its own: it has its own quantity rule (`evacuation_ride_plan`, `epinephrine_check`, the staged
  pet food and water) or a `once_if_*` rule keys on its id. The catalogue test
  `free_actions_are_grouped_into_at_most_thirty_parents` holds the cap. Three more stand alone at
  the owner's request (the Deviant Ollam lessons, 2026-09-26): "Your trusted circle"
  (`community_trusted_circle`), "Legal readiness" (`docs_legal_readiness`) and "Lockout plan"
  (`security_lockout_plan`). Each is at most 120 words across name, spec, `look_for` and `avoid`
  and cites a source beyond the talk (`the_ollam_steps_stay_short_and_cited`).
- **Assumed basics.** `assumed_basic = true` marks an everyday thing most homes already have:
  blankets, warm layers, a cooking pot, a manual can opener, a phone, a bag for each person, three
  days of ordinary food, bath towels. The plan credits them when "assume basics" is on and lists them
  in the packet, so the household can check rather than buy. Flag only what most homes really have;
  be honest, not generous (a stocked family first-aid kit is not assumed). An assumed item keeps a
  real price band, what it costs if it is missing (the phone is the exception: the plan never buys
  one, so it is a free item), and a quantity rule that counts it the way a home holds it: per person
  (blankets, layers, towels, bags, food), per teenager and adult (phone) or once (pot, can opener).
- **Kits are containers.** A go-bag, get-home bag or pet go-kit is priced as the bag or carrier alone
  (about $20 to $40 new; its note says a bag you already own costs nothing), and its `look_for` is a
  contents checklist packed from supplies the plan already counts, so nothing is bought twice. Food
  and water staged in a bag use the `staged_*` rule variants in `docs/QUANTITY_RULES.md`
  (`go_bag_water`, `pet_go_food`, ...), and only free steps (moving what you have into the bag)
  carry them.
- `rare_catastrophic = true` marks items for rare, severe events (a year-long outage, a nuclear
  emergency). They sit in tier `y1`, even when free, and the app shows them only on request.
- `quantity_rule` is `once` or a row in `docs/QUANTITY_RULES.md`, the table shared with `rr-supply`.
  A rule the content needs is requested there as a row marked "requested by content".
- Items whose rules convert calories or gallons give `energy_kcal_per_unit` or
  `volume_l_per_unit`.
- `buckets` lists every consequence the item helps with; `hazard_extras` only for genuinely
  hazard-specific items (gas shut-off wrench: earthquake; N95: wildfire smoke and pandemic).
  Thermal items point one way, so the plan can count heat and cold coverage separately: heat items
  use `["heat_wave"]`, cold items `["cold_wave", "winter_weather", "ice_storm"]`.
- `maintenance` gives rotation and check intervals with a citation (water 6 months; food by date;
  batteries yearly; documents yearly).

## 4. Guidance blocks

`content/guidance/<id>.md` with front matter:

```yaml
id: bucket_water_out
title: No running water at home
kind: bucket                        # after, plan, hazard, bucket, tier, topic or family
applies_to: [bucket:water_out]      # targets of the block's own kind (table below)
citations: [ready_gov_water, cdc_water_storage]
```

Only these five keys are allowed. There is no `reading_level` key: the validator computes the grade
from the text. The file name is the id, and the id starts with its kind (`bucket_`, `hazard_`,
`family_`, `tier_`, `plan_`, `after_`, `topic_`). A block applies to at least one target of its own
kind; the plan workstream decides where each kind prints (`docs/PACKET.md`).

| Kind | Target | What it is |
| --- | --- | --- |
| `bucket` | `bucket:<bucket id>` | one consequence (no running water, no power) |
| `hazard` | `hazard:<hazard id>` | one ranked hazard. A rare hazard may be explained by its family block instead |
| `family` | `family:<lead hazard id>` | one of the nine rare families, named by its lead hazard (`rr_types::HazardId::family`: `family:nuclear_attack`, `family:geomagnetic_storm`, ...). It has a paragraph that starts `**What it changes in your plan.**` (test `every_rare_family_has_a_family_block`), and a location-driven family says why here |
| `tier` | `tier:<tier id>` | one phase of the plan |
| `plan` | `plan:<slug>` | a page of the household's own plan (`plan:shelter`, `plan:forecast_48h`, `plan:communication`) |
| `after` | `after:<slug>` | recovery (`after:first_30_days`) |
| `topic` | `topic:<slug>` | a Learn article (`topic:validation`, `topic:strategic_sites`, ...) |

Body rules: under 300 words (the Sources section is not counted), and at most 250 for every block
added since v0.2.0 (`new_blocks_stay_within_the_briefs_word_budget` counts every conditional span
and prints the table with `--nocapture`); the first sentence says what the consequence is and how
common it is (the engine substitutes the household's own frequency sentence where `{frequency}`
appears, and drops the placeholder when it has none; bucket, hazard and family blocks must open
with it); the second paragraph is what to do; the third is what not to do; end with a `## Sources`
section of footnotes. Every citation in the front matter is used inline as `[^id]` and defined as
`[^id]: Publisher, title (year).`, copying the registry's publisher and title (a test checks that
each footnote names its source's title). One idea per sentence. Eighth-grade reading level (the
validator computes Flesch-Kincaid and warns above 9). No countdowns, no scarcity, no imagery of
suffering, no "when the SHTF". The validator rejects pressure phrases ("you must", "act now",
"don't wait" and "do not wait", "hurry", "while supplies last", "before it's too late" and similar)
and warns on exclamation marks. Say what to do and when instead ("Meet the people next door before
you need them", "Leave as soon as authorities tell you to").

**Conditional spans.** A sentence for some households only is wrapped in a span, and the packet
keeps it only for them. Spans never nest and each closes with `{/if}`. Several values may be joined
with `|` (any one matches); an unknown, empty or repeated value is an error.

| Span | Kept when | Values |
| --- | --- | --- |
| `{if:<hazard id>}` | the hazard is likely enough for this household (`docs/PACKET.md`) | hazard ids. Hazard and family blocks may name only their own hazards; bucket and tier blocks take none; plan, after and topic blocks may name any |
| `{if:home:<kind>}`, `{if:not_home:<kind>}` | the home is, or is not, of that kind | `apartment_high_rise`, `apartment_low_rise`, `rowhouse`, `detached`, `mobile_home`, `rural_property` |
| `{if:need:<need>}` | someone in the household has that access or functional need | `hearing`, `vision`, `limited_english`, `cognitive`, `supervision`, `service_animal`, `dialysis`, `home_health` |
| `{if:has:<item id>}` | the household owns the item or the plan includes it | any catalogue item id |
| `{if:benefit:<benefit>}` | the household relies on that benefit | `federal_pay`, `snap_wic`, `ssi_ssdi`, `va`, `unemployment` |

A sentence for a condition no span covers (a well, wood heat) is written plainly and opens with its
condition ("If you heat with wood, ...").

**Life-safety sentences reach paper.** A sentence that could save a life (fire escape, gas leak,
carbon monoxide, downed lines, CPR, heat stroke, medicine storage) goes in a block the packet
prints for every household it concerns, and you confirm it in the regenerated packets
(`docs/PACKET.md`). The packet prints the "What helps" and "What to avoid" paragraphs of every
active bucket block and of each hazard card. When a bucket's only hazard has a card (medical
emergency), the bucket prints a pointer to the card instead, so its life-safety lines also go in
the hazard block (`hazard_medical`). Every printed sentence costs space in every packet, and so
does each new source line, so reuse a source the block already cites when it says the same thing.

## 5. Sensitive topics (mechanical enforcement of PRINCIPLES §9)

- A drug name followed by a dose is rejected in any text. Items in category `medical` are also
  rejected when `spec`, `look_for` or `avoid` match a dosing pattern (a number with mg, mL, tablets,
  "every N hours").
- Aquarium, fish, livestock or veterinary antibiotics may be named only in a medical item's `avoid`
  list, or in a guidance sentence that also carries a warning word ("never", "not", "unsafe"). The
  plan never sets an antibiotic quantity; the one antibiotics item is a free "talk with your own
  clinician" action.
- Firearm, ammunition and weapon words appear only in the free action "If you own firearms: safe
  storage and training" (`security_firearms_safe_storage`): free, unpriced, category `security`,
  tier `now`, quantity `once`. It includes one neutral, cited sentence that a firearm at home is
  linked to a higher suicide risk for the people who live there, with the 988 line. The rule holds
  for rare hazards too: guidance says "mass violence" and "an attack" (never the words the FBI's
  report title uses), and cited titles are neutral (`fbi_active_shooter_2024`,
  `ollam_2022_lawyer_passport_locksmith_gun`).
- Arrest and detention (`hazard_arrest_or_detention`) is factual and calm: the count says it counts
  arrests, not people, and says nothing about guilt; readiness steps come from mainstream
  know-your-rights sources (ACLU, National Lawyers Guild); nothing assumes anything about the
  household.
- Potassium iodide may be mentioned only together with "official" instructions. Nuclear and
  radiological guidance cites FEMA, CDC or NRC text and does not recommend it outside the emergency
  planning zone of a plant.
- Medicine storage follows the medicine's own rule, never a blanket timer. Insulin in its original
  vial or pen keeps working 28 days at 59–86 °F (FDA `fda_insulin_emergency`): keep it below 86 °F
  and never frozen, use it if it got warmer and nothing else is available, and replace it. For
  other refrigerated medicines the reader asks the pharmacist now and writes the answer on the
  medicine list. Do not repeat "throw away refrigerated medicine after a day" next to insulin.

## 6. UI copy

- Buttons are verbs. Headings are plain nouns. Errors say what to do next.
- Numbers: natural frequencies ("about 12 of 100 households like yours in the next ten years"),
  then the percentage in brackets on the expert view only. Round to two significant figures. Never
  show more precision than the data (`Prior` numbers show as ranges).
- Money: whole dollars; bands as "$30–45".
- Never "you must". Prefer "the plan includes" / "you could".
- Two sentences have fixed wording (round-2 review, 2026-09-26); use them word for word wherever
  they appear:
  - **Status line** (packet page 1, Start screen): "Ready Reckoner is an independent, open-source
    planning aid. It is not official emergency guidance, and not medical, legal or financial
    advice. Follow instructions from your local officials first."
  - **Dial sentence** (packet, CLI, web, `topic_the_dial`, the glossary): "For any one need,
    something worse than its target comes in about 1 of every 10 ten-year stretches. Across all
    your needs together, the chance that at least one runs out is higher, roughly 1 in 3. That is
    why the plan also gives you ways to cope when a target runs out." Never say the default makes
    you "90% sure nothing worse happens": that is true for one need at a time only.

## 7. Public-domain federal text

US federal text may be quoted at length; mark it with a blockquote and the citation. Do not edit
inside a quotation; paraphrase outside it. Do not quote state or NGO text beyond a sentence without
checking its licence.

## 8. Reading-level and voice checklist

Short words. Active voice. Second person. One idea per sentence. Concrete nouns. No jargon without a
plain phrase first ("be ready for what happens in 9 of 10 ten-year periods (the 90th percentile)").
Calm, competent, unhurried; the voice of someone who has done this for a living and is not selling
anything.

## 9. Glossary

`content/glossary.toml`, one `[[term]]` per entry: `plain` (the everyday words, shown first),
`term` (the jargon, shown second), `definition` (one or two plain sentences) and optional
`citations`. The same brand, pressure-word and dose checks apply.

## 10. The state table

`content/tables/state_registries.toml` has one `[[state]]` row for each state, DC and Puerto Rico
(52 rows), with the date it was checked and the state emergency management agency's own site
(`em_agency`, `em_url`). The packet prints the household's own row; nothing about the household
leaves the device.

- **Name a tool only where it was confirmed.** `zone`, `registry` and `alerts` each hold a
  state-level tool's name, address and an optional one-sentence note, and appear only where the
  tool was confirmed on its own page (or a page the agency links to) on the `checked` date.
  Everywhere else the row sends the household to the office that runs it: "Your county emergency
  management office handles ... Find yours through the state agency: <agency> (<url>)", citing
  Ready.gov's disability and evacuation pages (registries and zones are local). `local_office`
  names a different local office (parish, borough, city or town, municipio), or is empty where the
  state agency runs the service itself (DC). `none_found = true` says plainly what was not found
  for the whole jurisdiction (Puerto Rico).
- **Refill rules carry numbers, so they cite.** Each row's `refill` follows Healthcare Ready's
  review of state emergency-refill laws, with a second source where one was read (a state
  pharmacy board, NACDS). Where two sources disagree, the row says "Rules differ ... ask your
  pharmacist" and cites both (Virginia, North Carolina); where they agree once both are read, it
  says the rule plainly (Florida's 72 hours, and 30 days in a declared emergency).
- **No vendors, no lapsed domains.** Alert systems run on a vendor platform are linked through
  the state's own page; a registry domain that has lapsed or changed hands is dropped (test
  `no_row_points_at_a_vendor_or_a_lapsed_domain`). The validator checks the codes, https
  addresses, refill sources and the text rules on every line.
