# Public data sources for a client-side disaster-preparedness planner (US first, short global section)

Research date: 2026-09-25. Every URL, size and field below was checked with live requests from a Linux host that day: HTTP HEAD/GET, API calls, file downloads, and fetches of documentation pages. Numbers marked **measured** come from files I downloaded and processed in this session. Anything I could not confirm is marked **UNVERIFIED**.

Legend
- **PD**: US Government work, no copyright (17 U.S.C. 105). Terms of use can still apply, as with NRI and OpenFEMA.
- **CORS \***: the endpoint returned `Access-Control-Allow-Origin: *`, so a static site can fetch it directly. **CORS (reflect)**: the endpoint echoes the Origin header back, which also allows the fetch. **No CORS**: no such header came back, so the browser will block a direct fetch. Use build-time precompute or a proxy.
- Several agency web pages (fema.gov file paths, cdc.gov HTML, nrc.gov, eia.gov, nhtsa.gov, poweroutage.us) answered **403/503 to scripted clients** even though a browser can open them. For build-time ETL that means relying on their APIs, ArcGIS services, or mirrors, and caching raw inputs.

---

## 0. Findings that change the design (read first)

1. **NRI moved and changed version.** Every `https://hazards.fema.gov/nri/*` URL, including the old ZIP download paths, now returns a 301 redirect to FEMA's RAPT page. The current release is **v1.20.0, labelled "December 2025"**. Official bulk ZIPs now live under OpenFEMA at `https://www.fema.gov/about/reports-and-data/openfema/nri/v120/`. fema.gov's CDN answered 403 to curl for those ZIPs. The ArcGIS REST services and the ArcGIS Hub export API both work and send `CORS *`.
2. **NRI v1.20 is not comparable with v1.19 or older blog posts.** Riverine Flooding (`RFLD`) became **Inland Flooding (`IFLD`)**, which models pluvial and fluvial flooding and now treats about 100% of a county's building value as exposed. **Social Vulnerability now comes from Census Community Resilience Estimates**, not CDC SVI. Wildfire now uses USFS FSim, landslide uses the new USGS 90 m susceptibility model, and tsunami and earthquake were overhauled. The implied value of statistical life is **$13.7M** (measured). Connecticut is represented by **9 planning regions (FIPS 09110–09190)**.
3. **NRI terms are more than "public domain".** The ArcGIS item Terms & Conditions forbid reverse engineering or deriving underlying data. They require a citation and a "not endorsed by FEMA" disclaimer that names the dataset version and access date. They also let FEMA rescind use and ask users to destroy copies. Have someone review this before committing raw NRI tables to an open-source repository. Showing the required disclaimer in the app is mandatory.
4. **The Census Data API now requires a key on every data request.** An unkeyed call to `api.census.gov/data/2024/acs/acs5?...` returns a 302 to `missing_key.html`. The metadata endpoints still work without a key. A browser-only app cannot use the API without exposing the key, so ACS must be precomputed at build time with the key held as a CI secret.
5. **Useful endpoints without CORS:** Census Geocoder (JSONP only), NOAA Atlas 14 PFDS point service, US Drought Monitor data API, and the ThinkHazard! JSON API. These must be precomputed.
6. **Federal sources that were removed or moved:** HIFLD Open shut down in August 2025; archives are listed in section 5. EPA EJScreen was removed on 2025-02-05; epa.gov/ejscreen and gaftp.epa.gov/EJScreen both return 404, and a PEDP mirror exists. globalchange.gov, including the NCA5 site and the NCA Atlas front end, was **unreachable**. The **NCA5 Atlas ArcGIS services are still live and licensed CC BY 4.0**. Two NOAA sea-level-rise technical-report pages returned 404.
7. **A ZIP code does not map to one county.** Of 33,791 ZCTAs, 10,186 (**30.1%**, measured) span more than one county. The UI has to let the user pick a county when the ZIP is ambiguous. Also, USPS ZIPs and Census ZCTAs are not the same thing.
8. **County codes differ across datasets.** NRI v1.20 and the Census 2024 cartographic boundaries use the CT planning regions (`091xx`); I verified both. ACS 2022+ is expected to use them too (not re-checked). The Census 2020 ZCTA–county relationship file and CMRA (2019 counties) use the old CT counties (`09001`–`09015`); I confirmed 47 rows with `09001` and none with `0911x` in the relationship file. A CT crosswalk is required.
9. **The NFHL cannot be bundled.** It has **5,806,971 flood-hazard polygons**, 1,371,296 of them in the Special Flood Hazard Area (counted via REST). A per-point query works and the server allows cross-origin requests, but it sends the user's coordinates to FEMA.
10. **Raw sizes force a build-time ETL:**

    | Dataset | Raw size |
    |---|---|
    | NRI tract CSV | 465 MB (152 MB gzipped) |
    | NRI county GeoJSON from the Hub export | **557 MB** |
    | EAGLE-I outage data | 11.6 GB |
    | USFS wildfire rasters | 5.6–33.7 GB each |
    | USGS NSHM CONUS hazard curves | 934 MB |

---

## 1. FEMA National Risk Index (NRI): the core per-county hazard source

### 1.1 Status

| Item | Verified value |
|---|---|
| Version | **1.20.0 "December 2025"**. Sources: `NRI_VER` field = "December 2025"; data dictionary Version Date = December 2025; metadata "Last update 2025-12-12"; ArcGIS services last edited 2025-12-16/18; OpenFEMA page says v1.20 (Dec 2025), "Last Updated January 6, 2026". |
| Release-date conflict | FEMA's "Data Version and Update Documentation" (Dec 2025 PDF, which a parallel agent had fetched into this scratchpad) says v1.20.0 was publicly released on June 25, 2025. That conflicts with the December 2025 label, so the release date is **UNVERIFIED**. |
| Version history (same doc) | v1.17.0 Nov 2020, v1.18.0 Aug 2021, v1.18.1 Nov 2021, v1.19.0 Mar 2023, v1.20.0 Dec 2025 |
| Update cadence | "irregular" (metadata), roughly every 1–2.5 years |
| Records | **3,232 counties and 85,154 census tracts** (REST `returnCountOnly`). Tribal county and tract layers also exist. |
| Geography | 2021 TIGER/Line (tracts use 2020 definitions); Connecticut uses 2024 TIGER (planning regions) |
| Exposure basis | Building value from Hazus 6.0 and population from the 2020 Census, both in Dec 2024 dollars. Agriculture from the 2017 Census of Agriculture. |
| Methodology doc | https://www.fema.gov/sites/default/files/documents/fema_national-risk-index_technical-documentation.pdf (over 10 MB). RAPT user guide: https://www.fema.gov/sites/default/files/documents/fema_national-risk-index-rapt-user-guide_2025.pdf |
| Data dictionary | CSV with 479 field rows: https://fema.maps.arcgis.com/sharing/rest/content/items/4b9db412e99542029b3c37c37ad714bb/data. Metadata PDF: `.../items/e77b087b362f42d68ddbdbcab1c29d00/data`. Glossary: `.../items/6835fa58ad5646878860145288c7cc39/data` |
| Landing pages | https://www.fema.gov/flood-maps/products-tools/national-risk-index · https://www.fema.gov/about/openfema/data-sets/national-risk-index-data · RAPT viewer: https://experience.arcgis.com/experience/0a317e8998534c30a9b2d3861c814d42/ |

### 1.2 The 18 hazards and their field prefixes

`AVLN` Avalanche · `CFLD` Coastal Flooding · `CWAV` Cold Wave · `DRGT` Drought · `ERQK` Earthquake · `HAIL` Hail · `HWAV` Heat Wave · `HRCN` Hurricane · `ISTM` Ice Storm · `IFLD` Inland Flooding (was `RFLD` Riverine Flooding before v1.20) · `LNDS` Landslide · `LTNG` Lightning · `SWND` Strong Wind · `TRND` Tornado · `TSUN` Tsunami · `VLCN` Volcanic Activity · `WFIR` Wildfire · `WNTW` Winter Weather.

### 1.3 Fields (from the v1.20 data dictionary and the live layer, which has 467 county fields and 469 tract fields)

