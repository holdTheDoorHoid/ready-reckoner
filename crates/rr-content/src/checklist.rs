//! Incident checklists (DESIGN-DELTA-v3 §5.3, §5.4): `content/checklists/<id>.md`, one airline-style
//! page per hazard or everyday emergency, for tabs 6 to 8 of the binder.
//!
//! # The file
//!
//! ```text
//! ---
//! id: check_house_fire            # file name = id; ids start with check_
//! title: House fire
//! kind: checklist
//! onset: now                      # now | coming | ongoing: tab 6, 7 or 8
//! applies_to: [hazard:house_fire] # hazard:<id> and/or event:<id> (crate::ids::EVENTS)
//! citations: [ready_gov_home_fires, usfa_home_fire_escape_plans]
//! pages: 1                        # 1 (default) or 2; 2 only for check_hurricane, check_nuclear_attack
//! ---
//! ## Use this when
//! ## Do first        (numbered steps `1. **Bold lead.** …`, at most 6)
//! ## Then            (numbered steps, at most 10)
//! ## Leave or stay   (`- ` branches, at most 4)
//! ## Where and who   (`- ` lines, at most 3; the engine adds the household's own fields)
//! ## Do not          (`- ` bullets, at most 5)
//! ## When it is over (`- ` bullets, at most 5)
//! ## Sources         (`[^id]: Publisher, title (year).` footnotes)
//! ```
//!
//! A whole step or bullet may be conditional: the span opens right after `1. ` or `- ` and
//! closes at the end of the line (`4. {if:powered_device}**Switch devices to backup power.** …[^id]{/if}`).
//!
//! # Rendering
//!
//! [`Checklist::render_for`] applies the household's conditional spans, drops a step or bullet
//! left empty (the renderer numbers steps, not the source), and replaces each placeholder of
//! [`PLACEHOLDERS`] with the household's answer or a **blank marker**, [`blank_marker`]:
//! `"\u{3}blank:<n>\u{3}"`, a ruled blank of about `n` characters ([`parse_blank_marker`] reads one
//! back). It keeps `{ref:<page>}` cross-references ([`REF_TARGETS`]) and `[^id]` footnotes exactly as
//! written, and bold leads as Markdown (`**Get out.**`), for rr-plan to turn into inlines.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use rr_types::{CitationId, GuidanceMeta};

use crate::ids::GuidanceKind;
use crate::parse::{LoadError, footnote_definitions_in, footnote_references_in, prose_of};
use crate::policy::{self, HouseholdFacts};

/// The `##` sections of a checklist, in the only order allowed.
pub const SECTION_HEADINGS: [&str; 8] = [
    "Use this when",
    "Do first",
    "Then",
    "Leave or stay",
    "Where and who",
    "Do not",
    "When it is over",
    "Sources",
];

/// Most "Do first" steps: the items known without reading.
pub const MAX_DO_FIRST: usize = 6;
/// Most "Then" steps.
pub const MAX_THEN: usize = 10;
/// Most "Leave or stay" branches.
pub const MAX_LEAVE_OR_STAY: usize = 4;
/// Most "Where and who" lines of the block's own (the engine adds the household's fields).
pub const MAX_WHERE_WHO: usize = 3;
/// Most "Do not" bullets.
pub const MAX_DO_NOT: usize = 5;
/// Most "When it is over" bullets.
pub const MAX_WHEN_OVER: usize = 5;

/// Most words on a one-page checklist ([`Checklist::word_count`]).
pub const WORDS_ONE_PAGE: usize = 330;
/// Most words on a two-page checklist.
pub const WORDS_TWO_PAGES: usize = 620;

/// The checklists allowed two pages (DESIGN-DELTA-v3 §5.2, §5.4).
pub const TWO_PAGE_CHECKLISTS: [&str; 2] = ["check_hurricane", "check_nuclear_attack"];

