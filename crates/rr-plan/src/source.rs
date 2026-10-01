//! Where county data comes from: the [`CountySource`] trait, implemented by `rr-data`'s
//! [`DataStore`] (the national data pack) and by [`FixtureSource`] (fourteen built-in sample
//! counties, for tests and for an engine with no packs loaded; its locations say "sample data").
//!
//! Native callers load the repository's `data/` directory with [`load_data_dir`] (or
//! `Engine::with_data_dir`); the web app hands each pack file to `DataStore::load_pack` itself.

use std::collections::BTreeMap;

use rr_data::DataStore;
use rr_types::{
    Attribution, BaseRate, CountyRecord, Date, EngineError, ErrorCode, LocationInput,
    LocationResolved, RestorationCurve,
};

use crate::location;

/// Everything the engine needs from the data layer. All methods are pure lookups.
pub trait CountySource {
    /// The core-pack record for a county (five-digit FIPS), if it exists.
    fn county(&self, fips: &str) -> Option<&CountyRecord>;

    /// The counties a ZIP code covers, with the share of the ZIP code's addresses in each, largest
    /// first. Empty when the ZIP code is unknown.
    fn resolve_zip(&self, zip: &str) -> Vec<(String, f32)>;

    /// Counties matching free text ("phila", "42101", "Cook, IL"), best first, at most ten.
    fn search(&self, query: &str) -> Vec<LocationResolved>;

    /// National base rates for the societal and personal hazards.
    fn base_rates(&self) -> &[BaseRate];

    /// The pack's pooled power-restoration curves by region and cause (model review M-10: the
    /// regional restoration stretch and the Puerto Rico and Virgin Islands Maria curves). They
    /// are not per county, so the pipeline hands them to `rr-consequence` beside the county
    /// record (`CountyData::with_curves`). Empty for a source without them (the sample
    /// counties), which leaves the consequence model on each class's own durations.
    fn restoration_curves(&self) -> &[RestorationCurve];

    /// Credit lines and disclaimers the app and the packet must show (the National Risk Index
    /// statement first).
    fn attributions(&self) -> Vec<Attribution>;

    /// The resolved location for a county, reached through a ZIP code when one is given.
    fn location(&self, county_fips: &str, zip: Option<&str>) -> Option<LocationResolved>;

    /// Version of the loaded data, `None` before anything is loaded.
    fn pack_version(&self) -> Option<String>;

    /// Names of the packs loaded so far.
    fn packs_loaded(&self) -> Vec<String>;

    /// True once county records are available (`assess` answers `pack_missing` before that).
    fn has_counties(&self) -> bool;

    /// Resolves a location input: a county code wins over a ZIP code; a ZIP code whose largest
    /// county holds less than 80 % of it is `ambiguous_zip`. The default implementation follows
    /// `docs/ENGINE-API.md` using the lookups above; a source may override it.
    fn resolve(&self, input: &LocationInput) -> Result<LocationResolved, EngineError> {
        location::resolve_with(self, input)
    }

    /// The county's hospitals with emergency services, for the binder's Neighborhood page
    /// (DESIGN-DELTA-v3 §4.2, §8): `None` while the list is not loaded (the `places` pack, loaded
    /// when the binder is shown), so the page says nothing about it; `Some` with no rows for a
    /// county the list has no hospital for, which the page says in one sentence. The default is
    /// `None` (the sample counties carry no list).
    fn county_hospitals(&self, fips: &str) -> Option<CountyHospitals> {
        let _ = fips;
        None
    }
}

/// A county's hospitals with emergency services, as the binder prints them ([`CountySource`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CountyHospitals {
    /// The hospitals, as the dataset lists them (name, city, phone), in the dataset's order.
    pub rows: Vec<HospitalRow>,
    /// When the dataset was released, when the pack records it.
    pub released: Option<Date>,
}

