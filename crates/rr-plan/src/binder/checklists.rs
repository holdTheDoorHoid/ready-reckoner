//! Tabs 6 to 8: the incident checklists (DESIGN-DELTA-v3 §5). Which pages a household gets
//! ([`select`], §5.1), in which tab and order (§5.2), and each page in airline-checklist form
//! (§5.3), built from `rr_content::Checklist::render_for` with the household's own answers in
//! its placeholders; and tab 7's first page, the 48-hour list before a forecast storm, freeze or
//! heat wave, built from `plan_forecast_48h`.

use std::cell::RefCell;

use rr_content::checklist::braces;
use rr_content::{Checklist, Onset};
use rr_types::binder::{Block, Branch, Decision, FieldRow, Fit, Inline, Page, PageKind, Step};
use rr_types::{HazardDisplay, HazardProfile};

use super::inline::{USER_CLOSE, USER_OPEN, user};
use super::{Bx, b, heading, page, row, t};
use crate::packet::{Ctx, text};

/// The everyday emergencies' names, as the index lists them (`rr_content::ids::EVENTS`).
pub const EVENT_NAMES: [(&str, &str); 7] = [
    ("gas_leak_or_co", "Gas leak or carbon monoxide alarm"),
    ("missing_person", "Missing person"),
    ("evacuation_order", "Evacuation order"),
    ("shelter_in_place", "Shelter-in-place order"),
    ("boil_water_notice", "Boil-water notice"),
    ("power_outage", "Power outage at home"),
    ("something_else", "Something else"),
];

/// The generic page for anything the other checklists do not cover (§5.1, rule 4).
pub const SOMETHING_ELSE: &str = "something_else";

/// The name of an everyday emergency.
pub fn event_name(id: &str) -> &str {
    EVENT_NAMES
        .iter()
        .find(|(e, _)| *e == id)
        .map_or(id, |(_, n)| *n)
}

/// One checklist page a household gets, and what points to it.
#[derive(Debug, Clone)]
pub struct Selected<'a> {
    /// The block.
    pub checklist: &'a Checklist,
    /// The ranked hazards and opted-in rare families that point to it, most likely first.
    pub hazards: Vec<&'a HazardProfile>,
    /// The everyday emergencies that point to it.
    pub events: Vec<&'static str>,
}

impl Selected<'_> {
    /// The ten-year chance of its most likely hazard (0 for an everyday emergency's page).
    fn chance(&self) -> f64 {
        self.hazards
            .iter()
            .map(|p| crate::packet::risks::chance(p.rate_per_year, 10))
            .fold(0.0, f64::max)
    }

    /// Where it sorts in its tab: everyday emergencies first, "Something else" last.
    fn group(&self) -> u8 {
        if self.events.contains(&SOMETHING_ELSE) {
            2
        } else if self.events.is_empty() {
            1
        } else {
            0
        }
    }
}

/// The checklist pages of a household, in tab and page order, and what has no page.
#[derive(Debug, Clone, Default)]
pub struct Selection<'a> {
    /// The pages, tab by tab, each tab in its order (§5.1, §5.2).
    pub pages: Vec<Selected<'a>>,
    /// The ranked hazards and opted-in rare families with no checklist block yet.
    pub uncovered: Vec<&'a HazardProfile>,
    /// The everyday emergencies with no checklist block yet.
    pub uncovered_events: Vec<&'static str>,
    /// Each gap in words (see `super::problems`).
    pub missing: Vec<String>,
}

impl<'a> Selection<'a> {
    /// The page a hazard's index row points to.
    pub fn page_for_hazard(&self, id: rr_types::HazardId) -> Option<&Selected<'a>> {
        self.pages
            .iter()
            .find(|s| s.hazards.iter().any(|h| h.id == id))
    }

    /// The page an everyday emergency's index row points to.
    pub fn page_for_event(&self, id: &str) -> Option<&Selected<'a>> {
        self.pages.iter().find(|s| s.events.contains(&id))
    }

    /// The page with this checklist id.
    pub fn page(&self, id: &str) -> Option<&Selected<'a>> {
        self.pages.iter().find(|s| s.checklist.meta.id == id)
    }
}

/// The ranked hazards, most likely first (equal rates keep the register's order).
pub(crate) fn ranked<'a>(cx: &Ctx<'a>) -> Vec<&'a HazardProfile> {
    let mut v: Vec<&HazardProfile> =
        cx.a.hazards
            .profiles
            .iter()
            .filter(|p| p.display == HazardDisplay::Ranked)
            .collect();
    v.sort_by(|x, y| y.rate_per_year.total_cmp(&x.rate_per_year));
    v
}

