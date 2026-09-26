# Test data for `rr-hazards`

Hand-built inputs for the seven fixture households, in the shape the `core` data pack will have.
They let `rr-hazards` be tested before `rr-data` lands; they are not the data pack.

`counties/<fips>.json` holds `{ "county": CountyRecord, "location": LocationResolved }`.

| Field | Source |
| --- | --- |
| `nri.*` (AFREQ, exposure, loss ratios, expected annual losses, risk score) | FEMA National Risk Index v1.20 county table (research input `nri_counties.csv`, retrieved 2026-09-25), rounded to 4 significant figures as the pack will be. `afreq_kind` follows the NRI v1.20 update documentation (earthquake, landslide, volcanic activity and wildfire are annualised probabilities). |
| `climate.*` | CMRA 2025 county table (LOCA2 multi-model mean; `_hist` historical, `_2050` SSP2-4.5 mid-century, `_2050_high` SSP5-8.5 mid-century), 4 significant figures. |
| `outages` (Philadelphia, Coos only) | Household outage-event rate and fitted lognormal (median, p90) from research risk-model §8.2 and §9.2 (EAGLE-I 2018–2025); the `p_ge_*` shares are computed from that lognormal. |
| `population`, `building_value_usd` | NRI v1.20. |
| `households` | **Approximate**: population ÷ 2.5. The pack will carry Census counts. |
| `centroid` | **Approximate** county centres. Not used in any calculation. |
| `facilities` | **Approximate test inputs**, not facts: distance from the county centre to the nearest nuclear plant (Philadelphia–Limerick ≈ 35 km, Miami-Dade–Turkey Point ≈ 26 km, Maricopa–Palo Verde ≈ 35 km, Cook–Dresden ≈ 63 km, Fort Bend–South Texas Project ≈ 85 km), and rough TRI and dam counts. |

| `exposure` (contract v2) | The data pack v2 exposure columns (`CountyExposure`), copied from agent/data-hazard's own pack rows of 2026-09-26: `strategic.csv` (class, sites, distance and bearing, UASI county share and area), `geomag.csv`, `ground.csv` (karst, landslide terrain), `levees.csv`, `water_systems.csv` and `smoke.csv`; the resolved `strategic_places` follow `strategic_sites.toml`. `dams_high_total` repeats the `facilities` count. Eviction and storm-surge columns are left out (not built yet). |
| `events.dust_storm` (Maricopa only) | **Approximate test input**: 2 dust-storm episodes a year, until data-hazard's Storm Events "Dust Storm" rows land. |

`base_rates.json` holds the national personal and societal rates from research §6.1 and
data-sources §8, each with its source id.

`backtest/` holds the 22 households of the round-2 backtest (`round2/inputs/backtest` in the
briefs, the model review's frozen set of event and household pairs). `tests/pack.rs` runs them,
and every county, against the national data pack in `data/`.
