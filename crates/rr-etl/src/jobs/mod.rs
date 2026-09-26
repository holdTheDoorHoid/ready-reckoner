//! The ETL jobs, in the order `refresh` runs them, and the driver that records their provenance.

use crate::csvout::{Table, Written, col, read_table, write_table, write_text};
use crate::http::Http;
use crate::manifest::{Attribution, FileEntry, JobRecord, Manifest, Missing, Pack, SourceRecord};
use crate::{Result, data_err};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub mod base_rates;
pub mod climate;
pub mod events;
pub mod eviction;
pub mod facilities;
pub mod flood;
pub mod geography;
pub mod geomag;
pub mod ground;
pub mod levees;
pub mod nri;
pub mod outages;
pub mod seismic;
pub mod smoke;
pub mod strategic;
pub mod surge;
pub mod surge_proxy;
pub mod vulnerability;
pub mod water_systems;
pub mod wildfire_places;

/// Shared state for a refresh.
pub struct Ctx {
    /// The data directory (`data/`).
    pub data: PathBuf,
    /// HTTP client (knows about `--keep-raw`).
    pub http: Http,
}

/// Everything a job reports back.
#[derive(Debug, Default)]
pub struct JobOutput {
    /// Raw inputs used.
    pub sources: Vec<SourceRecord>,
    /// Files written.
    pub written: Vec<Written>,
    /// Relevant rows read from the sources.
    pub rows_in: u64,
    /// Methodology notes.
    pub notes: Vec<String>,
    /// Exact definitions downstream code must quote.
    pub definitions: BTreeMap<String, String>,
    /// Counties without data, by reason.
    pub missing: Vec<Missing>,
    /// Credit lines this job's sources require.
    pub attributions: Vec<Attribution>,
}

impl JobOutput {
    /// Write a table and remember it.
    pub fn table(&mut self, ctx: &Ctx, rel: &str, table: &mut Table) -> Result<()> {
        table.sort_and_check()?;
        let w = write_table(&ctx.data, rel, table)?;
        eprintln!("  wrote {rel}: {} rows, {} bytes", w.rows, w.bytes);
        self.written.push(w);
        Ok(())
    }

    /// Write a text file (TOML/JSON) and remember it.
    pub fn text(&mut self, ctx: &Ctx, rel: &str, text: &str, rows: u64) -> Result<()> {
        let w = write_text(&ctx.data, rel, text, rows)?;
        eprintln!("  wrote {rel}: {} entries, {} bytes", w.rows, w.bytes);
        self.written.push(w);
        Ok(())
    }

    /// Record a source.
    pub fn source(&mut self, s: SourceRecord) {
        self.sources.push(s);
    }
}

/// A job in the registry.
pub struct JobSpec {
    /// Id used with `--only`.
    pub id: &'static str,
    /// Plain-language title for the manifest.
    pub title: &'static str,
    /// Entry point.
    pub run: fn(&Ctx) -> Result<JobOutput>,
    /// Runs in a plain `refresh` (and so in the quarterly Action). Heavy one-off jobs that build
    /// optional packs are `false`: they run only when named with `--only` or with `--optional`.
    pub default: bool,
}

