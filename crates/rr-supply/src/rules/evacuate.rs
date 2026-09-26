//! Leaving home quickly (research §9.1, §9.4): go-bags sized to the warning time, water and food for
//! three days, pet carriers and supplies, the half-tank rule, and help for people who need it.

use rr_types::{Per, Person, Pets};

use super::Sizing;
use super::water::{daily, pets_phrase};
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{count, days as fmt_days, gallons, num};
use crate::household::is_4_plus;
use crate::rules::food::household_kcal;

/// How much warning the household can expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeBand {
    /// Under an hour (a local tsunami, a fast fire): the bag lives by the door.
    Minutes,
    /// Hours (a flash flood).
    Hours,
    /// A day or more (a hurricane).
    Days,
}

/// The band for the least warning to expect.
pub fn notice_band(notice_hours_low: f64) -> NoticeBand {
    let mut b = Basis::new();
    notice_band_with(&mut b, notice_hours_low)
}

fn notice_band_with(b: &mut Basis, hours: f64) -> NoticeBand {
    let minutes = b.k(keys::NOTICE_MINUTES_BAND_HOURS);
    let within_hours = b.k(keys::NOTICE_HOURS_BAND_HOURS);
    if hours < minutes {
        NoticeBand::Minutes
    } else if hours < within_hours {
        NoticeBand::Hours
    } else {
        NoticeBand::Days
    }
}

/// Go-bags: one per person aged 4 and over, packed for the warning the household can expect. Rule
/// `go_bag`.
pub fn go_bag(people_list: &[Person], notice_hours_low: f64, days_away: f64) -> Sizing {
    let mut b = Basis::new();
    let each = b.k(keys::GO_BAGS_PER_PERSON);
    b.cite("ready_gov_kit");
    b.cite("cdc_evacuation_psa");
    let band = notice_band_with(&mut b, notice_hours_low);
    let n = people_list.iter().filter(|p| is_4_plus(p)).count().max(1) as f64;
    let q = n * each;
    let babies = people_list.len() as f64 > n;
    let when = match band {
        NoticeBand::Minutes => {
            "You may get only minutes of warning (a local tsunami or a fast fire), so keep the bags packed by the door, and one at work if you work in a danger zone. Practise grabbing them and leaving."
        }
        NoticeBand::Hours => {
            "You may get only a few hours of warning (a flash flood), so keep the bags packed and know your route out."
        }
        NoticeBand::Days => {
            "You may get a day or more of warning (a hurricane): pack the car early, fill the tank and leave before the roads jam."
        }
    };
    let stay = if days_away > 0.0 {
        format!(
            " Plan where you would stay for about {}.",
            fmt_days(days_away)
        )
    } else {
        String::new()
    };
    let text = format!(
        "{}{}: water, food, medicines, copies of documents, a phone charger, cash, a flashlight, a whistle, a change of clothes and sturdy shoes. {when}{stay}",
        count(q, "go-bag", "go-bags"),
        if babies {
            ", one for each person aged 4 and over (babies' things go in a parent's bag)"
        } else {
            ", one for each person"
        },
    );
    Sizing::new(&b, "go_bag", "go_bag", q, "bag", Per::Person, text)
}

/// Water for the go-bags: three days at the household's level (the Red Cross evacuation supply).
/// Rule `go_bag_water`.
pub fn go_bag_water(
    people_list: &[Person],
    level: rr_types::WaterLevel,
    hot: bool,
    days_away: f64,
) -> Sizing {
    let mut b = Basis::new();
    let bag_days = b.k(keys::GO_BAG_DAYS);
    let carry = b.k(keys::WALK_WATER_CARRY_MAX_L);
    let d = daily(&mut b, people_list, &Pets::default(), level, hot);
    let days = if days_away > 0.0 {
        days_away.min(bag_days)
    } else {
        bag_days
    };
    let per_day = d.people_gal();
    let q = per_day * days;
    let text = format!(
        "Water for the go-bags: {} a day for {} × {} = {}. If you leave on foot, carry what you can (about {} L each) and a filter; the rest goes in the car.",
        gallons(per_day),
        count(people_list.len() as f64, "person", "people"),
        fmt_days(days),
        gallons(super::round_quantity("gallon", q)),
        num(carry, 1)
    );
    Sizing::new(
        &b,
        "go_bag_water",
        "go_bag_water",
        q,
        "gallon",
        Per::Person,
        text,
    )
    .per_day(days, per_day)
}

