//! Job 5 — earthquake shaking at each county's internal point (USGS, public domain).
//!
//! - **Hazard curves** for peak ground acceleration (PGA) on firm rock (site class BC,
//!   Vs30 = 760 m/s), read at 0.1 g and 0.2 g by log-log interpolation along the curve and
//!   converted from annual rate of exceedance to yearly probability, p = 1 - exp(-rate):
//!   - contiguous US and Alaska: the gridded 2023 NSHM data release, with bilinear interpolation
//!     of the log rate between the four grid points around the county's internal point;
//!   - Hawaii: the gridded 2021.R2 Hawaii release, the same way;
//!   - Puerto Rico and the US Virgin Islands (no grid published): the USGS NSHM web service,
//!     one small PGA-only call per county, one at a time.
//!
//!   The web service covers every region, but on a full run of about 3,200 calls (2026-09-25)
//!   it rate-limited (HTTP 429) and then answered with errors, so the grids are the primary
//!   source: three downloads, deterministic, and no load on the service. The 2026 revision of
//!   the contiguous-US grid (2023.R2) is offered only through ScienceBase's new file manager,
//!   which has no direct download link, so the grid is the original 2023 release.
//! - **MMI VI in 100 years** (2023 model, variable Vs30, so soil amplification is included): the
//!   nearest 0.05-degree grid point.
//!
//! Guam, the Northern Mariana Islands and American Samoa have no USGS model.

use super::{County, Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::Table;
use crate::http::{Sha256Acc, zip_entry};
use crate::manifest::{Attribution, SourceRecord};
use crate::num::sig4;
use crate::shp::{Shape, read_dbf, read_shp};
use crate::{Result, data_err};
use rr_types::math::{exp, ln};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;
use std::path::Path;

/// Seismic pack file.
pub const SEISMIC: &str = "core/seismic.csv";

const SERVICE: &str = "https://earthquake.usgs.gov/ws/nshmp";
const MMI_ZIP: &str = "https://www.sciencebase.gov/catalog/file/get/64ff8ca8d34ed30c2057b506?f=__disk__8c%2F1c%2F88%2F8c1c88ac2d28ca887cf1630e797c13398c653796";
const MMI_ITEM: &str = "https://www.sciencebase.gov/catalog/item/64ff8ca8d34ed30c2057b506";
const NSHM_DOI: &str = "https://doi.org/10.5066/P9GNPCOD";

/// Reference site condition (m/s): site class BC boundary, the USGS map default.
pub const VS30: u32 = 760;

/// A gridded hazard-curve archive.
struct GridSource {
    /// Label written to the `model` column.
    model: &'static str,
    /// States it serves (empty: the contiguous US).
    states: &'static [&'static str],
    /// Direct download URL.
    url: &'static str,
    /// Temporary file name under `data/raw/seismic/`.
    file: &'static str,
    /// Version label for the manifest.
    version: &'static str,
}

const GRIDS: &[GridSource] = &[
    GridSource {
        model: "conus-2023-grid",
        states: &[],
        url: "https://www.sciencebase.gov/catalog/file/get/64ff82d6d34ed30c2057b4be?f=__disk__3c%2F17%2F24%2F3c172403609fcdae537a03656de417e56c916255",
        file: "hazard_output_CONUS.zip",
        version: "2023 50-state NSHM data release, doi:10.5066/P9GNPCOD (2023-12-21), hazard_output_CONUS.zip: Vs30 760 PGA curves.csv",
    },
    GridSource {
        model: "alaska-2023-grid",
        states: &["AK"],
        url: "https://www.sciencebase.gov/catalog/file/get/64ff82d6d34ed30c2057b4be?f=__disk__68%2F70%2F7c%2F68707c0259e96cc72c2a7fd4101c44f59c75cb1f",
        file: "hazard_output_AK.zip",
        version: "2023 50-state NSHM data release, doi:10.5066/P9GNPCOD (2023-12-21), hazard_output_AK.zip: Vs30 760 PGA curves.csv",
    },
    GridSource {
        model: "hawaii-2021.R2-grid",
        states: &["HI"],
        url: "https://www.sciencebase.gov/catalog/file/get/6802a4fad4be0210cdcc996b?f=__disk__76%2Fc6%2F1c%2F76c61c80911adedb5e93ab638e325248bd46802f",
        file: "hawaii.2021.R2-MPRS-scBC-vs760-0p02.zip",
        version: "Revised 2023 50-state NSHM data release, doi:10.5066/P14VGAV4: Hawaii 2021.R2, hawaii.2021.R2-MPRS-scBC-vs760-0p02.zip: PGA curves.csv",
    },
];

