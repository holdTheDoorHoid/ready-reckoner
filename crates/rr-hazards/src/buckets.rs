//! Which consequence buckets each hazard feeds, for `HazardProfile::buckets` ("what it does to
//! you" on the hazard card). From the hazard-to-bucket map in research §2.4; `rr-consequence`
//! owns the conditional probabilities and durations.

use rr_types::{BucketId, HazardId};

/// The buckets `hazard` feeds, in `BucketId` order.
#[allow(deprecated)] // the retired `terrorism` still needs an arm while the id exists
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
        H::NuclearAttack => vec![Power, Supplies, Medication, Comms, Evacuate, Income],
        H::Terrorism => vec![Supplies, Security],
        H::JobLoss => vec![Income],
        H::HouseFire => vec![Evacuate, Fire, HomeLoss],
        H::MedicalEmergency => vec![MedicalEmergency],
        H::VehicleStranding => vec![GetHome],
        H::LocalUtilityOutage => vec![WaterBoil, WaterOut],
        H::Burglary => vec![Security],
        H::EarnerDeathOrDisability => vec![Income],
        H::ExtendedHouseholdIllness => vec![Supplies, Medication, Income],
        // Contract v2 (REVIEW §2.2, §2.3; hazard-candidates.csv "buckets").
        H::WildfireSmoke => vec![MedicalEmergency, CleanAir],
        H::DustStorm => vec![Supplies, GetHome, CleanAir],
        H::Sinkhole => vec![Evacuate, HomeLoss],
        H::GeomagneticStorm => vec![Power, WaterOut, Medication, Comms],
        H::Vei7Eruption => vec![Supplies, Income],
        H::DamFailure => vec![Power, WaterOut, Evacuate, HomeLoss],
        H::NetworkOutage => vec![Comms, MedicalEmergency],
        H::DrugShortage => vec![Medication],
        H::BenefitInterruption => vec![Supplies, Income],
        H::AttackDisruption => vec![Supplies, Comms, GetHome, Security],
        H::MultiMonthBlackout => vec![Power, WaterOut, Medication],
        H::WarInfrastructure => vec![Power, WaterOut, Supplies, Comms],
        H::CbrnAttack => vec![Supplies, Medication, Comms, Evacuate],
        H::SeverePandemic => vec![Supplies, Medication, Income],
        H::FinancialCrisis => vec![Comms, Income],
        H::MassViolence => vec![MedicalEmergency, Security],
        H::WaterDamage => vec![WaterOut, HomeLoss],
        H::Eviction => vec![Evacuate, Income, HomeLoss],
        H::ArrestOrDetention => vec![Income, HomeLoss],
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
        // Smoke and dust fill the clean-air bucket (owner decision, REVIEW §8).
        assert!(for_hazard(HazardId::WildfireSmoke).contains(&BucketId::CleanAir));
        assert!(for_hazard(HazardId::DustStorm).contains(&BucketId::CleanAir));
        assert_eq!(
            for_hazard(HazardId::ArrestOrDetention),
            [BucketId::Income, BucketId::HomeLoss]
        );
    }
}
