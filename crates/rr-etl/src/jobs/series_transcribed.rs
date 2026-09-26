//! Hand-copied national series (job `series`): federal funding gaps (CRS RS20348) and FCC
//! Disaster Information Reporting System cell-site outages. Both hosts refuse scripted clients
//! for their pages, so the figures were transcribed on 2026-09-26 (research report
//! `round2/phase2/research/data-model-series-research.md` §2 and §4) with the document versions
//! and ids recorded here; a refresh re-computes the arithmetic from them.

use super::{Ctx, JobOutput};
use crate::Result;
use crate::jobs::series::{SRate, SeriesDoc, Val, poisson90};
use crate::manifest::SourceRecord;

/// Funding gaps file.
pub const FUNDING_GAPS: &str = "core/series/funding_gaps.toml";
/// FCC DIRS file.
pub const FCC_DIRS: &str = "core/series/fcc_dirs.toml";
/// When the figures were transcribed.
pub const TRANSCRIBED: &str = "2026-09-26";

const CRS_PDF: &str = "https://www.everycrsreport.com/files/2026-05-26_RS20348_07052cf62f659c4b684f1ad762c81b435dedb6e4.pdf";
const CRS_PAGE: &str = "https://www.congress.gov/crs-product/RS20348";
const HOUSE: &str = "https://history.house.gov/Institution/Shutdown/Government-Shutdowns/";

/// CRS RS20348 Table 1 (version of 2026-05-26): fiscal year, first full day, day funding was
/// enacted, full days, furloughs ("yes", "limited", "no"), scope.
pub const GAPS: &[(u16, &str, &str, u16, &str, &str)] = &[
    (1977, "1976-10-01", "1976-10-11", 10, "no", "partial"),
    (
        1978,
        "1977-10-01",
        "1977-10-13",
        12,
        "no",
        "partial (Labor-HEW)",
    ),
    (
        1978,
        "1977-11-01",
        "1977-11-09",
        8,
        "no",
        "partial (Labor-HEW)",
    ),
    (
        1978,
        "1977-12-01",
        "1977-12-09",
        8,
        "no",
        "partial (Labor-HEW)",
    ),
    (1979, "1978-10-01", "1978-10-18", 17, "no", "partial"),
    (1980, "1979-10-01", "1979-10-12", 11, "no", "partial"),
    (1982, "1981-11-21", "1981-11-23", 2, "yes", "partial"),
    (1983, "1982-10-01", "1982-10-02", 1, "limited", "partial"),
    (1983, "1982-12-18", "1982-12-21", 3, "no", "partial"),
    (1984, "1983-11-11", "1983-11-14", 3, "no", "partial"),
    (1985, "1984-10-01", "1984-10-03", 2, "no", "partial"),
    (1985, "1984-10-04", "1984-10-05", 1, "limited", "partial"),
    (1987, "1986-10-17", "1986-10-18", 1, "limited", "full"),
    (1988, "1987-12-19", "1987-12-20", 1, "no", "partial"),
    (1991, "1990-10-06", "1990-10-09", 3, "limited", "full"),
    (1996, "1995-11-14", "1995-11-19", 5, "yes", "partial"),
    (1996, "1995-12-16", "1996-01-06", 21, "yes", "partial"),
    (2014, "2013-10-01", "2013-10-17", 16, "yes", "full"),
    (2018, "2018-01-20", "2018-01-22", 2, "yes", "full"),
    (2019, "2018-12-22", "2019-01-25", 34, "yes", "partial"),
    (2026, "2025-10-01", "2025-11-12", 42, "yes", "full"),
    (2026, "2026-01-31", "2026-02-03", 3, "yes", "partial"),
    (
        2026,
        "2026-02-14",
        "2026-04-30",
        75,
        "limited",
        "Homeland Security only",
    ),
];

/// Fiscal years since the Civiletti opinions (1980-1981) made a lapse mean a shutdown.
pub const POST_CIVILETTI: (u16, u16) = (1982, 2026);

