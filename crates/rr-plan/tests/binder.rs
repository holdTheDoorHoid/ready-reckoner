//! The binder (DESIGN-DELTA-v3 §4, §5; binder brief §3): on every fixture household, planned on
//! the repository's data packs (core and places), the binder has its ten parts in order, passes
//! `Binder::check`, leaves no placeholder, marker or footnote behind, gives every ranked hazard,
//! everyday emergency and opted-in rare family a checklist page in the right tab and order, keeps
//! every one-page and two-page promise by the fit proxy, prints the household's own words exactly
//! as written (and escaped in the Markdown), and numbers its sources in the order it first uses
//! them.

mod common;

use std::collections::BTreeSet;

use common::outputs;
use rr_plan::binder::{self, PARTS, REQUIRE_ALL_CHECKLISTS, fit, markdown};
use rr_types::binder::{Binder, Block, Fit, Inline, Page, PageKind};
use rr_types::{HazardDisplay, PlanInput, PlanOutput};

fn fixture(name: &str) -> &'static (&'static str, PlanInput, PlanOutput) {
    outputs()
        .iter()
        .find(|(n, _, _)| *n == name)
        .unwrap_or_else(|| panic!("no fixture {name}"))
}

/// Every string a page shows: its title, headings, running text (links as their words), field
/// labels and values, table headers, callout titles, decisions, map captions, cards and logs.
fn page_texts(p: &Page) -> Vec<String> {
    fn inl(v: &[Inline], out: &mut Vec<String>) {
        for i in v {
            match i {
                Inline::T(t) | Inline::B(t) => out.push(t.clone()),
                Inline::Link(l) => out.push(l.text.clone()),
                Inline::Cite(_) | Inline::Blank(_) => {}
            }
        }
    }
    fn block(b: &Block, out: &mut Vec<String>) {
        match b {
            Block::Heading(h) => out.push(h.text.clone()),
            Block::Para(v) => inl(v, out),
            Block::Bullets(items) | Block::Numbered(items) => {
                for i in items {
                    inl(i, out);
                }
            }
            Block::Steps(steps) => {
                for s in steps {
                    inl(&s.text, out);
                }
            }
            Block::Fields(rows) => {
                for r in rows {
                    out.push(r.label.clone());
                    out.extend(r.value.clone());
                }
            }
            Block::Table(t) => {
                out.extend(t.header.iter().cloned());
                for c in t.rows.iter().flatten() {
                    inl(c, out);
                }
            }
            Block::Callout(c) => {
                out.extend(c.title.clone());
                for b in &c.blocks {
                    block(b, out);
                }
            }
            Block::Decision(d) => {
                out.push(d.question.clone());
                for br in &d.branches {
                    inl(&br.when, out);
                    inl(&br.then, out);
                }
            }
            Block::MapSlot(m) => out.push(m.caption.clone()),
            Block::Cards(cards) => {
                for c in cards {
                    out.push(c.title.clone());
                    for l in &c.lines {
                        inl(l, out);
                    }
                }
            }
            Block::Log(l) => out.extend(l.columns.iter().cloned()),
            Block::PageBreak(_) => {}
        }
    }
    let mut out = vec![p.title.clone()];
    for b in &p.blocks {
        block(b, &mut out);
    }
    out
}

/// Every string in the binder, page by page.
fn all_texts(b: &Binder) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = vec![
        ("binder".into(), b.title.clone()),
        ("binder".into(), b.household.clone()),
        ("binder".into(), b.location.clone()),
    ];
    for p in b.pages() {
        v.extend(page_texts(p).into_iter().map(|t| (p.id.clone(), t)));
    }
    v
}

