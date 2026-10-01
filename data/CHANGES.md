# Data pack changes

Each `rr-etl refresh` appends a section: which files changed and how many rows were added, removed or changed (by key). Provenance for every file is in `manifest.json`.

## 2026-09-26T03:43:09Z — pack version e8b8cd6861e6 (initial build)

Jobs run: geography, nri, outages, events, seismic, climate, flood, facilities, vulnerability, base_rates. The full refresh ran on 2026-09-26; seismic (moved to the USGS gridded releases), outages (Puerto Rico as one island-wide series), climate (+3 °C `_high` ratios and CMRA counts; `climate_levels.csv` dropped) and base_rates (registry citation ids) were then re-run with fixes, and the other jobs re-run with the final code reproduced their files byte for byte.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/base_rates.toml` | 0 | 27 | 27 | 0 | 0 | new |
| `core/climate.csv` | 0 | 3231 | 3231 | 0 | 0 | new |
| `core/counties.csv` | 0 | 3232 | 3232 | 0 | 0 | new |
| `core/ct_crosswalk.csv` | 0 | 19 | 19 | 0 | 0 | new |
| `core/events.csv` | 0 | 44057 | 44057 | 0 | 0 | new |
| `core/facilities.csv` | 0 | 3232 | 3232 | 0 | 0 | new |
| `core/flood.csv` | 0 | 3144 | 3144 | 0 | 0 | new |
| `core/nri_counties.csv` | 0 | 3232 | 3232 | 0 | 0 | new |
| `core/nri_hazards.csv` | 0 | 45853 | 45853 | 0 | 0 | new |
| `core/nri_semantics.toml` | 0 | 18 | 18 | 0 | 0 | new |
| `core/outages.csv` | 0 | 3153 | 3153 | 0 | 0 | new |
| `core/outages_state.csv` | 0 | 53 | 53 | 0 | 0 | new |
| `core/seismic.csv` | 0 | 3225 | 3225 | 0 | 0 | new |
| `core/states.csv` | 0 | 56 | 56 | 0 | 0 | new |
| `core/vulnerability.csv` | 0 | 3144 | 3144 | 0 | 0 | new |
| `core/zip_centroids.csv` | 0 | 33791 | 33791 | 0 | 0 | new |
| `core/zip_county.csv` | 0 | 46772 | 46772 | 0 | 0 | new |
| `core/zip_facilities.csv` | 0 | 33791 | 33791 | 0 | 0 | new |
| `geo/counties.json` | 0 | 3222 | 3222 | 0 | 0 | new |

## 2026-09-26T08:24:54Z — pack version 25f3ed156688

Jobs run: geography, nri, facilities.

A trim; every source is unchanged (same checksums as the initial build). `nri_hazards.csv` drops
the `expb`, `ealb` and `alrb` columns, which no crate reads (every other value is identical);
`zip_centroids.csv` leaves the core pack (nothing in the engine read it; the facilities job now
reads the Census Gazetteer ZIP points itself, and `zip_facilities.csv` comes out byte-identical).
The core pack goes from 2.99 MB to 2.37 MB gzipped file by file (10.8 MB to 8.8 MB uncompressed);
a first visit fetches 1.99 MB of it instead of 2.36 MB.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/counties.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/ct_crosswalk.csv` | 19 | 19 | 0 | 0 | 0 | unchanged |
| `core/facilities.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/nri_counties.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/nri_hazards.csv` | 45853 | 45853 | 0 | 0 | 45853 | changed |
| `core/nri_semantics.toml` | 18 | 18 | 0 | 0 | 0 | unchanged |
| `core/states.csv` | 56 | 56 | 0 | 0 | 0 | unchanged |
| `core/zip_centroids.csv` | 33791 | 0 | 0 | 33791 | 0 | removed |
| `core/zip_county.csv` | 46772 | 46772 | 0 | 0 | 0 | unchanged |
| `core/zip_facilities.csv` | 33791 | 33791 | 0 | 0 | 0 | unchanged |
| `geo/counties.json` | 3222 | 3222 | 0 | 0 | 0 | unchanged |

## 2026-09-26T16:29:39Z — pack version c535534b0e6c

Jobs run: strategic.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/strategic.csv` | 0 | 3232 | 0 | 0 | 0 | new |
| `core/strategic_sites.toml` | 0 | 207 | 0 | 0 | 0 | new |

## 2026-09-26T16:39:49Z — pack version d886671badb4

Jobs run: geomag.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/geomag.csv` | 0 | 3232 | 0 | 0 | 0 | new |

