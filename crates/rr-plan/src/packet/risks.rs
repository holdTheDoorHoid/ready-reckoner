//! Section: your risks. Cards for the hazards a household most needs to know about, each with
//! its guidance block and its named sub-causes on one line: the likeliest ones, and the ones
//! that kill (see [`cards`] for the rule). Every other hazard is one row of a table; those under
//! 1 in 100 share one line, and everything under 1 in 100,000 a year is "Also checked", one
//! paragraph with its rates. Then the nine rare families as one collapsed table (how likely, as a
//! range only; if it reaches you; what it changes in the plan), the action of each family block
//! whose location factor raises the family above the national figure, and the plain caveats
//! behind the numbers.

use rr_content::Guidance;
use rr_types::{HazardDisplay, HazardId, HazardProfile, HousingKind, Mobility};

use super::text::{self, md};
use super::{Ctx, cite_all};

/// At most this many hazards get a card in all; see [`cards`] for which make room first. House
/// fire, Severe hazards, named scenarios' and compound events' hazards and the wind card are
/// never dropped, so a county with more of those prints more cards.
pub const CARDS: usize = 8;

/// The likeliest hazards: at most this many, each with at least [`CARD_MIN_P10`].
pub const FREQUENT_CARDS: usize = 6;

/// A hazard is one of the likeliest only with at least this chance of reaching the household in
/// ten years. The wind card uses the same cut.
pub const CARD_MIN_P10: f64 = 0.10;

/// A hazard that kills gets a card from this chance in ten years up (review S3, RR-P03, M-07).
pub const LIFE_SAFETY_MIN_P10: f64 = 0.01;

/// "Severe" on the severity scale the packet prints ([`text::severity`]).
pub const SEVERE: f64 = 0.6;

/// Below this a hazard is "Minor" ([`text::severity`]): a Minor card never displaces a Serious or
/// Severe one.
pub const MINOR: f64 = 0.2;

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

/// The hazards whose card tells the household where to shelter from wind (the tornado, wind,
/// hail and lightning block). The likeliest of them keeps its card from [`CARD_MIN_P10`] up,
/// unless one of them rated Severe already has a card (it shows the same block): storms are the
/// likeliest dangerous weather for most homes, and the shelter-spot advice is on that card (the
/// frequent-but-minor rows of v0.2, phone outages and arrests, had pushed it out in Philadelphia
/// and Miami).
pub const WIND_HAZARDS: [HazardId; 4] = [
    HazardId::Tornado,
    HazardId::StrongWind,
    HazardId::Hail,
    HazardId::Lightning,
];

/// Likeliest-hazard cards that stay when the cap is reached: the hazard has one consequence, and
/// that consequence's part under Your targets prints the same advice in full whenever the card is
/// missing (a medical emergency's first aid and CPR), so dropping the card saves no space.
pub const SINGLE_CONSEQUENCE_CARDS: [HazardId; 1] = [HazardId::MedicalEmergency];

/// A compound event class drives a target when its share of the target's rate is at least this.
pub const DRIVES_SHARE: f64 = 0.05;

/// The consequence model's compound event classes (model review M-11): the grid emergency in
/// extreme cold (a cold wave's `cold_emergency` rows) and the blackout during a heat wave (the
/// `heat_blackout` scenario's rows).
const COMPOUND_CLASSES: [&str; 1] = ["cold_emergency"];
const COMPOUND_SCENARIOS: [&str; 1] = ["heat_blackout"];

/// The national figure the nuclear family's location factor (the strategic class's share, f_S)
/// is compared with: its population-weighted mean over every county, 0.314
/// (`docs/RISK_MODEL.md`, "The rare families"; the class table's "unknown" row).
pub const NUCLEAR_NATIONAL_FACTOR: f64 = 0.314;

