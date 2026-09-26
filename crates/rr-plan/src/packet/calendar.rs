//! Section 9: the maintenance calendar. Rotation and check dates for everything in the plan,
//! computed from the planning date and the month each item enters the plan; short intervals as
//! repeating rows; drills; and the yearly review.

use std::collections::BTreeMap;

use rr_types::Date;

use super::text::{self, md};
use super::{Ctx, cite};

/// Intervals up to this many months are listed as repeating rows, not dates.
const REPEATING_MAX_MONTHS: u16 = 3;

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let start = a.input.planning_date;
    out.push("## Maintenance calendar".to_owned());
    out.push(String::new());
    out.push(format!(
        "Supplies only help if they still work. Dates count from when each item enters your \
         plan; move them if you buy things earlier or later.{}",
        cite("ready_gov_kit")
    ));
    out.push(String::new());

    // First month each item is in the plan.
    let mut first: BTreeMap<String, u16> = BTreeMap::new();
    for (m, i) in cx.steps() {
        first.entry(i.item_id.as_str().to_owned()).or_insert(m);
    }
    let mut repeating: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    let mut dated: BTreeMap<Date, Vec<String>> = BTreeMap::new();
    for (id, month) in &first {
        let Some(item) = cx.item(id) else {
            continue;
        };
        let Some(m) = item.maintenance else {
            continue;
        };
        let name = text::lower_first(&item.name);
        for (interval, verb) in [
            (m.rotate_months, "Use and restock"),
            (m.check_months, "Check"),
        ] {
            let Some(every) = interval.filter(|n| *n > 0) else {
                continue;
            };
            if every <= REPEATING_MAX_MONTHS {
                repeating
                    .entry(every)
                    .or_default()
                    .push(format!("{verb}: {}", md(&name)));
            } else {
                let due = text::month_start(start, month + every);
                dated.entry(due).or_default().push(format!(
                    "{verb}: {} (then every {})",
                    md(&name),
                    every_phrase(every)
                ));
            }
        }
    }
    let review = start.add_months(12).unwrap_or(start);
    dated.entry(review).or_default().push(
        "Yearly review: go through this plan again, update your household's answers, check the \
         documents and contact cards, and start a new calendar"
            .to_owned(),
    );

    out.push("| When | What |".to_owned());
    out.push("| --- | --- |".to_owned());
    for (every, what) in &repeating {
        out.push(format!(
            "| Every {} | {} |",
            every_phrase(*every),
            what.join("; ")
        ));
    }
    for (date, what) in &dated {
        out.push(format!("| {} | {} |", text::date(*date), what.join("; ")));
    }
    out.push(String::new());
}

fn every_phrase(months: u16) -> String {
    match months {
        1 => "month".to_owned(),
        12 => "year".to_owned(),
        m if m % 12 == 0 => format!("{} years", m / 12),
        m => format!("{m} months"),
    }
}
