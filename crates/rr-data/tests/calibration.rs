//! Integration tests of the data-pack v2 calibration files against the committed packs: the
//! backtest counties of the round-2 model review (Linn IA, Buncombe NC, Jefferson LA, Harris and
//! Travis TX, Hinds MS, San Juan PR) and the promises the outage model makes.

use rr_data::{DataStore, Manifest, outage_region};
use rr_types::{CountyRecord, PoolBasis};
use std::path::PathBuf;
use std::sync::OnceLock;

fn data_dir() -> PathBuf {
    std::env::var("RR_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"))
}

/// Every file the manifest lists, loaded (manifest first); `None` before the v2 files exist.
fn store() -> Option<&'static DataStore> {
    static STORE: OnceLock<Option<DataStore>> = OnceLock::new();
    STORE
        .get_or_init(|| {
            let mbytes = std::fs::read(data_dir().join("manifest.json")).ok()?;
            let m: Manifest = serde_json::from_slice(&mbytes).ok()?;
            m.file("core/outage_pooled.csv")?;
            let mut files: Vec<(String, Vec<u8>)> = vec![("manifest.json".into(), mbytes)];
            for f in m.packs.values().flat_map(|p| p.files.iter()) {
                let bytes = std::fs::read(data_dir().join(&f.path)).ok()?;
                files.push((f.path.clone(), bytes));
            }
            let refs: Vec<(&str, &[u8])> = files
                .iter()
                .map(|(n, b)| (n.as_str(), b.as_slice()))
                .collect();
            let mut s = DataStore::new();
            s.load_many(&refs).unwrap_or_else(|e| panic!("{e}"));
            Some(s)
        })
        .as_ref()
}

fn county(fips: &str) -> &'static CountyRecord {
    store()
        .unwrap()
        .county(fips)
        .unwrap_or_else(|| panic!("{fips}"))
}

fn stress_event(fips: &str) -> &'static rr_types::StressEvent {
    county(fips)
        .outage_model
        .as_ref()
        .and_then(|m| m.stress.as_ref())
        .unwrap_or_else(|| panic!("{fips}: no stress event"))
}

fn share_at(e: &rr_types::StressEvent, days: f32) -> f32 {
    e.share_out_at_days
        .iter()
        .find(|(d, _)| *d == days)
        .map(|(_, s)| *s)
        .unwrap_or(0.0)
}

#[test]
fn the_stress_table_names_the_record_storms() {
    let Some(_) = store() else { return };
    // Buncombe: Helene, recorded in Buncombe itself, a fifth of customers still out at two weeks
    // (about 9,300 were still out on day 17, Blue Ridge Public Radio, 14 October 2024).
    let b = stress_event("37021");
    assert_eq!(b.cause, "AL092024", "{b:?}");
    assert!(b.event.contains("Helene"), "{b:?}");
    assert_eq!(b.recorded_in.as_deref(), Some("37021"));
    assert!(share_at(b, 7.0) > 0.5 && share_at(b, 14.0) > 0.1, "{b:?}");
    // Linn: the August 2020 derecho.
    let l = stress_event("19113");
    assert!(l.event.contains("derecho"), "{l:?}");
    assert_eq!(l.date, "2020-08-10");
    assert_eq!(l.class, "wind");
    // Jefferson Parish: Ida.
    let j = stress_event("22051");
    assert_eq!(j.cause, "AL092021", "{j:?}");
    assert!(j.event.contains("Ida"));
    // San Juan: Maria, hand-copied (EAGLE-I's Puerto Rico series starts in 2021).
    let s = stress_event("72127");
    assert!(s.event.contains("Maria"), "{s:?}");
    assert!(s.source.starts_with("historic:"));
    assert!(share_at(s, 30.0) > 0.5);
    // Travis: the February 2021 winter storm and blackout.
    let t = stress_event("48453");
    assert!(t.date.starts_with("2021-02"), "{t:?}");
    // Harris and Hinds have a worst event in their region too.
    for f in ["48201", "28049"] {
        assert!(share_at(stress_event(f), 1.0) > 0.5, "{f}");
    }
}