## 2026-09-26T16:40:55Z — pack version faefbb31739f

Jobs run: geomag.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/geomag.csv` | 3232 | 3232 | 0 | 0 | 3187 | changed |

## 2026-09-26T16:50:06Z — pack version b5d9ae2bb099

Jobs run: ground.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/ground.csv` | 0 | 3225 | 0 | 0 | 0 | new |

## 2026-09-26T16:54:49Z — pack version b5d9ae2bb099

Jobs run: none.
Job **levees failed**: unexpected source data: leveed_pop_share for Apache County, AZ is 0.000, expected 0-0

## 2026-09-26T16:56:30Z — pack version 8f382775ca36

Jobs run: levees.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/levees.csv` | 0 | 3232 | 0 | 0 | 0 | new |

## 2026-09-26T16:59:28Z — pack version 99d062d2414a

Jobs run: levees.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/levees.csv` | 3232 | 3232 | 0 | 0 | 198 | changed |

## 2026-09-26T17:04:10Z — pack version 98f7c8387c2e

Jobs run: water_systems.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/water_systems.csv` | 0 | 3171 | 0 | 0 | 0 | new |

## 2026-09-26T17:11:51Z — pack version 2a85e3300602

Jobs run: water_systems.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/water_systems.csv` | 3171 | 3171 | 0 | 0 | 325 | changed |

## 2026-09-26T17:15:57Z — pack version e620dc2a2dca

Jobs run: water_systems.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/water_systems.csv` | 3171 | 3171 | 0 | 0 | 243 | changed |

## 2026-09-26T17:23:18Z — pack version 574f5260b3c4

Jobs run: smoke.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/smoke.csv` | 0 | 3225 | 0 | 0 | 0 | new |

## 2026-09-26T17:27:34Z — pack version d94a8c455c64

Jobs run: smoke, surge_proxy.
Job **facilities failed**: unexpected source data: no Oroville ZIP (95965, 95966) is counted downstream of Oroville Dam

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/smoke.csv` | 3225 | 3225 | 0 | 0 | 84 | changed |
| `core/surge_proxy.csv` | 0 | 3232 | 0 | 0 | 0 | new |

## 2026-09-26T17:30:02Z — pack version ee75b4a645b3

Jobs run: facilities.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/facilities.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/zip_facilities.csv` | 33791 | 33791 | 0 | 0 | 33791 | changed |

## 2026-09-26T17:32:17Z — pack version a5b6c9d722cc

Jobs run: facilities.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/facilities.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/zip_facilities.csv` | 33791 | 33791 | 0 | 0 | 6173 | changed |

## 2026-09-26T17:34:32Z — pack version a5b6c9d722cc

Jobs run: eviction.

## 2026-09-26T17:38:48Z — pack version 1c2677cdba4a

Jobs run: events.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/events.csv` | 44057 | 44397 | 340 | 0 | 0 | changed |

## 2026-09-26T17:53:48Z — pack version 9ade1a0dc473

Jobs run: wildfire_places.
Job **surge failed**: unexpected source data: GeoTIFF chunk 1498578: unsupported error: compression method Unknown(34887) is unsupported

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `opt/wildfire_places/places.csv` | 0 | 32038 | 0 | 0 | 0 | new |
| `opt/wildfire_places/zip_places.csv` | 0 | 35568 | 0 | 0 | 0 | new |

## 2026-09-26T18:01:12Z — pack version 9ade1a0dc473

Jobs run: none.
Job **surge failed**: unexpected source data: Category 3 surge share for Miami Beach, FL (33139) is 0.402, expected more than 0.8

## 2026-09-26T18:08:20Z — pack version 9ade1a0dc473

Jobs run: none.
Job **surge failed**: unexpected source data: Category 3 surge share for Miami Beach, FL (33139) is 0.793, expected more than 0.8

## 2026-09-26T18:14:10Z — pack version 16841a150ad0

Jobs run: surge.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `opt/surge/zip_surge.csv` | 0 | 26452 | 0 | 0 | 0 | new |

## 2026-09-26T18:19:44Z — pack version db145d8577c3

Jobs run: surge.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `opt/surge/zip_surge.csv` | 26452 | 26459 | 7 | 0 | 0 | changed |

## 2026-09-26T18:20:10Z — pack version ed4e59dff15a

Jobs run: wildfire_places.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `opt/wildfire_places/places.csv` | 32038 | 32038 | 0 | 0 | 7701 | changed |
| `opt/wildfire_places/zip_places.csv` | 35568 | 35568 | 0 | 0 | 17582 | changed |

