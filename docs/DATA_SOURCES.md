# Data sources and packs

Status: maintained. Derived from `docs/research/data-sources.md` (2026-09-25) and the ETL in
`crates/rr-etl`. Every number in a pack traces to a source recorded in `data/manifest.json` (URL as
fetched, version label, retrieval time, sha256 of the raw input, licence and obligations, rows in
and out). This document explains what each pack holds, where it comes from, what we owe the
source, and the judgement calls made along the way.

Contents: 1 building and loading · 2 the packs · 3 Connecticut · 4 ZIP codes · 5 the power-outage
event definition · 6 NRI terms and disclaimer · 7 attributions · 8 privacy · 9 optional lookups ·
10 limitations and data-quality findings · 11 sources not used · 12 refresh.

## 1. Building and loading

```
cargo run -p rr-etl -- refresh --out data [--only <job>[,<job>]] [--keep-raw]
cargo run -p rr-etl -- verify --data data
cargo run -p rr-etl -- jobs
```

- **Jobs** (run order): `geography`, `nri`, `outages`, `events`, `seismic`, `climate`, `flood`,
  `facilities`, `vulnerability`, `base_rates`. Later jobs read the county list and the
  Connecticut crosswalk written by `geography`.
- **Raw inputs are not kept.** Small sources are held in memory; the 11.6 GB of EAGLE-I outage
  files are streamed from figshare and parsed on the fly; the three USGS hazard-curve grid ZIPs
  (0.9 GB for the contiguous US) are written to `data/raw/seismic/` only while they are read (a
  ZIP needs random access) and deleted straight after. `--keep-raw` keeps copies of the small
  inputs and the grid ZIPs under `data/raw/` (git-ignored); the multi-gigabyte EAGLE-I years are
  never kept.
- **Deterministic output.** Rows are sorted by key, numbers are rounded to 4 significant figures
  (coordinates to fixed decimals), and transcendental maths uses the pure-Rust `rr_types::math`,
  so an unchanged input produces a byte-identical pack. Only the manifest's timestamps change.
- **`verify`** recomputes every file's sha256 and row count against the manifest and checks that
  every county on the map exists in every county-keyed pack or is listed under that job's
  `missing` reasons, that no pack uses an old Connecticut county code, and that ZIP shares sum
  to at most 1.
- **Loading in the app.** The web app fetches `data/manifest.json`, then each file listed under
  `packs.core.files` (same origin), and passes the bytes to `load_pack(path, bytes)` with the
  path exactly as the manifest lists it (for example `core/nri_hazards.csv`). The engine checks
  each file against its sha256 and answers `pack_corrupt` on a mismatch. `geo/counties.json` is
  only needed for the map. Note for the planner: `docs/ENGINE-API.md` describes `load_pack(name,
  bytes)` with a pack name like `core`; the core pack is several files, so the name is the file's
  manifest path. A single-file bundle can be added later without changing the files.

## 2. The packs

Sizes are gzip -9 as a static host would serve them. The core pack totals **2.37 MB**
gzipped file by file (2.42 MB as one `gzip -c data/core/*` stream; budget 5 MB) and 8.8 MB
uncompressed; a first visit fetches 1.99 MB of it (everything but the two ZIP tables, which load
when a ZIP code is typed). `geo/counties.json` is **0.30 MB** gzipped (budget 0.35 MB). Until
2026-09-26 the core pack was 2.99 MB: it also shipped `zip_centroids.csv` and three NRI columns
(`expb`, `ealb`, `alrb`) that nothing in the engine read.

Licence shorthand: **PD** = US Government work, public domain (17 U.S.C. 105).

### core/counties.csv — the canonical county list (3,232 rows)

| Column | Meaning |
| --- | --- |
| `fips` | Five-digit county FIPS (Connecticut uses planning regions `09110`–`09190`) |
| `name`, `name_full` | "Philadelphia", "Philadelphia County" |
| `state_abbr`, `state_name` | |
| `lat`, `lon` | Internal point (Census Gazetteer 2024; polygon centroid for the 10 island-area counties the Gazetteer omits), 4 decimals |
| `land_sqmi` | Land area |
| `nca_region` | NCA5 region: `northeast`, `southeast`, `midwest`, `northern_great_plains`, `southern_great_plains`, `northwest`, `southwest`, `alaska`, `hawaii_pacific`, `caribbean` |

Source: Census cartographic boundary counties 2024 (1:500,000) minus three uninhabited island
units with no NRI record (60030 Rose Island, 60040 Swains Island, 69085 Northern Islands) —
exactly the 3,232 counties NRI v1.20 covers; Gazetteer 2024; NCA5 Atlas region polygons (CC0),
assigned per state by point-in-polygon. PD. Refresh: yearly (new vintage each August).

