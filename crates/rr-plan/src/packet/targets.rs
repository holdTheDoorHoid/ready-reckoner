//! Section 3: your targets. The dial setting in words, the duration targets with their ranges and
//! relief rating, then one short part per bucket: what to do, its guidance block's "What helps"
//! and "What to avoid" paragraphs (the why, the numbers behind the target and the requirement
//! lines live in the app's Learn and explain views, and the quantities in the checklists). A
//! bucket whose advice a risk card already gave points to that card; leaving home and getting
//! home point to the family plan's steps but keep their warnings; a damaged home and lost income
//! point to Documents and money. Then named scenarios and any event that drives the answer.

use rr_types::{BucketId, BucketKind, ReturnPeriod, Target, TierId};

use super::text::{self, md};
use super::{Ctx, cite_all};

/// A target in words: "about 5 days (up to 10 days)", "about 4 months (2–7)".
pub(crate) fn target_phrase(t: &Target) -> String {
    match *t {
        Target::Days { value, low, high } => {
            if value <= 0.0 {
                "not needed at this setting".to_owned()
            } else {
                text::target_days(f64::from(value), f64::from(low), f64::from(high))
            }
        }
        Target::Months { value, low, high } => {
            if value <= 0.0 {
                "no savings goal".to_owned()
            } else {
                text::target_months(f64::from(value), f64::from(low), f64::from(high))
            }
        }
        Target::Evacuate { p_need_10yr, .. } | Target::Readiness { p_need_10yr, .. } => {
            format!(
                "{} households like yours in 10 years",
                text::households(p_need_10yr)
            )
        }
    }
}

/// What the dial means (model review M-04): each target holds for its own need, so the chance
/// that at least one need runs past its target is higher. At the default 1-in-100 setting these
/// are the canonical words the web app also shows; at other settings the per-need share is that
/// setting's and the joint one is only said to be higher.
pub fn dial_sentence(rp: ReturnPeriod) -> String {
    if rp == ReturnPeriod::OneIn100 {
        return "For any one need, something worse than its target comes in about 1 of every 10 \
                ten-year stretches. Across all your needs together, the chance that at least one \
                runs out is higher, roughly 1 in 3. That is why the plan also gives you ways to \
                cope when a target runs out."
            .to_owned();
    }
    let rate = rr_consequence::dial_rate(rp);
    format!(
        "For any one need, something worse than its target comes in about {} of every 100 \
         ten-year stretches. Across all your needs together, the chance that at least one runs \
         out is higher. That is why the plan also gives you ways to cope when a target runs out.",
        text::per_100(-rr_types::math::exp_m1(-10.0 * rate))
    )
}

fn relief_cell(days: Option<f32>) -> String {
    match days {
        Some(d) => format!("about {}", text::day_phrase(round_relief(f64::from(d)))),
        None => "not known".to_owned(),
    }
}

