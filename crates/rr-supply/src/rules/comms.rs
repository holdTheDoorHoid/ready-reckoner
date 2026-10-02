//! Communications, information and cash (research §7, §8.1).

use rr_types::{Finances, Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::{constants, keys};
use crate::format::{DAYS_PER_MONTH, count, days as fmt_days, num, usd};
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
/// aged 13 and over at about 15 Wh a day (an estimate), for at most the two weeks of phone power the
/// plan stores (`battery_pack_cap_days`); past that, a way to recharge (the power bucket's
/// `recharge_capability`) keeps them going (round-2 review P-04: two power banks "completed" a
/// 45-day goal). Rule `phone_power_wh`.
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
    let capped = days > constants().value(keys::BATTERY_PACK_CAP_DAYS);
    let stored = if capped {
        b.k(keys::BATTERY_PACK_CAP_DAYS)
    } else {
        days
    };
    let q = phones * wh * stored;
    let mut text = format!(
        "Power banks: {} × about {} Wh a day ({} to {}) × {} without power = {} Wh. Text instead of calling to save battery.",
        count(phones, "phone", "phones"),
        num(wh, 0),
        num(lo, 0),
        num(hi, 0),
        fmt_days(stored),
        num(super::round_quantity("Wh", q), 0)
    );
    if capped {
        text.push_str(&format!(
            " Your power target is {}: past the first {}, recharge the power banks from the car or a solar panel (see the recharge line) instead of storing more.",
            fmt_days(days),
            fmt_days(stored)
        ));
    }
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
        .per_day(stored, phones * wh),
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
        "{} for the {} aged 13 and over, to reach each other when phones are down. FRS radios need no license; GMRS radios reach farther but need a {} FCC license that covers the family for {} years.",
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

/// Cash in small bills: half of three days of the household's own spending (estimates: the share
/// that goes on essentials, and the days), rounded down to $20 and never less than $100; $100 until
/// the household gives its monthly expenses (round-2 review P-18). No agency gives a dollar figure.
/// Rule `cash_reserve_usd`.
pub fn cash_reserve_usd(target_days: f64, finances: &Finances) -> Sizing {
    let mut b = Basis::new();
    let usd_default = b.k(keys::CASH_DEFAULT_USD);
    b.cite("fema_effak");
    let expenses = finances
        .monthly_expenses_usd
        .map(f64::from)
        .filter(|e| e.is_finite() && *e > 0.0);
    let (q, text) = match expenses {
        Some(month) => {
            let days = b.k(keys::CASH_EXPENSE_DAYS);
            let share = b.k(keys::CASH_ESSENTIAL_SHARE);
            let step = b.k(keys::CASH_ROUND_USD);
            let raw = share * days * month / DAYS_PER_MONTH;
            let rounded = (raw / step).floor() * step;
            let q = rounded.max(usd_default);
            let how = if rounded > usd_default {
                format!(
                    "about {}: half of {} of your usual spending ({} a month), the part that goes on food, fuel and medicine",
                    usd(q),
                    fmt_days(days),
                    usd(month)
                )
            } else {
                format!(
                    "about {}, at least; half of {} of your usual spending ({} a month) is less than that",
                    usd(q),
                    fmt_days(days),
                    usd(month)
                )
            };
            (
                q,
                format!(
                    "Cash in small bills, kept with your documents, because ATMs and cards may not work in an outage: {how}. No agency gives a dollar amount."
                ),
            )
        }
        None => {
            let lo = b.k(keys::CASH_DAYS_MIN);
            let hi = b.k(keys::CASH_DAYS_MAX);
            let d = target_days.clamp(lo, hi);
            (
                usd_default,
                format!(
                    "Cash in small bills, kept with your documents, because ATMs and cards may not work in an outage: about {} to start, or enough for about {} of basics (food, fuel, medicine) at your own daily spending. No agency gives a dollar amount.",
                    usd(usd_default),
                    fmt_days(d)
                ),
            )
        }
    };
    Sizing::new(
        &b,
        "cash_reserve_usd",
        "cash",
        q,
        "usd",
        Per::Household,
        text,
    )
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
        // A 45-day power target stores two weeks of phone power: 3 × 15 × 14 = 630 Wh.
        let long = phone_power_wh(45.0, &p.people).unwrap();
        assert_eq!(long.quantity, 630.0);
        assert_eq!(long.days, Some(14.0));
        assert!(long.plain.contains("recharge"), "{}", long.plain);
        assert_eq!(two_way_radios(&p.people).unwrap().quantity, 3.0);
        assert_eq!(contact_cards(&p.people).quantity, 4.0);
        // $4,200 a month: half of 3 days is $210, rounded down to $200 (round-2 review P-18).
        let cash = cash_reserve_usd(1.4, &p.finances);
        assert_eq!(cash.quantity, 200.0);
        assert!(cash.prior && cash.plain.contains("$200") && cash.plain.contains("$4,200"));
        // Without expenses: $100 to start, and the days of basics the target suggests.
        let mut none = p.finances.clone();
        none.monthly_expenses_usd = None;
        let cash = cash_reserve_usd(1.4, &none);
        assert_eq!(cash.quantity, 100.0);
        assert!(cash.plain.contains("$100") && cash.plain.contains("3 days"));
        assert!(cash_reserve_usd(60.0, &none).plain.contains("14 days"));
        // Sugar Land ($7,000) gets $340; the Chicago student ($1,400) keeps the $100 floor.
        let sl = fixtures::get("sugar-land-ev-household-3").unwrap();
        assert_eq!(cash_reserve_usd(3.0, &sl.finances).quantity, 340.0);
        let chicago = fixtures::get("chicago-student-zero-budget-1").unwrap();
        let c = cash_reserve_usd(3.0, &chicago.finances);
        assert_eq!(c.quantity, 100.0);
        assert!(c.plain.contains("at least"), "{}", c.plain);
    }

    #[test]
    fn one_person_needs_no_two_way_radios() {
        let p = fixtures::get("miami-condo-retiree-1").unwrap();
        assert!(two_way_radios(&p.people).is_none());
        let r = two_way_radios(&fixtures::get("coos-bay-well-owner-2").unwrap().people).unwrap();
        assert!(r.plain.contains("$35") && r.plain.contains("10 years"));
    }
}
