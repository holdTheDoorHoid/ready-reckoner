//! Heat and cold (research §5): fans only below 90 °F indoors, cooling centres, blankets and warm
//! layers first (most homes have them), a sleeping bag or extra heavy blanket only for a long cold
//! target or someone 65 or over, one warm room, and the carbon monoxide rules.

use rr_types::{AgeBand, Heating, Housing, Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{count, days as fmt_days, num};

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
        "thermal_heat",
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
        "thermal_heat",
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
        "thermal_heat",
        1.0,
        "plan",
        Per::Household,
        text,
    )
}

/// People aged 1 and over: babies get warm sleepwear or a sleep sack instead of loose bedding (CDC).
fn over_one(people_list: &[Person]) -> f64 {
    people_list
        .iter()
        .filter(|p| p.age_band != AgeBand::Infant)
        .count() as f64
}

/// A warm blanket for each person aged 1 and over (Ready.gov, Sphere). Most homes already have
/// them, so the plan counts them first. Rule `blankets`.
pub fn blankets(people_list: &[Person]) -> Option<Sizing> {
    let n = over_one(people_list);
    if n == 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let each = b.k(keys::BLANKETS_PER_PERSON);
    let q = n * each;
    let babies = people_list.len() as f64 > n;
    let mut text = format!(
        "{}, one for each person{}: most homes already have them.",
        count(q, "warm blanket", "warm blankets"),
        if babies { " aged 1 and over" } else { "" }
    );
    if babies {
        b.cite("cdc_winter_safety");
        text.push_str(" Dress a baby in warm sleepwear or a sleep sack instead of loose blankets.");
    }
    Some(Sizing::new(
        &b,
        "blankets",
        "thermal_cold",
        q,
        "blanket",
        Per::Person,
        text,
    ))
}

