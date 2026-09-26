//! Job — ground that can give way: karst (sinkholes) and landslide-susceptible terrain, as a
//! share of each county. Both sources are USGS:
//!
//! - **Karst**: *Karst in the United States: A Digital Map Compilation and Database* (Weary and
//!   Doctor, USGS Open-File Report 2014-1156). Carbonate and evaporite rock polygons, published in
//!   Albers equal-area projections (one per region). The job measures, in that equal-area plane,
//!   the share of each county's land on those rocks at or near the surface or under thin cover
//!   (every polygon except carbonate or evaporite rock buried under more than 50 feet of glacial
//!   sediment, where sinkholes are rare). Karst is a precondition, not a rate: most karst land never
//!   sinks.
//! - **Landslides**: *Slope-Relief Threshold Landslide Susceptibility Models for the United States
//!   and Puerto Rico* (USGS, 2024, doi:10.5066/P13KAGU3, CC0): the published county table's
//!   proportion of each county's area that the model rates susceptible.
//!
//! The karst archive (282 MB) is written to `data/raw/ground/` only while it is read and deleted
//! straight after (unless `--keep-raw`).

use super::{Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::{Table, col, parse_delimited};
use crate::ct::Crosswalk;
use crate::geo::{Albers, Poly};
use crate::http::zip_entry_file;
use crate::manifest::Attribution;
use crate::num::sig;
use crate::raster::{CountyGeo, Lattice};
use crate::shp::{Shape, read_dbf_field, read_shp};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};

/// Karst and landslide shares per county.
pub const GROUND: &str = "core/ground.csv";

const KARST_URL: &str = "https://pubs.usgs.gov/of/2014/1156/downloads/USKarstMap.zip";
const KARST_PAGE: &str = "https://pubs.usgs.gov/of/2014/1156/";
const LANDSLIDE_URL: &str = "https://www.sciencebase.gov/catalog/file/get/65ccea5bd34ef4b119cb3bac?f=__disk__8c%2Fda%2F66%2F8cda66bbacf51b14a8c4c1b649b8b26688d59d27";
const LANDSLIDE_ITEM: &str = "https://www.sciencebase.gov/catalog/item/65ccea5bd34ef4b119cb3bac";

/// Lattice cell for the karst shares, metres (1 km² cells in the equal-area plane).
pub const KARST_CELL_M: f64 = 1000.0;

/// Exposure classes left out: rock buried under more than 50 feet of glacial sediment.
pub const EXCLUDED_EXPOSURE: &[&str] = &["B2"];

/// One karst region: its layers and its Albers projection (origin latitude, central meridian,
/// standard parallels), as the layers' `.prj` files state them.
struct Region {
    name: &'static str,
    layers: &'static [&'static str],
    albers: (f64, f64, f64, f64),
    state_fips: fn(&str) -> bool,
}

fn conus(st: &str) -> bool {
    !matches!(st, "02" | "15" | "72" | "78" | "60" | "66" | "69")
}

const REGIONS: &[Region] = &[
    Region {
        name: "contiguous US",
        layers: &[
            "Continguous48/Carbonates48.shp",
            "Continguous48/Evaporites48.shp",
        ],
        albers: (37.0, -96.0, 25.0, 50.0),
        state_fips: conus,
    },
    Region {
        name: "Alaska",
        layers: &["Alaska/AKcarbonates.shp"],
        albers: (50.0, -154.0, 55.0, 65.0),
        state_fips: |s| s == "02",
    },
    Region {
        name: "Hawaii",
        layers: &["Hawaii/HIcarbonates.shp"],
        albers: (13.0, -157.0, 8.0, 18.0),
        state_fips: |s| s == "15",
    },
    Region {
        name: "Puerto Rico and US Virgin Islands",
        layers: &[
            "PuertoRico_VirginIslands/PRcarbonates.shp",
            "PuertoRico_VirginIslands/VIcarbonates.shp",
        ],
        albers: (15.0, -66.5, 8.0, 18.0),
        state_fips: |s| s == "72" || s == "78",
    },
];

