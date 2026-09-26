//! First aid (research §3.1, §3.5–§3.8): the family kit, non-prescription medicines, masks, a
//! thermometer and oral rehydration salts.

use rr_types::{AgeBand, Per, Person};

use super::Sizing;
use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{ceil_count, count, days as fmt_days, num};
use crate::household::is_4_plus;

/// One family first-aid kit (the Red Cross list for four people) per four people. Rule
/// `first_aid_kit`.
pub fn first_aid_kit(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let per_kit = b.k(keys::FIRST_AID_PEOPLE_PER_KIT);
    let n = people_list.len().max(1) as f64;
    let kits = ceil_count(n / per_kit).max(1.0);
    let text = format!(
        "{} for {}: the Red Cross list for a family of {} (bandages, gauze, tape, antiseptic wipes, gloves, a breathing barrier, a cold pack, tweezers, an emergency blanket and a thermometer). Add your own medicines and emergency numbers, and replace what you use.",
        count(kits, "family first-aid kit", "family first-aid kits"),
        count(n, "person", "people"),
        num(per_kit, 0)
    );
    Sizing::new(
        &b,
        "first_aid_kit",
        "first_aid_kit",
        kits,
        "kit",
        Per::Household,
        text,
    )
}

/// Non-prescription medicines: one package of each of four kinds per two weeks for four people,
/// scaled by days and household size (an estimate). Rule `otc_medicines`.
pub fn otc_medicines(days: f64, people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let kinds = b.k(keys::OTC_KINDS);
    let per_package = b.k(keys::OTC_DAYS_PER_PACKAGE);
    let per_kit = b.k(keys::FIRST_AID_PEOPLE_PER_KIT);
    let n = people_list.len().max(1) as f64;
    let each = ceil_count((days.max(0.0) / per_package * n / per_kit).max(1.0));
    let q = kinds * each;
    let text = format!(
        "{} of each of {} kinds of non-prescription medicine (a pain reliever, an anti-diarrhea medicine, an antacid and a laxative): one package of each lasts about {} for {} people. That is {} for {}. Follow the label, and ask a pharmacist about children's versions.",
        count(each, "package", "packages"),
        num(kinds, 0),
        fmt_days(per_package),
        num(per_kit, 0),
        count(q, "package", "packages"),
        fmt_days(days)
    );
    Sizing::new(
        &b,
        "otc_medicines",
        "otc_medicines",
        q,
        "package",
        Per::Household,
        text,
    )
}

/// N95 masks for smoke or an outbreak: one a day per person aged 4 and over, for five days (an
/// estimate; no agency publishes a count). Rule `n95_masks`.
pub fn n95_masks(people_list: &[Person]) -> Option<Sizing> {
    let n = people_list.iter().filter(|p| is_4_plus(p)).count();
    if n == 0 {
        return None;
    }
    let mut b = Basis::new();
    let per_day = b.k(keys::N95_PER_PERSON_DAY);
    let days = b.k(keys::N95_DAYS);
    b.cite("ready_gov_kit");
    let q = n as f64 * per_day * days;
    let text = format!(
        "N95 masks for wildfire smoke or an outbreak: {} a day for each of the {} aged 4 and over, for {} = {} masks. A well-fitting N95 protects more than a cloth or surgical mask.",
        num(per_day, 0),
        count(n as f64, "person", "people"),
        fmt_days(days),
        num(q, 0)
    );
    Some(Sizing::new(
        &b,
        "n95_masks",
        "n95_mask",
        q,
        "mask",
        Per::Person,
        text,
    ))
}

/// An oral thermometer, and an infant thermometer when there is a baby. Rule `thermometer`.
pub fn thermometer(people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let base = b.k(keys::THERMOMETERS_PER_HOUSEHOLD);
    let baby = people_list.iter().any(|p| p.age_band == AgeBand::Infant);
    let q = if baby {
        b.cite("cdc_infant_emergency_checklist");
        base + 1.0
    } else {
        base
    };
    let text = if baby {
        format!(
            "{} oral thermometer (non-mercury, non-glass; it is on the Red Cross kit list) and 1 infant thermometer for the baby.",
            num(base, 0)
        )
    } else {
        format!(
            "{} oral thermometer (non-mercury, non-glass; it is on the Red Cross kit list).",
            num(base, 0)
        )
    };
    Sizing::new(
        &b,
        "thermometer",
        "thermometer",
        q,
        "thermometer",
        Per::Household,
        text,
    )
}

/// Oral rehydration salts: three packets per person per two weeks (an estimate), each mixed into a
/// litre of safe water. Rule `ors_packets`.
pub fn ors_packets(days: f64, people_list: &[Person]) -> Sizing {
    let mut b = Basis::new();
    let per_2wk = b.k(keys::ORS_PACKETS_PER_PERSON_2WK);
    let litres = b.k(keys::ORS_L_PER_PACKET);
    let n = people_list.len().max(1) as f64;
    let q = ceil_count(per_2wk * n * (days.max(0.0) / 14.0).max(1.0));
    let text = format!(
        "Oral rehydration salts for vomiting, diarrhea or heat illness: {} a person for up to 2 weeks = {} packets. Mix each packet into {} of safe water.",
        num(per_2wk, 0),
        num(q, 0),
        count(litres, "litre", "litres")
    );
    Sizing::new(
        &b,
        "ors_packets",
        "oral_rehydration_salts",
        q,
        "packet",
        Per::Person,
        text,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    #[test]
    fn philadelphia_first_aid() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        assert_eq!(first_aid_kit(&p.people).quantity, 1.0);
        assert_eq!(otc_medicines(10.0, &p.people).quantity, 4.0);
        assert_eq!(n95_masks(&p.people).unwrap().quantity, 20.0);
        assert_eq!(thermometer(&p.people).quantity, 1.0);
        assert_eq!(ors_packets(10.0, &p.people).quantity, 12.0);
    }

    #[test]
    fn five_people_get_two_kits_and_toddlers_no_masks() {
        let p = fixtures::get("hays-kansas-farm-5").unwrap();
        assert_eq!(first_aid_kit(&p.people).quantity, 2.0);
        // 4 people aged 4+ (the toddler is left out) × 1 × 5
        assert_eq!(n95_masks(&p.people).unwrap().quantity, 20.0);
        let sl = fixtures::get("sugar-land-ev-household-3").unwrap();
        assert_eq!(thermometer(&sl.people).quantity, 2.0);
    }

    #[test]
    fn longer_targets_scale_otc_and_ors() {
        let p = fixtures::get("philadelphia-renters-4").unwrap();
        // 28 days / 14 × 4 / 4 = 2 packages of each kind
        assert_eq!(otc_medicines(28.0, &p.people).quantity, 8.0);
        assert_eq!(ors_packets(28.0, &p.people).quantity, 24.0);
        assert!(otc_medicines(28.0, &p.people).prior);
    }
}
