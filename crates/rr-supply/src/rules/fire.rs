//! Fire and security readiness: an escape plan, smoke and carbon monoxide alarms, extinguishers,
//! an escape ladder, and neighbours. Firearms are never sized or listed here (docs/PRINCIPLES.md
//! §9).

use rr_types::{Housing, HousingKind, Per};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{ceil_count, count, num};

fn is_apartment(housing: &Housing) -> bool {
    matches!(
        housing.kind,
        HousingKind::ApartmentHighRise | HousingKind::ApartmentLowRise
    )
}

/// Levels of the home (the form does not ask; an estimate by kind of building), with or without
/// the basement.
fn levels(b: &mut Basis, housing: &Housing, with_basement: bool) -> f64 {
    match housing.kind {
        HousingKind::ApartmentHighRise
        | HousingKind::ApartmentLowRise
        | HousingKind::MobileHome => b.k(keys::ALARM_LEVELS_APARTMENT),
        HousingKind::Rowhouse | HousingKind::Detached | HousingKind::RuralProperty => {
            let house = b.k(keys::ALARM_LEVELS_HOUSE);
            if with_basement && housing.basement {
                house + b.k(keys::ALARM_LEVELS_BASEMENT)
            } else {
                house
            }
        }
    }
}

/// A fire escape plan: two ways out of every room and a meeting spot outside. Every household
/// gets it, alarms or not. Rule `fire_escape_plan`.
pub fn fire_escape_plan(housing: &Housing) -> Sizing {
    let mut b = Basis::new();
    b.cite("ready_gov_home_fires");
    let mut text = "A fire escape plan: two ways out of every room, a meeting spot outside, and a practice run with everyone.".to_owned();
    if is_apartment(housing) {
        text.push_str(" In a building, know both stairways and never use the elevator in a fire.");
    }
    if housing.alarms.smoke {
        text.push_str(" Test your smoke alarms every month.");
    }
    Sizing::new(
        &b,
        "fire_escape_plan",
        "fire_escape_plan",
        1.0,
        "plan",
        Per::Household,
        text,
    )
}

/// Smoke alarms on every level and in every bedroom when the household has none (bedrooms counted
/// as one per two people: an estimate). Rule `smoke_alarm_count`.
pub fn smoke_alarm_count(housing: &Housing, people: usize) -> Option<Sizing> {
    if housing.alarms.smoke {
        return None;
    }
    let mut b = Basis::new();
    b.cite("usfa_smoke_alarms");
    b.cite("ready_gov_home_fires");
    let lv = levels(&mut b, housing, true);
    let per_room = b.k(keys::PEOPLE_PER_BEDROOM);
    let bedrooms = ceil_count((people.max(1) as f64) / per_room);
    let q = lv + bedrooms;
    let text = format!(
        "No smoke alarms yet: put one on every level and in every bedroom, about {} and {} for a home like yours = {} alarms. Test them every month.",
        count(lv, "level", "levels"),
        count(bedrooms, "bedroom", "bedrooms"),
        num(q, 0)
    );
    Some(Sizing::new(
        &b,
        "smoke_alarm_count",
        "smoke_alarm",
        q,
        "alarm",
        Per::Household,
        text,
    ))
}

/// Carbon monoxide alarms on every level where people sleep, when the household has none (Ready.gov,
/// CDC). Rule `co_alarm_count`.
pub fn co_alarm_count(housing: &Housing) -> Option<Sizing> {
    if housing.alarms.co {
        return None;
    }
    let mut b = Basis::new();
    let replace = b.k(keys::CO_ALARM_REPLACE_YEARS);
    b.cite("ready_gov_power_outages");
    let n = levels(&mut b, housing, false);
    let text = format!(
        "No carbon monoxide alarm yet: put one on every level where people sleep ({} for a home like yours), with battery backup. Replace them every {} or as the maker says.",
        num(n, 0),
        count(replace, "year", "years")
    );
    Some(Sizing::new(
        &b,
        "co_alarm_count",
        "co_alarm",
        n,
        "alarm",
        Per::Household,
        text,
    ))
}