/// Longitude in the convention the grids use after loading (west of the antimeridian is below
/// -180, so the Aleutians stay next to the rest of Alaska).
fn west_lon(lon: f64) -> f64 {
    if lon > 0.0 { lon - 360.0 } else { lon }
}

/// Log-log interpolation of a hazard curve (`xs` ground motion in g, `ys` annual rate of
/// exceedance) at `x`. Returns `None` outside the curve's range.
pub fn interp_loglog(xs: &[f64], ys: &[f64], x: f64) -> Option<f64> {
    if xs.len() != ys.len() || xs.len() < 2 || x < xs[0] || x > xs[xs.len() - 1] {
        return None;
    }
    let i = xs.windows(2).position(|w| x >= w[0] && x <= w[1])?;
    let (x0, x1, y0, y1) = (xs[i], xs[i + 1], ys[i], ys[i + 1]);
    if y0 <= 0.0 || y1 <= 0.0 {
        // Linear in x where the curve reaches zero.
        return Some(y0 + (x - x0) * (y1 - y0) / (x1 - x0));
    }
    let t = (ln(x) - ln(x0)) / (ln(x1) - ln(x0));
    Some(exp(ln(y0) + t * (ln(y1) - ln(y0))))
}

/// The most common gap between consecutive distinct values (in 1e-4 degree units).
fn grid_step(values: &mut Vec<i64>) -> Option<f64> {
    values.sort_unstable();
    values.dedup();
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for w in values.windows(2) {
        *counts.entry(w[1] - w[0]).or_default() += 1;
    }
    let (gap, _) = counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))?;
    Some(gap as f64 / 1e4)
}

/// Annual exceedance rates at 0.1 g and 0.2 g at the nodes of a regular grid.
pub struct RateGrid {
    /// Grid spacing in degrees (longitude, latitude).
    pub step: (f64, f64),
    /// Coordinates of grid index (0, 0).
    origin: (f64, f64),
    nodes: HashMap<(i64, i64), (f64, f64)>,
}

