# Data sources and packs

Status: maintained. Derived from `docs/research/data-sources.md` (2026-09-25) and the ETL in
`crates/rr-etl`. Every number in a pack traces to a source recorded in `data/manifest.json` (URL as
fetched, version label, retrieval time, sha256 of the raw input, licence and obligations, rows in
and out). This document explains what each pack holds, where it comes from, what we owe the
source, and the judgement calls made along the way.

Contents: 1 building and loading · 2 the packs · 3 Connecticut · 4 ZIP codes · 5 the power-outage
event definition · 6 NRI terms and disclaimer · 7 attributions · 8 privacy · 9 optional lookups ·
10 limitations and data-quality findings · 11 sources not used · 12 refresh · 13 data pack v2:
exposure columns (strategic sites, UASI, geomagnetic, smoke, karst and landslide, levees, dams,
water systems, storm surge, eviction, dust storms, optional packs).

## 1. Building and loading

```
cargo run -p rr-etl -- refresh --out data [--only <job>[,<job>]] [--optional] [--keep-raw] [--keep-intermediate]
cargo run -p rr-etl -- verify --data data
cargo run -p rr-etl -- manifest --rehash --data data
cargo run -p rr-etl -- jobs
```

- **Jobs** (run order): `geography`, `nri`, `outages`, `events`, `seismic`, `climate`, `flood`,
  `strategic`, `geomag`, `ground`, `levees`, `water_systems`, `smoke`, `facilities`,
  `surge_proxy`, `eviction`, `surge` (optional), `wildfire_places` (optional), `vulnerability`,
  `base_rates`, `series`, `outage_model`, `climate_daily`, `reliability`, `displacement`. Later
  jobs read the county list and the Connecticut crosswalk written by `geography`; `facilities`
  reads `core/strategic_sites.toml` (written by `strategic`) and `surge_proxy` reads the NRI and
  events packs.
- **Optional jobs** (`default: false`) build optional packs under `data/opt/<pack>/` and are left
  out of a plain `refresh` (and so of the quarterly Action): name them with `--only`, or add
  `--optional` to a full run. `surge` downloads 0.25 GB of NHC map archives, one at a time, and
  takes about three minutes; `wildfire_places` builds a pack the core does not need (§13).
  `outage_model` is a default job: it writes core files and the optional pack `outage_events`.
- **Owner sign-offs.** `manifest.json` has a `sign_offs` section that refreshes keep as they are.
  A job gated on a key writes its output only when `approved` is true (today: `eviction`, key
  `eviction_lab_odc_by`, §13.10). Only a person flips it.
- **Intermediate files.** `outages` hands every repaired outage event, each unit's months of data
  and the outage customer-hours per local day to `outage_model` and `climate_daily`; `events` hands
  the 2014+ Storm Events county-episodes (UTC) and the HURDAT2 fixes with their times to
  `outage_model`; `series` hands the OE-417 reports to `outage_model`. They live in
  `data/raw/intermediate/` (about 20 MB, git-ignored) and are deleted at the end of a refresh unless
  `--keep-raw` or `--keep-intermediate` is given, so `--only outage_model` works only after a run
  that kept them.
- **Raw inputs are not kept.** Small sources are held in memory; the 11.6 GB of EAGLE-I outage
  files are streamed from figshare and parsed on the fly; the three USGS hazard-curve grid ZIPs
  (0.9 GB for the contiguous US) are written to `data/raw/seismic/` only while they are read (a
  ZIP needs random access) and deleted straight after. `--keep-raw` keeps copies of the small
  inputs and the grid ZIPs under `data/raw/` (git-ignored); the multi-gigabyte EAGLE-I years are
  never kept.
- **Deterministic output.** Rows are sorted by key, numbers are rounded to 4 significant figures
  (coordinates to fixed decimals), and transcendental maths uses the pure-Rust `rr_types::math`,
  so an unchanged input produces a byte-identical pack. Only the manifest's timestamps change.
- **`manifest --rehash`** recomputes every listed file's sha256, size and row count, each job's
  `rows_out` and `pack_version` from the files on disk, and notes the change in `data/CHANGES.md`;
  it downloads nothing and runs no job. Use it after merging two data branches (a file both
  rebuilt matches neither side's checksum, as `core/events.csv` did when data-model met
  data-hazard in v0.2.0) or after a hand edit of a pack file, then run `verify`. It leaves
  `generated` and the jobs' sources and timestamps alone: they date the refreshes that fetched
  the data.
- **`verify`** recomputes every file's sha256 and row count against the manifest and checks that
  every county on the map exists in every county-keyed pack or is listed under that job's
  `missing` reasons, that no pack uses an old Connecticut county code, and that ZIP shares sum
  to at most 1. `rr-etl verify` also reports the core pack's gzipped size (file by file) and fails
  when it is over the 5 MB budget, and lists the optional packs' sizes.
- **Loading in the app.** The web app fetches `data/manifest.json`, then each file listed under
  `packs.core.files` (same origin), and passes the bytes to `load_pack(path, bytes)` with the
  path exactly as the manifest lists it (for example `core/nri_hazards.csv`). The engine checks
  each file against its sha256 and answers `pack_corrupt` on a mismatch. `geo/counties.json` is
  only needed for the map. Note for the planner: `docs/ENGINE-API.md` describes `load_pack(name,
  bytes)` with a pack name like `core`; the core pack is several files, so the name is the file's
  manifest path. A single-file bundle can be added later without changing the files.

## 2. The packs

Sizes are gzip -9 as a static host would serve them. The core pack totals **2.86 MB**
gzipped file by file (budget 5 MB; data pack v2's exposure columns added 0.18 MB of it, §13, and
its calibration files and wider outage tables 0.30 MB, below) and 11.2 MB uncompressed; a first
visit fetches about 2.4 MB of it (everything but the two ZIP tables, which load when a ZIP code is
typed). `geo/counties.json` is **0.30 MB** gzipped (budget
0.35 MB). The optional packs (§13.12) are not part of the core and load only when a feature asks.
Until 2026-09-26 the core pack was 2.99 MB: it also shipped `zip_centroids.csv` and three NRI
columns (`expb`, `ealb`, `alrb`) that nothing in the engine read.

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
| `p_ge_1d`, `p_ge_3d`, `p_ge_7d`, `p_ge_14d`, `p_ge_30d` | Share of those customer outages lasting at least 1, 3, 7, 14, 30 days (`p_ge_30d` since 2026-09-26) |
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
fallback. The engine uses a state's row for every county with no row in `outages.csv` (72 counties
in 9 states; `rr-data` marks the record `state_series` and gives it the years its state's
counties cover); American Samoa, Guam and the Northern Mariana Islands have no state row. Source: ORNL EAGLE-I recorded electricity outages 2014–2025, figshare
doi:10.6084/m9.figshare.24237376 (v4, 2026-02-25), `MCC.csv`, `coverage_history.csv`; CDC SVI 2022
household counts. **CC BY 4.0: the credit line in §7 must be shown.** Refresh: yearly (a new year
is added each spring).

### core/events.csv (44,397 rows)

Long format: `fips`, `event_type`, `rate_per_year`, `share_damaging`, `median_days`, `p90_days`,
`events`, `share_injury`, `source`, `years`. A missing row means no recorded event of that type in
the period (rate 0), except where the source does not cover the place (below).

| `event_type` | Source and rule | Period |
| --- | --- | --- |
| `tropical_storm_passage`, `hurricane_passage`, `major_hurricane_passage` | NHC HURDAT2 (Atlantic, eastern and central Pacific): best track within 50 nautical miles of the county's internal point with interpolated wind ≥ 34 / 64 / 96 kt, any status | 1950–2025 |
| `tornado`, `tornado_ef2plus` | SPC severe database: tornadoes touching the county (per-state segment records, `sn = 1`, counties `f1`–`f4`); EF2+ by rating | 1996–2025 |
| `hail_1in_day`, `hail_2in_day` | SPC: days with a report of hail ≥ 1 in / ≥ 2 in in the county | 1996–2025 |
| `severe_wind_day`, `severe_wind_65kt_day` | SPC: days with any severe-wind report / ≥ 65 kt | 1996–2025 |
| `winter_storm` (Winter Storm, Blizzard, Heavy Snow, Lake-Effect Snow), `ice_storm`, `extreme_cold`, `heat`, `flood`, `flash_flood`, `coastal_flood` (incl. storm surge, lakeshore flood), `wildfire`, `high_wind`, `drought`, `dust_storm` (Dust Storm; not Dust Devil; added 2026-09-26, 340 counties), `tropical_cyclone_impact` | NOAA NCEI Storm Events: one observation per county per NOAA episode (`EPISODE_ID`) and type; zone records apply to every county in the NWS zone (zone-county correlation files); retired zones fall back to the county the zone is named after | 1996–2025 |

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
NID dams rated High hazard potential; the engine's `dams_high_total`), `nuclear_within_16km` /
`nuclear_within_80km` (any part of the county within 10 / 50 miles of an operating plant),
`significant_hazard_dams`, `dams_high_poor_condition` (§13.7).
`zip_facilities.csv`: `nearest_nuclear_km` (from the ZIP centroid, 0.1 km), `tri_within_5km`,
`dams_high_within_10km_naming_town` (§13.7), `strategic_km`, `strategic_bearing`,
`strategic_site` (§13.1).
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

