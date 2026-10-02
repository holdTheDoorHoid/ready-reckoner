# Changelog

Ready Reckoner has no accounts and no server, so nothing changes on you without notice. This page
lists what shipped in each version, grouped by area, in plain language. It also lists what we
already know still needs work.

## v0.3.0 — October 2026

The biggest change is what you end up with. Earlier versions handed you a packet to read once,
with a page budget to stay inside. Ready Reckoner now builds a **during-event binder** — as long as
it needs to be, organized into ten tabs so you can find the one page you need without reading the
rest — alongside a short **Prepare sheet** for the things to do beforehand. The interview now asks,
optionally, for the details the binder's pages need. You can add printable maps of your area. A
saved file with sensitive details in it is protected with a passphrase by default. And the renter's
eviction figure is rebuilt on a sounder footing. Nothing about privacy changes in kind — still no
server, no accounts, nothing sent anywhere by default — but maps are the first feature that, with
your say-so, talks to an outside service; see "Maps" below and the rewritten
[privacy page](docs/PRIVACY.md).

### The binder

- **The printable packet is now two documents.** The **binder** is what you print and put in a
  ring binder with ten tabs, for reference *during* something happening. The **Prepare** tab (see
  below) keeps the before-an-event material: the budget, the purchase checklists, the maintenance
  calendar.
- **Ten tabs:** Start here (a cover, how to use the binder, a two-minute "which checklist?" index,
  and every phone number you gave in one place); People (a page per person, then wallet cards);
  Home and places (your home, every other place in your life, your neighborhood, getting out);
  Pets, vehicles and documents; What you have (a one-page-style inventory and a risks-at-a-glance
  summary); three tabs of checklists, grouped by how much warning you get (happening now / it is
  coming / it goes on); After (the first 30 days, and logs to fill in by hand); Sources.
- **57 airline-style checklists** — "go here, do this" — cover all 60 things a household's own risks
  and everyday life might throw at it: every hazard on your ranked list, every family you opted into
  (nuclear, a severe pandemic and the rest), and seven everyday emergencies every home can face
  regardless of where it is (a gas leak or CO alarm, a missing person, an evacuation order, a
  shelter-in-place order, a boil-water notice, a power outage, or something else). Each page has the
  same shape: **Use this when**, **Do first** (the handful of things worth knowing from memory),
  **Then**, **Leave or stay?**, your own answers filled in where they matter, **Do not**, and
  **When it is over** — every instruction numbered to its source. A practitioner review pass checked
  every page against its authorities afterward: distances and rules were made consistent across
  pages (downed power lines, for instance, now say 35 feet everywhere, matching the Red Cross,
  instead of a mix of 10, 30 and 35 feet depending on which page you read), a handful of missing
  life-safety steps were added (going to high ground on foot during a tsunami or earthquake; leaving
  early if someone in the household needs extra time or help; reporting stolen cards right away
  after a break-in), and property steps (calling the insurer, unplugging electronics) were moved out
  of "Do first" to make room for the steps that actually save a life in the first minutes.
