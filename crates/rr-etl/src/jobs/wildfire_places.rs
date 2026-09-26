//! Optional job `wildfire_places` — wildfire exposure by Census place, for the optional
//! `wildfire_places` pack (issue #15), from the USDA Forest Service *Wildfire Risk to
//! Communities* tabular download (2nd edition; the "Communities" sheet: Census places).
//!
//! Per place: the share of its buildings directly exposed (in or next to burnable vegetation:
//! flames can reach them), indirectly exposed (embers and home-to-home spread), and the national
//! percentile rank of risk to homes. Places flagged `INSUFFICIENT_DATA` keep their name but no
//! values. The pack also carries which places each ZIP code overlaps (Census 2020 ZCTA-to-place
//! relationship file: the share of the ZIP's land in each place, parts under 1% dropped), so the
//! engine can describe a ZIP without another source. ZIP land outside every place (unincorporated
//! areas) has no place value: the county picture applies there.
//!
//! Not in the quarterly refresh (`default: false`); run with `--only wildfire_places`.

use super::{Ctx, JobOutput};
use crate::csvout::{Table, col, parse_delimited};
use crate::manifest::Attribution;
use crate::num::sig;
use crate::{Result, data_err};
use std::collections::BTreeMap;

/// Wildfire exposure by place (optional pack).
pub const PLACES: &str = "opt/wildfire_places/places.csv";
/// Places each ZIP overlaps (optional pack).
pub const ZIP_PLACES: &str = "opt/wildfire_places/zip_places.csv";

const WORKBOOK: &str =
    "https://wildfirerisk.org/wp-content/uploads/2026/04/wrc_download_20260415.xlsx";
const ZCTA_PLACE: &str = "https://www2.census.gov/geo/docs/maps-data/data/rel2020/zcta520/tab20_zcta520_place20_natl.txt";
const SITE: &str = "https://wildfirerisk.org/";

/// Smallest share of a ZIP's land a place must hold to be listed for it.
pub const MIN_ZIP_SHARE: f64 = 0.01;

/// One place from the workbook.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// Seven-digit place GEOID.
    pub geoid: String,
    /// Name ("Paradise, CA").
    pub name: String,
    /// Share of buildings directly exposed.
    pub direct: Option<f64>,
    /// Share indirectly exposed.
    pub indirect: Option<f64>,
    /// National percentile rank of risk to homes (0 to 1).
    pub risk_rank: Option<f64>,
}

