//! The data pack's v2 exposure columns (DESIGN-DELTA §2) as rr-hazards reads them.
//!
//! County columns come from `CountyRecord::exposure` (`CountyExposure`, written by the data
//! pack's `strategic`, `geomag`, `smoke`, `ground`, `levees`, `water_systems`, `facilities` and
//! `eviction` jobs and assembled by rr-data). ZIP columns, which only a ZIP code can give (the
//! high-hazard dams whose listed downstream town lies in the ZIP), come from
//! `LocationResolved::exposure`, the copy the app shows; a county field the record lacks falls
//! back to that copy too. Absent means unknown, never zero: every hazard that reads a column
//! says what it does without it.
//!
//! awaiting: data-hazard — `CountyRecord::exposure` is data-hazard's type, copied byte for byte
//! into this branch until that branch merges; the field names here are its column names.

use rr_types::{
    CountyExposure, CountyRecord, LocationResolved, Sourced, StrategicClass, StrategicPlace,
};

use crate::params::UASI_FY2026;

/// A value, if present and finite.
fn finite(v: Option<f32>) -> Option<f64> {
    v.map(f64::from).filter(|x| x.is_finite())
}

/// A share, if present, finite and between 0 and 1.
fn share(v: Option<f32>) -> Option<f64> {
    finite(v).filter(|x| (0.0..=1.0).contains(x))
}

/// A share from the app's copy, if present and between 0 and 1.
fn shown_share(v: &Option<Sourced<f64>>) -> Option<f64> {
    v.as_ref()
        .map(|s| s.value)
        .filter(|x| x.is_finite() && (0.0..=1.0).contains(x))
}

/// The metro weight for the attack and CBRN rows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Uasi {
    /// The county lies in a funded urban area: its name and the area's share of the money.
    Funded {
        /// FEMA's name for the urban area (as in [`UASI_FY2026`]), when known.
        area: Option<String>,
        /// The area's share of the national UASI total, 0 to 1.
        metro_share: f64,
    },
    /// The county lies outside every funded urban area.
    NotFunded,
    /// The pack does not say.
    Unknown,
}

/// Smoke-day counts for the county (2016–2023 means).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Smoke {
    /// Days a year with smoke and 24-hour PM2.5 of 35.5 µg/m³ or more.
    pub days_35: f64,
    /// Days a year with smoke and 55.5 µg/m³ or more, when known.
    pub days_55: Option<f64>,
    /// Imputed from satellite smoke maps rather than the county's own monitors.
    pub imputed: bool,
}

/// People behind levees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Levees {
    /// Share of the county's residents living behind a levee.
    pub pop_share: f64,
    /// Of those, the share behind levees USACE rates High or Very High risk (0 when unknown).
    pub high_share: f64,
}

/// High-hazard dams.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Dams {
    /// High-hazard dams within 10 km of the ZIP code whose listed downstream town lies in it.
    pub zip_downstream: Option<u16>,
    /// High-hazard dams in the county.
    pub county_total: Option<u16>,
    /// Of those, dams whose latest condition rating is Poor or Unsatisfactory.
    pub county_poor: Option<u16>,
}

/// Geomagnetic position of the county.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Geomag {
    /// NERC TPL-007 scaling factor α, 0.1 to 1.
    pub factor: f64,
    /// Geomagnetic latitude of the county, degrees, when known.
    pub lat: Option<f64>,
}

/// The exposure columns for one household's county and ZIP code.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Exposure<'a> {
    county: &'a CountyExposure,
    shown: &'a rr_types::Exposure,
    facility_dams: Option<u16>,
}

impl<'a> Exposure<'a> {
    /// The columns of `county`, with the ZIP-level ones from `location`.
    pub fn new(county: &'a CountyRecord, location: &'a LocationResolved) -> Self {
        Exposure {
            county: &county.exposure,
            shown: &location.exposure,
            facility_dams: county.facilities.as_ref().map(|f| f.high_hazard_dams),
        }
    }

    /// The county's strategic-exposure class for the nuclear family.
    pub fn strategic_class(&self) -> Option<StrategicClass> {
        self.county.strategic_class.or_else(|| {
            self.shown
                .strategic_class
                .as_ref()
                .and_then(|s| s.value.parse().ok())
        })
    }

