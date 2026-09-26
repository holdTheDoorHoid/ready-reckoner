//! Job `displacement` — federal disaster declarations per county and FEMA housing-assistance
//! outcomes by type of disaster, from OpenFEMA (data audit §9.7; DESIGN-DELTA §1.3
//! `RecoveryInfo.county_declarations_5yr`, consequence brief item 5).
//!
//! - `core/declarations.csv`: per county, major-disaster declarations (type DR) covering it in the
//!   five full years before the refresh, since 2000, with Individual Assistance, and for
//!   hurricanes, tropical storms, coastal storms or floods. A declaration counts once per county
//!   however many rows it has; statewide and tribal designations are not counted.
//! - `core/series/ihp_displacement.toml`: FEMA Individuals and Households Program housing
//!   assistance by incident type (HousingAssistanceRenters and HousingAssistanceOwners, joined to
//!   the declarations by disaster number): the share of inspected renter households with major or
//!   substantial damage, the share of inspected owners with more than $30,000 of FEMA-inspected
//!   damage, and rental assistance paid per approved household.
//!
//! Months displaced by hazard, which the audit proposed from IHP registrations, is not built:
//! only 99,762 of the 3,309,028 rental-eligible registrations carry a rental-assistance end date,
//! nearly all recent (research 2026-09-26, §5.3).

use super::{Ctx, JobOutput, load_counties};
use crate::csvout::Table;
use crate::ct::{Crosswalk, is_old_ct};
use crate::http::Sha256Acc;
use crate::jobs::series::{SRate, SeriesDoc, Val};
use crate::manifest::SourceRecord;
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};

/// County declarations file.
pub const DECLARATIONS: &str = "core/declarations.csv";
/// IHP series file.
pub const IHP: &str = "core/series/ihp_displacement.toml";

const API: &str = "https://www.fema.gov/api/open/v2";
const DECL_PAGE: &str =
    "https://www.fema.gov/openfema-data-page/disaster-declarations-summaries-v2";
const OWNERS_PAGE: &str =
    "https://www.fema.gov/openfema-data-page/housing-assistance-program-data-owners-v2";
const RENTERS_PAGE: &str =
    "https://www.fema.gov/openfema-data-page/housing-assistance-program-data-renters-v2";

/// Incident types counted as hurricanes or floods.
pub const HURRICANE_OR_FLOOD: &[&str] = &[
    "Hurricane",
    "Tropical Storm",
    "Tropical Depression",
    "Typhoon",
    "Coastal Storm",
    "Flood",
];

/// Page through an OpenFEMA v2 dataset with `$skip` (ordered by `id`), returning the rows.
fn page_all(
    ctx: &Ctx,
    dataset: &str,
    select: &str,
    filter: &str,
) -> Result<(
    Vec<serde_json::Map<String, serde_json::Value>>,
    SourceRecord,
)> {
    let mut acc = Sha256Acc::new();
    let retrieved = crate::timefmt::now_utc();
    let mut rows = Vec::new();
    let mut pages = 0u64;
    loop {
        let mut url = format!(
            "{API}/{dataset}?$select={select}&$orderby=id&$top=10000&$skip={}",
            rows.len()
        );
        if !filter.is_empty() {
            url.push_str(&format!("&$filter={filter}"));
        }
        let f = ctx.http.get(&url, None)?;
        acc.update(&f.bytes);
        pages += 1;
        let v: serde_json::Value = serde_json::from_slice(&f.bytes)?;
        if let Some(e) = v.get("error") {
            return Err(data_err(format!("OpenFEMA {dataset}: {e}")));
        }
        let page = v[dataset]
            .as_array()
            .ok_or_else(|| data_err(format!("OpenFEMA {dataset}: no rows array")))?;
        let n = page.len();
        rows.extend(page.iter().filter_map(|r| r.as_object().cloned()));
        if n < 10_000 {
            break;
        }
    }
    let bytes = acc.len();
    Ok((
        rows,
        SourceRecord {
            name: format!("OpenFEMA {dataset} v2"),
            url: format!("{API}/{dataset}?$select={select} (pages of 10000 ordered by id)"),
            version: format!("v2; {pages} pages"),
            retrieved,
            sha256: acc.finish(),
            bytes,
            license: "OpenFEMA Terms and Conditions (public data; citation and statement required)"
                .into(),
            obligations: format!(
                "Cite the OpenFEMA dataset page with access date; show: {}",
                super::flood::OPENFEMA_STATEMENT
            ),
        },
    ))
}