/// Read the Communities sheet rows (header first) into places.
pub fn parse_places(rows: &[Vec<String>]) -> Result<Vec<Place>> {
    let header = rows
        .first()
        .ok_or_else(|| data_err("Communities sheet is empty"))?;
    let ix = |n: &str| col(header, n);
    let (i_fq, i_name, i_de, i_ie, i_rank, i_ins) = (
        ix("GEOIDFQ")?,
        ix("NAME")?,
        ix("BUILDINGS_FRACTION_DE")?,
        ix("BUILDINGS_FRACTION_IE")?,
        ix("RISK_NATIONAL_RANK")?,
        ix("INSUFFICIENT_DATA")?,
    );
    let cell = |r: &Vec<String>, i: usize| r.get(i).cloned().unwrap_or_default();
    let num = |r: &Vec<String>, i: usize| {
        cell(r, i)
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
    };
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        let fq = cell(r, i_fq);
        let Some(geoid) = fq.rsplit("US").next().filter(|g| g.len() == 7) else {
            continue;
        };
        let insufficient = cell(r, i_ins).trim().eq_ignore_ascii_case("yes");
        out.push(Place {
            geoid: geoid.to_string(),
            name: cell(r, i_name).trim().to_string(),
            direct: if insufficient { None } else { num(r, i_de) },
            indirect: if insufficient { None } else { num(r, i_ie) },
            risk_rank: if insufficient { None } else { num(r, i_rank) },
        });
    }
    Ok(out)
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let wb = ctx
        .http
        .get(WORKBOOK, Some("wildfire_places/wrc_download.xlsx"))?;
    out.source(super::source_from(
        "USDA Forest Service Wildfire Risk to Communities, tabular download (Communities sheet)",
        &wb,
        "wrc_download_20260415.xlsx (2nd edition; last update 2026-04-15)",
        "USDA Forest Service data; free to use, citation requested",
        "Cite: USDA Forest Service. 2026. Wildfire Risk to Communities. https://wildfirerisk.org [date accessed].",
    ));
    let rows = crate::xlsx::read_sheet(&wb.bytes, "Communities")?;
    let places = parse_places(&rows)?;
    out.rows_in += places.len() as u64;
    if places.len() < 25_000 {
        return Err(data_err(format!(
            "Wildfire Risk to Communities: only {} places",
            places.len()
        )));
    }
    let by_id: BTreeMap<&str, &Place> = places.iter().map(|p| (p.geoid.as_str(), p)).collect();
    for (id, what, lo, hi) in [
        ("0655520", "Paradise town, CA (direct exposure)", 0.3, 1.0),
        ("3651000", "New York city, NY (direct exposure)", 0.0, 0.05),
    ] {
        let v = by_id.get(id).and_then(|p| p.direct).unwrap_or(-1.0);
        if !(lo..=hi).contains(&v) {
            return Err(data_err(format!("{what} is {v:.3}, expected {lo}-{hi}")));
        }
    }
    let mut table = Table::new(
        &[
            "place",
            "name",
            "buildings_direct",
            "buildings_indirect",
            "risk_national_rank",
        ],
        1,
    );
    for p in &places {
        table.push(vec![
            p.geoid.clone(),
            p.name.clone(),
            p.direct.map(|v| sig(v, 3)).unwrap_or_default(),
            p.indirect.map(|v| sig(v, 3)).unwrap_or_default(),
            p.risk_rank.map(|v| sig(v, 3)).unwrap_or_default(),
        ]);
    }
    out.table(ctx, PLACES, &mut table)?;

    let rel = ctx.http.get(
        ZCTA_PLACE,
        Some("wildfire_places/tab20_zcta520_place20_natl.txt"),
    )?;
    out.source(super::source_from(
        "Census 2020 ZCTA to place relationship file",
        &rel,
        "rel2020 zcta520-place20",
        super::PUBLIC_DOMAIN,
        "",
    ));
    let (h, rrows) = parse_delimited(&rel.text(), b'|')?;
    let (i_z, i_zl, i_p, i_part) = (
        col(&h, "GEOID_ZCTA5_20")?,
        col(&h, "AREALAND_ZCTA5_20")?,
        col(&h, "GEOID_PLACE_20")?,
        col(&h, "AREALAND_PART")?,
    );
    let mut zt = Table::new(&["zip", "place", "zip_land_share"], 2);
    let mut pairs = 0usize;
    for r in &rrows {
        if r[i_z].is_empty() || !by_id.contains_key(r[i_p].as_str()) {
            continue;
        }
        let (zl, part) = (
            r[i_zl].parse::<f64>().unwrap_or(0.0),
            r[i_part].parse::<f64>().unwrap_or(0.0),
        );
        if zl <= 0.0 || part / zl < MIN_ZIP_SHARE {
            continue;
        }
        zt.push(vec![
            r[i_z].clone(),
            r[i_p].clone(),
            sig((part / zl).min(1.0), 3),
        ]);
        pairs += 1;
    }
    out.rows_in += rrows.len() as u64;
    out.table(ctx, ZIP_PLACES, &mut zt)?;
    out.notes.push(format!(
        "{} Census places from the Wildfire Risk to Communities Communities sheet (2nd edition, updated 2026-04-15); INSUFFICIENT_DATA places keep their name without values. buildings_direct / buildings_indirect: BUILDINGS_FRACTION_DE / _IE (share of the place's buildings directly / indirectly exposed); risk_national_rank: RISK_NATIONAL_RANK (national percentile of risk to homes). zip_places.csv: {pairs} ZIP-place pairs from the Census 2020 ZCTA-to-place file where the place holds at least {:.0}% of the ZIP's land.",
        places.len(),
        100.0 * MIN_ZIP_SHARE
    ));
    out.definitions.insert("buildings_direct".into(), "Share of the place's buildings directly exposed to wildfire (flames can reach them from adjacent burnable vegetation), USDA Forest Service Wildfire Risk to Communities.".into());
    out.attributions.push(Attribution {
        source: "USDA Forest Service Wildfire Risk to Communities".into(),
        text: format!(
            "USDA Forest Service. 2026. Wildfire Risk to Communities. https://wildfirerisk.org (accessed {}).",
            crate::timefmt::today_utc()
        ),
        license: "USDA Forest Service data (free to use; citation requested)".into(),
        url: SITE.into(),
        version: Some("2nd edition, tabular download of 2026-04-15".into()),
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_parse_with_the_full_geoid() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let rows = vec![
            s(&[
                "GEOID",
                "GEOIDFQ",
                "NAME",
                "BUILDINGS_FRACTION_DE",
                "BUILDINGS_FRACTION_IE",
                "RISK_NATIONAL_RANK",
                "INSUFFICIENT_DATA",
            ]),
            s(&[
                "100100",
                "1600000US0100100",
                "Abanda, AL",
                "1",
                "0",
                "0.83499999999999996",
                "No",
            ]),
            s(&[
                "600001",
                "1600000US0600001",
                "Tiny, CA",
                "0.5",
                "0.5",
                "0.9",
                "Yes",
            ]),
        ];
        let p = parse_places(&rows).unwrap();
        assert_eq!(p[0].geoid, "0100100");
        assert_eq!(p[0].direct, Some(1.0));
        assert!((p[0].risk_rank.unwrap() - 0.835).abs() < 1e-9);
        assert_eq!((p[1].direct, p[1].risk_rank), (None, None));
    }
}
