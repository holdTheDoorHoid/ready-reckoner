//! Section: your targets. The dial setting in words (computed from the model, model review
//! M-04), the duration targets with their ranges and relief rating (with the worst event on
//! record where the target's own event has no restoration record), "How well do these numbers
//! hold up?" from the frozen backtest, then one short part per bucket: what to do, its guidance
//! block's "What helps" and "What to avoid" paragraphs, and the consequence model's own lines
//! where they apply (the worst event on record, the water system's record, a target rounded up
//! to the ladder). A bucket whose advice a risk card already gave points to that card; leaving
//! home and getting home point to the family plan; a damaged home and lost income point to
//! Documents and money. Then named scenarios and any event that drives the answer.

use rr_consequence::ConsequenceAssessment;
use rr_types::{BucketAssessment, BucketId, BucketKind, StressTest, Target, TierId};

use super::text::{self, md};
use super::{Ctx, cite_all};

/// What the dial means, after the model's own sentence (model review Part 3.4): each target holds
/// for its own need, so the plan also gives ways to cope when one runs out.
pub const COPE_SENTENCE: &str =
    "That is why the plan also gives you ways to cope when a target runs out.";

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

/// What the dial means (model review M-04, DESIGN-DELTA §3): the consequence model's own
/// sentence, computed from its event list (`ConsequenceAssessment::dial_sentence`: about 1 in 10
/// for any one kind of disruption, and the share that runs past at least one target, from the
/// joint rate), then [`COPE_SENTENCE`].
///
/// The model's sentence counts in tens and never says fewer than 1 in 10, which is right for
/// every dial but the rarest: at 1 in 500 the chance for one need is about 2 in 100 in ten years,
/// and "about 1 in 10" would overstate it five times. When either chance is under 1 in 10, the
/// same sentence is written here from the model's two rates with that chance out of 100
/// ([`dial_share`]).
pub fn dial_sentence(c: &ConsequenceAssessment) -> String {
    let years = c.horizon_years.max(1.0);
    let one = per_100(c.dial_rate, years);
    let all = per_100(c.joint_rate(), years).max(one);
    let model = if one >= 9.5 && all >= 9.5 {
        c.dial_sentence()
    } else {
        format!(
            "At this setting, about {} households like yours will face a longer disruption of \
             any one kind in {}; about {} will face at least one kind that runs past its target.",
            dial_share(one),
            rr_consequence::words::horizon_phrase(years as u8),
            dial_share(all)
        )
    };
    format!("{model} {COPE_SENTENCE}")
}

/// Out of 100 households, how many see at least one event at `rate` a year in `years` years:
/// 100 · (1 − e^(−years · rate)), as the consequence model counts it.
fn per_100(rate: f64, years: f64) -> f64 {
    (-100.0 * rr_types::math::exp_m1(-years * rate)).clamp(0.0, 100.0)
}

