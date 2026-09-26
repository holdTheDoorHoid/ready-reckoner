//! Job — people living behind levees, per county, from the USACE National Levee Database (NLD).
//!
//! - **Systems** (public JSON API, one query per state): each levee system's modelled people at
//!   risk (`peopleAtRisk`, from the USACE National Structure Inventory) and its Levee Screening
//!   and Assessment rating (`lsacRatingId`: 1 Very High, 2 High, 3 Moderate, 4 Low, 5 Very Low,
//!   6 No Verdict, 7 In Progress, 8 Not Screened, 9 Not Applicable).
//! - **Leveed areas** (the NLD "Leveed Areas" feature layer): the land each system protects.
//!
//! A system's people are split among the counties its leveed areas cover. Inside the leveed
//! areas, the split follows where people live: the Census 2020 block-group centres of population
//! that fall inside the system's leveed areas, summed by county. Where no block-group centre falls
//! inside (small or empty areas), the split follows land area instead (a lattice of points every
//! 0.005 degrees inside each leveed area, located in the Census 1:500k county outlines); areas too
//! small for a lattice point go to the county holding their first vertex; systems with no
//! leveed-area polygon go to the county holding the system's point. Splitting by the county list
//! instead overstates badly (a first pass put 3.9 million "leveed" people in Broward County, more
//! than live there), and splitting by land alone puts the people of a big river system into its
//! emptiest counties. Nested systems can count people twice, so the county share is capped at 1.

use super::{Ctx, JobOutput, arcgis_query_ex, attr_f64, load_counties};
use crate::csvout::{Table, col, read_table};
use crate::http::Sha256Acc;
use crate::manifest::{Attribution, SourceRecord};
use crate::num::sig;
use crate::raster::{CountyGeo, Lattice, Rings, bbox_of, cell_weight, norm_rings};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};

/// People behind levees per county.
pub const LEVEES: &str = "core/levees.csv";

const SYSTEMS_API: &str = "https://levees.sec.usace.army.mil/api-local/systems/query";
const LEVEED_AREAS: &str =
    "https://geospatial.sec.usace.army.mil/dls/rest/services/NLD/Public/FeatureServer/16";
const NLD_PAGE: &str = "https://levees.sec.usace.army.mil/";
const CENPOP_BG: &str =
    "https://www2.census.gov/geo/docs/reference/cenpop2020/blkgrp/CenPop2020_Mean_BG.txt";

/// Lattice step for splitting a leveed area among counties, degrees.
pub const LATTICE_DEG: f64 = 0.005;

/// LSAC ratings counted as high risk.
pub const HIGH_RISK: &[i64] = &[1, 2];

/// One levee system from the API.
#[derive(Debug, Clone, PartialEq)]
pub struct System {
    /// NLD system id.
    pub id: i64,
    /// Modelled people at risk.
    pub people: f64,
    /// LSAC rating id (8 = not screened).
    pub lsac: i64,
    /// The system's point (lon, lat), where given.
    pub point: Option<(f64, f64)>,
}

/// Parse one state's systems from the API's JSON array.
pub fn parse_systems(bytes: &[u8]) -> Result<Vec<System>> {
    let v: serde_json::Value = serde_json::from_slice(bytes)?;
    let arr = v
        .as_array()
        .ok_or_else(|| data_err("NLD systems query: expected a JSON array"))?;
    let mut out = Vec::new();
    for s in arr {
        let Some(id) = s["systemId"].as_i64().or_else(|| s["id"].as_i64()) else {
            continue;
        };
        let point = s["coords"].as_str().and_then(|c| {
            let (lon, lat) = c.split_once(',')?;
            Some((lon.trim().parse().ok()?, lat.trim().parse().ok()?))
        });
        out.push(System {
            id,
            people: s["peopleAtRisk"].as_f64().unwrap_or(0.0).max(0.0),
            lsac: s["lsacRatingId"].as_i64().unwrap_or(8),
            point,
        });
    }
    Ok(out)
}

