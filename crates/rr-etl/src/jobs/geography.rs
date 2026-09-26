//! Job 1 — geography: the canonical county list, states, ZIP-to-county shares, ZIP centroids,
//! the Connecticut crosswalk and the county map.
//!
//! Sources (all Census Bureau, public domain, except the NCA5 region polygons, CC0):
//! - Cartographic boundary counties 2024, 1:500,000 (names, territories) and 1:20,000,000 (map);
//! - 2024 Gazetteer county and ZCTA files (internal points, land area);
//! - 2020 ZCTA-to-county relationship file (land area of each ZIP inside each county);
//! - the Connecticut county-subdivision crosswalk and the 2022 CT town-to-ZCTA relationship
//!   file (so Connecticut ZIPs point at planning regions, not the retired counties);
//! - NCA5 Atlas region polygons (to assign each state its National Climate Assessment region).

use super::{Ctx, JobOutput, PUBLIC_DOMAIN, source_from};
use crate::csvout::{Table, col, col_prefix, parse_delimited};
use crate::ct::Crosswalk;
use crate::geo::{Poly, group_rings_rfc7946, round_ring};
use crate::http::{Fetched, zip_entry};
use crate::manifest::Attribution;
use crate::num::{fixed, sig4};
use crate::shp::{Shape, read_dbf, read_shp};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

/// Canonical county list.
pub const COUNTIES: &str = "core/counties.csv";
/// States and territories.
pub const STATES: &str = "core/states.csv";
/// ZIP (ZCTA) to county land shares.
pub const ZIP_COUNTY: &str = "core/zip_county.csv";
/// ZIP (ZCTA) internal points.
pub const ZIP_CENTROIDS: &str = "core/zip_centroids.csv";
/// County map for the web app.
pub const GEO_COUNTIES: &str = "geo/counties.json";

const CB500: &str = "https://www2.census.gov/geo/tiger/GENZ2024/shp/cb_2024_us_county_500k.zip";
const CB20: &str = "https://www2.census.gov/geo/tiger/GENZ2024/shp/cb_2024_us_county_20m.zip";
const GAZ_COUNTIES: &str =
    "https://www2.census.gov/geo/docs/maps-data/data/gazetteer/2024_Gazetteer/2024_Gaz_counties_national.zip";
const GAZ_ZCTA: &str = "https://www2.census.gov/geo/docs/maps-data/data/gazetteer/2024_Gazetteer/2024_Gaz_zcta_national.zip";
const REL_ZCTA_COUNTY: &str =
    "https://www2.census.gov/geo/docs/maps-data/data/rel2020/zcta520/tab20_zcta520_county20_natl.txt";
const CT_TOWNS: &str = "https://www2.census.gov/geo/docs/reference/ct_change/ct_cou_to_cousub_crosswalk.txt";
const CT_ZCTA_TOWN: &str = "https://www2.census.gov/geo/docs/maps-data/data/rel2022/acs22_cousub22_zcta520_st09.txt";
const NCA_REGIONS: &str = "https://services3.arcgis.com/0Fs3HcaFfvzXvm7w/arcgis/rest/services/NCA_Regions/FeatureServer/0/query?where=1%3D1&outFields=RegionName&returnGeometry=true&outSR=4326&maxAllowableOffset=0.02&geometryPrecision=3&f=json";

/// Island areas in the 1:500k file that have no permanent population and no NRI record.
const EXCLUDED: &[(&str, &str)] = &[
    ("60030", "Rose Island, American Samoa (uninhabited atoll)"),
    ("60040", "Swains Island, American Samoa (no NRI record)"),
    ("69085", "Northern Islands, Northern Mariana Islands (no NRI record)"),
];

/// ZIP parts smaller than this share of the ZIP's land are dropped as boundary slivers.
pub const MIN_ZIP_SHARE: f64 = 0.001;

