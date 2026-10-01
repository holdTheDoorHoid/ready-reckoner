//! Tab 4, Pets, vehicles and documents (DESIGN-DELTA-v3 §4.2): a page for the animals and one
//! for the vehicles (each only when the household has them), and documents and money (FEMA's
//! four-part kit, the accounts and policies, where the papers and the cash are, and what a home
//! too damaged to live in costs), always.

use rr_types::binder::{Block, FieldRow, Inline, Page, PageKind, Table};

use super::{Bx, contact_text, heading, page, row, row_lines, t};
use crate::packet::text;

pub(super) fn pages(bx: &Bx<'_>) -> Vec<Page> {
    let mut pages = Vec::new();
    pages.extend(pets(bx));
    pages.extend(vehicles(bx));
    pages.push(documents(bx));
    pages
}

/// Requirement lines with these rules as bullets, cited: the needs and, with `staged`, the
/// staging lines (a pet's water and food packed from household stock), as the v2 packet printed
/// them.
pub(crate) fn line_bullets(bx: &Bx<'_>, rules: &[&str], staged: bool) -> Vec<Vec<Inline>> {
    bx.a()
        .lines
        .iter()
        .filter(|l| {
            rules.contains(&l.line.rule.as_str())
                && (l.kind == rr_supply::LineKind::Need
                    || (staged && l.kind == rr_supply::LineKind::Alternative && l.quantity > 0.0))
        })
        .map(|l| {
            let mut v = vec![t(text::without_source_names(&l.line.plain))];
            v.extend(bx.cite(&l.line.citations));
            v
        })
        .collect()
}

/// The fields of one animal.
fn pet_fields(p: Option<&rr_types::PetInfo>) -> Vec<FieldRow> {
    let p = p.cloned().unwrap_or_default();
    vec![
        row("Name", p.name.as_deref()),
        row("Kind", p.kind.as_deref()),
        row("What it looks like", p.description.as_deref()),
        row("Medicines", p.medications.as_deref()),
        row("Vet", contact_text(p.vet.as_ref()).as_deref()),
        row("Microchip or tag number", p.microchip.as_deref()),
        row("Where its records are", p.records_where.as_deref()),
    ]
}

/// The animals (§4.2): one block per animal (the household's own answers, or, without them, one
/// block per animal step 3 counted, to fill in), who takes them, and the plan's pet lines.
fn pets(bx: &Bx<'_>) -> Option<Page> {
    let a = bx.a();
    let fp = super::family(&bx.cx);
    let counted = &a.input.pets;
    let any = u16::from(counted.dogs)
        + u16::from(counted.cats)
        + u16::from(counted.small)
        + u16::from(counted.large_animals)
        > 0;
    if !any && fp.pets.is_empty() && fp.who_takes_animals.is_none() {
        return None;
    }
    let mut blocks: Vec<Block> = Vec::new();
    if fp.pets.is_empty() {
        let mut n = 0;
        for (count, word) in [
            (counted.dogs, "Dog"),
            (counted.cats, "Cat"),
            (counted.small, "Small pet"),
        ] {
            for k in 0..count {
                if n >= rr_types::PETS_MAX {
                    break;
                }
                n += 1;
                let label = if count > 1 {
                    format!("{word} {}", k + 1)
                } else {
                    word.to_owned()
                };
                blocks.push(heading(2, &label));
                blocks.push(Block::Fields(pet_fields(None)));
            }
        }
        if counted.large_animals > 0 {
            blocks.push(heading(
                2,
                &format!("Large animals ({})", counted.large_animals),
            ));
            blocks.push(Block::Fields(vec![
                row_lines("Where they are kept", None, 2),
                row("Vet", None),
                row("Where their records are", None),
            ]));
        }
    } else {
        for (i, p) in fp.pets.iter().enumerate() {
            let label = match (p.name.as_deref(), p.kind.as_deref()) {
                (Some(n), Some(k)) => format!("{n} ({k})"),
                (Some(n), None) => n.to_owned(),
                (None, Some(k)) => format!("Animal {}: {k}", i + 1),
                (None, None) => format!("Animal {}", i + 1),
            };
            blocks.push(heading(2, &label));
            blocks.push(Block::Fields(pet_fields(Some(p))));
        }
    }
    blocks.push(heading(2, "If we cannot care for them"));
    blocks.push(Block::Fields(vec![row_lines(
        "Who takes the animals",
        fp.who_takes_animals.as_deref(),
        2,
    )]));
    let lines = line_bullets(bx, &["pet_food_lb", "pet_carrier", "livestock_water"], true);
    if !lines.is_empty() {
        blocks.push(heading(2, "What to have for them"));
        blocks.push(Block::Bullets(lines));
    }
    Some(page("pets", "Pets and animals", PageKind::Pets, blocks))
}

