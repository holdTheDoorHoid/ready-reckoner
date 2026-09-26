//! Sections 6 to 8: the family plan (meeting places, contacts, school and work, leaving home,
//! the go/stay card, pets), documents and money (the Emergency Financial First Aid Kit,
//! insurance questions, cash, savings), and special needs (medicine, powered devices, babies,
//! older adults, mobility, pregnancy, animals, stress and mental health). Advice comes from the
//! catalogue's free steps (each a parent action with its sub-steps) and four topic blocks:
//! talking with children, neighbours, drills, and mental health. Where a topic block covers a
//! step, the step is listed by name only, so the packet does not say the same thing twice.

use rr_types::{AgeBand, BucketId, Mobility, PoweredDevice, Target};

use super::text::{self, md};
use super::{Ctx, cite, cite_all};

/// Advice paragraphs for catalogue items offered to this household, in the order given.
fn advice(cx: &Ctx<'_>, ids: &[&str], out: &mut Vec<String>) -> usize {
    let mut n = 0;
    for id in ids {
        if cx.a.offers.get(id).is_none() {
            continue;
        }
        if let Some(p) = cx.item_advice(id) {
            out.push(format!("- {p}"));
            n += 1;
        }
    }
    if n > 0 {
        out.push(String::new());
    }
    n
}

/// Steps offered to this household, by name only: the topic block that follows covers them.
fn named(cx: &Ctx<'_>, ids: &[&str], out: &mut Vec<String>) -> usize {
    let mut n = 0;
    for id in ids {
        if cx.a.offers.get(id).is_none() {
            continue;
        }
        if let Some(it) = cx.item(id) {
            out.push(format!("- **{}.**", md(&it.name)));
            n += 1;
        }
    }
    if n > 0 {
        out.push(String::new());
    }
    n
}

/// Requirement lines with these rules, as a list: the needs, and with `staged` also the staging
/// lines (a pet's water and food packed in its go-kit from household stock).
fn lines_with(cx: &Ctx<'_>, rules: &[&str], staged: bool, out: &mut Vec<String>) -> usize {
    let mut n = 0;
    for l in cx.a.lines.iter().filter(|l| {
        rules.contains(&l.line.rule.as_str())
            && (l.kind == rr_supply::LineKind::Need
                || (staged && l.kind == rr_supply::LineKind::Alternative && l.quantity > 0.0))
    }) {
        out.push(format!(
            "- {}{}",
            md(&l.line.plain),
            cite_all(&l.line.citations)
        ));
        n += 1;
    }
    if n > 0 {
        out.push(String::new());
    }
    n
}

/// Requirement lines (needs only) with these rules, as a list.
fn lines(cx: &Ctx<'_>, rules: &[&str], out: &mut Vec<String>) -> usize {
    lines_with(cx, rules, false, out)
}

/// A topic block under its own small heading, without its opening paragraph.
fn block(cx: &Ctx<'_>, target: &str, out: &mut Vec<String>) {
    if let Some(g) = cx.blocks_for(target).first() {
        out.push(format!("#### {}", md(&g.meta.title)));
        out.push(String::new());
        for para in super::topic_paragraphs(&cx.guidance(g, None, None)) {
            out.push(para);
            out.push(String::new());
        }
    }
}