/// One FCC DIRS activation.
#[derive(Debug, Clone, Copy)]
pub struct DirsEvent {
    /// Storm and year.
    pub event: &'static str,
    /// States in the reporting area.
    pub states: &'static str,
    /// First daily report.
    pub first: &'static str,
    /// Last daily report.
    pub last: &'static str,
    /// First report's document id.
    pub first_doc: &'static str,
    /// Last report's document id.
    pub last_doc: &'static str,
    /// Peak share of cell sites out across the reporting area.
    pub peak: f64,
    /// Hardest-hit county with 20 or more sites.
    pub worst: &'static str,
    /// Its peak share out.
    pub worst_share: f64,
    /// Days from the first report until the area-wide share first fell under 5%.
    pub days_first_under: u16,
    /// Days until it stayed under 5% (`None`: never, by the last report).
    pub days_stayed_under: Option<u16>,
}

/// FCC DIRS activations for eight hurricanes, 2017-2024.
pub const DIRS: &[DirsEvent] = &[
    DirsEvent {
        event: "Hurricane Harvey 2017",
        states: "TX, LA",
        first: "2017-08-26",
        last: "2017-09-05",
        first_doc: "DOC-346368A1",
        last_doc: "DOC-346499A1",
        peak: 0.047,
        worst: "Calhoun TX",
        worst_share: 0.852,
        days_first_under: 0,
        days_stayed_under: Some(0),
    },
    DirsEvent {
        event: "Hurricane Maria 2017, Puerto Rico",
        states: "PR",
        first: "2017-09-21",
        last: "2018-03-21",
        first_doc: "DOC-346840A1",
        last_doc: "DOC-349827A1",
        peak: 0.956,
        worst: "48 of 78 municipios",
        worst_share: 1.0,
        days_first_under: 155,
        days_stayed_under: Some(155),
    },
    DirsEvent {
        event: "Hurricane Maria 2017, US Virgin Islands",
        states: "VI",
        first: "2017-09-21",
        last: "2018-03-21",
        first_doc: "DOC-346840A1",
        last_doc: "DOC-349827A1",
        peak: 0.766,
        worst: "all three islands",
        worst_share: 0.6,
        days_first_under: 182,
        days_stayed_under: None,
    },
    DirsEvent {
        event: "Hurricane Michael 2018",
        states: "FL, GA, AL",
        first: "2018-10-11",
        last: "2018-10-26",
        first_doc: "DOC-354510A1",
        last_doc: "DOC-354814A1",
        peak: 0.188,
        worst: "Bay FL",
        worst_share: 0.783,
        days_first_under: 5,
        days_stayed_under: Some(12),
    },
    DirsEvent {
        event: "Hurricane Ida 2021",
        states: "LA, MS",
        first: "2021-08-30",
        last: "2021-09-20",
        first_doc: "DOC-375318A1",
        last_doc: "DOC-375878A1",
        peak: 0.281,
        worst: "Terrebonne LA",
        worst_share: 1.0,
        days_first_under: 8,
        days_stayed_under: Some(8),
    },
    DirsEvent {
        event: "Hurricane Ian 2022",
        states: "FL, GA, SC, NC",
        first: "2022-09-28",
        last: "2022-10-10",
        first_doc: "DOC-387679A1",
        last_doc: "DOC-388051A1",
        peak: 0.109,
        worst: "Glades FL",
        worst_share: 0.828,
        days_first_under: 3,
        days_stayed_under: Some(3),
    },
    DirsEvent {
        event: "Hurricane Idalia 2023",
        states: "FL, GA, SC",
        first: "2023-08-30",
        last: "2023-09-04",
        first_doc: "DOC-396514A1",
        last_doc: "DOC-396621A1",
        peak: 0.019,
        worst: "Hamilton FL",
        worst_share: 0.591,
        days_first_under: 0,
        days_stayed_under: Some(3),
    },
    DirsEvent {
        event: "Hurricane Helene 2024",
        states: "FL, GA, NC, SC, TN, VA",
        first: "2024-09-26",
        last: "2024-10-19",
        first_doc: "DOC-405827A1",
        last_doc: "DOC-406771A1",
        peak: 0.115,
        worst: "Tazewell VA",
        worst_share: 0.977,
        days_first_under: 10,
        days_stayed_under: Some(21),
    },
    DirsEvent {
        event: "Hurricane Milton 2024",
        states: "FL",
        first: "2024-10-10",
        last: "2024-10-14",
        first_doc: "DOC-406468A1",
        last_doc: "DOC-406535A1",
        peak: 0.123,
        worst: "Hardee FL",
        worst_share: 0.512,
        days_first_under: 3,
        days_stayed_under: Some(3),
    },
];