/// Food for the go-bags: three days that need no cooking. Rule `go_bag_food`.
pub fn go_bag_food(people_list: &[Person], days_away: f64) -> Option<Sizing> {
    let mut b = Basis::new();
    let bag_days = b.k(keys::GO_BAG_DAYS);
    b.cite("wa_emd_prepare_in_a_year");
    let per_day = household_kcal(&mut b, people_list);
    if per_day <= 0.0 {
        return None;
    }
    let days = if days_away > 0.0 {
        days_away.min(bag_days)
    } else {
        bag_days
    };
    let q = per_day * days;
    let text = format!(
        "Food for the go-bags: about {} kcal a day × {} = {} kcal of food that needs no cooking or refrigeration.",
        num(per_day, 0),
        fmt_days(days),
        num(super::round_quantity("kcal", q), 0)
    );
    Some(
        Sizing::new(
            &b,
            "go_bag_food",
            "go_bag_food",
            q,
            "kcal",
            Per::Person,
            text,
        )
        .per_day(days, per_day),
    )
}

/// A carrier for each pet. Rule `pet_carrier`.
pub fn pet_carrier(pets: &Pets) -> Option<Sizing> {
    let n = u32::from(pets.dogs) + u32::from(pets.cats) + u32::from(pets.small);
    if n == 0 {
        return None;
    }
    let mut b = Basis::new();
    b.cite("aspca_disaster_prep");
    b.cite("ready_gov_evacuation");
    let text = format!(
        "{}, one for each pet. Public shelters may take only service animals, so find pet-friendly places to stay ahead of time.",
        count(f64::from(n), "pet carrier", "pet carriers")
    );
    Some(Sizing::new(
        &b,
        "pet_carrier",
        "pet_carrier",
        f64::from(n),
        "carrier",
        Per::Pet,
        text,
    ))
}

/// Water in the pets' go-kit: a week (ASPCA), replaced every two months. Rule `pet_go_water`.
pub fn pet_go_water(pets: &Pets) -> Option<Sizing> {
    if u32::from(pets.dogs) + u32::from(pets.cats) + u32::from(pets.small) == 0 {
        return None;
    }
    let mut b = Basis::new();
    let days = b.k(keys::PET_EVAC_WATER_DAYS);
    let rotate = b.k(keys::PET_KIT_ROTATION_MONTHS);
    let d = daily(&mut b, &[], pets, rr_types::WaterLevel::Basic, false);
    let per_day = d.pets_gal();
    let q = per_day * days;
    let text = format!(
        "Water in the pet go-kit for {}: {} = {}. Replace it every {}.",
        pets_phrase(d.pets),
        fmt_days(days),
        gallons(super::round_quantity("gallon", q)),
        count(rotate, "month", "months")
    );
    Some(
        Sizing::new(&b, "pet_go_water", "pet_water", q, "gallon", Per::Pet, text)
            .per_day(days, per_day),
    )
}

/// Food in the pets' go-kit: ten days (ASPCA 7–10), replaced every two months. Rule `pet_go_food`.
pub fn pet_go_food(pets: &Pets) -> Option<Sizing> {
    let n = u32::from(pets.dogs) + u32::from(pets.cats) + u32::from(pets.small);
    if n == 0 {
        return None;
    }
    let mut b = Basis::new();
    let days = b.k(keys::PET_EVAC_FOOD_DAYS);
    let (lo, hi) = b.range(keys::PET_EVAC_FOOD_DAYS);
    let rotate = b.k(keys::PET_KIT_ROTATION_MONTHS);
    let q = f64::from(n) * days;
    let text = format!(
        "Food in the pet go-kit for {}: {} ({} to {} days) = {} pet-days, with any medicine they take, their records and a photo. Replace the food every {}.",
        pets_phrase([pets.dogs, pets.cats, pets.small]),
        fmt_days(days),
        num(lo, 0),
        num(hi, 0),
        num(q, 0),
        count(rotate, "month", "months")
    );
    Some(
        Sizing::new(&b, "pet_go_food", "pet_food", q, "pet_day", Per::Pet, text)
            .per_day(days, f64::from(n)),
    )
}

