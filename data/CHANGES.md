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
