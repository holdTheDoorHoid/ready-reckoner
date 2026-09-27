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
use super::{Ctx, cite, cite_all};

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
/// sentence, computed from its event list (`ConsequenceAssessment::dial_sentence`: the chance of
/// a longer disruption of any one kind, and the higher chance that at least one kind runs past its
/// target, from the joint rate, both on one scale, so the rarer settings read "about 2 in 100"),
/// then [`COPE_SENTENCE`].
pub fn dial_sentence(c: &ConsequenceAssessment) -> String {
    format!("{} {COPE_SENTENCE}", c.dial_sentence())
}

fn relief_cell(days: Option<f32>) -> String {
    match days {
        Some(d) => format!("about {}", text::day_phrase(round_relief(f64::from(d)))),
        None => "not known".to_owned(),
    }
}

/// Relief days in plain words, as a person would say them and as the explain view says them:
/// whole days under a month ("3 days" for 3.2), whole months under a year ("9 months" for 270,
/// a month and a half kept), years to the nearest half beyond ("about 3 years" for the Oregon
/// Resilience Plan's 1,095 days; verification R3-20). They are what a source says, not targets,
/// so they are never rounded up to the target ladder, which stops at a year.
fn round_relief(d: f64) -> f64 {
    if d < 0.75 {
        return 0.5;
    }
    if d < 30.0 {
        return d.round().max(1.0);
    }
    if d < 365.0 {
        if (d - 45.0).abs() < 7.5 {
            return 45.0;
        }
        let months = (d / 30.0).round();
        return if months >= 12.0 { 365.0 } else { months * 30.0 };
    }
    (d / 365.0 * 2.0).round() / 2.0 * 365.0
}

/// A home counts as still out at a mark while at least this share of customers is (0.5 in 100),
/// the app's threshold (`BACK` in `web/src/lib/targets.ts`).
const STILL_OUT: f32 = 0.005;

/// How long the worst event on record kept homes out, read as the app reads it (`stressLine` in
/// `web/src/lib/targets.ts`, verification R3-05), so print and screen agree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RecordSpan {
    /// Some homes were out up to this many days: the first mark after the last one with at least
    /// 0.5 in 100 still out, or a water event's nine-in-ten duration.
    UpTo(f32),
    /// Still out at the last mark: longer than this many days.
    MoreThan(f32),
    /// Nearly every home was back by the first mark.
    Within(f32),
    /// A water event recorded as one duration.
    About(f32),
}

impl RecordSpan {
    /// "up to 2 weeks", "more than 1 month", "within 1 day", "about 6 days".
    pub(crate) fn words(self) -> String {
        match self {
            RecordSpan::UpTo(d) => format!("up to {}", text::day_phrase(f64::from(d))),
            RecordSpan::MoreThan(d) => format!("more than {}", text::day_phrase(f64::from(d))),
            RecordSpan::Within(d) => format!("within {}", text::day_phrase(f64::from(d))),
            RecordSpan::About(d) => format!("about {}", text::day_phrase(f64::from(d))),
        }
    }
}

/// The record's span from its marks. Power events carry the share of customers still out after
/// 1, 3, 7, 14 and 30 days: the span runs to the first mark after the last one with at least
/// [`STILL_OUT`] still out (a home still out at day 7 and back by day 14 reads "up to 2 weeks",
/// not "up to 7 days"). A water event carries the median home and the ninth in ten
/// (`[[median, 0.5], [p90, 0.1]]`), read as up to the ninth in ten, or one duration.
pub(crate) fn record_span(st: &StressTest, bucket: BucketId) -> Option<RecordSpan> {
    let mut pts: Vec<(f32, f32)> = st
        .share_out_at_days
        .iter()
        .copied()
        .filter(|(d, _)| d.is_finite() && *d > 0.0)
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let first = *pts.first()?;
    let water = matches!(bucket, BucketId::WaterOut | BucketId::WaterBoil);
    if water && pts.len() == 2 && pts[0].1 >= 0.5 && pts[1].1 <= 0.1 {
        return Some(RecordSpan::UpTo(pts[1].0));
    }
    if water && pts.len() == 1 {
        return Some(RecordSpan::About(first.0));
    }
    let Some(last_out) = pts.iter().rposition(|(_, s)| *s >= STILL_OUT) else {
        return Some(RecordSpan::Within(first.0));
    };
    Some(match pts.get(last_out + 1) {
        Some((d, _)) => RecordSpan::UpTo(*d),
        None => RecordSpan::MoreThan(pts[last_out].0),
    })
}

