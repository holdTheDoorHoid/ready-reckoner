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
