//! End to end on every fixture household: the pipeline runs, every plan item cites, spending
//! never outruns the money, the packet has every section, and the outputs are well formed.

mod common;

use std::collections::BTreeSet;

use common::{cited_numbers, outputs};
use rr_plan::packet::{PLACEHOLDERS, SECTION_HEADINGS};
use rr_types::{BucketId, BucketKind, PlanItemKind, Target};

#[test]
fn every_fixture_assesses() {
    assert_eq!(outputs().len(), rr_types::fixtures::RAW.len());
    for (name, _, out) in outputs() {
        assert_eq!(out.api_version, rr_types::ENGINE_API_VERSION, "{name}");
        assert_eq!(out.content_version, rr_content::CONTENT_VERSION, "{name}");
        assert_eq!(
            Some(out.data_pack_version.as_str()),
            rr_data::DataStore::pack_version(common::engine().store()),
            "{name}: planned against the data packs"
        );
        assert!(
            out.location.data_note.as_deref() != Some(rr_plan::source::FIXTURE_DATA_NOTE),
            "{name}: real county records, not the sample counties"
        );
        assert_eq!(out.buckets.len(), 15, "{name}"); // 15 since contract v2 (clean_air)
        let ids: Vec<BucketId> = out.buckets.iter().map(|b| b.id).collect();
        assert_eq!(ids, BucketId::ALL, "{name}: buckets in BucketId order");
        assert!(!out.register.is_empty(), "{name}");
        assert!(!out.requirements.is_empty(), "{name}");
        assert!(!out.plan.months.is_empty(), "{name}");
    }
}

#[test]
fn every_plan_item_cites_a_source_in_the_provenance() {
    let content = rr_content::content();
    for (name, _, out) in outputs() {
        let provenance: BTreeSet<&str> = out.provenance.iter().map(|c| c.id.as_str()).collect();
        for m in &out.plan.months {
            for it in &m.items {
                let item = content
                    .item(it.item_id.as_str())
                    .unwrap_or_else(|| panic!("{name}: {} is not a catalogue item", it.item_id));
                assert!(
                    !item.citations.is_empty(),
                    "{name}: {} has no citation",
                    item.id
                );
                for c in &item.citations {
                    assert!(
                        provenance.contains(c.as_str()),
                        "{name}: {} cites {c}, missing from provenance",
                        item.id
                    );
                }
                assert!(!it.why.is_empty(), "{name}: {} has no why", it.item_id);
            }
        }
    }
}

#[test]
fn every_referenced_citation_is_in_the_provenance_once() {
    for (name, _, out) in outputs() {
        let ids: Vec<&str> = out.provenance.iter().map(|c| c.id.as_str()).collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(
            unique.len(),
            ids.len(),
            "{name}: duplicate provenance entries"
        );
        let mut referenced: Vec<&str> = Vec::new();
        for p in &out.register {
            referenced.extend(p.sources.iter().map(|c| c.as_str()));
        }
        for b in &out.buckets {
            referenced.extend(b.sources.iter().map(|c| c.as_str()));
            if let Some(r) = &b.relief {
                referenced.extend(r.sources.iter().map(|c| c.as_str()));
            }
        }
        for s in &out.scenarios {
            referenced.extend(s.sources.iter().map(|c| c.as_str()));
        }
        for l in &out.requirements {
            referenced.extend(l.citations.iter().map(|c| c.as_str()));
        }
        for id in referenced {
            assert!(
                unique.contains(id),
                "{name}: {id} is referenced but not in provenance"
            );
        }
        for c in &out.provenance {
            assert!(!c.url.is_empty() && !c.title.is_empty(), "{name}: {}", c.id);
        }
    }
}