/// The fields of a page: (label, value).
fn fields(p: &Page) -> Vec<(String, Option<String>)> {
    p.blocks
        .iter()
        .flat_map(|b| match b {
            Block::Fields(rows) => rows
                .iter()
                .map(|r| (r.label.clone(), r.value.clone()))
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn every_binder_has_the_ten_parts_in_order_and_passes_the_check() {
    for (name, _, out) in outputs() {
        let b = &out.binder;
        let parts: Vec<(u8, &str, &str, &str)> = b
            .parts
            .iter()
            .map(|p| {
                (
                    p.tab,
                    p.id.as_str(),
                    p.title.as_str(),
                    p.short_title.as_str(),
                )
            })
            .collect();
        assert_eq!(parts, PARTS.to_vec(), "{name}");
        assert_eq!(b.check(), Vec::<String>::new(), "{name}");
        assert!(b.parts.iter().all(|p| !p.pages.is_empty()), "{name}");
        // The pages every binder has, and the targets of every `{ref:…}` a checklist may use.
        for id in binder::FIXED_PAGES.iter().map(|p| p.0) {
            assert!(b.page(id).is_some(), "{name}: no page {id}");
        }
        for id in rr_content::checklist::REF_TARGETS {
            assert!(b.page(id).is_some(), "{name}: no page for {{ref:{id}}}");
        }
        assert_eq!(b.generated_on, out.binder.generated_on);
        assert!(b.title.starts_with("Emergency binder for "), "{name}");
        assert_eq!(b.status_line, rr_plan::packet::STATUS_LINE);
        assert!(
            !b.credits.is_empty() && b.credits[0].contains("National Risk Index"),
            "{name}: the National Risk Index statement comes first"
        );
    }
    // On the sample counties too (no data packs, no hospital list).
    for (name, input) in rr_types::fixtures::all() {
        let out = common::fixture_engine().assess(&input).unwrap();
        assert_eq!(
            out.binder.check(),
            Vec::<String>::new(),
            "{name} (sample counties)"
        );
        assert_eq!(out.binder.parts.len(), 10, "{name}");
    }
}

#[test]
fn nothing_is_left_over() {
    let placeholders: Vec<String> = rr_content::checklist::PLACEHOLDERS
        .iter()
        .map(|(n, _)| format!("{{{n}}}"))
        .chain(
            rr_plan::packet::PLACEHOLDERS
                .iter()
                .map(|s| (*s).to_owned()),
        )
        .collect();
    for (name, _, out) in outputs() {
        for (page, t) in all_texts(&out.binder) {
            for bad in ["{ref:", "{if:", "{/if}", "[^", "blank:"] {
                assert!(!t.contains(bad), "{name} {page}: {bad} in {t:?}");
            }
            for p in &placeholders {
                assert!(!t.contains(p.as_str()), "{name} {page}: {p} in {t:?}");
            }
            assert!(
                !t.chars().any(|c| ('\u{1}'..='\u{6}').contains(&c)),
                "{name} {page}: a marker in {t:?}"
            );
        }
        let md = markdown::render(&out.binder);
        for bad in ["{ref:", "{if:", "[^", "\u{3}", "\u{1}", "\u{5}"] {
            assert!(!md.contains(bad), "{name}: {bad:?} in the Markdown");
        }
    }
}

/// Every ranked hazard, everyday emergency and opted-in rare family has a checklist page
/// (DESIGN-DELTA-v3 §5.1): a failure once the content groups are in
/// ([`REQUIRE_ALL_CHECKLISTS`]), a list of gaps before.
#[test]
fn every_ranked_hazard_has_a_checklist() {
    for (name, input, out) in outputs() {
        let a = common::run(input);
        let gaps = binder::problems(&a, rr_content::content());
        if REQUIRE_ALL_CHECKLISTS {
            assert_eq!(gaps, Vec::<String>::new(), "{name}");
        } else {
            for g in &gaps {
                eprintln!("{name}: {g}");
            }
        }
        let b = &out.binder;
        let content = rr_content::content();
        let opted = input.dials.rare_families();
        for p in out
            .register
            .iter()
            .filter(|p| p.display == HazardDisplay::Ranked || opted.contains(&p.id.as_str()))
        {
            match content.checklist_for(&format!("hazard:{}", p.id)) {
                Some(c) => assert!(b.page(&c.meta.id).is_some(), "{name}: {} has no page", p.id),
                None if REQUIRE_ALL_CHECKLISTS => panic!("{name}: {} has no block", p.id),
                None => {}
            }
        }
        for e in rr_content::ids::EVENTS {
            match content.checklist_for(&format!("event:{e}")) {
                Some(c) => assert!(b.page(&c.meta.id).is_some(), "{name}: {e} has no page"),
                None if REQUIRE_ALL_CHECKLISTS => panic!("{name}: {e} has no block"),
                None => {}
            }
        }
    }
}

/// Tabs 6 to 8 hold the checklists by onset (§5.2); tab 7 opens with the forecast list; within
/// a tab the everyday emergencies come first, "Something else" last, and the rest by the ten-year
/// chance of their most likely hazard, highest first (§5.1).
#[test]
fn checklists_sit_in_their_tabs_in_order() {
    let content = rr_content::content();
    for (name, input, out) in outputs() {
        let b = &out.binder;
        assert_eq!(
            b.parts[6].pages.first().map(|p| p.id.as_str()),
            Some("forecast"),
            "{name}"
        );
        let opted = input.dials.rare_families();
        let chance = |id: &str| {
            let c = content.checklist(id).unwrap();
            out.register
                .iter()
                .filter(|p| p.display == HazardDisplay::Ranked || opted.contains(&p.id.as_str()))
                .filter(|p| c.applies_to(&format!("hazard:{}", p.id)))
                .map(|p| -rr_types::math::exp_m1(-10.0 * p.rate_per_year))
                .fold(0.0, f64::max)
        };
        for (tab, onset) in [
            (6usize, rr_content::Onset::Now),
            (7, rr_content::Onset::Coming),
            (8, rr_content::Onset::Ongoing),
        ] {
            let pages: Vec<&Page> = b.parts[tab - 1]
                .pages
                .iter()
                .filter(|p| p.id.starts_with("check_"))
                .collect();
            let mut last_group = 0;
            let mut last_chance = f64::INFINITY;
            for p in &pages {
                let c = content.checklist(&p.id).unwrap();
                assert_eq!(c.onset, onset, "{name}: {} in tab {tab}", p.id);
                assert_eq!(p.kind, PageKind::Checklist, "{name}");
                let event = c.meta.applies_to.iter().any(|a| a.starts_with("event:"));
                let group = if c.applies_to("event:something_else") {
                    2
                } else if event {
                    0
                } else {
                    1
                };
                assert!(group >= last_group, "{name}: {} out of its group", p.id);
                if group != last_group {
                    last_chance = f64::INFINITY;
                }
                if group == 1 {
                    let x = chance(&p.id);
                    assert!(x <= last_chance + 1e-12, "{name}: {} out of order", p.id);
                    last_chance = x;
                }
                last_group = group;
            }
        }
        // Every checklist page is in the part its onset names, once.
        let ids: Vec<&str> = b.pages().map(|p| p.id.as_str()).collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "{name}");
    }
}

/// Which checklist? lists every everyday emergency and every ranked hazard by name, with a link
/// to its page, and the rare families the household did not opt into in one line.
#[test]
fn the_index_names_every_hazard_and_event() {
    for (name, input, out) in outputs() {
        let b = &out.binder;
        let index = b.page("index").unwrap();
        let rows: Vec<(String, Vec<Inline>)> = index
            .blocks
            .iter()
            .flat_map(|bl| match bl {
                Block::Table(t) => t
                    .rows
                    .iter()
                    .map(|r| (binder::text_of(&r[0]), r[1].clone()))
                    .collect(),
                _ => Vec::new(),
            })
            .collect();
        let named = |n: &str| rows.iter().any(|(x, _)| x == n);
        for (id, label) in binder::EVENT_NAMES {
            if id != binder::SOMETHING_ELSE {
                assert!(named(label), "{name}: no row for {label}");
            }
        }
        for p in out
            .register
            .iter()
            .filter(|p| p.display == HazardDisplay::Ranked)
        {
            assert!(named(&p.name), "{name}: no row for {}", p.name);
        }
        for (row, turn) in &rows {
            assert!(
                turn.iter().any(|i| matches!(i, Inline::Link(_))),
                "{name}: {row} points nowhere"
            );
        }
        let opted = input.dials.rare_families();
        let text = page_texts(index).join("\n");
        for p in out
            .register
            .iter()
            .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        {
            if opted.contains(&p.id.as_str()) {
                assert!(named(&p.name), "{name}: opted-in {} not listed", p.name);
            } else if !named(&p.name) {
                assert!(
                    text.contains(&lower_first(&p.name)),
                    "{name}: {} neither listed nor named as left out",
                    p.name
                );
            }
        }
        assert!(text.contains("tick the family under Your settings") || opted.len() == 9);
    }
}

/// A name used mid-sentence: its first letter lower case, unless it opens an acronym.
fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match (c.next(), s.chars().nth(1)) {
        (Some(_), Some(second)) if second.is_uppercase() => s.to_owned(),
        (Some(f), _) => f.to_lowercase().chain(c).collect(),
        (None, _) => String::new(),
    }
}

/// The index also lists each page's title and the warning names people hear (the practitioner
/// review), pointing to the same page, when that page is in the binder; never a name twice.
#[test]
fn the_index_lists_titles_and_the_names_people_hear() {
    let content = rr_content::content();
    for (name, _, out) in outputs() {
        let b = &out.binder;
        let index = b.page("index").unwrap();
        let rows: Vec<(String, Vec<Inline>)> = index
            .blocks
            .iter()
            .flat_map(|bl| match bl {
                Block::Table(t) => t
                    .rows
                    .iter()
                    .map(|r| (binder::text_of(&r[0]), r[1].clone()))
                    .collect(),
                _ => Vec::new(),
            })
            .collect();
        let names: Vec<String> = rows.iter().map(|(n, _)| n.to_lowercase()).collect();
        let unique: BTreeSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "{name}: a name twice");
        let points_to = |n: &str| {
            rows.iter().find(|(x, _)| x == n).and_then(|(_, turn)| {
                turn.iter().find_map(|i| match i {
                    Inline::Link(l) => Some(l.to.clone()),
                    _ => None,
                })
            })
        };
        // A title that only adds an article to a listed name ("An attack or threat closes your
        // area") is not listed again.
        let key = |n: &str| {
            let l = n.to_lowercase();
            ["a ", "an ", "the "]
                .iter()
                .find_map(|a| l.strip_prefix(a).map(str::to_owned))
                .unwrap_or(l)
        };
        for p in b.pages().filter(|p| p.id.starts_with("check_")) {
            let said = names.iter().any(|n| key(n) == key(&p.title));
            assert!(said, "{name}: the title {} is not in the index", p.title);
        }
        for (alias, target) in binder::ALIASES {
            let Some(page) = content.checklist_for(target).map(|c| c.meta.id.clone()) else {
                continue;
            };
            if b.page(&page).is_none() {
                continue;
            }
            let covered = names.iter().any(|n| n.contains(&alias.to_lowercase()));
            assert!(covered, "{name}: {alias} is not in the index");
            if let Some(to) = points_to(alias) {
                assert_eq!(to, page, "{name}: {alias}");
            }
        }
    }
    let (_, _, phl) = fixture("philadelphia-renters-4");
    let text = page_texts(phl.binder.page("index").unwrap()).join("\n");
    for alias in [
        "Flash flood warning",
        "Extreme heat warning",
        "Terrorist attack",
    ] {
        assert!(text.contains(alias), "{alias}");
    }
    assert!(
        !text.contains("\nBlizzard\n"),
        "already in the title Winter storm or blizzard"
    );
}

