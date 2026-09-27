//! Documents and money (the Emergency Financial First Aid Kit, the plan's decisions with the
//! household's own insurance lines, cash, savings, and what a damaged home costs) and special
//! needs (medicine, powered devices, babies, older adults, mobility, pregnancy, animals, stress
//! and mental health). Advice comes from the catalogue (free steps, decision items), the
//! requirement lines and the mental-health topic block. The family plan is `family.rs`.

use rr_types::{AgeBand, Mobility, PoweredDevice};

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
            md(&text::without_source_names(&l.line.plain)),
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
/// A topic block as its own subsection: the block's title as the `###` heading (one heading, not
/// a section heading over a topic heading that says the same), then its paragraphs.
fn block(cx: &Ctx<'_>, target: &str, out: &mut Vec<String>) {
    if let Some(g) = cx.blocks_for(target).first() {
        out.push(format!("### {}", md(&g.meta.title)));
        out.push(String::new());
        for para in super::topic_paragraphs(&cx.guidance(g, None, None)) {
            out.push(para);
            out.push(String::new());
        }
    }
}

/// A decision's name without its "Decide:" opening, lower case, for a list.
fn decision_name(name: &str) -> String {
    text::lower_first(name.strip_prefix("Decide: ").unwrap_or(name))
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
        "**Money and legal papers:** insurance policies, the lease or deed, a will and powers of \
         attorney, bank and card contact numbers (not PINs), and recent tax returns.",
        "**Medical papers:** insurance cards, the written medicine list, prescriptions, and \
         vaccination records.",
        "**Contacts:** family, doctors, the insurance agent, the landlord or lender, and \
         employers.",
    ] {
        out.push(format!("- [ ] {part}"));
    }
    out.push(String::new());

    // The plan's decisions (insurance, ID for every person, home repairs; `PlanItem::decision`):
    // an insurance decision says it in the household's own words (its requirement line), the
    // others in the catalogue's.
    let decisions: Vec<&rr_types::PlanItem> = cx
        .steps()
        .into_iter()
        .map(|(_, i)| i)
        .filter(|i| i.decision && !i.done)
        .collect();
    let insurance_lines: Vec<&rr_supply::SizedLine> = a
        .lines
        .iter()
        .filter(|l| l.kind == rr_supply::LineKind::Need && l.line.rule.starts_with("insurance_"))
        .collect();
    if !decisions.is_empty() || !insurance_lines.is_empty() {
        out.push("### Decisions".to_owned());
        out.push(String::new());
        let mut used: Vec<&str> = Vec::new();
        // Home repairs (a stronger roof, a safe room, a backflow valve, a retrofit, wildfire
        // hardening) are decisions for when the time comes: one line, their names, and where
        // grants and discounts are asked for (their specs, with the grant details, are the app's).
        let repair = |d: &rr_types::PlanItem| {
            cx.item(d.item_id.as_str())
                .is_some_and(|it| it.quantity_rule.starts_with("once_if_owned"))
        };
        let repairs: Vec<&rr_types::PlanItem> =
            decisions.iter().copied().filter(|d| repair(d)).collect();
        for d in decisions.iter().filter(|d| !repair(d)) {
            let item = cx.item(d.item_id.as_str());
            let line = item.and_then(|it| {
                insurance_lines
                    .iter()
                    .find(|l| l.line.rule == it.quantity_rule)
            });
            let (text, cites) = match (line, item) {
                (Some(l), _) => {
                    used.push(l.line.id.as_str());
                    (
                        md(&text::without_source_names(&l.line.plain)),
                        cite_all(&l.line.citations),
                    )
                }
                (None, Some(it)) => (it.spec.clone(), cite_all(&it.citations)),
                (None, None) => continue,
            };
            out.push(format!("- [ ] **{}.** {text}{cites}", md(&d.name)));
        }
        if !repairs.is_empty() {
            let names: Vec<String> = repairs.iter().map(|d| decision_name(&d.name)).collect();
            out.push(format!(
                "- [ ] **Home repairs to weigh when the time comes:** {}. Some come with grants or \
                 insurance discounts: ask your state emergency management office and your \
                 insurer.{}",
                md(&names.join("; ")),
                cite_all(
                    repairs
                        .iter()
                        .filter_map(|d| cx.item(d.item_id.as_str()))
                        .flat_map(|it| it.citations.iter())
                )
            ));
        }
        // Insurance lines with no decision in the plan (the plan already has the cover).
        for l in insurance_lines
            .iter()
            .filter(|l| !used.contains(&l.line.id.as_str()))
        {
            out.push(format!(
                "- {}{}",
                md(&text::without_source_names(&l.line.plain)),
                cite_all(&l.line.citations)
            ));
        }
        out.push(String::new());
    }

    out.push("### Cash".to_owned());
    out.push(String::new());
    lines(cx, &["cash_reserve_usd"], out);

    out.push("### Savings".to_owned());
    out.push(String::new());
    match &a.budget.plan.savings_track {
        Some(s) => {
            out.push(format!("{}{}", md(&s.why), cite_all(&a.budget.citations)));
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
    if let Some(m) = &a.budget.plan.first_milestone {
        let date = text::month_year(text::month_start(a.input.planning_date, m.by_month));
        out.push(format!(
            "**A first step:** {} ({}) by month {} ({date}).",
            text::usd(f64::from(m.usd)),
            text::months_phrase(f64::from(m.months)),
            m.by_month
        ));
        out.push(String::new());
    }

    // If damage forced the household out: how long, and what living elsewhere costs (the
    // consequence model's home-loss line, model review M-08).
    let home = a.bucket(rr_types::BucketId::HomeLoss);
    if let Some(sentence) = home
        .frequency_sentences
        .iter()
        .find(|s| s.starts_with("If damage forced you out"))
    {
        out.push("### If damage forces you out".to_owned());
        out.push(String::new());
        out.push(format!("{}{}", md(sentence), cite_all(&home.sources)));
        out.push(String::new());
    }
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
        // The medicine line already says to keep a written list and how refills work (the
        // `med_list_written` step is in the plan by name).
        // The cold-storage line already gives the cooler and insulin's temperatures.
        let cold_line = a
            .lines
            .iter()
            .any(|l| l.line.rule == "rx_cold_storage" && l.kind == rr_supply::LineKind::Need);
        if !cold_line {
            advice(cx, &["med_cooler_refrigerated_rx"], out);
        }
        advice(cx, &["med_epinephrine_plan"], out);
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
    // Older adults: the plan's own step (`special_older_adult_plan`), unless the household's
    // getting-around lines below already cover leaving with help.
    if people.iter().any(|p| p.age_band == AgeBand::Senior)
        && people.iter().all(|p| p.medical.mobility == Mobility::None)
    {
        any = true;
        out.push("### Older adults".to_owned());
        out.push(String::new());
        advice(cx, &["special_older_adult_plan"], out);
    }
    if people.iter().any(|p| p.medical.mobility != Mobility::None) {
        any = true;
        out.push("### Getting around".to_owned());
        out.push(String::new());
        // The line says what the access-needs step does; the step is in the plan by name.
        lines(
            cx,
            &["evacuation_assistance_plan", "wheelchair_battery"],
            out,
        );
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
        // The go-kit's water and food come from household stock (the carrier's line says so;
        // the staging steps are in the plan by name).
        lines(cx, &["pet_food_lb", "pet_carrier", "livestock_water"], out);
    }
    if !any {
        out.push(
            "Nobody in the household listed medical or access needs. Keep a written medicine \
             list anyway, and update this plan if that changes."
                .to_owned(),
        );
        out.push(String::new());
    }
    // "Stress, mental health and the 988 line" (`topic:mental_health`), always.
    block(cx, "topic:mental_health", out);
}
