//! Sanitation and hygiene (research §4): the two-bucket toilet, toilet paper, soap, period
//! products, diapers and wipes.

use rr_types::{AgeBand, Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::{constants, keys};
use crate::format::{
    CUPS_PER_GAL, DAYS_PER_MONTH, ceil_count, count, day_adjective, days as fmt_days, num,
};

/// Two buckets (pee and poo) with a seat for when toilets cannot flush. Rule `toilet_buckets`.
pub fn toilet_buckets() -> Sizing {
    let mut b = Basis::new();
    let q = b.k(keys::TOILET_BUCKETS_PER_HOUSEHOLD);
    let text = format!(
        "A two-bucket toilet for when toilets can't flush: {} sturdy buckets with a seat, one for pee and one for poo.",
        num(q, 0)
    );
    Sizing::new(
        &b,
        "toilet_buckets",
        "toilet_bucket",
        q,
        "bucket",
        Per::Household,
        text,
    )
}

/// Everyone but babies under 1 (who are in diapers).
fn toilet_users(people_list: &[Person]) -> f64 {
    people_list
        .iter()
        .filter(|p| p.age_band != AgeBand::Infant)
        .count() as f64
}

/// Days of toilet supplies to store: the no-water target, but no more than
/// `toilet_supplies_cap_days` (30, an estimate) unless an earthquake drives the target, since a
/// working sewer or septic system usually outlasts the tap water and an earthquake can break the
/// sewer itself (round-2 review P-16: 329 bags for a year). Returns the days and whether the cap
/// applied.
fn toilet_days(b: &mut Basis, days: f64, earthquake: bool) -> (f64, bool) {
    let cap = constants().value(keys::TOILET_SUPPLIES_CAP_DAYS);
    if !earthquake && days > cap {
        (b.k(keys::TOILET_SUPPLIES_CAP_DAYS), true)
    } else {
        (days, false)
    }
}

/// Heavy bags for the two-bucket toilet (an estimate from Oregon's and RDPO's guidance), for the
/// days [`toilet_days`] gives. Rule `toilet_bags`.
pub fn toilet_bags(days: f64, people_list: &[Person], earthquake: bool) -> Option<Sizing> {
    let n = toilet_users(people_list);
    if n == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per = b.k(keys::TOILET_BAGS_PER_PERSON_DAY);
    let (lo, hi) = b.range(keys::TOILET_BAGS_PER_PERSON_DAY);
    let (d, capped) = toilet_days(&mut b, days, earthquake);
    let q = ceil_count(n * per * d);
    let mut text = format!(
        "Toilet bags: about {} heavy garbage bags a person a day ({} to {}), counting both bags of a double-bagged load: {} × {} = {} bags. Fill halfway, tie, double-bag, and store away from food, water, children and pets. Don't put them in curbside trash or bury them unless officials say so.",
        num(per, 2),
        num(lo, 1),
        num(hi, 1),
        count(n, "person", "people"),
        fmt_days(d),
        num(q, 0)
    );
    if capped {
        text.push_str(&format!(
            " Bags are counted for the first {} of your {} target: if the sewer or septic system still works, it is a better toilet than bags, and officials will say when it is safe to flush.",
            fmt_days(d),
            day_adjective(days)
        ));
    }
    Some(
        Sizing::new(
            &b,
            "toilet_bags",
            "toilet_bags",
            q,
            "bag",
            Per::Person,
            text,
        )
        .per_day(d, n * per),
    )
}

/// Dry cover material for the two-bucket toilet (an estimate from RDPO's "a handful per poo"), for
/// the days [`toilet_days`] gives. Rule `toilet_cover_material`.
pub fn toilet_cover_material(
    days: f64,
    people_list: &[Person],
    earthquake: bool,
) -> Option<Sizing> {
    let n = toilet_users(people_list);
    if n == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per = b.k(keys::TOILET_COVER_CUPS_PER_PERSON_DAY);
    let (lo, hi) = b.range(keys::TOILET_COVER_CUPS_PER_PERSON_DAY);
    let (d, _) = toilet_days(&mut b, days, earthquake);
    let cups = ceil_count(n * per * d);
    let text = format!(
        "Cover material for the poo bucket (sawdust, shredded paper, dry leaves or pet bedding): about {} a person a day ({} to {}) × {} × {} = {} cups, about {}.",
        count(per, "cup", "cups"),
        num(lo, 1),
        num(hi, 1),
        count(n, "person", "people"),
        fmt_days(d),
        num(cups, 0),
        crate::format::gallons(cups / CUPS_PER_GAL)
    );
    Some(
        Sizing::new(
            &b,
            "toilet_cover_material",
            "toilet_cover_material",
            cups,
            "cup",
            Per::Person,
            text,
        )
        .per_day(d, n * per),
    )
}

/// Paper tableware, garbage bags and ties, and waterproof matches for a no-water target longer than
/// the 14 stored days (round-2 review P-23; item N-16): one kit per `household_ops_kit_days` (14, an
/// estimate). Ready.gov's kit lists paper cups, plates, towels and plastic utensils, garbage bags and
/// plastic ties, and matches in a waterproof container; paper plates save the water washing dishes
/// would take. Rule `household_ops_kits`.
pub fn household_ops_kits(days: f64) -> Option<Sizing> {
    let cap = constants().value(keys::WATER_STORED_CAP_DAYS);
    if !(days > cap) {
        return None;
    }
    let mut b = Basis::new();
    let per = b.k(keys::HOUSEHOLD_OPS_KIT_DAYS);
    let q = ceil_count(days / per);
    let text = format!(
        "{} of paper plates, cups and utensils, garbage bags with ties, and waterproof matches, one for every {} of your {} no-water target: paper plates save the water that washing dishes takes.",
        count(q, "household kit", "household kits"),
        fmt_days(per),
        day_adjective(days)
    );
    Some(Sizing::new(
        &b,
        "household_ops_kits",
        "household_ops",
        q,
        "kit",
        Per::Household,
        text,
    ))
}

/// Toilet paper: about one roll per person every five days (an estimate; Oregon says measure a
/// week's use and double it). Rule `toilet_paper_rolls`.
pub fn toilet_paper_rolls(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let n = toilet_users(people_list);
    if n == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per_roll = b.k(keys::TOILET_PAPER_PERSON_DAYS_PER_ROLL);
    let q = ceil_count(n * days / per_roll);
    let text = format!(
        "Toilet paper: about 1 roll a person every {} × {} × {} = {} rolls. Better still, measure a week's use and double it for two weeks.",
        fmt_days(per_roll),
        count(n, "person", "people"),
        fmt_days(days),
        num(q, 0)
    );
    Some(
        Sizing::new(
            &b,
            "toilet_paper_rolls",
            "toilet_paper",
            q,
            "roll",
            Per::Person,
            text,
        )
        .per_day(days, n / per_roll),
    )
}

/// Bathing and laundry soap in person-months (Sphere: 250 g and 200 g a person a month). Rule
/// `soap_person_months`.
pub fn soap_person_months(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let n = people_list.len() as f64;
    if n == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let bath = b.k(keys::SOAP_BATH_G_PER_PERSON_MONTH);
    let laundry = b.k(keys::SOAP_LAUNDRY_G_PER_PERSON_MONTH);
    let months = ceil_count(days / DAYS_PER_MONTH);
    let q = n * months;
    let text = format!(
        "Soap for {}: {} × {}. Each is {} g of bathing soap and {} g of laundry soap, about {} g in all.",
        count(q, "person-month", "person-months"),
        count(n, "person", "people"),
        count(months, "month", "months"),
        num(bath, 0),
        num(laundry, 0),
        num(q * (bath + laundry), 0)
    );
    Some(Sizing::new(
        &b,
        "soap_person_months",
        "soap",
        q,
        "person_month",
        Per::Person,
        text,
    ))
}

/// Period products in cycles: two cycles (plus one per 28 days beyond a month) for half of the
/// adults and teens who are not pregnant or nursing, because the form does not ask sex. Rule
/// `menstrual_cycles`.
pub fn menstrual_cycles(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let candidates = people_list
        .iter()
        .filter(|p| matches!(p.age_band, AgeBand::Adult | AgeBand::Teen) && !p.pregnant_or_nursing)
        .count() as f64;
    if candidates == 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let share = b.k(keys::MENSTRUATING_SHARE);
    let base_cycles = b.k(keys::MENSTRUAL_CYCLES_KIT);
    let per_cycle = b.k(keys::MENSTRUAL_PRODUCTS_PER_CYCLE);
    let (lo, hi) = b.range(keys::MENSTRUAL_PRODUCTS_PER_CYCLE);
    let cycles_each = if days > DAYS_PER_MONTH {
        let cycle_days = b.k(keys::MENSTRUAL_CYCLE_DAYS);
        base_cycles + ceil_count((days - DAYS_PER_MONTH) / cycle_days)
    } else {
        base_cycles
    };
    b.cite("oregon_b2wr_toolkit");
    let q = ceil_count(candidates * share * cycles_each);
    let text = format!(
        "Period products for {} each, about {} products a cycle ({} to {}). The form doesn't ask sex, so the plan counts half of the {} aged 13 to 64 who are not pregnant or nursing: {}, about {} products. Keep soap, clean underwear and pain relievers with them, and put used products in a separate bag.",
        count(cycles_each, "cycle", "cycles"),
        num(per_cycle, 0),
        num(lo, 0),
        num(hi, 0),
        count(candidates, "person", "people"),
        count(q, "cycle's worth", "cycles' worth"),
        num(q * per_cycle, 0)
    );
    Some(Sizing::new(
        &b,
        "menstrual_cycles",
        "menstrual_products",
        q,
        "cycle",
        Per::Person,
        text,
    ))
}

/// Diapers for babies and toddlers (estimates), for at least three days. Rule `diapers`.
pub fn diapers(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let infants = people_list
        .iter()
        .filter(|p| p.age_band == AgeBand::Infant)
        .count() as f64;
    let toddlers = people_list
        .iter()
        .filter(|p| p.age_band == AgeBand::Toddler)
        .count() as f64;
    if infants + toddlers == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let min_days = b.k(keys::BABY_SUPPLY_MIN_DAYS);
    let d = days.max(min_days);
    let mut per_day = 0.0;
    let mut parts = Vec::new();
    if infants > 0.0 {
        let each = b.k(keys::DIAPERS_PER_DAY_INFANT);
        let (lo, hi) = b.range(keys::DIAPERS_PER_DAY_INFANT);
        per_day += infants * each;
        parts.push(format!(
            "about {} a day for each baby ({} to {}; newborns use the most)",
            num(each, 0),
            num(lo, 0),
            num(hi, 0)
        ));
    }
    if toddlers > 0.0 {
        let each = b.k(keys::DIAPERS_PER_DAY_TODDLER);
        per_day += toddlers * each;
        parts.push(format!(
            "about {} a day for each toddler still in diapers",
            num(each, 0)
        ));
    }
    let q = ceil_count(per_day * d);
    let text = format!(
        "Diapers: {}, × {} = {} diapers. Keep at least one large pack on hand.",
        crate::format::and_list(&parts),
        fmt_days(d),
        num(q, 0)
    );
    Some(Sizing::new(&b, "diapers", "diapers", q, "diaper", Per::Person, text).per_day(d, per_day))
}

/// Baby wipes: two packs per child in diapers for up to two weeks, more for longer (CDC). Rule
/// `baby_wipes`.
pub fn baby_wipes(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let children = people_list
        .iter()
        .filter(|p| matches!(p.age_band, AgeBand::Infant | AgeBand::Toddler))
        .count() as f64;
    if children == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per_2wk = b.k(keys::WIPES_PACKS_PER_CHILD_2WK);
    let q = ceil_count(children * per_2wk * (days / 14.0).max(1.0));
    let text = format!(
        "Baby wipes: at least {} packs for each child in diapers for up to 2 weeks: {} packs.",
        num(per_2wk, 0),
        num(q, 0)
    );
    Some(Sizing::new(
        &b,
        "baby_wipes",
        "baby_wipes",
        q,
        "pack",
        Per::Person,
        text,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn philadelphia_sanitation() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        assert_eq!(toilet_buckets().quantity, 2.0);
        // 4 × 0.45 × 3 = 5.4 → 6 bags; 4 × 1 × 3 = 12 cups
        assert_eq!(toilet_bags(3.0, &p.people, false).unwrap().quantity, 6.0);
        assert_eq!(
            toilet_cover_material(3.0, &p.people, false)
                .unwrap()
                .quantity,
            12.0
        );
        // 4 people × 10 days ÷ 5 = 8 rolls
        assert_eq!(toilet_paper_rolls(10.0, &p.people).unwrap().quantity, 8.0);
        // 4 people × 1 month (250 g + 200 g each)
        let soap = soap_person_months(10.0, &p.people).unwrap();
        assert_eq!(soap.quantity, 4.0);
        assert!(soap.plain.contains("1,800 g"), "{}", soap.plain);
        // half of 2 adults × 2 cycles
        let m = menstrual_cycles(10.0, &p.people).unwrap();
        assert_eq!(m.quantity, 2.0);
        assert!(m.prior && m.plain.contains("half") && m.plain.contains("40 products"));
        assert!(diapers(10.0, &p.people).is_none());
    }

    /// Round-2 review P-16: bags and cover material stop at 30 days unless an earthquake drives the
    /// no-water target (Coos Bay's year bought 329 bags).
    #[test]
    fn toilet_supplies_stop_at_a_month_unless_an_earthquake_breaks_the_sewer() {
        let p = fixtures::get("coos-bay-well-owner-2").unwrap();
        // 2 people × 0.45 × 30 days = 27 bags, not 2 × 0.45 × 365 = 329.
        let capped = toilet_bags(365.0, &p.people, false).unwrap();
        assert_eq!(capped.quantity, 27.0);
        assert_eq!(capped.days, Some(30.0));
        assert!(capped.plain.contains("sewer or septic"), "{}", capped.plain);
        assert!(capped.prior);
        assert_eq!(toilet_bags(365.0, &p.people, true).unwrap().quantity, 329.0);
        assert_eq!(
            toilet_cover_material(365.0, &p.people, false)
                .unwrap()
                .quantity,
            60.0
        );
        // Under the cap nothing changes.
        assert_eq!(toilet_bags(14.0, &p.people, false).unwrap().quantity, 13.0);
    }

    #[test]
    fn longer_plans_add_cycles() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        // 60 days: 2 + ceil(30 / 28) = 4 cycles each, for half of 2 people
        assert_eq!(menstrual_cycles(60.0, &p.people).unwrap().quantity, 4.0);
        assert_eq!(menstrual_cycles(30.0, &p.people).unwrap().quantity, 2.0);
    }

    #[test]
    fn babies_and_toddlers() {
        let sl = fixtures::get("sugar-land-ev-household-3").unwrap();
        assert_eq!(diapers(7.0, &sl.people).unwrap().quantity, 56.0); // 8 × 7
        assert_eq!(diapers(1.0, &sl.people).unwrap().quantity, 24.0); // at least 3 days
        assert_eq!(baby_wipes(7.0, &sl.people).unwrap().quantity, 2.0);
        assert_eq!(baby_wipes(28.0, &sl.people).unwrap().quantity, 4.0);
        // The baby is not a toilet user.
        assert_eq!(toilet_bags(10.0, &sl.people, false).unwrap().quantity, 9.0); // 2 × 0.45 × 10
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        assert_eq!(diapers(10.0, &hays.people).unwrap().quantity, 60.0); // toddler 6 a day
        // Adults and teens not pregnant: 1 adult + 1 teen → half → 1 × 2 cycles
        assert_eq!(menstrual_cycles(10.0, &hays.people).unwrap().quantity, 2.0);
    }
}
