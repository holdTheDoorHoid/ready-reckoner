//! Job 5 — earthquake shaking at each county's internal point (USGS, public domain).
//!
//! - USGS National Seismic Hazard Model web service (2023 50-state model, revised; Puerto Rico and
//!   US Virgin Islands 2025): the mean hazard curve for peak ground acceleration (PGA) on firm
//!   rock (site class BC, Vs30 = 760 m/s), read at 0.1 g and 0.2 g by log-log interpolation and
//!   converted from annual rate of exceedance to yearly probability, p = 1 - exp(-rate).
//! - USGS "Chance of potentially damaging ground shaking (MMI VI) in 100 years" (2023 model,
//!   variable Vs30, so soil amplification is included): the nearest 0.05-degree grid point.
//!
//! The service is called once per county (about 3,200 calls) by three workers at a time, each
//! request waiting for the previous one, so the load stays modest. Guam, the Northern Mariana
//! Islands and American Samoa have no model in the service.

use super::{County, Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::Table;
use crate::http::{Sha256Acc, zip_entry};
use crate::manifest::{Attribution, SourceRecord};
use crate::num::sig4;
use crate::shp::{Shape, read_dbf, read_shp};
use crate::{Result, data_err};
use rr_types::math::{exp, ln};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::Mutex;

/// Seismic pack file.
pub const SEISMIC: &str = "core/seismic.csv";

const SERVICE: &str = "https://earthquake.usgs.gov/ws/nshmp";
const MMI_ZIP: &str = "https://www.sciencebase.gov/catalog/file/get/64ff8ca8d34ed30c2057b506?f=__disk__8c%2F1c%2F88%2F8c1c88ac2d28ca887cf1630e797c13398c653796";
const MMI_ITEM: &str = "https://www.sciencebase.gov/catalog/item/64ff8ca8d34ed30c2057b506";
const NSHM_DOI: &str = "https://doi.org/10.5066/P14VGAV4";

/// Reference site condition (m/s): site class BC boundary, the USGS map default.
pub const VS30: u32 = 760;
/// Concurrent requests to the USGS service.
pub const WORKERS: usize = 3;

/// Which model serves a point: (model id, lon min, lon max, lat min, lat max).
const MODELS: &[(&str, f64, f64, f64, f64)] = &[
    ("conus-2023", -125.0, -65.0, 24.4, 50.0),
    ("alaska-2023", -190.0, -128.0, 49.5, 72.0),
    ("hawaii-2021", -160.5, -154.3, 18.6, 22.5),
    ("prvi-2025", -70.95, -61.7, 15.0, 21.2),
];

fn model_for(c: &County) -> Option<(&'static str, f64)> {
    let lon = if c.state_abbr == "AK" && c.lon > 0.0 {
        c.lon - 360.0
    } else {
        c.lon
    };
    let conus = !super::is_outside_conus(&c.state_abbr);
    for (id, x0, x1, y0, y1) in MODELS {
        let right_region = match *id {
            "conus-2023" => conus,
            "alaska-2023" => c.state_abbr == "AK",
            "hawaii-2021" => c.state_abbr == "HI",
            "prvi-2025" => c.state_abbr == "PR" || c.state_abbr == "VI",
            _ => false,
        };
        if right_region && lon >= *x0 && lon <= *x1 && c.lat >= *y0 && c.lat <= *y1 {
            return Some((id, lon));
        }
    }
    None
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

/// Pull the total PGA curve out of a service response.
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

/// Grid points (lon, lat, probability) bucketed by quarter-degree cell.
type Cells = HashMap<(i32, i32), Vec<(f64, f64, f64)>>;

struct Grid {
    cells: Cells,
}

impl Grid {
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
    let shp = read_shp(&zip_entry(&mmi.bytes, "_pts.shp")?)?;
    let dbf = read_dbf(&zip_entry(&mmi.bytes, "_pts.dbf")?)?;
    let iv = dbf.field("PctProb100")?;
    let mut grid = Grid {
        cells: HashMap::new(),
    };
    for (s, r) in shp.iter().zip(&dbf.records) {
        if let (Shape::Point([x, y]), Ok(v)) = (s, r[iv].parse::<f64>()) {
            let x = if *x > 0.0 { x - 360.0 } else { *x };
            grid.cells
                .entry(Grid::key(x, *y))
                .or_default()
                .push((x, *y, v / 100.0));
        }
    }
    out.rows_in += shp.len() as u64;

    // --- Hazard service ---------------------------------------------------------------------
    let mut todo: VecDeque<(usize, &'static str, f64)> = VecDeque::new();
    let mut no_model = BTreeSet::new();
    for (i, c) in counties.iter().enumerate() {
        match model_for(c) {
            Some((m, lon)) => todo.push_back((i, m, lon)),
            None => {
                no_model.insert(c.fips.clone());
            }
        }
    }
    let total = todo.len();
    eprintln!("  querying the USGS hazard service for {total} counties with {WORKERS} workers");
    let queue = Mutex::new(todo);
    // Each worker parses its response and keeps only what the pack needs, plus the response's
    // sha256 (the manifest hash is taken over these digests in county order).
    struct Answer {
        model: &'static str,
        rates: Option<(f64, f64)>,
        version: Option<String>,
        base_url: String,
        sha256: String,
    }
    let results: Mutex<BTreeMap<usize, Answer>> = Mutex::new(BTreeMap::new());
    let failures: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
    let started = crate::timefmt::now_utc();
    std::thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().ok().and_then(|mut q| q.pop_front());
                    let Some((i, model, lon)) = next else { break };
                    let c = &counties[i];
                    let url = format!(
                        "{SERVICE}/{model}/dynamic/hazard/{lon:.4}/{:.4}/{VS30}",
                        c.lat
                    );
                    match ctx.http.get(&url, None) {
                        Ok(f) => {
                            let v: Option<serde_json::Value> =
                                serde_json::from_slice(&f.bytes).ok();
                            let rates = v.as_ref().and_then(pga_curve).and_then(|(xs, ys)| {
                                Some((interp_loglog(&xs, &ys, 0.1)?, interp_loglog(&xs, &ys, 0.2)?))
                            });
                            let version = v.as_ref().and_then(|v| {
                                v["response"]["metadata"]["server"]["version"]
                                    .as_str()
                                    .map(|s| s.to_string())
                            });
                            let base_url = f
                                .final_url
                                .split("/dynamic")
                                .next()
                                .unwrap_or(&f.final_url)
                                .to_string();
                            if let Ok(mut r) = results.lock() {
                                r.insert(
                                    i,
                                    Answer {
                                        model,
                                        rates,
                                        version,
                                        base_url,
                                        sha256: f.sha256,
                                    },
                                );
                                let n = r.len();
                                if n % 250 == 0 {
                                    eprintln!("    {n}/{total}");
                                }
                            }
                        }
                        Err(e) => {
                            if let Ok(mut fl) = failures.lock() {
                                fl.push((c.fips.clone(), e.to_string()));
                            }
                        }
                    }
                }
            });
        }
    });
    let results = results
        .into_inner()
        .map_err(|_| data_err("seismic worker panicked"))?;
    let failures = failures
        .into_inner()
        .map_err(|_| data_err("seismic worker panicked"))?;
    let mut acc = Sha256Acc::new();
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
    let mut covered = BTreeSet::new();
    let mut versions: BTreeSet<String> = BTreeSet::new();
    for (i, a) in &results {
        let c = &counties[*i];
        acc.update(a.sha256.as_bytes());
        if let Some(ver) = &a.version {
            versions.insert(format!("{} (server {ver}; {})", a.model, a.base_url));
        }
        let Some((r1, r2)) = a.rates else { continue };
        let p = |rate: f64| -rr_types::math::exp_m1(-rate.max(0.0));
        let mmi6 = grid.nearest(if c.lon > 0.0 { c.lon - 360.0 } else { c.lon }, c.lat, 8.0);
        table.push(vec![
            c.fips.clone(),
            sig4(p(r1)),
            sig4(p(r2)),
            mmi6.map(sig4).unwrap_or_default(),
            a.model.to_string(),
        ]);
        covered.insert(c.fips.clone());
    }
    out.rows_in += results.len() as u64;
    out.source(SourceRecord {
        name: "USGS NSHM hazard curve web service (PGA, Vs30 760 m/s), one call per county internal point".into(),
        url: format!("{SERVICE}/{{model}}/dynamic/hazard/{{lon}}/{{lat}}/{VS30}"),
        version: versions.into_iter().collect::<Vec<_>>().join("; "),
        retrieved: started,
        sha256: acc.finish(),
        bytes: results.len() as u64,
        license: super::PUBLIC_DOMAIN.into(),
        obligations: "sha256 is taken over the per-response sha256 digests in county order; bytes counts responses.".into(),
    });
    out.table(ctx, SEISMIC, &mut table)?;

    let failed: BTreeMap<String, String> = failures.into_iter().collect();
    out.missing = missing_groups(&counties, &covered, |c| {
        if no_model.contains(&c.fips) {
            "The USGS hazard service has no model for this island area".to_string()
        } else if failed.contains_key(&c.fips) {
            "The USGS hazard service did not answer for this point".to_string()
        } else {
            "The USGS hazard curve for this point could not be read".to_string()
        }
    });
    if !failed.is_empty() {
        out.notes.push(format!(
            "Service errors (after retries) for {} counties, e.g. {:?}.",
            failed.len(),
            failed.iter().next()
        ));
    }
    out.notes.push(format!(
        "Annual probability of peak ground acceleration of at least 0.1 g and 0.2 g on firm rock (Vs30 {VS30} m/s, site class BC) at the county's internal point, from the USGS mean hazard curve by log-log interpolation; p = 1 - exp(-annual rate). Softer soils shake more: mmi6_100yr (USGS, variable Vs30) includes that. mmi6_100yr is the chance of at least MMI VI shaking in 100 years at the nearest 0.05-degree grid point within 8 km (contiguous US, Alaska, Hawaii only). Yearly equivalent: 1 - (1 - p100)^(1/100)."
    ));
    out.attributions.push(Attribution {
        source: "USGS National Seismic Hazard Model".into(),
        text: "Earthquake shaking probabilities from the U.S. Geological Survey National Seismic Hazard Model (2023 50-state model and revisions; Puerto Rico and U.S. Virgin Islands 2025).".into(),
        license: super::PUBLIC_DOMAIN.into(),
        url: NSHM_DOI.into(),
        version: Some("2023 (revised 2026), PRVI 2025".into()),
        accessed: mmi.retrieved[..10].to_string(),
    });
    out.notes.push(format!("MMI VI product page: {MMI_ITEM}."));
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
    fn models_route_by_region() {
        let c = |st: &str, lat: f64, lon: f64| County {
            fips: "x".into(),
            name: "x".into(),
            state_abbr: st.into(),
            lat,
            lon,
        };
        assert_eq!(model_for(&c("PA", 40.0, -75.1)).unwrap().0, "conus-2023");
        assert_eq!(
            model_for(&c("AK", 52.0, 174.0)).unwrap(),
            ("alaska-2023", 174.0 - 360.0)
        );
        assert_eq!(model_for(&c("HI", 21.3, -157.8)).unwrap().0, "hawaii-2021");
        assert_eq!(model_for(&c("PR", 18.4, -66.1)).unwrap().0, "prvi-2025");
        assert!(model_for(&c("GU", 13.4, 144.8)).is_none());
    }
}
