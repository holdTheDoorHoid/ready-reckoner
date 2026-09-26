//! Job 2 — FEMA National Risk Index v1.20, county level, trimmed to the fields the model needs.
//!
//! Fetched through the ArcGIS FeatureServer that FEMA publishes for programmatic use (fema.gov's
//! file downloads refuse scripted clients). The job:
//! - reads the item's Terms & Conditions and extracts the disclaimer FEMA requires, verbatim;
//! - checks the data dictionary still says version 1.20 (a new version must be reviewed, because
//!   field meanings changed between 1.19 and 1.20 and may change again);
//! - pages through all counties, keeps 9 fields per hazard and 6 county fields, rounds to 4
//!   significant figures, and never writes the raw table;
//! - writes `nri_semantics.toml` so `rr-hazards` knows what each hazard's `AFREQ` means.

use super::{Ctx, JobOutput, load_counties, missing_groups};
use crate::csvout::Table;
use crate::http::Sha256Acc;
use crate::manifest::{Attribution, SourceRecord};
use crate::num::opt4;
use crate::{Result, data_err};
use std::collections::BTreeSet;
use std::str::FromStr;

/// County-level NRI fields.
pub const NRI_COUNTIES: &str = "core/nri_counties.csv";
/// Per-hazard NRI fields (long format: one row per county and hazard).
pub const NRI_HAZARDS: &str = "core/nri_hazards.csv";
/// What each hazard's AFREQ means.
pub const NRI_SEMANTICS: &str = "core/nri_semantics.toml";

const LAYER: &str = "https://services.arcgis.com/XG15cJAlne2vxtgt/arcgis/rest/services/National_Risk_Index_Counties/FeatureServer/0";
const ITEM: &str =
    "https://www.arcgis.com/sharing/rest/content/items/39485e8035d446a5bff03259508ae355?f=json";
const ITEM_PAGE: &str =
    "https://fema.maps.arcgis.com/home/item.html?id=39485e8035d446a5bff03259508ae355";
const DICTIONARY: &str =
    "https://fema.maps.arcgis.com/sharing/rest/content/items/4b9db412e99542029b3c37c37ad714bb/data";
const TECH_DOC: &str = "https://www.fema.gov/sites/default/files/documents/fema_national-risk-index_technical-documentation.pdf";
const TECH_DOC_ARCHIVE: &str = "https://web.archive.org/web/20260720090240/https://www.fema.gov/sites/default/files/documents/fema_national-risk-index_technical-documentation.pdf";

/// The NRI version this job's semantics table was written for.
pub const EXPECTED_VERSION: &str = "1.20";

/// Per-hazard fields kept, as (NRI suffix, pack column).
const HAZARD_FIELDS: &[(&str, &str)] = &[
    ("AFREQ", "afreq"),
    ("EXPB", "expb"),
    ("EXPP", "expp"),
    ("EALB", "ealb"),
    ("EALP", "ealp"),
    ("EALT", "ealt"),
    ("HLRB", "hlrb"),
    ("ALRB", "alrb"),
    ("RISKS", "risk_score"),
];

/// County fields kept, as (NRI field, pack column).
const COUNTY_FIELDS: &[(&str, &str)] = &[
    ("POPULATION", "population"),
    ("BUILDVALUE", "building_value_usd"),
    ("EAL_VALT", "eal_valt"),
    ("SOVI_SCORE", "sovi_score"),
    ("RESL_SCORE", "resl_score"),
    ("CRF_VALUE", "crf_value"),
];

/// What one hazard's annualised frequency means in NRI v1.20 (Technical Documentation,
/// December 2025: §5.2 Table 5 and the hazard's own "Annualized Frequency" section).
pub struct Semantics {
    /// NRI field prefix.
    pub code: &'static str,
    /// Ready Reckoner hazard id.
    pub id: &'static str,
    /// `events_per_year` or `annual_probability` (the `AfreqKind` wire strings).
    pub afreq_kind: &'static str,
    /// `distinct_events`, `event_days` or `modelled`.
    pub basis: &'static str,
    /// Unit of AFREQ in plain words.
    pub unit: &'static str,
    /// Period of record, as the documentation states it.
    pub period: &'static str,
    /// Whether `1 - exp(-AFREQ)` is a sound "chance of at least one event per year".
    pub poisson_ok: bool,
    /// Section of the technical documentation.
    pub section: &'static str,
    /// What downstream code must know.
    pub notes: &'static str,
}

