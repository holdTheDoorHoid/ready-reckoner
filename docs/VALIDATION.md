# How well do these numbers hold up? The frozen backtest

We checked the targets against 22 real disasters. This file is the pre-registered test: the
events, the household used for each, what happened, the rule that scores a target, and the
verdicts recorded for each engine version. It is frozen before each data refresh (see
[Refreshing](#refreshing)), so a change that fixes or breaks a real case shows up as a changed
verdict, and the misses stay on the record. The public `#/validation` page and the packet's
sources section summarise it; `EngineInfo.validation` carries the tally.

Run it with `cargo test -p rr-consequence --test backtest` (the table goes to
`target/backtest.md`); `rr validate` prints the same table from the CLI (plan workstream). The
test fails when a verdict changes.

## The scoring rule

Defined in the round-2 model review (Part 2) before the events were scored, and kept:

- **covered**: the 1-in-100 target is at least the duration of about 9 in 10 affected households;
- **partial**: it covers the median affected household but not the tail;
- **short**: it is below the median;
- **over**: it is more than three times the event (counted as covered);
- **not modelled**: the model has no consequence for it.

An event with several needs takes the worst of them. For an evacuation, the target is covered
when the household is told to keep a go-bag (a ten-year chance of 2 in 100 or more), the warning
it is told to plan for from the event's own hazard is no longer than the warning people had, and
the time away it plans for reaches the median evacuee's; partial when that time away is within a
factor of three; short otherwise.

Targets are read at the 1-in-100 setting (the default). "Median" and "9 in 10" are the days until
about half and about nine in ten of the affected households had service back, from the sources
below; where a source gives only one number or a description, the other is an estimate and is
marked.

## The households

`fixtures/backtest/*.json`, made by the model review (`round2/inputs/backtest/make_households.py`):
two adults (one commuting), a child, a detached home on city water with gas heat and central air,
$100 a month, the default dial; only what the event needs changes (a heat pump in Austin, a well
in rural Buncombe, a basement flat in Queens, a retired couple on daily prescriptions in Paradise,
a 20th-floor flat in Manhattan, a single parent on SNAP at $10 a month). Planning date 2026-10-01.
The county records, locations and national rates are frozen with them
(`crates/rr-consequence/tests/data/backtest/counties.json`).

**The v2 answers.** Contract v2 asks questions the review's households could not answer. The
third and fourth runs give the answers a household there would have given before the event:

| Household | Answer | Why |
| --- | --- | --- |
| `ida-queens-basement-2` | someone sleeps below street level | the flat is on floor −1 |
| `snap-philadelphia-3` | relies on SNAP or WIC | the household is on SNAP |
| `jackson-hinds-3` | water system: frequent problems | a month without water in February 2021, hundreds of boil notices before 2022 |
| `helene-asheville-3` | water system: frequent problems | the December 2022 freeze left more than 38,000 customers without water for about 10 days (the city's Independent Review Committee, June 2023): the review's "out for more than a week in ten years" |
| `maria-sanjuan-3`, `maria-utuado-3` | water system: frequent problems | the island's system had chronic problems before 2017 (as the `san-juan-2` fixture answers) |

Those runs also carry stand-in rates for the new ranked hazards that `rr-hazards` does not emit
yet (medicine shortage 5 in 100 per person on a daily prescription, benefit lapse 1 in 13 a year,
from `round2/reports/hazard-candidates.csv`); its own rates replace them when it does.

## The events

\* in-sample: the event is inside the records the model learned from.

| # | Event | Household | Needs scored: median / 9 in 10 (days) | Sources | In-sample |
| --- | --- | --- | --- | --- | --- |
| 1 | Winter Storm Uri, Austin, Feb 2021 | `uri-austin-3` | power 2 / 4; heat or cold 2 / 4; boil water 6 / 6 | UT Austin timeline (blackouts 15-18 Feb); `kut_austin_boil_2021` (boil notice 17-23 Feb). Power median and tail are estimates from the blackout's four days | \* the storm is in the 2014-2025 outage records, and the cold-emergency class exists because of it |
| 2 | Winter Storm Uri, Houston, Feb 2021 | `uri-houston-3` | power 2 / 4; heat or cold 2 / 4 | UT Austin timeline (estimates as above) | \* outage records |
| 3 | Hurricane Helene, Asheville (city water), Sep 2024 | `helene-asheville-3` | no tap water 18 / 21; boil water 52 / 52; power 7 / 14 | `avl_watchdog_water_2024` (75-80 % back on day 19), `nchn_asheville_water_2024` (90-95 % on day 21), `epa_asheville_boil_notice_2024` (notice lifted 18 Nov); BPR (9,338 Buncombe customers out on day 17). The review's "3 to 5 weeks" was read from a looser summary; power median is an estimate | \* outage records; the county's five-year violation record (0.94) is mostly after Helene (it was clean before, data audit §6.1) |
| 4 | Hurricane Helene, rural Buncombe (well), Sep 2024 | `helene-buncombe-well-2` | no tap water 7 / 21 | review ("wells down while power was out: 1-3 weeks"), estimate | \* outage records |
| 5 | Hurricane Ida, Jefferson Parish, Aug 2021 | `ida-jefferson-3` | power 8 / 18 | Entergy 16 Sep 2021 (2 % of the parish out on day 18); median an estimate | \* outage records |
| 6 | Ida remnants, Queens basement flat, Sep 2021 | `ida-queens-basement-2` | leave home: flooding, 15 minutes' warning, 7 days away | NYC Health; Scientific American (11 drowned in basement homes); warning and time away are estimates | |
| 7 | Camp Fire, Paradise, Nov 2018 | `campfire-paradise-2` | leave home: wildfire, 1.5 hours' warning, 180 days away | NIST TN 2252 (ignition 6:29, first order 8:03, 95 % of the town burned); time away an estimate (months to years) | |
| 8 | Lahaina fire, Maui, Aug 2023 | `lahaina-maui-3` | leave home: wildfire, 15 minutes' warning, 180 days away | USFA preliminary after-action report; warning minutes and time away UNVERIFIED | |
| 9 | Jackson water crisis, Aug-Sep 2022 | `jackson-hinds-3` | no tap water 7 / 10; boil water 48 / 48 | `npr_jackson_water_restored_2022` (service lost from 29 Aug, restored by 7 Sep), `npr_jackson_boil_2022` (notice late July to 15 Sep); the 10-day tail is UNVERIFIED | \* the violation record's window includes 2022 (earlier violations and EPA's 2020 emergency order came first) |
| 10 | East Palestine derailment, Feb 2023 | `eastpalestine-3` | leave home: chemical release, 1 hour's warning, 5 days away | Ideastream (evacuation 3-8 Feb) | |
| 11 | Colonial Pipeline, May 2021 | `colonial-gwinnett-3`, `colonial-mecklenburg-3` | fuel at the pump | WBTV, CNBC | not modelled |
| 12 | Change Healthcare (Feb 2024) and CrowdStrike (Jul 2024), Franklin County OH | `pharmacy-franklin-oh-2` | medicine 7 / 15 | UnitedHealth (e-prescribing and claims back on day 15); CRS (CrowdStrike, days); median an estimate | |
| 13 | Hurricane Maria, San Juan, Sep 2017 | `maria-sanjuan-3` | power 84 / 170; no tap water 68 / 150 | EIA, DOE situation reports (`doe_maria_situation_reports`: 8 % still out on day 175), `kishore_2018_maria` (households averaged 84 days without power, 68 without water; the means stand in for the medians, read from a summary, UNVERIFIED); the water tail is an estimate | \* the island grids' major-hurricane class is built from Maria's own restoration record |
| 14 | Hurricane Maria, Utuado, Sep 2017 | `maria-utuado-3` | as San Juan (the island's figures; Utuado was among the last restored) | as above | \* as above |
| 15 | SNAP lapse, Philadelphia, Nov 2025 | `snap-philadelphia-3` | food 12 / 12 | Maine DHHS (allotments suspended from 1 Nov), Axios (full payments after 12 Nov) | |
| 16 | Superstorm Sandy, Staten Island, Oct 2012 | `sandy-statenisland-3` | power 5 / 13 | CBS New York (Con Ed's last restorable customers about 2 weeks after); median an estimate | |
| 17 | Superstorm Sandy, Long Beach NY, Oct 2012 | `sandy-longbeach-2` | power 10 / 13; no tap water 13 / 13 | `cbs_long_beach_water_2012` (water and sewers back 11 Nov, day 13; 90 % of power on 11 Nov); power median an estimate | |
| 18 | Northeast blackout, Cleveland, Aug 2003 | `blackout-cleveland-3` | power 1.5 / 2; boil water 3 / 3 | `cleveland19_blackout_2003` (three-day boil advisories); power days UNVERIFIED | |
| 19 | Northeast blackout, Manhattan 20th floor, Aug 2003 | `blackout-manhattan-highrise-2` | power 1.2 / 2; no tap water 1.2 / 2 | Baruch NYCdata (about 29 hours; up to 4 days in some neighbourhoods) | |
| 20 | Ice storm, Oklahoma City, Oct 2020 | `icestorm-okc-3` | power 5 / 10 | weather.com, KFOR (40,000 of about 370,000 still out on day 10); median an estimate | \* outage records |
| 21 | Ice storm, Austin, Feb 2023 | `uri-austin-3` | power 3 / 7 | Austin American-Statesman (30 % out at peak); end date UNVERIFIED, both figures estimates | \* outage records |
| 22 | Derecho, Linn County IA, Aug 2020 | `derecho-linn-3` | power 4 / 8 | City of Cedar Rapids (about 90 % restored in 8 days, all by day 17-18); median an estimate | \* outage records |

## Results

**Tally** (22 events):

| Engine and data | Covered | Partial | Short | Not modelled |
| --- | --- | --- | --- | --- |
| v0.1.0, as the review scored it | 6 | 5 | 10 | 1 |
| v0.2 as merged before tier 1 (`3c46b9b`), this rule | 6 | 5 | 10 | 1 |
| This version, today's data pack (county-only model) | 7 | 5 | 9 | 1 |
| This version, with the data pack v2 tables | 5 | 8 | 8 | 1 |
| This version, with the tables and the v2 answers | 7 | 8 | 6 | 1 |
| The same, with Buncombe's pre-Helene water record | 7 | 8 | 6 | 1 |

The data pack v2 tables are the regional outage model and restoration curves (`agent/data-model`)
and the drinking-water violations and smoke days (`agent/data-hazard`), frozen in
`crates/rr-consequence/tests/data/backtest/regional.json` until those branches merge. The
covered count falls with them on purpose: four power verdicts were covered only because the storm
was in the county's own record, and pooling the region's records takes that luck away.

**By event** (target at 1 in 100; before = v0.2 as merged, after = this version with the tables
and the v2 answers; the county-only and pre-event runs are in `target/backtest.md`):

| # | Event | Before | After | What changed |
| --- | --- | --- | --- | --- |
| 1 | Uri, Austin\* | short: boil water 5 d vs 6 | covered: power 5, heat or cold 5, boil water 21 d | a grid emergency in extreme cold is its own class (M-11); Austin's violation record raises the water rows (partly in-sample) |
| 2 | Uri, Houston\* | covered: power 7 d | covered: power 10 d | hurricane restoration stretched by the region's factor (1.6) |
| 3 | Helene, Asheville\* | short: no tap water 3 d, boil 5 d | short: no tap water 21 d (covered), boil 45 d (short), power 10 d (partial) | floods that shut the water plant, the boil notice after a system failure, and the water-system multiplier (M-03). With the pre-Helene record: no tap water 7 d, short |
| 4 | Helene, well\* | covered: 45 d | covered: 45 d | |
| 5 | Ida, Jefferson\* | partial: power 14 d | partial: power 10 d | hurricanes no longer counted twice (M-10) |
| 6 | Ida, Queens basement | short: flood warning 1 h | partial: flash flooding below street level, 3 minutes' warning; away 3 d vs 7 | the below-grade answer and its row (M-08, M-09); the flood rate itself is still the county's |
| 7 | Camp Fire | short: away 3 d vs months | short | time away for homes that burn needs the wildland-edge exposure (M-09) |
| 8 | Lahaina | short: away 2 d vs months | short | as Paradise |
| 9 | Jackson\* | short: no tap water 3 d, boil 7 d | short: no tap water 21 d (covered), boil 30 d (short) | as Asheville; the boil notice ran 48 days |
| 10 | East Palestine | partial: away 2 d vs 5 | partial | |
| 11 | Colonial Pipeline | not modelled | not modelled | fuel is still a gap (a free step covers it) |
| 12 | Change and CrowdStrike | partial: medicine 14 d vs 7 / 15 | covered: medicine 30 d | the medicine-shortage hazard (stand-in rate until rr-hazards emits it) |
| 13 | Maria, San Juan\* | short: power 30 d, water 7 d | partial: power 90 d; no tap water 180 d (covered) | Maria's own restoration curve for the island grid (M-10); public water fails with long power cuts (M-03) |
| 14 | Maria, Utuado\* | short: power 30 d, water 5 d | short: power 45 d, no tap water 60 d | inland, no major-hurricane scenario: the fixed 6 % major share, until rr-hazards passes the county's (about 17 %) |
| 15 | SNAP lapse | short: food 10 d vs 12 | covered: food 30 d | the benefit-lapse hazard (M-12; stand-in rate) |
| 16 | Sandy, Staten Island | partial: power 5 d | partial: power 5 d | |
| 17 | Sandy, Long Beach | short: power 7 d, water 3 d | short: power 5 d, water 5 d | a barrier-island city averaged with its county: needs ZIP-level surge exposure (M-09) |
| 18 | 2003, Cleveland | covered | covered | |
| 19 | 2003, Manhattan | covered | covered (water over) | New York City's violation record (one reservoir-cover case) raises the water rows |
| 20 | Ice storm, Oklahoma City\* | covered: power 10 d | partial: power 7 d | pooling: the storm no longer sets the county's tail alone (M-01) |
| 21 | Ice storm, Austin\* | partial: power 5 d | partial: power 5 d | |
| 22 | Derecho, Linn\* | covered: power 10 d | partial: power 5 d | pooling: without its own derecho Linn looks like its neighbours (M-01; the review's out-of-sample run gave 3 d) |

## What the misses need

- **Long Beach** (and every barrier island): the county average cannot see a city whose water
  plant and homes sit in the surge zone. The water rows for storm surge are in place; the rate
  needs the ZIP code's surge share (`agent/data-hazard`'s optional surge pack) in `rr-hazards`'
  coastal-flood rate.
- **Paradise and Lahaina**: time away after a wildfire is months where homes burn; the county's
  share of homes that burn dilutes it (the wildland-edge question and the USFS tract pack, M-09).
- **Utuado**: the major-hurricane share should be the county's own (about 17 % in HURDAT2); the
  table takes it as soon as `rr-hazards` passes the hurricane split as rate parts.
- **Boil notices of seven weeks** (Asheville, Jackson) sit beyond a 1-in-100 design for stored
  treatment supplies; a stove, bleach or a filter covers a notice of any length while gas or
  power runs, which the plan's boil-water guidance says.
- **Asheville out of sample**: the flood that broke the mains was rarer than the dial; with the
  pre-Helene record the target is 7 days. The mechanism now exists, so a more cautious setting
  reaches it.
- **Fuel** (Colonial Pipeline): no consequence yet; the half-tank free step is the mitigation.

## Refreshing

- Freeze the events, households, actuals and rule here before each data refresh.
- Add each new major event as an out-of-sample test *before* the refresh that includes its year
  (EAGLE-I adds a year every spring), and record its verdict on the old data first.
- Regenerate the frozen county inputs only with a pack version change, recorded in the commit:
  `RR_WRITE_BACKTEST=1 cargo test -p rr-consequence --test backtest -- --ignored`.

## UNVERIFIED

- Medians where the sources give only a tail or a description (marked "estimate" above).
- Jackson's 10-day tail; Lahaina's minutes of warning; how long Paradise and Lahaina residents
  were away; Cleveland's power days; the Austin 2023 ice storm's end.
- Puerto Rico's household means (Kishore et al. 2018, from a summary).