/// The vehicles (§4.2): each one (the household's own answers, or, without them, one block per
/// vehicle step 3 listed, to fill in), roadside assistance, and the stranded-driver checklist.
fn vehicles(bx: &Bx<'_>) -> Option<Page> {
    let a = bx.a();
    let fp = super::family(&bx.cx);
    let listed = &a.input.mobility.vehicles;
    if listed.is_empty() && fp.vehicles.is_empty() {
        return None;
    }
    let mut blocks: Vec<Block> = Vec::new();
    let fields = |v: Option<&rr_types::VehicleInfo>| {
        let v = v.cloned().unwrap_or_default();
        vec![
            row("Description", v.description.as_deref()),
            row("License plate", v.plate.as_deref()),
            row("Insurer", contact_text(v.insurer.as_ref()).as_deref()),
            row("Policy number", v.policy_number.as_deref()),
            row_lines("What stays in the car", v.kept_in_car.as_deref(), 2),
        ]
    };
    if fp.vehicles.is_empty() {
        for (i, v) in listed.iter().enumerate() {
            let fuel = match v.fuel {
                rr_types::Fuel::Gas => "gas",
                rr_types::Fuel::Diesel => "diesel",
                rr_types::Fuel::Hybrid => "hybrid",
                rr_types::Fuel::Ev => "electric",
            };
            blocks.push(heading(2, &format!("Vehicle {} ({fuel})", i + 1)));
            blocks.push(Block::Fields(fields(None)));
        }
    } else {
        for (i, v) in fp.vehicles.iter().enumerate() {
            let label = match v.description.as_deref() {
                Some(d) => format!("Vehicle {}: {d}", i + 1),
                None => format!("Vehicle {}", i + 1),
            };
            blocks.push(heading(2, &label));
            blocks.push(Block::Fields(fields(Some(v))));
        }
    }
    blocks.push(Block::Fields(vec![row(
        "Roadside assistance",
        fp.roadside_assistance.as_deref(),
    )]));
    let mut v = vec![t("If you are stuck on the road: ")];
    if let Some(l) = bx.link_inline("check_vehicle_stranding") {
        v.push(l);
        v.push(t("."));
        blocks.push(Block::Para(v));
    }
    Some(page("vehicles", "Vehicles", PageKind::Vehicles, blocks))
}

/// FEMA's Emergency Financial First Aid Kit, in four parts: what goes in each.
const KIT: [(&str, &str); 4] = [
    (
        "Who you are",
        "Photo IDs, birth certificates, Social Security cards, passports and pet records.",
    ),
    (
        "Money and legal papers",
        "Insurance policies, the lease or deed, a will and powers of attorney, bank and card \
         contact numbers (not PINs), and recent tax returns.",
    ),
    (
        "Medical papers",
        "Insurance cards, the written medicine list, prescriptions and vaccination records.",
    ),
    (
        "Contacts",
        "Family, doctors, the insurance agent, the landlord or lender, and employers.",
    ),
];

/// A cell with the household's words, or a line to write on.
fn cell(v: Option<&str>) -> Vec<Inline> {
    match v {
        Some(s) => vec![t(s)],
        None => vec![Inline::Blank(12)],
    }
}