/// The semantics table for NRI v1.20.
pub const SEMANTICS: &[Semantics] = &[
    Semantics {
        code: "AVLN",
        id: "avalanche",
        afreq_kind: "events_per_year",
        basis: "distinct_events",
        unit: "loss-causing avalanche events per year (county)",
        period: "30 years (1994-2023)",
        poisson_ok: true,
        section: "6.5",
        notes: "Counts from SHELDUS, the Colorado Avalanche Information Center and NOAA. Counties in an avalanche susceptibility zone with no recorded loss-causing event get a minimum of 0.01 per year.",
    },
    Semantics {
        code: "CFLD",
        id: "coastal_flooding",
        afreq_kind: "events_per_year",
        basis: "modelled",
        unit: "modelled coastal flood occurrences per year, area-weighted over developed land",
        period: "not applicable (modelled)",
        poisson_ok: false,
        section: "7.4",
        notes: "Sum of sub-type frequencies: 0.01 inside the 1% annual-chance coastal floodplain, 0.002 inside the 0.2% floodplain, plus NOAA high-tide flooding classes (minor, moderate, major) at the midpoints of NOAA's probability classes. High-tide flooding recurs, so values above 1 are common and do not measure the chance of a damaging surge. Use the expected annual loss fields and flood.csv (share of homes in the flood zone) for household risk.",
    },
    Semantics {
        code: "CWAV",
        id: "cold_wave",
        afreq_kind: "events_per_year",
        basis: "event_days",
        unit: "cold-wave days per year (NWS alerts), area-weighted",
        period: "19.0 years (2005-11-12 to 2024-11-12)",
        poisson_ok: false,
        section: "8.5",
        notes: "Table 5 of the documentation calls the measure an 'annualized probability', but the section text and the values (often above 1) are event-days per year. Days cluster into episodes: divide by a typical episode length before treating the result as episodes per year.",
    },
    Semantics {
        code: "DRGT",
        id: "drought",
        afreq_kind: "events_per_year",
        basis: "event_days",
        unit: "drought days per year (US Drought Monitor event-weeks times 7), area-weighted",
        period: "25.25 years",
        poisson_ok: false,
        section: "9.5",
        notes: "Event-days, not events: a single drought can contribute hundreds of days. Divide by a typical drought length before treating it as droughts per year.",
    },
    Semantics {
        code: "ERQK",
        id: "earthquake",
        afreq_kind: "annual_probability",
        basis: "modelled",
        unit: "annual probability of at least minor-damage shaking (USGS probability grid), area-weighted",
        period: "not applicable (modelled)",
        poisson_ok: false,
        section: "10.4",
        notes: "Already a yearly probability; do not transform. seismic.csv carries USGS exceedance probabilities at 0.1 g and 0.2 g at the county centroid for cross-checking.",
    },
    Semantics {
        code: "HAIL",
        id: "hail",
        afreq_kind: "events_per_year",
        basis: "distinct_events",
        unit: "hail events per year (hail 0.75 in or larger; 49-km fishnet, scaled), area-weighted",
        period: "38.0 years (1986-2023)",
        poisson_ok: true,
        section: "11.5",
        notes: "Fishnet counts include hail that fell near, not only inside, the county; a regional frequency rather than the chance hail hits one home.",
    },
    Semantics {
        code: "HWAV",
        id: "heat_wave",
        afreq_kind: "events_per_year",
        basis: "event_days",
        unit: "heat-wave days per year (NWS alerts), area-weighted",
        period: "19.0 years (2005-11-12 to 2024-11-12)",
        poisson_ok: false,
        section: "12.5",
        notes: "Event-days per year. Days cluster into episodes: divide by a typical episode length before treating the result as episodes per year.",
    },
    Semantics {
        code: "HRCN",
        id: "hurricane",
        afreq_kind: "events_per_year",
        basis: "distinct_events",
        unit: "hurricane events per year (buffered HURDAT2 tracks on a 49-km fishnet)",
        period: "173.86 years for the Atlantic (1851-01-01 to 2024-09-30); 76.0 years for the Pacific (1949-01-01 to 2024-11-30)",
        poisson_ok: true,
        section: "13.5",
        notes: "Minimum frequencies are assigned to blocks in hurricane-prone counties with no recorded track. events.csv adds hurricane and tropical-storm passage rates within 50 nautical miles of the county centroid for cross-checking.",
    },
    Semantics {
        code: "ISTM",
        id: "ice_storm",
        afreq_kind: "events_per_year",
        basis: "event_days",
        unit: "ice-storm days per year (49-km fishnet), area-weighted",
        period: "67.16 years (1946-12-31 to 2014-02-12)",
        poisson_ok: false,
        section: "14.5",
        notes: "Event-days per year; the record ends in 2014. events.csv has ice-storm episode rates from NOAA Storm Events 1996-2025.",
    },
    Semantics {
        code: "IFLD",
        id: "riverine_flooding",
        afreq_kind: "events_per_year",
        basis: "event_days",
        unit: "inland flood days per year (NCEI Storm Events flood and flash-flood records, one per county-day)",
        period: "27 years (1996-2023, as stated)",
        poisson_ok: false,
        section: "15.5",
        notes: "New in v1.20: Inland Flooding replaced Riverine Flooding (RFLD) and includes rain-driven (pluvial) flooding. About 100% of a county's building value is treated as exposed, so EALB/EXPB is a county-wide average rather than an in-floodplain rate. Counties crossing the 1% floodplain with no recorded flood day get a minimum of 0.01. Pair with flood.csv (share of homes in the flood zone).",
    },
    Semantics {
        code: "LNDS",
        id: "landslide",
        afreq_kind: "events_per_year",
        basis: "modelled",
        unit: "landslide events per year (USGS county rate, weighted by the 2024 USGS susceptibility model)",
        period: "not applicable (modelled rate)",
        poisson_ok: true,
        section: "16.5",
        notes: "County value is the USGS estimated events per county per year; tracts inherit it in proportion to susceptibility.",
    },
    Semantics {
        code: "LTNG",
        id: "lightning",
        afreq_kind: "events_per_year",
        basis: "event_days",
        unit: "days per year with a cloud-to-ground lightning strike (SPC daily lightning climatology), area-weighted",
        period: "29 years in the contiguous US (1995-2023); 12 years elsewhere (2012-2024)",
        poisson_ok: false,
        section: "17.5",
        notes: "Table 5 calls the measure an 'annualized probability', but values are days per year (often 20 to 80). No lightning data for Guam, the Northern Mariana Islands, American Samoa or the US Virgin Islands.",
    },
    Semantics {
        code: "SWND",
        id: "strong_wind",
        afreq_kind: "events_per_year",
        basis: "distinct_events",
        unit: "strong-wind events per year (49-km fishnet, scaled), area-weighted",
        period: "38.0 years (as stated; the documentation's date range reads 1996-2023)",
        poisson_ok: true,
        section: "18.5",
        notes: "Fishnet counts include events near, not only inside, the county.",
    },
    Semantics {
        code: "TRND",
        id: "tornado",
        afreq_kind: "events_per_year",
        basis: "distinct_events",
        unit: "tornado events per year (80-km buffered paths on a 49-km fishnet, nationally scaled by EF sub-type)",
        period: "74.0 years for EF2-EF5 (1950-2023); 38.0 years for EF0-EF1 (1986-2023)",
        poisson_ok: true,
        section: "19.5",
        notes: "A frequency of tornadoes in the county's surroundings, not the chance a tornado strikes one home; use EALB/EXPB or HLRB for damage. events.csv adds SPC tornado counts inside the county.",
    },
    Semantics {
        code: "TSUN",
        id: "tsunami",
        afreq_kind: "annual_probability",
        basis: "modelled",
        unit: "surrogate yearly frequency: regional historical runup rate plus P-2426 probabilistic frequency",
        period: "222.15 years for the historical part",
        poisson_ok: false,
        section: "20.3",
        notes: "FEMA states this value is a surrogate 'strictly for reference' and is not used in NRI's own loss calculations. Use it only to flag tsunami exposure (nri_counties.csv tsunami_zone) and use the expected annual loss fields for magnitude.",
    },
    Semantics {
        code: "VLCN",
        id: "volcanic_activity",
        afreq_kind: "annual_probability",
        basis: "modelled",
        unit: "yearly eruption frequency from USGS National Volcanic Threat Assessment recurrence scores (score 4 = 0.0101, 3 = 0.001, 2 = 0.0002)",
        period: "not applicable (recurrence classes)",
        poisson_ok: false,
        section: "21.4",
        notes: "Very small by construction; a per-volcano recurrence class, area-weighted.",
    },
    Semantics {
        code: "WFIR",
        id: "wildfire",
        afreq_kind: "annual_probability",
        basis: "modelled",
        unit: "annual burn probability from a large fire (USFS FSim), area-weighted over developed or agricultural land",
        period: "not applicable (modelled)",
        poisson_ok: false,
        section: "22.4",
        notes: "Already a yearly probability that fire reaches a point; do not transform. Building/population frequency can differ from agriculture frequency.",
    },
    Semantics {
        code: "WNTW",
        id: "winter_weather",
        afreq_kind: "events_per_year",
        basis: "event_days",
        unit: "winter-weather days per year (NWS alerts), area-weighted",
        period: "19.0 years (2005-11-12 to 2024-11-12)",
        poisson_ok: false,
        section: "23.5",
        notes: "Event-days per year. Days cluster into storms: divide by a typical storm length before treating the result as storms per year.",
    },
];

