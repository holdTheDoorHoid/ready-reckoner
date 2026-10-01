//! Tab 1, Start here (DESIGN-DELTA-v3 §4.2): the cover, how to use the binder, Quick start (the
//! first five minutes of any emergency, from memory), "Which checklist?" (every hazard and
//! everyday emergency with the page to turn to) and every number on one page.

use rr_types::binder::{
    Block, Callout, CalloutKind, FieldRow, Inline, Page, PageKind, Step, Table,
};

use super::checklists::{SOMETHING_ELSE, Selection, event_name};
use super::{Bx, b, contact_short, heading, page, row, t};
use crate::packet::text;
use crate::source::FIXTURE_DATA_NOTE;

/// The Poison Control number (Poison Help, run by HRSA), as printed.
pub const POISON_HELP: &str = "1-800-222-1222";

/// The citation behind the Poison Help number.
pub const POISON_HELP_CITATION: &str = "hrsa_poison_help";

pub(super) fn pages(bx: &Bx<'_>, sel: &Selection<'_>) -> Vec<Page> {
    vec![
        cover(bx),
        block_page(
            bx,
            "how_to_use",
            PageKind::HowToUse,
            "plan:how_to_use_binder",
        ),
        quick_start(bx),
        index(bx, sel),
        contacts(bx),
    ]
}

fn title_of(id: &str) -> &'static str {
    super::FIXED_PAGES
        .iter()
        .find(|p| p.0 == id)
        .map_or("", |p| p.2)
}

/// The cover: who, where, the home's address, when it was made and when to review it, and the
/// status line.
fn cover(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let fp = super::family(&bx.cx);
    let mut blocks = vec![
        Block::Para(vec![
            b("For:"),
            t(format!(" {}", text::household(&a.input))),
        ]),
        Block::Para(vec![
            b("Where:"),
            t(format!(" {}", super::location_line(&bx.cx))),
        ]),
        Block::Fields(vec![row(
            "Home address",
            fp.home.as_ref().and_then(|h| h.address.as_deref()),
        )]),
        Block::Para(vec![
            b("Made on:"),
            t(format!(" {}", text::date(a.input.planning_date))),
        ]),
    ];
    let mut review = vec![
        b("Review by:"),
        t(format!(
            " {}, and whenever something changes.",
            text::date(super::review_by(a))
        )),
    ];
    review.extend(bx.cite_strs(&["ready_gov_kit"]));
    blocks.push(Block::Para(review));
    blocks.push(Block::Callout(Callout {
        kind: CalloutKind::Note,
        title: None,
        blocks: vec![Block::Para(vec![t(crate::packet::STATUS_LINE)])],
    }));
    if a.location.data_note.as_deref() == Some(FIXTURE_DATA_NOTE) {
        blocks.push(Block::Callout(Callout {
            kind: CalloutKind::Warning,
            title: Some("Sample data".to_owned()),
            blocks: vec![Block::Para(vec![t(FIXTURE_DATA_NOTE)])],
        }));
    }
    page("cover", title_of("cover"), PageKind::Cover, blocks)
}

/// A page that is one guidance block, for this household.
fn block_page(bx: &Bx<'_>, id: &str, kind: PageKind, target: &str) -> Page {
    let blocks = bx
        .cx
        .blocks_for(target)
        .into_iter()
        .next()
        .map(|g| bx.prose_blocks(&bx.cx.guidance(g, None, None)))
        .unwrap_or_default();
    page(id, title_of(id), kind, blocks)
}

/// Quick start (`plan_quick_start`): its numbered list is the memory steps; then where to turn.
fn quick_start(bx: &Bx<'_>) -> Page {
    let mut p = block_page(bx, "quick_start", PageKind::QuickStart, "plan:quick_start");
    for block in &mut p.blocks {
        if let Block::Numbered(items) = block {
            *block = Block::Steps(
                std::mem::take(items)
                    .into_iter()
                    .map(|text| Step { text, memory: true })
                    .collect(),
            );
        }
    }
    let mut next = vec![b("Then:"), t(" find what is happening in ")];
    next.extend(bx.link_inline("index"));
    next.push(t(". Numbers to call: "));
    next.extend(bx.link_inline("contacts"));
    next.push(t("."));
    p.blocks.push(Block::Para(next));
    p.fit = super::fit::fit_for(&p);
    p
}

