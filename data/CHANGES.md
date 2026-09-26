# Data pack changes

Each `rr-etl refresh` appends a section: which files changed and how many rows were added, removed or changed (by key). Provenance for every file is in `manifest.json`.

## 2026-09-26T00:21:36Z — pack version e3b0c44298fc

Jobs run: none.
Job **geography failed**: unexpected source data: Connecticut crosswalk should link 8 old counties to 9 planning regions, found 17 and 18

## 2026-09-26T00:22:20Z — pack version e3b0c44298fc

Jobs run: none.
Job **geography failed**: unexpected source data: ZIP 96799 points at county 60040, which is not in the 2024 county list

## 2026-09-26T00:22:51Z — pack version 9cea7a0e28c8

Jobs run: geography.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/counties.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/ct_crosswalk.csv` | 19 | 19 | 0 | 0 | 0 | unchanged |
| `core/states.csv` | 56 | 56 | 0 | 0 | 0 | unchanged |
| `core/zip_centroids.csv` | 0 | 33791 | 0 | 0 | 0 | new |
| `core/zip_county.csv` | 0 | 46772 | 0 | 0 | 0 | new |
| `geo/counties.json` | 0 | 3222 | 0 | 0 | 0 | new |

## 2026-09-26T00:23:16Z — pack version 9cea7a0e28c8

Jobs run: geography.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/counties.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/ct_crosswalk.csv` | 19 | 19 | 0 | 0 | 0 | unchanged |
| `core/states.csv` | 56 | 56 | 0 | 0 | 0 | unchanged |
| `core/zip_centroids.csv` | 33791 | 33791 | 0 | 0 | 0 | unchanged |
| `core/zip_county.csv` | 46772 | 46772 | 0 | 0 | 0 | unchanged |
| `geo/counties.json` | 3222 | 3222 | 0 | 0 | 0 | unchanged |

## 2026-09-26T00:23:53Z — pack version 3d0e0f865336

Jobs run: geography.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/counties.csv` | 3232 | 3232 | 0 | 0 | 0 | unchanged |
| `core/ct_crosswalk.csv` | 19 | 19 | 0 | 0 | 0 | unchanged |
| `core/states.csv` | 56 | 56 | 0 | 0 | 0 | unchanged |
| `core/zip_centroids.csv` | 33791 | 33791 | 0 | 0 | 33459 | changed |
| `core/zip_county.csv` | 46772 | 46772 | 0 | 0 | 0 | unchanged |
| `geo/counties.json` | 3222 | 3222 | 0 | 0 | 0 | unchanged |

## 2026-09-26T00:26:38Z — pack version 3d0e0f865336

Jobs run: none.
Job **nri failed**: unexpected source data: NRI data dictionary no longer lists field DRGT_EXPB

## 2026-09-26T00:27:16Z — pack version c11c8a555ab8

Jobs run: nri.

| File | Rows before | Rows after | Added | Removed | Changed | Status |
|---|---:|---:|---:|---:|---:|---|
| `core/nri_counties.csv` | 0 | 3232 | 0 | 0 | 0 | new |
| `core/nri_hazards.csv` | 0 | 45853 | 0 | 0 | 0 | new |
| `core/nri_semantics.toml` | 0 | 18 | 0 | 0 | 0 | new |