/// Extract FEMA's required disclaimer (the quoted sentence after "Users must clearly state that").
pub fn extract_disclaimer(license_html: &str) -> Option<String> {
    let text = strip_html(license_html);
    let start_marker = text.find("clearly state that")?;
    let rest = &text[start_marker..];
    let open = rest.find(['\u{201c}', '"'])?;
    let after = &rest[open + rest[open..].chars().next()?.len_utf8()..];
    let close = after.find(['\u{201d}', '"'])?;
    let sentence = after[..close]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if sentence.len() < 40 {
        None
    } else {
        Some(sentence)
    }
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&rsquo;", "\u{2019}")
        .replace("&ldquo;", "\u{201c}")
        .replace("&rdquo;", "\u{201d}")
}

/// Run the job.
pub fn run(ctx: &Ctx) -> Result<JobOutput> {
    let mut out = JobOutput::default();
    let counties = load_counties(ctx)?;
    for s in SEMANTICS {
        rr_types::HazardId::from_str(s.id)
            .map_err(|_| data_err(format!("semantics table uses unknown hazard id {}", s.id)))?;
    }

    // Terms and the required disclaimer.
    let item = ctx.http.get(ITEM, Some("nri/item.json"))?;
    let meta: serde_json::Value = serde_json::from_slice(&item.bytes)?;
    let license = meta["licenseInfo"].as_str().unwrap_or_default();
    let disclaimer = extract_disclaimer(license)
        .ok_or_else(|| data_err("could not find FEMA's required NRI disclaimer in the item's Terms & Conditions; read them and update the job"))?;

    // Version check against the data dictionary.
    let dict = ctx
        .http
        .get(DICTIONARY, Some("nri/NRIDataDictionary.csv"))?;
    let (dh, drows) = crate::csvout::parse_delimited(&dict.text(), b',')?;
    let (di_field, di_ver, di_date) = (
        crate::csvout::col(&dh, "Field Name")?,
        crate::csvout::col(&dh, "Version")?,
        crate::csvout::col(&dh, "Version Date")?,
    );
    let version = drows.first().map(|r| r[di_ver].clone()).unwrap_or_default();
    let version_date = drows
        .first()
        .map(|r| r[di_date].clone())
        .unwrap_or_default();
    if !version.starts_with(EXPECTED_VERSION) {
        return Err(data_err(format!(
            "NRI data dictionary reports version {version} ({version_date}); this job's semantics table is for {EXPECTED_VERSION}. Review the technical documentation for changed field meanings, update SEMANTICS, then change EXPECTED_VERSION."
        )));
    }
    let dict_fields: BTreeSet<String> = drows.iter().map(|r| r[di_field].clone()).collect();

    let mut fields: Vec<String> = vec!["STCOFIPS".into(), "NRI_VER".into()];
    fields.extend(COUNTY_FIELDS.iter().map(|(f, _)| f.to_string()));
    for f in &fields {
        if !dict_fields.contains(f) {
            return Err(data_err(format!(
                "NRI data dictionary no longer lists field {f}"
            )));
        }
    }
    // Not every hazard has every field (drought is modelled for agriculture only, so it has no
    // building or population fields). Request what exists; absent fields stay empty.
    let mut absent = Vec::new();
    for s in SEMANTICS {
        for (suffix, _) in HAZARD_FIELDS {
            let f = format!("{}_{}", s.code, suffix);
            if dict_fields.contains(&f) {
                fields.push(f);
            } else {
                absent.push(f);
            }
        }
    }
    if absent.len() > 12 {
        return Err(data_err(format!(
            "NRI data dictionary lacks {} expected hazard fields (e.g. {:?}); the layout changed",
            absent.len(),
            &absent[..5]
        )));
    }
    if !absent.is_empty() {
        out.notes.push(format!("Fields that NRI v1.20 does not publish (left empty): {}. Drought is modelled for agriculture only.", absent.join(", ")));
    }

    // Page through the layer.
    let query_url = format!("{LAYER}/query");
    let mut acc = Sha256Acc::new();
    let mut records: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
    let page = 1000;
    let first_retrieved = crate::timefmt::now_utc();
    loop {
        let form = vec![
            ("where", "1=1".to_string()),
            ("outFields", fields.join(",")),
            ("orderByFields", "STCOFIPS".to_string()),
            ("resultOffset", records.len().to_string()),
            ("resultRecordCount", page.to_string()),
            ("returnGeometry", "false".to_string()),
            ("f", "json".to_string()),
        ];
        let resp = ctx.http.post_form(&query_url, &form)?;
        acc.update(&resp.bytes);
        let v: serde_json::Value = serde_json::from_slice(&resp.bytes)?;
        if let Some(err) = v.get("error") {
            return Err(data_err(format!("NRI query error: {err}")));
        }
        let feats = v["features"]
            .as_array()
            .ok_or_else(|| data_err("NRI query: no features array"))?;
        let n = feats.len();
        for f in feats {
            if let Some(a) = f["attributes"].as_object() {
                records.push(a.clone());
            }
        }
        let more = v["exceededTransferLimit"].as_bool().unwrap_or(false);
        if n == 0 || (!more && n < page) {
            break;
        }
    }
    let nri_ver = records
        .first()
        .and_then(|r| r.get("NRI_VER"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let bytes = acc.len();
    out.source(SourceRecord {
        name: "FEMA National Risk Index, counties (ArcGIS FeatureServer)".into(),
        url: format!("{query_url} (POST where=1=1, {} fields, ordered by STCOFIPS, pages of {page})", fields.len()),
        version: format!("v{version} ({version_date}); NRI_VER field \"{nri_ver}\"; item modified {}", meta["modified"]),
        retrieved: first_retrieved.clone(),
        sha256: acc.finish(),
        bytes,
        license: "US Government data subject to the NRI Terms & Conditions (see item page)".into(),
        obligations: format!("Cite the dataset with version and access date; show this statement: {disclaimer} Do not present modified data as FEMA's; FEMA may rescind use."),
    });
    out.source(super::source_from(
        "NRI item metadata and Terms & Conditions",
        &item,
        format!("item modified {}", meta["modified"]),
        "Terms & Conditions",
        "",
    ));
    out.source(super::source_from(
        "NRI data dictionary",
        &dict,
        format!("{version} ({version_date})"),
        "Terms & Conditions",
        "",
    ));
    out.rows_in = records.len() as u64;

    // Build tables.
    let canon: BTreeSet<String> = counties.iter().map(|c| c.fips.clone()).collect();
    let mut ct = Table::with_header(
        std::iter::once("fips".to_string())
            .chain(COUNTY_FIELDS.iter().map(|(_, c)| c.to_string()))
            .chain(["coastal".to_string(), "tsunami_zone".to_string()])
            .collect(),
        1,
    );
    let mut ht = Table::with_header(
        ["fips", "hazard"]
            .iter()
            .map(|s| s.to_string())
            .chain(HAZARD_FIELDS.iter().map(|(_, c)| c.to_string()))
            .collect(),
        2,
    );
    let mut covered = BTreeSet::new();
    let mut extra = Vec::new();
    let mut coastal_n = 0;
    let mut tsunami_n = 0;
    let get = |r: &serde_json::Map<String, serde_json::Value>, k: &str| {
        r.get(k).and_then(|v| v.as_f64()).filter(|v| v.is_finite())
    };
    for r in &records {
        let fips = r
            .get("STCOFIPS")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if !canon.contains(&fips) {
            extra.push(fips);
            continue;
        }
        covered.insert(fips.clone());
        let coastal = get(r, "CFLD_EXPB").is_some_and(|v| v > 0.0);
        let tsunami = get(r, "TSUN_EXPB").is_some_and(|v| v > 0.0)
            || get(r, "TSUN_EXPP").is_some_and(|v| v > 0.0);
        coastal_n += coastal as u32;
        tsunami_n += tsunami as u32;
        let mut row = vec![fips.clone()];
        row.extend(COUNTY_FIELDS.iter().map(|(f, _)| opt4(get(r, f))));
        row.push(coastal.to_string());
        row.push(tsunami.to_string());
        ct.push(row);
        for s in SEMANTICS {
            let vals: Vec<Option<f64>> = HAZARD_FIELDS
                .iter()
                .map(|(suf, _)| get(r, &format!("{}_{}", s.code, suf)))
                .collect();
            if vals.iter().all(|v| v.is_none()) {
                continue;
            }
            let mut hr = vec![fips.clone(), s.id.to_string()];
            hr.extend(vals.into_iter().map(opt4));
            ht.push(hr);
        }
    }
    if !extra.is_empty() {
        out.notes.push(format!(
            "NRI records not in the canonical county list were skipped: {}.",
            extra.join(", ")
        ));
    }
    out.table(ctx, NRI_COUNTIES, &mut ct)?;
    out.table(ctx, NRI_HAZARDS, &mut ht)?;
    let semantics_text = semantics_toml(&version, &version_date, &nri_ver);
    out.text(ctx, NRI_SEMANTICS, &semantics_text, SEMANTICS.len() as u64)?;

    out.missing = missing_groups(&counties, &covered, |_| {
        "No National Risk Index record for this county".to_string()
    });
    let date = &first_retrieved[..10];
    let citation = format!(
        "Federal Emergency Management Agency (FEMA), National Risk Index Dataset: National Risk Index Counties - v{version} ({version_date}). Retrieved from {ITEM_PAGE} on {date} (UTC)."
    );
    out.definitions
        .insert("nri_version".into(), format!("{version} ({version_date})"));
    out.definitions
        .insert("nri_citation".into(), citation.clone());
    out.definitions
        .insert("nri_disclaimer".into(), disclaimer.clone());
    out.definitions.insert(
        "coastal".into(),
        "A county is coastal when NRI v1.20 assigns it coastal-flooding building exposure (CFLD_EXPB > 0).".into(),
    );
    out.definitions.insert(
        "tsunami_zone".into(),
        "A county has a tsunami zone when NRI v1.20 assigns it tsunami building or population exposure (TSUN_EXPB > 0 or TSUN_EXPP > 0).".into(),
    );
    out.notes.push(format!(
        "{} counties, {} hazard rows. Kept per hazard: AFREQ, EXPB, EXPP, EALB, EALP, EALT, HLRB, ALRB, RISKS; per county: POPULATION, BUILDVALUE, EAL_VALT, SOVI_SCORE, RESL_SCORE, CRF_VALUE. Values rounded to 4 significant figures. Hazard rows where every field is empty (hazard not applicable) are omitted.",
        ct.rows.len(),
        ht.rows.len()
    ));
    out.notes.push(format!("{coastal_n} counties are coastal and {tsunami_n} have a tsunami zone by the definitions above (derived by Ready Reckoner, not FEMA fields)."));
    out.notes.push("Terms compliance: only the trimmed, rounded county fields the model uses are shipped (never the raw table); the About screen and the packet show the citation and disclaimer; numbers Ready Reckoner derives from NRI are labelled as ours.".into());
    out.attributions.push(Attribution {
        source: "FEMA National Risk Index".into(),
        text: format!("{citation} {disclaimer}"),
        license: "US Government data; NRI Terms & Conditions".into(),
        url: ITEM_PAGE.into(),
        version: Some(format!("{version} ({version_date})")),
        accessed: date.to_string(),
    });
    Ok(out)
}

fn toml_str(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

fn semantics_toml(version: &str, version_date: &str, nri_ver: &str) -> String {
    let mut s = String::new();
    s.push_str("# What each hazard's NRI annualised frequency (AFREQ) means, so rr-hazards never guesses.\n");
    s.push_str("# Written by rr-etl (job `nri`) from the NRI Technical Documentation, December 2025 (v1.20),\n");
    s.push_str("# section 5.2 Table 5 and each hazard's \"Annualized Frequency\" section. fema.gov refuses scripted\n");
    s.push_str(
        "# clients, so the document was read from the Internet Archive snapshot named below.\n",
    );
    s.push_str("#\n# afreq_kind: events_per_year (can exceed 1) or annual_probability (0 to 1).\n");
    s.push_str("# basis: distinct_events (counted events), event_days (counted days; episodes span several days)\n");
    s.push_str("#        or modelled (from a hazard model, not a count).\n");
    s.push_str("# poisson_ok: true when 1 - exp(-afreq) is a sound chance of at least one event in a year.\n\n");
    s.push_str(&format!("nri_version = {}\n", toml_str(version)));
    s.push_str(&format!("nri_version_date = {}\n", toml_str(version_date)));
    s.push_str(&format!("nri_ver_field = {}\n", toml_str(nri_ver)));
    s.push_str(&format!(
        "technical_documentation = {}\n",
        toml_str(TECH_DOC)
    ));
    s.push_str(&format!(
        "technical_documentation_archive = {}\n",
        toml_str(TECH_DOC_ARCHIVE)
    ));
    for h in SEMANTICS {
        s.push_str("\n[[hazard]]\n");
        s.push_str(&format!("id = {}\n", toml_str(h.id)));
        s.push_str(&format!("nri_code = {}\n", toml_str(h.code)));
        s.push_str(&format!("afreq_kind = {}\n", toml_str(h.afreq_kind)));
        s.push_str(&format!("basis = {}\n", toml_str(h.basis)));
        s.push_str(&format!("unit = {}\n", toml_str(h.unit)));
        s.push_str(&format!("period_of_record = {}\n", toml_str(h.period)));
        s.push_str(&format!("poisson_ok = {}\n", h.poisson_ok));
        s.push_str(&format!("section = {}\n", toml_str(h.section)));
        s.push_str(&format!("notes = {}\n", toml_str(h.notes)));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fema_disclaimer() {
        let html = "<p>Users must clearly state that \u{201c}This product uses the Federal Emergency Management Agency\u{2019}s National Risk Index dataset API or downloadable datasets but is not endorsed by FEMA. The Federal Government or FEMA cannot vouch for the data or analyses derived from these data after the data have been retrieved from the Agency's website(s).\u{201d}</p>";
        let d = extract_disclaimer(html).unwrap();
        assert!(d.starts_with("This product uses the Federal Emergency Management Agency"));
        assert!(d.ends_with("website(s)."));
    }

    #[test]
    fn semantics_cover_all_18_hazards_once() {
        let ids: BTreeSet<&str> = SEMANTICS.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), 18);
        let natural: BTreeSet<&str> = rr_types::HazardId::ALL
            .iter()
            .filter(|h| h.tier() == rr_types::HazardTier::Natural)
            .map(|h| h.as_str())
            .collect();
        assert_eq!(ids, natural);
        for s in SEMANTICS {
            assert!(s.afreq_kind == "events_per_year" || s.afreq_kind == "annual_probability");
        }
        let toml_text = semantics_toml("1.20.0", "December 2025", "December 2025");
        let v: toml::Table = toml_text.parse().unwrap();
        assert_eq!(v["hazard"].as_array().unwrap().len(), 18);
    }

    #[test]
    fn num_helper_parses() {
        assert_eq!(crate::jobs::num(" 1.5 "), Some(1.5));
        assert_eq!(crate::jobs::num(""), None);
    }
}