/// All jobs, in run order (later jobs read earlier jobs' outputs, e.g. the county list).
pub const JOBS: &[JobSpec] = &[
    JobSpec {
        id: "geography",
        title: "Counties, ZIP-to-county shares, states and the Connecticut crosswalk",
        run: geography::run,
        default: true,
    },
    JobSpec {
        id: "nri",
        title: "FEMA National Risk Index v1.20, trimmed to the model's county fields",
        run: nri::run,
        default: true,
    },
    JobSpec {
        id: "outages",
        title: "Power outage frequency and duration per county from ORNL EAGLE-I 2014-2025",
        run: outages::run,
        default: true,
    },
    JobSpec {
        id: "events",
        title: "Event rates per county from HURDAT2, SPC severe reports and NOAA Storm Events",
        run: events::run,
        default: true,
    },
    JobSpec {
        id: "seismic",
        title: "USGS NSHM ground-shaking exceedance at county centroids",
        run: seismic::run,
        default: true,
    },
    JobSpec {
        id: "climate",
        title: "Climate change multipliers from the NCA5 Atlas, LOCA2 and CMRA (projections)",
        run: climate::run,
        default: true,
    },
    JobSpec {
        id: "flood",
        title: "Flood priors from OpenFEMA NFIP penetration rates and claims v3",
        run: flood::run,
        default: true,
    },
    JobSpec {
        id: "strategic",
        title: "Strategic-site classes (nuclear family) and FEMA UASI shares by county",
        run: strategic::run,
        default: true,
    },
    JobSpec {
        id: "geomag",
        title: "Geomagnetic latitude (IGRF-14) and the NERC TPL-007 scaling factor by county",
        run: geomag::run,
        default: true,
    },
    JobSpec {
        id: "ground",
        title: "Karst (USGS OFR 2014-1156) and landslide-susceptible terrain (USGS 2024) by county",
        run: ground::run,
        default: true,
    },
    JobSpec {
        id: "levees",
        title: "People behind levees and levee risk ratings by county (USACE National Levee Database)",
        run: levees::run,
        default: true,
    },
    JobSpec {
        id: "water_systems",
        title: "Community water systems with health-based violations in five years (EPA ECHO SDWA)",
        run: water_systems::run,
        default: true,
    },
    JobSpec {
        id: "smoke",
        title: "Wildfire-smoke days per county from NOAA HMS smoke polygons and EPA AQS PM2.5",
        run: smoke::run,
        default: true,
    },
    JobSpec {
        id: "facilities",
        title: "Nuclear plants, TRI facilities and high-hazard dams by county and ZIP",
        run: facilities::run,
        default: true,
    },
    JobSpec {
        id: "surge_proxy",
        title: "County storm-surge proxy from NRI coastal-flood exposure, hurricane passages and evacuation zones",
        run: surge_proxy::run,
        default: true,
    },
    JobSpec {
        id: "eviction",
        title: "Eviction filings per renter household (Eviction Lab; written only with the owner's sign-off)",
        run: eviction::run,
        default: true,
    },
    JobSpec {
        id: "surge",
        title: "NOAA/NHC storm-surge area shares by ZIP (optional pack; large rasters)",
        run: surge::run,
        default: false,
    },
    JobSpec {
        id: "wildfire_places",
        title: "USFS Wildfire Risk to Communities by Census place, with ZIP-to-place shares (optional pack)",
        run: wildfire_places::run,
        default: false,
    },
    JobSpec {
        id: "vulnerability",
        title: "Census Community Resilience Estimates 2024 and CDC SVI 2022",
        run: vulnerability::run,
        default: true,
    },
    JobSpec {
        id: "base_rates",
        title: "National personal-risk base rates with sources",
        run: base_rates::run,
        default: true,
    },
];

/// Pack a file belongs to, from its path: `geo/...` is the map, `opt/<name>/...` is the
/// optional pack `<name>` (issue #15: loaded only when a feature asks for it), anything else is
/// `core`.
pub fn pack_of(path: &str) -> String {
    if path.starts_with("geo/") {
        "geo".to_string()
    } else if let Some(rest) = path.strip_prefix("opt/") {
        rest.split('/').next().unwrap_or("opt").to_string()
    } else {
        "core".to_string()
    }
}

/// Pack descriptions for the manifest.
pub fn pack_description(name: &str) -> &'static str {
    match name {
        "geo" => "County boundaries for the map thumbnail and click-to-select. Loaded lazily.",
        "core" => {
            "Everything the engine needs for county-level planning. Loaded at start; works offline."
        }
        "wildfire_places" => {
            "Optional: USFS Wildfire Risk to Communities by Census place, with the ZIP-to-place shares to use it. Not needed to plan; loaded only when the wildfire detail is shown."
        }
        "surge" => {
            "Optional: share of each ZIP code's land inside NOAA/NHC's Category 1 and Category 3 storm-surge areas. Built by hand (rr-etl refresh --only surge), not in the quarterly refresh. Not needed to plan."
        }
        _ => "Optional pack (not needed to plan; loaded only when a feature asks for it).",
    }
}

/// A county from the canonical list (`data/core/counties.csv`).
#[derive(Debug, Clone, PartialEq)]
pub struct County {
    /// Five-digit FIPS.
    pub fips: String,
    /// Short name ("Philadelphia").
    pub name: String,
    /// State abbreviation.
    pub state_abbr: String,
    /// Internal point latitude.
    pub lat: f64,
    /// Internal point longitude.
    pub lon: f64,
}