## 2026-09-26T18:39:12Z — pack version 615b12d602e8

Jobs run: strategic.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/strategic.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/strategic_sites.toml` | 207 | 207 | 0 | 0 | 207 | changed |

## 2026-09-26T18:42:20Z — pack version 0fdc6e19e670

Jobs run: strategic.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/strategic.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/strategic_sites.toml` | 207 | 207 | 0 | 0 | 207 | changed |

## 2026-09-26T18:49:36Z — pack version 685e9975f3ce

Jobs run: strategic.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/strategic.csv` | 3232 | 3232 | 0 | 0 | 3232 | changed |
| `core/strategic_sites.toml` | 207 | 207 | 0 | 0 | 0 | unchanged |

## 2026-09-26T19:26:12Z — pack version 2fb3538d5291

Jobs run: outages, events, series, outage_model, climate_daily, reliability, displacement
(data-model workstream, v0.2.0). One consolidated entry against the previous pack: the jobs were
run together on 2026-09-26, `displacement` again without the zero-filled damage columns, and the
six outage-dependent jobs (outages, events, series, outage_model, climate_daily, reliability) again
with the per-customer floor and the stricter dropout rule (docs/DATA_SOURCES.md §5.1); the rows
below compare with the pack before this workstream.

- `outages.csv` / `outages_state.csv`: EAGLE-I reporting dropouts repaired (utilities missing from
  a scrape for minutes to three days, doubled reports; docs/DATA_SOURCES.md §5.1) and the event
  curve used as a floor on the per-customer tail. Outage counts fall where flicker had inflated them
  and multi-day shares rise (Buncombe NC: 10.06 → 0.96 outages per customer-year, week-long share
  0% → 7%; Harris TX 1.06 → 0.23, three-day share 3% → 21%). New column `p_ge_30d`;
  `share_customer_hours_in_events` is now recorded hours in events over all recorded hours.
- `events.csv`: Storm Events times now read in UTC from `CZ_TIMEZONE`; only 8 county-types whose
  episodes spanned two time zones change their durations.
- New core files: the regional outage model (`outage_pooled`, `outage_causes`, `outage_curves`,
  `outage_stress`), `temperature` (nClimGrid-Daily), `reliability` (EIA-861), `declarations`
  (OpenFEMA) and seven national series under `core/series/`. New optional pack `outage_events`.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/declarations.csv` | 0 | 3232 | 3232 | 0 | 0 | new |
| `core/events.csv` | 44057 | 44057 | 0 | 0 | 8 | changed |
| `core/outage_causes.csv` | 0 | 3153 | 3153 | 0 | 0 | new |
| `core/outage_curves.csv` | 0 | 102 | 102 | 0 | 0 | new |
| `core/outage_pooled.csv` | 0 | 3209 | 3209 | 0 | 0 | new |
| `core/outage_stress.csv` | 0 | 3186 | 3186 | 0 | 0 | new |
| `core/outages.csv` | 3153 | 3153 | 0 | 0 | 3153 | changed (new columns: p_ge_30d) |
| `core/outages_state.csv` | 53 | 53 | 0 | 0 | 53 | changed (new columns: p_ge_30d) |
| `core/reliability.csv` | 0 | 3174 | 3174 | 0 | 0 | new |
| `core/series/drug_shortages.toml` | 0 | 14 | 14 | 0 | 0 | new |
| `core/series/fbi_arrests.toml` | 0 | 70 | 70 | 0 | 0 | new |
| `core/series/fcc_dirs.toml` | 0 | 19 | 19 | 0 | 0 | new |
| `core/series/fdic_failures.toml` | 0 | 92 | 92 | 0 | 0 | new |
| `core/series/funding_gaps.toml` | 0 | 27 | 27 | 0 | 0 | new |
| `core/series/ihp_displacement.toml` | 0 | 22 | 22 | 0 | 0 | new |
| `core/series/oe417.toml` | 0 | 288 | 288 | 0 | 0 | new |
| `core/temperature.csv` | 0 | 3107 | 3107 | 0 | 0 | new |
| `opt/outage_events/county_events.csv` | 0 | 29878 | 29878 | 0 | 0 | new |
| `opt/outage_events/holdout.csv` | 0 | 130 | 130 | 0 | 0 | new |

## 2026-09-26T21:14:06Z — pack version 01a46abb2d5d

