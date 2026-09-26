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

/// awaiting: content — blocks for the contract v2 bucket and hazards (`bucket_clean_air.md` and the
/// hazard and family blocks, DESIGN-DELTA §3).
const AWAITING_BUCKETS: &[BucketId] = &[BucketId::CleanAir];
const AWAITING_HAZARDS: &[HazardId] = &[
    HazardId::WildfireSmoke,
    HazardId::DustStorm,
    HazardId::Sinkhole,
    HazardId::GeomagneticStorm,
    HazardId::Vei7Eruption,
    HazardId::DamFailure,
    HazardId::NetworkOutage,
    HazardId::DrugShortage,
    HazardId::BenefitInterruption,
    HazardId::AttackDisruption,
    HazardId::MultiMonthBlackout,
    HazardId::WarInfrastructure,
    HazardId::CbrnAttack,
    HazardId::SeverePandemic,
    HazardId::FinancialCrisis,
    HazardId::MassViolence,
    HazardId::WaterDamage,
    HazardId::Eviction,
    HazardId::ArrestOrDetention,
];

#[test]
fn every_bucket_has_its_own_block() {
    for b in BucketId::ALL {
        if AWAITING_BUCKETS.contains(b) {
            continue;
        }
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
        if AWAITING_HAZARDS.contains(h) {
            continue;
        }
        let target = format!("hazard:{}", h.as_str());
        assert!(
            content().guidance_for(&target).next().is_some(),
            "no guidance block applies to {target}"
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
        "disability_access",
        "pets",
        "evs",
        "mental_health",
        "the_dial",
        "climate_horizon",
        "antibiotics",
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