/// Backtest counties in the same reports: county, event, peak share of cell sites out, days from
/// the first report until the share stayed under 5% (None: never over 5%, or never under by the
/// last report).
pub const DIRS_COUNTIES: &[(&str, &str, &str, f64, Option<u16>)] = &[
    (
        "37021",
        "Buncombe NC",
        "Hurricane Helene 2024",
        0.819,
        Some(21),
    ),
    (
        "22051",
        "Jefferson LA",
        "Hurricane Ida 2021",
        0.699,
        Some(11),
    ),
    (
        "48201",
        "Harris TX",
        "Hurricane Harvey 2017",
        0.051,
        Some(4),
    ),
    (
        "72127",
        "San Juan PR",
        "Hurricane Maria 2017",
        0.913,
        Some(123),
    ),
    ("12005", "Bay FL", "Hurricane Michael 2018", 0.783, None),
];

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

fn transcribed_source(name: &str, url: &str, version: &str) -> SourceRecord {
    SourceRecord {
        name: name.into(),
        url: url.into(),
        version: version.into(),
        retrieved: format!("{TRANSCRIBED}T00:00:00Z"),
        sha256: String::new(),
        bytes: 0,
        license: super::PUBLIC_DOMAIN.into(),
        obligations: String::new(),
    }
}

