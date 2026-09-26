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
