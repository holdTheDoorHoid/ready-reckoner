//! The savings track for the `income` bucket (DESIGN §4.4, §4.7): months of expenses to have
//! saved, never funded from the supplies budget. Surplus goes here once the supplies plan runs out
//! of things worth buying.
//!
//! The suggested monthly amount is the household's own supplies budget, freed when the supplies
//! plan is done; no saving pace is invented (FINRA and the St. Louis Fed give a 3–6 month goal but
//! no pace; the CFPB says to start small).

use rr_types::{BucketId, PlanInput, SavingsTrack, Target};

use crate::explain::dollars;
use crate::input::Risks;

pub(crate) fn track(
    household: &PlanInput,
    risks: &Risks,
    stopped: Option<u16>,
) -> Option<SavingsTrack> {
    let a = risks.assessments.get(&BucketId::Income)?;
    let Target::Months {
        value: target_months,
        ..
    } = a.target
    else {
        return None;
    };
    if !(target_months.is_finite() && target_months > 0.0) {
        return None;
    }
    let f = &household.finances;
    let current = f.emergency_fund_months.max(0.0);
    let expenses = f.monthly_expenses_usd.filter(|e| e.is_finite() && *e > 0.0);
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
    } else if monthly <= 0.0 {
        why.push("Start small: any amount set aside each month helps.".into());
    } else {
        why.push("Once your supplies plan is done, the same money can go here.".into());
    }
    why.push("This is a savings goal, kept separate from the supplies budget.".into());

    Some(SavingsTrack {
        target_months,
        target_usd: expenses.map_or(0.0, |e| target_months * e),
        current_months: current,
        monthly_suggestion_usd: suggestion,
        why: why.join(" "),
    })
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
}
