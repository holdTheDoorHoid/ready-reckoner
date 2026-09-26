//! Medication continuity (research §3.2–§3.4): prescriptions on hand, cold storage, the antibiotics
//! clinician card and epinephrine checks. No dosing, ever.

use rr_types::{Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
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
        " Keep a written list of each medicine, what it is for and the dose. After a disaster declaration many states let pharmacies give an emergency refill of up to {}; ask your pharmacist what yours allows.",
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

/// Days of cold storage for refrigerated medicine through a power outage (Ready.gov, CDC).
/// Rule `rx_cold_storage`.
pub fn rx_cold_storage(power_days: f64, people_list: &[Person]) -> Option<Sizing> {
    let n = people_list
        .iter()
        .filter(|p| p.medical.refrigerated_rx)
        .count();
    if n == 0 || power_days <= 0.0 {
        return None;
    }
    let mut b = Basis::new();
    let hold = b.k(keys::REFRIGERATED_RX_HOLD_DAYS);
    b.cite("cdc_insulin_emergency");
    let text = format!(
        "{} needs medicine kept cold, such as insulin: plan to keep it cool but never frozen for up to {} without power, with an insulated cooler and cold packs or ice you can replace. After {} without power, throw refrigerated medicine out unless its label says otherwise; ask your pharmacist how long yours can stay warm.",
        if n == 1 {
            "1 person".to_owned()
        } else {
            people(n as f64)
        },
        fmt_days(power_days),
        fmt_days(hold)
    );
    Some(
        Sizing::new(
            &b,
            "rx_cold_storage",
            "medicine_cooler",
            power_days,
            "day",
            Per::Household,
            text,
        )
        .days(power_days),
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
            "cdc_diabetes_emergency",
            "red_cross_survival_kit",
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
        let cold = rx_cold_storage(5.0, &p.people).unwrap();
        assert_eq!(cold.quantity, 5.0);
        assert!(cold.plain.contains("never frozen"));
        assert!(
            cold.citations
                .iter()
                .any(|c| c == "ready_gov_power_outages")
        );
    }

    #[test]
    fn antibiotics_are_always_zero_and_cited() {
        let s = antibiotics_none();
        assert_eq!(s.quantity, 0.0);
        assert!(s.citations.iter().any(|c| c == "cdc_antibiotics_aware"));
        assert!(s.citations.iter().any(|c| c == "fda_fish_antibiotics"));
        let lower = s.plain.to_lowercase();
        assert!(lower.contains("clinician") && lower.contains("fish"));
        for dose in ["mg", "tablet", "every 8 hours", "twice a day"] {
            assert!(!lower.contains(dose), "no dosing: {dose}");
        }
    }
}