/// A checklist never draws an empty box: a "Leave or stay" decision has branches, and a "Where
/// and who" heading is followed by rows.
#[test]
fn checklists_draw_no_empty_box() {
    for (name, _, out) in outputs() {
        for p in out.binder.pages().filter(|p| p.kind == PageKind::Checklist) {
            for (i, b) in p.blocks.iter().enumerate() {
                if let Block::Decision(d) = b {
                    assert!(!d.branches.is_empty(), "{name} {}", p.id);
                    for br in &d.branches {
                        assert!(!br.when.is_empty(), "{name} {}: an empty branch", p.id);
                    }
                }
                if matches!(b, Block::Heading(h) if h.text == "Where and who") {
                    assert!(
                        matches!(p.blocks.get(i + 1), Some(Block::Fields(rows)) if !rows.is_empty()),
                        "{name} {}",
                        p.id
                    );
                }
            }
        }
    }
}

/// Every `fit: one` page fits one printed page by the proxy, every `fit: two` page two
/// (DESIGN-DELTA-v3 §5.6); the checklists keep the promise their block makes.
#[test]
fn fit_one_and_two_pages_fit() {
    let content = rr_content::content();
    for (name, _, out) in outputs() {
        for p in out.binder.pages() {
            let load = fit::load(p);
            match p.fit {
                Fit::One => assert!(load <= 1.0, "{name} {}: {load:.2} pages", p.id),
                Fit::Two => assert!(load <= 2.0, "{name} {}: {load:.2} pages", p.id),
                Fit::Flow => {}
            }
            if let Some(c) = content.checklist(&p.id) {
                let want = if c.pages >= 2 { Fit::Two } else { Fit::One };
                assert_eq!(p.fit, want, "{name} {}", p.id);
            }
        }
    }
}

