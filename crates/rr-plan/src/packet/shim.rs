//! transitional: replaced by the binder workstream
//!
//! A binder built from the v2 packet's Markdown, so every crate and the web app keep working until
//! the binder workstream writes the real assembly (DESIGN-DELTA-v3 §4, §11; `docs/ENGINE-API.md`,
//! "Mock engine"). `web/src/engine/mock/binder-shim.ts` builds the same shape from the mock's
//! packet.
//!
//! - The text before the first `##` (the header lines under the `#` title) is the cover page.
//! - Each v2 `##` section is one page of `para` blocks, one per Markdown paragraph, the text kept
//!   exactly as it is (headings, lists, tables and citation brackets included).
//! - Each page goes in the part of DESIGN-DELTA-v3 §4.2 that will hold that material
//!   ([`SECTIONS`]); a part with no pages is left out, so the tabs rise but may skip numbers.
//! - `sources` are the plan's provenance list in order, so the packet's bracket numbers are the
//!   source numbers; `credits` are the data attributions.

use rr_content::Content;
use rr_types::binder::{Binder, Block, Fit, Inline, Page, PageKind, Part, SourceEntry};
use rr_types::{Attribution, Citation};

use super::text;
use super::{Ctx, STATUS_LINE};
use crate::pipeline::Assessment;

/// The ten parts of DESIGN-DELTA-v3 §4.2: tab, id, title and tab label.
pub const PARTS: [(u8, &str, &str, &str); 10] = [
    (1, "start", "Start here", "Start here"),
    (2, "people", "People", "People"),
    (3, "home_places", "Home and places", "Home & places"),
    (
        4,
        "pets_vehicles_documents",
        "Pets, vehicles and documents",
        "Pets & docs",
    ),
    (5, "have", "What you have", "What you have"),
    (6, "check_now", "Checklists: happening now", "Happening now"),
    (
        7,
        "check_coming",
        "Checklists: it is coming",
        "It is coming",
    ),
    (8, "check_ongoing", "Checklists: it goes on", "It goes on"),
    (9, "after", "After", "After"),
    (10, "sources", "Sources", "Sources"),
];

/// Every v2 `##` section: its title, the tab it goes behind, its page id and kind.
pub const SECTIONS: [(&str, u8, &str, PageKind); 17] = [
    ("Summary", 1, "summary", PageKind::QuickStart),
    ("Your family plan", 1, "family_plan", PageKind::Contacts),
    ("Wallet cards", 2, "wallet_cards", PageKind::WalletCards),
    (
        "Access and functional needs",
        2,
        "access_needs",
        PageKind::Person,
    ),
    ("Special needs", 2, "special_needs", PageKind::Person),
    ("Your shelter plan", 3, "shelter_plan", PageKind::Home),
    ("Local help", 3, "local_help", PageKind::Neighbourhood),
    ("Documents and money", 4, "documents", PageKind::Documents),
    ("Your risks", 5, "risks", PageKind::RisksGlance),
    ("Your targets", 5, "targets", PageKind::Inventory),
    ("Your plan", 5, "plan", PageKind::Inventory),
    ("Checklists", 5, "checklists", PageKind::Inventory),
    (
        "Maintenance calendar",
        5,
        "maintenance",
        PageKind::Inventory,
    ),
    (
        "When a storm, freeze or heat wave is forecast",
        7,
        "forecast",
        PageKind::Checklist,
    ),
    (
        "After a disaster: the first 30 days",
        9,
        "after",
        PageKind::After,
    ),
    ("If it lasts for months", 9, "months", PageKind::After),
    ("Sources", 10, "sources", PageKind::Sources),
];

/// Where a section the table does not name goes: the "What you have" tab.
const OTHER_TAB: u8 = 5;

/// The v2 packet's title, when its Markdown has none.
const DEFAULT_TITLE: &str = "Your preparedness packet";

/// The transitional binder for a finished v2 packet (`markdown`, with its Sources section).
pub(crate) fn binder(
    markdown: &str,
    a: &Assessment,
    content: &Content,
    provenance: &[Citation],
) -> Binder {
    let cx = Ctx { a, content };
    let (title, cover, sections) = split(markdown);
    let mut pages: Vec<(u8, Page)> = Vec::new();
    pages.push((
        1,
        page("cover", title.clone(), PageKind::Cover, paragraphs(&cover)),
    ));
    let mut used: Vec<String> = vec!["cover".to_owned()];
    for (heading, body) in sections {
        let (tab, id, kind) = match SECTIONS.iter().find(|(t, ..)| *t == heading) {
            Some(&(_, tab, id, kind)) => (tab, id.to_owned(), kind),
            None => (OTHER_TAB, slug(&heading), PageKind::Inventory),
        };
        let id = unique(id, &used);
        used.push(id.clone());
        pages.push((tab, page(&id, heading, kind, paragraphs(&body))));
    }
    let parts = PARTS
        .iter()
        .filter_map(|&(tab, id, title, short)| {
            let mine: Vec<Page> = pages
                .iter()
                .filter(|(t, _)| *t == tab)
                .map(|(_, p)| p.clone())
                .collect();
            (!mine.is_empty()).then(|| Part {
                id: id.to_owned(),
                tab,
                title: title.to_owned(),
                short_title: short.to_owned(),
                pages: mine,
            })
        })
        .collect();
    let zip = a
        .location
        .zip
        .as_deref()
        .map(|z| format!(" (ZIP code {z})"))
        .unwrap_or_default();
    let generated_on = a.input.planning_date;
    Binder {
        title,
        generated_on,
        household: text::household(&a.input),
        location: format!("{}{zip}", super::summary::place(&cx)),
        status_line: STATUS_LINE.to_owned(),
        // The v2 packet's yearly review (its maintenance calendar).
        review_by: generated_on.add_months(12).unwrap_or(generated_on),
        parts,
        sources: provenance
            .iter()
            .enumerate()
            .map(|(i, c)| source(i, c))
            .collect(),
        credits: a.attributions.iter().map(credit).collect(),
    }
}