/// Relief days on the target ladder, so "3.2 days" reads as "3 days".
fn round_relief(d: f64) -> f64 {
    if d < 0.75 {
        return 0.5;
    }
    if d < 14.0 {
        return d.round().max(1.0);
    }
    f64::from(rr_consequence::round_up_to_ladder(d))
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    out.push("## Your targets".to_owned());
    out.push(String::new());
    out.push(format!(
        "How long to be ready for each kind of disruption at the 1-in-{} setting. {}{}",
        a.input.dials.return_period.years(),
        dial_sentence(a.input.dials.return_period),
        cite_all([
            &rr_types::CitationId::from("rr_research_risk_model"),
            &rr_types::CitationId::from("rr_risk_model_priors"),
        ])
    ));
    out.push(String::new());
    out.push(
        "| If this happens | Be ready for | Outside help likely arrives | Mostly back to normal | \
         Enough at |"
            .to_owned(),
    );
    out.push("| --- | --- | --- | --- | --- |".to_owned());
    for b in &a.buckets {
        if b.id.kind() != BucketKind::Duration {
            continue;
        }
        out.push(format!(
            "| {} | {} | {} | {} | {} |",
            md(&b.name),
            target_phrase(&b.target),
            relief_cell(b.relief.as_ref().map(|r| r.help_arrives_days)),
            relief_cell(b.relief.as_ref().map(|r| r.mostly_restored_days)),
            tier_cell(b.tier_enough)
        ));
    }
    out.push(String::new());
    out.push(
        "The range in brackets shows how uncertain each target is. \"Not known\": there are no \
         restoration records for the event behind that target."
            .to_owned(),
    );
    out.push(String::new());
    // Blocks a risk card already showed, with the card's name; and cards whose hazard has one
    // consequence, whose advice is that bucket's (a medical emergency).
    let cards = super::risks::cards(cx);
    let shown: Vec<(String, String)> = cards
        .iter()
        .filter_map(|(p, g)| g.map(|g| (g.meta.id.clone(), p.name.clone())))
        .collect();
    let mut single: Vec<(BucketId, String)> = cards
        .iter()
        .filter(|(p, g)| g.is_some() && p.buckets.len() == 1)
        .map(|(p, _)| (p.buckets[0], p.name.clone()))
        .collect();
    // Dangerous heat or cold at home: when the heat-wave and cold-wave cards are both shown, their
    // advice (cooling places, a warm room, fans above 90°F, no oven or grill for heat) is this
    // bucket's, so it points to them. Not for a home heated with wood, whose stove and chimney
    // advice is this bucket's own.
    let card_name = |h: rr_types::HazardId| {
        cards
            .iter()
            .find(|(p, g)| p.id == h && g.is_some())
            .map(|(p, _)| p.name.clone())
    };
    let wood = a.input.housing.heating == rr_types::Heating::Wood;
    if let (false, Some(heat), Some(cold)) = (
        wood,
        card_name(rr_types::HazardId::HeatWave),
        card_name(rr_types::HazardId::ColdWave),
    ) {
        single.push((BucketId::Thermal, format!("{heat}\" and \"{cold}")));
    }

    // What to do about these is in the documents-and-money section.
    let elsewhere = [BucketId::HomeLoss, BucketId::Income];
    // What helps with these is the family plan's steps; their warnings stay here.
    let family = [BucketId::Evacuate, BucketId::GetHome];
    for b in a.buckets.iter().filter(|b| !elsewhere.contains(&b.id)) {
        let target = &b.target;
        if !is_active(target) {
            continue;
        }
        let heading = match b.id.kind() {
            BucketKind::Duration => format!("### {}: {}", md(&b.name), target_phrase(target)),
            _ if b.id == BucketId::Income => {
                format!("### {}: {}", md(&b.name), target_phrase(target))
            }
            _ => format!("### {}", md(&b.name)),
        };
        out.push(heading);
        out.push(String::new());
        let target_words = target_phrase(target);
        if let Some(g) = cx
            .blocks_for(&format!("bucket:{}", b.id))
            .into_iter()
            .next()
        {
            let card = shown
                .iter()
                .find(|(id, _)| *id == g.meta.id)
                .map(|(_, name)| name)
                .or_else(|| single.iter().find(|(x, _)| *x == b.id).map(|(_, n)| n));
            let advice = super::advice_paragraphs(&cx.guidance(g, None, Some(&target_words)));
            let avoid = advice.iter().filter(|p| !p.starts_with("**What helps.**"));
            match card {
                Some(card) => out.push(format!(
                    "**What helps.** See \"{}\" under Your risks.",
                    md(card)
                )),
                None if family.contains(&b.id) => {
                    out.push("**What helps.** See the steps under Family plan.".to_owned());
                    for para in avoid {
                        out.push(String::new());
                        out.push(para.clone());
                    }
                }
                None if advice.is_empty() => {
                    for para in super::helps_paragraph(&cx.guidance(g, None, Some(&target_words))) {
                        out.push(para);
                    }
                }
                None => {
                    // What to do: what helps, and what to avoid (the carbon monoxide, floodwater
                    // and do-not-drink warnings live here).
                    for (i, para) in advice.iter().enumerate() {
                        if i > 0 {
                            out.push(String::new());
                        }
                        out.push(para.clone());
                    }
                }
            }
            out.push(String::new());
        }
    }

    let pointed: Vec<String> = a
        .buckets
        .iter()
        .filter(|b| elsewhere.contains(&b.id) && is_active(&b.target))
        .map(|b| {
            match b.id {
                BucketId::HomeLoss => "a damaged home",
                _ => "lost income",
            }
            .to_owned()
        })
        .collect();
    if !pointed.is_empty() {
        out.push(format!(
            "What to do about {} is under Documents and money.",
            text::join_and(&pointed)
        ));
        out.push(String::new());
    }

    if !a.consequence.scenarios.is_empty() {
        out.push("### Named scenarios".to_owned());
        out.push(String::new());
        out.push(
            "A named scenario is one rare, severe event that would change your targets a lot. \
             The plan includes it or leaves it out as shown; you can change either on the risks \
             screen."
                .to_owned(),
        );
        out.push(String::new());
        for s in &a.consequence.scenarios {
            out.push(format!(
                "- **{}** ({}). {} {}{}",
                md(&s.name),
                if s.on {
                    "included in your plan"
                } else {
                    "left out of your plan"
                },
                md(&s.applies_because),
                md(&s.effect_summary),
                cite_all(&s.sources)
            ));
        }
        out.push(String::new());
    }
    let cliffs: Vec<&rr_types::Warning> = a
        .warnings
        .iter()
        .filter(|w| w.id.starts_with("cliff_"))
        .collect();
    if !cliffs.is_empty() {
        out.push("### When one event drives the answer".to_owned());
        out.push(String::new());
        for w in cliffs {
            out.push(format!("- **{}** {}", md(&w.message), md(&w.why)));
        }
        out.push(String::new());
    }
}

/// Whether a bucket has anything to plan for at this setting.
fn is_active(t: &Target) -> bool {
    match *t {
        Target::Days { value, .. } | Target::Months { value, .. } => value > 0.0,
        Target::Evacuate { p_need_10yr, .. } | Target::Readiness { p_need_10yr, .. } => {
            p_need_10yr > 0.0
        }
    }
}

fn tier_cell(t: TierId) -> String {
    match t {
        TierId::Now => "free steps".to_owned(),
        other => text::lower_first(other.name()),
    }
}