- **Identity and exposure:** `NRI_ID` (C42101 / T42101000101), `STATE`, `STATEABBRV`, `STATEFIPS`, `COUNTY`, `COUNTYTYPE`, `COUNTYFIPS`, `STCOFIPS`, `TRACT`, `TRACTFIPS` (tract layer), `POPULATION` (2020), `BUILDVALUE` ($), `AGRIVALUE` ($), `AREA` (sq mi), `NRI_VER`.
- **Composite fields:**
  - Risk Index: `RISK_VALUE`, `RISK_SCORE`, `RISK_RATNG`, `RISK_SPCTL`.
  - Expected Annual Loss: `EAL_VALT` (total $), `EAL_VALB` (building $), `EAL_VALP` (population, in persons), `EAL_VALPE` (population equivalence $), `EAL_VALA` (agriculture $), `EAL_SCORE`, `EAL_RATNG`, `EAL_SPCTL`.
  - Expected Annual Loss Rate: `ALR_VALB`, `ALR_VALP`, `ALR_VALA`, `ALR_NPCTL`, and `ALR_VRA_NPCTL` (national percentile adjusted for vulnerability and resilience).
  - Social Vulnerability: `SOVI_SCORE`, `SOVI_RATNG`, `SOVI_SPCTL`.
  - Community Resilience: `RESL_SCORE`, `RESL_RATNG`, `RESL_SPCTL`, `RESL_VALUE`.
  - Community Risk Factor: `CRF_VALUE`.
- **Per-hazard fields** (`<HZ>_*`):
  - Events and frequency: `EVNTS` (recorded events), `AFREQ` (annualized frequency), `EXP_AREA` (sq mi).
  - Exposure: `EXPB`, `EXPP`, `EXPPE`, `EXPA` (only where agriculture applies), `EXPT`.
  - Historic loss ratio: `HLRB`, `HLRP`, `HLRA`, `HLRR` (rating).
  - Expected annual loss: `EALB`, `EALP`, `EALPE`, `EALA`, `EALT`, `EALS`, `EALR`.
  - Loss rate: `ALRB`, `ALRP`, `ALRA`, `ALR_NPCTL`.
  - Hazard risk: `RISKV`, `RISKS`, `RISKR`.

### 1.4 Relationships verified on the Philadelphia County record (C42101)

- **EALB = EXPB × AFREQ × HLRB** holds exactly for CWAV, ERQK, HAIL, HWAV, HRCN, ISTM, LTNG, IFLD, SWND, TRND and WNTW. It does **not** hold for CFLD, LNDS or WFIR, which are computed on sub-county exposure. Do not try to recompute those from county aggregates.
- Per-hazard **`ALRB` = EALB / BUILDVALUE** (total county building value), not EALB/EXPB. The per-hazard ALRBs sum to EAL_VALB/BUILDVALUE = 0.0010685, but the composite `ALR_VALB` = 0.0011995. The composite rate is not the sum of the hazard rates.
- The implied value of statistical life is (EXPT − EXPB − EXPA) / EXPP = **$13,700,000** exactly, and `EAL_VALPE` = `EAL_VALP` × $13.7M.
- **`AFREQ` means different things for different hazards.** Philadelphia values: LTNG 35.7, HWAV 11.1, WNTW 10.5, IFLD 5.82, TRND 0.073, HRCN 0.092, ERQK 0.0016, WFIR 9.1e-5. The last is effectively a burn probability. Convert to P(≥1 per year) = 1 − e^(−AFREQ) only for hazards measured as event counts.
- **IFLD in v1.20:** EXPB is about 100% of BUILDVALUE, with EXP_AREA of 80 of 144 sq mi. That makes EALB/EXPB a county-wide average rather than an "if you are in the floodplain" rate. Pair it with the NFHL or the NFIP SFHA share (section 3).
- Example values for Philadelphia:
  - IFLD ALRB = 9.01e-4, which is about **$270/yr** on a $300k home.
  - ERQK ALRB = 8.5e-5, about $26/yr.
  - Heat wave EALP = 18.7 persons/yr, which is **1.17e-5 per person-year**.

### 1.5 Downloads and sizes

| Product | Format | Size | How obtained |
|---|---|---|---|
| County table, all fields (Hub export, alias headers, 467 cols) | CSV | **19.2 MB raw / 7.2 MB gz / 6.1 MB xz** (measured) | Hub API (below) |
| Tract table, all fields (469 cols) | CSV | **465 MB raw / 152 MB gz** (measured) | Hub API |
| County layer as GeoJSON (full-resolution geometry plus all attributes) | GeoJSON | **556.9 MB** (Content-Length, measured) | Hub API |
| Official OpenFEMA ZIPs: `NRI_Table_{States,Counties,CensusTracts,Tribal_Counties}.zip`, `NRI_Shapefile_*.zip`, `NRI_GDB_*.zip` | CSV / SHP / FGDB | Page says sizes range from 39 MB up to 411.09 MB. Per-file sizes **UNVERIFIED** because scripted HEAD requests got 403. | `https://www.fema.gov/about/reports-and-data/openfema/nri/v120/<name>.zip` |
| **Trimmed county pack**: 10 base fields plus 18 hazards × 10 fields (AFREQ, EXPB, EXPP, HLRB, HLRP, EALB, EALP, ALRB, ALRP, RISKR), 4 significant digits | CSV | **3.85 MB raw / 1.21 MB gz** (measured) | built in this session |
| **Trimmed tract packs, one per state** (same fields) | CSV | **97 MB raw / 23.2 MB gz in total**. Median state 0.29 MB gz. CA 2.02, TX 1.93, NY 1.41, FL 1.33, PA 0.95 MB gz (measured). | built in this session |

### 1.6 APIs (all send CORS \*)

- **ArcGIS REST** (FeatureServer; capabilities `Query,Extract,ChangeTracking`; each layer reports maxRecordCount 2000, while the county service root reports 1000, so page through results):
  - Counties: `https://services.arcgis.com/XG15cJAlne2vxtgt/arcgis/rest/services/National_Risk_Index_Counties/FeatureServer/0`
  - Tracts: `.../National_Risk_Index_Census_Tracts/FeatureServer/0`
  - The owner `FEMA_NationalRiskIndex` publishes 119 feature services, including States EAL, Tribal Counties/Tracts, and per-hazard rating layers.
  - Example: `.../National_Risk_Index_Counties/FeatureServer/0/query?where=STCOFIPS%3D%2742101%27&outFields=RISK_RATNG,EAL_VALT,IFLD_AFREQ,NRI_VER&returnGeometry=false&f=json`
  - Querying by FIPS reveals only the county. Querying by point (`geometry=lon,lat&geometryType=esriGeometryPoint&inSR=4326`) sends the user's coordinates to Esri/FEMA.
- **Hub bulk export** (asynchronous; returns `resultUrl`, which redirects to a signed Azure blob): `https://hub.arcgis.com/api/download/v1/items/39485e8035d446a5bff03259508ae355/csv?redirect=false&layers=0` for counties. The tracts item is `9da4eeb936544335a6db0cd7a8448a51`. `geojson` also works.

### 1.7 License and terms

- US Government data, subject to the NRI Terms & Conditions (paraphrased):
  - no reverse engineering or deriving the underlying datasets;
  - cite the endpoint or dataset with its version and access date/time;
  - include a statement that the product uses NRI data but is not endorsed by FEMA;
  - do not present modified data as FEMA's;
  - FEMA may rescind use and request destruction of copies;
  - no use of FEMA or DHS logos.
- "Planning purposes only."
- The CoreLogic flood inputs are proprietary, but only the derived outputs are distributed.

### 1.8 Mapping NRI to one household

- P(the county sees at least one event of hazard h per year) ≈ 1 − exp(−AFREQ_h), for event-count hazards.
- Expected annual building loss for a home with replacement value V:
  - V × `ALRB_h` as a county-average prior;
  - V × `EALB_h / EXPB_h` if the home is known to be in the hazard's exposure footprint (coastal flood zone, WUI, landslide-susceptible cell).
- Mean damage given an event ≈ V × `HLRB_h`. This is a mean, not a tail value.
- Per-person annual casualty risk ≈ `EALP_h / POPULATION`. The injury weighting inside EALP is **UNVERIFIED**.
- `SOVI`, `RESL` and `CRF` are community-level context. The household model should use the household's own attributes (vehicle, disability, age, income) instead.

---

## 2. US hazard sources beyond NRI

Columns: name · what it measures · spatial resolution · coverage · format · approximate size · license/terms · access · update cadence · how it maps to one household.