/// One hospital with emergency services.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HospitalRow {
    /// Its name, as the dataset lists it.
    pub name: String,
    /// The city or town.
    pub city: String,
    /// Its telephone number, as the dataset lists it.
    pub phone: String,
}

/// The pack file of the county hospital list (`rr-data`'s `places` pack).
pub const HOSPITALS_FILE: &str = "places/hospitals.csv";

/// The county hospital list of a loaded data store: `None` until [`HOSPITALS_FILE`] is loaded.
/// The release date comes from the manifest's `hospitals` job ("released 2026-08-13" in a
/// source's version, else its `released` definition, else the day the job ran).
pub fn store_hospitals(store: &DataStore, fips: &str) -> Option<CountyHospitals> {
    if !store.loaded().contains_key(HOSPITALS_FILE) {
        return None;
    }
    let rows = store
        .county_hospitals(fips)
        .iter()
        .map(|h| HospitalRow {
            name: one_space(&h.name),
            city: one_space(&h.city),
            phone: one_space(&h.phone),
        })
        .collect();
    let released = store.manifest().and_then(|m| {
        let job = m.jobs.get("hospitals")?;
        job.sources
            .iter()
            .find_map(|s| date_after(&s.version, "released "))
            .or_else(|| {
                job.definitions
                    .get("released")
                    .and_then(|d| date_after(d, "released this dataset on "))
            })
            .or_else(|| Date::parse(job.finished.get(..10)?).ok())
    });
    Some(CountyHospitals { rows, released })
}

/// The words of a dataset field with runs of spaces as one ("CENTER HOSPITAL  CAROLINA").
fn one_space(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The ISO date that follows `marker` in `text`.
fn date_after(text: &str, marker: &str) -> Option<Date> {
    let at = text.find(marker)? + marker.len();
    Date::parse(text.get(at..at + 10)?).ok()
}

/// One fixture county: the record and the resolved location, as in
/// `crates/rr-hazards/tests/data/counties/<fips>.json`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCounty {
    county: CountyRecord,
    location: LocationResolved,
}

/// The fixture counties, embedded at compile time. The first seven are rr-hazards' hand-built
/// test inputs (NRI v1.20 and CMRA values, research outage rates; see that directory's README);
/// the seven added in v0.2.0 are the core data pack's own records for the new fixture
/// households' counties (`fixtures/sample-counties/README.md`). Neither is the data pack itself.
const FIXTURE_COUNTIES: &[(&str, &str)] = &[
    (
        "04013",
        include_str!("../../rr-hazards/tests/data/counties/04013.json"),
    ),
    (
        "12086",
        include_str!("../../rr-hazards/tests/data/counties/12086.json"),
    ),
    (
        "17031",
        include_str!("../../rr-hazards/tests/data/counties/17031.json"),
    ),
    (
        "20051",
        include_str!("../../rr-hazards/tests/data/counties/20051.json"),
    ),
    (
        "41011",
        include_str!("../../rr-hazards/tests/data/counties/41011.json"),
    ),
    (
        "42101",
        include_str!("../../rr-hazards/tests/data/counties/42101.json"),
    ),
    (
        "48157",
        include_str!("../../rr-hazards/tests/data/counties/48157.json"),
    ),
    (
        "06067",
        include_str!("../../../fixtures/sample-counties/06067.json"),
    ),
    (
        "22023",
        include_str!("../../../fixtures/sample-counties/22023.json"),
    ),
    (
        "26163",
        include_str!("../../../fixtures/sample-counties/26163.json"),
    ),
    (
        "30063",
        include_str!("../../../fixtures/sample-counties/30063.json"),
    ),
    (
        "38101",
        include_str!("../../../fixtures/sample-counties/38101.json"),
    ),
    (
        "48167",
        include_str!("../../../fixtures/sample-counties/48167.json"),
    ),
    (
        "72127",
        include_str!("../../../fixtures/sample-counties/72127.json"),
    ),
];

