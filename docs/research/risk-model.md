# Household Preparedness Planner: Quantitative Core, Model Specification

Draft v0.1, 2026-09-25. Research-and-proposal document for an offline, beginner-first, US-first planner (PWA, no server).
Companion prototype (illustrative, ~300 lines of Python): `scratchpad/research/rm_proto/` (`core.py`, `params.py`, `income.py`, `alloc.py`, `items_*.py`). The prototype generated every modelled number in sections 8 and 9.

## How to read the numbers in this document

Every number carries one tag.

| Tag | Meaning |
|---|---|
| **[SRC Sn]** | Taken from source *n* in section 12, as published. |
| **[DERIVED Sn]** | My calculation from source *n*. The method is stated next to the number. |
| **[PRIOR]** | Expert judgement (mine, for this draft). It must be replaced by structured elicitation or data before release, and users must be able to see it and change it. |
| **[UNVERIFIED]** | A fact I believe is right but did not check against a source in this pass. |
| **[ILLUSTRATIVE]** | A placeholder, such as a retail price, used only to make the worked examples run. |

The research ran under a budget of 20 web searches (17 used). Most figures came from fetched documents and from data pulled directly: FEMA NRI v1.20 county records, CMRA county projections, EIA-861 2024 reliability data, and EAGLE-I outage records for 2014 and 2018–2025 for the example counties.

---

## 0. The model in plain language

**Five sentences for a non-programmer:**

1. We don't prepare you for *hurricanes* or *earthquakes* as such. We prepare you for what they do to a household: no power, no safe tap water, can't go out, have to leave, too hot or too cold, no medicine, no phone, no paycheck, no home.
2. For your county we look up how often each kind of disaster happens. For your household we look up how often personal emergencies happen, such as losing a job or having a house fire. Then we estimate how long each disruption usually lasts, and how long a bad one lasts.
3. You pick how cautious to be, for example "ready for the kind of disruption that has about a 1-in-100 chance each year". We turn that into a number of days (or months, for money) for each kind of disruption.
4. Your monthly budget buys the cheapest protection first: free steps, then the first three days, then two weeks, and so on. We never spend on days 30 to 33 of water while days 1 to 3 are missing, because the first days get used many times more often.
5. Every number shows where it came from, how uncertain it is, and whether it rests on data or on an expert's best guess.

**What the research concluded:**

- **The consequence-bucket design is sound, and I recommend it.** It needs four refinements:
  - three kinds of bucket: *duration* buckets measured in days, *readiness* buckets such as go-bags that you either have or don't, and *money* buckets measured in months;
  - explicit coupling rules, for example a private well means a power cut is also a water cut;
  - named-scenario toggles for rare catastrophes that sit near the user's dial, such as Cascadia;
  - "baseline inventory" questions about what the household already has.
- **Maths:** a single exceedance curve per bucket, Λ(d) = "how many times a year a disruption longer than *d* days happens to a household like yours". This one curve gives three outputs:
  - the target number of days for any setting of the dial;
  - the natural-frequency sentences;
  - the value of each extra day of supplies. The value of day *x* equals Λ(*x*), so diminishing returns fall out of the maths and need no tuning.
- **The claim "months of self-sufficiency come from income loss and pandemics, not natural hazards" mostly survives.** Three qualifications:
  1. It fails on the Cascadia coast. For Coos Bay at the default dial the model gives about 2 weeks for power and about 7 weeks for well water.
  2. Hurricane coasts and strong Cascadia or earthquake zones get *weeks*, not days.
  3. Natural disasters do create months-long *displacement*. That is a money and insurance problem, like income loss, not a stockpile problem, and it is about ten times rarer than long unemployment.

  A pandemic creates months of disruption but only about two weeks of stockpile need, because shopping stayed possible in 2020.
- **Biggest data gaps:**
  - There is no per-customer outage-duration distribution. EIA publishes averages and EAGLE-I publishes county totals.
  - There is no national boil-water or water-outage database. EPA told Congress in 2024 that one does not exist.
  - Most societal hazard rates (grid collapse, cyber, unrest, supply chain) and every "probability my household is hit, given a county event" are expert priors.
  - The unemployment figures available are current-spell lengths, not completed spells, and are not household-level.
  - Federal hosting of key climate and risk datasets proved unstable in 2025 (details in section 1.1).

---

## 1. Existing frameworks: what to borrow

### 1.1 FEMA National Risk Index (NRI): exact formulation, version 1.20

**Version in force.** NRI v1.20.0 is labelled "December 2025" [SRC S2, S4, S5]. The update documentation states that v1.20.0 was released publicly on June 25, 2025 [SRC S4].

**Hosting.**
- The old `hazards.fema.gov/nri/...` pages now redirect (HTTP 301) to FEMA's Resilience Analysis and Planning Tool page. I observed this on 2026-09-25.
- The data remain available as ArcGIS feature services owned by `FEMA_NationalRiskIndex` [SRC S1].

**Equations.** All are computed at the Census-tract level. County values are sums over tracts [SRC S3].

```
For each hazard h (18 natural hazards) and consequence type c ∈ {buildings, population, agriculture}:

  EAL_h,c = Exposure_h,c × AnnualizedFrequency_h × HistoricLossRatio_h,c          [SRC S2, S3, S4]

  Population is converted to dollars with the Value of Statistical Life:
  $13.7 million per fatality, or per 10 injuries                                  [SRC S2]

  EAL (composite) = Σ_h Σ_c EAL_h,c                                               [SRC S3]

  Risk = EAL × CRF,   CRF = f( SocialVulnerability / CommunityResilience )        [SRC S3, S4]
```

**The Community Risk Factor, f(·).** The update documentation gives it exactly [SRC S4]:

```
  CRF = f( SV value / CR value ),   f(·) → τ(a = 0.5, b = 2, c = 1)
```

Here τ is "a triangular distribution with minimum 0.5, maximum 2, and mode 1". FEMA describes f as "a transformation that maps the ratio between Social Vulnerability and Community Resilience to CRF values" [SRC S3, S4]. Read literally, the SV/CR ratio is ranked among all communities at the same level and pushed through the inverse cumulative distribution of that triangle.

The data confirm this reading [DERIVED S1]:
- Across all 3,144 counties with a value in v1.20, CRF runs from 0.515 to 2.000.
- The county median is **1.134**. The median of a triangular(0.5, 1, 2) distribution is 2 − √0.75 = **1.134**.
- CRF correlates with the SV score at r = 0.98 and with the CR score at r = −0.57, so in practice it is mostly social vulnerability.
- Philadelphia: EAL $667.1M × CRF 1.5377 = $1,025.7M, which equals the published `RISK_VALUE`.

**Scores and ratings.**
- Scores are national percentiles (0 to 100) within the same geographic level. They are relative, not absolute, and "should be expected to change over time" as other communities change [SRC S3].
- Risk and EAL ratings come from k-means clustering into 5 groups (scikit-learn; n_init = 20, max_iter = 500, random_state = 42) [SRC S3].
- Social vulnerability and community resilience ratings are fixed quintiles [SRC S3].
- **History.** Up to v1.18.1 the equation was **Risk Score = EAL Score × SV Score × 1/CR Score**, all in percentiles. v1.19.0 (March 23, 2023) replaced it with the dollar-valued EAL × CRF, and added Risk Index *values* for the first time [SRC S4].

**What changed in v1.20** [SRC S4, S5]:
- **Social vulnerability source.** Moved from CDC/ATSDR SVI to the **Census Community Resilience Estimates (CRE)**. The CRE uses 10 household-level factors:
  - income-to-poverty ratio;
  - single or zero caregiver household;
  - crowding;
  - communication barrier;
  - lack of employment;
  - disability;
  - no health insurance;
  - age 65 or over;
  - no vehicle;
  - no broadband.
- **Inland flooding.** "Riverine flooding" was renamed "Inland Flooding" and now includes pluvial (rainfall) flooding, via FEMA's Geospatial Flood Risk Assessment.
- **Other hazard models.** Wildfire now uses USFS FSim. Earthquake uses the Hazus 6.1 analysis. Landslide uses the USGS national susceptibility model.
- **Loss data.** SHELDUS losses run through 2023, and historic loss ratios are now back-tested.
- **Heat and cold deaths.** Population EAL for heat and cold waves is scaled to CDC compressed mortality for 1996 to 2018.
- **Exposure.** Hazus 6.0 exposure, inflated to December 2024 dollars.

**The same data for the three counties used in this document** [SRC S1, retrieved 2026-09-25; derived ratios marked]:

| County | Risk (score, rating) | EAL/yr | Top EAL hazards ($/yr; annualized frequency) | SV score | CR score | CRF |
|---|---|---|---|---|---|---|
| Philadelphia, PA | 99.6, Very High | $667M | Inland flood $266M (5.8/yr); **heat wave $256M (11.1 event-days/yr), of which 18.7 fatalities/yr × $13.7M ≈ all of it**; cold wave $57M; earthquake $42M (0.0016/yr); tornado $21M; hurricane $12M (0.092/yr) | 88.1 | 48.5 | 1.54 |
| Coos, OR | 91.9, Relatively Moderate | $90M | Earthquake $52M (0.0196/yr; building loss ratio 5.6% per event); inland flood $22M; tsunami $11.5M (0.145/yr); coastal flood $3.8M | 67.2 | 48.2 | 1.29 |
| Miami-Dade, FL | 99.6, Very High | $825M | Inland flood $356M; hurricane $331M (0.305/yr); **cold wave $41M (1.85 fatalities/yr)**; heat wave $31M; tornado $25M | 75.9 | 42.9 | 1.39 |

**Caveats that matter for a household planner.** Items marked [DERIVED] are my inferences from the data.

1. **Frequency is not household probability.** AFREQ counts events per year somewhere in the county. For heat, cold and winter weather it counts *event-days*, which is why Philadelphia shows 11 heat-wave "events" a year. It is not the chance that *your* home is hit.
2. **Frequency and severity are fused.** The historic loss ratio is an average loss across all exposed value. It cannot separate "1% of homes lose everything" from "10% of homes lose 10%". The planner needs P(hit) and severity separately. [DERIVED]
3. **Rankings depend on a value judgement.** Population losses are monetised at $13.7M per life. In Philadelphia that makes heat-wave deaths about 38% of the county's EAL ($256M of $667M). [DERIVED S1, S2]
4. **Visible artefacts.** Miami-Dade's cold wave ranks #3 by EAL ($41M, about 1.85 deaths/yr), above heat wave. Treat single-hazard EAL rankings as screening information, not truth. [DERIVED S1]
5. **Periods of record differ by hazard** [SRC S4]:

   | Hazard | Record period |
   |---|---|
   | Heat wave | 2005 to 2024 |
   | Inland flood | 1996 to 2023 |
   | Tornado | 1950 to 2019 |
   | Hurricane | 1851 to 2020 |
   | Tsunami | 1800 to 2021 |

   Short records underestimate rare, long events.
6. **FEMA's own limitation.** The NRI is "for broad nationwide comparisons and is not a substitute for localized risk assessment" [SRC S5].
7. **Ecological fallacy.** SV and CR describe the community's population. Applying the CRF to one family double-counts or mis-counts that family's actual traits. [DERIVED]
8. **Terms of use** [SRC S5]:
   - Cite the dataset version and retrieval date, with the prescribed "not endorsed by FEMA" statement.
   - A clause forbids attempts to "reverse engineer" or "derive the underlying datasets". Using published AFREQ values as model inputs appears to be ordinary use, but **flag this for legal review** before release.
9. **Hosting risk.**
   - The NRI web pages were retired into RAPT (observed).
   - FEMA's Future Risk Index was online only from mid-December 2024 to mid-February 2025 [SRC S6].
   - The National Climate Assessment site went offline in June and July 2025 [SRC S34].
   - The app must ship **versioned, hashed snapshots** of every dataset.

**Borrow from the NRI:**
- the 18-hazard taxonomy;
- tract and county AFREQ, as event-frequency priors;
- per-hazard EAL, as a screening signal ("is this hazard material here?");
- the conceptual split between hazard, vulnerability and coping capacity.

**Do not borrow:**
- EAL dollars as a household ranking metric;
- the CRF as a household multiplier.

**Where vulnerability and coping capacity come from instead:**
- *Vulnerability* comes from the household's own answers: age 65+, meds, devices, no car, renter, and so on. These are, usefully, the same factors the CRE uses.
- *Coping capacity* comes from the household's preparedness, which is what we are building. Community resilience enters only as a modifier on restoration speed.

### 1.2 FEMA THIRA/SPR (CPG 201, 3rd edition, May 2018)

- **THIRA** is three steps [SRC S7]:
  1. identify threats and hazards;
  2. give them context;
  3. set capability targets in standardized language: "Within (#) (time) of an incident, [action] (#) [units]". Targets are built from *impacts* and *timeframe metrics*.
- **SPR** then:
  1. assesses capabilities;
  2. identifies and addresses gaps;
  3. describes the impact of funding sources [SRC S7].

