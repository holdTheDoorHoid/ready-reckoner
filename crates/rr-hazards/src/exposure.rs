//! The data pack's v2 exposure columns (DESIGN-DELTA §2) as rr-hazards reads them.
//!
//! County columns come from `CountyRecord::exposure` (`CountyExposure`, written by the data
//! pack's `strategic`, `geomag`, `smoke`, `ground`, `levees`, `water_systems`, `facilities` and
//! `eviction` jobs and assembled by rr-data). ZIP columns, which only a ZIP code can give (the
//! high-hazard dams whose listed downstream town lies in the ZIP), come from
//! `LocationResolved::exposure`, the copy the app shows; a county field the record lacks falls
//! back to that copy too. Absent means unknown, never zero, with one exception: without the
//! urban area's UASI share the attack, CBRN and crude-device terms count the county as outside
//! every funded area, and a note says so. Every hazard that reads a column says what it does
//! without it.

use rr_types::{
    CountyExposure, CountyRecord, LocationResolved, Sourced, StrategicClass, StrategicPlace,
};

/// A value, if present and finite.
fn finite(v: Option<f32>) -> Option<f64> {
    v.map(f64::from).filter(|x| x.is_finite())
}

/// A share, if present, finite and between 0 and 1.
fn share(v: Option<f32>) -> Option<f64> {
    finite(v).filter(|x| (0.0..=1.0).contains(x))
}

/// A share as the decimal the pack wrote (0.02842, not 0.028419999…), if present and between 0
/// and 1. The UASI shares are published four-figure numbers, so the metro weight keeps them as
/// published.
fn written_share(v: Option<f32>) -> Option<f64> {
    v.filter(|x| x.is_finite())
        .map(|x| x.to_string().parse::<f64>().unwrap_or(f64::from(x)))
        .filter(|x| (0.0..=1.0).contains(x))
}

/// A share from the app's copy, if present and between 0 and 1.
fn shown_share(v: &Option<Sourced<f64>>) -> Option<f64> {
    v.as_ref()
        .map(|s| s.value)
        .filter(|x| x.is_finite() && (0.0..=1.0).contains(x))
}

/// The metro weight w_m for the attack, CBRN and crude-nuclear-device rows: the county's FEMA
/// urban area's own share of the national FY2026 UASI money. DHS picks the urban areas and sets
/// the amounts by relative terrorism risk, so the share weighs the metro area.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Uasi {
    /// The county lies in a funded urban area: its name and the area's share of the money.
    Funded {
        /// FEMA's name for the urban area, when known.
        area: Option<String>,
        /// The area's own share of the national UASI total, 0 to 1 (`uasi_area_share`).
        metro_share: f64,
    },
    /// The county lies outside every funded urban area (`uasi_area_share` is 0).
    NotFunded,
    /// The pack has no `uasi_area_share` for the county. The rows fall back to a share of 0, as
    /// outside every funded urban area, and a note says so.
    Absent,
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

    /// The metro weight: the county's urban area and that area's own share of the UASI money
    /// (`uasi_area_share`, the same for every county in the area). The county's population split
    /// (`uasi_share`, which the app's copy shows) is not a metro weight and is not read here.
    pub fn uasi(&self) -> Uasi {
        match written_share(self.county.uasi_area_share) {
            Some(s) if s > 0.0 => Uasi::Funded {
                area: self.county.uasi_area.clone(),
                metro_share: s,
            },
            Some(_) => Uasi::NotFunded,
            None => Uasi::Absent,
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
    fn the_metro_weight_is_the_area_share_not_the_county_split() {
        let (mut c, l) = record();
        // Philadelphia County holds 0.7295 % of the national total; its urban area 2.842 %.
        assert_eq!(c.exposure.uasi_share, Some(0.007_295));
        match Exposure::new(&c, &l).uasi() {
            Uasi::Funded { area, metro_share } => {
                assert_eq!(
                    area.as_deref(),
                    Some("Philadelphia-Camden-Wilmington, PA-NJ-DE-MD")
                );
                assert_eq!(metro_share, 0.02842);
            }
            other => panic!("{other:?}"),
        }
        // A share without its area's name still weighs the metro area.
        c.exposure.uasi_area = None;
        assert!(matches!(
            Exposure::new(&c, &l).uasi(),
            Uasi::Funded { area: None, .. }
        ));
        c.exposure.uasi_area_share = Some(0.0);
        assert_eq!(Exposure::new(&c, &l).uasi(), Uasi::NotFunded);
        // Without the column the county split is not used in its place.
        c.exposure.uasi_area_share = None;
        assert_eq!(Exposure::new(&c, &l).uasi(), Uasi::Absent);
        // Nor is a value outside 0 to 1.
        c.exposure.uasi_area_share = Some(1.5);
        assert_eq!(Exposure::new(&c, &l).uasi(), Uasi::Absent);
    }

    #[test]
    fn missing_columns_are_unknown_not_zero() {
        let (mut c, l) = record();
        c.exposure = CountyExposure::default();
        let e = Exposure::new(&c, &l);
        assert!(e.strategic_class().is_none() && e.smoke().is_none() && e.karst_share().is_none());
        assert!(e.levees().is_none() && e.geomag().is_none());
        assert_eq!(e.uasi(), Uasi::Absent);
        // Dams fall back to the facilities count, which the v1 pack already has.
        assert_eq!(e.dams().county_total, Some(1));
    }
}
