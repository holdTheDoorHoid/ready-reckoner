//! The printable packet (DESIGN §9): Markdown assembled from the plan's numbers and the content
//! guidance blocks, with the household's own numbers substituted. See `docs/PACKET.md` for the
//! sections, what feeds each, and the placeholders.
//!
//! Citations are written as markers while the packet is assembled; once the provenance list is
//! known they become numbers that point into the packet's numbered Sources section ("[3]",
//! "[3, 7]"), so the packet reads the same on paper as on screen.

mod calendar;
mod checklists;
mod people;
mod plan;
mod risks;
mod sources;
mod summary;
mod targets;
pub(crate) mod text;

pub use plan::DETAIL_MONTHS;
pub use risks::{CARD_MIN_P10, CARDS as HAZARD_CARDS};

use std::collections::BTreeMap;

use rr_content::{Content, Guidance};
use rr_types::{BucketId, Citation, CitationId, Item, PlanItem, PlanItemKind};

use crate::pipeline::Assessment;

/// Marks where a citation marker starts and ends while the packet is assembled. Neither character
/// can occur in content text.
const OPEN: char = '\u{1}';
const CLOSE: char = '\u{2}';

/// Placeholders a guidance block may contain; see `docs/PACKET.md`.
pub const PLACEHOLDERS: [&str; 5] = [
    "{frequency}",
    "{target}",
    "{county}",
    "{horizon}",
    "{household}",
];

/// The section headings, in order (a test checks every packet has each one).
pub const SECTION_HEADINGS: [&str; 10] = [
    "## Summary",
    "## Your risks",
    "## Your targets",
    "## Your plan",
    "## Checklists",
    "## Family plan",
    "## Documents and money",
    "## Special needs",
    "## Maintenance calendar",
    "## Sources",
];

/// A citation marker for one id.
pub(crate) fn cite(id: &str) -> String {
    format!("{OPEN}{id}{CLOSE}")
}

/// Markers for several ids (deduplicated, in order).
pub(crate) fn cite_all<'a>(ids: impl IntoIterator<Item = &'a CitationId>) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for id in ids {
        if !seen.contains(&id.as_str()) {
            seen.push(id.as_str());
        }
    }
    seen.into_iter().map(cite).collect()
}

/// What the sections share.
pub(crate) struct Ctx<'a> {
    pub a: &'a Assessment,
    pub content: &'a Content,
}

impl<'a> Ctx<'a> {
    /// The catalogue item with this id.
    pub fn item(&self, id: &str) -> Option<&'a Item> {
        self.content.item(id)
    }

    /// Every plan item with the month it is in.
    pub fn plan_items(&self) -> Vec<(u16, &'a PlanItem)> {
        self.a
            .budget
            .plan
            .months
            .iter()
            .flat_map(|m| m.items.iter().map(move |i| (m.index, i)))
            .collect()
    }

    /// Plan items that are steps (free actions and purchases, not savings deposits).
    pub fn steps(&self) -> Vec<(u16, &'a PlanItem)> {
        self.plan_items()
            .into_iter()
            .filter(|(_, i)| i.kind != PlanItemKind::Reserve)
            .collect()
    }

    /// Whether the plan includes this catalogue item (as a step).
    pub fn in_plan(&self, id: &str) -> bool {
        self.steps().iter().any(|(_, i)| i.item_id == id)
    }

    /// The first frequency sentence of a bucket.
    pub fn bucket_frequency(&self, b: BucketId) -> Option<&'a str> {
        self.a
            .bucket(b)
            .frequency_sentences
            .first()
            .map(String::as_str)
    }

    /// A guidance block's prose with the placeholders filled and its footnotes turned into
    /// citation markers. `frequency` fills `{frequency}` (the placeholder and the space after it
    /// are dropped when there is none); `target` fills `{target}`.
    pub fn guidance(&self, g: &Guidance, frequency: Option<&str>, target: Option<&str>) -> String {
        let mut body = g.prose().trim().to_owned();
        let county = format!(
            "{}, {}",
            self.a.location.county_name, self.a.location.state_name
        );
        let horizon = rr_consequence::words::horizon_phrase(self.a.input.dials.horizon_years);
        let household = text::household(&self.a.input);
        let fills: [(&str, Option<&str>); 5] = [
            ("{frequency}", frequency),
            ("{target}", target),
            ("{county}", Some(county.as_str())),
            ("{horizon}", Some(horizon.as_str())),
            ("{household}", Some(household.as_str())),
        ];
        for (key, value) in fills {
            body = match value {
                Some(v) => body.replace(key, v),
                None => body.replace(&format!("{key} "), "").replace(key, ""),
            };
        }
        footnotes_to_markers(&body)
    }

    /// The guidance blocks that apply to a target such as `bucket:power` or `tier:w2`.
    pub fn blocks_for(&self, target: &str) -> Vec<&'a Guidance> {
        self.content
            .guidance
            .iter()
            .filter(|g| g.meta.applies_to.iter().any(|x| x == target))
            .collect()
    }

    /// The catalogue item's spec with its citations, for advice that comes straight from the
    /// catalogue (family plan, documents, special needs).
    pub fn item_advice(&self, id: &str) -> Option<String> {
        let it = self.item(id)?;
        Some(format!(
            "**{}.** {}{}",
            text::md(&it.name),
            it.spec,
            cite_all(&it.citations)
        ))
    }
}

