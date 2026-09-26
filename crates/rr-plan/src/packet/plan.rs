//! Section 4: your plan. The free steps to do now, this month and next month, then the first
//! year month by month with quantities, prices, price bands and what each step adds; the months
//! after the first year as one compact table; savings toward bigger items; when the plan is done;
//! and the guardrail warnings.

use rr_types::{PlanItem, PlanItemKind, PlanMonth, WarningSeverity};

use super::text::{self, md};
use super::{Ctx, cite};

/// Months shown in detail (months 0 to 11); later months go in a compact table.
pub const DETAIL_MONTHS: u16 = 12;

/// What a step adds, in one sentence: the first sentence of its "why", after the "Free." that
/// opens a free step's.
pub(crate) fn short_why(why: &str) -> String {
    let rest = why.trim().strip_prefix("Free.").unwrap_or(why).trim_start();
    let end = rest
        .char_indices()
        .find(|&(i, c)| c == '.' && rest[i + 1..].starts_with(' '))
        .map_or(rest.len(), |(i, _)| i + 1);
    rest[..end].trim().to_owned()
}

/// The item a savings deposit is for: the allocator names it "Save toward: <item>".
fn saving_for(name: &str) -> &str {
    name.strip_prefix("Save toward: ").unwrap_or(name)
}

/// A free step by name only (after the first month the reasons repeat).
fn short_line(item: &PlanItem) -> String {
    let tick = if item.done { "x" } else { " " };
    format!("- [{tick}] {} (free)", md(&item.name))
}

/// One plan line as a checklist entry, with what it adds.
pub(crate) fn item_line(item: &PlanItem) -> String {
    let why = md(&short_why(&item.why));
    match item.kind {
        PlanItemKind::FreeAction => {
            let tick = if item.done { "x" } else { " " };
            format!("- [{tick}] **{}.** {why}", md(&item.name))
        }
        PlanItemKind::Purchase if item.done => format!(
            "- [x] **{}**: {}. {why}",
            md(&item.name),
            md(&text::quantity(f64::from(item.quantity), &item.unit))
        ),
        PlanItemKind::Purchase => format!(
            "- [ ] **{}**: {}, about {} (usually {}). {why}",
            md(&item.name),
            md(&text::quantity(f64::from(item.quantity), &item.unit)),
            text::usd(f64::from(item.est_cost_usd)),
            text::band(
                f64::from(item.price_band.low),
                f64::from(item.price_band.high)
            )
        ),
        PlanItemKind::Reserve => format!(
            "- Set aside {} toward **{}**.",
            text::usd(f64::from(item.est_cost_usd)),
            md(&text::lower_first(saving_for(&item.name)))
        ),
    }
}

/// A plan line after month 0: free steps by name, everything else in full.
fn later_line(item: &PlanItem) -> String {
    if item.kind == PlanItemKind::FreeAction {
        short_line(item)
    } else {
        item_line(item)
    }
}

/// A plan line in months 2 to 11: what, how much and what it costs (this month's and next
/// month's lines also say what each step adds).
fn brief_line(item: &PlanItem) -> String {
    match item.kind {
        PlanItemKind::FreeAction => short_line(item),
        PlanItemKind::Purchase => format!(
            "- [ ] **{}**: {}, about {} (usually {})",
            md(&item.name),
            md(&text::quantity(f64::from(item.quantity), &item.unit)),
            text::usd(f64::from(item.est_cost_usd)),
            text::band(
                f64::from(item.price_band.low),
                f64::from(item.price_band.high)
            )
        ),
        PlanItemKind::Reserve => item_line(item),
    }
}