/// The rare families the household opted into, by their lead hazard's register row.
pub(crate) fn opted_rare<'a>(cx: &Ctx<'a>) -> Vec<&'a HazardProfile> {
    let families = cx.a.input.dials.rare_families();
    cx.a.hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        .filter(|p| families.contains(&p.id.as_str()))
        .collect()
}

/// Which checklist pages the household gets (DESIGN-DELTA-v3 §5.1): one for every hazard in its
/// ranked matrix, every everyday emergency (`rr_content::ids::EVENTS`, "Something else"
/// included) and every rare family it opted into, by the family's lead hazard. A block that
/// applies to several prints once. Within a tab (the block's `onset`), everyday emergencies
/// come first and "Something else" last; the rest by the ten-year chance of their most likely
/// hazard, highest first; ties keep the content's order.
pub(crate) fn select<'a>(cx: &Ctx<'a>) -> Selection<'a> {
    let content = cx.content;
    let mut sel = Selection::default();
    let add_hazard = |sel: &mut Selection<'a>, p: &'a HazardProfile| match content
        .checklist_for(&format!("hazard:{}", p.id))
    {
        Some(c) => match sel
            .pages
            .iter_mut()
            .find(|s| s.checklist.meta.id == c.meta.id)
        {
            Some(s) => {
                if !s.hazards.iter().any(|h| h.id == p.id) {
                    s.hazards.push(p);
                }
            }
            None => sel.pages.push(Selected {
                checklist: c,
                hazards: vec![p],
                events: Vec::new(),
            }),
        },
        None => {
            sel.missing.push(format!(
                "hazard `{}` ({}) has no checklist block yet",
                p.id, p.name
            ));
            sel.uncovered.push(p);
        }
    };
    for p in ranked(cx) {
        add_hazard(&mut sel, p);
    }
    for p in opted_rare(cx) {
        add_hazard(&mut sel, p);
    }
    for &event in rr_content::ids::EVENTS {
        match content.checklist_for(&format!("event:{event}")) {
            Some(c) => match sel
                .pages
                .iter_mut()
                .find(|s| s.checklist.meta.id == c.meta.id)
            {
                Some(s) => s.events.push(event),
                None => sel.pages.push(Selected {
                    checklist: c,
                    hazards: Vec::new(),
                    events: vec![event],
                }),
            },
            None => {
                sel.missing.push(format!(
                    "everyday emergency `{event}` ({}) has no checklist block yet",
                    event_name(event)
                ));
                sel.uncovered_events.push(event);
            }
        }
    }
    // Content order breaks ties, so the order never depends on the order of the rules above.
    let order = |c: &Checklist| {
        content
            .checklists()
            .iter()
            .position(|x| x.meta.id == c.meta.id)
            .unwrap_or(usize::MAX)
    };
    sel.pages.sort_by(|x, y| {
        x.checklist
            .onset
            .cmp(&y.checklist.onset)
            .then(x.group().cmp(&y.group()))
            .then(y.chance().total_cmp(&x.chance()))
            .then(order(x.checklist).cmp(&order(y.checklist)))
    });
    sel
}

/// The pages of one tab, in order.
pub(super) fn pages(bx: &Bx<'_>, sel: &Selection<'_>, onset: Onset) -> Vec<Page> {
    sel.pages
        .iter()
        .filter(|s| s.checklist.onset == onset)
        .map(|s| checklist_page(bx, s))
        .collect()
}

/// A checklist tab with no page yet (before the content groups' blocks are in): one page that
/// says so and points to Quick start and the index.
pub(super) fn empty_tab(bx: &Bx<'_>, tab: u8) -> Page {
    let id = super::PARTS
        .iter()
        .find(|p| p.0 == tab)
        .map_or("tab", |p| p.1);
    let mut para = vec![t(
        "No checklist in this binder goes behind this tab yet. Until one does, use ",
    )];
    para.extend(bx.link_inline("quick_start"));
    para.push(t(" and "));
    para.extend(bx.link_inline("index"));
    para.push(t("."));
    page(
        &format!("{id}_pending"),
        "No checklist here yet",
        PageKind::Checklist,
        vec![Block::Para(para)],
    )
}

