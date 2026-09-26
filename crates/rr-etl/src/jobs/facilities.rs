//! Job 8 — facilities that change hazard chances (all public domain):
//!
//! - **FEMA Operating Nuclear Power Plant Sites** (57 sites): distance from each county's internal
//!   point and each ZIP centroid to the nearest site, and whether any part of the county lies
//!   within 10 miles (16 km, the plume emergency planning zone) or 50 miles (80 km, the
//!   ingestion planning zone) of a site.
//! - **EPA Toxics Release Inventory 2024** (the latest reporting year): facilities per county
//!   (by location inside the county boundary) and facilities within 5 km of each ZIP centroid.
//! - **USACE National Inventory of Dams**: dams with High and Significant hazard potential per
//!   county (hazard potential rates the consequence of failure, not the chance of it), High dams
//!   whose latest condition assessment is Poor or Unsatisfactory, and per ZIP the High dams within
//!   10 km whose inventory entry names, as the place a failure would most likely flood (NID
//!   `City`: "the nearest downstream city, town, or village"), a Census place the ZIP overlaps.
//! - **Strategic sites** (`core/strategic_sites.toml`, written by the `strategic` job): per ZIP,
//!   the nearest class A or C1 point site within 150 km, its distance and bearing.
//!
//! County boundaries: Census 2024 cartographic 1:500,000 counties. ZIP points: the Census 2024
//! Gazetteer's ZCTA internal points, rounded to 3 decimals (`geography::zcta_points`).

use super::{Ctx, JobOutput, arcgis_query, attr_f64, load_counties};
use crate::csvout::{Table, col, parse_delimited};
use crate::geo::{KM_PER_MILE, Poly, haversine_km};
use crate::http::zip_entry;
use crate::manifest::Attribution;
use crate::num::{fixed, sig4};
use crate::shp::{Shape, read_dbf, read_shp};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// County facility counts and nuclear distances.
pub const FACILITIES: &str = "core/facilities.csv";
/// ZIP-level nuclear distance and nearby TRI facilities.
pub const ZIP_FACILITIES: &str = "core/zip_facilities.csv";

const NUCLEAR: &str = "https://gis.fema.gov/arcgis/rest/services/Partner/Operating_Nuclear_Power_Plant_Sites/FeatureServer/0";
const TRI_URL: &str =
    "https://data.epa.gov/efservice/downloads/tri/mv_tri_basic_download/{year}_US/csv";
const NID_URL: &str = "https://nid.sec.usace.army.mil/api/nation/csv";
const CB500: &str = "https://www2.census.gov/geo/tiger/GENZ2024/shp/cb_2024_us_county_500k.zip";

/// Latest TRI reporting year to try first (the job falls back one year at a time).
pub const TRI_YEAR: i32 = 2024;
/// Radius for "TRI facilities near this ZIP".
pub const TRI_RADIUS_KM: f64 = 5.0;
/// Radius for "a High dam naming this ZIP's town is close".
pub const DAM_RADIUS_KM: f64 = 10.0;
/// A ZIP counts as overlapping a Census place when at least this share of the ZIP's land is in
/// the place, or this share of the place's land is in the ZIP (a small town in a large rural
/// ZIP).
pub const PLACE_SHARE: f64 = 0.1;
/// Farthest a strategic point site is reported from a ZIP, km (the class D reach).
pub const STRATEGIC_KM: f64 = 150.0;
const ZCTA_PLACE: &str = "https://www2.census.gov/geo/docs/maps-data/data/rel2020/zcta520/tab20_zcta520_place20_natl.txt";