/// One month after the first year as a table row: what to buy or do, and what it costs.
fn table_row(cx: &Ctx<'_>, m: &PlanMonth) -> String {
    let start = text::month_start(cx.a.input.planning_date, m.index);
    let mut what: Vec<String> = Vec::new();
    let mut spend = 0.0_f64;
    for i in m.items.iter().filter(|i| !i.done) {
        match i.kind {
            PlanItemKind::FreeAction => what.push(format!("{} (free)", md(&i.name))),
            PlanItemKind::Purchase => {
                spend += f64::from(i.est_cost_usd);
                let q = text::quantity(f64::from(i.quantity), &i.unit);
                what.push(if q.is_empty() {
                    md(&i.name)
                } else {
                    format!("{}: {}", md(&i.name), md(&q))
                });
            }
            PlanItemKind::Reserve => {
                spend += f64::from(i.est_cost_usd);
                what.push(format!(
                    "save toward {}",
                    md(&text::lower_first(saving_for(&i.name)))
                ));
            }
        }
    }
    format!(
        "| {} ({}) | {} | {} |",
        m.index,
        text::month_year(start),
        what.join("; "),
        text::usd(spend)
    )
}

fn month_heading(cx: &Ctx<'_>, m: &PlanMonth) -> String {
    let start = text::month_start(cx.a.input.planning_date, m.index);
    let money = if m.budget_usd > 0.0 {
        format!(", {} to spend", text::usd(f64::from(m.budget_usd)))
    } else {
        String::new()
    };
    format!("Month {} (from {}){money}", m.index, text::date(start))
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let f = &a.input.finances;
    let monthly = f64::from(f.monthly_budget_usd);
    let one_off = f64::from(f.one_off_budget_usd);
    let plan = &a.budget.plan;
    out.push("## Your plan".to_owned());
    out.push(String::new());
    let budget = match (monthly > 0.0, one_off > 0.0) {
        (true, true) => format!(
            "Your budget is {} a month, plus {} once at the start.",
            text::usd(monthly),
            text::usd(one_off)
        ),
        (true, false) => format!("Your budget is {} a month.", text::usd(monthly)),
        (false, true) => format!("Your budget is {} once, at the start.", text::usd(one_off)),
        (false, false) => "You have set no money aside, so the plan is free steps.".to_owned(),
    };
    out.push(format!(
        "{budget} The plan does the free steps first. Then it buys whatever protects you most for \
         each dollar, month by month, and stops once each need reaches the step that is enough \
         for it. Water, medicine and safety come first within each step.{}",
        cite("prior_harm_weights")
    ));
    out.push(String::new());

    let first = plan.months.first();
    let free0: Vec<&PlanItem> = first
        .map(|m| {
            m.items
                .iter()
                .filter(|i| i.kind == PlanItemKind::FreeAction && !i.done)
                .collect()
        })
        .unwrap_or_default();
    if !free0.is_empty() {
        out.push(format!(
            "### Start now: free steps (from {})",
            text::date(a.input.planning_date)
        ));
        out.push(String::new());
        for i in free0 {
            out.push(format!("- [ ] {}", md(&i.name)));
        }
        out.push(String::new());
    }

    out.push("### This month".to_owned());
    out.push(String::new());
    // What the household already has (listed or assumed) is not a step to take.
    let bought0: Vec<&PlanItem> = first
        .map(|m| {
            m.items
                .iter()
                .filter(|i| i.kind != PlanItemKind::FreeAction && !i.done)
                .collect()
        })
        .unwrap_or_default();
    if bought0.is_empty() {
        let why = if one_off <= 0.0 && monthly > 0.0 {
            "Your monthly money starts next month, so this month is the free steps above."
        } else if one_off <= 0.0 {
            "This month is the free steps above."
        } else {
            "Nothing to buy yet: what you have and the free steps cover this month's step."
        };
        out.push(why.to_owned());
    } else {
        for i in bought0 {
            out.push(item_line(i));
        }
    }
    out.push(String::new());

    if let Some(m) = plan.months.get(1) {
        out.push(format!("### Next month: {}", month_heading(cx, m)));
        out.push(String::new());
        let todo: Vec<&PlanItem> = m.items.iter().filter(|i| !i.done).collect();
        if todo.is_empty() {
            out.push("Nothing new this month.".to_owned());
        }
        for i in todo {
            out.push(later_line(i));
        }
        out.push(String::new());
    }

    let later: Vec<&PlanMonth> = plan
        .months
        .iter()
        .skip(2)
        .filter(|m| m.index < DETAIL_MONTHS && m.items.iter().any(|i| !i.done))
        .collect();
    if !later.is_empty() {
        out.push("### Month by month".to_owned());
        out.push(String::new());
        for m in later {
            out.push(format!("**{}**", month_heading(cx, m)));
            out.push(String::new());
            for i in m.items.iter().filter(|i| !i.done) {
                out.push(brief_line(i));
            }
            out.push(String::new());
        }
    }
    let after: Vec<&PlanMonth> = plan
        .months
        .iter()
        .filter(|m| m.index >= DETAIL_MONTHS && m.items.iter().any(|i| !i.done))
        .collect();
    if !after.is_empty() {
        out.push("### After the first year".to_owned());
        out.push(String::new());
        out.push("| Month | What | Spend |".to_owned());
        out.push("| --- | --- | --- |".to_owned());
        for m in after {
            out.push(table_row(cx, m));
        }
        out.push(String::new());
    }

    // Funds still open when the plan ends (the rest were spent on their item).
    let open: Vec<&rr_types::SavingsEnvelope> = plan
        .envelopes
        .iter()
        .filter(|e| e.saved_usd + 0.005 < e.needed_usd)
        .filter(|e| {
            !cx.steps()
                .iter()
                .any(|(_, i)| i.item_id == e.item_id && i.kind == PlanItemKind::Purchase && !i.done)
        })
        .collect();
    if !open.is_empty() {
        out.push("### Saving toward bigger items".to_owned());
        out.push(String::new());
        for e in open {
            let name = cx
                .item(e.item_id.as_str())
                .map_or(e.item_id.as_str().to_owned(), |i| i.name.clone());
            out.push(format!(
                "- **{}**: {} saved of {}.",
                md(&name),
                text::usd(f64::from(e.saved_usd)),
                text::usd(f64::from(e.needed_usd))
            ));
        }
        out.push(String::new());
    }

    out.push("### When you are done".to_owned());
    out.push(String::new());
    match plan.done_month {
        Some(m) => {
            let date = text::month_start(a.input.planning_date, m);
            out.push(format!(
                "By month {m} ({}) every need is covered to the step that is enough for your \
                 risks. After that you are done: keep up the maintenance calendar below, and \
                 put the same money toward your savings goal if you have one.",
                text::month_year(date)
            ));
        }
        None if monthly <= 0.0 && one_off <= 0.0 => {
            out.push(
                "With no money set aside, the plan is the free steps above, and they still cover \
                 a lot. If you can set aside even a few dollars a month, the plan starts with \
                 water and light."
                    .to_owned(),
            );
        }
        None => {
            out.push(
                "At this budget the plan runs past ten years. The early months do the most good, \
                 because the first days of each need are the ones you are most likely to use."
                    .to_owned(),
            );
        }
    }
    // The savings goal itself is under Documents and money.
    out.push(String::new());

    // The summary already lists the assumed basics, and the targets the cliffs.
    let watch: Vec<&rr_types::Warning> = a
        .warnings
        .iter()
        .filter(|w| {
            !w.id.starts_with("cliff_") && w.id != "citation_missing" && w.id != "assumed_basics"
        })
        .collect();
    if !watch.is_empty() {
        out.push("### Things to watch".to_owned());
        out.push(String::new());
        for w in watch {
            let lead = match w.severity {
                WarningSeverity::Warn => "Worth acting on",
                WarningSeverity::Note => "Worth knowing",
            };
            out.push(format!("- **{lead}: {}** {}", md(&w.message), md(&w.why)));
        }
        out.push(String::new());
    }
}
