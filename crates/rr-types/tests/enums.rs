//! Every enum in the contract round-trips through its string in every direction: `as_str`,
//! `Display`, `FromStr` and serde agree, strings are unique snake_case, and `ALL` is in order.

use std::collections::BTreeSet;
use std::fmt::{Debug, Display};
use std::str::FromStr;

use rr_types::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

fn check<T>(name: &str, all: &[T], strs: &[&str], as_str: fn(T) -> &'static str)
where
    T: Copy + Ord + Debug + Display + FromStr + Serialize + DeserializeOwned,
    <T as FromStr>::Err: Debug,
{
    assert!(!all.is_empty(), "{name} has no values");
    assert_eq!(
        all.len(),
        strs.len(),
        "{name}: ALL and STRS differ in length"
    );
    let unique: BTreeSet<&str> = strs.iter().copied().collect();
    assert_eq!(unique.len(), strs.len(), "{name}: duplicate strings");
    for (i, (&v, &s)) in all.iter().zip(strs).enumerate() {
        assert!(is_well_formed_id(s), "{name}: {s} is not snake_case");
        assert_eq!(as_str(v), s, "{name}");
        assert_eq!(v.to_string(), s, "{name}");
        assert_eq!(s.parse::<T>().unwrap(), v, "{name}");
        assert_eq!(
            serde_json::to_string(&v).unwrap(),
            format!("\"{s}\""),
            "{name}"
        );
        assert_eq!(
            serde_json::from_str::<T>(&format!("\"{s}\"")).unwrap(),
            v,
            "{name}"
        );
        if i > 0 {
            assert!(all[i - 1] < v, "{name}: ALL is not in declaration order");
        }
    }
    assert!("not_a_real_value".parse::<T>().is_err(), "{name}");
    assert!(
        serde_json::from_str::<T>("\"NOT_A_REAL_VALUE\"").is_err(),
        "{name}"
    );
}

macro_rules! check_all {
    ($($t:ident),+ $(,)?) => {
        $( check::<$t>(stringify!($t), $t::ALL, $t::STRS, $t::as_str); )+
    };
}

#[test]
fn every_enum_round_trips_through_its_string() {
    check_all!(
        HazardId,
        HazardTier,
        BucketId,
        BucketKind,
        TargetKind,
        TierId,
        Setting,
        HousingKind,
        Tenure,
        WaterSource,
        Wastewater,
        Heating,
        Cooling,
        BackupPower,
        AgeBand,
        Mobility,
        CommuteMode,
        Fuel,
        IncomeStability,
        ReturnPeriod,
        ClimateHorizon,
        WaterLevel,
        Stage,
        ProblemCode,
        DataConfidence,
        HazardDisplay,
        Per,
        PlanItemKind,
        WarningSeverity,
        Evidence,
        ErrorCode,
        ExplainKind,
        AfreqKind,
        AccessNeed,
        CookingFuel,
        RawWaterSource,
        WaterSystemRecord,
        Benefit,
        Holds,
        Season,
        GuidanceKind,
    );
}

#[test]
fn powered_device_round_trips() {
    for d in [
        PoweredDevice::None,
        PoweredDevice::Cpap,
        PoweredDevice::Oxygen,
        PoweredDevice::Other { watts: 75.5 },
    ] {
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(serde_json::from_str::<PoweredDevice>(&json).unwrap(), d);
    }
}
