//! `rr targets`: how long to be ready for each need at the household's dial, with the range, what
//! the plan covers, when outside help plausibly arrives and service is mostly back (the two-tier
//! relief rating), and the tier that is enough. `--sweep` runs the engine at every dial setting,
//! 1 in 10 to 1 in 500, so a target that jumps between settings (the cliff rule, DESIGN §4.4) is
//! visible at a glance.

use rr_plan::{Assessment, Engine};
use rr_types::{
    BucketAssessment, BucketId, BucketKind, PlanItemKind, ReturnPeriod, Target, TierId,
};

use super::{header, with_dial};
use crate::Output;
use crate::args::TargetsArgs;
use crate::error::CliError;
use crate::format::{self, Table, day_phrase, households, number, wrap};
use crate::household::{self, Household};
use crate::source::Source;

/// Width the sentences under each row wrap at.
const WRAP: usize = 96;

/// A target counts as a jump when it is at least this many times the setting to its left.
pub const JUMP_FACTOR: f32 = 3.0;

/// Runs `rr targets`.
///
/// # Errors
///
/// Household problems or an unknown location (exit 2); an engine error (exit 1).
pub fn run(engine: &Engine<Source>, args: &TargetsArgs) -> Result<Output, CliError> {
    let h = household::load(&args.household)?;
    let hint = engine.store().location_hint();
    let a = engine
        .run(&h.input)
        .map_err(|e| CliError::engine(&e, hint.as_deref()))?;
    let mut out = Output::text(if args.sweep {
        let mut runs = Vec::new();
        for rp in ReturnPeriod::ALL {
            let r = engine
                .run(&with_dial(&h.input, *rp))
                .map_err(|e| CliError::engine(&e, hint.as_deref()))?;
            runs.push((*rp, r));
        }
        sweep(engine, &h, &a, &runs)
    } else {
        targets(engine, &h, &a)
    });
    out.notes = household::scenario_notes(&h, &a.consequence.scenarios, &super::place(&a.location));
    Ok(out)
}

/// Of 100 ten-year stretches, how many bring something worse than one need's target at this
/// dial (each target holds for its own need; model review M-04).
fn worse_per_100(rp: ReturnPeriod) -> String {
    format::per_100(format::chance(rr_consequence::dial_rate(rp), 10.0))
}

fn relief_cell(days: Option<f32>) -> String {
    match days {
        Some(d) => format!("about {}", day_phrase(round_relief(f64::from(d)))),
        None => "not known".to_owned(),
    }
}

/// Relief days on the target ladder, as the packet rounds them ("3.2 days" reads "3 days").
fn round_relief(d: f64) -> f64 {
    if d < 0.75 {
        0.5
    } else if d < 14.0 {
        d.round().max(1.0)
    } else {
        f64::from(rr_consequence::round_up_to_ladder(d))
    }
}

fn tier_words(t: TierId) -> String {
    match t {
        TierId::Now => "free steps".to_owned(),
        other => format::lower_first(other.name()),
    }
}

fn contributions(b: &BucketAssessment) -> Option<String> {
    let parts: Vec<String> = b
        .contributions
        .iter()
        .filter(|c| c.share >= 0.005)
        .take(4)
        .map(|c| format!("{} {:.0}%", c.hazard.name(), 100.0 * c.share))
        .collect();
    (!parts.is_empty()).then(|| format!("Driven by: {}", parts.join(", ")))
}