#[test]
fn spending_never_outruns_the_money() {
    for (name, input, out) in outputs() {
        let mut money = 0.0_f64;
        let mut spent = 0.0_f64;
        for m in &out.plan.months {
            money += f64::from(m.budget_usd);
            for it in &m.items {
                if it.kind == PlanItemKind::Purchase && !it.done {
                    spent += f64::from(it.est_cost_usd);
                }
            }
            assert!(
                spent <= money + 0.05,
                "{name}: by month {} spent ${spent:.2} of ${money:.2}",
                m.index
            );
        }
        let f = &input.finances;
        let months = out.plan.months.len().saturating_sub(1) as f64;
        let most = f64::from(f.one_off_budget_usd) + f64::from(f.monthly_budget_usd) * months;
        assert!(spent <= most + 0.05, "{name}: ${spent:.2} over ${most:.2}");
    }
}

#[test]
fn the_packet_has_every_section_in_order_and_nothing_left_over() {
    for (name, _, out) in outputs() {
        let p = &out.packet_markdown;
        assert!(p.starts_with("# Your preparedness packet\n"), "{name}");
        let mut at = 0;
        for h in SECTION_HEADINGS {
            let found = p[at..]
                .find(&format!("\n{h}\n"))
                .unwrap_or_else(|| panic!("{name}: missing or out of order: {h}"));
            at += found + 1;
        }
        for ph in PLACEHOLDERS {
            assert!(
                !p.contains(ph),
                "{name}: placeholder {ph} left in the packet"
            );
        }
        assert!(
            !p.contains('\u{1}') && !p.contains('\u{2}'),
            "{name}: citation marker left"
        );
        assert!(
            !p.contains(rr_content::policy::CONDITION_OPEN)
                && !p.contains(rr_content::policy::CONDITION_CLOSE),
            "{name}: conditional marker left"
        );
        assert!(!p.contains("[^"), "{name}: footnote reference left");
        // Markdown only: nothing that could be an HTML tag.
        let bytes = p.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            if *b == b'<' {
                let next = bytes.get(i + 1).copied().unwrap_or(b' ');
                assert!(
                    !(next.is_ascii_alphabetic() || next == b'/' || next == b'!'),
                    "{name}: something like an HTML tag at byte {i}"
                );
            }
        }
        let n = out.provenance.len();
        let numbers = cited_numbers(p);
        assert!(!numbers.is_empty(), "{name}: the packet cites nothing");
        for k in numbers {
            assert!(
                k >= 1 && k <= n,
                "{name}: [{k}] points past the {n} sources"
            );
        }
        // The sources section lists every citation the brackets point to, numbered in order,
        // and counts the rest (they stay in the provenance list).
        let sources = &p[p.find("\n## Sources\n").unwrap()..];
        let body = &p[..p.find("\n## Sources\n").unwrap()];
        let cited: BTreeSet<usize> = cited_numbers(body).into_iter().collect();
        let listed = cited.len();
        assert_eq!(
            cited.iter().copied().collect::<Vec<_>>(),
            (1..=listed).collect::<Vec<_>>(),
            "{name}: the packet's own sources are numbered first, without gaps"
        );
        for k in 1..=listed {
            assert!(
                sources.contains(&format!("**{k}** ")),
                "{name}: source {k} not listed"
            );
        }
        assert!(
            !sources.contains(&format!("**{}** ", listed + 1)),
            "{name}: a source the packet does not cite is listed"
        );
        if n > listed {
            assert!(
                sources.contains(&format!("{} more source", n - listed)),
                "{name}: the uncited sources are not counted"
            );
        }
        assert!(
            !sources.contains("(retrieved"),
            "{name}: sources are compact"
        );
        assert!(
            sources.contains("not endorsed by FEMA"),
            "{name}: NRI statement missing"
        );
    }
}

