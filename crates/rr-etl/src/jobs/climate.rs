//! Job 6 — climate multipliers for the "2050" dial. Every value is a **projection**, written as a
//! ratio (future / present) the model can multiply a present-day frequency by.
//!
//! Sources:
//! - **NCA5 Interactive Atlas** (USGCRP / NOAA, CC BY 4.0): county changes at global warming
//!   levels of 1.5, 2 and 3 °C relative to 1991-2020 (SSP5-8.5 runs, LOCA2 and STAR-ESDM
//!   downscaling, contiguous US). Precipitation changes are percentages, so the ratio is
//!   1 + change/100. Day counts are changes in days per year, so a baseline is needed:
//! - **LOCA2 ensemble decadal series** (same publisher and licence): the 1990s, 2000s and 2010s
//!   decades averaged give a 1990-2019 baseline for days above 95/100/105 °F, warm nights and
//!   freezing / very cold nights. Ratio = (baseline + change) / baseline; left empty where the
//!   baseline is under two days a year (a ratio of near-zero counts is not meaningful).
//! - **CMRA** (Climate Mapping for Resilience and Adaptation, NOAA/Esri, US Government; licence
//!   field blank): ensemble-mean mid-century (2035-2064) RCP4.5 versus historical (1976-2005)
//!   for consecutive dry days, dry days, days above 90 °F, cooling degree days and days with more
//!   than 1 inch of rain. Mid-century RCP4.5 is close to 2 °C of warming, like the Atlas column.
//!
//! `climate.csv` holds the 2 °C ratios and the CMRA ratios (the 2050 dial). `climate_levels.csv`
//! holds the Atlas ratios at 1.5, 2 and 3 °C for sensitivity displays.

use super::{Ctx, JobOutput, arcgis_query, attr_f64, attr_str, load_counties, missing_groups};
use crate::csvout::Table;
use crate::ct::Crosswalk;
use crate::manifest::Attribution;
use crate::num::sig4;
use crate::Result;
use std::collections::{BTreeMap, BTreeSet};

/// 2050-dial multipliers.
pub const CLIMATE: &str = "core/climate.csv";
/// Atlas ratios at 1.5, 2 and 3 °C (long format).
pub const CLIMATE_LEVELS: &str = "core/climate_levels.csv";

const ATLAS: &str = "https://services3.arcgis.com/0Fs3HcaFfvzXvm7w/arcgis/rest/services";
const CMRA_ITEM: &str = "https://www.arcgis.com/home/item.html?id=54f4e2343500422bbddf1f5dafb30bbd";

/// Atlas layers by warming level: (label, service, field suffix).
const LEVELS: &[(&str, &str, &str)] = &[
    ("gwl15", "NCA_Atlas_Figures_Beta_Counties_view", "GWL1"),
    ("gwl2", "NCA_Atlas_GWL_2C", "GWL2"),
    ("gwl3", "NCA_Atlas_Global_Warming_Level_5_deg_F", "GWL3"),
];

/// How an Atlas variable becomes a ratio.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Change in percent: ratio = 1 + change / 100.
    Percent,
    /// Change in days per year: ratio = (baseline + change) / baseline, baseline from LOCA2.
    Days(&'static str, &'static str),
}

/// Atlas variables: (output name, Atlas field stem, kind, plain definition).
const ATLAS_VARS: &[(&str, &str, Kind, &str)] = &[
    ("hot_days_95f", "tmax_days_ge_95f", Kind::Days("LOCA2_Ensemble_SSP585_Hot_Days_1950_2100", "TMAXDAYSGE95F"), "days a year with a high of at least 95 °F"),
    ("hot_days_100f", "tmax_days_ge_100f", Kind::Days("LOCA2_Ensemble_SSP585_Hot_Days_1950_2100", "TMAXDAYSGE100F"), "days a year with a high of at least 100 °F"),
    ("hot_days_105f", "tmax_days_ge_105f", Kind::Days("LOCA2_Ensemble_SSP585_Hot_Days_1950_2100", "TMAXDAYSGE105F"), "days a year with a high of at least 105 °F"),
    ("warm_nights_70f", "tmin_days_ge_70f", Kind::Days("LOCA2_Ensemble_SSP585_Warm_Nights_1950_2100", "TMINDAYSGE70F"), "nights a year with a low of at least 70 °F"),
    ("freezing_nights", "tmin_days_le_32f", Kind::Days("LOCA2_Ensemble_SSP585_Cold_Days_1950_2100", "TMINDAYSLE32F"), "nights a year with a low at or below 32 °F"),
    ("very_cold_nights_0f", "tmin_days_le_0f", Kind::Days("LOCA2_Ensemble_SSP585_Cold_Days_1950_2100", "TMINDAYSLE0F"), "nights a year with a low at or below 0 °F"),
    ("wettest_day", "prmax1day", Kind::Percent, "rain on the wettest day of the year"),
    ("wettest_day_5yr", "prmax5yr", Kind::Percent, "rain on the wettest day in five years"),
    ("extreme_rain_total", "pr_above_nonzero_99th", Kind::Percent, "yearly rain that falls on days in the top 1% of historical daily amounts"),
    ("extreme_rain_days", "pr_days_above_nonzero_99th", Kind::Percent, "days a year with rain in the top 1% of historical daily amounts"),
    ("annual_rain", "pr_annual", Kind::Percent, "total yearly precipitation"),
];

