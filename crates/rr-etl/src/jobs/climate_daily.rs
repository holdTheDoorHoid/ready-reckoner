//! Job `climate_daily` — heat and cold days by county and month from NOAA nClimGrid-Daily, and
//! the share of each county's recorded outage hours that fell on dangerously hot or cold days
//! (model review M-11: heat and cold shares per county instead of one national constant).
//!
//! nClimGrid-Daily publishes county-average daily maximum and minimum temperature (°C) for the
//! 3,107 counties of the contiguous US, one CSV per variable and month
//! (`<var>-YYYYMM-cty-scaled.csv`). The county code in those files is **not** a FIPS code: it is
//! NCEI's alphabetical state number (01 Alabama, 02 Arizona, ... 06 Connecticut, ... 48
//! Wyoming) followed by the county's FIPS code, so the state is taken from the abbreviation in
//! the name column (`AZ: Apache County`) instead. Connecticut is reported by its eight old
//! counties and converted to the planning regions by land share.
//!
//! Output `core/temperature.csv`, per county:
//! - `tmax_ge_90f_MM`, `tmax_ge_100f_MM`: share of days in month MM (1991-2020) with a high of at
//!   least 90 °F / 100 °F; `tmin_le_20f_MM`, `tmin_le_0f_MM`: share with a low of at most 20 °F /
//!   0 °F;
//! - `outage_hot_share`, `outage_cold_share`: share of the county's outage customer-hours
//!   (EAGLE-I events 2014-2025, from the `outages` job) on days with a high of at least 95 °F /
//!   a low of at most 20 °F, and the same over the county's region (customer-hour weighted,
//!   within 400 km), steadier for small counties.

// Twelve months across four parallel share arrays: indexing by month reads more plainly.
#![allow(clippy::needless_range_loop)]

use super::{Ctx, JobOutput, load_counties};
use crate::csvout::Table;
use crate::ct::{Crosswalk, is_old_ct};
use crate::geo::haversine_km;
use crate::intermediate as im;
use crate::manifest::SourceRecord;
use crate::num::sig;
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Output file.
pub const TEMPERATURE: &str = "core/temperature.csv";

const BASE: &str = "https://www.ncei.noaa.gov/data/nclimgrid-daily/access/averages/";
const PRODUCT: &str = "https://www.ncei.noaa.gov/products/land-based-station/nclimgrid-daily";

/// Climate normal period for the monthly shares.
pub const NORMALS: (i32, i32) = (1991, 2020);
/// Years joined to the outage record.
pub const JOIN: (i32, i32) = (2014, 2025);

/// 90 °F in °C.
pub const F90: f32 = 32.222_22;
/// 95 °F in °C.
pub const F95: f32 = 35.0;
/// 100 °F in °C.
pub const F100: f32 = 37.777_78;
/// 20 °F in °C.
pub const F20: f32 = -6.666_67;
/// 0 °F in °C.
pub const F0: f32 = -17.777_78;
/// Radius of the regional outage-hour shares.
pub const REGION_KM: f64 = 400.0;

/// FIPS code for an nClimGrid county row: the state from the abbreviation in the name
/// (`"AZ: Apache County"`) and the county from the last three digits of the id.
pub fn fips_of(id: &str, name: &str, state_fips: &HashMap<&str, &str>) -> Option<String> {
    let abbr = name.split(':').next()?.trim();
    let st = state_fips.get(abbr)?;
    let county = id.trim().get(id.trim().len().checked_sub(3)?..)?;
    Some(format!("{st}{county}"))
}

/// One parsed county-month: FIPS, year, month, and the daily values (°C; `None` when missing).
pub type CountyMonth = (String, i32, u32, Vec<Option<f32>>);

/// Parse one nClimGrid-Daily county file.
pub fn parse_month(text: &str, state_fips: &HashMap<&str, &str>) -> Result<Vec<CountyMonth>> {
    let mut out = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    for rec in rdr.records() {
        let rec = rec?;
        if rec.len() < 7 || rec.get(0) != Some("cty") {
            continue;
        }
        let Some(fips) = fips_of(&rec[1], &rec[2], state_fips) else {
            continue;
        };
        let y: i32 = rec[3]
            .trim()
            .parse()
            .map_err(|_| data_err("nClimGrid: bad year"))?;
        let m: u32 = rec[4]
            .trim()
            .parse()
            .map_err(|_| data_err("nClimGrid: bad month"))?;
        let days: Vec<Option<f32>> = (6..rec.len())
            .map(|i| {
                rec[i]
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|v| *v > -500.0 && v.is_finite())
            })
            .collect();
        out.push((fips, y, m, days));
    }
    Ok(out)
}

