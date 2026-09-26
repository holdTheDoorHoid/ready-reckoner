//! Section 2: your risks. Cards for the hazards a household most needs to know about, each with
//! its guidance block: the likeliest ones, and the ones that kill (house fire always; any hazard
//! rated Severe or worse, or one that strikes with minutes of warning, or one this home is
//! exposed to, from a 1 in 100 chance in ten years; the hazard of any named scenario the plan
//! includes). Every other hazard is one row of a table (those under 1 in 100 share one line);
//! then the rare-and-catastrophic box with likelihood and severity as separate columns, and the
//! plain caveats behind the numbers.

use rr_content::Guidance;
use rr_types::{HazardDisplay, HazardId, HazardProfile, HousingKind, Mobility};

use super::text::{self, md};
use super::{Ctx, cite_all};

/// At most this many hazards get a card in all; the least severe of the likeliest ones make room
/// first, and house fire, Severe hazards and named scenarios' hazards are never dropped.
pub const CARDS: usize = 9;

/// The likeliest hazards: at most this many, each with at least [`CARD_MIN_P10`].
pub const FREQUENT_CARDS: usize = 6;

/// A hazard is one of the likeliest only with at least this chance of reaching the household in
/// ten years.
pub const CARD_MIN_P10: f64 = 0.10;

/// A hazard that kills gets a card from this chance in ten years up (review S3, RR-P03, M-07).
pub const LIFE_SAFETY_MIN_P10: f64 = 0.01;

/// "Severe" on the severity scale the packet prints ([`text::severity`]).
pub const SEVERE: f64 = 0.6;

/// Hazards that can strike with minutes of warning or none, where knowing what to do in the
/// moment saves lives (model review M-07): a card from [`LIFE_SAFETY_MIN_P10`] up.
pub const FAST_HAZARDS: [HazardId; 8] = [
    HazardId::Wildfire,
    HazardId::RiverineFlooding,
    HazardId::CoastalFlooding,
    HazardId::Tsunami,
    HazardId::Tornado,
    HazardId::Earthquake,
    HazardId::Landslide,
    HazardId::Avalanche,
];

/// Why a hazard has a card. Every reason but [`Why::Frequent`] protects the card from the cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Why {
    /// One of the likeliest hazards.
    Frequent,
    /// It strikes fast, or this home is exposed to it (a basement flat and floods, a mobile home
    /// and wind, someone slow to leave and wildfire or floods).
    Exposed,
    /// Rated Severe or worse.
    Severe,
    /// A named scenario the plan includes hangs on it.
    Scenario,
    /// House fire, always.
    HouseFire,
}

/// Whether this household's home or members make a hazard more dangerous for it (model review
/// M-07): a home below street level or with a basement and floods; a mobile home and wind or
/// hurricanes; someone who needs help to move and wildfire or floods.
fn household_exposed(cx: &Ctx<'_>, h: HazardId) -> bool {
    let input = &cx.a.input;
    let housing = &input.housing;
    let slow = input
        .people
        .iter()
        .any(|p| p.medical.mobility != Mobility::None);
    match h {
        HazardId::RiverineFlooding | HazardId::CoastalFlooding => {
            housing.floor <= 0 || housing.basement || slow
        }
        HazardId::Tornado | HazardId::StrongWind | HazardId::Hurricane => {
            housing.kind == HousingKind::MobileHome
        }
        HazardId::Wildfire => slow,
        _ => false,
    }
}

/// The chance of at least one event in `years` at a yearly rate.
fn chance(rate: f64, years: u8) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -rr_types::math::exp_m1(-rate * f64::from(years.max(1)))
    }
}

/// The ranked hazards (not the rare-and-catastrophic ones), most likely first.
fn ranked<'a>(cx: &Ctx<'a>) -> Vec<&'a HazardProfile> {
    let mut v: Vec<&HazardProfile> =
        cx.a.hazards
            .profiles
            .iter()
            .filter(|p| p.display == HazardDisplay::Ranked)
            .collect();
    // Stable: equal rates keep the register's order.
    v.sort_by(|x, y| y.rate_per_year.total_cmp(&x.rate_per_year));
    v
}

