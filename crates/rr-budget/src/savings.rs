//! The savings track for the `income` bucket (DESIGN §4.4, §4.7): months of expenses to have
//! saved, never funded from the supplies budget. Surplus goes here once the supplies plan runs out
//! of things worth buying.
//!
//! The suggested monthly amount is the household's own supplies budget, freed when the supplies
//! plan is done; no saving pace is invented (FINRA and the St. Louis Fed give a 3–6 month goal but
//! no pace; the CFPB says to start small).
//!
//! Contract v2 adds three things on the same pace:
//!
//! - **The first step** ([`first_milestone`], `Plan::first_milestone`): one month of expenses or
//!   $500, whichever is smaller, and the plan month it is reached (professional review RR-P09;
//!   the Federal Reserve's finding that many households could not cover a $400 emergency is why a
//!   small first goal comes before "3 to 6 months").
//! - **The three-month point** in the savings sentence, when the goal is longer than three months.
//! - **A legal-emergency line** for households that turn it on from the arrest row of the register
//!   (`Dials::legal_opt_in`): a cited typical bail figure with its wide range, shown apart from the
//!   months of income the goal protects (owner decision 2026-09-26).

use rr_types::{BucketId, CitationId, HazardId, PlanInput, SavingsMilestone, SavingsTrack, Target};

use crate::explain::dollars;
use crate::input::Risks;

/// The first savings step is one month of expenses or this many dollars, whichever is smaller
/// (RR-P09; REVIEW N4).
pub const FIRST_MILESTONE_USD: f32 = 500.0;

/// The savings sentence also gives the month three months of expenses are saved, when the goal is
/// longer (the low end of the 3–6 month goal FINRA and the St. Louis Fed give).
pub const THREE_MONTH_POINT: f32 = 3.0;

/// The legal-emergency line's typical figure: the median bail amount set for felony defendants
/// in the 75 largest US counties, $10,000 (Bureau of Justice Statistics, *Felony Defendants in
/// Large Urban Counties, 2009 — Statistical Tables*, NCJ 243777). Requested in
/// `docs/CITATION_IDS.md` as [`LEGAL_COST_CITATION`], with the figure to be confirmed against the
/// source by the content workstream.
pub const LEGAL_BAIL_MEDIAN_USD: f64 = 10_000.0;

/// The citation for [`LEGAL_BAIL_MEDIAN_USD`] (awaited: `rr_plan::provenance::AWAITING_CONTENT`).
pub const LEGAL_COST_CITATION: &str = "bjs_felony_defendants_2009";

/// The income target in months, when the plan has one above zero.
fn target_months(risks: &Risks) -> Option<f32> {
    let a = risks.assessments.get(&BucketId::Income)?;
    match a.target {
        Target::Months { value, .. } if value.is_finite() && value > 0.0 => Some(value),
        _ => None,
    }
}

/// Monthly expenses, when the household gave them.
fn expenses(household: &PlanInput) -> Option<f32> {
    household
        .finances
        .monthly_expenses_usd
        .filter(|e| e.is_finite() && *e > 0.0)
}

/// Whether the savings track shows the legal-emergency line: the household turned it on and the
/// arrest row is in its register.
pub(crate) fn legal_line_shown(household: &PlanInput, risks: &Risks) -> bool {
    household.dials.legal_opt_in
        && risks
            .register
            .get(&HazardId::ArrestOrDetention)
            .is_some_and(|r| *r > 0.0)
        && target_months(risks).is_some()
}

/// Citation ids behind numbers the savings track's own sentences print (beyond the income
/// bucket's): the legal line's bail figure when it is shown.
pub(crate) fn citations(household: &PlanInput, risks: &Risks) -> Vec<CitationId> {
    if legal_line_shown(household, risks) {
        vec![CitationId::from(LEGAL_COST_CITATION)]
    } else {
        Vec::new()
    }
}

/// The first savings step (`Plan::first_milestone`): one month of expenses or $500, whichever is
/// smaller, and the month the supplies budget, freed when the supplies plan is done (`stopped`),
/// reaches it. `None` when the goal or the step is already saved, when there is no monthly money
/// or the supplies plan never finishes (no saving pace), and when expenses are unknown but some
/// savings exist (whether $500 is reached cannot be told). With expenses unknown and nothing
/// saved, the step is $500 and `months` is 0.
pub(crate) fn first_milestone(
    household: &PlanInput,
    risks: &Risks,
    stopped: Option<u16>,
) -> Option<SavingsMilestone> {
    let goal = target_months(risks)?;
    let f = &household.finances;
    let current = f.emergency_fund_months.max(0.0);
    if current >= goal {
        return None;
    }
    let start = stopped?;
    let monthly = f.monthly_budget_usd.max(0.0);
    if monthly <= 0.0 {
        return None;
    }
    let (usd, months, saved) = match expenses(household) {
        Some(e) => {
            let usd = e.min(FIRST_MILESTONE_USD);
            (usd, usd / e, current * e)
        }
        None if current <= 0.0 => (FIRST_MILESTONE_USD, 0.0, 0.0),
        None => return None,
    };
    if saved + 0.005 >= usd {
        return None;
    }
    let months_needed = (f64::from(usd - saved) / f64::from(monthly) - 1e-9)
        .ceil()
        .max(1.0);
    Some(SavingsMilestone {
        months: (months * 100.0).round() / 100.0,
        usd,
        by_month: start.saturating_add(months_needed.min(f64::from(u16::MAX)) as u16),
    })
}

