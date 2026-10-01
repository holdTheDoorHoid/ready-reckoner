//! Tab 2, People (DESIGN-DELTA-v3 §4.2; brief 2): who is in this binder, one page per person
//! (their own answers as written, blanks where empty, their medicines, and what the interview's
//! step 2 said about them in words), "Special needs and health" (each part only when the
//! household needs it; stress and the 988 line always) and the wallet cards, one per person,
//! with names on them.

use rr_types::binder::{Block, Card, FieldRow, Inline, Page, PageKind, Table};
use rr_types::{AccessNeed, AgeBand, Mobility, Person, PoweredDevice};

use super::{Bx, b, contact_short, contact_text, heading, page, person_title, row, row_lines, t};
use crate::packet::{Ctx, NB_HYPHEN, text};

/// Each person's page id and title, in the household's order.
pub(super) fn outline(cx: &Ctx<'_>) -> Vec<(String, String)> {
    cx.a.input
        .people
        .iter()
        .enumerate()
        .map(|(i, p)| (format!("person_{}", i + 1), person_title(i, p)))
        .collect()
}

pub(super) fn pages(bx: &Bx<'_>, outline: &[(String, String)]) -> Vec<Page> {
    let mut pages = vec![overview(bx, outline)];
    for (i, p) in bx.a().input.people.iter().enumerate() {
        pages.push(person_page(bx, i, p, &outline[i]));
    }
    pages.push(special_needs(bx));
    pages.push(wallet_cards(bx));
    pages
}

/// "Who is in this binder" (page id `people`, the target of `{ref:people}`): one line per person
/// with a link to their page, then the pointers to the health page and the wallet cards.
fn overview(bx: &Bx<'_>, outline: &[(String, String)]) -> Page {
    let a = bx.a();
    let rows: Vec<Vec<Vec<Inline>>> = a
        .input
        .people
        .iter()
        .zip(outline)
        .map(|(p, (id, title))| {
            vec![
                vec![t(title.clone())],
                vec![t(text::upper_first(super::age_word(p.age_band)))],
                bx.link_inline(id).into_iter().collect(),
            ]
        })
        .collect();
    let mut cards = vec![t("Cut out a wallet card for each person: ")];
    cards.extend(bx.link_inline("wallet_cards"));
    cards.push(t(". Medicine, devices and other health needs: "));
    cards.extend(bx.link_inline("special_needs"));
    cards.push(t("."));
    let blocks = vec![
        Block::Table(Table {
            header: vec![
                "Who".to_owned(),
                "Age group".to_owned(),
                "Their page".to_owned(),
            ],
            rows,
        }),
        Block::Para(cards),
    ];
    page("people", title("people"), PageKind::Index, blocks)
}

fn title(id: &str) -> &'static str {
    super::FIXED_PAGES
        .iter()
        .find(|p| p.0 == id)
        .map_or("", |p| p.2)
}

/// An access or functional need in words.
pub(crate) fn need_words(n: AccessNeed) -> &'static str {
    match n {
        AccessNeed::Hearing => "deaf or hard of hearing",
        AccessNeed::Vision => "blind or low vision",
        AccessNeed::LimitedEnglish => "speaks or reads little English",
        AccessNeed::Cognitive => "memory or understanding affected",
        AccessNeed::Supervision => "needs someone with them",
        AccessNeed::ServiceAnimal => "has a service animal",
        AccessNeed::Dialysis => "needs dialysis",
        AccessNeed::HomeHealth => "gets home health care",
    }
}