/// A chance out of 100 in the dial sentence's words: in tens from 1 in 10 up (as the model's
/// sentence has it), out of 100 below ("2 in 100").
fn dial_share(n: f64) -> String {
    if n >= 9.5 {
        format!(
            "{} in 10",
            ((n / 10.0 + 0.5).floor()).clamp(1.0, 10.0) as i64
        )
    } else {
        format!("{} in 100", (n + 0.5).floor().max(1.0) as i64)
    }
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

/// The longest a stress event kept some homes out: the last of the data's marks (1, 3, 7, 14 and
/// 30 days) at which anyone was still out, as the app reads it (web-risks: "up to N").
fn stress_up_to(st: &StressTest) -> Option<f32> {
    st.share_out_at_days
        .iter()
        .filter(|(_, s)| *s > 0.0)
        .map(|(d, _)| *d)
        .fold(None, |m, d| Some(m.map_or(d, |x: f32| x.max(d))))
}

/// The relief fallback (round-2 follow-up): when the event behind a target has no restoration
/// record, the relief rating is not known; where the region's record has a worst event, the
/// "mostly back" cell says how long it kept homes out instead ("worst on record: up to 6 days"),
/// the note under the table says the target rests on an estimate, and the bucket's part below
/// names the event (its worst-event line).
pub(crate) fn relief_fallback_days(b: &BucketAssessment) -> Option<f32> {
    if b.relief.is_some() || !matches!(b.target, Target::Days { value, .. } if value > 0.0) {
        return None;
    }
    if !matches!(
        b.id,
        BucketId::Power | BucketId::WaterOut | BucketId::WaterBoil
    ) {
        return None;
    }
    stress_up_to(b.stress_test.as_ref()?)
}

/// The consequence model's sentences a bucket part prints besides the block's advice: the worst
/// event on record (power and both water buckets), the water system's record (said once, under
/// no tap water, or under boil water where there is no such part), and the note that a target
/// was rounded up to the ladder. They are the bucket's own frequency sentences, picked by how the
/// model opens them.
fn model_lines(cx: &Ctx<'_>, b: &BucketAssessment) -> Vec<String> {
    const STRESS: &str = "The worst ";
    const WATER: [&str; 2] = [
        "Water-system failures are counted",
        "We have no record of how your public water system",
    ];
    const ROUNDED: &str = "This rounds up to ";
    let water_here = match b.id {
        BucketId::WaterOut => true,
        BucketId::WaterBoil => !is_active(&cx.a.bucket(BucketId::WaterOut).target),
        _ => false,
    };
    b.frequency_sentences
        .iter()
        .filter(|s| {
            s.starts_with(STRESS)
                || s.starts_with(ROUNDED)
                || (water_here && WATER.iter().any(|w| s.starts_with(w)))
        })
        .map(|s| md(&dedupe_year(s, b.stress_test.as_ref())))
        .collect()
}

/// The worst-event sentence names the event and then its year, and some events' names end with
/// the year already ("Winter storm, March 2018 (2018)"): say the year once.
fn dedupe_year(sentence: &str, st: Option<&StressTest>) -> String {
    let Some(st) = st else {
        return sentence.to_owned();
    };
    let y = st.date.year().to_string();
    if st.event.trim_end().ends_with(&y) {
        sentence.replacen(&format!("{} ({y})", st.event), &st.event, 1)
    } else {
        sentence.to_owned()
    }
}

/// "How well do these numbers hold up?" (model review Part 3.1): the frozen backtest's tally
/// for this version, from the table bundled with the engine (`EngineInfo::validation`), and what
/// it means for the household (`topic:validation`).
fn validation(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let v = crate::validation::summary();
    if v.events_tested == 0 {
        return;
    }
    let meaning = cx
        .blocks_for("topic:validation")
        .into_iter()
        .next()
        .and_then(|g| {
            super::paragraphs(&cx.guidance(g, None, None))
                .into_iter()
                .find(|p| p.starts_with("**What it means for you.**"))
        })
        .map(|p| {
            // Its first two sentences: a floor, not a promise; plan for anything longer your area
            // has lived through. (The table is the address in brackets.)
            let body = p.trim_start_matches("**What it means for you.**").trim();
            let mut end = 0;
            for (n, (i, _)) in body.match_indices(". ").enumerate() {
                end = i + 1;
                if n == 1 {
                    break;
                }
            }
            if end == 0 {
                body.to_owned()
            } else {
                body[..end].to_owned()
            }
        })
        .unwrap_or_default();
    out.push(format!(
        "**How well do these numbers hold up?** Tested against {} real disasters, this version \
         covered {}, partly covered {}, fell short on {} and could not model {} (results: {}). \
         {meaning}",
        v.events_tested,
        v.covered,
        v.partial,
        v.short,
        v.not_modelled,
        crate::validation::DOC_URL
    ));
    out.push(String::new());
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    out.push("## Your targets".to_owned());
    out.push(String::new());
    out.push(format!(
        "How long to be ready for each kind of disruption at the 1-in-{} setting. {}{}",
        a.input.dials.return_period.years(),
        md(&dial_sentence(&a.consequence)),
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
    let mut fallback_any = false;
    for b in &a.buckets {
        if b.id.kind() != BucketKind::Duration {
            continue;
        }
        let back = match relief_fallback_days(b) {
            Some(d) => {
                fallback_any = true;
                format!("worst on record: up to {}", text::day_phrase(f64::from(d)))
            }
            None => relief_cell(b.relief.as_ref().map(|r| r.mostly_restored_days)),
        };
        out.push(format!(
            "| {} | {} | {} | {back} | {} |",
            md(&b.name),
            target_phrase(&b.target),
            relief_cell(b.relief.as_ref().map(|r| r.help_arrives_days)),
            tier_cell(b.tier_enough)
        ));
    }
    out.push(String::new());
    out.push(format!(
        "Brackets show how uncertain a target is. \"Not known\": no restoration records for the \
         event behind that target.{}",
        if fallback_any {
            " \"Worst on record\": the target rests on an estimate, so the worst event on record \
             stands in (named below)."
        } else {
            ""
        }
    ));
    out.push(String::new());
    validation(cx, out);

    // Blocks a risk card already showed, with the card's name; and cards whose hazard has one
    // consequence, whose advice is that bucket's (a medical emergency).
    let cards = super::risks::cards(cx);
    let shown: Vec<(String, String)> = cards
        .iter()
        .filter_map(|(p, g)| g.map(|g| (g.meta.id.clone(), p.name.clone())))
        .collect();
    // Cards whose hazard has one consequence, whose advice is that bucket's (a medical
    // emergency): the bucket points to the card. Each entry is the pointer line itself.
    let see = |names: &str| format!("**What helps.** See {names} under Your risks.");
    let mut single: Vec<(BucketId, String)> = cards
        .iter()
        .filter(|(p, g)| g.is_some() && p.buckets.len() == 1)
        .map(|(p, _)| (p.buckets[0], see(&format!("\"{}\"", md(&p.name)))))
        .collect();
    // Dangerous heat or cold at home: the heat-wave and cold-wave cards' advice (cooling places, a
    // warm room, fans above 90°F, no oven or grill for heat) is this bucket's, and the forecast
    // list's steps before a heat wave and a freeze cover the other one, so with either card shown
    // it points there. Not for a home heated with wood, whose stove and chimney advice is this
    // bucket's own.
    let card_name = |h: rr_types::HazardId| {
        cards
            .iter()
            .find(|(p, g)| p.id == h && g.is_some())
            .map(|(p, _)| md(&p.name))
    };
    let wood = a.input.housing.heating == rr_types::Heating::Wood;
    if !wood {
        match (
            card_name(rr_types::HazardId::HeatWave),
            card_name(rr_types::HazardId::ColdWave),
        ) {
            (Some(h), Some(c)) => {
                single.push((BucketId::Thermal, see(&format!("\"{h}\" and \"{c}\""))))
            }
            (Some(n), None) | (None, Some(n)) => single.push((
                BucketId::Thermal,
                format!(
                    "**What helps.** See \"{n}\" under Your risks, and the steps before a heat \
                     wave or a freeze on the forecast list."
                ),
            )),
            (None, None) => {}
        }
    }

    // What to do about these is in the documents-and-money section.
    let elsewhere = [BucketId::HomeLoss, BucketId::Income];
    // What helps with these is in a packet v2 section of its own (the family plan and wallet
    // cards, the shelter plan and the forecast list, the house-fire card and the safety rules,
    // the plan's free steps); their warnings stay here.
    let medicine_lines = a
        .input
        .people
        .iter()
        .any(|p| p.medical.daily_rx || p.medical.refrigerated_rx || p.medical.epinephrine);
    let pointer = |b: BucketId| -> Option<&'static str> {
        match b {
            BucketId::Evacuate | BucketId::GetHome => {
                Some("**What helps.** See Your family plan and Your shelter plan.")
            }
            // Lights, a radio and phone power are in the plan; the fridge rule and charging are
            // in the forecast list; registries are under Local help; a powered device or cold
            // medicine has its lines under Special needs.
            BucketId::Power => Some(
                "**What helps.** A light for each person, a radio and phone power are in your \
                 plan; keeping food cold and charging up are on the forecast list; registries \
                 are under Local help.",
            ),
            BucketId::Comms => Some(
                "**What helps.** See Your family plan (staying in touch), the wallet cards and \
                 the list for when a storm is forecast.",
            ),
            BucketId::Fire => Some(
                "**What helps.** See \"House fire\" under Your risks and the safety rules under \
                 Your plan.",
            ),
            BucketId::Security => {
                Some("**What helps.** See the home-security and trusted-circle steps in Your plan.")
            }
            BucketId::CleanAir => {
                Some("**What helps.** See the clean room under Your shelter plan.")
            }
            // The food itself is on the checklists, in calories; what to avoid stays here.
            BucketId::Supplies => Some(
                "**What helps.** The food is on your checklists, counted in calories, not \
                 servings: pick foods you already eat, and use the oldest first.",
            ),
            // Special needs prints the household's own medicine lines (how many days, the
            // written list, refills, cold storage), so what helps is there.
            BucketId::Medication if medicine_lines => {
                Some("**What helps.** See Medicine under Special needs.")
            }
            _ => None,
        }
    };
    for b in a.buckets.iter().filter(|b| !elsewhere.contains(&b.id)) {
        let target = &b.target;
        if !is_active(target) {
            continue;
        }
        let heading = match b.id.kind() {
            BucketKind::Duration => format!("### {}: {}", md(&b.name), target_phrase(target)),
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
            let card: Option<String> = shown
                .iter()
                .find(|(id, _)| *id == g.meta.id)
                .map(|(_, name)| see(&format!("\"{}\"", md(name))))
                .or_else(|| {
                    single
                        .iter()
                        .find(|(x, _)| *x == b.id)
                        .map(|(_, line)| line.clone())
                });
            let advice = super::advice_paragraphs(&cx.guidance(g, None, Some(&target_words)));
            let avoid = advice.iter().filter(|p| !p.starts_with("**What helps.**"));
            match card {
                Some(line) => out.push(line),
                None if pointer(b.id).is_some() => {
                    out.push(pointer(b.id).unwrap_or_default().to_owned());
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
        let lines = model_lines(cx, b);
        if !lines.is_empty() {
            out.push(format!("{}{}", lines.join(" "), cite_all(&b.sources)));
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
            "A named scenario is one rare, severe event that would change your targets a lot; \
             you can turn each on or off on the risks screen."
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
pub(crate) fn is_active(t: &Target) -> bool {
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
