//! Tab 5, What you have (DESIGN-DELTA-v3 §4.2; brief 1, 2): the inventory (the plan's things by
//! need, how much, whether the household has it or the month the plan gets it, a place to write
//! where it is kept, and its next check), what to expect (how long to be ready for each kind of
//! disruption, when help comes, when service is mostly back, the worst on record, and the
//! setting in words), and the risks at a glance (every ranked hazard and rare family with how
//! likely, how bad, and its checklist page).

use std::collections::BTreeMap;

use rr_types::binder::{Block, Inline, Page, PageKind, Table};
use rr_types::{BucketId, BucketKind, HazardDisplay, ItemId, PlanItemKind};

use super::checklists::{SOMETHING_ELSE, Selection, in_100};
use super::{Bx, heading, page, t};
use crate::packet::targets::{record_span, relief_cell, target_phrase};
use crate::packet::text;

pub(super) fn pages(bx: &Bx<'_>, sel: &Selection<'_>) -> Vec<Page> {
    vec![inventory(bx), what_to_expect(bx), risks_glance(bx, sel)]
}

/// One thing on the inventory: its plan lines merged.
struct Thing {
    id: ItemId,
    name: String,
    qty: f64,
    unit: String,
    bucket: BucketId,
    /// The first month the plan buys some of it (none when the household has it all).
    first_buy: Option<u16>,
}

/// The inventory (brief 2): every purchase in the plan, merged across months, by the need it is
/// for (its first bucket), with the quantity; "have" when what the household owns (listed, or
/// assumed as an everyday basic) meets it, otherwise the month the plan gets it; a blank for where
/// it is kept; its next check, test or rotation (the maintenance calendar's date).
fn inventory(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let cx = &bx.cx;
    let mut things: Vec<Thing> = Vec::new();
    for m in &a.budget.plan.months {
        for i in m.items.iter().filter(|i| i.kind == PlanItemKind::Purchase) {
            match things.iter_mut().find(|x| x.id == i.item_id) {
                Some(x) => {
                    x.qty += f64::from(i.quantity);
                    if !i.done && x.first_buy.is_none() {
                        x.first_buy = Some(m.index);
                    }
                }
                None => things.push(Thing {
                    id: i.item_id.clone(),
                    name: text::short_name(&i.name).to_owned(),
                    qty: f64::from(i.quantity),
                    unit: i.unit.clone(),
                    bucket: i.buckets.first().copied().unwrap_or(BucketId::Supplies),
                    first_buy: (!i.done).then_some(m.index),
                }),
            }
        }
    }
    let owned: BTreeMap<&str, f64> = {
        let mut m: BTreeMap<&str, f64> = BTreeMap::new();
        for o in &a.input.existing {
            *m.entry(o.item_id.as_str()).or_insert(0.0) += f64::from(o.qty);
        }
        for (id, q) in &a.assumed {
            *m.entry(id.as_str()).or_insert(0.0) += q;
        }
        m
    };
    let mut blocks: Vec<Block> = vec![Block::Para(vec![t(
        "Tick each thing when you have it, and write where it is kept. \"Have\" means you \
         listed it, or the plan counts it as an everyday basic most homes have.",
    )])];
    for &b in BucketId::ALL.iter() {
        let mine: Vec<&Thing> = things.iter().filter(|x| x.bucket == b).collect();
        if mine.is_empty() {
            continue;
        }
        blocks.push(heading(2, &a.bucket(b).name));
        let rows = mine
            .iter()
            .map(|x| {
                let have = owned.get(x.id.as_str()).copied().unwrap_or(0.0) + 1e-9 >= x.qty;
                let status = match (have, x.first_buy) {
                    (true, _) | (false, None) => "have".to_owned(),
                    (false, Some(m)) => format!("still to get (month {m})"),
                };
                let next = cx
                    .item(x.id.as_str())
                    .and_then(|it| {
                        crate::packet::calendar::next_due(
                            a.input.planning_date,
                            it,
                            if have { 0 } else { x.first_buy.unwrap_or(0) },
                        )
                    })
                    .unwrap_or_else(|| "—".to_owned());
                vec![
                    vec![t(x.name.clone())],
                    vec![t(text::quantity(x.qty, &x.unit))],
                    vec![t(status)],
                    vec![Inline::Blank(14)],
                    vec![t(next)],
                ]
            })
            .collect();
        blocks.push(Block::Table(Table {
            header: vec![
                "What".to_owned(),
                "How much".to_owned(),
                "Have it?".to_owned(),
                "Where kept".to_owned(),
                "Next check".to_owned(),
            ],
            rows,
        }));
    }
    let mut v = vec![t("Medicines: each person's list is on their page in ")];
    v.extend(bx.link_inline("people"));
    v.push(t("."));
    blocks.push(Block::Para(v));
    page("inventory", "What you have", PageKind::Inventory, blocks)
}

