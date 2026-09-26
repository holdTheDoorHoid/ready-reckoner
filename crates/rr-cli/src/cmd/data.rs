//! `rr data verify` and `rr data info`.
//!
//! `verify` runs the checks the data layer owns: the store's (every file the manifest lists
//! loads by its manifest path, matches its sha256 and parses, and its row count matches the
//! manifest) and the ETL's (`rr_etl::verify`: checksums, row counts, every county joins across
//! every pack or is listed as missing with a reason, the Connecticut crosswalk, ZIP shares), then
//! checks that every fixture household's location resolves. `info` prints the pack version,
//! when each source was retrieved, and the attributions the app and the packet must show.

use std::path::{Path, PathBuf};

use rr_plan::{CountySource, Engine};

use crate::Output;
use crate::args::{DataArgs, DataCommand};
use crate::error::{CliError, Exit};
use crate::format::{Table, fields, thousands, wrap};
use crate::source::{self, DEFAULT_DATA_DIR, Source};

/// What `data verify` and `data info` say when there is no pack.
pub const NO_PACK: &str = "no data pack; fixture counties in use";

/// Runs `rr data ...`.
///
/// # Errors
///
/// A pack that cannot be loaded at all (exit 1); `--data` without a manifest (exit 2).
pub fn run(data: &DataArgs, command: DataCommand) -> Result<Output, CliError> {
    match command {
        DataCommand::Verify => verify(data),
        DataCommand::Info => info(data),
    }
}

/// The pack directory to use, or `None` for the fixtures.
fn pack_dir(data: &DataArgs) -> Result<Option<PathBuf>, CliError> {
    if data.fixtures {
        return Ok(None);
    }
    match &data.data {
        Some(d) if d.join("manifest.json").is_file() => Ok(Some(d.clone())),
        Some(d) => Err(CliError::input(format!(
            "There is no data pack in {}: manifest.json is missing.",
            d.display()
        ))),
        None => {
            let d = Path::new(DEFAULT_DATA_DIR);
            Ok(d.join("manifest.json").is_file().then(|| d.to_path_buf()))
        }
    }
}

fn verify(data: &DataArgs) -> Result<Output, CliError> {
    let mut s = String::new();
    let mut problems: Vec<String> = Vec::new();
    let Some(dir) = pack_dir(data)? else {
        s.push_str(&format!(
            "{NO_PACK}.\n\nChecking the fixture counties instead:\n"
        ));
        let engine = Engine::new(Source::fixtures()?).map_err(|e| CliError::engine(&e, None))?;
        fixture_households(&engine, &mut s, &mut problems);
        return Ok(finish(s, problems));
    };

    s.push_str(&format!("Verifying the data pack in {}\n\n", dir.display()));
    // 1. The store: what the engine (and the web app) actually loads.
    let (manifest, _) = source::read_manifest(&dir)?;
    let listed = source::manifest_paths(&manifest);
    match source::load_dir(&dir) {
        Ok(src) => {
            let store = src.store().expect("load_dir returns a pack");
            let mut rows_ok = 0usize;
            for f in manifest.packs.values().flat_map(|p| p.files.iter()) {
                match store.loaded().get(&f.path) {
                    Some(&rows) if u64::from(rows) == f.rows => rows_ok += 1,
                    Some(&rows) => problems.push(format!(
                        "{}: the store read {rows} rows, the manifest says {}",
                        f.path, f.rows
                    )),
                    None => problems.push(format!("{}: listed but not loaded", f.path)),
                }
            }
            s.push_str(&format!(
                "  ok  store: {} files load by their manifest path and match their sha256 \
                 (pack {})\n",
                listed.len(),
                store.pack_version().unwrap_or("unversioned")
            ));
            s.push_str(&format!(
                "  {}  store: row counts match the manifest for {rows_ok} of {} files\n",
                if rows_ok == listed.len() { "ok" } else { "!!" },
                listed.len()
            ));
            s.push_str(&format!(
                "  ok  store: {} county records, {} ZIP codes, {} counties on the map\n",
                thousands(store.counties().count() as u64),
                thousands(rows_of(store, "core/zip_centroids.csv")),
                thousands(store.map_ids().len() as u64)
            ));
            let engine = Engine::new(src).map_err(|e| CliError::engine(&e, None))?;
            fixture_households(&engine, &mut s, &mut problems);
        }
        Err(e) => {
            problems.push(format!("store: {}", e.message));
            s.push_str(&format!("  !!  store: {}\n", e.message));
        }
    }

    // 2. The ETL's own checks on the files.
    match rr_etl::verify::verify(&dir) {
        Ok(rep) => {
            s.push_str(&format!(
                "  {}  rr-etl verify: {} files, {} checks, {} problem{}\n",
                if rep.problems.is_empty() { "ok" } else { "!!" },
                rep.files,
                rep.checks,
                rep.problems.len(),
                if rep.problems.len() == 1 { "" } else { "s" }
            ));
            for l in &rep.lines {
                s.push_str(&format!("        {}\n", wrap(l, 92, 8)));
            }
            problems.extend(rep.problems.iter().map(|p| format!("rr-etl verify: {p}")));
        }
        Err(e) => {
            problems.push(format!("rr-etl verify: {e}"));
            s.push_str(&format!("  !!  rr-etl verify: {e}\n"));
        }
    }
    Ok(finish(s, problems))
}

/// Rows the store read from one pack file (0 when it is not loaded).
fn rows_of(store: &rr_data::DataStore, path: &str) -> u64 {
    store.loaded().get(path).copied().map_or(0, u64::from)
}