pub(super) fn family(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let input = &a.input;
    let children = input.people.iter().any(|p| {
        matches!(
            p.age_band,
            AgeBand::Infant | AgeBand::Toddler | AgeBand::Child | AgeBand::Teen
        )
    });
    let pets = input.pets.dogs + input.pets.cats + input.pets.small + input.pets.large_animals > 0;
    let car = !input.mobility.vehicles.is_empty();
    out.push("## Family plan".to_owned());
    out.push(String::new());
    out.push(format!(
        "Fill this in together, and keep a copy in each go-bag and one on the fridge. Write \
         the numbers down: phones die.{}",
        cite("ready_gov_plan")
    ));
    out.push(String::new());
    out.push("| Plan | Your answer |".to_owned());
    out.push("| --- | --- |".to_owned());
    let mut rows: Vec<&str> = vec![
        "Meeting place near home",
        "Meeting place outside the neighborhood",
        "Out-of-area contact (name and phone)",
    ];
    if children {
        rows.push("Who picks up the children, and from where");
    }
    rows.push("Work and school plans");
    rows.push("Where we would stay if we had to leave");
    rows.push(if car {
        "Two routes out of the area"
    } else {
        "Who would drive us, or which bus or train leads out"
    });
    rows.push("Neighbors who check on us (names and phones)");
    if pets {
        rows.push("Who takes the animals if we cannot");
    }
    for r in rows {
        out.push(format!("| {r} | |"));
    }
    out.push(String::new());

    out.push("### Contacts and meeting places".to_owned());
    out.push(String::new());
    advice(cx, &["comms_contact_card"], out);
    // How to set up alerts is the phone-and-internet part of Your targets when that part prints
    // (alerts on every phone, a weather radio, when text-to-911 works); here it is named.
    if a.target_days(BucketId::Comms) > 0.0 {
        named(cx, &["comms_wea_alerts_on"], out);
    } else {
        advice(cx, &["comms_wea_alerts_on"], out);
    }

    out.push("### Leaving home: triggers and routes".to_owned());
    out.push(String::new());
    if let Target::Evacuate {
        p_need_10yr,
        notice_hours_low,
        notice_hours_high,
        days_away,
    } = a.bucket(BucketId::Evacuate).target
    {
        out.push(format!(
            "{} households like yours have to leave home quickly at least once in 10 years. \
             Warning can be {} ahead. Plan to be away for about {}.{}",
            text::upper_first(&text::households(p_need_10yr)),
            text::notice_range(f64::from(notice_hours_low), f64::from(notice_hours_high)),
            text::day_phrase(f64::from(days_away).max(1.0)),
            cite_all(&a.bucket(BucketId::Evacuate).sources)
        ));
        out.push(String::new());
    }
    advice(
        cx,
        &["evac_know_zone", "evac_ride_plan", "evac_half_tank"],
        out,
    );
    named(cx, &["evac_ten_minute_drills"], out);
    block(cx, "topic:drills", out);

    // School and work plans are rows of the table above, and each commuter's get-home bag is
    // under Checklists; what is left here is talking with children.
    if children {
        out.push("### Children".to_owned());
        out.push(String::new());
        block(cx, "topic:talking_with_children", out);
    }

    out.push("### Neighbors".to_owned());
    out.push(String::new());
    named(cx, &["community_know_two_neighbours"], out);
    block(cx, "topic:neighbours", out);

    if pets {
        out.push("### Pets".to_owned());
        out.push(String::new());
        advice(cx, &["special_pet_plan", "special_livestock_plan"], out);
    }
}

pub(super) fn documents(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    out.push("## Documents and money".to_owned());
    out.push(String::new());
    out.push(format!(
        "Keep paper copies in a waterproof pouch and photos you can reach from any phone. FEMA's \
         Emergency Financial First Aid Kit groups them in four parts.{}",
        cite("fema_effak")
    ));
    out.push(String::new());
    for part in [
        "**Who you are:** photo IDs, birth certificates, Social Security cards, passports, and \
         pet records.",
        "**Money and legal papers:** insurance policies, the lease or deed, bank and card \
         contact numbers (not PINs), and recent tax returns.",
        "**Medical papers:** insurance cards, the written medicine list, prescriptions, and \
         vaccination records.",
        "**Contacts:** family, doctors, the insurance agent, the landlord or lender, and \
         employers.",
    ] {
        out.push(format!("- [ ] {part}"));
    }
    out.push(String::new());
    advice(cx, &["docs_effak", "docs_document_pouch"], out);

    out.push("### Insurance questions".to_owned());
    out.push(String::new());
    let n = lines(
        cx,
        &[
            "insurance_home_or_renters",
            "insurance_flood",
            "insurance_earthquake",
        ],
        out,
    );
    if n == 0 {
        out.push(
            "You have the insurance the plan looks for. Check once a year that it still pays for \
             somewhere to stay if the home cannot be lived in."
                .to_owned(),
        );
        out.push(String::new());
    }

    out.push("### Cash".to_owned());
    out.push(String::new());
    lines(cx, &["cash_reserve_usd"], out);

    out.push("### Savings".to_owned());
    out.push(String::new());
    match &a.budget.plan.savings_track {
        Some(s) => {
            out.push(md(&s.why));
            out.push(String::new());
        }
        None => {
            out.push(
                "No one in the household earns wages, so the plan sets no income-gap goal. Keep \
                 a small cushion for emergencies if you can."
                    .to_owned(),
            );
            out.push(String::new());
        }
    }
    advice(cx, &["docs_start_emergency_fund"], out);
}