/// CMRA variables: (output name, field stem, minimum historical value for a ratio, definition).
const CMRA_VARS: &[(&str, &str, f64, &str)] = &[
    ("consecutive_dry_days_mid45", "CONSECDD", 2.0, "longest run of days without rain in a year"),
    ("dry_days_mid45", "PRLT0IN", 2.0, "days a year with less than 0.01 in of rain"),
    ("hot_days_90f_mid45", "TMAX90F", 2.0, "days a year with a high above 90 °F"),
    ("cooling_degree_days_mid45", "CDD", 50.0, "cooling degree days (a measure of air-conditioning need)"),
    ("heavy_rain_days_1in_mid45", "PR1IN", 1.0, "days a year with more than 1 in of rain"),
];

/// Smallest baseline (days per year) for a day-count ratio.
pub const MIN_BASELINE_DAYS: f64 = 2.0;

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let cw = Crosswalk::load(&ctx.data)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();

    // Atlas changes per level: level -> fips -> field stem -> change.
    let mut changes: BTreeMap<&str, BTreeMap<String, BTreeMap<&str, f64>>> = BTreeMap::new();
    for (label, service, suffix) in LEVELS {
        let fields: Vec<String> = std::iter::once("FIPS".to_string())
            .chain(ATLAS_VARS.iter().map(|(_, stem, _, _)| format!("{stem}_{suffix}")))
            .collect();
        let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        let q = arcgis_query(
            ctx,
            &format!("NCA5 Atlas county values at global warming level {label}"),
            &format!("{ATLAS}/{service}/FeatureServer/0"),
            "1=1",
            &field_refs,
            "FIPS",
            false,
            "NCA5 (2023) Interactive Atlas; changes relative to 1991-2020",
            "CC BY 4.0",
            "Credit: USGCRP, Fifth National Climate Assessment Interactive Atlas (CC BY 4.0).",
        )?;
        out.rows_in += q.rows.len() as u64;
        out.source(q.source);
        let m = changes.entry(label).or_default();
        for r in &q.rows {
            let Some(f) = attr_str(r, "FIPS") else { continue };
            let e = m.entry(f).or_default();
            for (_, stem, _, _) in ATLAS_VARS {
                if let Some(v) = attr_f64(r, &format!("{stem}_{suffix}")) {
                    e.insert(stem, v);
                }
            }
        }
    }

    // LOCA2 baselines: mean of the 1990s, 2000s and 2010s decades.
    let mut baselines: BTreeMap<(String, &str), f64> = BTreeMap::new(); // (fips, field) -> days
    let mut by_service: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (_, _, kind, _) in ATLAS_VARS {
        if let Kind::Days(svc, field) = kind {
            by_service.entry(svc).or_default().push(field);
        }
    }
    for (svc, fields) in &by_service {
        let mut fs = vec!["GEOID", "DECADE"];
        fs.extend(fields.iter().copied());
        let q = arcgis_query(
            ctx,
            &format!("LOCA2 ensemble decadal county series ({svc})"),
            &format!("{ATLAS}/{svc}/FeatureServer/0"),
            "DECADE IN (1990,2000,2010)",
            &fs,
            "GEOID,DECADE",
            false,
            "LOCA2 SSP5-8.5 ensemble; decades 1990, 2000, 2010",
            "CC BY 4.0",
            "Credit: USGCRP / NOAA LOCA2 county summaries (CC BY 4.0).",
        )?;
        out.rows_in += q.rows.len() as u64;
        out.source(q.source);
        let mut sums: BTreeMap<(String, &str), (f64, u32)> = BTreeMap::new();
        for r in &q.rows {
            let Some(g) = attr_str(r, "GEOID") else { continue };
            for f in fields {
                if let Some(v) = attr_f64(r, f) {
                    let e = sums.entry((g.clone(), f)).or_insert((0.0, 0));
                    e.0 += v;
                    e.1 += 1;
                }
            }
        }
        for (k, (s, n)) in sums {
            if n == 3 {
                baselines.insert(k, s / 3.0);
            }
        }
    }

    // Harmonise to 2024 counties first (the Atlas and CMRA report Connecticut's old counties,
    // LOCA2 its planning regions), then form ratios.
    let fix = |m: BTreeMap<String, f64>| -> BTreeMap<String, f64> {
        cw.apply_intensive(m).into_iter().filter(|(k, _)| canon.contains(k)).collect()
    };
    let mut base_by_field: BTreeMap<&str, BTreeMap<String, f64>> = BTreeMap::new();
    for ((fips, field), v) in baselines {
        base_by_field.entry(field).or_default().insert(fips, v);
    }
    let base_by_field: BTreeMap<&str, BTreeMap<String, f64>> = base_by_field.into_iter().map(|(f, m)| (f, fix(m))).collect();
    let mut ratios: BTreeMap<&str, BTreeMap<&str, BTreeMap<String, f64>>> = BTreeMap::new(); // level -> var -> fips -> ratio
    for (label, per_county) in &changes {
        for (name, stem, kind, _) in ATLAS_VARS {
            let raw: BTreeMap<String, f64> =
                per_county.iter().filter_map(|(f, vals)| vals.get(stem).map(|v| (f.clone(), *v))).collect();
            for (fips, ch) in fix(raw) {
                let ratio = match kind {
                    Kind::Percent => Some(1.0 + ch / 100.0),
                    Kind::Days(_, field) => base_by_field
                        .get(field)
                        .and_then(|m| m.get(&fips))
                        .filter(|b| **b >= MIN_BASELINE_DAYS)
                        .map(|b| ((b + ch) / b).max(0.0)),
                };
                if let Some(r) = ratio {
                    ratios.entry(label).or_default().entry(name).or_default().insert(fips, r);
                }
            }
        }
    }

    // CMRA mid-century RCP4.5 / historical.
    let mut cmra_fields = vec!["GEOID".to_string()];
    for (_, stem, _, _) in CMRA_VARS {
        cmra_fields.push(format!("HISTORIC_MEAN_{stem}"));
        cmra_fields.push(format!("RCP45MID_MEAN_{stem}"));
    }
    let refs: Vec<&str> = cmra_fields.iter().map(|s| s.as_str()).collect();
    let q = arcgis_query(
        ctx,
        "CMRA county climate projections (CMRA_Tool_Dev, layer Counties)",
        &format!("{ATLAS}/CMRA_Tool_Dev/FeatureServer/0"),
        "1=1",
        &refs,
        "GEOID",
        false,
        "CMRA 2025; LOCA ensemble mean, historical 1976-2005 and RCP4.5 mid-century 2035-2064",
        "US Government work (the ArcGIS item's licence field is blank)",
        "Credit: Climate Mapping for Resilience and Adaptation (NOAA / U.S. Climate Resilience Toolkit).",
    )?;
    out.rows_in += q.rows.len() as u64;
    out.source(q.source);
    let mut cmra: BTreeMap<&str, BTreeMap<String, f64>> = BTreeMap::new();
    for r in &q.rows {
        let Some(g) = attr_str(r, "GEOID") else { continue };
        for (name, stem, min_hist, _) in CMRA_VARS {
            let (Some(h), Some(f)) = (attr_f64(r, &format!("HISTORIC_MEAN_{stem}")), attr_f64(r, &format!("RCP45MID_MEAN_{stem}"))) else {
                continue;
            };
            if h >= *min_hist {
                cmra.entry(name).or_default().insert(g.clone(), (f / h).max(0.0));
            }
        }
    }

    // CMRA reports Connecticut's old counties: convert its ratios to planning regions.
    let cmra: BTreeMap<&str, BTreeMap<String, f64>> = cmra.into_iter().map(|(v, m)| (v, fix(m))).collect();

    // climate.csv: 2 °C Atlas ratios + CMRA ratios.
    let mut header = vec!["fips".to_string()];
    header.extend(ATLAS_VARS.iter().map(|(n, _, _, _)| n.to_string()));
    header.extend(CMRA_VARS.iter().map(|(n, _, _, _)| n.to_string()));
    let mut table = Table::with_header(header, 1);
    let mut covered = BTreeSet::new();
    let gwl2 = ratios.get("gwl2");
    for c in &counties {
        let mut row = vec![c.fips.clone()];
        let mut any = false;
        for (name, _, _, _) in ATLAS_VARS {
            let v = gwl2.and_then(|m| m.get(name)).and_then(|m| m.get(&c.fips)).copied();
            any |= v.is_some();
            row.push(v.map(sig4).unwrap_or_default());
        }
        for (name, _, _, _) in CMRA_VARS {
            let v = cmra.get(name).and_then(|m| m.get(&c.fips)).copied();
            any |= v.is_some();
            row.push(v.map(sig4).unwrap_or_default());
        }
        if any {
            table.push(row);
            covered.insert(c.fips.clone());
        }
    }
    out.table(ctx, CLIMATE, &mut table)?;

    // climate_levels.csv: Atlas ratios at every level, long format.
    let mut levels = Table::new(&["fips", "variable", "gwl15", "gwl2", "gwl3"], 2);
    for c in &counties {
        for (name, _, _, _) in ATLAS_VARS {
            let get = |l: &str| ratios.get(l).and_then(|m| m.get(name)).and_then(|m| m.get(&c.fips)).copied();
            let (a, b, d) = (get("gwl15"), get("gwl2"), get("gwl3"));
            if a.is_none() && b.is_none() && d.is_none() {
                continue;
            }
            levels.push(vec![
                c.fips.clone(),
                name.to_string(),
                a.map(sig4).unwrap_or_default(),
                b.map(sig4).unwrap_or_default(),
                d.map(sig4).unwrap_or_default(),
            ]);
        }
    }
    out.table(ctx, CLIMATE_LEVELS, &mut levels)?;

    out.missing = missing_groups(&counties, &covered, |c| {
        if super::is_outside_conus(&c.state_abbr) {
            "The NCA5 Atlas, LOCA2 and CMRA county projections cover the contiguous US only".to_string()
        } else {
            "No county projection values".to_string()
        }
    });
    for (name, stem, kind, def) in ATLAS_VARS {
        let how = match kind {
            Kind::Percent => format!("1 + change/100 of NCA5 Atlas `{stem}` at +2 °C"),
            Kind::Days(_, f) => format!("(baseline + change) / baseline, change = NCA5 Atlas `{stem}` at +2 °C, baseline = LOCA2 `{f}` 1990-2019 mean; empty when the baseline is under {MIN_BASELINE_DAYS} day a year"),
        };
        out.definitions.insert(name.to_string(), format!("Projected multiplier for {def}: {how}."));
    }
    for (name, stem, min_hist, def) in CMRA_VARS {
        out.definitions.insert(
            name.to_string(),
            format!("Projected multiplier for {def}: CMRA RCP45MID_MEAN_{stem} / HISTORIC_MEAN_{stem} (2035-2064 vs 1976-2005); empty when the historical value is under {min_hist}."),
        );
    }
    out.notes.push("All values are projections (a multiplier on today's frequency), not observations. The Atlas columns describe a world 2 °C warmer than pre-industrial (the NCA5 central case for mid-century); the CMRA columns are the RCP4.5 mid-century ensemble mean. Connecticut values are land-area-weighted averages of the old counties' values.".into());
    out.notes.push("No fire-weather index is published at county level by these sources; consecutive dry days, dry days and hot days are the closest proxies for wildfire weather.".into());
    let accessed = crate::timefmt::today_utc();
    out.attributions.push(Attribution {
        source: "NCA5 Atlas and LOCA2".into(),
        text: "Climate projections: U.S. Global Change Research Program, Fifth National Climate Assessment Interactive Atlas and LOCA2 county summaries (CC BY 4.0). Ratios derived by Ready Reckoner.".into(),
        license: "CC BY 4.0".into(),
        url: "https://www.arcgis.com/home/item.html?id=2f7a8715927b4ba083be450c85f7f761".into(),
        version: Some("NCA5 (2023)".into()),
        accessed: accessed.clone(),
    });
    out.attributions.push(Attribution {
        source: "CMRA".into(),
        text: "Climate projections: Climate Mapping for Resilience and Adaptation (CMRA), NOAA and the U.S. Climate Resilience Toolkit.".into(),
        license: "US Government work".into(),
        url: CMRA_ITEM.into(),
        version: Some("CMRA 2025".into()),
        accessed,
    });
    Ok(out)
}
