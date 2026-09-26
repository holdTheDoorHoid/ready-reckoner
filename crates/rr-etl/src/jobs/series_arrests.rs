//! `core/series/fbi_arrests.toml` (job `series`): arrests per 100,000 people by sex and age band,
//! for the `arrest_or_detention` hazard (DESIGN-DELTA §1.2, owner decision 2026-09-26).
//!
//! Source: the FBI's Crime in the United States "Persons Arrested" tables, downloaded from the
//! Crime Data Explorer (a signed-URL endpoint, no key): Table 29 (the national estimate of all
//! arrests), Tables 39 and 40 (arrests of males and of females by age, from the agencies that
//! reported all twelve months, 87-92% of the population). The FBI publishes no national estimate
//! by age or sex, so each band's reported count is scaled by Table 29 over the reported total (the
//! share-allocation the Council on Criminal Justice uses) and divided by the Census Bureau's
//! national population in that band. Arrests are events, not people: one person arrested twice
//! counts twice. The latest three years are published, with the mean as the rate and the lowest and
//! highest year as its range.

use super::Ctx;
use super::JobOutput;
use crate::jobs::series::{SRate, SeriesDoc, Val};
use crate::xlsx::{Sheet, norm, read_workbook};
use crate::{Result, data_err};
use std::collections::BTreeMap;

/// Output file.
pub const FBI_ARRESTS: &str = "core/series/fbi_arrests.toml";

const INDEX: &str = "https://cde.ucr.cjis.gov/LATEST/webapp/assets/JSON/downloads/cius.json";
const SIGNED: &str = "https://cde.ucr.cjis.gov/LATEST/s3/signedurl?key=";
const CDE: &str = "https://cde.ucr.cjis.gov/LATEST/webapp/#/pages/downloads";
const POP: &str = "https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/national/asrh/nc-est2025-agesex-res.csv";

/// Age bands: (id, first age, last age, table columns summed).
pub const BANDS: &[(&str, u32, u32, &[&str])] = &[
    ("10_17", 10, 17, &["10-12", "13-14", "15", "16", "17"]),
    ("18_24", 18, 24, &["18", "19", "20", "21", "22", "23", "24"]),
    ("25_34", 25, 34, &["25-29", "30-34"]),
    ("35_44", 35, 44, &["35-39", "40-44"]),
    ("45_54", 45, 54, &["45-49", "50-54"]),
    ("55_64", 55, 64, &["55-59", "60-64"]),
    ("65_plus", 65, 200, &["65 and over"]),
    ("all", 0, 200, &["Total all ages"]),
];

/// The `TOTAL` row of an arrests-by-age table: column label -> count, and the covered
/// population from the coverage line.
pub fn by_age(sheet: &Sheet) -> Option<(BTreeMap<String, f64>, Option<f64>)> {
    let h = sheet.iter().position(|r| {
        r.first()
            .is_some_and(|c| norm(c).starts_with("Offense charged"))
    })?;
    let header: Vec<String> = sheet[h].iter().map(|c| norm(c)).collect();
    let total = sheet.iter().skip(h + 1).find(|r| {
        r.first()
            .is_some_and(|c| norm(c).eq_ignore_ascii_case("total"))
    })?;
    let mut out = BTreeMap::new();
    for (i, label) in header.iter().enumerate() {
        if let Some(v) = total
            .get(i)
            .and_then(|c| c.trim().replace(',', "").parse::<f64>().ok())
        {
            out.insert(label.clone(), v);
        }
    }
    // "[14,108 agencies; 2025 estimated population 313,186,729]"
    let covered = sheet.iter().take(h).flatten().find_map(|c| {
        let t = norm(c);
        let i = t.find("population")?;
        let digits: String = t[i..]
            .chars()
            .filter(|ch| ch.is_ascii_digit() || *ch == ',')
            .collect::<String>()
            .replace(',', "");
        digits.parse::<f64>().ok()
    });
    Some((out, covered))
}

