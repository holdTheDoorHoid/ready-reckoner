# Changelog

Ready Reckoner has no accounts and no server, so nothing changes on you without notice. This page
lists what shipped in each version, grouped by area, in plain language. It also lists what we
already know still needs work.

## v0.1.1 — September 2026

A safety and correctness release, from a review by four independent panels (emergency management,
practitioner, catastrophe model, hazard data) and a walk through the site. Nothing in the engine
contract changed; saved plans load as before. The full review is in the project's briefs folder and
its findings are numbered in `docs/VERIFICATION.md` ("Round 2").

### Fixed: things that could have hurt someone
- **Refrigerated medicine.** The packet no longer tells anyone to throw away refrigerated medicine
  after a day without power. Insulin keeps working up to 28 days at 59–86 °F (FDA, CDC): keep it
  shaded and below 86 °F, never frozen, and never use insulin that froze or looks unusual. A cooler
  bag now counts for about one day; when the power target is two days or more and there is no
  backup power, a battery power station becomes a need and the cold-chain warning stays on until it
  is planned. A diabetes supply list and "ask about a 90-day fill" are added.
- **Leaving comes first where leaving is likely.** When the chance of having to leave is high, or a
  major hurricane or local tsunami is in the plan, the summary leads with the decision to leave.
  Shelter advice fits the home: no basement advice for apartments; high-rises shelter on or below
  the tenth floor in a hurricane; mobile homes are told to leave.
- **The hazards that can kill get a card.** Every packet has a house-fire card, plus cards for
  Severe hazards, fast ones (wildfire, floods, earthquakes) and ones the home is exposed to, up to
  nine. A new "Safety rules to learn now" section prints in every packet: two ways out, gas, the
  water heater, food at 40 °F, generator backfeed, CPR.
- **New safety lines** in every packet: a gas leak, carbon monoxide symptoms, downed power lines,
  hands-only CPR and defibrillators, private wells after a flood, yearly chimney checks. Heat stroke
  follows CDC (skin may be dry or damp; cool the person while waiting for help).
- **Life safety comes first in the plan.** Every plan starts with emergency alerts and 911, the
  household plan and contact card, and fire safety. The fire extinguisher and smoke alarms are
  life-safety items; the extinguisher is valued on all home fires, most of them small and unreported
  (CPSC). Smoke alarms: ask the fire department, the Red Cross or the landlord first; renters with
  none get a warning instead of a purchase. The bleeding-control kit is a three-day item where help
  is slow, with a counterfeit-tourniquet caution.
- **Well pumps.** A generator for a well pump is pump-rated and connected through an
  electrician-installed interlock or transfer switch (new item), with approved fuel cans (new item)
  and the CPSC/OSHA backfeed warning. Livestock store 14 days of water until pump power exists. A
  water filter counts beyond stored water only with a named raw-water source.
- **Rare-catastrophe gear** (radiation card, shielded bag) is listed only if you opt in.

### Fixed: numbers and sentences
- Landslide chances are bounded by the county's own loss record and the high-risk flood-zone
  yardstick (Utuado, Puerto Rico: 90 → 10 in 100 homes damaged over ten years), with roads cut off
  shown separately.
- Wildfire evacuations are no longer undercounted (Paradise, California: about 4.5 times more), and
  Hawaii no longer gets power shutoffs it does not have.
- Warning times include fast hazards (Lahaina: minutes, not "2 hours").
- "Mostly back to normal" describes the event behind the target (Asheville power: about 6 days,
  not half a day).
- The dial sentence says each target is for one need, and that across all needs the chance that at
  least one runs out is higher, roughly 1 in 3. The explainer, glossary and design notes agree.
- The rare-but-severe box shows a range only, and says the nuclear figure is the chance of a
  catastrophe anywhere in the world, not your household's; a location-aware version is coming.
- "Notice could be 1 minute", not "1 minutes".

### The site
- **A risk table at the top of Risks**: every risk, most likely first, with how likely, how bad and
  how sure. Each name jumps to its card; each card links back.
