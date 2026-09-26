//! Job 8 — facilities that change hazard chances (all public domain):
//!
//! - **FEMA Operating Nuclear Power Plant Sites** (57 sites): distance from each county's internal
//!   point and each ZIP centroid to the nearest site, and whether any part of the county lies
//!   within 10 miles (16 km, the plume emergency planning zone) or 50 miles (80 km, the
//!   ingestion planning zone) of a site.
//! - **EPA Toxics Release Inventory 2024** (the latest reporting year): facilities per county
//!   (by location inside the county boundary) and facilities within 5 km of each ZIP centroid.
//! - **USACE National Inventory of Dams**: dams with High and Significant hazard potential per
//!   county (hazard potential rates the consequence of failure, not the chance of it).
//!
//! County boundaries: Census 2024 cartographic 1:500,000 counties.

use super::{Ctx, JobOutput, arcgis_query, attr_f64, load_counties};
use crate::csvout::{Table, col, parse_delimited, read_table};
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
    let mut high: BTreeMap<String, u32> = BTreeMap::new();
    let mut significant: BTreeMap<String, u32> = BTreeMap::new();
    let mut nid_unplaced = 0u32;
    for r in &rows {
        let haz = r[i_haz].as_str();
        if haz != "High" && haz != "Significant" {
            continue;
        }
        let (Ok(lat), Ok(lon)) = (r[i_lat].parse::<f64>(), r[i_lon].parse::<f64>()) else {
            nid_unplaced += 1;
            continue;
        };
        match index.locate(lat, lon) {
            Some(c) => {
                *(if haz == "High" {
                    &mut high
                } else {
                    &mut significant
                })
                .entry(c.to_string())
                .or_default() += 1
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
        ]);
    }
    out.table(ctx, FACILITIES, &mut table)?;

    // ZIP table.
    let (zh, zrows) = read_table(&ctx.data, super::geography::ZIP_CENTROIDS)?;
    let (iz, ilat, ilon) = (col(&zh, "zip")?, col(&zh, "lat")?, col(&zh, "lon")?);
    let mut zt = Table::new(&["zip", "nearest_nuclear_km", "tri_within_5km"], 1);
    for r in &zrows {
        let (Ok(lat), Ok(lon)) = (r[ilat].parse::<f64>(), r[ilon].parse::<f64>()) else {
            continue;
        };
        let nearest = sites
            .iter()
            .map(|(a, b)| haversine_km(lat, lon, *a, *b))
            .fold(f64::INFINITY, f64::min);
        zt.push(vec![
            r[iz].clone(),
            fixed(nearest, 1),
            tri_grid.count_within(lat, lon, TRI_RADIUS_KM).to_string(),
        ]);
    }
    out.table(ctx, ZIP_FACILITIES, &mut zt)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_grid_counts_within_radius() {
        let pts = [(40.0, -75.0), (40.03, -75.0), (40.2, -75.0)];
        let g = PointGrid::new(0.1, &pts);
        // 0.03 degrees of latitude is about 3.3 km; 0.2 is about 22 km.
        assert_eq!(g.count_within(40.0, -75.0, 5.0), 2);
        assert_eq!(g.count_within(40.0, -75.0, 1.0), 1);
    }
}