/// What the interview's step 2 said about a person, in words (brief 2: "From your answers").
fn from_answers(bx: &Bx<'_>, i: usize, p: &Person) -> Vec<String> {
    let m = &p.medical;
    let mut out: Vec<String> = Vec::new();
    if m.daily_rx {
        out.push("Takes prescription medicine every day.".to_owned());
    }
    if m.refrigerated_rx {
        out.push("Has a medicine that must stay cold.".to_owned());
    }
    match m.powered_device {
        PoweredDevice::None => {}
        PoweredDevice::Cpap => out.push("Uses a CPAP or BiPAP breathing machine.".to_owned()),
        PoweredDevice::Oxygen => out.push("Uses an oxygen concentrator.".to_owned()),
        PoweredDevice::Other { watts } => out.push(format!(
            "Uses a powered medical device that draws about {} watts.",
            text::number(f64::from(watts))
        )),
    }
    if m.epinephrine {
        out.push("Carries an epinephrine auto-injector.".to_owned());
    }
    if p.pregnant_or_nursing {
        out.push("Pregnant or nursing.".to_owned());
    }
    match m.mobility {
        Mobility::None => {}
        Mobility::Limited => out.push("Walks with difficulty or needs help on stairs.".to_owned()),
        Mobility::Wheelchair => out.push("Uses a wheelchair.".to_owned()),
    }
    if !p.access_needs.is_empty() {
        let needs: Vec<String> = p
            .access_needs
            .iter()
            .map(|n| need_words(*n).to_owned())
            .collect();
        out.push(format!("{}.", text::upper_first(&text::join_and(&needs))));
    }
    if !m.dietary.is_empty() {
        out.push(format!("Diet: {}.", m.dietary.join(", ")));
    }
    if let Some(c) = &p.commute {
        let mode = match c.mode {
            rr_types::CommuteMode::Car => "by car",
            rr_types::CommuteMode::Transit => "by bus or train",
            rr_types::CommuteMode::Walk => "on foot",
            rr_types::CommuteMode::Bike => "by bike",
        };
        let walk = bx
            .a()
            .consequence
            .get_home
            .commuters
            .iter()
            .find(|w| w.person == i)
            .map(|w| {
                let d = rr_consequence::words::days_phrase(w.walk_hours / 24.0);
                let d = if d.starts_with("about ") {
                    d
                } else {
                    format!("about {d}")
                };
                format!(", {d} on foot")
            })
            .unwrap_or_default();
        out.push(format!(
            "Goes to work or school {mode}: a {} trip{walk}.",
            rr_consequence::words::distance_adjective(f64::from(c.distance_km))
        ));
    }
    out
}

/// One person's page: their answers as written (blanks where empty), their medicines, where
/// they spend the day, and step 2's answers in words.
fn person_page(bx: &Bx<'_>, i: usize, p: &Person, (id, page_title): &(String, String)) -> Page {
    let empty = rr_types::PersonProfile::default();
    let pr = p.profile.as_ref().unwrap_or(&empty);
    let ins = pr.insurance.clone().unwrap_or_default();
    let mut blocks: Vec<Block> = vec![Block::Fields(vec![
        row("Name", pr.name.as_deref()),
        row("Date of birth", pr.date_of_birth.as_deref()),
        row("Phone", pr.phone.as_deref()),
        row("Email", pr.email.as_deref()),
        row("Blood type", pr.blood_type.as_deref()),
        row("Allergies", pr.allergies.as_deref()),
        row_lines("Medical conditions", pr.conditions.as_deref(), 2),
        row("Doctor", contact_text(pr.doctor.as_ref()).as_deref()),
        row("Pharmacy", contact_text(pr.pharmacy.as_ref()).as_deref()),
    ])];

    blocks.push(heading(2, "Health insurance"));
    blocks.push(Block::Fields(vec![
        row("Insurance company", ins.carrier.as_deref()),
        row("Plan", ins.plan_name.as_deref()),
        row("Member ID", ins.member_id.as_deref()),
        row("Group number", ins.group_number.as_deref()),
        row("Insurance phone", ins.phone.as_deref()),
    ]));

    blocks.push(heading(2, "Medicines"));
    let mut rows: Vec<Vec<Vec<Inline>>> = pr
        .medications
        .iter()
        .map(|m| {
            [&m.name, &m.dose, &m.schedule, &m.purpose]
                .into_iter()
                .map(|v| match v.as_deref() {
                    Some(s) => vec![t(s)],
                    None => vec![Inline::Blank(12)],
                })
                .collect()
        })
        .collect();
    while rows.len() < 4 {
        rows.push(vec![vec![Inline::Blank(12)]; 4]);
    }
    blocks.push(Block::Table(Table {
        header: vec![
            "Medicine".to_owned(),
            "Dose".to_owned(),
            "When".to_owned(),
            "What for".to_owned(),
        ],
        rows,
    }));

    blocks.push(heading(2, "Where they spend the day"));
    let place = pr.place.as_ref();
    let mut fields: Vec<FieldRow> = vec![row(
        "Place",
        place
            .map(|pl| super::places::place_line(pl, &page_title_of(i, p)))
            .as_deref(),
    )];
    fields.push(row("ID documents", pr.id_notes.as_deref()));
    fields.push(row_lines(
        "Anything else a helper should know",
        pr.notes.as_deref(),
        2,
    ));
    blocks.push(Block::Fields(fields));
    if let Some(pid) = super::places::page_for(&bx.cx, i) {
        let mut v = vec![t("Its address, plan and pick-up rules: ")];
        v.extend(bx.link_inline(&pid));
        v.push(t("."));
        blocks.push(Block::Para(v));
    }

    blocks.push(heading(2, "From your answers"));
    let said = from_answers(bx, i, p);
    if said.is_empty() {
        blocks.push(Block::Para(vec![t(
            "No daily medicine, medical device or access need was listed for this person.",
        )]));
    } else {
        blocks.push(Block::Bullets(
            said.into_iter().map(|s| vec![t(s)]).collect(),
        ));
    }
    page(id, page_title, PageKind::Person, blocks)
}