/// Every fixture household's location resolves on this source.
fn fixture_households(engine: &Engine<Source>, s: &mut String, problems: &mut Vec<String>) {
    let mut ok = 0usize;
    let all = rr_types::fixtures::all();
    for (name, input) in &all {
        match engine.store().resolve(&input.location) {
            Ok(l) if engine.store().county(&l.county_fips).is_some() => ok += 1,
            Ok(l) => problems.push(format!(
                "{name}: resolves to {} but that county has no record",
                l.county_fips
            )),
            Err(e) => problems.push(format!("{name}: {e}")),
        }
    }
    s.push_str(&format!(
        "  {}  engine: {ok} of {} fixture households resolve to a county with a record\n",
        if ok == all.len() { "ok" } else { "!!" },
        all.len()
    ));
    if engine.store().is_fixtures() {
        let n = engine.store().base_rates().len();
        s.push_str(&format!("  ok  engine: {n} base rates load\n"));
    }
}

fn finish(mut s: String, problems: Vec<String>) -> Output {
    if problems.is_empty() {
        s.push_str("\nEverything checks out.\n");
        return Output::text(s);
    }
    s.push_str(&format!("\n{} problem(s):\n", problems.len()));
    for p in &problems {
        s.push_str(&format!("  - {}\n", wrap(p, 92, 4)));
    }
    let mut out = Output::text(s);
    out.exit = Exit::Failure;
    out
}

fn info(data: &DataArgs) -> Result<Output, CliError> {
    let dir = pack_dir(data)?;
    let src = match &dir {
        Some(d) => source::load_dir(d)?,
        None => Source::fixtures()?,
    };
    let engine = Engine::new(src).map_err(|e| CliError::engine(&e, None))?;
    let info = engine.engine_info();
    let store = engine.store();
    let mut rows: Vec<(&str, String)> = vec![
        (
            "Engine",
            format!("{} (API {})", info.engine_version, info.api_version),
        ),
        ("Content", info.content_version.clone()),
    ];
    let pack_version = info.data_pack_version.clone().unwrap_or_default();
    match store {
        Source::Pack { store: ds, dir } => {
            rows.push(("Data pack", format!("{pack_version} in {}", dir.display())));
            if let Some(m) = ds.manifest() {
                rows.push(("Refreshed", m.generated.clone()));
            }
            rows.push(("Packs loaded", info.packs_loaded.join(", ")));
            rows.push(("Counties", thousands(ds.counties().count() as u64)));
            rows.push(("National Risk Index", ds.nri_version().to_owned()));
        }
        Source::Fixtures(f) => {
            rows.push(("Data pack", format!("{NO_PACK} ({pack_version})")));
            let names: Vec<String> = f
                .fips_codes()
                .into_iter()
                .filter_map(|fips| {
                    CountySource::county(f, fips)
                        .map(|c| format!("{} {} ({fips})", c.name, c.state_abbr))
                })
                .collect();
            rows.push(("Counties", names.join(", ")));
        }
    }
    let mut s = fields(&rows, 0);

    if let Source::Pack { store: ds, dir } = store {
        if let Some(m) = ds.manifest() {
            s.push_str("\nPacks\n\n");
            let mut t = Table::new(["Pack", "Files", "Rows", "What it is"])
                .right(1)
                .right(2);
            for (name, p) in &m.packs {
                let rows: u64 = p.files.iter().map(|f| f.rows).sum();
                t.row([
                    name.clone(),
                    p.files.len().to_string(),
                    thousands(rows),
                    p.description.clone(),
                ]);
            }
            s.push_str(&t.render(1));
            s.push_str("\nSources, by ETL job\n\n");
            let mut t = Table::new([
                "Job",
                "Retrieved",
                "Sources",
                "Counties without data",
                "Licences",
            ])
            .right(2)
            .right(3);
            for (job, j) in &m.jobs {
                let mut dates: Vec<&str> = j
                    .sources
                    .iter()
                    .map(|src| src.retrieved.get(..10).unwrap_or(&src.retrieved))
                    .filter(|d| !d.is_empty())
                    .collect();
                dates.sort_unstable();
                dates.dedup();
                let retrieved = match (dates.first(), dates.last()) {
                    (Some(a), Some(b)) if a != b => format!("{a} to {b}"),
                    (Some(a), _) => (*a).to_owned(),
                    _ => j.finished.get(..10).unwrap_or(&j.finished).to_owned(),
                };
                let mut licences: Vec<&str> =
                    j.sources.iter().map(|src| src.license.as_str()).collect();
                licences.sort_unstable();
                licences.dedup();
                let without: usize = j.missing.iter().map(|x| x.fips.len()).sum();
                t.row([
                    job.clone(),
                    retrieved,
                    j.sources.len().to_string(),
                    without.to_string(),
                    licences.join("; "),
                ]);
            }
            s.push_str(&t.render(1));
            s.push_str(&format!(
                "\n Every source's URL, version and retrieval time is in {}/manifest.json \
                 (jobs.<job>.sources); the method notes are in docs/DATA_SOURCES.md.\n",
                dir.display()
            ));
        }
    }

    s.push_str("\nAttributions (the About screen and every packet show these)\n");
    for a in &info.attributions {
        let version = a
            .version
            .as_deref()
            .map(|v| format!(", version {v}"))
            .unwrap_or_default();
        s.push_str(&format!(
            "\n  {}{version}, accessed {}\n  {}\n    {}\n",
            a.source,
            a.accessed,
            a.url,
            wrap(&a.text, 92, 4)
        ));
    }
    Ok(Output::text(s))
}