/// State facts that are not in the Census files: marine shoreline and Great Lakes shoreline.
/// `(state FIPS, abbreviation, marine coast, Great Lakes coast)`.
pub const STATE_FACTS: &[(&str, &str, bool, bool)] = &[
    ("01", "AL", true, false), ("02", "AK", true, false), ("04", "AZ", false, false), ("05", "AR", false, false),
    ("06", "CA", true, false), ("08", "CO", false, false), ("09", "CT", true, false), ("10", "DE", true, false),
    ("11", "DC", false, false), ("12", "FL", true, false), ("13", "GA", true, false), ("15", "HI", true, false),
    ("16", "ID", false, false), ("17", "IL", false, true), ("18", "IN", false, true), ("19", "IA", false, false),
    ("20", "KS", false, false), ("21", "KY", false, false), ("22", "LA", true, false), ("23", "ME", true, false),
    ("24", "MD", true, false), ("25", "MA", true, false), ("26", "MI", false, true), ("27", "MN", false, true),
    ("28", "MS", true, false), ("29", "MO", false, false), ("30", "MT", false, false), ("31", "NE", false, false),
    ("32", "NV", false, false), ("33", "NH", true, false), ("34", "NJ", true, false), ("35", "NM", false, false),
    ("36", "NY", true, true), ("37", "NC", true, false), ("38", "ND", false, false), ("39", "OH", false, true),
    ("40", "OK", false, false), ("41", "OR", true, false), ("42", "PA", true, true), ("44", "RI", true, false),
    ("45", "SC", true, false), ("46", "SD", false, false), ("47", "TN", false, false), ("48", "TX", true, false),
    ("49", "UT", false, false), ("50", "VT", false, false), ("51", "VA", true, false), ("53", "WA", true, false),
    ("54", "WV", false, false), ("55", "WI", false, true), ("56", "WY", false, false), ("60", "AS", true, false),
    ("66", "GU", true, false), ("69", "MP", true, false), ("72", "PR", true, false), ("78", "VI", true, false),
];

/// NCA5 region ids, from the Atlas layer's `RegionName` values (renamed to the report's chapter
/// titles where they differ, e.g. "Southern Plains" is the Southern Great Plains chapter).
fn region_id(name: &str) -> Option<&'static str> {
    Some(match name.trim() {
        "Northeast" => "northeast",
        "Southeast" => "southeast",
        "Midwest" => "midwest",
        "Northern Great Plains" => "northern_great_plains",
        "Southern Plains" | "Southern Great Plains" => "southern_great_plains",
        "Northwest" => "northwest",
        "Southwest" => "southwest",
        "Alaska" => "alaska",
        "Caribbean" | "US Caribbean" => "caribbean",
        n if n.starts_with("Haw") => "hawaii_pacific",
        _ => return None,
    })
}

/// Region for territories whose islands the generalised Atlas polygons may miss.
fn fallback_region(state_abbr: &str) -> Option<&'static str> {
    match state_abbr {
        "AS" | "GU" | "MP" | "HI" => Some("hawaii_pacific"),
        "PR" | "VI" => Some("caribbean"),
        "AK" => Some("alaska"),
        _ => None,
    }
}

fn version_of(f: &Fetched, label: &str) -> String {
    match &f.last_modified {
        Some(lm) => format!("{label}; Last-Modified {lm}"),
        None => label.to_string(),
    }
}

struct CbCounty {
    fips: String,
    name: String,
    name_full: String,
    state_abbr: String,
    state_name: String,
    aland_m2: f64,
    poly: Poly,
}

