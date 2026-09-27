//! Packet v2 (DESIGN-DELTA §3): the family plan echoed as written, one wallet card per person,
//! the recovery page for every household, the access-and-functional-needs page only when someone
//! has a need, and the long-horizon section only when the plan has one.

mod common;

use common::outputs;
use rr_plan::packet::{NB_HYPHEN, WALLET_CARDS_HEADING};
use rr_types::{PlanInput, PlanOutput};

/// The part of a packet from one `##` heading to the next.
fn section<'a>(p: &'a str, heading: &str) -> Option<&'a str> {
    let start = p.find(&format!("\n{heading}\n"))? + 1;
    let rest = &p[start..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |e| e + 3);
    Some(&rest[..end])
}

fn fixture(name: &str) -> &'static (&'static str, PlanInput, PlanOutput) {
    outputs()
        .iter()
        .find(|(n, _, _)| *n == name)
        .unwrap_or_else(|| panic!("no fixture {name}"))
}

/// The wallet cards: one block quote per card, as the web app splits them.
fn cards(p: &str) -> Vec<Vec<&str>> {
    let s = section(p, WALLET_CARDS_HEADING.trim_start_matches('\n')).expect("wallet cards");
    let mut out: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in s.lines() {
        if let Some(l) = line.strip_prefix('>') {
            current.push(l.trim());
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

#[test]
fn every_person_has_a_wallet_card_and_none_is_empty() {
    const LINES: [&str; 7] = [
        "- Out-of-area contact: ",
        "- Meet near home: ",
        "- Meet outside the neighborhood: ",
        "- Lawyer: ",
        "- Trusted circle: ",
        "- Know by heart: ",
        "- Medical notes: ",
    ];
    for (name, input, out) in outputs() {
        let p = &out.packet_markdown;
        assert!(
            p.contains(&format!("\n{WALLET_CARDS_HEADING}\n")),
            "{name}: the app finds the cards by this heading"
        );
        let cards = cards(p);
        assert_eq!(cards.len(), input.people.len(), "{name}: one card a person");
        for (i, card) in cards.iter().enumerate() {
            assert!(
                card[0].starts_with(&format!("**Wallet card: person {} (", i + 1)),
                "{name}: {card:?}"
            );
            for want in LINES {
                let line = card
                    .iter()
                    .find(|l| l.starts_with(want))
                    .unwrap_or_else(|| panic!("{name}: card {} has no {want}", i + 1));
                // Something to read or a line to write on, never an empty field.
                assert!(line.len() > want.len(), "{name}: {line}");
                assert!(!line.contains("not answered"), "{name}: {line}");
            }
        }
    }
}

#[test]
fn phone_numbers_on_the_cards_never_break() {
    let (_, input, out) = fixture("detroit-snap-3");
    let plan = input
        .family_plan
        .as_ref()
        .expect("Detroit wrote a family plan");
    let cards = cards(&out.packet_markdown);
    let tanya = plan.out_of_area_contact.as_ref().unwrap();
    let phone = tanya
        .phone
        .as_deref()
        .unwrap()
        .replace('-', &NB_HYPHEN.to_string());
    for card in &cards {
        let text = card.join("\n");
        assert!(text.contains(&phone), "{text}");
        // No ASCII hyphen between two digits anywhere on a card.
        let chars: Vec<char> = text.chars().collect();
        for w in chars.windows(3) {
            assert!(
                !(w[0].is_ascii_digit() && w[1] == '-' && w[2].is_ascii_digit()),
                "a breakable hyphen in {text}"
            );
        }
    }
}

/// Everything the household wrote on the family-plan screen is printed word for word; nothing in
/// it is computed with. Detroit filled in the whole plan; Philadelphia wrote none, so every field
/// is a line to write on.
#[test]
fn the_family_plan_echoes_what_the_household_wrote() {
    let (_, input, out) = fixture("detroit-snap-3");
    let plan = input.family_plan.as_ref().unwrap();
    let s = section(&out.packet_markdown, "## Your family plan").expect("family plan");
    let nb = |t: &str| t.replace('-', &NB_HYPHEN.to_string());
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
        written.extend(c.phone.iter().map(|p| nb(p)));
    }
    for t in &plan.trusted_circle {
        written.extend(t.name.iter().cloned());
        written.extend(t.phone.iter().map(|p| nb(p)));
    }
    written.extend(plan.numbers_by_heart.iter().map(|n| nb(n)));
    assert!(written.len() >= 20, "{written:?}");
    for w in &written {
        assert!(s.contains(w.as_str()), "Detroit's plan lost {w:?}");
    }
    assert!(!s.contains("__________ |"), "Detroit filled in every row");
    assert!(s.contains("This is the plan you wrote"));

    let (_, input, out) = fixture("philadelphia-renters-4");
    assert!(input.family_plan.is_none());
    let s = section(&out.packet_markdown, "## Your family plan").unwrap();
    assert!(
        s.contains("| Meeting place near home | __________ |"),
        "{s}"
    );
    assert!(s.contains("| Out-of-area contact | __________ |"), "{s}");
    assert!(s.contains("Fill this in together"));
    assert!(!s.contains("not answered"));
}

/// "After a disaster: the first 30 days" (review RR-P06) prints for every household, with the
/// county's federal disaster declarations where the pack has them.
#[test]
fn every_packet_has_the_recovery_page() {
    for (name, _, out) in outputs() {
        let s = section(
            &out.packet_markdown,
            "## After a disaster: the first 30 days",
        )
        .unwrap_or_else(|| panic!("{name}: no recovery page"));
        assert!(s.lines().count() > 5, "{name}: {s}");
        let n = out
            .recovery
            .county_declarations_5yr
            .unwrap_or_else(|| panic!("{name}: no declaration count"));
        let words = match n {
            0 => "no federal major-disaster declarations".to_owned(),
            1 => "one federal major-disaster declaration".to_owned(),
            n => format!("{n} federal major-disaster declarations"),
        };
        assert!(s.contains(&words), "{name}: {words}");
        assert!(
            out.recovery
                .sources
                .iter()
                .any(|c| c == "openfema_declarations"),
            "{name}"
        );
    }
}

/// The access-and-functional-needs page (CMIST; review RR-P08) prints only when someone in the
/// household has a need, and names who.
#[test]
fn the_access_needs_page_prints_only_when_someone_has_a_need() {
    let mut with = 0;
    for (name, input, out) in outputs() {
        let needs = input.people.iter().any(|p| !p.access_needs.is_empty());
        let s = section(&out.packet_markdown, "## Access and functional needs");
        assert_eq!(s.is_some(), needs, "{name}");
        if let Some(s) = s {
            with += 1;
            assert!(s.contains("**In your household:** person "), "{name}");
        }
    }
    // Detroit (a grandparent who is hard of hearing), Galveston (home health care) and San Juan
    // (Spanish speakers) at least.
    assert!(with >= 3, "{with}");
    let (_, _, out) = fixture("detroit-snap-3");
    let s = section(&out.packet_markdown, "## Access and functional needs").unwrap();
    assert!(s.contains("deaf or hard of hearing"), "{s}");
}

/// The long-horizon section ("If it lasts for months") prints only when the plan has one, and
/// lists each of its items with the month it starts.
#[test]
fn the_long_horizon_section_prints_only_with_the_plan_s_long_horizon_items() {
    let mut with = 0;
    for (name, _, out) in outputs() {
        let s = section(&out.packet_markdown, "## If it lasts for months");
        assert_eq!(
            s.is_some(),
            !out.plan.long_horizon.is_empty(),
            "{name}: the section follows the plan"
        );
        if let Some(s) = s {
            with += 1;
            // How likely a cut of months is, from the household's own power curve, ranges only.
            assert!(
                s.contains(
                    "**How likely here.** From your own power curve, power out for two \
                     months or more: "
                ),
                "{name}: {s}"
            );
            assert!(s.contains("; for three months or more: "), "{name}");
            for i in &out.plan.long_horizon {
                assert!(
                    s.contains(&format!("- [ ] {}", i.name)),
                    "{name}: {} missing",
                    i.name
                );
            }
        }
    }
    // San Juan switched it on; Coos Bay, Hays and Cameron have a months-long power target.
    assert!(with >= 2, "{with}");
    assert!(
        section(
            &fixture("san-juan-2").2.packet_markdown,
            "## If it lasts for months"
        )
        .is_some()
    );
}

/// "How well do these numbers hold up?" cites the published table's registry entry
/// (`rr_validation_2026`) like any other source: a bracket in the line, the entry in the
/// provenance and in the Sources list with the registry's title and address, and no bare link.
#[test]
fn the_validation_line_cites_the_registry_entry() {
    let content = rr_content::content();
    let entry = content
        .citation(rr_plan::validation::CITATION)
        .expect("rr_validation_2026 is in content/citations.toml");
    for (name, _, out) in outputs() {
        let p = &out.packet_markdown;
        let targets = section(p, "## Your targets").unwrap();
        let line = targets
            .lines()
            .find(|l| l.starts_with("**How well do these numbers hold up?**"))
            .unwrap_or_else(|| panic!("{name}: no validation line"));
        assert!(!line.contains("http"), "{name}: a bare link in {line}");
        let n = out
            .provenance
            .iter()
            .position(|c| c.id == rr_plan::validation::CITATION)
            .map(|i| i + 1)
            .unwrap_or_else(|| panic!("{name}: rr_validation_2026 not in the provenance"));
        assert!(
            line.contains(&format!("could not model 1.[{n}]")),
            "{name}: [{n}] not on {line}"
        );
        let sources = &p[p.find("\n## Sources\n").unwrap()..];
        let listed = format!("**{n}** {}.", entry.title);
        assert!(sources.contains(&listed), "{name}: {listed}");
        assert!(
            sources.contains(&entry.url),
            "{name}: the registry's address"
        );
    }
}

/// The wind shelter advice prints once: the shelter plan's Strong wind paragraph is there for
/// every household (the content block, since a Serious card can displace the Minor wind card),
/// and a household without a wind card gets no other wind shelter line.
#[test]
fn the_wind_shelter_advice_prints_once() {
    const WIND_CARD_ADVICE: &str = "Pick your shelter spot now.";
    for (name, _, out) in outputs() {
        let p = &out.packet_markdown;
        let shelter = section(p, "## Your shelter plan").unwrap();
        assert_eq!(
            shelter.matches("**Strong wind.**").count(),
            1,
            "{name}: {shelter}"
        );
        assert_eq!(p.matches("**Strong wind.**").count(), 1, "{name}");
        let risks = section(p, "## Your risks").unwrap();
        let wind_card = risks.lines().any(|l| {
            l.starts_with("#### ")
                && ["Strong wind", "Tornado", "Hail", "Lightning"]
                    .iter()
                    .any(|h| l.ends_with(&format!(". {h}")))
        });
        if !wind_card {
            assert!(!p.contains(WIND_CARD_ADVICE), "{name}: a second wind line");
        }
    }
}
