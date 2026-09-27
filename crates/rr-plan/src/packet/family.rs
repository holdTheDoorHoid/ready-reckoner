//! Sections 2 and 3: the household's own family plan and its wallet cards (packet v2,
//! DESIGN-DELTA §3; review RR-P05). Everything the household wrote on the family-plan screen
//! (`PlanInput::family_plan`) is echoed word for word; nothing in it is computed with. A field
//! left empty prints as a line to write on, never as "not answered", so the page works on paper
//! too. Then the evacuation numbers from the consequence model ("Leaving home") and the
//! communication plan's four ways to reach each other (`plan:communication`, "Staying in
//! touch"), with its school and child-care paragraph only for households with children.
//!
//! The wallet cards are one block quote per household member under [`WALLET_CARDS_HEADING`]: the
//! web app finds the section by that heading and prints each block quote as a card to cut out.
//! Phone numbers carry non-breaking hyphens ([`NB_HYPHEN`]) so a number never breaks across two
//! lines of a card.

use rr_types::{AgeBand, BucketId, Contact, FamilyPlan, Heating, Holds, PlanInput};

use super::text::md;
use super::{Ctx, cite, cite_all};

/// The heading the web app finds the wallet cards by (`web/src/lib/markdown.ts`,
/// `cardsSection`).
pub const WALLET_CARDS_HEADING: &str = "## Wallet cards";

/// A non-breaking hyphen (U+2011): phone numbers on the cards use it so they never break across
/// lines.
pub const NB_HYPHEN: char = '\u{2011}';

/// A line to write on. Underscores alone are not a word, and the Markdown renderers print them
/// as they are.
const BLANK: &str = "__________";

/// The words of a phone number (or any text holding one), escaped for Markdown, with every
/// hyphen between two digits made non-breaking.
fn phone(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 4);
    for (i, c) in chars.iter().enumerate() {
        let between_digits = *c == '-'
            && i > 0
            && chars[i - 1].is_ascii_digit()
            && chars.get(i + 1).is_some_and(char::is_ascii_digit);
        out.push(if between_digits { NB_HYPHEN } else { *c });
    }
    md(&out)
}

/// A contact as written: "name, phone", either alone, or nothing.
fn contact(c: Option<&Contact>) -> Option<String> {
    let c = c?;
    let parts: Vec<String> = [c.name.as_deref().map(md), c.phone.as_deref().map(phone)]
        .into_iter()
        .flatten()
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// The household's text, escaped, or a line to write on.
fn or_blank(text: Option<&str>) -> String {
    text.map_or_else(|| BLANK.to_owned(), md)
}

/// The trusted circle on one line: "Tanya 555‑0140; Mrs. Owens 555‑0141".
fn circle_line(plan: &FamilyPlan) -> Option<String> {
    let people: Vec<String> = plan
        .trusted_circle
        .iter()
        .map(|p| {
            [p.name.as_deref().map(md), p.phone.as_deref().map(phone)]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|s| !s.is_empty())
        .collect();
    (!people.is_empty()).then(|| people.join("; "))
}

/// The numbers to know by heart on one line.
fn numbers_line(plan: &FamilyPlan) -> Option<String> {
    let n: Vec<String> = plan.numbers_by_heart.iter().map(|x| phone(x)).collect();
    (!n.is_empty()).then(|| n.join("; "))
}

/// What a trusted person holds, in words.
fn holds_words(h: Holds) -> &'static str {
    match h {
        Holds::SpareKey => "a spare key",
        Holds::Documents => "copies of documents",
        Holds::MedicalPoa => "medical power of attorney",
        Holds::BackupCodes => "backup codes",
    }
}

/// Whether a family plan holds a v2 answer, anything besides the contract v3 groups.
// transitional: replaced by the binder workstream
fn has_v2_answers(plan: &FamilyPlan) -> bool {
    let v2 = FamilyPlan {
        home: None,
        neighbourhood: None,
        pets: Vec::new(),
        vehicles: Vec::new(),
        documents: None,
        ..plan.clone()
    };
    !v2.is_empty()
}

fn has_children(input: &PlanInput) -> bool {
    input.people.iter().any(|p| {
        matches!(
            p.age_band,
            AgeBand::Infant | AgeBand::Toddler | AgeBand::Child | AgeBand::Teen
        )
    })
}

fn has_animals(input: &PlanInput) -> bool {
    let p = &input.pets;
    p.dogs + p.cats + p.small + p.large_animals > 0
}

/// The home has gas to shut off: gas or propane heat, or a gas stove.
fn has_gas(input: &PlanInput) -> bool {
    matches!(input.housing.heating, Heating::Gas | Heating::Propane)
        || input.housing.cooking == Some(rr_types::CookingFuel::Gas)
}

/// "adult", "older adult", "child": how a card names a person (the plan holds no names).
fn age_word(b: AgeBand) -> &'static str {
    match b {
        AgeBand::Infant => "baby",
        AgeBand::Toddler => "toddler",
        AgeBand::Child => "child",
        AgeBand::Teen => "teenager",
        AgeBand::Adult => "adult",
        AgeBand::Senior => "older adult",
    }
}

