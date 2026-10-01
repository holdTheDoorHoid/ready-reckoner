//! The Prepare sheet's opening: who and where, the plan date and setting, the status line, then
//! the summary: the step reached, the step that is enough with the plan's two done months, and
//! the everyday basics the plan assumes. The free steps to start with are the first list under
//! Your plan. The leave-first rule ([`leave_first`]) is the binder's Getting out page's.

use rr_types::{
    BackupPower, BucketId, ClimateHorizon, Cooling, Heating, HousingKind, TierId, WaterLevel,
    WaterSource,
};

use super::text::{self, md};
use super::{Ctx, cite, cite_all};
use crate::source::FIXTURE_DATA_NOTE;

/// "Philadelphia County, Pennsylvania".
pub(crate) fn place(cx: &Ctx<'_>) -> String {
    let loc = &cx.a.location;
    let kind = match loc.state_abbr.as_str() {
        "LA" => " Parish",
        "AK" => "",
        _ => " County",
    };
    format!("{}{kind}, {}", loc.county_name, loc.state_name)
}

fn home(cx: &Ctx<'_>) -> String {
    let h = &cx.a.input.housing;
    let tenure = match h.tenure {
        rr_types::Tenure::Own => "owned",
        rr_types::Tenure::Rent => "rented",
    };
    let kind = match h.kind {
        HousingKind::ApartmentHighRise => {
            format!("apartment on floor {} of a tall building", h.floor)
        }
        HousingKind::ApartmentLowRise => format!("apartment on floor {}", h.floor),
        HousingKind::Rowhouse => "rowhouse".to_owned(),
        HousingKind::Detached => "house".to_owned(),
        HousingKind::MobileHome => "mobile home".to_owned(),
        HousingKind::RuralProperty => "house on rural land".to_owned(),
    };
    let mut parts: Vec<String> = Vec::new();
    if h.basement {
        parts.push("a basement".to_owned());
    }
    parts.push(
        match h.water {
            WaterSource::Municipal => "city water",
            WaterSource::Well => "a private well",
        }
        .to_owned(),
    );
    parts.push(
        match h.heating {
            Heating::Gas => "gas heat",
            Heating::ElectricResistance => "electric heat",
            Heating::HeatPump => "a heat pump",
            Heating::Oil => "oil heat",
            Heating::Propane => "propane heat",
            Heating::Wood => "a wood stove",
            Heating::District => "building heat",
            Heating::None => "no heating",
        }
        .to_owned(),
    );
    parts.push(
        match h.cooling {
            Cooling::Central => "central air conditioning",
            Cooling::Window => "window air conditioning",
            Cooling::None => "no air conditioning",
        }
        .to_owned(),
    );
    match h.backup_power {
        BackupPower::None => {}
        BackupPower::PowerStation => parts.push("a battery power station".to_owned()),
        BackupPower::Generator => parts.push("a generator".to_owned()),
        BackupPower::SolarBattery => parts.push("solar panels with a battery".to_owned()),
    }
    let article = if tenure.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {tenure} {kind}, with {}", text::join_and(&parts))
}

/// "very serious disruptions: the kind that come about once in 100 years".
pub(crate) fn dial_phrase(cx: &Ctx<'_>) -> String {
    let d = &cx.a.input.dials;
    let name = text::lower_first(d.return_period.name());
    let label = if name.ends_with("disruptions") || name.ends_with("catastrophes") {
        name
    } else {
        format!("{name} disruptions")
    };
    let climate = match d.climate {
        ClimateHorizon::Today => "today's climate",
        ClimateHorizon::Y2050 => "the climate expected around 2050",
    };
    format!(
        "{label} (the 1-in-{} setting), in {climate}",
        d.return_period.years()
    )
}

/// The water level in words.
pub(crate) fn water_phrase(level: WaterLevel) -> &'static str {
    match level {
        WaterLevel::Survival => "the survival level, about 3 liters a person a day for drinking",
        WaterLevel::Basic => "the basic level, about 1 gallon a person a day",
        WaterLevel::Comfortable => {
            "the comfortable level, about 4 gallons (15 liters) a person a day"
        }
    }
}

fn tier_words(t: TierId) -> String {
    match t {
        TierId::Now => "free steps".to_owned(),
        other => format!("{} of supplies", text::lower_first(other.name())),
    }
}

