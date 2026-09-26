//! Where the WebAssembly engine's county data comes from: [`WasmSource`].
//!
//! Two sources sit behind one [`CountySource`]:
//!
//! - the **data packs** (`rr_data::DataStore`), fed file by file through `load_pack`. The web
//!   app loads `manifest.json` first and then every file of the `core` pack; each file is checked
//!   against the sha256 the manifest records.
//! - the **built-in sample counties** (`rr_plan::FixtureSource`): the seven hand-built counties
//!   the fixture households live in. They answer only while no pack file has been loaded, so the
//!   engine works (for those seven counties) before any data is fetched, in tests, and on a site
//!   built without `data/`. The goldens in `fixtures/golden/` are planned from the packs, not from
//!   this mode; this mode is checked against rr-plan's own sample-county engine instead.
//!
//! Once any pack file is loaded the packs decide everything, and the engine plans from them only
//! when the manifest and every file of the `core` pack are in: a half-loaded core pack would give
//! plausible but wrong numbers (county records without their hazard rows), so until then
//! `assess`, `resolve_location` and `county_search` answer `pack_missing`.
//!
//! One exception keeps the first download small: the three ZIP tables ([`ZIP_FILES`]) are read
//! only by ZIP-code lookups, so the web app loads them when a ZIP code is typed. With every other
//! core file in, a county plans (and `county_search` answers) as it will with the whole pack; any
//! location that carries a ZIP code answers `pack_missing` until the ZIP tables are in, because
//! a ZIP code decides the county and the facility distances.

use std::collections::BTreeSet;

use rr_data::DataStore;
use rr_plan::{CountySource, FixtureSource};
use rr_types::{
    Attribution, BaseRate, CountyRecord, EngineError, ErrorCode, LocationInput, LocationResolved,
    PackInfo, Problem, ProblemCode,
};

/// The pack the engine needs before it can plan from pack data.
pub const CORE_PACK: &str = "core";

/// The manifest's own path; it is loaded first.
pub const MANIFEST_PATH: &str = "manifest.json";

/// The core files only ZIP-code lookups read: which counties a ZIP code covers, the ZIP centres,
/// and facility distances from them. The web app loads them when a ZIP code is typed (the same
/// list is `ZIP_FILES` in `web/src/engine/data-files.ts`).
pub const ZIP_FILES: &[&str] = &[
    "core/zip_county.csv",
    "core/zip_centroids.csv",
    "core/zip_facilities.csv",
];

/// Which data is answering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// No pack file has been loaded: the built-in sample counties answer.
    Fixtures,
    /// Pack files are loading or loaded.
    Packs {
        /// The manifest and every core file except the ZIP tables are loaded: counties plan.
        counties: bool,
        /// The ZIP tables are loaded as well: the whole core pack is in.
        complete: bool,
    },
}

/// The engine's county data: the loaded packs, or the built-in sample counties before any pack
/// file arrives.
#[derive(Debug)]
pub struct WasmSource {
    store: DataStore,
    fixtures: FixtureSource,
}

impl WasmSource {
    /// A source with no packs loaded (the sample counties answer).
    ///
    /// # Errors
    ///
    /// `pack_corrupt` if the embedded sample counties do not parse (rr-plan's tests guarantee
    /// they do).
    pub fn new() -> Result<Self, EngineError> {
        Ok(Self {
            store: DataStore::new(),
            fixtures: FixtureSource::new()?,
        })
    }

    /// Which data is answering.
    pub fn mode(&self) -> Mode {
        if self.store.loaded().is_empty() {
            Mode::Fixtures
        } else {
            let counties = self.core_loaded(false);
            Mode::Packs {
                counties,
                complete: counties && self.core_loaded(true),
            }
        }
    }

    /// The loaded packs (empty before `load_pack`).
    pub fn store(&self) -> &DataStore {
        &self.store
    }

