//! Coverage and content checks on the guidance blocks: every bucket, hazard and tier has a block,
//! the topics the brief names exist, footnotes copy the registry, and the sensitive topics say
//! what they must.

use rr_content::policy::{self, find_phrases, tokens};
use rr_types::{BucketId, HazardId, TierId};

fn content() -> &'static rr_content::Content {
    rr_content::content()
}

fn block(id: &str) -> &'static rr_content::Guidance {
    content()
        .guidance(id)
        .unwrap_or_else(|| panic!("no guidance block `{id}`"))
}

#[test]
fn every_bucket_has_its_own_block() {
    for b in BucketId::ALL {
        let id = format!("bucket_{}", b.as_str());
        let g = block(&id);
        let target = format!("bucket:{}", b.as_str());
        assert!(
            g.meta.applies_to.contains(&target),
            "{id} does not apply to {target}"
        );
    }
}

#[test]
fn every_hazard_is_explained_by_a_block() {
    for h in HazardId::ALL {
        let target = format!("hazard:{}", h.as_str());
        // A rare hazard may be explained by its family block instead (`family:<id>`).
        let by_family = h.is_rare() && content().family_block(h.as_str()).is_some();
        assert!(
            by_family || content().guidance_for(&target).next().is_some(),
            "no guidance block applies to {target}"
        );
    }
}

#[test]
fn every_rare_family_has_a_family_block() {
    // A rare family is named by its lead hazard id; its block applies to `family:<id>` and says
    // what the family changes in the plan.
    for id in rr_types::rare_family_ids() {
        let g = content()
            .family_block(id)
            .unwrap_or_else(|| panic!("no family block applies to family:{id}"));
        assert_eq!(g.kind(), rr_content::GuidanceKind::Family);
        assert!(
            g.prose().contains("**What it changes in your plan.**"),
            "{}: say what the family changes in the plan",
            g.meta.id
        );
    }
}

#[test]
fn every_tier_has_its_own_block() {
    for t in TierId::ALL {
        let id = format!("tier_{}", t.as_str());
        let target = format!("tier:{}", t.as_str());
        assert!(
            block(&id).meta.applies_to.contains(&target),
            "{id} does not apply to {target}"
        );
    }
}

#[test]
fn the_topics_named_in_the_brief_exist() {
    for slug in [
        "consequences_not_causes",
        "how_numbers_are_made",
        "disaster_myths",
        "talking_with_children",
        "neighbours",
        "drills",
        "rotation",
        "renters",
        "access_needs",
        "pets",
        "evs",
        "mental_health",
        "the_dial",
        "climate_horizon",
        "antibiotics",
        // v0.2.0 (content brief, round 2 phase 2).
        "long_horizon",
        "validation",
        "strategic_sites",
        "before_you_need_them",
    ] {
        let g = block(&format!("topic_{slug}"));
        let target = format!("topic:{slug}");
        assert!(g.meta.applies_to.contains(&target));
    }
}

#[test]
fn footnotes_copy_the_registry() {
    for g in &content().guidance {
        for (id, text) in g.footnote_definitions() {
            let c = content()
                .citation(&id)
                .unwrap_or_else(|| panic!("{}: unknown citation {id}", g.meta.id));
            let title = c.title.trim_end_matches('.');
            assert!(
                text.contains(title),
                "{}: footnote {id} does not name the source's title `{title}`",
                g.meta.id
            );
        }
    }
}

#[test]
fn the_frequency_placeholder_renders_cleanly() {
    for g in &content().guidance {
        let with = g.render(Some("About 5 of 100 households like yours."));
        let without = g.render(None);
        for r in [&with, &without] {
            assert!(!r.contains(policy::FREQUENCY_PLACEHOLDER), "{}", g.meta.id);
            assert!(!r.starts_with(' '), "{} starts with a space", g.meta.id);
        }
    }
}

