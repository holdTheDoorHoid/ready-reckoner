//! Where county data comes from: the data packs in `data/` (or `--data <dir>`), loaded into an
//! `rr_data::DataStore` exactly as the web app loads them (the manifest first, then every file it
//! lists by its manifest path, each checked against its sha256), or the seven hand-built fixture
//! counties (`--fixtures`, or when there is no pack).
//!
//! `rr_plan::CountySource` and `rr_data::DataStore` live in different crates, so the store is
//! wrapped in [`Source`], which implements the trait by passing every call through.

use std::path::{Path, PathBuf};

use rr_data::{DataStore, Manifest};
use rr_plan::{CountySource, Engine, FixtureSource};
use rr_types::{Attribution, BaseRate, CountyRecord, EngineError, LocationInput, LocationResolved};

use crate::args::DataArgs;
use crate::error::CliError;

/// The data directory used when `--data` is not given.
pub const DEFAULT_DATA_DIR: &str = "data";

/// County data for the engine.
#[derive(Debug)]
pub enum Source {
    /// The seven hand-built fixture counties embedded in `rr-plan`.
    Fixtures(FixtureSource),
    /// A national data pack, loaded from a directory.
    Pack {
        /// The loaded store.
        store: Box<DataStore>,
        /// The directory it came from.
        dir: PathBuf,
    },
}

/// An engine and the note to print about where its data came from.
#[derive(Debug)]
pub struct Opened {
    /// The engine on the chosen source.
    pub engine: Engine<Source>,
    /// Said on standard error (for example that there was no pack and the fixtures are in use).
    pub notes: Vec<String>,
}

impl Source {
    /// The fixture counties.
    ///
    /// # Errors
    ///
    /// If the embedded fixtures do not parse (rr-plan's tests guarantee they do).
    pub fn fixtures() -> Result<Self, CliError> {
        FixtureSource::new()
            .map(Source::Fixtures)
            .map_err(|e| CliError::engine(&e, None))
    }

    /// True for the fixture counties.
    pub fn is_fixtures(&self) -> bool {
        matches!(self, Source::Fixtures(_))
    }

    /// The data store, when a pack is loaded.
    pub fn store(&self) -> Option<&DataStore> {
        match self {
            Source::Pack { store, .. } => Some(store),
            Source::Fixtures(_) => None,
        }
    }

    /// One line saying what this is, for headers: "data pack e8b8cd6861e6 (data)" or "the seven
    /// fixture counties (fixtures+1a2b3c4d)".
    pub fn describe(&self) -> String {
        match self {
            Source::Fixtures(f) => format!(
                "the seven fixture counties ({})",
                f.pack_version().unwrap_or_default()
            ),
            Source::Pack { store, dir } => format!(
                "data pack {} ({})",
                store.pack_version().unwrap_or("unversioned"),
                dir.display()
            ),
        }
    }

    /// A hint for an unknown location: which counties the fixture data knows.
    pub fn location_hint(&self) -> Option<String> {
        match self {
            Source::Fixtures(f) => {
                let names: Vec<String> = f
                    .fips_codes()
                    .into_iter()
                    .filter_map(|fips| {
                        CountySource::county(f, fips)
                            .map(|c| format!("{} {} ({fips})", c.name, c.state_abbr))
                    })
                    .collect();
                Some(format!(
                    "Only the seven fixture counties are loaded: {}. Leave out --fixtures, or \
                     point --data at a data pack, to plan anywhere in the US.",
                    names.join(", ")
                ))
            }
            Source::Pack { .. } => None,
        }
    }
}

/// Chooses and loads the source: `--fixtures`; `--data <dir>` (which must hold a pack); or by
/// default `data/` when it holds a manifest, otherwise the fixtures with a note.
///
/// # Errors
///
/// A pack that is missing a file, fails its checksum or does not parse (exit 1); `--data`
/// naming a directory without a manifest (exit 2).
pub fn open(args: &DataArgs) -> Result<Opened, CliError> {
    let mut notes = Vec::new();
    let source = if args.fixtures {
        Source::fixtures()?
    } else if let Some(dir) = &args.data {
        if !dir.join("manifest.json").is_file() {
            return Err(CliError::input(format!(
                "There is no data pack in {}: manifest.json is missing. Build one with \
                 `cargo run -p rr-etl -- refresh --out {}`, or use --fixtures.",
                dir.display(),
                dir.display()
            )));
        }
        load_dir(dir)?
    } else {
        let dir = Path::new(DEFAULT_DATA_DIR);
        if dir.join("manifest.json").is_file() {
            load_dir(dir)?
        } else {
            notes.push(format!(
                "note: no data pack in ./{DEFAULT_DATA_DIR} (manifest.json not found); using the \
                 seven fixture counties. Run from the repository root or pass --data <dir>."
            ));
            Source::fixtures()?
        }
    };
    let engine = Engine::new(source).map_err(|e| CliError::engine(&e, None))?;
    Ok(Opened { engine, notes })
}

