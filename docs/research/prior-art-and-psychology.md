# Prior Art and Behavioral Science for a Household Preparedness Planner

Research date: 2026-09-25. Scope: prior art (Part A) and behavioral science (Part B) for an open-source, privacy-preserving, browser-based planner. The planner computes a location- and household-specific risk profile, turns it into a phased, budgeted plan (72 hours, then get-home bag, 2 weeks, 1 month, 3+ months where the risk justifies it), and hands the user a coaching "packet".

## How this was researched, and the conventions used

- **Search budget:** all 20 WebSearch calls were used. I also fetched about 60 primary pages directly. Journal abstracts came from the PubMed E-utilities, Crossref and OpenAlex APIs. GitHub was surveyed with `gh search` (about 25 queries plus topic searches), and I read the source code of Tokyo's open-source stockpile calculator.
- **Blocked sites:** some primary sites refused automated access: the full Sphere handbook, NZ Get Ready, Australian Red Cross, Reddit, and PubMed's HTML pages. In those cases I used a mirror, an API, or search-result snippets, and I say which one each time.
- **UNVERIFIED** means I could not confirm the claim against a primary source in this session. "UNVERIFIED (recalled)" marks a standard figure from the literature that I did not reopen. Check both kinds before they go into user-facing copy.
- **Quotations:** word-for-word quotes come only from US federal sources (FEMA, Ready.gov and CDC, which are public domain) and from the European Commission (reuse authorized). Everything else, including other organizations' framing, is paraphrased for copyright reasons, with one short quote as the only exception. URLs are given throughout so the team can read the originals.
- **"Observed"** means I saw the behavior directly on 2026-09-25, for example a redirect, a parked domain or a 404.

---

## Executive summary

**The gap:** some tools answer "what is my hazard?" (the NRI, First Street, ClimateCheck, MyHazards). Others answer "what goes in a kit?" (Ready.gov, the Red Cross, the state 2 Weeks Ready campaigns, The Prepared). Others answer "what do I own?" (prepper inventory apps). Only one tool I found, Tokyo's Bichiku Navi, computes household-specific quantities, and it has no hazard model. None of these tools links the three questions. None turns local hazard probabilities into "how many days might this household be on its own without water, power or road access". None sizes supplies to that duration for the actual people in the house. None orders purchases by risk reduction per dollar within a budget. None grows the plan in phases with a clear point where it is enough, and none does all of this privately, offline, with citations. The field is also getting more fragile. Several risk-data sources were retired, moved or taken down between 2025 and 2026 (details under Surprises), which argues for bundled, versioned, offline data.

**The five strongest design implications** (full list in B.9):
1. Pair every risk with a specific, costed action and state how much it helps. The evidence says fear helps only when people also believe they can act (high efficacy); in the 2024 FEMA survey only 32% of respondents have high preparedness efficacy.
2. Show risk as natural frequencies for "households like yours" over 10 years, framed as consequences (days without power or water) rather than hazard names. Put the likely and the dramatic side by side.
3. Build the plan as small tiers with credit for things the household already has. Always show one next action. Set a budget cap and a "sufficient for your risk" line at every tier. This counters single-action bias, avoids choice overload and prevents "doomshopping".
4. Treat neighbors and social ties as a scored prep item. Social capital predicts survival, and 71% of people expect help from friends and family, but only 18% had received any information about helping neighbors.
5. Turn the plan into pre-commitments and rehearsals: if-then triggers, 10-minute drills, and rotation tied to existing routines.

---

# PART A: PRIOR ART

## A.1 Rubric

Every tool is scored on eight dimensions. **Y** = yes, **P** = partial, **N** = no.

| Code | Dimension | Meaning |
|---|---|---|
| Loc | Location-aware | Uses the user's address or area to change the risk or the recommendations |
| HH | Household-aware | Changes quantities or actions based on who lives there (ages, medical needs, pets, housing) |
| Quant | Quantitative | Expresses risk or needs as numbers (probabilities, days, liters, kcal), not just lists |
| Budget | Budgeted | Helps choose what to buy within a spending limit |
| Phase | Phased | Grows the plan in stages over time |
| Priv | Private/offline | Works without an account or server, and data stays on the device |
| Evid | Evidence-cited | Cites the basis for its recommendations |
| Maint | Maintenance | Supports rotation, expiry dates and review cycles |

## A.2 Government and NGO guidance