/// The pages laid out to fit one sheet of paper do, for every fixture: the cover, how to use,
/// quick start, every person, the wallet cards (up to four people), every place, the home's
/// companions and the logs.
#[test]
fn the_one_page_pages_stay_on_one_page() {
    for (name, input, out) in outputs() {
        for p in out.binder.pages() {
            let one = matches!(
                p.kind,
                PageKind::Cover
                    | PageKind::HowToUse
                    | PageKind::QuickStart
                    | PageKind::Person
                    | PageKind::Place
                    | PageKind::Log
                    | PageKind::Vehicles
            ) && p.id != "special_needs"
                || (p.kind == PageKind::WalletCards && input.people.len() <= 4)
                || p.id == "contacts"
                || p.id == "people";
            if one {
                assert_eq!(p.fit, Fit::One, "{name} {}: {:.2}", p.id, fit::load(p));
            }
        }
    }
}

/// For the report: printed pages per tab by the proxy, and the words on each checklist page.
#[test]
fn page_counts_for_the_report() {
    for (name, _, out) in outputs() {
        let per_tab: Vec<String> = out
            .binder
            .parts
            .iter()
            .map(|p| {
                let n: u32 = p.pages.iter().map(fit::printed_pages).sum();
                format!("{}:{n}", p.tab)
            })
            .collect();
        let total: u32 = out.binder.pages().map(fit::printed_pages).sum();
        eprintln!("PAGES {name}: {total} ({})", per_tab.join(" "));
        for p in out.binder.pages().filter(|p| fit::load(p) > 1.0) {
            eprintln!("OVER {name} {}: {:.2} ({:?})", p.id, fit::load(p), p.fit);
        }
    }
    let (_, _, phl) = fixture("philadelphia-renters-4");
    for p in phl.binder.pages().filter(|p| p.kind == PageKind::Checklist) {
        let words: f64 = page_texts(p).iter().map(|t| fit::words(t)).sum();
        eprintln!(
            "WORDS {}: {words} words, load {:.2} ({:?})",
            p.id,
            fit::load(p),
            p.fit
        );
    }
}

/// Sources are numbered in the order the binder first cites them, and they are the first entries
/// of the provenance list, so a `cite` n is `provenance[n - 1]`.
#[test]
fn sources_follow_first_use() {
    for (name, _, out) in outputs() {
        let b = &out.binder;
        let mut seen: Vec<u32> = Vec::new();
        for p in b.pages() {
            let md = markdown::render(&Binder {
                parts: vec![rr_types::binder::Part {
                    id: "x".into(),
                    tab: 1,
                    title: "x".into(),
                    short_title: "x".into(),
                    pages: vec![p.clone()],
                }],
                ..b.clone()
            });
            for n in common::cited_numbers(&md) {
                let n = n as u32;
                if !seen.contains(&n) {
                    seen.push(n);
                }
            }
        }
        let want: Vec<u32> = (1..=seen.len() as u32).collect();
        assert_eq!(seen, want, "{name}: first uses out of order");
        assert_eq!(seen.len(), b.sources.len(), "{name}: every source is cited");
        for (s, c) in b.sources.iter().zip(&out.provenance) {
            assert_eq!(s.title, c.title, "{name}");
            assert_eq!(s.url.as_deref(), Some(c.url.as_str()), "{name}");
        }
        // The Sources page lists them all, numbered, with the data credits after them.
        let page = b.page("sources").unwrap();
        let listed = page.blocks.iter().find_map(|bl| match bl {
            Block::Numbered(items) => Some(items.len()),
            _ => None,
        });
        assert_eq!(listed, Some(b.sources.len()), "{name}");
    }
}

/// Web addresses in the credits and the sources are never Markdown-escaped (the v0.3 merge note:
/// a backslash had crept into the Eviction Lab address in every packet).
#[test]
fn web_addresses_are_printed_as_they_are() {
    for (name, _, out) in outputs() {
        let md = markdown::with_prepare(&out.binder, &out.prepare_markdown);
        for credit in &out.binder.credits {
            for word in credit.split_whitespace().filter(|w| w.starts_with("http")) {
                let url = word.trim_end_matches(['.', ',', ';', ')']);
                assert!(md.contains(url), "{name}: {url} is not printed as it is");
            }
        }
        assert!(!md.contains("evictionlab.org/\\#"), "{name}");
        for s in &out.binder.sources {
            if let Some(u) = &s.url {
                assert!(md.contains(u.as_str()), "{name}: {u}");
            }
        }
    }
}