/// The title a person's page has (their name, or "Person 2 (child)").
fn page_title_of(i: usize, p: &Person) -> String {
    person_title(i, p)
}

/// "Special needs and health" (brief 1): medicine, powered devices, babies and toddlers, older
/// adults, getting around, pregnancy and nursing and access and functional needs, each only
/// when the household needs it; stress, mental health and the 988 line always. The v2 packet's
/// requirement lines and catalogue advice, as it printed them.
fn special_needs(bx: &Bx<'_>) -> Page {
    let mut lines: Vec<String> = Vec::new();
    crate::packet::people::special_needs_body(&bx.cx, &mut lines);
    let mut blocks = bx.md_blocks(&lines.join("\n"));
    // Access and functional needs (CMIST), when someone has one.
    let mut access: Vec<String> = Vec::new();
    crate::packet::pages::access_needs(&bx.cx, &mut access);
    if !access.is_empty() {
        let body: Vec<String> = access
            .into_iter()
            .skip_while(|l| l.starts_with("## ") || l.is_empty())
            .collect();
        // Before the mental-health part, which always comes last.
        let at = blocks
            .iter()
            .rposition(|b| matches!(b, Block::Heading(_)))
            .unwrap_or(blocks.len());
        let mut part = vec![heading(2, "Access and functional needs")];
        part.extend(bx.md_blocks(&body.join("\n")));
        blocks.splice(at..at, part);
    }
    page(
        "special_needs",
        title("special_needs"),
        PageKind::Person,
        blocks,
    )
}

/// Phone numbers on a card: every hyphen between two digits made non-breaking, so a number never
/// breaks across two lines of a card.
pub(crate) fn nb(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 4);
    for (i, c) in chars.iter().enumerate() {
        let between = *c == '-'
            && i > 0
            && chars[i - 1].is_ascii_digit()
            && chars.get(i + 1).is_some_and(char::is_ascii_digit);
        out.push(if between { NB_HYPHEN } else { *c });
    }
    out
}

/// A card line: the label, then the words or a line to write on.
fn card_line(label: &str, value: Option<String>) -> Vec<Inline> {
    let mut v = vec![b(format!("{label}: "))];
    match value {
        Some(s) => v.push(t(s)),
        None => v.push(Inline::Blank(20)),
    }
    v
}

/// The wallet cards (brief 2): one per person, with their name.
fn wallet_cards(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let fp = super::family(&bx.cx);
    let circle: Vec<String> = fp
        .trusted_circle
        .iter()
        .map(|p| {
            [p.name.as_deref(), p.phone.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|s| !s.is_empty())
        .collect();
    let cards: Vec<Card> = a
        .input
        .people
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let pr = p.profile.as_ref();
            let health: Vec<String> = [
                pr.and_then(|x| x.blood_type.as_deref())
                    .map(|v| format!("blood type {v}")),
                pr.and_then(|x| x.allergies.as_deref())
                    .map(|v| format!("allergies: {v}")),
            ]
            .into_iter()
            .flatten()
            .collect();
            Card {
                title: format!("Wallet card: {}", person_title(i, p)),
                lines: vec![
                    card_line(
                        "Out-of-area contact",
                        contact_short(fp.out_of_area_contact.as_ref()).map(|s| nb(&s)),
                    ),
                    card_line("Meet near home", fp.meeting_place_near.clone()),
                    card_line(
                        "Meet outside the neighborhood",
                        fp.meeting_place_far.clone(),
                    ),
                    card_line("Lawyer", contact_short(fp.lawyer.as_ref()).map(|s| nb(&s))),
                    card_line(
                        "Trusted circle",
                        (!circle.is_empty()).then(|| nb(&circle.join("; "))),
                    ),
                    card_line(
                        "Know by heart",
                        (!fp.numbers_by_heart.is_empty())
                            .then(|| nb(&fp.numbers_by_heart.join("; "))),
                    ),
                    card_line("Health", (!health.is_empty()).then(|| health.join("; "))),
                    vec![t("Medicines and doctors: see binder tab 2.")],
                ],
            }
        })
        .collect();
    let blocks = vec![
        Block::Para(vec![t(
            "Cut out a card for each person's wallet or phone case. Fill in any blank by hand.",
        )]),
        Block::Cards(cards),
    ];
    page(
        "wallet_cards",
        title("wallet_cards"),
        PageKind::WalletCards,
        blocks,
    )
}

/// Whether anyone in the household is a baby, toddler, child or teenager.
pub(crate) fn has_children(p: &[Person]) -> bool {
    p.iter().any(|x| {
        matches!(
            x.age_band,
            AgeBand::Infant | AgeBand::Toddler | AgeBand::Child | AgeBand::Teen
        )
    })
}