/// The relief fallback (round-2 follow-up): when the event behind a target has no restoration
/// record, the relief rating is not known; where the region's record has a worst event, the
/// "mostly back" cell says how long it kept homes out instead ("worst on record: up to 2 weeks"),
/// the note under the table says so, and the bucket's part below names the event (its
/// worst-event line).
pub(crate) fn relief_fallback(b: &BucketAssessment) -> Option<RecordSpan> {
    if b.relief.is_some() || !matches!(b.target, Target::Days { value, .. } if value > 0.0) {
        return None;
    }
    if !matches!(
        b.id,
        BucketId::Power | BucketId::WaterOut | BucketId::WaterBoil
    ) {
        return None;
    }
    record_span(b.stress_test.as_ref()?, b.id)
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
/// for this version, from the table bundled with the engine (`EngineInfo::validation`), cited to
/// the published table's registry entry ([`crate::validation::CITATION`]), and what it means for
/// the household (`topic:validation`).
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
            // has lived through. (The table is the source the bracket points to.)
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
         covered {}, partly covered {}, fell short on {} and could not model {}.{} {meaning}",
        v.events_tested,
        v.covered,
        v.partial,
        v.short,
        v.not_modelled,
        cite(crate::validation::CITATION)
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
        // Headings that read on into their cells ("Help likely in about 3 days") and leave the
        // columns room, so fewer rows wrap in print (R3-15).
        "| If this happens | Be ready for | Help likely in | Mostly back in | Enough at |"
            .to_owned(),
    );
    out.push("| --- | --- | --- | --- | --- |".to_owned());
    let mut fallback_any = false;
    for b in &a.buckets {
        if b.id.kind() != BucketKind::Duration {
            continue;
        }
        let back = match relief_fallback(b) {
            Some(span) => {
                fallback_any = true;
                format!("worst on record: {}", span.words())
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
         event behind that target{}",
        if fallback_any {
            "; \"worst on record\": no such records either, so the worst event on record stands \
             in (named below)."
        } else {
            "."
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stress(marks: &[(f32, f32)]) -> StressTest {
        StressTest {
            event: "Winter storm, April 2017".to_owned(),
            date: rr_types::Date::from_ymd(2017, 4, 29).unwrap(),
            region: "your region's records".to_owned(),
            share_out_at_days: marks.to_vec(),
            covered_by_target: false,
            sources: Vec::new(),
        }
    }

    #[test]
    fn the_record_reads_as_the_app_reads_it() {
        let power = BucketId::Power;
        // Hays: 3.5 in 100 still out on day 7, none on day 14.
        let hays = [
            (1.0, 0.14),
            (3.0, 0.089),
            (7.0, 0.035),
            (14.0, 0.0),
            (30.0, 0.0),
        ];
        assert_eq!(
            record_span(&stress(&hays), power),
            Some(RecordSpan::UpTo(14.0))
        );
        // Under 0.5 in 100 does not count as still out (Minot: 0.44 in 100 on day 7).
        let minot = [(1.0, 0.015), (3.0, 0.0063), (7.0, 0.0044), (14.0, 0.0)];
        assert_eq!(
            record_span(&stress(&minot), power),
            Some(RecordSpan::UpTo(7.0))
        );
        // Still out at the last mark: longer than it.
        let maria = [(1.0, 1.0), (7.0, 1.0), (14.0, 0.91), (30.0, 0.82)];
        let span = record_span(&stress(&maria), power).unwrap();
        assert_eq!(span, RecordSpan::MoreThan(30.0));
        assert_eq!(span.words(), "more than 1 month");
        // Nearly every home back by the first mark.
        let brief = [(1.0, 0.002), (3.0, 0.0)];
        assert_eq!(
            record_span(&stress(&brief), power).unwrap().words(),
            "within 1 day"
        );
        // Water: the median and the ninth in ten, or one duration.
        let two = [(68.0, 0.5), (150.0, 0.1)];
        assert_eq!(
            record_span(&stress(&two), BucketId::WaterOut)
                .unwrap()
                .words(),
            "up to 5 months"
        );
        assert_eq!(
            record_span(&stress(&[(6.0, 1.0)]), BucketId::WaterBoil)
                .unwrap()
                .words(),
            "about 6 days"
        );
        assert_eq!(record_span(&stress(&[]), power), None);
    }

    #[test]
    fn relief_says_what_the_source_says() {
        let words = |d: f64| text::day_phrase(round_relief(d));
        assert_eq!(words(0.4), "half a day");
        assert_eq!(words(3.2), "3 days");
        assert_eq!(words(5.19), "5 days");
        assert_eq!(words(14.0), "2 weeks");
        assert_eq!(words(45.0), "1½ months");
        assert_eq!(words(180.0), "6 months");
        assert_eq!(words(270.0), "9 months");
        assert_eq!(words(360.0), "1 year");
        assert_eq!(words(1095.0), "3 years");
        assert_eq!(words(540.0), "1½ years");
    }
}