impl RateGrid {
    /// Parse a `curves.csv`: a longitude and a latitude column (`lon`/`longitude`,
    /// `lat`/`latitude`), optional label columns, and one column per ground-motion level (g).
    pub fn parse(text: &str) -> Result<Self> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(text.as_bytes());
        let h: Vec<String> = rdr
            .headers()?
            .iter()
            .map(|s| s.trim().to_ascii_lowercase())
            .collect();
        let ilon = h
            .iter()
            .position(|c| c == "lon" || c == "longitude")
            .ok_or_else(|| data_err("hazard grid: no longitude column"))?;
        let ilat = h
            .iter()
            .position(|c| c == "lat" || c == "latitude")
            .ok_or_else(|| data_err("hazard grid: no latitude column"))?;
        let levels: Vec<(usize, f64)> = h
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != ilon && *i != ilat)
            .filter_map(|(i, c)| Some((i, c.parse::<f64>().ok()?)))
            .collect();
        if levels.len() < 5 {
            return Err(data_err(
                "hazard grid: too few ground-motion levels in the header",
            ));
        }
        let xs: Vec<f64> = levels.iter().map(|(_, x)| *x).collect();
        let num = |rec: &csv::StringRecord, i: usize| {
            rec.get(i).and_then(|s| s.trim().parse::<f64>().ok())
        };
        let mut pts: Vec<(f64, f64, f64, f64)> = Vec::new();
        for rec in rdr.records() {
            let rec = rec?;
            let (Some(lon), Some(lat)) = (num(&rec, ilon), num(&rec, ilat)) else {
                continue;
            };
            let ys: Vec<f64> = levels
                .iter()
                .map(|(i, _)| num(&rec, *i).unwrap_or(0.0))
                .collect();
            if let (Some(r1), Some(r2)) =
                (interp_loglog(&xs, &ys, 0.1), interp_loglog(&xs, &ys, 0.2))
            {
                pts.push((west_lon(lon), lat, r1, r2));
            }
        }
        let mut lons: Vec<i64> = pts.iter().map(|p| (p.0 * 1e4).round() as i64).collect();
        let mut lats: Vec<i64> = pts.iter().map(|p| (p.1 * 1e4).round() as i64).collect();
        let (sx, sy) = (grid_step(&mut lons), grid_step(&mut lats));
        let (sx, sy) = match (sx.or(sy), sy.or(sx)) {
            (Some(x), Some(y)) => (x, y),
            _ => return Err(data_err("hazard grid: cannot find the grid spacing")),
        };
        let origin = (lons[0] as f64 / 1e4, lats[0] as f64 / 1e4);
        let nodes = pts
            .into_iter()
            .map(|(lon, lat, r1, r2)| {
                (
                    (
                        ((lon - origin.0) / sx).round() as i64,
                        ((lat - origin.1) / sy).round() as i64,
                    ),
                    (r1, r2),
                )
            })
            .collect();
        Ok(Self {
            step: (sx, sy),
            origin,
            nodes,
        })
    }

    /// Number of grid nodes with a usable curve.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True when the grid has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Rates at a point: bilinear interpolation of the log rates of the four surrounding nodes,
    /// renormalised over the nodes that exist; the nearest node within two steps when none of
    /// the four exists; `None` beyond that.
    pub fn at(&self, lon: f64, lat: f64) -> Option<(f64, f64)> {
        let fx = (west_lon(lon) - self.origin.0) / self.step.0;
        let fy = (lat - self.origin.1) / self.step.1;
        let (x0, y0) = (fx.floor() as i64, fy.floor() as i64);
        let (tx, ty) = (fx - x0 as f64, fy - y0 as f64);
        let corners = [
            (0, 0, (1.0 - tx) * (1.0 - ty)),
            (1, 0, tx * (1.0 - ty)),
            (0, 1, (1.0 - tx) * ty),
            (1, 1, tx * ty),
        ];
        let (mut sum, mut wsum, mut any) = ([0.0f64; 2], [0.0f64; 2], false);
        for (dx, dy, w) in corners {
            if w <= 0.0 {
                continue;
            }
            let Some(&(r1, r2)) = self.nodes.get(&(x0 + dx, y0 + dy)) else {
                continue;
            };
            any = true;
            for (k, r) in [r1, r2].into_iter().enumerate() {
                if r > 0.0 {
                    sum[k] += w * ln(r);
                    wsum[k] += w;
                }
            }
        }
        if any {
            let v = |k: usize| {
                if wsum[k] > 0.0 {
                    exp(sum[k] / wsum[k])
                } else {
                    0.0
                }
            };
            return Some((v(0), v(1)));
        }
        let (cx, cy) = (fx.round() as i64, fy.round() as i64);
        let mut best: Option<(i64, (f64, f64))> = None;
        for dx in -2..=2i64 {
            for dy in -2..=2i64 {
                if let Some(v) = self.nodes.get(&(cx + dx, cy + dy)) {
                    let d = dx * dx + dy * dy;
                    if best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, *v));
                    }
                }
            }
        }
        best.map(|(_, v)| v)
    }
}

