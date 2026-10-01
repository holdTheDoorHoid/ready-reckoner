//! Tab 9, After (DESIGN-DELTA-v3 §4.2; brief 2): the first 30 days (the v2 recovery page, with
//! the county's federal disaster declarations), "If it lasts for months" when the plan has a
//! long-horizon part, and four blank logs to fill in by hand.

use rr_types::binder::{Block, Log, Page, PageKind};

use super::{Bx, page, t};

/// The title of the long-horizon page.
pub const MONTHS_TITLE: &str = "If it lasts for months";

/// The four logs: page id, title, columns.
pub const LOGS: [(&str, &str, [&str; 4]); 4] = [
    (
        "log_damage",
        "Damage log",
        ["What", "Where", "Photo taken", "Reported to"],
    ),
    (
        "log_expenses",
        "Expenses log",
        ["Date", "What", "Amount", "Receipt"],
    ),
    (
        "log_contacts",
        "People contacted",
        ["Date", "Who", "Number", "What they said"],
    ),
    (
        "log_medications",
        "Medications given",
        ["Date", "Who", "What", "Time"],
    ),
];

/// Blank rows on each log page.
pub const LOG_ROWS: u8 = 24;

pub(super) fn pages(bx: &Bx<'_>) -> Vec<Page> {
    let mut pages = vec![first_30_days(bx)];
    pages.extend(months(bx));
    for (id, title, columns) in LOGS {
        pages.push(page(
            id,
            title,
            PageKind::Log,
            vec![
                Block::Para(vec![t(
                    "Write it down as it happens: insurers and FEMA ask for dates, names and \
                     receipts.",
                )]),
                Block::Log(Log {
                    columns: columns.iter().map(|c| (*c).to_owned()).collect(),
                    rows: LOG_ROWS,
                }),
            ],
        ));
    }
    pages
}

/// The body of a v2 section writer without its `##` heading.
fn body(lines: Vec<String>) -> String {
    lines
        .into_iter()
        .skip_while(|l| l.starts_with("## ") || l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first 30 days (review RR-P06): the county's declarations, then `after_first_30_days`.
fn first_30_days(bx: &Bx<'_>) -> Page {
    let mut lines: Vec<String> = Vec::new();
    crate::packet::pages::after_disaster(&bx.cx, &mut lines);
    let mut blocks = bx.md_blocks(&body(lines));
    let mut v = vec![t("Keep track as you go: ")];
    for (i, (id, _, _)) in LOGS.iter().enumerate() {
        if i > 0 {
            v.push(t(if i + 1 == LOGS.len() { " and " } else { ", " }));
        }
        v.extend(bx.link_inline(id));
    }
    v.push(t("."));
    blocks.push(Block::Para(v));
    page(
        "after",
        "After a disaster: the first 30 days",
        PageKind::After,
        blocks,
    )
}

/// "If it lasts for months", only when the plan has a long-horizon part (the v2 page).
fn months(bx: &Bx<'_>) -> Option<Page> {
    let mut lines: Vec<String> = Vec::new();
    crate::packet::pages::long_horizon(&bx.cx, &mut lines);
    if lines.is_empty() {
        return None;
    }
    Some(page(
        "after_months",
        MONTHS_TITLE,
        PageKind::After,
        bx.md_blocks(&body(lines)),
    ))
}