/// The paragraphs of a rendered guidance block.
fn paragraphs(rendered: &str) -> Vec<String> {
    rendered
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The "what to do" paragraphs of a rendered guidance block: the ones that open with
/// "**What helps.**" or "**What to avoid.**" (none when the block has neither).
pub(crate) fn advice_paragraphs(rendered: &str) -> Vec<String> {
    paragraphs(rendered)
        .into_iter()
        .filter(|p| p.starts_with(HELPS) || p.starts_with(AVOID))
        .collect()
}

/// The paragraph of a rendered guidance block that says what to do ("**What helps.**"), or its
/// advice paragraphs, or the whole block when it has neither.
pub(crate) fn helps_paragraph(rendered: &str) -> Vec<String> {
    let advice = advice_paragraphs(rendered);
    if let Some(p) = advice.iter().find(|p| p.starts_with(HELPS)) {
        return vec![p.clone()];
    }
    if advice.is_empty() {
        paragraphs(rendered)
    } else {
        advice
    }
}

/// A topic block without its opening paragraph (the why, which the app's Learn view carries):
/// the paragraphs that open with a bold heading, or the whole block when none does.
pub(crate) fn headed_paragraphs(rendered: &str) -> Vec<String> {
    let all = paragraphs(rendered);
    let headed: Vec<String> = all
        .iter()
        .filter(|p| p.starts_with("**"))
        .cloned()
        .collect();
    if headed.is_empty() { all } else { headed }
}

/// How a guidance block's advice paragraphs open.
const HELPS: &str = "**What helps.**";
const AVOID: &str = "**What to avoid.**";

/// Turns `[^id]` footnote references into citation markers.
fn footnotes_to_markers(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("[^") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find(']') {
            Some(end) => {
                out.push_str(&cite(after[..end].trim()));
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// The packet with citation markers, sections 1 to 9. The Sources section is added by
/// [`finish`] once the provenance list is known.
pub(crate) fn render(a: &Assessment, content: &Content) -> String {
    let cx = Ctx { a, content };
    let mut out: Vec<String> = Vec::new();
    summary::write(&cx, &mut out);
    risks::write(&cx, &mut out);
    targets::write(&cx, &mut out);
    plan::write(&cx, &mut out);
    checklists::write(&cx, &mut out);
    people::family(&cx, &mut out);
    people::documents(&cx, &mut out);
    people::special_needs(&cx, &mut out);
    calendar::write(&cx, &mut out);
    out.join("\n")
}

/// Citation ids in the order the packet first uses them.
pub(crate) fn cited_ids(marked: &str) -> Vec<CitationId> {
    let mut out: Vec<CitationId> = Vec::new();
    let mut rest = marked;
    while let Some(start) = rest.find(OPEN) {
        let after = &rest[start + OPEN.len_utf8()..];
        let Some(end) = after.find(CLOSE) else {
            break;
        };
        let id = CitationId::from(&after[..end]);
        if !out.contains(&id) {
            out.push(id);
        }
        rest = &after[end + CLOSE.len_utf8()..];
    }
    out
}

/// Replaces the markers with source numbers and appends the Sources section.
pub(crate) fn finish(marked: &str, a: &Assessment, provenance: &[Citation]) -> String {
    let index: BTreeMap<&str, usize> = provenance
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.as_str(), i + 1))
        .collect();
    let mut out = String::with_capacity(marked.len() + 4096);
    let mut rest = marked;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        // A run of adjacent markers becomes one bracket: "[3, 7]".
        let mut numbers: Vec<usize> = Vec::new();
        let mut cursor = &rest[start..];
        while let Some(after) = cursor.strip_prefix(OPEN) {
            let Some(end) = after.find(CLOSE) else {
                break;
            };
            if let Some(n) = index.get(&after[..end]) {
                numbers.push(*n);
            }
            cursor = &after[end + CLOSE.len_utf8()..];
        }
        numbers.sort_unstable();
        numbers.dedup();
        if !numbers.is_empty() {
            let list: Vec<String> = numbers.iter().map(|n| n.to_string()).collect();
            out.push_str(&format!("[{}]", list.join(", ")));
        }
        rest = cursor;
    }
    out.push_str(rest);
    // The packet lists the sources its brackets point to: they come first in the provenance
    // order. The others (behind quantities, prices and warnings the packet does not quote) stay
    // in the plan's provenance list.
    let cited = cited_ids(marked);
    let shown = provenance
        .iter()
        .take_while(|c| cited.contains(&c.id))
        .count();
    let mut tail: Vec<String> = Vec::new();
    sources::write(a, &provenance[..shown], provenance.len() - shown, &mut tail);
    out.push('\n');
    out.push_str(&tail.join("\n"));
    // One trailing newline, no trailing spaces except Markdown line breaks.
    let trimmed = out.trim_end().to_owned();
    format!("{trimmed}\n")
}
