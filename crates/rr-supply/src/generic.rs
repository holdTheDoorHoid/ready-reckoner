//! Generic rules: counts (`per_person`, `per_vehicle`, ...) and switches (`once_if_*`, 1 or 0) that
//! the plan applies to catalogue items directly. They emit no requirement lines; a switch that is 0
//! keeps the item out of this household's plan (docs/QUANTITY_RULES.md).

use rr_types::{
    AgeBand, BackupPower, Fuel, Heating, HousingKind, Mobility, Per, PlanInput, PoweredDevice,
    Tenure, WaterSource,
};

use crate::SupplyContext;
use crate::household::{Household, is_13_plus};

/// The generic rule ids, in docs/QUANTITY_RULES.md order.
pub const GENERIC_RULES: &[&str] = &[
    "once",
    "per_person",
    "per_person_13_plus",
    "per_commuter",
    "per_vehicle",
    "per_pet",
    "per_infant_or_toddler",
    "once_if_daily_rx",
    "once_if_refrigerated_rx",
    "once_if_powered_device",
    "once_if_infant",
    "once_if_children",
    "once_if_pets",
    "once_if_large_animals",
    "once_if_vehicle",
    "once_if_ev",
    "once_if_commuter",
    "once_if_senior",
    "once_if_mobility_needs",
    "once_if_wheelchair",
    "once_if_pregnant_or_nursing",
    "once_if_renter",
    "once_if_well",
    "once_if_earner",
    "once_if_gas_service",
    "once_if_house",
    "once_if_house_ground_floor",
    "once_if_near_nuclear_plant",
    "once_if_generator",
];

fn switch(on: bool) -> (f64, Per) {
    (if on { 1.0 } else { 0.0 }, Per::Household)
}

