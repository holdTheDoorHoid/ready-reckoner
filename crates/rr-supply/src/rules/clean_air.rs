//! Unhealthy air indoors (contract v2's `clean_air` bucket; DESIGN-DELTA §1.2): respirators for
//! teens and adults, sized by the county's smoke days, an air cleaner sized to the clean room (EPA's
//! CADR table) or a cheaper box fan with a MERV 13 filter, and a clean-room plan.

use rr_types::{Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::{constants, keys};
use crate::format::{count, days as fmt_days, num};
use crate::household::is_13_plus;

/// Days of respirators to store: the county's days a year of unhealthy smoke, at least the five
/// days of the old outbreak estimate (`n95_days`) and at most `respirator_days_max` (30); five days
/// when the county's smoke days are not known. Returns the days and whether the smoke figure set
/// them.
fn respirator_days(b: &mut Basis, smoke_days: Option<f64>) -> (f64, bool) {
    let min = constants().value(keys::N95_DAYS);
    let max = constants().value(keys::RESPIRATOR_DAYS_MAX);
    match smoke_days.filter(|d| d.is_finite() && *d > min) {
        Some(d) if d > max => (b.k(keys::RESPIRATOR_DAYS_MAX), true),
        Some(d) => (d.round(), true),
        None => (b.k(keys::N95_DAYS), false),
    }
}

/// N95 respirators for smoke, dust or ash: one a day for each person aged 13 and over (NIOSH
/// respirators come in adult sizes; EPA: masks are not made to fit children, whose protection is
/// the clean room), for the county's smoke days (round-2 review P-22 and the clean-air bucket).
/// Rule `n95_masks`.
pub fn n95_masks(people_list: &[Person], smoke_days: Option<f64>) -> Option<Sizing> {
    let n = people_list.iter().filter(|p| is_13_plus(p)).count();
    if n == 0 {
        return None;
    }
    let mut b = Basis::new();
    let per_day = b.k(keys::N95_PER_PERSON_DAY);
    let (days, from_smoke) = respirator_days(&mut b, smoke_days);
    b.cite("cdc_wildfire_smoke");
    let q = n as f64 * per_day * days;
    let children = people_list.len() > n;
    let why = if from_smoke {
        format!(
            "your county averages about {} of unhealthy smoke a year",
            fmt_days(days)
        )
    } else {
        format!("for about {} of smoke, dust or ash", fmt_days(days))
    };
    let mut text = format!(
        "N95 respirators, {why}: {} a day for each of the {} aged 13 and over = {} respirators. A well-fitting N95 protects far more than a cloth or surgical mask; replace it when breathing through it gets harder.",
        num(per_day, 0),
        count(n as f64, "person", "people"),
        num(q, 0)
    );
    if children {
        b.cite("epa_children_wildfire_smoke");
        text.push_str(
            " Respirators are not made to fit young children (EPA): for them, the clean room is the protection.",
        );
    }
    Some(
        Sizing::new(&b, "n95_masks", "respirator", q, "mask", Per::Person, text).math(vec![
            format!(
                "{n} people aged 13+ × {} a day × {} days = {} masks",
                num(per_day, 0),
                num(days, 0),
                num(q, 0)
            ),
        ]),
    )
}

/// A portable air cleaner with a HEPA filter for the clean room, sized by EPA's table (a clean air
/// delivery rate of about 0.65 cubic feet a minute per square foot, 8-foot ceiling) to a bedroom of
/// about 200 square feet (an estimate). Its cheaper alternative is [`diy_filter_box`]. Rule
/// `air_cleaner_units`.
pub fn air_cleaner_units() -> Sizing {
    let mut b = Basis::new();
    let per_sqft = b.k(keys::AIR_CLEANER_CADR_PER_SQFT);
    let room = b.k(keys::CLEAN_ROOM_SQFT);
    b.cite("epa_wildfire_indoor_air");
    let cadr = (per_sqft * room).round();
    let text = format!(
        "1 portable air cleaner with a HEPA filter for your clean room: for a room of about {} square feet, a clean air delivery rate (CADR) of at least {} for smoke (EPA's sizing, more for high ceilings). Keep a spare filter, and avoid air cleaners that make ozone.",
        num(room, 0),
        num(cadr, 0)
    );
    Sizing::new(
        &b,
        "air_cleaner_units",
        "air_cleaner",
        1.0,
        "air cleaner",
        Per::Household,
        text,
    )
    .math(vec![format!(
        "{} sq ft × {} cfm per sq ft = {} cfm CADR",
        num(room, 0),
        num(per_sqft, 2),
        num(cadr, 0)
    )])
}

/// The cheaper way to clean the room's air: a box fan with a MERV 13 filter attached, which EPA
/// calls a cost-effective way to reduce smoke. An alternative line of `air_cleaner_units`, never an
/// addition. Rule `diy_filter_box`.
pub fn diy_filter_box() -> Sizing {
    let mut b = Basis::new();
    b.cite("epa_diy_air_cleaners");
    let text = "Cheaper than an air cleaner: a 20-inch box fan with a 20 by 20 inch MERV 13 filter taped to its intake side, which EPA calls a cost-effective way to cut smoke indoors. Use a newer fan with safety features, and do not leave it running while no one is home.".to_owned();
    Sizing::new(
        &b,
        "diy_filter_box",
        "air_cleaner",
        1.0,
        "filter box",
        Per::Household,
        text,
    )
}

/// A clean-room plan: one room kept as clean as possible on smoky days (EPA). Rule
/// `clean_room_plan`.
pub fn clean_room_plan(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    b.cite("epa_wildfire_indoor_air");
    b.cite("cdc_wildfire_smoke");
    let mut text = "A clean-room plan for smoky or dusty days: pick one room with few windows and doors where everyone can sleep, keep it closed, set the heating or cooling to recirculate (or turn off fans that pull in outside air), run the air cleaner there, and do not fry food, burn candles or vacuum on those days.".to_owned();
    if people_list.len() > people_list.iter().filter(|p| is_13_plus(p)).count() {
        text.push_str(" Children stay in the clean room: respirators do not fit them.");
    }
    Sizing::new(
        &b,
        "clean_room_plan",
        "clean_room_plan",
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
    fn respirators_count_teens_and_adults_for_the_smoke_days() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        // 3 people aged 13+ (the child is left out) × 1 × 5 days when smoke days are not known.
        let s = n95_masks(&p.people, None).unwrap();
        assert_eq!(s.quantity, 15.0);
        assert!(
            s.plain.contains("clean room is the protection"),
            "{}",
            s.plain
        );
        assert!(
            s.citations
                .iter()
                .any(|c| c == "epa_children_wildfire_smoke")
        );
        assert!(s.prior);
        // A smoky county: 12 days a year → 36 respirators; 60 days is capped at 30.
        assert_eq!(n95_masks(&p.people, Some(12.2)).unwrap().quantity, 36.0);
        assert_eq!(n95_masks(&p.people, Some(60.0)).unwrap().quantity, 90.0);
        // Fewer smoke days than the five-day floor keep the floor.
        assert_eq!(n95_masks(&p.people, Some(1.0)).unwrap().quantity, 15.0);
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        // 2 adults and a teen; the child and the toddler get the clean room instead.
        assert_eq!(n95_masks(&hays.people, None).unwrap().quantity, 15.0);
    }

    #[test]
    fn the_air_cleaner_is_sized_by_epas_table() {
        let s = air_cleaner_units();
        assert!(s.plain.contains("at least 130"), "{}", s.plain);
        assert!(s.citations.iter().any(|c| c == "epa_air_cleaner_guide"));
        assert!(s.prior, "the room size is an estimate");
        let d = diy_filter_box();
        assert!(d.plain.contains("MERV 13") && d.plain.contains("cost-effective"));
        assert!(!d.prior);
        let plan = clean_room_plan(&fixtures::get("philadelphia-renters-4").unwrap().people);
        assert!(plan.plain.contains("recirculate") && plan.plain.contains("Children"));
    }
}