/// The default report.
fn targets(engine: &Engine<Source>, h: &Household, a: &Assessment) -> String {
    let rp = h.input.dials.return_period;
    let mut s = header(
        &format!("Targets for {}", super::place(&a.location)),
        h,
        &a.location,
        engine.store(),
    );
    s.push_str(&format!(
        "\n{}\n",
        format::wrap(&rr_plan::packet::dial_sentence(rp), 100, 0)
    ));

    s.push_str("\nHow long to manage on your own\n\n");
    let mut t = Table::new([
        "Need",
        "Be ready for",
        "Plan covers",
        "Help arrives",
        "Mostly back",
        "Enough at",
    ]);
    for b in a
        .buckets
        .iter()
        .filter(|b| b.id.kind() == BucketKind::Duration)
    {
        let (target, covered) = match (b.target, b.covered) {
            (Target::Days { value, low, high }, Target::Days { value: c, .. }) => (
                format::target_days(f64::from(value), f64::from(low), f64::from(high)),
                if value > 0.0 {
                    day_phrase(f64::from(c))
                } else {
                    "-".to_owned()
                },
            ),
            _ => ("-".to_owned(), "-".to_owned()),
        };
        let active = matches!(b.target, Target::Days { value, .. } if value > 0.0);
        let (help, back) = if active {
            (
                relief_cell(b.relief.as_ref().map(|r| r.help_arrives_days)),
                relief_cell(b.relief.as_ref().map(|r| r.mostly_restored_days)),
            )
        } else {
            ("-".to_owned(), "-".to_owned())
        };
        t.row([
            b.name.clone(),
            target,
            covered,
            help,
            back,
            tier_words(b.tier_enough),
        ]);
        if active {
            for sentence in &b.frequency_sentences {
                t.note(sentence.clone());
            }
            if let Some(c) = contributions(b) {
                t.note(c);
            }
        }
    }
    s.push_str(&t.render(1));
    s.push_str(
        "\n The range in brackets is where the target could sit when the inputs behind it are \
         uncertain (10th to 90th\n percentile). \"Not known\" means no restoration records exist \
         for that kind of disruption.\n",
    );

    s.push_str("\nReadiness (a capability you have or do not have)\n\n");
    let mut t = Table::new(["Need", "Chance of needing it, 10 years", "In the plan"]);
    for b in &a.buckets {
        match (b.target, b.covered) {
            (
                Target::Evacuate {
                    p_need_10yr,
                    notice_hours_low,
                    notice_hours_high,
                    days_away,
                },
                Target::Evacuate {
                    days_away: bag_days,
                    ..
                },
            ) => {
                t.row([
                    b.name.clone(),
                    households(p_need_10yr),
                    if bag_days > 0.0 {
                        "a go-bag".to_owned()
                    } else {
                        "no go-bag yet".to_owned()
                    },
                ]);
                t.note(format!(
                    "Warning: {}. Plan to be away about {}.",
                    format::notice_range(f64::from(notice_hours_low), f64::from(notice_hours_high)),
                    day_phrase(f64::from(days_away))
                ));
            }
            (Target::Readiness { p_need_10yr, .. }, Target::Readiness { done, of, .. })
                if b.id.kind() == BucketKind::Readiness || b.id == BucketId::HomeLoss =>
            {
                t.row([
                    b.name.clone(),
                    households(p_need_10yr),
                    if of > 0 {
                        format!("{done} of {of} steps")
                    } else {
                        "-".to_owned()
                    },
                ]);
            }
            _ => {}
        }
        if matches!(b.id.kind(), BucketKind::Readiness) || b.id == BucketId::HomeLoss {
            if let Some(c) = contributions(b) {
                t.note(c);
            }
        }
    }
    s.push_str(&t.render(1));

    let income = a.bucket(BucketId::Income);
    if let Target::Months { value, low, high } = income.target {
        s.push_str("\nMoney (a savings goal, never paid from the supplies budget)\n\n");
        s.push_str(&format!(
            "  {}: {}.\n",
            income.name,
            format::target_months(f64::from(value), f64::from(low), f64::from(high))
        ));
        if let Some(st) = &a.budget.plan.savings_track {
            s.push_str(&format!(
                "  Saved now: {}. Goal: {} ({}). Suggested: {} a month.\n",
                format::months_phrase(f64::from(st.current_months)),
                format::months_phrase(f64::from(st.target_months)),
                format::usd(f64::from(st.target_usd)),
                format::usd(f64::from(st.monthly_suggestion_usd))
            ));
        }
        if let Some(c) = contributions(income) {
            s.push_str(&format!("  {c}\n"));
        }
    }

    let recommended = rr_supply::tier_recommended(&a.buckets);
    let now = match a.budget.tier_reached {
        TierId::Now => "getting started".to_owned(),
        t => format!("{} covered", format::lower_first(t.name())),
    };
    s.push_str(&format!(
        "\nEnough for this household: {}. Where you are now: {now}.\n",
        tier_words(recommended)
    ));
    let cliffs: Vec<&rr_types::Warning> = a
        .warnings
        .iter()
        .filter(|w| w.id.starts_with("cliff_"))
        .collect();
    if !cliffs.is_empty() {
        s.push_str("\nWhen one event drives the answer\n\n");
        for w in cliffs {
            s.push_str(&format!(
                "  - {}\n",
                wrap(&format!("{} {}", w.message, w.why), WRAP, 4)
            ));
        }
    }
    s
}