No job run: `rr-etl manifest --rehash` recomputed the checksums, row counts and pack version from the files on disk. Merge of the data-model workstream into v0.2 (v0.2.0): the pack now holds both workstreams' files (the two sections above). `core/events.csv` is data-model's table (Storm Events times in UTC, 8 values changed) plus data-hazard's 340 `dust_storm` rows, so it matched neither side's checksum; every other file is byte-identical to one side's.

| File | Rows before | Rows after | sha256 before | sha256 after |
|---|---:|---:|---|---|
| `core/events.csv` | 44057 | 44397 | `842d6c2dae85` | `1366f4179fc6` |

## 2026-10-01T12:40:57Z — pack version ad9c33416bad

Jobs run: geography, nri, outages, events, seismic, climate, flood, strategic, geomag, ground, levees, water_systems, smoke, facilities, surge_proxy, eviction, vulnerability, base_rates, series, outage_model, climate_daily, reliability, displacement.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/base_rates.toml` | 27 | 27 | 0 | 0 | 0 | unchanged |
| `core/climate.csv` | 3231 | 3231 | 0 | 0 | 0 | unchanged |
| `core/counties.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/ct_crosswalk.csv` | 19 | 19 | 0 | 0 | 0 | unchanged |
| `core/declarations.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/events.csv` | 44397 | 44397 | 0 | 0 | 0 | unchanged |
| `core/facilities.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/flood.csv` | 3144 | 3144 | 0 | 0 | 0 | unchanged |
| `core/geomag.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/ground.csv` | 3225 | 3225 | 0 | 0 | 0 | unchanged |
| `core/levees.csv` | 3232 | 3232 | 0 | 0 | 3 | changed |
| `core/nri_counties.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/nri_hazards.csv` | 45853 | 45853 | 0 | 0 | 0 | unchanged |
| `core/nri_semantics.toml` | 18 | 18 | 0 | 0 | 0 | unchanged |
| `core/outage_causes.csv` | 3153 | 3153 | 0 | 0 | 0 | unchanged |
| `core/outage_curves.csv` | 102 | 102 | 0 | 0 | 0 | unchanged |
| `core/outage_pooled.csv` | 3209 | 3209 | 0 | 0 | 0 | unchanged |
| `core/outage_stress.csv` | 3186 | 3186 | 0 | 0 | 0 | unchanged |
| `core/outages.csv` | 3153 | 3153 | 0 | 0 | 0 | unchanged |
| `core/outages_state.csv` | 53 | 53 | 0 | 0 | 0 | unchanged |
| `core/reliability.csv` | 3174 | 3174 | 0 | 0 | 0 | unchanged |
| `core/seismic.csv` | 3225 | 3225 | 0 | 0 | 0 | unchanged |
| `core/series/drug_shortages.toml` | 14 | 14 | 0 | 0 | 14 | changed |
| `core/series/fbi_arrests.toml` | 70 | 70 | 0 | 0 | 70 | changed |
| `core/series/fcc_dirs.toml` | 19 | 19 | 0 | 0 | 0 | unchanged |
| `core/series/fdic_failures.toml` | 92 | 92 | 0 | 0 | 92 | changed |
| `core/series/funding_gaps.toml` | 27 | 27 | 0 | 0 | 0 | unchanged |
| `core/series/ihp_displacement.toml` | 22 | 22 | 0 | 0 | 22 | changed |
| `core/series/oe417.toml` | 288 | 288 | 0 | 0 | 288 | changed |
| `core/smoke.csv` | 3225 | 3225 | 0 | 0 | 0 | unchanged |
| `core/states.csv` | 56 | 56 | 0 | 0 | 0 | unchanged |
| `core/strategic.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/strategic_sites.toml` | 207 | 207 | 0 | 0 | 0 | unchanged |
| `core/surge_proxy.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/temperature.csv` | 3107 | 3107 | 0 | 0 | 0 | unchanged |
| `core/vulnerability.csv` | 3144 | 3144 | 0 | 0 | 0 | unchanged |
| `core/water_systems.csv` | 3171 | 3171 | 0 | 0 | 0 | unchanged |
| `core/zip_county.csv` | 46772 | 46772 | 0 | 0 | 0 | unchanged |
| `core/zip_facilities.csv` | 33791 | 33791 | 0 | 0 | 0 | unchanged |
| `geo/counties.json` | 3222 | 3222 | 0 | 0 | 0 | unchanged |
| `opt/outage_events/county_events.csv` | 29878 | 29878 | 0 | 0 | 0 | unchanged |
| `opt/outage_events/holdout.csv` | 130 | 130 | 0 | 0 | 0 | unchanged |
