//! The packet v2 pages that are guidance blocks in their own right (DESIGN-DELTA §3, review N1,
//! N2, N3, N6): the shelter plan (`plan:shelter`), the list for the 48 hours before a forecast
//! storm (`plan:forecast_48h`), access and functional needs (`topic:access_needs`, CMIST) when
//! anyone in the household has one, the household's state row from the registries table, "After
//! a disaster: the first 30 days" (`after:first_30_days`) with the county's declarations, and the
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

/// `## Your shelter plan`: where to shelter from each danger that is likely enough here.
pub(super) fn shelter(cx: &Ctx<'_>, out: &mut Vec<String>) {
    out.push("## Your shelter plan".to_owned());
    out.push(String::new());
    push_paragraphs(out, block_paragraphs(cx, "plan:shelter"));
}

/// `## When a storm, freeze or heat wave is forecast`: the 48-hour list (practitioner P-08). The
/// freeze and heat-wave steps print where those hazards are likely enough to list (a ten-year
/// chance of 1 in 100, as the block's own hazard spans would keep them): no freeze steps in
/// Puerto Rico.
pub(super) fn forecast(cx: &Ctx<'_>, out: &mut Vec<String>) {
    out.push("## When a storm, freeze or heat wave is forecast".to_owned());
    out.push(String::new());
    let freeze = ["cold_wave", "winter_weather", "ice_storm"]
        .iter()
        .any(|h| cx.hazard_relevant(h));
    let heat = cx.hazard_relevant("heat_wave");
    let paras: Vec<String> = block_paragraphs(cx, "plan:forecast_48h")
        .into_iter()
        .filter(|p| freeze || !p.starts_with("**Before a hard freeze."))
        .filter(|p| heat || !p.starts_with("**Before a heat wave."))
        .collect();
    push_paragraphs(out, paras);
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
pub(super) fn access_needs(cx: &Ctx<'_>, out: &mut Vec<String>) {
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
                "person {} ({}): {}",
                i + 1,
                super::text::lower_first(age_word(p.age_band)),
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

fn age_word(b: rr_types::AgeBand) -> &'static str {
    match b {
        rr_types::AgeBand::Infant => "baby",
        rr_types::AgeBand::Toddler => "toddler",
        rr_types::AgeBand::Child => "child",
        rr_types::AgeBand::Teen => "teenager",
        rr_types::AgeBand::Adult => "adult",
        rr_types::AgeBand::Senior => "older adult",
    }
}

/// Escapes text for Markdown but leaves web addresses as they are, so the app can link them.
fn md_keep_urls(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let mut rest = s;
    while let Some(start) = rest.find("http") {
        out.push_str(&md(&rest[..start]));
        let url = &rest[start..];
        let end = url
            .char_indices()
            .find(|(_, c)| c.is_whitespace() || *c == ')')
            .map_or(url.len(), |(i, _)| i);
        // A sentence's full stop after the address is not part of it.
        let end = if url[..end].ends_with('.') {
            end - 1
        } else {
            end
        };
        out.push_str(&url[..end]);
        rest = &url[end..];
    }
    out.push_str(&md(rest));
    out
}

/// `## Local help`: the household's state row from the registries table (evacuation zones, the
/// registry for people who may need help, alerts, emergency prescription refills;
/// `docs/CONTENT_STANDARDS.md` §10).
pub(super) fn local_help(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let loc = &cx.a.location;
    out.push("## Local help".to_owned());
    out.push(String::new());
    let Some(row) = cx.content.states.row(&loc.state_abbr) else {
        out.push(
            "Your county emergency management office can tell you your evacuation zone, any \
             registry for people who may need help, and how to get local alerts."
                .to_owned(),
        );
        out.push(String::new());
        return;
    };
    out.push(format!(
        "Where to start in {}, checked {}:",
        md(&row.name),
        text::date(row.checked)
    ));
    out.push(String::new());
    for l in row.lines() {
        out.push(format!(
            "- **{}:** {}{}",
            md(l.topic),
            md_keep_urls(&l.text),
            cite_all(&l.sources)
        ));
    }
    out.push(String::new());
}

/// `## After a disaster: the first 30 days` (review RR-P06), with the county's federal disaster
/// declarations in the last five years (`PlanOutput::recovery`).
pub(super) fn after_disaster(cx: &Ctx<'_>, out: &mut Vec<String>) {
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
pub(super) fn long_horizon(cx: &Ctx<'_>, out: &mut Vec<String>) {
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