/// Why each ranked hazard has a card (life-safety reasons first), before the cap.
fn card_reasons<'a>(cx: &Ctx<'a>) -> Vec<(&'a HazardProfile, Why)> {
    let all = ranked(cx);
    let p10 = |p: &HazardProfile| chance(p.rate_per_year, 10);
    let frequent: Vec<HazardId> = all
        .iter()
        .filter(|p| p10(p) >= CARD_MIN_P10)
        .take(FREQUENT_CARDS)
        .map(|p| p.id)
        .collect();
    let scenario: Vec<HazardId> =
        cx.a.hazards
            .scenarios
            .iter()
            .filter(|s| s.on)
            .map(|s| s.hazard)
            .collect();
    let mut out: Vec<(&HazardProfile, Why)> = Vec::new();
    for p in
        cx.a.hazards
            .profiles
            .iter()
            .filter(|p| p.display == HazardDisplay::Ranked)
    {
        let likely_enough = p10(p) >= LIFE_SAFETY_MIN_P10;
        let why = if p.id == HazardId::HouseFire {
            Some(Why::HouseFire)
        } else if scenario.contains(&p.id) {
            Some(Why::Scenario)
        } else if likely_enough && p.severity >= SEVERE {
            Some(Why::Severe)
        } else if likely_enough && (FAST_HAZARDS.contains(&p.id) || household_exposed(cx, p.id)) {
            Some(Why::Exposed)
        } else if frequent.contains(&p.id) {
            Some(Why::Frequent)
        } else {
            None
        };
        if let Some(w) = why {
            out.push((p, w));
        }
    }
    out
}

