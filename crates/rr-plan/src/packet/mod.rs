//! The Prepare sheet (DESIGN-DELTA-v3 §4: `PlanOutput::prepare_markdown`) and the v2 packet's
//! writers the binder still uses. The Prepare sheet is the v2 packet's preparation content,
//! unchanged in substance: the summary's step reached and the plan's two done months with the
//! basics it assumes, "Your plan" (the budget, the free steps and the safety rules to learn now,
//! this month and next), the checklists of purchases by step, the decisions and savings of
//! "Documents and money", and the maintenance calendar, with its own numbered Sources at the end.
//! The during-event material is the binder's (`crate::binder`).
//!
//! Citations are written as markers while a document is assembled; once the order of first use is
//! known they become numbers that point into its numbered Sources ("[3]", "[3, 7]"), so it reads
//! the same on paper as on screen.

pub(crate) mod calendar;
mod checklists;
pub(crate) mod pages;
pub(crate) mod people;
mod plan;
pub(crate) mod risks;
mod safety;
pub(crate) mod sources;
pub(crate) mod summary;
pub(crate) mod targets;
pub(crate) mod text;

pub(crate) use pages::recovery_info;
pub use plan::DETAIL_MONTHS;
pub use risks::CARD_MIN_P10;
pub use safety::{SAFETY_RULES, SafetyRule};
pub use summary::{LEAVE_FIRST_FAST_HAZARDS, LEAVE_FIRST_P10, LEAVE_FIRST_SCENARIOS};
pub use targets::{COPE_SENTENCE, dial_sentence};
pub(crate) use targets::{RecordSpan, relief_fallback};

/// A non-breaking hyphen (U+2011): phone numbers on the wallet cards use it so a number never
/// breaks across two lines of a card.
pub const NB_HYPHEN: char = '\u{2011}';

use std::collections::BTreeMap;

use rr_content::{Content, Guidance};
use rr_types::{BucketId, Citation, CitationId, Item, PlanItem, PlanItemKind};

use crate::pipeline::Assessment;

/// The status line on page 1, under the header block (review C1, RR-P13): the exact words the
/// Start screen also shows.
pub const STATUS_LINE: &str = "Ready Reckoner is an independent, open-source planning aid. It is \
    not official emergency guidance, and not medical, legal or financial advice. Follow \
    instructions from your local officials first.";

/// Marks where a citation marker starts while a document is assembled. Neither marker character
/// can occur in content text.
pub(crate) const OPEN: char = '\u{1}';
/// Marks where a citation marker ends.
pub(crate) const CLOSE: char = '\u{2}';

/// Placeholders a guidance block may contain; see `docs/PACKET.md`.
pub const PLACEHOLDERS: [&str; 5] = [
    "{frequency}",
    "{target}",
    "{county}",
    "{horizon}",
    "{household}",
];

/// The Prepare sheet's sections, in order (a test checks every sheet has each one).
pub const SECTION_HEADINGS: [&str; 6] = [
    "## Summary",
    "## Your plan",
    "## Checklists",
    "## Documents and money: decisions and savings",
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
    /// benefit (`{if:benefit:snap_wic}`) when the household has it; `{if:children}`,
    /// `{if:pets}`, `{if:vehicle}` and `{if:powered_device}` (contract v3) from the household's
    /// own answers.
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
        // The county's full name where the pack has it ("Richmond city, Virginia"), else the
        // short one ("Philadelphia, Pennsylvania").
        let county = format!(
            "{}, {}",
            self.a
                .county
                .name_full
                .as_deref()
                .unwrap_or(&self.a.location.county_name),
            self.a.location.state_name
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

    fn has_children(&self) -> bool {
        self.a.input.people.iter().any(|p| {
            matches!(
                p.age_band,
                rr_types::AgeBand::Infant
                    | rr_types::AgeBand::Toddler
                    | rr_types::AgeBand::Child
                    | rr_types::AgeBand::Teen
            )
        })
    }

    fn has_pets(&self) -> bool {
        let p = &self.a.input.pets;
        u16::from(p.dogs) + u16::from(p.cats) + u16::from(p.small) + u16::from(p.large_animals) > 0
    }

    fn has_vehicle(&self) -> bool {
        !self.a.input.mobility.vehicles.is_empty()
    }

    fn has_powered_device(&self) -> bool {
        self.a
            .input
            .people
            .iter()
            .any(|p| p.medical.powered_device != rr_types::PoweredDevice::None)
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
pub(crate) fn footnotes_to_markers(s: &str) -> String {
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

/// The Prepare sheet with citation markers; its Sources section is added by [`finish`] once the
/// order of first use is known.
pub(crate) fn render(a: &Assessment, content: &Content) -> String {
    let cx = Ctx { a, content };
    let mut out: Vec<String> = Vec::new();
    summary::write(&cx, &mut out);
    plan::write(&cx, &mut out);
    checklists::write(&cx, &mut out);
    people::decisions_and_savings(&cx, &mut out);
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

/// Replaces the markers with source numbers and appends the Sources section: the sources are
/// `citations` (the ids [`cited_ids`] finds, in that order, resolved), numbered from 1, so the
/// sheet's numbers are its own.
pub(crate) fn finish(marked: &str, a: &Assessment, citations: &[Citation]) -> String {
    let index: BTreeMap<&str, usize> = citations
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
    let mut tail: Vec<String> = Vec::new();
    sources::write(a, citations, &mut tail);
    out.push('\n');
    out.push_str(&tail.join("\n"));
    // One trailing newline, no trailing spaces except Markdown line breaks.
    let trimmed = out.trim_end().to_owned();
    format!("{trimmed}\n")
}