fn s<'a>(r: &'a serde_json::Map<String, serde_json::Value>, k: &str) -> &'a str {
    r.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

fn n(r: &serde_json::Map<String, serde_json::Value>, k: &str) -> f64 {
    r.get(k)
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|x| x.parse().ok()))
        })
        .unwrap_or(0.0)
}

fn b(r: &serde_json::Map<String, serde_json::Value>, k: &str) -> bool {
    r.get(k).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    let cw = Crosswalk::load(&ctx.data)?;
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let build_year: i32 = crate::timefmt::today_utc()[..4].parse().unwrap_or(2026);
    let last5 = (build_year - 5, build_year - 1);

    // --- Declarations -------------------------------------------------------------------------
    let (decl, src) = page_all(
        ctx,
        "DisasterDeclarationsSummaries",
        "disasterNumber,declarationType,declarationDate,incidentType,ihProgramDeclared,iaProgramDeclared,fipsStateCode,fipsCountyCode,placeCode,designatedArea,id",
        "",
    )?;
    out.source(src);
    out.rows_in += decl.len() as u64;
    if decl.len() < 60_000 {
        return Err(data_err(format!(
            "OpenFEMA declarations: only {} rows (70,422 on 2026-09-26)",
            decl.len()
        )));
    }
    let mut incident_of: BTreeMap<i64, String> = BTreeMap::new();
    // county -> disaster -> (year, individual assistance, hurricane or flood)
    let mut per_county: BTreeMap<String, BTreeMap<i64, (i32, bool, bool)>> = BTreeMap::new();
    let mut statewide = 0u64;
    let mut unknown: BTreeSet<String> = BTreeSet::new();
    for r in &decl {
        let dn = n(r, "disasterNumber") as i64;
        let incident = s(r, "incidentType").to_string();
        incident_of.entry(dn).or_insert_with(|| incident.clone());
        if s(r, "declarationType") != "DR" {
            continue;
        }
        let year: i32 = s(r, "declarationDate")
            .get(..4)
            .and_then(|y| y.parse().ok())
            .unwrap_or(0);
        let county = s(r, "fipsCountyCode");
        if county.is_empty() || county == "000" {
            statewide += 1;
            continue;
        }
        let code = format!("{:0>2}{:0>3}", s(r, "fipsStateCode"), county);
        let ia = b(r, "ihProgramDeclared") || b(r, "iaProgramDeclared");
        let hf = HURRICANE_OR_FLOOD.contains(&incident.as_str());
        let targets: Vec<String> = if canon.contains(&code) {
            vec![code.clone()]
        } else if is_old_ct(&code) {
            cw.overlaps
                .iter()
                .filter(|o| o.old == code)
                .map(|o| o.region.clone())
                .collect()
        } else if let Some(new) = crate::ct::successors(&code) {
            new.iter().map(|x| x.to_string()).collect()
        } else {
            unknown.insert(code.clone());
            Vec::new()
        };
        for t in targets {
            let e = per_county
                .entry(t)
                .or_default()
                .entry(dn)
                .or_insert((year, false, false));
            e.0 = e.0.min(year);
            e.1 |= ia;
            e.2 |= hf;
        }
    }
    let mut table = Table::new(
        &[
            "fips",
            "last_5yr",
            "since_2000",
            "with_individual_assistance",
            "hurricane_or_flood",
        ],
        1,
    );
    for c in &counties {
        let d = per_county.get(&c.fips);
        let count = |f: &dyn Fn(&(i32, bool, bool)) -> bool| -> usize {
            d.map_or(0, |m| m.values().filter(|x| f(x)).count())
        };
        table.push(vec![
            c.fips.clone(),
            count(&|x| x.0 >= last5.0 && x.0 <= last5.1).to_string(),
            count(&|x| x.0 >= 2000).to_string(),
            count(&|x| x.0 >= 2000 && x.1).to_string(),
            count(&|x| x.0 >= 2000 && x.2).to_string(),
        ]);
    }
    // Guards: Harris County, Texas has had at least ten major-disaster declarations since 2000
    // (16 on 2026-09-26), and a typical county several.
    let since = |f: &str| -> usize {
        table
            .rows
            .iter()
            .find(|r| r[0] == f)
            .and_then(|r| r[2].parse().ok())
            .unwrap_or(0)
    };
    let mut all: Vec<usize> = table
        .rows
        .iter()
        .map(|r| r[2].parse().unwrap_or(0))
        .collect();
    all.sort_unstable();
    let median = all[all.len() / 2];
    if since("48201") < 10 || median < 3 {
        return Err(data_err(format!(
            "OpenFEMA declarations look wrong: Harris County TX has {} since 2000, the median county {median}",
            since("48201")
        )));
    }
    out.table(ctx, DECLARATIONS, &mut table)?;
    out.notes.push(format!("Declarations: major-disaster (DR) declarations only; one count per disaster per county; {statewide} statewide or tribal rows not counted; last_5yr = declared in {}-{}. Connecticut declarations on the old counties count for every planning region they overlap. hurricane_or_flood = incident type {}.{}", last5.0, last5.1, HURRICANE_OR_FLOOD.join(", "), if unknown.is_empty() { String::new() } else { format!(" County codes not placed: {}.", unknown.iter().take(20).cloned().collect::<Vec<_>>().join(", ")) }));
    out.definitions.insert(
        "county_declarations_5yr".into(),
        format!("Major-disaster declarations (FEMA type DR) that designated the county, declared in {}-{} (the five full years before the pack was built); a declaration counts once however many rows it has.", last5.0, last5.1),
    );

    // --- Housing assistance by incident type ---------------------------------------------------
    let (renters, src_r) = page_all(
        ctx,
        "HousingAssistanceRenters",
        "disasterNumber,validRegistrations,totalInspected,totalInspectedWithNoDamage,totalWithModerateDamage,totalWithMajorDamage,totalWithSubstantialDamage,approvedForFemaAssistance,rentalAmount,id",
        "",
    )?;
    out.source(src_r);
    let (owners, src_o) = page_all(
        ctx,
        "HousingAssistanceOwners",
        "disasterNumber,validRegistrations,noFemaInspectedDamage,femaInspectedDamageBetween1And10000,femaInspectedDamageBetween10001And20000,femaInspectedDamageBetween20001And30000,femaInspectedDamageGreaterThan30000,approvedForFemaAssistance,rentalAmount,id",
        "",
    )?;
    out.source(src_o);
    out.rows_in += (renters.len() + owners.len()) as u64;
    #[derive(Default, Clone)]
    struct Agg {
        disasters: BTreeSet<i64>,
        r_valid: f64,
        r_inspected: f64,
        r_major: f64,
        r_moderate: f64,
        r_approved: f64,
        r_rent: f64,
        o_valid: f64,
        o_inspected: f64,
        o_gt30k: f64,
        o_gt10k: f64,
        o_approved: f64,
        o_rent: f64,
    }
    let group = |incident: &str| -> String {
        match incident {
            "Hurricane" | "Tropical Storm" | "Tropical Depression" | "Typhoon" => "hurricane",
            "Flood" | "Coastal Storm" => "flood",
            "Fire" => "fire",
            "Severe Storm" | "Severe Storm(s)" => "severe_storm",
            "Tornado" => "tornado",
            "Earthquake" => "earthquake",
            "Snowstorm" | "Severe Ice Storm" | "Winter Storm" | "Freezing" => "winter",
            "Mud/Landslide" => "landslide",
            _ => "other",
        }
        .to_string()
    };
    let mut agg: BTreeMap<String, Agg> = BTreeMap::new();
    let mut unmatched = 0u64;
    for r in &renters {
        let dn = n(r, "disasterNumber") as i64;
        let Some(inc) = incident_of.get(&dn) else {
            unmatched += 1;
            continue;
        };
        let a = agg.entry(group(inc)).or_default();
        a.disasters.insert(dn);
        a.r_valid += n(r, "validRegistrations");
        // `totalInspected` is zero in every row of the v2 files (checked 2026-09-26): the
        // inspected count is the sum of the damage categories.
        a.r_inspected += n(r, "totalInspectedWithNoDamage")
            + n(r, "totalWithModerateDamage")
            + n(r, "totalWithMajorDamage")
            + n(r, "totalWithSubstantialDamage");
        a.r_major += n(r, "totalWithMajorDamage") + n(r, "totalWithSubstantialDamage");
        a.r_moderate += n(r, "totalWithModerateDamage");
        a.r_approved += n(r, "approvedForFemaAssistance");
        a.r_rent += n(r, "rentalAmount");
    }
    for r in &owners {
        let dn = n(r, "disasterNumber") as i64;
        let Some(inc) = incident_of.get(&dn) else {
            unmatched += 1;
            continue;
        };
        let a = agg.entry(group(inc)).or_default();
        a.disasters.insert(dn);
        a.o_valid += n(r, "validRegistrations");
        a.o_inspected += n(r, "noFemaInspectedDamage")
            + n(r, "femaInspectedDamageBetween1And10000")
            + n(r, "femaInspectedDamageBetween10001And20000")
            + n(r, "femaInspectedDamageBetween20001And30000")
            + n(r, "femaInspectedDamageGreaterThan30000");
        a.o_gt30k += n(r, "femaInspectedDamageGreaterThan30000");
        a.o_gt10k += n(r, "femaInspectedDamageGreaterThan30000")
            + n(r, "femaInspectedDamageBetween10001And20000")
            + n(r, "femaInspectedDamageBetween20001And30000");
        a.o_approved += n(r, "approvedForFemaAssistance");
        a.o_rent += n(r, "rentalAmount");
    }
    let retrieved = crate::timefmt::today_utc();
    let mut doc = SeriesDoc {
        id: "ihp_displacement".into(),
        title: "FEMA housing assistance after declared disasters, by type of disaster".into(),
        publisher: "Federal Emergency Management Agency (OpenFEMA)".into(),
        url: RENTERS_PAGE.into(),
        licence: "OpenFEMA Terms and Conditions (public data; citation and statement required)"
            .into(),
        attribution: super::flood::OPENFEMA_STATEMENT.into(),
        source: "openfema_housing_assistance".into(),
        retrieved,
        how: "fetched".into(),
        ..Default::default()
    };
    doc.notes.push(format!("HousingAssistanceRenters ({} rows) and HousingAssistanceOwners ({} rows) v2, per disaster and ZIP code, joined to DisasterDeclarationsSummaries by disaster number for the incident type; {unmatched} rows had no declaration.", renters.len(), owners.len()));
    doc.notes.push("OpenFEMA's totalInspected field is zero in every row of both files (checked 2026-09-26); the inspected counts here are the sums of the damage categories.".into());
    doc.notes.push("Counts registrations with FEMA, not all households hit: people who did not apply, or whose insurance covered them, are not in the data. Damage levels are FEMA's inspection categories for renters (major or substantial = the home is unlivable for a long time); owners are bucketed by FEMA-inspected damage in dollars instead. Rental assistance is what FEMA paid to help people live elsewhere, a floor on the cost of being displaced.".into());
    doc.notes.push(format!("See also {OWNERS_PAGE} and {DECL_PAGE}. Months displaced by hazard is not built: only 3% of rental-eligible registrations carry an end date."));
    let year = u16::try_from(build_year - 1).unwrap_or(2025);
    for (g, a) in &agg {
        if a.r_inspected > 0.0 {
            doc.rates.push(SRate {
                id: format!("ihp_renter_major_damage_share_{g}"),
                value: a.r_major / a.r_inspected,
                unit: format!("share of inspected renter registrations with major or substantial damage ({g} disasters)"),
                year,
                period: "all declarations in the OpenFEMA housing-assistance files".into(),
                figure: format!("{:.0} of {:.0} inspected renter registrations, {} disasters", a.r_major, a.r_inspected, a.disasters.len()),
                derivation: "(totalWithMajorDamage + totalWithSubstantialDamage) / (no damage + moderate + major + substantial)".into(),
                note: String::new(),
                ..Default::default()
            });
        }
        if a.o_inspected > 0.0 {
            doc.rates.push(SRate {
                id: format!("ihp_owner_damage_over_30k_share_{g}"),
                value: a.o_gt30k / a.o_inspected,
                unit: format!("share of inspected owner registrations with more than $30,000 of FEMA-inspected damage ({g} disasters)"),
                year,
                period: "all declarations in the OpenFEMA housing-assistance files".into(),
                figure: format!("{:.0} of {:.0} inspected owner registrations", a.o_gt30k, a.o_inspected),
                derivation: "femaInspectedDamageGreaterThan30000 / (no damage + all four damage buckets)".into(),
                note: "Nominal dollars across the years; the threshold means more in older disasters.".into(),
                ..Default::default()
            });
        }
        let approved = a.r_approved + a.o_approved;
        if approved > 0.0 {
            doc.rates.push(SRate {
                id: format!("ihp_rental_assistance_per_approved_usd_{g}"),
                value: (a.r_rent + a.o_rent) / approved,
                unit: format!("US dollars of FEMA rental assistance per household approved for assistance ({g} disasters, nominal)"),
                year,
                period: "all declarations in the OpenFEMA housing-assistance files".into(),
                figure: format!("${:.0} to {:.0} approved households", a.r_rent + a.o_rent, approved),
                derivation: "(renters rentalAmount + owners rentalAmount) / (renters + owners approvedForFemaAssistance)".into(),
                note: "Averages over everyone approved, including those who got no rental help.".into(),
                ..Default::default()
            });
        }
        doc.rows.push(vec![
            ("incident".into(), Val::S(g.clone())),
            ("disasters".into(), Val::I(a.disasters.len() as i64)),
            ("renter_registrations".into(), Val::N(a.r_valid)),
            ("renters_inspected".into(), Val::N(a.r_inspected)),
            ("renters_moderate_damage".into(), Val::N(a.r_moderate)),
            ("renters_major_or_substantial".into(), Val::N(a.r_major)),
            ("owner_registrations".into(), Val::N(a.o_valid)),
            ("owners_inspected".into(), Val::N(a.o_inspected)),
            ("owners_damage_over_10k".into(), Val::N(a.o_gt10k)),
            ("owners_damage_over_30k".into(), Val::N(a.o_gt30k)),
            ("rental_assistance_usd".into(), Val::N(a.r_rent + a.o_rent)),
        ]);
    }
    doc.write(ctx, &mut out, IHP)?;
    out.notes.push(format!("{} disaster groups in the housing-assistance series; values rounded to 4 significant figures.", agg.len()));
    out.attributions.push(crate::manifest::Attribution {
        source: "OpenFEMA declarations and housing assistance".into(),
        text: format!(
            "Federal Emergency Management Agency (FEMA), OpenFEMA Datasets: Disaster Declarations Summaries - v2, Housing Assistance Program Data - Owners - v2 and Renters - v2. Retrieved from {DECL_PAGE}, {OWNERS_PAGE} and {RENTERS_PAGE}. {}",
            super::flood::OPENFEMA_STATEMENT
        ),
        license: "OpenFEMA Terms and Conditions".into(),
        url: DECL_PAGE.into(),
        version: Some("v2".into()),
        accessed: crate::timefmt::today_utc(),
    });
    Ok(out)
}