| Name | Measures | Resolution | Coverage | Format | Size | License / terms | Access (verified) | Cadence | Household mapping |
|---|---|---|---|---|---|---|---|---|---|
| **USGS National Seismic Hazard Model 2023**, revised 2023.R1/R2 | Hazard curves: annual frequency of exceeding PGA, PGV and SA at 0.01–10 s. Uniform-hazard ground motions at 2/5/10% in 50 yr. MMI VI chance in 100 yr. | 0.2° grid (data release); any point via web service | CONUS, AK, HI | CSV in ZIP; JSON | CONUS `hazard_output_CONUS.zip` **934 MB**, AK 25 MB. `US_ProbMMI_VI_100Yrs_varVs30.zip` **32 MB**. | USGS, PD | ScienceBase doi:10.5066/P9GNPCOD (2023-12-21). Revised release doi:10.5066/P14VGAV4, published 2026-08-10. Service: `https://earthquake.usgs.gov/ws/nshmp/conus-2023.R2/dynamic/hazard/{lon}/{lat}/{vs30}` (CORS \*; `conus-2023` redirects to `.R2`). | Model roughly every 5 yr (1996, 2002, 2008, 2014, 2018, 2023) plus revisions | Read P(PGA ≥ threshold per yr) off the curve. Philadelphia at Vs30 760: PGA ≥ 0.134 g has 8.7e-4/yr (measured). Annual p from the 100-yr MMI VI map = 1 − (1 − p100)^(1/100). |
| USGS ASCE 7-22 design service | Code design values (SMS, SD1 and others) | point | US | JSON | tiny | PD | `https://earthquake.usgs.gov/ws/designmaps/asce7-22.json?...` redirects to `/ws/building-codes/asce7-22/calculate` (200 OK; CORS not checked) | with code cycles | Retrofit and bolting prompts only; not a probability. |
| **USGS Volcano Hazards: HANS public API** | 170 US volcanoes with lat/lon, NVEWS threat class (Very High 18, High 35, Moderate 48, Low 42, Very Low 20) and current alert level | point | US and territories | JSON | about 170 KB | PD | `https://volcanoes.usgs.gov/hans-public/api/volcano/getUSVolcanoes`, `.../getElevatedVolcanoes` (CORS \*) | alerts real time | Distance to a Very High or High threat volcano triggers an ash/lahar module; live alert banner. |
| **USGS National Volcanic Threat Layer v1.0** | Proximal hazard zones (ballistics, PDCs, lava, lahars, heavy ash) around 161 volcanoes | polygons | US | FGDB | `NationalVolcanicThreatLayer_v1_0.gdb.zip` **1.66 MB** | PD | ScienceBase item `6615a58cd34e7eb9eb7d55b5` | static | Point-in-polygon: is the home inside a proximal zone? |
| **NOAA NCEI Global Historical Tsunami Database** | Tsunami events and runups | points | global, including US coasts | JSON | small | NOAA, PD | `https://www.ngdc.noaa.gov/hazel/hazard-service/api/v1/tsunamis/events?country=USA` (CORS \*) | continuous | Historical frequency on the coast. Evacuation-zone polygons are state products; which states publish GIS is **UNVERIFIED**. NRI `TSUN` (overhauled in v1.20) is the national frequency. |
| **USFS Wildfire Risk to Communities, 2nd ed. (2024)** | Rasters: BP (annual burn probability, labelled "Wildfire Likelihood"), CFL, FLEP4/8, cRPS ("Wildfire Consequence"), RPS ("Risk to Homes"), Exposure type, WHP | **30 m**. BP was modelled at 270 m by FSim and upsampled. | CONUS; AK/HI via per-state zips | GeoTIFF in ZIP | CONUS: BP 32.3 GB, RPS 33.7 GB, cRPS 26.8 GB, CFL 29.1 GB, FLEP4 22.7 GB, FLEP8 17.4 GB, WHP 8.17 GB, Exposure 5.63 GB. Per-state examples: CT 338 MB, DE 102 KB. | USFS: the page says the data can be used without additional permissions or fees; citation requested | `https://www.fs.usda.gov/rds/archive/catalog/RDS-2020-0016-2` | v1 2020, v2 2024 | BP at the home equals the annual probability that fire reaches that pixel. RPS × home value gives relative expected loss. |
| WRC tabular summary | Per state, county (about 3,144), community (about 32,038 Census places) and tribal area: `BP_STATE_RANK`, `BP_NATIONAL_RANK`, `RISK_STATE_RANK`, `RISK_NATIONAL_RANK`, `TOTAL_BUILDINGS`, `BUILDINGS_FRACTION_ME/IE/DE` (minimal, indirect, direct exposure) | admin units | US | XLSX | **5.0 MB** | as above | `https://wildfirerisk.org/wp-content/uploads/2026/04/wrc_download_20260415.xlsx` | refreshed Apr 2026 | **Percentile ranks only, no absolute BP.** Absolute values require raster sampling at build time, or NRI `WFIR_AFREQ` at tract level. |
| USFS Wildfire Hazard Potential 2023 (4th ed.) | Relative index plus 5 classes | **270 m** | CONUS | raster | `Data.zip` 351 MB | same as WRC | `https://www.fs.usda.gov/rds/archive/catalog/RDS-2015-0047-4` | every few years | Class for WUI messaging only; not a probability. |
| **NOAA SPC severe reports (WCM/SVRGIS)** | Tornado tracks 1950–2025; hail and wind reports 1955–2025 | points / paths | CONUS | CSV, shapefile | `1950-2025_actual_tornadoes.csv` 9.0 MB. Zips: torn 2.0 MB, hail 8.3 MB, wind 11.9 MB (modified 2026-04-24). `mobile_home_percentage_county.zip` 7.7 MB. | NOAA, PD | `https://www.spc.noaa.gov/wcm/`, `https://www.spc.noaa.gov/gis/svrgis/` | annual (spring) | Build step: rate of ≥1 tornado, hail ≥1", or wind ≥50 kt within 25 mi of a county or ZCTA centroid gives P(≥1)/yr. Tornado path area ÷ county area gives P(the home is struck). |
| SPC climatology maps | Daily probability of a report within 25 mi | 80 km grid, smoothed; based on 1982–2011 | CONUS | **PNG images only** (no grid download found) | – | PD | `https://www.spc.noaa.gov/new/SVRclimo/climo.php?parm=allTorn` | static | Recompute from SVRGIS reports instead. |
| **NOAA NHC HURDAT2** | 6-hourly best tracks: Atlantic 1851–2025, NE Pacific 1949–2025 | track points | ocean basins | text | `hurdat2-1851-2025-091226.txt` **7.1 MB**; `hurdat2-nepac-1949-2025-091426.txt` 4.1 MB | NOAA, PD | `https://www.nhc.noaa.gov/data/hurdat/` | yearly plus reanalysis | Build step: count hurricanes by category within 50 nm of each county centroid to get return periods. NHC's own return-period maps are images from the 1987 HURISK program with data through 2010, so recompute. |
| **NOAA Atlas 14 (PFDS)** | Precipitation depth for 5-min to 60-day durations at 1–1000-yr ARI, with 90% CI | fine grid (exact resolution **UNVERIFIED**); any point | CONUS volumes plus AK/HI/PR | CGI text; ASCII grids per region/duration/ARI (e.g. `ne1yr02ha.zip` about 0.3 MB) | small per grid | NOAA, PD | Point (**no CORS**): `https://hdsc.nws.noaa.gov/cgi-bin/new/cgi_readH5.py?lat=39.95&lon=-75.16&type=pf&data=depth&units=english&series=pds`. Grids: `https://hdsc.nws.noaa.gov/pub/hdsc/data/<region>/` | static; being superseded | 100-yr 24-h depth drives pluvial-flood and sump/backflow prompts. |
| NOAA Atlas 15 | Vol 1: historical, trend-aware. Vol 2: future adjustment factors to 2100. | – | CONUS first | – | – | PD | `https://water.noaa.gov/about/atlas15`. Montana pilot released 2024-09-26 (preliminary). CONUS preliminary estimates for peer review due **"September 2026"**, published 2027. Outside CONUS: preliminary 2027, final 2028. | – | Swap in for Atlas 14 when final. |
| **NOAA NCEI Storm Events Database** | Event records with deaths, injuries and property/crop damage. Files run from 1950. The full event-type list is said to start in 1996 (**UNVERIFIED** here). | county or forecast zone (some lat/lon) | US | CSV.gz per year (details / locations / fatalities) | Details files 1950–2026: 77 files, **318 MB gz** total; about 12.6 MB gz per recent year | NOAA, PD | `https://www.ncei.noaa.gov/pub/data/swdi/stormevents/csvfiles/` | monthly (2026 file created 2026-09-18) | Build step: per-county annual counts and damage distributions for flash flood, heat, winter storm, ice storm and high wind. |
| NWS/WPC Winter Storm Severity Index | Forecast impact index | NWS grid | CONUS | KMZ / SHP | small | PD | `https://www.wpc.ncep.noaa.gov/wwd/wssi/wssi.php`; GIS at `.../wwd/wssi/gis/shp` | **forecast only; no climatology archive found** | Live banner only. For annual probability use NRI `WNTW`/`ISTM` or Storm Events. |
| **CDC/ATSDR Heat & Health Index (HHI)** | Heat vulnerability ranking combining historical heat, heat-related illness and community characteristics | **ZCTA** | US | CSV (ZIP) | `HHI_Data.zip` **18.4 MB** (modified 2025-09-04) | CDC, PD (no explicit license shown; cite) | `https://www.atsdr.cdc.gov/place-health/media/files/2024/08/HHI_Data.zip` | 2024 release, updated Sept 2025 | ZIP-level heat vulnerability multiplier. |
| CDC Heat & Health Tracker / Tracking Network API | HeatRisk forecasts, heat-related ED visits (NSSP), historical heat days | county / ZIP | US | web app; JSON API | – | CDC, PD | App blocks scripted access. API `https://ephtracking.cdc.gov/apigateway/api/v1/...` returned **429 "too many non-token requests"**, so register for a token. | daily | Use at build time with a token. NWS HeatRisk is a 7-day forecast, so treat it as a live feed. |
| **US Drought Monitor** | Weekly percent of area in D0–D4 | county, state and other units (weekly GIS polygons also exist; URL **UNVERIFIED**) | US, 2000–present | CSV/JSON API | small per query | public; attribution requested (**UNVERIFIED** exact terms) | `https://usdmdataservices.unl.edu/api/CountyStatistics/GetDroughtSeverityStatisticsByAreaPercent?aoi=42101&startdate=1/1/2024&enddate=12/31/2024&statisticsType=1` (**no CORS**) | weekly | Share of weeks in D2+ gives P(water restrictions or well stress) per year. |
| **NOAA Sea Level Rise Viewer data** | Inundation depth and extent at 0–10 ft above MHHW, mapping confidence, high-tide flooding, marsh migration | 3–10 m | all coastal states and territories except Alaska | FGDB/rasters; MapServer | multi-GB per state (not measured) | NOAA, PD | Downloads: `https://coast.noaa.gov/slrdata/`. Services: `https://coast.noaa.gov/arcgis/rest/services/dc_slr/slr_{0..10}ft/MapServer` in half-foot steps (CORS reflect). | irregular | Build step: share of a tract or ZCTA inundated at 1/2/3 ft. Optional on-demand identify at a snapped point. The 2022 SLR tech-report and NASA scenario-tool pages returned 404, so their location is **UNVERIFIED**. |
| **USGS Landslide susceptibility (slope–relief threshold, 2024)** | Susceptibility | **90 m** | CONUS, AK, HI, PR | GeoTIFF ZIP plus CSV | `n10_susc.zip` 588 MB, `lw_susc.zip` 565 MB, `landslides.csv` 67 MB, **`county_analysis.csv` 0.23 MB** | USGS, PD | doi:10.5066/P13KAGU3 (published 2024-08-22) | static | `county_analysis.csv` can be bundled as is. NRI `LNDS` v1.20 uses this model. |