/// Where an index row sends the reader: its page, or, with no page of its own yet, the
/// "Something else" page (or Quick start).
fn turn_to(bx: &Bx<'_>, sel: &Selection<'_>, page: Option<&str>) -> Vec<Inline> {
    if let Some(link) = page.and_then(|p| bx.link_inline(p)) {
        return vec![link];
    }
    let fallback = sel
        .page_for_event(SOMETHING_ELSE)
        .map_or("quick_start", |s| s.checklist.meta.id.as_str());
    let mut v = vec![t("No page of its own yet: use ")];
    v.extend(bx.link_inline(fallback));
    v
}

/// "Which checklist?": every everyday emergency and every hazard in the ranked matrix, most
/// likely first, with the page to turn to; the rare families the household opted into; the ones
/// it did not, in one line; the hazards too unlikely here to rank, in one line.
fn index(bx: &Bx<'_>, sel: &Selection<'_>) -> Page {
    let cx = &bx.cx;
    let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();
    for &event in rr_content::ids::EVENTS
        .iter()
        .filter(|e| **e != SOMETHING_ELSE)
    {
        let page = sel
            .page_for_event(event)
            .map(|s| s.checklist.meta.id.as_str());
        rows.push(vec![vec![t(event_name(event))], turn_to(bx, sel, page)]);
    }
    for p in super::checklists::ranked(cx) {
        let page = sel
            .page_for_hazard(p.id)
            .map(|s| s.checklist.meta.id.as_str());
        rows.push(vec![vec![t(p.name.clone())], turn_to(bx, sel, page)]);
    }
    // Rare families: the ones the household opted into, and any whose page is in the binder
    // for another hazard (a volcano page also covers a very large eruption).
    let opted = cx.a.input.dials.rare_families();
    let mut not_here: Vec<String> = Vec::new();
    for p in
        cx.a.hazards
            .profiles
            .iter()
            .filter(|p| p.display == rr_types::HazardDisplay::RareCatastrophic)
    {
        let page = sel
            .page_for_hazard(p.id)
            .map(|s| s.checklist.meta.id.as_str())
            .or_else(|| {
                bx.cx
                    .content
                    .checklist_for(&format!("hazard:{}", p.id))
                    .map(|c| c.meta.id.as_str())
                    .filter(|id| sel.page(id).is_some())
            });
        if opted.contains(&p.id.as_str()) || page.is_some() {
            rows.push(vec![vec![t(p.name.clone())], turn_to(bx, sel, page)]);
        } else {
            not_here.push(text::lower_first(&p.name));
        }
    }
    rows.push(vec![
        vec![t("Anything else")],
        turn_to(
            bx,
            sel,
            sel.page_for_event(SOMETHING_ELSE)
                .map(|s| s.checklist.meta.id.as_str()),
        ),
    ]);
    let mut blocks = vec![
        Block::Para(vec![t(
            "Find what is happening, then turn to its page. Everyday emergencies come first, then \
             the risks where you live, most likely first.",
        )]),
        Block::Table(Table {
            header: vec!["If this happens".to_owned(), "Turn to".to_owned()],
            rows,
        }),
    ];
    if !not_here.is_empty() {
        blocks.push(Block::Para(vec![t(format!(
            "Not in this binder: {}. To add one, tick the family under Your settings.",
            text::join_and(&not_here)
        ))]));
    }
    let also: Vec<String> =
        cx.a.hazards
            .also_checked
            .iter()
            .map(|c| text::lower_first(&c.name))
            .collect();
    if !also.is_empty() {
        blocks.push(Block::Para(vec![t(format!(
            "Also checked, and too unlikely here to need a page: {}.",
            text::join_and(&also)
        ))]));
    }
    page("index", title_of("index"), PageKind::Index, blocks)
}

/// A group of contacts: a heading and its rows.
fn group(blocks: &mut Vec<Block>, title: &str, rows: Vec<FieldRow>) {
    if rows.is_empty() {
        return;
    }
    blocks.push(heading(2, title));
    blocks.push(Block::Fields(rows));
}