**Borrow** the target grammar for household outputs. It is concrete and checkable:

> "Within 0 hours of losing tap water, supply 4 people with 1 gallon per person per day for 3 days."
> "Within 15 minutes of long shaking at work, reach ground above the tsunami zone on foot."

Also borrow the SPR loop as the app's gap view: target, current, gap, and what this month's money buys.

### 1.3 HAZUS

- Hazus 6.x supplies the building and population exposure under the NRI [SRC S5].
- The Hazus earthquake methodology includes **utility restoration functions**. These are the most widely used ready-made duration curves for post-earthquake water service; Table 8.22 gives electric-power equivalents. For potable-water components (after ATC-13, 1985) [SRC Hazus MR4 Tables 8.1.a/b]:

| Component | Damage state | Mean days out (standard deviation) |
|---|---|---|
| Treatment plant | Extensive | 32 (31) |
| Treatment plant | Complete | 95 (65) |
| Wells | Extensive | 10.5 (7.5) |
| Wells | Complete | 26 (14) |
| Storage tanks | Extensive | 93 (85) |

  Pipeline repair rates are modelled per worker-day [SRC Hazus MR4 Table 8.1.c].
- Hazus also estimates **displaced households and short-term shelter needs**.

**Borrow** these as default earthquake-to-water and earthquake-to-power duration priors, and the displacement logic for the structural bucket.

**Caveat:** the restoration curves are themselves 1985 expert opinion. Tag them as such.

### 1.4 ISO 31000

**Borrow the process skeleton only** [UNVERIFIED: standard not fetched]:
- establish context (the interview);
- identify, analyse and evaluate (the register and the dial, where the dial is the "risk criteria");
- treat (the budget plan);
- monitor and review (rotation reminders, annual re-run);
- communicate throughout.

ISO 31000 prescribes no maths. Its value here is making sure the app closes the loop: a plan that is never reviewed decays (water every 6 months, food by rotation).

### 1.5 FAIR (Open FAIR: O-RT and O-RA, The Open Group)

- **Structure.** Risk = probable frequency × magnitude of loss. Loss event frequency = threat event frequency × vulnerability [SRC S40].
- **Method.** Calibrated expert ranges (PERT), Monte Carlo simulation, and loss-exceedance curves [SRC S40].

**Borrow four things:**
1. The **factor tree**, re-read for households: household event rate = county event rate × P(household hit | event).
2. **Magnitude as a distribution**, here days out, not a point value.
3. **Calibrated ranges** (minimum, most likely, maximum) for every prior.
4. The **exceedance curve** as the central object. Our Λ_b(d) is a days-out loss-exceedance curve.

### 1.6 Actuarial return periods

For an event with annual rate λ:
- chance of at least one in T years = 1 − e^(−λT);
- return period = 1/λ.

A "1-in-100-year" event has:
- a 9.5% chance in 10 years;
- a 26% chance in 30 years.

"The 90th percentile of the worst event in a 10-year window" is the event with annual rate −ln(0.9)/10 = 0.0105, that is **about 1-in-95**.

Use this translation in the UI. Many people read "100-year flood" as "it happened, so we're safe for 99 years".

### 1.7 All-hazards and capability-based planning

National doctrine (PPD-8 and the National Preparedness Goal) is organised around **core capabilities**, not hazard-specific plans [SRC S7]. The consequence-bucket design is the household version: a bucket is a capability ("keep 4 people in drinking water").

The Oregon Resilience Plan adds a useful public-facing idea, a **two-tier rating** [SRC S30]:
- hours or days until *major relief arrives*;
- days or months until the community reaches *90% restoration*.

For the coast it listed "1–2 weeks" for relief and "3 years+" for restoration under present conditions.

**Borrow** this as the planner's headline pair: "bridge-to-relief" (what you must store) and "live-with-degraded-services" (what you must be able to do: treat water, heat safely, earn or cover income).

---

## 2. The key design idea: allocate by consequence bucket

### 2.1 Verdict

**Adopt it.** Four reasons:

1. **Supplies don't care which hazard used them.** Four gallons of water serve an ice storm, a main break and a chemical spill equally well. Allocating by hazard double-counts shared supplies and makes rare hazards look expensive.
2. **Consequence durations are where data exist.**
   - Utilities report outage hours.
   - Water systems report boil-water notices.
   - Surveys report displacement durations.
   - BLS reports unemployment spells.

   None of these are catalogued by "hazard".
3. **It speaks the user's language.** "3 days without tap water" is actionable. "Relatively High inland-flood EAL" is not.
4. **It is the household version of national capability-based planning** (§1.7), so targets map cleanly onto the THIRA grammar (§1.2).

**Failure modes to design out:**

| Failure mode | Fix |
|---|---|
| Buckets hide correlated events. A hurricane takes power, water, communications and roads *at once*. | Model *events*, not buckets. Each event draws durations for every bucket it touches, and each bucket's curve is built from the same event list, so correlation is preserved. Simultaneous-need checks (§4.2) use the shared event list. |
| Some needs are not durations. A go-bag is either ready or not. Income is in months. | Three bucket types (§2.3). |
| Rare catastrophes near the dial make targets jump (the "cliff"). | Named-scenario toggles and a cliff warning (§3.3). |
| Household specifics change the coupling. | Explicit coupling rules (§2.7). Examples: a well pump makes a power cut a water cut; a gas furnace needs electricity; a high-rise needs electric booster pumps. |

### 2.2 Bucket catalogue

| # | Bucket | Type | Unit | Sub-types | What covers it (examples) |
|---|---|---|---|---|---|
| B1 | No grid power at home | Duration | Days | Short (hours) / multi-day | Lights, power bank, radio, cooler plan, power station (optional) |
| B2 | No safe tap water | Duration | Days | **B2a boil-able** (a boil notice while taps still flow)<br>**B2b no water / do-not-use** (pressure loss, chemical, quake) | B2a: a way to boil, or a microbial filter.<br>B2b: stored water, water-heater reserve, filter plus raw source. |
| B3 | Food and household supplies access | Duration | Days | Can't go out (snow, curfew, stay-home order)<br>Shelves empty (pre-storm runs, shortages) | Deep pantry of foods you normally eat, rotated |
| B4 | Dangerous heat or cold with no working cooling or heating | Duration | Days | Heat / cold | Cooling plan, battery fans, cooling centres; warm layers, safe heat source |
| B5 | Medication and medical-supply continuity | Duration | Days | Oral / refrigerated / device consumables | Refill-at-7 rule, 90-day fills, cooler, written med list |
| B6 | Communications, information and payments | Duration | Days | Cell/internet down; card and ATM networks down | Power bank, battery or crank radio, written contacts, cash |
| B7 | Must leave home quickly | Readiness | P(need), notice time, days away | Minutes (tsunami, fire, wildfire)<br>Hours (flash flood)<br>Days (hurricane) | Go-bag, plan, routes, pet carriers, fuel rule |
| B8 | Stranded away from home | Readiness | P(need), distance | Commute distance × mode | Get-home bag sized to distance |
| B9 | Medical emergency when EMS is slow or unavailable | Readiness | P(need) | Injury, illness, chronic crisis | First-aid kit, training, med list |
| B10 | Loss of income | Money | Months of income gap after unemployment insurance (UI) | Job loss, disaster-related job loss, pandemic | Cash buffer (outside the supplies budget) |
| B11 | Structural damage or loss of home | Money + readiness | P(displacement), months displaced | Fire, flood, wind, quake | Insurance (renters, homeowners, flood, quake), documents, savings |

Notes on the list:
- **Supply-chain disruption** (a bucket in the brief) is folded into B3 and B5, because the same pantry and medicine buffer cover it.
- **"Cannot leave home"** (also in the brief) is B3's first sub-type.

### 2.3 Why three bucket types

**Duration buckets (B1 to B6).** Preparation scales with days, so the exceedance-curve target applies (§3.2).

**Readiness buckets (B7 to B9).** The question is whether to own the capability at all. The rule is:
- buy it when the 10-year chance of needing it exceeds a threshold (default 2% [PRIOR]); or
- buy it when it is cheap relative to the harm avoided.

Evacuation shows why a separate rule is needed. For a Philadelphia rowhouse the total rate is about 0.8% a year. That is *rarer* than the default dial (1.05% a year), so a pure duration-percentile rule would say "no go-bag". The 10-year chance of needing one is still about 5% [DERIVED from the prototype], which is enough to justify a $30 bag.

**Money buckets (B10, B11).** The target is months of income gap, or an insurance decision. It sits on a separate savings track, because turning a $60/month supplies budget into a months-long cash buffer would starve life-safety supplies.

### 2.4 Hazard to bucket map (default conditional durations)

Durations are shown as **median / bad case (90th percentile)**. In app builds, per-county data replace defaults where they exist; section 10 lists the data to use.

| Hazard | Buckets hit (median / P90) | Tag |
|---|---|---|
| Hurricane Cat 1–2 | **B1** 1.5 d / 5 d; **B2a** 6 d median (Texas hurricane notices); B7 evacuate with 1–3 d notice; B3 3–14 d | DERIVED S12 (Irma: exponential restoration rate β = 0.02/h, so median ≈ ln2/β = 35 h and P90 ≈ ln10/β = 115 h); SRC S14; PRIOR |
| Hurricane Cat 3+ / direct hit | **B1** 5 d / 16 d; **B2b** weeks (Asheville: about 7 weeks without potable water); B6 1–7 d; B11 address-specific | DERIVED S12 (Michael: β = 0.006/h, so median 116 h, P90 384 h); SRC S37; PRIOR |
| Winter / ice storm | **B1** 1.5 d / 5 d; **B2a** 7 d median (Texas winter notices); B3 1–3 d; **B4 cold** if heating needs power | PRIOR; SRC S14, S36 |
| Strong wind / derecho / nor'easter | **B1** Philadelphia: 12–14 h / 39–48 h. PECO's suburban counties in March 2018: time to restore half the peak (t50) 20–30 h; time to restore 90% (t10) 48–70 h. | DERIVED S10 |
| Heat wave | **B4 heat**: event-days per NRI; compound harm with B1 | SRC S1; SRC S35: a 5-day blackout during a heat wave would send more than half of Phoenix to emergency care |
| Inland / coastal flood | B7 (hours of notice); B11 (address: flood zone, basement); B2a | PRIOR; NRI HLR for screening |
| Earthquake (typical) | B11; **B2b** days to weeks (Hazus: wells extensive 10.5 d, plant extensive 32 d); B1 hours to days | SRC Hazus MR4; SRC S30 (Tohoku: over 90% of power back in 10 days) |
| **Cascadia M9** (named scenario) | Coast: **B1** 3–6 months; **B2b** 1–3 years (utility); relief 1–2 weeks; healthcare 3 years.<br>Valley: **B1** 1–3 months; **B2b** 1 month to 1 year. | SRC S30 (Oregon Resilience Plan 2013, "present conditions") |
| Tsunami (local source) | **B7**: 15–20 minutes to reach high ground on foot; B11 in the inundation zone | SRC S32 |
| Wildfire | **B7**: minutes to hours of notice; B3 or B4 smoke shelter-in-place for days; B1 utility shutoffs 1–3 d | PRIOR |
| Drought | B2 restrictions; private wells: low yield for weeks to months | PRIOR |
| Pandemic | **B3** 14 d / 45 d (stay-home orders: median 45 d, P10 to P90 27–73 d); **B10** months; B5 7 d / 30 d | DERIVED S19; PRIOR |
| Regional grid failure | **B1** 1 d / 3–4 d (the 2003 blackout lasted up to 4 days); B2b pumping; B6; B4 | SRC S10; PRIOR |
| Cyber outage | B6 payments 1–3 d; B5 pharmacy claims days to weeks | PRIOR |
| Civil unrest | B3 curfews 1–5 d | PRIOR |
| Chemical release | Shelter in place for hours; B2b do-not-use 1.5 d / 7 d | PRIOR |
| Nuclear / war / EMP | Shelter in place ≥ 24 h; everything else unknown | SRC S22; special handling in §6.3 |
| Job loss | **B10**: spell median about 10 weeks, P90 about 36 weeks, net of UI | PRIOR informed by S23, S25 |
| Home fire | **B7** minutes; **B11** weeks to months (long displacement most common after fires) | SRC S16, S27 |
| Medical emergency | B9 (hours); B10 if serious | SRC S26 |
| Vehicle failure / crash | B8 (hours); mobility cost; B9 or B10 after a crash | SRC S28; PRIOR |

### 2.5 Empirical duration evidence (what exists)

**Power**

*EIA national and state figures:*
- The average customer lost **11 hours** of power in 2024 [SRC S8]:
  - about 9 hours from major events, against an average of about 4 hours a year in 2014 to 2023;
  - about 2 hours from routine outages;
  - South Carolina: about 53 hours.
- Recomputed from EIA-861 2024 utility data (IEEE-standard reporters) [DERIVED S9]:
  - all reporters: 11.0 h (reproduces the published figure);
  - Pennsylvania 5.9 h; Oregon 9.7 h; Florida 24.8 h; South Carolina 52.6 h; Texas 26.9 h.

