# Sample counties added in v0.2.0

The engine plans without any data pack on fourteen built-in sample counties (`rr_plan::FixtureSource`,
used by the tests, by `rr --fixtures`, and by the web app until the national data has loaded). The
first seven, the counties of the v0.1 fixture households, are rr-hazards' hand-built test inputs in
`crates/rr-hazards/tests/data/counties/`. The seven here are the counties of the households added in
v0.2.0, and each is the core data pack's own record for that county, not a hand-built one.

| File | County | Household | ZIP code |
| --- | --- | --- | --- |
| `06067.json` | Sacramento, California | `sacramento-leveed-2` | 95834 |
| `22023.json` | Cameron Parish, Louisiana | `cameron-insulin-well-farm-2` | 70631 |
| `26163.json` | Wayne, Michigan | `detroit-snap-3` | 48227 |
| `30063.json` | Missoula, Montana | `missoula-smoke-2` | 59801 |
| `38101.json` | Ward, North Dakota | `minot-missile-field-3` | 58701 |
| `48167.json` | Galveston, Texas | `galveston-highrise-1` | 77550 |
| `72127.json` | San Juan, Puerto Rico | `san-juan-2` | 00907 |

Each file is `{ "county": CountyRecord, "location": LocationResolved }`, the same shape as the
hand-built ones: the county record exactly as `DataStore::county` returns it from core pack
`01a46abb2d5d` (2026-09-26), and the location as `DataStore::location` resolves the household's ZIP
code, with the ZIP code's county share and the data note left out (the sample source sets both; each
of these ZIP codes lies wholly in its county in the pack's ZIP table). The sources of every field are
the core pack's, listed in `data/manifest.json` and `docs/DATA_SOURCES.md`.

A plan made on the sample counties says so ("The engine is running on fourteen built-in sample
counties…"); the goldens in `fixtures/golden/` are planned from the data packs, not from these files.