#[test]
fn pooled_tails_blend_a_record_storm_into_the_region() {
    let Some(_) = store() else { return };
    let m = |f: &str| {
        county(f)
            .outage_model
            .clone()
            .unwrap_or_else(|| panic!("{f}"))
    };
    let linn = m("19113");
    assert_eq!(linn.basis, PoolBasis::Blend);
    // Linn's own record: the derecho left about 0.4 customer outages of a week or more per
    // customer in 11.5 years (0.035 a year). Pooled, the week-long rate is a fraction of that,
    // and the county's own weight at a week is small.
    let own7 = county("19113")
        .outages
        .as_ref()
        .map(|o| o.events_per_customer_year * o.p_ge_7d)
        .unwrap();
    assert!(own7 > 0.02, "{own7}");
    assert!(linn.lam_ge[2] < 0.25 * own7, "{} vs {own7}", linn.lam_ge[2]);
    assert!(linn.z[2] < 0.2 && linn.z[0] > 0.3, "{:?}", linn.z);
    // The Iowa counties of M-01 E2: the 3-day rate no longer depends on where one storm fell.
    let iowa = [
        "19113", "19103", "19163", "19153", "19155", "19013", "19193",
    ];
    let r3: Vec<f32> = iowa.iter().map(|f| m(f).lam_ge[1]).collect();
    let (lo, hi) = r3
        .iter()
        .fold((f32::MAX, 0.0f32), |(a, b), x| (a.min(*x), b.max(*x)));
    assert!(lo > 0.0 && hi / lo < 3.0, "{r3:?}");
    // Rates fall with length, weights too.
    for f in [
        "19113", "37021", "22051", "48201", "48453", "28049", "72127",
    ] {
        let x = m(f);
        for k in 1..5 {
            assert!(
                x.lam_ge[k] <= x.lam_ge[k - 1] + 1e-12,
                "{f}: {:?}",
                x.lam_ge
            );
            assert!(x.z[k] <= x.z[k - 1] + 1e-6, "{f}: {:?}", x.z);
        }
        assert!(x.region_counties > 0, "{f}");
    }
    // Hurricanes are carried by their own rows, so Jefferson Parish's pool leaves most of its
    // long outages out.
    let jeff = m("22051");
    assert!(
        jeff.causes.get("hurricane").copied().unwrap_or(0.0) > 0.1,
        "{:?}",
        jeff.causes
    );
    assert!(jeff.causes_ge_1d.get("hurricane").copied().unwrap_or(0.0) > 0.5);
}

#[test]
fn restoration_curves_cover_the_regions_and_the_island_grid() {
    let Some(s) = store() else { return };
    let pr = county("72127");
    assert_eq!(outage_region(pr), "puerto_rico");
    let maria = s
        .restoration_curves()
        .iter()
        .find(|c| c.region == "puerto_rico" && c.class.starts_with("historic:maria"))
        .expect("Maria curve");
    // The last customers were restored 328 days after Maria; nine in ten took months.
    assert!(maria.t90_days > 100.0, "{maria:?}");
    assert_eq!(maria.factor, Some(5.0));
    let mainland = s
        .restoration_curve("mainland", "hurricane")
        .expect("mainland hurricanes");
    assert!(
        mainland.events > 50 && mainland.t90_days > 1.0,
        "{mainland:?}"
    );
    for c in s.restoration_curves() {
        for w in c.share_out_at_days.windows(2) {
            assert!(w[1].1 <= w[0].1 + 1e-6, "{c:?}");
        }
    }
}

#[test]
fn national_series_load_with_sources() {
    let Some(s) = store() else { return };
    let oe = s.series("oe417").expect("oe417");
    assert_eq!(oe.licence, "CC BY 4.0");
    assert!(!oe.attribution.is_empty());
    let attacks = s
        .series_rate("oe417", "grid_physical_attack_reports_per_year")
        .expect("attack rate");
    assert!(attacks.value > 10.0 && attacks.low.unwrap() < attacks.value);
    assert!(
        s.series_rate("fdic_failures", "bank_failures_per_year")
            .is_some()
    );
    assert!(
        s.series_rate("drug_shortages", "drug_shortages_current")
            .unwrap()
            .value
            > 10.0
    );
    assert!(
        s.attributions()
            .iter()
            .any(|a| a.source.contains("PNNL") && a.text.contains("CC BY 4.0"))
    );
}

/// Spearman rank correlation.
fn spearman(pairs: &[(f64, f64)]) -> f64 {
    let n = pairs.len();
    let rank = |vals: Vec<f64>| -> Vec<f64> {
        let mut idx: Vec<usize> = (0..n).collect();
        idx.sort_by(|a, b| vals[*a].total_cmp(&vals[*b]));
        let mut r = vec![0.0; n];
        let mut i = 0;
        while i < n {
            let mut j = i;
            while j + 1 < n && vals[idx[j + 1]] == vals[idx[i]] {
                j += 1;
            }
            for k in i..=j {
                r[idx[k]] = (i + j) as f64 / 2.0 + 1.0;
            }
            i = j + 1;
        }
        r
    };
    let a = rank(pairs.iter().map(|p| p.0).collect());
    let b = rank(pairs.iter().map(|p| p.1).collect());
    let (ma, mb) = (
        a.iter().sum::<f64>() / n as f64,
        b.iter().sum::<f64>() / n as f64,
    );
    let num: f64 = (0..n).map(|i| (a[i] - ma) * (b[i] - mb)).sum();
    let den = ((0..n).map(|i| (a[i] - ma) * (a[i] - ma)).sum::<f64>()
        * (0..n).map(|i| (b[i] - mb) * (b[i] - mb)).sum::<f64>())
    .sqrt();
    num / den
}