/// Area weights of a polygon (rings in degrees, normalised longitudes) by county index, from a
/// lattice of points `step` degrees apart. Empty when the polygon is smaller than a lattice cell.
pub fn split_by_county(
    rings: &Rings,
    step: f64,
    locate: impl Fn(f64, f64) -> Option<usize>,
) -> BTreeMap<usize, f64> {
    let lat = Lattice::covering(bbox_of(rings), step);
    let mut w: BTreeMap<usize, f64> = BTreeMap::new();
    lat.fill(rings, |i, j| {
        let (lon, la) = (lat.lon(i), lat.lat(j));
        if let Some(c) = locate(la, lon) {
            *w.entry(c).or_default() += cell_weight(la);
        }
    });
    w
}

/// Block-group centres of population: (lat, lon, population, county index), with a grid index.
pub struct PeoplePoints {
    pts: Vec<(f64, f64, f64, usize)>,
    grid: std::collections::HashMap<(i32, i32), Vec<usize>>,
}

const PEOPLE_CELL: f64 = 0.1;

impl PeoplePoints {
    /// Build from points (lat, lon with normalised longitudes, population, county index).
    pub fn new(pts: Vec<(f64, f64, f64, usize)>) -> Self {
        let mut grid: std::collections::HashMap<(i32, i32), Vec<usize>> =
            std::collections::HashMap::new();
        for (i, p) in pts.iter().enumerate() {
            let key = (
                (p.1 / PEOPLE_CELL).floor() as i32,
                (p.0 / PEOPLE_CELL).floor() as i32,
            );
            grid.entry(key).or_default().push(i);
        }
        Self { pts, grid }
    }

    /// Population inside a polygon (even-odd across its rings), by county index.
    pub fn inside(&self, rings: &Rings) -> BTreeMap<usize, f64> {
        let b = bbox_of(rings);
        let mut out: BTreeMap<usize, f64> = BTreeMap::new();
        if !b[0].is_finite() {
            return out;
        }
        for x in (b[0] / PEOPLE_CELL).floor() as i32..=(b[2] / PEOPLE_CELL).floor() as i32 {
            for y in (b[1] / PEOPLE_CELL).floor() as i32..=(b[3] / PEOPLE_CELL).floor() as i32 {
                for &i in self.grid.get(&(x, y)).into_iter().flatten() {
                    let (lat, lon, pop, county) = self.pts[i];
                    if lon < b[0] || lon > b[2] || lat < b[1] || lat > b[3] {
                        continue;
                    }
                    let hits = rings
                        .iter()
                        .filter(|r| crate::geo::point_in_ring(lon, lat, r))
                        .count();
                    if hits % 2 == 1 {
                        *out.entry(county).or_default() += pop;
                    }
                }
            }
        }
        out
    }
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();

    // --- Systems, state by state (a national query answers HTTP 500) ------------------------
    let (h, rows) = read_table(&ctx.data, super::geography::COUNTIES)?;
    let i_sn = col(&h, "state_name")?;
    let states: BTreeSet<String> = rows.iter().map(|r| r[i_sn].clone()).collect();
    let mut acc = Sha256Acc::new();
    let retrieved = crate::timefmt::now_utc();
    let mut systems: BTreeMap<i64, System> = BTreeMap::new();
    for st in &states {
        let url = format!(
            "{SYSTEMS_API}?type=sy&sy=*&in=@state:{}",
            st.replace(' ', "%20")
        );
        let f = ctx.http.get(
            &url,
            Some(&format!("levees/systems_{}.json", st.replace(' ', "_"))),
        )?;
        acc.update(&f.bytes);
        for s in parse_systems(&f.bytes)? {
            systems.entry(s.id).or_insert(s);
        }
    }
    let api_people: f64 = systems.values().map(|s| s.people).sum();
    out.rows_in += systems.len() as u64;
    let api_bytes = acc.len();
    out.source(SourceRecord {
        name: "USACE National Levee Database: levee systems by state (public API)".into(),
        url: format!(
            "{SYSTEMS_API}?type=sy&sy=*&in=@state:<state name> ({} states)",
            states.len()
        ),
        version: format!("{} systems as served on the retrieval date", systems.len()),
        retrieved,
        sha256: acc.finish(),
        bytes: api_bytes,
        license: "US Government work; NLD: \"available for public use\"".into(),
        obligations: String::new(),
    });
    if systems.len() < 5000 || !(15e6..=30e6).contains(&api_people) {
        return Err(data_err(format!(
            "NLD API: {} systems, {api_people:.0} people at risk; expected about 6,000 and 22 million",
            systems.len()
        )));
    }

