//! Which consequence buckets each hazard feeds, for `HazardProfile::buckets` ("what it does to
//! you" on the hazard card). From the hazard-to-bucket map in research §2.4; `rr-consequence`
//! owns the conditional probabilities and durations.

use rr_types::{BucketId, HazardId};

/// The buckets `hazard` feeds, in `BucketId` order.
pub(crate) fn for_hazard(hazard: HazardId) -> Vec<BucketId> {
    use BucketId::*;
    use HazardId as H;
    let mut v: Vec<BucketId> = match hazard {
        H::Avalanche => vec![Power, Evacuate, GetHome],
        H::CoastalFlooding => vec![Power, WaterBoil, Evacuate, HomeLoss],
        H::ColdWave => vec![Power, WaterOut, Thermal],
        H::Drought => vec![WaterOut],
        H::Earthquake => vec![
            Power,
            WaterOut,
            Supplies,
            Medication,
            Comms,
            MedicalEmergency,
            HomeLoss,
        ],
        H::Hail => vec![Power, HomeLoss],
        H::HeatWave => vec![Power, Thermal],
        H::Hurricane => vec![
            Power, WaterBoil, WaterOut, Supplies, Comms, Evacuate, HomeLoss,
        ],
        H::IceStorm => vec![Power, WaterBoil, Supplies, Thermal],
        H::Landslide => vec![Evacuate, GetHome, HomeLoss],
        H::Lightning => vec![Power, Fire],
        H::RiverineFlooding => vec![Power, WaterBoil, Evacuate, HomeLoss],
        H::StrongWind => vec![Power, Comms, HomeLoss],
        H::Tornado => vec![Power, MedicalEmergency, HomeLoss],
        H::Tsunami => vec![Evacuate, HomeLoss],
        H::VolcanicActivity => vec![WaterBoil, Supplies, Evacuate],
        H::Wildfire => vec![Power, Evacuate, HomeLoss],
        H::WinterWeather => vec![Power, WaterBoil, Supplies, Thermal, GetHome],
        H::Pandemic => vec![Supplies, Medication, Income],
        H::GridFailure => vec![Power, WaterOut, Thermal, Comms],
        H::CyberOutage => vec![Medication, Comms],
        H::CivilUnrest => vec![Supplies, Security],
        H::SupplyChainDisruption => vec![Supplies, Medication],
        H::HazmatRelease => vec![WaterOut, Supplies, Evacuate],
        H::NuclearPlantIncident => vec![Supplies, Evacuate],
        H::NuclearAttack => vec![WaterOut, Supplies, Evacuate],
        H::Terrorism => vec![Supplies, Security],
        H::JobLoss => vec![Income],
        H::HouseFire => vec![Evacuate, Fire, HomeLoss],
        H::MedicalEmergency => vec![MedicalEmergency],
        H::VehicleStranding => vec![GetHome],
        H::LocalUtilityOutage => vec![WaterBoil, WaterOut],
        H::Burglary => vec![Security],
        H::EarnerDeathOrDisability => vec![Income],
        H::ExtendedHouseholdIllness => vec![Supplies, Medication, Income],
    };
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hazard_feeds_at_least_one_bucket_in_order() {
        for h in HazardId::ALL {
            let b = for_hazard(*h);
            assert!(!b.is_empty(), "{h}");
            assert!(b.windows(2).all(|w| w[0] < w[1]), "{h}");
        }
        assert_eq!(for_hazard(HazardId::JobLoss), [BucketId::Income]);
    }
}