/// The placeholders a checklist may use (DESIGN-DELTA-v3 §5.4), each with the width of the blank
/// that stands in when the household has not answered, in characters.
pub const PLACEHOLDERS: [(&str, u8); 18] = [
    ("meeting_near", 30),
    ("meeting_far", 30),
    ("shelter_home", 30),
    ("shelter_work", 30),
    ("where_go", 30),
    ("out_of_area_contact", 30),
    ("gas_shutoff", 30),
    ("water_shutoff", 30),
    ("electric_panel", 30),
    ("electric_utility", 24),
    ("gas_utility", 24),
    ("water_utility", 24),
    ("hospital", 30),
    ("pharmacy", 24),
    ("alerts", 30),
    ("county", 20),
    // The community warming, cooling or emergency shelter the household named
    // (`FamilyPlan.neighbourhood.shelter`; suggested by the tab-8 checklists).
    ("shelter", 28),
    // The county emergency management office the household noted, as "name, phone"
    // (`FamilyPlan.neighbourhood.county_emergency_office`; asked for by the checklist review).
    ("county_office", 28),
];

/// The pages a `{ref:<page>}` cross-reference may name: page ids of the binder
/// (DESIGN-DELTA-v3 §4.2, §5.4).
pub const REF_TARGETS: [&str; 7] = [
    "home",
    "getting_out",
    "neighbourhood",
    "contacts",
    "after",
    "documents",
    // The People part's opening page (who is in this binder, with a link to each person).
    "people",
];

/// Opens a cross-reference: `{ref:home}`.
pub const REF_OPEN: &str = "{ref:";

/// Opens and closes a blank marker ([`blank_marker`]). The character never occurs in content.
pub const BLANK_MARK: char = '\u{3}';

/// The marker [`Checklist::render_for`] writes for an unanswered placeholder: a ruled blank of
/// about `width` characters, `"\u{3}blank:<width>\u{3}"`.
pub fn blank_marker(width: u8) -> String {
    format!("{BLANK_MARK}blank:{width}{BLANK_MARK}")
}

/// A blank marker at the start of `text`: its width and the marker's length in bytes.
pub fn parse_blank_marker(text: &str) -> Option<(u8, usize)> {
    let rest = text.strip_prefix(BLANK_MARK)?.strip_prefix("blank:")?;
    let end = rest.find(BLANK_MARK)?;
    let width = rest[..end].parse().ok()?;
    let len = BLANK_MARK.len_utf8() * 2 + "blank:".len() + end;
    Some((width, len))
}

/// The blank width for a placeholder name, or `None` if it is not one of [`PLACEHOLDERS`].
pub fn placeholder_width(name: &str) -> Option<u8> {
    PLACEHOLDERS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, w)| w)
}

/// When an incident unfolds, which decides the tab (DESIGN-DELTA-v3 §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Onset {
    /// It is happening now (tab 6): fire, tornado, a gas leak.
    Now,
    /// It is coming (tab 7): forecast and warned hazards, orders from officials.
    Coming,
    /// It goes on (tab 8): outages and slow crises.
    Ongoing,
}

impl Onset {
    /// Every onset, in tab order.
    pub const ALL: [Onset; 3] = [Onset::Now, Onset::Coming, Onset::Ongoing];

    /// The front-matter word.
    pub const fn as_str(self) -> &'static str {
        match self {
            Onset::Now => "now",
            Onset::Coming => "coming",
            Onset::Ongoing => "ongoing",
        }
    }

    /// The binder tab its checklists go behind: 6, 7 or 8.
    pub const fn tab(self) -> u8 {
        match self {
            Onset::Now => 6,
            Onset::Coming => 7,
            Onset::Ongoing => 8,
        }
    }
}

impl fmt::Display for Onset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Onset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Onset::ALL
            .into_iter()
            .find(|o| o.as_str() == s)
            .ok_or_else(|| format!("unknown onset `{s}` (one of now, coming, ongoing)"))
    }
}

/// A checklist's sections, as the source writes them or as [`Checklist::render_for`] renders
/// them: the text of each step and bullet without its `1. ` or `- ` marker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChecklistSections {
    /// "Use this when": the trigger, as a person would notice it.
    pub use_when: String,
    /// "Do first": the memory items.
    pub do_first: Vec<String>,
    /// "Then".
    pub then: Vec<String>,
    /// "Leave or stay": the branches ("**Leave if** …", "**Stay if** …").
    pub leave_or_stay: Vec<String>,
    /// "Where and who": the block's own lines.
    pub where_who: Vec<String>,
    /// "Do not".
    pub do_not: Vec<String>,
    /// "When it is over".
    pub when_over: Vec<String>,
}

