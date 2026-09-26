//! A generic, national-ish rate set for any household, so property tests exercise every effect
//! row. These are test inputs, not estimates: rr-hazards supplies the real rates.

#![allow(dead_code)]

use rr_types::{CitationId, Evidence, HazardId, HouseholdEventRate, PlanInput};

fn r(h: HazardId, rate: f64) -> HouseholdEventRate {
    HouseholdEventRate {
        hazard: h,
        rate_per_year: rate,
        low: rate / 2.0,
        high: rate * 2.0,
        evidence: Evidence::Prior,
        sources: vec![CitationId::from("prior_rr_event_shares")],
    }
}

/// Every hazard at a plausible household rate (per-person and per-earner hazards scaled).
pub fn rates(input: &PlanInput) -> Vec<HouseholdEventRate> {
    use HazardId::*;
    let people = input.people.len() as f64;
    let earners = f64::from(input.finances.income.earners);
    let vehicles = input.mobility.vehicles.len().max(1) as f64;
    let mut v = vec![
        r(Avalanche, 0.0005),
        r(CoastalFlooding, 0.005),
        r(ColdWave, 0.2),
        r(Drought, 0.05),
        r(Earthquake, 0.004),
        r(Hail, 0.1),
        r(HeatWave, 1.0),
        r(Hurricane, 0.05),
        r(IceStorm, 0.05),
        r(Landslide, 0.002),
        r(Lightning, 0.1),
        r(RiverineFlooding, 0.01),
        r(StrongWind, 0.5),
        r(Tornado, 0.002),
        r(Tsunami, 0.001),
        r(VolcanicActivity, 0.0001),
        r(Wildfire, 0.02),
        r(WinterWeather, 0.3),
        r(Pandemic, 0.046),
        r(GridFailure, 0.005),
        r(CyberOutage, 0.02),
        r(CivilUnrest, 0.03),
        r(SupplyChainDisruption, 0.2),
        r(HazmatRelease, 0.02),
        r(NuclearPlantIncident, 0.0001),
        r(NuclearAttack, 0.0005),
        r(Terrorism, 0.0005),
        r(HouseFire, 0.0026),
        r(MedicalEmergency, 0.473 * people),
        r(VehicleStranding, 0.05 * vehicles),
        r(LocalUtilityOutage, 0.15),
        r(Burglary, 0.02),
        r(ExtendedHouseholdIllness, 0.02),
    ];
    if earners > 0.0 {
        v.push(r(JobLoss, 0.083 * earners));
        v.push(r(EarnerDeathOrDisability, 0.004 * earners));
    }
    v
}