const FIXTURE_BASE_RATES: &str = include_str!("../../rr-hazards/tests/data/base_rates.json");

/// The note every fixture location carries, so no screen or packet mistakes test data for the
/// national data pack.
pub const FIXTURE_DATA_NOTE: &str = "The engine is running on fourteen built-in sample \
    counties, not the national data pack. Hazards describe your whole county.";

/// When the fixture data was assembled (the retrieval date of the research inputs it copies).
const FIXTURE_ACCESSED: Date = match Date::from_ymd(2026, 9, 25) {
    Some(d) => d,
    None => panic!("fixture date is invalid"),
};

/// The fourteen fixture counties as a [`CountySource`]: Philadelphia, Coos (Oregon), Ellis
/// (Kansas), Miami-Dade, Maricopa, Fort Bend (Texas) and Cook (Illinois), and from v0.2.0
/// Cameron Parish (Louisiana), Wayne (Michigan), Galveston (Texas), Ward (North Dakota),
/// Missoula (Montana), Sacramento (California) and San Juan (Puerto Rico). ZIP codes resolve
/// through each fixture's own ZIP code (each lies wholly in its county). Extra ZIP rows can be
/// added for tests with [`FixtureSource::with_zip`].
#[derive(Debug, Clone)]
pub struct FixtureSource {
    counties: BTreeMap<String, FixtureCounty>,
    zips: BTreeMap<String, Vec<(String, f32)>>,
    base_rates: Vec<BaseRate>,
    version: String,
}

impl FixtureSource {
    /// Parses the embedded fixture counties.
    ///
    /// # Errors
    ///
    /// `pack_corrupt` if an embedded file does not parse (the test suite guarantees they do).
    pub fn new() -> Result<Self, EngineError> {
        let mut counties = BTreeMap::new();
        let mut zips: BTreeMap<String, Vec<(String, f32)>> = BTreeMap::new();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for (fips, json) in FIXTURE_COUNTIES {
            fnv1a(&mut hash, json.as_bytes());
            let mut f: FixtureCounty = serde_json::from_str(json).map_err(|e| {
                EngineError::new(
                    ErrorCode::PackCorrupt,
                    format!("The sample county {fips} could not be read: {e}"),
                )
            })?;
            if let Some(zip) = f.location.zip.take() {
                zips.entry(zip)
                    .or_default()
                    .push((f.county.fips.clone(), 1.0));
            }
            f.location.zip_county_share = None;
            counties.insert(f.county.fips.clone(), f);
        }
        fnv1a(&mut hash, FIXTURE_BASE_RATES.as_bytes());
        let base_rates: Vec<BaseRate> = serde_json::from_str(FIXTURE_BASE_RATES).map_err(|e| {
            EngineError::new(
                ErrorCode::PackCorrupt,
                format!("The sample base rates could not be read: {e}"),
            )
        })?;
        Ok(Self {
            counties,
            zips,
            base_rates,
            version: format!("fixtures+{:08x}", hash >> 32),
        })
    }

    /// The same source with one more ZIP code row: `shares` are `(county FIPS, share)` pairs.
    /// Used by tests to exercise `ambiguous_zip`; the shares are sorted largest first.
    pub fn with_zip(mut self, zip: &str, shares: &[(&str, f32)]) -> Self {
        let mut v: Vec<(String, f32)> = shares.iter().map(|(f, s)| ((*f).to_owned(), *s)).collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        self.zips.insert(zip.to_owned(), v);
        self
    }

    /// The FIPS codes of every fixture county, in order.
    pub fn fips_codes(&self) -> Vec<&str> {
        self.counties.keys().map(String::as_str).collect()
    }
}

