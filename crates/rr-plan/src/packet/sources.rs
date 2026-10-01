//! The Prepare sheet's Sources: every citation its brackets point to, numbered in the order the
//! sheet first uses them, compact: title, publisher, year and the URL once, run together ten to a
//! paragraph; the retrieval dates are in the plan's JSON. The expert estimates are marked. Then
//! the data credits, with the National Risk Index statement exactly as its terms require. Web
//! addresses are printed exactly as they are: never Markdown-escaped (a backslash before the `#`
//! in the Eviction Lab address, v0.3 merge note).

use rr_types::Citation;

use super::text::{self, md};
use crate::pipeline::Assessment;

/// A title without a trailing parenthetical (a journal's volume and pages, a report number): the
/// URL identifies the work, and the plan's JSON keeps the full title.
pub(crate) fn short_title(title: &str) -> &str {
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

pub(super) fn write(a: &Assessment, provenance: &[Citation], out: &mut Vec<String>) {
    out.push("## Sources".to_owned());
    out.push(String::new());
    out.push(
        "The numbers in brackets point to this list; \"expert estimate\" marks a judgement, not \
         measured data. The plan is Ready Reckoner's calculation from these."
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
                // In full: the app links a bare https address (and prints it once).
                first_with_url.push((c.url.as_str(), n));
                c.url.clone()
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
    if let Some(first) = a.attributions.first() {
        out.push("### Data credits".to_owned());
        out.push(String::new());
        // The first credit (the National Risk Index statement, whose terms require its version and
        // access date) says when; the others say it only when it differs.
        let when = first.accessed;
        for (i, at) in a.attributions.iter().enumerate() {
            let version = at
                .version
                .as_deref()
                .map(|v| format!(", version {v}"))
                .unwrap_or_default();
            let accessed = if i == 0 || at.accessed != when {
                format!(", accessed {}", text::date(at.accessed))
            } else {
                String::new()
            };
            // The URL once: many statements already name it.
            let url = if at.text.contains(at.url.as_str()) {
                String::new()
            } else {
                format!(" {}", at.url)
            };
            out.push(format!(
                "> **{}{version}{accessed}.** {}{url}",
                md(&at.source),
                crate::binder::markdown::esc(&at.text)
            ));
            out.push(String::new());
        }
        if a.attributions.len() > 1 {
            out.push(format!(
                "The other data sets were accessed {} too, unless they say otherwise.",
                text::date(when)
            ));
            out.push(String::new());
        }
    }
}