/// Why a hazard has a card. The order is the order in which cards make room above [`CARDS`]:
/// frequent before exposed; the rest are never dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Why {
    /// One of the likeliest hazards.
    Frequent,
    /// It strikes fast, or this home is exposed to it (a basement flat and floods, a mobile home
    /// and wind, someone slow to leave and wildfire or floods).
    Exposed,
    /// The household's likeliest wind hazard ([`WIND_HAZARDS`]).
    Wind,
    /// A card that would otherwise be dropped whose hazard has one consequence, and that
    /// consequence's part under Your targets prints the same advice when the card is missing (a
    /// medical emergency): dropping it saves no space.
    Single,
    /// Rated Severe or worse.
    Severe,
    /// It drives a compound event class behind a target ([`DRIVES_SHARE`]).
    Compound,
    /// A named scenario the plan includes hangs on it.
    Scenario,
    /// House fire, always.
    HouseFire,
}

impl Why {
    /// Never dropped by the cap.
    fn protected(self) -> bool {
        self >= Why::Wind
    }
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
            housing.floor <= 0 || housing.basement || housing.below_grade_bedroom || slow
        }
        HazardId::Tornado | HazardId::StrongWind | HazardId::Hurricane => {
            housing.kind == HousingKind::MobileHome
        }
        HazardId::Wildfire | HazardId::DamFailure => slow,
        _ => false,
    }
}

/// The chance of at least one event in `years` at a yearly rate.
pub(crate) fn chance(rate: f64, years: u8) -> f64 {
    if rate <= 0.0 {
        0.0
    } else {
        -rr_types::math::exp_m1(-rate * f64::from(years.max(1)))
    }
}

/// The ranked hazards (not the rare families), most likely first.
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

/// Whether a hazard's compound event class carries at least [`DRIVES_SHARE`] of some duration
/// target's rate (at the target).
fn drives_compound(cx: &Ctx<'_>, h: HazardId) -> bool {
    let scenarios = &cx.a.hazards.scenarios;
    cx.a.consequence.details.iter().any(|d| {
        let t = f64::from(d.ladder_days);
        if t <= 0.0 {
            return false;
        }
        let total: f64 = d.curve.terms.iter().map(|x| x.weight * x.sf(t)).sum();
        let part: f64 = d
            .curve
            .terms
            .iter()
            .filter(|x| x.hazard == h)
            .filter(|x| {
                COMPOUND_CLASSES.contains(&x.class.as_str())
                    || x.scenario.is_some_and(|i| {
                        scenarios
                            .get(i)
                            .is_some_and(|s| COMPOUND_SCENARIOS.contains(&s.id.as_str()))
                    })
            })
            .map(|x| x.weight * x.sf(t))
            .sum();
        total > 0.0 && part / total >= DRIVES_SHARE
    })
}

/// Why each ranked hazard has a card (the strongest reason), before the cap.
fn card_reasons<'a>(cx: &Ctx<'a>) -> Vec<(&'a HazardProfile, Why)> {
    let all = ranked(cx);
    let p10 = |p: &HazardProfile| chance(p.rate_per_year, 10);
    let frequent: Vec<HazardId> = all
        .iter()
        .filter(|p| p10(p) >= CARD_MIN_P10)
        .take(FREQUENT_CARDS)
        .map(|p| p.id)
        .collect();
    // The wind-shelter card: the likeliest wind hazard, unless a wind hazard that kills already
    // has a card for its own reason (a tornado or hail rated Severe shows the same block).
    let severe_wind = all.iter().any(|p| {
        WIND_HAZARDS.contains(&p.id) && p10(p) >= LIFE_SAFETY_MIN_P10 && p.severity >= SEVERE
    });
    let wind: Option<HazardId> = all
        .iter()
        .find(|p| WIND_HAZARDS.contains(&p.id) && p10(p) >= CARD_MIN_P10)
        .map(|p| p.id)
        .filter(|_| !severe_wind);
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
        // A hazard whose only consequence is lost income (a lost job, an earner's death or
        // disability) has no card: what to do about it is the savings goal and the insurance
        // decisions under Documents and money, where the income part of Your targets points.
        if p.buckets.iter().all(|b| *b == rr_types::BucketId::Income) {
            continue;
        }
        let why = if p.id == HazardId::HouseFire {
            Some(Why::HouseFire)
        } else if scenario.contains(&p.id) {
            Some(Why::Scenario)
        } else if likely_enough && drives_compound(cx, p.id) {
            Some(Why::Compound)
        } else if likely_enough && p.severity >= SEVERE {
            Some(Why::Severe)
        } else if wind == Some(p.id) {
            Some(Why::Wind)
        } else if likely_enough && (FAST_HAZARDS.contains(&p.id) || household_exposed(cx, p.id)) {
            Some(Why::Exposed)
        } else if frequent.contains(&p.id) {
            if SINGLE_CONSEQUENCE_CARDS.contains(&p.id) {
                Some(Why::Single)
            } else {
                Some(Why::Frequent)
            }
        } else {
            None
        };
        if let Some(w) = why {
            out.push((p, w));
        }
    }
    out
}

