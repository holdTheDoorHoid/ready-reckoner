//! The printable packet (DESIGN §9): Markdown assembled from the plan's numbers and the content
//! guidance blocks, with the household's own numbers substituted. See `docs/PACKET.md` for the
//! sections, what feeds each, and the placeholders.
//!
//! Contract v3 (DESIGN-DELTA-v3 §4): until the binder workstream lands, this v2 packet is
//! `PlanOutput.prepare_markdown`, unchanged, and [`shim`] builds a transitional
//! `PlanOutput.binder` from it.
//!
//! Citations are written as markers while the packet is assembled; once the provenance list is
//! known they become numbers that point into the packet's numbered Sources section ("[3]",
//! "[3, 7]"), so the packet reads the same on paper as on screen.

mod calendar;
mod checklists;
mod family;
mod pages;
mod people;
mod plan;
mod risks;
mod safety;
// transitional: replaced by the binder workstream (DESIGN-DELTA-v3 §4, §11).
pub mod shim;
mod sources;
mod summary;
mod targets;
pub(crate) mod text;

pub use family::{NB_HYPHEN, WALLET_CARDS_HEADING};
pub(crate) use pages::recovery_info;
pub use plan::DETAIL_MONTHS;
pub use risks::{
    CARD_MIN_P10, CARDS as HAZARD_CARDS, FAST_HAZARDS, FREQUENT_CARDS, LIFE_SAFETY_MIN_P10, MINOR,
    SEVERE, WIND_HAZARDS, family_block_prints,
};
pub use safety::{SAFETY_RULES, SafetyRule};
pub use summary::{LEAVE_FIRST_FAST_HAZARDS, LEAVE_FIRST_P10, LEAVE_FIRST_SCENARIOS};
pub use targets::{COPE_SENTENCE, dial_sentence};
pub(crate) use targets::{RecordSpan, relief_fallback};

use std::collections::BTreeMap;

use rr_content::{Content, Guidance};
use rr_types::{BucketId, Citation, CitationId, Item, PlanItem, PlanItemKind};

use crate::pipeline::Assessment;

/// The status line on page 1, under the header block (review C1, RR-P13): the exact words the
/// Start screen also shows.
pub const STATUS_LINE: &str = "Ready Reckoner is an independent, open-source planning aid. It is \
    not official emergency guidance, and not medical, legal or financial advice. Follow \
    instructions from your local officials first.";

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

/// The sections every packet has, in order (a test checks every packet has each one). Packet v2
/// (DESIGN-DELTA §3): the household's own family plan and its wallet cards come right after the
/// summary, the shelter plan and the forecast checklist after the plan, the local pointers,
/// documents and the recovery page after the checklists. Two sections print only when they
/// apply, in the places [`CONDITIONAL_HEADINGS`] names.
pub const SECTION_HEADINGS: [&str; 15] = [
    "## Summary",
    "## Your family plan",
    WALLET_CARDS_HEADING,
    "## Your risks",
    "## Your targets",
    "## Your plan",
    "## Your shelter plan",
    "## When a storm, freeze or heat wave is forecast",
    "## Checklists",
    "## Local help",
    "## Documents and money",
    "## After a disaster: the first 30 days",
    "## Special needs",
    "## Maintenance calendar",
    "## Sources",
];

/// Sections that print only for some households, each with the section it follows: access and
/// functional needs when anyone in the household has one (`Person::access_needs`), and the
/// long-horizon section when the plan has one (`Plan::long_horizon`).
pub const CONDITIONAL_HEADINGS: [(&str, &str); 2] = [
    ("## Access and functional needs", "## Checklists"),
    (
        "## If it lasts for months",
        "## After a disaster: the first 30 days",
    ),
];

/// A citation marker for one id.
pub(crate) fn cite(id: &str) -> String {
    format!("{OPEN}{id}{CLOSE}")
}