*Utilities in the worked examples* [DERIVED S9]. SAIDI is total outage minutes per customer per year; SAIFI is outages per customer per year. "Major event days" (MED) are the storm days utilities report separately.

| Utility (2024) | SAIDI with MED | SAIDI without MED | SAIFI with MED | SAIFI without MED |
|---|---|---|---|---|
| PECO | 212 min | 60 min | 0.97 | 0.67 |
| Coos-Curry Electric Co-op | 571 min | 181 min | 2.56 | 1.37 |
| Pacific Power (OR) | 368 min | 108 min | 1.41 | 0.88 |

*The tail is not published.* EIA gives averages, and utilities report system indices, not per-customer duration distributions. The restoration-modelling literature itself flags the scarcity of empirical restoration data [SRC S38].

*County-level outage events (Do et al. 2023)* [SRC S11]:
- Across 2,447 counties in 2018 to 2020, the median county had 2 (IQR 5) "8+ hour outages" a year.
- An "outage" here is a county event with ≥ 0.1% of customers out, not something that happens to a given customer.
- 62.1% of such events co-occurred with extreme weather.

*EAGLE-I* (ORNL) records county customers-out every 15 minutes, 2014 to 2025, covering 92% of customers by 2022 [SRC S10]. From it we can derive **restoration curves**: after an event's peak, the fraction of peak customers still out at time t approximates the survival curve of an affected customer's outage. My extraction, from single-year runs before spike filtering [DERIVED S10]:

| Event | Customers out (share of county) | Time to restore 50% of peak | Time to restore 90% | Time to restore 95% |
|---|---|---|---|---|
| Philadelphia Co., 2018-03-02 nor'easter | 32,117 (4.0%) | 14 h | 39 h | — |
| Philadelphia Co., 2020-06-03 storm | 43,823 (5.5%) | 12 h | 48 h | 70 h |
| Philadelphia Co., Isaias 2020-08-04 | 13,394 (1.7%) | 4.5 h | 36 h | — |
| Bucks, Montgomery, Delaware, Chester, 2018-03-02 | — | 20–30 h | 48–70 h | — |

Data-quality caveats:
- Several counties show a synchronized drop to zero at about 70 h, which suggests a collection gap.
- A 55% "outage" in Coos County on 2018-01-20 lasted under 15 minutes, which is a spike artefact.
- **The pipeline must filter spikes and gaps.**

*ORNL RePOWRD, hurricanes 2014 to 2020* [SRC S12]:
- Restoration follows N(h) = n_m + (N0 − n_m)·e^(−βh), where N0 is the peak number out, n_m the residual outages, h hours since the peak, and β the restoration rate.
- Time to 95% restoration, state-wide:

  | Hurricane | Hours |
  |---|---|
  | Michael | 463 |
  | Irma | 151 |
  | Harvey, Laura, Zeta | 71–223 |

- β ranged from 0.006 to 0.026 per hour.

*Cascadia (Oregon Resilience Plan)* [SRC S30]:
- Electricity: coast 3–6 months, valley 1–3 months.
- International comparisons in the same source: Tohoku 2011 restored over 90% of power in 10 days; Maule 2010 restored 95% within two weeks.

*PECO's five largest outages* [SRC S36]:

| Event | Customers out |
|---|---|
| Sandy, 2012 | 850,000 |
| Ice storm, 2014 | 715,000 |
| Ice storm, 1994 | 549,000 |
| Isabel, 2003 | 517,000 |
| Irene, 2011 | 508,000 |

Isaias (2020) reached over 300,000.

**Water**

*There is no national database.* The EPA's 2024 Report to Congress found no national boil-water tracking. Of about 4,000 advisories it located for 2021 [SRC S13]:
- 80% were for main breaks, distribution repairs or pressure loss;
- 2.2% were for power outages;
- 1.4% were for natural disasters;
- 29.5% appeared still open at year end, so duration data are unreliable.

*Texas, 2010 to 2022* [SRC S14]:
- 18,839 notices from 3,114 systems.
- Mean duration 10.9 days; mode 3–4 days (39.4% of notices); 3% lasted more than 31 days.
- Median for winter weather 7 days; for hurricanes 6 days.
- Winter Storm Uri produced more than 2,000 notices in one week, from about 40% of community water systems.

*Kentucky, 2004 to 2020* [SRC S15, from the abstract and summary; full text not accessed]:
- 36,673 advisories from 378 systems.
- Mean 5 days; 87% caused by line breaks.

*Catastrophic tail:*
- Asheville after Helene: the system-wide boil notice was lifted on Nov 18, 2024, about 7 weeks after landfall [SRC S37].
- Hazus earthquake curves (§1.3).
- Oregon Resilience Plan: coast drinking water and sewer 1–3 years; valley 1 month to 1 year [SRC S30].

**Displacement**

*Household Pulse Survey, national* [SRC S16, S17]:
- 1.5% of adults were displaced by a natural disaster in the past year (Dec 2022 to Sep 2023).
- In the earlier wave:
  - more than a third were out for less than a week;
  - 12% were out for more than 6 months;
  - 16% never returned.
- Renters: about 1 in 4 never returned, against about 1 in 10 homeowners.
- Long displacement is most common after fires.

**Stay-home orders**

- 42 states and territories issued mandatory orders between March and May 2020 [SRC S18].
- My computation for the 39 states with dated statewide orders [DERIVED S19]:
  - median 45 days;
  - P10 27 days; P90 73 days;
  - range: Alaska 24 days to Oregon 88 days.

**Income**

*Incidence and length, BLS* [SRC S25]:
- 8.3% of people who worked or looked for work in 2024 were unemployed at some point.
- Of those who both worked and were unemployed, 21.5% looked for work for 27 weeks or more.
- 18.7% had two or more spells.

*Current spells, August 2026* [SRC S23]:
- Median 11.4 weeks; mean 26.3 weeks; 27% at 27 weeks or more.
- These are *current* spells, which over-sample long ones. Completed spells are shorter.

*Displaced workers* [SRC S24]:
- 3.3 million long-tenured workers were displaced in 2023 to 2025.
- 66.1% were re-employed by January 2026.
- About 49% of re-employed former full-timers earned as much as before, or more.

*Household cushion* [SRC S29]:
- 63% of adults would cover a $400 emergency with cash or its equivalent.

**Pandemic**

- A COVID-intensity pandemic has an estimated recurrence of about 209 years (95% CI 182–244), roughly 0.5% a year, or a 38% lifetime chance [SRC S20].
- Faster emergence of novel diseases would scale this up roughly in proportion [SRC S20].

### 2.6 Where expert priors are unavoidable (label them PRIOR in the app)

**1. P(household hit | county event)** for every hazard.

This is the footprint fraction. EAGLE-I peak fractions give it for power. Nothing comparable exists for water, food access, communications or mobility.

**2. Most societal-hazard rates:**
- multi-day regional grid failure;
- cyber outage of payments, pharmacies or hospitals;
- civil-unrest curfews;
- chemical releases;
- supply-chain shocks for fuel, infant formula and medicines.

**3. Household-level rates** for boil-water notices and do-not-drink events, as opposed to per-system rates.

**4. Durations** for:
- medication-access disruption;
- communications loss;
- temperature-coincident outages;
- stranding.

**5. Mapping published ranges to distributions.**

For example, treating the Oregon Resilience Plan's "3–6 months" as median 3 months and P90 6 months.

**6. Income:**
- the completed-spell unemployment distribution;
- UI replacement rates and durations by state [UNVERIFIED in this pass];
- income-stability multipliers.

**7. Harm weights between buckets** (§4.1).

This is a value judgement and should be user-adjustable.

**Elicitation protocol for v1.0.** Use structured expert judgement with calibration questions:
- Cooke's classical model, or the SHELF protocol [UNVERIFIED citations];
- a minimum of 3 experts per parameter;
- a published rationale for each prior;
- a public "PRIOR registry" file, versioned with the data snapshot.

### 2.7 Household modifiers and coupling rules

Each rule is a line in a small table, so it stays explainable.

| Household fact | Effect in the model |
|---|---|
| **Private well with electric pump** | B2b inherits every B1 event, with q = 1. Adds well-pump failure (PRIOR 0.07/yr) and drought low yield (PRIOR). **Coos example.** |
| **High-rise (above the booster-pump floors)** | B2b and mobility (elevators) inherit B1 events [UNVERIFIED engineering generalisation; confirm with building]. **Miami condo case.** |
| **Gas furnace or boiler** | B4-cold inherits winter B1 events (ignition and blower need power) [UNVERIFIED generalisation]. |
| **Gas stove** | Covers B2a for as long as gas flows (manual lighting). **Philadelphia example.** |
| **Wood stove with fuel** | Covers B4-cold. **Coos example.** |
| **Refrigerated medication (insulin)** | B5 inherits B1 events longer than about 1 day unless a cooler plan exists. |
| **Powered medical device** | B1 harm weight × 3 [PRIOR]; battery backup becomes life-safety. |
| **Age 65+ or chronic illness** | B4 harm weight × 1.5 [PRIOR; SRC S35 for heat plus blackout]. |
| **Infant** | Water per day +50%, formula supply-chain sub-bucket [PRIOR]. |
| **Pets** | Water and food per day, carrier for B7; many shelters restrict pets [UNVERIFIED]. |
| **Renter** | No generator or transfer switch. B11 displacement more often permanent (1 in 4) [SRC S17]. Renters insurance with additional living expenses (ALE) is the key B11 instrument [UNVERIFIED product detail]. |
| **Rowhouse or attached house** | Neighbour-fire exposure (fire rate × 2 [PRIOR]); upper floors trap summer heat; shared walls retain winter heat [PRIOR]. |
| **No vehicle** | B7 mode = transit or walk, with longer lead times; B8 different. |
| **Commute distance D** | Get-home time ≈ D ÷ 3 mph on foot [PRIOR]. Sizes the B8 bag (water ≈ 0.5 L per hour in heat [PRIOR]). |
| **Income stability** | Unemployment incidence × 0.5 (tenured or public), × 1 (typical W-2), × 1.5–2 (gig, seasonal, commission) [PRIOR]. |
| **Utility reliability** | Scale routine and major-event B1 rates by the utility's SAIFI with and without MED, relative to its state [DERIVED S9 method]. |
| **Rural address** | Slower EMS response raises the value of B9 first-aid capability [S39; response-time figures UNVERIFIED in this pass]. |

---

## 3. From hazards to "days you need"

### 3.1 Event model

**Events.** For each hazard h at the household's location:

| Symbol | Meaning | Source |
|---|---|---|
| λ_h | County event rate (per year) | NRI AFREQ for natural hazards; national rates for personal and societal hazards (§6) |
| a_h | Footprint fraction: P(this household is affected \| event) | EAGLE-I peak fractions for power (DERIVED); flood zone and address for floods; 1 for personal events (already per household); PRIOR otherwise |
| m_h | Household modifier | §2.7 |

The household event rate is r_h = λ_h · a_h · m_h. Events are Poisson in time. Correlation between buckets comes from the fact that one event feeds several buckets.

**Bucket involvement.** Given an event, it triggers bucket b with probability q_{h,b}, and the duration D_{h,b} follows a distribution with survival function S_{h,b}(d) = P(D > d).

**Default duration form.** Durations are lognormal, parameterised by two numbers anyone can read, the **median** and the **bad case (P90)**:

  μ = ln(median),  σ = (ln P90 − ln median) / 1.2816

**Where the parameters come from:**
- **EAGLE-I restoration curves.** Per county and event type, median = t50 (time to 50% restored) and P90 = t10 (time to 90% restored). Mixtures over past events widen the tail automatically.
- **Pure exponential restoration** (ORNL form, rate β). Median = ln2/β and P90 = ln10/β.

### 3.2 The exceedance curve Λ_b(d), the central object

  **Λ_b(d) = Σ_h r_h · q_{h,b} · S_{h,b}(d)**

This is the expected number of times per year this household faces a bucket-b disruption **longer than d days**. It is a days-out version of FAIR's loss-exceedance curve (§1.5). Everything else follows from it.

**1. Target for a dial setting of "1-in-N years".** Solve Λ_b(d\*) = 1/N.

**2. Worst single event in a T-year window.** Under Poisson arrivals:

  P(worst ≤ d over T years) = exp(−T · Λ_b(d))

So "cover the worst event in T years with confidence c" means Λ_b(d\*) = −ln(c)/T. With T = 10 and c = 0.9 that gives Λ = 0.0105/yr, which is **1-in-95**.

**3. Natural frequencies.** Out of 100 households like yours, the number facing a disruption longer than d in the next T years is:

  100 · (1 − e^(−T·Λ_b(d)))

**4. Value of the x-th day of supplies.** The expected unmet days per year when you hold x days is:

  U_b(x) = ∫ₓ^∞ Λ_b(t) dt

