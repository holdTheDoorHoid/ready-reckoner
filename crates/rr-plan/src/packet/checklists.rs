//! Section 5: checklists per step (free steps, three days, two weeks, one month ...), each with
//! its tier guidance block, the get-home bag per commuter, and hazard-specific extras that sit
//! outside the budget.

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
        "Tick these off as you go. Each list is one step, in order. Quantities are what your \
         household needs for the step."
            .to_owned(),
    );
    out.push(String::new());

    // Items per tier, merged across months (quantities add up), in plan order.
    let mut by_tier: BTreeMap<TierId, Vec<Entry>> = BTreeMap::new();
    for (_, i) in cx.steps() {
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
    for (tier, items) in &by_tier {
        let title = match tier {
            TierId::Now => "Free steps".to_owned(),
            other => other.name().to_owned(),
        };
        out.push(format!("### {title}"));
        out.push(String::new());
        if let Some(g) = cx.blocks_for(&format!("tier:{tier}")).first() {
            out.push(cx.guidance(g, None, None));
            out.push(String::new());
        }
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
            out.push(format!(
                "**Person {n}**: a {} trip, about {} on foot.",
                rr_consequence::words::distance_adjective(w.distance_km),
                rr_consequence::words::days_phrase(w.walk_hours / 24.0)
            ));
            out.push(String::new());
            for l in lines {
                out.push(format!(
                    "- [ ] {}{}",
                    md(&l.line.plain),
                    cite_all(&l.line.citations)
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
                "- **{}** (usually {} per {}): {}{}",
                md(&it.name),
                text::band(
                    f64::from(it.price_band_usd.low),
                    f64::from(it.price_band_usd.high)
                ),
                md(&it.price_band_usd.per),
                md(&it.spec),
                cite_all(&it.citations)
            ));
        }
        out.push(String::new());
    }
}
