//! The binder (DESIGN-DELTA-v3 §4, §5): the during-event document `assess` returns in
//! `PlanOutput::binder`, a tree of ten parts (the tabs of a printed binder), pages and blocks,
//! assembled from the plan's numbers, the household's own answers (echoed, never computed with)
//! and the content's guidance and checklist blocks. [`markdown::render`] prints it for the CLI
//! and the goldens; the web app renders the same tree to HTML and PDF.
//!
//! | Tab | Part | Pages |
//! | --- | --- | --- |
//! | 1 | `start` | `cover`, `how_to_use`, `quick_start`, `index` (Which checklist?), `contacts` |
//! | 2 | `people` | `people` (who is in this binder), `person_1` … (one per person), `special_needs`, `wallet_cards` |
//! | 3 | `home_places` | `home`, `place_1` … (one per distinct place), `neighbourhood`, `getting_out` |
//! | 4 | `pets_vehicles_documents` | `pets` (with animals), `vehicles` (with vehicles), `documents` |
//! | 5 | `have` | `inventory`, `what_to_expect`, `risks_glance` |
//! | 6 | `check_now` | one page per checklist block (`check_…`), everyday emergencies first |
//! | 7 | `check_coming` | `forecast`, then the checklists |
//! | 8 | `check_ongoing` | the checklists, `check_something_else` last |
//! | 9 | `after` | `after`, `after_months` (with a long-horizon plan), four `log_…` pages |
//! | 10 | `sources` | `sources` |
//!
//! Citations are numbered in the order the binder first uses them ([`Draft::cited`]); the
//! provenance list starts with them, so `cite` n is `PlanOutput::provenance[n - 1]` as well as
//! `Binder::sources[n - 1]`.

mod after;
mod checklists;
pub mod fit;
mod have;
pub(crate) mod inline;
pub mod markdown;
mod people;
mod pets_docs;
mod places;
mod sources;
mod start;

use std::cell::RefCell;
use std::collections::BTreeMap;

use rr_content::Content;
use rr_types::binder::{Binder, Block, FieldRow, Inline, Page, Part, SourceEntry};
use rr_types::{Attribution, Citation, CitationId, Contact};

use crate::packet::{Ctx, STATUS_LINE, text};
use crate::pipeline::Assessment;
use crate::source::CountyHospitals;

pub(crate) use checklists::select;
pub use checklists::{EVENT_NAMES, SOMETHING_ELSE};
pub use inline::text_of;
pub use start::{ALIASES, POISON_HELP};

/// When `true`, a ranked hazard, an everyday emergency or an opted-in rare family with no
/// checklist block fails `every_ranked_hazard_has_a_checklist`; while it is `false` the binder
/// lists the gap in [`problems`] and the index points to the "Something else" page. The planner
/// turns it on once the four checklist content groups are merged (DESIGN-DELTA-v3 §5.1).
pub const REQUIRE_ALL_CHECKLISTS: bool = true;

/// The ten parts of DESIGN-DELTA-v3 §4.2: tab, id, title and tab label (at most
/// [`rr_types::binder::SHORT_TITLE_MAX`] characters).
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

/// The pages every binder has, with their tab and title: the targets of the checklists'
/// `{ref:…}` cross-references (`rr_content::checklist::REF_TARGETS`) among them.
pub const FIXED_PAGES: [(&str, u8, &str); 18] = [
    ("cover", 1, "Cover"),
    ("how_to_use", 1, "How to use this binder"),
    (
        "quick_start",
        1,
        "Quick start: the first five minutes of any emergency",
    ),
    ("index", 1, "Which checklist?"),
    ("contacts", 1, "Contacts at a glance"),
    ("people", 2, "Who is in this binder"),
    ("special_needs", 2, "Special needs and health"),
    ("wallet_cards", 2, "Wallet cards"),
    ("home", 3, "Home"),
    ("neighbourhood", 3, "Neighborhood"),
    ("getting_out", 3, "Getting out"),
    ("documents", 4, "Documents and money"),
    ("inventory", 5, "What you have"),
    ("what_to_expect", 5, "What to expect"),
    ("risks_glance", 5, "Risks at a glance"),
    ("forecast", 7, FORECAST_TITLE),
    ("after", 9, "After a disaster: the first 30 days"),
    ("sources", 10, "Sources"),
];

