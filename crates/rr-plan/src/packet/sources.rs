//! Section 10: sources. Every citation the packet's brackets point to, in their numbered order,
//! compact: title, publisher, year and the URL once (without its `https://`), run together ten to
//! a paragraph so the list prints in a few pages; the retrieval dates are in the plan's JSON. The
//! expert estimates are marked. Then the data credits, with the National Risk Index statement
//! exactly as its terms require.

use rr_types::Citation;

use super::text::{self, md};
use crate::pipeline::Assessment;

/// A title without a trailing parenthetical (a journal's volume and pages, a report number): the
/// URL identifies the work, and the plan's JSON keeps the full title.
fn short_title(title: &str) -> &str {
    let t = title.trim_end();
    if !t.ends_with(')') {
        return t;
    }
    // The opening bracket that matches the final closing one.
    let mut depth = 0usize;
    for (i, c) in t.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let head = t[..i].trim_end();
                    return if head.is_empty() { t } else { head };
                }
            }
            _ => {}
        }
    }
    t
}

/// Sources run together in paragraphs of this many.
const SOURCES_PER_PARAGRAPH: usize = 10;

pub(super) fn write(a: &Assessment, provenance: &[Citation], more: usize, out: &mut Vec<String>) {
    out.push("## Sources".to_owned());
    out.push(String::new());
    out.push(
        "The numbers in brackets point to this list; \"expert estimate\" marks a judgement, not \
         measured data. The targets and the plan are Ready Reckoner's calculations from these."
            .to_owned(),
    );
    out.push(String::new());
    let mut first_with_url: Vec<(&str, usize)> = Vec::new();
    let mut entries: Vec<String> = Vec::new();
    for (i, c) in provenance.iter().enumerate() {
        let n = i + 1;
        let year = c.year.map(|y| format!(", {y}")).unwrap_or_default();
        let prior = if c.prior { " Expert estimate." } else { "" };
        // Each URL once: a second source on the same page points to the first.
        let url = match first_with_url.iter().find(|(u, _)| *u == c.url.as_str()) {
            Some((_, first)) => format!("Same page as {first}."),
            None => {
                first_with_url.push((c.url.as_str(), n));
                c.url
                    .strip_prefix("https://")
                    .unwrap_or(c.url.as_str())
                    .to_owned()
            }
        };
        entries.push(format!(
            "**{n}** {}. {}{year}. {url}{prior}",
            md(short_title(&c.title)),
            md(&c.publisher)
        ));
    }
    for chunk in entries.chunks(SOURCES_PER_PARAGRAPH) {
        out.push(chunk.join(" "));
        out.push(String::new());
    }
    if more > 0 {
        out.push(format!(
            "{} more {} behind the plan's quantities and prices {} listed in the app, next to \
             each number.",
            more,
            if more == 1 { "source" } else { "sources" },
            if more == 1 { "is" } else { "are" }
        ));
        out.push(String::new());
    }
    if !a.attributions.is_empty() {
        out.push("### Data credits".to_owned());
        out.push(String::new());
        for at in &a.attributions {
            let version = at
                .version
                .as_deref()
                .map(|v| format!(", version {v}"))
                .unwrap_or_default();
            // The URL once: many statements already name it.
            let url = if at.text.contains(at.url.as_str()) {
                String::new()
            } else {
                format!(" {}", at.url)
            };
            out.push(format!(
                "> **{}{version}, accessed {}.** {}{url}",
                md(&at.source),
                text::date(at.accessed),
                md(&at.text)
            ));
            out.push(String::new());
        }
    }
}