---

## 3. Flood

| Name | Measures | Resolution | Coverage | Format | Size | License / terms | Access | Cadence | Household mapping |
|---|---|---|---|---|---|---|---|---|---|
| **FEMA NFHL** | Regulatory flood zones (`FLD_ZONE`, `ZONE_SUBTY`, `SFHA_TF`, `STATIC_BFE`, `DEPTH`, `VELOCITY`), plus LOMRs, BFEs, levees and FIRM panels | parcel-scale polygons | mapped communities (see NFHL Availability layer 0) | ArcGIS MapServer (v11.1, maxRecordCount 2000); state/county FGDB and SHP from FEMA's Map Service Center | **5,806,971** zone polygons, 1,371,296 of them SFHA (counted). Download size not measured (**UNVERIFIED**). | FEMA, PD (specific terms **UNVERIFIED**) | **Point query verified:** `https://hazards.fema.gov/arcgis/rest/services/public/NFHL/MapServer/28/query?geometry=-75.2240,40.0245&geometryType=esriGeometryPoint&inSR=4326&spatialRel=esriSpatialRelIntersects&outFields=FLD_ZONE,ZONE_SUBTY,SFHA_TF,STATIC_BFE,DFIRM_ID&returnGeometry=false&f=json` returns `AE`, `SFHA_TF=T`. A New Orleans test point returned X at 0.2%. CORS reflect. | continuous (LOMRs, new FIRMs) | A/V zones mean at least 1%/yr (about 26% over 30 yr). Shaded X means 0.2–1%. Unshaded X is below 0.2%. **Why not bundle:** 5.8M detailed polygons. The national download size is **UNVERIFIED** but is surely far beyond a browser budget. |
| **OpenFEMA API** (50 API datasets) | `DisasterDeclarationsSummaries` v2 (70,414 rows; incident type, county FIPS, IA/PA/HM flags). `NfipClaims` v3 (2,725,989 rows; `countyCode`, `censusGeoid`, `dateOfLoss`, paid amounts, `waterDepth`, `floodZoneCurrent`). `NfipPolicies` v3 (74.7M). **`NfipResidentialPenetrationRates`** v1 (3,159 counties: `totalResStructures`, `totalResStructuresSfha`, contracts in force, penetration rates; as of 2026-08-03). IHP: `HousingAssistanceOwners`/`Renters` v2 (per disaster × ZIP/county: registrations, inspected-damage bins, approved amounts), `IndividualsAndHouseholdsProgramValidRegistrations` v2 (26.2M rows). | record / ZIP / county | US | JSON, CSV, Parquet, JSONL | Declarations: CSV 10–50 MB, Parquet under 10 MB. NFIP claims: Parquet 50–500 MB, CSV 0.5–10 GB. IHP registrations CSV over 10 GB. | OpenFEMA T&Cs: redistribution allowed with citation, access date and a "not endorsed by FEMA" statement; no re-identification; statistical use only | `https://www.fema.gov/api/open/v2/DisasterDeclarationsSummaries?$filter=fipsStateCode eq '42' and fipsCountyCode eq '101'` returns 25 rows for Philadelphia since 1965 (CORS \*). **`FimaNfipClaims` v2 and `FimaNfipPolicies` v2 are deprecated as of 2026-10-15; use v3.** | Declarations about every 20 min; NFIP monthly | Declarations per year gives P(federal disaster in your county). IHP mean approved amount against inspected damage shows the uninsured gap. `totalResStructuresSfha / totalResStructures` is **the county prior for P(home in SFHA)**, e.g. Autauga AL: 628/20,789 = 3.0%. Penetration gives "x% of SFHA homes insured". |
| First Street (Risk Factor) | Proprietary property-level flood, fire, heat, wind and air risk | property | US | API, bulk county CSVs, AWS Data Marketplace (per search results) | – | **Proprietary, paid.** Redistribution terms **UNVERIFIED** (licensing help page returned 403), so assume not redistributable. | Free: consumer property lookups on riskfactor.com / firststreet.org | – | Link out only ("check your address at…"). Never bundle. |

---

## 4. Power grid

| Name | Measures | Resolution | Coverage | Format | Size | License / terms | Access | Cadence | Household mapping |
|---|---|---|---|---|---|---|---|---|---|
| **ORNL EAGLE-I recorded outages 2014–2025** | Customers out per county every 15 min, scraped from utility outage maps. Also `MCC.csv` (modelled customers per county, 2022), `coverage_history.csv`, `DQI.csv`. | county, 15 min | US; coverage improves after 2018 | CSV per year | **11.64 GB** total. 2024: 1.44 GB. 2025: 1.40 GB. `MCC.csv` 41 KB. | **CC BY 4.0** (attribution required) | figshare article 24237376, doi:10.6084/m9.figshare.24237376.v4 (v4 published 2026-02-25) via `https://api.figshare.com/v2/articles/24237376` | a year of data added annually | Build step, per county: P(outage affecting ≥x% of customers lasting ≥8/24/72/168 h per year) and expected customer-hours out. These size backup power, refrigeration and medical-device plans. |
| DOE-417 (OE-417) electric disturbance events | Event list: area/utility, NERC region, start and restore times, customers, MW | utility / region (**no county**) | US, 2000–present (per DOE page as reported by search) | XLS / PDF | small | PD (the ORNL mirror labels it Public Domain) | `https://www.oe.netl.doe.gov/OE417_annual_summary.aspx` timed out from the test host, so current contents are **UNVERIFIED**. ORNL mirror has **2023 only** (341 records): `https://openenergyhub.ornl.gov/explore/dataset/oe-417-annual-summaries/` | annual | Base rates for large events by state or region. |
| EIA-861 reliability (SAIDI/SAIFI with and without major events) | Average interruption hours per customer | utility / state | US | XLSX | **UNVERIFIED** (eia.gov returned 503 to scripted requests) | PD | EIA Today in Energy, verified via search: in 2024 major-event interruptions averaged about 9 h per customer versus about 4 h/yr in 2014–2023; non-major-event interruptions about 2 h/yr; South Carolina about 53 h in 2024 (`https://www.eia.gov/todayinenergy/detail.php?id=66744`) | annual | State-level expected outage hours per customer-year. |
| PowerOutage.us | Live and historical outage counts | county / utility | US | commercial | – | **Commercial**; the site blocks automated access, so terms are **UNVERIFIED** | – | – | Link out only. Not usable in an open-source bundle. |