/// The quantity for a generic rule, or `None` if `rule` is not generic.
pub(crate) fn quantity(rule: &str, input: &PlanInput, ctx: &SupplyContext) -> Option<(f64, Per)> {
    let h = Household::new(input);
    let people = &input.people;
    let any = |f: &dyn Fn(&rr_types::Person) -> bool| people.iter().any(f);
    let count =
        |f: &dyn Fn(&rr_types::Person) -> bool| people.iter().filter(|p| f(p)).count() as f64;
    Some(match rule {
        "once" => (1.0, Per::Household),
        "per_person" => (h.n() as f64, Per::Person),
        "per_person_13_plus" => (count(&is_13_plus), Per::Person),
        "per_commuter" => (h.commuters().len() as f64, Per::Commuter),
        "per_vehicle" => (h.vehicles() as f64, Per::Household),
        "per_pet" => (f64::from(h.household_pets()), Per::Pet),
        "per_infant_or_toddler" => (
            count(&|p| matches!(p.age_band, AgeBand::Infant | AgeBand::Toddler)),
            Per::Person,
        ),
        "once_if_daily_rx" => switch(any(&|p| p.medical.daily_rx)),
        "once_if_refrigerated_rx" => switch(any(&|p| p.medical.refrigerated_rx)),
        "once_if_powered_device" => {
            switch(any(&|p| p.medical.powered_device != PoweredDevice::None))
        }
        "once_if_infant" => switch(any(&|p| p.age_band == AgeBand::Infant)),
        "once_if_children" => switch(any(&|p| {
            matches!(
                p.age_band,
                AgeBand::Infant | AgeBand::Toddler | AgeBand::Child | AgeBand::Teen
            )
        })),
        "once_if_pets" => switch(h.household_pets() > 0),
        "once_if_large_animals" => switch(input.pets.large_animals > 0),
        "once_if_vehicle" => switch(h.vehicles() > 0),
        "once_if_ev" => switch(input.mobility.vehicles.iter().any(|v| v.fuel == Fuel::Ev)),
        "once_if_commuter" => switch(!h.commuters().is_empty()),
        "once_if_senior" => switch(any(&|p| p.age_band == AgeBand::Senior)),
        "once_if_mobility_needs" => switch(any(&|p| p.medical.mobility != Mobility::None)),
        "once_if_wheelchair" => switch(any(&|p| p.medical.mobility == Mobility::Wheelchair)),
        "once_if_pregnant_or_nursing" => switch(any(&|p| p.pregnant_or_nursing)),
        "once_if_renter" => switch(input.housing.tenure == Tenure::Rent),
        "once_if_well" => switch(input.housing.water == WaterSource::Well),
        "once_if_earner" => switch(any(&|p| p.earner)),
        "once_if_gas_service" => switch(matches!(
            input.housing.heating,
            Heating::Gas | Heating::Propane
        )),
        "once_if_house" => switch(matches!(
            input.housing.kind,
            HousingKind::Detached
                | HousingKind::Rowhouse
                | HousingKind::MobileHome
                | HousingKind::RuralProperty
        )),
        // An outdoor light at the door or path of a house the household lives in at street level
        // (floor 1 or below), not a flat upstairs (round-2 review P-14).
        "once_if_house_ground_floor" => switch(
            matches!(
                input.housing.kind,
                HousingKind::Detached
                    | HousingKind::Rowhouse
                    | HousingKind::MobileHome
                    | HousingKind::RuralProperty
            ) && input.housing.floor <= 1,
        ),
        "once_if_near_nuclear_plant" => switch(ctx.nuclear_plant_within_16km == Some(true)),
        "once_if_generator" => switch(input.housing.backup_power == BackupPower::Generator),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    fn q(name: &str, rule: &str) -> f64 {
        let input = fixtures::get(name).unwrap();
        quantity(rule, &input, &SupplyContext::default()).unwrap().0
    }

    #[test]
    fn counts_and_switches() {
        let p = "philadelphia-renters-4";
        assert_eq!(q(p, "per_person"), 4.0);
        assert_eq!(q(p, "per_person_13_plus"), 3.0);
        assert_eq!(q(p, "per_commuter"), 2.0);
        assert_eq!(q(p, "once_if_daily_rx"), 1.0);
        assert_eq!(q(p, "once_if_refrigerated_rx"), 0.0);
        assert_eq!(q(p, "once_if_children"), 1.0);
        assert_eq!(q(p, "once_if_renter"), 1.0);
        assert_eq!(q(p, "once_if_gas_service"), 1.0);
        assert_eq!(q(p, "once_if_house"), 1.0);
        assert_eq!(q(p, "once_if_senior"), 1.0);
        assert_eq!(q(p, "once_if_mobility_needs"), 1.0);
        assert_eq!(q(p, "once_if_near_nuclear_plant"), 0.0);
        let sl = "sugar-land-ev-household-3";
        assert_eq!(q(sl, "once_if_ev"), 1.0);
        assert_eq!(q(sl, "once_if_infant"), 1.0);
        assert_eq!(q(sl, "per_infant_or_toddler"), 1.0);
        let coos = "coos-bay-well-owner-2";
        assert_eq!(q(coos, "once_if_well"), 1.0);
        assert_eq!(q(coos, "once_if_generator"), 1.0);
        assert_eq!(q(coos, "per_pet"), 3.0);
        assert_eq!(q(p, "once_if_house_ground_floor"), 1.0);
        let miami = "miami-condo-retiree-1";
        assert_eq!(q(miami, "once_if_house"), 0.0);
        assert_eq!(q(miami, "once_if_house_ground_floor"), 0.0);
        assert_eq!(
            q("phoenix-apartment-cpap-1", "once_if_house_ground_floor"),
            0.0
        );
        let mut upstairs = fixtures::get(p).unwrap();
        upstairs.housing.floor = 2;
        assert_eq!(
            quantity(
                "once_if_house_ground_floor",
                &upstairs,
                &SupplyContext::default()
            )
            .unwrap()
            .0,
            0.0
        );
        assert_eq!(q(miami, "once_if_earner"), 0.0);
        assert_eq!(q("hays-kansas-farm-5", "once_if_large_animals"), 1.0);
        assert_eq!(q("hays-kansas-farm-5", "once_if_pregnant_or_nursing"), 1.0);
        let input = fixtures::get(p).unwrap();
        let near = SupplyContext {
            nuclear_plant_within_16km: Some(true),
            ..SupplyContext::default()
        };
        assert_eq!(
            quantity("once_if_near_nuclear_plant", &input, &near)
                .unwrap()
                .0,
            1.0
        );
        assert!(quantity("water_gallons", &input, &near).is_none());
    }

    #[test]
    fn every_generic_rule_answers() {
        let input = fixtures::get("hays-kansas-farm-5").unwrap();
        for r in GENERIC_RULES {
            let (q, _) =
                quantity(r, &input, &SupplyContext::default()).unwrap_or_else(|| panic!("{r}"));
            assert!(q >= 0.0 && q.is_finite(), "{r}");
        }
    }
}
