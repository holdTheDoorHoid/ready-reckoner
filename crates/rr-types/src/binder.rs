//! The binder (DESIGN-DELTA-v3 §4.1): the during-event document `assess` returns in place of the
//! v2 packet, as a tree the renderers walk. rr-plan renders it to Markdown (the CLI, the goldens)
//! and the web app to HTML and PDF.
//!
//! A [`Binder`] holds up to ten [`Part`]s, one per tab of a printed binder, each a list of
//! [`Page`]s made of [`Block`]s; running text is a list of [`Inline`]s. In JSON a block and an
//! inline are objects with exactly one key, the variant's snake_case name:
//!
//! ```json
//! {"heading": {"level": 2, "text": "Do first"}}
//! {"para": [{"t": "Leave by the nearest safe way. "}, {"cite": [3]}]}
//! {"steps": [{"text": [{"b": "Get out."}], "memory": true}]}
//! {"fields": [{"label": "Meeting place near home", "lines": 1}]}
//! {"page_break": true}
//! {"blank": 24}
//! ```
//!
//! [`Binder::check`] lists the structural problems every renderer relies on being absent
//! (duplicate page ids, a link or branch to no page, a citation number outside the source list,
//! tabs out of order, an empty part); rr-plan, the CLI and the tests share it.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Date;

/// Tabs are numbered from 1 to this (DESIGN-DELTA-v3 §4.2: ten parts, each a tab of the binder).
pub const MAX_TABS: u8 = 10;

/// Longest [`Part::short_title`], in characters: it is printed on a tab divider.
pub const SHORT_TITLE_MAX: usize = 14;

/// The whole binder: a cover's worth of facts, the parts in tab order, and the numbered sources
/// its citations point to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binder {
    /// The binder's title, for the cover and the PDF's metadata.
    pub title: String,
    /// The day it was made: the plan's planning date (the engine never reads the clock).
    pub generated_on: Date,
    /// Who it is for, in words ("2 adults, 1 older adult and 1 child, with 1 dog").
    pub household: String,
    /// Where, in words ("Philadelphia County, Pennsylvania (ZIP code 19147)").
    pub location: String,
    /// The line on the cover under the facts: what the binder is and is not.
    pub status_line: String,
    /// When to review it and print it again.
    pub review_by: Date,
    /// The parts, in tab order.
    pub parts: Vec<Part>,
    /// The sources, numbered as cited: `sources[i].n == i + 1`, and an [`Inline::Cite`] number
    /// `n` points to `sources[n - 1]`.
    pub sources: Vec<SourceEntry>,
    /// The data credits and disclaimers the data sets' terms require, each shown exactly.
    pub credits: Vec<String>,
}

/// One part of the binder: the pages behind one tab divider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// Stable id (`start`, `people`, `home_places`, …; DESIGN-DELTA-v3 §4.2).
    pub id: String,
    /// The tab number, 1 to [`MAX_TABS`], rising from part to part.
    pub tab: u8,
    /// The part's title.
    pub title: String,
    /// The tab label, at most [`SHORT_TITLE_MAX`] characters.
    pub short_title: String,
    /// The pages, in order. A part always has at least one.
    pub pages: Vec<Page>,
}

/// One page (or a run of pages that belong together).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    /// Stable id, unique in the binder; links and branches point to it (`home`,
    /// `getting_out`, `check_house_fire`).
    pub id: String,
    /// The page's title.
    pub title: String,
    /// What the page is.
    pub kind: PageKind,
    /// How much paper it promises to take.
    pub fit: Fit,
    /// The content, in order.
    pub blocks: Vec<Block>,
}

