//! Getting home when stranded (research §9.3): a bag per commuter sized to the walk, and a kit per
//! vehicle. The water and snacks for the walk come from home: `rr-supply` emits them as
//! alternative lines of that person's bag, never as something more to buy.

use rr_types::{Commute, CommuteMode, Per};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{KM_PER_MILE, count, num};

/// Miles and hours to walk home.
fn walk(b: &mut Basis, commute: &Commute) -> (f64, f64) {
    let mph = b.k(keys::WALK_MPH);
    let miles = f64::from(commute.distance_km).max(0.0) / KM_PER_MILE;
    (miles, miles / mph)
}

/// Litres of water for `hours` of walking (half a litre an hour; NIOSH's 0.71 in heat).
fn walk_water(b: &mut Basis, hours: f64, hot: bool) -> f64 {
    let l_per_hour = if hot {
        b.k(keys::WALK_WATER_L_PER_HOUR_HOT)
    } else {
        b.k(keys::WALK_WATER_L_PER_HOUR)
    };
    hours * l_per_hour
}

fn mode_words(mode: CommuteMode) -> &'static str {
    match mode {
        CommuteMode::Car => "by car",
        CommuteMode::Transit => "by bus or train",
        CommuteMode::Walk => "on foot",
        CommuteMode::Bike => "by bike",
    }
}