- Hazard cards' "What helps" fits the hazard (no warm-room step on the heat card, no fan on the
  cold card; first-aid and bleeding-control kits first for medical emergencies), keeps "N95" in
  capitals, and says "have it" for what you own.
- Plan: "Done so far" counts only steps you have checked off; the everyday basics the plan assumes
  show as "Already have". Cash is "set aside", not bought. The savings track shows a first goal,
  with a date, before the full goal.
- Fridge and freezer thermometers are a need wherever there is a power target, and an indoor
  thermometer wherever there is a heat target. The shut-off wrench and outdoor motion light appear
  only for houses. The assumed three days of food has its own help line.
- Learn shows the reviewed, cited articles; the "Draft text" labels are gone.
- Start, About and the packet's first page say that Ready Reckoner is an independent planning aid,
  not official guidance or advice, and that local officials come first.
- Sources: the FEMA household survey is cited from FEMA's own file (archived); a mis-sourced
  telehealth sentence is removed; 25 sources added.

### Packet length
The packet is about one printed page longer than v0.1.0 (Philadelphia: 23 pages). The added page
is the safety rules, the wider card rule and the cold-chain lines, and it was accepted on purpose;
v0.2.0 redesigns the packet with a fresh page budget.

## v0.1.0 — September 2026

The first version that works end to end, for the whole country, on real data. Everything below runs
in your browser. None of it needs a server.

### The risk register

- Every county in the United States now gets a risk score. That is 3,232 counties in all, including
  Connecticut's planning regions and Puerto Rico's municipios.
- Each county is scored for three kinds of hazard: natural (using FEMA, NOAA and USGS data),
  societal (a grid failure, a pandemic, a cyberattack, and others like them), and personal (a job
  loss, a house fire, a medical emergency, and others like them).
- Each hazard is turned into plain consequences: no power, no safe water, can't leave home, must
  leave home, no income, and more. Each consequence gets a target: a number of days to be ready for,
  based on how careful you want to be and how far ahead you want to plan.
- Two counting mistakes were found during testing and fixed before release. A small number of
  counties no longer show an impossible full year without power. The "major hurricane" scenario no
  longer counts some counties' storms two to four times over.

### Your plan and budget

- A month-by-month purchase plan. Free steps come first: copying documents, writing a family plan,
  storing water in containers you already own, testing your smoke alarms. Paid steps follow,
  cheapest risk reduction first, until every consequence is covered. Then the plan tells you to
  stop.
- The plan adjusts for your home and household: which floor you live on, a well or septic system, a
  mobile home, medical needs, pets, and vehicles.
- Every item says what to look for, what to avoid, and a typical price range. No brands. No store
  links.

### The interview and the website

- A full guided interview: where you live, who is in your household, how you get around, your
  budget, and what you already have. It runs the real planning engine in your browser
  (WebAssembly), not a placeholder.
- Works from a ZIP code or a county name. A ZIP code that covers more than one county shows each
  county's share, with a map, so you can pick the right one.
- Light and dark themes that follow your system setting. A keyboard-only path through the whole app.
  A printable packet that reads fine in black and white.
- Fixed: the plan screen used to fail to display when a saved-up purchase and a monthly deposit fell
  in the same month. It now shows both correctly.

### The data behind it

- A national data pack, about 3 MB compressed, built entirely from public federal sources. Every
  figure traces back to where it came from and when it was fetched. A scheduled job re-downloads and
  rebuilds the pack every quarter and opens a pull request. Nothing updates itself without a person
  reviewing it first.

### Content and sources

- A full catalogue of items and guidance text. Every number in it has a source, listed in
  `content/citations.toml`.
- Wording and citation fixes found during testing. A home-loss statistic now matches its source. An
  emergency-prescription-refill claim now says the real count — 10 of 51 states — instead of "many
  states." A food-cost constant was corrected. The nuclear-risk card no longer calls a calculated
  range an "expert estimate."

### Checked before release