/// The national estimate of all arrests (Table 29's first `Total` row).
pub fn national_total(sheet: &Sheet) -> Option<f64> {
    sheet.iter().find_map(|r| {
        let first = norm(r.first()?);
        if !first.to_ascii_lowercase().starts_with("total") {
            return None;
        }
        r.iter().skip(1).find_map(|c| {
            c.trim()
                .replace(',', "")
                .parse::<f64>()
                .ok()
                .filter(|v| *v > 1e5)
        })
    })
}

fn sheet_named(bytes: &[u8], member: &str) -> Result<Sheet> {
    let wb = read_workbook(&crate::http::zip_entry(bytes, member)?)?;
    wb.into_iter()
        .next()
        .map(|(_, s)| s)
        .ok_or_else(|| data_err(format!("FBI zip: {member} has no sheet")))
}

/// Build the file.
pub fn build(ctx: &Ctx, out: &mut JobOutput) -> Result<()> {
    let index = ctx.http.get(INDEX, None)?;
    let v: serde_json::Value = serde_json::from_slice(&index.bytes)?;
    let coll = v["collections"]
        .as_array()
        .and_then(|a| a.iter().find(|c| c["id"] == "persons-arrested"))
        .ok_or_else(|| data_err("FBI CDE: no persons-arrested collection in cius.json"))?;
    let mut years: Vec<(u16, String)> = coll["downloads"]
        .as_object()
        .map(|m| {
            m.iter()
                .filter_map(|(y, f)| Some((y.parse().ok()?, f.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    years.sort();
    let latest: Vec<(u16, String)> = years.iter().rev().take(3).rev().cloned().collect();
    if latest.len() < 3 {
        return Err(data_err(
            "FBI CDE: fewer than three years of persons-arrested tables",
        ));
    }
    // Census national population by single year of age and sex.
    let pop = ctx
        .http
        .get(POP, Some("series/nc-est2025-agesex-res.csv"))?;
    out.source(super::source_from(
        "Census Bureau national population by single year of age and sex, Vintage 2025",
        &pop,
        "NC-EST2025-AGESEX-RES",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let (ph, prows) = crate::csvout::parse_delimited(&pop.text(), b',')?;
    let (i_sex, i_age) = (
        crate::csvout::col(&ph, "SEX")?,
        crate::csvout::col(&ph, "AGE")?,
    );
    let pop_band = |year: u16, sex: &str, lo: u32, hi: u32| -> Option<f64> {
        let ci = crate::csvout::col(&ph, &format!("POPESTIMATE{year}")).ok()?;
        let s = if sex == "male" { "1" } else { "2" };
        let total: f64 = prows
            .iter()
            .filter(|r| r[i_sex] == s)
            .filter_map(|r| {
                let age: u32 = r[i_age].parse().ok()?;
                (age != 999 && age >= lo && age <= hi).then(|| r[ci].parse::<f64>().ok())?
            })
            .sum();
        (total > 0.0).then_some(total)
    };
    // rates[(sex, band)] -> per-year rate per 100,000
    let mut rates: BTreeMap<(String, String), Vec<(u16, f64)>> = BTreeMap::new();
    let mut doc = SeriesDoc {
        id: "fbi_arrests".into(),
        title: "Arrests per 100,000 people by sex and age, United States (FBI Crime in the United States)".into(),
        publisher: "Federal Bureau of Investigation, Uniform Crime Reporting Program (Crime Data Explorer)".into(),
        url: CDE.into(),
        licence: super::PUBLIC_DOMAIN.into(),
        source: "fbi_cde_arrests".into(),
        retrieved: index.retrieved[..10].to_string(),
        how: "fetched".into(),
        ..Default::default()
    };
    for (year, file) in &latest {
        let signed = ctx.http.get(&format!("{SIGNED}cius/{year}/{file}"), None)?;
        let sv: serde_json::Value = serde_json::from_slice(&signed.bytes)?;
        let url = sv
            .as_object()
            .and_then(|m| m.values().next())
            .and_then(|u| u.as_str())
            .ok_or_else(|| data_err(format!("FBI CDE: no signed URL for {file}")))?
            .to_string();
        let zip = ctx.http.get(&url, Some(&format!("series/{file}")))?;
        out.source(crate::manifest::SourceRecord {
            name: format!(
                "FBI Crime in the United States {year}: Persons Arrested tables (Tables 29, 39, 40)"
            ),
            url: format!("{SIGNED}cius/{year}/{file} (signed download)"),
            version: file.clone(),
            retrieved: zip.retrieved.clone(),
            sha256: zip.sha256.clone(),
            bytes: zip.bytes.len() as u64,
            license: super::PUBLIC_DOMAIN.into(),
            obligations: String::new(),
        });
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&zip.bytes))?;
        let names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
            .collect();
        let member = |t: &str| {
            names
                .iter()
                .find(|n| n.contains(t) && n.ends_with(".xlsx"))
                .cloned()
                .ok_or_else(|| data_err(format!("FBI {year}: no {t} workbook in {file}")))
        };
        let national = national_total(&sheet_named(&zip.bytes, &member("Table_29_")?)?)
            .ok_or_else(|| data_err(format!("FBI {year}: no national total in Table 29")))?;
        let (m, cov_m) = by_age(&sheet_named(&zip.bytes, &member("Table_39_")?)?)
            .ok_or_else(|| data_err(format!("FBI {year}: Table 39 layout changed")))?;
        let (f, _) = by_age(&sheet_named(&zip.bytes, &member("Table_40_")?)?)
            .ok_or_else(|| data_err(format!("FBI {year}: Table 40 layout changed")))?;
        let reported = m.get("Total all ages").copied().unwrap_or(0.0)
            + f.get("Total all ages").copied().unwrap_or(0.0);
        if reported <= 0.0 {
            return Err(data_err(format!("FBI {year}: no reported total")));
        }
        let scale = national / reported;
        doc.rows.push(vec![
            ("table".into(), Val::S("year".into())),
            ("year".into(), Val::I(i64::from(*year))),
            ("national_estimate".into(), Val::N(national)),
            ("reported_by_age_tables".into(), Val::N(reported)),
            (
                "population_covered".into(),
                Val::N(cov_m.unwrap_or(f64::NAN)),
            ),
        ]);
        for (sex, t) in [("male", &m), ("female", &f)] {
            for (band, lo, hi, cols) in BANDS {
                let count: f64 = cols.iter().filter_map(|c| t.get(*c)).sum();
                let (plo, phi) = if *band == "all" { (0, 200) } else { (*lo, *hi) };
                let Some(p) = pop_band(*year, sex, plo, phi) else {
                    continue;
                };
                let rate = count * scale / p * 100_000.0;
                rates
                    .entry((sex.to_string(), band.to_string()))
                    .or_default()
                    .push((*year, rate));
                doc.rows.push(vec![
                    ("table".into(), Val::S("band".into())),
                    ("year".into(), Val::I(i64::from(*year))),
                    ("sex".into(), Val::S(sex.into())),
                    ("band".into(), Val::S(band.to_string())),
                    ("reported_arrests".into(), Val::N(count)),
                    ("estimated_arrests".into(), Val::N(count * scale)),
                    ("population".into(), Val::N(p)),
                    ("per_100k".into(), Val::N(rate)),
                ]);
            }
        }
        out.rows_in += 1;
    }
    let (y0, y1) = (latest[0].0, latest[latest.len() - 1].0);
    for ((sex, band), v) in &rates {
        let mean = v.iter().map(|x| x.1).sum::<f64>() / v.len() as f64;
        let lo = v.iter().map(|x| x.1).fold(f64::MAX, f64::min);
        let hi = v.iter().map(|x| x.1).fold(0.0, f64::max);
        let who = if band == "all" {
            format!("{sex}s of all ages")
        } else {
            format!(
                "{sex}s aged {}",
                band.replace('_', " to ").replace(" to plus", " and over")
            )
        };
        doc.rates.push(SRate {
            id: format!("arrests_per_100k_{sex}_{band}"),
            value: mean,
            unit: format!("arrests per 100,000 {who} per year"),
            low: Some(lo),
            high: Some(hi),
            year: y1,
            period: format!("{y0}-{y1}"),
            figure: v.iter().map(|(y, r)| format!("{y}: {r:.0}")).collect::<Vec<_>>().join("; "),
            derivation: "band count in Tables 39/40 x (Table 29 national estimate / Tables 39+40 total) / Census population in the band x 100,000; mean of the years, range = lowest and highest year".into(),
            note: "Counts arrests, not people or convictions.".into(),
            unverified: false,
        });
    }
    // Guard against a layout change: men of all ages are arrested about 3,300 times per 100,000
    // a year and women about 1,200 (2023-2025).
    let all = |sex: &str| {
        rates
            .get(&(sex.to_string(), "all".to_string()))
            .and_then(|v| v.last())
            .map(|x| x.1)
    };
    match (all("male"), all("female")) {
        (Some(m), Some(f)) if (2_000.0..5_000.0).contains(&m) && (600.0..2_000.0).contains(&f) => {}
        other => {
            return Err(data_err(format!(
                "FBI arrests: implausible all-ages rates {other:?} (about 3,300 and 1,200 per 100,000 expected)"
            )));
        }
    }
    doc.notes.push("Tables 39 and 40 count arrests reported by agencies that sent all twelve months of data (87-92% of the population in 2023-2025); Table 29 estimates all arrests nationally. National figures by band are the band's reported share of the Table 29 estimate, a derivation: the FBI publishes no national estimate by age or sex.".into());
    doc.notes.push("Arrests are events: one person arrested twice counts twice. The 10 to 17 band leaves out children under 10 (almost no arrests), so it is the better juvenile rate.".into());
    doc.notes.push(format!("Population: {POP}."));
    doc.write(ctx, out, FBI_ARRESTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn arrests_by_age_table_reads_the_total_row() {
        let sheet: Sheet = vec![
            row(&["Table 39"]),
            row(&["Arrests, Males, by Age, 2025"]),
            row(&["[14,108 agencies; 2025 estimated population 313,186,729]"]),
            row(&[
                "Offense charged",
                "Total all\nages",
                "Ages under  15",
                "Under 10",
                "10-12",
                "18",
                "65 and over",
            ]),
            row(&["TOTAL", "4997901", "100", "1166", "5000", "90000", "145120"]),
            row(&["Total percent distribution1", "100.0", "", "", "", "", ""]),
        ];
        let (t, cov) = by_age(&sheet).unwrap();
        assert_eq!(t["Total all ages"], 4_997_901.0);
        assert_eq!(t["65 and over"], 145_120.0);
        assert_eq!(t["Ages under 15"], 100.0);
        assert_eq!(cov, Some(313_186_729.0));
        let t29: Sheet = vec![
            row(&["Table 29"]),
            row(&["Offense charged", "United States"]),
            row(&["Total1", "7,570,249"]),
        ];
        assert_eq!(national_total(&t29), Some(7_570_249.0));
    }

    #[test]
    fn bands_cover_ages_10_and_over_once() {
        let mut ages = vec![0u32; 101];
        for (id, lo, hi, _) in BANDS.iter().filter(|b| b.0 != "all") {
            for a in *lo..=(*hi).min(100) {
                ages[a as usize] += 1;
                assert!(!id.is_empty());
            }
        }
        assert!(ages[10..].iter().all(|n| *n == 1));
    }
}