**Ready.gov "Build a Kit"** (https://www.ready.gov/kit)
- *Does well:* the canonical US list. Its water rule, verbatim: "one gallon per person per day for several days, for drinking and sanitation". It points out that "About half of all Americans take a prescription medicine every day." On maintenance: "Replace expired items as needed. Re-think your needs every year and update your kit." It suggests separate kits for home, work and car.
- *Lacks:* on this page the duration is only "several days". Nothing is tailored to location or computed for a specific household, and there are no costs.

**Ready.gov "Make a Plan"** (https://www.ready.gov/plan)
- *Does well:* four steps: meet with your family, assess household needs, document the plan, practice it. Five core questions, verbatim: "How will I receive emergency alerts and warnings? What is my shelter plan? What is my evacuation route? What is my family/household communication plan? Do I need to update my emergency preparedness kit?" It covers ages, medical needs, disabilities, languages, cultural considerations, pets and school-age children.
- *Lacks:* the outputs are fillable PDFs (a family plan and a communication card), with no interactive logic. It tells users to consider the disasters that could affect their area but offers no lookup.

**Ready.gov "Low and No Cost Preparedness"** (https://www.ready.gov/low-and-no-cost)
- Verbatim: "Disasters are costly but preparing for them doesn't have to be." Also: "Build your emergency supply kit over time. Start with items you may already have in your home..." and pick up "an extra item each time that you use regularly".
- Free actions it lists: alerts, a communication plan, fire and earthquake drills, free smoke and CO alarms from fire departments, and free CPR courses.
- This is FEMA's closest thing to budget guidance, but it is static advice, not a budget planner.

**FEMA App** (https://www.fema.gov/about/news-multimedia/mobile-products)
- NWS alerts "for up to five locations nationwide", a shelter finder, kit and plan content, disaster recovery center locations and FEMA assistance eligibility. Screen-reader accessible.
- *Lacks:* no risk profile and no household plan builder. The page does not document offline behavior or data practices beyond links to policies.

**American Red Cross**
- Pages: https://www.redcross.org/get-help/how-to-prepare-for-emergencies/survival-kit-supplies.html, /make-a-plan.html and /mobile-apps.html
- *Does well:* water at one gallon per person per day. Its kit has **two tiers: a 3-day supply for evacuation and a 2-week supply for home**, plus a 7-day supply of medications and copies of documents. The plan page has 3 steps, advises planning for the emergencies most likely where you live, work and play, and offers free templates. It covers out-of-area contacts, pets and family members who are sometimes away. The Emergency app has NWS alerts, live weather maps, open shelters and short guides, in English and Spanish.
- *Lacks:* working out what is "most likely" is left to the user. There are no numbers and no personalization.

**Washington "2 Weeks Ready" and "Prepare in a Year"** (https://mil.wa.gov/preparedness)
- *Does well:* separate tracks for families, neighborhoods, pets and access/functional needs. A month-by-month "Prepare in a Year" calendar builds preparedness gradually. Spanish version available.
- *Lacks:* no interactive or location-aware tool, and no budget tool, in the visible content.

**Oregon "Be 2 Weeks Ready"** (https://www.oregon.gov/oem/hazardsprep/Pages/2-Weeks-Ready.aspx)
- *Does well:* 2 Weeks Ready means having a plan and supplies for everyone in the household to survive at least two weeks after a disaster. The toolkit has eight units done "one unit at a time, at your own pace". Available in six languages plus ASL, with 40 ASL videos and a pocket planner.
- *Where the "2 weeks" comes from:* the **Oregon Resilience Plan** (OSSPAC, February 2013), https://www.oregon.gov/oem/documents/oregon_resilience_plan_executive_summary.pdf. Its estimates of time to restore services under then-current conditions:
  - Electricity: 1–3 months in the Willamette Valley, 3–6 months on the coast
  - Drinking water and sewer: 1 month to 1 year in the Valley, 1–3 years on the coast
  - Healthcare facilities: 18 months in the Valley, 3 years on the coast
- The plan recommended changing individual preparedness messaging from the old 72-hour standard to at least two weeks "and possibly more".
- It also recommended a **two-tier community rating**: the hours or days a resident can expect to wait for major relief, and the days or months until the community reaches 90% restoration of roads and municipal services. And it noted that business continuity planning typically treats two weeks as the longest disruption a business can survive.
- *Why this matters:* it is a published government precedent for the exact quantitative output this project wants, "how long will I be on my own", expressed per service.

**California MyHazards (Cal OES)** (https://myhazards.caloes.ca.gov/)
- Observed: the address now redirects to an ArcGIS Instant "lookup" app.
- *Does well:* enter an address, city, ZIP or map point and see earthquake, flood, fire and tsunami hazards within a search radius, plus recommended actions. Source: the Cal OES "How does MyHazards work?" explainer and county pages, via search.
- *Lacks:* it shows whether you are in a hazard zone, not how likely an event is. It covers four hazards, with no household tailoring, budget or phases.

**Listos California** (https://www.listoscalifornia.org/)
- *Does well:* five steps: get alerts, make a plan, pack a **Go Bag**, build a **Stay Box**, help friends and neighbors. It adds a neighborhood block-party toolkit, peer-to-peer outreach, a ZIP-based alert sign-up that also explains what alerts mean, and online and text courses.
- Its framing is explicitly about equity: preparedness should not be limited to people who can access, understand and afford it (paraphrased).
- *Why this matters:* Go Bag vs Stay Box maps directly onto our phases, and "help neighbors" is a step in its own right. *Lacks:* numbers and personalization.

**Australian Red Cross RediPlan and the "Get Prepared" app**
- Page: https://www.redcross.org.au/emergencies/prepare/get-prepared-app/. The site blocked automated fetches; details come from search snippets and Richardson, Kelly & Mackay (2023), AJEM, summarized at https://knowledge.aidr.org.au/resources/ajem-july-2023-australian-red-cross-psychosocial-approach-to-disaster-preparedness/
- *Does well:* the app was co-created with NRMA Insurance and fills in a RediPlan digitally: key contacts, meeting places, animal plans, key documents, special items, medical information and **stress-management plans**.
- It is organized under four pillars: get in the know, get connected, get organised, get packing. It explicitly covers psychological preparedness, using Hobfoll et al.'s five elements (safety, calm, efficacy, connectedness, hope) and the AIM technique (anticipate, identify, manage). The tagline is about protecting "what matters most".
- Evaluation cited: the 2021 Perth Hills bushfire, and a 2019 Red Cross survey of 165 survivors in which feeling prepared was linked to lower stress. Both are self-reported. UNVERIFIED against the primary reports.
- *Lacks:* no quantitative or location-based risk.

**New Zealand "Get Ready"** (https://getready.govt.nz/)
- The site blocked automated fetches; details are from getready.govt.nz search snippets.
- Water: at least 3 L per person per day for at least 3 days (9 L per person). Heat and exertion can double this, and children, nursing mothers and sick people need more. Some areas may be without water for longer, and local Civil Defence groups can advise.
- Whether an online plan tool exists: UNVERIFIED.

**Tokyo: Tokyo Bousai, Tokyo Kurashi Bousai, and Tokyo Bichiku Navi**
- Links: https://www.bousai.metro.tokyo.lg.jp/1028036/index.html, https://www.bichiku.metro.tokyo.lg.jp/, and the code at https://github.com/Tokyo-Metro-Gov/bichiku-navi
- *Does well:*
  - **The books:** two official disaster-preparedness books, Tokyo Bousai and Tokyo Kurashi Bousai (a daily-life edition). Their illustrations and designs are free to reuse for outreach. The site is multilingual. The claim that the books were distributed to every household in 2015 is UNVERIFIED (recalled).
  - **Everyday stockpiling (日常備蓄):** a metro-wide project to keep extra of what you normally use and replace it as you eat it (often called "rolling stock"). Rotation happens through ordinary consumption.
  - **Tokyo Bichiku Navi:** three questions (the sex and age band of each household member, housing type, pets) produce a personalized list of items and quantities that can be shared on LINE, with shopping links. The source code is **MIT-licensed** (Nuxt/Vue, 25 stars, last push 2026-03-09).
- *What the code shows:*
  - Quantities are set per person per day by age/sex segment. For example, water is 3 L for adults and teens and 2.4 L for infants and primary-school children, and retort rice is 3 meals a day.
  - Quantities are multiplied by **7 days if the household answers that it lives on an upper floor, or 3 days otherwise**. This is the code's `isUpstairs` flag; I did not check the exact wording of the question.
  - A readiness score from 0 to 100 maps to 0–7 days of stock. Its encouraging messages first aim at 3 days, then at a week and beyond.
- *Why this matters:* it is the only official, household-aware, open-source stockpile calculator I found. It uses **housing type to set duration**: lifts and water pumps stop in high-rises. And its score gives progress feedback.
- *Lacks:* no hazard-probability model. It includes retailer shopping links (Amazon, Rakuten, Yahoo) and is specific to Japan.

**UK "Prepare"** (https://prepare.campaign.gov.uk/, published under Open Government Licence v3.0)
- *Does well:*
  - Sections on getting prepared, hazards (with links to official flood-risk checks for England, Wales, Scotland and Northern Ireland), getting involved in your community, coping with trauma, and advice for disabled people and carers.
  - It promotes utilities' Priority Services Registers and offers a printable household plan. It advises storing ICE/Medical ID on phones, with a caveat that anyone holding the phone can read it.
  - Supplies are for an emergency lasting "a few days". On water, it says there is no standard figure: the WHO survival minimum of 2.5–3 L per person per day, or about 10 L per person per day for comfort including basic cooking and hygiene.
  - It suggests building the kit up over time rather than buying it all at once, and gives concrete neighbor steps: swap contacts, start a street messaging group, ask about neighbors' support needs, and plan to check in during power cuts (all paraphrased).
- *Lacks:* no interactive tool beyond links and no household quantities. Launch date: UNVERIFIED.

**Canada "Get Prepared"** (https://www.canada.ca/en/services/policing/emergencies/preparedness/get-prepared.html)
- *Does well:* three steps: know the risks, make a plan, get a kit. The kit should keep you self-sufficient for at least 72 hours, with about 4 L of water per person per day, bought a few items at a time while running errands (paraphrased).
- An **online plan builder** takes about 20 minutes and produces a printable plan. It asks about exits, meeting places, who picks up the children, contacts, health and insurance, pets, regional risks, and where the utility shutoffs are. The page says nothing about how the data is handled.
- It states that one in three Canadian adults has lived through a major weather-related emergency or disaster.
- *Lacks:* location-specific numbers and budgeting.

**EU Preparedness Union Strategy** (26 March 2025, https://ec.europa.eu/commission/presscorner/detail/en/ip_25_856)
- Verbatim: it encourages the public to adopt practical measures "such as maintaining essential supplies for a minimum of 72 hours in emergencies". The strategy has 30 key actions, an EU Preparedness Day, school curricula, and an all-hazards, whole-of-society approach.
- National guidance goes further:
  - Sweden's civil defence agency (now **MCF**; msb.se redirects to mcf.se) frames home preparedness as "prepping for at least a week" (https://www.mcf.se/en/).
  - Germany's official stockpile calculator (BMEL/BLE) covers 1–28 days for 1–10 people, assuming 2,200 kcal per person per day and about 2 L of water per person per day (https://www.ernaehrungsvorsorge.de/private-vorsorge/notvorrat/vorratskalkulator/). BBK's 10-day recommendation: UNVERIFIED (recalled).

**Church of Jesus Christ of Latter-day Saints: home storage** (https://www.churchofjesuschrist.org/study/manual/gospel-topics/food-storage)
- *Does well:* a long-running, widely adopted **phased** model. Start small, then build up to a month, then three months. Add drinking water, a financial reserve, and long-term staples later.
- It explicitly warns against going to extremes and against going into debt, and says to build gradually so the cost is spread out (paraphrased).
- *Lacks:* it centers on food and is not based on risk.

### Table A.2: Government and NGO guidance

| Source | Loc | HH | Quant | Budget | Phase | Priv | Evid | Maint | Baseline duration |
|---|---|---|---|---|---|---|---|---|---|
| Ready.gov kit/plan | N | P | P (1 gal/day) | P (low/no-cost page) | N | Y (static PDFs) | N | P | "several days" |
| FEMA App | P (alerts, 5 places) | N | N | N | N | ? | N | N | — |
| American Red Cross | N | P | P | N | P (3 d evac / 2 wk home) | Y (templates) | N | P | 3 days + 2 weeks |
| WA 2 Weeks Ready | N | P | N | N | Y (Prepare in a Year) | Y | N | N | 2 weeks |
| OR 2 Weeks Ready + ORP | P (ORP zones) | P | P (ORP restoration table) | N | Y (8 units) | Y | Y (ORP) | N | 2 weeks+ |
| CA MyHazards | Y (zones) | N | N | N | N | ? | P | N | — |
| Listos California | P (ZIP alerts) | P | N | P (equity) | P (Go Bag / Stay Box) | Y | N | N | — |
| AU Red Cross RediPlan/app | N | Y (contacts, animals, meds, stress) | N | N | N | ? (app) | P | P | — |
| NZ Get Ready | N | P | P (3 L/day) | N | N | Y | N | P | ≥3 days |
| Tokyo Bichiku Navi | N | **Y** | **Y** (per person per day) | N (shop links) | P (3 → 7 days) | P (web) | N | P (score) | 3 or 7 days |
| UK Prepare | P (flood check links) | P | P (2.5–3 L / 10 L) | P (build over time) | N | Y | P (cites WHO) | P | "a few days" |
| Canada Get Prepared | P (regional risks) | P | P (4 L/day) | P | N | ? (online form) | N | N | ≥72 h |
| EU strategy / SE / DE | N / N / N | N / N / Y | N / N / Y | N | N | Y | N | N | 72 h / ≥1 week / 1–28 days |
| LDS home storage | N | P | P | **Y** (no debt) | **Y** | Y | N | Y (rotate) | 1 → 3 months+ |

## A.3 Location-risk tools

**FEMA National Risk Index (NRI)**
- *What it is:* 18 natural hazards, at county and census-tract level. Expected Annual Loss (EAL, in dollars) is scaled by a community risk factor built from social vulnerability and community resilience. Scores are **national percentiles from 0 to 100**, so they are relative, not absolute. FEMA's December 2025 documentation (FAQ, methodology and version pages at fema.gov/sites/default/files/documents/fema_national-risk-index_*.pdf) says the NRI supports planning and should not be used alone for local risk analysis; use local data where it exists (paraphrased from search results).
- *Status (observed):* `hazards.fema.gov/nri` now 301-redirects to FEMA's **Resilience Analysis and Planning Tool (RAPT)** page. That page says, verbatim: "The National Risk Index v1.20 data update is now available on RAPT." The page was last updated 2026-08-18. The standalone NRI map appears to be retired.
- The NRI **Future Risk Index** (with climate projections) was removed from public access in February 2025. Community replicas exist, for example https://github.com/fulton-ring/nri-future-risk (Unlicense, 39 stars), and Harvard EELP hosts the technical document.
- *Does well:* free and national. Per-hazard annualized frequency and EAL can be converted into natural frequencies.
- *Lacks:* percentile framing, dollars rather than people, tract or county resolution, no household actions, and increasingly fragile hosting.

**First Street ("Risk Factor")**
- Property-level scores for flood, wildfire, wind, heat and air quality, with forward projections. The exact scales of the current product are UNVERIFIED.
- Observed: `riskfactor.com` 302-redirects to `firststreet.org`, which sits behind a terms-acceptance gate. The free consumer site appears to have been retired or folded into the main site (UNVERIFIED which).
- **Zillow removed climate risk scores from listings effective 14 November 2025.** Agents and the California Regional MLS complained the scores looked arbitrary and hurt sales, and a couple sued over "stigma". Zillow still links to First Street data less prominently; Redfin and Realtor.com continue to show risk data. Sources:
  - Grist: https://grist.org/housing/zillow-deletes-climate-risk-data-from-listings-after-complaints-it-harms-sales/
  - CNN: https://www.cnn.com/2025/12/02/climate/zillow-climate-data-extreme-weather-first-street-redfin
  - TechCrunch: https://techcrunch.com/2025/12/01/zillow-drops-climate-risk-scores-after-agents-complained-of-lost-sales
- *Lesson:* precise-looking property scores are contested and commercially fragile, and they carry stigma. Show ranges, name the source, and always attach "what you can do".

**ClimateCheck** (https://climatecheck.com/)
- *Does well:* property-level ratings for heat, drought, wildfire, wind, storm surge, sea-level rise, precipitation/flooding, hurricanes and earthquakes, covering historic, current and future scenarios. The basic assessment is free.
- *Lacks:* it is aimed at real-estate businesses, and the homepage has no preparedness actions.

**Augurisk** (https://www.augurisk.com/)
- *Does well:* free risk reports for a property or neighborhood, covering disasters plus crime ("societal" risk), on a scale from Very Low to Very High. It claims 12 proprietary models and about 200K users a year.
- *Lacks:* no action guidance, and the homepage cites no data sources. Putting crime next to hazards also feeds the looting myth (see B.5).

**NOAA and federal climate data**
- The NOAA **Billion-Dollar Weather and Climate Disasters** database stopped updating on 7 May 2025. **Climate Central relaunched it on 22 October 2025** with the full archive back to 1980, the same methods and the same lead scientist (https://www.climatecentral.org/climate-matters/billion-dollar-disasters-oct-2025; CNN coverage: https://www.cnn.com/2025/10/22/climate/extreme-weather-disaster-trump-noaa).
- NWS alerts remain the canonical real-time source; the FEMA and Red Cross apps both use them.
- Other NOAA tools worth linking to, not re-checked this session: the Sea Level Rise Viewer (coast.noaa.gov/slr), the Precipitation Frequency Data Server (hdsc.nws.noaa.gov/pfds), and the Storm Events Database.

**Hazus (FEMA)** (https://www.fema.gov/flood-maps/products-tools/hazus)
- Version 7.2. It estimates building damage, economic losses, **displaced households**, casualties and debris for earthquake, flood and hurricane wind. It is free but depends on ArcGIS Pro.
- Best used offline as a source of consequence parameters such as displacement and restoration times, not run in the browser.

**OPB Aftershock / Hazard Ready** (https://github.com/hazard-ready/disaster-preparedness, GPL-3.0)
- An address-based interactive on Cascadia risk from Oregon Public Broadcasting (GeoDjango/PostGIS). It gave tailored short guidance for shaking, liquefaction, landslide and tsunami at a specific address.
- The repository was **archived in September 2026**, and `opb.org/aftershock` now returns 404 (observed). It was the most "coach-like" location tool I found. It was also server-hosted, and a server-hosted tool like this is costly to keep running.

**Ready.gov hazard lookup:** I found no address-based hazard lookup on Ready.gov itself. It offers per-hazard pages and links to national tools. Not exhaustively verified.

### Table A.3: Location-risk tools

| Tool | Resolution | Output | Actions? | Household? | Access | Status (2026-09) |
|---|---|---|---|---|---|---|
| FEMA NRI | county / tract | percentile scores, EAL ($), frequencies | N | N | free, public-domain data | standalone site redirects to RAPT (v1.20) |
| NRI Future Risk | county | climate-adjusted scores | N | N | removed Feb 2025 | community replica exists |
| First Street | property | 1–10 peril scores, projections | minimal | N | consumer site redirects | removed from Zillow Nov 2025 |
| ClimateCheck | property | ratings for 9+ hazards | N | N | free basic, paid B2B | active |
| Augurisk | property / area | hazard + crime ratings | N | N | free | active |
| Cal MyHazards | address radius | in or out of hazard zones + tips | P | N | free | active (ArcGIS app) |
| NOAA $B disasters | national / state | event losses | N | N | free | moved to Climate Central |
| Hazus 7.2 | scenario / region | damage, displacement, restoration | N | N | free, needs ArcGIS Pro | active |
| OPB Aftershock | address | Cascadia factors + tips | Y | N | free | gone (404), repo archived |

## A.4 Commercial preparedness

**The Prepared** (https://theprepared.com)
- *Does well:*
  - Six steps in order: personal finance and health first; a two-week home supply; bug-out bags; away-from-home gear (get-home bag, everyday carry); skills; community.
  - An explicit 80/20 approach and a clear "likely events first" ethos. The one short quote this report uses from a copyrighted source: "prepare for the 80% of likely scenarios, not the unlikely ones" (The Prepared, emergency preparedness checklist).
  - Its budgeting article names **"doomshopping"**: doomscrolling leading to fear-driven buying and then guilt. The fix it recommends is a fixed monthly budget and a month-by-month purchase plan with deliberate waiting periods. The article mentions a "Kit Builder" tool; its availability is UNVERIFIED.
  - Its myths article says looting and crime after ordinary disasters are far lower than people expect, that staying put should be the default, and that groups do better than lone wolves.
  - Medical and firearms framing: see B.8.
- *Lacks:* no personalization or location model. It is a content site.

**Judy**
- A design-led kit brand that leaned on optimism, not fear, in its marketing (Forbes, 2022: https://www.forbes.com/sites/karineldor/2022/12/30/how-pr-founder-simon-huck-leans-into-optimism-with-emergency-preparedness-kit-judy/).
- Observed: `judy.co` now serves a parked "for sale" page on the Afternic marketplace (its nameservers are ns1/ns2.afternic.com). I found no news of a closure, so its status is UNVERIFIED.

**Preppi** (preppi.co) and **Uncharted Supply Co.** (unchartedsupplyco.com)
- Premade survival kits and gear. Preppi's site description mentions a lifetime warranty and sponsoring KPCC's "The Big One" podcast.
- *Lacks:* both are product-first, sized per person at best, with no logic based on risk or location.

**Jase** (formerly Jase Medical; `jasemedical.com` now 301-redirects to `jase.com`)
- The "Jase Case" costs $289.95: 5 antibiotics plus 5 symptom-relief drugs for 50+ infections. There are also condition-specific kits and "Jase Daily", an extended supply of chronic medications.
- A telehealth physician reviews each order. The site says the medications are for situations where you cannot reach medical help, and that it does not condone stockpiling (paraphrased). Reviewers report a limit of one antibiotic pack per person per year (UNVERIFIED on the primary site).
- It also sells compounded ivermectin, which requires acknowledging it is not FDA-approved.

**Duration Health**
- Prescription-only "off-grid urgent care" kits prescribed online, with 20 kits drawn from 70+ medications. It excludes controlled substances and includes an emergency medical reference (from search snippets of durationhealth.com and MTEC).
- The domain did not resolve from our environment, so the snippets could not be checked against the live site (UNVERIFIED).

**Mountain House and ReadyWise** (the calorie math, observed on product pages)

| Product | kcal/day as sold | Servings | Price | Cost per 2,000 kcal (derived) |
|---|---|---|---|---|
| Mountain House "30-Day Emergency Food Supply" | ~1,732 | 180 (90 pouches) | $939.99 (sale) | ≈ $36 |
| ReadyWise "720 servings" | 158,640 kcal total, avg 220 kcal/serving | 720 | $1,769.94 | ≈ $22 |

- ReadyWise itself says the 720-serving kit is "88 days at 1,800 kcal/day".
- **Labels like "30-day" and "servings" understate what people need** compared with planning figures of 2,000–2,200 kcal per person per day. Germany's calculator uses 2,200; Sphere's 2,100 is UNVERIFIED (recalled).
- ReadyWise's calculator URL returned 404, and neither homepage linked to a calculator.
- *Design implication:* plan in kcal and liters, not servings. Show cost per 2,000 kcal. Rotate grocery staples first.

**Prepper inventory apps**
- Apps: PrepperPro, BeaglePrep, PPantry, Pantry Check, Prep & Pantry, The Ultimate Prep System, Prepper Nerd, Bug Out Bag App, The Prepper App.
- *Does well:* inventory tracking, expiry alerts and a days-of-supply score (BeaglePrep's "Prep Score"; PPantry's "days your household can last"). Several are offline-first.
- *Lacks:* a July 2026 comparison written by a competitor (https://beagleprep.com/blog/best-prepper-inventory-apps/, biased) says none of them assesses location-based hazards or generates plans.

**r/preppers**
- Posts carry flair for **"Prepping for Tuesday"** (everyday disruptions) or **"Prepping for Doomsday"** (reported by Grist: https://grist.org/culture/prepper-disaster-food-pantry-staples-advice-treat/). The community's own norm separates the likely from the dramatic.
- Wiki contents: UNVERIFIED (Reddit blocked automated access).

### Table A.4: Commercial preparedness

| Product | Loc | HH | Quant | Budget | Phase | Priv | Evid | Maint |
|---|---|---|---|---|---|---|---|---|
| The Prepared | N | N | P | P (monthly budget) | Y (6 steps) | Y (content) | P (expert contributors) | P |
| Judy / Preppi / Uncharted | N | P (kit sizes) | N | N | N | n/a | N | N |
| Jase / Duration Health | N | Y (medical intake) | N | N | N | N (telehealth accounts) | P | P (expiry) |
| Mountain House / ReadyWise | N | P | P (kcal) | N | P (kit sizes) | n/a | N | N (25–30 yr shelf life) |
| Prepper inventory apps | N | P (household size) | Y (days of supply) | N | N | P–Y (some offline) | N | **Y** |

## A.5 Open source on GitHub (surveyed 2026-09-25 with `gh search repos` and topic searches)

Queries included: emergency/disaster preparedness, prepper, emergency kit, bug out bag, go bag, 72 hour kit, food storage calculator, survival checklist, preparedness app/planner, household emergency plan, national risk index, hazard risk assessment, and the topics prepping, preparedness, emergency-preparedness, disaster-preparedness, survival, prepper and food-storage.

| Repo | Stars | License | Last push | What it does | Rubric notes |
|---|---|---|---|---|---|
| Tokyo-Metro-Gov/bichiku-navi | 25 | MIT | 2026-03-09 | Official Tokyo household stockpile calculator (Nuxt/Vue); per-person-per-day quantities by age/sex; 3 or 7 days depending on floor | HH Y, Quant Y, Phase P; no hazard model; best reusable data found |
| kakelakel/homeprep | 4 | MIT | 2026-09-15 | Home Assistant integration: inventory, containers, fixed "assets" (shutoffs, alarms), scenario plans, recurring checks, readiness score; explicit data-ownership principle | Maint Y, Priv Y; no risk model |
| SysAdminDoc/project-nomad-desktop | 13 | MIT | 2026-09-14 | Local-first desktop "field desk": readiness score (water, food, medical, comms, security, power, planning), burn rates, offline maps and library, optional local LLM | Quant P, Priv Y; heavy, ops-oriented |
| BFUR64/handa360 | 3 | MIT | 2026-08-16 | Hazard + locality + special needs produce one checklist with local hotlines (Philippines hackathon); JSON-driven data | Loc P, HH P; no numbers |
| Fargolnz/Tik | 4 | none | 2026-07-27 | Account-based household profile produces personalized checklists and before/during/after plans; focused on war | HH Y; server storage |
| hazard-ready/disaster-preparedness | 12 | GPL-3.0 | archived 2026-09 | Engine behind OPB Aftershock; address-based Cascadia risk and guidance snippets | Loc Y; dead deployment |
| fulton-ring/nri-future-risk | 39 | Unlicense | 2025-04-21 | Replica of FEMA's NRI Future Risk tool after its removal; bundles the master datasheet | Loc P (county); a data-rescue pattern |
| Nick-Heger/disaster-risk-calculator | 0 | none | 2026-02-14 | ZIP to county to NRI v1.20 via OpenFEMA, showing "approximate annual probability" | Loc P, Quant P |
| hndfaw/app-2026-w30-fallback | 0 | MIT | 2026-08-06 | Stress-tests a household plan against losing phone, power, internet or a caregiver; flags single points of failure; prints a fallback card; localStorage only | Priv Y; a clever idea to borrow |
| Random-Vibez/randomvibez-hearthplan | 0 | none | 2026-09-14 | Static browser-local planner: no fetch, no analytics, validated JSON import/export, wallet card | Priv Y (strong pattern) |
| edricked/homesafe | 0 | none | 2026-08-12 | Offline-first PWA with a one-action-at-a-time emergency flow | Priv Y |
| Mr-Salticidae/typhoon-eye | 2 | MIT | 2026-09-25 | Typhoon live info plus **tiered, checkable plans**; regional scenarios chosen by hand with **no GPS, IP location or upload** | Phase P, Priv Y |
| BEKO2210/Survival_kit | 10 | MIT (badge only) | 2026-04-12 | German offline PWA: shelf-life tables, checklists, on-device PDF export | Priv Y |
| danielmiessler/BugOutBag | 69 | none | 2022-04-30 | Two-week list for the "cautious but not nutty" | — |
| ligi/SurvivalManual | 1,298 | GPL-3.0 | 2025-09-03 | Offline survival manual app based on the US Army field manual | Offline knowledge |
| kylecorry31/Trail-Sense | 2,892 | MIT | 2026-09-25 | Offline, sensor-based wilderness toolkit | Offline and privacy precedent |
| Ganso/refugiOS | 625 | AGPL-3.0 | 2026-09-25 | Portable offline OS for emergencies | Offline knowledge |
| danieldurrans/Digital-Estate-Emergency-Kit | 62 | CC-BY-SA-4.0 | 2026-09-01 | Template for gathering your digital life (accounts, documents) | Useful for a "documents" module |

**Observations**
- **The open-source field is thin.** Most planners have fewer than 20 stars. The popular projects are offline knowledge libraries, not planners. **No repository combines quantitative location risk, household composition, budget and phased growth.**
- Newer small projects commonly store data only in the browser, make no network calls and have no accounts. That is now the minimum users will expect, and it is achievable.
- The best asset to reuse is Tokyo's MIT-licensed data model: per-person-per-day quantities by age/sex segment, plus a housing-type multiplier on duration.
- Two failure modes show up. hazard-ready shows **server-hosted tools die**. nri-future-risk shows **government data disappears**. Both argue for a static site with a versioned data bundle.

## A.6 Professional frameworks worth borrowing

1. **THIRA/SPR (FEMA)**
   - Source: https://www.fema.gov/emergency-managers/national-preparedness/goal/risk-capability-assessment
   - THIRA asks three questions: what threats and hazards can affect us; what impacts would they have; based on those impacts, what capabilities should we have. The answers set capability targets. The SPR then self-assesses current capability against those targets, including gaps, across five areas: planning, organization, equipment, training and exercises (known as POETE).
   - *Household version:* hazards, then impacts (days without water, power or road access; chance of displacement), then capability targets (liters, kcal, heating and cooling, days of medication, communications, sanitation, cash, evacuation), then the gap, then a budgeted plan. POETE translates to plan, people, supplies, skills and drills. FEMA designed THIRA for jurisdictions, not households, so this translation is new.
2. **CPG 101** (https://www.fema.gov/emergency-managers/national-preparedness/plan)
   - A six-step planning process: form a team, understand the situation, set goals and objectives, develop the plan, review and approve it, then implement and maintain it.
   - Version 3.0 dates from 2021. FEMA currently hosts a PDF whose filename is dated 05/21/2025; whether that is a revision is UNVERIFIED.
   - *Borrow:* the maintenance loop, meaning a plan has scheduled review dates and drills.
3. **Capability-based, all-hazards planning**
   - Plan for *functions*, not scenarios. Most hazards lead to the same few consequences: loss of power, water, access or shelter; a need to evacuate; injury; loss of income.
   - *Borrow:* the risk model should compute exposure to each consequence, and the plan should buy capabilities against consequences. This is also how you prepare for the likely event and the dramatic one at the same time.
4. **Hazus and the Oregon Resilience Plan**
   - Use them for consequence parameters: displacement, and restoration times per service.
   - Adopt ORP's **two-tier rating** as a user-facing output: time until outside help plausibly arrives, and time until services are mostly restored.
5. **Sphere minimum standards (2018)**, verified from a mirror of the Sphere 2018 WASH chapter:
   - Water: at least **15 L per person per day** for drinking and domestic hygiene. Survival intake is **2.5–3 L**; hygiene 2–6 L; basic cooking 3–6 L; total basic need **7.5–15 L**.
   - About 7.5 L may be acceptable for a short time in the acute phase of a drought; 50 L may be the minimum in urban middle-income settings.
   - At most 500 m to a water point, less than 30 minutes of queuing, and 1 toilet per 20 people in the medium term.
   - Food at **2,100 kcal** per person per day and **3.5 m²** of covered living space per person: UNVERIFIED this session (recalled; the handbook chapters were blocked).
6. **Hobfoll et al. (2007) five essential elements and AIM** (anticipate, identify, manage): the psychological-preparedness layer used by the Australian Red Cross (see A.2).
7. **WHO Interagency Emergency Health Kit:** medicines for 10,000 people for about 3 months (https://www.who.int/publications/i/item/9789241502115). An example of professional per-capita supply sizing. Use it as a concept only; it is not a consumer list.

### Table A.6: Duration and water baselines across jurisdictions

| Source | Duration baseline | Water (per person per day) | Notes |
|---|---|---|---|
| Ready.gov | "several days" | 1 US gal (≈3.8 L), drinking + sanitation | — |
| American Red Cross | 3 days evacuation / 2 weeks home | 1 gal | 7 days of medications |
| WA / OR | ≥2 weeks | — | ORP restoration: months to years |
| Canada | ≥72 h | ~4 L | — |
| New Zealand | ≥3 days | ≥3 L (9 L for 3 days) | more in heat, for children and nursing mothers |
| UK | "a few days" | 2.5–3 L survival; ~10 L comfort | cites WHO |
| Tokyo | 3 days (7 on upper floors) | 3 L adults, 2.4 L children | per-segment quantities |
| Germany (BMEL calculator) | 1–28 days (user choice) | ~2 L | 2,200 kcal |
| Sweden (MCF) | ≥1 week | — | — |
| EU strategy | ≥72 h | — | — |
| Sphere (humanitarian) | — | 15 L minimum; 2.5–3 L survival; 7.5–15 L basic | for camps and settlements |

*Design note:* official water figures range from 2 to 15 L per person per day because they measure different things (drinking only, drinking plus sanitation, or full domestic use). The planner should show three levels (survival, basic, comfortable), explain the difference, and let the user choose one, defaulting to about 4 L.

## A.7 The unmet need

**Gap statement.** The advice households get today is either generic or not actionable. Government campaigns are generic, reassuring and unquantified: "a few days", "2 weeks". Hazard and commercial tools are quantified but give no actions (property risk scores), or give actions but are generic and commercially slanted (kits, freeze-dried food by the "serving"). Nothing tells a specific household, in plain numbers, which disruptions it is most likely to face, how long it may be on its own for each, exactly what to have for its own members, and in what order to buy it within its budget. Nothing tells it when it has done enough for its risk, or how to keep the plan alive. Location-risk data is increasingly fragile online, and preparedness data is sensitive (what you have, where, and who is vulnerable). So this tool has to be private and offline by design, not as an afterthought.

**Five features that no existing tool combines:**
1. **Consequence-based, location-specific risk shown as natural frequencies and "days on your own".** Hazard frequencies (NRI) combined with restoration assumptions (Hazus/ORP-style) and housing modifiers (the Tokyo high-rise rule) give output such as "about N of 100 households like yours will lose power for 3+ days in the next 10 years" (template wording; N comes from the model, not from this report). Each figure comes with an uncertainty range and a source.
2. **Household-computed quantities.** Liters, kcal, medication-days, heating and cooling, and sanitation, computed for each person (age/sex band, medical needs, infants, pets, mobility) and tied to those durations. Tokyo's per-segment model is the precedent; the evidence-based duration is the new part.
3. **A budget-constrained optimizer.** Purchases ordered by risk reduction per dollar. Free actions come first (alerts, a plan, neighbors, documents, drills), within a monthly budget. It shows cost per 2,000 kcal and per liter and highlights low-value "fantasy gear".
4. **Phased growth with visible progress and a clear "enough" line.** 72 h, then get-home bag, then 2 weeks, then 1 month, then 3+ months only when the risk profile and resources justify it. Each tier is small, credits what the household already owns, and ends with a maintenance mode (rotation cues, review dates, drills) rather than more buying.
5. **Private by construction, with evidence-cited coaching.** A static site with no account, no network calls after load, and a bundled, versioned data snapshot. The packet is exportable and printable. Every recommendation shows who recommends it and why, including community, psychological and financial items as first-class entries, and sensitive topics handled the way public-health bodies handle them.

---

# PART B: BEHAVIORAL SCIENCE AND PSYCHOLOGY

Each section gives the **evidence** first, then the **design implications** (framing, sequencing, defaults and feedback).

## B.1 Protective Action Decision Model (Lindell & Perry)

**Evidence**
- **The model** (Lindell & Perry 2012, *Risk Analysis* 32(4):616–632, doi:10.1111/j.1539-6924.2011.01647.x):
  - Before any decision, people must *receive, attend to and understand* a warning or cue.
  - Three core perceptions then drive the response: **threat perceptions** (how likely, how bad, how soon), **protective-action perceptions** (does the action work, and what does it cost), and **stakeholder perceptions** (who is trustworthy and responsible).
  - Situational facilitators and impediments then shape what people actually do. The model applies to long-term hazard adjustments (preparedness) as well as imminent threats.
- **How people judge actions:**
  - Lindell & Whitney (2000), *Risk Analysis* 20(1):13–25, studied 12 household earthquake adjustments. How people perceived each action correlated with adoption *more strongly than demographics, perceived risk, knowledge or sense of responsibility*.
  - Lindell, Arlikatti & Prater (2009), *Risk Analysis* 29(8):1072–1088, found people judge an action on **hazard-related attributes** (how well it protects people and property, and its other everyday uses) and **resource-related attributes** (cost, time/effort, knowledge/skill, and whether others must cooperate). People largely agreed with each other on these ratings.

**Design implications**
- *Framing:* every action card shows **protects against**, **everyday use** (a flashlight or a camp stove is also useful on a normal day), **cost**, **time**, **skill** and **needs others?** Lead with efficacy.
- *Sequencing:* the default order is efficacy per unit of cost and effort, which puts free and quick actions first.
- *Stakeholders:* name the source of each recommendation (FEMA, CDC, Sphere, Red Cross, local emergency management). The 2024 NHS measured trust in: personal network 85%, nonprofits and community groups 78%, state and local officials 75%, federal government 75%, utilities 75%, news 66%.
- *Impediments:* renters, people on low incomes, people with disabilities and people without cars get alternative actions. Examples: renters insurance instead of retrofits, a support-network plan instead of "drive away", and utility medical-priority registers.
- *Pre-decision stage:* plain language, one screen per idea, and a one-page summary before any detail.

## B.2 FEMA National Household Survey on Disaster Preparedness (latest found: 2024)

The source is the FEMA slide deck *2024 National Household Survey on Disaster Preparedness Findings* (55 slides, some updated April 2025). The survey was fielded 25 April to 7 June 2024 as a web survey in English and Spanish, with 7,525 respondents weighted by age, sex, race, ethnicity, education and income. FEMA's own site does not list the report. I read a mirrored PDF: https://survivingcascadia.com/wp-content/uploads/2025/05/2024-national-household-survey-on-disaster-preparedness-findings.pdf. The underlying data is archived at DataLumos: https://www.datalumos.org/datalumos/project/218642/version/V1/view. I found **no published 2025 results**, only an April 2025 notice renewing the collection with the Office of Management and Budget. That absence is UNVERIFIED. The FEMA community page that hosted the 2023 summary did not resolve from our environment.

**Key figures (2024)**
- **Self-rated stage:** **57% "not prepared"**, up from 49% in 2023. 15% say they have been prepared for less than a year, and 28% for more than a year.
  - The overall split of the "not prepared" group is not reported. From the subgroup tables I estimate: about 12–13% not prepared and not intending to prepare in the next year (18–59: 13%, 60+: 12%, owners 11%, renters 16%); about 19–20% intending to start in the next year; about 25% intending to within six months.
  - These map onto the stages of change: precontemplation ≈13%, contemplation ≈20%, preparation ≈25%, action 15%, maintenance 28%.
  - Trend in "not prepared": 2017 58%, 2018 48%, 2019 41%, 2020 49%, 2021 56%, 2022 56%, 2023 49%, 2024 57%.
- **Actions:** 83% report at least three preparedness actions, up from 57% in 2023. FEMA says the jump "may reflect differences in the survey instrument, the influence of the COVID-19 pandemic on preparedness actions and/or other factors."
- **Supplies:** **69% have assembled supplies**. Of those, 88% say the supplies would last more than 3 days, 61% more than 1 week, **34% more than 2 weeks**, 18% more than 1 month, and 8% more than 3 months. Derived: only about **23% of all respondents** have supplies they believe would last more than two weeks.
- **Self-sufficiency at home:**

| Could live at home for… | >3 days | >1 week | >2 weeks | >1 month | >3 months |
|---|---|---|---|---|---|
| without running water | 61% | 34% | 17% | 9% | 6% |
| without power, cold weather | 59% | 40% | 24% | 15% | 9% |
| without power, warm weather | 63% | 43% | 26% | 16% | 11% |

- **Efficacy:** **32% have high preparedness efficacy**, meaning both confident they can prepare and convinced it would help. 68% have low efficacy. 43% are "very/extremely" confident they can prepare, and 60% believe preparing would help "quite a bit/a great deal", but only 32% say both (checked against the rendered slide). *Confidence in one's own ability is the weaker link.*
- **Risk perception and experience:**
  - 32% think a disaster is "very" or "extremely" likely to affect them; 43% say they or their family have experienced a disaster's impacts.
  - Share saying each natural hazard would have a big local impact, vs share who have experienced it: thunderstorm 51% vs 15%; tornado 49% vs 13%; winter storm 45% vs 15%; flood 42% vs 15%; extreme heat 40% vs 10%; hurricane 32% vs 19%; earthquake 31% vs 8%; wildfire 30% vs 6%.
  - Human-caused events, perceived big impact vs experienced: **power outage 63% vs 22%** (the most commonly experienced event on the list); home fire 56% vs 8%; food or water contamination 54% vs 7%; utility interruption 47% vs 13%; **active shooter 47% vs 5%**; terrorist attack 36% vs 3%; cyberattack 35% vs 3%; financial emergency 28% vs 7%.
- **Worries in the past year:** own or family health 53%, paying bills 42%, a disaster affecting the family 30%, providing food 29%, losing a job 26%. Asked for the *single* top worry: health 27%, bills 24%, disaster only 10%.
- **Barriers:** **cost 26%**, **"don't know what else to do" 25%**, no time 23%, "won't matter" 21%, needing help because of health or disability 16%, not wanting to make the effort 13%.
- **Motivators:** keeping self and family safe 44%, disasters elsewhere 32%, responsibility for family 32%, past experience 22%, someone encouraged me 18%.
- **Information received in the past year:** 78% saw some. Topics: kits 42%, plans 34%, alerts 32%, evacuation 27%, protecting the home 26%, saving money 25%, documents 24%, family contact 23%, drills 21%, insurance 21%, community involvement 19%, **helping neighbors 18%**. Channels: internet 45%, social media 36%, TV 35%, print 23%, texts or alert calls 20%, in person 17%, radio 15%.
- **What would help most:** where to shelter safely 57%, **what basic supplies to set aside 56%**, an evacuation plan for the area 53%.
- **Who people expect help from:** friends and family 71%, state/local/tribal/territorial government 53%, federal government 52%, insurer 48%, nonprofits 32%, faith community 27%.
- **Equity gaps:**
  - Owners vs renters: supplies 73% vs 60%; insurance 87% vs 43%; money set aside 72% vs 43%; a plan 59% vs 50%; prepared for more than a year 33% vs 16%.
  - Socioeconomically disadvantaged vs not: supplies 58% vs 71%; insurance 39% vs 82%; savings 35% vs 70%.
  - Age 60+ vs 18–59: drills 7% vs 27%; prepared for more than a year 42% vs 22%.
- **Moving and remodeling:** of the 17% who moved in the past year, about one in three said hazards or building codes affected the decision.

**Design implications**
- *Stage-matched onboarding:* ask the NHS stage question ("Which best describes you?") and branch.
  - Not intending to prepare: one free, 5-minute action, linked to their existing worries about health and bills.
  - Intending within a year: a 30-minute starter plan.
  - Intending within six months: the full budgeted plan.
  - Already prepared: gap analysis (most are short of two weeks) and maintenance.
- *Attack the top barriers directly:*
  - Cost (26%): free actions first, a budget cap, and cost per 2,000 kcal.
  - "Don't know what else to do" (25%): exactly one next best action, always visible.
  - No time (23%): 5-minute tasks.
  - "Won't matter" (21%): show how effective each action is.
  - Disability (16%): a support-network plan and medical-device power planning.
- *Link to existing worries (the finite pool of worry, B.4):* present preparedness as protection for health and finances ("a stocked pantry is also a buffer if a paycheck is late"), not only as disaster readiness.
- *Aim at the two-week cliff:* among people with supplies, coverage drops from 61% at one week to 34% at two. The "2 weeks" tier should be the first major milestone after 72 h in places where large events are plausible.
- *Use the "likely vs dramatic" data:* **power outage is the most commonly experienced disruption (22%)**, while an active shooter is perceived as high-impact by 47% but experienced by 5%. The default first tier should cover power, water and heat or cold disruption.
- *Make community a core item:* 71% expect help from friends and family, but only 18% have heard how to help neighbors.
- *Serve renters and low-income households explicitly:* renters insurance, small savings goals, and "no-drill" home-safety actions.
- *Opportunity:* a "compare two addresses" mode for people who are moving, since hazards shaped the decision for about a third of recent movers.
- *Don't assume fast outside help:* 52% expect federal support. In large events, ORP-style estimates of waiting time argue for planning to be self-reliant.

## B.3 Extended Parallel Process Model (Witte) and the fear-appeal evidence

**Evidence**
- **Witte & Allen (2000)**, meta-analysis, *Health Education & Behavior* 27(5):591–615: strong fear appeals raise perceived severity and susceptibility and persuade more than weak ones. The **most behavior change comes from strong fear paired with high-efficacy messages**. Strong fear with low efficacy produces the **most defensive responses** (avoidance, reactance).
- **Peters, Ruiter & Kok (2013)**, revised meta-analysis of factorial studies only, *Health Psychology Review* 7(S1):S8–S31: threat had an effect **only under high efficacy (d = 0.31)**, and efficacy had an effect only under high threat (d = 0.71). Their conclusion: use threatening communication only when you know the intervention actually raises efficacy.
- **Tannenbaum et al. (2015)**, *Psychological Bulletin* 141(6):1178–1204, 127 articles, N = 27,372: fear appeals are effective overall (d = 0.29). They are more effective with efficacy statements, with high depicted susceptibility and severity, and for **one-time rather than repeated behaviors**. The authors found **no circumstances in which fear appeals backfired**.
- **Bubeck, Botzen & Aerts (2012)**, *Risk Analysis* 32(9):1481–1495, review: flood risk perception alone is rarely linked to mitigation behavior. **Coping appraisal** (self-efficacy, response efficacy, cost) consistently is.
- **The NHS efficacy gap:** only 32% have high preparedness efficacy (B.2).

**Design implications**
- *Framing rule: "every threat travels with its fix."* No risk figure appears on a screen without the specific action, its cost and its effect next to it. For example (illustrative numbers only, not findings): "Power outage of 3+ days: about N in 100 households like yours per decade. A $40 kit (lights, power bank, radio) covers the first 72 hours for your household of 3."
- *Be honest about threat.* Tannenbaum found no backfire, so do not play risk down. What goes wrong is showing threat *without* efficacy.
- *Put one-time actions early* (buy water containers, sign up for alerts, write the contact card), where persuasion works best. Use habit design (B.7) for repeated behaviors such as rotation and drills.
- *Measure efficacy:* ask the two NHS efficacy questions at the start and again at export, and show the change as feedback.
- *Self-efficacy is the bottleneck, not response efficacy.* In 2024, 60% believed preparing would help but only 43% felt confident they could do it. Build confidence through small, completable steps with visible success. Mastery experiences are the strongest source of self-efficacy in Bandura's theory (UNVERIFIED this session; recalled). Explain the "how" in concrete steps, and keep reassurance that "preparing works" short.
- *Visual tone:* avoid dramatic disaster imagery as hero art. Use capability imagery instead (a filled water jug, a checked-off card).

## B.4 Normalcy, optimism and availability biases, and presenting risk as natural frequencies

**Evidence**
- **Normalcy bias:** Omer & Alon (1994), *American Journal of Community Psychology* 22(2):273–287, define it as underestimating how likely or how large a disruption will be. They also define the opposite, **"abnormalcy bias"**: underestimating victims' ability to cope. Both lead to bad decisions.
- **Optimism bias:** Weinstein (1980), *JPSP* 39(5):806–820, found people believe bad events are less likely to happen to them than to others. Sharot (2011), *Current Biology* 21(23):R941–R945, reviews how widespread and persistent the bias is.
- **Availability after disasters:**
  - Gallagher (2014), *AEJ: Applied* 6(3):206–233: flood insurance take-up **spikes the year after a flood and then declines steadily to baseline**. Non-flooded communities in the same TV media market increase take-up at about one-third the rate. The pattern fits a model of learning with forgetting.
  - Michel-Kerjan, Lemoyne de Forges & Kunreuther (2012), *Risk Analysis* 32(4):644–658: the median life of a new flood insurance policy is only **2–4 years** before it lapses.
- **Single-action bias and the finite pool of worry** (Weber; summarized by Columbia's CRED guide, http://guide.cred.columbia.edu/guide/sec4.html):
  - After taking one protective action, people tend to stop, because their worry has dropped, even when the action gave only partial protection.
  - People can hold only a limited number of worries at once, so new concerns push out old ones. Later work adds qualifications ("A Finite Pool of Worry or a Finite Pool of Attention?", 2020, via ResearchGate).
  - The underlying paper is Weber (2006), *Climatic Change* 77:103–120.
- **The Ostrich Paradox** (Meyer & Kunreuther 2017, Wharton School Press) names six biases: myopia, amnesia, optimism, inertia, simplification and herding. It proposes a "behavioral risk audit". The definitions are UNVERIFIED this session because the publisher page blocked access; they are standard in the book (recalled).
- **Natural frequencies:**
  - Gigerenzer et al. (2007), *Psychological Science in the Public Interest* 8(2):53–96, recommend frequency statements over single-event probabilities, **absolute over relative risks**, and **natural frequencies over conditional probabilities**.
  - The Cochrane review (Akl et al. 2011, CD006776, 35 studies) found natural frequencies are **understood better than probabilities** (standardized mean difference 0.69, 95% CI 0.45–0.93). Relative risk reductions are judged larger and more persuasive, which risks misleading people.

**Design implications**
- *Default risk format:* "Of 100 households like yours in [county], about **N** will [go 3+ days without power] in the **next 10 years**." Show an icon array of 100 houses. Use 10- and 30-year horizons, because a 3% annual figure reads as "never" (myopia) while the same risk over 10 years, 1 - 0.97^10 ≈ 26 in 100, does not. Use absolute numbers only, never relative ones like "twice as likely".
- *Reference-class framing counters optimism bias.* Talk about "households like yours", not predictions about *you*. Show a **most-likely** scenario next to a **worst plausible** one.
- *Name both biases, gently.* The plan can say that most people underestimate disruption, and that people also cope better than they expect. That counters normalcy bias without adding fear, and counters the abnormalcy bias, which also underlies myths of helplessness and panic.
- *Redirect availability:* if the user says they worry about dramatic hazard X, acknowledge it. Then show that the likely disruptions (power, water, heat) are covered by the same capabilities, and what X would add on top. This uses consequence-based planning (A.6).
- *Counter single-action bias:* never celebrate "kit complete" after one purchase. Show "Tier 1: 3 of 7 done" with the next item already visible.
- *Defaults against inertia and simplification:* a pre-built plan with the first action already selected; the user edits rather than composes. *Against amnesia:* seasonal reminders and a plan "anniversary". *Herding:* social proof that encourages ("most households have some supplies; about a third of them have two weeks, so you can get there") and never one that makes low preparedness look normal.
- *Finite pool of worry:* show at most three headline disruptions. Frame preparation as *reducing* existing worries (health, bills), not adding new ones.

## B.5 Disaster myths, prosocial behavior, and social capital as a resilience predictor

**Evidence**
- **Panic is rare.** Clarke (2002), "Panic: Myth or Reality?", *Contexts* 1(3):21–26: group panic is relatively rare, and in disasters people are often models of civility and cooperation (paraphrased from the abstract).
  - This matches decades of disaster sociology from Fritz, Quarantelli and Drabek, including the Disaster Research Center's work. Specific citations for Quarantelli's panic essays (for example 2001, 2008 *Social Research* 75(3)) are UNVERIFIED this session (recalled).
- **Looting myths and the media.** Tierney, Bevc & Kuligowski (2006), "Metaphors Matter", *Annals of the AAPSS* 604:57–81: disaster myths such as looting and social disorder are widely believed, and the media spread them. After Katrina, coverage used a "civil unrest" frame and later likened survivors to combatants in urban warfare, which *greatly exaggerated* looting and lawlessness and fed calls for militarized response.
- **Shared identity in crowds.** Drury, Cocking & Reicher (2009), *British Journal of Social Psychology* 48(3):487–506: interviews with 21 survivors of 11 emergencies. A shared identity in the crowd increases solidarity and reduces "panic", and that identity can *arise from the emergency itself*.
- **Social capital and survival.**
  - Aldrich & Sawada (2015), *Social Science & Medicine* 124:66–75, covering 133 municipalities in Tohoku after the 2011 tsunami: mortality ranged from 0 to 10% of residents. Wave height, **stocks of social capital** and political factors strongly influenced mortality.
  - Ye & Aldrich (2019), *SSM – Population Health* 7:100403, covering 542 flooded neighborhoods: social capital was linked to lower mortality **especially for elderly and low-income residents**.
  - Aldrich & Meyer (2015), *American Behavioral Scientist* 59(2):254–269, argue that social, not physical, infrastructure drives resilience, and give policy recommendations.
- **Isolation kills in heat waves.** Semenza et al. (1996), *NEJM* 335:84–90, a case-control study of the 1995 Chicago heat wave (700+ excess deaths): living alone raised the odds of heat death (OR 2.3), and so did not leaving home daily (OR 6.7) and being confined to bed (OR 8.2 adjusted). Social contacts were protective, as were working air conditioning (OR 0.3) and access to transportation (OR 0.3). Klinenberg's *Heat Wave* (2002) is the sociological account.
- **Networks cut both ways.** Eisenman et al. (2007), *AJPH* 97(S1):S109–S115, interviewed 58 Katrina evacuees: strong ties to family and community both **helped and hindered** evacuation decisions. Community-based communication strategies are needed.
- **The survey gap:** 71% expect help from friends and family, but only 18% received information on helping neighbors (B.2).
- **Practitioner framing:** The Prepared says crime after ordinary disasters is far lower than people fear, and that groups beat lone wolves (paraphrased). UK Prepare and Listos both make neighbor actions explicit steps.

**Design implications**
- *A scored "people" category.* It covers: who checks on you, who you check on, an out-of-area contact, someone holding a spare key, a neighbor who knows your medical needs, and joining CERT, a block party or a mutual-aid group. It gets the same weight as water in the readiness score.
- *Risk-weighted priority:* for older adults living alone in heat- or cold-exposed places, a **check-in plan** is a tier-1 action, ahead of most purchases (Semenza).
- *Security module with no fear-mongering:* "security" defaults to fire safety, locks and lighting, documents, cyber hygiene and the neighbor network. Say plainly that looting and panic are far rarer than the media suggests (cite Tierney and Clarke).
- *Collective-efficacy framing:* "your street is a resource". Include templates for a contact list and a block-party invitation (Listos-style).
- *Evacuation realism:* ask about transport and caregiving ties, since networks can delay evacuation (Eisenman), and pre-arrange who goes with whom.

## B.6 Amanda Ripley's survival arc, rehearsal and pre-commitment

**Evidence**
- **The survival arc.** Ripley, *The Unthinkable: Who Survives When Disaster Strikes, and Why* (2008), is organized around **denial**, then **deliberation**, then the **decisive moment**.
  - Her examples include World Trade Center evacuees who delayed before leaving, milling about, talking and collecting belongings. Secondary sources cite NIST findings of **delays averaging about 6 minutes**; the precise NIST figures are UNVERIFIED against the NCSTAR report.
  - Her site stresses that the brain performs best under stress when it has already been through a few rehearsals (paraphrased; https://www.amandaripley.com/the-unthinkable).
- **Milling.** Wood, Mileti, Bean, Liu et al. (2018), *Environment and Behavior* 50(5):535–566: people seek confirmation from others before acting. **Longer, more complete warning messages reduced this confirmation-seeking** and shortened the delay compared with short messages.
- **Drills.** Vinnell, Wallis, Becker & Johnston (2020), *International Journal of Disaster Risk Reduction*, evaluated New Zealand's ShakeOut drills of 2012 and 2015. Participants had **better knowledge of correct protective actions, used them more during real shaking, took more additional preparedness actions, and showed weaker fatalism**. The study is observational, so self-selection is possible. Summary: https://resiliencechallenge.nz/outputs/evaluating-the-shakeout-drill-in-aotearoa-new-zealand-effects-on-knowledge-attitudes-and-behaviour/
- **Training and resilience.** Gargano et al. (2017), *Journal of Emergency Management* 15(5):275–284, studied WTC evacuees: **previous emergency training** was among the factors protecting against PTSD, and lack of training was a risk factor.
- **Implementation intentions.** Gollwitzer & Sheeran (2006), *Advances in Experimental Social Psychology* 38:69–119: "if situation X, then I will do Y" plans have a medium-to-large effect on reaching goals (d ≈ 0.65). The effect size is UNVERIFIED this session (recalled).
- **The Rescorla example.** Rick Rescorla ran repeated evacuation drills for Morgan Stanley at the WTC, and nearly all of its employees escaped on 9/11. Widely reported; UNVERIFIED this session (recalled).

**Design implications**
- *Pre-commitment in the packet.* Generate if-then triggers for each relevant hazard. For example: "If an evacuation WARNING covers zone ___, we leave within 15 minutes; the bag is at ___; we meet at ___; ___ picks up the kids." These shorten the denial and deliberation phases.
- *Decision cards for the decisive moment:* a one-page "go or stay" rule per hazard (shelter-in-place vs evacuate), written as long, complete messages, since Wood et al. found completeness reduces milling.
- *Drill scheduler:* 10-minute household drills (fire escape, drop-cover-hold-on, a power-out night, grabbing the go bag against a timer, shelter-in-place). Show a streak and "last practiced" date. Drill completion counts toward readiness, not just purchases.
- *Warning literacy:* explain local alert terms and sign-up channels, as Listos does.

## B.7 Checklists, decision fatigue, goal gradients, habit loops, and burnout and overspending

**Evidence**
- **Checklists work in high-stakes settings.** Haynes et al. (2009), *NEJM* 360:491–499: a 19-item surgical checklist across 8 hospitals was followed by deaths falling from **1.5% to 0.8%** and complications from **11.0% to 7.0%** (a before-and-after design).
- **Choice overload is real but depends on conditions.**
  - Chernev, Böckenholt & Goodman (2015), *Journal of Consumer Psychology* 25(2):333–358, 99 observations: overload increases with complex choice sets, hard decisions, uncertain preferences and a goal of minimizing effort.
  - Scheibehenne et al. (2010), *Journal of Consumer Research* 37(3):409–425, found a mean effect near zero across studies. That figure is UNVERIFIED this session (recalled).
- **"Decision fatigue" as ego depletion is weak evidence.** A multi-lab preregistered replication (Hagger et al. 2016, *Perspectives on Psychological Science* 11(4):546–573, 23 labs, N = 2,141) found a small effect whose confidence interval included zero. *Design for reducing choice complexity, not for "willpower depletion".*
- **Goal gradient.** Kivetz, Urminsky & Zheng (2006), *Journal of Marketing Research* 43(1):39–58: people speed up as a reward gets closer. Illusory progress (a 12-stamp card with 2 bonus stamps) sped up completion compared with a plain 10-stamp card, and a stronger acceleration predicted retention.
- **Endowed progress.** Nunes & Drèze (2006), *Journal of Consumer Research* 32(4):504–512: turning an 8-step task into a 10-step task with 2 steps already done raised completion rates and cut completion time. The field car-wash completion rates (about 34% vs 19%) are UNVERIFIED this session (recalled).
- **Habits take a long, variable time.** Lally et al. (2010), *European Journal of Social Psychology* 40(6):998–1009: automaticity took **18 to 254 days** to plateau, and **missing a single opportunity did not materially set people back**.
- **Rotation as a habit already in daily life:** Tokyo's everyday stockpiling (日常備蓄); the LDS "use and rotate"; The Prepared's first-in-first-out advice to store what you normally eat.
- **Overspending and burnout:**
  - The Prepared's "doomshopping" cycle and its fixed-monthly-budget remedy (A.4).
  - Mills (2018/2019), *Journal of Risk Research* 22(10):1267–1279, an ethnography of 39 preppers in 18 states: prepping is driven by **uncertain anxieties about many non-apocalyptic threats**, often amplified by mainstream news, rather than by apocalyptic certainty.
  - Practitioner sources describe prepper burnout as overwhelm from endless threats and impractical advice, plus trying to do everything at once. Examples: https://www.survivalsullivan.com/prepper-burnout/, https://prepper.com/prepper-fatigue/, and https://www.mdlinx.com/article/doomsday-prepping-when-preparedness-becomes-a-health-concern/6cKxSijuow3XUawk8XARsv. These are grey literature, not peer-reviewed.
- **Commercial kit math misleads** (A.4): "30-day" kits at about 1,700 kcal per day, and "720 servings" that last 88 days at 1,800 kcal.

**Design implications**
- *Short tiers:* at most about 7–10 items per tier, each a concrete item or action with a quantity for this household. The advanced detail is collapsed.
- *Endowed progress at onboarding:* an "already have" sweep (flashlight, first-aid basics, pantry items, bottled water, a charged power bank) credits existing items: "You're already 30% of the way to 72 hours" (example copy). Only credit items that really qualify.
- *Goal-gradient feedback:* a progress bar per tier, with the next tier previewed but locked until the current one is about 80% done, so users focus. Celebrate at the end of each tier, never after a single item.
- *Defaults over choice:* recommend one product *class* (for example "two 7-gal water containers"), not brand catalogs. Alternatives go behind "other options". Use no affiliate links; that also protects trust.
- *Habit loops:* tie rotation to cues people already have (grocery trips, daylight-saving time changes, birthdays, the start of hurricane or wildfire season). Monthly 5-minute check-ins. Tolerate missed checks: "one missed month is fine".
- *Stop rules against overspending and burnout:*
  - Each tier has a **"sufficient for your risk" line**, plus a **monthly budget cap** and a **cooling-off** suggestion for purchases over a threshold.
  - Flag items with low risk reduction per dollar, such as tactical gear or long-shelf-life kits that only make sense for storage-constrained people.
  - Offer a **"maintenance mode"** screen: "You're done for now. Here's the 10-minute monthly routine."
  - Tiers of 3+ months only unlock when the risk (for example isolation, or long restoration times) *and* resources justify them. Otherwise, beyond two weeks, steer toward a financial buffer, insurance and home hardening.

## B.8 How reputable organizations handle sensitive content

**Medical supplies and antibiotics**
- CDC, verbatim (https://www.cdc.gov/antibiotic-use/about/index.html): "Antibiotics can save lives, but any time antibiotics are used, they can cause side effects and contribute to the development of antibiotic resistance." "Antibiotics DO NOT work on viruses." "Taking antibiotics when you do not need them will not help you, and their side effects can still cause harm."
- Ready.gov, verbatim: kits should include "Prescription medications. About half of all Americans take a prescription medicine every day." (ready.gov/kit). Also: "Include items that meet your individual needs, such as medicines, medical supplies, batteries and chargers, in your emergency supply kit." (ready.gov/older-adults)
- American Red Cross: a **7-day** medication supply. UK Prepare: enough medication for several days, paper instructions for medical devices, and charged backup batteries (paraphrased).
- The Prepared (paraphrased): get prescriptions through a doctor; antibiotic resistance is real; don't self-medicate carelessly; it disclaims responsibility for misuse.
- Jase (paraphrased): prescriptions come through a telehealth physician; the medications are for when care is out of reach; it says it does not condone stockpiling.
- **Design:** the default medical module covers:
  - Talking to your prescriber or pharmacist about an emergency supply of chronic medications.
  - A printed medication list with doses.
  - Refrigerated-medication and powered-device plans.
  - First-aid and Stop the Bleed training.
- Rx emergency kits are mentioned *neutrally*, as something to discuss with a clinician, alongside CDC's stewardship message. The tool **never** gives dosing, never suggests fish or veterinary antibiotics, and never ranks antibiotics above water, food or chronic medications.

**Firearms and security**
- Ready.gov's security framing is **"Run. Hide. Fight."**, verbatim: "Getting away from the attacker is the top priority." "Fight only as a last resort." Its only weapon guidance is improvised items, and it makes no recommendation about owning firearms (https://www.ready.gov/public-spaces). The government and NGO preparedness sites I reviewed are silent on firearms.
- The Prepared's beginner guide (paraphrased) treats guns as a personal choice. It puts universal safety rules first, stresses locked storage away from children and others who shouldn't have access, and urges training before an emergency. In my reading it does not address suicide risk.
- Evidence: Anglemyer, Horvath & Rutherford (2014), *Annals of Internal Medicine* 160(2):101–110, pooled 16 observational studies. Household firearm access was associated with **suicide (OR 3.24, 95% CI 2.41–4.40)** and being a **homicide victim (OR 2.00)**.
- **Design:** firearms are *out of scope by default*. If a user opts in to a security topic, show safe-storage guidance, a plain note on suicide risk with the 988 crisis line, legal and training considerations, and an explicit reminder that firearms do not replace water, heat, medications or neighbors. Never include weapons in phase tiers or scoring.

**Nuclear and radiological**
- Ready.gov, verbatim (https://www.ready.gov/nuclear-explosion): "Get inside. Stay inside. Stay tuned." "Remain in the most protective location (basement or center of a large building) for the first 24 hours unless threatened by an immediate hazard." "Radiation levels decrease rapidly, becoming significantly less dangerous, during the first 24 hours."
- CDC on potassium iodide (KI), verbatim: KI "protects **only your thyroid**" against radioactive iodine; "Do not take KI unless you are instructed by public health or emergency response officials or a healthcare provider." (https://www.cdc.gov/radiation-emergencies/treatment/potassium-iodide.html)
- **Design:** a short, empowering card: go inside, get to the center or basement, stay for 24 hours, listen for instructions. The existing 72-hour kit is the preparation, so there are no fallout-shelter shopping lists. KI appears only as "follow official instructions"; local KI programs near nuclear plants are a link-out whose details are UNVERIFIED. Probability framing must stay honest: very rare for almost every household.

**Psychological preparedness**
- UK Prepare has a "Coping with trauma" section. The Australian Red Cross RediPlan includes a stress-management plan, Hobfoll's five elements and AIM.
- **Design:** a "calm plan" card covering what I'll feel, what helps, and who I'll call. Also a section of support resources for after an event.

## B.9 Consolidated design implications, ranked by strength of evidence and impact

1. **Efficacy first, always paired with threat** (EPPM, PMT/coping appraisal, three meta-analyses, NHS efficacy 32%). Every risk statement travels with the specific action, its cost and its effect. Measure efficacy before and after.
2. **Natural frequencies, reference classes and consequences** (Gigerenzer; Cochrane review; NHS perceived vs experienced). Say "N of 100 households like yours, over 10 years, will be without X for 3+ days", and show the likely next to the dramatic.
3. **Small phased tiers with endowed progress, one next action, and explicit "enough" lines** (goal gradient, endowed progress, single-action bias, choice-overload moderators, LDS/Tokyo phased precedents, doomshopping). Budget caps and a maintenance mode are part of the design, not add-ons.
4. **Community as a scored prep item, with myth-correcting security framing** (Aldrich & Sawada; Ye & Aldrich; Semenza; Drury; Tierney; Clarke; NHS 71% vs 18%).
5. **Pre-commitment and rehearsal** (Ripley; Wood et al.; ShakeOut evaluation; WTC training; implementation intentions). If-then triggers, go/stay cards, a drill scheduler, and drills that count toward readiness.
6. **Stage-matched onboarding and direct handling of barriers** (NHS stages; cost 26%, don't-know 25%, time 23%, won't-matter 21%).
7. **Rotation through everyday use, and habit cues** (Tokyo everyday stockpiling; Lally; FIFO). Tolerate lapses.
8. **Equity defaults** (NHS renter and low-income gaps). Renters insurance, no-cost actions, and alternatives for people without a car or with disabilities.
9. **Sensitive topics follow public-health practice** (CDC, Ready.gov, the Anglemyer evidence). Neutral, safety-first framing; opt-in for firearms; no dosing; nuclear kept short and honest.
10. **Plan in kcal and liters with cost per unit** (the commercial kit math). Never "servings".

---

# SURPRISES AND THINGS THE TEAM SHOULD KNOW

1. **Hazard data is disappearing from the web.** All observed or documented as of 2026-09-25:
   - FEMA's standalone NRI site redirects into RAPT.
   - The NRI Future Risk Index was pulled in February 2025.
   - NOAA stopped its billion-dollar disaster database in May 2025; Climate Central now runs it.
   - OPB Aftershock returns 404 and its repository is archived.
   - riskfactor.com redirects to First Street's main site.
   - Zillow removed climate scores after pressure from real-estate agents.
   - The FEMA community page for the NHS summary did not resolve from our environment (possibly transient), and FEMA's research page lists no NHS reports.
   - *Implication:* ship a bundled, versioned, offline data snapshot with provenance, and never depend on live federal endpoints at runtime.
2. **The US preparedness mood got worse even as reported actions rose.** In 2024, 57% said they were "not prepared", up from 49%, while 83% reported 3+ actions (FEMA flags a change in the survey instrument). Supplies are common (69%) but shallow: only about 23% of all adults have supplies they think would last more than two weeks.
3. **Fear appeals do not backfire.** Tannenbaum's 127-article meta-analysis found no condition in which they backfired, so "never mention the threat" is also a myth. The robust rule is threat *with* efficacy (Peters: threat works only when efficacy is high).
4. **Tokyo's open-source calculator sets duration by floor level**: 7 days on upper floors, 3 days otherwise. Housing type is a real driver of duration that hazard maps ignore. The code and quantities are MIT-licensed and reusable.
5. **The Oregon Resilience Plan (2013) already proposed a two-tier "how long until relief / until 90% restoration" rating**, almost exactly the output this project wants. Its recommendation to move messaging from 72 hours to at least two weeks is the evident basis for Oregon's "2 Weeks Ready".
6. **Commercial food kits undersell calories.** "30-day" kits provide about 1,732 kcal a day, and "720 servings" last 88 days at 1,800 kcal. Planning in servings misleads.
7. **The public already ranks the likely events correctly but is not equipped for them.** Power outage is the most experienced disruption (22%) and 63% rank it high-impact. Yet only 24–26% could last two weeks without power at home.
8. **The neighbor gap:** 71% expect help from friends and family, but only 18% got information on helping neighbors, even though social capital is among the strongest survival predictors in tsunami and heat-wave data.
9. **"Decision fatigue" is a shaky basis for design** (the ego-depletion replication was null or small). Choice complexity and preference uncertainty are the reliable levers.
10. **People already believe preparing works (60%), but only 43% feel able to do it** (2024 NHS, checked against the rendered slide). The tool's main job is building confidence through doable steps, not persuading people that preparedness matters.
11. **Market signals:** Judy's domain is parked for sale (business status unknown), and a competitor's July 2026 review found no prepper inventory app that assesses location hazards or builds a plan. The niche is unoccupied, but commercial prep brands are fragile.

---

# SOURCES

**Government and NGO**
- Ready.gov Build a Kit: https://www.ready.gov/kit
- Ready.gov Make a Plan: https://www.ready.gov/plan
- Ready.gov Low and No Cost Preparedness: https://www.ready.gov/low-and-no-cost
- Ready.gov Older Adults: https://www.ready.gov/older-adults
- Ready.gov Nuclear Explosion: https://www.ready.gov/nuclear-explosion
- Ready.gov Public Spaces (Run. Hide. Fight.): https://www.ready.gov/public-spaces
- FEMA App: https://www.fema.gov/about/news-multimedia/mobile-products
- American Red Cross survival kit: https://www.redcross.org/get-help/how-to-prepare-for-emergencies/survival-kit-supplies.html
- American Red Cross make a plan: https://www.redcross.org/get-help/how-to-prepare-for-emergencies/make-a-plan.html
- American Red Cross mobile apps: https://www.redcross.org/get-help/how-to-prepare-for-emergencies/mobile-apps.html
- Washington EMD preparedness: https://mil.wa.gov/preparedness
- Oregon 2 Weeks Ready: https://www.oregon.gov/oem/hazardsprep/Pages/2-Weeks-Ready.aspx
- Oregon Resilience Plan executive summary (2013): https://www.oregon.gov/oem/documents/oregon_resilience_plan_executive_summary.pdf
- Cal OES MyHazards: https://myhazards.caloes.ca.gov/ (redirects to an ArcGIS Instant app)
- MyHazards explainer: https://sj-admin.s3-us-west-2.amazonaws.com/0000_0000_CalOES_MyHazardsMaptool.pdf
- Listos California: https://www.listoscalifornia.org/
- Australian Red Cross Get Prepared app: https://www.redcross.org.au/emergencies/prepare/get-prepared-app/
- AJEM 2023 summary: https://knowledge.aidr.org.au/resources/ajem-july-2023-australian-red-cross-psychosocial-approach-to-disaster-preparedness/
- NZ Get Ready, storing water: https://getready.govt.nz/prepared/household/supplies (via search snippet)
- Tokyo disaster books: https://www.bousai.metro.tokyo.lg.jp/1028036/index.html
- Tokyo stockpiling project: https://www.bousai.metro.tokyo.lg.jp/kyojyo/1001855/index.html
- Tokyo Bichiku Navi: https://www.bichiku.metro.tokyo.lg.jp/
- Tokyo Bichiku Navi source code: https://github.com/Tokyo-Metro-Gov/bichiku-navi
- UK Prepare: https://prepare.campaign.gov.uk/ and https://prepare.campaign.gov.uk/get-prepared-for-emergencies/
- Canada Get Prepared: https://www.canada.ca/en/services/policing/emergencies/preparedness/get-prepared.html
- Canada emergency kits: https://www.canada.ca/en/services/policing/emergencies/preparedness/get-prepared/emergency-kits.html
- Canada make a plan: https://www.canada.ca/en/services/policing/emergencies/preparedness/get-prepared/make-plan.html
- EU Preparedness Union Strategy press release: https://ec.europa.eu/commission/presscorner/detail/en/ip_25_856
- Sweden MCF (formerly MSB): https://www.mcf.se/en/
- Germany BMEL/BLE stockpile calculator: https://www.ernaehrungsvorsorge.de/private-vorsorge/notvorrat/vorratskalkulator/
- LDS home storage: https://www.churchofjesuschrist.org/study/manual/gospel-topics/food-storage
- CDC antibiotic use: https://www.cdc.gov/antibiotic-use/about/index.html
- CDC potassium iodide: https://www.cdc.gov/radiation-emergencies/treatment/potassium-iodide.html

**Risk data, tools and frameworks**
- FEMA RAPT (where the NRI now lives): https://www.fema.gov/emergency-managers/practitioners/resilience-analysis-and-planning-tool
- FEMA NRI overview: https://www.fema.gov/flood-maps/products-tools/national-risk-index
- NRI documentation (December 2025): https://www.fema.gov/sites/default/files/documents/fema_national-risk-index_methodology-hazards-overview.pdf
- NRI Future Risk replica: https://github.com/fulton-ring/nri-future-risk
- First Street: https://firststreet.org/
- ClimateCheck: https://climatecheck.com/
- Augurisk: https://www.augurisk.com/
- Zillow's removal of climate risk scores:
  - https://grist.org/housing/zillow-deletes-climate-risk-data-from-listings-after-complaints-it-harms-sales/
  - https://www.cnn.com/2025/12/02/climate/zillow-climate-data-extreme-weather-first-street-redfin
  - https://techcrunch.com/2025/12/01/zillow-drops-climate-risk-scores-after-agents-complained-of-lost-sales
- Billion-dollar disasters database:
  - https://www.climatecentral.org/climate-matters/billion-dollar-disasters-oct-2025
  - https://www.cnn.com/2025/10/22/climate/extreme-weather-disaster-trump-noaa
- Hazus: https://www.fema.gov/flood-maps/products-tools/hazus
- THIRA/SPR: https://www.fema.gov/emergency-managers/national-preparedness/goal/risk-capability-assessment
- CPG 101: https://www.fema.gov/emergency-managers/national-preparedness/plan
- Sphere 2018 WASH chapter (mirror): https://ksrpmi.uns.ac.id/wp-content/uploads/2019/09/Sphere-Handbook-2018-WATER-SUPPLY.pdf
- Sphere handbook (canonical, blocked to automated access): https://spherestandards.org/handbook/
- WHO Interagency Emergency Health Kit: https://www.who.int/publications/i/item/9789241502115
- OPB Aftershock engine: https://github.com/hazard-ready/disaster-preparedness

**Commercial**
- The Prepared checklist: https://theprepared.com/prepping-basics/guides/emergency-preparedness-checklist/
- The Prepared on budgeting and "fantasy gear": https://theprepared.com/blog/how-to-shop-for-preps-without-going-over-budget-or-buying-fantasy-gear/
- The Prepared myths: https://theprepared.com/prepping-basics/guides/survival-disaster-prepper-myths/
- The Prepared guide to guns: https://theprepared.com/self-defense/guides/beginners-guide-to-guns/
- The Prepared home medical supplies: https://theprepared.com/homestead/guides/home-medical-supplies-list/
- Jase: https://jase.com/
- Duration Health: https://durationhealth.com/kit/1/about (did not resolve from our environment)
- Mountain House 30-day kit: https://mountainhouse.com/products/30-day-emergency-food-supply-kit
- ReadyWise 720 servings: https://readywise.com/products/720-servings-of-ready-wise-emergency-survival-food-storage
- Preppi: https://www.preppi.co/
- Uncharted Supply: https://unchartedsupplyco.com/
- Judy (Forbes, 2022): https://www.forbes.com/sites/karineldor/2022/12/30/how-pr-founder-simon-huck-leans-into-optimism-with-emergency-preparedness-kit-judy/
- Prepper app comparison (written by a competitor): https://beagleprep.com/blog/best-prepper-inventory-apps/
- r/preppers "Prepping for Tuesday" (Grist): https://grist.org/culture/prepper-disaster-food-pantry-staples-advice-treat/

**Behavioral science, in the order cited**
- Lindell & Perry 2012, doi:10.1111/j.1539-6924.2011.01647.x (PMID 21689129)
- Lindell & Whitney 2000, doi:10.1111/0272-4332.00002 (PMID 10795335)
- Lindell, Arlikatti & Prater 2009, doi:10.1111/j.1539-6924.2009.01243.x (PMID 19508448)
- FEMA 2024 National Household Survey findings (mirror): https://survivingcascadia.com/wp-content/uploads/2025/05/2024-national-household-survey-on-disaster-preparedness-findings.pdf
- NHS data archive: https://www.datalumos.org/datalumos/project/218642/version/V1/view
- Witte & Allen 2000, doi:10.1177/109019810002700506 (PMID 11009129)
- Peters, Ruiter & Kok 2013, doi:10.1080/17437199.2012.703527 (PMID 23772231)
- Tannenbaum et al. 2015, doi:10.1037/a0039729 (PMID 26501228)
- Bubeck, Botzen & Aerts 2012, doi:10.1111/j.1539-6924.2011.01783.x (PMID 22394258)
- Kohn et al. 2012, integrative review of personal preparedness, doi:10.1001/dmp.2012.47 (PMID 23077264)
- Ejeta, Ardalan & Paton 2015, systematic review of behavioral theories applied to preparedness, PLoS Currents (PMID 26203400)
- Omer & Alon 1994, doi:10.1007/BF02506866
- Weinstein 1980, doi:10.1037/0022-3514.39.5.806
- Sharot 2011, doi:10.1016/j.cub.2011.10.030
- Gallagher 2014, doi:10.1257/app.6.3.206
- Michel-Kerjan, Lemoyne de Forges & Kunreuther 2012, doi:10.1111/j.1539-6924.2011.01671.x
- Weber 2006, doi:10.1007/s10584-006-9060-3
- CRED guide: http://guide.cred.columbia.edu/guide/sec4.html
- Meyer & Kunreuther 2017, *The Ostrich Paradox*, Wharton School Press
- Gigerenzer et al. 2007, doi:10.1111/j.1539-6053.2008.00033.x
- Akl et al. 2011 (Cochrane), doi:10.1002/14651858.CD006776.pub2
- Clarke 2002, doi:10.1525/ctx.2002.1.3.21
- Tierney, Bevc & Kuligowski 2006, doi:10.1177/0002716205285589
- Drury, Cocking & Reicher 2009, doi:10.1348/014466608X357893
- Aldrich & Sawada 2015, doi:10.1016/j.socscimed.2014.11.025
- Ye & Aldrich 2019, doi:10.1016/j.ssmph.2019.100403
- Aldrich & Meyer 2015, doi:10.1177/0002764214550299
- Semenza et al. 1996, doi:10.1056/NEJM199607113350203
- Eisenman et al. 2007, doi:10.2105/AJPH.2005.084335
- Ripley 2008, *The Unthinkable*: https://www.amandaripley.com/the-unthinkable
- Wood et al. 2018, doi:10.1177/0013916517709561
- Vinnell et al. 2020, IJDRR: https://www.sciencedirect.com/science/article/pii/S2212420920306750
- Gargano et al. 2017, doi:10.5055/jem.2017.0336
- Gollwitzer & Sheeran 2006, doi:10.1016/S0065-2601(06)38002-1
- Haynes et al. 2009, doi:10.1056/NEJMsa0810119
- Chernev, Böckenholt & Goodman 2015, doi:10.1016/j.jcps.2014.08.002
- Scheibehenne, Greifeneder & Todd 2010, doi:10.1086/651235
- Hagger et al. 2016, doi:10.1177/1745691616652873
- Kivetz, Urminsky & Zheng 2006, doi:10.1509/jmkr.43.1.39
- Nunes & Drèze 2006, doi:10.1086/500480
- Lally et al. 2010, doi:10.1002/ejsp.674
- Mills 2019, doi:10.1080/13669877.2018.1466825
- Anglemyer, Horvath & Rutherford 2014, doi:10.7326/M13-1301 (PMID 24592495)
