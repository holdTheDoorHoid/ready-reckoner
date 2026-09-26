//! Section 2: your risks. The five hazards most likely to reach the household as cards (with the
//! matching hazard guidance block), the rest as a table, the rare-and-catastrophic box with
//! likelihood and severity as separate columns, and the plain caveats behind the numbers.

use rr_types::{BucketId, HazardDisplay, HazardProfile};

use super::text::{self, md};
use super::{Ctx, cite, cite_all};

/// Hazards shown as cards; the rest go in a table.
const CARDS: usize = 5;

/// What a consequence bucket means for a household, mid-sentence.
pub(crate) fn bucket_effect(b: BucketId) -> &'static str {
    match b {
        BucketId::Power => "no power",
        BucketId::WaterBoil => "boil-water notices",
        BucketId::WaterOut => "no tap water",
        BucketId::Supplies => "no way to get to a store",
        BucketId::Thermal => "dangerous heat or cold at home",
        BucketId::Medication => "gaps in medicine",
        BucketId::Comms => "no phone, internet or card payments",
        BucketId::Evacuate => "having to leave home",
        BucketId::GetHome => "being stranded away from home",
        BucketId::MedicalEmergency => "a medical emergency",
        BucketId::Fire => "a fire at home",
        BucketId::Security => "a break-in or trouble nearby",
        BucketId::Income => "lost income",
        BucketId::HomeLoss => "damage to the home",
    }
}

fn effects(p: &HazardProfile) -> String {
    let parts: Vec<String> = p
        .buckets
        .iter()
        .map(|b| bucket_effect(*b).to_owned())
        .collect();
    text::join_and(&parts)
}

/// The ten-year chance of at least one event at a yearly rate, for a table cell.
fn chance(rate: f64, years: u8) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -rr_types::math::exp_m1(-rate * f64::from(years.max(1)))
    }
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let years = a.input.dials.horizon_years.max(1);
    let horizon = rr_consequence::words::horizon_phrase(years);
    out.push("## Your risks".to_owned());
    out.push(String::new());
    out.push(format!(
        "These are the events most likely to reach a household like yours in {}, over {horizon}. \
         Common events come first: a lost job, a house fire or a week-long outage is far more \
         likely than a disaster from the movies, and the plan is ordered the same way.",
        md(&super::summary::place(cx))
    ));
    out.push(String::new());

    let ranked: Vec<&HazardProfile> = a
        .hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::Ranked)
        .collect();
    let rare: Vec<&HazardProfile> = a
        .hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        .collect();

    out.push("### The ones most likely to reach you".to_owned());
    out.push(String::new());
    let mut used_blocks: Vec<&str> = Vec::new();
    for (i, p) in ranked.iter().take(CARDS).enumerate() {
        out.push(format!("#### {}. {}", i + 1, md(&p.name)));
        out.push(String::new());
        let block = cx
            .blocks_for(&format!("hazard:{}", p.id))
            .into_iter()
            .find(|g| !used_blocks.contains(&g.meta.id.as_str()));
        match block {
            Some(g) => {
                used_blocks.push(g.meta.id.as_str());
                // The block opens with the card's own sentence, cited to the card's sources.
                let lead = format!("{}{}", p.frequency_sentence, cite_all(&p.sources));
                out.push(cx.guidance(g, Some(&lead), None));
            }
            None => {
                out.push(format!(
                    "{}{}",
                    md(&p.frequency_sentence),
                    cite_all(&p.sources)
                ));
            }
        }
        out.push(String::new());
        out.push(format!("- **What it can do to you:** {}.", md(&effects(p))));
        out.push(format!(
            "- **How bad one can be:** {}. **How sure we are:** {}.",
            text::severity(p.severity),
            text::confidence(p.confidence)
        ));
        out.push(String::new());
    }

    if ranked.len() > CARDS {
        out.push("### Other risks we checked".to_owned());
        out.push(String::new());
        out.push(format!(
            "| What could happen | Households like yours, {horizon} | How bad | How sure |"
        ));
        out.push("| --- | --- | --- | --- |".to_owned());
        for p in ranked.iter().skip(CARDS) {
            out.push(format!(
                "| {} | {}{} | {} | {} |",
                md(&p.name),
                text::households(chance(p.rate_per_year, years)),
                cite_all(&p.sources),
                text::severity(p.severity),
                text::confidence(p.confidence)
            ));
        }
        out.push(String::new());
    }

    if !rare.is_empty() {
        out.push("### Rare but severe".to_owned());
        out.push(String::new());
        out.push(
            "These are shown apart, with how likely and how bad as separate columns. A tiny \
             chance times a huge loss would otherwise crowd out everything else, and the plan \
             never lets them take over the budget."
                .to_owned(),
        );
        out.push(String::new());
        out.push("| What | How likely | How bad |".to_owned());
        out.push("| --- | --- | --- |".to_owned());
        for p in &rare {
            out.push(format!(
                "| {} | {}{} | {} |",
                md(&p.name),
                md(&p.frequency_sentence),
                cite_all(&p.sources),
                text::severity(p.severity)
            ));
        }
        out.push(String::new());
        out.push(format!(
            "The first advice for a nuclear emergency is to get inside, stay inside and stay \
             tuned.{} Your three-day supplies already cover that first stretch of sheltering.",
            cite("ready_gov_nuclear")
        ));
        out.push(String::new());
        if let Some(g) = rare.iter().find_map(|p| {
            cx.blocks_for(&format!("hazard:{}", p.id))
                .into_iter()
                .next()
        }) {
            if !used_blocks.contains(&g.meta.id.as_str()) {
                out.push(cx.guidance(g, None, None));
                out.push(String::new());
            }
        }
    }

    if !a.hazards.notes.is_empty() {
        out.push("### Notes on these numbers".to_owned());
        out.push(String::new());
        for n in &a.hazards.notes {
            out.push(format!("- {}", md(n)));
        }
        out.push(String::new());
    }
}