/// Load the canonical county list written by the geography job.
pub fn load_counties(ctx: &Ctx) -> Result<Vec<County>> {
    let (h, rows) = read_table(&ctx.data, geography::COUNTIES)?;
    let (i_f, i_n, i_s, i_lat, i_lon) = (
        col(&h, "fips")?,
        col(&h, "name")?,
        col(&h, "state_abbr")?,
        col(&h, "lat")?,
        col(&h, "lon")?,
    );
    rows.iter()
        .map(|r| {
            Ok(County {
                fips: r[i_f].clone(),
                name: r[i_n].clone(),
                state_abbr: r[i_s].clone(),
                lat: r[i_lat]
                    .parse()
                    .map_err(|_| data_err(format!("bad lat for {}", r[i_f])))?,
                lon: r[i_lon]
                    .parse()
                    .map_err(|_| data_err(format!("bad lon for {}", r[i_f])))?,
            })
        })
        .collect()
}

/// Group the counties that a job did not cover by a reason chosen per county.
pub fn missing_groups(
    counties: &[County],
    covered: &BTreeSet<String>,
    reason: impl Fn(&County) -> String,
) -> Vec<Missing> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for c in counties {
        if !covered.contains(&c.fips) {
            groups.entry(reason(c)).or_default().push(c.fips.clone());
        }
    }
    groups
        .into_iter()
        .map(|(reason, mut fips)| {
            fips.sort();
            Missing { reason, fips }
        })
        .collect()
}

/// States and territories outside the contiguous US (plus DC is inside).
pub fn is_outside_conus(state_abbr: &str) -> bool {
    matches!(
        state_abbr,
        "AK" | "HI" | "PR" | "VI" | "GU" | "AS" | "MP" | "UM"
    )
}

/// Island territories other than Puerto Rico.
pub fn is_island_territory(state_abbr: &str) -> bool {
    matches!(state_abbr, "VI" | "GU" | "AS" | "MP" | "UM")
}

/// Outcome of a refresh.
#[derive(Debug, Default)]
pub struct RefreshSummary {
    /// Jobs that finished.
    pub ok: Vec<String>,
    /// Jobs that failed, with the error.
    pub failed: Vec<(String, String)>,
    /// Per-file change summaries.
    pub changes: Vec<Written>,
}

/// Run the selected jobs, update the manifest and `data/CHANGES.md`. With `only` empty every
/// default job runs, plus the optional ones when `optional` is set; a job named in `only` runs
/// whether it is a default job or not.
pub fn refresh(ctx: &Ctx, only: &[String], optional: bool) -> Result<RefreshSummary> {
    for id in only {
        if !JOBS.iter().any(|j| j.id == id) {
            return Err(data_err(format!(
                "unknown job {id}; jobs are: {}",
                JOBS.iter().map(|j| j.id).collect::<Vec<_>>().join(", ")
            )));
        }
    }
    std::fs::create_dir_all(ctx.data.join("core"))?;
    std::fs::create_dir_all(ctx.data.join("geo"))?;
    let mut manifest = Manifest::load_or_default(&ctx.data)?;
    manifest.schema = crate::manifest::SCHEMA;
    // Decisions a job waits for: present (unapproved) so a person knows where to approve.
    manifest
        .sign_offs
        .entry(eviction::SIGN_OFF.to_string())
        .or_insert_with(|| crate::manifest::SignOff {
            approved: false,
            what: eviction::SIGN_OFF_WHAT.to_string(),
            by: String::new(),
        });
    let mut summary = RefreshSummary::default();
    for job in JOBS {
        let selected = if only.is_empty() {
            job.default || optional
        } else {
            only.iter().any(|o| o == job.id)
        };
        if !selected {
            continue;
        }
        eprintln!("== {} ({})", job.id, job.title);
        let started = std::time::Instant::now();
        match (job.run)(ctx) {
            Ok(out) => {
                eprintln!("   done in {:.0} s", started.elapsed().as_secs_f64());
                let previous: Vec<FileEntry> = manifest
                    .packs
                    .values()
                    .flat_map(|p| p.files.iter())
                    .filter(|f| f.job == job.id)
                    .cloned()
                    .collect();
                let (changes, stale) = reconcile(&previous, &out.written);
                for path in &stale {
                    crate::http::remove_raw(&ctx.data.join(path))?;
                    eprintln!("  removed {path} (no longer written by {})", job.id);
                }
                record(&mut manifest, job, &out);
                summary.changes.extend(changes);
                summary.ok.push(job.id.to_string());
                // Save after every job so partial progress is never lost.
                finish_manifest(&mut manifest);
                manifest.save(&ctx.data)?;
            }
            Err(e) => {
                eprintln!("   FAILED: {e}");
                summary.failed.push((job.id.to_string(), e.to_string()));
            }
        }
    }
    finish_manifest(&mut manifest);
    manifest.save(&ctx.data)?;
    crate::changes::append(&ctx.data, &manifest, &summary)?;
    Ok(summary)
}