/// One get-home bag for a commuter (person `index`, 1-based), kept in the car or at work. Its water
/// and snacks are that person's staged lines ([`get_home_water`], [`get_home_food`]). Rule
/// `get_home_bag`.
pub fn get_home_bag(index: usize, commute: &Commute) -> Sizing {
    let mut b = Basis::new();
    let (miles, hours) = walk(&mut b, commute);
    let mph = b.k(keys::WALK_MPH);
    let shelter = b.k(keys::WORK_SHELTER_HOURS);
    b.cite("ready_gov_kit");
    let place = if commute.mode == CommuteMode::Car {
        "in the car"
    } else {
        "at work or in your daily bag"
    };
    let text = format!(
        "Person {index} travels {} miles ({} km) {} to work or school; walking home at about {} mph takes about {}. Keep a get-home bag {place}: comfortable walking shoes, a light, a paper map, cash in small bills, a rain or warm layer, and water and snacks from home. Be ready to stay at work for {} if you can't leave.",
        num(miles, 1),
        num(f64::from(commute.distance_km), 1),
        mode_words(commute.mode),
        num(mph, 0),
        count(crate::format::round_dp(hours, 1), "hour", "hours"),
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

/// Water for person `index`'s walk home: hours on foot × half a litre an hour (0.71 L in heat,
/// NIOSH), filled from home, so it is an alternative line of that person's bag, never an addition.
/// Past about 2.5 L, a filter beats carrying more. Rule `get_home_water`.
pub fn get_home_water(index: usize, commute: &Commute, hot: bool) -> Sizing {
    let mut b = Basis::new();
    let (_, hours) = walk(&mut b, commute);
    let litres = walk_water(&mut b, hours, hot);
    let carry_max = b.k(keys::WALK_WATER_CARRY_MAX_L);
    let rate = litres / hours.max(f64::MIN_POSITIVE);
    let mut text = format!(
        "Water for person {index}'s walk home: {} × about {} L an hour{} = {} L, in a bottle filled from home rather than bought extra.",
        count(crate::format::round_dp(hours, 1), "hour", "hours"),
        num(rate, 2),
        if hot { " in heat" } else { "" },
        num(super::round_quantity("liter", litres), 1)
    );
    if litres > carry_max && hot {
        text.push_str(&format!(
            " That is more than the {} L worth carrying, but in a county as hot as yours there may be no water to filter on the way: carry more water, and wait out the worst heat at work if you can.",
            num(carry_max, 1)
        ));
    } else if litres > carry_max {
        text.push_str(&format!(
            " Past about {} L, carry a filter or purification tablets and an empty bottle instead of all the water (see the filter line).",
            num(carry_max, 1)
        ));
    }
    Sizing::new(
        &b,
        "get_home_water",
        "walking_water",
        litres,
        "liter",
        Per::Commuter,
        text,
    )
}

/// A small water filter for person `index`'s walk home, only when the walk needs more water than is
/// worth carrying (`walk_water_carry_max_l`, 2.5 L, an estimate) and the county is not hot: in a hot
/// county there may be no water to filter, so the staged water line says to carry more instead
/// (round-2 review P-15: filters were bought for walks needing 0.6 L). A filter with an absolute pore
/// size of 0.3 micron or smaller removes bacteria and parasites (CDC). Rule `get_home_filter`.
pub fn get_home_filter(index: usize, commute: &Commute, hot: bool) -> Option<Sizing> {
    let mut b = Basis::new();
    let (_, hours) = walk(&mut b, commute);
    let litres = walk_water(&mut b, hours, hot);
    let carry_max = b.k(keys::WALK_WATER_CARRY_MAX_L);
    if hot || litres <= carry_max {
        return None;
    }
    b.cite("cdc_water_disinfection");
    let text = format!(
        "A small water filter or purification tablets for person {index}'s walk home, which needs about {} L of water, more than the {} L worth carrying: pack an empty bottle and refill it on the way. Choose a filter rated 0.3 micron or smaller; it does not remove viruses or chemicals.",
        num(super::round_quantity("liter", litres), 1),
        num(carry_max, 1)
    );
    Some(Sizing::new(
        &b,
        "get_home_filter",
        "walking_filter",
        1.0,
        "filter",
        Per::Commuter,
        text,
    ))
}

/// Snacks for person `index`'s walk home: about 1,250 kcal per 12 hours on foot (an estimate), at
/// least 100 kcal (one rounding step), from the household's food, so it is an alternative line of
/// that person's bag, never an addition. Rule `get_home_food`.
pub fn get_home_food(index: usize, commute: &Commute) -> Sizing {
    let mut b = Basis::new();
    let (_, hours) = walk(&mut b, commute);
    let kcal_12h = b.k(keys::WALK_KCAL_PER_12H);
    let kcal = super::round_quantity("kcal", hours / 12.0 * kcal_12h).max(100.0);
    let text = format!(
        "Snacks for person {index}'s walk home: about {} kcal for {} on foot (about {} kcal every 12 hours), from your food at home rather than bought extra.",
        num(kcal, 0),
        count(crate::format::round_dp(hours, 1), "hour", "hours"),
        num(kcal_12h, 0)
    );
    Sizing::new(
        &b,
        "get_home_food",
        "walking_food",
        kcal,
        "kcal",
        Per::Commuter,
        text,
    )
    .math(vec![format!(
        "max({} h ÷ 12 × {} kcal, 100) = {} kcal",
        num(hours, 2),
        num(kcal_12h, 0),
        num(kcal, 0)
    )])
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
        "{}: jumper cables or a charged jump pack you have tried, a flashlight, warm clothes, a blanket, bottled water and snacks, and your roadside-assistance number on paper; sand or cat litter in snow country.",
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
        let bag = get_home_bag(1, car);
        // 19 km = 11.8 mi ÷ 3 mph = 3.9 h
        assert!(
            bag.plain.contains("11.8 miles (19 km) by car"),
            "{}",
            bag.plain
        );
        assert!(bag.plain.contains("3.9 hours") && bag.plain.contains("in the car"));
        assert!(bag.plain.contains("water and snacks from home"));
        // The bag's own line no longer carries the walk's water or food amounts, nor their sources.
        assert!(!bag.plain.contains(" L of water") && !bag.plain.contains("kcal"));
        assert!(!bag.citations.iter().any(|c| c == "usda_dga_2020_2025"));
        let water = get_home_water(1, car, false);
        assert_eq!(water.quantity, 2.0); // 3.94 h × 0.5 L
        assert!(water.prior);
        assert!(water.plain.contains("person 1's walk") && water.plain.contains("from home"));
        let food = get_home_food(1, car);
        assert_eq!(food.quantity, 400.0); // 3.94 h ÷ 12 × 1,250 = 410
        assert!(food.citations.iter().any(|c| c == "usda_dga_2020_2025"));
        let transit = p.people[1].commute.as_ref().unwrap();
        assert!(get_home_bag(2, transit).plain.contains("at work"));
        assert_eq!(get_home_water(2, transit, false).quantity, 0.6);
        assert_eq!(
            get_home_food(2, transit).quantity,
            100.0,
            "1.2 h: 125 kcal rounds to 100"
        );
    }

    #[test]
    fn long_walks_carry_a_filter_except_in_heat() {
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        let c = hays.people[1].commute.as_ref().unwrap(); // 35 km
        let w = get_home_water(2, c, true);
        // 21.7 mi ÷ 3 = 7.25 h × 0.71 = 5.1 L
        assert_eq!(w.quantity, 5.1);
        // Hot: no water to filter on the way, so carry more (round-2 review P-15).
        assert!(w.plain.contains("carry more water"), "{}", w.plain);
        assert!(get_home_filter(2, c, true).is_none());
        assert!(w.citations.iter().any(|c| c == "cdc_niosh_heat_hydration"));
        // Temperate: 7.25 h × 0.5 L = 3.6 L, past 2.5 L: a filter for that commuter.
        let f = get_home_filter(2, c, false).unwrap();
        assert_eq!((f.quantity, f.unit), (1.0, "filter"));
        assert!(f.plain.contains("3.6 L"), "{}", f.plain);
        // Philadelphia's walks need 2 L and 0.6 L: no filters (the old rule bought two).
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        for (i, person) in p.people.iter().enumerate() {
            if let Some(c) = person.commute.as_ref() {
                assert!(get_home_filter(i + 1, c, false).is_none());
            }
        }
        let kit = car_kit(2).unwrap();
        assert_eq!(kit.quantity, 2.0);
        assert!(kit.plain.contains("jump pack") && kit.plain.contains("roadside"));
        assert!(car_kit(0).is_none());
    }
}