so −dU/dx = Λ_b(x). **The marginal value of one more day equals the annual rate of events that outlast it.** Diminishing returns are built in. For Philadelphia water, the first 3 days of storage cover about **60×** more expected disruption-days than days 30–33; for power about 1,600×; for food about 190×. For Coos Bay well water the ratio is only about **19×**, because Cascadia makes long outages comparatively likely. That is exactly why Coos should store deeper. [DERIVED, prototype]

**5. Consumption (for rotation and cost planning).** Expected total days per year = Σ_h r_h q_{h,b} E[D].

**Why the worst single event, not days per year?** Stock is sized to the longest event, because you restock between events. Consumption, and therefore rotation cost, scales with total days per year. The model reports both.

### 3.3 The dial: default, justification, and the cliff

**Default:** "the 90th percentile of the worst disruption in 10 years", about **1-in-95**, shown to users as **"about a 1-in-100-year disruption"**.

**Why this default:**
1. **A familiar civic yardstick.** It matches the 1%-annual-chance standard behind FEMA flood maps [UNVERIFIED: NFIP definition, not fetched in this pass]. Users can be told "the same yardstick FEMA uses for flood zones".
2. **A 10-year horizon matches the plan's lifetime.** It is about how long gear and pantries last before replacement, and roughly how long a household stays in one place [PRIOR].
3. **"90% sure nothing you face in the next 10 years will be worse"** is a sentence people understand.
4. **It reproduces official guidance in ordinary counties.** In the Philadelphia example it yields 3 days for power and water, about 10 days of food and 2 weeks of medicine. That matches Ready.gov's "several days" of water [SRC S22] and the common "2 weeks at home" advice.
5. **The dial changes the finish line, not the order of purchase.** The allocator (§4) always buys the high-value early days first. A cautious dial never takes money from basics.

**User-visible settings:**

| Setting | Label | Annual rate | Chance in 10 years |
|---|---|---|---|
| 1-in-10 | "Common disruptions" | 0.10 | 65% |
| 1-in-50 | "Serious" | 0.02 | 18% |
| ≈1-in-95 (default) | "Very serious (1-in-100)" | 0.0105 | 10% |
| 1-in-500 | "Rare catastrophes" | 0.002 | 2% |

Every screen shows how the targets move across the settings.

**The cliff rule.** When a single hazard's household rate r_h is within a factor of about 3 of the dial rate, targets become hypersensitive to both the dial and r_h.

Example, Coos Bay well water [DERIVED, prototype]:

| Dial | Target |
|---|---|
| 1-in-50 | 14 days |
| 1-in-75 | 35 days |
| 1-in-95 | 50 days |
| 1-in-150 | 83 days |
| 1-in-500 | 196 days |

Switching the Cascadia rate from the published time-dependent estimate (1.0%/yr) to a time-independent recurrence rate (about 0.4%/yr, PRIOR reading of the same record) moves the default from **50 to 23 days**.