- **A page for every person, home and place that answers your own questions.** Step 6–8 of the
  interview (below) feeds a page per household member — name, birth date, medications, allergies,
  insurance, where they spend the day — plus a page for your home, one for every other place in your
  life, your neighborhood (including a list of the county's hospitals with an emergency room), and
  getting out. Anything you skip prints as a blank line to fill in by hand, never an error.
- **No page limit, by design.** A typical household's binder runs 75 to 90-some pages — this is a
  reference to keep, not something meant to be read start to finish. In the one household size we
  measured closely in the finished PDF, running every checklist and reference page at normal type
  size, it came to 88 Letter pages (90 with three maps added); every page still keeps its one- or
  two-page promise, down to crowded pages printing in a smaller type rather than running long.
- **Download it as a PDF, right from your browser, and it still works offline.** The PDF has a table
  of contents with real page numbers and working links, a header and footer on every page, and a
  page of tab labels sized for a standard ten-tab divider set, to cut out and write on. Printing
  double-sided starts every tab on a right-hand page. It reads fine in black and white: a callout box
  says what kind it is in a word (STOP, WARNING, DECIDE, NOTE) as well as by its shading. The
  browser's own Print remains as a fallback.

### Your answers: three new, optional steps

- **The Family plan tab is gone — its questions are now part of the interview**, as three new
  optional steps after the five that make a plan: **Your people** (step 6), **Your places** (step
  7), and **Contacts, pets, vehicles and documents** (step 8). Skip any of it with one tap; nothing
  you leave blank blocks your plan, and it prints as a blank line on the matching binder page, ready
  to fill in by hand.
- **Ask everything, once, about each person** who might end up in the binder: date of birth, phone,
  medical conditions and medications, allergies, blood type, doctor, pharmacy, insurance, and where
  they spend the day — easy to update and reprint the moment something changes.
- **The app still never asks for a full bank account number** — only an optional last four digits,
  even if you paste in the whole thing.

### Prepare (was: Plan)

- **Renamed to say what it is: the things to do *before* something happens.** Same budget,
  month-by-month checklists, decisions and maintenance calendar as before; it is simply no longer
  mixed into the same document as the during-event binder.

### Maps

- **Add printable maps of your home's area, from OpenStreetMap, on request.** A household can ask
  for three maps — the immediate neighborhood, the wider area, and the region between home and
  where you'd go — each marked with the household's own points (home, meeting places, nearby
  pharmacies, schools and emergency rooms) and, where they apply, flood zones or wildfire hazard.
  Nothing is fetched until you press "Fetch maps," on a screen that names every outside service by
  name and says in one sentence what it would receive; that screen reappears every time, so a later
  visit never fetches anything you haven't just agreed to again.
- **Set your home point by dragging a pin on a map**, or type an address instead if you'd rather —
  that option carries its own one-time warning, since it sends the typed address to OpenStreetMap's
  search service.
- **The map images themselves stay only on your device**, in the browser's own storage, never inside
  the file you save or share; only the pins and routes you drew travel with your saved plan.

### Keeping your saved file safe

- **A saved file with sensitive details in it is protected with a passphrase by default.** Once your
  plan holds something sensitive — a person's medical details, your home address, an account, a
  pet's microchip number, or a map pin or route marking where you live — "Save a copy" offers to
  protect the file, ticked on by default, with the
  plain-file option left one clear warning away. Opening a protected file asks for the passphrase
  back and says plainly if it's wrong; there is no way to recover a forgotten one, so the printed
  binder is worth keeping as a backup.

### Your risks and the numbers behind them

- **The renter's eviction figure is rebuilt.** It previously leaned on an unverified rule of thumb
  (that roughly 4 in 10 eviction filings end in an actual eviction). It now counts **households
  taken to court**, not court cases (a renter can be filed against more than once a year), and uses
  the Eviction Lab's own published share of those households that are actually ordered to leave
  (about 1 in 3). In the counties where landlords file repeatedly against the same renters — 27 of
  them, 14 in Maryland — a county's own figure is now capped at 7 in 100 renting households a year,
  with a line explaining why. Net effect: most counties' figures come down somewhat (Detroit's
  highest-risk renters: about 81 in 100 over ten years → about 61; Philadelphia: about 27 → 20), a
  few that had been incorrectly falling back to the national average now show their own, higher,
  capped figure instead (three Maryland counties where landlords file more than once a year per
  renter household go from a flat 21 in 100 to a capped 50).
- **County names read the way the Census Bureau writes them**, for 172 places that used to read
  wrong: Baltimore and St. Louis's independent cities no longer read as "Baltimore County" or
  "St. Louis County," Virginia's 38 independent cities are named correctly, and Alaska's boroughs,
  Puerto Rico's municipios, Connecticut's planning regions, DC and the island areas all read
  correctly too.
