//! Getting home when stranded (research §9.3): a bag per commuter sized to the walk, and a kit per
//! vehicle.

use rr_types::{Commute, CommuteMode, Per};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{KM_PER_MILE, count, num};

/// Hours to walk home and litres of water for the walk.
fn walk(b: &mut Basis, commute: &Commute, hot: bool) -> (f64, f64, f64) {
    let mph = b.k(keys::WALK_MPH);
    let miles = f64::from(commute.distance_km).max(0.0) / KM_PER_MILE;
    let hours = miles / mph;
    let l_per_hour = if hot {
        b.k(keys::WALK_WATER_L_PER_HOUR_HOT)
    } else {
        b.k(keys::WALK_WATER_L_PER_HOUR)
    };
    (miles, hours, hours * l_per_hour)
}

fn mode_words(mode: CommuteMode) -> &'static str {
    match mode {
        CommuteMode::Car => "by car",
        CommuteMode::Transit => "by bus or train",
        CommuteMode::Walk => "on foot",
        CommuteMode::Bike => "by bike",
    }
}

/// One get-home bag for a commuter (person `index`, 1-based), kept in the car or at work. Rule
/// `get_home_bag`.
pub fn get_home_bag(index: usize, commute: &Commute, hot: bool) -> Sizing {
    let mut b = Basis::new();
    let (miles, hours, litres) = walk(&mut b, commute, hot);
    let mph = b.k(keys::WALK_MPH);
    let kcal_12h = b.k(keys::WALK_KCAL_PER_12H);
    let shelter = b.k(keys::WORK_SHELTER_HOURS);
    b.cite("ready_gov_kit");
    let kcal = super::round_quantity("kcal", hours / 12.0 * kcal_12h).max(100.0);
    let place = if commute.mode == CommuteMode::Car {
        "in the car"
    } else {
        "at work or in your daily bag"
    };
    let text = format!(
        "Person {index} travels {} miles ({} km) {} to work or school; walking home at about {} mph takes about {}. Keep a get-home bag {place}: comfortable walking shoes, a light, a paper map, cash in small bills, a rain or warm layer, about {} L of water and {} kcal of snacks. Be ready to stay at work for {} if you can't leave.",
        num(miles, 1),
        num(f64::from(commute.distance_km), 1),
        mode_words(commute.mode),
        num(mph, 0),
        count(crate::format::round_dp(hours, 1), "hour", "hours"),
        num(super::round_quantity("litre", litres), 1),
        num(kcal, 0),
        count(shelter, "hour", "hours")
    );
    Sizing::new(
        &b,
        "get_home_bag",
        "get_home_bag",
        1.0,
        "bag",
        Per::Commuter,
        text,
    )
    .math(vec![format!(
        "{} mi ÷ {} mph = {} h",
        num(miles, 2),
        num(mph, 1),
        num(hours, 2)
    )])
}

/// Water for the walk home: hours on foot × half a litre an hour (0.71 L in heat, NIOSH). Past about
/// 2.5 L, a filter beats carrying more. Rule `get_home_water`.
pub fn get_home_water(commute: &Commute, hot: bool) -> Sizing {
    let mut b = Basis::new();
    let (_, hours, litres) = walk(&mut b, commute, hot);
    let carry_max = b.k(keys::WALK_WATER_CARRY_MAX_L);
    let rate = litres / hours.max(f64::MIN_POSITIVE);
    let mut text = format!(
        "Water for the walk home: {} × about {} L an hour{} = {} L.",
        count(crate::format::round_dp(hours, 1), "hour", "hours"),
        num(rate, 2),
        if hot { " in heat" } else { "" },
        num(super::round_quantity("litre", litres), 1)
    );
    if litres > carry_max {
        text.push_str(&format!(
            " Past about {} L, carry a filter or purification tablets and an empty bottle instead of all the water.",
            num(carry_max, 1)
        ));
    }
    Sizing::new(
        &b,
        "get_home_water",
        "walking_water",
        litres,
        "litre",
        Per::Commuter,
        text,
    )
}

/// An emergency kit for each vehicle (Ready.gov winter guidance). Rule `car_kit`.
pub fn car_kit(vehicles: usize) -> Option<Sizing> {
    if vehicles == 0 {
        return None;
    }
    let mut b = Basis::new();
    let each = b.k(keys::CAR_KITS_PER_VEHICLE);
    let q = vehicles as f64 * each;
    let text = format!(
        "{}: jumper cables, a flashlight, warm clothes, a blanket, bottled water and snacks; sand or cat litter in snow country.",
        count(q, "car emergency kit", "car emergency kits")
    );
    Some(Sizing::new(
        &b,
        "car_kit",
        "car_kit",
        q,
        "kit",
        Per::Household,
        text,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn philadelphia_commuters() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        let car = p.people[0].commute.as_ref().unwrap();
        let bag = get_home_bag(1, car, false);
        // 19 km = 11.8 mi ÷ 3 mph = 3.9 h
        assert!(
            bag.plain.contains("11.8 miles (19 km) by car"),
            "{}",
            bag.plain
        );
        assert!(bag.plain.contains("3.9 hours") && bag.plain.contains("in the car"));
        let water = get_home_water(car, false);
        assert_eq!(water.quantity, 2.0); // 3.94 h × 0.5 L
        assert!(water.prior);
        let transit = p.people[1].commute.as_ref().unwrap();
        assert!(get_home_bag(2, transit, false).plain.contains("at work"));
        assert_eq!(get_home_water(transit, false).quantity, 0.6);
    }

    #[test]
    fn long_walks_in_heat_suggest_a_filter() {
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        let c = hays.people[1].commute.as_ref().unwrap(); // 35 km
        let w = get_home_water(c, true);
        // 21.7 mi ÷ 3 = 7.25 h × 0.71 = 5.1 L
        assert_eq!(w.quantity, 5.1);
        assert!(w.plain.contains("filter"));
        assert!(w.citations.iter().any(|c| c == "niosh_heat_stress"));
        assert_eq!(car_kit(2).unwrap().quantity, 2.0);
        assert!(car_kit(0).is_none());
    }
}