pub(crate) fn track(
    household: &PlanInput,
    risks: &Risks,
    stopped: Option<u16>,
) -> Option<SavingsTrack> {
    let a = risks.assessments.get(&BucketId::Income)?;
    let target_months = target_months(risks)?;
    let f = &household.finances;
    let current = f.emergency_fund_months.max(0.0);
    let expenses = expenses(household);
    let gap_months = (target_months - current).max(0.0);
    let monthly = f.monthly_budget_usd.max(0.0);
    let suggestion = if gap_months > 0.0 && stopped.is_some() {
        monthly
    } else {
        0.0
    };

    let mut why: Vec<String> = Vec::new();
    if let Some(s) = a.frequency_sentences.first() {
        why.push(s.clone());
    }
    let goal = match expenses {
        Some(e) => format!(
            "The goal is about {} of expenses (about {}); you have {} saved.",
            months_text(target_months),
            dollars(f64::from(target_months * e)),
            months_text(current)
        ),
        None => format!(
            "The goal is about {} of expenses; you have {} saved. Add your monthly expenses to see \
             it in dollars.",
            months_text(target_months),
            months_text(current)
        ),
    };
    why.push(goal);
    if gap_months <= 0.0 {
        why.push("You already have enough saved for this goal.".into());
    } else if let (Some(month), true) = (stopped, suggestion > 0.0) {
        let mut s = format!(
            "Your supplies plan is done by month {month}. After that, your {} a month for supplies \
             could go here",
            dollars(f64::from(monthly))
        );
        if let Some(e) = expenses {
            let months_needed = f64::from(gap_months * e) / f64::from(monthly);
            s.push_str(&format!(
                ", reaching the goal in {}",
                duration_text(months_needed)
            ));
        }
        s.push('.');
        why.push(s);
        if let Some(steps) = steps_sentence(household, risks, month) {
            why.push(steps);
        }
    } else if monthly <= 0.0 {
        why.push("Start small: any amount set aside each month helps.".into());
    } else {
        why.push("Once your supplies plan is done, the same money can go here.".into());
    }
    why.push("This is a savings goal, kept separate from the supplies budget.".into());
    if legal_line_shown(household, risks) {
        why.push(legal_sentence());
    }

    Some(SavingsTrack {
        target_months,
        target_usd: expenses.map_or(0.0, |e| target_months * e),
        current_months: current,
        monthly_suggestion_usd: suggestion,
        why: why.join(" "),
    })
}

/// "The first step, $500, comes by month 49; three months of expenses, about $12,600, in about
/// 17 years." Only the steps still ahead are named.
fn steps_sentence(household: &PlanInput, risks: &Risks, start: u16) -> Option<String> {
    let f = &household.finances;
    let monthly = f64::from(f.monthly_budget_usd.max(0.0));
    let current = f.emergency_fund_months.max(0.0);
    let first = first_milestone(household, risks, Some(start)).map(|m| {
        let what = if m.months >= 0.995 {
            "one month of expenses".to_owned()
        } else {
            dollars(f64::from(m.usd))
        };
        format!("The first step, {what}, comes by month {}", m.by_month)
    });
    let goal = target_months(risks)?;
    let three = expenses(household).and_then(|e| {
        if goal <= THREE_MONTH_POINT || current >= THREE_MONTH_POINT || monthly <= 0.0 {
            return None;
        }
        let usd = f64::from(THREE_MONTH_POINT * e);
        let months = ((usd - f64::from(current * e)) / monthly - 1e-9)
            .ceil()
            .max(1.0);
        let at = f64::from(start) + months;
        let when = if at <= 36.0 {
            format!("by month {}", at as u32)
        } else {
            format!("in {}", duration_text(at))
        };
        Some(format!(
            "three months of expenses, about {}, {when}",
            dollars(usd)
        ))
    });
    match (first, three) {
        (Some(a), Some(b)) => Some(format!("{a}; {b}.")),
        (Some(a), None) => Some(format!("{a}.")),
        (None, Some(b)) => Some(format!("{}.", upper_first(&b))),
        (None, None) => None,
    }
}

/// The legal-emergency line (`Dials::legal_opt_in`), apart from the months of income.
fn legal_sentence() -> String {
    format!(
        "Apart from these months: an arrest in the household can mean paying bail and a lawyer. \
         Many people are released without bail and amounts vary widely, but in the largest \
         counties the middle amount set in felony cases is about {}. \
         Money you can reach quickly, and someone in your trusted circle who knows where it is, \
         helps.",
        dollars(LEGAL_BAIL_MEDIAN_USD)
    )
}

fn upper_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn months_text(m: f32) -> String {
    let r = (f64::from(m) * 10.0).round() / 10.0;
    let n = if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r:.1}")
    };
    if n == "1" {
        "1 month".into()
    } else {
        format!("{n} months")
    }
}

fn duration_text(months: f64) -> String {
    if months < 23.5 {
        let n = months.ceil().max(1.0) as i64;
        if n == 1 {
            "about a month".into()
        } else {
            format!("about {n} months")
        }
    } else {
        format!("about {} years", (months / 12.0).round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_durations() {
        assert_eq!(months_text(3.8), "3.8 months");
        assert_eq!(months_text(4.0), "4 months");
        assert_eq!(months_text(1.0), "1 month");
        assert_eq!(duration_text(0.4), "about a month");
        assert_eq!(duration_text(7.2), "about 8 months");
        assert_eq!(duration_text(231.0), "about 19 years");
    }

    #[test]
    fn the_legal_sentence_cites_one_figure_with_its_range_in_words() {
        let s = legal_sentence();
        assert!(s.contains("$10,000"), "{s}");
        assert!(s.contains("released without bail"), "{s}");
        assert!(s.starts_with("Apart from these months"), "{s}");
    }
}