/// Read the Vs30 760 PGA `curves.csv` out of a gridded-release ZIP on disk.
fn read_curves_entry(path: &Path) -> Result<String> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let mut found = None;
    for i in 0..archive.len() {
        let name = archive.by_index(i)?.name().to_ascii_lowercase();
        if name.ends_with("pga/curves.csv") && name.contains("760") {
            found = Some(i);
            break;
        }
    }
    let i = found.ok_or_else(|| {
        data_err(format!(
            "{}: no Vs30 760 PGA curves.csv entry",
            path.display()
        ))
    })?;
    let mut s = String::new();
    archive.by_index(i)?.read_to_string(&mut s)?;
    Ok(s)
}

/// Pull the total PGA curve out of a web-service response.
fn pga_curve(v: &serde_json::Value) -> Option<(Vec<f64>, Vec<f64>)> {
    let curves = v["response"]["hazardCurves"].as_array()?;
    let pga = curves
        .iter()
        .find(|c| c["imt"]["value"].as_str() == Some("PGA"))?;
    let total = pga["data"]
        .as_array()?
        .iter()
        .find(|d| d["component"].as_str() == Some("Total"))?;
    let xs: Vec<f64> = total["values"]["xs"]
        .as_array()?
        .iter()
        .filter_map(|x| x.as_f64())
        .collect();
    let ys: Vec<f64> = total["values"]["ys"]
        .as_array()?
        .iter()
        .filter_map(|x| x.as_f64())
        .collect();
    Some((xs, ys))
}

/// The model repository version a service response reports (e.g. "nshm-prvi 2.0.0").
fn model_version(v: &serde_json::Value) -> Option<String> {
    v["metadata"]["repositories"]
        .as_array()?
        .iter()
        .find(|r| {
            r["projectName"]
                .as_str()
                .is_some_and(|n| n.starts_with("nshm-"))
        })
        .and_then(|r| {
            Some(format!(
                "{} {}",
                r["projectName"].as_str()?,
                r["version"].as_str()?
            ))
        })
}

/// Grid points (lon, lat, probability) bucketed by quarter-degree cell.
type Cells = HashMap<(i32, i32), Vec<(f64, f64, f64)>>;

struct MmiGrid {
    cells: Cells,
}

impl MmiGrid {
    fn key(lon: f64, lat: f64) -> (i32, i32) {
        ((lon * 4.0).floor() as i32, (lat * 4.0).floor() as i32)
    }
    fn nearest(&self, lon: f64, lat: f64, max_km: f64) -> Option<f64> {
        let (kx, ky) = Self::key(lon, lat);
        let mut best: Option<(f64, f64)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for (x, y, v) in self.cells.get(&(kx + dx, ky + dy)).into_iter().flatten() {
                    let d = crate::geo::haversine_km(lat, lon, *y, *x);
                    if d <= max_km && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, *v));
                    }
                }
            }
        }
        best.map(|(_, v)| v)
    }
}