    /// Loads one pack file. `name` is the file's path as `data/manifest.json` lists it
    /// (`manifest.json`, `core/nri_hazards.csv`, `geo/counties.json`).
    ///
    /// # Errors
    ///
    /// `bad_input` for a name the engine does not know (with one `schema` problem on `name`);
    /// `pack_corrupt` for a file that does not match its manifest checksum or cannot be read.
    pub fn load_pack(&mut self, name: &str, bytes: &[u8]) -> Result<PackInfo, EngineError> {
        if name.trim().is_empty() {
            return Err(name_problem(
                "Give the pack file's path from data/manifest.json, like core/counties.csv.",
            ));
        }
        self.store.load_pack(name, bytes).map_err(|e| {
            // rr-data reports an unknown file name as `bad_input` without details; the contract
            // gives every `bad_input` a problem list.
            if e.code == ErrorCode::BadInput && e.problems().is_none() {
                name_problem(&e.message)
            } else {
                e
            }
        })
    }

    /// The manifest is loaded and so is every core file: the ZIP tables among them when
    /// `with_zip`, the others otherwise.
    fn core_loaded(&self, with_zip: bool) -> bool {
        let Some(manifest) = self.store.manifest() else {
            return false;
        };
        let loaded = self.store.loaded();
        manifest.packs.get(CORE_PACK).is_some_and(|core| {
            !core.files.is_empty()
                && core
                    .files
                    .iter()
                    .filter(|f| ZIP_FILES.contains(&f.path.as_str()) == with_zip)
                    .all(|f| loaded.contains_key(&f.path))
        })
    }

    /// The loaded pack data version, `None` while the sample counties answer (the contract's
    /// "absent until a pack is loaded").
    pub fn loaded_pack_version(&self) -> Option<String> {
        match self.mode() {
            Mode::Fixtures => None,
            Mode::Packs { .. } => self.store.pack_version().map(str::to_owned),
        }
    }

    /// Why the packs cannot answer a county lookup yet, when they cannot.
    pub fn missing(&self) -> Option<EngineError> {
        match self.mode() {
            Mode::Packs {
                counties: false, ..
            } => Some(self.pack_missing()),
            _ => None,
        }
    }

    /// The county data (every core file but the ZIP tables) is not all in.
    fn pack_missing(&self) -> EngineError {
        let loaded = self.store.loaded();
        let message = match self.store.manifest().and_then(|m| m.packs.get(CORE_PACK)) {
            None => "The county data has not loaded yet. Wait a moment and try again, or reload \
                     the page."
                .to_owned(),
            Some(core) => {
                let needed = || {
                    core.files
                        .iter()
                        .filter(|f| !ZIP_FILES.contains(&f.path.as_str()))
                };
                let have = needed().filter(|f| loaded.contains_key(&f.path)).count();
                format!(
                    "The county data has not finished loading ({have} of {} files). Wait a \
                     moment and try again, or reload the page.",
                    needed().count()
                )
            }
        };
        EngineError::new(ErrorCode::PackMissing, message)
    }

    /// The ZIP tables are not all in.
    fn zips_missing() -> EngineError {
        EngineError::new(
            ErrorCode::PackMissing,
            "The list of ZIP codes has not loaded yet. Wait a moment and try again, or search \
             for your county by name.",
        )
    }
}

/// The ZIP code a location carries, if any (blank counts as none).
fn zip_of(input: &LocationInput) -> Option<&str> {
    input
        .zip
        .as_deref()
        .map(str::trim)
        .filter(|z| !z.is_empty())
}

/// A `bad_input` error about the pack file name.
fn name_problem(message: &str) -> EngineError {
    EngineError::bad_input(vec![Problem::new(ProblemCode::Schema, "name", message)])
}

impl CountySource for WasmSource {
    fn county(&self, fips: &str) -> Option<&CountyRecord> {
        match self.mode() {
            Mode::Fixtures => self.fixtures.county(fips),
            Mode::Packs { counties: true, .. } => self.store.county(fips.trim()),
            Mode::Packs {
                counties: false, ..
            } => None,
        }
    }

    fn resolve_zip(&self, zip: &str) -> Vec<(String, f32)> {
        match self.mode() {
            Mode::Fixtures => self.fixtures.resolve_zip(zip),
            Mode::Packs { complete: true, .. } => self.store.resolve_zip(zip),
            Mode::Packs {
                complete: false, ..
            } => Vec::new(),
        }
    }

