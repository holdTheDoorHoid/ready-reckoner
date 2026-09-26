//! Job `reliability` — utility reliability indices (EIA-861, 2015-2024) mapped to counties, as a
//! cross-check on the EAGLE-I outage record (model review M-02; data audit §10.1).
//!
//! EIA-861 reports each utility's SAIDI (minutes without power per customer per year) and SAIFI
//! (interruptions per customer per year), with and without major event days, under the IEEE 1366
//! standard or another method. Its service-territory file lists the counties each utility serves,
//! by name, with no customer split. Per year, a county's value is the mean over the utilities
//! serving it, each weighted by its customers divided evenly over the counties it serves; the
//! published value is the mean over the years with data.
//!
//! The job also compares the result with `core/outages.csv`: EAGLE-I counts only outages in
//! events that reach 1% of a county's customers, so its outages per customer-year should sit below
//! SAIFI; a county far above it points at reporting flicker (M-02 proposed flagging more than
//! three times SAIFI).

use super::{Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::{Table, col, read_table};
use crate::ct::Crosswalk;
use crate::num::sig;
use crate::xlsx::{Sheet, norm, read_workbook};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Output file.
pub const RELIABILITY: &str = "core/reliability.csv";

const PAGE: &str = "https://www.eia.gov/electricity/data/eia861/";
/// Years of EIA-861 reliability data used (the 2014 service-territory file is in the old binary
/// Excel format, which the reader does not handle).
pub const YEARS: (u16, u16) = (2015, 2024);

/// Territory names the county matcher cannot place by itself: (state, EIA name, FIPS codes).
const OVERRIDES: &[(&str, &str, &[&str])] = &[
    ("AK", "valdez cordova", &["02063", "02066"]),
    ("AK", "wrangell petersburg", &["02275", "02195"]),
    ("AK", "prince of wales ketchikan", &["02198"]),
    ("AK", "prince of wales outer ketchikan", &["02198"]),
    ("AK", "skagway hoonah angoon", &["02230", "02105"]),
    ("AK", "wade hampton", &["02158"]),
    ("SD", "shannon", &["46102"]),
    ("VA", "bedford city", &["51019"]),
    ("VA", "clifton forge city", &["51005"]),
    ("VA", "south boston city", &["51083"]),
];

/// Fold the accented letters county names use (Añasco, Doña Ana, Bayamón) to plain ASCII.
fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'Á' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'Ó' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' => 'u',
            'ñ' | 'Ñ' => 'n',
            _ => c,
        })
        .collect()
}

/// Matching key: the normalised name without spaces, so "DeWitt" meets "De Witt".
pub fn compact(s: &str) -> String {
    s.replace(' ', "")
}

/// Normalise a county name for matching.
pub fn norm_county(s: &str) -> String {
    let t = fold(s)
        .to_ascii_lowercase()
        .replace('&', " and ")
        .replace(['.', '\''], "")
        .replace(['-', ','], " ");
    let t = format!(" {t} ")
        .replace(" saint ", " st ")
        .replace(" ste ", " sainte ");
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_kind(s: &str) -> String {
    let mut t = s.to_string();
    for suffix in [
        " city and borough",
        " census area",
        " municipality",
        " municipio",
        " borough",
        " county",
        " parish",
    ] {
        if let Some(x) = t.strip_suffix(suffix) {
            t = x.to_string();
            break;
        }
    }
    t
}

/// One utility-state reliability row.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UtilityRow {
    /// EIA utility number.
    pub utility: String,
    /// State.
    pub state: String,
    /// SAIDI with major event days, minutes.
    pub saidi_with: Option<f64>,
    /// SAIFI with major event days.
    pub saifi_with: Option<f64>,
    /// SAIDI without major event days.
    pub saidi_without: Option<f64>,
    /// SAIFI without major event days.
    pub saifi_without: Option<f64>,
    /// Customers.
    pub customers: Option<f64>,
}