/// `## Your family plan`: the plan as written, with lines to write on where it is empty.
pub(super) fn plan(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let input = &a.input;
    let empty = FamilyPlan::default();
    // transitional: replaced by the binder workstream. The v2 packet prints only the v2 fields,
    // so a plan holding nothing but the contract v3 groups (home, neighbourhood, pets, vehicles,
    // documents: DESIGN-DELTA-v3 §3.2) reads as not yet written here.
    let written = input.family_plan.as_ref().filter(|p| has_v2_answers(p));
    let p = written.unwrap_or(&empty);
    out.push("## Your family plan".to_owned());
    out.push(String::new());
    out.push(format!(
        "{} Keep a copy in each go-bag and one on the fridge: phones die.{}",
        if written.is_some() {
            "This is the plan you wrote; fill in anything still blank together."
        } else {
            "Fill this in together, here or on the \"Your family plan\" screen."
        },
        cite("ready_gov_plan")
    ));
    out.push(String::new());

    let mut rows: Vec<(&str, String)> = vec![
        (
            "Meeting place near home",
            or_blank(p.meeting_place_near.as_deref()),
        ),
        (
            "Meeting place outside the neighborhood",
            or_blank(p.meeting_place_far.as_deref()),
        ),
        (
            "Out-of-area contact",
            contact(p.out_of_area_contact.as_ref()).unwrap_or_else(|| BLANK.to_owned()),
        ),
    ];
    if has_children(input) || p.school_pickup.is_some() {
        rows.push((
            "Who picks up the children",
            or_blank(p.school_pickup.as_deref()),
        ));
    }
    rows.push(("Work and school plans", or_blank(p.work_plans.as_deref())));
    rows.push((
        "Shelter spot at home",
        or_blank(p.shelter_spot_home.as_deref()),
    ));
    rows.push((
        "Shelter spot at work or school",
        or_blank(p.shelter_spot_work.as_deref()),
    ));
    rows.push((
        "Where we would go",
        or_blank(p.where_we_would_go.as_deref()),
    ));
    rows.push((
        "First way out",
        or_blank(p.routes.first().map(String::as_str)),
    ));
    rows.push((
        "Second way out",
        or_blank(p.routes.get(1).map(String::as_str)),
    ));
    rows.push((
        "Neighbors who check on us",
        or_blank(p.neighbours_who_check.as_deref()),
    ));
    if has_animals(input) || p.who_takes_animals.is_some() {
        rows.push((
            "Who takes the animals",
            or_blank(p.who_takes_animals.as_deref()),
        ));
    }
    if has_gas(input) || p.shutoff_gas.is_some() {
        rows.push(("Gas shut-off", or_blank(p.shutoff_gas.as_deref())));
    }
    rows.push(("Water shut-off", or_blank(p.shutoff_water.as_deref())));
    rows.push(("Electrical panel", or_blank(p.shutoff_electric.as_deref())));
    if !input.mobility.vehicles.is_empty() || p.roadside_assistance.is_some() {
        rows.push((
            "Roadside assistance",
            p.roadside_assistance
                .as_deref()
                .map_or_else(|| BLANK.to_owned(), phone),
        ));
    }
    rows.push((
        "Lawyer",
        contact(p.lawyer.as_ref()).unwrap_or_else(|| BLANK.to_owned()),
    ));
    out.push("| Plan | Ours |".to_owned());
    out.push("| --- | --- |".to_owned());
    for (label, value) in rows {
        out.push(format!("| {label} | {value} |"));
    }
    out.push(String::new());

    // The trusted circle (Ollam lessons): who has agreed to help, and what each holds.
    out.push(
        "**Our trusted circle:** people who agreed to help, and what they hold for us.".to_owned(),
    );
    out.push(String::new());
    out.push("| Name | Phone | Holds for us |".to_owned());
    out.push("| --- | --- | --- |".to_owned());
    for t in &p.trusted_circle {
        let holds: Vec<&str> = t.holds.iter().map(|h| holds_words(*h)).collect();
        out.push(format!(
            "| {} | {} | {} |",
            or_blank(t.name.as_deref()),
            t.phone.as_deref().map_or_else(|| BLANK.to_owned(), phone),
            if holds.is_empty() {
                BLANK.to_owned()
            } else {
                holds.join(", ")
            }
        ));
    }
    for _ in p.trusted_circle.len()..2 {
        out.push(format!("| {BLANK} | {BLANK} | {BLANK} |"));
    }
    out.push(String::new());
    out.push(format!(
        "**Numbers we know by heart:** {}",
        numbers_line(p).unwrap_or_else(|| BLANK.to_owned())
    ));
    out.push(String::new());

    // Leaving home: the ten-year chance, the warning by cause and the time away, as the
    // consequence model words them (model review M-08).
    // The first sentence (the ten-year chance) opens the summary when leaving comes first there.
    let evac = a.bucket(BucketId::Evacuate);
    let skip = usize::from(super::summary::leave_first(cx).is_some());
    let sentences: Vec<&str> = evac
        .frequency_sentences
        .iter()
        .skip(skip)
        .map(String::as_str)
        .collect();
    if !sentences.is_empty() {
        out.push("### Leaving home".to_owned());
        out.push(String::new());
        out.push(format!(
            "{}{}",
            super::md_marked(&sentences.join(" ")),
            cite_all(&evac.sources)
        ));
        out.push(String::new());
    }

    // Staying in touch: the four ways to reach each other (PACE), and school and child care for
    // households with children. The cards themselves are the next section.
    if let Some(g) = cx.blocks_for("plan:communication").into_iter().next() {
        let children = has_children(input);
        let paras: Vec<String> = super::headed_with_lists(&cx.guidance(g, None, None))
            .into_iter()
            .filter(|p| !p.starts_with("**Wallet cards."))
            .filter(|p| children || !p.starts_with("**School and child care."))
            .collect();
        if !paras.is_empty() {
            out.push("### Staying in touch".to_owned());
            out.push(String::new());
            for para in paras {
                out.push(para);
                out.push(String::new());
            }
        }
    }
}

