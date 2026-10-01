//! The targets in words, for the binder's What to expect page and the explain view: a target
//! with its range ("about 5 days (up to 10 days)"), when help likely arrives and service is mostly
//! back, how long the worst event on record kept homes out (read as the app reads it), the dial
//! setting in words (computed from the model, model review M-04) and "How well do these numbers
//! hold up?" from the frozen backtest.

use rr_consequence::ConsequenceAssessment;
use rr_types::{BucketAssessment, BucketId, StressTest, Target};

use super::text;
use super::{Ctx, cite};

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

pub(crate) fn relief_cell(days: Option<f32>) -> String {
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

/// The "How well do these numbers hold up?" line, with its citation marker; `None` when the
/// backtest table is empty.
pub(crate) fn validation_line(cx: &Ctx<'_>) -> Option<String> {
    let v = crate::validation::summary();
    if v.events_tested == 0 {
        return None;
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
    Some(format!(
        "**How well do these numbers hold up?** Tested against {} real disasters, this version \
         covered {}, partly covered {}, fell short on {} and could not model {}.{} {meaning}",
        v.events_tested,
        v.covered,
        v.partial,
        v.short,
        v.not_modelled,
        cite(crate::validation::CITATION)
    ))
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
