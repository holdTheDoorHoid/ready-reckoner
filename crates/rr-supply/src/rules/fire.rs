//! Fire and security readiness: smoke and carbon monoxide alarms, an extinguisher, and neighbours.
//! Firearms are never sized or listed here (docs/PRINCIPLES.md §9).

use rr_types::{Housing, HousingKind, Per};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{count, num};

/// Levels of the home for alarms (the form does not ask; an estimate by kind of building).
fn levels(b: &mut Basis, housing: &Housing) -> f64 {
    match housing.kind {
        HousingKind::ApartmentHighRise
        | HousingKind::ApartmentLowRise
        | HousingKind::MobileHome => b.k(keys::ALARM_LEVELS_APARTMENT),
        HousingKind::Rowhouse | HousingKind::Detached | HousingKind::RuralProperty => {
            let house = b.k(keys::ALARM_LEVELS_HOUSE);
            if housing.basement {
                house + b.k(keys::ALARM_LEVELS_BASEMENT)
            } else {
                house
            }
        }
    }
}

/// Smoke alarms on every level when the household has none. Rule `smoke_alarm`.
pub fn smoke_alarm(housing: &Housing) -> Option<Sizing> {
    if housing.alarms.smoke {
        return None;
    }
    let mut b = Basis::new();
    b.cite("ready_gov_home_fires");
    let n = levels(&mut b, housing);
    let text = format!(
        "No smoke alarms yet: put one on every level ({} for a home like yours; more inside bedrooms), and test them every month.",
        num(n, 0)
    );
    Some(Sizing::new(
        &b,
        "smoke_alarm",
        "smoke_alarm",
        n,
        "alarm",
        Per::Household,
        text,
    ))
}

/// Carbon monoxide alarms on every level when the household has none (Ready.gov, CDC). Rule
/// `co_alarm`.
pub fn co_alarm(housing: &Housing) -> Option<Sizing> {
    if housing.alarms.co {
        return None;
    }
    let mut b = Basis::new();
    let replace = b.k(keys::CO_ALARM_REPLACE_YEARS);
    b.cite("ready_gov_power_outages");
    let n = levels(&mut b, housing);
    let text = format!(
        "No carbon monoxide alarm yet: put one on every level ({} for a home like yours), near where people sleep, with battery backup. Replace them every {} or as the maker says.",
        num(n, 0),
        count(replace, "year", "years")
    );
    Some(Sizing::new(
        &b,
        "co_alarm",
        "co_alarm",
        n,
        "alarm",
        Per::Household,
        text,
    ))
}

/// A fire extinguisher when the household has none (an estimate: one, near the kitchen). Rule
/// `fire_extinguisher`.
pub fn fire_extinguisher(housing: &Housing) -> Option<Sizing> {
    if housing.alarms.extinguisher {
        return None;
    }
    let mut b = Basis::new();
    let n = b.k(keys::EXTINGUISHERS_PER_HOUSEHOLD);
    let text = format!(
        "{}, kept near the kitchen and an exit. Learn how to use it before you need it.",
        count(n, "fire extinguisher", "fire extinguishers")
    );
    Some(Sizing::new(
        &b,
        "fire_extinguisher",
        "fire_extinguisher",
        n,
        "extinguisher",
        Per::Household,
        text,
    ))
}

/// Swap numbers with neighbours and agree who checks on whom (an estimate of how many). Rule
/// `neighbour_contacts`.
pub fn neighbour_contacts() -> Sizing {
    let mut b = Basis::new();
    let n = b.k(keys::NEIGHBOUR_CONTACTS);
    let text = format!(
        "Swap phone numbers with {} and agree who checks on whom after a storm or an outage. Neighbours are usually the first help to arrive.",
        count(n, "neighbour", "neighbours")
    );
    Sizing::new(
        &b,
        "neighbour_contacts",
        "neighbour_contacts",
        n,
        "contact",
        Per::Household,
        text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn philadelphia_has_smoke_alarms_but_no_co_alarm() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        assert!(smoke_alarm(&p.housing).is_none());
        let co = co_alarm(&p.housing).unwrap();
        assert_eq!(co.quantity, 3.0, "rowhouse: 2 levels plus a basement");
        assert!(co.citations.iter().any(|c| c == "ready_gov_power_outages"));
        assert_eq!(fire_extinguisher(&p.housing).unwrap().quantity, 1.0);
    }

    #[test]
    fn apartments_count_one_level() {
        let p = fixtures::get("miami-condo-retiree-1").unwrap();
        assert_eq!(co_alarm(&p.housing).unwrap().quantity, 1.0);
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        assert!(co_alarm(&coos.housing).is_none());
        assert_eq!(neighbour_contacts().quantity, 2.0);
    }
}