/// The hazards shown as cards, most likely first, each with the guidance block its card shows.
/// The targets section uses this to point to a card instead of repeating the same block.
///
/// The rule (review S3): the likeliest hazards ([`FREQUENT_CARDS`] with at least
/// [`CARD_MIN_P10`]), plus house fire always, plus any hazard with at least
/// [`LIFE_SAFETY_MIN_P10`] that is rated Severe or worse, strikes fast ([`FAST_HAZARDS`]) or
/// meets this home ([`household_exposed`]), plus the hazard of any named scenario the plan
/// includes. Above [`CARDS`], the least severe of the likeliest make room first, then the least
/// likely fast or exposed ones; house fire, Severe hazards and scenario hazards always stay.
pub(crate) fn cards<'a>(cx: &Ctx<'a>) -> Vec<(&'a HazardProfile, Option<&'a Guidance>)> {
    let mut chosen = card_reasons(cx);
    while chosen.len() > CARDS {
        // The weakest reason first; among equals the least severe, then the least likely.
        let drop = chosen
            .iter()
            .enumerate()
            .filter(|(_, (_, w))| matches!(w, Why::Frequent | Why::Exposed))
            .min_by(|(_, (a, wa)), (_, (b, wb))| {
                wa.cmp(wb)
                    .then(if *wa == Why::Frequent {
                        a.severity.total_cmp(&b.severity)
                    } else {
                        core::cmp::Ordering::Equal
                    })
                    .then(a.rate_per_year.total_cmp(&b.rate_per_year))
            })
            .map(|(i, _)| i);
        match drop {
            Some(i) => {
                chosen.remove(i);
            }
            None => break,
        }
    }
    // Most likely first; equal rates keep the register's order.
    chosen.sort_by(|(x, _), (y, _)| y.rate_per_year.total_cmp(&x.rate_per_year));
    let mut used: Vec<&str> = Vec::new();
    chosen
        .into_iter()
        .map(|(p, _)| {
            let block = cx
                .blocks_for(&format!("hazard:{}", p.id))
                .into_iter()
                .find(|g| !used.contains(&g.meta.id.as_str()));
            if let Some(g) = block {
                used.push(g.meta.id.as_str());
            }
            (p, block)
        })
        .collect()
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let years = a.input.dials.horizon_years.max(1);
    let horizon = rr_consequence::words::horizon_phrase(years);
    out.push("## Your risks".to_owned());
    out.push(String::new());
    out.push(format!(
        "What could reach a household like yours in {} over {horizon}: the likeliest events, and \
         the rarer ones that can kill, most likely first. A lost job or a week-long outage is far \
         more likely than a disaster from the movies, and the plan is ordered the same way.",
        md(&super::summary::place(cx))
    ));
    out.push(String::new());

    let cards = cards(cx);
    if !cards.is_empty() {
        out.push("### What to know and do".to_owned());
        out.push(String::new());
    }
    // Which card showed each family block, so a later card of the same family can point to it.
    let mut shown: Vec<(&str, &str)> = Vec::new();
    for (i, (p, block)) in cards.iter().enumerate() {
        out.push(format!("#### {}. {}", i + 1, md(&p.name)));
        out.push(String::new());
        // The card's own sentence, cited to the card's sources, then what to do from its block
        // (the block's opening paragraph, the why, is the app's Learn view).
        let lead = format!("{}{}", md(&p.frequency_sentence), cite_all(&p.sources));
        let advice = block
            .map(|g| super::advice_paragraphs(&cx.guidance(g, None, None)))
            .unwrap_or_default();
        match block {
            Some(g) if advice.is_empty() => out.push(cx.guidance(g, Some(&lead), None)),
            _ => out.push(lead),
        }
        out.push(String::new());
        match block {
            Some(g) => shown.push((g.meta.id.as_str(), p.name.as_str())),
            None => {
                // The family's block is on an earlier card (a cold wave and a winter storm share
                // one): point there, so the threat still comes with what to do (PRINCIPLES §4).
                let earlier = cx
                    .blocks_for(&format!("hazard:{}", p.id))
                    .into_iter()
                    .find_map(|g| shown.iter().find(|(id, _)| *id == g.meta.id.as_str()));
                if let Some((_, name)) = earlier {
                    out.push(format!(
                        "**What helps.** The steps under \"{}\" above apply here too.",
                        md(name)
                    ));
                    out.push(String::new());
                }
            }
        }
        for para in advice {
            out.push(para);
            out.push(String::new());
        }
        // How bad and how sure; what it does to the household is its sentence above, and each
        // consequence has its own part under Your targets.
        out.push(format!(
            "**How bad:** {}. **How sure:** {}.",
            text::severity(p.severity),
            text::confidence(p.confidence)
        ));
        out.push(String::new());
    }

    // Every other ranked hazard: a row each, and the ones under 1 in 100 together in one line.
    let rest: Vec<&HazardProfile> = ranked(cx)
        .into_iter()
        .filter(|p| !cards.iter().any(|(c, _)| c.id == p.id))
        .collect();
    let (rows, faint): (Vec<&HazardProfile>, Vec<&HazardProfile>) = rest
        .into_iter()
        .partition(|p| text::per_100(chance(p.rate_per_year, years)) != "fewer than 1");
    if !rows.is_empty() || !faint.is_empty() {
        out.push("### Other risks we checked".to_owned());
        out.push(String::new());
    }
    if !rows.is_empty() {
        out.push(format!(
            "| What could happen | Households like yours, {horizon} | How bad | How sure |"
        ));
        out.push("| --- | --- | --- | --- |".to_owned());
        for p in &rows {
            out.push(format!(
                "| {} | {}{} | {} | {} |",
                md(&p.name),
                text::households(chance(p.rate_per_year, years)),
                cite_all(&p.sources),
                text::severity(p.severity),
                text::confidence(p.confidence)
            ));
        }
        out.push(String::new());
    }
    if !faint.is_empty() {
        let names: Vec<String> = faint.iter().map(|p| text::lower_first(&p.name)).collect();
        out.push(format!(
            "Fewer than 1 in 100 households like yours, {horizon}: {}.{}",
            md(&text::join_and(&names)),
            cite_all(faint.iter().flat_map(|p| p.sources.iter()))
        ));
        out.push(String::new());
    }

    let rare: Vec<&HazardProfile> = a
        .hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        .collect();
    if !rare.is_empty() {
        out.push("### Rare but severe".to_owned());
        out.push(String::new());
        out.push(
            "Shown apart, so that a tiny chance of a huge loss cannot crowd out everything else or \
             take over the budget."
                .to_owned(),
        );
        out.push(String::new());
        out.push("| What | How likely | How bad |".to_owned());
        out.push("| --- | --- | --- |".to_owned());
        for p in &rare {
            out.push(format!(
                "| {} | {}{} | {} |",
                md(&p.name),
                md(&p.frequency_sentence),
                cite_all(&p.sources),
                text::severity(p.severity)
            ));
        }
        out.push(String::new());
        // What to do: the first rare hazard's block, its advice paragraphs only (the table says
        // how likely), unless a card already showed it. Outside a nuclear plant's 10-mile zone
        // only the shelter guidance prints: the "What helps" paragraph to its second source
        // (get inside, stay inside, stay tuned; where, and for how long). The rest, potassium
        // iodide included, matters inside the zone (review RR-P03: the space pays for the fire
        // card every packet now carries).
        let near_plant = a.supply_context.nuclear_plant_within_16km == Some(true);
        if let Some(g) = rare.iter().find_map(|p| {
            cx.blocks_for(&format!("hazard:{}", p.id))
                .into_iter()
                .next()
        }) {
            if !cards
                .iter()
                .any(|(_, b)| b.is_some_and(|b| b.meta.id == g.meta.id))
            {
                for para in super::advice_paragraphs(&cx.guidance(g, None, None)) {
                    let para = if near_plant {
                        para
                    } else if para.starts_with("**What helps.**") {
                        super::up_to_citation_runs(&para, 2)
                    } else {
                        continue;
                    };
                    out.push(para);
                    out.push(String::new());
                }
            }
        }
    }

    if !a.hazards.notes.is_empty() {
        out.push("### Notes on these numbers".to_owned());
        out.push(String::new());
        for n in &a.hazards.notes {
            out.push(format!("- {}", md(n)));
        }
        out.push(String::new());
    }
}