---

## 5. Demographics, vulnerability, facilities

| Name | Measures | Resolution | Coverage | Format | Size | License / terms | Access | Cadence | Household mapping |
|---|---|---|---|---|---|---|---|---|---|
| **CDC/ATSDR SVI 2022** (latest; the 2024 county file URL returns 404) | 16 ACS variables grouped into 4 themes, plus an overall percentile | county, tract | US | CSV, SHP, FGDB | county CSV **2.3 MB**; tract CSV **61.1 MB** | CDC, PD; citation requested. MP_CROWD values were corrected on 2024-12-11. | `https://svi.cdc.gov/Documents/Data/2022/csv/states_counties/SVI_2022_US_county.csv`, `https://svi.cdc.gov/Documents/Data/2022/csv/states/SVI_2022_US.csv` | biennial | Community context only. NRI v1.20 no longer uses it. |
| **Census Community Resilience Estimates (CRE) 2024** | Share of residents with 0, 1–2, or 3+ of 10 risk factors (`PRED0`, `PRED12`, `PRED3`, each with MOE) | tract, county, state, CBSA | US | CSV | `CRE_24_County.csv` **374 KB**; `CRE_24_Tract.csv` **12 MB**; ranking tables | Census, PD | `https://www2.census.gov/programs-surveys/demo/datasets/community-resilience/2024/` (files dated 2026-01-29) | annual | Now NRI's Social Vulnerability source. Bundle the county file. |
| **Census ACS 5-year**; the 2020–2024 vintage is live (`/data/2024/acs/acs5` metadata) | Verified variable IDs: `B25010_001E` average household size, `B19013_001E` median household income, `B08201_002E` households with no vehicle, `B25044_003E` owner-occupied with no vehicle, `B18101` disability by age, `B01001` sex by age, `B25024_010E` mobile homes, `B25034` year built, `B28002_013E` no internet access, `B16005` English ability, `B25003_003E` renter occupied | county, tract, block group, ZCTA | US | JSON API | small per query | Census, PD | `https://api.census.gov/data/2024/acs/acs5?get=NAME,B25010_001E,B19013_001E&for=county:101&in=state:42&key=KEY`. **A key is now required** (unkeyed calls redirect to `missing_key.html`). CORS \*. | annual (Dec/Jan) | Prefill the household form defaults and compare the household with its community. **Build time only**, with the key kept as a CI secret. |
| EPA EJScreen | Environmental burden indicators | block group | US | – | – | EPA removed the tool 2025-02-05; `epa.gov/ejscreen` and `gaftp.epa.gov/EJScreen` both return 404. Mirror license **UNVERIFIED**. | PEDP reconstruction of v2.3: `https://screening-tools.com/epa-ejscreen` (200 OK); GitHub `Public-Environmental-Data-Partners/EJScreen` | frozen | Optional. Prefer the original sources (TRI, RMP) listed below. |
| HIFLD Open | Infrastructure layers (formerly 300+ datasets) | – | – | – | – | **Shut down**; public access removed 2025-08-25/26 (per search results). DHS published a crosswalk to the source agencies. | Archives: `https://www.hsdl.org/hifld/`, Data Rescue Project portal, `https://source.coop/seerai/hifld` | frozen | Use the source agencies instead (rows below). |
| **FEMA Operating Nuclear Power Plant Sites** | 57 sites with lat/lon, MW, NRC region and update date | point | US | ArcGIS FeatureServer | tiny | FEMA, PD (terms **UNVERIFIED**) | `https://gis.fema.gov/arcgis/rest/services/Partner/Operating_Nuclear_Power_Plant_Sites/FeatureServer/0`. Distance rings at 2/5/10/50 mi (228 lines, updated 2022-08-01): `https://services.arcgis.com/XG15cJAlne2vxtgt/arcgis/rest/services/NPP_Distance_Range_Rings/FeatureServer/80` | as plants change | Inside the 10-mi EPZ triggers a shelter/evacuation and potassium iodide (KI) module. Inside 50 mi triggers ingestion-pathway notes. **NRC's own reactor pages returned 403 to scripted clients (UNVERIFIED).** |
| **EPA TRI** | Facility-by-chemical release records with lat/lon | point | US | CSV per year | 2024 national file **60.6 MB**: 77,295 rows, **21,482 facilities** (measured) | EPA, PD | `https://data.epa.gov/efservice/downloads/tri/mv_tri_basic_download/2024_US/csv`. Envirofacts REST (cross-origin allowed): `https://data.epa.gov/efservice/tri_facility/state_abbr/PA/rows/0:1/JSON` | annual | Distance to the nearest large emitter triggers shelter-in-place kit items. |
| EPA RMP (chemical accident plans) | RMP submissions excluding the off-site consequence analysis (OCA) | facility | US | JSON by state/facility | **UNVERIFIED** | Officially available only through reading rooms or FOIA. The Data Liberation Project's FOIA copy was updated Jan 2026 (submissions through 2025-12-31). **The rmpmap.org API is CC BY-SA 4.0 (share-alike).** | `https://www.data-liberation-project.org/datasets/epa-risk-management-program-database/`, `https://rmpmap.org/api-docs` | irregular | Presence of nearby RMP facilities feeds shelter-in-place readiness. Worst-case radii (OCA) are not public. |
| **USACE National Inventory of Dams** | **92,766 dams** (measured): hazard potential High 17,050, Significant 11,353, Low 60,298, Undetermined 4,065. Also condition assessment, EAP prepared, storage and lat/lon. | point | US | CSV, GeoPackage | `nation.csv` **67.3 MB** (84 cols); `nation.gpkg` 78.5 MB (CORS \*) | public download. The CSV has an "Inundation Maps Added to NID?" flag, but the maps are not in the public file; details of access limits **UNVERIFIED**. | `https://nid.sec.usace.army.mil/api/nation/csv`, `.../api/nation/gpkg`. File header: "Data Last Updated: 2026-9-23". | continuous | A High-hazard dam upstream within x km triggers a dam-failure and evacuation note. Public inundation maps are not available. |

---

## 6. Geography, offline geocoding, crosswalks

| Name | Contents | Size (verified) | License | Access / notes |
|---|---|---|---|---|
| **Census cartographic boundary counties 2024** | County polygons | `cb_2024_us_county_20m.zip` **0.90 MB**, `_5m` 2.98 MB, `_500k` 11.63 MB. A 2025 `_500k` (11.76 MB) also exists. **Converted to GeoJSON with 3-decimal coordinates and GEOID only (measured):** 20m 1.29 MB / **0.31 MB gz** (3,222 features, a few territories missing), 5m 4.15 / **1.03 MB gz** (3,235), 500k 17.7 / 3.6 MB gz. | PD | `https://www2.census.gov/geo/tiger/GENZ2024/shp/` |
| Census cartographic boundary tracts 2024 | Tract polygons | national `cb_2024_us_tract_500k.zip` **57.8 MB**; CA 4.28 MB; PA 1.68 MB | PD | same directory, `cb_2024_<ss>_tract_500k.zip` |
| TIGER/Line full resolution | Counties, ZCTAs | `tl_2024_us_county.zip` 83.9 MB; `tl_2024_us_zcta520.zip` **528.8 MB**. The ZCTA cartographic file exists only for 2020: `cb_2020_us_zcta520_500k.zip` **66.7 MB**. | PD | Too large to ship. Use centroids instead. |
| **Gazetteer internal points** | ZCTA, county and tract centroids (INTPTLAT/INTPTLONG) plus land area | `2024_Gaz_zcta_national.zip` **1.01 MB**; counties 0.14 MB; tracts 2.42 MB; 2025 ZCTA file 0.95 MB | PD | `https://www2.census.gov/geo/docs/maps-data/data/gazetteer/2024_Gazetteer/` |
| **ZCTA to county relationship (2020)** | Land/water overlap areas | `tab20_zcta520_county20_natl.txt` **6.8 MB**. **Trimmed to `zcta,lat,lon,county:landShare;...`: 1.31 MB raw / 0.40 MB gz** (measured). 33,791 ZCTAs, 10,186 (30.1%) spanning more than one county. | PD | `https://www2.census.gov/geo/docs/maps-data/data/rel2020/zcta520/`. Uses the **old CT counties**, so add a planning-region crosswalk. |
| HUD–USPS ZIP crosswalk | Real USPS ZIPs mapped to tract, county, CBSA or congressional district, with RES/BUS/OTH/TOT ratios. Quarterly, derived from USPS vacancy data; 2020 geographies from 2023 Q1. | – | Files require a HUD USER **login**. The API requires a **token** (401 without one; `x-ratelimit-limit: 60`). **Redistribution terms UNVERIFIED.** | `https://www.huduser.gov/portal/datasets/usps_crosswalk.html`, API `https://www.huduser.gov/hudapi/public/usps?type=2&query=19103`. Better than ZCTA for PO-box ZIPs, but licensing is unclear, so use it at build time only. |
| **Census Geocoder** | Address to lat/lon, county, tract and block | – | PD | `https://geocoding.geo.census.gov/geocoder/geographies/onelineaddress?address=...&benchmark=Public_AR_Current&vintage=Current_Current&format=json`. **No CORS header**, but `format=jsonp&callback=cb` works; JSONP means executing a remote script. **Privacy: the full street address goes to the Census Bureau in a URL query string, which servers and proxies log.** Do not make this the default. |