/// Philadelphia filled in every profile: each answer is on its person's page (the place's on
/// the place page) exactly as written, the Markdown prints it unchanged, and anywhere else it
/// appears unchanged or, on the wallet cards only, with non-breaking hyphens in phone numbers.
#[test]
fn the_household_s_own_words_are_printed_as_written() {
    let (_, input, out) = fixture("philadelphia-renters-4");
    let b = &out.binder;
    let nb = |s: &str| s.replace('-', "\u{2011}");
    for (i, p) in input.people.iter().enumerate() {
        let pr = p.profile.as_ref().expect("a profile");
        let mut mine: Vec<String> = [
            &pr.name,
            &pr.date_of_birth,
            &pr.phone,
            &pr.email,
            &pr.conditions,
            &pr.allergies,
            &pr.blood_type,
            &pr.id_notes,
            &pr.notes,
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
        for c in [&pr.doctor, &pr.pharmacy].into_iter().flatten() {
            mine.extend(
                [&c.name, &c.phone, &c.address]
                    .into_iter()
                    .flatten()
                    .cloned(),
            );
        }
        if let Some(ins) = &pr.insurance {
            mine.extend(
                [
                    &ins.carrier,
                    &ins.plan_name,
                    &ins.member_id,
                    &ins.group_number,
                    &ins.phone,
                ]
                .into_iter()
                .flatten()
                .cloned(),
            );
        }
        for m in &pr.medications {
            mine.extend(
                [&m.name, &m.dose, &m.schedule, &m.purpose]
                    .into_iter()
                    .flatten()
                    .cloned(),
            );
        }
        let page = b.page(&format!("person_{}", i + 1)).unwrap();
        assert_eq!(page.title, pr.name.clone().unwrap());
        let texts = page_texts(page);
        let md = markdown::render(&Binder {
            parts: vec![rr_types::binder::Part {
                id: "x".into(),
                tab: 2,
                title: "x".into(),
                short_title: "x".into(),
                pages: vec![page.clone()],
            }],
            ..b.clone()
        });
        for s in &mine {
            assert!(
                texts.iter().any(|t| t.contains(s.as_str())),
                "{s:?} not on {}'s page",
                page.title
            );
            assert!(
                md.contains(&markdown::esc(s)),
                "{s:?} changed in the Markdown"
            );
        }
        // The place: on its own page, as written.
        if let Some(pl) = &pr.place {
            let place = b
                .pages()
                .find(|x| x.kind == PageKind::Place && Some(&x.title) == pl.name.as_ref())
                .expect("a place page");
            let texts = page_texts(place);
            for s in [
                &pl.address,
                &pl.phone,
                &pl.plan,
                &pl.pickup,
                &pl.safest_spot,
            ]
            .into_iter()
            .flatten()
            {
                assert!(texts.iter().any(|t| t.contains(s.as_str())), "{s:?}");
            }
        }
        // Elsewhere: unchanged, or with non-breaking hyphens on the wallet cards only.
        for (pid, t) in all_texts(b) {
            for s in &mine {
                if s.contains('-') && t.contains(&nb(s)) {
                    assert_eq!(pid, "wallet_cards", "{s:?} altered on {pid}");
                }
            }
        }
    }
}

/// Chicago answered none of the optional steps: every field is a line to write on, and no page
/// is empty.
#[test]
fn a_household_with_no_optional_answers_gets_blanks_and_no_empty_page() {
    let (_, input, out) = fixture("chicago-student-zero-budget-1");
    assert!(input.family_plan.is_none() && input.people.iter().all(|p| p.profile.is_none()));
    for p in out.binder.pages() {
        assert!(!p.blocks.is_empty(), "{} is empty", p.id);
        let words: f64 = page_texts(p).iter().skip(1).map(|t| fit::words(t)).sum();
        assert!(words > 0.0, "{} says nothing", p.id);
        if p.kind == PageKind::Checklist {
            // A checklist's own write-in lines and the household's answers are blank too.
            for (label, value) in fields(p) {
                if label != "County emergency office" {
                    assert!(value.is_none(), "{}: {label} = {value:?}", p.id);
                }
            }
            continue;
        }
        for (label, value) in fields(p) {
            assert!(value.is_none(), "{}: {label} has {value:?}", p.id);
        }
    }
    let person = out.binder.page("person_1").unwrap();
    assert_eq!(person.title, "Person 1 (adult)");
    assert!(
        out.binder.title.ends_with("1 adult"),
        "{}",
        out.binder.title
    );
}

/// The household's words are escaped in the Markdown, so they can never start emphasis, a table
/// cell, a heading or a link, and come back unchanged in the JSON.
#[test]
fn user_text_is_escaped_in_the_markdown() {
    let mut input = common::household("chicago-student-zero-budget-1");
    let weird = "**Bold** | pipe # hash [link](x) _under_";
    input.people[0].profile = Some(rr_types::PersonProfile {
        name: Some("Alex *star*".into()),
        notes: Some(weird.into()),
        ..Default::default()
    });
    input.family_plan = Some(rr_types::FamilyPlan {
        meeting_place_near: Some("The #1 [corner] | store".into()),
        ..Default::default()
    });
    let out = common::assess(&input);
    let md = markdown::render(&out.binder);
    assert!(md.contains(&markdown::esc(weird)), "the notes, escaped");
    assert!(md.contains("\\*\\*Bold\\*\\* \\| pipe \\# hash \\[link\\](x) \\_under\\_"));
    assert!(!md.contains(weird), "never raw");
    assert!(md.contains("## Alex \\*star\\*"), "the page title, escaped");
    assert!(
        md.contains("The \\#1 \\[corner\\] \\| store"),
        "the meeting place"
    );
    // Printed as written in the tree.
    let notes = fields(out.binder.page("person_1").unwrap())
        .into_iter()
        .find(|(l, _)| l == "Anything else a helper should know")
        .and_then(|(_, v)| v);
    assert_eq!(notes.as_deref(), Some(weird));
    // No table cell is split by a pipe in the household's words.
    for line in md.lines().filter(|l| l.starts_with('|')) {
        assert!(
            !line.contains(" | pipe") && !line.contains("] | store"),
            "{line}"
        );
    }
}

/// Philadelphia's county has hospitals with emergency rooms (CMS): a table of at most twelve, by
/// name, with the dataset's date and the citation, once the `places` pack is loaded (as the web
/// app loads it when the binder is shown, and as `common::engine()` now loads it too,
/// `DATA_DIR_PACKS`, wasm3 2026-10-01); a county the list has none for gets one sentence; with
/// the list not loaded (the core pack on its own; the sample counties) the page says nothing
/// about it.
#[test]
fn the_hospital_table_appears_where_the_county_has_hospitals() {
    let input = common::household("philadelphia-renters-4");
    let out = common::binder_engine().assess(&input).unwrap();
    assert_eq!(out.binder.check(), Vec::<String>::new());
    let page = out.binder.page("neighbourhood").unwrap();
    let table = page
        .blocks
        .iter()
        .find_map(|b| match b {
            Block::Table(t) if t.header.first().map(String::as_str) == Some("Hospital") => Some(t),
            _ => None,
        })
        .expect("a hospital table");
    assert!(!table.rows.is_empty() && table.rows.len() <= 12);
    let names: Vec<String> = table.rows.iter().map(|r| binder::text_of(&r[0])).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "alphabetical");
    assert!(
        names
            .iter()
            .any(|n| n.contains("TEMPLE UNIVERSITY HOSPITAL")),
        "{names:?}"
    );
    let text = page_texts(page).join("\n");
    assert!(text.contains("Hospitals with emergency rooms in Philadelphia County"));
    assert!(text.contains("August 13, 2026"), "the dataset's date");
    assert!(text.contains("Check before you need it"));

    // Blount County, Alabama: no hospital with an emergency room in the list.
    let mut input = common::household("philadelphia-renters-4");
    input.location.zip = None;
    input.location.county_fips = Some("01009".into());
    let out = common::binder_engine().assess(&input).unwrap();
    let page = out.binder.page("neighbourhood").unwrap();
    assert!(!page.blocks.iter().any(
        |b| matches!(b, Block::Table(t) if t.header.first().map(String::as_str) == Some("Hospital"))
    ));
    let text = page_texts(page).join("\n");
    assert!(
        text.contains("shows no hospital with an emergency room in Blount County, Alabama"),
        "{text}"
    );

    // Without the list (the core pack on its own; the sample counties), nothing.
    // `common::engine()` now loads `places` too (`DATA_DIR_PACKS`, wasm3 2026-10-01: the CLI's
    // default and the goldens show the table), so "core alone" needs its own engine here.
    let input = common::household("philadelphia-renters-4");
    let core_store = rr_plan::source::load_data_dir_packs(&rr_plan::golden::data_dir(), &["core"])
        .expect("the core pack alone loads");
    let core_only = rr_plan::Engine::new(core_store).expect("engine");
    for out in [
        core_only.assess(&input).unwrap(),
        common::fixture_engine().assess(&input).unwrap(),
    ] {
        let text = page_texts(out.binder.page("neighbourhood").unwrap()).join("\n");
        assert!(!text.contains("federal list of hospitals"), "{text}");
    }
    // With the list, every fixture's binder still holds together and keeps its promises.
    for (name, input) in rr_types::fixtures::all() {
        let out = common::binder_engine().assess(&input).unwrap();
        assert_eq!(out.binder.check(), Vec::<String>::new(), "{name}");
        for p in out.binder.pages() {
            let load = fit::load(p);
            match p.fit {
                Fit::One => assert!(load <= 1.0, "{name} {}: {load:.2}", p.id),
                Fit::Two => assert!(load <= 2.0, "{name} {}: {load:.2}", p.id),
                Fit::Flow => {}
            }
        }
    }
}

