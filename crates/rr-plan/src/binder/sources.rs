//! Tab 10, Sources (DESIGN-DELTA-v3 §4.2; brief 2): the sources the binder's brackets point to,
//! numbered in order of first use (the list itself is added by `Draft::finish`, once the numbers
//! are known: [`source_list`]), then the data credits exactly as their terms require, then how the
//! binder was made. Web addresses are kept exactly as they are, never escaped.

use rr_types::binder::{Block, Inline, Page, PageKind, SourceEntry};

use super::{Bx, heading, t};
use crate::packet::text;

/// The sources page without its numbered list.
pub(super) fn page(bx: &Bx<'_>) -> Page {
    let a = bx.a();
    let mut blocks = vec![Block::Para(vec![t(
        "The numbers in brackets point to this list; \"expert estimate\" marks a judgement, not \
         measured data. The targets and the plan are Ready Reckoner's calculations from these.",
    )])];
    if !a.attributions.is_empty() {
        blocks.push(heading(1, "Data credits"));
        blocks.push(Block::Bullets(
            a.attributions
                .iter()
                .map(|at| vec![t(super::credit(at))])
                .collect(),
        ));
    }
    blocks.push(heading(1, "How this binder was made"));
    let e = bx.extras;
    blocks.push(Block::Bullets(vec![
        vec![t(format!(
            "Made by Ready Reckoner {} on {}, from the plan's answers.",
            e.engine_version,
            text::date(a.input.planning_date)
        ))],
        vec![t(format!("Data pack {}.", e.data_pack_version))],
        vec![t(format!("Content {}.", e.content_version))],
        vec![t(format!(
            "Review it and print it again by {}, and whenever something changes.",
            text::date(super::review_by(a))
        ))],
    ]));
    super::page("sources", "Sources", PageKind::Sources, blocks)
}

/// The numbered sources as a list (one entry per source, in order, so a list's numbers are the
/// sources' numbers): title, publisher, year, the web address once (a later source on the same
/// page says "Same page as 3."), expert estimates marked.
pub(super) fn source_list(sources: &[SourceEntry]) -> Block {
    let mut first_url: Vec<(&str, u32)> = Vec::new();
    Block::Numbered(
        sources
            .iter()
            .map(|s| {
                let year = s.year.map(|y| format!(", {y}")).unwrap_or_default();
                let mut text = format!(
                    "{}. {}{year}.",
                    crate::packet::sources::short_title(&s.title),
                    s.publisher
                );
                if let Some(url) = s.url.as_deref() {
                    match first_url.iter().find(|(u, _)| *u == url) {
                        Some((_, n)) => text.push_str(&format!(" Same page as {n}.")),
                        None => {
                            first_url.push((url, s.n));
                            text.push(' ');
                            text.push_str(url);
                        }
                    }
                }
                if s.expert {
                    text.push_str(" Expert estimate.");
                }
                vec![Inline::T(text)]
            })
            .collect(),
    )
}
