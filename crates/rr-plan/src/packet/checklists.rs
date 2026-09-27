//! Section: checklists. The free steps from month 2 on, then one list per step (three days, two
//! weeks, one month ...) up to the step recommended for the household's risks, each line with its
//! quantity added up across months and the month the plan gets to it (packet v2: from month 2
//! the plan is these lists, so each purchase prints once); the get-home bag per commuter; and
//! hazard-specific extras that sit outside the budget. Why each step matters is the app's Learn
//! view; the lists here are for ticking off.

use std::collections::BTreeMap;

use rr_types::{ItemId, PlanItemKind, TierId};

use super::text::{self, md};
use super::{Ctx, cite_all};

/// One item on a step's checklist, merged across the months it is bought in.
struct Entry {
    id: ItemId,
    name: String,
    qty: f64,
    unit: String,
    done: bool,
    free: bool,
    /// The first and last month it is bought in.
    months: (u16, u16),
}

/// "(month 6)", "(months 1–4)".
fn months_words((first, last): (u16, u16)) -> String {
    if first == last {
        format!("(month {first})")
    } else {
        format!("(months {first}–{last})")
    }
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    out.push("## Checklists".to_owned());
    out.push(String::new());
    out.push(
        "One list per step, up to the one your risks need, with each thing's month. This \
         month's and next month's steps are under Your plan."
            .to_owned(),
    );
    out.push(String::new());

    // The free steps from month 2 on (the first two months' are under Your plan); decisions are
    // under Documents and money.
    let free_later: Vec<(u16, &rr_types::PlanItem)> = cx
        .steps()
        .into_iter()
        .filter(|(m, i)| {
            *m >= super::DETAIL_MONTHS
                && i.kind == PlanItemKind::FreeAction
                && !i.done
                && !i.decision
        })
        .collect();
    if !free_later.is_empty() {
        out.push("### Free steps".to_owned());
        out.push(String::new());
        // One line a month: "Month 2: keep your vehicle ready; legal readiness; ...".
        let mut months: Vec<(u16, Vec<String>)> = Vec::new();
        for (m, i) in free_later {
            let name = text::lower_first(text::short_name(&i.name));
            match months.last_mut() {
                Some((last, names)) if *last == m => names.push(name),
                _ => months.push((m, vec![name])),
            }
        }
        for (m, names) in months {
            out.push(format!("- [ ] Month {m}: {}.", md(&names.join("; "))));
        }
        out.push(String::new());
    }

    // Items per tier, merged across months (quantities add up), in plan order.
    let mut by_tier: BTreeMap<TierId, Vec<Entry>> = BTreeMap::new();
    // What the household has (listed or assumed) is in the summary, not on a list to tick.
    for (m, i) in cx.steps().into_iter().filter(|(_, i)| !i.done) {
        let list = by_tier.entry(i.tier).or_default();
        match list.iter_mut().find(|e| e.id == i.item_id) {
            Some(e) => {
                e.qty += f64::from(i.quantity);
                e.done &= i.done;
                e.months.1 = e.months.1.max(m);
            }
            None => list.push(Entry {
                id: i.item_id.clone(),
                name: i.name.clone(),
                qty: f64::from(i.quantity),
                unit: i.unit.clone(),
                done: i.done,
                free: i.kind == PlanItemKind::FreeAction,
                months: (m, m),
            }),
        }
    }
    // The free steps are the first items of Your plan, each with its own box to tick, so the
    // lists here start at the first step that costs money.
    let recommended = rr_supply::tier_recommended(&a.buckets);
    for (tier, items) in by_tier
        .iter()
        .filter(|(t, _)| **t != TierId::Now && **t <= recommended)
    {
        out.push(format!("### {}", tier.name()));
        out.push(String::new());
        for e in items {
            let tick = if e.done { "x" } else { " " };
            let amount = if e.free {
                String::new()
            } else {
                let q = text::quantity(e.qty, &e.unit);
                if q.is_empty() {
                    String::new()
                } else {
                    format!(": {}", md(&q))
                }
            };
            out.push(format!(
                "- [{tick}] {}{amount} {}",
                md(text::short_name(&e.name)),
                months_words(e.months)
            ));
        }
        out.push(String::new());
    }

    // The get-home bag, per commuter: the trip, the bag line (without the trip the person's line
    // already gives) and the water and snacks for the walk in one line. When every commuter's bag
    // line is the same (two people who both keep theirs in the car), it prints once, for each
    // person, above them.
    let walks = &a.consequence.get_home.commuters;
    if !walks.is_empty() {
        out.push("### Get-home bag for each person who commutes".to_owned());
        out.push(String::new());
        struct Commuter {
            trip: String,
            bag: Vec<String>,
            walk: Option<String>,
        }
        let commuters: Vec<Commuter> = walks
            .iter()
            .map(|w| {
                let n = w.person + 1;
                let walk = rr_consequence::words::days_phrase(w.walk_hours / 24.0);
                let walk = if walk.starts_with("about ") {
                    walk
                } else {
                    format!("about {walk}")
                };
                let trip = format!(
                    "**Person {n}**: a {} trip, {walk} on foot.",
                    rr_consequence::words::distance_adjective(w.distance_km)
                );
                let mut bag: Vec<String> = Vec::new();
                let mut from_home: Vec<String> = Vec::new();
                let mut cites: Vec<&rr_types::CitationId> = Vec::new();
                for l in a
                    .lines
                    .iter()
                    .filter(|l| l.line.id.ends_with(&format!(".person_{n}")))
                {
                    if l.kind == rr_supply::LineKind::Need {
                        bag.push(format!(
                            "{}{}",
                            md(after_first_sentence(&l.line.plain)),
                            cite_all(&l.line.citations)
                        ));
                    } else if l.quantity > 0.0 {
                        let what = if l.line.unit == "kcal" {
                            "of snacks"
                        } else {
                            "of water"
                        };
                        from_home.push(format!(
                            "{} {what}",
                            md(&text::quantity(l.quantity, &l.line.unit))
                        ));
                        cites.extend(l.line.citations.iter());
                    }
                }
                let walk = (!from_home.is_empty()).then(|| {
                    format!(
                        "- [ ] For the walk, from home: {}.{}",
                        text::join_and(&from_home),
                        cite_all(cites)
                    )
                });
                Commuter { trip, bag, walk }
            })
            .collect();
        let shared = commuters.len() > 1
            && !commuters[0].bag.is_empty()
            && commuters.windows(2).all(|p| p[0].bag == p[1].bag);
        if shared {
            for b in &commuters[0].bag {
                out.push(format!("- [ ] For each person: {}", text::lower_first(b)));
            }
            out.push(String::new());
        }
        for c in &commuters {
            out.push(c.trip.clone());
            out.push(String::new());
            if !shared {
                for b in &c.bag {
                    out.push(format!("- [ ] {b}"));
                }
            }
            if let Some(w) = &c.walk {
                out.push(w.clone());
            }
            out.push(String::new());
        }
    }

    // Hazard-specific extras the budget does not measure. Gear for rare catastrophes (a radiation
    // meter, a shielded bag) prints only when the household opted in to that group
    // (`dials.rare_catastrophic_opt_in`, CONTENT_STANDARDS §3; review S4, RR-P11).
    let opted_in = a.input.dials.rare_catastrophic_opt_in;
    let extras: Vec<&rr_types::Item> = a
        .offers
        .extras
        .iter()
        .filter(|id| !cx.in_plan(id.as_str()))
        .filter_map(|id| cx.item(id.as_str()))
        .filter(|it| opted_in || !it.rare_catastrophic)
        .collect();
    if !extras.is_empty() {
        out.push("### Extras for the hazards you face".to_owned());
        out.push(String::new());
        out.push(
            "These help with one hazard, not a whole need, so they sit outside the budget."
                .to_owned(),
        );
        out.push(String::new());
        for it in extras {
            out.push(format!(
                "- {} (usually {} per {}){}",
                md(&it.name),
                text::band(
                    f64::from(it.price_band_usd.low),
                    f64::from(it.price_band_usd.high)
                ),
                md(&it.price_band_usd.per),
                cite_all(&it.citations)
            ));
        }
        out.push(String::new());
    }
}

/// Everything after the first sentence, or the whole text when it is one sentence.
fn after_first_sentence(s: &str) -> &str {
    match s.find(". ") {
        Some(i) if i + 2 < s.len() => &s[i + 2..],
        _ => s,
    }
}