/// "about 40 in 100", "fewer than 1 in 100", "almost all": a chance as households in 100.
pub(crate) fn in_100(p: f64) -> String {
    match text::per_100(p).as_str() {
        "almost all" => "almost all".to_owned(),
        "fewer than 1" => "fewer than 1 in 100".to_owned(),
        n => format!("about {n} in 100"),
    }
}

/// How likely a hazard is here over the dial's horizon, in words: a ranked hazard as households
/// in 100, a rare family as a range only ("between 1 in 3,300 and 1 in 28").
pub(crate) fn likely_words(cx: &Ctx<'_>, p: &HazardProfile) -> String {
    let years = cx.a.input.dials.horizon_years.max(1);
    match p.display {
        HazardDisplay::Ranked => in_100(crate::packet::risks::chance(p.rate_per_year, years)),
        HazardDisplay::RareCatastrophic => {
            crate::packet::risks::range_words(p.rate_range[0], p.rate_range[1], years)
        }
    }
}

/// The household's line on a checklist page (§5.3): "Here: about 40 in 100 households like yours
/// in the next 10 years · How bad: Severe", cited to the hazard's sources; with the hazard's
/// name first when the page covers several.
fn hazard_line(bx: &Bx<'_>, p: &HazardProfile, several: bool) -> Vec<Inline> {
    let horizon = rr_consequence::words::horizon_phrase(bx.a().input.dials.horizon_years.max(1));
    let lead = if several {
        format!("{} here: ", p.name)
    } else {
        "Here: ".to_owned()
    };
    let mut v = vec![
        b(lead),
        t(format!(
            "{} households like yours in {horizon} · How bad: {}",
            likely_words(&bx.cx, p),
            text::severity(p.severity)
        )),
    ];
    v.extend(bx.cite(&p.sources));
    v
}

/// The household's answer for a checklist placeholder (`rr_content::checklist::PLACEHOLDERS`),
/// as written; `None` when it has not given one.
pub(crate) fn answer(bx: &Bx<'_>, name: &str) -> Option<String> {
    let cx = &bx.cx;
    let fp = super::family(cx);
    let home = fp.home.clone().unwrap_or_default();
    let hood = fp.neighbourhood.clone().unwrap_or_default();
    match name {
        "meeting_near" => fp.meeting_place_near.clone(),
        "meeting_far" => fp.meeting_place_far.clone(),
        "shelter_home" => fp.shelter_spot_home.clone(),
        "shelter_work" => fp.shelter_spot_work.clone(),
        "where_go" => fp.where_we_would_go.clone(),
        "out_of_area_contact" => super::contact_short(fp.out_of_area_contact.as_ref()),
        "gas_shutoff" => fp.shutoff_gas.clone(),
        "water_shutoff" => fp.shutoff_water.clone(),
        "electric_panel" => fp.shutoff_electric.clone(),
        "electric_utility" => super::contact_short(home.electric_utility.as_ref()),
        "gas_utility" => super::contact_short(home.gas_utility.as_ref()),
        "water_utility" => super::contact_short(home.water_utility.as_ref()),
        "hospital" => super::contact_short(hood.hospital.as_ref()),
        "pharmacy" => super::contact_short(hood.pharmacy.as_ref()).or_else(|| {
            cx.a.input
                .people
                .iter()
                .filter_map(|p| p.profile.as_ref()?.pharmacy.as_ref())
                .find_map(|c| super::contact_short(Some(c)))
        }),
        "alerts" => hood.alerts.clone(),
        "county" => Some(crate::packet::summary::place(cx)),
        "shelter" => super::contact_short(hood.shelter.as_ref()),
        // awaiting: rr-content (the planner adds `county_office` to `PLACEHOLDERS`, width 28)
        "county_office" => super::contact_short(hood.county_emergency_office.as_ref()),
        _ => None,
    }
}

/// The label of a placeholder's answer in "Where and who".
pub(crate) fn answer_label(name: &str) -> &'static str {
    match name {
        "meeting_near" => "Meeting place near home",
        "meeting_far" => "Meeting place outside the neighborhood",
        "shelter_home" => "Shelter spot at home",
        "shelter_work" => "Shelter spot at work or school",
        "where_go" => "Where we would go",
        "out_of_area_contact" => "Out-of-area contact",
        "gas_shutoff" => "Gas shut-off",
        "water_shutoff" => "Water shut-off",
        "electric_panel" => "Electrical panel",
        "electric_utility" => "Electric company",
        "gas_utility" => "Gas company",
        "water_utility" => "Water company",
        "hospital" => "Hospital",
        "pharmacy" => "Pharmacy",
        "alerts" => "How we get alerts",
        "shelter" => "Community shelter",
        "county_office" => "County emergency office",
        _ => "",
    }
}