/// Words as a reader counts them: tokens with a letter or digit, citation brackets left out.
fn words(markdown: &str) -> usize {
    let mut text = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(start) = rest.find('[') {
        text.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find(']') {
            Some(end)
                if !after[..end].is_empty()
                    && after[..end].split(", ").all(|n| n.parse::<usize>().is_ok()) =>
            {
                text.push(' ');
                rest = &after[end + 1..];
            }
            _ => {
                text.push('[');
                rest = after;
            }
        }
    }
    text.push_str(rest);
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

/// The packet is the short version (POLISH_ROUND, packet v2, docs/PACKET.md): hazard cards only
/// for the hazards that most need one, one what-to-do part per bucket, checklists up to the
/// recommended step with each line's month, four topics at most, this month and next month in
/// detail, compact sources. This cap catches the packet growing back (it was 26,000 words in
/// v0.1.0 and 14,100 before packet v2).
#[test]
fn the_packet_stays_short() {
    const MAX_WORDS: usize = 12_500;
    for (name, _, out) in outputs() {
        let p = &out.packet_markdown;
        let n = words(p);
        assert!(n <= MAX_WORDS, "{name}: {n} words");
        // Bucket parts carry what to do, not the why or the requirement lines.
        for gone in [
            "**Your numbers.**",
            "**When help comes.**",
            "**What counts toward it:**",
            "**Also worth knowing:**",
        ] {
            assert!(!p.contains(gone), "{name}: {gone}");
        }
        // Four topics at most, and only these.
        let topics: Vec<&str> = p
            .lines()
            .filter_map(|l| l.strip_prefix("#### "))
            .filter(|t| !t.starts_with(|c: char| c.is_ascii_digit()))
            .collect();
        for t in &topics {
            assert!(
                [
                    "Drills and if-then plans",
                    "Talking with children about emergencies",
                    "Neighbours and mutual aid",
                    "Stress, mental health and the 988 line",
                ]
                .contains(t),
                "{name}: topic {t}"
            );
        }
        // This month and next month in detail; later months are the checklists, each line with
        // its month, so the plan lists each purchase once (no table of later months).
        for m in rr_plan::packet::DETAIL_MONTHS..=120 {
            assert!(
                !p.contains(&format!("**Month {m} (from ")),
                "{name}: month {m} in detail"
            );
        }
        assert!(!p.contains("### Later months"), "{name}: the old table");
        let later = out.plan.months.iter().any(|m| {
            m.index >= rr_plan::packet::DETAIL_MONTHS
                && m.items
                    .iter()
                    .any(|i| !i.done && i.kind != rr_types::PlanItemKind::Reserve)
        });
        assert_eq!(
            p.contains("### After that"),
            later,
            "{name}: the pointer to the checklists"
        );
        // Checklists stop at the recommended step.
        let beyond: Vec<&str> = rr_types::TierId::ALL
            .iter()
            .filter(|t| **t > out.tier_recommended)
            .map(|t| t.name())
            .collect();
        let start = p.find("\n## Checklists\n").unwrap();
        let end = p[start + 1..]
            .find("\n## ")
            .map_or(p.len(), |e| start + 1 + e);
        let lists = &p[start..end];
        for t in beyond {
            assert!(
                !lists.contains(&format!("\n### {t}\n")),
                "{name}: {t} checklist"
            );
        }
        if later {
            assert!(
                lists.contains(" (month ") || lists.contains("\n- [ ] Month "),
                "{name}: checklist lines carry their month"
            );
        }
    }
}

/// Hazard cards (review S3, packet v2): the likeliest hazards, house fire always, every hazard
/// that can kill (Severe or worse, fast, or meeting this home) from a 1 in 100 chance in ten years,
/// the hazard of a named scenario the plan includes, and the likeliest wind hazard; none for a
/// hazard whose only consequence is lost income; at most eight unless the protected ones need
/// more; a Minor card never displaces a Serious or Severe one; most likely first.
#[test]
fn hazard_cards_follow_the_life_safety_rule() {
    use rr_plan::packet::{
        CARD_MIN_P10, FREQUENT_CARDS, HAZARD_CARDS, LIFE_SAFETY_MIN_P10, MINOR, SEVERE,
        WIND_HAZARDS,
    };
    for (name, input, out) in outputs() {
        let a = common::run(input);
        let p = &out.packet_markdown;
        let risks =
            &p[p.find("\n## Your risks\n").unwrap()..p.find("\n## Your targets\n").unwrap()];
        let cards: Vec<&rr_types::HazardProfile> = risks
            .lines()
            .filter_map(|l| l.strip_prefix("#### "))
            .filter_map(|l| l.split_once(". ").map(|(_, t)| t))
            .map(|c| {
                a.hazards
                    .profiles
                    .iter()
                    .find(|p| p.name == c)
                    .unwrap_or_else(|| panic!("{name}: card {c}"))
            })
            .collect();
        let p10 = |p: &rr_types::HazardProfile| -rr_types::math::exp_m1(-10.0 * p.rate_per_year);
        let income_only = |p: &rr_types::HazardProfile| {
            p.buckets.iter().all(|b| *b == rr_types::BucketId::Income)
        };
        let scenario: Vec<rr_types::HazardId> = a
            .hazards
            .scenarios
            .iter()
            .filter(|s| s.on)
            .map(|s| s.hazard)
            .collect();
        // Kept above the cap: house fire, scenario hazards, Severe hazards, and a medical
        // emergency (its advice would otherwise print in full under Your targets). Compound
        // drivers are kept too; `storm_and_heat_cards_stay_where_they_matter` checks Sugar Land's.
        let protected = |p: &rr_types::HazardProfile| {
            !income_only(p)
                && (p.id == rr_types::HazardId::HouseFire
                    || p.id == rr_types::HazardId::MedicalEmergency
                    || scenario.contains(&p.id)
                    || (p10(p) >= LIFE_SAFETY_MIN_P10 && p.severity >= SEVERE))
        };
        let ranked: Vec<&rr_types::HazardProfile> = {
            let mut v: Vec<&rr_types::HazardProfile> = a
                .hazards
                .profiles
                .iter()
                .filter(|p| p.display == rr_types::HazardDisplay::Ranked)
                .collect();
            v.sort_by(|x, y| y.rate_per_year.total_cmp(&x.rate_per_year));
            v
        };
        let frequent: Vec<rr_types::HazardId> = ranked
            .iter()
            .filter(|p| p10(p) >= CARD_MIN_P10)
            .take(FREQUENT_CARDS)
            .map(|p| p.id)
            .collect();
        let wind =
            |p: &rr_types::HazardProfile| WIND_HAZARDS.contains(&p.id) && p10(p) >= CARD_MIN_P10;
        if cards.len() > HAZARD_CARDS {
            assert!(
                cards.iter().all(|c| protected(c)),
                "{name}: {} cards, not all of them protected",
                cards.len()
            );
        }
        for c in &cards {
            assert!(!income_only(c), "{name}: {} is income only", c.name);
            assert!(
                protected(c)
                    || frequent.contains(&c.id)
                    || wind(c)
                    || p10(c) >= LIFE_SAFETY_MIN_P10,
                "{name}: {} has no reason for a card",
                c.name
            );
        }
        for p in ranked.iter().filter(|p| protected(p)) {
            assert!(
                cards.iter().any(|c| c.id == p.id),
                "{name}: {} ({}) has no card",
                p.name,
                p.severity
            );
        }
        // A Minor card never displaces a Serious or Severe one: while any Minor card is shown
        // (other than a named scenario's hazard, which always keeps its card), every likeliest
        // hazard rated Serious or worse (that is not income only) has a card.
        if cards
            .iter()
            .any(|c| c.severity < MINOR && !scenario.contains(&c.id))
        {
            for p in ranked
                .iter()
                .filter(|p| frequent.contains(&p.id) && p.severity >= 0.4 && !income_only(p))
            {
                assert!(
                    cards.iter().any(|c| c.id == p.id),
                    "{name}: {} ({}) displaced by a Minor card",
                    p.name,
                    p.severity
                );
            }
        }
        // Most likely first.
        for w in cards.windows(2) {
            assert!(w[0].rate_per_year >= w[1].rate_per_year, "{name}: order");
        }
    }
}

/// The cases the packet v2 card rule was written for (brief: plan2; merge-data-model's note).
/// The frequent-but-minor rows (phone and internet outages, arrests) never take a card, so they
/// cannot push the storm cards out; the likeliest wind hazard keeps its card against Minor and
/// Moderate rows (Chicago, Coos Bay) but, as a Minor card, yields to a Serious or Severe one of
/// the household's own: Philadelphia's wildfire smoke (Serious for its older adult) and Miami's
/// cold wave (Serious for its retiree). Sugar Land keeps its heat wave, which drives the
/// blackout-in-a-heat-wave class behind its targets, though the medical-emergency card is a hair
/// likelier; Galveston keeps its heat wave (Serious) over a Minor wind card.
#[test]
fn storm_and_heat_cards_stay_where_they_matter() {
    let cards = |name: &str| -> Vec<String> {
        let (_, _, out) = outputs().iter().find(|(n, _, _)| *n == name).unwrap();
        let p = &out.packet_markdown;
        let risks =
            &p[p.find("\n## Your risks\n").unwrap()..p.find("\n## Your targets\n").unwrap()];
        risks
            .lines()
            .filter_map(|l| l.strip_prefix("#### "))
            .filter_map(|l| l.split_once(". ").map(|(_, t)| t.to_owned()))
            .collect()
    };
    for (name, _, _) in outputs() {
        let c = cards(name);
        for minor in [
            "Phone or internet outage",
            "A household member is arrested or detained",
        ] {
            assert!(!c.iter().any(|t| t == minor), "{name}: {c:?}");
        }
    }
    for name in ["chicago-student-zero-budget-1", "coos-bay-well-owner-2"] {
        let c = cards(name);
        assert!(c.iter().any(|t| t == "Strong wind"), "{name}: {c:?}");
    }
    for (name, serious) in [
        ("philadelphia-renters-4", "Wildfire smoke"),
        ("miami-condo-retiree-1", "Cold wave"),
    ] {
        let c = cards(name);
        assert!(c.iter().any(|t| t == serious), "{name}: {c:?}");
        assert!(!c.iter().any(|t| t == "Strong wind"), "{name}: {c:?}");
        assert!(c.len() <= rr_plan::packet::HAZARD_CARDS, "{name}: {c:?}");
    }
    let c = cards("sugar-land-ev-household-3");
    assert!(c.iter().any(|t| t == "Heat wave"), "Sugar Land: {c:?}");
    assert!(
        c.iter().any(|t| t == "Medical emergency"),
        "Sugar Land: {c:?}"
    );
    let c = cards("galveston-highrise-1");
    assert!(c.iter().any(|t| t == "Heat wave"), "Galveston: {c:?}");
}

#[test]
fn targets_come_from_consequence_and_tiers_from_supply() {
    for (name, input, out) in outputs() {
        let a = common::run(input);
        for (b, c) in out.buckets.iter().zip(&a.consequence.buckets) {
            assert_eq!(b.id, c.id);
            match (b.target, c.target) {
                (
                    Target::Days { value, low, high },
                    Target::Days {
                        value: v,
                        low: l,
                        high: h,
                    },
                ) => {
                    assert_eq!((value, low, high), (v, l, h), "{name} {}", b.id)
                }
                (Target::Months { value, .. }, Target::Months { value: v, .. }) => {
                    assert_eq!(value, v, "{name} {}", b.id)
                }
                _ => {}
            }
            assert_eq!(b.tier_enough, rr_supply::tier_enough(b), "{name} {}", b.id);
            assert_eq!(b.covered.kind(), b.target.kind(), "{name} {}", b.id);
            if let (Target::Days { value, .. }, Target::Days { value: c, .. }) =
                (b.target, b.covered)
            {
                assert!(
                    c <= value + 1e-6,
                    "{name} {}: covered {c} > target {value}",
                    b.id
                );
            }
        }
        assert_eq!(
            out.tier_recommended,
            rr_supply::tier_recommended(&out.buckets),
            "{name}"
        );
        let ids: Vec<&str> = out.warnings.iter().map(|w| w.id.as_str()).collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(
            unique.len(),
            ids.len(),
            "{name}: duplicate warning ids {ids:?}"
        );
        let envelopes: Vec<&str> = out
            .plan
            .envelopes
            .iter()
            .map(|e| e.item_id.as_str())
            .collect();
        let unique: BTreeSet<&str> = envelopes.iter().copied().collect();
        assert_eq!(
            unique.len(),
            envelopes.len(),
            "{name}: one envelope per item"
        );
    }
}

/// `docs/RISK_MODEL.md` § "End to end with rr-hazards' rates": Philadelphia at the default dial
/// gets power 3 d (the research's 2.8; the pack's pooled restoration curves, passed to the model
/// since v0.2.0, bring it back from 5), boil-water 5 d, no tap water 5 d, food 14 d, heat or cold
/// 3 d, medicine 3 weeks (a daily prescription, v0.2.0), phone 3 d, income 4 months; one month is
/// the step that is enough.
#[test]
fn philadelphia_targets_match_the_risk_model_report() {
    let (_, _, out) = outputs()
        .iter()
        .find(|(n, _, _)| *n == "philadelphia-renters-4")
        .unwrap();
    let days = |b: BucketId| match out.buckets.iter().find(|x| x.id == b).unwrap().target {
        Target::Days { value, .. } => value,
        Target::Months { value, .. } => value,
        _ => f32::NAN,
    };
    assert_eq!(days(BucketId::Power), 3.0);
    assert_eq!(days(BucketId::WaterBoil), 5.0);
    assert_eq!(days(BucketId::WaterOut), 5.0);
    assert_eq!(days(BucketId::Supplies), 14.0);
    assert_eq!(days(BucketId::Thermal), 3.0);
    assert_eq!(days(BucketId::Medication), 21.0);
    assert_eq!(days(BucketId::Comms), 3.0);
    assert_eq!(days(BucketId::Income), 4.0);
    assert_eq!(out.tier_recommended, rr_types::TierId::M1);
    // Water: 21.6 gallons for four people and a dog over 5 days (4.32 a day, as the 12.9 over 3
    // days of v0.1.1; docs/RISK_MODEL.md, Supply sizing), met by the free reused bottles and
    // bottled water.
    let water = out
        .requirements
        .iter()
        .find(|l| l.id == "water_out.water_gallons")
        .unwrap();
    assert!((water.quantity - 21.6).abs() < 0.05, "{}", water.quantity);
}

#[test]
fn duration_buckets_the_plan_covers_reach_their_targets() {
    for (name, input, out) in outputs() {
        if out.plan.done_month.is_none() {
            continue;
        }
        let _ = input;
        for b in out
            .buckets
            .iter()
            .filter(|b| b.id.kind() == BucketKind::Duration)
        {
            if let (Target::Days { value, .. }, Target::Days { value: c, .. }) =
                (b.target, b.covered)
            {
                assert!(
                    (c - value).abs() < 0.11,
                    "{name} {}: done but covered {c} of {value}",
                    b.id
                );
            }
        }
    }
}

#[test]
fn family_blocks_keep_only_the_hazards_that_apply_here() {
    let packet = |name: &str| {
        outputs()
            .iter()
            .find(|(n, _, _)| *n == name)
            .map(|(_, _, o)| o.packet_markdown.clone())
            .unwrap()
    };
    // Philadelphia: avalanches and tsunamis do not reach a rowhouse, so the cold-wave card says
    // nothing about beacons and no earthquake advice mentions the shore.
    let phl = packet("philadelphia-renters-4");
    assert!(
        !phl.contains("avalanche"),
        "avalanche advice in Philadelphia"
    );
    assert!(!phl.contains("walk to high ground"));
    // Coos Bay, in a tsunami zone: the earthquake card keeps the tsunami steps.
    let coos = packet("coos-bay-well-owner-2");
    assert!(
        coos.contains("walk to high ground or inland"),
        "tsunami steps missing"
    );
}