/// Escapes text for Markdown ([`text::md`]) but leaves citation markers as they are, for a
/// sentence that already carries its markers.
pub(crate) fn md_marked(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let mut rest = s;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&text::md(&rest[..start]));
        let after = &rest[start..];
        match after.find(CLOSE) {
            Some(end) => {
                out.push_str(&after[..end + CLOSE.len_utf8()]);
                rest = &after[end + CLOSE.len_utf8()..];
            }
            None => {
                out.push_str(&text::md(after));
                rest = "";
            }
        }
    }
    out.push_str(&text::md(rest));
    out
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

    /// Whether advice about a hazard belongs in this household's packet: its ten-year chance
    /// (over the dial's horizon) is at least 1 in 100, the same cut the risk section uses to
    /// list a hazard rather than name it among the faint ones. Unknown ids are kept.
    pub fn hazard_relevant(&self, id: &str) -> bool {
        let Ok(h) = id.parse::<rr_types::HazardId>() else {
            return true;
        };
        let rate = self.a.hazard_rate(h);
        let years = f64::from(self.a.input.dials.horizon_years.max(1));
        let chance = if rate <= 0.0 {
            0.0
        } else {
            -rr_types::math::exp_m1(-rate * years)
        };
        text::per_100(chance) != "fewer than 1"
    }

    /// Whether a conditional span belongs in this household's packet
    /// (`docs/CONTENT_STANDARDS.md` §4, `rr_content::policy::Condition`): a span about one hazard
    /// of a family block (`{if:avalanche}…{/if}`) when that hazard is relevant here
    /// ([`Ctx::hazard_relevant`]); a span about a kind of home (`{if:home:apartment_high_rise}`,
    /// `{if:not_home:…}`) when the household's home is (or is not) of that kind; an access need
    /// (`{if:need:hearing}`), an item (`{if:has:power_generator}`: owned, or in the plan) or a
    /// benefit (`{if:benefit:snap_wic}`) when the household has it.
    pub fn condition_holds(&self, id: &str) -> bool {
        match rr_content::policy::Condition::parse(id) {
            Ok(c) => c.holds(self),
            // Malformed conditions never pass the content validator; keep the text.
            Err(_) => true,
        }
    }

    /// Whether the household has a catalogue item: it lists it as owned (any quantity above 0)
    /// or the plan includes it as a step.
    pub fn has_item(&self, id: &str) -> bool {
        self.a
            .input
            .existing
            .iter()
            .any(|o| o.item_id == id && o.qty > 0.0)
            || self.in_plan(id)
    }

    /// A guidance block's prose with the placeholders filled and its footnotes turned into
    /// citation markers. `frequency` fills `{frequency}` (the placeholder and the space after it
    /// are dropped when there is none); `target` fills `{target}`. Conditional spans stay only
    /// when their condition holds for this household ([`Ctx::condition_holds`]).
    pub fn guidance(&self, g: &Guidance, frequency: Option<&str>, target: Option<&str>) -> String {
        let mut body =
            rr_content::policy::apply_conditions(g.prose().trim(), |id| self.condition_holds(id));
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

/// The household as the guidance blocks' conditions see it (`rr_content::policy::HouseholdFacts`),
/// so the packet keeps exactly the spans the web app keeps.
impl rr_content::policy::HouseholdFacts for Ctx<'_> {
    fn hazard_relevant(&self, hazard: &str) -> bool {
        Ctx::hazard_relevant(self, hazard)
    }

    fn home(&self) -> rr_types::HousingKind {
        self.a.input.housing.kind
    }

    fn has_access_need(&self, need: &str) -> bool {
        self.a
            .input
            .people
            .iter()
            .any(|p| p.access_needs.iter().any(|n| n.as_str() == need))
    }

    fn has_item(&self, item: &str) -> bool {
        Ctx::has_item(self, item)
    }

    fn has_benefit(&self, benefit: &str) -> bool {
        self.a
            .input
            .finances
            .benefits
            .iter()
            .any(|b| b.as_str() == benefit)
    }
}

/// The paragraphs of a rendered guidance block.
pub(crate) fn paragraphs(rendered: &str) -> Vec<String> {
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

/// A topic block's what-to-do: its "What helps" and "What to avoid" paragraphs when it has a
/// "What helps", otherwise every paragraph that opens with a bold heading (a block written as
/// steps, such as drills), otherwise the whole block. The why (opening paragraph, figures) is
/// the app's Learn view.
pub(crate) fn topic_paragraphs(rendered: &str) -> Vec<String> {
    let advice = advice_paragraphs(rendered);
    if advice.iter().any(|p| p.starts_with(HELPS)) {
        return advice;
    }
    headed_paragraphs(rendered)
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

/// A block's paragraphs that open with a bold heading, each with the list that follows it (a
/// heading such as "**Four ways to reach each other.**" introduces a numbered list, which is a
/// paragraph of its own); the opening why and anything else unheaded are left out.
pub(crate) fn headed_with_lists(rendered: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in paragraphs(rendered) {
        let list = p.starts_with("- ")
            || p.starts_with("* ")
            || p.split_once(". ")
                .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if p.starts_with("**") {
            out.push(p);
        } else if list {
            if let Some(last) = out.last_mut() {
                last.push_str("\n\n");
                last.push_str(&p);
            }
        }
    }
    out
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
    family::plan(&cx, &mut out);
    family::wallet_cards(&cx, &mut out);
    risks::write(&cx, &mut out);
    targets::write(&cx, &mut out);
    plan::write(&cx, &mut out);
    pages::shelter(&cx, &mut out);
    pages::forecast(&cx, &mut out);
    checklists::write(&cx, &mut out);
    pages::access_needs(&cx, &mut out);
    pages::local_help(&cx, &mut out);
    people::documents(&cx, &mut out);
    pages::after_disaster(&cx, &mut out);
    pages::long_horizon(&cx, &mut out);
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