#[derive(Debug, Default, Clone)]
struct MonthCounts {
    days_tmax: [u32; 12],
    days_tmin: [u32; 12],
    ge90: [u32; 12],
    ge100: [u32; 12],
    le20: [u32; 12],
    le0: [u32; 12],
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let cw = Crosswalk::load(&ctx.data)?;
    let state_fips: HashMap<&str, &str> = crate::jobs::geography::STATE_FACTS
        .iter()
        .map(|(f, a, _, _)| (*a, *f))
        .collect();
    let day0 = crate::timefmt::days_from_civil(i64::from(JOIN.0), 1, 1);
    let n_days = (crate::timefmt::days_from_civil(i64::from(JOIN.1) + 1, 1, 1) - day0) as usize;
    let mut counts: BTreeMap<String, MonthCounts> = BTreeMap::new();
    // Daily highs and lows for the join years, per nClimGrid county (old Connecticut codes kept).
    let mut daily: HashMap<String, (Vec<f32>, Vec<f32>)> = HashMap::new();
    let mut acc = [crate::http::Sha256Acc::new(), crate::http::Sha256Acc::new()];
    let mut files = 0u64;
    let first = NORMALS.0.min(JOIN.0);
    let last = NORMALS.1.max(JOIN.1);
    let retrieved = crate::timefmt::now_utc();
    for y in first..=last {
        let in_normals = (NORMALS.0..=NORMALS.1).contains(&y);
        let in_join = (JOIN.0..=JOIN.1).contains(&y);
        if !in_normals && !in_join {
            continue;
        }
        eprintln!("  nClimGrid {y}");
        for m in 1..=12u32 {
            for (vi, var) in ["tmax", "tmin"].iter().enumerate() {
                let url = format!("{BASE}{y}/{var}-{y}{m:02}-cty-scaled.csv");
                let f = ctx
                    .http
                    .get(&url, Some(&format!("climate_daily/{var}-{y}{m:02}.csv")))?;
                acc[vi].update(&f.bytes);
                files += 1;
                let rows = parse_month(&f.text(), &state_fips)?;
                out.rows_in += rows.len() as u64;
                for (fips, ry, rm, days) in rows {
                    if ry != y || rm != m {
                        return Err(data_err(format!(
                            "nClimGrid {url}: row for {ry}-{rm:02} in the {y}-{m:02} file"
                        )));
                    }
                    let dim = days_in_month(y, m);
                    let vals: Vec<(usize, f32)> = days
                        .iter()
                        .take(dim)
                        .enumerate()
                        .filter_map(|(d, v)| v.map(|x| (d, x)))
                        .collect();
                    if in_normals {
                        let c = counts.entry(fips.clone()).or_default();
                        let k = (m - 1) as usize;
                        for (_, x) in &vals {
                            if vi == 0 {
                                c.days_tmax[k] += 1;
                                c.ge90[k] += u32::from(*x >= F90);
                                c.ge100[k] += u32::from(*x >= F100);
                            } else {
                                c.days_tmin[k] += 1;
                                c.le20[k] += u32::from(*x <= F20);
                                c.le0[k] += u32::from(*x <= F0);
                            }
                        }
                    }
                    if in_join {
                        let e = daily
                            .entry(fips)
                            .or_insert_with(|| (vec![f32::NAN; n_days], vec![f32::NAN; n_days]));
                        let base =
                            (crate::timefmt::days_from_civil(i64::from(y), m, 1) - day0) as usize;
                        for (d, x) in vals {
                            if base + d < n_days {
                                if vi == 0 {
                                    e.0[base + d] = x;
                                } else {
                                    e.1[base + d] = x;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for (vi, var) in ["daily maximum", "daily minimum"].iter().enumerate() {
        let a = std::mem::take(&mut acc[vi]);
        out.source(SourceRecord {
            name: format!("NOAA NCEI nClimGrid-Daily county averages, {var} temperature"),
            url: format!(
                "{BASE}<year>/{}-<yyyymm>-cty-scaled.csv ({first}-{last}, {} monthly files)",
                if vi == 0 { "tmax" } else { "tmin" },
                files / 2
            ),
            version: "nClimGrid-Daily v1.0.0 (scaled county averages)".into(),
            retrieved: retrieved.clone(),
            bytes: a.len(),
            sha256: a.finish(),
            license: super::PUBLIC_DOMAIN.into(),
            obligations: String::new(),
        });
    }

    // --- Outage hours on hot and cold days ----------------------------------------------------
    let mut unit_hours: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new(); // total, hot, cold
    let mut joined = 0u64;
    im::for_each(&ctx.data, im::OUTAGE_DAILY, "outages", |r| {
        let Some(t) = daily.get(&r[0]) else {
            return Ok(());
        };
        let day: i64 = r[1].parse().unwrap_or(0);
        let h: f64 = r[2].parse().unwrap_or(0.0);
        let i = day - day0;
        if i < 0 || i as usize >= n_days {
            return Ok(());
        }
        let (hi, lo) = (t.0[i as usize], t.1[i as usize]);
        if hi.is_nan() || lo.is_nan() {
            return Ok(());
        }
        let e = unit_hours.entry(r[0].to_string()).or_default();
        e.0 += h;
        if hi >= F95 {
            e.1 += h;
        }
        if lo <= F20 {
            e.2 += h;
        }
        joined += 1;
        Ok(())
    })?;

    // Canonical counties: Connecticut regions from the old counties by land share.
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let mut by_county: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
    for code in counts.keys() {
        if canon.contains(code) {
            by_county
                .entry(code.clone())
                .or_default()
                .push((code.clone(), 1.0));
        } else if is_old_ct(code) {
            for o in cw.overlaps.iter().filter(|o| &o.old == code) {
                by_county
                    .entry(o.region.clone())
                    .or_default()
                    .push((code.clone(), o.share_of_region));
            }
        } else if let Some(new) = crate::ct::successors(code) {
            for n in new {
                by_county
                    .entry(n.to_string())
                    .or_default()
                    .push((code.clone(), 1.0));
            }
        }
    }
    let mut hours: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new();
    for (county, parts) in &by_county {
        let mut e = (0.0, 0.0, 0.0);
        for (code, w) in parts {
            if let Some(u) = unit_hours.get(code) {
                e.0 += w * u.0;
                e.1 += w * u.1;
                e.2 += w * u.2;
            }
        }
        if e.0 > 0.0 {
            hours.insert(county.clone(), e);
        }
    }
    let loc: HashMap<&str, (f64, f64)> = counties
        .iter()
        .map(|c| (c.fips.as_str(), (c.lat, c.lon)))
        .collect();

    let mut header: Vec<String> = vec!["fips".into()];
    for p in ["tmax_ge_90f", "tmax_ge_100f", "tmin_le_20f", "tmin_le_0f"] {
        for m in 1..=12 {
            header.push(format!("{p}_{m:02}"));
        }
    }
    for c in [
        "outage_hot_share",
        "outage_cold_share",
        "region_outage_hot_share",
        "region_outage_cold_share",
    ] {
        header.push(c.into());
    }
    let mut table = Table::with_header(header, 1);
    // Day shares to whole percentage points (a third of a day a month): the normals move by more
    // than that from one decade to the next, and the file is part of every first visit.
    let share2 = |x: f64| crate::num::fixed(x, 2);
    for (county, parts) in &by_county {
        let mut mc = [[0.0f64; 12]; 4];
        let mut wsum = 0.0;
        for (code, w) in parts {
            let Some(c) = counts.get(code) else { continue };
            wsum += w;
            for k in 0..12 {
                let dx = f64::from(c.days_tmax[k].max(1));
                let dn = f64::from(c.days_tmin[k].max(1));
                mc[0][k] += w * f64::from(c.ge90[k]) / dx;
                mc[1][k] += w * f64::from(c.ge100[k]) / dx;
                mc[2][k] += w * f64::from(c.le20[k]) / dn;
                mc[3][k] += w * f64::from(c.le0[k]) / dn;
            }
        }
        if wsum <= 0.0 {
            continue;
        }
        let mut row = vec![county.clone()];
        for v in &mc {
            row.extend(v.iter().map(|x| share2(x / wsum)));
        }
        match hours.get(county) {
            Some((t, h, c)) => {
                row.push(sig(h / t, 3));
                row.push(sig(c / t, 3));
            }
            None => {
                row.push(String::new());
                row.push(String::new());
            }
        }
        // Region: customer-hour weighted over the county and its neighbours within 400 km.
        let (mut t, mut h, mut c) = (0.0, 0.0, 0.0);
        if let Some(&(lat, lon)) = loc.get(county.as_str()) {
            for (other, (ot, oh, oc)) in &hours {
                let Some(&(la, lo)) = loc.get(other.as_str()) else {
                    continue;
                };
                if (la - lat).abs() > 4.0 {
                    continue;
                }
                let km = haversine_km(lat, lon, la, lo);
                if km < REGION_KM {
                    let u = km / REGION_KM;
                    let w = (1.0 - u * u) * (1.0 - u * u);
                    t += w * ot;
                    h += w * oh;
                    c += w * oc;
                }
            }
        }
        if t > 0.0 {
            row.push(sig(h / t, 3));
            row.push(sig(c / t, 3));
        } else {
            row.push(String::new());
            row.push(String::new());
        }
        table.push(row);
    }
    if table.rows.len() < 3_000 {
        return Err(data_err(format!(
            "nClimGrid: only {} counties (3,107 contiguous-US counties expected)",
            table.rows.len()
        )));
    }
    // Guards: Maricopa's Julys are hot, Miami-Dade never reaches 20 °F.
    let get = |f: &str, col: &str| -> Option<f64> {
        let i = table.header.iter().position(|h| h == col)?;
        table.rows.iter().find(|r| r[0] == f)?[i].parse().ok()
    };
    if get("04013", "tmax_ge_90f_07").unwrap_or(0.0) < 0.9
        || get("12086", "tmin_le_20f_01").unwrap_or(1.0) > 0.0
    {
        return Err(data_err(
            "nClimGrid sanity check failed (Maricopa July heat or Miami-Dade January cold)",
        ));
    }
    let covered: BTreeSet<String> = table.rows.iter().map(|r| r[0].clone()).collect();
    out.table(ctx, TEMPERATURE, &mut table)?;
    out.missing = super::missing_groups(&counties, &covered, |c| {
        if super::is_outside_conus(&c.state_abbr) {
            "nClimGrid-Daily covers the contiguous US only".to_string()
        } else {
            "No nClimGrid-Daily county series".to_string()
        }
    });
    out.notes.push(format!("Monthly shares: days in {}-{} with a county-average high of at least 90 °F (32.2 °C) or 100 °F (37.8 °C), or a low of at most 20 °F (-6.7 °C) or 0 °F (-17.8 °C), divided by the days with data in that calendar month. County averages run cooler than the hottest spot in a county and warmer than the coldest.", NORMALS.0, NORMALS.1));
    out.notes.push(format!("Outage shares: EAGLE-I event customer-hours ({}-{}, repaired series, from the outages job) by local calendar day (standard time from the county's longitude), on days with a county-average high of at least 95 °F (35 °C) or a low of at most 20 °F; {joined} unit-days joined. Days without temperature data are left out of both parts of the share. The region share weights the county and every county within {REGION_KM:.0} km by customer-hours and by (1 - (distance/{REGION_KM:.0} km)^2)^2.", JOIN.0, JOIN.1));
    out.notes.push("nClimGrid's county code is NCEI's alphabetical state number followed by the county FIPS code (02001 is Apache County, Arizona), so the state comes from the abbreviation in the name column. Connecticut's eight old counties are converted to planning regions by land share.".into());
    out.attributions.push(crate::manifest::Attribution {
        source: "NOAA nClimGrid-Daily".into(),
        text: "Daily temperature from NOAA's NClimGrid-Daily county averages (Durre et al., NOAA National Centers for Environmental Information).".into(),
        license: super::PUBLIC_DOMAIN.into(),
        url: PRODUCT.into(),
        version: Some("v1.0.0".into()),
        accessed: retrieved[..10].to_string(),
    });
    Ok(out)
}

/// Days in a month.
pub fn days_in_month(y: i32, m: u32) -> usize {
    let next = if m == 12 {
        crate::timefmt::days_from_civil(i64::from(y) + 1, 1, 1)
    } else {
        crate::timefmt::days_from_civil(i64::from(y), m + 1, 1)
    };
    (next - crate::timefmt::days_from_civil(i64::from(y), m, 1)) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states() -> HashMap<&'static str, &'static str> {
        crate::jobs::geography::STATE_FACTS
            .iter()
            .map(|(f, a, _, _)| (*a, *f))
            .collect()
    }

    #[test]
    fn ncei_state_numbers_are_not_fips() {
        let s = states();
        // 02001 in nClimGrid is Apache County, Arizona (FIPS 04001), not Alaska.
        assert_eq!(
            fips_of("02001", "AZ: Apache County", &s).as_deref(),
            Some("04001")
        );
        // Connecticut (NCEI 06) keeps its old county codes, as EAGLE-I does.
        assert_eq!(
            fips_of("06001", "CT: Fairfield County", &s).as_deref(),
            Some("09001")
        );
        assert_eq!(
            fips_of("01001", "AL: Autauga", &s).as_deref(),
            Some("01001")
        );
        assert_eq!(fips_of("99001", "XX: Nowhere", &s), None);
    }

    #[test]
    fn a_month_parses_with_missing_days() {
        let text = "cty,02001,AZ: Apache County,2023,02,TMIN,   -11.04,   -12.12,  -999.99\n";
        let rows = parse_month(text, &states()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "04001");
        assert_eq!((rows[0].1, rows[0].2), (2023, 2));
        assert_eq!(rows[0].3, vec![Some(-11.04), Some(-12.12), None]);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2023, 12), 31);
    }

    #[test]
    fn thresholds_in_celsius() {
        let c = |f: f32| (f - 32.0) * 5.0 / 9.0;
        for (t, f) in [
            (F90, 90.0),
            (F95, 95.0),
            (F100, 100.0),
            (F20, 20.0),
            (F0, 0.0),
        ] {
            assert!((t - c(f)).abs() < 1e-4, "{t} vs {f}");
        }
    }
}