/// One `##` section as the file writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSection {
    /// The heading, without `## `.
    pub heading: String,
    /// The text under it, up to the next heading.
    pub text: String,
}

/// An incident checklist.
#[derive(Debug, Clone, PartialEq)]
pub struct Checklist {
    /// The front matter's `id`, `title`, `applies_to` and `citations`; `kind` is always
    /// `checklist`.
    pub meta: GuidanceMeta,
    /// Which tab it goes behind.
    pub onset: Onset,
    /// Printed pages it may take: 1, or 2 for [`TWO_PAGE_CHECKLISTS`].
    pub pages: u8,
    /// The sections, parsed.
    pub sections: ChecklistSections,
    /// Every `##` section in the order written (the validator checks the order).
    pub raw: Vec<RawSection>,
    /// Text before the first `##` heading (there should be none).
    pub preamble: String,
    /// The Markdown after the front matter, the `## Sources` section included.
    pub body: String,
    /// The file it came from, relative to `content/`.
    pub file: String,
}

impl Checklist {
    /// Whether the block applies to this target (`"hazard:tornado"`, `"event:power_outage"`).
    pub fn applies_to(&self, target: &str) -> bool {
        self.meta.applies_to.iter().any(|a| a == target)
    }

    /// The body before `## Sources`.
    pub fn prose(&self) -> &str {
        prose_of(&self.body)
    }

    /// The footnote definitions under `## Sources`, as `(citation id, text)`.
    pub fn footnote_definitions(&self) -> Vec<(String, String)> {
        footnote_definitions_in(&self.body)
    }

    /// The citation ids used inline as `[^id]`, in order of first use.
    pub fn footnote_references(&self) -> Vec<String> {
        footnote_references_in(self.prose())
    }

    /// Most words allowed: [`WORDS_ONE_PAGE`], or [`WORDS_TWO_PAGES`] for a two-page block.
    pub fn word_budget(&self) -> usize {
        if self.pages >= 2 {
            WORDS_TWO_PAGES
        } else {
            WORDS_ONE_PAGE
        }
    }

    /// The source's steps, branches, lines and bullets, section by section, with "Use this when"
    /// first.
    pub fn items(&self) -> Vec<&str> {
        let s = &self.sections;
        let mut out: Vec<&str> = vec![s.use_when.as_str()];
        for list in [
            &s.do_first,
            &s.then,
            &s.leave_or_stay,
            &s.where_who,
            &s.do_not,
            &s.when_over,
        ] {
            out.extend(list.iter().map(String::as_str));
        }
        out
    }

    /// The words the budget counts: every step, branch, line and bullet with every conditional
    /// span kept (a reader who sees them all), footnote references left out, each placeholder and
    /// `{ref:}` one word; the `##` headings and Sources are not counted.
    pub fn word_count(&self) -> usize {
        self.items()
            .iter()
            .map(|item| {
                let all = policy::apply_conditions(item, |_| true);
                crate::readability::words(&strip_footnotes(&all)).len()
            })
            .sum()
    }

    /// The Flesch-Kincaid grade of the text, every span kept, each placeholder and cross-reference
    /// read as one short word; `None` with no words.
    pub fn reading_grade(&self) -> Option<f64> {
        let text: Vec<String> = self
            .items()
            .iter()
            .map(|item| {
                let all = policy::apply_conditions(item, |_| true);
                replace_braces(&all, |_| Some("it".to_owned()))
            })
            .collect();
        crate::readability::flesch_kincaid_grade(&crate::readability::plain_text(&text.join("\n")))
    }

