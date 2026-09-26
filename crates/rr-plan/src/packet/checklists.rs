//! Section 5: checklists per step (free steps, three days, two weeks, one month ...) up to the
//! step recommended for the household's risks, the get-home bag per commuter, and
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
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    out.push("## Checklists".to_owned());
    out.push(String::new());
    out.push(
        "Tick these off as you go: one list per step, up to the step that is enough for your \
         risks, with what your household needs. The free steps are under Your plan."
            .to_owned(),
    );
    out.push(String::new());

    // Items per tier, merged across months (quantities add up), in plan order.
    let mut by_tier: BTreeMap<TierId, Vec<Entry>> = BTreeMap::new();
    // What the household has (listed or assumed) is in the summary, not on a list to tick.
    for (_, i) in cx.steps().into_iter().filter(|(_, i)| !i.done) {
        let list = by_tier.entry(i.tier).or_default();
        match list.iter_mut().find(|e| e.id == i.item_id) {
            Some(e) => {
                e.qty += f64::from(i.quantity);
                e.done &= i.done;
            }
            None => list.push(Entry {
                id: i.item_id.clone(),
                name: i.name.clone(),
                qty: f64::from(i.quantity),
                unit: i.unit.clone(),
                done: i.done,
                free: i.kind == PlanItemKind::FreeAction,
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
            out.push(format!("- [{tick}] {}{amount}", md(&e.name)));
        }
        out.push(String::new());
    }

    // The get-home bag, per commuter.
    let walks = &a.consequence.get_home.commuters;
    if !walks.is_empty() {
        out.push("### Get-home bag for each person who commutes".to_owned());
        out.push(String::new());
        for w in walks {
            let n = w.person + 1;
            let lines: Vec<&rr_supply::SizedLine> = a
                .lines
                .iter()
                .filter(|l| l.line.id.ends_with(&format!(".person_{n}")))
                .collect();
            let walk = rr_consequence::words::days_phrase(w.walk_hours / 24.0);
            let walk = if walk.starts_with("about ") {
                walk
            } else {
                format!("about {walk}")
            };
            out.push(format!(
                "**Person {n}**: a {} trip, {walk} on foot.",
                rr_consequence::words::distance_adjective(w.distance_km)
            ));
            out.push(String::new());
            // The bag line, without the trip the heading above already gives; then the water
            // and snacks for the walk in one line.
            let mut from_home: Vec<String> = Vec::new();
            let mut cites: Vec<&rr_types::CitationId> = Vec::new();
            for l in lines {
                if l.kind == rr_supply::LineKind::Need {
                    out.push(format!(
                        "- [ ] {}{}",
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
            if !from_home.is_empty() {
                out.push(format!(
                    "- [ ] For the walk, from home: {}.{}",
                    text::join_and(&from_home),
                    cite_all(cites)
                ));
            }
            out.push(String::new());
        }
    }

    // Hazard-specific extras the budget does not measure.
    let extras: Vec<&rr_types::Item> = a
        .offers
        .extras
        .iter()
        .filter(|id| !cx.in_plan(id.as_str()))
        .filter_map(|id| cx.item(id.as_str()))
        .collect();
    if !extras.is_empty() {
        out.push("### Extras for the hazards you face".to_owned());
        out.push(String::new());
        out.push(
            "These help with one hazard rather than a whole need, so they sit outside the \
             budget. Consider them once the steps above are done."
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
