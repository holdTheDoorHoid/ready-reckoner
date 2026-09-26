//! Medication continuity (research §3.2–§3.4): prescriptions on hand, cold storage, the antibiotics
//! clinician card and epinephrine checks. No dosing, ever.

use rr_types::{BackupPower, Housing, Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::{constants, keys};
use crate::format::{count, days as fmt_days, num, people};

/// Days of prescription medicine to keep on hand for everyone who takes daily or refrigerated
/// medicine: the medication target clamped to 7–30 days, or 14 when there is no target.
/// Rule `medication_days`.
pub fn medication_days(target_days: Option<f64>, people_list: &[Person]) -> Option<Sizing> {
    let n = people_list
        .iter()
        .filter(|p| p.medical.daily_rx || p.medical.refrigerated_rx)
        .count();
    if n == 0 {
        return None;
    }
    let mut b = Basis::new();
    let default = b.k(keys::RX_DAYS_ON_HAND);
    let (floor, cap) = b.range(keys::RX_DAYS_ON_HAND);
    let refill = b.k(keys::RX_EMERGENCY_REFILL_DAYS);
    let days = target_days.map_or(default, |t| t.clamp(floor, cap));
    let q = n as f64 * days;
    let who = if n == 1 {
        "1 person takes prescription medicine every day".to_owned()
    } else {
        format!("{} take prescription medicine every day", people(n as f64))
    };
    let each = if n == 1 {
        String::new()
    } else {
        format!(" each ({} person-days)", num(q, 1))
    };
    let mut text = format!("{who}: keep {} of it on hand{each}.", fmt_days(days));
    match target_days {
        None => text.push_str(&format!(
            " {} is the usual advice (Florida, CDC); the Red Cross says at least {}.",
            fmt_days(default),
            fmt_days(floor)
        )),
        Some(t) if t < floor => text.push_str(&format!(
            " Your risk alone would suggest {}, but agencies advise at least {}.",
            fmt_days(t),
            fmt_days(floor)
        )),
        Some(t) if t > cap => text.push_str(&format!(
            " Your target is {}; beyond {}, ask your prescriber or insurer about a longer fill.",
            fmt_days(t),
            fmt_days(cap)
        )),
        Some(_) => {}
    }
    text.push_str(&format!(
        " Keep a written list of each medicine, what it is for and the dose. After a disaster declaration some states let pharmacies give an emergency refill of up to {}, and many allow only a few days; ask your pharmacist what yours allows.",
        fmt_days(refill)
    ));
    let math = vec![format!(
        "{n} × clamp({}, {}, {}) = {} person-days",
        target_days.map_or("none".to_owned(), |t| num(t, 2)),
        num(floor, 0),
        num(cap, 0),
        num(q, 1)
    )];
    Some(
        Sizing::new(
            &b,
            "medication_days",
            "prescription_medicine",
            q,
            "person_day",
            Per::Person,
            text,
        )
        .per_day(days, n as f64)
        .math(math),
    )
}

/// Whether refrigerated medicine needs a power source through the outage, not only a cooler bag:
/// someone takes it, the power target is at least `rx_power_min_days` (2 days, an estimate), and the
/// household has no backup power of its own (a generator, a power station, or solar with a
/// battery). The power bucket's `power_station_units` line is then a need, and the cold-storage
/// line counts the whole power target.
pub fn rx_power_needed(power_days: Option<f64>, people_list: &[Person], housing: &Housing) -> bool {
    let min = constants().value(keys::RX_POWER_MIN_DAYS);
    people_list.iter().any(|p| p.medical.refrigerated_rx)
        && housing.backup_power == BackupPower::None
        && power_days.is_some_and(|d| d.is_finite() && d >= min)
}

/// Days of cold storage for refrigerated medicine through a power outage. An insulated bag with
/// fresh cold packs keeps medicine cool for about a day (`cooler_hold_days`, an estimate); insulin
/// keeps working for up to 28 days between 59 °F and 86 °F (FDA), so the job is keeping it in the
/// shade and below 86 °F, never frozen. When a power source is needed ([`rx_power_needed`]) the
/// line counts the whole power target, which the bag covers only a day of; otherwise (a short
/// power target, or backup power the household already has) it counts the bag's day. `cold_days`
/// is the power target (the medication target when there is none). Rule `rx_cold_storage`.
pub fn rx_cold_storage(
    cold_days: f64,
    power_days: Option<f64>,
    people_list: &[Person],
    housing: &Housing,
) -> Option<Sizing> {
    let n = people_list
        .iter()
        .filter(|p| p.medical.refrigerated_rx)
        .count();
    if n == 0 || cold_days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let hold = b.k(keys::COOLER_HOLD_DAYS);
    let insulin_days = b.k(keys::INSULIN_ROOM_TEMP_DAYS);
    let low_f = b.k(keys::INSULIN_ROOM_TEMP_MIN_F);
    let high_f = b.k(keys::INSULIN_ROOM_TEMP_MAX_F);
    b.cite("cdc_insulin_emergency");
    let needed = rx_power_needed(power_days, people_list, housing);
    let days = if needed {
        cold_days
    } else {
        cold_days.min(hold)
    };
    let who = if n == 1 {
        "1 person needs".to_owned()
    } else {
        format!("{} need", people(n as f64))
    };
    let mut text = format!(
        "{who} medicine kept cold, such as insulin. An insulated bag with fresh cold packs keeps it cool, never frozen, for about {}. Insulin in its vial or pen keeps working up to {} between {} °F and {} °F, so in a longer power cut keep it in the shade and below {} °F; never use insulin that froze. For other medicines, ask your pharmacist now how long they keep out of the fridge and write it on your medicine list.",
        fmt_days(hold),
        fmt_days(insulin_days),
        num(low_f, 0),
        num(high_f, 0),
        num(high_f, 0)
    );
    let mut math = Vec::new();
    if needed {
        let min = b.k(keys::RX_POWER_MIN_DAYS);
        text.push_str(&format!(
            " Your power target is {}: plan a battery power station to run a small 12-volt cooler or the fridge (see its line), and a place with power you could go to.",
            fmt_days(cold_days)
        ));
        math.push(format!(
            "power target {} days ≥ {} days, no backup power: cold storage for all {} days (the bag covers {})",
            num(cold_days, 2),
            num(min, 0),
            num(cold_days, 2),
            num(hold, 1)
        ));
    } else {
        match housing.backup_power {
            BackupPower::Generator => text.push_str(
                " Your generator can keep the fridge or a small cooler running; keep fuel for it.",
            ),
            BackupPower::PowerStation => text.push_str(
                " Your power station can keep a small 12-volt cooler or the fridge running for a while; keep it charged.",
            ),
            BackupPower::SolarBattery => text.push_str(
                " Your solar panels and battery can keep the fridge or a small cooler running; check how long they last on cloudy days.",
            ),
            BackupPower::None if cold_days > hold => {
                let min = b.k(keys::RX_POWER_MIN_DAYS);
                text.push_str(&format!(
                    " For a power cut of up to {}, replace the cold packs with ice or frozen water bottles.",
                    fmt_days(min)
                ));
            }
            BackupPower::None => {}
        }
        math.push(format!(
            "cooler bag: min({} days, {} days) = {} days",
            num(cold_days, 2),
            num(hold, 1),
            num(days, 2)
        ));
    }
    Some(
        Sizing::new(
            &b,
            "rx_cold_storage",
            "medicine_cooler",
            days,
            "day",
            Per::Household,
            text,
        )
        .days(days)
        .math(math),
    )
}

/// No antibiotic quantity is ever sized; the line points to the clinician card (research §3.4).
/// Rule `antibiotics_none`.
pub fn antibiotics_none() -> Sizing {
    let mut b = Basis::new();
    let q = b.k(keys::ANTIBIOTIC_QUANTITY);
    let text = "No antibiotics are included in this plan. If you have a specific, foreseeable need, such as remote travel, talk to your own clinician about a standby prescription with written instructions. Never use fish or pet antibiotics, never share prescriptions, and don't use expired ones.".to_owned();
    Sizing::new(
        &b,
        "antibiotics_none",
        "antibiotics_clinician_card",
        q,
        "course",
        Per::Household,
        text,
    )
}

/// A date check for each person who carries an epinephrine auto-injector. Rule
/// `epinephrine_check`.
pub fn epinephrine_check(people_list: &[Person]) -> Option<Sizing> {
    let n = people_list.iter().filter(|p| p.medical.epinephrine).count();
    if n == 0 {
        return None;
    }
    let mut b = Basis::new();
    b.cite("ready_gov_disability");
    let text = format!(
        "{} an epinephrine auto-injector: check the expiry date, and ask the prescriber how many to keep at home, in the go-bag and on the person.",
        if n == 1 {
            "1 person carries".to_owned()
        } else {
            format!("{} carry", count(n as f64, "person", "people"))
        }
    );
    Some(Sizing::new(
        &b,
        "epinephrine_check",
        "epinephrine_auto_injector",
        n as f64,
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
    fn philadelphia_senior_keeps_fourteen_days() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        let s = medication_days(Some(14.0), &p.people).unwrap();
        assert_eq!(s.quantity, 14.0);
        assert_eq!(s.unit, "person_day");
        assert!(
            s.plain.starts_with(
                "1 person takes prescription medicine every day: keep 14 days of it on hand."
            ),
            "{}",
            s.plain
        );
        for id in [
            "florida_dem_medication",
            "cdc_diabetes_emergencies",
            "redcross_survival_kit",
            "healthcare_ready_refill_laws",
        ] {
            assert!(s.citations.iter().any(|c| c == id), "{id}");
        }
    }

    #[test]
    fn reserve_is_clamped_to_seven_to_thirty_days() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        assert_eq!(medication_days(Some(2.0), &p.people).unwrap().quantity, 7.0);
        assert_eq!(
            medication_days(Some(71.0), &p.people).unwrap().quantity,
            30.0
        );
        assert_eq!(medication_days(None, &p.people).unwrap().quantity, 14.0);
        let chicago = fixtures::get("chicago-student-zero-budget-1").unwrap();
        assert!(medication_days(Some(14.0), &chicago.people).is_none());
    }

    #[test]
    fn insulin_counts_and_needs_cold_storage() {
        let p = fixtures::get("sugar-land-ev-household-3").unwrap();
        assert_eq!(
            medication_days(Some(21.0), &p.people).unwrap().quantity,
            21.0
        );
        // A 5-day power target and no backup power: the line counts all 5 days, which the bag
        // covers only one of, so the power station line is a need.
        assert!(rx_power_needed(Some(5.0), &p.people, &p.housing));
        let cold = rx_cold_storage(5.0, Some(5.0), &p.people, &p.housing).unwrap();
        assert_eq!(cold.quantity, 5.0);
        assert!(cold.plain.contains("never frozen"));
        assert!(
            cold.plain.contains("up to 28 days between 59 °F and 86 °F"),
            "{}",
            cold.plain
        );
        assert!(cold.plain.contains("for about 1 day"), "{}", cold.plain);
        assert!(
            cold.plain.contains("battery power station"),
            "{}",
            cold.plain
        );
        assert!(cold.plain.contains("ask your pharmacist"), "{}", cold.plain);
        // Never the one-day discard rule (round-2 review S1).
        let lower = cold.plain.to_lowercase();
        assert!(
            !lower.contains("throw") && !lower.contains("discard"),
            "{lower}"
        );
        for id in [
            "fda_insulin_emergency",
            "cdc_insulin_emergency",
            "rr_expert_prior",
        ] {
            assert!(cold.citations.iter().any(|c| c == id), "{id}");
        }
        assert!(
            !cold
                .citations
                .iter()
                .any(|c| c == "ready_gov_power_outages")
        );
    }

    #[test]
    fn a_cooler_bag_counts_one_day_unless_a_power_source_is_needed() {
        let p = fixtures::get("sugar-land-ev-household-3").unwrap();
        // Under 2 days of power target the bag (with ice replaced) is the plan: 1 day counted.
        assert!(!rx_power_needed(Some(1.5), &p.people, &p.housing));
        let short = rx_cold_storage(1.5, Some(1.5), &p.people, &p.housing).unwrap();
        assert_eq!(short.quantity, 1.0);
        assert!(
            short.plain.contains("replace the cold packs with ice"),
            "{}",
            short.plain
        );
        // A generator the household owns keeps the fridge running: the bag's day, no station.
        let mut owns = p.clone();
        owns.housing.backup_power = BackupPower::Generator;
        assert!(!rx_power_needed(Some(5.0), &owns.people, &owns.housing));
        let g = rx_cold_storage(5.0, Some(5.0), &owns.people, &owns.housing).unwrap();
        assert_eq!(g.quantity, 1.0);
        assert!(g.plain.contains("Your generator"), "{}", g.plain);
        // An electric car is not counted as a power source: the engine cannot tell whether it can
        // power a cooler (a v0.2.0 input), so Sugar Land still needs a station.
        assert!(
            owns.mobility
                .vehicles
                .iter()
                .any(|v| v.fuel == rr_types::Fuel::Ev)
        );
        // No power target: the medication target stands in, and only the bag's day is counted.
        assert!(!rx_power_needed(None, &p.people, &p.housing));
        assert_eq!(
            rx_cold_storage(10.0, None, &p.people, &p.housing)
                .unwrap()
                .quantity,
            1.0
        );
        // Nobody on refrigerated medicine: no line.
        let philly = fixtures::get("philadelphia-renters-4").unwrap();
        assert!(rx_cold_storage(3.0, Some(3.0), &philly.people, &philly.housing).is_none());
        assert!(!rx_power_needed(Some(3.0), &philly.people, &philly.housing));
    }

    #[test]
    fn antibiotics_are_always_zero_and_cited() {
        let s = antibiotics_none();
        assert_eq!(s.quantity, 0.0);
        assert!(s.citations.iter().any(|c| c == "cdc_antibiotic_use"));
        assert!(
            s.citations
                .iter()
                .any(|c| c == "fda_fish_antibiotics_warning_2023")
        );
        let lower = s.plain.to_lowercase();
        assert!(lower.contains("clinician") && lower.contains("fish"));
        for dose in ["mg", "tablet", "every 8 hours", "twice a day"] {
            assert!(!lower.contains(dose), "no dosing: {dose}");
        }
    }
}