    // --- Leveed areas ---------------------------------------------------------------------
    let q = arcgis_query_ex(
        ctx,
        "USACE National Levee Database: leveed areas (feature layer 16)",
        LEVEED_AREAS,
        "1=1",
        &["SYSTEM_ID", "LEVEED_ID"],
        "OBJECTID",
        true,
        &[
            ("maxAllowableOffset", "0.0005".to_string()),
            ("geometryPrecision", "5".to_string()),
        ],
        "NLD Public FeatureServer layer 16 (Leveed Areas), generalised to 0.0005 degrees",
        "US Government work; NLD: \"available for public use\"",
        "",
    )?;
    out.rows_in += q.rows.len() as u64;
    out.source(q.source);
    let mut areas: BTreeMap<i64, Vec<Rings>> = BTreeMap::new();
    for (row, g) in q.rows.iter().zip(&q.geometry) {
        let Some(sys) = attr_f64(row, "SYSTEM_ID") else {
            continue;
        };
        let rings: Rings = g["rings"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| {
                r.as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| Some([p.get(0)?.as_f64()?, p.get(1)?.as_f64()?]))
                    .collect()
            })
            .collect();
        if !rings.is_empty() {
            areas.entry(sys as i64).or_default().push(norm_rings(rings));
        }
    }

    let (geo, cb_source) = CountyGeo::download(ctx, &canon, "levees/cb_2024_us_county_500k.zip")?;
    out.source(cb_source);
    let locate = |lat: f64, lon: f64| geo.locate_index(lat, lon);

    // Where people live: Census 2020 block-group centres of population, placed in 2024 counties
    // by their own code (Connecticut's, which use the retired counties, by point-in-polygon).
    let bg = ctx
        .http
        .get(CENPOP_BG, Some("levees/CenPop2020_Mean_BG.txt"))?;
    out.source(super::source_from(
        "Census 2020 centers of population by block group",
        &bg,
        "CenPop2020_Mean_BG",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let index_of: BTreeMap<&str, usize> = geo
        .polys
        .iter()
        .enumerate()
        .map(|(i, (f, _))| (f.as_str(), i))
        .collect();
    let people_points = {
        let (bh, brows) = crate::csvout::parse_delimited(&bg.text(), b',')?;
        let (is, ic, ip, ila, ilo) = (
            col(&bh, "STATEFP")?,
            col(&bh, "COUNTYFP")?,
            col(&bh, "POPULATION")?,
            col(&bh, "LATITUDE")?,
            col(&bh, "LONGITUDE")?,
        );
        let mut pts = Vec::with_capacity(brows.len());
        for r in &brows {
            let (Ok(pop), Ok(lat), Ok(lon)) = (
                r[ip].parse::<f64>(),
                r[ila].trim_start_matches('+').parse::<f64>(),
                r[ilo].trim_start_matches('+').parse::<f64>(),
            ) else {
                continue;
            };
            if pop <= 0.0 {
                continue;
            }
            let code = format!("{}{}", r[is], r[ic]);
            let county = match index_of.get(code.as_str()) {
                Some(i) => Some(*i),
                None => locate(lat, lon),
            };
            if let Some(c) = county {
                pts.push((lat, crate::raster::norm_lon(lon), pop, c));
            }
        }
        out.rows_in += brows.len() as u64;
        PeoplePoints::new(pts)
    };

    // --- Split each system's people among counties ---------------------------------------------
    let n = geo.polys.len();
    let mut people = vec![0.0f64; n];
    let mut high = vec![0.0f64; n];
    let (mut by_people, mut by_area, mut by_vertex, mut by_point, mut unplaced) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut unplaced_people = 0.0;
    for s in systems.values() {
        if s.people <= 0.0 {
            continue;
        }
        let mut w: BTreeMap<usize, f64> = BTreeMap::new();
        if let Some(polys) = areas.get(&s.id) {
            for rings in polys {
                for (c, p) in people_points.inside(rings) {
                    *w.entry(c).or_default() += p;
                }
            }
            if !w.is_empty() {
                by_people += 1;
            }
        }
        if w.is_empty()
            && let Some(polys) = areas.get(&s.id)
        {
            for rings in polys {
                let part = split_by_county(rings, LATTICE_DEG, locate);
                if part.is_empty() {
                    // Smaller than a lattice cell: its first vertex decides.
                    if let Some(c) = rings
                        .first()
                        .and_then(|r| r.first())
                        .and_then(|p| locate(p[1], p[0]))
                    {
                        *w.entry(c).or_default() += 1e-9;
                    }
                } else {
                    for (c, x) in part {
                        *w.entry(c).or_default() += x;
                    }
                }
            }
            if !w.is_empty() {
                if w.values().all(|x| *x <= 1e-9) {
                    by_vertex += 1;
                } else {
                    by_area += 1;
                }
            }
        }
        if w.is_empty()
            && let Some((lon, lat)) = s.point
            && let Some(c) = locate(lat, lon)
        {
            w.insert(c, 1.0);
            by_point += 1;
        }
        let total: f64 = w.values().sum();
        if total <= 0.0 {
            unplaced += 1;
            unplaced_people += s.people;
            continue;
        }
        for (c, x) in w {
            let share = s.people * x / total;
            people[c] += share;
            if HIGH_RISK.contains(&s.lsac) {
                high[c] += share;
            }
        }
    }

    // --- Shares of county population ---------------------------------------------------------
    let (h, rows) = read_table(&ctx.data, super::nri::NRI_COUNTIES)?;
    let (i_f, i_p) = (col(&h, "fips")?, col(&h, "population")?);
    let pop: BTreeMap<String, f64> = rows
        .iter()
        .filter_map(|r| Some((r[i_f].clone(), r[i_p].parse::<f64>().ok()?)))
        .collect();
    let mut table = Table::new(&["fips", "leveed_pop_share", "levee_risk_high_share"], 1);
    let mut capped = Vec::new();
    let mut placed_people = 0.0;
    let mut share_of: BTreeMap<String, f64> = BTreeMap::new();
    for c in &counties {
        let idx = geo.polys.iter().position(|(f, _)| f == &c.fips);
        let (p, hi) = idx.map(|i| (people[i], high[i])).unwrap_or((0.0, 0.0));
        let population = pop.get(&c.fips).copied().unwrap_or(0.0);
        let share = if population > 0.0 {
            p / population
        } else {
            0.0
        };
        if share > 1.0 {
            capped.push(format!("{} ({:.2})", c.fips, share));
        }
        let share = share.min(1.0);
        placed_people += p.min(population.max(0.0));
        share_of.insert(c.fips.clone(), share);
        let high_share = if p > 0.0 { (hi / p).min(1.0) } else { 0.0 };
        // Three significant figures: people at risk are modelled counts.
        table.push(vec![c.fips.clone(), sig(share, 3), sig(high_share, 3)]);
    }

    for (fips, what, lo, hi) in [
        ("22089", "St. Charles Parish, LA", 0.5, 1.0),
        ("06067", "Sacramento County, CA", 0.2, 1.0),
        ("04001", "Apache County, AZ", 0.0, 0.01),
    ] {
        let v = share_of.get(fips).copied().unwrap_or(-1.0);
        if !(lo..=hi).contains(&v) {
            return Err(data_err(format!(
                "leveed_pop_share for {what} is {v:.3}, expected {lo}-{hi}"
            )));
        }
    }
    if (placed_people - api_people).abs() > 0.25 * api_people {
        return Err(data_err(format!(
            "people placed in counties ({placed_people:.0}) differ from the API total ({api_people:.0}) by more than 25%"
        )));
    }
    out.table(ctx, LEVEES, &mut table)?;
    out.notes.push(format!(
        "{} levee systems with {:.1} million people at risk (NLD API). People split among counties: {by_people} systems by the Census 2020 block-group population inside their leveed areas, {by_area} by leveed-area land (lattice every {LATTICE_DEG} degrees) where no block-group centre falls inside, {by_vertex} by the first vertex of an area too small for the lattice, {by_point} without a leveed-area polygon by the system's point; {unplaced} systems ({unplaced_people:.0} people) could not be placed. After capping each county at its population, {:.1} million people are counted.",
        systems.len(),
        api_people / 1e6,
        placed_people / 1e6
    ));
    out.notes.push(format!(
        "leveed_pop_share = people behind levees in the county / county population (NRI 2020), capped at 1 because nested systems can count people twice ({}). levee_risk_high_share = of those people, the share behind systems USACE rates Very High or High risk (LSAC); two thirds of systems are not screened, so a 0 there often means 'not rated', not 'low risk'. Counties with no levee system are 0.",
        if capped.is_empty() { "none capped".to_string() } else { format!("capped: {}", capped.join(", ")) }
    ));
    out.definitions.insert("leveed_pop_share".into(), "Share of the county's residents living in areas USACE's National Levee Database shows behind a levee (modelled people at risk from the National Structure Inventory, split among counties by where people live inside the leveed areas).".into());
    out.definitions.insert("levee_risk_high_share".into(), "Share of the county's leveed residents behind levee systems with a USACE Levee Screening and Assessment rating of Very High or High.".into());
    out.attributions.push(Attribution {
        source: "USACE National Levee Database".into(),
        text: "People behind levees and levee risk ratings from the U.S. Army Corps of Engineers National Levee Database.".into(),
        license: "US Government work (public domain)".into(),
        url: NLD_PAGE.into(),
        version: None,
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systems_parse_from_the_api_shape() {
        let json = br#"[{"systemId": 1305000371, "peopleAtRisk": 2, "lsacRatingId": 8, "coords": "-75.5868,39.6505"},
                        {"id": 5, "peopleAtRisk": null, "lsacRatingId": 2}]"#;
        let s = parse_systems(json).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].id, 1305000371);
        assert_eq!(s[0].people, 2.0);
        assert_eq!(s[0].point, Some((-75.5868, 39.6505)));
        assert_eq!((s[1].id, s[1].people, s[1].lsac), (5, 0.0, 2));
    }

    #[test]
    fn a_leveed_area_splits_by_land_in_each_county() {
        // A leveed area 0.1 x 0.1 degrees straddling a county line at lon -89.97: 30% west.
        let rings = vec![vec![
            [-90.0, 30.0],
            [-89.9, 30.0],
            [-89.9, 30.1],
            [-90.0, 30.1],
            [-90.0, 30.0],
        ]];
        let w = split_by_county(&rings, 0.005, |_, lon| {
            Some(if lon < -89.97 { 0 } else { 1 })
        });
        let total: f64 = w.values().sum();
        assert!((w[&0] / total - 0.3).abs() < 0.03, "{w:?}");
        // Smaller than a cell: nothing (the caller uses the first vertex).
        let tiny = vec![vec![
            [-90.0, 30.0],
            [-89.999, 30.0],
            [-89.999, 30.001],
            [-90.0, 30.0],
        ]];
        assert!(split_by_county(&tiny, 0.005, |_, _| Some(0)).is_empty());
    }
}