### Data-pack v2 calibration files (job `outage_model`, `climate_daily`, `reliability`, `displacement`, `series`)

Added 2026-09-26 for v0.2.0 (the `data-model` workstream). Every county file is keyed `fips` over
the canonical counties, lists the counties it lacks under its job's `missing` reasons, and loads
into `CountyRecord` fields defined in `rr_types::calibration` (`outage_model`, `temperature`,
`reliability`, `declarations`). Sizes are gzip -9.

| File | Rows | gz | What it holds |
| --- | --- | --- | --- |
| `core/outage_pooled.csv` | 3,209 | 78.6 KB | regional pooled outage tail and credibility weights (§5.3) |
| `core/outage_causes.csv` | 3,153 | 39.5 KB | share of recorded outages by cause (§5.2) |
| `core/outage_curves.csv` | 102 | 2.8 KB | pooled restoration curves by region and cause, restoration factors (§5.5) |
| `core/outage_stress.csv` | 3,186 | 30.4 KB | worst outage event in each county's region (§5.5) |
| `core/temperature.csv` | 3,107 | 83.8 KB | heat and cold days by month; outage hours on hot and cold days |
| `core/reliability.csv` | 3,174 | 25.8 KB | utility SAIDI and SAIFI (EIA-861 2015–2024) |
| `core/declarations.csv` | 3,232 | 14.1 KB | major-disaster declarations per county (OpenFEMA) |
| `core/series/*.toml` (7 files) | 532 entries | 13.2 KB | national series (below) |

These add **288 KB** gzipped to the core pack (0.30 MB with the `p_ge_30d` column added to the
outage tables); with the exposure columns of §13 the core pack totals **2.86 MB** gzipped file by
file (budget 5 MB).