/// Compare a job's new files with the files it wrote last time: fill in the previous row count
/// for rewritten text files (their diff cannot count rows) and list files the job no longer
/// writes, as `removed` entries for `CHANGES.md` and as paths to delete.
fn reconcile(previous: &[FileEntry], written: &[Written]) -> (Vec<Written>, Vec<String>) {
    let mut changes: Vec<Written> = written.to_vec();
    for w in &mut changes {
        if !w.diff.new_file && !w.diff.identical && w.diff.previous_rows == 0 {
            if let Some(p) = previous.iter().find(|p| p.path == w.path) {
                w.diff.previous_rows = p.rows;
            }
        }
    }
    let mut stale = Vec::new();
    for p in previous {
        if written.iter().any(|w| w.path == p.path) {
            continue;
        }
        stale.push(p.path.clone());
        changes.push(Written {
            path: p.path.clone(),
            rows: 0,
            bytes: 0,
            sha256: String::new(),
            key: p.key.clone(),
            diff: crate::csvout::DiffSummary {
                previous_rows: p.rows,
                removed: p.rows,
                removed_file: true,
                ..Default::default()
            },
        });
    }
    (changes, stale)
}

fn record(manifest: &mut Manifest, job: &JobSpec, out: &JobOutput) {
    // Drop this job's previous files and attributions, then add the new ones.
    for pack in manifest.packs.values_mut() {
        pack.files.retain(|f| f.job != job.id);
    }
    for w in &out.written {
        let name = pack_of(&w.path);
        let pack = manifest.packs.entry(name.clone()).or_insert_with(|| Pack {
            description: pack_description(&name).to_string(),
            files: Vec::new(),
        });
        pack.files.push(FileEntry {
            path: w.path.clone(),
            job: job.id.to_string(),
            sha256: w.sha256.clone(),
            bytes: w.bytes,
            rows: w.rows,
            key: w.key.clone(),
        });
        pack.files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    let mut attributions: Vec<Attribution> = manifest
        .attributions
        .iter()
        .filter(|a| !out.attributions.iter().any(|b| b.source == a.source))
        .cloned()
        .collect();
    attributions.extend(out.attributions.iter().cloned());
    // A job whose files all sit in one optional pack: its credit lines belong to that pack.
    let packs: BTreeSet<String> = out.written.iter().map(|w| pack_of(&w.path)).collect();
    for a in &out.attributions {
        match packs.iter().next() {
            Some(p) if packs.len() == 1 && p != "core" && p != "geo" => {
                manifest
                    .attribution_packs
                    .insert(a.source.clone(), p.clone());
            }
            _ => {
                manifest.attribution_packs.remove(&a.source);
            }
        }
    }
    attributions.sort_by(|a, b| a.source.cmp(&b.source));
    manifest.attributions = attributions;
    manifest.jobs.insert(
        job.id.to_string(),
        JobRecord {
            title: job.title.to_string(),
            finished: crate::timefmt::now_utc(),
            sources: out.sources.clone(),
            rows_in: out.rows_in,
            rows_out: out.written.iter().map(|w| w.rows).sum(),
            outputs: out.written.iter().map(|w| w.path.clone()).collect(),
            notes: out.notes.clone(),
            definitions: out.definitions.clone(),
            missing: out.missing.clone(),
        },
    );
}

fn finish_manifest(manifest: &mut Manifest) {
    manifest.packs.retain(|_, p| !p.files.is_empty());
    for (name, pack) in manifest.packs.iter_mut() {
        pack.description = pack_description(name).to_string();
    }
    manifest.recompute_version();
    manifest.generated = crate::timefmt::now_utc();
}

/// Parse a number cell, treating empty as `None`.
pub fn num(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok().filter(|v| v.is_finite())
    }
}

/// Build a [`SourceRecord`] for a fetched document.
pub fn source_from(
    name: &str,
    f: &crate::http::Fetched,
    version: impl Into<String>,
    license: &str,
    obligations: &str,
) -> SourceRecord {
    SourceRecord {
        name: name.to_string(),
        url: f.final_url.clone(),
        version: version.into(),
        retrieved: f.retrieved.clone(),
        sha256: f.sha256.clone(),
        bytes: f.bytes.len() as u64,
        license: license.to_string(),
        obligations: obligations.to_string(),
    }
}