- **A list of the county's hospitals with an emergency room** now appears on the Neighborhood page,
  from CMS's own public hospital directory, dated so you know how fresh it is; a county with none
  listed gets a plain sentence saying so instead of an empty space.

### Data and size

- The data pack adds a county hospital list and the geographic center of every ZIP code (used to
  center the pin map); both load only when they're actually needed, so a household that never opens
  the binder or adds a map never downloads them.
- The app itself (the WebAssembly engine, now including 57 checklists' worth of content and the
  logic that assembles the binder) is about 1.55 MB compressed, against a budget raised to 1.75 MB
  to make room for it. A first visit — enough to use the app fully offline afterward — downloads
  about 5.85 MB in total; maps, the PDF tool and the county hospital list only add to that if and
  when you actually use them.

### The site

- **Spelling is more consistent in this release.** The planning engine's own text — every
  checklist, guidance block and catalogue item — now reads in American English throughout; it was a
  mix before. A few screens the engine doesn't write (a handful of headings and field labels in the
  interview, written by hand) still read "neighbourhood" rather than "neighborhood"; bringing those
  the rest of the way is a small follow-up, not a new problem introduced here.

### Known limitations

- **Nearby places on the maps depend on a shared public service (Overpass) that is sometimes slow
  or briefly unavailable.** When it doesn't answer, the map still comes back — just without that
  layer, and the map says so with a dated note.
- **Storm surge is not drawn on the maps.** NOAA does not currently offer it as a map service this
  app can use; where it would matter, the map points instead to your state's own "know your
  evacuation zone" tool.
- **Whether OpenStreetMap's tile rules allow keeping the composed map images awaits a direct answer
  from OpenStreetMap Foundation volunteers.** The app only fetches map tiles when you press the
  button, shows the result right away, and keeps the finished images rather than the raw tiles — we
  believe that is within the spirit of the rules, but have asked to be sure, and will switch to a
  public-domain Census base map instead if the answer is no.
- **Five Texas counties that the Eviction Lab's data shows as exactly zero** use the national average
  instead, because a true zero is less plausible than a gap in the underlying records.
- **A household that answers every question at the maximum length allowed** can push about 29 of the
  binder's pages past their one-page promise onto a second sheet. Nothing is cut or lost — the page
  is simply longer than planned for.
- **If the binder is opened for the first time with no internet connection**, the county hospital
  list hasn't had a chance to download yet, and the Neighborhood page says so rather than silently
  showing nothing.
- **The full plan-and-binder calculation now typically takes about 50 milliseconds in a real
  browser**, right at the target we set for feeling instant, mostly from building the much larger
  binder. Comfortable, but no longer the wide margin it was before; worth watching as more content
  is added.
- **A one-page inventory and a one-page risk summary, as originally hoped for, turned out not to fit
  at a readable type size** for any household we tried; both print as a two-page spread instead.
  We're leaving this open rather than shrinking the type further or cutting rows without asking
  first.
- Ten adversarial test households (deliberately malformed, maximum-length, or containing attempted
  script injection in their answers) were run through the full binder-and-PDF pipeline; none crashed
  it, and nothing typed into the app was ever treated as a program to run.

### Credits

This release adds three public data sources to the ones already credited on the About screen and in
every binder's Sources tab: the **Eviction Lab at Princeton University** (eviction figures, under
the Open Data Commons Attribution License), the **Centers for Medicare & Medicaid Services**
(hospital locations), and the **U.S. Census Bureau** (ZIP-code center points, with a public-domain
Census map as a fallback for OpenStreetMap). The maps feature, when you turn it on, also draws on
**OpenStreetMap contributors** (map tiles and nearby places, under the Open Database License), the
**Federal Emergency Management Agency's** flood-hazard maps, the **U.S. Forest Service's**
wildfire-hazard maps, and **OpenStreetMap Foundation's Nominatim** service for address search.
Several checklists also draw on the **National Fire Protection Association**, the **U.S. Fire
Administration**, the **Storm Prediction Center** and the **National Weather Service**, each cited
on its own page.