/// "14 (10–30)"; "-" when there is no target.
fn cell(value: f32, low: f32, high: f32) -> String {
    if value <= 0.0 {
        return "-".to_owned();
    }
    let v = number(f64::from(value));
    if (low - value).abs() < 1e-6 && (high - value).abs() < 1e-6 {
        v
    } else {
        format!(
            "{v} ({}–{})",
            number(f64::from(low)),
            number(f64::from(high))
        )
    }
}

/// The value and range of a days or months target.
fn span(t: &Target) -> Option<(f32, f32, f32)> {
    match *t {
        Target::Days { value, low, high } | Target::Months { value, low, high } => {
            Some((value, low, high))
        }
        _ => None,
    }
}

/// Spend on purchases the plan makes (not savings deposits, not things already owned).
fn purchases(a: &Assessment) -> f64 {
    a.budget
        .plan
        .months
        .iter()
        .flat_map(|m| m.items.iter())
        .filter(|i| i.kind == PlanItemKind::Purchase && !i.done)
        .map(|i| f64::from(i.est_cost_usd))
        .sum()
}

/// The dial sweep.
fn sweep(
    engine: &Engine<Source>,
    h: &Household,
    a: &Assessment,
    runs: &[(ReturnPeriod, Assessment)],
) -> String {
    let current = h.input.dials.return_period;
    let mut s = header(
        &format!(
            "Targets at every dial setting for {}",
            super::place(&a.location)
        ),
        h,
        &a.location,
        engine.store(),
    );
    s.push_str(
        "\nDays to be ready for (income in months), with the range in brackets. * marks the \
         household's setting.\n\n",
    );
    let mut head: Vec<String> = vec!["Need".to_owned()];
    for (rp, _) in runs {
        let mark = if *rp == current { " *" } else { "" };
        head.push(format!("{}{mark}", format::dial_short(*rp)));
    }
    let mut t = Table::new(head);
    let mut jumps = 0usize;
    for id in BucketId::ALL {
        let spans: Vec<Option<(f32, f32, f32)>> = runs
            .iter()
            .map(|(_, r)| span(&r.bucket(*id).target))
            .collect();
        if spans.iter().all(Option::is_none) {
            continue;
        }
        let name = if *id == BucketId::Income {
            format!("{} (months)", id.name())
        } else {
            id.name().to_owned()
        };
        let mut row = vec![name];
        let mut prev: Option<f32> = None;
        for sp in &spans {
            let (v, lo, hi) = sp.unwrap_or((0.0, 0.0, 0.0));
            let mut c = cell(v, lo, hi);
            if let Some(p) = prev.filter(|p| *p > 0.0) {
                if v >= JUMP_FACTOR * p {
                    c.push_str(" !");
                    jumps += 1;
                }
            }
            prev = Some(v);
            row.push(c);
        }
        t.row(row);
    }
    t.row(vec![String::new(); runs.len() + 1]);
    let mut worse = vec!["Worse for one need, of 100 ten-year stretches".to_owned()];
    let mut tier = vec!["Enough for this household".to_owned()];
    let mut done = vec!["Plan done by month".to_owned()];
    let mut spend = vec!["Purchases in the plan".to_owned()];
    for (rp, r) in runs {
        worse.push(worse_per_100(*rp));
        tier.push(tier_words(rr_supply::tier_recommended(&r.buckets)));
        done.push(match r.budget.plan.done_month {
            Some(m) => m.to_string(),
            None => "not in 10 years".to_owned(),
        });
        spend.push(format::usd(purchases(r)));
    }
    t.row(worse);
    t.row(tier);
    t.row(done);
    t.row(spend);
    s.push_str(&t.render(1));
    if jumps > 0 {
        s.push_str(&format!(
            "\n ! = at least {JUMP_FACTOR:.0} times the setting to its left: the answer jumps \
             between settings, usually because\n one rare event sits close to the dial (the cliff \
             rule). The engine's own warnings follow.\n"
        ));
    }

    // Cliff warnings at any setting: the settings that raise each one, and the engine's words at
    // the raising setting nearest the household's own (its "the level you chose" means that one).
    let position = |rp: ReturnPeriod| ReturnPeriod::ALL.iter().position(|x| *x == rp).unwrap_or(0);
    let mut cliffs: Vec<(String, Vec<(ReturnPeriod, &rr_types::Warning)>)> = Vec::new();
    for (rp, r) in runs {
        for w in r.warnings.iter().filter(|w| w.id.starts_with("cliff_")) {
            match cliffs.iter_mut().find(|c| c.0 == w.id) {
                Some(c) => c.1.push((*rp, w)),
                None => cliffs.push((w.id.clone(), vec![(*rp, w)])),
            }
        }
    }
    if !cliffs.is_empty() {
        s.push_str("\nWhen one event drives the answer\n\n");
        for (id, at) in &cliffs {
            let bucket = id
                .strip_prefix("cliff_")
                .and_then(|b| b.parse::<BucketId>().ok())
                .map(|b| b.name().to_owned())
                .unwrap_or_else(|| id.clone());
            let settings: Vec<String> = at.iter().map(|(rp, _)| format::dial_short(*rp)).collect();
            let nearest = at
                .iter()
                .min_by_key(|(rp, _)| position(*rp).abs_diff(position(current)))
                .map(|(rp, w)| (*rp, *w));
            if let Some((rp, w)) = nearest {
                let line = format!(
                    "{bucket}, raised at {}. At {}: {} {}",
                    format::join_and(&settings),
                    format::dial_short(rp),
                    w.message,
                    w.why
                );
                s.push_str(&format!("  - {}\n", wrap(&line, WRAP, 4)));
            }
        }
    }
    let scenarios: Vec<String> = a
        .consequence
        .scenarios
        .iter()
        .map(|sc| format!("{} {}", sc.id, if sc.on { "on" } else { "off" }))
        .collect();
    if !scenarios.is_empty() {
        s.push_str(&format!(
            "\nNamed scenarios in this sweep: {}. See the plan without one with --scenario \
             <id>=off --sweep.\n",
            scenarios.join(", ")
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_show_the_range_only_when_there_is_one() {
        assert_eq!(cell(14.0, 10.0, 30.0), "14 (10–30)");
        assert_eq!(cell(3.0, 3.0, 3.0), "3");
        assert_eq!(cell(0.5, 0.5, 1.0), "0.5 (0.5–1)");
        assert_eq!(cell(0.0, 0.0, 0.0), "-");
    }

    #[test]
    fn relief_rounds_like_the_packet() {
        assert_eq!(round_relief(0.3), 0.5);
        assert_eq!(round_relief(3.2), 3.0);
        assert_eq!(round_relief(16.0), 21.0);
    }
}