/// One wallet card per person, with their name or "Person 2 (child)", none empty; phone numbers
/// carry non-breaking hyphens so they never break across a card's lines (Detroit wrote its plan).
#[test]
fn every_person_has_a_named_wallet_card() {
    for (name, input, out) in outputs() {
        let page = out.binder.page("wallet_cards").unwrap();
        let cards = page
            .blocks
            .iter()
            .find_map(|b| match b {
                Block::Cards(c) => Some(c),
                _ => None,
            })
            .expect("cards");
        assert_eq!(cards.len(), input.people.len(), "{name}");
        for (i, (c, p)) in cards.iter().zip(&input.people).enumerate() {
            let who = match p.profile.as_ref().and_then(|x| x.name.as_deref()) {
                Some(n) => n.to_owned(),
                None => format!("Person {} (", i + 1),
            };
            assert!(c.title.contains(&who), "{name}: {}", c.title);
            assert!(c.lines.len() >= 7, "{name}");
            for l in &c.lines {
                assert!(!l.is_empty(), "{name}: an empty line");
            }
        }
    }
    let (_, input, out) = fixture("detroit-snap-3");
    let plan = input.family_plan.as_ref().unwrap();
    let phone = plan
        .out_of_area_contact
        .as_ref()
        .and_then(|c| c.phone.clone())
        .unwrap()
        .replace('-', "\u{2011}");
    let text = page_texts(out.binder.page("wallet_cards").unwrap()).join("\n");
    assert!(text.contains(&phone), "{text}");
    let chars: Vec<char> = text.chars().collect();
    for w in chars.windows(3) {
        assert!(
            !(w[0].is_ascii_digit() && w[1] == '-' && w[2].is_ascii_digit()),
            "a breakable hyphen on a card"
        );
    }
}

