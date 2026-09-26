//! Section 2: your risks. Cards for the hazards most likely to reach the household (at most six,
//! by household rate, among those with at least a 10 in 100 chance in ten years) plus the hazard
//! of any named scenario the plan includes, each with its guidance block; every other hazard as
//! one row of a table (those under 1 in 100 share one line); the rare-and-catastrophic box with
//! likelihood and severity as separate columns; and the plain caveats behind the numbers.

use rr_content::Guidance;
use rr_types::{BucketId, HazardDisplay, HazardProfile};

use super::text::{self, md};
use super::{Ctx, cite_all};

/// At most this many hazards get a card (named scenarios come on top).
pub const CARDS: usize = 6;

/// A hazard gets a card only with at least this chance of reaching the household in ten years.
pub const CARD_MIN_P10: f64 = 0.10;

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

/// The chance of at least one event in `years` at a yearly rate.
fn chance(rate: f64, years: u8) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -rr_types::math::exp_m1(-rate * f64::from(years.max(1)))
    }
}

/// The ranked hazards (not the rare-and-catastrophic ones), most likely first.
fn ranked<'a>(cx: &Ctx<'a>) -> Vec<&'a HazardProfile> {
    let mut v: Vec<&HazardProfile> =
        cx.a.hazards
            .profiles
            .iter()
            .filter(|p| p.display == HazardDisplay::Ranked)
            .collect();
    // Stable: equal rates keep the register's order.
    v.sort_by(|x, y| y.rate_per_year.total_cmp(&x.rate_per_year));
    v
}

/// The hazards shown as cards, in order, each with the guidance block its card shows. The
/// targets section uses this to point to a card instead of repeating the same block.
pub(crate) fn cards<'a>(cx: &Ctx<'a>) -> Vec<(&'a HazardProfile, Option<&'a Guidance>)> {
    let all = ranked(cx);
    let mut chosen: Vec<&HazardProfile> = all
        .iter()
        .copied()
        .filter(|p| chance(p.rate_per_year, 10) >= CARD_MIN_P10)
        .take(CARDS)
        .collect();
    // The hazard behind each named scenario the plan includes, if it is not already a card.
    for s in cx.a.hazards.scenarios.iter().filter(|s| s.on) {
        if chosen.iter().any(|p| p.id == s.hazard) {
            continue;
        }
        if let Some(p) = cx.a.hazards.profiles.iter().find(|p| p.id == s.hazard) {
            chosen.push(p);
        }
    }
    let mut used: Vec<&str> = Vec::new();
    chosen
        .into_iter()
        .map(|p| {
            let block = cx
                .blocks_for(&format!("hazard:{}", p.id))
                .into_iter()
                .find(|g| !used.contains(&g.meta.id.as_str()));
            if let Some(g) = block {
                used.push(g.meta.id.as_str());
            }
            (p, block)
        })
        .collect()
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let years = a.input.dials.horizon_years.max(1);
    let horizon = rr_consequence::words::horizon_phrase(years);
    out.push("## Your risks".to_owned());
    out.push(String::new());
    out.push(format!(
        "The events most likely to reach a household like yours in {}, over {horizon}, most \
         likely first. A lost job or a week-long outage is far more likely than a disaster from \
         the movies, and the plan is ordered the same way.",
        md(&super::summary::place(cx))
    ));
    out.push(String::new());

    let cards = cards(cx);
    if !cards.is_empty() {
        out.push("### The ones most likely to reach you".to_owned());
        out.push(String::new());
    }
    // Which card showed each family block, so a later card of the same family can point to it.
    let mut shown: Vec<(&str, &str)> = Vec::new();
    for (i, (p, block)) in cards.iter().enumerate() {
        out.push(format!("#### {}. {}", i + 1, md(&p.name)));
        out.push(String::new());
        // The card's own sentence, cited to the card's sources, then what to do from its block
        // (the block's opening paragraph, the why, is the app's Learn view).
        let lead = format!("{}{}", md(&p.frequency_sentence), cite_all(&p.sources));
        let advice = block
            .map(|g| super::advice_paragraphs(&cx.guidance(g, None, None)))
            .unwrap_or_default();
        match block {
            Some(g) if advice.is_empty() => out.push(cx.guidance(g, Some(&lead), None)),
            _ => out.push(lead),
        }
        out.push(String::new());
        match block {
            Some(g) => shown.push((g.meta.id.as_str(), p.name.as_str())),
            None => {
                // The family's block is on an earlier card (a cold wave and a winter storm share
                // one): point there, so the threat still comes with what to do (PRINCIPLES §4).
                let earlier = cx
                    .blocks_for(&format!("hazard:{}", p.id))
                    .into_iter()
                    .find_map(|g| shown.iter().find(|(id, _)| *id == g.meta.id.as_str()));
                if let Some((_, name)) = earlier {
                    out.push(format!(
                        "**What helps.** The steps under \"{}\" above apply here too.",
                        md(name)
                    ));
                    out.push(String::new());
                }
            }
        }
        for para in advice {
            out.push(para);
            out.push(String::new());
        }
        out.push(format!(
            "**What it can do:** {}. **How bad:** {}. **How sure:** {}.",
            md(&effects(p)),
            text::severity(p.severity),
            text::confidence(p.confidence)
        ));
        out.push(String::new());
    }

    // Every other ranked hazard: a row each, and the ones under 1 in 100 together in one line.
    let rest: Vec<&HazardProfile> = ranked(cx)
        .into_iter()
        .filter(|p| !cards.iter().any(|(c, _)| c.id == p.id))
        .collect();
    let (rows, faint): (Vec<&HazardProfile>, Vec<&HazardProfile>) = rest
        .into_iter()
        .partition(|p| text::per_100(chance(p.rate_per_year, years)) != "fewer than 1");
    if !rows.is_empty() || !faint.is_empty() {
        out.push("### Other risks we checked".to_owned());
        out.push(String::new());
    }
    if !rows.is_empty() {
        out.push(format!(
            "| What could happen | Households like yours, {horizon} | How bad | How sure |"
        ));
        out.push("| --- | --- | --- | --- |".to_owned());
        for p in &rows {
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
    if !faint.is_empty() {
        let names: Vec<String> = faint.iter().map(|p| text::lower_first(&p.name)).collect();
        out.push(format!(
            "Fewer than 1 in 100 households like yours, {horizon}: {}.{}",
            md(&text::join_and(&names)),
            cite_all(faint.iter().flat_map(|p| p.sources.iter()))
        ));
        out.push(String::new());
    }

    let rare: Vec<&HazardProfile> = a
        .hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        .collect();
    if !rare.is_empty() {
        out.push("### Rare but severe".to_owned());
        out.push(String::new());
        out.push(
            "Shown apart, because a tiny chance times a huge loss would otherwise crowd out \
             everything else. The plan never lets them take over the budget."
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
        // What to do: the first rare hazard's block, its advice paragraphs only (the table says
        // how likely), unless a card already showed it.
        if let Some(g) = rare.iter().find_map(|p| {
            cx.blocks_for(&format!("hazard:{}", p.id))
                .into_iter()
                .next()
        }) {
            if !cards
                .iter()
                .any(|(_, b)| b.is_some_and(|b| b.meta.id == g.meta.id))
            {
                for para in super::advice_paragraphs(&cx.guidance(g, None, None)) {
                    out.push(para);
                    out.push(String::new());
                }
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