/// Normalise a county name for matching the landslide table to the county list: lower case,
/// accents folded, "Saint" as "st", punctuation dropped.
pub fn norm_county(s: &str) -> String {
    let folded: String = s
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    folded
        .split_whitespace()
        .map(|w| if w == "saint" { "st" } else { w })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Landslide-table rows whose names are not in the 2024 county list: renamed, merged or split
/// counties (retired Connecticut counties go through the crosswalk instead).
const LANDSLIDE_OVERRIDES: &[(&str, &str, &[&str])] = &[
    ("AK", "Petersburg Census Area", &["02195"]), // Petersburg Borough since 2013
    ("AK", "Valdez-Cordova Census Area", &["02063", "02066"]), // split in 2019
    ("AK", "Wade Hampton Census Area", &["02158"]), // renamed Kusilvak in 2015
    ("LA", "La Salle Parish", &["22059"]),        // spelled LaSalle
    ("SD", "Shannon County", &["46102"]),         // renamed Oglala Lakota in 2015
    ("VA", "Bedford city", &[]),                  // merged into Bedford County (51019) in 2013
];

/// Connecticut's retired counties in the landslide table.
const CT_OLD: &[(&str, &str)] = &[
    ("Fairfield County", "09001"),
    ("Hartford County", "09003"),
    ("Litchfield County", "09005"),
    ("Middlesex County", "09007"),
    ("New Haven County", "09009"),
    ("New London County", "09011"),
    ("Tolland County", "09013"),
    ("Windham County", "09015"),
];

/// Parse the landslide county table into shares by 2024 county code. Returns the shares and the
/// table rows that matched nothing.
pub fn landslide_shares(
    text: &str,
    names: &BTreeMap<(String, String), String>,
    cw: &Crosswalk,
) -> Result<(BTreeMap<String, f64>, Vec<String>)> {
    let (h, rows) = parse_delimited(text, b',')?;
    let (ic, ist, ip) = (col(&h, "COUNTY")?, col(&h, "ST")?, col(&h, "prop_susc")?);
    let mut out: BTreeMap<String, f64> = BTreeMap::new();
    let mut ct_old: BTreeMap<String, f64> = BTreeMap::new();
    let mut unmatched = Vec::new();
    for r in &rows {
        let Ok(share) = r[ip].parse::<f64>() else {
            continue;
        };
        let share = share.clamp(0.0, 1.0);
        let (st, county) = (r[ist].as_str(), r[ic].as_str());
        if let Some((_, _, fips)) = LANDSLIDE_OVERRIDES
            .iter()
            .find(|(s, c, _)| *s == st && *c == county)
        {
            for f in *fips {
                out.insert(f.to_string(), share);
            }
            continue;
        }
        if st == "CT"
            && let Some((_, old)) = CT_OLD.iter().find(|(n, _)| *n == county)
        {
            ct_old.insert(old.to_string(), share);
            continue;
        }
        match names.get(&(st.to_string(), norm_county(county))) {
            Some(f) => {
                out.insert(f.clone(), share);
            }
            None => unmatched.push(format!("{county}, {st}")),
        }
    }
    for (region, v) in cw.intensive(&ct_old) {
        out.insert(region, v);
    }
    Ok((out, unmatched))
}

/// Karst polygons of one layer, in the layer's own plane, without the excluded exposure classes.
fn karst_layer(shp: &[u8], dbf: &[u8]) -> Result<(Vec<Vec<Vec<[f64; 2]>>>, usize, usize)> {
    let shapes = read_shp(shp)?;
    let exposure = read_dbf_field(dbf, "Exposure").unwrap_or_default();
    let mut polys = Vec::new();
    let (mut kept, mut dropped) = (0, 0);
    for (i, s) in shapes.into_iter().enumerate() {
        let Shape::Polygon(rings) = s else { continue };
        let ex = exposure.get(i).map(String::as_str).unwrap_or("");
        if EXCLUDED_EXPOSURE.contains(&ex) {
            dropped += 1;
            continue;
        }
        kept += 1;
        polys.push(rings);
    }
    Ok((polys, kept, dropped))
}

/// Share of each county (index into `county_rings`) covered by the karst polygons, measured on a
/// lattice of `cell` metres in the equal-area plane. `None` for a county too small to hold a
/// cell centre (the caller falls back to a point test).
pub fn karst_shares(
    county_rings: &[Vec<Vec<[f64; 2]>>],
    karst: &[Vec<Vec<[f64; 2]>>],
    cell: f64,
) -> Vec<Option<f64>> {
    let mut bbox = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for rings in county_rings {
        let b = crate::raster::bbox_of(rings);
        bbox = [
            bbox[0].min(b[0]),
            bbox[1].min(b[1]),
            bbox[2].max(b[2]),
            bbox[3].max(b[3]),
        ];
    }
    if !bbox[0].is_finite() {
        return Vec::new();
    }
    let lat = Lattice::covering(bbox, cell);
    let mut owner = vec![u16::MAX; lat.nx * lat.ny];
    for (i, rings) in county_rings.iter().enumerate() {
        let idx = i as u16;
        lat.fill(rings, |x, y| owner[y * lat.nx + x] = idx);
    }
    let mut hit = vec![false; owner.len()];
    let lb = [
        lat.lon0,
        lat.lat0,
        lat.lon0 + lat.nx as f64 * cell,
        lat.lat0 + lat.ny as f64 * cell,
    ];
    for rings in karst {
        let b = crate::raster::bbox_of(rings);
        if b[2] < lb[0] || b[0] > lb[2] || b[3] < lb[1] || b[1] > lb[3] {
            continue;
        }
        lat.fill(rings, |x, y| hit[y * lat.nx + x] = true);
    }
    let mut total = vec![0u64; county_rings.len()];
    let mut karst_n = vec![0u64; county_rings.len()];
    for (k, o) in owner.iter().enumerate() {
        if *o != u16::MAX {
            total[*o as usize] += 1;
            if hit[k] {
                karst_n[*o as usize] += 1;
            }
        }
    }
    total
        .iter()
        .zip(&karst_n)
        .map(|(t, k)| (*t > 0).then(|| *k as f64 / *t as f64))
        .collect()
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let cw = Crosswalk::load(&ctx.data)?;

    // --- Landslide susceptibility (county table) -------------------------------------------
    let ls = ctx
        .http
        .get(LANDSLIDE_URL, Some("ground/county_analysis.csv"))?;
    out.source(super::source_from(
        "USGS Slope-Relief Threshold Landslide Susceptibility Models for the US and Puerto Rico: county_analysis.csv",
        &ls,
        "doi:10.5066/P13KAGU3 (ScienceBase item 65ccea5bd34ef4b119cb3bac, 2024-08-22)",
        "Creative Commons Zero v1.0 Universal (public domain dedication)",
        "",
    ));
    let geo_names = {
        let (h, rows) = crate::csvout::read_table(&ctx.data, super::geography::COUNTIES)?;
        let (i_f, i_nf, i_st) = (
            col(&h, "fips")?,
            col(&h, "name_full")?,
            col(&h, "state_abbr")?,
        );
        rows.iter()
            .map(|r| ((r[i_st].clone(), norm_county(&r[i_nf])), r[i_f].clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let (landslide, unmatched) = landslide_shares(&ls.text(), &geo_names, &cw)?;
    if !unmatched.is_empty() {
        return Err(data_err(format!(
            "landslide table rows with no county: {} (add them to LANDSLIDE_OVERRIDES)",
            unmatched.join("; ")
        )));
    }
    out.rows_in += landslide.len() as u64;

    // --- Karst (polygons, measured in each region's equal-area plane) -----------------------
    let (geo, cb_source) = CountyGeo::download(ctx, &canon, "ground/cb_2024_us_county_500k.zip")?;
    out.source(cb_source);
    let zip_path = ctx.http.raw_dir.join("ground").join("USKarstMap.zip");
    let streamed = ctx.http.download_to(KARST_URL, &zip_path)?;
    out.source(crate::manifest::SourceRecord {
        name:
            "USGS Karst in the United States: digital map compilation (OFR 2014-1156), shapefiles"
                .into(),
        url: streamed.final_url.clone(),
        version: "Open-File Report 2014-1156 (Weary and Doctor, 2014), USKarstMap.zip".into(),
        retrieved: streamed.retrieved.clone(),
        sha256: streamed.sha256.clone(),
        bytes: streamed.bytes,
        license: super::PUBLIC_DOMAIN.into(),
        obligations: String::new(),
    });
    let result = (|| -> Result<(BTreeMap<String, f64>, Vec<String>)> {
        let mut karst: BTreeMap<String, f64> = BTreeMap::new();
        let mut notes = Vec::new();
        for region in REGIONS {
            let (lat0, lon0, lat1, lat2) = region.albers;
            let proj = Albers::grs80(lat0, lon0, lat1, lat2);
            let mut layer_polys = Vec::new();
            let mut summary = Vec::new();
            for layer in region.layers {
                let shp = zip_entry_file(&zip_path, layer)?;
                let dbf = zip_entry_file(&zip_path, &layer.replace(".shp", ".dbf"))?;
                let (polys, kept, dropped) = karst_layer(&shp, &dbf)?;
                summary.push(format!(
                    "{layer}: {kept} polygons ({dropped} deeply buried left out)"
                ));
                layer_polys.extend(polys);
            }
            let members: Vec<&(String, Poly)> = geo
                .polys
                .iter()
                .filter(|(f, _)| (region.state_fips)(&f[..2]))
                .collect();
            let projected: Vec<Vec<Vec<[f64; 2]>>> = members
                .iter()
                .map(|(_, p)| {
                    p.rings
                        .iter()
                        .map(|r| {
                            r.iter()
                                .map(|q| {
                                    let (x, y) = proj.forward(q[1], q[0]);
                                    [x, y]
                                })
                                .collect()
                        })
                        .collect()
                })
                .collect();
            let shares = karst_shares(&projected, &layer_polys, KARST_CELL_M);
            let boxes: Vec<[f64; 4]> = layer_polys
                .iter()
                .map(|r| crate::raster::bbox_of(r))
                .collect();
            let in_karst = |x: f64, y: f64| {
                layer_polys.iter().zip(&boxes).any(|(rings, b)| {
                    x >= b[0]
                        && x <= b[2]
                        && y >= b[1]
                        && y <= b[3]
                        && rings
                            .iter()
                            .filter(|r| crate::geo::point_in_ring(x, y, r))
                            .count()
                            % 2
                            == 1
                })
            };
            let mut tiny = Vec::new();
            for (i, (fips, _)) in members.iter().enumerate() {
                let share = match shares.get(i).copied().flatten() {
                    Some(s) => s,
                    None => {
                        // Too small for a cell centre: test the county's internal point.
                        let c = counties.iter().find(|c| &c.fips == fips);
                        let inside = c.is_some_and(|c| {
                            let (x, y) = proj.forward(c.lat, c.lon);
                            in_karst(x, y)
                        });
                        tiny.push(fips.clone());
                        if inside { 1.0 } else { 0.0 }
                    }
                };
                karst.insert(fips.clone(), share);
            }
            notes.push(format!(
                "{}: {}; {} counties{}.",
                region.name,
                summary.join(", "),
                members.len(),
                if tiny.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; too small for a 1 km cell, judged by their internal point: {}",
                        tiny.join(", ")
                    )
                }
            ));
        }
        Ok((karst, notes))
    })();
    if !ctx.http.keep_raw {
        crate::http::remove_raw(&zip_path)?;
    }
    let (karst, karst_notes) = result?;

    // Sanity anchors: central Florida's covered karst, Kentucky's Pennyroyal, the Florida
    // lowlands and Puerto Rico's mountains for landslides.
    for (_fips, what, lo, hi, v) in [
        (
            "12083",
            "Marion County, FL karst",
            0.8,
            1.0,
            karst.get("12083"),
        ),
        (
            "21227",
            "Warren County, KY karst",
            0.5,
            1.0,
            karst.get("21227"),
        ),
        ("42101", "Philadelphia karst", 0.0, 0.1, karst.get("42101")),
        (
            "72141",
            "Utuado, PR landslide",
            0.8,
            1.0,
            landslide.get("72141"),
        ),
        (
            "12086",
            "Miami-Dade landslide",
            0.0,
            0.05,
            landslide.get("12086"),
        ),
    ] {
        match v {
            Some(v) if (lo..=hi).contains(v) => {}
            other => {
                return Err(data_err(format!(
                    "{what} is {other:?}, expected {lo}-{hi}: check the layers or the matching"
                )));
            }
        }
    }

    let mut table = Table::new(&["fips", "karst_share", "landslide_susceptible_share"], 1);
    let mut covered = BTreeSet::new();
    for c in &counties {
        let (k, l) = (karst.get(&c.fips), landslide.get(&c.fips));
        if k.is_none() && l.is_none() {
            continue;
        }
        // Three significant figures: the source maps are national-scale compilations.
        table.push(vec![
            c.fips.clone(),
            k.map(|v| sig(*v, 3)).unwrap_or_default(),
            l.map(|v| sig(*v, 3)).unwrap_or_default(),
        ]);
        covered.insert(c.fips.clone());
    }
    out.table(ctx, GROUND, &mut table)?;
    out.missing = missing_groups(&counties, &covered, |c| {
        if matches!(c.state_abbr.as_str(), "GU" | "AS" | "MP") {
            "Neither USGS source covers Guam, American Samoa or the Northern Mariana Islands".into()
        } else {
            "No karst or landslide value".into()
        }
    });
    let no_landslide: Vec<&str> = counties
        .iter()
        .filter(|c| karst.contains_key(&c.fips) && !landslide.contains_key(&c.fips))
        .map(|c| c.fips.as_str())
        .collect();
    out.notes.push("karst_share: share of the county's land on carbonate or evaporite rock at or near the surface, or buried under insoluble or up to 50 feet of glacial sediment (USGS OFR 2014-1156 exposure classes E, B1 and B3); rock under more than 50 feet of glacial sediment (B2) is left out. Measured on a 1 km lattice in each region's Albers equal-area plane (the layers' own projection), with county outlines from the Census 1:500,000 file. Karst is where sinkholes can form, not how often they do.".into());
    out.notes.extend(karst_notes);
    out.notes.push(format!(
        "landslide_susceptible_share: USGS 2024 slope-relief threshold model ({LANDSLIDE_ITEM}), county table (prop_susc: susceptible area / county area), matched by state and name; renamed or split counties by an override table, Connecticut's retired counties converted to planning regions by land share. No landslide value (the model covers the 50 states and Puerto Rico): {}.",
        if no_landslide.is_empty() { "none".to_string() } else { no_landslide.join(", ") }
    ));
    out.definitions.insert("karst_share".into(), "Share of the county's land on karst-prone rock (carbonates and evaporites at or near the surface or under thin cover), USGS OFR 2014-1156.".into());
    out.definitions.insert("landslide_susceptible_share".into(), "Share of the county's area rated susceptible by the USGS 2024 slope-relief threshold landslide susceptibility model.".into());
    out.attributions.push(Attribution {
        source: "USGS karst and landslide maps".into(),
        text: "Karst: Weary, D.J., and Doctor, D.H., 2014, Karst in the United States: A digital map compilation and database, USGS Open-File Report 2014-1156. Landslides: USGS, Slope-Relief Threshold Landslide Susceptibility Models for the United States and Puerto Rico, 2024, doi:10.5066/P13KAGU3 (CC0).".into(),
        license: "US Government works (public domain); CC0".into(),
        url: KARST_PAGE.into(),
        version: None,
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Vec<[f64; 2]>> {
        vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]]
    }

    #[test]
    fn names_normalise_for_matching() {
        assert_eq!(norm_county("Doña Ana County"), "dona ana county");
        assert_eq!(norm_county("Saint Louis city"), "st louis city");
        assert_eq!(norm_county("St. Louis City"), "st louis city");
        assert_eq!(
            norm_county("Lewis and Clark County"),
            "lewis and clark county"
        );
    }

    #[test]
    fn karst_share_is_the_covered_area() {
        // Two 10 x 10 km counties; karst over the east half of the first and a sliver of the second.
        let counties = vec![
            rect(0.0, 0.0, 10_000.0, 10_000.0),
            rect(10_000.0, 0.0, 20_000.0, 10_000.0),
        ];
        let karst = vec![
            rect(5_000.0, 0.0, 12_000.0, 10_000.0),
            rect(6_000.0, 2_000.0, 7_000.0, 3_000.0),
        ];
        let s = karst_shares(&counties, &karst, 1000.0);
        assert!((s[0].unwrap() - 0.5).abs() < 1e-9, "{s:?}");
        assert!((s[1].unwrap() - 0.2).abs() < 1e-9, "{s:?}");
        // A county smaller than a cell centre gets None (the caller tests its point).
        let s = karst_shares(&[rect(100.0, 100.0, 200.0, 200.0)], &karst, 1000.0);
        assert_eq!(s, vec![None]);
    }

    #[test]
    fn landslide_rows_match_counties_overrides_and_connecticut() {
        let mut names = BTreeMap::new();
        names.insert(
            ("AL".to_string(), norm_county("Autauga County")),
            "01001".to_string(),
        );
        names.insert(
            ("LA".to_string(), norm_county("LaSalle Parish")),
            "22059".to_string(),
        );
        let towns: Vec<(String, String, f64)> = vec![
            ("09001".into(), "09120".into(), 30.0),
            ("09001".into(), "09190".into(), 70.0),
            ("09003".into(), "09110".into(), 50.0),
            ("09005".into(), "09160".into(), 50.0),
            ("09007".into(), "09130".into(), 50.0),
            ("09009".into(), "09170".into(), 50.0),
            ("09011".into(), "09180".into(), 50.0),
            ("09013".into(), "09150".into(), 50.0),
            ("09015".into(), "09140".into(), 50.0),
        ];
        let cw = Crosswalk::from_towns(&towns).unwrap();
        let text = "COUNTY,STATE,ST,county_area,susc_area,prop_susc,v2_ls_count,v2_density\n\
Autauga County,Alabama,AL,1511.6,921.6,0.6097,0,0\n\
La Salle Parish,Louisiana,LA,1591.7,450.0,0.2827,0,0\n\
Fairfield County,Connecticut,CT,1673.8,1143.1,0.6829,3,0.0026\n\
Bedford city,Virginia,VA,17.9,14.4,0.8036,0,0\n\
Nowhere County,Alabama,AL,1,1,0.5,0,0\n";
        let (s, unmatched) = landslide_shares(text, &names, &cw).unwrap();
        assert!((s["01001"] - 0.6097).abs() < 1e-9);
        assert!((s["22059"] - 0.2827).abs() < 1e-9);
        assert!((s["09120"] - 0.6829).abs() < 1e-9 && (s["09190"] - 0.6829).abs() < 1e-9);
        assert!(!s.contains_key("09110"));
        assert_eq!(unmatched, vec!["Nowhere County, AL".to_string()]);
    }
}