#[test]
fn the_antibiotics_topic_routes_to_a_clinician_and_names_no_drugs() {
    let g = block("topic_antibiotics");
    let prose = g.prose().to_lowercase();
    let toks = tokens(&prose);
    let named = find_phrases(&toks, policy::DRUG_NAMES);
    assert!(
        named.is_empty(),
        "drug names in topic_antibiotics: {named:?}"
    );
    for must in [
        "prescription",
        "clinician",
        "$300",
        "expired",
        "never use aquarium",
    ] {
        assert!(
            prose.contains(must),
            "topic_antibiotics should mention `{must}`"
        );
    }
    let cited: Vec<&str> = g.meta.citations.iter().map(|c| c.as_str()).collect();
    for c in [
        "cdc_antibiotic_use",
        "usc_21_353",
        "fda_fish_antibiotics_warning_2023",
        "mo_med_2026_antibiotic_kits",
    ] {
        assert!(cited.contains(&c), "topic_antibiotics should cite {c}");
    }
}

#[test]
fn the_mental_health_topic_gives_both_lines() {
    let prose = block("topic_mental_health").prose();
    assert!(prose.contains("988"));
    assert!(prose.contains("1-800-985-5990"));
}

#[test]
fn nuclear_guidance_keeps_potassium_iodide_to_official_instruction() {
    let prose = block("hazard_nuclear").prose().to_lowercase();
    assert!(prose.contains("get inside, stay inside, stay tuned"));
    assert!(prose.contains("potassium iodide unless public health or emergency officials"));
}

#[test]
fn the_glossary_covers_the_terms_the_brief_names() {
    let terms: Vec<String> = content()
        .glossary
        .iter()
        .map(|t| t.term.to_lowercase())
        .collect();
    for want in [
        "return period",
        "exceedance",
        "special flood hazard area",
        "public safety power shutoff",
        "boil water advisory",
        "cert",
        "effak",
        "kcal",
        "wh (watt-hour)",
        "saidi",
    ] {
        assert!(
            terms.iter().any(|t| t.contains(want)),
            "the glossary has no entry for `{want}`"
        );
    }
    for t in &content().glossary {
        assert!(
            !t.plain.eq_ignore_ascii_case(&t.term),
            "`{}`: the plain phrase comes first and differs from the term",
            t.term
        );
    }
}

/// The blocks the v0.2.0 content brief added, which it caps at 250 words each (the validator's
/// limit for every block is 300). Run with `--nocapture` for the word-count table.
const NEW_IN_V0_2: &[&str] = &[
    "after_first_30_days",
    "plan_shelter",
    "plan_forecast_48h",
    "plan_communication",
    "topic_access_needs",
    "bucket_clean_air",
    "hazard_water_damage",
    "hazard_wildfire_smoke",
    "hazard_dam_levee",
    "hazard_network_outage",
    "hazard_drug_shortage",
    "hazard_benefit_interruption",
    "hazard_eviction",
    "hazard_attack_disruption",
    "hazard_dust_storm",
    "hazard_sinkhole",
    "hazard_arrest_or_detention",
    "family_nuclear",
    "family_solar_storm",
    "family_long_blackout",
    "family_war_infrastructure",
    "family_cbrn",
    "family_severe_pandemic",
    "family_large_eruption",
    "family_financial_crisis",
    "family_mass_violence",
    "topic_long_horizon",
    "topic_validation",
    "topic_strategic_sites",
    "topic_before_you_need_them",
];

#[test]
fn new_blocks_stay_within_the_briefs_word_budget() {
    println!("| Block | Words | Grade |\n| --- | ---: | ---: |");
    let mut total = 0;
    let mut over = Vec::new();
    for id in NEW_IN_V0_2 {
        let plain = rr_content::readability::plain_text(block(id).prose());
        let words = rr_content::readability::words(&plain).len();
        let grade = rr_content::readability::flesch_kincaid_grade(&plain).unwrap_or(0.0);
        println!("| `{id}` | {words} | {grade:.1} |");
        total += words;
        if words > 250 {
            over.push(format!("{id} ({words})"));
        }
    }
    println!("| total ({} blocks) | {total} | |", NEW_IN_V0_2.len());
    assert!(over.is_empty(), "over the brief's 250 words: {over:?}");
}