/// Every number in the binder on one page (§4.2; brief 2): the emergency numbers, cited, then
/// the household's own numbers, grouped, with lines to write on where it gave none; then the
/// four ways to reach each other.
fn contacts(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let fp = super::family(&bx.cx);
    let home = fp.home.clone().unwrap_or_default();
    let hood = fp.neighbourhood.clone().unwrap_or_default();
    let mut blocks: Vec<Block> = Vec::new();

    let call = |number: &str, ids: &[&str]| {
        let mut v = vec![b(number)];
        v.extend(bx.cite_strs(ids));
        v
    };
    blocks.push(heading(2, "Emergency numbers"));
    blocks.push(Block::Table(Table {
        header: vec!["For".to_owned(), "Call".to_owned()],
        rows: vec![
            vec![
                vec![t(
                    "Police, fire or an ambulance (text 911 where you cannot call)",
                )],
                call("911", &["fcc_text_911"]),
            ],
            vec![
                vec![t("Suicide and Crisis Lifeline: call, text or chat")],
                call("988", &["samhsa_988"]),
            ],
            vec![
                vec![t("Poison Control")],
                call(POISON_HELP, &[POISON_HELP_CITATION]),
            ],
            vec![
                vec![t("Disaster Distress Helpline: call or text")],
                call("1-800-985-5990", &["samhsa_disaster_distress"]),
            ],
        ],
    }));

    let people: Vec<FieldRow> = a
        .input
        .people
        .iter()
        .enumerate()
        .map(|(i, p)| {
            row(
                &super::person_title(i, p),
                p.profile.as_ref().and_then(|pr| pr.phone.as_deref()),
            )
        })
        .collect();
    group(&mut blocks, "Our people", people);
    group(
        &mut blocks,
        "Out-of-area contact and lawyer",
        vec![
            row(
                "Out-of-area contact",
                contact_short(fp.out_of_area_contact.as_ref()).as_deref(),
            ),
            row("Lawyer", contact_short(fp.lawyer.as_ref()).as_deref()),
            row(
                "Numbers we know by heart",
                (!fp.numbers_by_heart.is_empty())
                    .then(|| fp.numbers_by_heart.join("; "))
                    .as_deref(),
            ),
        ],
    );

    // The trusted circle: who agreed to help, and what each holds (at least two rows).
    blocks.push(heading(2, "Trusted circle"));
    let mut circle: Vec<Vec<Vec<Inline>>> = fp
        .trusted_circle
        .iter()
        .map(|p| {
            let holds: Vec<&str> = p.holds.iter().map(|h| holds_words(*h)).collect();
            vec![
                cell(p.name.as_deref()),
                cell(p.phone.as_deref()),
                if holds.is_empty() {
                    vec![Inline::Blank(20)]
                } else {
                    vec![t(holds.join(", "))]
                },
            ]
        })
        .collect();
    while circle.len() < 2 {
        circle.push(vec![
            vec![Inline::Blank(20)],
            vec![Inline::Blank(12)],
            vec![Inline::Blank(20)],
        ]);
    }
    blocks.push(Block::Table(Table {
        header: vec![
            "Name".to_owned(),
            "Phone".to_owned(),
            "Holds for us".to_owned(),
        ],
        rows: circle,
    }));

    // Doctors and pharmacies, each once, with whose they are.
    let mut care: Vec<(String, String, Vec<String>)> = Vec::new();
    for (i, p) in a.input.people.iter().enumerate() {
        let Some(pr) = &p.profile else { continue };
        let who = super::person_title(i, p);
        for (kind, c) in [("Doctor", &pr.doctor), ("Pharmacy", &pr.pharmacy)] {
            if let Some(v) = contact_short(c.as_ref()) {
                match care.iter_mut().find(|(k, x, _)| *k == kind && *x == v) {
                    Some((_, _, whose)) => whose.push(who.clone()),
                    None => care.push((kind.to_owned(), v, vec![who.clone()])),
                }
            }
        }
    }
    if let Some(v) = contact_short(hood.pharmacy.as_ref())
        && !care.iter().any(|(k, x, _)| k == "Pharmacy" && *x == v)
    {
        care.push(("Pharmacy".to_owned(), v, Vec::new()));
    }
    let mut care_rows: Vec<FieldRow> = care
        .iter()
        .map(|(kind, v, whose)| {
            let label = if whose.is_empty() || whose.len() == a.input.people.len() {
                kind.clone()
            } else {
                format!("{kind} for {}", text::join_and(whose))
            };
            row(&label, Some(v))
        })
        .collect();
    for kind in ["Doctor", "Pharmacy"] {
        if !care.iter().any(|(k, _, _)| k == kind) {
            care_rows.push(row(kind, None));
        }
    }
    group(&mut blocks, "Doctors and pharmacies", care_rows);

    let landlord = if a.input.housing.tenure == rr_types::Tenure::Rent {
        "Landlord"
    } else {
        "Mortgage company"
    };
    group(
        &mut blocks,
        "Home and utilities (outage numbers)",
        vec![
            row(
                "Electric company",
                contact_short(home.electric_utility.as_ref()).as_deref(),
            ),
            row(
                "Gas company",
                contact_short(home.gas_utility.as_ref()).as_deref(),
            ),
            row(
                "Water company",
                contact_short(home.water_utility.as_ref()).as_deref(),
            ),
            row(
                "Home insurer",
                contact_short(home.insurer.as_ref()).as_deref(),
            ),
            row(
                landlord,
                contact_short(home.landlord_or_mortgage.as_ref()).as_deref(),
            ),
        ],
    );

    let mut more: Vec<FieldRow> = Vec::new();
    let mut vets: Vec<String> = Vec::new();
    for pet in &fp.pets {
        if let Some(v) = contact_short(pet.vet.as_ref())
            && !vets.contains(&v)
        {
            vets.push(v);
        }
    }
    let animals = rr_content::policy::HouseholdFacts::has_pets(&bx.cx);
    if vets.is_empty() && animals {
        more.push(row("Vet", None));
    }
    for v in &vets {
        more.push(row("Vet", Some(v)));
    }
    let vehicles = !a.input.mobility.vehicles.is_empty() || !fp.vehicles.is_empty();
    if vehicles || fp.roadside_assistance.is_some() {
        more.push(row(
            "Roadside assistance",
            fp.roadside_assistance.as_deref(),
        ));
    }
    for (i, v) in fp.vehicles.iter().enumerate() {
        if let Some(ins) = contact_short(v.insurer.as_ref()) {
            let label = match v.description.as_deref() {
                Some(_) if fp.vehicles.len() > 1 => format!("Car insurance, vehicle {}", i + 1),
                _ => "Car insurance".to_owned(),
            };
            more.push(row(&label, Some(&ins)));
        }
    }
    group(&mut blocks, "Animals and vehicles", more);

    group(
        &mut blocks,
        "Help nearby",
        vec![
            row("Hospital", contact_short(hood.hospital.as_ref()).as_deref()),
            row(
                "Urgent care",
                contact_short(hood.urgent_care.as_ref()).as_deref(),
            ),
            row(
                "Community shelter",
                contact_short(hood.shelter.as_ref()).as_deref(),
            ),
            row(
                "County emergency office",
                contact_short(hood.county_emergency_office.as_ref()).as_deref(),
            ),
        ],
    );

    // Four ways to reach each other (the communication plan's PACE list).
    if let Some(g) = bx.cx.blocks_for("plan:communication").into_iter().next() {
        let paras: Vec<String> = crate::packet::headed_with_lists(&bx.cx.guidance(g, None, None))
            .into_iter()
            .filter(|p| p.starts_with("**Four ways to reach each other."))
            .collect();
        if !paras.is_empty() {
            blocks.push(heading(2, "How to reach each other"));
            for p in paras {
                // The lead is the heading; keep what follows it.
                let body = p
                    .strip_prefix("**Four ways to reach each other.**")
                    .unwrap_or(&p)
                    .trim();
                blocks.extend(bx.prose_blocks(body));
            }
        }
    }
    page("contacts", title_of("contacts"), PageKind::Contacts, blocks)
}

/// A table cell with the household's words, or a line to write on.
fn cell(v: Option<&str>) -> Vec<Inline> {
    match v {
        Some(s) => vec![t(s)],
        None => vec![Inline::Blank(16)],
    }
}

/// What a trusted person holds, in words.
pub(crate) fn holds_words(h: rr_types::Holds) -> &'static str {
    match h {
        rr_types::Holds::SpareKey => "a spare key",
        rr_types::Holds::Documents => "copies of documents",
        rr_types::Holds::MedicalPoa => "medical power of attorney",
        rr_types::Holds::BackupCodes => "backup codes",
    }
}