---

## 7. Climate projections

| Name | Variables | Resolution / coverage | Format / size | License | Access (verified) | Household mapping |
|---|---|---|---|---|---|---|
| **NCA5 (2023)** | Report and Atlas | `nca2023.globalchange.gov`, `www.globalchange.gov` and `atlas.globalchange.gov` all **failed to connect** on 2026-09-25. Report PDFs are re-hosted at `https://toolkit.climate.gov/NCA5`. | – | – | – | – |
| **NCA5 Atlas services** (owner `maps_nationalclimate`, modified 2026-05-06) | County values at **global warming levels 1.5/2/3/4 °C**: `tmax_days_ge_95f/100f/105f`, `tmax1day`, `tmean_jja`, `tmin_days_le_32f`, `tmin_days_le_0f`, `tmin_days_ge_70f`, `tmin_jja`, `pr_annual`, `prmax1day`, `prmax5yr`, `pr_above_nonzero_99th`, `pr_days_above_nonzero_99th` (each with a `_GWLx` suffix) | 3,111 CONUS counties | ArcGIS FeatureServer; small | **CC BY 4.0** (per item license) | `https://services3.arcgis.com/0Fs3HcaFfvzXvm7w/arcgis/rest/services/NCA_Atlas_GWL_2C/FeatureServer/0`. Service names are inconsistent: 1.5 °C is `NCA_Atlas_Figures_Beta_Counties_view`; 3 °C and 4 °C are `NCA_Atlas_Global_Warming_Level_5_deg_F` and `..._7_deg_F`. | "Days ≥100 °F: now vs +2 °C" drives heat-plan urgency; extreme-precipitation change drives flood-plan urgency. |
| NCA5 / LOCA2 ensemble decadal series | e.g. `LOCA2_Ensemble_SSP370_Hot_Days_1950_2100` with `TMAXDAYSGE85F`…`110F` per decade. SSP2-4.5 precipitation and SSP3-7.0 temperature services also exist. | County (49,744 rows = counties × 16 decades), tribal areas, HUC8 | FeatureServer | CC BY 4.0 (same owner; per-service license not individually checked) | `https://services3.arcgis.com/0Fs3HcaFfvzXvm7w/arcgis/rest/services/LOCA2_Ensemble_SSP370_Hot_Days_1950_2100/FeatureServer/0` | Trend lines for the UI. |
| **CMRA (2025 version)** | 18 LOCA variables (cooling degree days, consecutive dry/wet days, heavy precipitation, hot days and others) as min/mean/max across models, historical plus early/mid/late century. Adds NRI hazard scores, BCAT building-code adoption, SLR (CONUS), and ACS population/poverty. **Scenario naming conflict:** fields are `RCP45*`/`RCP85*` while the item text says SSP2-4.5/SSP5-8.5, and it lists CMIP5 LOCA sources (**UNVERIFIED** which is correct). | 3,233 counties including territories, plus AIANNH areas | Hub CSV export **19.1 MB raw / 8.1 MB gz**, 440 cols (measured) | License field is blank (US Gov; **UNVERIFIED**) | FeatureServer `https://services3.arcgis.com/0Fs3HcaFfvzXvm7w/arcgis/rest/services/CMRA_Tool_Dev/FeatureServer/0` (item `54f4e2343500422bbddf1f5dafb30bbd`). UI: `https://resilience.climate.gov/` | County SLR impact and hot-day changes. |
| NOAA Climate Explorer (`crt-climate-explorer.nemac.org`, site up) | County and point charts of LOCA thresholds | CONUS | Backed by the ACIS GridData API `https://grid2.rcc-acis.org/GridData`, which is **CORS \*** and supports `area_reduce=county_mean` (verified POST) | NOAA/RCC; ACIS terms **UNVERIFIED** | – | Build-time or on-demand by county; a county-level query reveals only the county. |
| LOCA2 (CMIP6 downscaled) | Daily temperature and precipitation | about 6 km, CONUS/N. America, multiple GCMs and SSPs (details **UNVERIFIED**) | netCDF; tens of TB (**UNVERIFIED**) | **UNVERIFIED** | Directory `https://cirrus.ucsd.edu/~pierce/LOCA2/` is live (`CONUS_regions_split/`, `NAmer/`). `loca.ucsd.edu` had a TLS certificate error. | Do not use the raw files. Use the NCA5 or CMRA county summaries. |
| Climate Toolbox (`climatetoolbox.org`, site up) | Web tools built on gridMET/MACA | – | No bulk API identified | **UNVERIFIED** | – | Link out only. |
| FEMA Future Risk Index | NRI climate-future variant | – | Only community-uploaded "(Archive)" copies found on ArcGIS Online | **UNVERIFIED** status and terms | – | Avoid. |

---

## 8. Personal-scale base rates (national) for a "personal emergencies" tier

| Risk | Verified figure | Source | Per-household / per-person conversion |
|---|---|---|---|
| Households (denominator) | 131,434k (2023), 132,216k (2024), 134,790k (2025, flagged "t" in the table, likely a series break) | Census CPS table HH-1: `https://www2.census.gov/programs-surveys/demo/tables/families/time-series/households/hh1.xls` (downloaded and parsed) | – |
| **Residential building fire** | **344,600 fires, 2,890 deaths, 10,400 injuries, $11.27B loss (2023)** | USFA: `https://www.usfa.fema.gov/statistics/residential-fires/` | 344,600 / 131.43M = **0.262% per household-year (about 1 in 381)**. Mean loss about **$32,700 per fire**. Fire death about 2.2e-5 and injury about 7.9e-5 per household-year. The NFPA "Home Structure Fires" page content could not be retrieved (JS-rendered). |
| **Involuntary job loss** | JOLTS layoffs & discharges rate: monthly average **1.117% (2025)** and 1.058% (2024). **21.17M layoff/discharge events in 2025** (sum of monthly levels). | BLS API series `JTS000000000000000LDR` / `...LDL` via `https://api.bls.gov/publicAPI/v2/timeseries/data/` | Naive P(≥1 per worker-year) = 1 − (1 − 0.0112)^12 ≈ **12.6%. This is an upper bound**, because events cluster in high-churn jobs. A person-based rate (Displaced Worker Survey) was not fetched (**UNVERIFIED**). |
| Unintentional injury death | **197,449 deaths; 58.1 per 100k (2024)** | NCHS FastStats (NVSS via CDC WONDER): `https://www.cdc.gov/nchs/fastats/accidental-injury.htm` | 5.8e-4 per person-year |
| **Emergency department visits** | **155.4M visits; 47.3 per 100 persons; 11.5% admitted; 43.5M injury-related (2022)**. 26.2M ED visits for unintentional injuries (2022). | NCHS FastStats (NHAMCS 2022): `https://www.cdc.gov/nchs/fastats/emergency-department.htm` | 0.47 ED visits per person-year, 0.13 of them injury-related (derived), about 0.054 admissions via the ED (derived). For a 3-person household, about 1.4 ED visits per year. |
| **Motor-vehicle crashes** | **36,640 deaths (2025 early estimate); 39,254 (2024); 1.10 deaths per 100M VMT (2025)**. 2023: 40,901 deaths and **2.44M injured**. | NHTSA DOT HS 813 800 (Apr 2026), `https://crashstats.nhtsa.dot.gov/Api/Public/ViewPublication/813800`; DOT HS 813 705 (Apr 2025), `.../813705` | Mileage-scaled societal rate: 12,000 mi/yr × 1.10e-8 = 1.3e-4 fatalities per driver-year. This counts all fatalities per VMT, not only occupant risk. |
| **Pandemic** | Influenza pandemics 1918 (~675,000 US deaths), 1957 (~116,000), 1968 (~100,000), 2009 H1N1 (~12,500), plus COVID-19 (2020–). COVID death toll not re-verified (**UNVERIFIED**). | CDC archive pages via search (`archive.cdc.gov/.../pandemic-resources/...`) | 5 onsets in 108 years (1918–2025) gives λ ≈ 0.046/yr, so **P(onset in a given year) ≈ 4.5%** (crude Poisson). |
| Power interruption | 2024: about 9 h per customer from major events plus about 2 h from other events. The 2014–2023 major-event average was about 4 h/yr. | EIA Today in Energy 66744 (via search) | Use EAGLE-I county statistics for duration tails. |
| Medical / other base rates not fetched | e.g. household-level cardiac arrest, residential burglary, water-main breaks | – | **UNVERIFIED**. Add from BJS NCVS and CDC as needed. |