string_enum! {
    /// What a page is (DESIGN-DELTA-v3 §4.2), which tells a renderer how to lay it out.
    pub enum PageKind: "page kind" {
        /// The cover: household, place, date, status line, review date.
        Cover = "cover",
        /// How to use this binder.
        HowToUse = "how_to_use",
        /// The first five minutes of any emergency.
        QuickStart = "quick_start",
        /// Which checklist: every hazard and everyday emergency, with its tab and page.
        Index = "index",
        /// Every number in the binder on one page.
        Contacts = "contacts",
        /// One person.
        Person = "person",
        /// The wallet cards to cut out.
        WalletCards = "wallet_cards",
        /// The home.
        Home = "home",
        /// A place in the household's life (work, school, child care).
        Place = "place",
        /// The neighbourhood: meeting places, hospital, pharmacy, shelter, alerts.
        Neighbourhood = "neighbourhood",
        /// Getting out: where to go and how.
        GettingOut = "getting_out",
        /// Pets and animals.
        Pets = "pets",
        /// Vehicles.
        Vehicles = "vehicles",
        /// Documents and money.
        Documents = "documents",
        /// What the household has and where it is.
        Inventory = "inventory",
        /// The risks at a glance.
        RisksGlance = "risks_glance",
        /// An incident checklist.
        Checklist = "checklist",
        /// After the event.
        After = "after",
        /// A blank log to fill in by hand.
        Log = "log",
        /// The numbered sources.
        Sources = "sources",
    }
}

string_enum! {
    /// How much paper a page promises to take (DESIGN-DELTA-v3 §4.1, §5.6).
    pub enum Fit: "page fit" {
        /// Fits one printed page.
        One = "one",
        /// Fits two printed pages.
        Two = "two",
        /// Runs as long as it needs.
        Flow = "flow",
    }
}

/// A block of a page. In JSON, an object with one key, the variant's snake_case name:
/// `{"heading": {...}}`, `{"para": [...]}`, `{"page_break": true}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Block {
    /// A heading inside the page (the page's own title is [`Page::title`]).
    Heading(Heading),
    /// A paragraph.
    Para(Vec<Inline>),
    /// A bulleted list, one entry per item.
    Bullets(Vec<Vec<Inline>>),
    /// A numbered list, one entry per item.
    Numbered(Vec<Vec<Inline>>),
    /// Airline-checklist steps.
    Steps(Vec<Step>),
    /// Labelled answers, or ruled lines where there is no answer.
    Fields(Vec<FieldRow>),
    /// A table.
    Table(Table),
    /// A boxed note.
    Callout(Callout),
    /// A leave-or-stay decision.
    Decision(Decision),
    /// A place for a map the web app fills (DESIGN-DELTA-v3 §9); the CLI prints a boxed
    /// placeholder.
    MapSlot(MapSlot),
    /// Wallet cards to cut out.
    Cards(Vec<Card>),
    /// A blank log to fill in by hand.
    Log(Log),
    /// Start a new printed page here. Its JSON value is always `true`.
    PageBreak(True),
}

impl Block {
    /// `{"page_break": true}`.
    pub const fn page_break() -> Block {
        Block::PageBreak(True)
    }
}

/// The payload of [`Block::PageBreak`]: the JSON value `true`, and nothing else.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct True;

impl Serialize for True {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for True {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if bool::deserialize(deserializer)? {
            Ok(True)
        } else {
            Err(serde::de::Error::custom("`page_break` is always `true`"))
        }
    }
}

/// A heading inside a page ([`Block::Heading`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Heading {
    /// 1, 2 or 3: 1 is the largest heading inside a page.
    pub level: u8,
    /// The heading's words.
    pub text: String,
}

/// One airline-checklist step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// The step, usually opening with bold lead words.
    pub text: Vec<Inline>,
    /// A "do first, from memory" item: rendered bold, known without reading.
    pub memory: bool,
}

/// One labelled answer ([`Block::Fields`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRow {
    /// What the answer is ("Meeting place near home").
    pub label: String,
    /// The household's own words, printed as written; absent when not answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Ruled lines to write on when there is no answer.
    pub lines: u8,
}

/// A table ([`Block::Table`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    /// Column headings.
    pub header: Vec<String>,
    /// Rows of cells; each cell is running text.
    pub rows: Vec<Vec<Vec<Inline>>>,
}