/// Fire extinguishers, one per level, when the household has none (an estimate for the level
/// count). Rule `extinguisher_count`.
pub fn extinguisher_count(housing: &Housing) -> Option<Sizing> {
    if housing.alarms.extinguisher {
        return None;
    }
    let mut b = Basis::new();
    b.cite("usfa_extinguishers");
    let n = levels(&mut b, housing, true);
    let text = format!(
        "No fire extinguisher yet: {}, one on each level including the kitchen's. Learn how to use one before you need it.",
        count(n, "extinguisher", "extinguishers")
    );
    Some(Sizing::new(
        &b,
        "extinguisher_count",
        "fire_extinguisher",
        n,
        "extinguisher",
        Per::Household,
        text,
    ))
}

/// An escape ladder for an upstairs bedroom: houses (assumed to sleep upstairs) and apartments on the
/// second or third floor (an estimate). Rule `escape_ladder_count`.
pub fn escape_ladder_count(housing: &Housing) -> Option<Sizing> {
    let mut b = Basis::new();
    let needed = match housing.kind {
        HousingKind::Detached | HousingKind::Rowhouse | HousingKind::RuralProperty => {
            b.k(keys::ALARM_LEVELS_HOUSE) > 1.0
        }
        HousingKind::ApartmentHighRise | HousingKind::ApartmentLowRise => {
            let lo = b.k(keys::ESCAPE_LADDER_LOWEST_FLOOR);
            let hi = b.k(keys::ESCAPE_LADDER_HIGHEST_FLOOR);
            (lo..=hi).contains(&f64::from(housing.floor))
        }
        HousingKind::MobileHome => false,
    };
    if !needed {
        return None;
    }
    b.cite("ready_gov_home_fires");
    let text = "1 escape ladder for an upstairs bedroom window, in case the stairs are blocked by fire. Keep it by the window and practise with it.".to_owned();
    Some(Sizing::new(
        &b,
        "escape_ladder_count",
        "escape_ladder",
        1.0,
        "ladder",
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
        "Swap phone numbers with {} and agree who checks on whom after a storm or an outage, when phones and roads may be down.",
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
        assert!(smoke_alarm_count(&p.housing, 4).is_none());
        let co = co_alarm_count(&p.housing).unwrap();
        assert_eq!(co.quantity, 2.0, "a rowhouse sleeps on 2 levels");
        assert!(co.citations.iter().any(|c| c == "ready_gov_power_outages"));
        // 2 levels + a basement
        assert_eq!(extinguisher_count(&p.housing).unwrap().quantity, 3.0);
        assert_eq!(escape_ladder_count(&p.housing).unwrap().quantity, 1.0);
        let plan = fire_escape_plan(&p.housing);
        assert_eq!(plan.quantity, 1.0);
        assert!(plan.plain.contains("two ways out") && plan.plain.contains("every month"));
    }

    #[test]
    fn counts_without_alarms() {
        let mut p = fixtures::get("philadelphia-renters-4").unwrap();
        p.housing.alarms.smoke = false;
        // 3 levels (with the basement) + 2 bedrooms for 4 people
        let s = smoke_alarm_count(&p.housing, 4).unwrap();
        assert_eq!(s.quantity, 5.0);
        assert!(s.prior && s.citations.iter().any(|c| c == "usfa_smoke_alarms"));
    }

    #[test]
    fn apartments_count_one_level_and_ladders_follow_the_floor() {
        let p = fixtures::get("miami-condo-retiree-1").unwrap();
        assert_eq!(co_alarm_count(&p.housing).unwrap().quantity, 1.0);
        assert!(
            escape_ladder_count(&p.housing).is_none(),
            "14th floor: use the stairs"
        );
        let chicago = fixtures::get("chicago-student-zero-budget-1").unwrap();
        assert_eq!(
            escape_ladder_count(&chicago.housing).unwrap().quantity,
            1.0,
            "3rd floor"
        );
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        assert!(co_alarm_count(&coos.housing).is_none());
        assert!(extinguisher_count(&coos.housing).is_none());
        assert_eq!(neighbour_contacts().quantity, 2.0);
    }
}