pub(super) fn write(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    let input = &a.input;
    out.push("# Prepare: what to do before".to_owned());
    out.push(String::new());
    out.push(format!("**For:** {}  ", md(&text::household(input))));
    let zip = a
        .location
        .zip
        .as_deref()
        .map(|z| format!(" (ZIP code {z})"))
        .unwrap_or_default();
    out.push(format!("**Where:** {}{zip}  ", md(&place(cx))));
    out.push(format!("**Home:** {}  ", md(&home(cx))));
    out.push(format!(
        "**Plan date:** {}  ",
        text::date(input.planning_date)
    ));
    out.push(format!(
        "**Ready for:** {}. Water is planned at {}.{}",
        dial_phrase(cx),
        water_phrase(input.dials.water_level),
        cite("ready_gov_water")
    ));
    out.push(String::new());
    out.push(format!("> {}", super::STATUS_LINE));
    out.push(String::new());
    if a.location.data_note.as_deref() == Some(FIXTURE_DATA_NOTE) {
        out.push(format!("> **Sample data.** {FIXTURE_DATA_NOTE}"));
        out.push(String::new());
    }

    out.push("## Summary".to_owned());
    out.push(String::new());
    let reached = a.budget.tier_reached;
    let recommended = a
        .buckets
        .iter()
        .map(|b| b.id)
        .next()
        .map(|_| rr_supply::tier_recommended(&a.buckets))
        .unwrap_or(TierId::H72);
    let now = if reached == TierId::Now {
        "getting started".to_owned()
    } else {
        format!(
            "you already have what the {} step needs",
            tier_words(reached)
        )
    };
    let mut when = String::new();
    let monthly = f64::from(input.finances.monthly_budget_usd);
    let date = |m: u16| text::month_year(text::month_start(input.planning_date, m));
    // The plan's two done months (rr-budget v0.2.0): the bare-minimum kit first, then everything.
    const KIT: &str = "the bare minimum (three days of water, light, warmth and medicine)";
    match (a.budget.plan.minimum_done_month, a.budget.plan.done_month) {
        (Some(k), Some(m)) if k < m => {
            when = format!(
                " At your budget, {KIT} is in place by month {k} ({}), and everything by month \
                 {m} ({}).",
                date(k),
                date(m)
            );
        }
        (_, Some(m)) => {
            when = format!(
                " At your budget, the plan gets there by month {m} ({}).",
                date(m)
            );
        }
        (Some(k), None) if monthly > 0.0 => {
            when = format!(
                " At your budget, {KIT} is in place by month {k} ({}); the rest takes longer.",
                date(k)
            );
        }
        _ => {}
    }
    if when.is_empty() && monthly <= 0.0 && input.finances.one_off_budget_usd <= 0.0 {
        when = " With no money set aside, the plan is the free steps; they still cover a lot."
            .to_owned();
    }
    out.push(format!(
        "**Where you are now:** {now}. **What is enough for your risks:** {}.{when} Start with \
         the free steps under Your plan.",
        tier_words(recommended)
    ));
    out.push(String::new());

    assumptions(cx, out);
}

/// What the plan assumed the household already has (`assume_basics`), and how to undo it, in one
/// paragraph (packet v2: the page budget).
fn assumptions(cx: &Ctx<'_>, out: &mut Vec<String>) {
    let a = cx.a;
    if !a.assumed.is_empty() {
        let names: Vec<String> = a
            .assumed
            .iter()
            .filter_map(|(id, _)| cx.item(id.as_str()))
            .map(|it| text::lower_first(text::short_name(&it.name)))
            .collect();
        out.push(format!(
            "**What the plan assumes you already have:** {}. If any is missing, untick \"Assume \
             everyday basics\" on the Have screen and the plan will add it.",
            md(&text::join_and(&names))
        ));
        out.push(String::new());
    } else if !a.input.assume_basics {
        out.push(
            "You unticked \"Assume everyday basics\", so the plan counts only what you listed as \
             already owned."
                .to_owned(),
        );
        out.push(String::new());
    }
}

/// From this ten-year chance of having to leave home quickly, the summary leads with the decision
/// to leave (review S2, RR-P02).
pub const LEAVE_FIRST_P10: f64 = 0.25;

