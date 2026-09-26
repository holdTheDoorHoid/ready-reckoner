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

fn is_house(housing: &Housing) -> bool {
    matches!(
        housing.kind,
        HousingKind::Rowhouse | HousingKind::Detached | HousingKind::RuralProperty
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
/// gets it, alarms or not. A house at street level is not assumed to sleep upstairs, so its plan
/// asks for a second way out of any upstairs bedroom instead of sizing a ladder. Rule
/// `fire_escape_plan`.
pub fn fire_escape_plan(housing: &Housing) -> Sizing {
    let mut b = Basis::new();
    b.cite("ready_gov_home_fires");
    let mut text = "A fire escape plan: two ways out of every room, a meeting spot outside, and a practice run with everyone.".to_owned();
    if is_apartment(housing) {
        text.push_str(" In a building, know both stairways and never use the elevator in a fire.");
    } else if is_house(housing) && housing.floor <= 1 {
        text.push_str(" If anyone sleeps upstairs, check that each bedroom has a second way out, such as a window onto a porch roof or an escape ladder.");
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

/// Fire extinguishers when the household has none: one on each floor people live on (an apartment
/// or mobile home 1, a house 2, not the basement; an estimate), the ground-floor one near the
/// kitchen. The form does not say where the kitchen is, so it is assumed to be on the floor with
/// the way out; the line says to add one when it is not. Rule `extinguisher_count`.
pub fn extinguisher_count(housing: &Housing) -> Option<Sizing> {
    if housing.alarms.extinguisher {
        return None;
    }
    let mut b = Basis::new();
    b.cite("usfa_extinguishers");
    let floors = levels(&mut b, housing, false);
    let each = b.k(keys::EXTINGUISHERS_PER_FLOOR);
    let n = floors * each;
    let text = if floors <= 1.0 {
        format!(
            "No fire extinguisher yet: {} for a home on one floor, kept near the kitchen. Learn how to use it before you need it.",
            count(
                n,
                "multipurpose (A-B-C) extinguisher",
                "multipurpose (A-B-C) extinguishers"
            )
        )
    } else {
        format!(
            "No fire extinguisher yet: {}, one on each of the {} floors a home like yours has, the ground-floor one near the kitchen. If your kitchen is on another floor, keep one there too. Learn how to use one before you need it.",
            count(
                n,
                "multipurpose (A-B-C) extinguisher",
                "multipurpose (A-B-C) extinguishers"
            ),
            num(floors, 0)
        )
    };
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

/// An escape ladder only where people sleep above the ground with no second way out assumed: a home
/// (any kind but a mobile home) whose floor is 2 or 3 (an estimate; a ladder does not reach higher,
/// where the stairs are the way out). The form's floor is where the household lives, so a house at
/// street level gets the upstairs-bedroom advice in its escape plan instead. Rule
/// `escape_ladder_count`.
pub fn escape_ladder_count(housing: &Housing) -> Option<Sizing> {
    if housing.kind == HousingKind::MobileHome {
        return None;
    }
    let mut b = Basis::new();
    let lo = b.k(keys::ESCAPE_LADDER_LOWEST_FLOOR);
    let hi = b.k(keys::ESCAPE_LADDER_HIGHEST_FLOOR);
    let floor = f64::from(housing.floor);
    if !(lo..=hi).contains(&floor) {
        return None;
    }
    b.cite("ready_gov_home_fires");
    let text = format!(
        "You live on floor {}, above the ground: 1 escape ladder for a bedroom window, in case fire or smoke blocks the way out, unless every bedroom already has a second way out such as a fire escape. Keep it by the window and practise with it.",
        num(floor, 0)
    );
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
        // One per floor of a two-floor rowhouse, not the basement; the kitchen is assumed to be
        // on the ground floor.
        let ext = extinguisher_count(&p.housing).unwrap();
        assert_eq!(ext.quantity, 2.0);
        assert!(ext.plain.contains("near the kitchen") && ext.plain.contains("A-B-C"));
        assert!(ext.prior && ext.citations.iter().any(|c| c == "usfa_extinguishers"));
        // Street level: no ladder; the escape plan asks about upstairs bedrooms instead.
        assert!(escape_ladder_count(&p.housing).is_none());
        let plan = fire_escape_plan(&p.housing);
        assert_eq!(plan.quantity, 1.0);
        assert!(plan.plain.contains("two ways out") && plan.plain.contains("every month"));
        assert!(plan.plain.contains("sleeps upstairs"));
        assert!(!plan.prior);
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
        let phoenix = fixtures::get("phoenix-apartment-cpap-1").unwrap();
        let ladder = escape_ladder_count(&phoenix.housing).unwrap();
        assert!(ladder.plain.starts_with("You live on floor 2"));
        assert_eq!(extinguisher_count(&phoenix.housing).unwrap().quantity, 1.0);
        // A house whose household lives upstairs (floor 2) gets one; a mobile home never does.
        let mut upstairs = fixtures::get("philadelphia-renters-4").unwrap().housing;
        upstairs.floor = 2;
        assert_eq!(escape_ladder_count(&upstairs).unwrap().quantity, 1.0);
        upstairs.kind = HousingKind::MobileHome;
        assert!(escape_ladder_count(&upstairs).is_none());
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        assert!(co_alarm_count(&coos.housing).is_none());
        assert!(extinguisher_count(&coos.housing).is_none());
        assert_eq!(neighbour_contacts().quantity, 2.0);
    }
}