string_enum! {
    /// How a boxed note reads ([`Callout::kind`]). Never carried by colour alone.
    pub enum CalloutKind: "callout kind" {
        /// Stop: do not do this.
        Stop = "stop",
        /// A warning.
        Warning = "warning",
        /// A note.
        Note = "note",
        /// A decision to make.
        Decision = "decision",
    }
}

/// A boxed note ([`Block::Callout`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Callout {
    /// What kind of note.
    pub kind: CalloutKind,
    /// Its title, if it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What it says.
    pub blocks: Vec<Block>,
}

/// A leave-or-stay decision ([`Block::Decision`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// The question ("Leave or stay?").
    pub question: String,
    /// The answers, each a condition and what to do.
    pub branches: Vec<Branch>,
}

/// One answer of a [`Decision`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    /// The condition ("Leave if you are told to").
    pub when: Vec<Inline>,
    /// What to do then.
    pub then: Vec<Inline>,
    /// The page to turn to (a [`Page::id`]), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub go_to: Option<String>,
}

string_enum! {
    /// Which of the three maps goes in a [`MapSlot`] (DESIGN-DELTA-v3 §9.2).
    pub enum MapSlotKind: "map slot kind" {
        /// The home and "where we would go", with the ways out.
        Region = "region",
        /// The city or county around the home.
        Area = "area",
        /// About 1.5 km around the home.
        Neighbourhood = "neighbourhood",
    }
}

/// A place for a map ([`Block::MapSlot`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSlot {
    /// Stable id of the slot.
    pub id: String,
    /// Which map.
    pub kind: MapSlotKind,
    /// The caption under the map, or in the placeholder box.
    pub caption: String,
}

/// One wallet card ([`Block::Cards`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    /// Whose card, or what it is for.
    pub title: String,
    /// Its lines.
    pub lines: Vec<Vec<Inline>>,
}

/// A blank log to fill in by hand ([`Block::Log`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Log {
    /// Column headings.
    pub columns: Vec<String>,
    /// Blank rows to print.
    pub rows: u8,
}

/// Running text. In JSON, an object with one key: `{"t": "text"}`, `{"b": "bold text"}`,
/// `{"cite": [3, 7]}`, `{"link": {"to": "home", "text": "Tab 3, Home"}}`, `{"blank": 24}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Inline {
    /// Plain text.
    T(String),
    /// Bold text.
    B(String),
    /// Citation numbers, each pointing into [`Binder::sources`] (1 is the first).
    Cite(Vec<u32>),
    /// A cross-reference to another page.
    Link(Link),
    /// A ruled blank of about this many characters, to write on.
    Blank(u8),
}

/// A cross-reference ([`Inline::Link`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    /// The page it points to (a [`Page::id`]).
    pub to: String,
    /// The words to show ("Tab 3, Home").
    pub text: String,
}

/// One numbered source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEntry {
    /// Its number, from 1, in citation order.
    pub n: u32,
    /// The work's title.
    pub title: String,
    /// Who published it.
    pub publisher: String,
    /// The year, where known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    /// Where to find it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// An expert estimate rather than measured data.
    pub expert: bool,
}

impl Binder {
    /// Every page, part by part, in order.
    pub fn pages(&self) -> impl Iterator<Item = &Page> {
        self.parts.iter().flat_map(|p| p.pages.iter())
    }

    /// The page with this id.
    pub fn page(&self, id: &str) -> Option<&Page> {
        self.pages().find(|p| p.id == id)
    }