/// The most "Where and who" rows the engine adds to a checklist's own lines (the household's
/// answers the page's steps use), so the page keeps its one-page promise.
pub const MAX_ADDED_ROWS: usize = 4;

/// One checklist page (§5.3).
fn checklist_page(bx: &Bx<'_>, s: &Selected<'_>) -> Page {
    let c = s.checklist;
    let used: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let fill = |name: &str| {
        let mut u = used.borrow_mut();
        if !u.iter().any(|x| x == name) {
            u.push(name.to_owned());
        }
        answer(bx, name).map(|v| user(&v))
    };
    let r = c.render_for(&bx.cx, &fill);
    let mark = |s: &str| crate::packet::footnotes_to_markers(s);

    let mut blocks: Vec<Block> = Vec::new();
    let several = s.hazards.len() > 1;
    for p in &s.hazards {
        blocks.push(Block::Para(hazard_line(bx, p, several)));
    }
    if !r.use_when.is_empty() {
        blocks.push(heading(1, "Use this when"));
        blocks.push(Block::Para(bx.inl(&mark(&r.use_when))));
    }
    let steps = |items: &[String], memory: bool| {
        Block::Steps(
            items
                .iter()
                .map(|i| Step {
                    text: bx.inl(&mark(i)),
                    memory,
                })
                .collect(),
        )
    };
    if !r.do_first.is_empty() {
        blocks.push(heading(1, "Do first"));
        blocks.push(steps(&r.do_first, true));
    }
    if !r.then.is_empty() {
        blocks.push(heading(1, "Then"));
        blocks.push(steps(&r.then, false));
    }
    if !r.leave_or_stay.is_empty() {
        blocks.push(Block::Decision(Decision {
            question: "Leave or stay?".to_owned(),
            branches: r
                .leave_or_stay
                .iter()
                .map(|x| branch(bx, &mark(x)))
                .collect(),
        }));
    }
    // Where and who: the block's own lines, then the household's answers its steps use.
    let own: Vec<&str> = c
        .sections
        .where_who
        .iter()
        .flat_map(|l| braces(l))
        .collect();
    let mut rows: Vec<FieldRow> = r.where_who.iter().map(|l| where_row(l)).collect();
    let mut added = 0;
    for n in used.borrow().iter() {
        let label = answer_label(n);
        if n == "county" || label.is_empty() || own.contains(&n.as_str()) {
            continue;
        }
        let value = answer(bx, n);
        // A write-in line of the block's own with the same label takes the answer instead.
        if let Some(r) = rows
            .iter_mut()
            .find(|r| r.label.trim().eq_ignore_ascii_case(label))
        {
            if r.value.is_none() {
                r.value = value;
            }
            continue;
        }
        if added < MAX_ADDED_ROWS {
            rows.push(row(label, value.as_deref()));
            added += 1;
        }
    }
    // Where the engine's rows sit in `blocks`, so they can yield to the page's promise below.
    let fields_at = (!rows.is_empty()).then(|| {
        blocks.push(heading(1, "Where and who"));
        blocks.push(Block::Fields(rows));
        blocks.len() - 1
    });
    let bullets =
        |items: &[String]| Block::Bullets(items.iter().map(|i| bx.inl(&mark(i))).collect());
    if !r.do_not.is_empty() {
        blocks.push(heading(1, "Do not"));
        blocks.push(bullets(&r.do_not));
    }
    if !r.when_over.is_empty() {
        blocks.push(heading(1, "When it is over"));
        blocks.push(bullets(&r.when_over));
    }
    let mut p = page(&c.meta.id, &c.meta.title, PageKind::Checklist, blocks);
    // The block promises its length (DESIGN-DELTA-v3 §5.4); the proxy test holds it to it. The
    // household's own answers can be long, so the rows the engine added to "Where and who" make
    // room first, last added first (they repeat answers the steps already print).
    let pages = if c.pages >= 2 { 2.0 } else { 1.0 };
    p.fit = if c.pages >= 2 { Fit::Two } else { Fit::One };
    if let Some(at) = fields_at {
        while added > 0 && super::fit::load(&p) > pages {
            if let Block::Fields(rows) = &mut p.blocks[at] {
                rows.pop();
            }
            added -= 1;
        }
        // Nothing left to show: no heading over an empty box.
        if matches!(&p.blocks[at], Block::Fields(rows) if rows.is_empty()) {
            p.blocks.drain(at - 1..=at);
        }
    }
    p
}