fn read_cb(zip: &[u8]) -> Result<Vec<CbCounty>> {
    let shp = read_shp(&zip_entry(zip, ".shp")?)?;
    let dbf = read_dbf(&zip_entry(zip, ".dbf")?)?;
    if shp.len() != dbf.records.len() {
        return Err(data_err("county shapefile: .shp and .dbf record counts differ"));
    }
    let (ig, in_, il, is, isn, ia) = (
        dbf.field("GEOID")?,
        dbf.field("NAME")?,
        dbf.field("NAMELSAD")?,
        dbf.field("STUSPS")?,
        dbf.field("STATE_NAME")?,
        dbf.field("ALAND")?,
    );
    let mut out = Vec::new();
    for (shape, rec) in shp.into_iter().zip(dbf.records) {
        let rings = match shape {
            Shape::Polygon(r) => r,
            _ => continue,
        };
        out.push(CbCounty {
            fips: rec[ig].clone(),
            name: rec[in_].clone(),
            name_full: rec[il].clone(),
            state_abbr: rec[is].clone(),
            state_name: rec[isn].clone(),
            aland_m2: rec[ia].parse().unwrap_or(0.0),
            poly: Poly::new(rings),
        });
    }
    Ok(out)
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();

    // --- Counties -----------------------------------------------------------------------
    let cb500 = ctx.http.get(CB500, Some("geography/cb_2024_us_county_500k.zip"))?;
    out.source(source_from("Census cartographic boundary counties 2024, 1:500,000", &cb500, version_of(&cb500, "GENZ2024"), PUBLIC_DOMAIN, ""));
    let excluded: BTreeSet<&str> = EXCLUDED.iter().map(|(f, _)| *f).collect();
    let mut cb: Vec<CbCounty> = read_cb(&cb500.bytes)?.into_iter().filter(|c| !excluded.contains(c.fips.as_str())).collect();
    cb.sort_by(|a, b| a.fips.cmp(&b.fips));
    out.rows_in += cb.len() as u64;

    let gaz = ctx.http.get(GAZ_COUNTIES, Some("geography/2024_Gaz_counties_national.zip"))?;
    out.source(source_from("Census 2024 Gazetteer, counties", &gaz, version_of(&gaz, "2024 Gazetteer"), PUBLIC_DOMAIN, ""));
    let gtext = String::from_utf8_lossy(&zip_entry(&gaz.bytes, ".txt")?).to_string();
    let (gh, grows) = parse_delimited(&gtext, b'\t')?;
    let (g_id, g_lat, g_lon, g_land) = (col(&gh, "GEOID")?, col(&gh, "INTPTLAT")?, col(&gh, "INTPTLONG")?, col(&gh, "ALAND_SQMI")?);
    let mut gaz_pts: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new();
    for r in &grows {
        let (Ok(lat), Ok(lon)) = (r[g_lat].parse::<f64>(), r[g_lon].parse::<f64>()) else { continue };
        gaz_pts.insert(r[g_id].clone(), (lat, lon, r[g_land].parse().unwrap_or(f64::NAN)));
    }
    out.rows_in += grows.len() as u64;

    // --- NCA5 regions by point-in-polygon ---------------------------------------------------
    let regions_doc = ctx.http.get(NCA_REGIONS, Some("geography/nca_regions.json"))?;
    out.source(source_from(
        "NCA5 Interactive Atlas regions (ArcGIS layer NCA_Regions)",
        &regions_doc,
        "NCA5 (2023) Atlas regions; generalised to 0.02 degrees for this lookup",
        "CC0 1.0 Universal",
        "",
    ));
    let rj: serde_json::Value = serde_json::from_slice(&regions_doc.bytes)?;
    let mut region_polys: Vec<(&'static str, Poly)> = Vec::new();
    for f in rj["features"].as_array().ok_or_else(|| data_err("NCA regions: no features"))? {
        let name = f["attributes"]["RegionName"].as_str().unwrap_or_default();
        let id = region_id(name).ok_or_else(|| data_err(format!("NCA regions: unknown region name {name:?}")))?;
        let mut rings = Vec::new();
        for ring in f["geometry"]["rings"].as_array().into_iter().flatten() {
            let pts: Vec<[f64; 2]> = ring
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|p| Some([p.get(0)?.as_f64()?, p.get(1)?.as_f64()?]))
                .collect();
            rings.push(pts);
        }
        region_polys.push((id, Poly::new(rings)));
    }

    // Internal point per county: Gazetteer where available, else polygon centroid.
    let mut points: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new();
    let mut centroid_fallback = Vec::new();
    for c in &cb {
        if let Some(p) = gaz_pts.get(&c.fips) {
            points.insert(c.fips.clone(), *p);
        } else {
            let cen = c.poly.centroid().ok_or_else(|| data_err(format!("no centroid for {}", c.fips)))?;
            points.insert(c.fips.clone(), (cen[1], cen[0], c.aland_m2 / 2_589_988.110336));
            centroid_fallback.push(c.fips.clone());
        }
    }

    // State region = the region holding most of its counties' internal points.
    let mut votes: BTreeMap<String, BTreeMap<&'static str, u32>> = BTreeMap::new();
    for c in &cb {
        let (lat, lon, _) = points[&c.fips];
        if let Some((id, _)) = region_polys.iter().find(|(_, p)| p.contains(lon, lat)) {
            *votes.entry(c.state_abbr.clone()).or_default().entry(id).or_default() += 1;
        }
    }
    let mut state_region: BTreeMap<String, &'static str> = BTreeMap::new();
    let mut used_fallback = Vec::new();
    for c in &cb {
        if state_region.contains_key(&c.state_abbr) {
            continue;
        }
        let voted = votes.get(&c.state_abbr).and_then(|v| v.iter().max_by_key(|(id, n)| (**n, std::cmp::Reverse(**id))).map(|(id, _)| *id));
        let region = match voted {
            Some(r) => r,
            None => {
                used_fallback.push(c.state_abbr.clone());
                fallback_region(&c.state_abbr).ok_or_else(|| data_err(format!("no NCA region for {}", c.state_abbr)))?
            }
        };
        state_region.insert(c.state_abbr.clone(), region);
    }

    let mut counties = Table::new(&["fips", "name", "name_full", "state_abbr", "state_name", "lat", "lon", "land_sqmi", "nca_region"], 1);
    for c in &cb {
        let (lat, lon, land) = points[&c.fips];
        counties.push(vec![
            c.fips.clone(),
            c.name.clone(),
            c.name_full.clone(),
            c.state_abbr.clone(),
            c.state_name.clone(),
            fixed(lat, 4),
            fixed(lon, 4),
            sig4(land),
            state_region[&c.state_abbr].to_string(),
        ]);
    }
    out.table(ctx, COUNTIES, &mut counties)?;

    // --- States -------------------------------------------------------------------------
    let mut states = Table::new(&["state_fips", "state_abbr", "state_name", "nca_region", "coastal", "great_lakes"], 1);
    let mut seen = BTreeSet::new();
    for c in &cb {
        if !seen.insert(c.state_abbr.clone()) {
            continue;
        }
        let fact = STATE_FACTS
            .iter()
            .find(|f| f.1 == c.state_abbr)
            .ok_or_else(|| data_err(format!("no state facts for {}", c.state_abbr)))?;
        states.push(vec![
            fact.0.to_string(),
            c.state_abbr.clone(),
            c.state_name.clone(),
            state_region[&c.state_abbr].to_string(),
            fact.2.to_string(),
            fact.3.to_string(),
        ]);
    }
    out.table(ctx, STATES, &mut states)?;

    // --- Connecticut crosswalk ------------------------------------------------------------
    let ct_towns = ctx.http.get(CT_TOWNS, Some("geography/ct_cou_to_cousub_crosswalk.txt"))?;
    out.source(source_from("Census Connecticut county-to-county-subdivision crosswalk", &ct_towns, version_of(&ct_towns, "ct_change crosswalk"), PUBLIC_DOMAIN, ""));
    let ct_zcta = ctx.http.get(CT_ZCTA_TOWN, Some("geography/acs22_cousub22_zcta520_st09.txt"))?;
    out.source(source_from("Census 2022 Connecticut county subdivision to 2020 ZCTA relationship file", &ct_zcta, version_of(&ct_zcta, "rel2022 acs22"), PUBLIC_DOMAIN, ""));
    let (th, trows) = parse_delimited(&ct_towns.text(), b'|')?;
    let (t_state, t_old, t_new, t_geoid) =
        (col_prefix(&th, "STATEFP")?, col_prefix(&th, "OLD_COUNTYFP")?, col_prefix(&th, "NEW_COUNTYFP")?, col_prefix(&th, "NEW_COUSUB_GEOID")?);
    let (zh, zrows) = parse_delimited(&ct_zcta.text(), b'|')?;
    let (z_town, z_town_land, z_zcta, z_zland, z_part) = (
        col(&zh, "GEOID_COUSUB_22")?,
        col(&zh, "AREALAND_COUSUB_22")?,
        col(&zh, "GEOID_ZCTA5_20")?,
        col(&zh, "AREALAND_ZCTA5_20")?,
        col(&zh, "AREALAND_PART")?,
    );
    let mut town_land: BTreeMap<String, f64> = BTreeMap::new();
    for r in &zrows {
        if let Ok(v) = r[z_town_land].parse::<f64>() {
            town_land.insert(r[z_town].clone(), v);
        }
    }
    let mut towns = Vec::new();
    for r in &trows {
        let state = &r[t_state];
        // The file ends with footnote lines; keep only Connecticut rows with real town codes
        // (skip "County subdivisions not defined", which is water).
        let (old, new, geoid) = (&r[t_old], &r[t_new], &r[t_geoid]);
        let is_code = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
        if state != "09" || !is_code(old, 3) || !is_code(new, 3) || !is_code(geoid, 10) || geoid.ends_with("00000") {
            continue;
        }
        let land = town_land.get(geoid).copied().unwrap_or(0.0);
        towns.push((format!("{state}{old}"), format!("{state}{new}"), land));
    }
    let cw = Crosswalk::from_towns(&towns)?;
    out.rows_in += trows.len() as u64;
    let mut cw_table = cw.to_table();
    out.table(ctx, crate::ct::PATH, &mut cw_table)?;

    // --- ZIP to county ------------------------------------------------------------------
    let rel = ctx.http.get(REL_ZCTA_COUNTY, Some("geography/tab20_zcta520_county20_natl.txt"))?;
    out.source(source_from("Census 2020 ZCTA to county relationship file", &rel, version_of(&rel, "rel2020"), PUBLIC_DOMAIN, ""));
    let (rh, rrows) = parse_delimited(&rel.text(), b'|')?;
    let (r_z, r_zland, r_zwater, r_c, r_land, r_water) = (
        col(&rh, "GEOID_ZCTA5_20")?,
        col(&rh, "AREALAND_ZCTA5_20")?,
        col(&rh, "AREAWATER_ZCTA5_20")?,
        col(&rh, "GEOID_COUNTY_20")?,
        col(&rh, "AREALAND_PART")?,
        col(&rh, "AREAWATER_PART")?,
    );
    let mut zip_total: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut parts: BTreeMap<String, BTreeMap<String, (f64, f64)>> = BTreeMap::new();
    let mut ct_old_land: BTreeMap<String, f64> = BTreeMap::new();
    for r in &rrows {
        if r[r_z].is_empty() {
            continue;
        }
        out.rows_in += 1;
        let (zl, zw) = (r[r_zland].parse::<f64>().unwrap_or(0.0), r[r_zwater].parse::<f64>().unwrap_or(0.0));
        zip_total.insert(r[r_z].clone(), (zl, zw));
        let (pl, pw) = (r[r_land].parse::<f64>().unwrap_or(0.0), r[r_water].parse::<f64>().unwrap_or(0.0));
        if r[r_c].starts_with("09") {
            *ct_old_land.entry(r[r_z].clone()).or_default() += pl;
            continue; // Connecticut parts come from the town file below.
        }
        let e = parts.entry(r[r_z].clone()).or_default().entry(r[r_c].clone()).or_insert((0.0, 0.0));
        e.0 += pl;
        e.1 += pw;
    }
    let mut ct_new_land: BTreeMap<String, f64> = BTreeMap::new();
    for r in &zrows {
        if r[z_zcta].is_empty() {
            continue;
        }
        let pl = r[z_part].parse::<f64>().unwrap_or(0.0);
        let region = r[z_town][..5].to_string();
        *ct_new_land.entry(r[z_zcta].clone()).or_default() += pl;
        let e = parts.entry(r[z_zcta].clone()).or_default().entry(region).or_insert((0.0, 0.0));
        e.0 += pl;
        zip_total.entry(r[z_zcta].clone()).or_insert((r[z_zland].parse().unwrap_or(0.0), 0.0));
    }
    // Check the two Census files agree on Connecticut land by ZIP (within 0.5%).
    let mut ct_mismatch = 0;
    for (z, old) in &ct_old_land {
        let new = ct_new_land.get(z).copied().unwrap_or(0.0);
        if (old - new).abs() > 0.005 * old.max(1.0) {
            ct_mismatch += 1;
        }
    }
    if ct_mismatch > 0 {
        out.notes.push(format!("{ct_mismatch} Connecticut ZIPs have a land-area difference over 0.5% between the 2020 county file and the 2022 town file; the town file is used."));
    }

    let canon: BTreeSet<String> = cb.iter().map(|c| c.fips.clone()).collect();
    let mut zip_county = Table::new(&["zip", "county_fips", "land_share"], 2);
    let mut multi = 0usize;
    let mut ambiguous = 0usize;
    let mut dropped_excluded: Vec<String> = Vec::new();
    for (z, cmap) in &parts {
        let (tl, tw) = zip_total.get(z).copied().unwrap_or((0.0, 0.0));
        let mut shares: Vec<(String, f64)> = cmap
            .iter()
            .map(|(c, (l, w))| (c.clone(), if tl > 0.0 { l / tl } else if tw > 0.0 { w / tw } else { 0.0 }))
            .filter(|(_, s)| *s >= MIN_ZIP_SHARE)
            .collect();
        shares.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        if shares.len() > 1 {
            multi += 1;
        }
        if shares.first().is_none_or(|s| s.1 < 0.8) {
            ambiguous += 1;
        }
        for (c, s) in shares {
            if excluded.contains(c.as_str()) {
                dropped_excluded.push(format!("{z}->{c}"));
                continue;
            }
            if !canon.contains(&c) {
                return Err(data_err(format!("ZIP {z} points at county {c}, which is not in the 2024 county list")));
            }
            zip_county.push(vec![z.clone(), c, sig4(s.min(1.0))]);
        }
    }
    out.table(ctx, ZIP_COUNTY, &mut zip_county)?;
    if !dropped_excluded.is_empty() {
        out.notes.push(format!(
            "ZIP parts in the excluded uninhabited island areas were dropped: {}.",
            dropped_excluded.join(", ")
        ));
    }
    out.notes.push(format!(
        "{} ZIPs (ZCTAs); {multi} span more than one county; {ambiguous} have no county holding 80% or more of their land (the engine asks the user to choose).",
        parts.len()
    ));

    // --- ZIP centroids --------------------------------------------------------------------
    let gz = ctx.http.get(GAZ_ZCTA, Some("geography/2024_Gaz_zcta_national.zip"))?;
    out.source(source_from("Census 2024 Gazetteer, ZCTAs", &gz, version_of(&gz, "2024 Gazetteer"), PUBLIC_DOMAIN, ""));
    let ztext = String::from_utf8_lossy(&zip_entry(&gz.bytes, ".txt")?).to_string();
    let (zgh, zgrows) = parse_delimited(&ztext, b'\t')?;
    let (zg_id, zg_lat, zg_lon) = (col(&zgh, "GEOID")?, col(&zgh, "INTPTLAT")?, col(&zgh, "INTPTLONG")?);
    let mut zc = Table::new(&["zip", "lat", "lon"], 1);
    for r in &zgrows {
        let (Ok(lat), Ok(lon)) = (r[zg_lat].parse::<f64>(), r[zg_lon].parse::<f64>()) else { continue };
        zc.push(vec![r[zg_id].clone(), fixed(lat, 3), fixed(lon, 3)]);
    }
    out.rows_in += zgrows.len() as u64;
    out.table(ctx, ZIP_CENTROIDS, &mut zc)?;

    // --- County map (1:20m, 3 decimals) ---------------------------------------------------
    let cb20 = ctx.http.get(CB20, Some("geography/cb_2024_us_county_20m.zip"))?;
    out.source(source_from("Census cartographic boundary counties 2024, 1:20,000,000", &cb20, version_of(&cb20, "GENZ2024"), PUBLIC_DOMAIN, ""));
    let mut map = read_cb(&cb20.bytes)?;
    map.sort_by(|a, b| a.fips.cmp(&b.fips));
    let (json, features) = geojson(&map, &points)?;
    let gz_size = gzip_len(json.as_bytes());
    out.text(ctx, GEO_COUNTIES, &json, features as u64)?;
    let map_ids: BTreeSet<String> = map.iter().map(|c| c.fips.clone()).collect();
    let not_on_map: Vec<String> = canon.iter().filter(|c| !map_ids.contains(*c)).cloned().collect();
    out.notes.push(format!(
        "County map: {features} features from the 1:20,000,000 file, coordinates rounded to 3 decimals, RFC 7946 winding (exterior counter-clockwise); {:.3} MB gzipped. Not on the map (not in the 1:20m file): {}.",
        gz_size as f64 / 1e6,
        if not_on_map.is_empty() { "none".to_string() } else { not_on_map.join(", ") }
    ));

    out.notes.push(format!(
        "Canonical county list: the 2024 1:500,000 cartographic county file ({} counties and equivalents, including Connecticut's 9 planning regions and Puerto Rico), minus {}.",
        cb.len(),
        EXCLUDED.iter().map(|(f, why)| format!("{f} ({why})")).collect::<Vec<_>>().join(", ")
    ));
    if !centroid_fallback.is_empty() {
        out.notes.push(format!(
            "Internal points come from the 2024 Gazetteer; for {} island-area counties not in the Gazetteer ({}) the polygon centroid is used.",
            centroid_fallback.len(),
            centroid_fallback.join(", ")
        ));
    }
    out.notes.push(format!(
        "NCA5 region per state: the region polygon holding most of the state's county internal points{}.",
        if used_fallback.is_empty() { String::new() } else { format!("; islands outside the generalised polygons use their chapter region ({})", used_fallback.join(", ")) }
    ));
    out.notes.push("Connecticut: ZIP shares for Connecticut come from the 2022 town-to-ZCTA file, so they point at planning regions (091xx). ct_crosswalk.csv records the land-area overlap of the 8 retired counties and 9 regions, used to convert sources that still report old counties.".to_string());
    out.notes.push("states.csv: `coastal` means a marine (ocean, gulf or tidal estuary) shoreline; Pennsylvania counts through the tidal Delaware estuary. `great_lakes` means a Great Lakes shoreline. These two flags are curated, not downloaded.".to_string());
    out.attributions.push(Attribution {
        source: "NCA5 Atlas regions".to_string(),
        text: "Region boundaries: U.S. Global Change Research Program, Fifth National Climate Assessment Interactive Atlas (CC0 1.0).".to_string(),
        license: "CC0 1.0".to_string(),
        url: "https://www.arcgis.com/home/item.html?id=d6614156fe694956be25f4bb9f52b378".to_string(),
        version: Some("NCA5 (2023)".to_string()),
        accessed: regions_doc.retrieved[..10].to_string(),
    });
    Ok(out)
}