/// A sleeping bag or an extra heavy blanket, beyond the blankets and warm layers most homes have:
/// a need for everyone aged 1 and over when the cold target is longer than 3 days (an estimate),
/// otherwise for each person 65 or over (CDC: most at risk in the cold). A wood stove heats without
/// power, so there it is only an optional upgrade, as it is for a short target with nobody 65 or
/// over. Gives the sizing and whether it is a need; `None` with nobody aged 1 or over. The line
/// records the target's days when they decide it. Rule `sleeping_bag_or_blanket`.
pub fn sleeping_bag_or_blanket(
    days: f64,
    people_list: &[Person],
    housing: &Housing,
) -> Option<(Sizing, bool)> {
    let everyone = over_one(people_list);
    if everyone == 0.0 {
        return None;
    }
    let seniors = people_list
        .iter()
        .filter(|p| p.age_band == AgeBand::Senior)
        .count() as f64;
    let mut b = Basis::new();
    let each = b.k(keys::SLEEPING_BAGS_PER_PERSON);
    let one = |n: f64| {
        count(
            n * each,
            "sleeping bag or extra heavy blanket",
            "sleeping bags or extra heavy blankets",
        )
    };
    if housing.heating == Heating::Wood {
        let text = format!(
            "An upgrade, not a need: {}, one for each person. Your wood stove heats without power, so keep dry wood instead.",
            one(everyone)
        );
        let s = Sizing::new(
            &b,
            "sleeping_bag_or_blanket",
            "thermal_cold",
            everyone * each,
            "item",
            Per::Person,
            text,
        );
        return Some((s, false));
    }
    let min_days = b.k(keys::SLEEPING_BAGS_MIN_DAYS);
    let (n, needed, text) = if days > min_days {
        (
            everyone,
            true,
            format!(
                "{}, one for each person{}, rated for the coldest nights where you live: your cold target of {} is longer than the {} that blankets and warm layers cover.",
                one(everyone),
                if people_list.len() as f64 > everyone {
                    " aged 1 and over"
                } else {
                    ""
                },
                fmt_days(days),
                fmt_days(min_days)
            ),
        )
    } else if seniors > 0.0 {
        b.cite("cdc_winter_safety");
        (
            seniors,
            true,
            format!(
                "{} for {} 65 or over, who {} most at risk in the cold. For a cold target of {}, blankets and warm layers do for everyone else.",
                one(seniors),
                if seniors == 1.0 {
                    "the person".to_owned()
                } else {
                    format!("the {} people", num(seniors, 0))
                },
                if seniors == 1.0 { "is" } else { "are" },
                fmt_days(days)
            ),
        )
    } else {
        (
            everyone,
            false,
            format!(
                "An upgrade, not a need: {}, one for each person. Blankets and warm layers cover a cold target of {}.",
                one(everyone),
                fmt_days(days)
            ),
        )
    };
    let s = Sizing::new(
        &b,
        "sleeping_bag_or_blanket",
        "thermal_cold",
        n * each,
        "item",
        Per::Person,
        text,
    )
    .days(days);
    Some((s, needed))
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
        "thermal_cold",
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
        "thermal_cold",
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
        let blankets = blankets(&p.people).unwrap();
        assert_eq!(blankets.quantity, 4.0);
        assert_eq!(blankets.unit, "blanket");
        assert!(blankets.plain.contains("most homes already have them"));
        assert_eq!(warm_layers(&p.people).quantity, 4.0);
        let plan = warm_room_plan(&p.housing, &p.people);
        assert!(plan.plain.contains("furnaces usually need electricity"));
        assert!(plan.plain.contains("95 °F"));
    }

    #[test]
    fn sleeping_bags_only_for_long_cold_or_people_over_65() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        // 1.7 days of cold with a senior in a gas-heated rowhouse: one, for the senior.
        let (s, needed) = sleeping_bag_or_blanket(1.7, &p.people, &p.housing).unwrap();
        assert!(needed);
        assert_eq!(s.quantity, 1.0);
        assert!(s.plain.contains("the person 65 or over"), "{}", s.plain);
        assert!(s.citations.iter().any(|c| c == "cdc_winter_safety"));
        assert_eq!(s.days, Some(1.7));
        // More than 3 days: everyone.
        let (s, needed) = sleeping_bag_or_blanket(5.0, &p.people, &p.housing).unwrap();
        assert!(needed && s.prior);
        assert_eq!(s.quantity, 4.0);
        assert!(s.plain.contains("longer than the 3 days"), "{}", s.plain);
        // A short target with nobody 65 or over: an upgrade only.
        let young: Vec<Person> = p
            .people
            .iter()
            .filter(|x| x.age_band != AgeBand::Senior)
            .cloned()
            .collect();
        let (s, needed) = sleeping_bag_or_blanket(3.0, &young, &p.housing).unwrap();
        assert!(!needed);
        assert!(s.plain.starts_with("An upgrade, not a need"));
        // A wood stove works without power: an upgrade even for a long target.
        let coos = fixtures::get("coos-bay-well-owner-2").unwrap();
        let (s, needed) = sleeping_bag_or_blanket(20.0, &coos.people, &coos.housing).unwrap();
        assert!(!needed && s.plain.contains("wood stove"));
        assert_eq!(s.days, None, "the target does not decide it");
        assert!(!s.prior);
    }

    #[test]
    fn babies_get_a_sleep_sack_not_a_blanket() {
        let p = fixtures::get("sugar-land-ev-household-3").unwrap();
        let babies = p
            .people
            .iter()
            .filter(|x| x.age_band == AgeBand::Infant)
            .count();
        assert_eq!(babies, 1);
        let b = blankets(&p.people).unwrap();
        assert_eq!(b.quantity, (p.people.len() - 1) as f64);
        assert!(b.plain.contains("sleep sack") && b.plain.contains("aged 1 and over"));
        let (s, _) = sleeping_bag_or_blanket(7.0, &p.people, &p.housing).unwrap();
        assert_eq!(s.quantity, (p.people.len() - 1) as f64);
        assert!(blankets(&p.people[..0]).is_none());
    }

    #[test]
    fn wood_heat_is_named() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        let plan = warm_room_plan(&p.housing, &p.people);
        assert!(plan.plain.contains("wood stove"));
        assert!(!plan.prior);
    }
}