An adversarial review ran the plan for 500 households, picked at random, in every part of the
country. It also checked specific numbers against their original sources. Nothing crashed. No plan
ever spent more than its budget. Every number and step had a source. And the targets always moved
the right way when a setting changed — more people never needed less water, for example. Full
results are in [docs/VERIFICATION.md](docs/VERIFICATION.md).

### Documentation

- Added this changelog, a dedicated [privacy page](docs/PRIVACY.md), and the screenshots now in the
  README.
- Documented `rr.prefs.v1`. This is a small, separate storage entry that remembers your theme and
  your expert-view choice. Earlier documentation only described the plan's own storage entry,
  `rr.plan.v1`.

## Known limitations

The README's "What it is not" section covers what Ready Reckoner does not try to do at all: give
medical, legal or financial advice; work outside the United States; or plan at better than county
detail. This list is different. It covers specific rough edges found while building v0.1.0, which we
plan to smooth out.

- **79 counties have no outage history of their own** (37 in Nebraska, 21 in Alaska, among others).
  For now, they fall back to a simpler estimate, and can read lower than a similar county next door.
  A fix is planned that borrows the state's outage pattern instead.
- **A few coastal counties may read too high.** We are re-checking how earthquake risk and outage
  history combine in some Pacific coast counties. Coos Bay, Oregon is one example, where water and
  power targets come out higher than expected.
- **A hazard card can still carry a related hazard's advice** in the packet (a cold-wave card
  showing avalanche lines, snow advice in a hot county). The site's "What helps" lists are fixed in
  v0.1.1; the packet's shared guidance blocks are split in v0.2.0.
- **Heat wave and cold wave can read as less serious than they are** for a household with a baby, an
  older adult, or no cooling or heating. A fix is planned to weigh who is in the household, not just
  typical dollar cost.
- **A well-pump generator arrives late.** The pump-rated generator, its interlock and fuel cans
  are separate lines, so on a small budget the generator can land in year two or three while the
  animals rely on 14 days of stored water. A "requires" link that keeps a bundle together is planned
  for v0.2.0.
- **Life-safety items can push other things later.** Because the extinguisher and bleeding kit now
  come first, Philadelphia's carbon monoxide alarms moved from month 2 to month 6. A per-item value
  for readiness items (v0.2.0) will settle the order.
- **The printed packet's monthly spending can look higher than it is.** In a month where a saved-up
  purchase completes and a regular deposit also lands, both currently show in that month's total. A
  fix is proposed.
- **The packet needs print polish.** Its source list is not laid out in columns yet, and a section
  can still split across a page break.
- **A handful of source links point to a secondary copy** — a mirror, or a search results page —
  instead of the original document. We are replacing these with direct links where one exists.
- **Some price ranges run higher than typical** in a few item categories. These are being checked
  against current prices.
- **Long checklists show every step at once.** A few lists, such as everything involved in leaving
  home quickly, show every step instead of the most important few first.
- **Loading can briefly pause the page**, well under a second, when the full ZIP code list or the
  first batch of county data comes in. This is more noticeable on a slower device or connection.
- **Offline use was not confirmed in every browser** during this round of testing. If the app does
  not load without a connection for you, please open an issue.
- **The content security policy can't cover everything.** GitHub Pages can't send the custom
  response headers a full policy needs, so ours is delivered as a tag in the page instead. It still
  blocks inline scripts and outside script or data sources. But it cannot stop another site from
  loading this one inside a frame. See [docs/PRIVACY.md](docs/PRIVACY.md).
- **This site currently shares its address with the project owner's other work**
  (`holdthedoorhoid.github.io`), so browser storage is technically shared with them too. A dedicated
  address is recommended before relying on this for a real emergency. See
  [docs/PRIVACY.md](docs/PRIVACY.md).
- **No independent accessibility audit yet.** The interface is built to the WCAG 2.2 AA standard:
  keyboard navigation, colour-blind-safe severity colours, reduced-motion support. But it has not
  yet had a dedicated audit, the way the numbers and the reading level have.
