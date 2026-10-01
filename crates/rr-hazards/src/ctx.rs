//! The inputs of one `assess` call, with lookups into the county record and base rates.

use rr_types::{
    AfreqKind, BaseRate, ClimateHorizon, CountyRecord, EventRate, HazardId, LocationResolved,
    NriHazard, PlanInput, Setting, WaterSource, math,
};

use crate::exposure::Exposure;

/// Notes collected while computing, in the order they arose.
#[derive(Debug, Default)]
pub(crate) struct Notes(pub Vec<String>);

impl Notes {
    /// Adds a note once.
    pub fn add(&mut self, note: impl Into<String>) {
        let note = note.into();
        if !self.0.contains(&note) {
            self.0.push(note);
        }
    }
}

/// Everything one `assess` call reads.
pub(crate) struct Ctx<'a> {
    pub input: &'a PlanInput,
    pub county: &'a CountyRecord,
    pub base_rates: &'a [BaseRate],
    pub location: &'a LocationResolved,
}

impl<'a> Ctx<'a> {
    pub fn new(
        input: &'a PlanInput,
        county: &'a CountyRecord,
        base_rates: &'a [BaseRate],
        location: &'a LocationResolved,
    ) -> Self {
        Ctx {
            input,
            county,
            base_rates,
            location,
        }
    }

    /// Urban, suburban or rural, as the household describes it.
    pub fn setting(&self) -> Setting {
        self.input.location.setting
    }

    /// The 2050 dial is on.
    pub fn y2050(&self) -> bool {
        self.input.dials.climate == ClimateHorizon::Y2050
    }

    /// `dials.horizon_years`, at least 1.
    pub fn years(&self) -> u8 {
        self.input.dials.horizon_years.max(1)
    }