#[test]
fn eaglei_outage_rates_track_utility_saifi() {
    let Some(s) = store() else { return };
    // Where a county has both, EAGLE-I's outages per customer-year (events of 1% or more of the
    // county) should rank with the utilities' SAIFI and sit below it (SAIFI counts every
    // sustained interruption); few counties should exceed three times SAIFI (M-02's flicker flag).
    let mut pairs = Vec::new();
    let mut above = 0;
    for c in s.counties() {
        let (Some(o), Some(r)) = (c.outages.as_ref(), c.reliability.as_ref()) else {
            continue;
        };
        if o.state_series.is_some() || c.state_abbr == "PR" {
            continue;
        }
        let Some(saifi) = r.saifi_with_med else {
            continue;
        };
        let e = f64::from(o.events_per_customer_year);
        pairs.push((e, f64::from(saifi)));
        if e > 3.0 * f64::from(saifi) {
            above += 1;
        }
    }
    if pairs.is_empty() {
        return; // reliability.csv not built yet
    }
    assert!(pairs.len() > 2_500, "{}", pairs.len());
    let rho = spearman(&pairs);
    assert!(rho > 0.2, "rank correlation {rho}");
    let below = pairs.iter().filter(|(e, s)| e < s).count();
    assert!(
        below as f64 > 0.8 * pairs.len() as f64,
        "{below} of {}",
        pairs.len()
    );
    assert!(
        (above as f64) < 0.02 * pairs.len() as f64,
        "{above} above 3x SAIFI"
    );
}

#[test]
fn temperature_shares_and_outage_hours() {
    let Some(_) = store() else { return };
    let t = |f: &str| county(f).temperature.clone();
    let Some(maricopa) = t("04013") else { return };
    assert!(maricopa.tmax_ge_90f[6] > 0.9, "{:?}", maricopa.tmax_ge_90f);
    assert!(maricopa.tmax_ge_100f[6] > 0.5);
    assert_eq!(
        t("12086").unwrap().tmin_le_20f[0],
        0.0,
        "Miami-Dade never reaches 20 °F"
    );
    assert!(
        t("27053").unwrap().tmin_le_0f[0] > 0.05,
        "Hennepin January nights below 0 °F"
    );
    // February 2021: a share of Travis County's outage hours fell on days at or below 20 °F.
    let travis = t("48453").unwrap();
    assert!(travis.outage_cold_share.unwrap() > 0.05, "{travis:?}");
    // Alaska, Hawaii and the territories are outside nClimGrid.
    assert!(t("02020").is_none() && t("15003").is_none() && t("72127").is_none());
}

#[test]
fn declarations_and_recovery_counts() {
    let Some(_) = store() else { return };
    let d = |f: &str| county(f).declarations.clone();
    let Some(harris) = d("48201") else { return };
    assert!(
        harris.since_2000 >= 10 && harris.hurricane_or_flood >= 5,
        "{harris:?}"
    );
    assert!(harris.with_individual_assistance <= harris.since_2000);
    assert!(harris.last_5yr <= harris.since_2000);
    let n = store()
        .unwrap()
        .counties()
        .filter(|c| c.declarations.is_some())
        .count();
    assert_eq!(n, 3232);
}

#[test]
fn arrests_and_funding_gaps() {
    let Some(s) = store() else { return };
    let Some(m) = s.series_rate("fbi_arrests", "arrests_per_100k_male_all") else {
        return;
    };
    // About 3,300 arrests per 100,000 men and 1,200 per 100,000 women a year (FBI CIUS 2023-2025,
    // scaled to the national estimate).
    assert!((m.value - 3_300.0).abs() < 400.0, "{}", m.value);
    let f = s
        .series_rate("fbi_arrests", "arrests_per_100k_female_all")
        .unwrap();
    assert!((f.value - 1_200.0).abs() < 200.0, "{}", f.value);
    let young = s
        .series_rate("fbi_arrests", "arrests_per_100k_male_25_34")
        .unwrap();
    let old = s
        .series_rate("fbi_arrests", "arrests_per_100k_male_65_plus")
        .unwrap();
    assert!(young.value > 5.0 * old.value);
    // Four fiscal years since 1981 with a lapse of 14 days or more: FY1996, FY2014, FY2019, FY2026.
    let g = s
        .series_rate("funding_gaps", "funding_gap_ge_14d_per_year")
        .unwrap();
    assert!((g.value - 4.0 / 45.0).abs() < 1e-3, "{}", g.value);
    let dirs = s.series("fcc_dirs").unwrap();
    assert!(dirs.rate.iter().all(|r| r.unverified));
}