## v0.2.0 — September 2026

A much larger release than v0.1.1, built as separate workstreams under one shared engine contract
(contract v2), then merged, tested and checked against real disasters together. It roughly triples what Ready Reckoner plans for, adds a family plan and wallet cards,
rebuilds the outage and water models on real records instead of simple estimates, and adds a public
page that shows exactly how well the model's predictions have held up. Nothing about how your data
is handled has changed: still no server, no accounts, nothing sent anywhere.

### Your risks, including nine rare-but-severe families

- **Ten new hazards on the ranked list**, alongside the ones already there: burst pipes and other
  water leaks, wildfire smoke, dust storms, sinkholes, dam and levee failure, phone and internet
  outages, medicine shortages, a benefit or federal paycheck stopping, eviction (for renters), and a
  household member being arrested or detained (this counts arrests, never guilt). The old
  "terrorism" row is retired — it mixed two different things — and split into a disruption
  ("an attack or threat closes your area") and a personal-safety row (below). 53 hazards in total are
  now tracked, natural, societal and personal combined.
- **A new rare-but-severe table** for nine catastrophes that are real but very unlikely: nuclear
  attack or EMP, a severe solar storm, a power cut lasting months from any cause, a war that reaches
  US infrastructure, a chemical/biological/radiological attack, a pandemic far worse than COVID-19, a
  very large volcanic eruption, a financial crisis, and a mass shooting or bombing. They are sorted by
  how likely each one is **where you live**, never by how bad it would be, so a wildly unlikely but
  catastrophic event can't dominate the list. Each row shows a range (never a single made-up number),
  what it would mean if it reached you, why it's rated the way it is for your county, and what — if
  anything — it changes in your plan. For most households, most rows, the answer is "nothing beyond
  your basics."
- **The nuclear row is now specific to where you live**, from a curated list of 39 real sites
  (missile fields, command posts, the ten largest metro areas, refineries, ports) that sorts every
  county into one of six exposure classes. A household in Minot, North Dakota — home to Minot Air
  Force Base and a missile field — sees "between 1 in 1,700 and 1 in 26" over ten years and "blast or
  heavy fallout near likely targets." The same household in Coos Bay, Oregon sees "between 1 in
  100,000 and 1 in 250" and "shortages, power cuts and lost income, not blast or heavy fallout." Both
  numbers, and the plain-language reasoning behind them, are on the page.
- **At most eight risk cards now** (was nine), and a less severe hazard always gives up its spot
  first: a common-but-minor risk like a phone outage can no longer push a Serious wildfire-smoke or
  house-fire card off the page. A new ranked table at the top of the risks screen lists every hazard,
  most likely first, and every name jumps straight to its card.
- Cards can now show their named sub-causes ("**Includes:** far from emergency care; falls.") and a
  "How this number is made" drawer with the range, the sources, and why the range is as wide as it is.
- **The arrest-or-detention rate was corrected to count people, not events.** The first version
  counted arrests, so the small number of people arrested more than once inflated everyone's odds;
  Philadelphia's ten-year figure came down from about 50 of 100 households to about 30.
- Two named earthquake scenarios were renamed to say exactly what they model — "Magnitude 6.75 or
  larger earthquake on the Wasatch Front" (greater Salt Lake City) and "Magnitude 6.5 or larger
  shallow earthquake around Puget Sound" (greater Seattle) — and the Puget Sound one was recalculated
  against USGS's own regional numbers: its ten-year chance for the four Puget Sound counties is now
  about 4 in 100 households, up from a flat 5% that wasn't yet tied to USGS's own regional model.

### Your targets, and the data behind them

