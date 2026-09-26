//! Bucket targets for the fixture households, built the way `rr-consequence` will return them.
//!
//! Philadelphia and Coos Bay use the research targets at the default dial
//! (docs/research/risk-model.md §3.8; the planner's summary: Philadelphia about 3 days of power and
//! water, 10 days of food, 14 days of medication; Coos Bay about 13 days of power and 50 days
//! without well water). The other fixtures get a plain generic set.
#![allow(dead_code)]

use rr_types::{
    BucketAssessment, BucketId, CitationId, Contribution, HazardId, PlanInput, Target, TierId,
};

/// A stand-in for the consequence crate's own sources.
pub const TARGET_SOURCE: &str = "consequence_target_source";

fn assessment(id: BucketId, target: Target) -> BucketAssessment {
    BucketAssessment {
        id,
        name: id.name().to_owned(),
        target,
        covered: target,
        covered_today: target,
        tier_enough: TierId::H72,
        contributions: Vec::new(),
        frequency_sentences: Vec::new(),
        sources: vec![CitationId::from(TARGET_SOURCE)],
        relief: None,
    }
}

pub fn days(id: BucketId, value: f32) -> BucketAssessment {
    assessment(
        id,
        Target::Days {
            value,
            low: value,
            high: value,
        },
    )
}

pub fn months(value: f32) -> BucketAssessment {
    assessment(
        BucketId::Income,
        Target::Months {
            value,
            low: value,
            high: value,
        },
    )
}

pub fn readiness(id: BucketId, p: f64) -> BucketAssessment {
    assessment(
        id,
        Target::Readiness {
            p_need_10yr: p,
            done: 0,
            of: 5,
        },
    )
}

pub fn evacuate(p: f64, notice_low: f32, notice_high: f32, days_away: f32) -> BucketAssessment {
    assessment(
        BucketId::Evacuate,
        Target::Evacuate {
            p_need_10yr: p,
            notice_hours_low: notice_low,
            notice_hours_high: notice_high,
            days_away,
        },
    )
}

pub fn driven_by(mut b: BucketAssessment, hazards: &[(HazardId, f32)]) -> BucketAssessment {
    b.contributions = hazards
        .iter()
        .map(|&(hazard, share)| Contribution { hazard, share })
        .collect();
    b
}

/// Philadelphia rowhouse renters (research §8.4, default dial).
pub fn philadelphia() -> Vec<BucketAssessment> {
    vec![
        days(BucketId::Power, 3.0),
        days(BucketId::WaterBoil, 4.0),
        days(BucketId::WaterOut, 3.0),
        days(BucketId::Supplies, 10.0),
        driven_by(
            days(BucketId::Thermal, 1.7),
            &[
                (HazardId::HeatWave, 0.6),
                (HazardId::ColdWave, 0.2),
                (HazardId::WinterWeather, 0.2),
            ],
        ),
        days(BucketId::Medication, 14.0),
        days(BucketId::Comms, 1.4),
        evacuate(0.05, 1.0, 24.0, 3.0),
        readiness(BucketId::GetHome, 0.1),
        readiness(BucketId::MedicalEmergency, 0.9),
        readiness(BucketId::Fire, 0.05),
        readiness(BucketId::Security, 0.1),
        months(3.8),
        driven_by(
            readiness(BucketId::HomeLoss, 0.05),
            &[
                (HazardId::RiverineFlooding, 0.5),
                (HazardId::HouseFire, 0.5),
            ],
        ),
    ]
}

/// Coos Bay well owners (the planner's targets: 13 days of power, 50 days without well water; the
/// rest from research §3.8 at the default dial). The wood stove covers cold (thermal 0); a
/// private well has no boil-water notices.
pub fn coos_bay() -> Vec<BucketAssessment> {
    vec![
        days(BucketId::Power, 13.0),
        days(BucketId::WaterOut, 50.0),
        days(BucketId::Supplies, 17.0),
        days(BucketId::Thermal, 0.0),
        days(BucketId::Medication, 21.0),
        days(BucketId::Comms, 6.0),
        evacuate(0.1, 0.25, 0.33, 14.0),
        readiness(BucketId::GetHome, 0.05),
        readiness(BucketId::MedicalEmergency, 0.9),
        readiness(BucketId::Fire, 0.03),
        readiness(BucketId::Security, 0.1),
        months(5.8),
        driven_by(
            readiness(BucketId::HomeLoss, 0.1),
            &[
                (HazardId::Earthquake, 0.6),
                (HazardId::RiverineFlooding, 0.2),
                (HazardId::HouseFire, 0.2),
            ],
        ),
    ]
}

/// A plain set for the other fixtures.
pub fn generic() -> Vec<BucketAssessment> {
    vec![
        days(BucketId::Power, 3.0),
        days(BucketId::WaterBoil, 3.0),
        days(BucketId::WaterOut, 3.0),
        days(BucketId::Supplies, 10.0),
        days(BucketId::Thermal, 2.0),
        days(BucketId::Medication, 14.0),
        days(BucketId::Comms, 2.0),
        evacuate(0.05, 12.0, 48.0, 3.0),
        readiness(BucketId::GetHome, 0.1),
        readiness(BucketId::MedicalEmergency, 0.9),
        readiness(BucketId::Fire, 0.05),
        readiness(BucketId::Security, 0.1),
        months(4.0),
        readiness(BucketId::HomeLoss, 0.05),
    ]
}

/// The targets used for each fixture household.
pub fn targets_for(name: &str) -> Vec<BucketAssessment> {
    match name {
        "philadelphia-renters-4" => philadelphia(),
        "coos-bay-well-owner-2" => coos_bay(),
        _ => generic(),
    }
}

/// Every fixture with its targets.
pub fn all() -> Vec<(&'static str, PlanInput, Vec<BucketAssessment>)> {
    rr_types::fixtures::all()
        .into_iter()
        .map(|(name, input)| (name, input, targets_for(name)))
        .collect()
}