---

## 9. Global sources (short)

| Name | What | Resolution | License | Access (verified) |
|---|---|---|---|---|
| GFDRR ThinkHazard! | Hazard level (High / Med / Low / Very low) for 11 hazards, verified via the API: river, urban and coastal flood, earthquake, landslide, tsunami, volcano, cyclone, water scarcity, extreme heat, wildfire. Given per admin unit. | GAUL admin 0–2 | Code is open source (GitHub). **Data license UNVERIFIED.** | `https://thinkhazard.org/en/report/257-united-states.json`. Search: `https://thinkhazard.org/en/administrativedivision?q=Philadelphia` returns code 30971. **No CORS header.** |
| INFORM Risk (EC JRC) | Composite crisis/disaster risk: hazard and exposure, vulnerability, lack of coping capacity | country, plus 24+ subnational models | Described as "open-source"; **explicit license UNVERIFIED** | `https://drmkc.jrc.ec.europa.eu/inform-index` |
| GDACS | Near-real-time alerts (EQ, TC, FL, VO, DR, WF) with alert levels | event | **UNVERIFIED** terms | `https://www.gdacs.org/gdacsapi/api/events/geteventlist/SEARCH?eventlist=EQ;TC;FL;VO;DR;WF&fromDate=2026-09-01&toDate=2026-09-25&alertlevel=Orange;Red` returns GeoJSON, **CORS \*** |
| Copernicus EMS | Rapid, preparedness and recovery mapping products | event AOIs | Free public viewing and download, except sensitive activations; license wording not on the page | `https://mapping.emergency.copernicus.eu/` |
| WorldPop | Gridded population, e.g. 2020 USA `usa_ppp_2020.tif` | 100 m / 1 km | **CC BY 4.0** (`https://hub.worldpop.org/data/licence.txt`) | `https://hub.worldpop.org/rest/data/pop/wpgp?iso3=USA` |
| GEM Global Seismic Hazard Map **v2026.1** | PGA at 10%/50 yr on rock (Vs30 800), up to 42 layers | raster / vector | **CC BY-NC-SA 4.0** for data (**non-commercial, share-alike**); poster/PNG CC BY-SA 4.0; commercial use needs a license | `https://www.globalquakemodel.org/product/global-seismic-hazard-map` |
| WRI Aqueduct Floods | Riverine and coastal inundation, current and 2030/2050/2080 | about 1 km (**UNVERIFIED**) | **CC BY** (WRI CKAN `license_id: cc-by`) | `https://www.wri.org/data/aqueduct-floods-hazard-maps` |

For a global version, use ThinkHazard! (admin-2 classes) plus GEM (non-commercial) or Aqueduct (CC BY) for magnitudes, and GDACS for live alerts. There is no global equivalent of NRI's dollar-denominated EAL.

---

## 10. Bundling plan for a static site

### 10.1 Tier A: core bundle, loaded on first visit (target ≤ 5 MB gz)

| Asset | Rows | Size (gz) | Status |
|---|---|---|---|
| NRI county, trimmed to 182 fields | 3,232 | **1.21 MB** | measured. Dropping `RISKR`/`EXPP` would bring it to about 0.7 MB (estimate). |
| County polygons, CB 2024 20m, GeoJSON with 3-decimal coordinates (for click-to-select) | 3,222 | **0.31 MB** (5m: 1.03 MB) | measured. TopoJSON would be smaller. |
| ZCTA → centroid + county land-share list | 33,791 | **0.40 MB** | measured |
| CRE 2024 county (vulnerability) | ~3,200 | ~0.1 MB (374 KB raw) | estimate from raw size |
| ACS county household defaults, about 15 variables (built with key) | ~3,222 | ~0.15 MB | estimate |
| NFIP penetration and SFHA share | 3,159 | <0.1 MB | estimate |
| EAGLE-I derived county outage statistics (about 10 metrics) | ~3,100 | ~0.1 MB | estimate |
| Derived county hazard rates from HURDAT2, SPC and Storm Events (about 20 fields) | ~3,200 | ~0.3 MB | estimate |
| NSHM PGA exceedance at 3 thresholds per ZCTA | 33,791 | ~0.3 MB | estimate |
| NCA5 Atlas county values at 4 warming levels × ~15 variables | 3,111 | ~0.4 MB | estimate |
| Volcano list plus simplified threat polygons (source gdb.zip 1.66 MB) | 170 + polygons | ~0.3–0.6 MB | estimate |
| Nuclear sites (57), county counts of TRI facilities and High-hazard dams | – | ~0.2 MB | estimate |
| Base-rate constants (section 8), with a source and date on every number | – | <10 KB | – |
| **Total** | | **≈ 3.5–4.5 MB gz** | Well inside a 5–20 MB budget, and works fully offline for county-level planning. |

### 10.2 Tier B: lazy per-state packs, served from the app's own static host

- **NRI tract pack** (trimmed): median 0.29 MB gz, largest CA 2.02 MB; 23.2 MB for all states (measured).
- Tract polygons per state, for click-to-select tract. Source zips are CA 4.28 MB and PA 1.68 MB. After simplification and quantization, expect a few hundred KB to about 1 MB gz (**estimate**).
- Tract-level ACS/CRE/SVI, ZCTA-level HHI, and WRC community ranks.
- NID High/Significant-hazard dams and TRI facility points per state, with nearest-distance precomputed on ZCTA centroids.
- Privacy note: the static host learns which **state** was requested, and nothing finer. For maximum privacy or offline use, offer "download all states" (about 25–30 MB gz).

### 10.3 Tier C: on-demand fetches (opt-in, with plain-language disclosure)

| Fetch | Why it can't be bundled | Browser-able? | Privacy-preserving variant |
|---|---|---|---|
| NFHL flood zone at the home | 5.8M polygons | Yes (CORS reflect) | Instead of sending the exact point, request the **envelope of a snapped grid cell** (e.g. 0.01°, about 1 km) with `returnGeometry=true&maxAllowableOffset=...` and run point-in-polygon locally, so FEMA sees only the cell. **Design suggestion; payload size in dense areas not tested.** Fallback with no fetch at all: the county SFHA share from NFIP penetration data. |
| NOAA SLR depth at the home | 3–10 m rasters | Yes (CORS reflect) | Same snapped-envelope identify, or use the precomputed tract share. |
| Live alerts | real time | NWS `api.weather.gov/alerts/active?zone=PAC101` (CORS \*; queried by county or zone code, not a point). USGS HANS (CORS \*). GDACS (CORS \*). OpenFEMA declarations by county (CORS \*). | Query by county or zone code. |
| **Not from the browser** | – | Census Data API (key), Census Geocoder (no CORS; JSONP; sends the address), NOAA PFDS (no CORS), USDM API (no CORS), ThinkHazard! (no CORS) | Precompute at build time. |

### 10.4 Tier D: build-time ETL jobs (CI job; cache raw inputs; record every source URL, version and access timestamp)

1. **NRI**: fetch through the Hub export, which returns signed-blob URLs, or the OpenFEMA ZIPs (need a browser-like client; fema.gov returns 403 to curl). Trim, round to 4 significant digits, split tracts by state, and embed `NRI_VER`, access time and the required disclaimer. Inputs: 465 MB tract CSV and 19 MB county CSV.
2. **Event rates**: HURDAT2 (11 MB), SPC SVRGIS (about 22 MB), Storm Events (318 MB gz) → per-county annual rates, P(≥1), and typical damage and duration.
3. **Seismic**: NSHM `hazard_output_CONUS.zip` (934 MB) plus AK/HI and the MMI VI 100-yr map (32 MB) → per-ZCTA/county exceedance probabilities. No runtime calls to the USGS service needed.
4. **Wildfire**: WRC xlsx (5 MB) for ranks. Optionally, zonal statistics of BP and RPS rasters (8–34 GB each) per tract or ZCTA, run occasionally on a large runner. Otherwise NRI `WFIR` at tract level.
5. **Rainfall**: Atlas 14 region grids → 100-yr 24-h depth per county/ZCTA. Schedule a swap to Atlas 15 (CONUS preliminary Sept 2026, final 2027).
6. **Drought**: USDM county-statistics API, 2000–present → share of weeks in D2+ and D3+ per county (about 3,200 requests; throttle).
7. **Outages**: EAGLE-I 11.6 GB → per-county duration-exceedance curves normalized by `MCC.csv` customers, plus EIA-861 state SAIDI (download manually if eia.gov blocks CI).
8. **Demographics**: ACS 2020–2024 via API (key in a CI secret), CRE 2024, SVI 2022, HHI 2024.
9. **Flood priors**: NFIP penetration (SFHA share) and NFIP claims v3 (claims per year and mean paid, by county). Migrate off the `Fima*` v2 endpoints before 2026-10-15.
10. **Climate**: NCA5 Atlas at each warming level plus LOCA2 decadal series, and CMRA 2025 (8 MB gz), trimmed to about 15 variables.
11. **Facilities**: NID (67 MB), TRI (60 MB/yr), FEMA nuclear sites and rings, volcano threat layer → per-county lists plus nearest distance from each ZCTA centroid.
12. **Geography**: CB county 20m/5m, per-state tract CB, Gazetteer ZCTA centroids, ZCTA↔county relationship file, **CT old-county ↔ planning-region crosswalk**, Alaska county changes. Add a consistency test that every county FIPS joins across all packs.