fn fnv1a(hash: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *hash ^= u64::from(*b);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

impl CountySource for FixtureSource {
    fn county(&self, fips: &str) -> Option<&CountyRecord> {
        self.counties.get(fips.trim()).map(|f| &f.county)
    }

    fn resolve_zip(&self, zip: &str) -> Vec<(String, f32)> {
        self.zips.get(zip.trim()).cloned().unwrap_or_default()
    }

    fn search(&self, query: &str) -> Vec<LocationResolved> {
        location::search_records(self.counties.values().map(|f| &f.county), query)
            .into_iter()
            .filter_map(|fips| self.location(&fips, None))
            .collect()
    }

    fn base_rates(&self) -> &[BaseRate] {
        &self.base_rates
    }

    /// The sample counties carry no pooled curves.
    fn restoration_curves(&self) -> &[RestorationCurve] {
        &[]
    }

    fn attributions(&self) -> Vec<Attribution> {
        fixture_attributions()
    }

    fn location(&self, county_fips: &str, zip: Option<&str>) -> Option<LocationResolved> {
        let f = self.counties.get(county_fips.trim())?;
        let mut loc = f.location.clone();
        let zip = zip.map(str::trim).filter(|z| !z.is_empty());
        let share = zip.and_then(|z| {
            self.resolve_zip(z)
                .into_iter()
                .find(|(c, _)| c == county_fips)
                .map(|(_, s)| s)
        });
        loc.zip = share.and(zip.map(str::to_owned));
        loc.zip_county_share = share;
        loc.data_note = Some(FIXTURE_DATA_NOTE.to_owned());
        Some(loc)
    }

    fn pack_version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn packs_loaded(&self) -> Vec<String> {
        vec!["fixtures".to_owned()]
    }

    fn has_counties(&self) -> bool {
        !self.counties.is_empty()
    }
}

/// The credit lines the fixture data needs: the FEMA National Risk Index statement (its terms
/// require the dataset version, the access date and the non-endorsement text, copied exactly from
/// the citation registry) and the CC BY credit for the EAGLE-I outage records.
fn fixture_attributions() -> Vec<Attribution> {
    let content = rr_content::try_content().ok();
    let quote = |id: &str| {
        content
            .and_then(|c| c.citation(id))
            .and_then(|c| c.quote.clone())
    };
    let mut out = Vec::new();
    if let Some(text) = quote("fema_nri_disclaimer") {
        out.push(Attribution {
            source: "FEMA National Risk Index".to_owned(),
            text,
            url: "https://www.fema.gov/about/openfema/data-sets/national-risk-index-data"
                .to_owned(),
            version: Some("1.20.0 (December 2025)".to_owned()),
            accessed: FIXTURE_ACCESSED,
        });
    }
    if let Some(c) = content.and_then(|c| c.citation("ornl_eagle_i_outages")) {
        out.push(Attribution {
            source: "EAGLE-I power outage records (Oak Ridge National Laboratory)".to_owned(),
            text: format!(
                "Contains data from \"{}\" by {}, licensed {}.",
                c.title, c.publisher, c.license
            ),
            url: c.url.clone(),
            version: None,
            accessed: FIXTURE_ACCESSED,
        });
    }
    out
}

impl CountySource for DataStore {
    fn county(&self, fips: &str) -> Option<&CountyRecord> {
        DataStore::county(self, fips.trim())
    }

    fn resolve_zip(&self, zip: &str) -> Vec<(String, f32)> {
        DataStore::resolve_zip(self, zip)
    }

    fn search(&self, query: &str) -> Vec<LocationResolved> {
        self.search_locations(query)
    }

    fn base_rates(&self) -> &[BaseRate] {
        DataStore::base_rates(self)
    }

    fn restoration_curves(&self) -> &[RestorationCurve] {
        DataStore::restoration_curves(self)
    }

    /// The store puts the National Risk Index statement first.
    fn attributions(&self) -> Vec<Attribution> {
        DataStore::attributions(self)
    }

    fn location(&self, county_fips: &str, zip: Option<&str>) -> Option<LocationResolved> {
        DataStore::location(self, county_fips.trim(), zip)
    }

    fn pack_version(&self) -> Option<String> {
        DataStore::pack_version(self).map(str::to_owned)
    }

    /// The manifest's packs with at least one file loaded (`core`, `geo`).
    fn packs_loaded(&self) -> Vec<String> {
        let loaded = self.loaded();
        match self.manifest() {
            Some(m) => m
                .packs
                .iter()
                .filter(|(_, p)| p.files.iter().any(|f| loaded.contains_key(&f.path)))
                .map(|(name, _)| name.clone())
                .collect(),
            None => Vec::new(),
        }
    }

    fn has_counties(&self) -> bool {
        self.counties().next().is_some()
    }

    fn resolve(&self, input: &LocationInput) -> Result<LocationResolved, EngineError> {
        DataStore::resolve(self, input)
    }

    fn county_hospitals(&self, fips: &str) -> Option<CountyHospitals> {
        store_hospitals(self, fips)
    }
}

/// The core pack and the `places` pack: the county hospital list the binder's Neighborhood page
/// prints, which the web app loads when the binder is shown ([`load_data_dir_packs`]).
pub const BINDER_PACKS: [&str; 2] = ["core", "places"];

/// The packs [`load_data_dir`] loads: every lookup the engine makes, plus the county hospital
/// list the binder's Neighborhood page prints, so `rr plan`'s default and the goldens show the
/// table (DESIGN-DELTA-v3 §8; wasm3, 2026-10-01: before this, `WasmSource` had no
/// `county_hospitals` to show it with, so neither did; the `places` pack is cheap and read-only,
/// so loading it by default costs nothing a lookup needs). The `geo` pack still only draws the
/// map and stays its own, separate opt-in. Equal to [`BINDER_PACKS`] (kept as a separate name: it
/// says what a *native* caller without `--data`/`--fixtures` gets, which is one thing `places`
/// happens to be for right now and might not always be).
pub const DATA_DIR_PACKS: [&str; 2] = BINDER_PACKS;

/// Loads a data directory the way the web app loads the packs to plan: `manifest.json` first,
/// then every file of the packs in [`DATA_DIR_PACKS`], each checked against its sha256. Native
/// only.
///
/// # Errors
///
/// `pack_missing` when a file cannot be read, `pack_corrupt` when one fails its checksum or
/// cannot be decoded.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_data_dir(dir: &std::path::Path) -> Result<DataStore, EngineError> {
    load_data_dir_packs(dir, &DATA_DIR_PACKS)
}

