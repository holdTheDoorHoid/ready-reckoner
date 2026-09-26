//! Sanitation and hygiene (research §4): the two-bucket toilet, soap, period products, diapers and
//! wipes.

use rr_types::{AgeBand, Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{CUPS_PER_GAL, DAYS_PER_MONTH, ceil_count, count, days as fmt_days, num};

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

fn toilet_users(people_list: &[Person]) -> usize {
    people_list
        .iter()
        .filter(|p| p.age_band != AgeBand::Infant)
        .count()
}

/// Heavy bags for the two-bucket toilet (an estimate from Oregon's and RDPO's guidance). Rule
/// `toilet_bags`.
pub fn toilet_bags(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let n = toilet_users(people_list) as f64;
    if n == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per = b.k(keys::TOILET_BAGS_PER_PERSON_DAY);
    let (lo, hi) = b.range(keys::TOILET_BAGS_PER_PERSON_DAY);
    let q = ceil_count(n * per * days);
    let text = format!(
        "Toilet bags: about {} heavy garbage bags a person a day ({} to {}), counting both bags of a double-bagged load: {} × {} = {} bags. Fill halfway, tie, double-bag, and store away from food, water, children and pets. Don't put them in curbside trash or bury them unless officials say so.",
        num(per, 2),
        num(lo, 1),
        num(hi, 1),
        count(n, "person", "people"),
        fmt_days(days),
        num(q, 0)
    );
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
        .per_day(days, n * per),
    )
}

/// Dry cover material for the two-bucket toilet (an estimate from RDPO's "a handful per poo").
/// Rule `toilet_cover_material`.
pub fn toilet_cover_material(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let n = toilet_users(people_list) as f64;
    if n == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let per = b.k(keys::TOILET_COVER_CUPS_PER_PERSON_DAY);
    let (lo, hi) = b.range(keys::TOILET_COVER_CUPS_PER_PERSON_DAY);
    let cups = ceil_count(n * per * days);
    let text = format!(
        "Cover material for the poo bucket (sawdust, shredded paper, dry leaves or pet bedding): about {} a person a day ({} to {}) × {} × {} = {} cups, about {}.",
        count(per, "cup", "cups"),
        num(lo, 1),
        num(hi, 1),
        count(n, "person", "people"),
        fmt_days(days),
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
        .per_day(days, n * per),
    )
}

/// Bathing and laundry soap for the supplies target (Sphere). Rule `soap_grams`.
pub fn soap_grams(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let n = people_list.len() as f64;
    if n == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let bath = b.k(keys::SOAP_BATH_G_PER_PERSON_MONTH);
    let laundry = b.k(keys::SOAP_LAUNDRY_G_PER_PERSON_MONTH);
    let q = (bath + laundry) * n * days / DAYS_PER_MONTH;
    let text = format!(
        "Soap: {} g of bathing soap and {} g of laundry soap a person a month × {} × {} = about {} g.",
        num(bath, 0),
        num(laundry, 0),
        count(n, "person", "people"),
        fmt_days(days),
        num(super::round_quantity("gram", q), 0)
    );
    Some(
        Sizing::new(&b, "soap_grams", "soap", q, "gram", Per::Person, text)
            .per_day(days, (bath + laundry) * n / DAYS_PER_MONTH),
    )
}

/// Period products for two cycles, plus one per 28 days beyond a month, for the adults and teens
/// who may need them (half of them, because the form does not ask sex). Rule `menstrual_products`.
pub fn menstrual_products(days: f64, people_list: &[Person]) -> Option<Sizing> {
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
    let month = DAYS_PER_MONTH;
    let cycles = if days > month {
        let cycle_days = b.k(keys::MENSTRUAL_CYCLE_DAYS);
        base_cycles + ceil_count((days - month) / cycle_days)
    } else {
        base_cycles
    };
    b.cite("oregon_b2wr_toolkit");
    let people_eq = candidates * share;
    let q = ceil_count(people_eq * cycles * per_cycle);
    let text = format!(
        "Period products for {} cycles, about {} a cycle ({} to {}). The form doesn't ask sex, so the plan counts half of the {} aged 13 to 64 who are not pregnant or nursing: {} products. Keep soap, clean underwear and pain relievers with them, and put used products in a separate bag.",
        num(cycles, 0),
        num(per_cycle, 0),
        num(lo, 0),
        num(hi, 0),
        count(candidates, "person", "people"),
        num(q, 0)
    );
    Some(Sizing::new(
        &b,
        "menstrual_products",
        "menstrual_products",
        q,
        "product",
        Per::Person,
        text,
    ))
}

/// Diapers for babies and toddlers (estimates; CDC asks for at least one large pack). Rule
/// `diapers`.
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
    let mut per_day = 0.0;
    let mut parts = Vec::new();
    if infants > 0.0 {
        let each = b.k(keys::DIAPERS_PER_DAY_INFANT);
        let (lo, hi) = b.range(keys::DIAPERS_PER_DAY_INFANT);
        per_day += infants * each;
        parts.push(format!(
            "about {} a day for each baby ({} to {})",
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
    let q = ceil_count(per_day * days);
    let text = format!(
        "Diapers: {}, × {} = {} diapers. Keep at least one large pack on hand.",
        crate::format::and_list(&parts),
        fmt_days(days),
        num(q, 0)
    );
    Some(
        Sizing::new(&b, "diapers", "diapers", q, "diaper", Per::Person, text)
            .per_day(days, per_day),
    )
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
        assert_eq!(toilet_bags(3.0, &p.people).unwrap().quantity, 6.0);
        assert_eq!(
            toilet_cover_material(3.0, &p.people).unwrap().quantity,
            12.0
        );
        // (250 + 200) × 4 × 10 / 30 = 600 g
        assert_eq!(soap_grams(10.0, &p.people).unwrap().quantity, 600.0);
        // half of 2 adults × 2 cycles × 20 = 40
        let m = menstrual_products(10.0, &p.people).unwrap();
        assert_eq!(m.quantity, 40.0);
        assert!(m.prior && m.plain.contains("half"));
        assert!(diapers(10.0, &p.people).is_none());
    }

    #[test]
    fn longer_plans_add_cycles() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        // 60 days: 2 + ceil(30 / 28) = 4 cycles × 20 = 80
        assert_eq!(menstrual_products(60.0, &p.people).unwrap().quantity, 80.0);
    }

    #[test]
    fn babies_and_toddlers() {
        let sl = fixtures::get("sugar-land-ev-household-3").unwrap();
        assert_eq!(diapers(7.0, &sl.people).unwrap().quantity, 70.0);
        assert_eq!(baby_wipes(7.0, &sl.people).unwrap().quantity, 2.0);
        assert_eq!(baby_wipes(28.0, &sl.people).unwrap().quantity, 4.0);
        // The infant is not a toilet user.
        assert_eq!(toilet_bags(10.0, &sl.people).unwrap().quantity, 9.0); // 2 × 0.45 × 10
        let hays = fixtures::get("hays-kansas-farm-5").unwrap();
        // Toddler: 6 a day (an estimate).
        assert_eq!(diapers(10.0, &hays.people).unwrap().quantity, 60.0);
        // Adults and teens not pregnant: 1 adult + 1 teen → half → 1 × 2 × 20
        assert_eq!(
            menstrual_products(10.0, &hays.people).unwrap().quantity,
            40.0
        );
    }
}
