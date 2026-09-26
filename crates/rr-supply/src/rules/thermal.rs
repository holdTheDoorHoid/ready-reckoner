//! Heat and cold (research §5): fans only below 90 °F indoors, cooling centres, bedding and layers,
//! one warm room, and the carbon monoxide rules.

use rr_types::{AgeBand, Heating, Housing, Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{count, num};

/// Battery fans: one for the home plus one per person most at risk from heat (an estimate). Rule
/// `battery_fan`.
pub fn battery_fan(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let base = b.k(keys::BATTERY_FANS_BASE);
    let per_vulnerable = b.k(keys::BATTERY_FANS_PER_VULNERABLE);
    let max_f = b.k(keys::FAN_MAX_INDOOR_F);
    let vulnerable = people_list
        .iter()
        .filter(|p| {
            matches!(p.age_band, AgeBand::Senior | AgeBand::Infant) || p.pregnant_or_nursing
        })
        .count() as f64;
    let q = base + per_vulnerable * vulnerable;
    let extra = if vulnerable > 0.0 {
        format!(
            " plus {} for the {} most at risk from heat (65 and over, babies, pregnancy)",
            num(per_vulnerable * vulnerable, 0),
            count(vulnerable, "person", "people")
        )
    } else {
        String::new()
    };
    let text = format!(
        "{}: {} for the home{extra}. Fans help only while it is below {} °F indoors; above that, go to a cooling centre.",
        count(q, "battery fan", "battery fans"),
        num(base, 0),
        num(max_f, 0)
    );
    Sizing::new(
        &b,
        "battery_fan",
        "battery_fan",
        q,
        "fan",
        Per::Household,
        text,
    )
}

/// A cooling towel per person (an estimate). Rule `cooling_towel`.
pub fn cooling_towel(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let each = b.k(keys::COOLING_TOWELS_PER_PERSON);
    let q = (people_list.len() as f64 * each).max(1.0);
    let text = format!(
        "{}, one for each person: wet them and wear them on the neck in a heat wave.",
        count(q, "cooling towel", "cooling towels")
    );
    Sizing::new(
        &b,
        "cooling_towel",
        "cooling_towel",
        q,
        "towel",
        Per::Person,
        text,
    )
}

/// A heat plan: the nearest cooling centre, the coolest room, who checks on whom. Rule
/// `cooling_plan`.
pub fn cooling_plan() -> Sizing {
    let mut b = Basis::new();
    b.cite("cdc_heat_health");
    b.cite("ready_gov_heat");
    let text = "A heat plan: find your nearest cooling centre (dial 2-1-1), pick the coolest room, cover sunny windows, and agree who checks on whom. Never leave people or pets in a closed car.".to_owned();
    Sizing::new(
        &b,
        "cooling_plan",
        "cooling_plan",
        1.0,
        "plan",
        Per::Household,
        text,
    )
}

/// A sleeping bag or warm blanket for each person (Ready.gov). Rule `sleeping_bag_or_blanket`.
pub fn sleeping_bag_or_blanket(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let each = b.k(keys::SLEEPING_BAGS_PER_PERSON);
    let q = (people_list.len() as f64 * each).max(1.0);
    let text = format!(
        "{} (a sleeping bag or warm blanket for each person), rated for the coldest nights where you live.",
        count(q, "sleeping bag or blanket", "sleeping bags or blankets")
    );
    Sizing::new(
        &b,
        "sleeping_bag_or_blanket",
        "sleeping_bag",
        q,
        "item",
        Per::Person,
        text,
    )
}

/// A set of warm layers (coat, hat, gloves) per person. Rule `warm_layers`.
pub fn warm_layers(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let each = b.k(keys::WARM_LAYER_SETS_PER_PERSON);
    let q = (people_list.len() as f64 * each).max(1.0);
    let text = format!(
        "{} of warm layers (coat, hat, gloves, dry socks), one for each person.",
        count(q, "set", "sets")
    );
    Sizing::new(
        &b,
        "warm_layers",
        "warm_layers",
        q,
        "set",
        Per::Person,
        text,
    )
}

/// A cold plan: one warm room, and never unvented combustion indoors; the wording follows the
/// home's heating. Rule `warm_room_plan`.
pub fn warm_room_plan(housing: &Housing, people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let hypothermia = b.k(keys::HYPOTHERMIA_F);
    b.cite("cdc_co_basics");
    let mut text = "A cold plan: pick one room to keep warm, close off the others, put towels under doors and cover windows at night. Never heat with a gas oven, grill, camp stove or generator indoors (carbon monoxide).".to_owned();
    match housing.heating {
        Heating::Wood => text.push_str(
            " Your wood stove keeps you warm as long as you have dry wood; keep the chimney clear.",
        ),
        Heating::Gas | Heating::Oil | Heating::Propane => {
            b.cite(crate::constants::PRIOR_SOURCE);
            text.push_str(" Gas, oil and propane furnaces usually need electricity for their fans and controls, so plan for cold without power.");
        }
        Heating::ElectricResistance | Heating::HeatPump => {
            text.push_str(" Electric heat stops when the power does.");
        }
        Heating::District | Heating::None => {}
    }
    if people_list.iter().any(|p| p.age_band == AgeBand::Infant) {
        text.push_str(" Babies under 1 should never sleep in a cold room.");
    }
    if people_list.iter().any(|p| p.age_band == AgeBand::Senior) {
        text.push_str(" People 65 and over should check the indoor temperature often.");
    }
    text.push_str(&format!(
        " A body temperature below {} °F needs medical help at once.",
        num(hypothermia, 0)
    ));
    Sizing::new(
        &b,
        "warm_room_plan",
        "warm_room_plan",
        1.0,
        "plan",
        Per::Household,
        text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn philadelphia_heat_and_cold() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        let fans = battery_fan(&p.people);
        assert_eq!(fans.quantity, 2.0); // 1 + the senior
        assert!(fans.plain.contains("90 °F"));
        assert!(fans.prior);
        assert_eq!(cooling_towel(&p.people).quantity, 4.0);
        assert_eq!(sleeping_bag_or_blanket(&p.people).quantity, 4.0);
        assert_eq!(warm_layers(&p.people).quantity, 4.0);
        let plan = warm_room_plan(&p.housing, &p.people);
        assert!(plan.plain.contains("furnaces usually need electricity"));
        assert!(plan.plain.contains("95 °F"));
    }

    #[test]
    fn wood_heat_is_named() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        let plan = warm_room_plan(&p.housing, &p.people);
        assert!(plan.plain.contains("wood stove"));
        assert!(!plan.prior);
    }
}