fn num_cell(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    if t.is_empty() || t == "." {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

/// Reliability rows from one sheet: finds the header row (the one naming "Utility Number"),
/// forward-fills the standard (IEEE / Other) and event-set labels above it, and takes IEEE values
/// where the utility reports them, otherwise the other standard's.
pub fn reliability_rows(sheet: &Sheet) -> Vec<UtilityRow> {
    let Some(h) = sheet.iter().take(6).position(|r| {
        r.iter()
            .any(|c| norm(c).eq_ignore_ascii_case("utility number"))
    }) else {
        return Vec::new();
    };
    let width = sheet[h].len();
    let fill = |row: Option<&Vec<String>>| -> Vec<String> {
        let mut out = vec![String::new(); width];
        let mut cur = String::new();
        if let Some(r) = row {
            for (i, o) in out.iter_mut().enumerate() {
                let c = norm(r.get(i).map(String::as_str).unwrap_or(""));
                if !c.is_empty() {
                    cur = c;
                }
                *o = cur.clone();
            }
        }
        out
    };
    // The standard (IEEE / Other) is in a row above the field names; the event set is either in
    // a row of its own ("All Events (With Major Event Days)", 2021 on) or in the field name
    // ("SAIDI With MED", 2015-2020).
    let std_idx = (0..h).rev().find(|&k| {
        sheet[k]
            .iter()
            .any(|c| norm(c).to_ascii_lowercase().starts_with("ieee"))
    });
    let std_row = fill(std_idx.and_then(|k| sheet.get(k)));
    let set_row = match std_idx {
        Some(k) if k + 1 < h => fill(sheet.get(h - 1)),
        _ => vec![String::new(); width],
    };
    let fields: Vec<String> = sheet[h]
        .iter()
        .map(|c| norm(c).to_ascii_lowercase())
        .collect();
    let find = |standard: &str, with_med: Option<bool>, field: &str| -> Option<usize> {
        (0..width).find(|&i| {
            let f = &fields[i];
            let set = set_row[i].to_ascii_lowercase();
            if !std_row[i].to_ascii_lowercase().starts_with(standard) || !f.starts_with(field) {
                return false;
            }
            if f.contains("minus los") || set.starts_with("loss of supply") {
                return false;
            }
            match with_med {
                None => true,
                Some(true) => set.starts_with("all events") || f.contains(" with med"),
                Some(false) => set.starts_with("without major") || f.contains("without med"),
            }
        })
    };
    let i_u = fields.iter().position(|f| f == "utility number");
    let i_s = fields.iter().position(|f| f == "state");
    let (Some(i_u), Some(i_s)) = (i_u, i_s) else {
        return Vec::new();
    };
    let cols = |standard: &str| {
        (
            find(standard, Some(true), "saidi"),
            find(standard, Some(true), "saifi"),
            find(standard, Some(false), "saidi"),
            find(standard, Some(false), "saifi"),
            find(standard, None, "number of customers"),
        )
    };
    let ieee = cols("ieee");
    let other = cols("other");
    let mut out = Vec::new();
    for r in sheet.iter().skip(h + 1) {
        let get = |i: Option<usize>| i.and_then(|i| r.get(i)).and_then(|c| num_cell(c));
        let u = r.get(i_u).map(|c| norm(c)).unwrap_or_default();
        let s = r.get(i_s).map(|c| norm(c)).unwrap_or_default();
        if u.is_empty() || s.len() != 2 {
            continue;
        }
        let pick = if get(ieee.0).is_some() { ieee } else { other };
        let row = UtilityRow {
            utility: u.trim_end_matches(".0").to_string(),
            state: s,
            saidi_with: get(pick.0),
            saifi_with: get(pick.1),
            saidi_without: get(pick.2),
            saifi_without: get(pick.3),
            customers: get(pick.4).or_else(|| get(ieee.4)).or_else(|| get(other.4)),
        };
        if row.saidi_with.is_some() || row.saifi_with.is_some() {
            out.push(row);
        }
    }
    out
}

/// (utility, state, county name) rows of a service-territory sheet.
pub fn territory_rows(sheet: &Sheet) -> Vec<(String, String, String)> {
    let Some(h) = sheet.iter().take(4).position(|r| {
        r.iter()
            .any(|c| norm(c).eq_ignore_ascii_case("utility number"))
    }) else {
        return Vec::new();
    };
    let fields: Vec<String> = sheet[h]
        .iter()
        .map(|c| norm(c).to_ascii_lowercase())
        .collect();
    let (Some(i_u), Some(i_s), Some(i_c)) = (
        fields.iter().position(|f| f == "utility number"),
        fields.iter().position(|f| f == "state"),
        fields.iter().position(|f| f == "county"),
    ) else {
        return Vec::new();
    };
    sheet
        .iter()
        .skip(h + 1)
        .filter_map(|r| {
            let u = norm(r.get(i_u)?).trim_end_matches(".0").to_string();
            let s = norm(r.get(i_s)?);
            let c = norm(r.get(i_c)?);
            (!u.is_empty() && s.len() == 2 && !c.is_empty()).then_some((u, s, c))
        })
        .collect()
}

fn find_links(html: &str) -> BTreeMap<u16, String> {
    let mut out = BTreeMap::new();
    let mut rest = html;
    while let Some(i) = rest.find("href=\"") {
        rest = &rest[i + 6..];
        let Some(j) = rest.find('"') else { break };
        let href = &rest[..j];
        rest = &rest[j..];
        let name = href.rsplit('/').next().unwrap_or("");
        if let Some(y) = name
            .strip_prefix("f861")
            .and_then(|x| x.strip_suffix(".zip"))
            .and_then(|x| x.parse::<u16>().ok())
        {
            out.insert(y, format!("{PAGE}{href}"));
        }
    }
    out
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let cw = Crosswalk::load(&ctx.data)?;
    let (h, rows) = read_table(&ctx.data, super::geography::COUNTIES)?;
    let (i_f, i_nf, i_n, i_s) = (
        col(&h, "fips")?,
        col(&h, "name_full")?,
        col(&h, "name")?,
        col(&h, "state_abbr")?,
    );
    // (state, normalised name) -> FIPS, full names first so "Fairfax city" and "Fairfax County"
    // stay apart.
    let mut by_full: HashMap<(String, String), String> = HashMap::new();
    let mut by_short: HashMap<(String, String), Vec<String>> = HashMap::new();
    for r in &rows {
        let full = norm_county(&r[i_nf]);
        by_full.insert((r[i_s].clone(), compact(&full)), r[i_f].clone());
        by_full
            .entry((r[i_s].clone(), compact(&strip_kind(&full))))
            .or_insert_with(|| r[i_f].clone());
        by_short
            .entry((r[i_s].clone(), compact(&norm_county(&r[i_n]))))
            .or_default()
            .push(r[i_f].clone());
    }
    const OLD_CT: &[(&str, &str)] = &[
        ("fairfield", "09001"),
        ("hartford", "09003"),
        ("litchfield", "09005"),
        ("middlesex", "09007"),
        ("new haven", "09009"),
        ("new london", "09011"),
        ("tolland", "09013"),
        ("windham", "09015"),
    ];
    let mut unmatched: BTreeSet<String> = BTreeSet::new();
    let mut place = |state: &str, name: &str| -> Vec<(String, f64)> {
        let n = norm_county(name);
        if n == "not applicable" || n.is_empty() {
            return Vec::new();
        }
        if state == "CT"
            && let Some((_, old)) = OLD_CT.iter().find(|(c, _)| *c == strip_kind(&n))
        {
            return cw
                .overlaps
                .iter()
                .filter(|o| o.old == *old)
                .map(|o| (o.region.clone(), o.share_of_old))
                .collect();
        }
        if let Some((_, _, f)) = OVERRIDES.iter().find(|(s, x, _)| *s == state && *x == n) {
            let w = 1.0 / f.len() as f64;
            return f.iter().map(|x| (x.to_string(), w)).collect();
        }
        let key = |x: String| (state.to_string(), compact(&x));
        if let Some(f) = by_full.get(&key(n.clone())) {
            return vec![(f.clone(), 1.0)];
        }
        if let Some(f) = by_full.get(&key(strip_kind(&n))) {
            return vec![(f.clone(), 1.0)];
        }
        if let Some(v) = by_short.get(&key(strip_kind(&n)))
            && v.len() == 1
        {
            return vec![(v[0].clone(), 1.0)];
        }
        unmatched.insert(format!("{state}: {name}"));
        Vec::new()
    };

    let page = ctx.http.get(PAGE, None)?;
    let links = find_links(&page.text());
    // county -> year -> (sum w*saidi_with, w, sum w*saifi_with, w, ... ) per measure
    type Acc = [(f64, f64); 4];
    let mut acc: BTreeMap<String, BTreeMap<u16, Acc>> = BTreeMap::new();
    let mut utility_years: BTreeMap<String, u32> = BTreeMap::new();
    let mut years_done = Vec::new();
    for y in YEARS.0..=YEARS.1 {
        let Some(url) = links.get(&y) else {
            return Err(data_err(format!("EIA-861: no link for {y} on {PAGE}")));
        };
        let zip = ctx
            .http
            .get(url, Some(&format!("reliability/f861{y}.zip")))?;
        out.source(super::source_from(
            &format!("EIA-861 annual electric power industry report, {y} (reliability and service territory)"),
            &zip,
            format!("f861{y}.zip"),
            super::PUBLIC_DOMAIN,
            "",
        ));
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&zip.bytes))?;
        let names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
            .collect();
        let entry = |prefix: &str| {
            names
                .iter()
                .find(|n| {
                    n.rsplit('/').next().is_some_and(|b| {
                        b.to_ascii_lowercase().starts_with(prefix) && b.ends_with(".xlsx")
                    })
                })
                .cloned()
        };
        let (Some(rel), Some(ter)) = (entry("reliability"), entry("service_territory")) else {
            return Err(data_err(format!(
                "EIA-861 {y}: reliability or service-territory workbook not found in {names:?}"
            )));
        };
        let read = |n: &str| -> Result<Vec<(String, Sheet)>> {
            read_workbook(&crate::http::zip_entry(&zip.bytes, n)?)
        };
        let mut rel_rows = Vec::new();
        for (_, sheet) in read(&rel)? {
            rel_rows.extend(reliability_rows(&sheet));
        }
        let mut terr: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        for (_, sheet) in read(&ter)? {
            for (u, s, c) in territory_rows(&sheet) {
                terr.entry((u, s)).or_default().push(c);
            }
        }
        out.rows_in += rel_rows.len() as u64;
        let mut used = 0;
        for r in &rel_rows {
            let Some(names) = terr.get(&(r.utility.clone(), r.state.clone())) else {
                continue;
            };
            let places: Vec<(String, f64)> =
                names.iter().flat_map(|n| place(&r.state, n)).collect();
            if places.is_empty() {
                continue;
            }
            let cust = r.customers.unwrap_or(1.0).max(1.0);
            let share: f64 = places.iter().map(|(_, w)| w).sum();
            used += 1;
            for (f, w) in &places {
                let wt = cust * w / share;
                let e = acc
                    .entry(f.clone())
                    .or_default()
                    .entry(y)
                    .or_insert([(0.0, 0.0); 4]);
                for (k, v) in [r.saidi_with, r.saifi_with, r.saidi_without, r.saifi_without]
                    .into_iter()
                    .enumerate()
                {
                    if let Some(v) = v {
                        e[k].0 += wt * v;
                        e[k].1 += wt;
                    }
                }
                *utility_years.entry(f.clone()).or_default() += 1;
            }
        }
        eprintln!(
            "  EIA-861 {y}: {} utility rows, {used} placed in counties",
            rel_rows.len()
        );
        years_done.push((y, rel_rows.len(), used));
    }

    let mut table = Table::new(
        &[
            "fips",
            "saidi_with_med_min",
            "saifi_with_med",
            "saidi_without_med_min",
            "saifi_without_med",
            "utility_years",
        ],
        1,
    );
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let mut values: BTreeMap<String, [Option<f64>; 4]> = BTreeMap::new();
    for (f, years) in &acc {
        if !canon.contains(f) {
            continue;
        }
        let mut v = [None; 4];
        for k in 0..4 {
            let annual: Vec<f64> = years
                .values()
                .filter(|a| a[k].1 > 0.0)
                .map(|a| a[k].0 / a[k].1)
                .collect();
            if !annual.is_empty() {
                v[k] = Some(annual.iter().sum::<f64>() / annual.len() as f64);
            }
        }
        values.insert(f.clone(), v);
        table.push(vec![
            f.clone(),
            v[0].map(|x| sig(x, 3)).unwrap_or_default(),
            v[1].map(|x| sig(x, 3)).unwrap_or_default(),
            v[2].map(|x| sig(x, 3)).unwrap_or_default(),
            v[3].map(|x| sig(x, 3)).unwrap_or_default(),
            utility_years.get(f).copied().unwrap_or(0).to_string(),
        ]);
    }
    if table.rows.len() < 3_000 {
        return Err(data_err(format!(
            "EIA-861: only {} counties placed (about 3,100 expected)",
            table.rows.len()
        )));
    }
    let covered: BTreeSet<String> = table.rows.iter().map(|r| r[0].clone()).collect();
    out.table(ctx, RELIABILITY, &mut table)?;
    out.missing = missing_groups(&counties, &covered, |c| {
        if super::is_island_territory(&c.state_abbr) {
            "EIA-861 does not report this island area's utility by county".to_string()
        } else {
            "No utility serving the county reported reliability indices to EIA-861".to_string()
        }
    });

    // --- Cross-check against the EAGLE-I record -----------------------------------------------
    let (oh, orows) = read_table(&ctx.data, super::outages::OUTAGES)?;
    let (oi_f, oi_e, oi_h, oi_b) = (
        col(&oh, "fips")?,
        col(&oh, "events_per_customer_year")?,
        col(&oh, "customer_hours_per_customer_year")?,
        col(&oh, "duration_basis")?,
    );
    let mut pairs_f: Vec<(f64, f64)> = Vec::new();
    let mut pairs_d: Vec<(f64, f64)> = Vec::new();
    let mut above = Vec::new();
    for r in &orows {
        let Some(v) = values.get(&r[oi_f]) else {
            continue;
        };
        if r[oi_b] == "island" {
            continue;
        }
        let (Ok(e), Ok(hh)) = (r[oi_e].parse::<f64>(), r[oi_h].parse::<f64>()) else {
            continue;
        };
        if let Some(saifi) = v[1] {
            pairs_f.push((e, saifi));
            if saifi > 0.0 && e > 3.0 * saifi {
                above.push(r[oi_f].clone());
            }
        }
        if let Some(saidi) = v[0] {
            pairs_d.push((hh * 60.0, saidi));
        }
    }
    let rf = spearman(&pairs_f);
    let rd = spearman(&pairs_d);
    let lt = pairs_f.iter().filter(|(e, s)| e < s).count();
    out.notes.push(format!("Cross-check with EAGLE-I (core/outages.csv, repaired series): over {} counties, the rank correlation of EAGLE-I outages per customer-year with SAIFI (major event days included) is {rf:.2}, and of EAGLE-I customer-hours with SAIDI {rd:.2} ({} counties). EAGLE-I counts only outages in events that reach 1% of a county's customers, so it should sit below SAIFI: it does in {lt} of {} counties. {} counties show more than three times their utilities' SAIFI (the M-02 flag for reporting flicker){}.", pairs_f.len(), pairs_d.len(), pairs_f.len(), above.len(), if above.is_empty() { String::new() } else { format!(": {}", above.iter().take(20).cloned().collect::<Vec<_>>().join(", ")) }));
    out.definitions.insert(
        "reliability_county".into(),
        format!("Per year {}-{}, a county's SAIDI and SAIFI are the mean over the utilities EIA-861 lists as serving it, each weighted by its customers divided evenly over the counties it serves (EIA publishes no customer split by county); IEEE 1366 values where the utility reports them, otherwise its other method. The published value is the mean over the years with data; utility_years counts the utility-year records behind it.", YEARS.0, YEARS.1),
    );
    for (y, n, used) in &years_done {
        out.notes.push(format!("{y}: {n} utility-state reliability rows, {used} matched to counties through the service-territory list."));
    }
    if !unmatched.is_empty() {
        out.notes.push(format!(
            "Territory county names not placed ({}): {}.",
            unmatched.len(),
            unmatched
                .iter()
                .take(40)
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    Ok(out)
}

/// Spearman rank correlation (average ranks for ties).
pub fn spearman(pairs: &[(f64, f64)]) -> f64 {
    let n = pairs.len();
    if n < 3 {
        return f64::NAN;
    }
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
            let avg = (i + j) as f64 / 2.0 + 1.0;
            for k in i..=j {
                r[idx[k]] = avg;
            }
            i = j + 1;
        }
        r
    };
    let a = rank(pairs.iter().map(|p| p.0).collect());
    let b = rank(pairs.iter().map(|p| p.1).collect());
    let ma = a.iter().sum::<f64>() / n as f64;
    let mb = b.iter().sum::<f64>() / n as f64;
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for i in 0..n {
        sab += (a[i] - ma) * (b[i] - mb);
        saa += (a[i] - ma) * (a[i] - ma);
        sbb += (b[i] - mb) * (b[i] - mb);
    }
    sab / (saa * sbb).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn county_names_normalise() {
        assert_eq!(norm_county("St. Louis City"), "st louis city");
        assert_eq!(norm_county("Saint Mary's"), "st marys");
        assert_eq!(norm_county("Matanuska-Susitna"), "matanuska susitna");
        assert_eq!(strip_kind("orleans parish"), "orleans");
        assert_eq!(strip_kind("juneau city and borough"), "juneau");
    }

    #[test]
    fn reliability_sheet_prefers_ieee_values() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let sheet: Sheet = vec![
            s(&[
                "Utility Characteristics",
                "",
                "",
                "IEEE Standard",
                "",
                "",
                "",
                "",
                "Other Standard",
                "",
                "",
                "",
                "",
            ]),
            s(&[
                "",
                "",
                "",
                "All Events (With Major Event Days)",
                "",
                "Without Major Event Days",
                "",
                "",
                "All Events (With Major Event Days)",
                "",
                "Without Major Event Days",
                "",
                "",
            ]),
            s(&[
                "Utility Number",
                "State",
                "Ownership",
                "SAIDI (minutes per year)",
                "SAIFI (times per year)",
                "SAIDI (minutes per year)",
                "SAIFI (times per year)",
                "Number of Customers",
                "SAIDI (minutes per year)",
                "SAIFI (times per year)",
                "SAIDI (minutes per year)",
                "SAIFI (times per year)",
                "Number of Customers",
            ]),
            s(&[
                "84",
                "MD",
                "Cooperative",
                ".",
                ".",
                ".",
                ".",
                ".",
                "29.102",
                "0.23",
                "29.102",
                "0.23",
                "317",
            ]),
            s(&[
                "195",
                "AL",
                "Investor Owned",
                "150.5",
                "1.2",
                "95",
                "0.9",
                "1500000",
                "",
                "",
                "",
                "",
                "",
            ]),
        ];
        let r = reliability_rows(&sheet);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].saidi_with, Some(29.102));
        assert_eq!(r[0].customers, Some(317.0));
        assert_eq!(r[1].saifi_without, Some(0.9));
        assert_eq!(r[1].customers, Some(1_500_000.0));
    }

    #[test]
    fn older_two_row_headers_read_too() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let sheet: Sheet = vec![
            s(&[
                "Utility Characteristics",
                "",
                "",
                "IEEE Standard",
                "",
                "",
                "",
                "",
                "",
                "",
                "Other Standard",
                "",
                "",
            ]),
            s(&[
                "Utility Number",
                "State",
                "Ownership",
                "SAIDI With MED",
                "SAIFI With MED",
                "SAIDI Without MED",
                "SAIFI Without MED",
                "SAIDI With MED Minus LOS",
                "SAIFI With MED Minus LOS",
                "Number of Customers",
                "SAIDI With MED",
                "SAIFI With MED",
                "Number of Customers",
            ]),
            s(&[
                "195",
                "AL",
                "Investor Owned",
                "150.5",
                "1.2",
                "95",
                "0.9",
                "140",
                "1.1",
                "1500000",
                "",
                "",
                "",
            ]),
            s(&[
                "84",
                "MD",
                "Cooperative",
                ".",
                ".",
                ".",
                ".",
                ".",
                ".",
                ".",
                "29.1",
                "0.23",
                "317",
            ]),
        ];
        let r = reliability_rows(&sheet);
        assert_eq!(r.len(), 2);
        assert_eq!(
            (r[0].saidi_with, r[0].saifi_without),
            (Some(150.5), Some(0.9))
        );
        assert_eq!(r[0].customers, Some(1_500_000.0));
        assert_eq!((r[1].saidi_with, r[1].customers), (Some(29.1), Some(317.0)));
        assert_eq!(compact(&norm_county("Añasco")), "anasco");
        assert_eq!(
            compact(&norm_county("De Witt")),
            compact(&norm_county("DeWitt"))
        );
    }

    #[test]
    fn spearman_basics() {
        let p = [(1.0, 10.0), (2.0, 20.0), (3.0, 30.0), (4.0, 40.0)];
        assert!((spearman(&p) - 1.0).abs() < 1e-12);
        let q = [(1.0, 40.0), (2.0, 30.0), (3.0, 20.0), (4.0, 10.0)];
        assert!((spearman(&q) + 1.0).abs() < 1e-12);
    }

    #[test]
    fn links_from_the_eia_page() {
        let html = r#"<a href="zip/f8612024.zip">2024</a><a href="archive/zip/f8612023.zip">2023</a><a href="zip/f8612023er.zip">er</a>"#;
        let l = find_links(html);
        assert_eq!(
            l.get(&2024).unwrap(),
            "https://www.eia.gov/electricity/data/eia861/zip/f8612024.zip"
        );
        assert_eq!(
            l.get(&2023).unwrap(),
            "https://www.eia.gov/electricity/data/eia861/archive/zip/f8612023.zip"
        );
        assert_eq!(l.len(), 2);
    }
}