/// The manifest in a data directory.
///
/// # Errors
///
/// If it cannot be read or parsed.
pub fn read_manifest(dir: &Path) -> Result<(Manifest, Vec<u8>), CliError> {
    let path = dir.join("manifest.json");
    let bytes = std::fs::read(&path)
        .map_err(|e| CliError::failure(format!("{} cannot be read: {e}", path.display())))?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| {
        CliError::failure(format!("{} is not a data manifest: {e}", path.display()))
    })?;
    Ok((manifest, bytes))
}

/// Every file the manifest lists, by manifest path, in manifest order.
pub fn manifest_paths(manifest: &Manifest) -> Vec<String> {
    manifest
        .packs
        .values()
        .flat_map(|p| p.files.iter().map(|f| f.path.clone()))
        .collect()
}

/// Loads every file the manifest in `dir` lists into a `DataStore` (manifest first; each file is
/// checked against its sha256).
///
/// # Errors
///
/// A listed file that cannot be read (exit 1) or that the store rejects: a checksum mismatch or
/// a file that does not parse (`pack_corrupt`, exit 1).
pub fn load_dir(dir: &Path) -> Result<Source, CliError> {
    let (manifest, manifest_bytes) = read_manifest(dir)?;
    let mut files: Vec<(String, Vec<u8>)> = vec![("manifest.json".to_owned(), manifest_bytes)];
    for p in manifest_paths(&manifest) {
        let path = dir.join(&p);
        let bytes = std::fs::read(&path).map_err(|e| {
            CliError::failure(format!(
                "{} is listed in the manifest but cannot be read: {e}\n  `rr data verify` \
                 checks every file; --fixtures runs on the seven sample counties meanwhile.",
                path.display()
            ))
        })?;
        files.push((p, bytes));
    }
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let mut store = DataStore::new();
    store.load_many(&refs).map_err(|e| {
        CliError::engine(
            &e,
            Some(&format!(
                "(loading the data pack in {}; `rr data verify` checks every file, and \
                 --fixtures runs on the seven sample counties meanwhile)",
                dir.display()
            )),
        )
    })?;
    Ok(Source::Pack {
        store: Box::new(store),
        dir: dir.to_path_buf(),
    })
}

impl CountySource for Source {
    fn county(&self, fips: &str) -> Option<&CountyRecord> {
        match self {
            Source::Fixtures(f) => CountySource::county(f, fips),
            Source::Pack { store, .. } => store.county(fips.trim()),
        }
    }

    fn resolve_zip(&self, zip: &str) -> Vec<(String, f32)> {
        match self {
            Source::Fixtures(f) => CountySource::resolve_zip(f, zip),
            Source::Pack { store, .. } => store.resolve_zip(zip),
        }
    }

    fn search(&self, query: &str) -> Vec<LocationResolved> {
        match self {
            Source::Fixtures(f) => CountySource::search(f, query),
            Source::Pack { store, .. } => store.search_locations(query),
        }
    }

    fn base_rates(&self) -> &[BaseRate] {
        match self {
            Source::Fixtures(f) => CountySource::base_rates(f),
            Source::Pack { store, .. } => store.base_rates(),
        }
    }

    fn attributions(&self) -> Vec<Attribution> {
        match self {
            Source::Fixtures(f) => CountySource::attributions(f),
            // The store puts the National Risk Index statement first.
            Source::Pack { store, .. } => store.attributions(),
        }
    }

    fn location(&self, county_fips: &str, zip: Option<&str>) -> Option<LocationResolved> {
        match self {
            Source::Fixtures(f) => CountySource::location(f, county_fips, zip),
            Source::Pack { store, .. } => store.location(county_fips.trim(), zip),
        }
    }

    fn pack_version(&self) -> Option<String> {
        match self {
            Source::Fixtures(f) => CountySource::pack_version(f),
            Source::Pack { store, .. } => store.pack_version().map(str::to_owned),
        }
    }

    fn packs_loaded(&self) -> Vec<String> {
        match self {
            Source::Fixtures(f) => CountySource::packs_loaded(f),
            Source::Pack { store, .. } => {
                // A pack counts as loaded once every one of its files is.
                let Some(m) = store.manifest() else {
                    return Vec::new();
                };
                m.packs
                    .iter()
                    .filter(|(_, p)| {
                        !p.files.is_empty()
                            && p.files.iter().all(|f| store.loaded().contains_key(&f.path))
                    })
                    .map(|(name, _)| name.clone())
                    .collect()
            }
        }
    }

    fn has_counties(&self) -> bool {
        match self {
            Source::Fixtures(f) => CountySource::has_counties(f),
            Source::Pack { store, .. } => store.counties().next().is_some(),
        }
    }

    fn resolve(&self, input: &LocationInput) -> Result<LocationResolved, EngineError> {
        match self {
            Source::Fixtures(f) => CountySource::resolve(f, input),
            // rr-data owns the ZIP rules for the pack (shares, the 80 % rule, suggestions).
            Source::Pack { store, .. } => store.resolve(input),
        }
    }
}