fn geojson(map: &[CbCounty], points: &BTreeMap<String, (f64, f64, f64)>) -> Result<(String, usize)> {
    let mut s = String::with_capacity(2_000_000);
    s.push_str("{\"type\":\"FeatureCollection\",\"features\":[\n");
    let mut n = 0;
    for c in map {
        let polys: Vec<Vec<Vec<[f64; 2]>>> = group_rings_rfc7946(&c.poly.rings)
            .into_iter()
            .filter_map(|poly| {
                let mut rings = poly.iter();
                let ext = round_ring(rings.next()?, 3)?;
                let mut out = vec![ext];
                out.extend(rings.filter_map(|h| round_ring(h, 3)));
                Some(out)
            })
            .collect();
        if polys.is_empty() {
            continue;
        }
        if n > 0 {
            s.push_str(",\n");
        }
        n += 1;
        let (lat, lon) = points.get(&c.fips).map(|p| (p.0, p.1)).unwrap_or((f64::NAN, f64::NAN));
        s.push_str(&format!(
            "{{\"type\":\"Feature\",\"id\":{},\"properties\":{{\"name\":{},\"state\":{},\"lat\":{},\"lon\":{}}},\"geometry\":",
            serde_json::to_string(&c.fips)?,
            serde_json::to_string(&c.name)?,
            serde_json::to_string(&c.state_abbr)?,
            fixed(lat, 3),
            fixed(lon, 3)
        ));
        let ring_str = |r: &Vec<[f64; 2]>| {
            let pts: Vec<String> = r.iter().map(|p| format!("[{},{}]", fixed(p[0], 3), fixed(p[1], 3))).collect();
            format!("[{}]", pts.join(","))
        };
        let poly_str = |p: &Vec<Vec<[f64; 2]>>| format!("[{}]", p.iter().map(ring_str).collect::<Vec<_>>().join(","));
        if polys.len() == 1 {
            s.push_str(&format!("{{\"type\":\"Polygon\",\"coordinates\":{}}}}}", poly_str(&polys[0])));
        } else {
            s.push_str(&format!(
                "{{\"type\":\"MultiPolygon\",\"coordinates\":[{}]}}}}",
                polys.iter().map(poly_str).collect::<Vec<_>>().join(",")
            ));
        }
    }
    s.push_str("\n]}\n");
    Ok((s, n))
}

/// Size of `bytes` after gzip at the default level (how a static host would serve it).
pub fn gzip_len(bytes: &[u8]) -> usize {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = enc.write_all(bytes);
    enc.finish().map(|v| v.len()).unwrap_or(0)
}
