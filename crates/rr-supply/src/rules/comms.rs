//! Communications, information and cash (research §7, §8.1).

use rr_types::{Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{count, days as fmt_days, num, usd};
use crate::household::{is_4_plus, is_13_plus};

/// A battery or hand-crank radio with NOAA Weather Radio. Rule `noaa_radio`.
pub fn noaa_radio() -> Sizing {
    let mut b = Basis::new();
    let q = b.k(keys::NOAA_RADIOS_PER_HOUSEHOLD);
    let text = format!(
        "{} that gets NOAA Weather Radio, with a tone alert, for warnings when phones and power are down.",
        count(
            q,
            "battery or hand-crank radio",
            "battery or hand-crank radios"
        )
    );
    Sizing::new(
        &b,
        "noaa_radio",
        "weather_radio",
        q,
        "radio",
        Per::Household,
        text,
    )
}

/// Power-bank capacity to keep phones charged through the outage target: one phone per person
/// aged 13 and over at about 15 Wh a day (an estimate). Rule `phone_power_wh`.
pub fn phone_power_wh(days: f64, people_list: &[Person]) -> Option<Sizing> {
    let phones = people_list.iter().filter(|p| is_13_plus(p)).count() as f64;
    if phones == 0.0 || days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let wh = b.k(keys::PHONE_WH_PER_DAY);
    let (lo, hi) = b.range(keys::PHONE_WH_PER_DAY);
    b.cite("ready_gov_kit");
    b.cite("ready_gov_earthquakes");
    let q = phones * wh * days;
    let text = format!(
        "Power banks: {} × about {} Wh a day ({} to {}) × {} without power = {} Wh. Text instead of calling to save battery.",
        count(phones, "phone", "phones"),
        num(wh, 0),
        num(lo, 0),
        num(hi, 0),
        fmt_days(days),
        num(super::round_quantity("Wh", q), 0)
    );
    Some(
        Sizing::new(
            &b,
            "phone_power_wh",
            "power_bank",
            q,
            "Wh",
            Per::Person,
            text,
        )
        .per_day(days, phones * wh),
    )
}

/// Two-way radios for everyone aged 13 and over, when there are at least two of them. Rule
/// `two_way_radios`.
pub fn two_way_radios(people_list: &[Person]) -> Option<Sizing> {
    let n = people_list.iter().filter(|p| is_13_plus(p)).count() as f64;
    let mut b = Basis::new();
    let min = b.k(keys::TWO_WAY_RADIO_MIN_PEOPLE);
    if n < min {
        return None;
    }
    let fee = b.k(keys::GMRS_LICENSE_USD);
    let years = b.k(keys::GMRS_LICENSE_YEARS);
    b.cite("fcc_frs");
    let text = format!(
        "{} for the {} aged 13 and over, to reach each other when phones are down. FRS radios need no licence; GMRS radios reach farther but need a {} FCC licence that covers the family for {} years.",
        count(n, "two-way radio", "two-way radios"),
        count(n, "person", "people"),
        usd(fee),
        num(years, 0)
    );
    Some(Sizing::new(
        &b,
        "two_way_radios",
        "two_way_radio",
        n,
        "radio",
        Per::Person,
        text,
    ))
}

/// A written contact card for each person aged 4 and over. Rule `contact_cards`.
pub fn contact_cards(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let each = b.k(keys::CONTACT_CARDS_PER_PERSON);
    let n = people_list.iter().filter(|p| is_4_plus(p)).count().max(1) as f64;
    let q = n * each;
    let text = format!(
        "{}, one for each person aged 4 and over: an out-of-area contact, meeting places and key numbers. Phones die; paper does not.",
        count(q, "written contact card", "written contact cards")
    );
    Sizing::new(
        &b,
        "contact_cards",
        "contact_card",
        q,
        "card",
        Per::Person,
        text,
    )
}

/// A paper map of the area. Rule `local_map`.
pub fn local_map() -> Sizing {
    let mut b = Basis::new();
    let q = b.k(keys::LOCAL_MAPS_PER_HOUSEHOLD);
    let text = format!(
        "{} of your area, marked with meeting places and ways out. Save offline maps on your phone too.",
        count(q, "paper map", "paper maps")
    );
    Sizing::new(&b, "local_map", "paper_map", q, "map", Per::Household, text)
}

/// Cash in small bills: $100 to start (an estimate; no agency gives a figure), or the household's
/// own daily spending for the communications target clamped to 3–14 days. Rule
/// `cash_reserve_usd`.
pub fn cash_reserve_usd(target_days: f64) -> Sizing {
    let mut b = Basis::new();
    let usd_default = b.k(keys::CASH_DEFAULT_USD);
    let lo = b.k(keys::CASH_DAYS_MIN);
    let hi = b.k(keys::CASH_DAYS_MAX);
    b.cite("fema_effak");
    let d = target_days.clamp(lo, hi);
    let text = format!(
        "Cash in small bills, kept with your documents, because ATMs and cards may not work in an outage: about {} to start, or enough for about {} of basics (food, fuel, medicine) at your own daily spending. No agency gives a dollar amount.",
        usd(usd_default),
        fmt_days(d)
    );
    Sizing::new(
        &b,
        "cash_reserve_usd",
        "cash",
        usd_default,
        "usd",
        Per::Household,
        text,
    )
    .days(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn philadelphia_comms() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        assert_eq!(noaa_radio().quantity, 1.0);
        let pb = phone_power_wh(3.0, &p.people).unwrap();
        assert_eq!(pb.quantity, 140.0); // 3 phones × 15 × 3 = 135
        assert!(pb.prior);
        assert_eq!(two_way_radios(&p.people).unwrap().quantity, 3.0);
        assert_eq!(contact_cards(&p.people).quantity, 4.0);
        let cash = cash_reserve_usd(1.4);
        assert_eq!(cash.quantity, 100.0);
        assert!(cash.prior && cash.plain.contains("$100") && cash.plain.contains("3 days"));
        assert!(cash_reserve_usd(60.0).plain.contains("14 days"));
    }

    #[test]
    fn one_person_needs_no_two_way_radios() {
        let p = fixtures::get("miami-condo-retiree-1").unwrap();
        assert!(two_way_radios(&p.people).is_none());
        let r = two_way_radios(&fixtures::get("coos-bay-well-owner-2").unwrap().people).unwrap();
        assert!(r.plain.contains("$35") && r.plain.contains("10 years"));
    }
}
