//! The fit proxy (DESIGN-DELTA-v3 §5.6): a page's load in word units against
//! [`CAPACITY`] units per printed US Letter page. A heading counts [`HEADING`], a step or bullet
//! its words plus 2, a field row [`FIELD_ROW`], a table row [`TABLE_ROW`], a paragraph its words.
//! The rules the delta leaves open: the page title is a heading; a field with room for several
//! lines counts 4 more for each extra line; a decision's question is a heading and each branch an
//! item; a callout counts its blocks and 2; a map slot [`MAP_SLOT`] (about half a page, the
//! printed map and its legend); a wallet card its title as a heading and each line as a field
//! row, halved (two cards print side by side); a log its rows as table rows. Words are tokens with
//! a letter or a digit; citations are not counted.
//!
//! A checklist page promises what its block says (`pages: 1` or `2`); every other page's `fit` is
//! what the proxy finds (one page, two, or more: `flow`), so a renderer knows which pages to keep
//! whole. `fit_one_pages_fit` and `fit_two_pages_fit` in the tests hold the checklists to their
//! promise; the capacity is a starting estimate that web-binder calibrates against the real PDF
//! (a calibration changes [`CAPACITY`], not the content).

use rr_types::binder::{Block, Fit, Inline, Page};

/// Word units on one printed US Letter page (DESIGN-DELTA-v3 §5.6).
pub const CAPACITY: f64 = 480.0;

/// A heading (the page title included).
pub const HEADING: f64 = 6.0;

/// A field row.
pub const FIELD_ROW: f64 = 8.0;

/// A table row (the header row included).
pub const TABLE_ROW: f64 = 10.0;

/// A map slot: about half a printed page.
pub const MAP_SLOT: f64 = 240.0;

/// Words in running text: tokens with a letter or a digit.
pub fn words(s: &str) -> f64 {
    s.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count() as f64
}

fn inline_words(v: &[Inline]) -> f64 {
    let mut n = 0.0;
    for i in v {
        n += match i {
            Inline::T(s) | Inline::B(s) => words(s),
            Inline::Link(l) => words(&l.text),
            Inline::Blank(_) => 1.0,
            Inline::Cite(_) => 0.0,
        };
    }
    // Text split across inlines ("Get out." + " Leave now") counts a word twice at a seam only
    // when the seam falls inside a word; seams fall at spaces in practice.
    n
}

/// A block's load in word units.
pub fn block_units(b: &Block) -> f64 {
    match b {
        Block::Heading(_) => HEADING,
        Block::Para(v) => inline_words(v),
        Block::Bullets(items) | Block::Numbered(items) => {
            items.iter().map(|i| inline_words(i) + 2.0).sum()
        }
        Block::Steps(steps) => steps.iter().map(|s| inline_words(&s.text) + 2.0).sum(),
        Block::Fields(rows) => rows
            .iter()
            .map(|r| {
                let extra = if r.value.is_none() {
                    f64::from(r.lines.saturating_sub(1)) * 4.0
                } else {
                    0.0
                };
                FIELD_ROW + extra
            })
            .sum(),
        Block::Table(t) => TABLE_ROW * (t.rows.len() as f64 + 1.0),
        Block::Callout(c) => 2.0 + c.blocks.iter().map(block_units).sum::<f64>(),
        Block::Decision(d) => {
            HEADING
                + d.branches
                    .iter()
                    .map(|br| inline_words(&br.when) + inline_words(&br.then) + 2.0)
                    .sum::<f64>()
        }
        Block::MapSlot(_) => MAP_SLOT,
        Block::Cards(cards) => cards
            .iter()
            .map(|c| (HEADING + FIELD_ROW * c.lines.len() as f64) / 2.0)
            .sum(),
        Block::Log(l) => TABLE_ROW * (f64::from(l.rows) + 1.0),
        Block::PageBreak(_) => 0.0,
    }
}

/// A page's load in printed pages: its title and blocks against [`CAPACITY`].
pub fn load(p: &Page) -> f64 {
    (HEADING + p.blocks.iter().map(block_units).sum::<f64>()) / CAPACITY
}

/// The fit the proxy finds for a page: one page, two, or more.
pub fn fit_for(p: &Page) -> Fit {
    let l = load(p);
    if l <= 1.0 {
        Fit::One
    } else if l <= 2.0 {
        Fit::Two
    } else {
        Fit::Flow
    }
}

/// Printed pages a page takes by the proxy: its load rounded up, at least one.
pub fn printed_pages(p: &Page) -> u32 {
    let l = load(p);
    (l.ceil() as u32).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::binder::{FieldRow, PageKind, Step, Table};

    #[test]
    fn units_follow_the_delta() {
        let p = Page {
            id: "x".into(),
            title: "X".into(),
            kind: PageKind::Checklist,
            fit: Fit::One,
            blocks: vec![
                Block::Heading(rr_types::binder::Heading {
                    level: 1,
                    text: "Do first".into(),
                }),
                Block::Steps(vec![Step {
                    text: vec![
                        Inline::B("Get out.".into()),
                        Inline::T(" Leave now.".into()),
                        Inline::Cite(vec![1]),
                    ],
                    memory: true,
                }]),
                Block::Fields(vec![FieldRow {
                    label: "Meet".into(),
                    value: None,
                    lines: 1,
                }]),
                Block::Table(Table {
                    header: vec!["A".into()],
                    rows: vec![vec![vec![Inline::T("a".into())]]],
                }),
                Block::Para(vec![Inline::T("One two three, 4.".into())]),
            ],
        };
        // Title 6 + heading 6 + step (4 words + 2) + field 8 + table (2 rows) 20 + para 4.
        assert_eq!(load(&p) * CAPACITY, 6.0 + 6.0 + 6.0 + 8.0 + 20.0 + 4.0);
        assert_eq!(fit_for(&p), Fit::One);
        assert_eq!(printed_pages(&p), 1);
        assert_eq!(words("— a, 3"), 2.0);
    }
}