fn page(id: &str, title: String, kind: PageKind, blocks: Vec<Block>) -> Page {
    Page {
        id: id.to_owned(),
        title,
        kind,
        fit: Fit::Flow,
        blocks,
    }
}

/// The packet split into its `#` title, the text before the first `##`, and each `##` section as
/// (heading, body).
fn split(markdown: &str) -> (String, String, Vec<(String, String)>) {
    let mut title: Option<String> = None;
    let mut cover = String::new();
    let mut sections: Vec<(String, String)> = Vec::new();
    for line in markdown.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            sections.push((h.trim().to_owned(), String::new()));
            continue;
        }
        if title.is_none()
            && sections.is_empty()
            && let Some(h) = line.strip_prefix("# ")
        {
            title = Some(h.trim().to_owned());
            continue;
        }
        let body = match sections.last_mut() {
            Some((_, body)) => body,
            None => &mut cover,
        };
        body.push_str(line);
        body.push('\n');
    }
    (
        title.unwrap_or_else(|| DEFAULT_TITLE.to_owned()),
        cover,
        sections,
    )
}

/// One `para` block per Markdown paragraph (text between blank lines), the text as it is.
fn paragraphs(body: &str) -> Vec<Block> {
    body.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| Block::Para(vec![Inline::T(p.to_owned())]))
        .collect()
}

/// A page id for a heading the table does not name: lowercase letters and digits, runs of
/// anything else as one underscore.
fn slug(heading: &str) -> String {
    let mut out = String::new();
    for c in heading.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let trimmed = out.trim_end_matches('_');
    if trimmed.is_empty() {
        "section".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `id`, or `id_2`, `id_3`, … when it is taken.
fn unique(id: String, used: &[String]) -> String {
    if !used.contains(&id) {
        return id;
    }
    (2..)
        .map(|n| format!("{id}_{n}"))
        .find(|candidate| !used.contains(candidate))
        .unwrap_or(id)
}

fn source(i: usize, c: &Citation) -> SourceEntry {
    SourceEntry {
        n: u32::try_from(i + 1).unwrap_or(u32::MAX),
        title: c.title.clone(),
        publisher: c.publisher.clone(),
        year: c.year,
        url: (!c.url.trim().is_empty()).then(|| c.url.clone()),
        expert: c.prior,
    }
}

/// A data credit as plain text: the source, its version and access date, then the statement
/// exactly as its terms require, and the address when the statement does not name it.
fn credit(at: &Attribution) -> String {
    let version = at
        .version
        .as_deref()
        .map(|v| format!(", version {v}"))
        .unwrap_or_default();
    let url = if at.text.contains(at.url.as_str()) {
        String::new()
    } else {
        format!(" {}", at.url)
    };
    format!(
        "{}{version}, accessed {}. {}{url}",
        at.source,
        text::date(at.accessed),
        at.text
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_part_table_follows_the_delta() {
        let tabs: Vec<u8> = PARTS.iter().map(|p| p.0).collect();
        assert_eq!(tabs, (1..=10).collect::<Vec<u8>>());
        for (_, id, _, short) in PARTS {
            assert!(
                short.chars().count() <= rr_types::binder::SHORT_TITLE_MAX,
                "{id}: {short}"
            );
        }
        for (title, tab, ..) in SECTIONS {
            assert!(PARTS.iter().any(|p| p.0 == tab), "{title}");
        }
        let heads: Vec<&str> = super::super::SECTION_HEADINGS
            .iter()
            .chain(super::super::CONDITIONAL_HEADINGS.iter().map(|(h, _)| h))
            .map(|h| h.strip_prefix("## ").unwrap())
            .collect();
        for h in &heads {
            assert!(SECTIONS.iter().any(|s| s.0 == *h), "no page for ## {h}");
        }
        assert_eq!(heads.len(), SECTIONS.len());
    }

    #[test]
    fn split_keeps_the_cover_the_sections_and_the_text() {
        let md = "# Title here\n\n**For:** 1 adult  \n**Where:** X\n\n> A note.\n\n## Summary\n\nOne.\n\n### Three\n\n1. a\n2. b\n\n## Odd one out\n\nText [3].\n";
        let (title, cover, sections) = split(md);
        assert_eq!(title, "Title here");
        assert_eq!(
            paragraphs(&cover),
            vec![
                Block::Para(vec![Inline::T("**For:** 1 adult  \n**Where:** X".into())]),
                Block::Para(vec![Inline::T("> A note.".into())]),
            ]
        );
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, "Summary");
        assert_eq!(
            paragraphs(&sections[0].1),
            vec![
                Block::Para(vec![Inline::T("One.".into())]),
                Block::Para(vec![Inline::T("### Three".into())]),
                Block::Para(vec![Inline::T("1. a\n2. b".into())]),
            ]
        );
        assert_eq!(slug(&sections[1].0), "odd_one_out");
        assert_eq!(
            slug("After a disaster: the first 30 days"),
            "after_a_disaster_the_first_30_days"
        );
        assert_eq!(slug("!!!"), "section");
        let used = vec!["a".to_owned(), "a_2".to_owned()];
        assert_eq!(unique("a".into(), &used), "a_3");
        assert_eq!(unique("b".into(), &used), "b");
    }
}
