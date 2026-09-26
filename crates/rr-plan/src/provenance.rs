//! Provenance: every citation id the plan and the packet refer to, resolved against
//! `content/citations.toml` and deduplicated. The packet's own citations come first, in the order
//! it first uses them (so its numbered brackets count up through the text); ids used only by the
//! JSON output follow, sorted.
//!
//! An unknown id is a bug in the crate that emits it. Debug builds (tests) fail loudly, except
//! for ids another workstream has formally requested and that are still waiting for their entry
//! ([`AWAITING_CONTENT`]); release builds show a placeholder source and a warning instead of
//! failing a household's plan.

use std::collections::BTreeSet;

use rr_content::Content;
use rr_types::{Citation, CitationId, Date, Warning, WarningSeverity};

use crate::pipeline::Assessment;

/// Ids requested in `docs/CITATION_IDS.md` that `content/citations.toml` does not define yet:
/// tolerated with a placeholder, even in debug builds. A test fails once content defines one, so
/// the list only shrinks. // awaiting: rr-content
pub const AWAITING_CONTENT: &[&str] = &["county_boil_water_records"];

/// Where a placeholder source points: the index where requested ids wait for their entry.
const PLACEHOLDER_URL: &str =
    "https://github.com/holdTheDoorHoid/ready-reckoner/blob/main/docs/CITATION_IDS.md";

const PLACEHOLDER_DATE: Date = match Date::from_ymd(2026, 9, 25) {
    Some(d) => d,
    None => panic!("placeholder date is invalid"),
};

/// Every citation id the JSON output refers to (register, buckets, relief, scenarios,
/// requirement lines, catalogue items in the plan, and the allocator's harm weights).
pub fn output_ids(a: &Assessment, content: &Content) -> BTreeSet<CitationId> {
    let mut ids: BTreeSet<CitationId> = BTreeSet::new();
    for p in &a.hazards.profiles {
        ids.extend(p.sources.iter().cloned());
    }
    for b in &a.buckets {
        ids.extend(b.sources.iter().cloned());
        if let Some(r) = &b.relief {
            ids.extend(r.sources.iter().cloned());
        }
    }
    for s in &a.consequence.scenarios {
        ids.extend(s.sources.iter().cloned());
    }
    for l in &a.lines {
        ids.extend(l.line.citations.iter().cloned());
    }
    for m in &a.budget.plan.months {
        for i in &m.items {
            if let Some(item) = content.item(i.item_id.as_str()) {
                ids.extend(item.citations.iter().cloned());
            }
        }
    }
    for id in crate::coverage::COVERAGE_CITATIONS {
        ids.insert(CitationId::from(id));
    }
    ids.insert(CitationId::from(rr_budget::weights::HARM_WEIGHT_CITATION));
    ids
}

/// The provenance order: `first` (the packet's order of first use), then the rest sorted.
pub fn order(first: &[CitationId], rest: &BTreeSet<CitationId>) -> Vec<CitationId> {
    let mut out: Vec<CitationId> = Vec::new();
    for id in first.iter().chain(rest.iter()) {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

/// Resolves ids to citations. Unknown ids become placeholders (see the module docs) and are
/// returned so the caller can warn.
///
/// # Panics
///
/// In debug builds, on an unknown id that is not in [`AWAITING_CONTENT`].
pub fn resolve(ids: &[CitationId], content: &Content) -> (Vec<Citation>, Vec<CitationId>) {
    let mut out = Vec::with_capacity(ids.len());
    let mut unknown = Vec::new();
    for id in ids {
        match content.citation(id.as_str()) {
            Some(c) => out.push(c.clone()),
            None => {
                if cfg!(debug_assertions) && !AWAITING_CONTENT.contains(&id.as_str()) {
                    panic!(
                        "citation id `{id}` is not in content/citations.toml: add it there, or \
                         list it under \"Requested\" in docs/CITATION_IDS.md and in \
                         rr_plan::provenance::AWAITING_CONTENT"
                    );
                }
                unknown.push(id.clone());
                out.push(Citation {
                    id: id.clone(),
                    title: format!("Source entry still being added ({id})"),
                    publisher: "Ready Reckoner".to_owned(),
                    year: None,
                    url: PLACEHOLDER_URL.to_owned(),
                    retrieved: PLACEHOLDER_DATE,
                    quote: None,
                    license: "CC BY-SA 4.0".to_owned(),
                    prior: false,
                });
            }
        }
    }
    (out, unknown)
}

/// The warning for placeholder sources.
pub fn missing_warning(unknown: &[CitationId]) -> Option<Warning> {
    if unknown.is_empty() {
        return None;
    }
    Some(Warning {
        id: "citation_missing".to_owned(),
        severity: WarningSeverity::Note,
        message: "A few numbers point to sources that are still being added to the source list."
            .to_owned(),
        why: "The numbers themselves are unchanged. Their source entries are listed as \
              placeholders until the source list catches up."
            .to_owned(),
        related: unknown.iter().map(|c| c.as_str().to_owned()).collect(),
    })
}