    /// The sites or areas behind the class, membership first, then nearest first.
    pub fn strategic_places(&self) -> &'a [StrategicPlace] {
        &self.county.strategic_places
    }

    /// Distance (km) and compass bearing (degrees) from the county to the first distance-based
    /// reason for its class (classes B and D).
    pub fn strategic_distance(&self) -> Option<(f64, Option<f64>)> {
        let km = finite(self.county.strategic_km).filter(|k| *k >= 0.0)?;
        let bearing = finite(self.county.strategic_bearing);
        Some((km, bearing))
    }

    /// The metro weight: the county's urban area and that area's share of the UASI money.
    pub fn uasi(&self) -> Uasi {
        let by_name = |name: &str| {
            UASI_FY2026
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, s)| *s)
        };
        let county_share = share(self.county.uasi_share);
        if let Some(area) = self.county.uasi_area.as_deref() {
            if let Some(s) = by_name(area) {
                return Uasi::Funded {
                    area: Some(area.to_owned()),
                    metro_share: s,
                };
            }
        }
        match (county_share, shown_share(&self.shown.uasi_share)) {
            (Some(c), _) if c <= 0.0 => Uasi::NotFunded,
            // The app's copy holds the area's own share (contract v2).
            (_, Some(s)) if s > 0.0 => Uasi::Funded {
                area: self.county.uasi_area.clone(),
                metro_share: s,
            },
            (_, Some(_)) => Uasi::NotFunded,
            // A county share without its area: a lower bound on the area's share.
            (Some(c), None) => Uasi::Funded {
                area: self.county.uasi_area.clone(),
                metro_share: c,
            },
            (None, None) => Uasi::Unknown,
        }
    }

    /// The county's geomagnetic scaling factor.
    pub fn geomag(&self) -> Option<Geomag> {
        let factor = finite(self.county.geomag_factor)
            .or_else(|| self.shown.geomag_factor.as_ref().map(|s| s.value))
            .filter(|f| f.is_finite() && *f > 0.0)?;
        Some(Geomag {
            factor,
            lat: finite(self.county.geomag_lat),
        })
    }

    /// The county's smoke days.
    pub fn smoke(&self) -> Option<Smoke> {
        let days_35 = finite(self.county.smoke_days_35)
            .or_else(|| self.shown.smoke_days_35.as_ref().map(|s| s.value))
            .filter(|d| d.is_finite() && *d >= 0.0)?;
        Some(Smoke {
            days_35,
            days_55: finite(self.county.smoke_days_55).filter(|d| *d >= 0.0),
            imputed: self
                .county
                .smoke_basis
                .as_deref()
                .is_some_and(|b| b != "monitor"),
        })
    }

    /// Share of the county on karst ground.
    pub fn karst_share(&self) -> Option<f64> {
        share(self.county.karst_share).or_else(|| shown_share(&self.shown.karst_share))
    }

    /// Share of the county that is landslide-susceptible terrain.
    #[allow(dead_code)] // read by the landslide cap once the pack carries it everywhere
    pub fn landslide_susceptible_share(&self) -> Option<f64> {
        share(self.county.landslide_susceptible_share)
            .or_else(|| shown_share(&self.shown.landslide_susceptible_share))
    }

    /// People behind levees.
    pub fn levees(&self) -> Option<Levees> {
        let pop_share = share(self.county.leveed_pop_share)
            .or_else(|| shown_share(&self.shown.leveed_pop_share))?;
        Some(Levees {
            pop_share,
            high_share: share(self.county.levee_risk_high_share).unwrap_or(0.0),
        })
    }

    /// High-hazard dams near the ZIP code and in the county.
    pub fn dams(&self) -> Dams {
        Dams {
            zip_downstream: self.shown.dams_high_within_10km.as_ref().map(|s| s.value),
            county_total: self.county.dams_high_total.or(self.facility_dams),
            county_poor: self.county.dams_high_poor_condition,
        }
    }

    /// Eviction filings per renter household a year in the county (present only once the
    /// owner has approved Eviction Lab's attribution licence).
    pub fn eviction_filing_rate(&self) -> Option<f64> {
        share(self.county.eviction_filing_rate).or_else(|| shown_share(&self.shown.eviction_rate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> (CountyRecord, LocationResolved) {
        let json = include_str!("../tests/data/counties/42101.json");
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        (
            serde_json::from_value(v["county"].clone()).unwrap(),
            serde_json::from_value(v["location"].clone()).unwrap(),
        )
    }

    #[test]
    fn the_area_name_finds_the_metro_share() {
        let (mut c, l) = record();
        c.exposure.uasi_area = Some("Philadelphia-Camden-Wilmington, PA-NJ-DE-MD".into());
        c.exposure.uasi_share = Some(0.007_295);
        assert_eq!(
            Exposure::new(&c, &l).uasi(),
            Uasi::Funded {
                area: Some("Philadelphia-Camden-Wilmington, PA-NJ-DE-MD".into()),
                metro_share: 0.02842
            }
        );
        c.exposure.uasi_area = None;
        c.exposure.uasi_share = Some(0.0);
        assert_eq!(Exposure::new(&c, &l).uasi(), Uasi::NotFunded);
        c.exposure.uasi_share = None;
        assert_eq!(Exposure::new(&c, &l).uasi(), Uasi::Unknown);
    }

    #[test]
    fn missing_columns_are_unknown_not_zero() {
        let (mut c, l) = record();
        c.exposure = CountyExposure::default();
        let e = Exposure::new(&c, &l);
        assert!(e.strategic_class().is_none() && e.smoke().is_none() && e.karst_share().is_none());
        assert!(e.levees().is_none() && e.geomag().is_none());
        // Dams fall back to the facilities count, which the v1 pack already has.
        assert_eq!(e.dams().county_total, Some(1));
    }
}
