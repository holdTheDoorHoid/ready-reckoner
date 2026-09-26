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
        "Dates count from when each item enters your plan; move them if you buy earlier or \
         later.{}",
        cite("ready_gov_kit")
    ));
    out.push(String::new());

    // First month each item is in the plan.
    let mut first: BTreeMap<String, u16> = BTreeMap::new();
    for (m, i) in cx.steps() {
        first.entry(i.item_id.as_str().to_owned()).or_insert(m);
    }
    // Each entry: (what to do, how often in months, the item). A row names each "what" once:
    // "Check: a; b. Use and restock: c", and a dated row "Check, then every 6 months: a; b".
    let mut repeating: BTreeMap<u16, Vec<(&str, String)>> = BTreeMap::new();
    let mut dated: BTreeMap<Date, Vec<(&str, u16, String)>> = BTreeMap::new();
    for (id, month) in &first {
        let Some(item) = cx.item(id) else {
            continue;
        };
        let Some(m) = item.maintenance else {
            continue;
        };
        let name = md(&text::lower_first(&item.name));
        for (interval, verb) in [
            (m.check_months, "Check"),
            (m.rotate_months, "Use and restock"),
        ] {
            let Some(every) = interval.filter(|n| *n > 0) else {
                continue;
            };
            if every <= REPEATING_MAX_MONTHS {
                repeating
                    .entry(every)
                    .or_default()
                    .push((verb, name.clone()));
            } else {
                let due = text::month_start(start, month + every);
                dated
                    .entry(due)
                    .or_default()
                    .push((verb, every, name.clone()));
            }
        }
    }
    let review = start.add_months(12).unwrap_or(start);

    out.push("| When | What |".to_owned());
    out.push("| --- | --- |".to_owned());
    for (every, what) in &repeating {
        let mut groups: Vec<(&str, Vec<&str>)> = Vec::new();
        for (verb, name) in what {
            match groups.iter_mut().find(|(v, _)| v == verb) {
                Some((_, names)) => names.push(name),
                None => groups.push((verb, vec![name])),
            }
        }
        let cell: Vec<String> = groups
            .iter()
            .map(|(verb, names)| format!("{verb}: {}", names.join("; ")))
            .collect();
        out.push(format!(
            "| Every {} | {} |",
            every_phrase(*every),
            cell.join(". ")
        ));
    }
    let dates: std::collections::BTreeSet<Date> = dated.keys().copied().chain([review]).collect();
    for date in dates {
        let mut groups: Vec<((&str, u16), Vec<&str>)> = Vec::new();
        for (verb, every, name) in dated.get(&date).map(Vec::as_slice).unwrap_or_default() {
            match groups.iter_mut().find(|(k, _)| *k == (*verb, *every)) {
                Some((_, names)) => names.push(name),
                None => groups.push(((verb, *every), vec![name])),
            }
        }
        let mut cell: Vec<String> = groups
            .iter()
            .map(|((verb, every), names)| {
                format!(
                    "{verb}, then every {}: {}",
                    every_phrase(*every),
                    names.join("; ")
                )
            })
            .collect();
        if date == review {
            cell.push(
                "Yearly review: go through this plan again, update your household's answers, \
                 check the documents and contact cards, and start a new calendar"
                    .to_owned(),
            );
        }
        out.push(format!("| {} | {} |", text::date(date), cell.join(". ")));
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