### core/states.csv (56 rows)

`state_fips`, `state_abbr`, `state_name`, `nca_region`, `coastal` (marine shoreline, including
tidal estuaries: Pennsylvania counts via the Delaware), `great_lakes` (Great Lakes shoreline). The
two shoreline flags are curated in the ETL (`STATE_FACTS`), not downloaded.

### core/ct_crosswalk.csv (19 rows) — see §3

`old_fips`, `region_fips`, `land_share_of_old`, `land_share_of_region`, `towns`.

### core/zip_county.csv (46,772 rows) — see §4

`zip`, `county_fips`, `land_share` (share of the ZIP's land in the county; parts under 0.1% are
dropped as boundary slivers). Source: Census 2020 ZCTA-to-county relationship file; Connecticut
from the Census 2022 town-to-ZCTA file. PD. Refresh: with each decennial relationship file.

### core/nri_counties.csv (3,232 rows) and core/nri_hazards.csv (45,853 rows)

| File | Columns |
| --- | --- |
| `nri_counties.csv` | `fips`, `population` (2020), `building_value_usd`, `eal_valt` (expected annual loss, all hazards, $), `sovi_score`, `resl_score`, `crf_value`, `coastal`, `tsunami_zone` |
| `nri_hazards.csv` | `fips`, `hazard` (our 18 natural hazard ids), `afreq`, `expp` (people), `ealp` (people/yr), `ealt` ($/yr), `hlrb`, `risk_score` (0–100) |

Source: FEMA National Risk Index v1.20.0 ("December 2025"), ArcGIS FeatureServer
`National_Risk_Index_Counties` (fema.gov file downloads refuse scripted clients). Terms: see §6.
Only these trimmed, rounded fields ship; the raw table is never written. Building exposure, building
loss and the building loss rate (`EXPB`, `EALB`, `ALRB`) were dropped on 2026-09-26 because no crate
reads them (`EXPB` of coastal flooding and tsunami is still read, only to set the two flags below).
`ealp` (population loss) stays although nothing reads it yet: it is the input a health-based
severity for heat and cold would use. Hazard rows where every field is empty (hazard not
applicable) are omitted; drought has no building or population fields (NRI models it for
agriculture only). `coastal` = NRI assigns coastal-flooding building exposure
(`CFLD_EXPB > 0`, 536 counties); `tsunami_zone` = tsunami building or population exposure (118
counties). Both flags are ours, derived from NRI. Refresh: when FEMA publishes a new version (the
job refuses to run on anything but v1.20 until the semantics are reviewed).

### core/nri_semantics.toml (18 entries)

What each hazard's `AFREQ` means, from the NRI Technical Documentation (December 2025, v1.20:
§5.2 Table 5 and each hazard's "Annualized Frequency" section; read from the Internet Archive copy
because fema.gov refuses scripted clients). Per hazard: `afreq_kind` (`events_per_year` or
`annual_probability`, the `rr_types::AfreqKind` strings), `basis` (`distinct_events`, `event_days`
or `modelled`), `unit`, `period_of_record`, `poisson_ok` (whether `1 - exp(-afreq)` is a sound
yearly chance) and `notes`. The traps it records:

- **Event-days, not events**, for cold wave, drought, heat wave, ice storm, inland flooding,
  lightning and winter weather: days cluster into episodes, so divide by a typical episode length
  before treating `afreq` as episodes per year (`events.csv` has episode lengths).
- **Yearly probabilities** for earthquake, wildfire (burn probability), volcanic activity and the
  tsunami surrogate. Table 5 also labels cold wave and lightning "annualized probability", but
  their section text and values (lightning 20–80) are event-days per year; we follow the text.
- **Coastal flooding** is a modelled sum including recurring high-tide flooding (often > 1 a
  year); it is not the chance of a damaging surge.
- **Inland flooding** replaced riverine flooding in v1.20 and treats about 100% of a county's
  buildings as exposed: its building loss ratio (`EALB`/`EXPB`, not shipped) is a county-wide
  average, not an in-floodplain rate.

### core/outages.csv (3,153 rows) and core/outages_state.csv (53 rows) — see §5

