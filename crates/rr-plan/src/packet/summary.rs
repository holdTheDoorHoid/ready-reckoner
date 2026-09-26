//! Section 1: the summary page. Who the plan is for, the date and setting, the step reached and
//! the step that is enough, the three sentences that matter most, and what the household should
//! be able to handle.

use rr_types::{
    BackupPower, BucketId, ClimateHorizon, Cooling, Heating, HousingKind, PlanItemKind, TierId,
    WaterLevel, WaterSource,
};

use super::text::{self, md};
use super::{Ctx, cite};
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
    format!("a {tenure} {kind}, with {}", text::join_and(&parts))
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
        "{label}, the kind that come about once in {} years (the 1-in-{} setting), in {climate}",
        d.return_period.years(),
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
    out.push("# Your preparedness packet".to_owned());
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
    if let Some(m) = a.budget.plan.done_month {
        let date = text::month_start(input.planning_date, m);
        when = format!(
            " At your budget, the plan gets there by month {m} ({}).",
            text::month_year(date)
        );
    } else if monthly <= 0.0 && input.finances.one_off_budget_usd <= 0.0 {
        when = " With no money set aside, the plan is the free steps; they still cover a lot."
            .to_owned();
    }
    out.push(format!(
        "**Where you are now:** {now}. **What is enough for your risks:** {}; each need stops at \
         the step that covers it.{when}",
        tier_words(recommended)
    ));
    out.push(String::new());

    out.push("### The three things that matter most".to_owned());
    out.push(String::new());
    for (i, s) in three_things(cx).iter().enumerate() {
        out.push(format!("{}. {s}", i + 1));
    }
    out.push(String::new());

    if !a.consequence.statement.is_empty() {
        out.push("### What your household should be able to handle".to_owned());
        out.push(String::new());
        for s in &a.consequence.statement {
            out.push(format!("- {}", md(s)));
        }
        out.push(String::new());
    }

    let first: Vec<&str> = a
        .budget
        .plan
        .months
        .first()
        .map(|m| {
            m.items
                .iter()
                .filter(|i| i.kind == PlanItemKind::FreeAction && !i.done)
                .map(|i| i.name.as_str())
                .collect()
        })
        .unwrap_or_default();
    if !first.is_empty() {
        out.push("### Start here".to_owned());
        out.push(String::new());
        out.push(format!(
            "These cost nothing and come first. This month's {} free steps are all under Your \
             plan.",
            first.len()
        ));
        out.push(String::new());
        for name in first.iter().take(3) {
            out.push(format!("- [ ] {}", md(name)));
        }
        out.push(String::new());
    }
    out.push(
        "Every number in this packet is explained, with its sources, in the sections that follow."
            .to_owned(),
    );
    out.push(String::new());
}

/// The three sentences that matter most: a threat paired with what to do about it.
fn three_things(cx: &Ctx<'_>) -> Vec<String> {
    let a = cx.a;
    let input = &a.input;
    let mut out: Vec<String> = Vec::new();
    let days = |b: BucketId| a.target_days(b);
    let (power, water) = (days(BucketId::Power), days(BucketId::WaterOut));

    // 1. Power and water.
    if power > 0.0 || water > 0.0 {
        let lead = if power > 0.0 {
            cx.bucket_frequency(BucketId::Power)
        } else {
            cx.bucket_frequency(BucketId::WaterOut)
        };
        let action = if power > 0.0 && (power - water).abs() < 1e-6 {
            format!(
                "Be ready to manage about {} at home with no power or tap water.",
                text::day_phrase(power)
            )
        } else if power > 0.0 && water > 0.0 {
            format!(
                "Be ready to manage about {} at home with no power, and about {} with no tap \
                 water.",
                text::day_phrase(power),
                text::day_phrase(water)
            )
        } else if power > 0.0 {
            format!(
                "Be ready to manage about {} at home with no power.",
                text::day_phrase(power)
            )
        } else {
            format!(
                "Be ready to manage about {} with no tap water.",
                text::day_phrase(water)
            )
        };
        out.push(join(lead, &action));
    }

    // 2. Food and medicine.
    let food = days(BucketId::Supplies);
    let meds = days(BucketId::Medication);
    let rx = input
        .people
        .iter()
        .any(|p| p.medical.daily_rx || p.medical.refrigerated_rx);
    if food > 0.0 {
        let meds_clause = if rx && meds > 0.0 {
            format!(", and {} of daily medicine on hand", text::day_phrase(meds))
        } else {
            String::new()
        };
        out.push(join(
            cx.bucket_frequency(BucketId::Supplies),
            &format!(
                "Keep about {} of food you normally eat{meds_clause}.",
                text::day_phrase(food)
            ),
        ));
    }

    // 3. One event that drives the answer, the biggest long disruption, or leaving home.
    let income_months = match a.bucket(BucketId::Income).target {
        rr_types::Target::Months { value, .. } => f64::from(value),
        _ => 0.0,
    };
    if let Some(w) = a.warnings.iter().find(|w| w.id.starts_with("cliff_")) {
        let scenario = w
            .related
            .first()
            .and_then(|id| a.consequence.scenarios.iter().find(|s| &s.id == id));
        let tail = match scenario {
            Some(s) if s.on => " The plan includes it; Your targets shows what it changes.",
            Some(_) => " The plan leaves it out; Your targets shows what it would change.",
            None => " Your targets shows how much it moves the numbers.",
        };
        out.push(format!("{}{tail}", w.message));
    } else if input.finances.income.earners > 0 && income_months > 0.0 {
        let lead = a
            .bucket(BucketId::Income)
            .frequency_sentences
            .first()
            .map(String::as_str);
        out.push(join(
            lead,
            &format!(
                "Losing a paycheck is the longest disruption most households face. Aim for about \
                 {} of expenses in savings over time, apart from this supplies budget.",
                text::months_phrase(income_months)
            ),
        ));
    } else {
        let evac = a.bucket(BucketId::Evacuate);
        if let Some(lead) = evac.frequency_sentences.first() {
            out.push(join(
                Some(lead),
                "Keep a packed go-bag for each person near the door.",
            ));
        }
    }
    if out.len() < 3 {
        if let Some(p) = a
            .hazards
            .profiles
            .iter()
            .find(|p| p.display == rr_types::HazardDisplay::Ranked)
        {
            out.push(format!(
                "{} The plan starts with the free steps that help most with it.",
                p.frequency_sentence
            ));
        }
    }
    out.into_iter().take(3).map(|s| md_sentence(&s)).collect()
}

fn join(lead: Option<&str>, action: &str) -> String {
    match lead {
        Some(l) => format!("{} {action}", l.trim()),
        None => action.to_owned(),
    }
}

/// Escapes a whole sentence for Markdown (numbers in brackets and dashes stay readable).
fn md_sentence(s: &str) -> String {
    md(s)
}