/// `## Wallet cards`: one card per person, a block quote each, to cut out (web-interview; the
/// app prints each block quote as a card, two across).
pub(super) fn wallet_cards(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let input = &cx.a.input;
    let empty = FamilyPlan::default();
    let p = input.family_plan.as_ref().unwrap_or(&empty);
    out.push(WALLET_CARDS_HEADING.to_owned());
    out.push(String::new());
    out.push("Cut out a card for each person's wallet or phone case.".to_owned());
    out.push(String::new());
    let lines: [(&str, String); 7] = [
        (
            "Out-of-area contact",
            contact(p.out_of_area_contact.as_ref()).unwrap_or_else(|| BLANK.to_owned()),
        ),
        ("Meet near home", or_blank(p.meeting_place_near.as_deref())),
        (
            "Meet outside the neighborhood",
            or_blank(p.meeting_place_far.as_deref()),
        ),
        (
            "Lawyer",
            contact(p.lawyer.as_ref()).unwrap_or_else(|| BLANK.to_owned()),
        ),
        (
            "Trusted circle",
            circle_line(p).unwrap_or_else(|| BLANK.to_owned()),
        ),
        (
            "Know by heart",
            numbers_line(p).unwrap_or_else(|| BLANK.to_owned()),
        ),
        ("Medical notes", BLANK.to_owned()),
    ];
    for (i, person) in input.people.iter().enumerate() {
        out.push(format!(
            "> **Wallet card: person {} ({})**",
            i + 1,
            age_word(person.age_band)
        ));
        out.push(">".to_owned());
        for (label, value) in &lines {
            out.push(format!("> - {label}: {value}"));
        }
        out.push(String::new());
    }
}