| Column | Meaning |
| --- | --- |
| `events_per_customer_year` | Customer outages in qualifying events, per customer per year of data |
| `p_ge_1d`, `p_ge_3d`, `p_ge_7d`, `p_ge_14d` | Share of those customer outages lasting at least 1, 3, 7, 14 days |
| `median_hours`, `p90_hours` | Median and 90th-percentile customer outage length |
| `years_covered` | First–last year with data, e.g. `2014-2025` |
| `years_of_data` | Months with at least one EAGLE-I record ÷ 12 (the rate's denominator) |
| `duration_basis` | `county`; `state` when the county has fewer than 10 events and the duration columns come from the state pool; `island` for Puerto Rico (§10) |
| `events_per_year`, `events` | County events per year and in total |
| `customers` | Customer count used (larger of ORNL MCC and households, §10); for Puerto Rico the municipio's own count, while its rates are the island's |
| `customer_hours_per_customer_year` | All recorded outage time per customer (comparable to SAIDI with major events) |
| `share_customer_hours_in_events` | How much of that time falls inside qualifying events |
| `longest_event_hours` | Longest qualifying event |

`outages_state.csv` pools the same statistics by state (customer-weighted), for small-sample
fallback. Source: ORNL EAGLE-I recorded electricity outages 2014–2025, figshare
doi:10.6084/m9.figshare.24237376 (v4, 2026-02-25), `MCC.csv`, `coverage_history.csv`; CDC SVI 2022
household counts. **CC BY 4.0: the credit line in §7 must be shown.** Refresh: yearly (a new year
is added each spring).

### core/events.csv (44,057 rows)

Long format: `fips`, `event_type`, `rate_per_year`, `share_damaging`, `median_days`, `p90_days`,
`events`, `share_injury`, `source`, `years`. A missing row means no recorded event of that type in
the period (rate 0), except where the source does not cover the place (below).

| `event_type` | Source and rule | Period |
| --- | --- | --- |
| `tropical_storm_passage`, `hurricane_passage`, `major_hurricane_passage` | NHC HURDAT2 (Atlantic, eastern and central Pacific): best track within 50 nautical miles of the county's internal point with interpolated wind ≥ 34 / 64 / 96 kt, any status | 1950–2025 |
| `tornado`, `tornado_ef2plus` | SPC severe database: tornadoes touching the county (per-state segment records, `sn = 1`, counties `f1`–`f4`); EF2+ by rating | 1996–2025 |
| `hail_1in_day`, `hail_2in_day` | SPC: days with a report of hail ≥ 1 in / ≥ 2 in in the county | 1996–2025 |
| `severe_wind_day`, `severe_wind_65kt_day` | SPC: days with any severe-wind report / ≥ 65 kt | 1996–2025 |
| `winter_storm` (Winter Storm, Blizzard, Heavy Snow, Lake-Effect Snow), `ice_storm`, `extreme_cold`, `heat`, `flood`, `flash_flood`, `coastal_flood` (incl. storm surge, lakeshore flood), `wildfire`, `high_wind`, `drought`, `tropical_cyclone_impact` | NOAA NCEI Storm Events: one observation per county per NOAA episode (`EPISODE_ID`) and type; zone records apply to every county in the NWS zone (zone-county correlation files); retired zones fall back to the county the zone is named after | 1996–2025 |

`share_damaging`: share with reported property (or crop) damage above zero — a lower bound, many
records leave damage blank. `share_injury`: direct or indirect injuries or deaths. Episode length:
earliest begin to latest end of the episode's records in the county. HURDAT2 does not cover the
western or southern Pacific (Guam, Northern Mariana Islands, American Samoa): use NRI hurricane
fields or `tropical_cyclone_impact` there. PD. Refresh: yearly.

### core/seismic.csv (3,225 rows)

`fips`, `p_pga_ge_0_1g_per_year`, `p_pga_ge_0_2g_per_year` (yearly chance of peak ground
acceleration ≥ 0.1 g / ≥ 0.2 g on firm rock, Vs30 760 m/s, at the county's internal point:
`1 - exp(-annual rate)`; each grid node's USGS mean hazard curve is read at 0.1 g and 0.2 g by
log-log interpolation, and the county value is the bilinear interpolation of the log rate between
the four surrounding nodes), `mmi6_100yr` (chance of shaking of intensity VI or more in 100 years,
USGS map with local soil, nearest 0.05° grid point within 8 km; contiguous US, Alaska and Hawaii
only), `model` (`conus-2023-grid`, `alaska-2023-grid`, `hawaii-2021.R2-grid`, `prvi-2025.R1`).

Sources, all USGS and PD:
- contiguous US and Alaska: gridded hazard curves from the 2023 50-state NSHM data release
  (doi:10.5066/P9GNPCOD; `hazard_output_CONUS.zip` 0.9 GB and `hazard_output_AK.zip` 25 MB, both
  on a 0.2° grid, about 20 km);
- Hawaii: the 2021.R2 grid from the revised release (doi:10.5066/P14VGAV4; 0.02° grid);
- Puerto Rico and the US Virgin Islands (no grid published): the NSHM web service
  (`prvi-2025`, now answering as `prvi-2025.R1`), one PGA-only call per county, one at a time;
- `mmi6_100yr`: the MMI VI 100-year data release (child of doi:10.5066/P9GNPCOD).

Why grids: the web service covers every region, but on a full run of about 3,200 calls it
rate-limited (HTTP 429) and then answered with errors. The grids are three downloads, give the
same answer every time and put no load on the service. The grid ZIPs are only on disk while they
are read. Guam, the Northern Mariana Islands and American Samoa have no USGS model. Refresh: with
each NSHM release (about every five years); see §10 for the 2026 model revision.

### core/climate.csv (3,231 rows) — projections

Every value is a **projection** for the 2050 dial. Ratio columns multiply today's frequency; each
has a central value and a `_high` value so 2050 numbers can show a range.

| Column | Definition |
| --- | --- |
| `hot_days_95f`, `hot_days_100f`, `hot_days_105f` | Days a year with a high ≥ 95/100/105 °F at +2 °C: (baseline + NCA5 Atlas change) ÷ baseline, baseline = LOCA2 1990–2019 mean; empty when the baseline is under 2 days a year |
| `warm_nights_70f` | Nights with a low ≥ 70 °F, same method |
| `freezing_nights`, `very_cold_nights_0f` | Nights with a low ≤ 32 °F / ≤ 0 °F, same method (usually < 1) |
| `wettest_day`, `wettest_day_5yr` | Rain on the wettest day of the year / of 5 years: 1 + Atlas % change |
| `extreme_rain_total`, `extreme_rain_days` | Rain on, and number of, days in the top 1% of historical amounts: 1 + % change |
| `annual_rain` | Total precipitation: 1 + % change |
| each of the above `_high` | Same at +3 °C of warming (Atlas GWL3 layer) |
| `consecutive_dry_days_mid45`, `dry_days_mid45`, `hot_days_90f_mid45`, `cooling_degree_days_mid45`, `heavy_rain_days_1in_mid45` | CMRA ensemble mean, RCP4.5 mid-century (2035–2064) ÷ historical (1976–2005); `_high` uses RCP8.5 |
| `days_over_95f_hist`, `_2050`, `_2050_high` | CMRA county count of days a year above 95 °F: historical (1976–2005, modelled), RCP4.5 and RCP8.5 mid-century |
| `days_over_2in_*`, `icing_days_*`, `dry_spell_days_*` | Same for days with more than 2 in of rain, days at or below freezing all day, and the longest dry spell |
| `days_over_90f_hist` | CMRA historical days a year above 90 °F (the heat cap `rr-hazards` applies) |

The count columns are the keys `rr-hazards` reads first (docs/RISK_MODEL.md, "Hazard rates");
ratios are its fallback. Sources: NCA5 Interactive Atlas county layers at global warming levels 2
and 3 °C (changes vs 1991–2020, SSP5-8.5 runs; **CC BY 4.0**), LOCA2 ensemble decadal county series
(**CC BY 4.0**), CMRA 2025 (NOAA / U.S. Climate Resilience Toolkit; the ArcGIS item's licence field
is blank, treated as US Government work). The Atlas 1.5 °C layer is read but not written, as
nothing uses it. Coverage: the Atlas and LOCA2 columns cover the contiguous US; CMRA covers every
state and island area, but publishes no RCP4.5 values for Alaska, so Alaska rows have only the
`_high` (RCP8.5) CMRA ratios and counts. Chugach and Copper River (Alaska, created in 2019) take the
values CMRA reports for the Valdez-Cordova Census Area they were split from; Ketchikan Gateway has
no CMRA values and is the one county missing. No county fire-weather index exists in these sources;
dry-day and hot-day ratios are the closest proxies. `rr-data::climate_multiplier` maps hazards to
the ratio columns by a documented default (`variables_for`), clamped to 0.2–5; `rr-hazards` has its
own order of preference. Refresh: when NCA6 or CMRA publish new county values.

### core/flood.csv (3,144 rows)

| Column | Meaning |
| --- | --- |
| `sfha_home_share` | Residential structures in the Special Flood Hazard Area (1% annual chance floodplain) ÷ all residential structures |
| `claims_per_1000_policies_year` | 1,000 × residential NFIP claims with a loss in 1996–2025 ÷ 30 ÷ residential policies in force today (rough: today's policy base) |
| `mean_paid_usd` | Mean building + contents + ICC payment over paid claims (nominal $) |
| `residential_structures`, `residential_structures_sfha`, `policies_in_force`, `sfha_policy_share`, `claims_per_year` | Inputs and context |
| `sfha_share_basis` | `structures`, or `policies_lower_bound` where OpenFEMA reports flood-zone policies but zero flood-zone structures (§10) |

Sources: OpenFEMA NFIP Residential Penetration Rates v1 and FIMA NFIP Redacted Claims **v3** (v2 is
withdrawn 2026-10-15). OpenFEMA terms: citation with access date and FEMA's statement (§6).
Refresh: quarterly.

### core/facilities.csv (3,232 rows) and core/zip_facilities.csv (33,791 rows)

`facilities.csv`: `nearest_nuclear_km` (from the county's internal point), `tri_facilities`
(EPA Toxics Release Inventory 2024 facilities located in the county), `high_hazard_dams` (USACE
NID dams rated High hazard potential), `nuclear_within_16km` / `nuclear_within_80km` (any part of
the county within 10 / 50 miles of an operating plant), `significant_hazard_dams`.
`zip_facilities.csv`: `nearest_nuclear_km` (from the ZIP centroid, 0.1 km), `tri_within_5km`.
ZIP centroids are the Census Gazetteer 2024 ZCTA internal points rounded to 3 decimals (≈ 110 m);
the job reads them itself and they are not shipped (until 2026-09-26 they were
`core/zip_centroids.csv`, which nothing in the engine read).
Sources: FEMA Operating Nuclear Power Plant Sites (57), EPA TRI basic data file 2024, USACE National
Inventory of Dams (nation CSV), Census 2024 1:500k county boundaries for point-in-polygon, Census
Gazetteer 2024 ZCTAs. PD.
Hazard potential rates the consequence of a failure (High: probable loss of life), not its
likelihood. Refresh: yearly.

### core/vulnerability.csv (3,144 rows)

`cre_share_3plus_risk_factors` (Census Community Resilience Estimates 2024: share of residents with
3+ of 10 risk factors), `svi_percentile` (CDC/ATSDR SVI 2022 overall national percentile, 0–1),
`households` (ACS 2018–2022 estimate as published in SVI; the engine's `households`),
`cre_share_0_risk_factors`, `cre_share_1_2_risk_factors`. PD; SVI asks for a citation (§7).
Community context only — the household model uses the household's own answers. ACS through the
Census API is not used (it needs a key).

### core/base_rates.toml (16 rates, 11 publications)

National rates for societal and personal hazards, each computed in the ETL from a quoted figure:
residential fires, fire deaths and injuries per household-year and mean loss per fire (USFA 2023 ÷
Census CPS households 2023); layoffs per worker-month and a yearly upper bound (BLS JOLTS, fetched
live); the chance of being unemployed at some point in a year (BLS work-experience unemployment
rate, 8.3% in 2024); emergency department visits, admissions and injury visits per person-year
(NCHS 2022); unintentional-injury deaths (NCHS 2024); traffic deaths per 100 million vehicle miles
(NHTSA 2025 early estimate) and injuries per person-year (NHTSA 2023 ÷ Census Vintage 2025
population, fetched live); pandemic onset per year with an exact Poisson 90% range (5 onsets in
108 years); national power interruption hours (EIA 2024). Every `[[rate]]` has `value`, `unit`,
optional `low`/`high`, `source`, `year`, `figure`, `derivation`, `note`. `source` is the citation
id from the content registry (`docs/CITATION_IDS.md`; one id per source): `census_households_cps`,
`usfa_residential_fires`, `bls_jolts_layoffs`, `bls_work_experience_2024`, `cdc_nchs_ed_visits`,
`nchs_accidental_injury_2024`, `nhtsa_crashes_2023`, `cdc_pandemic_history`,
`eia_outage_hours_2024`, and two the registry does not have yet, `nhtsa_early_estimate_2025` and
`census_popest_vintage_2025` (each `[[publication]]` gives the title, publisher and URL for the
content layer).

### geo/counties.json (3,222 features)

GeoJSON FeatureCollection; feature `id` = FIPS; properties `name`, `state`, `lat`, `lon` (internal
point, 3 decimals); geometry from the Census 2024 1:20,000,000 cartographic file, coordinates
rounded to 3 decimals, **RFC 7946 winding (exterior rings counter-clockwise)** — d3's spherical
`geoPath` expects the opposite winding, so rewind or use a planar projection. The 10 island-area
counties are not in the 1:20m file and have no polygon. PD.

## 3. Connecticut

Since 2022 the Census Bureau uses Connecticut's nine planning regions (`09110`–`09190`) as county
equivalents. NRI v1.20, the Census 2024 files, CRE 2024, SVI 2022, OpenFEMA and LOCA2 use the
regions; the 2020 ZCTA relationship file, EAGLE-I, Storm Events, SPC, the NWS zone file, the NCA5
Atlas and CMRA use the eight retired counties (`09001`–`09015`). Towns nest in both, so the Census
town-level crosswalk (`ct_cou_to_cousub_crosswalk.txt`) gives the exact overlaps, weighted by town
land area (`ct_crosswalk.csv`). Conversions: rates, shares and ratios are land-weighted averages of
the overlapping old counties; counts are split by land share. ZIP shares for Connecticut come
straight from the 2022 town-to-ZCTA file, so they point at regions. `verify` fails if any pack
contains an old Connecticut code.

## 4. ZIP codes and the ambiguity rule

ZIP codes are USPS delivery routes; the bundled crosswalk uses Census ZIP Code Tabulation Areas
(ZCTAs), which approximate them. Of 33,791 ZCTAs, 10,069 span more than one county (after dropping
slivers under 0.1% of the ZIP's land) and **4,602 have no county holding 80% of their land**. The
rule (`rr_data::AMBIGUOUS_ZIP_SHARE`): a ZIP resolves to its largest county only when that county
holds at least 80% of the ZIP's land; otherwise the engine answers `ambiguous_zip` with every
county the ZIP spans, largest share first, and the app asks the user to choose (the choice is
stored as `location.county_fips`). Shares are by land area, not addresses — the address-based HUD
crosswalk needs a login and has unclear redistribution terms. PO-box-only ZIPs have no ZCTA and
answer `unknown_zip` with suggestions from ZIPs sharing the first three digits.

## 5. The power-outage event definition (EAGLE-I)

This is the empirical duration data for the `power` bucket, so the definition is spelled out.
`OutageStats.event_definition` carries the exact text from the manifest.

**Definition.** An outage event starts when at least **1%** of the county's electricity customers
(minimum 10) are reported without power in EAGLE-I's 15-minute snapshots, needs at least **one
hour** at that level, and lasts until fewer than **0.25%** (minimum 5) remain out; dips or missing
snapshots of up to **2 hours** are bridged. Inside an event, reversals smaller than half of the
current peak or trough (or smaller than the 1% level) are treated as reporting noise. Customers are
assumed to be restored **in the order they lost power**, which splits each event's customer-hours
into individual outage lengths. Rates are customer outages in such events per customer per year of
data.

**Why this shape.** The research precedent (Do et al. 2023, *Nature Communications* 14:2470,
PowerOutage.us data) counts time with at least 0.1% of county customers out. We start higher (1%)
because we need per-customer durations: in large counties the everyday background of scattered
outages sits near 0.1%, and a threshold there turns weeks of unrelated small outages into one
"event". We end lower (0.25%) so the slow tail of a big restoration — the customers out longest,
who matter most for "days without power" — is not cut off at the start threshold. Per-customer
durations matter because county-level event length overstates what a household lives through: in
the February 2023 ice storm, Wayne County, Michigan went from 311,000 customers out to under 5,000
over eight days; most households were restored within three.

**Why the noise filter.** Scraped outage counts flicker (a map refresh drops and re-adds thousands
of customers). Without filtering, each flicker reads as customers restored and new customers out,
which inflates outage counts and truncates long outages. The filter keeps genuine second waves (a
second storm) and removes flicker; on the 2023 test year it changed customer-hours by under 2%
while restoring the multi-day tail (Wayne County: share of outages lasting 3+ days from 3% to
14%, consistent with the raw curve). Assuming last-out-first-restored instead raises the multi-day shares by up to about a third in
the hardest-hit counties, so the ordering assumption matters at the margin, not in kind.

**Denominators.** Years of data count months with at least one record (EAGLE-I lists only
snapshots with someone out); state-years in 2018–2022 with under 50% customer coverage (ORNL's
coverage history) are dropped (Montana 2018, Nebraska 2018–2022, South Dakota 2018). Customers are
the larger of ORNL's modelled count and the county's households (§10).

**Result, nationally.** Weighting counties by customers, a customer has about 0.05 outages a year
lasting at least a day and about 0.011 lasting at least three days. The median county records 5.8
hours of outage per customer per year, close to EIA's national interruption figures with major
events.

## 6. NRI terms, disclaimer text and where the app shows it

The NRI Terms & Conditions (ArcGIS item `39485e8035d446a5bff03259508ae355`) forbid reverse
engineering or deriving the underlying datasets, require a citation naming the dataset version and
access date, require the statement below, forbid presenting modified data as FEMA's, and let FEMA
rescind use. We comply by shipping only the trimmed, rounded fields the model needs, labelling
every number we derive from NRI as ours, and showing the citation and statement on the About
screen and in the packet's sources (`EngineInfo.attributions`). The statement, extracted verbatim
from the terms by the ETL on every refresh and recorded in the manifest (`jobs.nri.definitions`):

> This product uses the Federal Emergency Management Agency’s National Risk Index dataset API or
> downloadable datasets but is not endorsed by FEMA. The Federal Government or FEMA cannot vouch
> for the data or analyses derived from these data after the data have been retrieved from the
> Agency's website(s).

Citation form (with the access date filled in by the ETL): "Federal Emergency Management Agency
(FEMA), National Risk Index Dataset: National Risk Index Counties - v1.20.0 (December 2025).
Retrieved from https://fema.maps.arcgis.com/home/item.html?id=39485e8035d446a5bff03259508ae355 on
<date> (UTC)."

OpenFEMA (flood pack) requires the parallel statement: "This product uses the Federal Emergency
Management Agency’s OpenFEMA API, but is not endorsed by FEMA. The Federal Government or FEMA
cannot vouch for the data or analyses derived from these data after the data have been retrieved
from the Agency's website(s)."

## 7. Attributions the app must show

`rr_data::DataStore::attributions()` returns these from the manifest (text, URL, version, access
date): FEMA National Risk Index (citation + statement), OpenFEMA (citation + statement), ORNL
EAGLE-I (CC BY 4.0 credit), NCA5 Atlas and LOCA2 (CC BY 4.0 credit), NCA5 Atlas regions (CC0),
CMRA, USGS National Seismic Hazard Model, CDC/ATSDR SVI 2022 (requested citation), and EPA TRI /
USACE NID / FEMA nuclear sites.

## 8. Privacy: what leaves the device

| What | Leaves the device? |
| --- | --- |
| ZIP code, county, household details, budget, inventory | **No.** Resolution, search and every calculation run in the browser against bundled packs |
| Pack files | Fetched from the app's own origin; the host learns only that the app loaded (no location) |
| `geo/counties.json` | Same origin, lazy; reveals nothing about the user |
| Anything else | Nothing by default. The engine makes no network calls |

## 9. Optional on-demand lookups (a later workstream)

Neither is needed for the plan; each would sit behind a per-use consent screen naming the
recipient and what it learns.

- **Flood zone at the home (FEMA NFHL).** Request the envelope of a snapped grid cell (about 1 km)
  from the NFHL MapServer with geometry, and test the point locally, so FEMA learns only the cell.
  CORS allowed. Payload size in dense areas is untested. Without it, `flood.csv`'s county share is
  the prior.
- **Live weather alerts (NWS).** `api.weather.gov/alerts/active?zone=<zone>` by county or zone
  code (CORS allowed): reveals the county, not the address.

## 10. Limitations and data-quality findings

- **ORNL modelled customer counts are too low in 540 counties** (e.g. Mecklenburg County, NC:
  28,172 vs 446,584 households; several counties show single digits). Dividing by them would
  inflate per-customer outage rates many times over, so the outage job uses the larger of MCC and
  households. This departs from "normalised by MCC.csv" as briefed.
- **Very large counties have few qualifying outage events** (1% of Manhattan is 8,700 customers
  out at once): New York County records none in 2014–2025, so its rate is 0 and its durations
  come from the state pool. Rates there are lower bounds; pair them with the national base rate
  (`power_interruption_hours_per_customer_year`).
- **Small counties are noisy** (a few hundred customers: reporting flicker of 10 customers makes an
  event). Use the `customers` column to pool small counties with `outages_state.csv`.
- **Puerto Rico**: EAGLE-I files LUMA's outages by utility region, each under one "hub"
  municipio (the hubs' customer counts in the 2024 file sum to the island's 1.49 million). Read
  per municipio, those hubs showed up to 66 outages per customer-year. The ETL sums the hubs into
  one island-wide series and gives every municipio the island's rates, durations and event counts
  (`duration_basis = island`; `customers` stays the municipio's own count, so weighting by it
  counts the island once); data start in 2021. The US Virgin Islands are reported per district; their rates are
  high and, for St. John (about 2,700 customers), noisy.
- **EAGLE-I coverage** before 2018 is not published by state; partial utility coverage then biases
  rates low. 72 mainland counties have no usable records.
- **OpenFEMA reports zero flood-zone structures in counties with many flood-zone policies**
  (St. Tammany Parish, Queens, Brooklyn, Staten Island, Virginia Beach, Philadelphia, ...). There
  `sfha_home_share` is insured flood-zone homes ÷ all homes, a lower bound, marked
  `policies_lower_bound`. Recommendation for the contract: make `FloodPriors.sfha_home_share`
  optional or add a basis field.
- **NFIP claim rates** divide 30 years of claims by today's policy count; counties whose insured
  base changed a lot are off accordingly. Mean paid amounts are nominal dollars.
- **Storm Events** damage fields are often blank (`share_damaging` is a lower bound); 5.1% of the
  zone records used (34,015 records in 382 retired forecast zones) could not be placed in a county
  and are skipped, so rates for zone-based types (winter storm, heat, high wind, coastal flood)
  are slightly low where zones were redrawn.
- **Climate multipliers**: the Atlas and LOCA2 ratios exist for the contiguous US only. Hawaii,
  Puerto Rico and the island areas have CMRA ratios and counts; Alaska has only the RCP8.5
  (`_high`) CMRA columns, so the default mapping (`climate_multiplier`, central columns) gives 1
  there.
- **Seismic** values are for firm rock (Vs30 760 m/s); softer soils shake more (`mmi6_100yr`
  includes soil). No values for Guam, the Northern Mariana Islands or American Samoa. The
  contiguous-US and Alaska grids are the original 2023 release on a 0.2° (about 20 km) grid; the
  USGS web service now runs a 2026 revision (2023.R2) whose grid is offered only through
  ScienceBase's new file manager, which has no direct download link. Against the 307
  contiguous-US and Alaska counties the service answered before it failed, the grid values differ
  by a median of 8–9% (87–92% of counties within ±20%, at 0.2 g and 0.1 g), but they are about
  twice the revised values on the Colorado Plateau (Apache, Navajo and Coconino in Arizona; San
  Juan in Utah) and 2.5–3 times in Southeast Alaska (Wrangell, Petersburg, Ketchikan, Juneau).
  Treat shaking chances as order-of-magnitude there, and switch to the revised grid when USGS
  publishes a direct link.
- **NRI is county-scale**; a ZIP in a large county can sit in a very different flood or wildfire
  regime. Copy must say "your county" until tract packs exist.

## 11. Sources not used, or unreachable

- **fema.gov** file downloads and the NRI technical documentation answer 403 to scripted clients:
  NRI comes through ArcGIS, the documentation and the OpenFEMA terms were read from Internet
  Archive copies.
- **Census Data API (ACS)** needs a key: skipped for v1 as briefed (households come from SVI).
- **USGS hazard web service** rate-limited (HTTP 429) and then answered `{"status":"error"}`
  during a full run on 2026-09-25 (about 3,200 calls from three workers; 330 answered). The contiguous US, Alaska and Hawaii now come from the gridded data releases; the
  service is used only for Puerto Rico and the US Virgin Islands (81 small calls, one at a time).
  It has no Guam or American Samoa model (`guam-2025`, `amsam-2025` return the web app, not a
  service).
- **ScienceBase's new file manager** (where the 2026 revised NSHM grids are listed) is a
  JavaScript app with no direct file links; the classic `catalog/file/get` links work for the
  original 2023 grids and the Hawaii 2021.R2 grid, which are what the ETL uses.
- **Not bundled by policy:** GEM (non-commercial share-alike), rmpmap (share-alike), First Street,
  PowerOutage.us.
- **Optional, not built yet:** US Drought Monitor weeks in D2+, NOAA Atlas 14 100-year 24-hour
  depth, USFS Wildfire Risk to Communities ranks, volcano threat polygons, tract-level NRI.

## 12. Refresh

`.github/workflows/data-refresh.yml` runs quarterly (and on demand with an optional job list),
builds with `cargo run --release -p rr-etl -- refresh --out data`, gates on `rr-etl verify`, and
opens a pull request; nothing merges automatically. A failing source keeps its previous pack and is
named in `data/CHANGES.md`; a file a job stops writing is deleted and listed there as removed. The About screen shows `manifest.pack_version` and `manifest.generated`
so a stale snapshot is obvious. The full run takes about half an hour (streaming 11.6 GB of EAGLE-I
files dominates).
