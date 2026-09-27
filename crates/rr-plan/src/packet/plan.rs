//! Section: your plan. The free steps to do now; this month and next month in detail, with
//! quantities, prices and, where prices spread widely, the usual price band, and the decisions
//! due that month grouped on one line; a pointer to the checklists for the months after that
//! (each line there carries its month, so every purchase is listed once); savings toward bigger
//! items; when every need is covered; the guardrail warnings, including the deferred list when
//! the plan runs long; and the rare-event allowance, in the allocator's words.

use rr_types::{PlanItem, PlanItemKind, PlanMonth, WarningSeverity};

use super::text::{self, md};
use super::{Ctx, cite};

/// Months shown in detail: this month and next month (v0.1.1 showed six, v0.1.0 twelve). From
/// month 2 the plan is the checklists, each line with the month the plan gets to it, so the
/// packet lists each purchase once (packet v2: the page budget pays for the family plan, the
/// shelter plan, the forecast list and the recovery page).
pub const DETAIL_MONTHS: u16 = 2;

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

/// A free step by its short name (what it does is in the app, and its how-to where the packet
/// has one).
fn short_line(item: &PlanItem) -> String {
    let tick = if item.done { "x" } else { " " };
    format!("- [{tick}] {}", md(text::short_name(&item.name)))
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

/// The price band after the estimate, when it says something the estimate does not: prices
/// that spread more than a quarter either side of the middle ("about $12 (usually $3–20)");
/// a narrow band ("about $483", usually $479–488) is left to the app.
fn band_words(item: &PlanItem) -> String {
    let (low, high) = (
        f64::from(item.price_band.low),
        f64::from(item.price_band.high),
    );
    let est = f64::from(item.est_cost_usd).max(0.01);
    if high - low > 0.5 * est {
        format!(" (usually {})", text::band(low, high))
    } else {
        String::new()
    }
}

/// A plan line after month 0: free steps by name, purchases with quantity, cost and price band
/// (what each adds is the app's).
fn later_line(item: &PlanItem) -> String {
    match item.kind {
        PlanItemKind::FreeAction => short_line(item),
        PlanItemKind::Purchase if !item.done => format!(
            "- [ ] **{}**: {}, about {}{}",
            md(&item.name),
            md(&text::quantity(f64::from(item.quantity), &item.unit)),
            text::usd(f64::from(item.est_cost_usd)),
            band_words(item)
        ),
        _ => item_line(item),
    }
}

/// A decision's name without its "Decide:" opening, for the grouped line.
fn decision_name(name: &str) -> String {
    text::lower_first(name.strip_prefix("Decide: ").unwrap_or(name))
}

/// The month's decisions (insurance, ID and home repairs: `PlanItem::decision`) as one grouped
/// line, not one line each (budget2: month 1 holds up to 13 of them).
fn decisions_line(m: &PlanMonth) -> Option<String> {
    let names: Vec<String> = m
        .items
        .iter()
        .filter(|i| i.decision && !i.done)
        .map(|i| decision_name(&i.name))
        .collect();
    (!names.is_empty()).then(|| {
        format!(
            "- [ ] **Decide this month** (see Documents and money): {}.",
            md(&names.join("; "))
        )
    })
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
        "{budget} Free steps come first, then what protects you most for each dollar, water, \
         medicine and safety first, until each need reaches the step that is enough for it.{}",
        cite("prior_harm_weights")
    ));
    out.push(String::new());

    let first = plan.months.first();
    let free0: Vec<&PlanItem> = first
        .map(|m| {
            m.items
                .iter()
                .filter(|i| i.kind == PlanItemKind::FreeAction && !i.done && !i.decision)
                .collect()
        })
        .unwrap_or_default();
    if !free0.is_empty() || first.and_then(decisions_line).is_some() {
        out.push(format!(
            "### Start now: free steps (from {})",
            text::date(a.input.planning_date)
        ));
        out.push(String::new());
        for i in free0 {
            out.push(short_line(i));
        }
        out.extend(first.and_then(decisions_line));
        out.push(String::new());
    }
    // The warning of each life-safety step, whatever month the step falls in.
    super::safety::write(out);

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
            out.push(later_line(i));
        }
    }
    out.push(String::new());

    if let Some(m) = plan.months.get(1) {
        out.push(format!("### Next month: {}", month_heading(cx, m)));
        out.push(String::new());
        let todo: Vec<&PlanItem> = m.items.iter().filter(|i| !i.done && !i.decision).collect();
        let decide = decisions_line(m);
        if todo.is_empty() && decide.is_none() {
            out.push("Nothing new this month.".to_owned());
        }
        for i in todo {
            out.push(later_line(i));
        }
        out.extend(decide);
        out.push(String::new());
    }

    // From month 2 the plan works through the checklists, each line with its month.
    if plan
        .months
        .iter()
        .any(|m| m.index >= DETAIL_MONTHS && m.items.iter().any(|i| !i.done))
    {
        out.push("### After that".to_owned());
        out.push(String::new());
        out.push(
            "From month 2 the plan follows Checklists, free steps first; each line gives its \
             month."
                .to_owned(),
        );
        out.push(String::new());
    }
    rare_allowance(cx, out);

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
                "Every need is covered by month {m} ({}). After that, keep up the maintenance \
                 calendar and put the same money toward savings.",
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
            // The bare-minimum warning's why names what still falls after three years (budget2's
            // deferred list).
            out.push(format!("- **{lead}: {}** {}", md(&w.message), md(&w.why)));
        }
        out.push(String::new());
    }
}

/// Purchases paid from the rare-event allowance, each with the allocator's own words (the family
/// it is for, the 1-in-1,000 threshold it passed, never a point estimate; budget2), only when the
/// household opted in and the plan buys something with it.
fn rare_allowance(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let lines: Vec<String> = cx
        .steps()
        .into_iter()
        .filter(|(_, i)| i.kind == PlanItemKind::Purchase && !i.done)
        .filter(|(_, i)| {
            cx.item(i.item_id.as_str())
                .is_some_and(|it| it.rare_catastrophic)
        })
        .map(|(m, i)| {
            format!(
                "- **{}** (month {m}, about {}): {}",
                md(&i.name),
                text::usd(f64::from(i.est_cost_usd)),
                md(&i.why)
            )
        })
        .collect();
    if lines.is_empty() {
        return;
    }
    out.push("### The rare-event allowance".to_owned());
    out.push(String::new());
    out.extend(lines);
    out.push(String::new());
}