    fn search(&self, query: &str) -> Vec<LocationResolved> {
        match self.mode() {
            Mode::Fixtures => self.fixtures.search(query),
            Mode::Packs { counties: true, .. } => self.store.search_locations(query),
            Mode::Packs {
                counties: false, ..
            } => Vec::new(),
        }
    }

    fn base_rates(&self) -> &[BaseRate] {
        match self.mode() {
            Mode::Fixtures => self.fixtures.base_rates(),
            Mode::Packs { .. } => self.store.base_rates(),
        }
    }

    /// The National Risk Index statement first (its terms require it to be shown; the trait
    /// promises it first): both sources already order it so.
    fn attributions(&self) -> Vec<Attribution> {
        match self.mode() {
            Mode::Fixtures => self.fixtures.attributions(),
            Mode::Packs { .. } => self.store.attributions(),
        }
    }

    /// With a ZIP code, only once the ZIP tables are in: the ZIP code's facility distances
    /// replace the county's.
    fn location(&self, county_fips: &str, zip: Option<&str>) -> Option<LocationResolved> {
        let has_zip = zip.is_some_and(|z| !z.trim().is_empty());
        match self.mode() {
            Mode::Fixtures => self.fixtures.location(county_fips, zip),
            Mode::Packs { complete: true, .. } => self.store.location(county_fips.trim(), zip),
            Mode::Packs { counties: true, .. } if !has_zip => {
                self.store.location(county_fips.trim(), None)
            }
            Mode::Packs { .. } => None,
        }
    }

    /// The version stamped on every plan: the fixtures' version while they answer (so a plan says
    /// it came from sample data), otherwise the manifest's `pack_version`.
    fn pack_version(&self) -> Option<String> {
        match self.mode() {
            Mode::Fixtures => self.fixtures.pack_version(),
            Mode::Packs { .. } => self.store.pack_version().map(str::to_owned),
        }
    }

    /// Each pack whose files are all loaded, by name (`core`, `geo`); while a pack is still
    /// loading, its loaded files by path (and `manifest.json` until a pack is complete). Empty
    /// while the sample counties answer.
    fn packs_loaded(&self) -> Vec<String> {
        let loaded = self.store.loaded();
        let Some(manifest) = self.store.manifest() else {
            return loaded.keys().cloned().collect();
        };
        let mut out = Vec::new();
        let mut covered: BTreeSet<&str> = BTreeSet::new();
        for (name, pack) in &manifest.packs {
            if !pack.files.is_empty() && pack.files.iter().all(|f| loaded.contains_key(&f.path)) {
                out.push(name.clone());
                covered.extend(pack.files.iter().map(|f| f.path.as_str()));
            }
        }
        let any_complete = !out.is_empty();
        for path in loaded.keys() {
            let is_manifest = path == MANIFEST_PATH;
            if covered.contains(path.as_str()) || (is_manifest && any_complete) {
                continue;
            }
            out.push(path.clone());
        }
        out
    }

    fn has_counties(&self) -> bool {
        match self.mode() {
            Mode::Fixtures => self.fixtures.has_counties(),
            Mode::Packs { counties, .. } => counties,
        }
    }

    fn resolve(&self, input: &LocationInput) -> Result<LocationResolved, EngineError> {
        match self.mode() {
            Mode::Fixtures => self.fixtures.resolve(input),
            Mode::Packs {
                counties: false, ..
            } => Err(self.pack_missing()),
            // A ZIP code decides the county (or, with a county chosen, the facility distances), so
            // it waits for the ZIP tables rather than silently planning without them.
            Mode::Packs {
                complete: false, ..
            } if zip_of(input).is_some() => Err(Self::zips_missing()),
            // rr-data's own rules: the same ones as rr-plan's, plus nearby counties as suggestions
            // for an unknown ZIP code.
            Mode::Packs { .. } => self.store.resolve(input),
        }
    }
}