/// The title of tab 7's first page, built from `plan_forecast_48h`.
pub const FORECAST_TITLE: &str = "When a storm, freeze or heat wave is forecast";

/// The map slots the web app fills (DESIGN-DELTA-v3 §9.2): slot id, kind, and the page it is on.
pub const MAP_SLOTS: [(&str, &str, &str); 3] = [
    ("map-neighbourhood", "neighbourhood", "neighbourhood"),
    ("map-area", "area", "getting_out"),
    ("map-region", "region", "getting_out"),
];

/// A line to write on, in the Markdown and in words counted by the fit proxy.
pub const BLANK_MD: &str = "__________";

/// What the binder needs besides the assessment: the county hospital list and the versions for
/// "How this binder was made".
#[derive(Debug, Clone, Copy)]
pub struct Extras<'a> {
    /// The county's hospitals with emergency services, when the list is loaded.
    pub hospitals: Option<&'a CountyHospitals>,
    /// The engine's version.
    pub engine_version: &'a str,
    /// The data pack's version.
    pub data_pack_version: &'a str,
    /// The content's version.
    pub content_version: &'a str,
}

/// What the pages share while the binder is assembled: the packet's context (the assessment
/// and the content), the extras, the directory of pages links point to, and the citations in
/// the order first interned (renumbered by first use once the tree is built).
pub(crate) struct Bx<'a> {
    pub(crate) cx: Ctx<'a>,
    pub(crate) extras: Extras<'a>,
    pub(crate) directory: BTreeMap<String, (u8, String)>,
    cites: RefCell<Vec<CitationId>>,
}

impl<'a> Bx<'a> {
    /// The assessment.
    pub(crate) fn a(&self) -> &'a Assessment {
        self.cx.a
    }

    /// A temporary key for a citation id (its place in the intern list, from 1).
    pub(crate) fn cite_key(&self, id: &str) -> u32 {
        let mut cites = self.cites.borrow_mut();
        let pos = match cites.iter().position(|c| c.as_str() == id) {
            Some(p) => p,
            None => {
                cites.push(CitationId::from(id));
                cites.len() - 1
            }
        };
        u32::try_from(pos + 1).unwrap_or(u32::MAX)
    }

    /// A citation inline for these ids (empty ids give nothing).
    pub(crate) fn cite<'b>(&self, ids: impl IntoIterator<Item = &'b CitationId>) -> Option<Inline> {
        let mut keys: Vec<u32> = Vec::new();
        for id in ids {
            let k = self.cite_key(id.as_str());
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        (!keys.is_empty()).then_some(Inline::Cite(keys))
    }

    /// A citation inline for ids given as strings.
    pub(crate) fn cite_strs(&self, ids: &[&str]) -> Option<Inline> {
        let ids: Vec<CitationId> = ids.iter().map(|s| CitationId::from(*s)).collect();
        self.cite(&ids)
    }
}

/// A page: its id, title, kind, and blocks; the fit is the proxy's (see [`fit::fit_for`]) unless
/// the caller sets it.
pub(crate) fn page(
    id: &str,
    title: &str,
    kind: rr_types::binder::PageKind,
    blocks: Vec<Block>,
) -> Page {
    let mut p = Page {
        id: id.to_owned(),
        title: title.to_owned(),
        kind,
        fit: rr_types::binder::Fit::Flow,
        blocks,
    };
    p.fit = fit::fit_for(&p);
    p
}

/// A heading block inside a page.
pub(crate) fn heading(level: u8, text: &str) -> Block {
    Block::Heading(rr_types::binder::Heading {
        level,
        text: text.to_owned(),
    })
}

/// A field row: the household's words as written, or a line to write on.
pub(crate) fn row(label: &str, value: Option<&str>) -> FieldRow {
    FieldRow {
        label: label.to_owned(),
        value: value.map(str::to_owned),
        lines: 1,
    }
}

/// A field row with room for several lines when empty.
pub(crate) fn row_lines(label: &str, value: Option<&str>, lines: u8) -> FieldRow {
    FieldRow {
        label: label.to_owned(),
        value: value.map(str::to_owned),
        lines,
    }
}