    /// The structural problems of the binder, in a fixed order; empty when there are none. Every
    /// renderer relies on these being absent:
    ///
    /// - a part whose tab is outside 1 to [`MAX_TABS`], or not above the tab before it;
    /// - a part id used twice, a part with no pages, a tab label longer than
    ///   [`SHORT_TITLE_MAX`] characters;
    /// - a page id used twice anywhere in the binder;
    /// - a link ([`Inline::Link`]) or a decision branch ([`Branch::go_to`]) to no page;
    /// - a citation number outside 1 to `sources.len()`, or a source whose `n` is not its place
    ///   in the list;
    /// - a heading level outside 1 to 3.
    pub fn check(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut last_tab: Option<u8> = None;
        let mut part_ids: BTreeSet<&str> = BTreeSet::new();
        for part in &self.parts {
            if !(1..=MAX_TABS).contains(&part.tab) {
                problems.push(format!(
                    "part `{}`: tab {} is outside 1 to {MAX_TABS}",
                    part.id, part.tab
                ));
            }
            if let Some(prev) = last_tab
                && part.tab <= prev
            {
                problems.push(format!(
                    "part `{}`: tab {} comes after tab {prev}; tabs must rise from part to part",
                    part.id, part.tab
                ));
            }
            last_tab = Some(part.tab);
            if !part_ids.insert(part.id.as_str()) {
                problems.push(format!("part id `{}` is used twice", part.id));
            }
            if part.pages.is_empty() {
                problems.push(format!("part `{}` has no pages", part.id));
            }
            let label = part.short_title.chars().count();
            if label > SHORT_TITLE_MAX {
                problems.push(format!(
                    "part `{}`: the tab label `{}` has {label} characters; the most is \
                     {SHORT_TITLE_MAX}",
                    part.id, part.short_title
                ));
            }
        }

        let mut page_ids: BTreeSet<&str> = BTreeSet::new();
        for page in self.pages() {
            if !page_ids.insert(page.id.as_str()) {
                problems.push(format!("page id `{}` is used twice", page.id));
            }
        }

        let sources = self.sources.len();
        for (i, s) in self.sources.iter().enumerate() {
            if usize::try_from(s.n).ok() != Some(i + 1) {
                problems.push(format!("source {} of {sources} is numbered {}", i + 1, s.n));
            }
        }

        for page in self.pages() {
            let mut walk = Walk {
                page: &page.id,
                page_ids: &page_ids,
                sources,
                problems: &mut problems,
            };
            for block in &page.blocks {
                walk.block(block);
            }
        }
        problems
    }
}

/// The walk behind [`Binder::check`]: the references, citations and headings of one page.
struct Walk<'a> {
    page: &'a str,
    page_ids: &'a BTreeSet<&'a str>,
    sources: usize,
    problems: &'a mut Vec<String>,
}