- **A new "unhealthy air indoors" target**, sized from each county's own smoke and dust days. Where
  the ten-year chance is 2 in 100 or higher it adds N95 respirators, a way to clean the air in one
  room (an air cleaner, or a box fan and a good filter), and a free sealed-room plan.
- **The power-outage record was rebuilt** from repaired, hour-by-hour utility data (ORNL EAGLE-I,
  with bad readings and gaps fixed first), then blended with a credibility-weighted average of nearby
  counties, so a county with a thin history of its own borrows from its region instead of reading as
  artificially calm — or, just as often, artificially extreme from one noisy year. Restoration curves
  from real events now drive a "worst on record" line under each target ("The worst power cut in your
  region's records … was winter storm, March 2018; being ready for 3 days would have left some homes
  that lost power still waiting.").
- **Water targets now weigh two things that used to be ignored:** the county's own safe
  drinking-water violation record, and the household's own answer about past problems. Philadelphia's
  clean record brought its "tap water must be treated" target down from 7 days to 5; its harder "no
  tap water at all" target rose from 3 days to 5, because that failure is now modelled more
  realistically rather than assumed rare.
- **Compound events**: a cold snap that also knocks out the power, and a heat wave that does the
  same, are now modelled as their own combined event, with their own target, instead of two separate
  risks that happen to double-book a household.
- **A public validation page and a matching packet section.** The model is checked against 22 real
  disasters — the 2021 Texas freeze, Hurricane Ida, Hurricane Helene, Hurricane Maria, the Camp Fire,
  the Lahaina fire, the 2003 Northeast blackout, the CrowdStrike outage, and more — and reports itself
  honestly: 6 fully covered, 9 partly covered, 6 fell short, 1 not modelled yet. Every miss stays on
  the page, along with what changed and why. `rr validate` on the command line fails the build if a
  verdict ever changes, better or worse, without someone noticing.
- **Household numbers moved**, mostly upward as the model got more realistic: Philadelphia's
  "can't get to a store" target rose from 10 days to 2 weeks and its medicine target from 2 weeks to 3
  weeks; Miami's medicine target rose from about 3 weeks to a month, while its "no tap water" target
  came down from 3 weeks to 2 (a more realistic reading of the county's own record); Chicago's
  ten-year chance of losing power for a day or more rose from about 30 of 100 households to 35.

### Your plan and kit

- **Bare-minimum mode.** For a long plan, or on request, the very first purchases are a small kit —
  three days of water, one light, warmth, and three days of medicine and device power — so even a
  slow, low-budget plan has an early finish line. Every one of the 14 test households now reaches this
  bare minimum within the first six months, even where the full plan takes years (Philadelphia: bare
  minimum by month 1, everything by month 40; Hays, Kansas: bare minimum by month 2, everything by
  month 58).
- **Capabilities before consumables.** Batteries are capped at two weeks' worth; after that, the plan
  asks for a way to recharge (a car charger, a small inverter, or a solar panel) instead of piling on
  more batteries forever.
- **Cooking without power** becomes a need wherever a store target is two weeks or longer, or a
  formula-fed baby is under a boil-water notice.
- **All 22 practitioner-recommended kit items**, plus a jump starter, a key safe, cleanup gear, a
  tarp kit and insect repellent, and 13 free decisions: insurance checks (home or renters, flood,
  earthquake, sewer backup, life or disability), ID and passport guidance, and five home-hardening
  steps gated by hazard and ownership, each with a grant pointer.
- **A rare-catastrophe allowance**, opt in per family and capped, that only spends after the
  three-day basics are covered: a Minot household that opts into the nuclear row can add a $25
  radiation dosimeter card and, later, a $68 shielded bag for electronics — never before its everyday
  safety.
- **A long-horizon section** for the households whose plans reach that far, naming what a two- or
  three-month power cut would mean using their own region's numbers.
- Plans generally take longer to finish in full than before — there is a great deal more to cover —
  but every household's core safety now arrives sooner, not later.

### The packet

- **Redesigned front to back.** Fifteen sections print for every household, in the same order every
  time, and two more print only when they apply: summary; your family plan; wallet cards; your risks;
  your targets; your plan; your shelter plan; the 48-hour storm/freeze/heat-wave list; checklists;
  access and functional needs; local help; documents and money; the first 30 days after a disaster; if
  it lasts for months; special needs; the maintenance calendar; sources.
- **A family plan page** you fill in by hand or on screen — meeting places, an out-of-area contact,
  a trusted circle of up to four people and what each holds, a lawyer — and **one wallet card per
  person**, sized to cut out, with phone numbers that never break across a line.
- **A page budget that held despite covering three times as much:** 25 US Letter pages (24 on A4)
  for the reference household, and all 14 test households stay at or under that even though the
  packet now covers roughly three times as many hazards as before. The extra pages (the family plan,
  wallet cards, the shelter plan, the forecast list, access needs, local help, the recovery page, the
  long-horizon section) were paid for by trimming duplicated explanation elsewhere, not by shrinking
  the type.

### The site

- **A new family-plan screen** and printable wallet cards; **a validation page** (`#/validation`)
  showing the same 22-disaster scorecard as the packet, in more detail; **a risk matrix** at the top
  of the risks screen ranking every hazard with a jump link from each name.
- **New interview questions, every one optional:** access and functional needs for each person,
  sleeping below street level, water-system problems, what you cook on, a nearby source of water to
  filter, benefits you rely on, and "show me the bare minimum first."
- 334 automated web tests; an accessibility sweep (axe) with zero violations across 66 pages in
  light, dark and phone layouts; and an end-to-end check, run against the site the way GitHub Pages
  actually serves it, confirming it still works offline and never contacts anywhere but itself.

### The command line

- `rr` now builds plans from the core data pack only, by default — a smaller, faster download.
  `--optional` or `--all-packs` bring in the surge, wildfire-places and outage-event packs for anyone
  who wants them.
- `rr explain warning <id>` now explains every warning the plan can raise, not just some of them.
- `rr validate` reproduces the full 22-disaster backtest from the command line.

### Known limitations

- **Storage lines are not built.** The plan does not yet say how much space your supplies take or
  where to keep them, beyond water, for lack of a public source on the size and weight of the rest.
- **77 counties, mostly in western Washington and coastal Alaska, run their windstorm number on a
  thin-record fallback**, because a gap in how storm events are matched to counties leaves them
  without their own wind and winter-storm history. Their numbers are best treated as rough until that
  gap is closed.
- **The validation page's 22 rows are a hand-kept copy** of the engineering record
  (`docs/VALIDATION.md`), not read live from it. A test checks that the two agree, but they are
  still two documents to keep in sync, not one.
- **The rare-catastrophe "shielded bag" for electronics** is offered whenever a household's own
  ten-year chance of an EMP passes 1 in 1,000 — the same threshold and the same nationwide chance for
  everyone in the lower 48 states. Whether that is the right gate, or the right item, is still open;
  say if you'd rather it worked differently.
- **The extra data packs (surge exposure, wildfire-prone places, individual outage events) are only
  available from the command line today**, not offered as a choice on the website. *(Addressed in
  v0.3.0: all three are now bundled into every household's data automatically, with no choice to
  make — everyone gets them.)*
- **A data column for eviction-filing rates was built** but is not shipped in this release, pending
  sign-off on the source's licence (Eviction Lab, ODC-BY). *(Addressed in v0.3.0: the licence was
  approved, the column shipped, and the underlying figure was then rebuilt on a sounder footing —
  see the v0.3.0 entry above.)*
- **The printed packet fills its page budget.** 25 US Letter pages (24 A4) for the reference
  household is a real ceiling now, not headroom — a household with a lot to plan for (a well, insulin,
  a long Puerto Rico outage) can run a page or two longer. *(Superseded in v0.3.0: the printable
  packet, and its page budget, no longer exist in this form — see "The binder" in the v0.3.0 entry
  above.)*
- **The WebAssembly download is 1.40 MB compressed, against a 1.5 MB ceiling** — about 7% of headroom
  left before the app needs to either trim data or raise the budget. *(Updated in v0.3.0: the budget
  was raised to 1.75 MB to fit the binder; see the v0.3.0 entry above for the current figure.)*
- **The full plan calculation runs in about 46 ms in a command-line JavaScript engine and about 38 ms
  in a real browser, against a 50 ms target.** Comfortable today; worth watching as more hazards and
  data are added. *(Updated in v0.3.0: building the larger binder moved this to about 50 ms in a real
  browser — right at the target; see the v0.3.0 entry above.)*
- **A household with no phone on file gets only one free step** ("get a charged mobile phone") and
  nothing else in the plan adjusts — a weather radio, for instance, doesn't move any earlier.
- **British and American spellings are mixed** through the packet and the site (neighbour/neighbor,
  litre/liter, practise, labour) — not yet passed through a single consistent style. *(Addressed in
  v0.3.0 for the planning engine's own text (checklists, guidance and catalogue items), now
  consistently American English; a handful of hand-written interview screens still read British and
  are a small follow-up.)*
- **The rare-but-severe rows are sorted by the middle of their range**, so a row can appear above one
  whose range is actually higher at both ends. It is a defensible choice, but it means the order isn't
  always the one you'd get from the low end or the high end alone.
- **A few day-ranges start at zero** ("about 2 days (0–7)"), which can read strangely; it means "up
  to 7 days, possibly none," not that the low end is meaningful on its own.

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
  A fix is planned that borrows the state's outage pattern instead. *(Addressed in v0.2.0: every
  county's outage rate now blends its own record with a credibility-weighted average of nearby
  counties, so a thin local record borrows from its region automatically instead of falling back to a
  separate, simpler estimate.)*