In these cases the app should:
1. say so plainly: "Your answer depends mostly on one event: a Cascadia earthquake";
2. present it as a **named scenario toggle** (on by default where state guidance addresses it, such as Oregon's call to prepare for "a minimum of two weeks" [SRC S30]);
3. show both the scenario plan and the no-scenario plan;
4. prefer **capabilities over stockpiles** for the long tail: a water filter plus a raw water source rather than 120 gallons.

### 3.4 Readiness buckets (B7 to B9)

Compute the 10-year need probability, P_need = 1 − e^(−10·Σr). Include the capability if either:
- P_need ≥ 2% [PRIOR default]; or
- value per dollar (§4) beats the current tier's marginal item.

Report the notice time distribution as well: minutes (tsunami 15–20 min [SRC S32]; fire), hours (flash flood) or days (hurricane). Notice time decides *what* the bag must contain, and whether it lives at home, in the car or at work.

### 3.5 Money buckets (B10, B11)

**Income (B10).**

*Spell frequency:* each earner has r ≈ 0.083/yr [SRC S25], multiplied by the income-stability factor [PRIOR].

*Spell length (weeks):* lognormal with median 10 and P90 36 [PRIOR]. This is consistent with BLS showing 21.5% of workers with unemployment in a year looking 27+ weeks [SRC S25]. It is shorter than current-spell statistics (median 11.4 weeks [SRC S23]), which over-sample long spells.

*Converting to a household gap* (earner share s, UI replacement ρ for W weeks) [PRIOR]:

  gap_months(D) = [ s·(1−ρ)·min(D, W) + s·max(D − W, 0) ] / 4.345

Defaults: s = 0.5, ρ = 0.5, W = 26 [UNVERIFIED state-specific; the app should look up state UI rules].

Λ_income(g) and the dial target follow exactly as in §3.2.

**Structural (B11).** The output is an insurance and documents decision plus a displacement-cost estimate. Its drivers:
- the NRI loss ratio as a screening signal;
- address flood zone;
- fire rate;
- Household Pulse displacement durations [SRC S16, S17].

No stockpile target.

### 3.6 Parameter uncertainty

**Every rate and duration carries a range.** Examples:
- rate ~ lognormal with a stated "factor of k" uncertainty;
- durations with uncertain median and P90.

**Outer Monte Carlo:** draw θ about 2,000 times, compute d\*_b(θ), and report the median and the P10–P90 of d\*.

**Round up to a ladder:** {½, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90, 180, 365} days. Show "about 3 days (2–5)".

**Sensitivity (one-at-a-time).** Name the one or two parameters that drive the range, for example "mostly depends on how often big ice storms hit your street (expert estimate)".

### 3.7 Output statement format ("how many days, for what")

> "Plan to manage **about 3 days** at home without power or tap water, with **food for about 10 days** and **2 weeks of Grandma's medicine**.
> Keep a **get-home plan** for the 12-mile trip.
> Your family's biggest long disruption is **losing a paycheck**. The target there is **about 4 months** of income gap. That is a savings goal, separate from supplies."

Each phrase links to its bucket panel: curve, sources, range, dial sensitivity.

### 3.8 Claim check: "months come from income and pandemic, not natural hazards"

**At the default dial** [DERIVED, prototype with the parameters in §8 and §9]:

| Bucket | Philadelphia rowhouse (renter) | Coos Bay rural homeowner, private well |
|---|---|---|
| Power | 2.8 d | 13 d |
| Tap water (no-flow or do-not-use) | 2.8 d (boil-notice type 4 d, covered by gas stove) | **50 d** (well pump plus Cascadia) |
| Food and supplies | 9.6 d | 17 d |
| Medication | 12 d | 21 d |
| Communications | 1.4 d | 6 d |
| **Income gap** | **3.8 months** | **5.8 months** |

**National context from the NRI v1.20 county file** [DERIVED S1]:
- **Hurricanes:**
  - 27% of the US population lives in counties with hurricane AFREQ ≥ 0.1/yr;
  - 10.8% where it is ≥ 0.2/yr.
- **Earthquakes:** 20% live where the damaging-earthquake AFREQ is ≥ 0.01/yr.
- **Cascadia outer coast:** Clatsop to Curry in Oregon, Pacific to Clallam and Jefferson in Washington, and Del Norte to Mendocino in California hold roughly 0.2% of the US population. The wider Cascadia region, including the I-5 valley, holds a few percent.

**Displacement against income, national rates** [DERIVED S16, S17, S25]:
- Disaster displacement longer than 6 months: about 1.5%/yr × 12% ≈ **0.18% of adults per year**.
- Unemployment of 27+ weeks: about 8.3% × 21.5% ≈ **1.8% of labour-force participants per year**, roughly **10× more common**.

**Verdict: the claim survives for most households, with three qualifications.**

1. **Most counties:** the grid, water and supply buckets come out at **3–14 days**. Months come from **income** (and from home loss, which is also a money problem).
2. **Pandemics are months of disruption but not months of stockpile.** Stay-home orders had a median of 45 days, but shopping continued. The pandemic's stockpile contribution is about 2 weeks. Its months show up in income.
3. **Exceptions where nature produces weeks or months:**
   - **Cascadia coast:** power for months, water for months to years [SRC S30]. At the default dial the model gives about 2 weeks of power and about 7 weeks of well water. At 1-in-500, 4–6 months.
   - **Hurricane coasts:** weeks. Michael's restoration rate implies a P90 of about 16 days [DERIVED S12]. Asheville went 7 weeks without potable water [SRC S37].
   - **Private-well or high-rise households** anywhere: water inherits power outages.
   - **US territories and remote Alaska** are plausible exceptions not analysed here (for example Puerto Rico after Maria [UNVERIFIED duration]).

---

## 4. Budget allocation

### 4.1 Value per dollar, with diminishing returns built in

**Value of an item.** For an item that adds Δ days of coverage to bucket b, moving coverage from x to x + Δ, over a 10-year horizon:

  V = 10 · w_b · ∫ₓ^{min(x+Δ, target)} Λ_b(t) dt

This is **expected weighted disruption-days covered per decade**. Readiness items use:

  V = 10 · w · r_need · (day-equivalents of harm avoided per use)

**Harm weights w_b** [PRIOR, user-adjustable]:

| Bucket | Weight |
|---|---|
| Water, dependent's medicine | 3 |
| Temperature exposure (with a vulnerable member) | 2 |
| Food, power, communications | 1 |

Two refinements:
- Coverage **caps at the target**: no value is credited beyond the dial.
- **Baseline inventory** (the "what you already have" checklist) sets the starting x.

This makes the problem a knapsack with concave value per bucket. For divisible supplies (water, food) greedy by value per dollar is optimal. For lumpy items it is near-optimal and fully explainable. An exact 12-month dynamic program can run as a hidden check.

### 4.2 Algorithm (what the prototype implements)

```
Month 0:
  apply all zero-cost actions (§4.3); update coverage

Each month m:
  cash += monthly_budget

  For each tier k in [T1 72 h, T2 get-home bag, T3 2 weeks, T4 1 month, T5 3 months, T6 1 year]:
    horizon H_k (days)
    tier target per bucket: t_b,k = min(d*_b, H_k)

    candidates = items unlocked at tier ≤ k with positive value under the t_b,k caps
    order: life-safety items first, then value per dollar
    promotion: add any cheap later-tier item whose value per dollar ≥ 5 × this tier's best
               (example: a $10 extra week of a dependent's medicine)
    buy the best affordable candidate
    sinking fund: if the best candidate costs ≤ 2 × budget and the best affordable one
                  is worth < ¼ of it, save instead of frittering

    Stop when no tier has a positive-value candidate.

When all tiers are done:
  route surplus to the income bucket's cash buffer, or suggest raising the dial
  schedule rotation: water every 6 months [SRC S22]; food by eating it (first in, first out)
```

**Tiers T6 and T5 appear only when needed:**
- T6 appears only if some d\*_b > 90 days (for example Cascadia water at 1-in-500).
- T5 is mostly the money track.

**Simultaneous-need check.** Because events are shared across buckets, run the stock-out check on the joint draw as well. A Cat 3 hurricane must be covered for power, water and food *at the same time*. This matters for storage space, not for money.

### 4.3 Zero-cost, high-value actions (always month 0)

**Plan and contacts**
- Meeting places, an out-of-area contact, and who collects the children.
- Written contact cards: phones die.
- Swap numbers with two neighbours.
  - Social ties speed disaster recovery [UNVERIFIED claim; a large literature exists].

**Documents**
- Phone photos plus paper copies in a zip bag: IDs, lease or deed, insurance, medication list, pet records.

**Alerts**
- Local text alerts, Wireless Emergency Alerts on, a weather app.

**Free water**
- Fill reused drink bottles (not milk jugs [UNVERIFIED]) and replace them every 6 months [SRC S22 for replacement].
- **Know that your water heater holds 40–50 gallons** [UNVERIFIED typical tank size]. Close its inlet on a do-not-drink notice so contaminated water doesn't enter. Strap it in quake country.

**Medication**
- Refill when 7 days remain.
- Ask about 90-day fills.
- Keep a written medication list.
- Know where the nearest 24-hour pharmacy is.

**Temperature plan**
- Nearest cooling centre, coolest room and warmest room.
- A check-in schedule for vulnerable members.
- **Never run a generator, grill or camp stove indoors** (carbon monoxide).

**Fire and CO**
- Test smoke alarms.
- Confirm a CO alarm is present (often the landlord's legal duty [UNVERIFIED by state]).
- Two ways out.

**Car and home**
- Keep the tank at least half full.
- Know where the spare tyre is.
- Know your utility's outage-report channel.

**Existing assets count as coverage**

| Asset | Covers |
|---|---|
| Gas stove | B2a (boil) |
| Wood stove | B4-cold |
| Bathtub | Water, *if there is warning* |
| Freezer | Food for about 1–2 days if unopened [UNVERIFIED] |

**Insurance check**
- Renters or homeowners, flood and quake: *decide*, don't buy blindly (§6).

### 4.4 Item catalogue schema (a data file, not code)

```yaml
- id: water-5gal-x2
  name: "Two 5-gallon water containers + unscented bleach"
  price_usd: 35                # ILLUSTRATIVE; regional price table, user-editable
  tier: 1
  life_safety: false
  adds: { B2b: {per_person_days: 2.5, household_scale: people} }
  shelf_life_months: 6         # water refresh
  space_litres: 40
  constraints: [renter_ok]
  why: "covers days 3–5 of a no-tap-water event"
  sources: [S22]
```

Coverage scales with household size (people, pets) and climate (water needs "can double" in heat [SRC S22]).

### 4.5 Presentation

Each month shows **one card**: "This month: **$60**".

> **$35 — lights for everyone** (2 headlamps, 2 lanterns, batteries).
> *Why:* short outages are the disruption you're most likely to face. In 100 households like yours, about 26 will have a power cut lasting a day or more in the next 10 years, and nearly all will have shorter ones.
> **Source:** PECO 2024 reliability data and EAGLE-I county outage records (data), plus an expert estimate for big regional storms (PRIOR).
> *What this completes:* "Power: 3 of 3 days ✔".

The card also shows:
- a progress bar per bucket, from current days to the target;
- "what's next month";
- **the last line always offers a free action.**

---

## 5. Climate adjustment ("today" vs "around 2050")

### 5.1 Principles

- **Change frequencies, not restoration durations.** Climate projections speak to how often hazards happen and how intense they are. There is no defensible basis for adjusting how long utilities take to restore service. The one exception is a modest intensity shift for hurricanes, tagged PRIOR.
- **Use county-resolution, open, reproducible sources. Show ranges across emissions pathways and models.**
- **Threshold-count variables (days above 95°F) use additive deltas, not ratios.** Ratios of small counts are unstable: Philadelphia's days above 95°F go from 3.3 to 19.0, a ratio of 5.8×, and the model spread is 0–69 days.
- **"Today" means the NRI baseline.** Its periods of record end between 2019 and 2024, so recent warming is already partly included.
- **"Around 2050" means 2036–2065.** SSP2-4.5 is the central case and SSP5-8.5 the high case. This matches both CMRA and FEMA's Future Risk Index [SRC S6, S33].

### 5.2 Data

**CMRA 2025 county data** (LOCA2 downscaled CMIP6; multi-model mean, with minimum–maximum in brackets) [SRC S33, retrieved 2026-09-25]:

| County | Variable | Historical | SSP2-4.5 mid-century | SSP5-8.5 mid-century |
|---|---|---|---|---|
| Philadelphia | Days > 95°F | 3.3 [0–18] | 19.0 [0–69] | 26.6 [0.5–74] |
| Philadelphia | Days > 90°F | 18 | 53 | 64 |
| Philadelphia | Hottest 5-day maximum (°F) | 93.1 | 98.0 | 99.5 |
| Philadelphia | Days > 2 in. precipitation | 1.5 | 2.0 | 2.2 |
| Philadelphia | Icing days | 14.2 | 3.9 | 2.4 |
| Coos | Days > 90°F | 0.2 | 1.0 | 1.4 |
| Coos | Longest dry spell (days) | 53 | 61 | 62 |
| Coos | Days > 2 in. precipitation | 3.2 | 3.9 | 3.9 |
| Miami-Dade | Days > 95°F | 1.4 | 24.8 | 45.8 |
| Miami-Dade | Days > 90°F | 61 | 138 | 158 |

**NCA5 (2023), observed change in heaviest-1% daily precipitation, 1958–2021** [SRC S34]:

| Region | Change |
|---|---|
| Northeast | +60% |
| Midwest | +45% |
| Southeast | +37% |
| Northern Great Plains | +24% |
| Southern Great Plains | +21% |
| Alaska | +21% |
| Southwest | +17% |
| Northwest | +1% |
| Hawai'i | −19% |
| Puerto Rico | −24% |

**NCA5 projections** [SRC S34]:
- heatwaves become more frequent, severe and long;
- heavier extreme precipitation;
- stronger tropical cyclones (about 5% faster winds at 2°C of global warming), and Category 4–5 frequency likely to rise;
- more frequent co-occurring hot and dry conditions, raising extreme wildfire risk.

**Hosting caveats:**
- The official NCA5 site is offline; the text above is read from an unofficial, unaltered mirror [SRC S34].
- FEMA's **Future Risk Index** (removed February 2025; data mirrored [SRC S6]) covers five hazards (coastal flooding, drought, extreme heat, hurricane, wildfire) under four scenarios. Its method:
  - a Hazard Multiplier, HM = max(1, anticipated ÷ historical benchmark), applied to EAL;
  - exposure, loss ratios and the CRF held constant;
  - a single climate model (MPI-ESM1-HR) for heat;
  - results published as ratings rather than values, because of uncertainty [SRC S6].

### 5.3 Method in the planner

| Hazard | Adjustment | Confidence |
|---|---|---|
| Heat (B4-heat, and heat-driven outages) | Add the CMRA delta in days above the local threshold to the heat event-days. Scale the "outage coincides with dangerous heat" factor by the change in days > 90°F, capped at ×3. Philadelphia example: thermal target 1.7 d → 2.4–2.9 d; 10-year chance of ≥ 1 day without cooling in dangerous heat 16% → 24–31% [DERIVED, prototype]. | High for the direction; moderate for the size |
| Heavy precipitation / pluvial flooding | Multiply the inland-flood rate by the ratio of days > 2 in. (Philadelphia 1.33–1.47×). Flood-exposed addresses only. | Moderate |
| Coastal flooding | NOAA sea-level-rise scenarios converted to high-tide-flood frequency (the FRI approach) | High for the direction |
| Hurricane | Intensity shift: raise P(Cat 3+) and household hit fraction by a modest factor (PRIOR ×1.1–1.3); keep the county frequency. **Do not** change county hurricane frequency. | Low to moderate |
| Wildfire / drought | FRI hazard multiplier where available; otherwise the change in longest dry spell as a PRIOR scaling. | Moderate in the West |
| Cold / ice / winter weather | Allow **multipliers below 1** (Philadelphia icing days 14 → 2–4), unlike FRI's floor at 1. Keep a minimum ("fewer, not none"). Label clearly. | Moderate for cold; **low for ice storms** |

**Show both panels side by side,** "Today" and "Around 2050 (range)":
- climate-adjusted numbers carry the tag **[CLIMATE-ADJ SSP2-4.5 / SSP5-8.5]**;
- the register never silently re-ranks. It shows arrows.

### 5.4 Not defensible (do not do)

- **County-level changes in hurricane *frequency*.**
- **Tornado, hail, or ice-storm frequency changes.** Confidence is low; say "unclear".
- **Changes to utility restoration durations.**
- **Stacking multipliers:** heat × precipitation × wildfire on the same event.
- **Sub-county precision from county-average projections.** FEMA itself warns that county aggregation dilutes extremes [SRC S6].
- **Any climate adjustment to earthquakes, tsunamis, volcanoes, or to societal and personal hazards.**
- **Using a single climate model's number** without showing the model range.

---

## 6. Personal and societal base rates

### 6.1 National annual base rates

| Event | Rate | Tag |
|---|---|---|
| Reported residential fire | 344,600 fires (2023); ≈ 0.26% of households/yr (assuming ≈ 131M households) | SRC S27; DERIVED; household count UNVERIFIED |
| Residential fire deaths / injuries | 2,890 / 10,400 (2023) | SRC S27 |
| Displaced by a natural disaster | 1.5% of adults/yr (Dec 2022 – Sep 2023) | SRC S16 |
| Unemployed at some point in the year | 8.3% of those who worked or looked for work (2024); 21.5% of those with unemployment looked 27+ weeks | SRC S25 |
| Long-tenured job displacement | 3.3M over 2023–2025 (≈ 0.65%/yr of the labour force) | SRC S24; DERIVED; labour force ≈ 168M UNVERIFIED |
| Emergency-department visit | 47.3 per 100 persons/yr (155.4M visits, 2022) | SRC S26 |
| Injured in a traffic crash | 2,442,581 injured (2023) ≈ 0.73% of people/yr; 6.14M police-reported crashes; 40,901 deaths | SRC S28; DERIVED |
| Power interruption | 11 h per customer (2024); about 4 h/yr from major events in 2014–2023; about 2 h routine | SRC S8 |
| Outage ≥ 8 h at a given home | **Not published.** Model: ≈ 8%/yr for a Philadelphia rowhouse, ≈ 16%/yr for rural Coos County. EAGLE-I county-scale events alone (2018–2025): ≈ 1.4%/yr and ≈ 10.9%/yr. | DERIVED S9, S10; PRIOR |
| Boil-water advisory | Per system: Kentucky ≈ 5.7 per system-year; Texas ≈ 0.5 per issuing system-year. **Per household: unknown.** PRIOR 2–10%/yr. | SRC S14, S15; DERIVED; PRIOR |
| Pandemic of COVID intensity | ≈ 0.5%/yr (recurrence ≈ 209 yr); PRIOR 0.5–2%/yr for "disruptive to daily life" | SRC S20; PRIOR |
| Nuclear catastrophe (> 10M deaths within 5 years) | Experts 5% by 2045 (≈ 0.24%/yr); superforecasters 1% (≈ 0.05%/yr) | SRC S21; DERIVED annualisation |

**What these mean for the Philadelphia family** (two earners, four people) [DERIVED]:

| Event | Chance in the next 10 years |
|---|---|
| At least one unemployment spell | about **8 in 10** |
| Someone in the household visits an emergency department | expected about 2 visits a year (near certain in any given year) |
| Someone injured in a crash | about **1 in 4** |
| Home fire, including a neighbour's attached rowhouse | about **1 in 20** |
| Power cut of ≥ 1 day | about **1 in 4** |
| Power cut of ≥ 3 days | about **1 in 11** |

These **dominate the register on likelihood**. For months-long disruptions, long unemployment is about 10× more common than months-long disaster displacement (§3.8). In the Philadelphia example, income is the only bucket whose target runs to months.

### 6.2 Societal hazards: priors used in the examples

Every entry here is a PRIOR, to be replaced by elicitation.

| Hazard | Rate | Duration |
|---|---|---|
| Multi-day regional grid failure | 0.5%/yr (Philadelphia) | median 1 d, P90 3 d |
| Pharmacy or insurer IT outage | 2%/yr | median 5 d, P90 21 d |
| Civil-unrest curfew | 3%/yr | median 2 d, P90 5 d |
| Chemical do-not-drink advisory | 2%/yr | median 1.5 d, P90 7 d |
| Store shortages / pre-storm runs | 20%/yr | median 2 d, P90 5 d |
| Pandemic affecting food access | 1%/yr | median 14 d, P90 45 d |

**Show these with a "why we think this" note and a user override.**

### 6.3 Nuclear, EMP and war: honest presentation without budget hijack

**1. Separate category.** Show it in a "Rare and catastrophic" box. Never rank it by expected-loss product, because p × huge loss lets tiny probabilities dominate. Show likelihood and severity as two separate columns.

**2. Natural-frequency range, not a point estimate.** "Experts' estimates of a worldwide nuclear catastrophe range from about 1 in 2,000 to about 1 in 400 per year [SRC S21]. There is no reliable estimate for effects at your address."

**3. Show the overlap with general preparedness.**
- Official guidance: **"Get inside, stay inside, stay tuned."** Stay in a basement or the centre of a building for **at least 24 hours** [SRC S22].
- The household's tier-1 and tier-3 supplies already cover that sheltering phase.
- So do the battery radio (B6) and the basement plan (a free action).

**4. Budget cap.**
- Specialised items (radiation meters, Faraday storage, potassium iodide) default to **$0**.
- An opt-in allows at most 10% of the monthly budget.
- Potassium iodide is shown only as "take only on official instruction" [UNVERIFIED medical detail].

**5. EMP.** Explain that the effects are highly uncertain. Recommend nothing specific beyond general supplies and a battery radio.

**6. Same treatment for other low-probability, high-consequence scenarios:** asteroid, supervolcano, multi-state months-long grid collapse.

---

## 7. Uncertainty, communication, calibration

### 7.1 Natural frequencies

**Formula:** N per 100 = 100·(1 − e^(−T·Λ)).

**Rounding:**

| Value | Say |
|---|---|
| Under 1 | "fewer than 1 in 100" |
| 1–10 | a whole number |
| Above 10 | the nearest 5 |

**Always give a range:** "about 9 (5–15) in 100 households like yours will lose power for 3 days or more in the next 10 years."

**Offer a 10-year horizon by default,** with 1-year and 30-year toggles.

### 7.2 Anti-false-precision rules

- No decimals on days above 10.
- At most 2 significant figures on any rate.
- Durations snap to the ladder in §3.6.
- **Every number is paired with a range and a provenance tag** (SRC / DERIVED / PRIOR / CLIMATE-ADJ / USER).
- **Hide EAL dollars from the household view.** Put them behind "community context", with FEMA's disclaimer [SRC S5].
- Use words plus numbers: "unlikely (about 1 in 50 over 10 years)".
- Show only *relative* comparisons between hazards if the absolute number is a PRIOR with more than a 3× range.
- A "confidence badge" per bucket:

  | Badge | Meaning | Example |
  |---|---|---|
  | Data-based | built from data | Philadelphia power: EIA and EAGLE-I |
  | Mixed | part data, part prior | |
  | Expert estimate | prior only | medication disruption |

### 7.3 Provenance and "show your work"

**Every number has a drawer.** It contains:
- the formula;
- the inputs, each with a tag and citation;
- the dataset version and retrieval date (FEMA requires this [SRC S5]);
- the P10–P90 range;
- the top sensitivity driver.

**Priors live in a public registry.** Each entry records who, why, when, and how to override.

### 7.4 Sensitivity displays

- **Dial slider:** targets update live. Show the cliff warning when one hazard dominates (§3.3).
- **Scenario toggles:** Cascadia, direct major hurricane.
- **Prior sliders** for the top two drivers.
- **"Today vs 2050."**
- **"What would change our answer":**
  - your address's flood zone;
  - whether your heat or water needs electricity;
  - your utility.

### 7.5 Calibration and validation plan

**1. Backtest the power model against EAGLE-I (2014–2025, per county).**
- Fit on the earlier years, predict the later years.
- Check that about 10% of county-years exceed the predicted 1-in-10 duration, and draw a reliability diagram across counties.
- **Pool across about 3,000 counties** to test rare-event priors that a single county's 12 years cannot see.
- *This already changed the draft.* My first-pass priors implied a 46% (Philadelphia) and 72% (Coos) chance of a power cut of ≥ 1 day in 10 years. EAGLE-I county-scale events for 2018–2025 imply about 5% and 15% for storm-class events. The revised model gives 26% and 44%. It keeps explicit priors only for event classes absent from that window: in Philadelphia, Sandy-class storms and regional grid failure; in Coos, big windstorms, utility shutoffs and Cascadia [DERIVED S10].

**2. Cross-check against EIA-861.**
- Model-implied expected outage hours per customer should match utility SAIDI with and without major event days.
- My recomputation reproduces EIA's published 11 h national and about 53 h South Carolina figures for 2024, validating the pipeline [DERIVED S9].

**3. Displacement.** Compare the model's B7 and B11 rates with Household Pulse state estimates (national 1.5%/yr) [SRC S16].

**4. Water.** Compare boil-notice durations with the Texas and Kentucky distributions [SRC S14, S15]. Add states as tracking data become available; EPA found no consistent national tracking [SRC S13].

**5. Income.** Compare with the BLS work-experience rate [SRC S25]. Use CPS gross flows for completed-spell distributions, which is a v1.1 task.

**6. Expert priors.** Structured elicitation with calibration questions (§2.6), in a versioned registry, re-run annually.

**7. Monitoring without a server.** The app is an offline PWA [project decision]. Offer an **optional, anonymous, manual** "did you have a disruption this year?" export that the project can aggregate. Collect no personal data by default.

---

## 8. Worked example A: Philadelphia family of four, renting a rowhouse

### 8.1 Inputs

**Household**
- 2 adults (both W-2 employees, "typical" income stability), a 6-year-old, and a 70-year-old on daily oral medication (not refrigerated).
- Renting a two-storey brick rowhouse with a basement.
- **Gas furnace and gas stove.** Window air-conditioning units.
- One car; one adult commutes 12 miles.
- **$60/month** budget.

**Baseline inventory (from the checklist)**
- About 2 days of food.
- One flashlight.
- Medicine refilled monthly.
- No stored water.

### 8.2 Location data used

| Item | Value | Tag |
|---|---|---|
| NRI risk | Very High (99.6) | SRC S1 |
| Top EAL hazards | Inland flooding $266M/yr; heat wave $256M/yr (almost all population loss) | SRC S1 |
| Heat-wave event-days | 11.1/yr | SRC S1 |
| Winter weather | 10.5/yr | SRC S1 |
| Ice storm | 1.4/yr | SRC S1 |
| Hurricane | 0.09/yr | SRC S1 |
| Damaging earthquake | 0.0016/yr | SRC S1 |
| Community vulnerability and resilience | SV 88, CR 48, CRF 1.54 (context only, not applied to the household) | SRC S1 |
| PECO reliability, 2024 | SAIDI 212 min (60 min without MED); SAIFI 0.97 (0.67 without MED) | DERIVED S9 |
| EAGLE-I county outage events, 2018–2025 | 49 events ≥ 0.5% of customers (spike-filtered), about 6 a year. A given home sits inside one about 0.065 times a year. Largest: June 2020 storm (5.4%; restored to 50% in 12 h, to 90% in 51 h), March 2018 nor'easter (3.8%; 13 h and 50 h), June 2025 storm (2.6%; 4 h and 50 h). | DERIVED S10 |
| PECO record outages | Sandy (850k customers) and the 2014 ice storm (715k) sit outside the EAGLE-I window, so they enter as a PRIOR "major regional" class: 0.16 events/yr across PECO × 0.15 chance of hitting a city rowhouse | SRC S36; PRIOR |
| CMRA mid-century | Days > 95°F: 3.3 → 19 (SSP2-4.5), 27 (SSP5-8.5); days > 2 in. rain: 1.5 → 2.0–2.2 | SRC S33 |

### 8.3 Risk register (ranked)

Figures are per 100 households like this one over 10 years.

| # | What could happen | Chance in 10 yrs | Typical → bad case | Bucket | Basis |
|---|---|---|---|---|---|
| 1 | One adult loses a job | **~81** | Income gap after UI ~1 month; a gap > 3 months: 13; > 6 months: 5 | B10 | SRC S25 + PRIOR |
| 2 | Someone needs an emergency department | ~2 visits/yr expected | Hours; can disrupt work | B9 | SRC S26 |
| 3 | Can't shop for ≥ 3 days (snow, runs on stores, curfews) | **~59** | 1–2 d → 5 d; pandemic 2 wk → 6 wk | B3 | PRIOR |
| 4 | Grandparent's medicine interrupted ≥ 3 days (with no buffer) | ~38 | 1.5 d → 4 d; IT or insurer outage 5 d → 3 wk | B5 | PRIOR |
| 5 | Boil-water notice ≥ 1 day | ~33 | 2 d → 6 d (**covered: gas stove**) | B2a | PRIOR; SRC S14 |
| 6 | Power out ≥ 1 day | **~26** | ≥ 3 d: ~9; ≥ 7 d: ~3 | B1 | DERIVED S9, S10 + PRIOR |
| 7 | Crash injury in the household | ~25 | Medical, plus possible lost income | B9/B10 | DERIVED S28 |
| 8 | No usable tap water ≥ 1 day | ~24 | ≥ 3 d: ~10; ≥ 7 d: ~4 | B2b | PRIOR |
| 9 | ≥ 1 day of dangerous heat or cold with no working cooling or heating | ~16 (→ 24–31 by 2050) | ≥ 3 d: ~5; heat plus blackout is especially dangerous for the 70-year-old | B4 | PRIOR; SRC S35 |
| 10 | Pandemic stay-home period | ~10 | 45 d median (2020) | B3/B10 | SRC S20; DERIVED S19 |
| 11 | Home fire (own or attached neighbour's) forces you out | ~5 | Days → months; renters often don't return | B7/B11 | DERIVED S27 + PRIOR; SRC S17 |
| 12 | Flooding at your address | **Ask:** depends on flood zone and basement. Inland flooding is the county's #1 EAL hazard | Hours of notice | B7/B11 | SRC S1 |
| — | *Rare and catastrophic (shown separately):* damaging earthquake (~1.6 in 100 per decade), multi-week grid collapse, nuclear | — | General supplies cover the sheltering phase | — | SRC S1, S21, S22 |

### 8.4 Bucket targets by dial setting

Output of the prototype [DERIVED].

| Bucket | 1-in-10 | 1-in-50 | **Default (~1-in-95)** | 1-in-500 |
|---|---|---|---|---|
| Power (B1) | 7 h | 1.6 d | **2.8 d** | 8 d |
| Tap water, no-flow or do-not-use (B2b) | 3 h | 1.4 d | **2.8 d** | 13 d |
| Boil-able notice (B2a) | — | 2.5 d | 4 d (gas stove covers it) | 9 d |
| Food and supplies (B3) | 2.8 d | 6.6 d | **9.6 d** | 31 d |
| Medicine (B5) | 1.2 d | 6.9 d | **12 d** | 39 d |
| Heat or cold without working systems (B4) | — | 19 h | **1.7 d** | 5 d |
| Communications (B6) | — | 19 h | **1.4 d** | 4.6 d |
| Income gap (B10), months | 0.4 | 2.2 | **3.8** | 9.5 |

**Readiness buckets:**

| Bucket | Result |
|---|---|
| Evacuation (B7) | 10-year need ≈ 5% (above the 2% threshold): go-bag |
| Stranded (B8) | 12 miles ≈ 4–5 hours on foot: get-home bag |
| Medical (B9) | High frequency: first-aid kit |

### 8.5 The self-sufficiency statement shown to the family

> "Be ready to manage **about 3 days** at home with no power or tap water. Your gas stove handles boil-water notices.
> Keep **about 10 days of food** you normally eat, and **2 weeks of Grandma's medicine** on hand at all times.
> Have a **cooling plan** for her for a hot day with no power.
> The commuter keeps a **get-home bag** for a 12-mile walk.
> Your biggest long disruption is **income**. About 1 in 8 families like yours will have a gap of more than 3 months within 10 years. Aim for **about 4 months** of income gap in savings over time, separate from this supplies budget."

**Tier targets:**

| Tier | What it covers |
|---|---|
| T1 (72 h) | Water about 3 days (≈ 12 gal for 4); food 3 days; medicine ≥ 7-day buffer; lights; power bank; heat and cold plan |
| T2 | Get-home bag and go-bag |
| T3 (2 weeks) | Food to about 10–14 days; medicine to 2 weeks; $100 cash; masks |
| T4 | Nothing: supply targets are met |
| T5 | Income buffer (savings track) |
| T6 | Not warranted |

### 8.6 Month 0 and the first six months

Allocator output. Prices are [ILLUSTRATIVE] and editable.

| Month | Spend | Buy / do | Why (plain language, as shown in the app) |
|---|---|---|---|
| 0 | $0 | **Free steps.** Household plan and contact cards; document photos and a zip bag; text alerts; **fill ~6 gal of reused bottles**; **learn to draw water from the 40-gal water heater** (close its inlet on a do-not-drink notice); **refill medicine when 7 days remain** and ask for 90-day fills; cooling-centre and basement plan; test smoke alarms and confirm a CO alarm; swap numbers with 2 neighbours; keep the car ≥ ½ tank; check whether you have renters insurance | These alone cover ~2.5 of the 3 water days and 7 medicine days |
| 1 | $40 | Extra 7-day medicine cushion ($10, copay-dependent); 3 extra days of shelf-stable food you already eat ($30) | "Grandma's medicine is the one thing you can't substitute." "About 6 in 10 families like yours will be unable to shop for 3+ days in 10 years." |
| 2 | $65 | First-aid kit ($30); lights: 2 headlamps, 2 lanterns, batteries ($35) | "Nationally there are about 47 ER visits per 100 people a year, about 2 a year for a family of four on average. Many small injuries don't need the ER if you have supplies." "About 1 in 4 families here will lose power for a day or more in 10 years; almost everyone gets shorter cuts." |
| 3 | $55 | Battery fans and cooling towels ($30); power bank ($25) | "A heat wave during a power cut is the most dangerous combination for Grandma." "Keeps phones alive for alerts." |
| 4 | $80 | Food to 7 days ($40); blankets or sleeping bags *if not owned* ($40) | "About 1 in 6 families face a week or more of trouble getting food in 10 years." "Your gas furnace needs electricity in an ice storm." |
| 5 | $35 | Two 5-gal water containers and unscented bleach ($35) | "Completes 3 days of water, with a buffer. About 1 in 10 families lose usable tap water for 3+ days in 10 years." |
| 6 | $70 | Get-home bag ($40); go-bag additions ($30) | "A 12-mile walk is 4–5 hours." "About 1 in 20 families here have to leave quickly (fire, flood, gas leak) within 10 years." |

**Totals:** 6 months = $345 of $360 ($15 carried over).

**Months 7–12 (preview):** KN95 masks ($20), $100 cash, food to 14 days ($70). After that, **supply targets are met**, and the app suggests either:
- routing the $60 to the income buffer (B10); or
- optional upgrades, such as a power station (~$250), or raising the dial.

**Why water comes "late":** two free steps in month 0 already cover ~2.5 of the 3 target days. A family that skips them sees water containers move to month 1. This is the kind of explanation the "why" drawer must give.

### 8.7 Sensitivity

**Dial**
- At 1-in-10 the plan stops after tier 1.
- At 1-in-500 the targets become:
  - power 8 d;
  - water 13 d (≈ 50 gal);
  - food ~1 month;
  - medicine ~5 weeks;
  - income 9.5 months.

**2050 toggle (SSP2-4.5 to SSP5-8.5)**
- Heat-with-no-cooling target: 1.7 → 2.4–2.9 d.
- Add a second cooling option.
- Icing days fall (14 → 2–4), but ice-storm confidence is low, so the cold plan stays.
- Flood-exposed addresses: inland-flood rate × 1.3–1.5.

**What would change the answer most**
- Address in a flood zone or with a flood-prone basement.
- The grandparent's medicine becoming refrigerated (insulin), which couples B5 to B1.
- An electric stove (loses boil-notice coverage).
- The major-regional-storm prior. It drives the 1-in-500 power number.

---

## 9. Worked example B: Coos Bay, Oregon homeowner with a well

### 9.1 Inputs

**Household**
- 2 adults (55 and 58, both W-2) and a dog.
- Own a 1970s single-family house on 2 acres east of Coos Bay, **outside the tsunami zone**.
- **Private well with an electric submersible pump**; gravity septic.
- Heat pump plus a **wood stove with a winter's wood**.
- Two vehicles. One adult works 8 miles away at a **waterfront job inside the tsunami inundation zone**.
- $60/month.

**Baseline inventory**
- About 5 days of food.
- One flashlight.
- No stored water.

### 9.2 Location data used

| Item | Value | Tag |
|---|---|---|
| NRI | EAL $90M/yr: earthquake $52M (damaging-shaking AFREQ 0.0196/yr; building loss ratio 5.6% per event), inland flood $22M, tsunami $11.5M (0.145/yr, mostly distant-source), coastal flood $3.8M; landslide AFREQ 12.8/yr; heat negligible | SRC S1 |
| Cascadia | "40 percent chance of a major earthquake in the Coos Bay region during the next 50 years" (≈ 1.0%/yr). Southern-margin recurrence "every 240 years or so". 19 full-margin plus 22 southern-only events in 10,000 years. | SRC S31 |
| After an M9 (coast, present conditions) | Electricity 3–6 months; drinking water and sewer 1–3 years; healthcare 3 years; relief 1–2 weeks | SRC S30 |
| Local tsunami | 15–20 minutes | SRC S32 |
| Coos-Curry Electric, 2024 | SAIDI 571 min (181 without MED); SAIFI 2.56 (1.37 without MED) | DERIVED S9 |
| EAGLE-I 2018–2025 | A given home sits inside a county outage event about 1.1 times a year. Largest long events: 2022-01-03 (35% of customers; restored to 50% in 10.5 h, to 90% in 31 h) and 2022-12-27 (18%; 13.5 h and 33 h). Some 27–55% "events" are spikes under an hour, filtered out. | DERIVED S10 |
| CMRA | Heat negligible even by 2050; longest dry spell 53 → 61 days; days > 2 in. rain +22% | SRC S33 |

### 9.3 Risk register (ranked)

Per 100 similar households over 10 years.

| # | What could happen | Chance in 10 yrs | Typical → bad case | Bucket |
|---|---|---|---|---|
| 1 | **Well stops because the power stops** | ≥ 1 d: **~70**; ≥ 7 d: ~25; ≥ 30 d: ~13 | Hours → days; Cascadia months | B2b (via B1) |
| 2 | One adult loses a job (older workers: longer spells) | ~8 in 10; gap > 3 months: **~21**; > 6 months: ~10 | Includes the Cascadia economic shock | B10 |
| 3 | **Cascadia M9 earthquake** | **~10** | Everything at once: power 3–6 months, isolation 1–2 weeks, healthcare disrupted for years | All |
| 4 | Power out ≥ 1 day | ~44 including Cascadia; ~38 without. EAGLE-I 2018–2025 storms alone give ~15; the rest is the PRIOR for rarer big windstorms and utility shutoffs. | Storms: hours → 1.5 d; big windstorm P90 5 d (PRIOR) | B1 |
| 5 | Well-pump failure | ~50 | 2 d → 7 d | B2b |
| 6 | Can't shop ≥ 3 days | ~62 | Road closures and storms; Cascadia 3 wk → 2 months | B3 |
| 7 | Medicine interrupted ≥ 7 days (with no buffer) | ~22 | Cascadia 1 → 3 months | B5 |
| 8 | Drought lowers well yield | ~10 | 1 → 3 months | B2b |
| 9 | **Tsunami while at work in the zone** | ~3 | **15–20 minutes to walk to high ground** | B7 |
| 10 | Home fire | ~3 | Rural response is slower | B11 |
| — | *Rare and catastrophic:* nuclear; distant-source tsunami (hours of warning; affects the waterfront job) | — | — | — |

### 9.4 Targets, and the Cascadia cliff

Prototype output [DERIVED].

| Bucket | 1-in-10 | 1-in-50 | **Default (~1-in-95)** | 1-in-500 |
|---|---|---|---|---|
| Power | 14 h | 3.1 d | **13 d** | 143 d |
| Well water | 1.3 d | 14 d | **50 d** | 196 d |
| Food | 2.9 d | 9 d | **17 d** | 54 d |
| Medicine | 1.2 d | 9.3 d | **21 d** | 71 d |
| Communications | 14 h | 2.9 d | **6 d** | 37 d |
| Heat or cold | — | — | — (wood stove) | 2 d |
| Income gap, months | 0.6 | 3.5 | **5.8** | 15.5 |

**The cliff.** Water is 14 d at 1-in-50, 35 d at 1-in-75, 50 d at 1-in-95 and 83 d at 1-in-150. With a time-independent Cascadia rate (≈ 0.4%/yr) instead of the published 1.0%/yr, the default drops from **50 to 23 days**.

**The app therefore shows:**
- a **"Plan for Cascadia" toggle**, default on, citing Oregon's recommendation to prepare for "a minimum of two weeks" [SRC S30];
- the plan without Cascadia beside it: about 3 days of power, about 9 days of food and medicine, and about 2 weeks of water (the well plus a drought prior; about 1 week without drought) [DERIVED, prototype].

### 9.5 Statement shown to the household

> "Your well pump runs on electricity, so **every power cut is a water cut**.
> Plan for **about 2 weeks** at home without power, and **about 7 weeks** without well water. Do that as **stored water for the first 2–4 weeks plus a way to make rainwater drinkable**, not as 120 gallons in the garage.
> Keep **~3 weeks of food and medicine**.
> **At work: if the shaking is long or strong, walk uphill immediately.** You have about 15–20 minutes. Don't wait for an alert and don't drive.
> Expect **1–2 weeks before outside help** arrives after a Cascadia quake.
> Your biggest long disruption is again **income**, about 6 months, which here includes the regional economy after a quake."

### 9.6 Month 0 and the first six months

Allocator output. Prices are [ILLUSTRATIVE].

| Month | Spend | Buy / do | Why |
|---|---|---|---|
| 0 | $0 | Tsunami route from work (walk uphill at once), practised twice a year; documents; alerts; **fill ~6 gal of bottles**; **know how to draw water from the 50-gal water heater and switch off the pump breaker**; medicine refill-at-7 and 90-day fills; wood stove counts as heat; vehicles ≥ ½ tank; check-on-each-other agreement with 2 neighbours | Covers ~6 water-days and 7 medicine-days |
| 1 | $60 | **Strap the water heater and secure tall furniture** ($40, life-safety); medicine cushion toward 30 days ($20) | "In a quake, a strapped tank is 50 gallons of drinking water; an unstrapped one is a gas leak." |
| 2 | $55 | Bucket-toilet kit ($30); power bank ($25) | "No well water means no flushing." |
| 3 | $40 | First-aid and trauma basics ($40) | "After a big quake, ambulances may be days away." |
| 4 | $45 | Two 7-gal water jugs and bleach ($45) | "Takes you from ~6 to ~12 days of water." |
| 5 | $90 | **55-gal water drum and siphon** ($90) | "About 3 weeks of water for two people and a dog. About 1 in 4 homes like yours lose well water for a week or more in 10 years." |
| 6 | $55 | Lights ($35); 3 more days of food, including dog food ($20) | "Winter storms bring multi-hour outages about once a year." |

**Totals:** 6 months = $345 ($15 carried over).

**Months 7–12:** food to 2 weeks ($60); crank/solar radio ($30); **get-to-high-ground bag at work** ($40); go-bag ($30); **gravity filter plus rain-barrel diverter** ($130, month 11). The filter and diverter extend water from ~30 days to "as long as it rains" (Coos gets ~70+ in./yr [SRC S33]; summers are dry, so keep the drum full).

**Flagged big-ticket items** (separate saving; the app explains, doesn't push):
- a manual well hand pump, or a generator with transfer switch [ILLUSTRATIVE $1,000–$5,000];
- **seismic retrofit for a 1970s house** (foundation bolting and cripple-wall bracing) [ILLUSTRATIVE $3,000–$7,000; check state programs, UNVERIFIED];
- an earthquake-insurance decision. The NRI building loss ratio is 5.6% per damaging quake [SRC S1]; deductibles are typically high [UNVERIFIED].

### 9.7 What changed between the two households, and why

| | Philadelphia | Coos Bay |
|---|---|---|
| Dominant supply bucket | Food and medicine (frequent short disruptions) | **Water** (well coupling + Cascadia) |
| First paid item | Medicine cushion, food | **Water-heater strapping** (quake life-safety) |
| Readiness | Get-home bag (12-mile walk) | **Tsunami evacuation from work** (minutes) |
| Default horizon | 3 days to 2 weeks | **2 weeks to 7 weeks** (Cascadia) |
| Long tail handled by | Savings | **Capability** (filter + rain) plus savings |
| Diminishing returns (days 0–3 vs 30–33 of water) | ~60× | **~19×**, so going deep is worth it |
| Months | Income only | Income **and** water (Cascadia) |

### 9.8 Short note: how a Miami-Dade condo differs (not modelled)

**Hazard data**
- Hurricane AFREQ 0.305/yr [SRC S1].
- FPL 2024: SAIDI 702 min with major events vs 46 min without (Milton) [DERIVED S9]. Routine reliability is excellent; major events dominate.
- Days > 95°F projected at 1.4 → 25–46 by mid-century [SRC S33].

**Coupling rules**
- **High-rise:** water above the booster-pump floors and elevators depend on power, and generator coverage varies by building [UNVERIFIED generalisation]. So B2b and mobility inherit B1.
- **Heat:** B4-heat becomes the dominant life-safety bucket during outages.

**Evacuation (B7)**
- Evacuation zones give **days of notice**, so the readiness plan is about a *destination and transport*, not a bag within minutes.

**Money (B11)**
- Condo association finances and special assessments, plus flood insurance for contents.

**Expected default output:** roughly 1–2 weeks for power and water, with heat mitigation at the top of tier 1.

---

## 10. Data gaps and the build pipeline

### 10.1 Biggest gaps, ranked by effect on answers

**1. How long a given household's outage lasts.**
- EIA publishes averages only.
- EAGLE-I gives county totals every 15 minutes, with spikes, gaps and changing coverage [SRC S10].
- OE-417 records events, not customers.
- *Mitigation:*
  - the EAGLE-I restoration-curve method, per county and event class;
  - pooled regional tails for rare classes;
  - validation against EIA-861 SAIDI;
  - ask state utility commissions for customers-experiencing-long-interruption metrics where reported [UNVERIFIED availability].

**2. Water outages at the household level.**
- No national boil-notice or outage database exists [SRC S13].
- Only state datasets exist (Texas, Kentucky), and even they don't record the share of customers affected.
- *Mitigation:*
  - priors by system size and source, using SDWIS system attributes;
  - ingest state feeds as they appear.

**3. Footprint fractions** (P(household hit | county event)) for everything except power.

**4. Societal hazard rates:** grid collapse, cyber outages of payments or pharmacies, unrest, supply-chain shocks. There is no actuarial base. Use structured elicitation (§2.6).

**5. Rare events outside short records.**
- Sandy-class storms and PECO's ice storms predate EAGLE-I.
- *Mitigation:* use OE-417 (2000 onward), utility histories and regional pooling.

**6. Income.**
- Completed-spell unemployment distributions by age and occupation (CPS flows).
- State UI rules, which are ingestible [UNVERIFIED format].

**7. Medication and pharmacy disruption rates and durations.**

**8. Address-level lookups** (flood zone, tsunami zone) without sending the address anywhere.
- Ship tiles to the client, or ask a yes/no question.

**9. Unstable hosting.**
- NRI moved into RAPT.
- The Future Risk Index was removed.
- NCA5 went offline.
- CMRA is served from ArcGIS services whose names end in "_Dev".
- *Mitigation:* snapshot, hash, version, and cite retrieval dates [SRC S5, S6, S34].

**10. Prices.** Regional, drifting, user-editable.

### 10.2 Offline data pack (built by CI, shipped with the PWA)

| Input | Processing | Output per county |
|---|---|---|
| NRI v1.20 county and tract CSVs [S1] | Keep AFREQ, EAL and HLR by hazard; version, retrieval date, FEMA disclaimer | Hazard frequencies; screening flags |
| EAGLE-I 2014–2025 [S10] | Spike filter (3-point rolling median; ≥ 30 min); gap flags; modelled-county-customer (MCC) denominators; event catalogue with peak fraction, time to 50% and 90% restored, customer-hours | Household event rate; lognormal mixture per event class |
| EIA-861 Reliability + Service Territory [S9] | Utility SAIDI and SAIFI with and without major events; utility-to-county map | Utility picker; routine and major-event scaling |
| CMRA LOCA2 county [S33]; FRI multipliers [S6] | Mid-century deltas and ranges under SSP2-4.5 and SSP5-8.5 | Climate toggle |
| Oregon Resilience Plan and other state resilience plans [S30] | Scenario restoration ranges → lognormal | Named scenarios (Cascadia; also New Madrid and a SoCal scenario, to be sourced) |
| BLS, DOL, Household Pulse [S16, S23–S25] | Income priors by state; displacement calibration | Income and displacement parameters |
| PRIOR registry (YAML) | Elicitation metadata, ranges, rationale | Every non-data parameter |

A county pack is a few KB, so the national pack is a few MB [PRIOR estimate]. That fits the offline PWA constraint.

---

## 11. Reference implementation sketch

The runnable prototype is in `rm_proto/`. The core is below.

```python
Z90 = 1.2815516

def S(d, med, p90):                   # lognormal survival P(D > d), from median and P90
    s = (ln(p90) - ln(med)) / Z90
    return 0.5 * erfc((ln(d) - ln(med)) / (s * sqrt(2)))

def Lam(events, d):                   # exceedance rate: disruptions per year longer than d days
    return sum(e.rate * e.q * S(d, e.med, e.p90) for e in events)

def target(events, dial_rate):        # dial_rate = 1/N, or -ln(c)/T; solve Lam(d*) = dial_rate
    return bisect_log(lambda d: Lam(events, d) - dial_rate, 1e-4, 3650)

def value(bucket, x0, x1, w, T=10):   # expected weighted disruption-days covered per decade
    return T * w * integrate(lambda t: Lam(bucket.events, t), x0, min(x1, bucket.target))

def natural_frequency(events, d, T=10):
    return 100 * (1 - exp(-T * Lam(events, d)))
```

**Outer loop:** sample the parameters about 2,000 times and report the P10, median and P90 of each target.

**Allocator:** see §4.2.

**Income bucket:** `rm_proto/income.py`.

---

## 12. Sources

All accessed 2026-09-25 unless noted. Items "via summary" were read through a search-engine or abstract summary, not the full text.

- **S1** FEMA National Risk Index v1.20 ("December 2025") county feature service, `National_Risk_Index_Counties` (owner FEMA_NationalRiskIndex): https://services.arcgis.com/XG15cJAlne2vxtgt/arcgis/rest/services/National_Risk_Index_Counties/FeatureServer/0. Retrieved 2026-09-25. *This product uses the Federal Emergency Management Agency's National Risk Index dataset API or downloadable datasets but is not endorsed by FEMA.*
- **S2** FEMA NRI Data Glossary (Dec 2025, v1.20), ArcGIS item 6835fa58ad5646878860145288c7cc39: https://www.arcgis.com/sharing/rest/content/items/6835fa58ad5646878860145288c7cc39/data
- **S3** FEMA NRI Technical Documentation, Ch. 3 "Risk Analysis Overview" (March 2023), as reproduced in https://missionknox.org/wp-content/uploads/2025/05/Appendix_4.3-FEMA_Scoring_Matrix-1.pdf. The full technical documentation is at https://www.fema.gov/sites/default/files/documents/fema_national-risk-index_technical-documentation.pdf (over 10 MB; not retrieved).
- **S4** FEMA NRI Data Version and Update Documentation (Dec 2025): https://www.fema.gov/sites/default/files/documents/fema_national-risk-index_data-version-update-documentation.pdf
- **S5** FEMA NRI Metadata (Dec 2025), including terms of use and citation rules, ArcGIS item e77b087b362f42d68ddbdbcab1c29d00.
- **S6** FEMA NRI Future Risk Technical Documentation (Dec 2024), hosted by Harvard EELP: https://eelp.law.harvard.edu/wp-content/uploads/2025/03/NRI_Future_Risk_Technical_Document.pdf. Removal tracker: https://eelp.law.harvard.edu/tracker/rollback-fema-removed-future-risk-index/. Data mirror: https://screening-tools.com/fema-future-risk
- **S7** FEMA CPG 201, THIRA/SPR Guide, 3rd ed. (May 2018): https://www.fema.gov/sites/default/files/2020-04/CPG201Final20180525.pdf
- **S8** EIA, "Hurricanes in 2024 led to the most hours without power in the United States in 10 years", Today in Energy (Dec 1, 2025): https://www.eia.gov/todayinenergy/detail.php?id=66744
- **S9** EIA-861 Annual Electric Power Industry Report, 2024 data (Reliability_2024.xlsx in f8612024.zip): https://www.eia.gov/electricity/data/eia861/. Utility and state figures are my calculations (IEEE-standard reporters, customer-weighted).
- **S10** Brelsford C. et al. (2024), "A dataset of recorded electricity outages by United States county 2014–2022", *Scientific Data* 11:271, doi:10.1038/s41597-024-03095-5. Data: figshare doi:10.6084/m9.figshare.24237376 (now 2014–2025, updated 2026-02-25). Event analyses for Philadelphia, PECO-area and Coos counties are my calculations.
- **S11** Do V. et al. (2023), "Spatiotemporal distribution of power outages with climate events and social vulnerability in the USA", *Nature Communications* 14:2470, doi:10.1038/s41467-023-38084-6 (PMC10147900).
- **S12** Kar B. et al. (2022), *RePOWRD: Restoration of Power Outage from Wide-area Severe Weather Disruptions*, ORNL/TM-2022/2621: https://info.ornl.gov/sites/publications/Files/Pub183672.pdf
- **S13** US EPA (2024), *National Occurrence and Causes of Boil Water Advisories in the United States*, Report to Congress, EPA 810-R-24-003: https://www.epa.gov/system/files/documents/2025-01/10586_boil-water-advisories_final_rtc_20240603_admin.pdf
- **S14** Shaffer M., Awad N., Davenport F., Fakhreddine S. (2026), "Evaluating the Resilience of Drinking Water Systems to Severe Weather Events Using Boil Water Notices and Bottled Water Sales", *Environmental Science & Technology* 60(31):21648–21660, doi:10.1021/acs.est.5c18579 (PMC13472146).
- **S15** "Public Water Service Disruptions: A Descriptive Analysis of Boil Water Advisories", *Water* 16(3):443 (2024), doi:10.3390/w16030443. Kentucky figures via summary; full text not accessed.
- **S16** Aung T.W., Sehgal A.R. (2025), "Prevalence, Correlates, and Impacts of Displacement Because of Natural Disasters in the United States From 2022 to 2023", *AJPH* 115(1):55–65, doi:10.2105/AJPH.2024.307854.
- **S17** Census Household Pulse Survey displacement results, as reported by Axios (Jan 6, 2023): https://www.axios.com/2023/01/06/adults-displaced-natural-diasters-survey; and NLIHC: https://nlihc.org/resource/new-data-household-pulse-survey-suggest-disparities-among-households-displaced-disasters (via summary).
- **S18** Moreland A. et al. (2020), "Timing of State and Territorial COVID-19 Stay-at-Home Orders and Changes in Population Movement — United States, March 1–May 31, 2020", *MMWR* 69(35):1198–1203: https://www.cdc.gov/mmwr/volumes/69/wr/mm6935a2.htm
- **S19** Wikipedia, "U.S. state and local government responses to the COVID-19 pandemic" (state order dates). Durations are my calculation for 39 states; cross-check against the CUSP database before release.
- **S20** Marani M., Katul G.G., Pan W.K., Parolari A.J. (2021), "Intensity and frequency of extreme novel epidemics", *PNAS* 118(35):e2105482118: https://pmc.ncbi.nlm.nih.gov/articles/PMC8536331/
- **S21** Forecasting Research Institute (Oct 29, 2024), "Can Humanity Achieve a Century of Nuclear Peace?": https://forecastingresearch.org/research/nuclear-risk. Annualisation is my calculation.
- **S22** Ready.gov, Water: https://www.ready.gov/water. Ready.gov, Nuclear Explosion: https://www.ready.gov/nuclear-explosion
- **S23** BLS, Employment Situation Table A-12, Unemployed persons by duration (Aug 2026): https://www.bls.gov/news.release/empsit.t12.htm
- **S24** BLS, Displaced Workers Summary (released Aug 27, 2026): https://www.bls.gov/news.release/disp.nr0.htm
- **S25** BLS, Work Experience of the Population — 2024 (released Jan 22, 2026): https://www.bls.gov/news.release/work.nr0.htm
- **S26** CDC NCHS FastStats, Emergency Department Visits (NHAMCS 2022): https://www.cdc.gov/nchs/fastats/emergency-department.htm
- **S27** US Fire Administration, Residential fire statistics (2023 data): https://www.usfa.fema.gov/statistics/residential-fires/
- **S28** NHTSA Traffic Safety Facts, 2023 data, overview (6,138,359 crashes; 40,901 killed; 2,442,581 injured): https://crashstats.nhtsa.dot.gov/ (via summary).
- **S29** Federal Reserve, *Economic Well-Being of U.S. Households in 2024* (May 2025): https://www.federalreserve.gov/publications/2025-economic-well-being-of-us-households-in-2024-executive-summary.htm
- **S30** Oregon Seismic Safety Policy Advisory Commission, *The Oregon Resilience Plan* (Feb 2013). Executive summary: https://www.oregon.gov/oem/documents/oregon_resilience_plan_executive_summary.pdf. Coastal Communities chapter: https://www.oregon.gov/oem/Documents/03_ORP_Coastal_Communities.pdf
- **S31** Oregon State University press release on Goldfinger et al. (2012), USGS Professional Paper 1661-F, via phys.org (Aug 2012): https://phys.org/news/2012-08-year-cascadia-northwest-earthquake.html
- **S32** DOGAMI, Oregon Tsunami Clearinghouse FAQ: https://www.oregon.gov/dogami/tsuclearinghouse/pages/faq-tsunami.aspx (15–20 minutes; via summary).
- **S33** Climate Mapping for Resilience and Adaptation (CMRA), Climate Assessment Data (2025 version), county layer `CMRA_Tool_Dev`: https://services3.arcgis.com/0Fs3HcaFfvzXvm7w/arcgis/rest/services/CMRA_Tool_Dev/FeatureServer/0. Retrieved 2026-09-25.
- **S34** Marvel K. et al. (2023), NCA5 Chapter 2 "Climate Trends", read from the unofficial unaltered mirror https://nca5.climate.us/chapter/2 (the original nca2023.globalchange.gov does not resolve). Offline status: https://eelp.law.harvard.edu/tracker/noaa-released-the-fifth-national-climate-assessment-nca5/
- **S35** Stone B. Jr. et al. (2023), "How Blackouts during Heat Waves Amplify Mortality and Morbidity Risk", *Environmental Science & Technology* 57(22):8245–8255, doi:10.1021/acs.est.2c09588 (via summary).
- **S36** Philadelphia Inquirer, "Power outages in Philadelphia history: Peco's biggest storms" (2025): https://www.inquirer.com/weather/power-outages-peco-most-history-20250626.html (via summary).
- **S37** US EPA news release, "City of Asheville lifts systemwide boil water notice issued after Hurricane Helene": https://www.epa.gov/newsreleases/city-asheville-lifts-systemwide-boil-water-notice-issued-after-hurricane-helene; Spectrum News (Nov 18, 2024) (via summary).
- **S38** Martell M., Miles S., Choe Y. (2020), "Modeling of Lifeline Infrastructure Restoration Using Empirical Quantitative Data", arXiv:2008.00991. Notes the scarcity of empirical restoration data.
- **S39** Mell H.K. et al. (2017), "Emergency Medical Services Response Times in Rural, Suburban, and Urban Areas", *JAMA Surgery*, doi:10.1001/jamasurg.2017.2230. Cited for the B9 rural modifier; figures UNVERIFIED in this pass.
- **S40** The Open Group, Open FAIR Risk Taxonomy (O-RT) and Risk Analysis (O-RA) standards: https://pubs.opengroup.org/security/o-ra/; FAIR Institute terminology: https://www.fairinstitute.org/blog/fair-terminology-101-risk-threat-event-frequency-and-vulnerability (via summary).
- **Hazus** FEMA, HAZUS-MH MR4 Earthquake Technical Manual, §8.1.7 and Tables 8.1.a–c (restoration functions after ATC-13, 1985), mirror: http://www.civil.ist.utl.pt/~mlopes/conteudos/DamageStates/hazus_mr4_earthquake_tech_manual.pdf. Current version: Hazus 6.1 Earthquake Technical Manual (July 2024), https://www.fema.gov/sites/default/files/documents/fema_hazus-earthquake-model-technical-manual-6-1.pdf (not retrieved).
- **ISO 31000:2018**, *Risk management — Guidelines*: https://www.iso.org/standard/65694.html (page blocked; process described from general knowledge; UNVERIFIED).
