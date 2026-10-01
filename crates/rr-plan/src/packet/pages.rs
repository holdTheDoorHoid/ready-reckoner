//! The v2 packet pages the binder still prints from their guidance blocks: access and functional
//! needs (`topic:access_needs`, CMIST) when anyone in the household has one, "After a disaster:
//! the first 30 days" (`after:first_30_days`) with the county's declarations, and the
//! long-horizon section ("If it lasts for months", `topic:long_horizon`) when the plan has one.
//! Each block's conditional spans are kept or dropped for this household; the text is the
//! block's, and the code only joins it with the household's own facts.

use rr_types::{AccessNeed, CitationId, PlanItemKind};

use super::text::{self, md};
use super::{Ctx, cite, cite_all};

/// A guidance block's paragraphs for this household, in order (conditional spans applied).
fn block_paragraphs(cx: &Ctx<'_>, target: &str) -> Vec<String> {
    cx.blocks_for(target)
        .into_iter()
        .next()
        .map(|g| super::paragraphs(&cx.guidance(g, None, None)))
        .unwrap_or_default()
}

fn push_paragraphs(out: &mut Vec<String>, paras: Vec<String>) {
    for p in paras {
        out.push(p);
        out.push(String::new());
    }
}

/// An access or functional need in words, for "person 3 (older adult): …".
fn need_words(n: AccessNeed) -> &'static str {
    match n {
        AccessNeed::Hearing => "deaf or hard of hearing",
        AccessNeed::Vision => "blind or low vision",
        AccessNeed::LimitedEnglish => "speaks or reads little English",
        AccessNeed::Cognitive => "memory or understanding affected",
        AccessNeed::Supervision => "needs someone with them",
        AccessNeed::ServiceAnimal => "has a service animal",
        AccessNeed::Dialysis => "needs dialysis",
        AccessNeed::HomeHealth => "gets home health care",
    }
}

/// `## Access and functional needs` (CMIST; review RR-P08), only when someone has one.
pub(crate) fn access_needs(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let people = &cx.a.input.people;
    let who: Vec<String> = people
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.access_needs.is_empty())
        .map(|(i, p)| {
            let needs: Vec<String> = p
                .access_needs
                .iter()
                .map(|n| need_words(*n).to_owned())
                .collect();
            format!(
                "{}: {}",
                crate::binder::person_title(i, p),
                text::join_and(&needs)
            )
        })
        .collect();
    if who.is_empty() {
        return;
    }
    out.push("## Access and functional needs".to_owned());
    out.push(String::new());
    out.push(format!("**In your household:** {}.", md(&who.join("; "))));
    out.push(String::new());
    push_paragraphs(out, block_paragraphs(cx, "topic:access_needs"));
}

/// `## After a disaster: the first 30 days` (review RR-P06), with the county's federal disaster
/// declarations in the last five years (`PlanOutput::recovery`).
pub(crate) fn after_disaster(cx: &Ctx<'_>, out: &mut Vec<String>) {
    out.push("## After a disaster: the first 30 days".to_owned());
    out.push(String::new());
    if let Some(n) = cx.a.county.declarations.as_ref().map(|d| d.last_5yr) {
        out.push(format!(
            "**Your county.** {} had {} in the last five full years.{}",
            md(&super::summary::place(cx)),
            match n {
                0 => "no federal major-disaster declarations".to_owned(),
                1 => "one federal major-disaster declaration".to_owned(),
                n => format!("{n} federal major-disaster declarations"),
            },
            cite(RECOVERY_SOURCE)
        ));
        out.push(String::new());
    }
    push_paragraphs(out, block_paragraphs(cx, "after:first_30_days"));
}

/// Where the county's declarations come from.
pub(crate) const RECOVERY_SOURCE: &str = "openfema_declarations";

/// The recovery page's facts for `PlanOutput::recovery`.
pub(crate) fn recovery_info(county: &rr_types::CountyRecord) -> rr_types::RecoveryInfo {
    match county.declarations.as_ref() {
        Some(d) => rr_types::RecoveryInfo {
            county_declarations_5yr: Some(d.last_5yr),
            sources: vec![CitationId::from(RECOVERY_SOURCE)],
        },
        None => rr_types::RecoveryInfo::default(),
    }
}

/// `## If it lasts for months`: the long-horizon items (they stay in their months too, so here
/// they are one line each with the quantities added up and the month each starts), how likely a
/// cut of two or three months is here (the consequence model's `multi_month`, from the household's
/// own power curve, ranges only), and the block's pointers. Only when the plan has the section
/// (`Plan::long_horizon`).
pub(crate) fn long_horizon(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let items = &cx.a.budget.plan.long_horizon;
    if items.is_empty() {
        return;
    }
    out.push("## If it lasts for months".to_owned());
    out.push(String::new());
    let first_month = |id: &rr_types::ItemId| {
        cx.steps()
            .into_iter()
            .find(|(_, i)| i.item_id == *id)
            .map(|(m, _)| m)
    };
    for i in items {
        let q = if i.kind == PlanItemKind::FreeAction {
            String::new()
        } else {
            let q = text::quantity(f64::from(i.quantity), &i.unit);
            if q.is_empty() {
                String::new()
            } else {
                format!(": {}", md(&q))
            }
        };
        let when = first_month(&i.item_id)
            .map(|m| format!(" (month {m})"))
            .unwrap_or_default();
        out.push(format!("- [ ] {}{q}{when}", md(&i.name)));
    }
    out.push(String::new());
    // How likely a cut of months is here, from the household's own power curve (the consequence
    // model's 60- and 90-day rates with their 10th and 90th percentiles), as ranges only, like
    // the rare table's row.
    let mm = &cx.a.consequence.multi_month;
    let years = cx.a.input.dials.horizon_years.max(1);
    if mm.range_60_days[1] > 0.0 {
        out.push(format!(
            "**How likely here.** From your own power curve, power out for two months or more: {} \
             households like yours in {}; for three months or more: {}.{}",
            super::risks::range_words(mm.range_60_days[0], mm.range_60_days[1], years),
            rr_consequence::words::horizon_phrase(years),
            super::risks::range_words(mm.range_90_days[0], mm.range_90_days[1], years),
            cite_all(&mm.sources)
        ));
        out.push(String::new());
    }
    // The pointer about wells is for households on one, the one about stored fuel for households
    // with a generator or fuel in the plan.
    let well = cx.a.input.housing.water == rr_types::WaterSource::Well;
    let fuel = cx.a.input.housing.backup_power == rr_types::BackupPower::Generator
        || ["power_generator", "power_generator_fuel", "power_fuel_cans"]
            .iter()
            .any(|id| cx.has_item(id));
    let paras: Vec<String> = block_paragraphs(cx, "topic:long_horizon")
        .into_iter()
        .filter(|p| well || !p.starts_with("**Wells."))
        .filter(|p| fuel || !p.starts_with("**Fuel."))
        .collect();
    push_paragraphs(out, paras);
}