/// Normalise a place name for matching NID's downstream town to Census places: lower case,
/// "City of" and legal suffixes (city, town, village, CDP, borough, ...) dropped, a trailing
/// ", ST" dropped, "Saint"/"Mount"/"Fort" abbreviated, punctuation removed. "none" and other
/// fillers give an empty string.
pub fn norm_place(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut t = lower.split(',').next().unwrap_or("").to_string();
    if let Some(i) = t.find('(') {
        t.truncate(i);
    }
    for prefix in [
        "city of ",
        "town of ",
        "village of ",
        "borough of ",
        "township of ",
    ] {
        if let Some(rest) = t.trim_start().strip_prefix(prefix) {
            t = rest.to_string();
        }
    }
    let cleaned: String = t
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let mut words: Vec<&str> = cleaned.split_whitespace().collect();
    const SUFFIX: &[&str] = &[
        "city",
        "town",
        "village",
        "cdp",
        "borough",
        "municipality",
        "comunidad",
        "urbana",
        "zona",
        "township",
        "government",
        "metropolitan",
        "metro",
        "consolidated",
        "unified",
        "urban",
        "county",
    ];
    while words.len() > 1 && words.last().is_some_and(|w| SUFFIX.contains(w)) {
        words.pop();
    }
    let joined = words
        .iter()
        .map(|w| match *w {
            "saint" => "st",
            "mount" => "mt",
            "fort" => "ft",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ");
    if matches!(
        joined.as_str(),
        "none" | "na" | "n a" | "unknown" | "no" | "tbd"
    ) {
        String::new()
    } else {
        joined
    }
}

/// A uniform grid of points for radius queries.
struct PointGrid {
    cell: f64,
    cells: HashMap<(i32, i32), Vec<(f64, f64)>>,
}

impl PointGrid {
    fn new(cell: f64, pts: &[(f64, f64)]) -> Self {
        let mut cells: HashMap<(i32, i32), Vec<(f64, f64)>> = HashMap::new();
        for &(lat, lon) in pts {
            cells
                .entry(((lon / cell).floor() as i32, (lat / cell).floor() as i32))
                .or_default()
                .push((lat, lon));
        }
        Self { cell, cells }
    }
    fn count_within(&self, lat: f64, lon: f64, km: f64) -> usize {
        let (kx, ky) = (
            (lon / self.cell).floor() as i32,
            (lat / self.cell).floor() as i32,
        );
        let mut n = 0;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &(plat, plon) in self.cells.get(&(kx + dx, ky + dy)).into_iter().flatten() {
                    if haversine_km(lat, lon, plat, plon) <= km {
                        n += 1;
                    }
                }
            }
        }
        n
    }
}

/// Counties as polygons with a coarse bounding-box index for point-in-polygon.
struct CountyIndex {
    polys: Vec<(String, Poly)>,
    grid: HashMap<(i32, i32), Vec<usize>>,
}