/// Named scenarios that make leaving the first thing that matters when the plan includes them.
pub const LEAVE_FIRST_SCENARIOS: [&str; 2] = ["major_hurricane_direct_hit", "local_tsunami"];

/// Hazards that give minutes of warning and force people out (packet v2): from a ten-year chance
/// of [`super::CARD_MIN_P10`] they put leaving first too.
pub const LEAVE_FIRST_FAST_HAZARDS: [rr_types::HazardId; 2] =
    [rr_types::HazardId::Wildfire, rr_types::HazardId::DamFailure];

/// Whether the home is in a storm-surge area: the ZIP code's Category 1–3 surge share (the
/// optional surge pack) or else the county's surge class, as the guardrail reads them
/// (`rr_budget::is_surge_zone`, review RR-P02).
fn surge_zone(cx: &Ctx<'_>) -> Option<rr_types::CitationId> {
    let e = &cx.a.location.exposure;
    let share = e.surge_cat3_share.as_ref();
    let class = e.surge_proxy_class.as_ref();
    rr_budget::is_surge_zone(share.map(|s| s.value), class.map(|c| c.value.as_str())).then(|| {
        share
            .map(|s| s.source.clone())
            .or_else(|| class.map(|c| c.source.clone()))
            .unwrap_or_else(|| rr_types::CitationId::from("rr_surge_proxy"))
    })
}

/// The decision to leave, when it comes before managing at home (review S2): the evacuation
/// bucket's ten-year chance is at least [`LEAVE_FIRST_P10`]; the plan includes a major hurricane
/// or a local tsunami; the home is in a storm-surge area; or a hazard that gives minutes of
/// warning ([`LEAVE_FIRST_FAST_HAZARDS`]: wildfire, a dam or levee failure) has a ten-year chance
/// of at least [`super::CARD_MIN_P10`]. The sentence is the evacuation bucket's first, the
/// action, the surge area when there is one, the consequence model's warning line for the fast
/// hazards when it names them, and a local tsunami's shaking rule.
pub(crate) fn leave_first(cx: &Ctx<'_>) -> Option<String> {
    let a = cx.a;
    let p = match a.bucket(BucketId::Evacuate).target {
        rr_types::Target::Evacuate { p_need_10yr, .. } => p_need_10yr,
        _ => 0.0,
    };
    let on = |id: &str| a.consequence.scenarios.iter().any(|s| s.on && s.id == id);
    let years = a.input.dials.horizon_years;
    let fast = LEAVE_FIRST_FAST_HAZARDS
        .iter()
        .any(|h| super::risks::chance(a.hazard_rate(*h), years) >= super::CARD_MIN_P10);
    let surge = surge_zone(cx);
    if p < LEAVE_FIRST_P10
        && !LEAVE_FIRST_SCENARIOS.iter().any(|id| on(id))
        && surge.is_none()
        && !fast
    {
        return None;
    }
    let mut action = String::new();
    if let Some(src) = &surge {
        action.push_str(&format!(
            "Parts of your area flood in a hurricane's storm surge.{} ",
            cite(src.as_str())
        ));
    }
    action.push_str(&format!(
        "Know your evacuation zone and where you would go; leave when told.{}",
        cite_all([
            &rr_types::CitationId::from("ready_gov_evacuation"),
            &rr_types::CitationId::from("ready_gov_hurricanes"),
        ])
    ));
    // The warning by cause for the fast hazards, as the consequence model words it ("For
    // wildfires, plan for as little as 15 minutes of warning; ...").
    let evac = a.bucket(BucketId::Evacuate);
    if let Some(line) = evac
        .frequency_sentences
        .iter()
        .find(|s| s.starts_with("For ") && s.contains("plan for as little as"))
        .filter(|_| fast)
    {
        action.push_str(&format!(" {line}{}", cite_all(&evac.sources)));
    }
    if on("local_tsunami") {
        action.push_str(&format!(
            " On the coast, strong shaking is the warning: walk to high ground as soon as it \
             stops.{}",
            cite("dogami_tsunami_faq")
        ));
    }
    Some(join(cx.bucket_frequency(BucketId::Evacuate), &action))
}

fn join(lead: Option<&str>, action: &str) -> String {
    match lead {
        Some(l) => format!("{} {action}", l.trim()),
        None => action.to_owned(),
    }
}