/// A "Where and who" line (`Label: answer`, `Label: blank` or `Label:`) as a field row.
fn where_row(line: &str) -> FieldRow {
    let (label, rest) = match line.split_once(':') {
        Some((l, r)) => (l.trim(), r.trim()),
        None => (line.trim(), ""),
    };
    let value: Option<String> =
        if rest.is_empty() || rest.starts_with(rr_content::checklist::BLANK_MARK) {
            None
        } else {
            let v: String = rest
                .chars()
                .filter(|c| *c != USER_OPEN && *c != USER_CLOSE)
                .collect();
            (!v.trim().is_empty()).then(|| v.trim().to_owned())
        };
    row(label, value.as_deref())
}

/// A "Leave or stay" line as a branch. The content writes two forms: "**Leave if** <condition>.
/// <action>." (the condition is the bold lead and the rest of its first sentence) and "**Do this
/// if** <condition>: <action>." (the condition runs to the first colon); a line with neither
/// ("**Go to** the shelter if …") is all condition. A `{ref:page}` in it is where to turn.
/// Punctuation stays where the content put it, so the condition and the action read back as
/// written.
fn branch(bx: &Bx<'_>, marked: &str) -> Branch {
    let (text, go_to) = take_ref(bx, marked);
    let (when, then) = split_condition(&text);
    Branch {
        when: bx.inl(when.trim()),
        then: bx.inl(then.trim()),
        go_to,
    }
}

/// The text without its first cross-reference to a page in the binder, and that page.
fn take_ref(bx: &Bx<'_>, marked: &str) -> (String, Option<String>) {
    let open = rr_content::checklist::REF_OPEN;
    let Some(start) = marked.find(open) else {
        return (marked.to_owned(), None);
    };
    let after = &marked[start + open.len()..];
    let Some(end) = after.find('}') else {
        return (marked.to_owned(), None);
    };
    let page = after[..end].trim();
    if !bx.directory.contains_key(page) {
        return (marked.to_owned(), None);
    }
    let text = format!("{}{}", &marked[..start], &after[end + 1..]);
    (text.trim().to_owned(), Some(page.to_owned()))
}

/// The byte offset just past the first sentence of marked text (after its bold lead, if any):
/// a full stop, question or exclamation mark outside the household's words, with any citation
/// markers after it, followed by a space or the end.
fn first_sentence_end(s: &str) -> Option<usize> {
    // Past the bold lead, if the text opens with one.
    let mut i = match s.strip_prefix("**") {
        Some(rest) => 2 + rest.find("**").map_or(0, |e| e + 2),
        None => 0,
    };
    let mut in_user = false;
    while i < s.len() {
        let c = s[i..].chars().next()?;
        let next = i + c.len_utf8();
        match c {
            USER_OPEN => in_user = true,
            USER_CLOSE => in_user = false,
            '.' | '?' | '!' if !in_user => {
                let mut j = next;
                while s[j..].starts_with(crate::packet::OPEN) {
                    match s[j..].find(crate::packet::CLOSE) {
                        Some(e) => j += e + crate::packet::CLOSE.len_utf8(),
                        None => break,
                    }
                }
                if j >= s.len() || s[j..].starts_with(' ') {
                    return Some(j);
                }
            }
            _ => {}
        }
        i = next;
    }
    None
}

/// A branch's condition and its action: split after the first colon outside the household's
/// words when it comes before the end of the first sentence, else after the first sentence.
fn split_condition(s: &str) -> (&str, &str) {
    let sentence = first_sentence_end(s).unwrap_or(s.len());
    // Inside the household's words, a blank or a citation marker, a colon is not the content's.
    let mut inside: Option<char> = None;
    for (i, c) in s.char_indices() {
        if i >= sentence {
            break;
        }
        match (inside, c) {
            (None, USER_OPEN) => inside = Some(USER_CLOSE),
            (None, rr_content::checklist::BLANK_MARK) => {
                inside = Some(rr_content::checklist::BLANK_MARK);
            }
            (None, crate::packet::OPEN) => inside = Some(crate::packet::CLOSE),
            (Some(close), c) if c == close => inside = None,
            (None, ':') => return (&s[..=i], &s[i + 1..]),
            _ => {}
        }
    }
    split_first_sentence(s)
}

