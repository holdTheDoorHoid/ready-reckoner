//! Where county data comes from: the [`CountySource`] trait and [`FixtureSource`], the seven
//! hand-built fixture counties the engine runs on until the national data pack lands.
//!
//! `rr-data`'s `DataStore` will implement [`CountySource`] with the same meaning for every method
//! (its `county`, `resolve_zip`, `search_locations`, `base_rates`, `attributions`, `location` and
//! `pack_version` already match). // awaiting: rr-data

use std::collections::BTreeMap;

use rr_types::{
    Attribution, BaseRate, CountyRecord, Date, EngineError, ErrorCode, LocationInput,
    LocationResolved,
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
}

/// One fixture county: the record and the resolved location, as in
/// `crates/rr-hazards/tests/data/counties/<fips>.json`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCounty {
    county: CountyRecord,
    location: LocationResolved,
}

/// The fixture counties, embedded at compile time. They are rr-hazards' hand-built test inputs
/// (NRI v1.20 and CMRA values, research outage rates; see that directory's README), not the data
/// pack.
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
];

const FIXTURE_BASE_RATES: &str = include_str!("../../rr-hazards/tests/data/base_rates.json");

/// The note every fixture location carries, so no screen or packet mistakes test data for the
/// national data pack.
pub const FIXTURE_DATA_NOTE: &str = "The engine is running on seven hand-built sample \
    counties, not the national data pack. Hazards describe your whole county.";

/// When the fixture data was assembled (the retrieval date of the research inputs it copies).
const FIXTURE_ACCESSED: Date = match Date::from_ymd(2026, 9, 25) {
    Some(d) => d,
    None => panic!("fixture date is invalid"),
};

/// The seven fixture counties as a [`CountySource`]: Philadelphia, Coos (Oregon), Ellis
/// (Kansas), Miami-Dade, Maricopa, Fort Bend (Texas) and Cook (Illinois). ZIP codes resolve
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