impl CountyIndex {
    fn new(polys: Vec<(String, Poly)>) -> Self {
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, (_, p)) in polys.iter().enumerate() {
            let (x0, y0, x1, y1) = (
                p.bbox[0].floor() as i32,
                p.bbox[1].floor() as i32,
                p.bbox[2].floor() as i32,
                p.bbox[3].floor() as i32,
            );
            // Guard against polygons spanning the antimeridian (Aleutians): index by each ring.
            if x1 - x0 > 180 {
                for r in &p.rings {
                    let sub = Poly::new(vec![r.clone()]);
                    for x in sub.bbox[0].floor() as i32..=sub.bbox[2].floor() as i32 {
                        for y in sub.bbox[1].floor() as i32..=sub.bbox[3].floor() as i32 {
                            grid.entry((x, y)).or_default().push(i);
                        }
                    }
                }
                continue;
            }
            for x in x0..=x1 {
                for y in y0..=y1 {
                    grid.entry((x, y)).or_default().push(i);
                }
            }
        }
        for v in grid.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        Self { polys, grid }
    }
    fn locate(&self, lat: f64, lon: f64) -> Option<&str> {
        let cands = self.grid.get(&(lon.floor() as i32, lat.floor() as i32))?;
        cands
            .iter()
            .find(|i| self.polys[**i].1.contains(lon, lat))
            .map(|i| self.polys[*i].0.as_str())
    }
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();

    // County polygons.
    let cb = ctx
        .http
        .get(CB500, Some("facilities/cb_2024_us_county_500k.zip"))?;
    out.source(super::source_from(
        "Census cartographic boundary counties 2024, 1:500,000 (for point-in-polygon)",
        &cb,
        "GENZ2024",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let shp = read_shp(&zip_entry(&cb.bytes, ".shp")?)?;
    let dbf = read_dbf(&zip_entry(&cb.bytes, ".dbf")?)?;
    let ig = dbf.field("GEOID")?;
    let mut polys = Vec::new();
    for (s, r) in shp.into_iter().zip(dbf.records.iter()) {
        if let Shape::Polygon(rings) = s
            && canon.contains(&r[ig])
        {
            polys.push((r[ig].clone(), Poly::new(rings)));
        }
    }
    let index = CountyIndex::new(polys);

    // Nuclear sites.
    let q = arcgis_query(
        ctx,
        "FEMA Operating Nuclear Power Plant Sites",
        NUCLEAR,
        "1=1",
        &["plant_name", "latitude", "longitude", "update_dat"],
        "plant_name",
        false,
        "FEMA Partner layer Operating_Nuclear_Power_Plant_Sites",
        super::PUBLIC_DOMAIN,
        "",
    )?;
    let sites: Vec<(f64, f64)> = q
        .rows
        .iter()
        .filter_map(|r| Some((attr_f64(r, "latitude")?, attr_f64(r, "longitude")?)))
        .collect();
    if sites.len() < 40 {
        return Err(data_err(format!(
            "FEMA nuclear site layer returned only {} sites",
            sites.len()
        )));
    }
    out.rows_in += sites.len() as u64;
    out.source(q.source);
    let epz = 10.0 * KM_PER_MILE;
    let ipz = 50.0 * KM_PER_MILE;

    // TRI (latest year available).
    let mut tri_pts: Vec<(f64, f64)> = Vec::new();
    let mut tri_by_county: BTreeMap<String, u32> = BTreeMap::new();
    let mut tri_no_loc = 0u32;
    let mut tri_year_used = 0;
    for year in (TRI_YEAR - 2..=TRI_YEAR).rev() {
        let url = TRI_URL.replace("{year}", &year.to_string());
        let Ok(f) = ctx
            .http
            .get(&url, Some(&format!("facilities/tri_{year}_us.csv")))
        else {
            continue;
        };
        let text = f.text();
        if text.len() < 1_000_000 {
            continue;
        }
        out.source(super::source_from(
            &format!("EPA Toxics Release Inventory basic data file {year}, US"),
            &f,
            format!("TRI reporting year {year}"),
            super::PUBLIC_DOMAIN,
            "",
        ));
        let (h, rows) = parse_delimited(&text, b',')?;
        let find = |suffix: &str| {
            h.iter()
                .position(|x| x.ends_with(suffix))
                .ok_or_else(|| data_err(format!("TRI: no column ending {suffix}")))
        };
        let (i_id, i_lat, i_lon) = (find(". TRIFD")?, find(". LATITUDE")?, find(". LONGITUDE")?);
        let mut seen: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for r in &rows {
            let (Ok(lat), Ok(lon)) = (r[i_lat].parse::<f64>(), r[i_lon].parse::<f64>()) else {
                if !seen.contains_key(&r[i_id]) {
                    tri_no_loc += 1;
                }
                continue;
            };
            if lat == 0.0 && lon == 0.0 {
                continue;
            }
            seen.entry(r[i_id].clone()).or_insert((lat, lon));
        }
        for (lat, lon) in seen.values() {
            tri_pts.push((*lat, *lon));
            if let Some(c) = index.locate(*lat, *lon) {
                *tri_by_county.entry(c.to_string()).or_default() += 1;
            }
        }
        out.rows_in += rows.len() as u64;
        tri_year_used = year;
        break;
    }
    if tri_year_used == 0 {
        return Err(data_err("no TRI basic data file could be downloaded"));
    }
    let tri_grid = PointGrid::new(0.1, &tri_pts);

    // NID.
    let nid = ctx.http.get(NID_URL, Some("facilities/nid_nation.csv"))?;
    let text = nid.text();
    let (first, rest) = text
        .split_once('\n')
        .ok_or_else(|| data_err("NID: empty file"))?;
    let nid_date = first.split(',').nth(1).unwrap_or("").trim().to_string();
    out.source(super::source_from(
        "USACE National Inventory of Dams (nation CSV)",
        &nid,
        format!("Data Last Updated {nid_date}"),
        super::PUBLIC_DOMAIN,
        "",
    ));
    let (h, rows) = parse_delimited(rest, b',')?;
    let (i_lat, i_lon) = (col(&h, "Latitude")?, col(&h, "Longitude")?);
    let i_haz = h
        .iter()
        .position(|x| x.starts_with("Hazard Potential"))
        .ok_or_else(|| data_err("NID: no Hazard Potential column"))?;
    let (i_cond, i_city, i_state) = (
        col(&h, "Condition Assessment")?,
        col(&h, "City")?,
        col(&h, "State")?,
    );
    let mut high: BTreeMap<String, u32> = BTreeMap::new();
    let mut high_poor: BTreeMap<String, u32> = BTreeMap::new();
    let mut significant: BTreeMap<String, u32> = BTreeMap::new();
    let mut nid_unplaced = 0u32;
    // High dams with coordinates: (lat, lon, state name, downstream town as written).
    let mut high_dams: Vec<(f64, f64, String, String)> = Vec::new();
    for r in &rows {
        let haz = r[i_haz].as_str();
        if haz != "High" && haz != "Significant" {
            continue;
        }
        let (Ok(lat), Ok(lon)) = (r[i_lat].parse::<f64>(), r[i_lon].parse::<f64>()) else {
            nid_unplaced += 1;
            continue;
        };
        if haz == "High" {
            high_dams.push((lat, lon, r[i_state].clone(), r[i_city].clone()));
        }
        match index.locate(lat, lon) {
            Some(c) => {
                *(if haz == "High" {
                    &mut high
                } else {
                    &mut significant
                })
                .entry(c.to_string())
                .or_default() += 1;
                if haz == "High" && matches!(r[i_cond].as_str(), "Poor" | "Unsatisfactory") {
                    *high_poor.entry(c.to_string()).or_default() += 1;
                }
            }
            None => nid_unplaced += 1,
        }
    }
    out.rows_in += rows.len() as u64;

    // County table.
    let mut table = Table::new(
        &[
            "fips",
            "nearest_nuclear_km",
            "tri_facilities",
            "high_hazard_dams",
            "nuclear_within_16km",
            "nuclear_within_80km",
            "significant_hazard_dams",
            "dams_high_poor_condition",
        ],
        1,
    );
    let poly_of: BTreeMap<&str, &Poly> = index.polys.iter().map(|(f, p)| (f.as_str(), p)).collect();
    for c in &counties {
        let nearest = sites
            .iter()
            .map(|(lat, lon)| haversine_km(c.lat, c.lon, *lat, *lon))
            .fold(f64::INFINITY, f64::min);
        let (w16, w80) = match poly_of.get(c.fips.as_str()) {
            Some(p) => {
                let d = sites
                    .iter()
                    .filter(|(lat, lon)| p.bbox_distance_km(*lon, *lat) <= ipz)
                    .map(|(lat, lon)| p.distance_km(*lon, *lat))
                    .fold(f64::INFINITY, f64::min);
                (d <= epz, d <= ipz)
            }
            None => (nearest <= epz, nearest <= ipz),
        };
        table.push(vec![
            c.fips.clone(),
            sig4(nearest),
            tri_by_county.get(&c.fips).copied().unwrap_or(0).to_string(),
            high.get(&c.fips).copied().unwrap_or(0).to_string(),
            w16.to_string(),
            w80.to_string(),
            significant.get(&c.fips).copied().unwrap_or(0).to_string(),
            high_poor.get(&c.fips).copied().unwrap_or(0).to_string(),
        ]);
    }
    out.table(ctx, FACILITIES, &mut table)?;

    // ZIP table, measured from each ZIP's internal point (Census Gazetteer).
    let zcta = super::geography::zcta_points(ctx)?;
    out.source(zcta.source);
    out.rows_in += zcta.rows_in;

    // Dams whose named downstream town the ZIP overlaps: Census 2020 ZCTA-to-place file.
    let rel = ctx.http.get(
        ZCTA_PLACE,
        Some("facilities/tab20_zcta520_place20_natl.txt"),
    )?;
    out.source(super::source_from(
        "Census 2020 ZCTA to place relationship file",
        &rel,
        "rel2020 zcta520-place20",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let state_fips: BTreeMap<String, String> = crate::jobs::geography::STATE_FACTS
        .iter()
        .map(|(f, a, _, _)| (a.to_string(), f.to_string()))
        .collect();
    let (gh, grows) = crate::csvout::read_table(&ctx.data, super::geography::COUNTIES)?;
    let (g_st, g_sn) = (col(&gh, "state_abbr")?, col(&gh, "state_name")?);
    let name_to_fips: BTreeMap<String, String> = grows
        .iter()
        .filter_map(|r| Some((r[g_sn].to_lowercase(), state_fips.get(&r[g_st])?.clone())))
        .collect();
    // (state FIPS, normalised place name) -> place GEOIDs; place GEOID -> ZIPs with >= 10% of
    // their land in it.
    let (rh, rrows) = parse_delimited(&rel.text(), b'|')?;
    let (r_z, r_zl, r_p, r_pn, r_pl, r_part) = (
        col(&rh, "GEOID_ZCTA5_20")?,
        col(&rh, "AREALAND_ZCTA5_20")?,
        col(&rh, "GEOID_PLACE_20")?,
        col(&rh, "NAMELSAD_PLACE_20")?,
        col(&rh, "AREALAND_PLACE_20")?,
        col(&rh, "AREALAND_PART")?,
    );
    let mut place_by_name: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut zips_of_place: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &rrows {
        if r[r_p].len() != 7 {
            continue;
        }
        place_by_name
            .entry((r[r_p][..2].to_string(), norm_place(&r[r_pn])))
            .or_default()
            .insert(r[r_p].clone());
        let (zl, pl, part) = (
            r[r_zl].parse::<f64>().unwrap_or(0.0),
            r[r_pl].parse::<f64>().unwrap_or(0.0),
            r[r_part].parse::<f64>().unwrap_or(0.0),
        );
        let share_of_zip = if zl > 0.0 { part / zl } else { 0.0 };
        let share_of_place = if pl > 0.0 { part / pl } else { 0.0 };
        if !r[r_z].is_empty() && (share_of_zip >= PLACE_SHARE || share_of_place >= PLACE_SHARE) {
            zips_of_place
                .entry(r[r_p].clone())
                .or_default()
                .insert(r[r_z].clone());
        }
    }
    out.rows_in += rrows.len() as u64;
    let zip_point: BTreeMap<&str, (f64, f64)> = zcta
        .points
        .iter()
        .map(|(z, la, lo)| (z.as_str(), (*la, *lo)))
        .collect();
    let mut dams_naming: BTreeMap<String, u32> = BTreeMap::new();
    let (mut named, mut matched) = (0u32, 0u32);
    for (lat, lon, state, city) in &high_dams {
        let key = norm_place(city);
        if key.is_empty() {
            continue;
        }
        named += 1;
        let Some(st) = name_to_fips.get(&state.trim().to_lowercase()) else {
            continue;
        };
        let Some(places) = place_by_name.get(&(st.clone(), key)) else {
            continue;
        };
        matched += 1;
        let zips: BTreeSet<&String> = places
            .iter()
            .filter_map(|p| zips_of_place.get(p))
            .flatten()
            .collect();
        for z in zips {
            if let Some((zl, zo)) = zip_point.get(z.as_str())
                && haversine_km(*zl, *zo, *lat, *lon) <= DAM_RADIUS_KM
            {
                *dams_naming.entry(z.clone()).or_default() += 1;
            }
        }
    }
    let match_rate = matched as f64 / named.max(1) as f64;
    if !(0.5..=0.95).contains(&match_rate) {
        return Err(data_err(format!(
            "NID downstream towns matched to Census places: {matched} of {named} ({:.0}%); expected 50-90%",
            100.0 * match_rate
        )));
    }
    if !["95965", "95966"]
        .iter()
        .any(|z| dams_naming.contains_key(*z))
    {
        return Err(data_err(
            "no Oroville ZIP (95965, 95966) is counted downstream of Oroville Dam",
        ));
    }

    // Nearest strategic point site (class A, or a C1 weapons-complex site) within 150 km.
    let strategic_points = strategic_points(ctx)?;
    let mut zt = Table::new(
        &[
            "zip",
            "nearest_nuclear_km",
            "tri_within_5km",
            "dams_high_within_10km_naming_town",
            "strategic_km",
            "strategic_bearing",
            "strategic_site",
        ],
        1,
    );
    let mut strategic_rows = 0usize;
    for (zip, lat, lon) in &zcta.points {
        let (lat, lon) = (*lat, *lon);
        let nearest = sites
            .iter()
            .map(|(a, b)| haversine_km(lat, lon, *a, *b))
            .fold(f64::INFINITY, f64::min);
        let near_site = strategic_points
            .iter()
            .map(|(id, slat, slon)| (haversine_km(lat, lon, *slat, *slon), id, *slat, *slon))
            .filter(|(d, ..)| *d <= STRATEGIC_KM)
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
        let (skm, sbear, sid) = match near_site {
            Some((d, id, slat, slon)) => {
                strategic_rows += 1;
                // Whole kilometres and degrees: the rules work at 30 and 150 km.
                (
                    format!("{:.0}", d.round()),
                    format!(
                        "{:.0}",
                        crate::geo::bearing_deg(lat, lon, slat, slon)
                            .round()
                            .rem_euclid(360.0)
                    ),
                    id.clone(),
                )
            }
            None => (String::new(), String::new(), String::new()),
        };
        zt.push(vec![
            zip.clone(),
            fixed(nearest, 1),
            tri_grid.count_within(lat, lon, TRI_RADIUS_KM).to_string(),
            dams_naming.get(zip).copied().unwrap_or(0).to_string(),
            skm,
            sbear,
            sid,
        ]);
    }
    out.table(ctx, ZIP_FACILITIES, &mut zt)?;
    out.notes.push(format!(
        "dams_high_within_10km_naming_town: High-hazard dams within {DAM_RADIUS_KM} km of the ZIP's centre whose NID City (the nearest downstream place most likely to be flooded by a failure, as dam owners report it) matches, by normalised name in the same state, a Census 2020 place that holds at least {:.0}% of the ZIP's land or has at least that share of its own land in the ZIP. {named} High dams name a town; {matched} ({:.0}%) match a Census place. {} ZIPs have at least one. Inundation maps are not public; the named town is not an inundation boundary.",
        100.0 * PLACE_SHARE,
        100.0 * match_rate,
        dams_naming.len()
    ));
    out.notes.push(format!(
        "dams_high_poor_condition: High-hazard dams whose latest NID condition assessment is Poor or Unsatisfactory ({} in the counties; condition is not rated for about a fifth of High dams). high_hazard_dams is the county's total of High dams (dams_high_total in the engine's record).",
        high_poor.values().sum::<u32>()
    ));
    out.notes.push(format!(
        "strategic_km / strategic_bearing / strategic_site: the nearest class A point site or NNSA weapons-complex site (strategic_sites.toml, rule point_30km) within {STRATEGIC_KM} km of the ZIP's centre, its distance (whole km), the compass bearing from the ZIP to it (whole degrees) and its id; empty beyond {STRATEGIC_KM} km ({strategic_rows} ZIPs have one). Lets a ZIP next to a site in a neighbouring county (Jefferson County, WA across Hood Canal from Bangor) be recognised.",
    ));

    out.notes.push(format!(
        "{} nuclear sites. nuclear_within_16km / _80km: any part of the county (1:500k boundary) within 10 / 50 miles of a site. nearest_nuclear_km is measured from the county's internal point; zip_facilities.csv measures from the ZIP centroid (0.1 km).",
        sites.len()
    ));
    out.notes.push(format!(
        "TRI reporting year {tri_year_used}: {} facilities with coordinates, placed in counties by point-in-polygon ({tri_no_loc} without coordinates skipped); tri_within_5km counts facilities within {TRI_RADIUS_KM} km of the ZIP centroid.",
        tri_pts.len()
    ));
    out.notes.push(format!(
        "NID ({nid_date}): dams rated High or Significant hazard potential, placed by their coordinates ({nid_unplaced} could not be placed). Hazard potential describes what a failure would cause (High: probable loss of life), not how likely a failure is."
    ));
    out.attributions.push(Attribution {
        source: "EPA TRI, USACE NID, FEMA nuclear sites".into(),
        text: format!("Facility counts from EPA Toxics Release Inventory ({tri_year_used}), USACE National Inventory of Dams ({nid_date}) and FEMA's Operating Nuclear Power Plant Sites layer."),
        license: super::PUBLIC_DOMAIN.into(),
        url: "https://nid.sec.usace.army.mil/".into(),
        version: None,
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

/// Point sites from `core/strategic_sites.toml` (written by the `strategic` job): `(id, lat,
/// lon)` for every site whose rule is `point_30km`.
fn strategic_points(ctx: &Ctx) -> Result<Vec<(String, f64, f64)>> {
    let path = ctx.data.join(super::strategic::STRATEGIC_SITES);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        data_err(format!(
            "{} is needed ({e}); run the strategic job first",
            path.display()
        ))
    })?;
    let v: toml::Table = text
        .parse()
        .map_err(|e| data_err(format!("{}: {e}", path.display())))?;
    let mut out = Vec::new();
    for s in v
        .get("site")
        .and_then(|x| x.as_array())
        .into_iter()
        .flatten()
    {
        if s.get("rule").and_then(|x| x.as_str()) != Some("point_30km") {
            continue;
        }
        let (Some(id), Some(lat), Some(lon)) = (
            s.get("id").and_then(|x| x.as_str()),
            s.get("lat").and_then(|x| x.as_float()),
            s.get("lon").and_then(|x| x.as_float()),
        ) else {
            continue;
        };
        out.push((id.to_string(), lat, lon));
    }
    if out.len() < 20 {
        return Err(data_err(format!(
            "{}: only {} point sites",
            path.display(),
            out.len()
        )));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_names_normalise() {
        assert_eq!(norm_place("Oroville city"), "oroville");
        assert_eq!(norm_place("City of St. Louis"), "st louis");
        assert_eq!(norm_place("Saint Louis city"), "st louis");
        assert_eq!(norm_place("Horn Lake, Ms"), "horn lake");
        assert_eq!(norm_place("Mount Vernon town"), "mt vernon");
        assert_eq!(norm_place("Mt. Vernon"), "mt vernon");
        assert_eq!(norm_place("Magalia CDP"), "magalia");
        assert_eq!(
            norm_place("Nashville-Davidson metropolitan government (balance)"),
            "nashville davidson"
        );
        assert_eq!(norm_place("Junction City"), "junction");
        assert_eq!(norm_place("Junction City city"), "junction");
        assert_eq!(norm_place("none"), "");
        assert_eq!(norm_place("   Bishop     "), "bishop");
    }

    #[test]
    fn point_grid_counts_within_radius() {
        let pts = [(40.0, -75.0), (40.03, -75.0), (40.2, -75.0)];
        let g = PointGrid::new(0.1, &pts);
        // 0.03 degrees of latitude is about 3.3 km; 0.2 is about 22 km.
        assert_eq!(g.count_within(40.0, -75.0, 5.0), 2);
        assert_eq!(g.count_within(40.0, -75.0, 1.0), 1);
    }
}