/// Licence text for US Government works.
pub const PUBLIC_DOMAIN: &str = "US Government work, public domain (17 U.S.C. 105)";

/// Result of a paged ArcGIS FeatureServer query.
pub struct ArcgisRows {
    /// Attribute maps, in the order returned (ordered by `order_by`).
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    /// Geometries (when requested), parallel to `rows`.
    pub geometry: Vec<serde_json::Value>,
    /// Source record for the manifest (sha256 over all pages in request order).
    pub source: SourceRecord,
}

/// Page through an ArcGIS FeatureServer layer with POST queries.
#[allow(clippy::too_many_arguments)]
pub fn arcgis_query(
    ctx: &Ctx,
    name: &str,
    layer_url: &str,
    where_clause: &str,
    fields: &[&str],
    order_by: &str,
    geometry: bool,
    version: &str,
    license: &str,
    obligations: &str,
) -> Result<ArcgisRows> {
    arcgis_query_ex(
        ctx,
        name,
        layer_url,
        where_clause,
        fields,
        order_by,
        geometry,
        &[],
        version,
        license,
        obligations,
    )
}

/// [`arcgis_query`] with extra form parameters (for example `maxAllowableOffset` and
/// `geometryPrecision` to generalise polygons on the server). The extra parameters are recorded
/// in the source URL.
#[allow(clippy::too_many_arguments)]
pub fn arcgis_query_ex(
    ctx: &Ctx,
    name: &str,
    layer_url: &str,
    where_clause: &str,
    fields: &[&str],
    order_by: &str,
    geometry: bool,
    extra: &[(&str, String)],
    version: &str,
    license: &str,
    obligations: &str,
) -> Result<ArcgisRows> {
    let url = format!("{layer_url}/query");
    let mut acc = crate::http::Sha256Acc::new();
    let mut rows = Vec::new();
    let mut geoms = Vec::new();
    let retrieved = crate::timefmt::now_utc();
    let page = 1000usize;
    loop {
        let mut form = vec![
            ("where", where_clause.to_string()),
            ("outFields", fields.join(",")),
            ("orderByFields", order_by.to_string()),
            ("resultOffset", rows.len().to_string()),
            ("resultRecordCount", page.to_string()),
            ("returnGeometry", geometry.to_string()),
            ("f", "json".to_string()),
        ];
        if geometry {
            form.push(("outSR", "4326".to_string()));
        }
        form.extend(extra.iter().cloned());
        let resp = ctx.http.post_form(&url, &form)?;
        acc.update(&resp.bytes);
        let v: serde_json::Value = serde_json::from_slice(&resp.bytes)?;
        if let Some(err) = v.get("error") {
            return Err(data_err(format!("{name}: ArcGIS query error: {err}")));
        }
        let feats = v["features"]
            .as_array()
            .ok_or_else(|| data_err(format!("{name}: no features array")))?;
        let n = feats.len();
        for f in feats {
            if let Some(a) = f["attributes"].as_object() {
                rows.push(a.clone());
                geoms.push(
                    f.get("geometry")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                );
            }
        }
        let more = v["exceededTransferLimit"].as_bool().unwrap_or(false);
        if n == 0 || (!more && n < page) {
            break;
        }
    }
    let bytes = acc.len();
    Ok(ArcgisRows {
        source: SourceRecord {
            name: name.to_string(),
            url: format!(
                "{url} (POST where={where_clause}; fields {}; ordered by {order_by}{})",
                fields.join(","),
                extra
                    .iter()
                    .map(|(k, v)| format!("; {k}={v}"))
                    .collect::<String>()
            ),
            version: version.to_string(),
            retrieved,
            sha256: acc.finish(),
            bytes,
            license: license.to_string(),
            obligations: obligations.to_string(),
        },
        rows,
        geometry: geoms,
    })
}