impl Walk<'_> {
    fn block(&mut self, block: &Block) {
        match block {
            Block::Heading(h) => {
                if !(1..=3).contains(&h.level) {
                    self.problems.push(format!(
                        "page `{}`: the heading `{}` has level {}; levels are 1 to 3",
                        self.page, h.text, h.level
                    ));
                }
            }
            Block::Para(text) => self.inlines(text),
            Block::Bullets(items) | Block::Numbered(items) => {
                for item in items {
                    self.inlines(item);
                }
            }
            Block::Steps(steps) => {
                for step in steps {
                    self.inlines(&step.text);
                }
            }
            Block::Fields(_) | Block::MapSlot(_) | Block::Log(_) | Block::PageBreak(_) => {}
            Block::Table(table) => {
                for cell in table.rows.iter().flatten() {
                    self.inlines(cell);
                }
            }
            Block::Callout(callout) => {
                for inner in &callout.blocks {
                    self.block(inner);
                }
            }
            Block::Decision(decision) => {
                for branch in &decision.branches {
                    self.inlines(&branch.when);
                    self.inlines(&branch.then);
                    if let Some(to) = &branch.go_to {
                        self.target(to, "a decision branch");
                    }
                }
            }
            Block::Cards(cards) => {
                for line in cards.iter().flat_map(|c| &c.lines) {
                    self.inlines(line);
                }
            }
        }
    }

    fn inlines(&mut self, text: &[Inline]) {
        for inline in text {
            match inline {
                Inline::Cite(numbers) => {
                    for &n in numbers {
                        let ok = n >= 1 && usize::try_from(n).is_ok_and(|n| n <= self.sources);
                        if !ok {
                            self.problems.push(format!(
                                "page `{}`: citation {n} is outside 1 to {} (the binder has {} \
                                 sources)",
                                self.page, self.sources, self.sources
                            ));
                        }
                    }
                }
                Inline::Link(link) => self.target(&link.to, "a link"),
                Inline::T(_) | Inline::B(_) | Inline::Blank(_) => {}
            }
        }
    }

    fn target(&mut self, to: &str, what: &str) {
        if !self.page_ids.contains(to) {
            self.problems.push(format!(
                "page `{}`: {what} points to `{to}`, which is not a page in the binder",
                self.page
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: u16, m: u8, d: u8) -> Date {
        Date::from_ymd(y, m, d).unwrap()
    }

    fn t(s: &str) -> Inline {
        Inline::T(s.to_owned())
    }

    fn page(id: &str, blocks: Vec<Block>) -> Page {
        Page {
            id: id.into(),
            title: id.into(),
            kind: PageKind::Home,
            fit: Fit::One,
            blocks,
        }
    }

    fn part(id: &str, tab: u8, pages: Vec<Page>) -> Part {
        Part {
            id: id.into(),
            tab,
            title: id.into(),
            short_title: id.into(),
            pages,
        }
    }

    fn source(n: u32) -> SourceEntry {
        SourceEntry {
            n,
            title: "Home Fires".into(),
            publisher: "FEMA / Ready.gov".into(),
            year: None,
            url: None,
            expert: false,
        }
    }

    fn binder(parts: Vec<Part>, sources: Vec<SourceEntry>) -> Binder {
        Binder {
            title: "Your binder".into(),
            generated_on: date(2026, 10, 1),
            household: "1 adult".into(),
            location: "Philadelphia County, Pennsylvania".into(),
            status_line: "Not official guidance.".into(),
            review_by: date(2027, 10, 1),
            parts,
            sources,
            credits: vec![],
        }
    }

    fn one(b: &Binder, needle: &str) {
        let problems = b.check();
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains(needle), "{problems:#?}");
    }

    #[test]
    fn a_sound_binder_has_no_problems() {
        let b = binder(
            vec![
                part(
                    "start",
                    1,
                    vec![page(
                        "cover",
                        vec![Block::Para(vec![
                            t("See "),
                            Inline::Link(Link {
                                to: "home".into(),
                                text: "Tab 3, Home".into(),
                            }),
                            Inline::Cite(vec![1, 2]),
                        ])],
                    )],
                ),
                part(
                    "home_places",
                    3,
                    vec![page(
                        "home",
                        vec![Block::Decision(Decision {
                            question: "Leave or stay?".into(),
                            branches: vec![Branch {
                                when: vec![t("Leave if told to.")],
                                then: vec![t("Go.")],
                                go_to: Some("cover".into()),
                            }],
                        })],
                    )],
                ),
            ],
            vec![source(1), source(2)],
        );
        assert_eq!(b.check(), Vec::<String>::new());
        assert_eq!(b.pages().count(), 2);
        assert_eq!(b.page("home").map(|p| p.kind), Some(PageKind::Home));
        assert!(b.page("nowhere").is_none());
    }

    #[test]
    fn page_break_is_true_and_only_true() {
        let json = serde_json::to_string(&Block::page_break()).unwrap();
        assert_eq!(json, r#"{"page_break":true}"#);
        assert_eq!(
            serde_json::from_str::<Block>(&json).unwrap(),
            Block::page_break()
        );
        assert!(serde_json::from_str::<Block>(r#"{"page_break":false}"#).is_err());
        assert!(serde_json::from_str::<Block>(r#""page_break""#).is_err());
    }

    #[test]
    fn check_names_each_problem() {
        let para = |inlines: Vec<Inline>| vec![Block::Para(inlines)];
        // Tab outside 1..=10, and tabs out of order.
        one(
            &binder(vec![part("a", 11, vec![page("p", vec![])])], vec![]),
            "outside 1 to 10",
        );
        one(
            &binder(vec![part("a", 0, vec![page("p", vec![])])], vec![]),
            "tab 0 is outside",
        );
        one(
            &binder(
                vec![
                    part("a", 3, vec![page("p", vec![])]),
                    part("b", 2, vec![page("q", vec![])]),
                ],
                vec![],
            ),
            "tab 2 comes after tab 3",
        );
        one(
            &binder(
                vec![
                    part("a", 3, vec![page("p", vec![])]),
                    part("b", 3, vec![page("q", vec![])]),
                ],
                vec![],
            ),
            "tab 3 comes after tab 3",
        );
        // An empty part; a part id twice; a long tab label.
        one(&binder(vec![part("a", 1, vec![])], vec![]), "has no pages");
        one(
            &binder(
                vec![
                    part("a", 1, vec![page("p", vec![])]),
                    part("a", 2, vec![page("q", vec![])]),
                ],
                vec![],
            ),
            "part id `a` is used twice",
        );
        let mut long = part("a", 1, vec![page("p", vec![])]);
        long.short_title = "Pets, vehicles, documents".into();
        one(&binder(vec![long], vec![]), "has 25 characters");
        // A page id twice, even across parts.
        one(
            &binder(
                vec![
                    part("a", 1, vec![page("p", vec![])]),
                    part("b", 2, vec![page("p", vec![])]),
                ],
                vec![],
            ),
            "page id `p` is used twice",
        );
        // A link and a branch to nowhere.
        let link = para(vec![Inline::Link(Link {
            to: "nowhere".into(),
            text: "Tab 9".into(),
        })]);
        one(
            &binder(vec![part("a", 1, vec![page("p", link)])], vec![]),
            "a link points to `nowhere`",
        );
        let branch = vec![Block::Decision(Decision {
            question: "Leave?".into(),
            branches: vec![Branch {
                when: vec![],
                then: vec![],
                go_to: Some("gone".into()),
            }],
        })];
        one(
            &binder(vec![part("a", 1, vec![page("p", branch)])], vec![]),
            "a decision branch points to `gone`",
        );
        // Citations outside the source list, anywhere inline text can be.
        let cites = vec![
            Block::Para(vec![Inline::Cite(vec![0])]),
            Block::Bullets(vec![vec![Inline::Cite(vec![3])]]),
            Block::Steps(vec![Step {
                text: vec![Inline::Cite(vec![1, 9])],
                memory: true,
            }]),
            Block::Callout(Callout {
                kind: CalloutKind::Stop,
                title: None,
                blocks: vec![Block::Numbered(vec![vec![Inline::Cite(vec![4])]])],
            }),
            Block::Table(Table {
                header: vec!["A".into()],
                rows: vec![vec![vec![Inline::Cite(vec![5])]]],
            }),
            Block::Cards(vec![Card {
                title: "Card".into(),
                lines: vec![vec![Inline::Cite(vec![6])]],
            }]),
        ];
        let b = binder(
            vec![part("a", 1, vec![page("p", cites)])],
            vec![source(1), source(2)],
        );
        let problems = b.check();
        let outside: Vec<&String> = problems
            .iter()
            .filter(|p| p.contains("is outside 1 to 2"))
            .collect();
        assert_eq!(outside.len(), 6, "{problems:#?}");
        for n in ["citation 0 ", "citation 3 ", "citation 9 ", "citation 6 "] {
            assert!(problems.iter().any(|p| p.contains(n)), "{n}: {problems:#?}");
        }
        // Sources numbered out of place, and a heading level out of range.
        one(
            &binder(
                vec![part("a", 1, vec![page("p", vec![])])],
                vec![source(1), source(3)],
            ),
            "source 2 of 2 is numbered 3",
        );
        let heading = vec![Block::Heading(Heading {
            level: 4,
            text: "Deep".into(),
        })];
        one(
            &binder(vec![part("a", 1, vec![page("p", heading)])], vec![]),
            "has level 4",
        );
    }
}
