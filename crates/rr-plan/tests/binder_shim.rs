//! Contract v3 (DESIGN-DELTA-v3 §4): every fixture's binder passes `Binder::check`, on the data
//! packs and on the sample counties. Until the binder workstream lands the binder is the
//! transitional one built from the v2 packet (`packet/shim.rs`), which must carry every paragraph
//! of the packet, number its sources as the packet does, and leave the Prepare sheet (the v2
//! packet, compared with the `.md` goldens) untouched.

mod common;

use rr_types::binder::{Block, Inline, PageKind};

/// The binder's paragraphs, in part and page order.
fn paragraphs(b: &rr_types::Binder) -> Vec<String> {
    b.pages()
        .flat_map(|p| &p.blocks)
        .map(|block| match block {
            Block::Para(inlines) => inlines
                .iter()
                .map(|i| match i {
                    Inline::T(t) => t.clone(),
                    other => panic!("the transitional binder holds only text: {other:?}"),
                })
                .collect::<String>(),
            other => panic!("the transitional binder holds only paragraphs: {other:?}"),
        })
        .collect()
}

#[test]
fn every_fixture_binder_passes_the_structural_check() {
    for (name, _, out) in common::outputs() {
        assert_eq!(out.binder.check(), Vec::<String>::new(), "{name}");
    }
    for (name, input) in rr_types::fixtures::all() {
        let out = common::fixture_engine().assess(&input).unwrap();
        assert_eq!(
            out.binder.check(),
            Vec::<String>::new(),
            "{name} (sample counties)"
        );
    }
}

#[test]
fn the_transitional_binder_carries_the_whole_packet() {
    for (name, input, out) in common::outputs() {
        let b = &out.binder;
        let md = &out.prepare_markdown;
        // Every paragraph of the packet is on some page, and nothing else is.
        let mut from_packet: Vec<String> = md
            .split("\n\n")
            .map(str::trim)
            .filter(|p| !p.is_empty() && !p.starts_with("# ") && !p.starts_with("## "))
            .map(str::to_owned)
            .collect();
        let mut from_binder = paragraphs(b);
        from_packet.sort();
        from_binder.sort();
        assert_eq!(from_binder, from_packet, "{name}");

        // One page per `##` section, plus the cover; the section titles are the page titles.
        let sections: Vec<&str> = md.lines().filter_map(|l| l.strip_prefix("## ")).collect();
        assert_eq!(b.pages().count(), sections.len() + 1, "{name}");
        for s in &sections {
            assert!(
                b.pages().any(|p| p.title == *s),
                "{name}: no page for ## {s}"
            );
        }
        assert_eq!(b.pages().next().map(|p| p.kind), Some(PageKind::Cover));
        assert_eq!(b.title, "Your preparedness packet");

        // The parts are the delta's, in its order, and the pages sit where §4.2 puts them.
        let part_of = |page: &str| {
            b.parts
                .iter()
                .find(|p| p.pages.iter().any(|pg| pg.id == page))
                .map(|p| p.id.as_str())
        };
        assert_eq!(part_of("wallet_cards"), Some("people"), "{name}");
        assert_eq!(part_of("documents"), Some("pets_vehicles_documents"));
        assert_eq!(part_of("after"), Some("after"));
        assert_eq!(part_of("sources"), Some("sources"));
        for part in &b.parts {
            let (tab, _, title, _) = rr_plan::packet::shim::PARTS
                .iter()
                .find(|p| p.1 == part.id)
                .unwrap_or_else(|| panic!("{name}: unknown part {}", part.id));
            assert_eq!((part.tab, part.title.as_str()), (*tab, *title));
        }

        // Sources are the provenance list, numbered as the packet's brackets number them.
        assert_eq!(b.sources.len(), out.provenance.len(), "{name}");
        for (i, (s, c)) in b.sources.iter().zip(&out.provenance).enumerate() {
            assert_eq!(s.n as usize, i + 1);
            assert_eq!((s.title.as_str(), s.expert), (c.title.as_str(), c.prior));
        }
        let most = common::cited_numbers(md).into_iter().max().unwrap_or(0);
        assert!(most <= b.sources.len(), "{name}: [{most}] has no source");

        // The cover facts.
        assert_eq!(b.generated_on, input.planning_date);
        assert_eq!(
            b.review_by,
            input.planning_date.add_months(12).unwrap(),
            "{name}"
        );
        assert!(md.contains(&format!("**Where:** {}", b.location)), "{name}");
        assert!(md.contains(&b.status_line), "{name}");
        assert!(!b.credits.is_empty(), "{name}: the data packs have credits");
        assert!(
            b.credits[0].contains("National Risk Index"),
            "{name}: the National Risk Index statement comes first: {}",
            b.credits[0]
        );
    }
}

#[test]
fn the_new_answers_leave_the_v2_packet_alone() {
    // Philadelphia and Minot carry the contract v3 sample answers (DESIGN-DELTA-v3 §10); the v2
    // packet prints none of them, and still asks the household to fill in its family plan.
    for name in ["philadelphia-renters-4", "minot-missile-field-3"] {
        let input = common::household(name);
        assert!(input.family_plan.is_some());
        let with = common::assess(&input);
        let mut bare = input.clone();
        for p in &mut bare.people {
            p.profile = None;
        }
        bare.family_plan = None;
        let without = common::assess(&bare);
        assert_eq!(with.prepare_markdown, without.prepare_markdown, "{name}");
        assert!(
            with.prepare_markdown
                .contains("Fill this in together, here or on the \"Your family plan\" screen."),
            "{name}"
        );
        for echo in ["555-0101", "555-0150", "Sample Street", "Biscuit"] {
            assert!(!with.prepare_markdown.contains(echo), "{name}: {echo}");
        }
    }
}