/// A numeric attribute from an ArcGIS row.
pub fn attr_f64(row: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<f64> {
    match row.get(key)? {
        serde_json::Value::Number(n) => n.as_f64().filter(|v| v.is_finite()),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok().filter(|v| v.is_finite()),
        _ => None,
    }
}

/// A string attribute from an ArcGIS row (numbers are formatted).
pub fn attr_str(row: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    match row.get(key)? {
        serde_json::Value::String(s) => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csvout::DiffSummary;

    fn entry(path: &str, rows: u64) -> FileEntry {
        FileEntry {
            path: path.into(),
            job: "climate".into(),
            sha256: "x".into(),
            bytes: 1,
            rows,
            key: vec!["fips".into()],
        }
    }

    fn written(path: &str, rows: u64, diff: DiffSummary) -> Written {
        Written {
            path: path.into(),
            rows,
            bytes: 1,
            sha256: "y".into(),
            key: vec!["fips".into()],
            diff,
        }
    }

    #[test]
    fn optional_packs_have_their_own_names_and_jobs_are_registered_once() {
        assert_eq!(pack_of("core/smoke.csv"), "core");
        assert_eq!(pack_of("geo/counties.json"), "geo");
        assert_eq!(pack_of("opt/surge/zip_surge.csv"), "surge");
        assert_eq!(pack_of("opt/wildfire_places/places.csv"), "wildfire_places");
        let mut ids: Vec<&str> = JOBS.iter().map(|j| j.id).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate job ids");
        let optional: Vec<&str> = JOBS.iter().filter(|j| !j.default).map(|j| j.id).collect();
        assert_eq!(optional, vec!["surge", "wildfire_places"]);
        // Jobs that read other jobs' files come after them.
        let pos = |id: &str| JOBS.iter().position(|j| j.id == id).unwrap();
        assert!(pos("strategic") < pos("facilities"));
        assert!(pos("events") < pos("surge_proxy") && pos("nri") < pos("surge_proxy"));
        assert!(pos("nri") < pos("levees") && pos("nri") < pos("water_systems"));
    }

    #[test]
    fn optional_pack_jobs_tag_their_credit_lines() {
        let credit = |source: &str| Attribution {
            source: source.into(),
            text: "t".into(),
            license: "l".into(),
            url: "u".into(),
            version: None,
            accessed: "2026-09-26".into(),
        };
        let job = |id: &'static str| JobSpec {
            id,
            title: "t",
            run: |_| Ok(JobOutput::default()),
            default: false,
        };
        let mut m = Manifest::default();
        let places = JobOutput {
            written: vec![written(
                "opt/wildfire_places/places.csv",
                1,
                DiffSummary::default(),
            )],
            attributions: vec![credit("USFS")],
            ..Default::default()
        };
        record(&mut m, &job("wildfire_places"), &places);
        let smoke = JobOutput {
            written: vec![written("core/smoke.csv", 1, DiffSummary::default())],
            attributions: vec![credit("HMS")],
            ..Default::default()
        };
        record(&mut m, &job("smoke"), &smoke);
        assert_eq!(m.attribution_packs.len(), 1);
        assert_eq!(m.attribution_packs["USFS"], "wildfire_places");
        assert_eq!(m.attributions.len(), 2);
        // A source that moves into the core pack loses its optional tag.
        let moved = JobOutput {
            written: vec![written("core/places.csv", 1, DiffSummary::default())],
            attributions: vec![credit("USFS")],
            ..Default::default()
        };
        record(&mut m, &job("wildfire_places"), &moved);
        assert!(m.attribution_packs.is_empty());
    }

    #[test]
    fn reconcile_lists_dropped_files_and_fills_text_row_counts() {
        let previous = [
            entry("core/climate.csv", 3202),
            entry("core/climate_levels.csv", 26780),
            entry("core/base_rates.toml", 25),
        ];
        let now = [
            written(
                "core/climate.csv",
                3231,
                DiffSummary {
                    previous_rows: 3202,
                    rows: 3231,
                    added: 29,
                    ..Default::default()
                },
            ),
            // A rewritten TOML file: write_text cannot count the old rows.
            written(
                "core/base_rates.toml",
                27,
                DiffSummary {
                    rows: 27,
                    changed: 27,
                    ..Default::default()
                },
            ),
        ];
        let (changes, stale) = reconcile(&previous, &now);
        assert_eq!(stale, vec!["core/climate_levels.csv".to_string()]);
        let dropped = changes
            .iter()
            .find(|w| w.path == "core/climate_levels.csv")
            .unwrap();
        assert!(dropped.diff.removed_file);
        assert_eq!(
            (
                dropped.diff.previous_rows,
                dropped.diff.rows,
                dropped.diff.removed
            ),
            (26780, 0, 26780)
        );
        let toml = changes
            .iter()
            .find(|w| w.path == "core/base_rates.toml")
            .unwrap();
        assert_eq!(toml.diff.previous_rows, 25);
        let csv = changes
            .iter()
            .find(|w| w.path == "core/climate.csv")
            .unwrap();
        assert_eq!(csv.diff.previous_rows, 3202);
    }
}