/// A severity's class on the printed scale: Minor 0, Moderate 1, Serious 2, Severe or worse 3.
fn severity_class(s: f64) -> u8 {
    if s < MINOR {
        0
    } else if s < 0.4 {
        1
    } else if s < SEVERE {
        2
    } else {
        3
    }
}

/// The hazards shown as cards, most likely first, each with the guidance block its card shows.
/// The targets section uses this to point to a card instead of repeating the same block.
///
/// The rule (review S3; round-2 packet v2): the likeliest hazards ([`FREQUENT_CARDS`] with at
/// least [`CARD_MIN_P10`]), leaving out those whose only consequence is lost income (their
/// advice is under Documents and money); house fire always; any hazard with at least [`LIFE_SAFETY_MIN_P10`]
/// that is rated Severe or worse, drives a compound event class behind a target (the grid
/// emergency in extreme cold, the blackout in a heat wave), strikes fast ([`FAST_HAZARDS`]) or
/// meets this home ([`household_exposed`]); the hazard of any named scenario the plan includes;
/// and the household's likeliest wind hazard ([`WIND_HAZARDS`]) from [`CARD_MIN_P10`]. Above
/// [`CARDS`], the less severe cards make room first, so a Minor hazard never displaces a Serious
/// or Severe one; among cards of one severity, the likeliest-hazard cards before the fast or
/// exposed ones, and the least likely first. House fire, Severe hazards, scenario and compound
/// hazards and the wind card always stay.
pub(crate) fn cards<'a>(cx: &Ctx<'a>) -> Vec<(&'a HazardProfile, Option<&'a Guidance>)> {
    let mut chosen = card_reasons(cx);
    while chosen.len() > CARDS {
        let drop = chosen
            .iter()
            .enumerate()
            .filter(|(_, (_, w))| !w.protected())
            .min_by(|(_, (a, wa)), (_, (b, wb))| {
                severity_class(a.severity)
                    .cmp(&severity_class(b.severity))
                    .then(wa.cmp(wb))
                    .then(a.rate_per_year.total_cmp(&b.rate_per_year))
            })
            .map(|(i, _)| i);
        // With only protected cards left above the cap, the wind card (the one protected card
        // that can be Minor) makes room: it never displaces a Serious or Severe card.
        let drop = drop.or_else(|| {
            chosen
                .iter()
                .enumerate()
                .filter(|(_, (_, w))| *w == Why::Wind)
                .min_by(|(_, (a, _)), (_, (b, _))| a.rate_per_year.total_cmp(&b.rate_per_year))
                .map(|(i, _)| i)
        });
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

/// "1 in 2,600": two significant figures, whole numbers below 10; "fewer than 1 in 1,000,000".
fn one_in(p: f64) -> String {
    if p < 1e-6 {
        return "fewer than 1 in 1,000,000".to_owned();
    }
    let n = 1.0 / p;
    if n < 10.0 {
        return format!("1 in {}", n.round().max(2.0) as u64);
    }
    // Two significant figures.
    let mut scale = 1.0_f64;
    while n / scale >= 100.0 {
        scale *= 10.0;
    }
    let r = ((n / scale).round() * scale) as u64;
    format!("1 in {}", text::thousands(r))
}

/// A rare row's chance over the horizon as a range only (never a point; `range_only`), worded
/// as the app words it (`web/src/lib/format.ts` `rangeOnly`): "between 1 in 3,300 and 1 in 28",
/// or "very unlikely: less than 1 in 25,000" when the range spans more than a thousandfold.
pub(crate) fn range_words(low: f64, high: f64, years: u8) -> String {
    let lo = chance(low.max(0.0), years);
    let hi = chance(high.max(0.0), years);
    if hi <= 0.0 {
        return "no known chance".to_owned();
    }
    if low <= 0.0 || high / low > 1000.0 || lo < 1e-6 {
        return format!("very unlikely: less than {}", one_in(hi));
    }
    let (a, b) = (one_in(lo), one_in(hi));
    if a == b {
        format!("about {a}")
    } else {
        format!("between {a} and {b}")
    }
}

/// Whether the family's location factor raises it above the national figure, so its block's
/// action prints below the rare table: the nuclear family where the strategic class's share is
/// above its population-weighted mean ([`NUCLEAR_NATIONAL_FACTOR`]: classes A, B and C1), a
/// solar storm where the geomagnetic factor is above the national average (above 1). The other
/// families are the same everywhere, or their factor is a share with no national figure to
/// compare with.
pub fn family_block_prints(p: &HazardProfile) -> bool {
    let Some(lf) = &p.location_factor else {
        return false;
    };
    match p.id {
        HazardId::NuclearAttack => lf.multiplier[1] > NUCLEAR_NATIONAL_FACTOR,
        HazardId::GeomagneticStorm => lf.multiplier[1] > 1.0,
        _ => false,
    }
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let years = a.input.dials.horizon_years.max(1);
    let horizon = rr_consequence::words::horizon_phrase(years);
    out.push("## Your risks".to_owned());
    out.push(String::new());
    out.push(format!(
        "What could reach a household like yours in {} over {horizon}: the likeliest events, and \
         the rarer ones that can kill, most likely first.",
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
        // The card's own sentence, cited to the card's sources; its named sub-causes on one line
        // (their notes and numbers are the app's); then what to do from its block (the block's
        // opening paragraph, the why, is the app's Learn view).
        let lead = format!("{}{}", md(&p.frequency_sentence), cite_all(&p.sources));
        let advice = block
            .map(|g| super::advice_paragraphs(&cx.guidance(g, None, None)))
            .unwrap_or_default();
        match block {
            Some(g) if advice.is_empty() => out.push(cx.guidance(g, Some(&lead), None)),
            _ => out.push(lead),
        }
        out.push(String::new());
        // A sub-cause counted on another card ("not here") is not part of this one.
        let subs: Vec<String> = p
            .sub_causes
            .iter()
            .filter(|s| !s.note.contains("not here"))
            .map(|s| text::lower_first(&s.name))
            .collect();
        if !subs.is_empty() {
            out.push(format!("**Includes:** {}.", md(&subs.join("; "))));
            out.push(String::new());
        }
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
        out.push(format!(
            "**How bad:** {} ({}).",
            text::severity(p.severity),
            text::lower_first(text::confidence(p.confidence))
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
    let also = also_checked(cx);
    if !rows.is_empty() || !faint.is_empty() || also.is_some() {
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
    if let Some(p) = also {
        out.push(p);
        out.push(String::new());
    }

    rare(cx, out);

    let notes: Vec<&String> = a
        .hazards
        .notes
        .iter()
        .filter(|n| !n.starts_with(ALSO_CHECKED))
        .collect();
    if !notes.is_empty() {
        out.push("### Notes on these numbers".to_owned());
        out.push(String::new());
        for n in notes {
            out.push(format!("- {}", md(n)));
        }
        out.push(String::new());
    }
}

/// How `rr-hazards` opens its "Also checked" note.
const ALSO_CHECKED: &str = "Also checked";

/// "Also checked": every hazard and rare sub-row under 1 in 100,000 a year here, with its rate,
/// as one paragraph (REVIEW §2.4), in `rr-hazards`' words and cited to their sources. Rows that
/// share their words ("fewer than 1 in 1,000,000 a year", "none recorded here") are listed
/// together with the words once ([`group_also_checked`]).
fn also_checked(cx: &Ctx<'_>) -> Option<String> {
    let note =
        cx.a.hazards
            .notes
            .iter()
            .find(|n| n.starts_with(ALSO_CHECKED))?;
    let names: Vec<&str> =
        cx.a.hazards
            .also_checked
            .iter()
            .map(|c| c.name.as_str())
            .collect();
    let text = group_also_checked(note, &names).unwrap_or_else(|| note.clone());
    Some(format!(
        "{}{}",
        md(&text),
        cite_all(
            cx.a.hazards
                .also_checked
                .iter()
                .flat_map(|x| x.sources.iter())
        )
    ))
}

/// `rr-hazards`' note ("…here: coastal floods (fewer than 1 in 1,000,000 a year), sinkholes
/// (none recorded here), landslides (fewer than 1 in 1,000,000 a year) and …") with the rows
/// that share their words listed together, in the order each wording first appears: "…here:
/// coastal floods and landslides (each fewer than 1 in 1,000,000 a year); sinkholes (none
/// recorded here); …". `None` when a row's name is not found as "name (" in the note, so the
/// note prints as it came.
fn group_also_checked(note: &str, names: &[&str]) -> Option<String> {
    let (head, _) = note.split_once(": ")?;
    let mut groups: Vec<(String, Vec<&str>)> = Vec::new();
    for name in names {
        let key = format!("{name} (");
        let at = note.find(&key)? + key.len();
        let words = &note[at..at + note[at..].find(')')?];
        match groups.iter_mut().find(|(w, _)| w == words) {
            Some((_, g)) => g.push(name),
            None => groups.push((words.to_owned(), vec![name])),
        }
    }
    if groups.len() == names.len() {
        // Nothing shared: the note as it came.
        return None;
    }
    let parts: Vec<String> = groups
        .into_iter()
        .map(|(words, g)| {
            let list: Vec<String> = g.iter().map(|n| (*n).to_owned()).collect();
            let each = if g.len() > 1 && words.contains(" in ") {
                "each "
            } else {
                ""
            };
            format!("{} ({each}{words})", text::join_and(&list))
        })
        .collect();
    Some(format!("{head}: {}.", parts.join("; ")))
}

/// The nine rare families as one collapsed table, sorted by how likely each is here (never by
/// how bad; REVIEW §2.4), then the action of each family block whose location factor raises the
/// family above the national figure ([`family_block_prints`]).
fn rare(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let years = a.input.dials.horizon_years.max(1);
    let rare: Vec<&HazardProfile> = a
        .hazards
        .profiles
        .iter()
        .filter(|p| p.display == HazardDisplay::RareCatastrophic)
        .collect();
    if rare.is_empty() {
        return;
    }
    out.push("### Rare but severe".to_owned());
    out.push(String::new());
    out.push(format!(
        "Ranges only, shown apart so a tiny chance of a huge loss cannot take over the plan; for \
         households like yours, {}.",
        rr_consequence::words::horizon_phrase(years)
    ));
    out.push(String::new());
    out.push("| What | How likely | If it reaches you | What it changes in your plan |".to_owned());
    out.push("| --- | --- | --- | --- |".to_owned());
    for p in &rare {
        out.push(format!(
            "| {} | {}{} | {} | {} |",
            md(&p.name),
            text::upper_first(&range_words(p.rate_range[0], p.rate_range[1], years)),
            cite_all(&p.sources),
            md(p.if_it_reaches_you.as_deref().unwrap_or("")),
            md(p.what_it_changes.as_deref().unwrap_or(""))
        ));
    }
    out.push(String::new());
    // Families where the place itself raises the chance above the national figure: why here
    // (the location factor's words). What to do if it happens is in the shelter plan (fallout)
    // and the power part of Your targets.
    for p in rare.iter().copied().filter(|p| family_block_prints(p)) {
        if let Some(lf) = &p.location_factor {
            out.push(format!(
                "**{}: why here.** {}{}",
                md(&p.name),
                md(&lf.label),
                cite_all(&lf.sources)
            ));
            out.push(String::new());
        }
    }
}