/// What to expect (brief 1): the duration targets with when help likely arrives, when service
/// is mostly back and the worst event on record; the setting in words; how well the numbers hold
/// up.
fn what_to_expect(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();
    for b in a
        .buckets
        .iter()
        .filter(|b| b.id.kind() == BucketKind::Duration)
    {
        let worst = b
            .stress_test
            .as_ref()
            .filter(|_| {
                matches!(
                    b.id,
                    BucketId::Power | BucketId::WaterOut | BucketId::WaterBoil
                )
            })
            .and_then(|st| record_span(st, b.id))
            .map_or_else(|| "—".to_owned(), |s| s.words());
        rows.push(vec![
            vec![t(b.name.clone())],
            vec![t(target_phrase(&b.target))],
            vec![t(relief_cell(
                b.relief.as_ref().map(|r| r.help_arrives_days),
            ))],
            vec![t(relief_cell(
                b.relief.as_ref().map(|r| r.mostly_restored_days),
            ))],
            vec![t(worst)],
        ]);
    }
    let mut setting = vec![t(format!(
        "How long to be ready for each kind of disruption at the 1-in-{} setting. {}",
        a.input.dials.return_period.years(),
        crate::packet::dial_sentence(&a.consequence)
    ))];
    setting.extend(bx.cite_strs(&["rr_research_risk_model", "rr_risk_model_priors"]));
    let mut blocks = vec![
        Block::Para(setting),
        Block::Table(Table {
            header: vec![
                "If this happens".to_owned(),
                "Be ready for".to_owned(),
                "Help likely in".to_owned(),
                "Mostly back in".to_owned(),
                "Worst on record".to_owned(),
            ],
            rows,
        }),
        Block::Para(vec![t(
            "Brackets show how uncertain a target is. \"Not known\": no restoration records for \
             the event behind that target. \"Worst on record\": how long the worst power or water \
             event in your region's records kept homes waiting.",
        )]),
    ];
    if let Some(line) = crate::packet::targets::validation_line(&bx.cx) {
        blocks.push(Block::Para(bx.inl(&line)));
    }
    page(
        "what_to_expect",
        "What to expect",
        PageKind::RisksGlance,
        blocks,
    )
}

/// The checklist cell of a risk row: the hazard's page, or "Something else" when it has none.
fn checklist_cell(bx: &Bx<'_>, sel: &Selection<'_>, id: rr_types::HazardId) -> Vec<Inline> {
    let page = sel
        .page_for_hazard(id)
        .map(|s| s.checklist.meta.id.clone())
        .or_else(|| {
            bx.cx
                .content
                .checklist_for(&format!("hazard:{id}"))
                .filter(|c| sel.page(&c.meta.id).is_some())
                .map(|c| c.meta.id.clone())
        });
    match page.and_then(|p| bx.link_inline(&p)) {
        Some(l) => vec![l],
        None => {
            let mut v = vec![t("Not in this binder")];
            if let Some(s) = sel.page_for_event(SOMETHING_ELSE)
                && let Some(l) = bx.link_inline(&s.checklist.meta.id)
            {
                v.push(t("; see "));
                v.push(l);
            }
            v
        }
    }
}

/// The risks at a glance (§4.2): the ranked matrix as one table, then the rare families with the
/// same columns (how likely as a range only), then the notes behind the numbers.
fn risks_glance(bx: &Bx<'_>, sel: &Selection<'_>) -> Page {
    let a = bx.a();
    let years = a.input.dials.horizon_years.max(1);
    let horizon = rr_consequence::words::horizon_phrase(years);
    let ranked = super::checklists::ranked(&bx.cx);
    let rows: Vec<Vec<Vec<Inline>>> = ranked
        .iter()
        .map(|p| {
            let mut likely = vec![t(in_100(crate::packet::risks::chance(
                p.rate_per_year,
                years,
            )))];
            likely.extend(bx.cite(&p.sources));
            vec![
                vec![t(p.name.clone())],
                likely,
                vec![t(text::severity(p.severity))],
                checklist_cell(bx, sel, p.id),
            ]
        })
        .collect();
    let mut blocks = vec![
        Block::Para(vec![t(format!(
            "What could reach a household like yours in {} over {horizon}, most likely first.",
            crate::packet::summary::place(&bx.cx)
        ))]),
        Block::Table(Table {
            header: vec![
                "What could happen".to_owned(),
                format!(
                    "Households like yours, {years} year{}",
                    if years == 1 { "" } else { "s" }
                ),
                "How bad".to_owned(),
                "Checklist".to_owned(),
            ],
            rows,
        }),
    ];
    let rare: Vec<&rr_types::HazardProfile> = a
        .hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        .collect();
    if !rare.is_empty() {
        blocks.push(heading(2, "Rare but severe"));
        blocks.push(Block::Table(Table {
            header: vec![
                "What".to_owned(),
                "How likely (a range only)".to_owned(),
                "How bad".to_owned(),
                "Checklist".to_owned(),
            ],
            rows: rare
                .iter()
                .map(|p| {
                    let mut likely = vec![t(text::upper_first(
                        &crate::packet::risks::range_words(p.rate_range[0], p.rate_range[1], years),
                    ))];
                    likely.extend(bx.cite(&p.sources));
                    vec![
                        vec![t(p.name.clone())],
                        likely,
                        vec![t(text::severity(p.severity))],
                        checklist_cell(bx, sel, p.id),
                    ]
                })
                .collect(),
        }));
    }
    let notes: Vec<Vec<Inline>> = a
        .hazards
        .notes
        .iter()
        .filter(|n| !n.starts_with("Also checked"))
        .map(|n| vec![t(n.clone())])
        .collect();
    if !notes.is_empty() {
        blocks.push(heading(2, "Notes on these numbers"));
        blocks.push(Block::Bullets(notes));
    }
    page(
        "risks_glance",
        "Risks at a glance",
        PageKind::RisksGlance,
        blocks,
    )
}