/// Documents and money (§4.2): the four-part kit with a place to say where each part is, where
/// the originals, copies and backup are, the accounts (last four digits only) and policies, the
/// cash line, and what living elsewhere costs if damage forces you out.
fn documents(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let fp = super::family(&bx.cx);
    let docs = fp.documents.clone().unwrap_or_default();
    let home = fp.home.clone().unwrap_or_default();
    let mut blocks: Vec<Block> = Vec::new();
    let mut intro = vec![t(
        "Keep paper copies in a waterproof pouch and photos you can reach from any phone. \
         FEMA's Emergency Financial First Aid Kit groups them in four parts.",
    )];
    intro.extend(bx.cite_strs(&["fema_effak"]));
    blocks.push(Block::Para(intro));
    blocks.push(Block::Table(Table {
        header: vec![
            "Part".to_owned(),
            "What goes in it".to_owned(),
            "Where it is".to_owned(),
        ],
        rows: KIT
            .iter()
            .map(|(part, what)| vec![vec![t(*part)], vec![t(*what)], vec![Inline::Blank(16)]])
            .collect(),
    }));
    blocks.push(Block::Fields(vec![
        row("Where the originals are", docs.where_originals.as_deref()),
        row("Where the copies are", docs.where_copies.as_deref()),
        row(
            "Where the digital backup is",
            docs.digital_backup.as_deref(),
        ),
    ]));

    blocks.push(heading(2, "Accounts"));
    let mut rows: Vec<Vec<Vec<Inline>>> = docs
        .accounts
        .iter()
        .map(|x| {
            vec![
                cell(x.institution.as_deref()),
                cell(x.kind.as_deref()),
                cell(x.phone.as_deref()),
                cell(x.last4.as_deref()),
            ]
        })
        .collect();
    while rows.len() < 3 {
        rows.push(vec![vec![Inline::Blank(12)]; 4]);
    }
    blocks.push(Block::Table(Table {
        header: vec![
            "Bank or institution".to_owned(),
            "Kind".to_owned(),
            "Phone".to_owned(),
            "Last 4 digits".to_owned(),
        ],
        rows,
    }));

    blocks.push(heading(2, "Insurance policies"));
    let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();
    if let Some(ins) = home
        .insurer
        .as_ref()
        .filter(|c| c.name.is_some() || c.phone.is_some())
    {
        rows.push(vec![
            cell(ins.name.as_deref()),
            vec![t("Home or renters")],
            cell(home.policy_number.as_deref()),
            cell(ins.phone.as_deref()),
        ]);
    }
    rows.extend(docs.policies.iter().map(|x| {
        vec![
            cell(x.insurer.as_deref()),
            cell(x.kind.as_deref()),
            cell(x.policy_number.as_deref()),
            cell(x.phone.as_deref()),
        ]
    }));
    while rows.len() < 2 {
        rows.push(vec![vec![Inline::Blank(12)]; 4]);
    }
    blocks.push(Block::Table(Table {
        header: vec![
            "Insurer".to_owned(),
            "Kind".to_owned(),
            "Policy number".to_owned(),
            "Phone".to_owned(),
        ],
        rows,
    }));

    blocks.push(heading(2, "Cash"));
    let cash = line_bullets(bx, &["cash_reserve_usd"], false);
    if !cash.is_empty() {
        blocks.push(Block::Bullets(cash));
    }
    blocks.push(Block::Fields(vec![row(
        "Where the cash is",
        home.where_cash.as_deref(),
    )]));

    // If damage forced the household out: how long, and what living elsewhere costs.
    let loss = a.bucket(rr_types::BucketId::HomeLoss);
    if let Some(sentence) = loss
        .frequency_sentences
        .iter()
        .find(|s| s.starts_with("If damage forced you out"))
    {
        blocks.push(heading(2, "If damage forces you out"));
        let mut v = vec![t(sentence.clone())];
        v.extend(bx.cite(&loss.sources));
        blocks.push(Block::Para(v));
    }
    let mut v = vec![t(
        "After a disaster, start a claim and keep every receipt: ",
    )];
    v.extend(bx.link_inline("after"));
    v.push(t("."));
    blocks.push(Block::Para(v));
    page(
        "documents",
        "Documents and money",
        PageKind::Documents,
        blocks,
    )
}