- **A few coastal counties may read too high.** We are re-checking how earthquake risk and outage
  history combine in some Pacific coast counties. Coos Bay, Oregon is one example, where water and
  power targets come out higher than expected.
- **A hazard card can still carry a related hazard's advice** in the packet (a cold-wave card
  showing avalanche lines, snow advice in a hot county). The site's "What helps" lists are fixed in
  v0.1.1; the packet's shared guidance blocks are split in v0.2.0. *(Fixed in v0.2.0: each hazard's
  guidance is now its own short block, so a cold-wave card no longer carries avalanche lines, and a
  hot county's card no longer carries snow advice.)*
- **Heat wave and cold wave can read as less serious than they are** for a household with a baby, an
  older adult, or no cooling or heating. A fix is planned to weigh who is in the household, not just
  typical dollar cost.
- **A well-pump generator arrives late.** The pump-rated generator, its interlock and fuel cans
  are separate lines, so on a small budget the generator can land in year two or three while the
  animals rely on 14 days of stored water. A "requires" link that keeps a bundle together is planned
  for v0.2.0.
- **Life-safety items can push other things later.** Because the extinguisher and bleeding kit now
  come first, Philadelphia's carbon monoxide alarms moved from month 2 to month 6. A per-item value
  for readiness items (v0.2.0) will settle the order. *(Addressed in v0.2.0: every item now carries
  its own readiness value, so cheap, life-critical items move earlier and low-value accessories move
  later, rather than one blanket rule for every safety item.)*
- **The printed packet's monthly spending can look higher than it is.** In a month where a saved-up
  purchase completes and a regular deposit also lands, both currently show in that month's total. A
  fix is proposed.
- **The packet needs print polish.** Its source list is not laid out in columns yet, and a section
  can still split across a page break. *(Addressed in v0.2.0: sources now print in two columns, and a
  heading is never left alone at the bottom of a page.)*
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