/// Write both files.
pub fn build(ctx: &Ctx, out: &mut JobOutput) -> Result<()> {
    // --- Funding gaps -------------------------------------------------------------------------
    let mut doc = SeriesDoc {
        id: "funding_gaps".into(),
        title: "Federal funding gaps and shutdowns since fiscal year 1977 (CRS RS20348)".into(),
        publisher: "Congressional Research Service (J. V. Saturno), with the House Historian's shutdown column".into(),
        url: CRS_PAGE.into(),
        licence: "CRS reports are works of the US Government, not subject to copyright".into(),
        source: "crs_rs20348_funding_gaps".into(),
        retrieved: TRANSCRIBED.into(),
        how: "transcribed".into(),
        ..Default::default()
    };
    doc.notes.push(format!("Table 1 of CRS RS20348, version 48 (updated 2026-05-26), read from {CRS_PDF} (congress.gov refuses scripted clients); the furlough column from the House Historian's table ({HOUSE}). Full days as CRS counts them: days with no budget authority, from the day after authority lapsed to the day before new funding was enacted."));
    doc.notes.push("Before the Attorney General's opinions of 1980 and 1981 (Civiletti), agencies kept working through lapses; rates use fiscal years 1982-2026. The overnight lapse of February 2018 is not in the CRS table.".into());
    let (a, z) = POST_CIVILETTI;
    let years = f64::from(z - a + 1);
    let long_years: std::collections::BTreeSet<u16> = GAPS
        .iter()
        .filter(|g| g.0 >= a && g.3 >= 14)
        .map(|g| g.0)
        .collect();
    let furlough_gaps = GAPS.iter().filter(|g| g.0 >= a && g.4 == "yes").count() as u64;
    let n_long = long_years.len() as u64;
    let (lo, hi) = poisson90(n_long);
    doc.rates.push(SRate {
        id: "funding_gap_ge_14d_per_year".into(),
        value: n_long as f64 / years,
        unit: "chance a fiscal year has a federal funding gap of 14 full days or more".into(),
        low: Some(lo / years),
        high: Some(hi / years),
        year: z,
        period: format!("FY{a}-FY{z}"),
        figure: format!("{n_long} of {years:.0} fiscal years (FY{})", long_years.iter().map(|y| y.to_string()).collect::<Vec<_>>().join(", FY")),
        derivation: format!("{n_long} / {years:.0}; range = exact Poisson 90% interval on {n_long} events"),
        note: "Long lapses delay pay for federal workers and contractors and can delay benefits run by the lapsed agencies.".into(),
        unverified: false,
    });
    let (lo, hi) = poisson90(furlough_gaps);
    doc.rates.push(SRate {
        id: "shutdown_with_furloughs_per_year".into(),
        value: furlough_gaps as f64 / years,
        unit: "funding gaps with widespread furloughs per year".into(),
        low: Some(lo / years),
        high: Some(hi / years),
        year: z,
        period: format!("FY{a}-FY{z}"),
        figure: format!("{furlough_gaps} gaps with widespread furloughs in {years:.0} fiscal years"),
        derivation: format!("{furlough_gaps} / {years:.0}; range = exact Poisson 90% interval"),
        note: "Limited furloughs (a half day, a weekend of closed parks, one department) are not counted.".into(),
        unverified: false,
    });
    for g in GAPS {
        doc.rows.push(vec![
            ("fiscal_year".into(), Val::I(i64::from(g.0))),
            ("first_day".into(), Val::S(g.1.into())),
            ("funding_enacted".into(), Val::S(g.2.into())),
            ("full_days".into(), Val::I(i64::from(g.3))),
            ("furloughs".into(), Val::S(g.4.into())),
            ("scope".into(), Val::S(g.5.into())),
        ]);
    }
    doc.write(ctx, out, FUNDING_GAPS)?;
    out.source(transcribed_source(
        "CRS RS20348, Federal Funding Gaps: A Brief Overview (Table 1)",
        CRS_PDF,
        "version 48, updated 2026-05-26; figures transcribed",
    ));
    out.source(transcribed_source(
        "US House of Representatives, Office of the Historian: Funding Gaps and Shutdowns in the Federal Government",
        HOUSE,
        "shutdown-procedures column transcribed",
    ));

    // --- FCC DIRS --------------------------------------------------------------------------------
    let mut doc = SeriesDoc {
        id: "fcc_dirs".into(),
        title: "Cell sites out of service after major hurricanes (FCC Disaster Information Reporting System)".into(),
        publisher: "Federal Communications Commission, Public Safety and Homeland Security Bureau".into(),
        url: "https://docs.fcc.gov/public/attachments/DOC-353805A1.pdf".into(),
        licence: super::PUBLIC_DOMAIN.into(),
        source: "fcc_dirs_reports".into(),
        retrieved: TRANSCRIBED.into(),
        how: "transcribed".into(),
        ..Default::default()
    };
    doc.notes.push("Daily Communications Status Reports (docs.fcc.gov/public/attachments/<DOC id>.pdf) read from their per-county tables; shares are cell sites out / cell sites served in the reporting area. The FCC widens and later narrows that area, which moves the area-wide share; the worst county counts only counties with 20 or more sites. Site outages include sites down for power or backhaul and do not map one to one onto lost service.".into());
    doc.notes.push("Transcribed by a research agent with pdftotext and a per-county parse, checked against the reports' headline sentence (the FCC's own headline is wrong on Helene 2024-09-28 and 10-01; the tables are used). Not re-read line by line here, so the rates carry unverified = true.".into());
    let peaks: Vec<f64> = DIRS
        .iter()
        .filter(|e| !e.event.contains("Virgin"))
        .map(|e| e.peak)
        .collect();
    let worst: Vec<f64> = DIRS
        .iter()
        .filter(|e| !e.event.contains("Virgin"))
        .map(|e| e.worst_share)
        .collect();
    let stayed: Vec<f64> = DIRS
        .iter()
        .filter_map(|e| e.days_stayed_under.map(f64::from))
        .collect();
    doc.rates.push(SRate {
        id: "cell_sites_out_peak_area_share_median".into(),
        value: median(peaks.clone()),
        unit: "median peak share of cell sites out across a hurricane's reporting area".into(),
        low: peaks.iter().cloned().reduce(f64::min),
        high: peaks.iter().cloned().reduce(f64::max),
        year: 2024,
        period: "8 hurricanes, 2017-2024".into(),
        figure: DIRS
            .iter()
            .filter(|e| !e.event.contains("Virgin"))
            .map(|e| format!("{} {:.1}%", e.event, e.peak * 100.0))
            .collect::<Vec<_>>()
            .join("; "),
        derivation: "median of the events' peak area-wide shares; range = lowest and highest"
            .into(),
        note: String::new(),
        unverified: true,
    });
    doc.rates.push(SRate {
        id: "cell_sites_out_worst_county_share_median".into(),
        value: median(worst.clone()),
        unit: "median peak share of cell sites out in the hardest-hit county (20 or more sites)"
            .into(),
        low: worst.iter().cloned().reduce(f64::min),
        high: worst.iter().cloned().reduce(f64::max),
        year: 2024,
        period: "8 hurricanes, 2017-2024".into(),
        figure: DIRS
            .iter()
            .filter(|e| !e.event.contains("Virgin"))
            .map(|e| format!("{} {:.1}%", e.worst, e.worst_share * 100.0))
            .collect::<Vec<_>>()
            .join("; "),
        derivation: "median over events".into(),
        note: String::new(),
        unverified: true,
    });
    doc.rates.push(SRate {
        id: "cell_sites_days_until_under_5pct_median".into(),
        value: median(stayed.clone()),
        unit: "median days from the first report until the area-wide share of cell sites out stayed under 5%".into(),
        low: stayed.iter().cloned().reduce(f64::min),
        high: stayed.iter().cloned().reduce(f64::max),
        year: 2024,
        period: "8 hurricanes, 2017-2024 (Maria in the Virgin Islands never fell under 5%)".into(),
        figure: DIRS.iter().filter_map(|e| e.days_stayed_under.map(|d| format!("{} {d}", e.event))).collect::<Vec<_>>().join("; "),
        derivation: "median over events".into(),
        note: "The hardest-hit counties take longer: Buncombe NC 21 days after Helene, San Juan PR about 4 months after Maria.".into(),
        unverified: true,
    });
    for e in DIRS {
        doc.rows.push(vec![
            ("table".into(), Val::S("event".into())),
            ("event".into(), Val::S(e.event.into())),
            ("states".into(), Val::S(e.states.into())),
            ("first_report".into(), Val::S(e.first.into())),
            ("last_report".into(), Val::S(e.last.into())),
            ("first_doc".into(), Val::S(e.first_doc.into())),
            ("last_doc".into(), Val::S(e.last_doc.into())),
            ("peak_area_share".into(), Val::N(e.peak)),
            ("worst_county".into(), Val::S(e.worst.into())),
            ("worst_county_share".into(), Val::N(e.worst_share)),
            (
                "days_first_under_5pct".into(),
                Val::I(i64::from(e.days_first_under)),
            ),
            (
                "days_stayed_under_5pct".into(),
                e.days_stayed_under
                    .map_or(Val::S("never".into()), |d| Val::I(i64::from(d))),
            ),
        ]);
    }
    for c in DIRS_COUNTIES {
        doc.rows.push(vec![
            ("table".into(), Val::S("county".into())),
            ("fips".into(), Val::S(c.0.into())),
            ("county".into(), Val::S(c.1.into())),
            ("event".into(), Val::S(c.2.into())),
            ("peak_share".into(), Val::N(c.3)),
            (
                "days_stayed_under_5pct".into(),
                c.4.map_or(Val::S("not by the last report".into()), |d| {
                    Val::I(i64::from(d))
                }),
            ),
        ]);
    }
    doc.write(ctx, out, FCC_DIRS)?;
    out.source(transcribed_source(
        "FCC Communications Status Reports (DIRS), daily, eight hurricanes 2017-2024",
        "https://docs.fcc.gov/public/attachments/",
        "DOC ids per event in core/series/fcc_dirs.toml; figures transcribed",
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funding_gap_table_matches_crs() {
        assert_eq!(GAPS.len(), 23);
        // The 2025 lapse: 42 full days, ended by P.L. 119-37 on 12 November.
        let g = GAPS.iter().find(|g| g.1 == "2025-10-01").unwrap();
        assert_eq!((g.2, g.3), ("2025-11-12", 42));
        // 14 days or more after 1981: FY1996, FY2014, FY2019, FY2026.
        let long: std::collections::BTreeSet<u16> = GAPS
            .iter()
            .filter(|g| g.0 >= 1982 && g.3 >= 14)
            .map(|g| g.0)
            .collect();
        assert_eq!(
            long.into_iter().collect::<Vec<_>>(),
            vec![1996, 2014, 2019, 2026]
        );
    }

    #[test]
    fn dirs_medians() {
        let peaks: Vec<f64> = DIRS
            .iter()
            .filter(|e| !e.event.contains("Virgin"))
            .map(|e| e.peak)
            .collect();
        assert_eq!(peaks.len(), 8);
        assert!((median(peaks) - 0.119).abs() < 1e-9);
    }
}