/// Keep every vehicle at least half full (half charged for an electric car). Rule
/// `fuel_half_tank`.
pub fn fuel_half_tank(vehicles: usize, evs: usize) -> Option<Sizing> {
    if vehicles == 0 {
        return None;
    }
    let mut b = Basis::new();
    let share = b.k(keys::GAS_TANK_MIN_SHARE);
    let half = if (share - 0.5).abs() < 1e-9 {
        "half".to_owned()
    } else {
        format!("{} %", num(share * 100.0, 0))
    };
    let mut text = format!(
        "Keep {} at least {half} full at all times, and fill up when leaving looks likely.",
        if vehicles == 1 {
            "the vehicle".to_owned()
        } else {
            format!("all {vehicles} vehicles")
        }
    );
    if evs > 0 {
        b.cite(crate::constants::PRIOR_SOURCE);
        text.push_str(&format!(" Keep an electric car at least {half} charged too, and charge it fully when a storm is forecast."));
    }
    Some(Sizing::new(
        &b,
        "fuel_half_tank",
        "fuel_half_tank",
        vehicles as f64,
        "vehicle",
        Per::Household,
        text,
    ))
}

/// A plan for how to leave without a car. Rule `evacuation_ride_plan`.
pub fn evacuation_ride_plan(vehicles: usize) -> Option<Sizing> {
    if vehicles > 0 {
        return None;
    }
    let mut b = Basis::new();
    b.cite("ready_gov_evacuation");
    let text = "No car: decide now who would drive you, or which bus or train leads out of the area, and leave early, because it takes longer without a car.".to_owned();
    Some(Sizing::new(
        &b,
        "evacuation_ride_plan",
        "evacuation_ride_plan",
        1.0,
        "plan",
        Per::Household,
        text,
    ))
}

/// A transport plan for each person who needs help to leave (limited mobility or a wheelchair).
/// Rule `evacuation_assistance_plan`.
pub fn evacuation_assistance_plan(people_list: &[Person]) -> Option<Sizing> {
    let n = people_list
        .iter()
        .filter(|p| p.medical.mobility != rr_types::Mobility::None)
        .count() as f64;
    if n == 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let each = b.k(keys::EVACUATION_PLANS_PER_PERSON);
    let text = format!(
        "{} may need help to leave quickly: arrange a ride and a helper now, and show them where the medicines and equipment are.",
        count(n * each, "person", "people")
    );
    Some(Sizing::new(
        &b,
        "evacuation_assistance_plan",
        "evacuation_assistance_plan",
        n * each,
        "person",
        Per::Person,
        text,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn bands() {
        assert_eq!(notice_band(0.25), NoticeBand::Minutes);
        assert_eq!(notice_band(1.0), NoticeBand::Hours);
        assert_eq!(notice_band(23.9), NoticeBand::Hours);
        assert_eq!(notice_band(24.0), NoticeBand::Days);
    }

    #[test]
    fn philadelphia_go_bags() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        let bags = go_bag(&p.people, 2.0, 3.0);
        assert_eq!(bags.quantity, 4.0);
        assert!(bags.plain.contains("few hours"));
        let w = go_bag_water(&p.people, rr_types::WaterLevel::Basic, false, 3.0);
        assert_eq!(w.quantity, 12.0); // 4 people × 1 gal × 3 days (pets have their own kit)
        assert!(w.citations.iter().any(|c| c == "red_cross_survival_kit"));
        let f = go_bag_food(&p.people, 3.0).unwrap();
        assert_eq!(f.quantity, 24_600.0);
        assert_eq!(pet_carrier(&p.pets).unwrap().quantity, 1.0);
        assert_eq!(pet_go_water(&p.pets).unwrap().quantity, 2.2); // 0.3125 × 7
        assert_eq!(pet_go_food(&p.pets).unwrap().quantity, 10.0);
        assert_eq!(fuel_half_tank(1, 0).unwrap().quantity, 1.0);
        assert!(evacuation_ride_plan(1).is_none());
        assert_eq!(evacuation_assistance_plan(&p.people).unwrap().quantity, 1.0);
    }

    #[test]
    fn minutes_of_warning_keep_bags_by_the_door() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        let bags = go_bag(&p.people, 0.25, 14.0);
        assert!(bags.plain.contains("minutes"));
        assert!(bags.plain.contains("14 days"));
        assert_eq!(
            go_bag_water(&p.people, rr_types::WaterLevel::Basic, false, 14.0).quantity,
            6.0
        );
    }

    #[test]
    fn no_car_and_evs() {
        assert!(evacuation_ride_plan(0).is_some());
        let s = fuel_half_tank(2, 1).unwrap();
        assert!(s.plain.contains("electric car") && s.prior);
        let babies = fixtures::get("sugar-land-ev-household-3").unwrap();
        assert_eq!(go_bag(&babies.people, 48.0, 5.0).quantity, 2.0);
    }
}