    /// The sections for one household (see the module docs): conditional spans applied and
    /// emptied steps and bullets dropped; each placeholder of [`PLACEHOLDERS`] replaced with
    /// `fill(name)` (the name without braces, for example `"meeting_near"`; the answer is inserted
    /// exactly as returned, so wrap user text in a marker of your own if you need to tell it apart
    /// from content Markdown) or, when `fill` gives `None`, with [`blank_marker`] of the
    /// placeholder's width; `{ref:<page>}` and `[^id]` kept as written.
    pub fn render_for(
        &self,
        facts: &impl HouseholdFacts,
        fill: &impl Fn(&str) -> Option<String>,
    ) -> ChecklistSections {
        let one = |text: &str| -> String {
            let kept = policy::apply_conditions_for(text, facts);
            fill_placeholders(&kept, fill).trim().to_owned()
        };
        let many = |items: &[String]| -> Vec<String> {
            items
                .iter()
                .map(|i| one(i))
                .filter(|i| !i.is_empty())
                .collect()
        };
        let s = &self.sections;
        ChecklistSections {
            use_when: one(&s.use_when),
            do_first: many(&s.do_first),
            then: many(&s.then),
            leave_or_stay: many(&s.leave_or_stay),
            where_who: many(&s.where_who),
            do_not: many(&s.do_not),
            when_over: many(&s.when_over),
        }
    }
}

/// `text` with each `{placeholder}` of [`PLACEHOLDERS`] replaced by `fill(name)` or a blank
/// marker; every other brace (a cross-reference, an unknown name) is kept as written.
pub fn fill_placeholders(text: &str, fill: &impl Fn(&str) -> Option<String>) -> String {
    replace_braces(text, |inner| {
        placeholder_width(inner).map(|w| fill(inner).unwrap_or_else(|| blank_marker(w)))
    })
}

