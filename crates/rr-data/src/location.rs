//! Turning a ZIP code or county code into a [`LocationResolved`].

use crate::DataStore;
use crate::exposure::{clean_f64, exposure_source};
use rr_types::{
    CitationId, CountyRecord, EngineError, ErrorCode, Exposure, FacilityFlags, LocationInput,
    LocationResolved, Sourced,
};

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
        let exposure = self.location_exposure(c, zip.as_deref());
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
            exposure,
        })
    }

    /// The exposure values for a county, reached through a ZIP code when one is given (DESIGN-DELTA
    /// §1.3): the ZIP's own values where the pack has them (nearest strategic site, dams naming
    /// the ZIP's town, the optional surge pack), the county's otherwise. Each carries the
    /// citation id from [`crate::EXPOSURE_SOURCES`].
    pub fn location_exposure(&self, c: &CountyRecord, zip: Option<&str>) -> Exposure {
        let e = &c.exposure;
        let z = zip.and_then(|z| self.zip_record(z));
        let src = |field: &str| CitationId::from(exposure_source(field));
        let num = |v: Option<f32>, field: &str| {
            v.map(|x| Sourced {
                value: clean_f64(x),
                source: src(field),
            })
        };
        let text = |v: Option<&str>, field: &str| {
            v.map(|x| Sourced {
                value: x.to_string(),
                source: src(field),
            })
        };
        // Strategic distance: the ZIP's nearest point site within 150 km, else the county's
        // distance to the place behind its class (the "Why here" sentence's distance).
        let strategic_km = z.as_ref().and_then(|z| z.strategic_km).or(e.strategic_km);
        Exposure {
            strategic_class: text(e.strategic_class.map(|k| k.as_str()), "strategic_class"),
            strategic_km: num(strategic_km, "strategic_km"),
            surge_cat3_share: num(
                z.as_ref().and_then(|z| z.surge_cat3_share),
                "surge_cat3_share",
            ),
            surge_proxy_class: text(e.surge_proxy_class.map(|k| k.as_str()), "surge_proxy_class"),
            smoke_days_35: num(e.smoke_days_35, "smoke_days_35"),
            leveed_pop_share: num(e.leveed_pop_share, "leveed_pop_share"),
            dams_high_within_10km: z
                .as_ref()
                .and_then(|z| z.dams_high_within_10km_naming_town)
                .map(|n| Sourced {
                    value: n,
                    source: src("dams_high_within_10km"),
                }),
            karst_share: num(e.karst_share, "karst_share"),
            landslide_susceptible_share: num(
                e.landslide_susceptible_share,
                "landslide_susceptible_share",
            ),
            water_system_flag: num(e.sdwis_violation_pop_share, "water_system_flag"),
            geomag_factor: num(e.geomag_factor, "geomag_factor"),
            // The app-facing field is the urban area's share (what rr-hazards weights the attack,
            // CBRN and crude-device terms by), not the county split, so the number a user sees
            // is the number the engine used.
            uasi_share: num(e.uasi_area_share, "uasi_share"),
            eviction_rate: num(e.eviction_filing_rate, "eviction_rate"),
        }
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
