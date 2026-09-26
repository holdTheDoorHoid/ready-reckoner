//! Section 3: your targets. The dial setting in words, the duration targets with their ranges and
//! relief rating, then one part per bucket: the bucket's guidance block with the household's own
//! frequency sentence, the rest of its numbers, and the requirement lines that count toward it;
//! then named scenarios and any event that drives the answer.

use rr_supply::{LineKind, SizedLine};
use rr_types::{BucketId, BucketKind, Target, TierId};

use super::text::{self, md};
use super::{Ctx, cite, cite_all};
use crate::pipeline::Assessment;

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

/// Out of 100, how many ten-year stretches bring something worse than the targets at this dial.
fn worse_per_100(a: &Assessment) -> String {
    let rate = rr_consequence::dial_rate(a.input.dials.return_period);
    text::per_100(-rr_types::math::exp_m1(-10.0 * rate))
}

/// The requirement lines of a bucket, split into needs and everything else worth knowing.
fn lines_of(a: &Assessment, b: BucketId) -> (Vec<&SizedLine>, Vec<&SizedLine>) {
    let mine: Vec<&SizedLine> = a.lines.iter().filter(|l| l.line.bucket == b).collect();
    let needs = mine
        .iter()
        .copied()
        .filter(|l| l.kind == LineKind::Need && l.quantity > 0.0)
        .collect();
    let other = mine
        .iter()
        .copied()
        .filter(|l| {
            matches!(
                l.kind,
                LineKind::Alternative | LineKind::Optional | LineKind::Note
            ) || (l.kind == LineKind::Need && l.quantity <= 0.0)
        })
        .collect();
    (needs, other)
}

fn line_item(l: &SizedLine) -> String {
    format!("- {}{}", md(&l.line.plain), cite_all(&l.line.citations))
}

fn write_lines(a: &Assessment, b: BucketId, out: &mut Vec<String>) {
    let (needs, other) = lines_of(a, b);
    if !needs.is_empty() {
        out.push("**What counts toward it:**".to_owned());
        out.push(String::new());
        for l in needs {
            out.push(line_item(l));
        }
        out.push(String::new());
    }
    if !other.is_empty() {
        out.push("**Also worth knowing:**".to_owned());
        out.push(String::new());
        for l in other {
            out.push(line_item(l));
        }
        out.push(String::new());
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

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    out.push("## Your targets".to_owned());
    out.push(String::new());
    out.push(format!(
        "How long to be ready for each kind of disruption, at the setting you chose: {}. At \
         this setting, something worse than these targets comes in about {} of every 100 \
         ten-year stretches.{} Water is planned at {}.",
        super::summary::dial_phrase(cx),
        worse_per_100(a),
        cite("rr_research_risk_model"),
        super::summary::water_phrase(a.input.dials.water_level)
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
        "The range in brackets shows where the target could sit when the inputs behind it are \
         uncertain. \"Not known\" means no restoration records exist for that kind of disruption."
            .to_owned(),
    );
    out.push(String::new());
    if a.input.dials.climate == rr_types::ClimateHorizon::Y2050 {
        if let Some(g) = cx.blocks_for("topic:climate_horizon").first() {
            out.push(format!("#### {}", md(&g.meta.title)));
            out.push(String::new());
            out.push(cx.guidance(g, None, None));
            out.push(String::new());
        }
    }

    for b in &a.buckets {
        let target = &b.target;
        let active = match *target {
            Target::Days { value, .. } | Target::Months { value, .. } => value > 0.0,
            Target::Evacuate { p_need_10yr, .. } | Target::Readiness { p_need_10yr, .. } => {
                p_need_10yr > 0.0
            }
        };
        if !active {
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
        let mut sentences = b.frequency_sentences.iter();
        let first = sentences.next();
        let lead = first.map(|s| format!("{s}{}", cite_all(&b.sources)));
        let block = cx
            .blocks_for(&format!("bucket:{}", b.id))
            .into_iter()
            .next();
        let target_words = target_phrase(target);
        match block {
            Some(g) => out.push(cx.guidance(g, lead.as_deref(), Some(&target_words))),
            None => {
                if let Some(l) = &lead {
                    out.push(md(l));
                }
            }
        }
        out.push(String::new());
        let rest: Vec<String> = sentences.map(|s| md(s)).collect();
        if !rest.is_empty() {
            out.push(format!(
                "**Your numbers.** {}{}",
                rest.join(" "),
                cite_all(&b.sources)
            ));
            out.push(String::new());
        }
        if let Some(r) = &b.relief {
            out.push(format!(
                "**When help comes.** For the kind of disruption behind this target, outside \
                 help plausibly arrives in about {} and service is mostly back in about {}.{}",
                text::day_phrase(round_relief(f64::from(r.help_arrives_days))),
                text::day_phrase(round_relief(f64::from(r.mostly_restored_days))),
                cite_all(&r.sources)
            ));
            out.push(String::new());
        }
        if b.id == BucketId::Income {
            if let Some(s) = &a.budget.plan.savings_track {
                out.push(format!("**Your savings goal.** {}", md(&s.why)));
                out.push(String::new());
            }
        }
        write_lines(a, b.id, out);
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

fn tier_cell(t: TierId) -> String {
    match t {
        TierId::Now => "free steps".to_owned(),
        other => text::lower_first(other.name()),
    }
}