/// Marked text split after its first sentence: (the sentence, the rest).
fn split_first_sentence(s: &str) -> (&str, &str) {
    match first_sentence_end(s) {
        Some(at) => (&s[..at], &s[at..]),
        None => (s, ""),
    }
}

/// Tab 7's first page (§4.2, brief 1): `plan_forecast_48h` as a checklist-shaped page. The
/// block's opening paragraph says when to use it; each list becomes steps, and each headed
/// paragraph ("Before a hard freeze.") a heading with the paragraph as one step. The freeze steps
/// print where a cold wave, winter storm or ice storm is likely enough to list (a ten-year
/// chance of 1 in 100), the heat-wave steps where heat waves are, as in the v2 packet.
pub(super) fn forecast(bx: &Bx<'_>) -> Page {
    let cx = &bx.cx;
    let freeze = ["cold_wave", "winter_weather", "ice_storm"]
        .iter()
        .any(|h| cx.hazard_relevant(h));
    let heat = cx.hazard_relevant("heat_wave");
    let mut blocks: Vec<Block> = Vec::new();
    if let Some(g) = cx.blocks_for("plan:forecast_48h").into_iter().next() {
        let paras = crate::packet::paragraphs(&cx.guidance(g, None, None));
        for (i, p) in paras.iter().enumerate() {
            if p.starts_with("**Before a hard freeze.") && !freeze {
                continue;
            }
            if p.starts_with("**Before a heat wave.") && !heat {
                continue;
            }
            match bx.prose_block(p) {
                Block::Bullets(items) | Block::Numbered(items) => blocks.push(Block::Steps(
                    items
                        .into_iter()
                        .map(|text| Step {
                            text,
                            memory: false,
                        })
                        .collect(),
                )),
                Block::Heading(h) => blocks.push(heading(1, &h.text)),
                _ if i == 0 => {
                    blocks.push(heading(1, "Use this when"));
                    blocks.push(Block::Para(bx.inl(p)));
                }
                _ => {
                    // "**Before a hard freeze.** Let … . Make sure …[^id]": a heading, then the
                    // paragraph as one step, so it keeps its citation.
                    let (lead, rest) = match p.strip_prefix("**").and_then(|r| r.split_once("**")) {
                        Some((lead, rest)) => (Some(lead.trim_end_matches('.')), rest),
                        None => (None, p.as_str()),
                    };
                    if let Some(l) = lead {
                        blocks.push(heading(1, l));
                    }
                    blocks.push(Block::Steps(vec![Step {
                        text: bx.inl(rest.trim()),
                        memory: false,
                    }]));
                }
            }
        }
    }
    page(
        "forecast",
        super::FORECAST_TITLE,
        PageKind::Checklist,
        blocks,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branches_split_where_the_content_splits_them() {
        let (when, then) = split_first_sentence("**Leave if** it burns. Go now.");
        assert_eq!((when, then.trim()), ("**Leave if** it burns.", "Go now."));
        let (when, then) = split_first_sentence("**Go to** somewhere safe");
        assert_eq!((when, then), ("**Go to** somewhere safe", ""));
        let (when, then) = split_condition("**Do this if** you are sick: stay home. Rest.");
        assert_eq!(
            (when, then.trim()),
            ("**Do this if** you are sick:", "stay home. Rest.")
        );
        let (when, then) = split_condition("**Leave if** told to. Go to: the school.");
        assert_eq!(
            (when, then.trim()),
            ("**Leave if** told to.", "Go to: the school.")
        );
        let u = user("Room 2: the hall");
        let line = format!("**Go to** {u} now.");
        let (when, then) = split_condition(&line);
        assert_eq!(
            (when, then),
            (line.as_str(), ""),
            "a colon in the household's words"
        );
        let blank = rr_content::checklist::blank_marker(30);
        let line = format!("**Go to** {blank} if the home cannot be lived in.");
        let (when, then) = split_condition(&line);
        assert_eq!(
            (when, then),
            (line.as_str(), ""),
            "the colon in a blank marker"
        );
    }

    #[test]
    fn households_in_100_read_as_the_delta_words_them() {
        assert_eq!(in_100(0.4), "about 40 in 100");
        assert_eq!(in_100(0.004), "fewer than 1 in 100");
        assert_eq!(in_100(0.99), "almost all");
        assert_eq!(event_name("boil_water_notice"), "Boil-water notice");
        assert_eq!(event_name("x"), "x");
    }
}