/// `text` with each `{inner}` for which `with` gives `Some` replaced; the rest kept.
fn replace_braces(text: &str, with: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &after[..end];
        match with(inner) {
            Some(v) => out.push_str(&v),
            None => out.push_str(&rest[start..start + end + 2]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Every `{…}` in `text`, without the braces, in order.
pub fn braces(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            break;
        };
        out.push(&after[..end]);
        rest = &after[end + 1..];
    }
    out
}

/// `text` without its footnote references (`[^id]`).
fn strip_footnotes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("[^") {
        out.push_str(&rest[..start]);
        match rest[start..].find(']') {
            Some(end) => rest = &rest[start + end + 1..],
            None => {
                rest = &rest[start..];
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A step or bullet's text without a leading conditional-span marker (`{if:pets}`), for the rules
/// about how it opens.
pub fn without_leading_condition(item: &str) -> &str {
    let t = item.trim_start();
    if t.starts_with(policy::CONDITION_OPEN)
        && let Some(end) = t.find('}')
    {
        return t[end + 1..].trim_start();
    }
    t
}

/// A list section scanned: its items (text after the marker, continuation lines joined with a
/// space) and the lines that do not belong in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListScan {
    /// The items, in order.
    pub items: Vec<String>,
    /// Plain-language problems: text outside an item, or the other kind of list marker.
    pub problems: Vec<String>,
}

/// The marker a list line opens with and the text after it: `1. ` (numbered) or `- `.
fn list_marker(line: &str) -> Option<(bool, &str)> {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix("- ") {
        return Some((false, rest));
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if digits > 0
        && let Some(rest) = t[digits..].strip_prefix(". ")
    {
        return Some((true, rest));
    }
    None
}

/// Scans a list section: numbered steps (`numbered`) or `- ` bullets.
pub fn scan_list(text: &str, numbered: bool, section: &str) -> ListScan {
    let mut scan = ListScan::default();
    let mut open = false;
    for line in text.lines() {
        if line.trim().is_empty() {
            open = false;
            continue;
        }
        match list_marker(line) {
            Some((is_numbered, rest)) if is_numbered == numbered => {
                scan.items.push(rest.trim().to_owned());
                open = true;
            }
            Some(_) => {
                let want = if numbered {
                    "numbered steps (`1. **Lead words.** …`)"
                } else {
                    "`- ` bullets"
                };
                scan.problems.push(format!(
                    "`{section}` holds {want}; `{}` is the other kind",
                    line.trim()
                ));
                open = false;
            }
            None if open => {
                if let Some(last) = scan.items.last_mut() {
                    last.push(' ');
                    last.push_str(line.trim());
                }
            }
            None => scan.problems.push(format!(
                "`{section}` has text outside its {}: `{}`",
                if numbered { "steps" } else { "bullets" },
                line.trim()
            )),
        }
    }
    scan
}

/// The sections of a checklist body, and the text before the first heading.
fn split_sections(body: &str) -> (String, Vec<RawSection>) {
    let mut preamble = String::new();
    let mut raw: Vec<RawSection> = Vec::new();
    for line in body.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            raw.push(RawSection {
                heading: h.trim().to_owned(),
                text: String::new(),
            });
            continue;
        }
        let text = match raw.last_mut() {
            Some(section) => &mut section.text,
            None => &mut preamble,
        };
        text.push_str(line);
        text.push('\n');
    }
    (preamble, raw)
}

/// The sections by heading (the first of each; the validator reports repeats and order).
fn sections_of(raw: &[RawSection]) -> ChecklistSections {
    let text = |h: &str| {
        raw.iter()
            .find(|s| s.heading == h)
            .map(|s| s.text.as_str())
            .unwrap_or("")
    };
    let use_when = text("Use this when")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let steps = |h: &str| scan_list(text(h), true, h).items;
    let bullets = |h: &str| scan_list(text(h), false, h).items;
    ChecklistSections {
        use_when,
        do_first: steps("Do first"),
        then: steps("Then"),
        leave_or_stay: bullets("Leave or stay"),
        where_who: bullets("Where and who"),
        do_not: bullets("Do not"),
        when_over: bullets("When it is over"),
    }
}

/// Parses one checklist file (see the module docs).
pub(crate) fn parse_checklist(path: &str, text: &str) -> Result<Checklist, LoadError> {
    let (fields, body): (BTreeMap<String, String>, String) = crate::parse::front_matter(
        path,
        text,
        "a checklist",
        &[
            "id",
            "title",
            "kind",
            "onset",
            "applies_to",
            "citations",
            "pages",
        ],
    )?;
    let need = |name: &str| crate::parse::required(path, &fields, name);
    let id = need("id")?;
    let title = need("title")?;
    let kind = need("kind")?;
    if kind != GuidanceKind::Checklist.as_str() {
        return Err(LoadError::new(
            path,
            format!("`kind: {kind}`: files in checklists/ are `kind: checklist`"),
        ));
    }
    let onset = need("onset")?
        .parse::<Onset>()
        .map_err(|e| LoadError::new(path, e))?;
    let applies_to = crate::parse::parse_list(path, "applies_to", &need("applies_to")?)?;
    let citations = crate::parse::parse_list(path, "citations", &need("citations")?)?
        .into_iter()
        .map(CitationId::new)
        .collect();
    let pages = match fields.get("pages") {
        None => 1,
        Some(v) => v
            .trim()
            .parse::<u8>()
            .map_err(|_| LoadError::new(path, format!("`pages: {v}` is not a number of pages")))?,
    };
    let (preamble, raw) = split_sections(&body);
    let sections = sections_of(&raw);
    Ok(Checklist {
        meta: GuidanceMeta {
            id,
            title,
            applies_to,
            citations,
            kind: Some(GuidanceKind::Checklist),
        },
        onset,
        pages,
        sections,
        raw,
        preamble,
        body,
        file: path.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "---\nid: check_x\ntitle: X\nkind: checklist\nonset: coming\n\
        applies_to: [hazard:tornado, event:power_outage]\ncitations: [ready_gov_tornadoes]\n---\n\
        ## Use this when\n\nA warning is issued\nfor your area.\n\n\
        ## Do first\n\n1. **Go in.** Get to {shelter_home} now.[^ready_gov_tornadoes]\n\
        2. {if:pets}**Bring the pets.** Leashes too.[^ready_gov_tornadoes]{/if}\n\
        3. **Cover up.** Protect your head\n   with your arms.[^ready_gov_tornadoes]\n\n\
        ## Then\n\n1. **Listen.** Tune to {alerts}.[^ready_gov_tornadoes]\n\n\
        ## Leave or stay\n\n- **Leave if** told to. Go to {where_go}. {ref:getting_out}\n\n\
        ## Where and who\n\n- Meet at: {meeting_near}\n\n\
        ## Do not\n\n- Do not open windows.[^ready_gov_tornadoes]\n\n\
        ## When it is over\n\n- Check on neighbours in {county}.[^ready_gov_tornadoes]\n- Start the after pages. {ref:after}\n\n\
        ## Sources\n\n[^ready_gov_tornadoes]: FEMA / Ready.gov, Tornadoes (2026).\n";

    struct Home {
        pets: bool,
    }

    impl HouseholdFacts for Home {
        fn hazard_relevant(&self, _: &str) -> bool {
            true
        }
        fn home(&self) -> rr_types::HousingKind {
            rr_types::HousingKind::Detached
        }
        fn has_access_need(&self, _: &str) -> bool {
            false
        }
        fn has_item(&self, _: &str) -> bool {
            false
        }
        fn has_benefit(&self, _: &str) -> bool {
            false
        }
        fn has_children(&self) -> bool {
            false
        }
        fn has_pets(&self) -> bool {
            self.pets
        }
        fn has_vehicle(&self) -> bool {
            false
        }
        fn has_powered_device(&self) -> bool {
            false
        }
    }

    fn block() -> Checklist {
        parse_checklist("checklists/check_x.md", BLOCK).unwrap()
    }

    #[test]
    fn parses_the_front_matter_and_the_sections() {
        let c = block();
        assert_eq!(c.meta.id, "check_x");
        assert_eq!(c.meta.kind, Some(GuidanceKind::Checklist));
        assert_eq!(c.onset, Onset::Coming);
        assert_eq!(c.onset.tab(), 7);
        assert_eq!(c.pages, 1, "pages defaults to 1");
        assert!(c.applies_to("event:power_outage") && !c.applies_to("hazard:hail"));
        let s = &c.sections;
        assert_eq!(s.use_when, "A warning is issued for your area.");
        assert_eq!(s.do_first.len(), 3);
        assert_eq!(
            s.do_first[2], "**Cover up.** Protect your head with your arms.[^ready_gov_tornadoes]",
            "continuation lines join the step"
        );
        assert_eq!(s.then.len(), 1);
        assert_eq!(s.leave_or_stay.len(), 1);
        assert_eq!(s.where_who, ["Meet at: {meeting_near}"]);
        assert_eq!(s.when_over.len(), 2);
        let headings: Vec<&str> = c.raw.iter().map(|r| r.heading.as_str()).collect();
        assert_eq!(headings, SECTION_HEADINGS);
        assert!(c.preamble.trim().is_empty());
        assert_eq!(c.footnote_references(), ["ready_gov_tornadoes"]);
        assert_eq!(c.footnote_definitions()[0].0, "ready_gov_tornadoes");
    }

    #[test]
    fn render_for_fills_or_blanks_and_drops_empty_steps() {
        let c = block();
        let answers = |name: &str| match name {
            "shelter_home" => Some("the basement".to_owned()),
            "county" => Some("Ward County, North Dakota".to_owned()),
            _ => None,
        };
        let with_pets = c.render_for(&Home { pets: true }, &answers);
        assert_eq!(
            with_pets.do_first,
            [
                "**Go in.** Get to the basement now.[^ready_gov_tornadoes]",
                "**Bring the pets.** Leashes too.[^ready_gov_tornadoes]",
                "**Cover up.** Protect your head with your arms.[^ready_gov_tornadoes]",
            ]
        );
        let blank30 = blank_marker(30);
        assert_eq!(
            with_pets.leave_or_stay,
            [format!(
                "**Leave if** told to. Go to {blank30}. {{ref:getting_out}}"
            )]
        );
        assert_eq!(
            with_pets.then,
            [format!(
                "**Listen.** Tune to {blank30}.[^ready_gov_tornadoes]"
            )]
        );
        assert_eq!(with_pets.where_who, [format!("Meet at: {blank30}")]);
        assert_eq!(
            with_pets.when_over,
            [
                "Check on neighbours in Ward County, North Dakota.[^ready_gov_tornadoes]",
                "Start the after pages. {ref:after}",
            ]
        );
        // Without pets the conditional step disappears, and nothing else changes.
        let no_pets = c.render_for(&Home { pets: false }, &answers);
        assert_eq!(no_pets.do_first.len(), 2);
        assert_eq!(no_pets.do_first[1], with_pets.do_first[2]);
        // A household with no answers at all gets blanks everywhere a placeholder was.
        let nothing = c.render_for(&Home { pets: false }, &|_: &str| None);
        assert_eq!(
            nothing.do_first[0],
            format!("**Go in.** Get to {blank30} now.[^ready_gov_tornadoes]")
        );
        assert!(nothing.when_over[0].contains(&blank_marker(20)));
        for text in nothing.do_first.iter().chain(&nothing.where_who) {
            assert!(!text.contains("{shelter_home}") && !text.contains("{meeting_near}"));
        }
    }

    #[test]
    fn blank_markers_read_back() {
        let m = blank_marker(24);
        assert_eq!(m, "\u{3}blank:24\u{3}");
        assert_eq!(parse_blank_marker(&m), Some((24, m.len())));
        assert_eq!(
            parse_blank_marker(&format!("{m} and on")),
            Some((24, m.len()))
        );
        assert_eq!(parse_blank_marker("blank:24"), None);
        assert_eq!(parse_blank_marker("\u{3}blank:x\u{3}"), None);
        assert_eq!(placeholder_width("county"), Some(20));
        assert_eq!(placeholder_width("frequency"), None);
    }

    #[test]
    fn words_count_every_span_and_one_per_placeholder() {
        let c = block();
        // "A warning is issued for your area." = 7, then the steps and bullets.
        let mut want = 7;
        for item in [
            "Go in. Get to shelter_home now.",
            "Bring the pets. Leashes too.",
            "Cover up. Protect your head with your arms.",
            "Listen. Tune to alerts.",
            "Leave if told to. Go to where_go. ref:getting_out",
            "Meet at: meeting_near",
            "Do not open windows.",
            "Check on neighbours in county.",
            "Start the after pages. ref:after",
        ] {
            want += item.split_whitespace().count();
        }
        assert_eq!(c.word_count(), want);
        assert!(c.reading_grade().is_some_and(|g| g < 8.0));
    }

    #[test]
    fn scan_list_reports_what_does_not_belong() {
        let s = scan_list(
            "1. **A.** a\n- b\n\nloose text\n2. **C.** c\n",
            true,
            "Do first",
        );
        assert_eq!(s.items, ["**A.** a", "**C.** c"]);
        assert_eq!(s.problems.len(), 2, "{:?}", s.problems);
        assert!(s.problems[0].contains("the other kind"));
        assert!(s.problems[1].contains("text outside its steps"));
        let b = scan_list("- one\n  more\n- two\n", false, "Do not");
        assert_eq!(b.items, ["one more", "two"]);
        assert!(b.problems.is_empty());
        assert_eq!(
            without_leading_condition("{if:pets}**Go.** x{/if}"),
            "**Go.** x{/if}"
        );
        assert_eq!(without_leading_condition("**Go.** x"), "**Go.** x");
    }

    #[test]
    fn front_matter_problems_are_load_errors() {
        let e = parse_checklist(
            "checklists/check_x.md",
            &BLOCK.replace("onset: coming", "onset: soon"),
        )
        .unwrap_err();
        assert!(e.message.contains("soon"), "{e}");
        let e = parse_checklist(
            "checklists/check_x.md",
            &BLOCK.replace("kind: checklist", "kind: hazard"),
        )
        .unwrap_err();
        assert!(e.message.contains("kind: checklist"), "{e}");
        let e = parse_checklist(
            "checklists/check_x.md",
            &BLOCK.replace("onset: coming\n", "onset: coming\npages: two\n"),
        )
        .unwrap_err();
        assert!(e.message.contains("pages"), "{e}");
        let e = parse_checklist(
            "checklists/check_x.md",
            &BLOCK.replace("onset: coming\n", ""),
        )
        .unwrap_err();
        assert!(e.message.contains("missing `onset`"), "{e}");
        let e = parse_checklist(
            "checklists/check_x.md",
            &BLOCK.replace("title: X\n", "title: X\nreading_level: 8\n"),
        )
        .unwrap_err();
        assert!(e.message.contains("reading_level"), "{e}");
        let two = parse_checklist(
            "checklists/check_x.md",
            &BLOCK.replace("onset: coming\n", "onset: coming\npages: 2\n"),
        )
        .unwrap();
        assert_eq!((two.pages, two.word_budget()), (2, WORDS_TWO_PAGES));
    }
}