**core/outage_pooled.csv** — the regional pooled outage tail (§5.3): `basis` (`blend`,
`region_only` for counties with no record of their own, `own_only` where no neighbour has one),
`rate` (customer outages per customer-year in the pool, all lengths), `lam_ge_1d` … `lam_ge_30d`
(blended rate of customer outages lasting at least 1, 3, 7, 14 and 30 days, per customer-year),
`z_1d` … `z_30d` (the weight on the county's own record at each length) and `region_counties`
(neighbours with records inside the 400 km radius). Rates cover the outages the model does not
carry in rows of its own: those attributed to hurricanes, wildfires, floods, cold-driven grid
emergencies and other grid failures are left out (their shares are in `outage_causes.csv`).
Three significant figures for rates, two for weights.

**core/outage_causes.csv** — share of the county's recorded customer outages by cause (§5.2):
`hurricane_share`, `ice_share`, `winter_share`, `wind_share`, `wildfire_share`, `heat_share`,
`cold_grid_share`, `flood_share`, `grid_share`, `unattributed_share` (all lengths), and for the five
causes outside the pool the share of outages lasting a day or more (`hurricane_share_ge_1d` …).
Shares under half a percent are written as 0.

**core/outage_curves.csv** — pooled restoration curves by `region` (NCA5 region, `puerto_rico`,
`virgin_islands`, or `mainland` = the contiguous US) and `class` (a cause, `all`, or
`historic:<id>` for a hand-copied event): `events` (major county events pooled: at least 10% of the
county's customers and 2,000 customers out at the peak), `customers` (their summed peaks), `s_1d` …
`s_30d` (peak-weighted mean share of the peak still out that many days after the peak),
`t50_days`, `t90_days` (peak-weighted median days until half and nine in ten are back) and `factor`
(t90 over the mainland's for the same cause, bounded 0.5–5: the restoration factor of M-10).

**core/outage_stress.csv** — the worst outage event in each county's region (§5.5): `event` (display
name), `class`, `cause` (HURDAT2 storm id or Storm Events type), `date` (UTC date the county outage
began), `recorded_in` (the county whose curve it is), `distance_km`, `peak_share` (share of that
county's customers out at the peak), `s_1d` … `s_30d` (share of the peak still out), `source`
(`eaglei`, or `historic:<id>` for a hand-copied event from `crates/rr-etl/data/historic_outages.toml`).

**core/temperature.csv** — NOAA nClimGrid-Daily (public domain), contiguous US: `tmax_ge_90f_MM`,
`tmax_ge_100f_MM`, `tmin_le_20f_MM`, `tmin_le_0f_MM` for months 01–12 (share of days, 1991–2020,
whole percentage points), and `outage_hot_share`, `outage_cold_share` (share of the county's EAGLE-I
event customer-hours 2014–2025 on days with a county-average high of at least 95 °F / a low of at
most 20 °F) with `region_outage_hot_share`, `region_outage_cold_share` (the same, customer-hour
weighted over the county and every county within 400 km). nClimGrid's county code is NCEI's
alphabetical state number followed by the county FIPS (`02001` is Apache County, **Arizona**), so the
state is taken from the abbreviation in the file's name column. Connecticut's eight old counties
are converted by land share. Alaska, Hawaii and the territories have no nClimGrid series.

**core/reliability.csv** — EIA-861 (public domain), 2015–2024: `saidi_with_med_min`,
`saifi_with_med`, `saidi_without_med_min`, `saifi_without_med`, `utility_years`. Per year, a county's
value is the mean over the utilities EIA lists as serving it, each weighted by its customers divided
evenly over its counties (EIA publishes no customer split by county); IEEE 1366 values where the
utility reports them, otherwise its other method; the published value is the mean over the years.
The 2014 service-territory file is in the old binary Excel format and is not read.

**core/declarations.csv** — OpenFEMA Disaster Declarations Summaries v2: `last_5yr` (major-disaster
declarations, type DR, that designated the county in the five full years before the build),
`since_2000`, `with_individual_assistance`, `hurricane_or_flood` (hurricane, tropical storm or
depression, typhoon, coastal storm, flood). One count per disaster per county; statewide and tribal
designations are not counted; Connecticut declarations on the old counties count for every planning
region they overlap. This is `RecoveryInfo.county_declarations_5yr`.

**core/series/*.toml** — national series, one file per source, each with a header (`title`,
`publisher`, `url`, `licence`, `attribution`, `source` citation id, `retrieved`, `how` = `fetched`
or `transcribed`, `notes`), `[[rate]]` entries (value, unit, optional `low`/`high`, `year`,
`period`, the `figure` it comes from, the `derivation`, `note`, and `unverified` for hand-copied
figures not re-checked) and `[[row]]` entries holding the table behind the rates:

| File | Source (licence) | How | Rates |
| --- | --- | --- | --- |
| `drug_shortages.toml` | U.S. Food and Drug Administration (openFDA) (CC0 1.0 (https://open.fda.gov/license/)) | fetched | `drug_shortages_current` |
| `fbi_arrests.toml` | Federal Bureau of Investigation, Uniform Crime Reporting Program (Crime Data Explorer) (US Government work, public domain (17 U.S.C. 105)) | fetched | `arrests_per_100k_female_10_17`, `arrests_per_100k_female_18_24`, `arrests_per_100k_female_25_34`, `arrests_per_100k_female_35_44`, `arrests_per_100k_female_45_54`, `arrests_per_100k_female_55_64` and 10 more |
| `fcc_dirs.toml` | Federal Communications Commission, Public Safety and Homeland Security Bureau (US Government work, public domain (17 U.S.C. 105)) | transcribed | `cell_sites_out_peak_area_share_median`, `cell_sites_out_worst_county_share_median`, `cell_sites_days_until_under_5pct_median` |
| `fdic_failures.toml` | Federal Deposit Insurance Corporation (US Government work, public domain (17 U.S.C. 105)) | fetched | `bank_failures_per_year`, `bank_failure_cluster_year_share` |
| `funding_gaps.toml` | Congressional Research Service (J. V. Saturno), with the House Historian's shutdown column (CRS reports are works of the US Government, not subject to copyright) | transcribed | `funding_gap_ge_14d_per_year`, `shutdown_with_furloughs_per_year` |
| `ihp_displacement.toml` | Federal Emergency Management Agency (OpenFEMA) (OpenFEMA Terms and Conditions (public data; citation and statement required)) | fetched | `ihp_rental_assistance_per_approved_usd_earthquake`, `ihp_rental_assistance_per_approved_usd_fire`, `ihp_rental_assistance_per_approved_usd_flood`, `ihp_rental_assistance_per_approved_usd_hurricane`, `ihp_rental_assistance_per_approved_usd_landslide`, `ihp_rental_assistance_per_approved_usd_other` and 3 more |
| `oe417.toml` | Pacific Northwest National Laboratory (from DOE OE-417 reports and ORNL EAGLE-I) (CC BY 4.0) | fetched | `grid_weather_reports_per_year`, `grid_operations_reports_per_year`, `grid_physical_attack_reports_per_year`, `grid_suspicious_activity_reports_per_year`, `grid_cyber_reports_per_year`, `grid_fuel_supply_reports_per_year` |

**Optional pack `outage_events`** (issue #15; `opt/outage_events/`, loaded only by the expert views
and the validation page): `county_events.csv` — every county event that left at least 0.25% of the
county's customers (and at least 5) out for a day or more: `county_fips`, `start` (UTC), `class`,
`cause` (storm id or Storm Events type), `peak_share`, `s_1d` … `s_30d`, `ge_1d_share` and
`ge_7d_share` (customer outages of a day / a week or more, per county customer); Puerto Rico's
island-wide series is filed once under San Juan (72127) and `rr-data` serves it for every municipio.
`holdout.csv` — the held-out test of §5.4 (split, length, estimator, measure, reliability bin,
predicted, observed, n). Sizes: `county_events.csv` 29,878 rows, 356 KB gzipped; `holdout.csv` 130 rows, 1.6 KB gzipped.

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
snapshots of up to **2 hours** are bridged, and up to **72 hours** when the event had been at the 1%
level for six hours, held it for the hour before the gap and holds at least half of that level for
an hour after it (§5.1). Inside an event the counts are repaired
before durations are read (§5.1), and reversals smaller than half of the current peak or trough
(or smaller than the 1% level) are treated as reporting noise. Customers are assumed to be restored
**in the order they lost power**, which splits each event's customer-hours into individual outage
lengths, except that customers still out N days after the peak count as out at least N days.
Rates are customer outages in such events per customer per year of data.

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
second storm) and removes flicker. Assuming last-out-first-restored instead raises the multi-day
shares by up to about a third in the hardest-hit counties, so the ordering assumption matters at the
margin, not in kind.

**Denominators.** Years of data count months with at least one record (EAGLE-I lists only
snapshots with someone out); state-years in 2018–2022 with under 50% customer coverage (ORNL's
coverage history) are dropped (Montana 2018, Nebraska 2018–2022, South Dakota 2018). Customers are
the larger of ORNL's modelled count and the county's households (§10).

**Result, nationally.** Weighting counties by customers, a customer has about 0.055 outages a year lasting at least a day and about 0.0135 lasting at least three days (the unrepaired record said 0.05 and 0.011). The median county records 5.8 hours of outage per customer per year, close to EIA's national interruption figures with major events.

### 5.1 Repairing reporting dropouts (model review M-02; 2026-09-26)

EAGLE-I's county count is the sum of the utilities' outage maps, scraped every 15 minutes. During
the storms that matter most, a utility is often missing from a scrape: the count drops by most of
the outage for 15 minutes to a few hours, or a map fails for days. Some snapshots count a utility
twice, and some are stale: a utility's last figure repeated for an hour or a day. In Buncombe County during Helene the count fell from about 115,000 to about 9,700 for a
quarter of an hour every hour or two and doubled now and then; read as it stands, each drop was
"everyone restored" and each return "everyone cut off again", so first-out-first-restored turned a
two-week outage into about 9.7 million "customer outages" among 181,000 customers, none lasting a week, while about 9,300 customers were still out on day 17 (Blue Ridge Public Radio, 14 October 2024). The fix, inside each event:

1. A gap or dip longer than 2 hours is bridged up to 72 hours when the event had been at the 1%
   level for at least six hours, held it for the hour before the gap, and holds at least half of that
   level for an hour after it; readings under half of the level inside the gap (the utilities still
   reporting while the big one is missing) are dropped, so the level carries across. Hurricane
   Michael's restoration in Jackson County, Florida, has a day of readings of about 9 customers and
   two days of scattered readings while about 10,000 customers were still out. The six-hour and
   one-hour conditions keep stale readings from being bridged: in June 2019 Cherokee County,
   Alabama, read 1 or 2 customers out for days, broken by one-hour plateaus of 510 and 506 at the
   same hour each morning; an earlier version of this rule, which asked only that the count before
   the gap reach 1% and the first reading after it reach half, joined them into a six-day outage
   and added customer-hours in about 1,400 small and mid-sized counties.
2. Where the 3-hour running median shows a sustained outage, counts more than 1.5 times the median
   are cut to it (reports counted twice).
3. Dips narrower than 24 hours are filled to the level around them (a morphological closing, which
   leaves onsets and restorations unchanged).
4. The reversal filter and first-out-first-restored then run as before, except that customers
   still out N days after the peak are counted as out at least N days (the event's own curve is a
   floor on the per-customer tail): a second wave after the peak otherwise makes first-out-first-
   restored retire the earliest customers first (Irma in Baker County, Florida: 300 customers out
   for a week after the peak read as none).

Effect on the committed pack (before → after):

| | Before | After |
| --- | --- | --- |
| Buncombe NC (Helene): outages per customer-year; share lasting 3 / 7 / 14 days | 10.06; 0.5% / 0 / 0 | 0.96; 9.0% / 7.4% / 2.5% |
| Harris TX (Uri, Beryl): outages per customer-year; share lasting 3 / 7 / 14 days | 1.06; 3.1% / 0.2% / 0 | 0.23; 21.1% / 4.6% / 0 |
| Linn IA (derecho): outages per customer-year; share lasting 3 / 7 / 14 days | 0.55; 11.6% / 4.3% / 0 | 0.34; 19.7% / 10.6% / 0.4% |
| Jackson FL (Michael): outages per customer-year; share lasting 3 / 7 / 14 days | 0.82; 3.8% / 2.2% / 0 | 0.53; 7.8% / 4.8% / 3.9% |
| Counties with an event of a week or more but no week-long customer outage | 336 of 761 | 99 of 799 |
| Counties with an event of two weeks or more but no two-week customer outage | 126 of 272 | 36 of 274 |
| Counties with more than 3 outages a customer-year and a median of 2 hours or less (flicker) | 128 | 16 |

Across all counties the repair bridged 9,050 gaps in 1,929 counties, and inside events the repaired
series holds 7% more customer-hours than the recorded snapshots (median county +5%, 90th percentile
+12%; the manifest's `outages` notes). Events of two storms less than a day apart now merge, and a
genuine restoration followed within three days by a return to half the previous level that holds
for an hour reads as one outage: both err toward longer outages. Each event is also recorded (optional pack) with its **restoration curve**: the share of
the peak still out 1, 2, 3, 5, 7, 10, 14, 21, 30, 60 and 90 days after the peak, the running minimum
of the repaired count, so a customer counted at day N had been out at least N days (the lower bound
M-02 proposes). `rr-etl verify` checks that every county whose event kept at least 1% of its
customers out for a week shows week-long customer outages (on the committed pack it passes for
every county).

### 5.2 Cause attribution (M-10, M-18)

EAGLE-I has no cause field. Each county event is matched, in order, to:

1. a **tropical cyclone** when a HURDAT2 track point of at least 34 kt (fixes interpolated hourly;
   the ETL used to discard their times) passes within 300 km of the county's internal point between
   36 hours before the outage began and 24 hours after it reached its peak (capped at two days after
   the start), so rain ahead of a storm does not hide it (Helene's outage in Buncombe began on
   25 September 2024 and peaked on the 27th);
2. otherwise the highest-priority **NOAA Storm Events** episode in the county or its forecast zone
   (times now converted to UTC from `CZ_TIMEZONE`) overlapping the six hours before the start to two
   hours after the rise: tropical types, ice, tornado and thunderstorm wind, winter storm, wildfire,
   high wind, flood, lightning and hail, heat, cold. A winter, ice or cold match that coincides with
   a DOE OE-417 load-shed, energy-emergency, fuel-supply or system-operations report in the same
   state (span within a day of the rise) becomes **cold_grid** (the February 2021 Texas blackout);
3. otherwise an OE-417 report that is not weather (operations, transmission, attack, cyber, fuel) in
   the same state within a day: **grid**;
4. otherwise **unattributed** ("power cuts, cause not recorded").

On the committed pack: 571,912 county events read; 7,467 matched to a tropical cyclone track, 88,595
to Storm Events episodes, 14,025 to an OE-417 grid disturbance, the rest are "cause not recorded".
Of the 857 events with customers still out a week after the peak, 619 (72%) are attributed. 453,587
Storm Events county-episodes and 467 tropical cyclones (2014 onward) were available.
The OE-417 reports come from PNNL's linkage file (CC BY 4.0, credited in §7),
which links reports to states and times, not places: a large state's report can touch an unrelated
county's outage.

### 5.3 Regional pooled tails with credibility weights (M-01)

A county's own twelve years rarely hold the storm that sets a one-in-a-hundred target, so its long
tail is blended with its region's. For each county *c* and length *d* (1, 3, 7, 14, 30 days):

- λ_c(d) = customer outages lasting at least *d* days per customer per year in its own record
  (first-out-first-restored on the repaired series), leaving out outages attributed to hurricanes,
  wildfires, floods, cold_grid and grid, which the model carries in rows of their own;
- λ_R(d) = the same rate over the county **and** every county within R(d) of it — **R = 400 km for
  1 and 3 days, 800 km for 7, 14 and 30 days** — each weighted by customers × years of data ×
  (1 − (distance / R)²)² (a biweight); separate island grids (Puerto Rico, the Virgin Islands,
  Hawaii, Alaska) are never pooled with the mainland;
- E_c(d) = the number of events reaching *d* days that a county with *c*'s years of data would
  record at the pooled rate (an event reaches *d* days when at least 0.25% of the county's
  customers, and at least 5, were out that long);
- **Z = E / (E + k), k = 5** (the weight the model already gives five hurricane passages when it
  shrinks a county's major-hurricane share), and **λ̂_c(d) = Z · λ_c(d) + (1 − Z) · λ_R(d)**.

A county's record gets full weight only where it can be expected to hold many events of that
length, so one extreme storm no longer sets its tail: Linn County, Iowa's record holds the 2020 derecho, about 0.036 customer outages of a week or more per customer-year; its weight at a week is Z = 0.022 and the blended rate 0.0014. Across the seven Iowa counties of M-01 (Linn, Johnson, Scott, Polk, Pottawattamie, Black Hawk, Woodbury) the blended three-day rate ranges 0.0071–0.0146 per customer-year, where their own records range 0.0000–0.0663.

Choices were compared on the held-out test (§5.4, mean Poisson deviance of the blend, lower is
better; development runs of 2026-09-26 on an earlier version of the repair, so the values differ
slightly from §5.4):

| Pooling | 2014–19 → 2020–25: 1 d / 3 d / 7 d / 14 d | even → odd years: 1 d / 3 d / 7 d / 14 d |
| --- | --- | --- |
| 250 km, same NCA5 region, county left out | 4.96 / 2.67 / 1.38 / 0.467 | 2.17 / 1.31 / 1.97 / 0.241 |
| 250 km, same region, county included | 4.84 / 2.63 / 1.34 / 0.464 | 1.97 / 1.26 / 1.89 / 0.179 |
| 250 km, distance only, county included | 4.49 / 2.40 / 1.26 / 0.443 | 1.97 / 1.27 / 1.86 / 0.178 |
| 400 km, distance only | 4.39 / 1.96 / 1.08 / 0.388 | 2.02 / 1.24 / 1.77 / 0.155 |
| 800 km, distance only | 4.47 / 1.89 / 0.73 / 0.307 | 2.11 / 1.29 / 1.78 / 0.151 |
| **400 km (1, 3 d) and 800 km (7 d +)** | **4.39 / 1.96 / 0.73 / 0.307** | **2.02 / 1.24 / 1.78 / 0.151** |

Leaving the county out of its own region (usual for validation, wrong for the estimate) made the
county a record storm hit look safer than its neighbours. k from 2 to 20 changed little. Each row can
be rerun with `RR_OUTAGE_POOL=radius_km,k,same_region,include_self[,radius_km_for_7d_and_longer]`
and `refresh --only outage_model` after a run with `--keep-intermediate`; the manifest note records
the pooling a build used. Because the
radius was chosen on these two splits, §5.4's results for the chosen setting are slightly
optimistic.

### 5.4 Held-out test

Fit on 2014–2019 and predict 2020–2025 (the brief's split), and fit on even years and predict odd
years (not biased by the lower EAGLE-I coverage before 2018). Estimators: the county's own record,
the region alone, and the blend. Mean Poisson deviance of each county's count of qualifying events
(lower is better; a county-only estimate that saw no event of that length predicts none and is
penalised when one comes), and observed ÷ predicted customer outages summed over all counties:

| Split | Length | Deviance: county only / region only / blend | Observed ÷ predicted: county / region / blend |
| --- | --- | --- | --- |
| 2014–2019 → 2020–2025 | 1 d | 22.56 / 4.73 / **3.70** | 1.45 / 1.65 / 1.56 |
| 2014–2019 → 2020–2025 | 3 d | 15.48 / 2.05 / **1.98** | 1.90 / 2.48 / 2.41 |
| 2014–2019 → 2020–2025 | 7 d | 3.10 / 0.75 / **0.74** | 5.25 / 5.39 / 5.38 |
| 2014–2019 → 2020–2025 | 14 d | 0.80 / 0.38 / **0.38** | 3.80 / 3.96 / 3.96 |
| even years → odd years | 1 d | 7.90 / 3.12 / **1.98** | 1.04 / 1.09 / 1.05 |
| even years → odd years | 3 d | 8.33 / 1.46 / **1.35** | 1.08 / 1.11 / 1.10 |
| even years → odd years | 7 d | 3.17 / 1.84 / **1.83** | 1.02 / 1.04 / 1.03 |
| even years → odd years | 14 d | 0.41 / 0.18 / **0.18** | 1.31 / 1.32 / 1.32 |

Reading it: the blend beats the county's own record everywhere, by a factor of about four to eight
at one and three days, and edges out the region alone. In the even/odd split, predicted and observed
customer outages agree within about 10% at one to seven days. The 2014–2019 fit under-predicts
2020–2025 at every length and for every estimator, by 1.5–1.6 times at one day and 1.9–5.4 times at
three days and longer: EAGLE-I covered fewer utilities before 2018, and 2020–2025 held the derecho,
Uri, Ida, Ian, Helene and Beryl.
The reliability rows in `opt/outage_events/holdout.csv` (predicted yearly chance of a qualifying
event against the share of county-years that had one) show the same (even → odd years; bins with
at least 500 county-years):

- 3+ days, predicted 3 to 10 in 100: 0.063 predicted against 0.070 observed (7,940 county-years)
- 3+ days, predicted 10 to 30 in 100: 0.168 predicted against 0.150 observed (7,615 county-years)
- 3+ days, predicted 30 in 100 or more: 0.478 predicted against 0.615 observed (865 county-years)
- 7+ days, predicted under 1 in 100: 0.008 predicted against 0.022 observed (8,229 county-years)
- 7+ days, predicted 1 to 3 in 100: 0.018 predicted against 0.015 observed (7,876 county-years)
- 7+ days, predicted 3 to 10 in 100: 0.038 predicted against 0.057 observed (560 county-years)

The smaller bins, all in `holdout.csv` and the manifest notes, are noisier: the 18 county-years
given the highest week-long chances by the 2014–2019 fit (predicted 0.29–0.34) had one such event
between them, and the 355 county-years it gave the lowest three-day chance (0.004) had 19 (the
2014–2019 fit under-predicts, as above).

### 5.5 Restoration curves, the island grids and the stress table

**Pooled curves** (`outage_curves.csv`): on the mainland, hurricane 1,722 events, t50 1.4 and t90 3.8 days, 4.9% of the peak still out at a week; ice 354 events, t50 1.2 and t90 2.9 days, 4.9% of the peak still out at a week; winter 1,705 events, t50 0.7 and t90 1.8 days, 0.4% of the peak still out at a week; wind 7,397 events, t50 0.6 and t90 1.5 days, 0.2% of the peak still out at a week; all 24,602 events, t50 0.6 and t90 1.4 days, 1.3% of the peak still out at a week. Times under a day are not resolved (the curve starts at one day; t50 0.5 and t90 0.9 mean "within a day"). Puerto Rico's own hurricane events in EAGLE-I (3, mainly Fiona 2022): 40% of the peak still out at a week, t90 17.8 days, restoration factor 4.7 against the mainland.

**Puerto Rico and the Virgin Islands.** EAGLE-I's Puerto Rico series starts in 2021 (Fiona 2022,
the island-wide blackouts of 2024–2025). Each island grid is its own pooling region, and its curves
carry a restoration factor against the mainland. Hurricane Maria (2017) is hand-copied from DOE's
60 situation reports, the Government of Puerto Rico's status.pr and PREPA (via NPR):
all 1.57 million customers out for the first week, 91% still out at two weeks, about 82% at a month (a load figure: PREPA counted no customers from 13 October to 2 January), 36.5% at day 119, 4.2% at day 195, the last customer on day 328. Its curve row is `historic:maria_2017_pr` with the restoration factor at its bound of 5. The Virgin Islands after Irma and Maria are copied the same way (customer counts start on
day 26; the first week is qualitative, so the event is marked unverified).

**The worst-event stress table** (`outage_stress.csv`, about 30 KB gzipped): for each county, the
major county event within 250 km in its NCA5 region (or the county itself) with the largest share of
the peak still out a week after the peak, as recorded where it was worst, with its name (HURDAT2 for
tropical cyclones; a short list of named events such as the August 2020 derecho and the February
2021 winter storm in `historic_outages.toml`; otherwise the cause and month) and curve.
Hand-copied events stand in where they are worse than anything in EAGLE-I (Maria for Puerto Rico,
Irma and Maria for the Virgin Islands). The backtest counties:

| County | Worst event in the region | Recorded in | Out at peak | Share of the peak still out at 1 / 3 / 7 / 14 / 30 days |
| --- | --- | --- | --- | --- |
| Buncombe NC | Hurricane Helene (2024) | Buncombe NC | 100% | 92% / 78% / 69% / 22% / 0% |
| Linn IA | August 2020 Midwest derecho | Linn IA | 93% | 97% / 78% / 37% / 1% / 0% |
| Jefferson LA | Hurricane Ida (2021) | Lafourche LA, 35 km | 100% | 100% / 100% / 98% / 42% / 1% |
| Harris TX | Hurricane Harvey (2017) | Orange TX, 147 km | 98% | 87% / 85% / 25% / 0% / 0% |
| Travis TX | February 2021 winter storm and Texas blackout (Uri) | Gillespie TX, 120 km | 55% | 74% / 51% / 29% / 4% / 0% |
| Hinds MS | Hurricane Ida (2021) | St. Helena LA, 162 km | 100% | 94% / 86% / 83% / 56% / 0% |
| San Juan PR | Hurricane Maria (2017) | (hand-copied) | 100% | 100% / 100% / 100% / 91% / 82% |

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
USACE NID / FEMA nuclear sites. Data pack v2 adds: Strategic sites (the compilation and its
sources), IGRF-14 and NERC TPL-007, NOAA HMS and EPA AQS, USGS karst and landslide maps, USACE
National Levee Database, EPA ECHO SDWA, the storm-surge proxy, and for the optional packs NOAA
NHC storm surge maps (with Zachry et al. 2015) and USDA Forest Service Wildfire Risk to
Communities (requested citation). An optional pack's credit line is shown only once one of that
pack's files is loaded (the manifest's `attribution_packs` names the pack for each such source),
so a packet never credits data it did not read. The Eviction Lab credit line (ODC-BY 1.0) joins
them only once the owner approves the source (§13.10).

The v2 calibration files (§2, "Data-pack v2 calibration files") add: the PNNL Event-correlated
Outage Dataset (**CC BY 4.0: the credit line must be shown**, like EAGLE-I's), NOAA
nClimGrid-Daily, and OpenFEMA's declarations and housing-assistance datasets (citation +
statement). The national series cite their sources in their own headers (FBI Crime in the United
States, CRS RS20348, FCC DIRS reports, openFDA, FDIC); their `source` ids for
`content/citations.toml` are `pnnl_oe417_linkage`, `openfda_drug_shortages`, `fdic_failed_banks`,
`crs_rs20348_funding_gaps`, `fcc_dirs_reports`, `fbi_cde_arrests` and
`openfema_housing_assistance`; the county files use `ornl_eagle_i_outages`, `noaa_hurdat2`,
`noaa_storm_events` (existing), `noaa_nclimgrid_daily`, `eia_861_reliability` and
`openfema_declarations` (new ids for the content registry).

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
- **EAGLE-I during big storms** (2026-09-26): utilities drop out of the scrape for minutes to days
  and are sometimes counted twice; §5.1's repair bridges the gaps and cuts doubled counts, erring
  toward longer outages. Data gaps longer than the 72-hour bridge, gaps in an event's first six
  hours, and gaps after which the data never hold for an hour still split an event (the continuation
  then reads as a new event whose curve starts at the reappearance).
- **Cause attribution** is by time and place only: a thunderstorm near a weakening tropical storm is
  called a hurricane outage, a wind-driven wildfire safety shutoff with no Storm Events wildfire
  record is called wind (California shutoff logs are not machine-readable), and OE-417 reports are
  matched by state. Three in ten week-long events stay "cause not recorded".
- **The pooled tail** is a regional average with a credibility weight; it cannot see a county's
  individual vulnerability (a feeder on a ridge, an undergrounded suburb). The held-out test (§5.4)
  checks calibration on average, not for any one county.
- **EIA-861 cross-check** (repaired `outages.csv`): over 3,017 counties the rank correlation of
  EAGLE-I outages per customer-year with the utilities' SAIFI (major event days included) is 0.44,
  and of EAGLE-I customer-hours with SAIDI 0.58 (3,040 counties). EAGLE-I counts only outages in
  events that reach 1% of a county's customers, so it should sit below SAIFI: it does in 2,544 of
  3,017 counties. 29 counties show more than three times their utilities' SAIFI (the M-02 flag for
  reporting flicker; `rr-etl verify` lists them), among them 06009, 06043, 06051, 06063, 06091,
  06109, 08019, 08021, 16015 and 22107.
- **nClimGrid** is a county average on a 5 km grid: it runs cooler than the hottest spot in a county
  and warmer than the coldest, and covers the contiguous US only.
- **FBI arrests** count arrests (events), not people, from agencies covering 87–92% of the population;
  national figures by age and sex are a share-allocation of the FBI's national estimate (§2).
- **OpenFEMA declarations:** major-disaster (DR) declarations only, one count per disaster per
  county; 942 statewide or tribal rows are not counted; `last_5yr` = declared in 2021–2025. County
  codes that cannot be placed on the canonical list include Alaska's old 02201 and codes for American
  Samoa's outer islands, Micronesia and the Marshall Islands (the manifest note lists the first 20).
- **OpenFEMA housing assistance:** every damage column of the v2 housing-assistance files (renters'
  moderate, major and substantial damage, owners' inspected-damage buckets, `totalInspected`) is
  zero in every row (checked 2026-09-26 by asking the API for rows above zero: none), so the series
  publishes registrations, approvals and rental assistance only, and no damage share. Months
  displaced by hazard is not built either: only 99,762 of 3.3 million rental-eligible registrations
  carry a rental-assistance end date.
- **Hurricane Maria's curve** mixes customer counts with load for days 30–90 (PREPA reported no
  customer counts then); load overstates household restoration.

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
  PowerOutage.us, Childs et al. 2022 county smoke PM2.5 (CC BY-SA 4.0; its v2.0 beta has no
  licence): smoke days come from NOAA HMS and EPA AQS instead (§13.4).
- **Not machine-readable or not public:** NAPB-90 / TR-82 fallout risk maps (scanned; cited as a
  method precedent only), state boil-water notice records (no bulk download; the consequence
  model uses the published Texas and Kentucky figures), USACE dam inundation maps (restricted),
  NERC's earth-conductivity regions (an image only, so `geomag_factor` has no beta term).
- **Elevation for the surge proxy:** no national elevation source is cheap enough to read per ZIP
  (the NHC rasters in the optional `surge` pack answer the ZIP question properly).
- **Optional, not built yet:** US Drought Monitor weeks in D2+, NOAA Atlas 14 100-year 24-hour
  depth, volcano threat polygons, tract-level NRI, tract-level wildfire exposure (needs zonal
  statistics over the 30 m rasters).
- **Considered for the outage model and national series, not used (2026-09-26):** the DOE OE-417
  annual summaries on `oe.netl.doe.gov` (host refused connections; PNNL's CC BY 4.0 linkage covers
  2014–2023 instead), FCC NORS filings (confidential under 47 CFR 4.2; the public DIRS reports are
  used), the ASHP/University of Utah shortage series (proprietary; openFDA is used), the HHS OCR
  breach portal (counts data breaches, not service outages), CPUC safety-shutoff reports (not
  machine-readable), and IHP registrations for months displaced (end dates on 3% of records).

## 12. Refresh

`.github/workflows/data-refresh.yml` runs quarterly (and on demand with an optional job list),
builds with `cargo run --release -p rr-etl -- refresh --out data`, gates on `rr-etl verify`
(which now includes the core size budget), and opens a pull request; nothing merges
automatically. The optional jobs (`surge`, `wildfire_places`) do not run there; build them by
hand with `--only` when their sources change (NHC publishes a new map version every few years;
Wildfire Risk to Communities about yearly). A failing source keeps its previous pack and is
named in `data/CHANGES.md`; a file a job stops writing is deleted and listed there as removed. The About screen shows `manifest.pack_version` and `manifest.generated`
so a stale snapshot is obvious. The full run takes about 45 minutes: streaming 11.6 GB of EAGLE-I
files dominates (about 17 minutes); the v2 calibration jobs add about 5 (840 monthly nClimGrid files,
ten EIA-861 years, OpenFEMA paging). That was timed before the §13 exposure jobs joined the run;
they add their own downloads.

## 13. Data pack v2: exposure columns

Added 2026-09-26 for the v0.2.0 hazards (DESIGN-DELTA §2; research: `data-audit.md`,
`strategic-sites.md`). Each column is loaded into `CountyRecord.exposure` (a
`rr_types::CountyExposure`) or `rr_types::ZipRecord` (`DataStore::zip_record`), and
`DataStore::location` copies the values the app shows into `LocationResolved.exposure`, each with
a citation id (`rr_data::EXPOSURE_SOURCES`, §13.13). Sizes are gzip -9 of the file (or of the
columns added to an existing file).

| File (job) | Columns | Rows | Size (gz) |
| --- | --- | ---: | ---: |
| `core/strategic.csv` (`strategic`) | `strategic_class`, `strategic_site_ids`, `strategic_km`, `strategic_bearing`, `uasi_share`, `uasi_area_share`, `uasi_area` | 3,232 | 17.0 KB |
| `core/strategic_sites.toml` (`strategic`) | classes with f_S priors and "Why here" templates, 39 sites, metros, ports, refineries, UASI areas, sources | 207 entries | 16.5 KB |
| `core/geomag.csv` (`geomag`) | `geomag_lat`, `geomag_factor` | 3,232 | 13.0 KB |
| `core/smoke.csv` (`smoke`) | `smoke_days_35`, `smoke_days_55`, `smoke_trend`, `hms_smoke_days`, `smoke_basis` | 3,225 | 28.2 KB |
| `core/ground.csv` (`ground`) | `karst_share`, `landslide_susceptible_share` | 3,225 | 19.3 KB |
| `core/levees.csv` (`levees`) | `leveed_pop_share`, `levee_risk_high_share` | 3,232 | 10.6 KB |
| `core/water_systems.csv` (`water_systems`) | `sdwis_violation_pop_share`, `cws_pop_share` | 3,171 | 17.7 KB |
| `core/surge_proxy.csv` (`surge_proxy`) | `surge_proxy_class`, `coastal_flood_pop_share` | 3,232 | 9.8 KB |
| `core/facilities.csv` (`facilities`) | + `dams_high_poor_condition` | 3,232 | +1.5 KB |
| `core/zip_facilities.csv` (`facilities`) | + `dams_high_within_10km_naming_town`, `strategic_km`, `strategic_bearing`, `strategic_site` | 33,791 | +43.6 KB |
| `core/events.csv` (`events`) | + `dust_storm` rows | 340 | +3.4 KB |
| **core total added** | | | **180 KB** (2.37 → 2.55 MB) |
| `core/eviction.csv` (`eviction`, gated) | `eviction_filing_rate` | 3,144 | 13.6 KB, not shipped until approved |

Approximate cost of each column on its own (gzip -9 of the key plus that column, minus the key
alone; a file compresses a little better than the sum of its columns):

| Column | KB | Column | KB |
| --- | ---: | --- | ---: |
| `strategic_class` | 1.3 | `smoke_days_35` | 5.3 |
| `strategic_site_ids` | 2.7 | `smoke_days_55` | 3.9 |
| `strategic_km` (county) | 2.9 | `smoke_trend` | 5.7 |
| `strategic_bearing` (county) | 2.0 | `hms_smoke_days` | 4.6 |
| `uasi_share` | 1.9 | `smoke_basis` | 1.5 |
| `uasi_area_share` | 0.9 | `leveed_pop_share` | 3.3 |
| `uasi_area` | 0.4 | | |
| `geomag_lat` | 4.3 | `levee_risk_high_share` | 0.5 |
| `geomag_factor` | 2.6 | `sdwis_violation_pop_share` | 5.9 |
| `karst_share` | 5.4 | `cws_pop_share` | 5.0 |
| `landslide_susceptible_share` | 7.2 | `surge_proxy_class` | 1.1 |
| `dams_high_poor_condition` | 1.4 | `coastal_flood_pop_share` | 2.2 |
| `dams_high_within_10km_naming_town` (ZIP) | 7.2 | `strategic_km` (ZIP) | 14.6 |
| `strategic_bearing` (ZIP) | 15.9 | `strategic_site` (ZIP) | 9.8 |

### 13.1 Strategic sites and classes (`strategic`)

The curated list is `crates/rr-etl/data/strategic_sites.toml` (compiled into the ETL; hand-edited,
reviewed 2026-09-26). It holds the six classes (A counterforce and command sites; C1 the ten
largest metros, the National Capital Region and NNSA weapons-complex sites; B the missile-field
fallout corridor; C2 other metros of a million or more, big refineries, the ten busiest ports and
other listed installations; D downwind of a target; E the rest), their f_S factors (expert priors,
`prior = true`, shown only as ranges), the rules as numbers, 45 site rows (39 included, 6 excluded
as commonly listed but unsupported), 56 metros, 31 refineries, 10 ports, the 44 FY2026 UASI urban
areas and 31 sources with how each was read (`fetched`, `search_summary`, `not_retrieved`).

Rules, applied in precedence A > C1 > B > C2 > D > E from each county's **Census 2020 centre of
population** (19 counties without one, Connecticut's planning regions and the island areas, use
the Gazetteer internal point): A = the county holds a missile field's launch facilities, contains
an A site, or is within 30 km of an A point site; C1 = top-ten metro, National Capital Region, or
an NNSA site by county list or 30 km; B = within 800 km of a launch-facility county's internal
point at a bearing of 45-135 degrees from it, or within 75 km; C2 = metro of a million or more,
refinery county (200,000 barrels per calendar day or more), principal port county, or a listed
installation; D = within 150 km of an A point site, an NNSA site or a C1 county's centre at a
bearing of 45-135 degrees. Result: **A 68 counties (3.5% of people), C1 128 (26.0%), B 533
(4.4%), C2 334 (28.9%), D 245 (5.8%), E 1,924 (31.4%)** (the research first cut on internal
points: A 60, B 537, C1 129, C2 332, D 245, E 1,929; population centres move Amarillo, Council
Bluffs and Montgomery County, MD into A, as the research expected). Minot A, Hays B, Coos Bay E,
Miami and Phoenix C1 are checked on every run.

`strategic_site_ids` lists what put the county in its class (membership first, then by distance);
ids resolve in `strategic_sites.toml` (site ids, `metro_<cbsa>`, `port_<rank>`,
`refinery_<county>`), and rr-data resolves them into `strategic_places` (name, kind, plain role,
state, sources, unverified flags) so the "Why here" sentence needs no other lookup.
`strategic_km` / `strategic_bearing` (county): distance and compass bearing from the county's
centre to the first distance-based reason (for B, where the missile field lies from you); empty for
membership reasons and class E. ZIP columns in `zip_facilities.csv`: the nearest A or NNSA point
site within 150 km of the ZIP's centre (whole km and degrees, and its id), for 6,808 ZIPs; this
lets a ZIP across a county line from a site (Port Townsend, 36 km from Bangor) be recognised.

Checks done while curating: the three missile-field county lists match the **Sentinel Final EIS
Volume I** (sections 2.1.6.1 p. 2-13, 2.1.7.1 p. 2-33, 2.1.8.1 p. 2-40, read in full); the NNSA
locations page was read in full. Precedent for telling the public: FEMA, *Protection in the
Nuclear Age* (1985), p. 12 ("Designating a place as a 'risk' area does not mean that it will be
attacked; it does indicate a greater potential for attack"); NAPB-90 (1987, released 2005) is a
method precedent only (recorded in the manifest's definitions).

**UASI.** `uasi_share` = the county's share of the national FY2026 UASI total ($584,250,000 across
44 urban areas): the area's allocation share split among its counties by 2020 population (NRI); 0
outside every funded area; `uasi_area_share` = the urban area's own share of the national total,
the same in each of its counties (New York-White Plains 0.2439 in all ten; 0 outside), for the
hazard crates' metro tier; `uasi_area` = the area's rank. Both shares cite `fema_hsgp_fy2026`.
All 44 allocations were checked digit by digit against FEMA's FY2026 HSGP NOFO PDF (Appendix I,
pp. 71-73; sha256 recorded in the manifest). The county footprint of each urban area is a proposal: FEMA publishes none (each urban
area working group sets its own), so the footprints stay **UNVERIFIED**. Per county the shares
favour single-county areas (Los Angeles County 0.058 is the largest); per resident, the New
York-White Plains counties lead.

### 13.2 Geomagnetic latitude and NERC factor (`geomag`)

IGRF-14 dipole terms of the latest main-field epoch (2025.0: g10 -29350.0, g11 -1410.3, h11
4545.5 nT; geomagnetic north pole 80.79 N, 72.76 W) give each county's geomagnetic latitude
(internal point, 0.1 degree). `geomag_factor` is NERC TPL-007's benchmark scaling factor
alpha = 0.001 x exp(0.115 x |latitude|), bounded 0.1 to 1 (Benchmark GMD Event Description,
Appendix II, eq. II.1, read in the NERC PDF; the benchmark field is 8 V/km x alpha x beta for a
1-in-100-year storm). Miami 0.1 (floor), Philadelphia 0.29, Seattle 0.44, Minot 0.63, Anchorage 1
(cap). Beta (earth conductivity, 0.2-1.2 across US earth models) is omitted: NERC publishes its
regions only as an image. Values are dipole geomagnetic latitudes, not corrected geomagnetic
coordinates (a degree or two different over the US).

### 13.3 Karst and landslide susceptibility (`ground`)

`karst_share`: share of the county's land on carbonate or evaporite rock at or near the surface
or under thin cover, from USGS OFR 2014-1156 (Weary and Doctor, *Karst in the United States*;
exposure classes E, B1 and B3; rock under more than 50 feet of glacial sediment, B2, left out:
5,272 carbonate and 122 evaporite polygons). The layers are published in Albers equal-area
projections (one per region); the job projects the Census 1:500k county outlines into each
layer's own plane (`geo::Albers`, checked against Snyder's worked example) and counts 1 km cells,
so every cell has the same area. The 282 MB archive is on disk only while it is read. Marion
County, FL 1.0, Warren County, KY 0.90, Philadelphia 0.003. Karst is where sinkholes can form, not
how often they do.

`landslide_susceptible_share`: the county table (`prop_susc`) of the USGS 2024 slope-relief
threshold landslide susceptibility model (doi:10.5066/P13KAGU3, CC0), matched to the county list
by state and name with an override table (Petersburg, Valdez-Cordova, Wade Hampton, La Salle
Parish, Shannon County SD, Bedford city) and Connecticut's retired counties converted by land
share. Utuado, PR 0.99, Miami-Dade 0.009. The model rates slope and relief generously (Los
Angeles County 0.59, Philadelphia 0.34); it caps the landslide footprint rather than predicting
slides. Neither source covers Guam, American Samoa or the Northern Mariana Islands; the landslide
table leaves out the US Virgin Islands.

### 13.4 Wildfire-smoke days (`smoke`)

Public-domain sources only (the brief's decision; Childs et al. 2022 is CC BY-SA). A county is
*smoke-covered* on a day when its internal point lies inside a Medium or Heavy NOAA HMS smoke
polygon (2,920 daily shapefiles 2016-2023; 2016-03-06 and 2016-11-12 are missing on the server).
Its PM2.5 for the day is the highest EPA AQS daily 24-hour average among its monitors (sample
durations `24 HOUR` and `24-HR BLK AVG`, rows with exceptional-event data excluded left out; sites
placed by their coordinates, since AQS still files Connecticut under the retired counties).
`smoke_days_35` / `smoke_days_55`: mean days a year that are smoke-covered with PM2.5 of at least
35.5 / 55.5 µg/m³ (the AQI's "unhealthy for sensitive groups" and "unhealthy" breakpoints).
Monitors do not all run daily, so each year's count is (observed smoke-covered days over the
threshold / observed smoke-covered days) x smoke-covered days. `smoke_basis`: `monitor` (641
counties with at least ten observed smoke-covered days), `imputed` (2,498: the rate of the
nearest such county within 150 km in the same NCA5 region, else the region's pooled rate,
applied to the county's own smoke-covered days), `hms_only` (86: no rate to borrow; counts are 0
where HMS never showed smoke and blank otherwise, for example Honolulu). `smoke_trend`:
least-squares slope of the yearly `smoke_days_35` estimate (days a year per year).
`hms_smoke_days`: mean smoke-covered days a year.

Missoula 7.6 days a year, Butte CA 8.3, Jackson OR 12.8, Cook IL 0.9, **Philadelphia 1.1** (the
brief's "0-ish": most of it is June 2023, when Canadian smoke pushed daily PM2.5 past 100),
Maricopa 0. New York County's June 2023 smoke is checked on every run. The window is the brief's
2016-2023; 2024 and 2025 are published and would be a one-constant change.

### 13.5 People behind levees (`levees`)

USACE National Levee Database: the public API's levee systems state by state (6,159 systems,
22.3 million people at risk from the National Structure Inventory, and each system's Levee
Screening and Assessment rating) and the "Leveed Areas" feature layer (6,145 polygons,
generalised to 0.0005 degrees on the server). Each system's people are split among counties by
where people live inside its leveed areas (Census 2020 block-group centres of population; 1,129
systems); where no block-group centre falls inside, by leveed-area land (a 0.005-degree lattice;
2,418 systems), then by an area's first vertex (805) or the system's point (11). Splitting by
land alone, as the audit proposed, put up to nine times a county's population into empty
Mississippi Delta counties; splitting by the county list overstated worse (Broward). Shares are
capped at 1 because nested systems count people twice (42 counties hit the cap, most in the lower
Mississippi valley, also Broward FL and Cass ND). `leveed_pop_share` = leveed people / county population (NRI 2020). `levee_risk_high_share`
= of those people, the share behind systems rated Very High or High; two thirds of systems are not
screened, so 0 often means "not rated". Sacramento 0.45 (0.94 of them behind high-risk levees),
St. Charles Parish 1.0, Apache County AZ about 0. Counties with no levee are 0, not missing.

### 13.6 Water systems with health-based violations (`water_systems`)

EPA ECHO: the weekly system file (active community water systems, `PWS_TYPE_CODE = CWS`,
`PWS_ACTIVITY_CODE = A`: 48,519 systems serving 326.9 million people) and the quarterly SDWA
archive's violations table (424 MB zipped, 4.1 GB inside; streamed from a temporary copy, never
extracted, deleted after reading). A system is flagged when it had a health-based violation
(`IS_HEALTH_BASED_IND = Y`: maximum contaminant level, maximum residual disinfectant level or
treatment technique) open at some time in the five years to the file's latest begin date
(2021-06-05 to 2026-06-05): 22,774 systems, 58.8 million people. Counting only violations that
*began* in the window (the audit's reading) gives 21,307 systems and 47.8 million people; it
misses long-open cases such as New York City's treatment-technique violation open since 2017
(the uncovered Hillview Reservoir). Each system's population is split over the counties it lists
in proportion to their residents (EPA publishes no split; an equal split gave Brooklyn 0.6 of its
people on public water); 859 systems (4.1 million people) list no county. `sdwis_violation_pop_share`
= flagged population / community-system population; `cws_pop_share` = community-system
population / county population, capped at 1. Hinds MS 0.87 (Jackson), Buncombe NC 0.94
(post-Helene), the five New York City boroughs 1.0, Philadelphia 0; 61 counties have no community
system on record. A compliance record, not a measure of how easily pipes break: use it as a
bounded multiplier. No boil-notice column: no bulk source exists.

### 13.7 Dams (`facilities`)

`dams_high_poor_condition` (county): NID High-hazard dams whose latest condition assessment is Poor
or Unsatisfactory (2,791; about a fifth of High dams are not rated). The county total of High dams
is the existing `high_hazard_dams` column, which the engine reads as `dams_high_total` (not
duplicated). `dams_high_within_10km_naming_town` (ZIP): High dams within 10 km of the ZIP's centre
whose NID `City` ("the nearest downstream city, town, or village that is most likely to be
affected by floods resulting from the failure of the dam") matches, by normalised name in the same
state, a Census 2020 place that holds at least 10% of the ZIP's land or has at least 10% of its
own land in the ZIP. 12,342 High dams name a town; 8,535 (69%) match a place; 5,184 ZIPs have at
least one (Oroville's ZIPs are checked on every run). The named town is chosen by dam owners and
states; it is not an inundation boundary (inundation maps are restricted), and hazard potential
rates the consequence of a failure, not its chance.

### 13.8 Storm-surge proxy (`surge_proxy`) and the optional surge pack (`surge`)

The core pack's county **proxy** (the brief's decision; data audit §5): `coastal_flood_pop_share`
= people in NRI v1.20 coastal-flood exposure areas / population, and `surge_proxy_class`:
`none` (not an NRI coastal-flood county, or tropical storms within 50 nmi fewer than 0.02 a year;
Guam, the Northern Mariana Islands and American Samoa use NRI's hurricane frequency, since
HURDAT2 does not cover them), `high` (at least 10% of residents in coastal-flood areas and at least
0.05 hurricanes a year), `moderate` (at least 2% and 0.1 tropical storms a year, or official
evacuation zones and 0.02 hurricanes a year), `low` otherwise. Official zones: statewide "know your
zone" tools in FL, GA, MD, NC, SC and VA; coastal-county tools in TX and LA; Baldwin and Mobile
(AL), Jackson (MS) and New York City (from the state registry research; unconfirmed rows left
out). Result: high 137 counties (9.0% of people), moderate 182 (13.3%), low 86 (6.4%), none 2,827
(71.3%); Miami-Dade and Galveston high, Cook IL and Maricopa none. It cannot place an address in or
out of a zone, and NRI's coastal-flood areas are smaller than NHC's Category 3 extent. No
elevation is used (§11).

The **optional pack** `surge` (job `surge`, `default: false`): NOAA/NHC National Storm Surge Risk
Maps (SLOSH Maximum of Maximums at high tide): Texas to Maine **version 2** (2016, 30 m cells),
Southern California v3 (mapped for Categories 1 and 2 only, so Category 2 stands in for 3 there),
Hawaii, Puerto Rico, the US Virgin Islands, Guam v3 and American Samoa v3. Version 4 for Texas to
Maine (June 2026, 10 m, reprocessed with 2025 SLOSH grids) was downloaded and tried: its GeoTIFFs
use Esri's LERC compression (TIFF code 34887), which the pure-Rust `tiff` crate cannot decode, so
the job reads v2 until a LERC decoder exists (one constant in `surge.rs`). The Category 1 and 3
GeoTIFFs are extracted one at a time to `data/raw/surge/` and read chunk by chunk (the `tiff`
crate: LZW, BigTIFF; world-file georeferencing; empty tiles skipped); each Census 2020 1:500k ZCTA
outline meeting a raster is sampled on a 3 arc-second lattice (about 90 m); a point is inundated
when its cell holds a depth class (1-21 feet bins). The share is the inundated points' area over
the ZIP's **land** area (Census 2024 Gazetteer), capped at 1: ZIP outlines include open water,
which the maps leave blank, so a share of all lattice points understated coastal ZIPs by half
(Miami Beach read 0.40 that way). Guam's map lies east of the antimeridian; its world file is
moved to the outlines' west-longitude convention so Guam's ZIPs meet it.
`opt/surge/zip_surge.csv`: `zip`, `surge_cat1_share`, `surge_cat3_share` (the Category 3 area
contains the Category 1 and 2 areas): 26,459 ZIPs whose outline meets a map, 3,252 with some
Category 3 area; 80.9 KB gzipped. ZIPs meeting no raster have no row: outside the mapped area,
not "no risk"; a row of zeros means the ZIP lies inside a map's rectangular extent (Texas to
Maine reaches far inland) and none of its land is in a surge area.
Examples (Category 3 / Category 1): Battery Park City 10280 0.99 / 0.40, Charleston 29401
0.96 / 0.60, Boston 02109 0.94 / 0.66, Galveston 77550 0.90 / 0.34, Miami Beach 33139
0.79 / 0.38, Virginia Beach 23451 0.78 / 0.11, Key West 33040 0.63 / 0.57, Waikiki 96815
0.53 / 0.39, central New Orleans 70112 0 (behind levees). NHC's terms: for general education and
awareness at a city or community level, never a replacement for official evacuation zones;
leveed areas (central New Orleans) are hatched in NHC's viewer, not mapped. Checked on every
build (all values reported together): Miami Beach at least 0.7, Key West at least 0.5, Galveston
at least 0.3, Denver outside every map, at least 3,000 ZIPs with Category 3 area, Guam's ZIPs
sampled. The Miami Beach floor was first set at 0.8 and lowered to 0.7 after the land-area run
read 0.793: generalised 1:500k outlines and a 90 m lattice lose a few percent along narrow
islands, and the check exists to catch a broken pipeline (0.40 or 0), not to grade the map.

### 13.9 Dust storms (`events`)

The Storm Events job now also counts `Dust Storm` (zone-based; `Dust Devil` is a different type
and not counted) as `dust_storm` rows in `events.csv`, by the same rules as the other Storm Events
types: 340 counties in 1996-2025 (Pinal AZ 4.0 a year, Maricopa 2.3, Riverside CA 1.4).

### 13.10 Eviction filings (`eviction`, gated)

Princeton University Eviction Lab, *Estimating Eviction Prevalence across the United States*,
county estimates 2000-2018 (deposited May 13, 2022). Licence: "This data is shared under the
terms of the Open Data Commons Attribution License (ODC-BY 1.0)" (data-downloads.evictionlab.org).
ODC-BY allows redistribution with attribution, but CLAUDE.md rule 5 asks for the owner's approval
first, so the job writes `core/eviction.csv` **only when** `manifest.json` has
`sign_offs.eviction_lab_odc_by.approved = true` (present, false, today). Until then it downloads
nothing and records why. To ship it: set `approved` to true and `by` to who approved and when,
then run `cargo run -p rr-etl -- refresh --out data --only eviction`. The credit line (also stored
in the manifest as `jobs.eviction.definitions.attribution_if_approved`), from the Lab's own "How
to cite":

> Eviction filing rates derived by Ready Reckoner from: Ashley Gromis, Ian Fellows, James R.
> Hendrickson, Lavar Edmonds, Lillian Leung, Adam Porton, and Matthew Desmond. Estimating Eviction
> Prevalence across the United States. Princeton University Eviction Lab.
> https://data-downloads.evictionlab.org/#estimating-eviction-prevalance-across-us/. Deposited May
> 13, 2022. This data is shared under the terms of the Open Data Commons Attribution License
> (ODC-BY 1.0).

`eviction_filing_rate` = mean over 2014-2018 of the Lab's `filings_estimate / renting_hh`
(Connecticut converted by land share). Tested in a scratch copy: 3,144 counties, 13.6 KB gzipped;
county mean 0.048; Baltimore County, MD 1.37 (filings can exceed one per renter household where
landlords file again and again). Most county-years are model estimates; filings are not completed
evictions; the data end in 2018.

### 13.11 What is not done

No boil-water notice column (no bulk source); no NERC beta; no elevation in the surge proxy; no
population-weighted surge shares (block data would be 3-4 GB); UASI footprints unverified; the
2026 revised NSHM grids and tract-level wildfire exposure are as before (§10, §11).

### 13.12 Optional packs (issue #15)

Files under `data/opt/<pack>/` belong to the pack `<pack>` in the manifest (`pack_of`); the web app
loads only `packs.core`, so optional packs cost nothing until a feature asks for them, and the core
budget counts `core/` only. `rr-data` accepts them (`DataStore::zip_record` merges `surge` and
`wildfire_places` into the ZIP record when loaded; `outage_events` answers
`DataStore::county_outage_events` and `DataStore::outage_holdout`).

| Pack | File | Rows | Size (gz) |
| --- | --- | ---: | ---: |
| `surge` | `opt/surge/zip_surge.csv` | 26,459 | 80.9 KB |
| `wildfire_places` | `opt/wildfire_places/places.csv` | 32,038 | 463.5 KB |
| `wildfire_places` | `opt/wildfire_places/zip_places.csv` | 35,568 | 223.2 KB |
| `outage_events` | `opt/outage_events/county_events.csv` | 29,878 | 356.0 KB |
| `outage_events` | `opt/outage_events/holdout.csv` | 130 | 1.6 KB |

An optional pack's credit line (NOAA NHC storm surge maps; USDA Forest Service Wildfire Risk to
Communities) is listed by `DataStore::attributions()` only once a file of that pack is loaded
(§7). Note that `rr plan` in the CLI loads every file the manifest lists, optional packs
included, while the web app and the goldens (`rr_plan::source::load_data_dir`) load the core
pack only.

- `surge`: `opt/surge/zip_surge.csv` (§13.8).
- `wildfire_places` (job `wildfire_places`, `default: false`): USDA Forest Service *Wildfire Risk to
  Communities* (2nd edition, tabular download of 2026-04-15; "can be used without additional
  permissions or fees", citation requested: "USDA Forest Service. 2026. Wildfire Risk to
  Communities. https://wildfirerisk.org"). `opt/wildfire_places/places.csv`: `place` (GEOID),
  `name`, `buildings_direct` / `buildings_indirect` (share of buildings directly / indirectly
  exposed), `risk_national_rank` (places marked insufficient data keep only their name);
  `opt/wildfire_places/zip_places.csv`: `zip`, `place`, `zip_land_share` (Census 2020 ZCTA-to-place,
  parts under 1% dropped). ZIP land outside every place has no place value. Checked on every build:
  Paradise, CA mostly directly exposed; New York City about 0.
- `outage_events` (written by the default job `outage_model` alongside its core files, so it is
  rebuilt by every refresh): every county outage of a day or more with its restoration curve and
  cause, and the outage model's held-out test (§2, "Data-pack v2 calibration files"; §5.4). It has
  no credit line of its own: it is built from EAGLE-I outage records matched to NOAA storm records
  and the PNNL OE-417 linkage, all credited with the core pack (§7).

### 13.13 Citation ids for the exposure values

`LocationResolved.exposure` carries these ids (`rr_data::EXPOSURE_SOURCES`). `usace_nld` and
`fema_hsgp_fy2026` are in `content/citations.toml`; the others are requested from the content
workstream:

| id | Title | Publisher, year | URL |
| --- | --- | --- | --- |
| `rr_strategic_sites` | Strategic sites and county strategic-exposure classes (compiled list, method in docs/DATA_SOURCES.md §13.1) | Ready Reckoner, 2026 | https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/crates/rr-etl/data/strategic_sites.toml |
| `nhc_storm_surge_maps` | National Storm Surge Risk Maps (SLOSH MOM): Texas to Maine version 2, regional maps for Southern California, Hawaii, Puerto Rico, the US Virgin Islands, Guam and American Samoa | NOAA National Hurricane Center, 2016 (Texas to Maine v2) | https://www.nhc.noaa.gov/nationalsurge/ |
| `rr_surge_proxy` | County storm-surge proxy (NRI coastal-flood exposure, HURDAT2 passages, evacuation-zone tools; method in docs/DATA_SOURCES.md §13.8) | Ready Reckoner, 2026 | https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/docs/DATA_SOURCES.md |
| `epa_aqs_daily_pm25` | Air Quality System pre-generated daily PM2.5 data (FRM/FEM, parameter 88101), with NOAA HMS smoke polygons for smoke days | U.S. EPA, 2026 | https://aqs.epa.gov/aqsweb/airdata/download_files.html |
| `usace_nid` | National Inventory of Dams | U.S. Army Corps of Engineers, 2026 | https://nid.sec.usace.army.mil/ |
| `usgs_karst_2014` | Karst in the United States: A Digital Map Compilation and Database (Open-File Report 2014-1156) | Weary and Doctor, USGS, 2014 | https://pubs.usgs.gov/of/2014/1156/ |
| `usgs_landslide_2024` | Slope-Relief Threshold Landslide Susceptibility Models for the United States and Puerto Rico | USGS, 2024 | https://doi.org/10.5066/P13KAGU3 |
| `epa_echo_sdwa` | ECHO Safe Drinking Water Act data downloads | U.S. EPA, 2026 | https://echo.epa.gov/tools/data-downloads/sdwa-download-summary |
| `nerc_tpl007_gmd` | Benchmark Geomagnetic Disturbance Event Description (TPL-007), with IGRF-14 for geomagnetic latitude | NERC, 2014 | https://www.nerc.com/globalassets/standards/projects/2013-03/benchmark_gmd_event_aug27_clean.pdf |
| `eviction_lab_county_estimates` | Estimating Eviction Prevalence across the United States (county estimates 2000-2018) | Princeton University Eviction Lab, 2022 | https://data-downloads.evictionlab.org/ |
