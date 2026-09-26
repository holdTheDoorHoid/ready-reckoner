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
  `free_actions_are_grouped_into_at_most_thirty_parents` holds the cap.
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
applies_to: [bucket:water_out]      # bucket:<id>, hazard:<id>, tier:<id>, topic:<slug>
citations: [ready_gov_water, cdc_water_storage]
```

Only these four keys are allowed. There is no `reading_level` key: the validator computes the grade
from the text. The file name is the id. Ids are `bucket_<bucket>`, `hazard_<family>`,
`tier_<tier>` and `topic_<slug>`.

Body rules: under 300 words (the Sources section is not counted); the first sentence says what the
consequence is and how common it is (the engine substitutes the household's own frequency sentence
where `{frequency}` appears, and drops the placeholder when it has none; bucket and hazard blocks
must open with it); in a block for several hazards, a sentence about one of them is wrapped in
`{if:<hazard_id>}…{/if}` so the packet shows it only where that hazard is likely enough
(`docs/PACKET.md`); the second paragraph is what to do; the third is what not to do; end with a
`## Sources` section of footnotes. Every citation in the front matter is used inline as `[^id]` and
defined as `[^id]: Publisher, title (year).`, copying the registry's publisher and title (a test
checks that each footnote names its source's title). One idea per sentence. Eighth-grade reading level (the
validator computes Flesch-Kincaid and warns above 9). No countdowns, no scarcity, no imagery of
suffering, no "when the SHTF". The validator rejects pressure phrases ("you must", "act now", "don't
wait", "hurry", "while supplies last", "before it's too late" and similar) and warns on exclamation
marks.

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
  linked to a higher suicide risk for the people who live there, with the 988 line.
- Potassium iodide may be mentioned only together with "official" instructions. Nuclear and
  radiological guidance cites FEMA, CDC or NRC text and does not recommend it outside the emergency
  planning zone of a plant.

## 6. UI copy

- Buttons are verbs. Headings are plain nouns. Errors say what to do next.
- Numbers: natural frequencies ("about 12 of 100 households like yours in the next ten years"),
  then the percentage in brackets on the expert view only. Round to two significant figures. Never
  show more precision than the data (`Prior` numbers show as ranges).
- Money: whole dollars; bands as "$30–45".
- Never "you must". Prefer "the plan includes" / "you could".

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
