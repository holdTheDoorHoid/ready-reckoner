//! Turning a ZIP code or county code into a [`LocationResolved`].

use crate::DataStore;
use rr_types::{EngineError, ErrorCode, Exposure, FacilityFlags, LocationInput, LocationResolved};

/// A ZIP code resolves to one county only when that county holds at least this share of the ZIP's
/// land; otherwise the engine answers `ambiguous_zip` and the app asks the user to choose.
pub const AMBIGUOUS_ZIP_SHARE: f32 = 0.8;

/// 10 miles, the plume emergency planning zone, in km.
const EPZ_KM: f32 = 16.093_44;
/// 50 miles, the ingestion planning zone, in km.
const IPZ_KM: f32 = 80.4672;

impl DataStore {
    /// Build the resolved location for a county, optionally reached through a ZIP code.
    pub fn location(&self, county_fips: &str, zip: Option<&str>) -> Option<LocationResolved> {
        let c = self.county(county_fips)?;
        let zip = zip.map(|z| z.trim().to_string()).filter(|z| !z.is_empty());
        let share = zip.as_deref().and_then(|z| {
            self.resolve_zip(z)
                .into_iter()
                .find(|(f, _)| f == county_fips)
                .map(|(_, s)| s)
        });
        let zipf = zip.as_deref().and_then(|z| self.zip_facilities(z));
        let county_flags = self.county_facility_flags(county_fips);
        let facility_flags = match (zipf, county_flags) {
            (Some(z), _) => FacilityFlags {
                nuclear_plant_within_16km: z.nearest_nuclear_km.is_some_and(|d| d <= EPZ_KM),
                nuclear_plant_within_80km: z.nearest_nuclear_km.is_some_and(|d| d <= IPZ_KM),
                hazmat_facilities_within_5km: z.tri_within_5km,
            },
            (None, Some(f)) => FacilityFlags {
                nuclear_plant_within_16km: f.nuclear_within_16km,
                nuclear_plant_within_80km: f.nuclear_within_80km,
                hazmat_facilities_within_5km: c
                    .facilities
                    .as_ref()
                    .map(|x| x.tri_facilities)
                    .unwrap_or(0),
            },
            (None, None) => FacilityFlags {
                nuclear_plant_within_16km: false,
                nuclear_plant_within_80km: false,
                hazmat_facilities_within_5km: 0,
            },
        };
        let data_note = if zipf.is_some() {
            "Hazards describe your county; facility distances are from your ZIP code's centre. Tract-level data is not loaded yet."
        } else {
            "Hazards and facility counts describe your whole county. Tract-level data is not loaded yet."
        };
        Some(LocationResolved {
            country: "US".to_string(),
            county_fips: c.fips.clone(),
            county_name: c.name.clone(),
            state_abbr: c.state_abbr.clone(),
            state_name: c.state_name.clone(),
            zip: if share.is_some() { zip } else { None },
            zip_county_share: share,
            centroid: c.centroid,
            nca_region: c.nca_region.clone(),
            coastal: c.coastal,
            tsunami_zone: c.tsunami_zone,
            facility_flags,
            data_note: Some(data_note.to_string()),
            // awaiting: data-hazard — fill from the exposure columns (DESIGN-DELTA §1.3, §2).
            exposure: Exposure::default(),
        })
    }

    /// Resolve a [`LocationInput`]. A county code wins over a ZIP code. A ZIP code whose largest
    /// county holds less than [`AMBIGUOUS_ZIP_SHARE`] of its land gives an `ambiguous_zip` error
    /// listing every county it spans, largest share first.
    pub fn resolve(&self, input: &LocationInput) -> Result<LocationResolved, EngineError> {
        if self.counties().next().is_none() {
            return Err(EngineError::new(
                ErrorCode::PackMissing,
                "The county data has not loaded yet. Wait a moment and try again.",
            ));
        }
        if let Some(fips) = input
            .county_fips
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return self
                .location(fips, input.zip.as_deref())
                .ok_or_else(|| EngineError::unknown_county(fips, self.search_locations(fips)));
        }
        let Some(zip) = input
            .zip
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Err(EngineError::new(
                ErrorCode::BadInput,
                "Enter a ZIP code or choose your county.",
            ));
        };
        let shares = self.resolve_zip(zip);
        match shares.first() {
            None => {
                // Suggest the counties of ZIP codes that share the first three digits.
                let prefix: String = zip.chars().take(3).collect();
                let mut seen = Vec::new();
                for (z, list) in self.zip_county_iter() {
                    if z.starts_with(&prefix) {
                        for (f, _) in list {
                            if !seen.contains(f) {
                                seen.push(f.clone());
                            }
                        }
                    }
                    if seen.len() >= 5 {
                        break;
                    }
                }
                Err(EngineError::unknown_zip(
                    zip,
                    seen.iter().filter_map(|f| self.location(f, None)).collect(),
                ))
            }
            Some((fips, share)) if *share >= AMBIGUOUS_ZIP_SHARE => self
                .location(fips, Some(zip))
                .ok_or_else(|| EngineError::unknown_zip(zip, Vec::new())),
            Some(_) => Err(EngineError::ambiguous_zip(
                zip,
                shares
                    .iter()
                    .filter_map(|(f, _)| self.location(f, Some(zip)))
                    .collect(),
            )),
        }
    }

    /// True when a ZIP code spans several counties and none holds [`AMBIGUOUS_ZIP_SHARE`].
    pub fn is_ambiguous_zip(&self, zip: &str) -> bool {
        self.resolve_zip(zip)
            .first()
            .is_some_and(|(_, s)| *s < AMBIGUOUS_ZIP_SHARE)
    }
}