/// A contact as written, its parts joined: "name, phone, address"; `None` when all are empty.
pub(crate) fn contact_text(c: Option<&Contact>) -> Option<String> {
    let c = c?;
    let parts: Vec<&str> = [c.name.as_deref(), c.phone.as_deref(), c.address.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// A contact's name and phone as written ("name, phone"), without the address.
pub(crate) fn contact_short(c: Option<&Contact>) -> Option<String> {
    let c = c?;
    let parts: Vec<&str> = [c.name.as_deref(), c.phone.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// Plain text as one inline (never read for Markdown).
pub(crate) fn t(s: impl Into<String>) -> Inline {
    Inline::T(s.into())
}

/// Bold text as one inline.
pub(crate) fn b(s: impl Into<String>) -> Inline {
    Inline::B(s.into())
}

/// A binder being assembled: the tree with temporary citation keys, the citation ids in the
/// order they were interned, and the soft problems ([`problems`]).
#[derive(Debug, Clone)]
pub struct Draft {
    binder: Binder,
    cites: Vec<CitationId>,
    /// Gaps the binder works around (a hazard without its checklist), for tests and the CLI.
    pub problems: Vec<String>,
}

impl Draft {
    /// The citation ids in the order the binder first uses them, in reading order (part by part,
    /// page by page, block by block).
    pub fn cited(&self) -> Vec<CitationId> {
        let mut order: Vec<u32> = Vec::new();
        walk_cites(&self.binder, &mut |keys| {
            for k in keys.iter() {
                if !order.contains(k) {
                    order.push(*k);
                }
            }
        });
        order
            .iter()
            .filter_map(|k| self.cites.get(*k as usize - 1).cloned())
            .collect()
    }

    /// The finished binder: every citation renumbered by first use, and `sources` from
    /// `resolved`, the citations of [`Draft::cited`] in the same order (resolved against the
    /// registry, placeholders included).
    pub fn finish(mut self, resolved: &[Citation]) -> Binder {
        let order = self.cited();
        let number = |key: u32| -> Option<u32> {
            let id = self.cites.get(key as usize - 1)?;
            let at = order.iter().position(|c| c == id)?;
            u32::try_from(at + 1).ok()
        };
        let mut binder = std::mem::replace(&mut self.binder, empty_binder());
        rewrite_cites(&mut binder, &mut |keys| {
            let mut n: Vec<u32> = keys.iter().filter_map(|k| number(*k)).collect();
            n.sort_unstable();
            n.dedup();
            *keys = n;
        });
        binder.sources = resolved
            .iter()
            .take(order.len())
            .enumerate()
            .map(|(i, c)| SourceEntry {
                n: u32::try_from(i + 1).unwrap_or(u32::MAX),
                title: c.title.clone(),
                publisher: c.publisher.clone(),
                year: c.year,
                url: (!c.url.trim().is_empty()).then(|| c.url.clone()),
                expert: c.prior,
            })
            .collect();
        // The Sources page lists them, numbered, after its opening paragraph.
        let list = sources::source_list(&binder.sources);
        if let Some(p) = binder
            .parts
            .iter_mut()
            .flat_map(|part| part.pages.iter_mut())
            .find(|p| p.id == "sources")
        {
            let at = p.blocks.len().min(1);
            p.blocks.insert(at, list);
            p.fit = fit::fit_for(p);
        }
        binder
    }
}

fn empty_binder() -> Binder {
    Binder {
        title: String::new(),
        generated_on: rr_types::PLACEHOLDER_PLANNING_DATE,
        household: String::new(),
        location: String::new(),
        status_line: String::new(),
        review_by: rr_types::PLACEHOLDER_PLANNING_DATE,
        parts: Vec::new(),
        sources: Vec::new(),
        credits: Vec::new(),
    }
}

/// Visits every citation in reading order.
fn walk_cites(b: &Binder, f: &mut impl FnMut(&Vec<u32>)) {
    fn inlines(v: &[Inline], f: &mut impl FnMut(&Vec<u32>)) {
        for i in v {
            if let Inline::Cite(k) = i {
                f(k);
            }
        }
    }
    fn block(bl: &Block, f: &mut impl FnMut(&Vec<u32>)) {
        match bl {
            Block::Para(v) => inlines(v, f),
            Block::Bullets(items) | Block::Numbered(items) => {
                for it in items {
                    inlines(it, f);
                }
            }
            Block::Steps(steps) => {
                for s in steps {
                    inlines(&s.text, f);
                }
            }
            Block::Table(t) => {
                for cell in t.rows.iter().flatten() {
                    inlines(cell, f);
                }
            }
            Block::Callout(c) => {
                for inner in &c.blocks {
                    block(inner, f);
                }
            }
            Block::Decision(d) => {
                for br in &d.branches {
                    inlines(&br.when, f);
                    inlines(&br.then, f);
                }
            }
            Block::Cards(cards) => {
                for line in cards.iter().flat_map(|c| &c.lines) {
                    inlines(line, f);
                }
            }
            Block::Heading(_)
            | Block::Fields(_)
            | Block::MapSlot(_)
            | Block::Log(_)
            | Block::PageBreak(_) => {}
        }
    }
    for p in b.pages() {
        for bl in &p.blocks {
            block(bl, f);
        }
    }
}

/// Rewrites every citation in place.
fn rewrite_cites(b: &mut Binder, f: &mut impl FnMut(&mut Vec<u32>)) {
    fn inlines(v: &mut [Inline], f: &mut impl FnMut(&mut Vec<u32>)) {
        for i in v {
            if let Inline::Cite(k) = i {
                f(k);
            }
        }
    }
    fn block(bl: &mut Block, f: &mut impl FnMut(&mut Vec<u32>)) {
        match bl {
            Block::Para(v) => inlines(v, f),
            Block::Bullets(items) | Block::Numbered(items) => {
                for it in items {
                    inlines(it, f);
                }
            }
            Block::Steps(steps) => {
                for s in steps {
                    inlines(&mut s.text, f);
                }
            }
            Block::Table(t) => {
                for cell in t.rows.iter_mut().flatten() {
                    inlines(cell, f);
                }
            }
            Block::Callout(c) => {
                for inner in &mut c.blocks {
                    block(inner, f);
                }
            }
            Block::Decision(d) => {
                for br in &mut d.branches {
                    inlines(&mut br.when, f);
                    inlines(&mut br.then, f);
                }
            }
            Block::Cards(cards) => {
                for line in cards.iter_mut().flat_map(|c| &mut c.lines) {
                    inlines(line, f);
                }
            }
            Block::Heading(_)
            | Block::Fields(_)
            | Block::MapSlot(_)
            | Block::Log(_)
            | Block::PageBreak(_) => {}
        }
    }
    for part in &mut b.parts {
        for p in &mut part.pages {
            for bl in &mut p.blocks {
                block(bl, f);
            }
        }
    }
}

/// The binder for an assessment, with temporary citation keys: [`Draft::cited`] gives the
/// citation order, [`Draft::finish`] numbers them.
pub fn draft(a: &Assessment, content: &Content, extras: Extras<'_>) -> Draft {
    let cx = Ctx { a, content };
    let selection = select(&cx);
    let people = people::outline(&cx);
    let places = places::distinct(&cx);
    let mut directory: BTreeMap<String, (u8, String)> = BTreeMap::new();
    for (id, tab, title) in FIXED_PAGES {
        directory.insert(id.to_owned(), (tab, title.to_owned()));
    }
    for (id, title) in &people {
        directory.insert(id.clone(), (2, title.clone()));
    }
    for p in &places {
        directory.insert(p.id.clone(), (3, p.title.clone()));
    }
    for s in &selection.pages {
        directory.insert(
            s.checklist.meta.id.clone(),
            (s.checklist.onset.tab(), s.checklist.meta.title.clone()),
        );
    }
    for (id, title) in after::LOGS.iter().map(|(id, title, _)| (*id, *title)) {
        directory.insert(id.to_owned(), (9, title.to_owned()));
    }
    if !a.budget.plan.long_horizon.is_empty() {
        directory.insert(
            "after_months".to_owned(),
            (9, after::MONTHS_TITLE.to_owned()),
        );
    }
    let bx = Bx {
        cx,
        extras,
        directory,
        cites: RefCell::new(Vec::new()),
    };

    let mut pages_by_tab: Vec<Vec<Page>> = vec![
        start::pages(&bx, &selection),
        people::pages(&bx, &people),
        places::pages(&bx, &places),
        pets_docs::pages(&bx),
        have::pages(&bx, &selection),
        checklists::pages(&bx, &selection, rr_content::Onset::Now),
        checklists::pages(&bx, &selection, rr_content::Onset::Coming),
        checklists::pages(&bx, &selection, rr_content::Onset::Ongoing),
        after::pages(&bx),
        vec![sources::page(&bx)],
    ];
    // Tab 7 opens with the forecast list.
    pages_by_tab[6].insert(0, checklists::forecast(&bx));
    let parts: Vec<Part> = PARTS
        .iter()
        .zip(pages_by_tab)
        .map(|(&(tab, id, title, short), mut pages)| {
            if pages.is_empty() {
                pages.push(checklists::empty_tab(&bx, tab));
            }
            Part {
                id: id.to_owned(),
                tab,
                title: title.to_owned(),
                short_title: short.to_owned(),
                pages,
            }
        })
        .collect();

    let generated_on = a.input.planning_date;
    let binder = Binder {
        title: format!("Emergency binder for {}", household_line(&bx.cx)),
        generated_on,
        household: text::household(&a.input),
        location: location_line(&bx.cx),
        status_line: STATUS_LINE.to_owned(),
        review_by: review_by(a),
        parts,
        sources: Vec::new(),
        credits: a.attributions.iter().map(credit).collect(),
    };
    let cites = bx.cites.into_inner();
    Draft {
        binder,
        cites,
        problems: selection.missing.clone(),
    }
}

/// The gaps the binder works around, without building it: every ranked hazard, everyday
/// emergency and opted-in rare family with no checklist block (see [`REQUIRE_ALL_CHECKLISTS`]).
pub fn problems(a: &Assessment, content: &Content) -> Vec<String> {
    select(&Ctx { a, content }).missing
}

/// One year after the plan date: the yearly review (the v2 maintenance calendar's).
pub fn review_by(a: &Assessment) -> rr_types::Date {
    let d = a.input.planning_date;
    d.add_months(12).unwrap_or(d)
}

/// Who the binder is for: the household's names when every person has one ("Dana, Sam, Riley
/// and Grandpa Joe"), otherwise the household in words ("2 adults and 1 child, with 1 dog").
pub(crate) fn household_line(cx: &Ctx<'_>) -> String {
    let names: Vec<String> =
        cx.a.input
            .people
            .iter()
            .filter_map(|p| p.profile.as_ref().and_then(|pr| pr.name.clone()))
            .collect();
    if !names.is_empty() && names.len() == cx.a.input.people.len() {
        text::join_and(&names)
    } else {
        text::household(&cx.a.input)
    }
}

/// "Philadelphia County, Pennsylvania (ZIP code 19147)".
pub(crate) fn location_line(cx: &Ctx<'_>) -> String {
    let zip =
        cx.a.location
            .zip
            .as_deref()
            .map(|z| format!(" (ZIP code {z})"))
            .unwrap_or_default();
    format!("{}{zip}", crate::packet::summary::place(cx))
}

/// A data credit as plain text: the source, its version and access date, then the statement
/// exactly as its terms require, and the address when the statement does not name it. Web
/// addresses are left exactly as they are (never escaped).
pub fn credit(at: &Attribution) -> String {
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

/// How a person is named: their name as written, or "Person 2 (child)".
pub(crate) fn person_title(i: usize, p: &rr_types::Person) -> String {
    match p.profile.as_ref().and_then(|pr| pr.name.as_deref()) {
        Some(n) => n.to_owned(),
        None => format!("Person {} ({})", i + 1, age_word(p.age_band)),
    }
}

/// "adult", "older adult", "child".
pub(crate) fn age_word(b: rr_types::AgeBand) -> &'static str {
    match b {
        rr_types::AgeBand::Infant => "baby",
        rr_types::AgeBand::Toddler => "toddler",
        rr_types::AgeBand::Child => "child",
        rr_types::AgeBand::Teen => "teenager",
        rr_types::AgeBand::Adult => "adult",
        rr_types::AgeBand::Senior => "older adult",
    }
}

/// The household's family plan, or an empty one.
pub(crate) fn family<'a>(cx: &Ctx<'a>) -> std::borrow::Cow<'a, rr_types::FamilyPlan> {
    match cx.a.input.family_plan.as_ref() {
        Some(p) => std::borrow::Cow::Borrowed(p),
        None => std::borrow::Cow::Owned(rr_types::FamilyPlan::default()),
    }
}