### 10.5 Privacy: which lookups send location to a third party

| Lookup | What leaves the device | Default in the app | Mitigation |
|---|---|---|---|
| Census Geocoder | **full street address** | **off** | Bundled ZCTA centroids plus county polygons; map click for tract |
| NFHL / NOAA SLR / NRI REST by point / USGS NSHM service | **exact lat/lon** | off (opt-in) | Snapped-cell envelope queries; precomputed grids |
| NWS alerts, OpenFEMA, ACIS by county | county FIPS or zone code | on (coarse) | Optionally prefetch all counties at build time |
| State pack from the app's own host | state | on | "Download everything" offline mode |
| CDNs, web fonts, analytics | IP address and referrer | – | Self-host all assets; no analytics or third-party scripts |
| Household details (size, medical needs, budget) | nothing | stays local | Keep in localStorage or IndexedDB only; export and import as a file |

### 10.6 Attribution and license obligations to surface in the app

- NRI: disclaimer, version and access date.
- OpenFEMA: disclaimer.
- EAGLE-I and NCA5 Atlas: CC BY 4.0 attribution.
- WorldPop: CC BY 4.0. Aqueduct Floods: CC BY.
- USFS WRC: citation.
- CDC SVI and HHI: citation.
- **Avoid bundling:** GEM (NC-SA), rmpmap (BY-SA), First Street, PowerOutage.us.
- HUD crosswalk: terms unclear, so use it at build time only.

---

## 11. Consolidated UNVERIFIED items

- Per-file sizes of the OpenFEMA NRI ZIPs (the page gives a 39–411.09 MB range) and the exact v1.20 public release date (June 25, 2025 or Dec 2025).
- NRI population-loss weighting of injuries within `EALP`.
- NFHL bulk download size and terms. The share of NFIP claims outside the SFHA.
- NOAA Atlas 14 grid resolution. Current contents of the DOE OE-417 site (it timed out). EIA-861 file URL and size. PowerOutage.us, First Street and HUD crosswalk redistribution terms.
- NRC reactor-list pages (blocked). CDC WONDER API restrictions (blocked). US Drought Monitor terms. ACIS terms.
- LOCA2 resolution, model count and volume. Climate Toolbox license. CMRA scenario labeling (RCP vs SSP) and license. Location of the NOAA 2022 SLR scenario data.
- ThinkHazard! data license, INFORM license, GDACS terms, Aqueduct resolution.
- Which states publish tsunami evacuation-zone GIS.
- COVID-19 US death toll. A person-based job-displacement rate.
- EJScreen mirror license and completeness.

## 12. Key source URLs (all accessed 2026-09-25)

- NRI: https://www.fema.gov/about/openfema/data-sets/national-risk-index-data · https://services.arcgis.com/XG15cJAlne2vxtgt/arcgis/rest/services/National_Risk_Index_Counties/FeatureServer/0 · https://fema.maps.arcgis.com/home/item.html?id=9da4eeb936544335a6db0cd7a8448a51 (tracts item: terms, description) · data dictionary https://fema.maps.arcgis.com/sharing/rest/content/items/4b9db412e99542029b3c37c37ad714bb/data
- OpenFEMA: https://www.fema.gov/api/open/v1/DataSets · https://www.fema.gov/about/openfema/terms-conditions
- NFHL: https://hazards.fema.gov/arcgis/rest/services/public/NFHL/MapServer
- USGS: https://doi.org/10.5066/P9GNPCOD · https://doi.org/10.5066/P14VGAV4 · https://earthquake.usgs.gov/ws/nshmp/ · https://volcanoes.usgs.gov/hans-public/api/volcano/getUSVolcanoes · https://doi.org/10.5066/P13KAGU3
- USFS: https://www.fs.usda.gov/rds/archive/catalog/RDS-2020-0016-2 · https://www.fs.usda.gov/rds/archive/catalog/RDS-2015-0047-4 · https://wildfirerisk.org/download/
- NOAA: https://www.spc.noaa.gov/wcm/ · https://www.nhc.noaa.gov/data/hurdat/ · https://hdsc.nws.noaa.gov/pfds/ · https://water.noaa.gov/about/atlas15 · https://www.ncei.noaa.gov/pub/data/swdi/stormevents/csvfiles/ · https://coast.noaa.gov/slrdata/ · https://www.wpc.ncep.noaa.gov/wwd/wssi/wssi.php · https://www.ngdc.noaa.gov/hazel/hazard-service/api/v1/tsunamis/events
- Drought: https://usdmdataservices.unl.edu/api/
- Heat: https://atsdr.cdc.gov/place-health/php/hhi/index.html
- Power: https://api.figshare.com/v2/articles/24237376 · https://openenergyhub.ornl.gov/explore/dataset/oe-417-annual-summaries/ · https://www.eia.gov/todayinenergy/detail.php?id=66744
- Demographics: https://svi.cdc.gov/Documents/Data/2022/csv/states_counties/SVI_2022_US_county.csv · https://www2.census.gov/programs-surveys/demo/datasets/community-resilience/2024/ · https://api.census.gov/data/2024/acs/acs5 · https://screening-tools.com/epa-ejscreen · https://www.hsdl.org/hifld/
- Facilities: https://nid.sec.usace.army.mil/api/nation/csv · https://data.epa.gov/efservice/downloads/tri/mv_tri_basic_download/2024_US/csv · https://www.data-liberation-project.org/datasets/epa-risk-management-program-database/ · https://gis.fema.gov/arcgis/rest/services/Partner/Operating_Nuclear_Power_Plant_Sites/FeatureServer/0
- Geography: https://www2.census.gov/geo/tiger/GENZ2024/shp/ · https://www2.census.gov/geo/docs/maps-data/data/rel2020/zcta520/ · https://www2.census.gov/geo/docs/maps-data/data/gazetteer/2024_Gazetteer/ · https://www.huduser.gov/portal/datasets/usps_crosswalk.html · https://geocoding.geo.census.gov/geocoder/
- Climate: https://services3.arcgis.com/0Fs3HcaFfvzXvm7w/arcgis/rest/services/NCA_Atlas_GWL_2C/FeatureServer/0 · https://toolkit.climate.gov/NCA5 · https://resilience.climate.gov/ · https://grid2.rcc-acis.org/GridData · https://cirrus.ucsd.edu/~pierce/LOCA2/
- Base rates: https://www.usfa.fema.gov/statistics/residential-fires/ · https://www2.census.gov/programs-surveys/demo/tables/families/time-series/households/hh1.xls · https://api.bls.gov/publicAPI/v2/timeseries/data/ · https://www.cdc.gov/nchs/fastats/accidental-injury.htm · https://www.cdc.gov/nchs/fastats/emergency-department.htm · https://crashstats.nhtsa.dot.gov/Api/Public/ViewPublication/813800 · https://crashstats.nhtsa.dot.gov/Api/Public/ViewPublication/813705 · https://archive.cdc.gov/www_cdc_gov/flu/pandemic-resources/1918-pandemic-h1n1.html
- Global: https://thinkhazard.org/ · https://drmkc.jrc.ec.europa.eu/inform-index · https://www.gdacs.org/gdacsapi/ · https://mapping.emergency.copernicus.eu/ · https://hub.worldpop.org/ · https://www.globalquakemodel.org/product/global-seismic-hazard-map · https://www.wri.org/data/aqueduct-floods-hazard-maps

Scratch artifacts from this session (can be deleted) are in `research/ds/` plus `nri_counties.csv`, `nri_tracts.csv`, `NRIDataDictionary.csv` and `trim_nri.py` in the same folder. `trim_nri.py` reproduces the trimmed NRI pack sizes.
