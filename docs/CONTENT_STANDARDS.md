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
(quoted under fair use)`, etc.). Prefer, in order: federal agencies (FEMA, Ready.gov, CDC, NOAA, USGS,
USFA, EIA, Census), standards bodies (Sphere, WHO, NFPA), peer-reviewed research, state emergency
management agencies, reputable non-profits (Red Cross, ASPCA). Never: retailers (except as a price
observation), forums, influencer content, product marketing.

## 3. Items

One `[[item]]` per distinct thing a household would acquire or do. Fields are in `docs/DESIGN.md`
§4.6. Rules:

- `name` is what a person would call it. "Stored drinking water", not "H2O reserve".
- `spec` is one or two sentences saying what the thing must do. It never names a brand or model.
  The validator rejects a denylist of brand tokens; if a generic term is also a brand (Thermos,
  Band-Aid), use the generic word (vacuum flask, adhesive bandage).
- `look_for` and `avoid` are 2–5 bullets each, concrete and checkable in a store aisle.
- `price_band_usd` is a range for the spec, from at least two current retail observations, with a
  `note` ("DIY with reused bottles is free; bottled water about $1 per gallon"). Bands are reviewed
  at each data refresh.
- `free = true` for actions that cost nothing (documents, plans, phone numbers, testing alarms).
  Free items still have `quantity_rule` (usually `once`).
- `buckets` lists every consequence the item helps with; `hazard_extras` only for genuinely
  hazard-specific items (gas shut-off wrench: earthquake; N95: wildfire smoke and pandemic).
- `maintenance` gives rotation and check intervals with a citation (water 6 months; food by date;
  batteries yearly; documents yearly).

## 4. Guidance blocks

`content/guidance/<id>.md` with front matter:

```yaml
id: bucket_water
applies_to: [bucket:water]          # bucket:<id>, hazard:<id>, tier:<id>, topic:<slug>
title: Going without safe tap water
reading_level: 8
citations: [ready_gov_water, cdc_water_treatment]
```

Body rules: under 300 words; the first sentence says what the consequence is and how common it is
(the engine substitutes the household's own frequency sentence where `{frequency}` appears); the
second paragraph is what to do; the third is what not to do; end with "Sources" as footnotes. One
idea per sentence. Eighth-grade reading level (the validator computes Flesch-Kincaid and warns above
9). No countdowns, no scarcity, no imagery of suffering, no "when the SHTF".

## 5. Sensitive topics (mechanical enforcement of PRINCIPLES §9)

- The validator rejects items in category `medical` whose `spec`, `look_for` or `avoid` match a
  dosing pattern (a drug name followed by mg/mL/tablets/"every N hours"), and rejects the tokens
  "fish antibiotics", "aquarium", "veterinary" in medical items.
- The validator rejects any item whose text matches firearm, ammunition or weapon tokens if it has a
  price band or is not `free`. The one permitted item is the free action "If you own firearms:
  safe storage and training", category `security`.
- Nuclear/radiological guidance must cite FEMA/CDC/NRC text and may not recommend potassium iodide
  outside the emergency planning zone of a plant.

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