/// As [`load_data_dir`], with these packs (for example [`BINDER_PACKS`]). Native only.
///
/// # Errors
///
/// As [`load_data_dir`].
#[cfg(not(target_arch = "wasm32"))]
pub fn load_data_dir_packs(
    dir: &std::path::Path,
    packs: &[&str],
) -> Result<DataStore, EngineError> {
    let read = |rel: &str| -> Result<Vec<u8>, EngineError> {
        let path = dir.join(rel);
        std::fs::read(&path).map_err(|e| {
            EngineError::new(
                ErrorCode::PackMissing,
                format!("The data file {} could not be read: {e}", path.display()),
            )
        })
    };
    let manifest_bytes = read("manifest.json")?;
    let manifest: rr_data::Manifest = serde_json::from_slice(&manifest_bytes).map_err(|e| {
        EngineError::new(
            ErrorCode::PackCorrupt,
            format!("data/manifest.json could not be read: {e}"),
        )
    })?;
    let mut files: Vec<(String, Vec<u8>)> = vec![("manifest.json".to_owned(), manifest_bytes)];
    for pack in packs {
        if let Some(p) = manifest.packs.get(*pack) {
            for f in &p.files {
                files.push((f.path.clone(), read(&f.path)?));
            }
        }
    }
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let mut store = DataStore::new();
    store.load_many(&refs)?;
    Ok(store)
}
