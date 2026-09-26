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
        sources: vec![CitationId::from("rr_risk_model_priors")],
    }
}

/// Every ranked hazard at a plausible household rate (per-person and per-earner hazards
/// scaled), plus the rare families at their published middle rates so the tests can check that
/// they never reach a bucket. The retired `terrorism` id is gone: `attack_disruption` (ranked)
/// and `mass_violence` (rare) replace it.
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
        r(HouseFire, 0.0026),
        r(MedicalEmergency, 0.473 * people),
        r(VehicleStranding, 0.05 * vehicles),
        r(LocalUtilityOutage, 0.15),
        r(Burglary, 0.02),
        r(ExtendedHouseholdIllness, 0.02),
    ];
    // Contract v2 ranked hazards (hazard-candidates.csv typical rates; rr-hazards gates the
    // conditional ones, which the effects rows gate again by household).
    let daily_rx = input.people.iter().filter(|p| p.medical.daily_rx).count() as f64;
    v.extend([
        r(WildfireSmoke, 0.5),
        r(DustStorm, 0.05),
        r(Sinkhole, 0.0002),
        r(DamFailure, 0.00001),
        r(NetworkOutage, 0.3),
        r(AttackDisruption, 0.0009),
        r(WaterDamage, 0.015),
        r(ArrestOrDetention, 0.02 * people),
    ]);
    if daily_rx > 0.0 {
        v.push(r(DrugShortage, 0.05 * daily_rx));
    }
    if !input.finances.benefits.is_empty() {
        v.push(r(BenefitInterruption, 0.077));
    }
    if input.housing.tenure == rr_types::Tenure::Rent {
        v.push(r(Eviction, 0.023));
    }
    // Rare families: never in a bucket's curve.
    v.extend([
        r(GeomagneticStorm, 0.00027),
        r(Vei7Eruption, 0.0018),
        r(WarInfrastructure, 0.0025),
        r(CbrnAttack, 0.00015),
        r(SeverePandemic, 0.0015),
        r(FinancialCrisis, 0.002),
        r(MassViolence, 0.000001),
    ]);
    if earners > 0.0 {
        v.push(r(JobLoss, 0.083 * earners));
        v.push(r(EarnerDeathOrDisability, 0.004 * earners));
    }
    v
}