/// Detroit filled in the whole v2 family plan: every answer reaches a page, as written.
#[test]
fn the_family_plan_answers_reach_their_pages() {
    let (_, input, out) = fixture("detroit-snap-3");
    let plan = input.family_plan.as_ref().unwrap();
    let all: Vec<String> = all_texts(&out.binder)
        .into_iter()
        .filter(|(p, _)| p != "wallet_cards")
        .map(|(_, t)| t)
        .collect();
    let mut written: Vec<String> = [
        &plan.meeting_place_near,
        &plan.meeting_place_far,
        &plan.school_pickup,
        &plan.work_plans,
        &plan.shelter_spot_home,
        &plan.shelter_spot_work,
        &plan.where_we_would_go,
        &plan.neighbours_who_check,
        &plan.shutoff_gas,
        &plan.shutoff_water,
        &plan.shutoff_electric,
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect();
    written.extend(plan.routes.iter().cloned());
    for c in [&plan.out_of_area_contact, &plan.lawyer]
        .into_iter()
        .flatten()
    {
        written.extend(c.name.iter().cloned());
        written.extend(c.phone.iter().cloned());
    }
    for t in &plan.trusted_circle {
        written.extend(t.name.iter().cloned());
        written.extend(t.phone.iter().cloned());
    }
    assert!(written.len() >= 20);
    for w in &written {
        assert!(
            all.iter().any(|t| t.contains(w.as_str())),
            "Detroit lost {w:?}"
        );
    }
    let contacts = fields(out.binder.page("contacts").unwrap());
    assert!(contacts.iter().any(|(l, v)| l == "Out-of-area contact"
        && v.as_deref() == Some("Cousin Tanya in Columbus, 555-0140")));
}

/// The shelter plan on the Home page fits the home: no basement advice on the 14th floor in
/// Miami, the tenth-floor rule instead; an inside hallway for an apartment's tornado steps.
#[test]
fn shelter_advice_fits_the_home() {
    let home = |name: &str| page_texts(fixture(name).2.binder.page("home").unwrap()).join("\n");
    let miami = home("miami-condo-retiree-1");
    assert!(miami.contains("on or below the 10th floor"), "{miami}");
    assert!(!miami.contains(" or basement"), "{miami}");
    let coos = home("coos-bay-well-owner-2");
    assert!(coos.contains("to an inside room or basement"), "{coos}");
    let chicago = home("chicago-student-zero-budget-1");
    assert!(!chicago.contains("or basement"), "{chicago}");
    assert!(!home("philadelphia-renters-4").contains("10th floor"));
    // Where to shelter from strong wind prints once, on the Home page, for every household.
    for (name, _, out) in outputs() {
        let md = markdown::render(&out.binder);
        assert_eq!(md.matches("**Strong wind.**").count(), 1, "{name}");
        assert!(home(name).contains("Strong wind."), "{name}");
    }
    // Chicago is on the third floor of an apartment building: the tornado page's apartment step.
    let tornado = page_texts(
        fixture("chicago-student-zero-budget-1")
            .2
            .binder
            .page("check_tornado")
            .unwrap(),
    )
    .join("\n");
    assert!(tornado.contains("Use an inside hallway"), "{tornado}");
    assert!(!tornado.contains("Leave the mobile home"), "{tornado}");
}

/// The cover carries the status line, the dates and the place; What to expect carries the dial
/// sentence the model computes, the targets and the validation line cited to its registry entry.
#[test]
fn the_cover_and_what_to_expect_say_what_they_must() {
    for (name, input, out) in outputs() {
        let cover = page_texts(out.binder.page("cover").unwrap()).join("\n");
        assert!(cover.contains(rr_plan::packet::STATUS_LINE), "{name}");
        assert!(cover.contains("and whenever something changes"), "{name}");
        assert_eq!(
            out.binder.review_by,
            input.planning_date.add_months(12).unwrap()
        );
        let a = common::run(input);
        let expect = out.binder.page("what_to_expect").unwrap();
        let text = page_texts(expect).join("\n");
        assert!(
            text.contains(&rr_plan::packet::dial_sentence(&a.consequence)),
            "{name}"
        );
        assert!(
            text.contains("How well do these numbers hold up?"),
            "{name}"
        );
        let n = out
            .provenance
            .iter()
            .position(|c| c.id == rr_plan::validation::CITATION)
            .map(|i| i as u32 + 1)
            .expect("rr_validation_2026 in the provenance");
        let cited = expect.blocks.iter().any(|b| match b {
            Block::Para(v) => v
                .iter()
                .any(|i| matches!(i, Inline::Cite(c) if c.contains(&n))),
            _ => false,
        });
        assert!(cited, "{name}: the validation line cites [{n}]");
    }
}

/// After a disaster prints for every household with the county's declarations; "If it lasts for
/// months" only with the plan's long-horizon items; the four logs always.
#[test]
fn the_after_tab_has_the_recovery_page_and_the_logs() {
    for (name, _, out) in outputs() {
        let b = &out.binder;
        let after = page_texts(b.page("after").unwrap()).join("\n");
        let n = out.recovery.county_declarations_5yr.expect("declarations");
        let words = match n {
            0 => "no federal major-disaster declarations".to_owned(),
            1 => "one federal major-disaster declaration".to_owned(),
            n => format!("{n} federal major-disaster declarations"),
        };
        assert!(after.contains(&words), "{name}: {words}");
        assert_eq!(
            b.page("after_months").is_some(),
            !out.plan.long_horizon.is_empty(),
            "{name}"
        );
        if let Some(p) = b.page("after_months") {
            let text = page_texts(p).join("\n");
            for i in &out.plan.long_horizon {
                assert!(text.contains(&i.name), "{name}: {}", i.name);
            }
        }
        for id in [
            "log_damage",
            "log_expenses",
            "log_contacts",
            "log_medications",
        ] {
            let p = b.page(id).unwrap();
            assert!(
                p.blocks
                    .iter()
                    .any(|b| matches!(b, Block::Log(l) if l.rows >= 20))
            );
        }
    }
}

/// Special needs and health: the access-needs part (CMIST) only when someone has a need, naming
/// who; the 988 line always.
#[test]
fn special_needs_parts_print_only_when_needed() {
    for (name, input, out) in outputs() {
        let text = page_texts(out.binder.page("special_needs").unwrap()).join("\n");
        let needs = input.people.iter().any(|p| !p.access_needs.is_empty());
        assert_eq!(
            text.contains("Access and functional needs"),
            needs,
            "{name}"
        );
        assert!(text.contains("988"), "{name}");
    }
    let detroit = page_texts(
        fixture("detroit-snap-3")
            .2
            .binder
            .page("special_needs")
            .unwrap(),
    )
    .join("\n");
    assert!(detroit.contains("deaf or hard of hearing"), "{detroit}");
}

/// The map slots the web app fills: the neighborhood map on the Neighborhood page, the area and
/// region maps on Getting out.
#[test]
fn the_map_slots_are_where_the_web_app_looks() {
    for (name, _, out) in outputs() {
        for (id, kind, page) in binder::MAP_SLOTS {
            let p = out.binder.page(page).unwrap();
            assert!(
                p.blocks.iter().any(
                    |b| matches!(b, Block::MapSlot(m) if m.id == id && m.kind.as_str() == kind)
                ),
                "{name}: {id} not on {page}"
            );
        }
    }
}

/// The Prepare sheet: its sections in order, nothing left over, its own sources numbered from 1
/// without gaps, the data credits, the life-safety rules.
#[test]
fn the_prepare_sheet_has_its_sections_and_its_own_sources() {
    for (name, _, out) in outputs() {
        let p = &out.prepare_markdown;
        assert!(p.starts_with("# Prepare: what to do before\n"), "{name}");
        let mut at = 0;
        for h in rr_plan::packet::SECTION_HEADINGS {
            let found = p[at..]
                .find(&format!("\n{h}\n"))
                .unwrap_or_else(|| panic!("{name}: missing or out of order: {h}"));
            at += found + 1;
        }
        for bad in ["\u{1}", "\u{2}", "{if:", "{/if}", "[^", "{frequency}"] {
            assert!(!p.contains(bad), "{name}: {bad:?}");
        }
        let sources = &p[p.find("\n## Sources\n").unwrap()..];
        let body = &p[..p.find("\n## Sources\n").unwrap()];
        let cited: BTreeSet<usize> = common::cited_numbers(body).into_iter().collect();
        assert_eq!(
            cited.iter().copied().collect::<Vec<_>>(),
            (1..=cited.len()).collect::<Vec<_>>(),
            "{name}: its own sources, numbered without gaps"
        );
        for k in 1..=cited.len() {
            assert!(sources.contains(&format!("**{k}** ")), "{name}: {k}");
        }
        assert!(
            !sources.contains(&format!("**{}** ", cited.len() + 1)),
            "{name}"
        );
        assert!(sources.contains("not endorsed by FEMA"), "{name}");
        let plan = &p[p.find("\n## Your plan\n").unwrap()..];
        for r in &rr_plan::packet::SAFETY_RULES {
            assert!(
                plan.contains(&format!("- **{}:** {}", r.label, r.text)),
                "{name}: {}",
                r.label
            );
        }
        // The binder's material is not repeated here.
        for gone in [
            "## Wallet cards",
            "## Your risks",
            "## Your targets",
            "## Your family plan",
        ] {
            assert!(!p.contains(gone), "{name}: {gone}");
        }
    }
}

/// The golden Markdown is the binder, a rule, then the Prepare sheet.
#[test]
fn the_golden_markdown_joins_the_two() {
    let (_, _, out) = fixture("minot-missile-field-3");
    let md = markdown::with_prepare(&out.binder, &out.prepare_markdown);
    let (binder_md, prepare) = md.split_once(markdown::PREPARE_SEPARATOR).unwrap();
    assert_eq!(format!("{binder_md}\n"), markdown::render(&out.binder));
    assert_eq!(prepare, out.prepare_markdown);
    assert!(md.starts_with("# Emergency binder for Chris, Jordan and Avery\n"));
}

/// Pages a page renderer must recognise keep their kinds.
#[test]
fn page_kinds_are_what_the_renderers_expect() {
    let (_, _, out) = fixture("philadelphia-renters-4");
    let kind = |id: &str| out.binder.page(id).map(|p| p.kind);
    assert_eq!(kind("cover"), Some(PageKind::Cover));
    assert_eq!(kind("how_to_use"), Some(PageKind::HowToUse));
    assert_eq!(kind("quick_start"), Some(PageKind::QuickStart));
    assert_eq!(kind("index"), Some(PageKind::Index));
    assert_eq!(kind("contacts"), Some(PageKind::Contacts));
    assert_eq!(kind("wallet_cards"), Some(PageKind::WalletCards));
    assert_eq!(kind("home"), Some(PageKind::Home));
    assert_eq!(kind("neighbourhood"), Some(PageKind::Neighbourhood));
    assert_eq!(kind("getting_out"), Some(PageKind::GettingOut));
    assert_eq!(kind("pets"), Some(PageKind::Pets));
    assert_eq!(kind("vehicles"), Some(PageKind::Vehicles));
    assert_eq!(kind("documents"), Some(PageKind::Documents));
    assert_eq!(kind("inventory"), Some(PageKind::Inventory));
    assert_eq!(kind("risks_glance"), Some(PageKind::RisksGlance));
    assert_eq!(kind("after"), Some(PageKind::After));
    assert_eq!(kind("log_damage"), Some(PageKind::Log));
    assert_eq!(kind("sources"), Some(PageKind::Sources));
    // Quick start's steps are the memory items.
    let qs = out.binder.page("quick_start").unwrap();
    assert!(
        qs.blocks
            .iter()
            .any(|b| matches!(b, Block::Steps(s) if !s.is_empty() && s.iter().all(|x| x.memory)))
    );
}

/// A county-equivalent is named as the Census names it everywhere the binder prints the place
/// (hazards3 follow-up: `CountyRecord::name_full`): Richmond city, not Richmond County; San Juan
/// Municipio. The sample counties, which have no full name, keep the old rule.
#[test]
fn county_equivalents_are_named_as_the_census_names_them() {
    let mut input = common::household("philadelphia-renters-4");
    input.location.zip = None;
    input.location.county_fips = Some("51760".into());
    let out = common::assess(&input);
    assert!(
        out.binder.location.starts_with("Richmond city, Virginia"),
        "{}",
        out.binder.location
    );
    let md = markdown::render(&out.binder);
    assert!(
        !md.contains("Richmond County, Virginia"),
        "the old rule leaks"
    );
    assert!(md.contains("Richmond city, Virginia"));
    let (_, _, sj) = fixture("san-juan-2");
    assert!(
        sj.binder
            .location
            .starts_with("San Juan Municipio, Puerto Rico")
    );
    let sample = common::fixture_engine()
        .assess(&common::household("philadelphia-renters-4"))
        .unwrap();
    assert!(
        sample
            .binder
            .location
            .starts_with("Philadelphia County, Pennsylvania"),
        "{}",
        sample.binder.location
    );
}