    /// The county's name for sentences, as the Census Bureau writes it ("Coos County", "Baltimore
    /// city", "Orleans Parish", "Anchorage Municipality"). A record without the full name (the
    /// hand-built sample counties) rebuilds it from the short name.
    pub fn county_label(&self) -> String {
        if let Some(full) = self
            .county
            .name_full
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return full.to_owned();
        }
        let name = &self.county.name;
        // Parishes, boroughs and independent cities keep their own word.
        let lower = name.to_ascii_lowercase();
        if [
            "county",
            "parish",
            "borough",
            "city",
            "municipality",
            "census area",
        ]
        .iter()
        .any(|w| lower.ends_with(w))
        {
            name.clone()
        } else if self.county.state_abbr == "LA" {
            format!("{name} Parish")
        } else {
            format!("{name} County")
        }
    }

    /// The NRI fields for `hazard`, if the county has them.
    pub fn nri(&self, hazard: HazardId) -> Option<&'a NriHazard> {
        self.county.nri.get(&hazard)
    }

    /// NRI annualised frequency as events a year, following the record's `afreq_kind`: an
    /// annual probability p becomes the rate −ln(1 − p). Absent, zero or not finite: `None`.
    ///
    /// A value marked as a probability but at or above 1 cannot be one; it is read as a count
    /// (NRI v1.20's update documentation labels county landslide frequencies "annualized
    /// probability", yet Coos County's is 12.8). This is a data-labelling matter, documented in
    /// `docs/RISK_MODEL.md`, not a caveat for the household, so no note is added.
    pub fn afreq_rate(&self, hazard: HazardId) -> Option<f64> {
        let n = self.nri(hazard)?;
        let a = f64::from(n.afreq?);
        if !(a.is_finite() && a > 0.0) {
            return None;
        }
        Some(match n.afreq_kind {
            AfreqKind::EventsPerYear => a,
            AfreqKind::AnnualProbability if a < 1.0 => -math::ln_1p(-a),
            AfreqKind::AnnualProbability => a,
        })
    }

    /// NRI annualised frequency exactly as recorded (for wildfire, a burn probability).
    pub fn afreq_raw(&self, hazard: HazardId) -> Option<f64> {
        let a = f64::from(self.nri(hazard)?.afreq?);
        (a.is_finite() && a > 0.0).then_some(a)
    }

    /// Share of the county's residents in the hazard's exposure area: NRI `EXPP` / population,
    /// capped at 1.
    pub fn exposure_share(&self, hazard: HazardId) -> Option<f64> {
        let expp = f64::from(self.nri(hazard)?.expp?);
        let pop = f64::from(self.county.population?);
        (expp.is_finite() && expp >= 0.0 && pop > 0.0).then(|| (expp / pop).min(1.0))
    }

    /// NRI historic loss ratio for buildings (`HLRB`), if positive.
    pub fn hlrb(&self, hazard: HazardId) -> Option<f64> {
        let h = f64::from(self.nri(hazard)?.hlrb?);
        (h.is_finite() && h > 0.0).then_some(h)
    }

    /// Expected annual loss per household in US dollars: NRI `EALT` / households.
    pub fn eal_per_household(&self, hazard: HazardId) -> Option<f64> {
        let ealt = f64::from(self.nri(hazard)?.ealt?);
        let hh = f64::from(self.county.households?);
        (ealt.is_finite() && ealt >= 0.0 && hh > 0.0).then(|| ealt / hh)
    }

    /// A per-county event rate (the `events` table), keyed by hazard id or event type.
    pub fn event(&self, key: &str) -> Option<&'a EventRate> {
        self.county
            .events
            .get(key)
            .filter(|e| e.rate_per_year.is_finite() && e.rate_per_year > 0.0)
    }

    /// The first base rate with one of these ids and a usable value.
    pub fn base_rate(&self, ids: &[&str]) -> Option<&'a BaseRate> {
        ids.iter().find_map(|id| {
            self.base_rates
                .iter()
                .find(|b| b.id == *id && b.value.is_finite() && b.value >= 0.0)
        })
    }

    /// People marked as earners.
    pub fn earners(&self) -> usize {
        self.input.people.iter().filter(|p| p.earner).count()
    }

    /// People in the household.
    pub fn people(&self) -> usize {
        self.input.people.len()
    }

    /// The household's vehicles.
    pub fn vehicles(&self) -> usize {
        self.input.mobility.vehicles.len()
    }

    /// The household lives at street level or has a basement (flood-exposed).
    pub fn ground_level(&self) -> bool {
        self.input.housing.floor <= 1
    }

    /// The home draws water from a private well.
    pub fn well(&self) -> bool {
        self.input.housing.water == WaterSource::Well
    }

    /// The data pack's v2 exposure columns for this county and ZIP code.
    pub fn exposure(&self) -> Exposure<'a> {
        Exposure::new(self.county, self.location)
    }

    /// The county's plain name with its state ("Jefferson County, Texas").
    pub fn county_and_state(&self) -> String {
        format!("{}, {}", self.county_label(), self.county.state_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(county: &CountyRecord) -> String {
        let input = rr_types::fixtures::get("philadelphia-renters-4").unwrap();
        let json = include_str!("../tests/data/counties/42101.json");
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let location: LocationResolved = serde_json::from_value(v["location"].clone()).unwrap();
        Ctx::new(&input, county, &[], &location).county_label()
    }

    #[test]
    fn the_label_is_the_census_name() {
        // The pack's own county table (core/counties.csv), read as the engine reads it.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/core/counties.csv");
        let Ok(csv) = std::fs::read(path) else {
            eprintln!("data/core/counties.csv not found: skipped");
            return;
        };
        let mut s = rr_data::DataStore::new();
        s.load_pack("core/counties.csv", &csv).unwrap();
        for (fips, census) in [
            // Independent cities are not the county of the same name.
            ("24510", "Baltimore city"),
            ("51760", "Richmond city"),
            ("29510", "St. Louis city"),
            ("32510", "Carson City"),
            ("24005", "Baltimore County"),
            // And the other county types the short name lost.
            ("51036", "Charles City County"),
            ("12057", "Hillsborough County"),
            ("22071", "Orleans Parish"),
            ("02020", "Anchorage Municipality"),
            ("02110", "Juneau City and Borough"),
            ("72127", "San Juan Municipio"),
            ("11001", "District of Columbia"),
        ] {
            assert_eq!(label(s.county(fips).unwrap()), census, "{fips}");
        }
    }

    #[test]
    fn a_record_without_the_census_name_rebuilds_it() {
        let json = include_str!("../tests/data/counties/42101.json");
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut c: CountyRecord = serde_json::from_value(v["county"].clone()).unwrap();
        assert_eq!(c.name_full, None, "the hand-built sample counties lack it");
        assert_eq!(label(&c), "Philadelphia County");
        (c.name, c.state_abbr) = ("Cameron".into(), "LA".into());
        assert_eq!(label(&c), "Cameron Parish");
        (c.name, c.state_abbr) = ("Carson City".into(), "NV".into());
        assert_eq!(label(&c), "Carson City");
        // A blank full name is no name.
        c.name_full = Some(" ".into());
        assert_eq!(label(&c), "Carson City");
        c.name_full = Some("Carson City".into());
        assert_eq!(label(&c), "Carson City");
    }
}