pub(super) fn special_needs(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let people = &a.input.people;
    out.push("## Special needs".to_owned());
    out.push(String::new());
    let mut any = false;

    let rx = people
        .iter()
        .any(|p| p.medical.daily_rx || p.medical.refrigerated_rx);
    let epi = people.iter().any(|p| p.medical.epinephrine);
    if rx || epi {
        any = true;
        out.push("### Medicine".to_owned());
        out.push(String::new());
        lines(
            cx,
            &["medication_days", "rx_cold_storage", "epinephrine_check"],
            out,
        );
        advice(
            cx,
            &[
                "med_list_written",
                "med_cooler_refrigerated_rx",
                "med_epinephrine_plan",
            ],
            out,
        );
    }
    out.push("### Antibiotics".to_owned());
    out.push(String::new());
    lines(cx, &["antibiotics_none"], out);

    if people
        .iter()
        .any(|p| p.medical.powered_device != PoweredDevice::None)
    {
        any = true;
        out.push("### Powered medical devices".to_owned());
        out.push(String::new());
        lines(
            cx,
            &[
                "medical_device_wh",
                "device_battery_units",
                "power_station_units",
            ],
            out,
        );
        advice(cx, &["med_device_power_plan"], out);
    }
    if people
        .iter()
        .any(|p| matches!(p.age_band, AgeBand::Infant | AgeBand::Toddler))
    {
        any = true;
        out.push("### Babies and toddlers".to_owned());
        out.push(String::new());
        lines(
            cx,
            &[
                "infant_formula_oz",
                "nursing_supplies",
                "diapers",
                "baby_wipes",
                "thermometer",
            ],
            out,
        );
        advice(cx, &["special_infant_go_kit"], out);
    }
    if people.iter().any(|p| p.age_band == AgeBand::Senior) {
        any = true;
        out.push("### Older adults".to_owned());
        out.push(String::new());
        advice(cx, &["special_older_adult_plan"], out);
    }
    if people.iter().any(|p| p.medical.mobility != Mobility::None) {
        any = true;
        out.push("### Getting around".to_owned());
        out.push(String::new());
        lines(
            cx,
            &["evacuation_assistance_plan", "wheelchair_battery"],
            out,
        );
        advice(cx, &["special_access_needs_plan"], out);
    }
    if people.iter().any(|p| p.pregnant_or_nursing) {
        any = true;
        out.push("### Pregnancy and nursing".to_owned());
        out.push(String::new());
        advice(cx, &["special_pregnancy_plan"], out);
    }
    let pets = &a.input.pets;
    if pets.dogs + pets.cats + pets.small + pets.large_animals > 0 {
        any = true;
        out.push("### Pets and animals".to_owned());
        out.push(String::new());
        // The go-kit's water and food are staged from household stock, so they are listed with
        // the carrier they are packed in.
        lines_with(
            cx,
            &[
                "pet_food_lb",
                "pet_carrier",
                "pet_go_water",
                "pet_go_food",
                "livestock_water",
            ],
            true,
            out,
        );
    }
    if !any {
        out.push(
            "Nobody in the household listed medical or access needs. Keep a written medicine \
             list anyway, and update this plan if that changes."
                .to_owned(),
        );
        out.push(String::new());
    }
    out.push("### Stress and mental health".to_owned());
    out.push(String::new());
    block(cx, "topic:mental_health", out);
}