fn serves(g: &GridSource, c: &County) -> bool {
    if g.states.is_empty() {
        !super::is_outside_conus(&c.state_abbr)
    } else {
        g.states.contains(&c.state_abbr.as_str())
    }
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;

    // --- MMI VI in 100 years ------------------------------------------------------------------
    let mmi = ctx
        .http
        .get(MMI_ZIP, Some("seismic/US_ProbMMI_VI_100Yrs_varVs30.zip"))?;
    out.source(super::source_from(
        "USGS chance of damaging shaking (MMI VI) in 100 years, 2023 NSHM, variable Vs30",
        &mmi,
        "Data release doi:10.5066/P9GNPCOD (2023-12-21), child item 06",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let accessed = mmi.retrieved[..10].to_string();
    let shp = read_shp(&zip_entry(&mmi.bytes, "_pts.shp")?)?;
    let dbf = read_dbf(&zip_entry(&mmi.bytes, "_pts.dbf")?)?;
    let iv = dbf.field("PctProb100")?;
    let mut mmi_grid = MmiGrid {
        cells: HashMap::new(),
    };
    for (s, r) in shp.iter().zip(&dbf.records) {
        if let (Shape::Point([x, y]), Ok(v)) = (s, r[iv].parse::<f64>()) {
            let x = west_lon(*x);
            mmi_grid
                .cells
                .entry(MmiGrid::key(x, *y))
                .or_default()
                .push((x, *y, v / 100.0));
        }
    }
    out.rows_in += shp.len() as u64;
    drop((shp, dbf, mmi));

    // --- Gridded hazard curves: contiguous US, Alaska, Hawaii -----------------------------------
    let mut rates: BTreeMap<String, (f64, f64, String)> = BTreeMap::new();
    let raw_dir = ctx.http.raw_dir.join("seismic");
    for g in GRIDS {
        let path = raw_dir.join(g.file);
        eprintln!("  downloading {}", g.file);
        let streamed = ctx.http.download_to(g.url, &path)?;
        let text = read_curves_entry(&path);
        if !ctx.http.keep_raw {
            crate::http::remove_raw(&path)?;
        }
        let grid = RateGrid::parse(&text?)?;
        eprintln!(
            "  {}: {} nodes, step {} x {} degrees",
            g.model,
            grid.len(),
            grid.step.0,
            grid.step.1
        );
        out.rows_in += grid.len() as u64;
        out.source(SourceRecord {
            name: format!("USGS NSHM gridded hazard curves ({})", g.model),
            url: streamed.final_url.clone(),
            version: format!(
                "{}; grid step {} x {} degrees",
                g.version, grid.step.0, grid.step.1
            ),
            retrieved: streamed.retrieved.clone(),
            sha256: streamed.sha256.clone(),
            bytes: streamed.bytes,
            license: super::PUBLIC_DOMAIN.into(),
            obligations: String::new(),
        });
        for c in counties.iter().filter(|c| serves(g, c)) {
            if let Some((r1, r2)) = grid.at(c.lon, c.lat) {
                rates.insert(c.fips.clone(), (r1, r2, g.model.to_string()));
            }
        }
    }

    // --- Web service: Puerto Rico and the US Virgin Islands -------------------------------------
    let started = crate::timefmt::now_utc();
    let mut acc = Sha256Acc::new();
    let mut calls = 0u64;
    let mut versions: BTreeSet<String> = BTreeSet::new();
    let mut service_failed: BTreeSet<String> = BTreeSet::new();
    let mut first_error: Option<String> = None;
    for c in counties
        .iter()
        .filter(|c| c.state_abbr == "PR" || c.state_abbr == "VI")
    {
        let url = format!(
            "{SERVICE}/prvi-2025/dynamic/hazard/{:.4}/{:.4}/{VS30}?imt=PGA",
            c.lon, c.lat
        );
        calls += 1;
        let answer = ctx.http.get(&url, None).map(|f| {
            acc.update(f.sha256.as_bytes());
            let v: Option<serde_json::Value> = serde_json::from_slice(&f.bytes).ok();
            let model = f
                .final_url
                .split("/nshmp/")
                .nth(1)
                .and_then(|s| s.split('/').next())
                .unwrap_or("prvi-2025")
                .to_string();
            let rates = v.as_ref().and_then(pga_curve).and_then(|(xs, ys)| {
                Some((interp_loglog(&xs, &ys, 0.1)?, interp_loglog(&xs, &ys, 0.2)?))
            });
            (model, v.as_ref().and_then(model_version), rates)
        });
        match answer {
            Ok((model, version, Some((r1, r2)))) => {
                versions.insert(match version {
                    Some(ver) => format!("{model} ({ver})"),
                    None => model.clone(),
                });
                rates.insert(c.fips.clone(), (r1, r2, model));
            }
            Ok((_, _, None)) => {
                first_error
                    .get_or_insert_with(|| format!("{}: no PGA curve in the answer", c.fips));
                service_failed.insert(c.fips.clone());
            }
            Err(e) => {
                first_error.get_or_insert_with(|| format!("{}: {e}", c.fips));
                service_failed.insert(c.fips.clone());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    out.rows_in += calls;
    out.source(SourceRecord {
        name: "USGS NSHM hazard curve web service, Puerto Rico and US Virgin Islands (PGA, Vs30 760 m/s), one call per county internal point".into(),
        url: format!("{SERVICE}/prvi-2025/dynamic/hazard/{{lon}}/{{lat}}/{VS30}?imt=PGA"),
        version: versions.into_iter().collect::<Vec<_>>().join("; "),
        retrieved: started,
        sha256: acc.finish(),
        bytes: calls,
        license: super::PUBLIC_DOMAIN.into(),
        obligations: "sha256 is taken over the per-response sha256 digests in county order; bytes counts responses.".into(),
    });

    // --- Output ---------------------------------------------------------------------------------
    let mut table = Table::new(
        &[
            "fips",
            "p_pga_ge_0_1g_per_year",
            "p_pga_ge_0_2g_per_year",
            "mmi6_100yr",
            "model",
        ],
        1,
    );
    let p = |rate: f64| -rr_types::math::exp_m1(-rate.max(0.0));
    let mut covered = BTreeSet::new();
    for c in &counties {
        let Some((r1, r2, model)) = rates.get(&c.fips) else {
            continue;
        };
        let mmi6 = mmi_grid.nearest(west_lon(c.lon), c.lat, 8.0);
        table.push(vec![
            c.fips.clone(),
            sig4(p(*r1)),
            sig4(p(*r2)),
            mmi6.map(sig4).unwrap_or_default(),
            model.clone(),
        ]);
        covered.insert(c.fips.clone());
    }
    out.table(ctx, SEISMIC, &mut table)?;
    out.missing = missing_groups(&counties, &covered, |c| {
        if matches!(c.state_abbr.as_str(), "GU" | "MP" | "AS" | "UM") {
            "USGS publishes no seismic hazard model for this island area".to_string()
        } else if service_failed.contains(&c.fips) {
            "The USGS hazard web service did not return a curve for this point (Puerto Rico and the US Virgin Islands have no published grid)".to_string()
        } else {
            "The county's internal point is outside the USGS hazard grid".to_string()
        }
    });
    if let Some(e) = &first_error {
        out.notes.push(format!(
            "The USGS web service failed for {} Puerto Rico / US Virgin Islands counties, e.g. {e}.",
            service_failed.len()
        ));
    }
    out.notes.push(format!(
        "Annual probability of peak ground acceleration of at least 0.1 g and 0.2 g on firm rock (Vs30 {VS30} m/s, site class BC) at the county's internal point. Each grid node's hazard curve is read at 0.1 g and 0.2 g by log-log interpolation; the county value is the bilinear interpolation of the log rate between the four surrounding nodes; p = 1 - exp(-annual rate). Softer soils shake more: mmi6_100yr (USGS, variable Vs30) includes that. mmi6_100yr is the chance of at least MMI VI shaking in 100 years at the nearest 0.05-degree grid point within 8 km (contiguous US, Alaska, Hawaii). Yearly equivalent: 1 - (1 - p100)^(1/100). MMI product page: {MMI_ITEM}."
    ));
    out.notes.push("Source choice: during a full run of about 3,200 calls on 2026-09-25 the USGS web service rate-limited (HTTP 429) and then answered with errors, so the gridded data releases are the primary source for the contiguous US, Alaska and Hawaii, and the service is used only for Puerto Rico and the US Virgin Islands (about 80 small calls, one at a time), which have no published grid. The contiguous-US and Alaska grids are the original 2023 release: the 2026 revision the service now runs (2023.R2) is offered only through ScienceBase's new file manager, which has no direct download link.".into());
    out.attributions.push(Attribution {
        source: "USGS National Seismic Hazard Model".into(),
        text: "Earthquake shaking probabilities from the U.S. Geological Survey National Seismic Hazard Model (2023 50-state model data release; Hawaii 2021.R2; Puerto Rico and U.S. Virgin Islands 2025).".into(),
        license: super::PUBLIC_DOMAIN.into(),
        url: NSHM_DOI.into(),
        version: Some("NSHM 2023 grids, Hawaii 2021.R2 grid, PRVI 2025 service".into()),
        accessed,
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loglog_interpolation() {
        let xs = [0.05, 0.1, 0.2, 0.4];
        let ys = [1e-2, 1e-3, 1e-4, 1e-5];
        assert!((interp_loglog(&xs, &ys, 0.1).unwrap() - 1e-3).abs() < 1e-12);
        // Halfway in log space between 0.1 and 0.2 lies ~0.1414 g -> ~3.162e-4
        let v = interp_loglog(&xs, &ys, 0.1f64 * 2f64.sqrt()).unwrap();
        assert!((v - 3.1623e-4).abs() < 1e-7, "{v}");
        assert!(interp_loglog(&xs, &ys, 0.01).is_none());
    }

    #[test]
    fn grid_parses_and_interpolates() {
        // Original-release layout (label column first), 0.2-degree grid; rates halve going east.
        let csv = "site,longitude,latitude,0.05,0.1,0.2,0.4,0.8\n\
                   a,-75.2,40.0,1e-2,1e-3,1e-4,1e-5,1e-6\n\
                   b,-75.0,40.0,5e-3,5e-4,5e-5,5e-6,5e-7\n\
                   c,-75.2,40.2,1e-2,1e-3,1e-4,1e-5,1e-6\n\
                   d,-75.0,40.2,5e-3,5e-4,5e-5,5e-6,5e-7\n";
        let g = RateGrid::parse(csv).unwrap();
        assert!((g.step.0 - 0.2).abs() < 1e-9 && (g.step.1 - 0.2).abs() < 1e-9);
        assert_eq!(g.len(), 4);
        let (r1, r2) = g.at(-75.2, 40.0).unwrap();
        assert!((r1 - 1e-3).abs() < 1e-12 && (r2 - 1e-4).abs() < 1e-13);
        // Midway in longitude: the geometric mean of 1e-3 and 5e-4.
        let (r1, _) = g.at(-75.1, 40.1).unwrap();
        assert!((r1 - (1e-3f64 * 5e-4).sqrt()).abs() < 1e-9, "{r1}");
        // A point just outside the grid falls back to the nearest node; far away gives nothing.
        assert!(g.at(-74.95, 40.0).is_some());
        assert!(g.at(-70.0, 40.0).is_none());
    }

    #[test]
    fn grid_handles_offsets_and_the_antimeridian() {
        // Revised-release layout (no label column), nodes offset from whole multiples of the
        // step, straddling 180 degrees as the Aleutians do.
        let csv = "lon,lat,0.05,0.1,0.2,0.4,0.8\n\
                   179.9,51.95,1e-1,1e-2,1e-3,1e-4,1e-5\n\
                   -179.9,51.95,1e-1,1e-2,1e-3,1e-4,1e-5\n\
                   179.9,52.15,1e-1,1e-2,1e-3,1e-4,1e-5\n\
                   -179.9,52.15,1e-1,1e-2,1e-3,1e-4,1e-5\n";
        let g = RateGrid::parse(csv).unwrap();
        assert!((g.step.0 - 0.2).abs() < 1e-9 && (g.step.1 - 0.2).abs() < 1e-9);
        let (r1, r2) = g.at(179.95, 52.0).unwrap();
        assert!((r1 - 1e-2).abs() < 1e-12 && (r2 - 1e-3).abs() < 1e-13);
        assert!(g.at(-180.0, 52.05).is_some());
    }

    #[test]
    fn model_version_is_read_from_the_repository_list() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"metadata":{"repositories":[{"projectName":"nshmp-haz","version":"2.11.4"},{"projectName":"nshm-prvi","version":"2.0.0"}]}}"#,
        )
        .unwrap();
        assert_eq!(model_version(&v).as_deref(), Some("nshm-prvi 2.0.0"));
    }
}
