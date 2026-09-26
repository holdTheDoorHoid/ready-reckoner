//! Guardrails (DESIGN §4.7, PRINCIPLES §11): the plan looks off, so say so, and never block.
//!
//! | id | severity | fires when |
//! | --- | --- | --- |
//! | `zero_budget` | note | no monthly and no one-off money |
//! | `device_power_plan` | warn | a powered medical device, no backup power at home, and no device-power item by month 3 |
//! | `cold_chain_plan` | warn | refrigerated medicine, and by month 3 no cooler for it, or, where it needs a power source (a power target of 2 days or more and no backup power: rr-supply's `power_for_cold_medicine` line), that power arrives after month 3 (the design gives no month; this reuses the device rule's) |
//! | `cold_chain_power` | warn | refrigerated medicine that needs a power source, and nothing in the plan ever covers that power (contract v2; REVIEW S1) |
//! | `no_stored_water_by_month_3` | warn | free steps (refilled drink bottles) leave the stored-water need short, and no stored water is owned or bought by month 3 |
//! | `smoke_alarms_landlord` | warn | renters with no working smoke alarms: the plan buys none, because the landlord comes first (round-2 review RR-P16), so it says so |
//! | `evacuation_no_go_bag` | warn | a ten-year chance of having to leave of 10 % or more (`Prior`) and no go-bag by month 6 (`Prior`) |
//! | `insurance_flood` / `insurance_quake` | warn | an owner in a flood- or quake-prone area without that policy |
//! | `cliff_<bucket>` | note | `rr-consequence` found one rare event driving the bucket's target |
//! | `uncovered_<bucket>` | note | the plan ran out of things to buy but the bucket is still short of its goal (a catalogue gap) |
//! | `surge_zone_stay_home` | warn | a storm-surge zone ([`is_surge_zone`]) or a ten-year chance of having to leave of at least [`LEAVING_LIKELY_P10`] (`Prior`), and no step in the plan is about leaving (contract v2; REVIEW S2) |
//! | `benefit_lapse` | warn | the household relies on federal pay or a benefit (`Finances::benefits`) and has less than [`BENEFIT_BUFFER_DAYS`] of food (or its food target, if shorter) at the end of month [`BENEFIT_BUFFER_BY_MONTH`] (contract v2; REVIEW H7) |
//! | `plan_too_long` | warn | the plan in the normal buying order would run past 36 months: bare-minimum mode takes over, and `related` lists what falls beyond three years even so (contract v2; REVIEW R6) |
//! | `no_raw_water_source` | warn | a water filter is in the plan, the home is not on a well, no raw water source is named, and no rain barrel is planned (contract v2; REVIEW S6) |
//! | `no_cooking_capability` | warn | the household needs a way to cook without power (rr-supply offers the camp stove: no gas range or wood stove covers it) and the plan never gets one (contract v2; REVIEW K1) |
//! | `simultaneous_need` | note | the plan ran out of things to buy, but one event that sets a target would need more stored water, food or power at once than the plan holds (DESIGN §4.7's simultaneous-need check, from `rr-consequence`) |

use rr_types::{
    BackupPower, BucketId, HazardId, ItemId, PlanInput, RawWaterSource, Tenure, Warning,
    WarningSeverity, WaterSource,
};

use crate::explain::{days_text, households_phrase, short_name};
use crate::input::{GuardrailContext, Risks};
use crate::value::{annual_rate_from_10yr, per_100};

/// Month by which a powered medical device (and refrigerated medicine) should have a plan.
pub const DEVICE_PLAN_BY_MONTH: u16 = 3;

/// The part of the power bucket that keeps refrigerated medicine cold: `rr-plan` names parts
/// after `rr-supply`'s item classes, and the power station line for refrigerated medicine that
/// needs a power source has the class `power_for_cold_medicine` (`rr_supply::COLD_MEDICINE_POWER_CLASS`).
/// The cold-chain guardrail stays on until this part is covered.
pub const COLD_MEDICINE_POWER_PART: &str = "power for cold medicine";

/// Ten-year chance of having to leave at or above which a household counts as evacuation-heavy
/// (`Prior`).
pub const EVACUATION_HEAVY_P10: f64 = 0.10;

/// Month by which an evacuation-heavy household should have a go-bag (`Prior`).
pub const GO_BAG_BY_MONTH: u16 = 6;

/// Month by which a household with a no-water target should have stored water beyond the free
/// step of refilling drink bottles (PRINCIPLES §11: "no water at all after month three").
pub const STORED_WATER_BY_MONTH: u16 = 3;

/// A ZIP code with at least this share inside the Category 1–3 storm-surge zone is a surge zone
/// for `surge_zone_stay_home` (`Prior`: most of the area floods in a category 3 storm).
pub const SURGE_ZIP_SHARE: f64 = 0.5;

/// A ten-year chance of having to leave home at or above this makes leaving likely for
/// `surge_zone_stay_home` (`Prior`, a quarter; the go-bag guardrail's evacuation-heavy line is
/// 10 %).
pub const LEAVING_LIKELY_P10: f64 = 0.25;

/// Days of food a household that relies on pay or benefits a lapse can stop should have by
/// [`BENEFIT_BUFFER_BY_MONTH`]: the typical length of a lapse in food benefits in the consequence
/// model (`rr-consequence`'s benefit-interruption row: median 10 days, `Prior`, from the November
/// 2025 SNAP suspension, `me_dhhs_snap_2025`), or the household's food target if shorter.
pub const BENEFIT_BUFFER_DAYS: f64 = 10.0;

/// The month by which a benefit household should have its food buffer (the same month-3 line as
/// the stored-water and device guardrails).
pub const BENEFIT_BUFFER_BY_MONTH: u16 = 3;

/// The stored supplies the simultaneous-need check compares with an event's needs: water, food
/// and power ("the budget crate compares the stored water, food and fuel with it", RISK_MODEL).
pub const SIMULTANEOUS_BUCKETS: [BucketId; 3] =
    [BucketId::WaterOut, BucketId::Supplies, BucketId::Power];

/// A need counts in the simultaneous-need check when the event brings it at least this often
/// (`Prior`: more often than not).
pub const SIMULTANEOUS_MIN_CHANCE: f64 = 0.5;

/// A need counts as short when the plan holds less than it by more than this many days (half a
/// day: the day ladder's first step).
pub const SIMULTANEOUS_SLACK_DAYS: f64 = 0.5;

/// Whether the home is in a storm-surge zone: the ZIP code's surge share when the optional surge
/// pack gives it, otherwise the county's core-pack proxy class `high`. The packet's
/// evacuate-first rule can call this too, so the two never disagree.
pub fn is_surge_zone(zip_share: Option<f64>, county_class: Option<&str>) -> bool {
    match zip_share {
        Some(s) if s.is_finite() => s >= SURGE_ZIP_SHARE,
        _ => county_class == Some("high"),
    }
}

/// What the allocator found, for the checks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Facts {
    /// First month a device-power item is in hand (owned, free, or bought), if ever.
    pub device_power_month: Option<u16>,
    /// First month a cold-chain item (a cooler for refrigerated medicine) is in hand.
    pub cooler_month: Option<u16>,
    /// Refrigerated medicine needs a power source here ([`COLD_MEDICINE_POWER_PART`] exists).
    pub cold_power_needed: bool,
    /// First month something that covers that power is in hand, if ever.
    pub cold_power_month: Option<u16>,
    /// First month a go-bag is in hand.
    pub go_bag_month: Option<u16>,
    /// The household has a no-water target that free steps and what it has leave short.
    pub water_needed: bool,
    /// First month stored water beyond free steps is in hand (owned or bought), if ever.
    pub stored_water_month: Option<u16>,
    /// Buckets still short of their goal when the plan ran out of things to buy.
    pub uncovered_when_stopped: Vec<BucketId>,
    /// Some step in the plan (owned, done, free or bought) is about leaving home.
    pub leaving_in_plan: bool,
    /// Days of food at the end of month [`BENEFIT_BUFFER_BY_MONTH`] (the plan's own timeline).
    pub food_at_month_3: Option<f64>,
    /// The food (supplies) target in days, when there is one.
    pub food_target: Option<f64>,
    /// A water filter is in the plan (owned or bought).
    pub filter_in_plan: bool,
    /// A rain barrel (a raw water source) is in the plan.
    pub rain_in_plan: bool,
    /// The household needs a way to cook without power (the catalogue offers one).
    pub cooking_needed: bool,
    /// The plan gets one (owned or bought).
    pub cooking_in_plan: bool,
    /// Events that would need more stored water, food or power at once than the plan holds.
    pub shortfalls: Vec<Shortfall>,
    /// The normal plan runs past three years (bare-minimum mode).
    pub too_long: Option<TooLong>,
}

/// One event the simultaneous-need check finds short.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shortfall {
    /// The event in words.
    pub event: String,
    /// Its hazard.
    pub hazard: HazardId,
    /// (bucket, days the event needs at once, days the plan holds).
    pub short: Vec<(BucketId, f64, f64)>,
}

/// What `plan_too_long` says.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TooLong {
    /// When the plan in the normal buying order runs out of things to buy (`None`: not within the
    /// plan horizon).
    pub full_plan_month: Option<u16>,
    /// When the bare-minimum kit is complete in the plan shown.
    pub minimum_month: Option<u16>,
    /// What falls beyond three years even so: (item, name in lower case).
    pub deferred: Vec<(ItemId, String)>,
}

fn warning(
    id: impl Into<String>,
    severity: WarningSeverity,
    message: String,
    why: &str,
    related: &[&str],
) -> Warning {
    Warning {
        id: id.into(),
        severity,
        message,
        why: why.to_owned(),
        related: related.iter().map(|s| (*s).to_owned()).collect(),
    }
}

pub(crate) fn check(
    household: &PlanInput,
    context: &GuardrailContext,
    risks: &Risks,
    facts: &Facts,
) -> Vec<Warning> {
    let mut out = Vec::new();
    let f = &household.finances;
    let late = |m: Option<u16>, by: u16| m.is_none_or(|m| m > by);

    if f.monthly_budget_usd <= 0.0 && f.one_off_budget_usd <= 0.0 {
        out.push(warning(
            "zero_budget",
            WarningSeverity::Note,
            "Your plan uses free steps only.".into(),
            "With no money set aside, the plan lists the free steps that protect you most. Even a \
             few dollars a month buys water and light first.",
            &[],
        ));
    }

    let device = household
        .people
        .iter()
        .any(crate::weights::has_powered_device);
    if device
        && household.housing.backup_power == BackupPower::None
        && late(facts.device_power_month, DEVICE_PLAN_BY_MONTH)
    {
        out.push(warning(
            "device_power_plan",
            WarningSeverity::Warn,
            "No backup power for a medical device by month 3.".into(),
            "A breathing machine or other powered device stops when the power goes out. Ask the \
             device supplier which battery or backup works with it, and ask your power company \
             about its medical-needs list.",
            &["power"],
        ));
    }

    let cold = household.people.iter().any(|p| p.medical.refrigerated_rx);
    let power_late = facts.cold_power_needed
        && facts
            .cold_power_month
            .is_some_and(|m| m > DEVICE_PLAN_BY_MONTH);
    if cold && (late(facts.cooler_month, DEVICE_PLAN_BY_MONTH) || power_late) {
        out.push(warning(
            "cold_chain_plan",
            WarningSeverity::Warn,
            "No way to keep refrigerated medicine cool through a power cut by month 3.".into(),
            "Insulin keeps working out of the fridge for a while if it stays cool, but a home \
             without power in hot weather can get too warm, and other medicines must stay in the \
             fridge. A cooler bag with cold packs helps only for a short time; a battery power \
             station can keep a small cooler or the fridge running. Ask your pharmacist how long \
             yours can stay out of the fridge.",
            &["medication", "power"],
        ));
    }
    if cold && facts.cold_power_needed && facts.cold_power_month.is_none() {
        out.push(warning(
            Warning::COLD_CHAIN_POWER,
            WarningSeverity::Warn,
            "Refrigerated medicine needs a power source in a long power cut, and the plan has none."
                .into(),
            "A cooler bag keeps medicine cool only for a short time. For a longer power cut, a \
             battery power station keeps a small cooler or the fridge running, and a 12-volt fridge \
             can run from the car. Ask your pharmacist how long yours can stay out of the fridge.",
            &["medication", "power"],
        ));
    }

    // Renters with no smoke alarms: the plan budgets none (the landlord, the fire department or
    // the Red Cross come first), so the packet must say so where it cannot be missed.
    if !household.housing.alarms.smoke && household.housing.tenure == Tenure::Rent {
        out.push(warning(
            "smoke_alarms_landlord",
            WarningSeverity::Warn,
            "Your home has no working smoke alarms: ask your landlord to put them in.".into(),
            "Smoke alarms give the warning that gets people out of a house fire. Ask your landlord \
             in writing first. If that does not work, ask your fire department or the Red Cross, \
             which install free smoke alarms in many places. Buy them yourself only if nobody can.",
            &["fire"],
        ));
    }

    if facts.water_needed && late(facts.stored_water_month, STORED_WATER_BY_MONTH) {
        out.push(warning(
            "no_stored_water_by_month_3",
            WarningSeverity::Warn,
            "No stored water beyond refilled bottles by month 3.".into(),
            "Refilled drink bottles are a good start, but they hold only a little. Water is the one \
             supply you cannot go long without: a few gallons of bottled water or a water jug is the \
             next step, and costs little.",
            &["water_out"],
        ));
    }

    let p_leave = risks.p_need_10yr(BucketId::Evacuate);
    if p_leave >= EVACUATION_HEAVY_P10 && late(facts.go_bag_month, GO_BAG_BY_MONTH) {
        let years = household.dials.horizon_years.max(1);
        let n = per_100(annual_rate_from_10yr(p_leave), f64::from(years));
        out.push(warning(
            "evacuation_no_go_bag",
            WarningSeverity::Warn,
            format!(
                "No go-bag in the first six months. {} have to leave home quickly in the next {years} years.",
                households_phrase(n)
            ),
            "A packed bag by the door turns a scramble into a few minutes' work.",
            &["evacuate"],
        ));
    }

    let owner = household.housing.tenure == Tenure::Own;
    if owner && context.flood_zone && !f.insurance.flood {
        out.push(warning(
            "insurance_flood",
            WarningSeverity::Warn,
            "You own your home in a flood-prone area and have no flood insurance.".into(),
            "Standard home policies do not cover flooding, and a new flood policy usually has a \
             waiting period before it starts. Get a quote before storm season.",
            &["home_loss"],
        ));
    }
    if owner && context.quake_zone && !f.insurance.earthquake {
        out.push(warning(
            "insurance_quake",
            WarningSeverity::Warn,
            "You own your home in an earthquake-prone area and have no earthquake insurance."
                .into(),
            "Standard home policies do not cover earthquake damage. Compare the cost of a policy \
             with the repairs you could afford yourself.",
            &["home_loss"],
        ));
    }

    // Surge zone or a likely evacuation, and nothing in the plan about leaving.
    let surge = is_surge_zone(
        context.surge_zip_share,
        context.surge_county_class.as_deref(),
    );
    let leaving_likely = p_leave >= LEAVING_LIKELY_P10;
    if (surge || leaving_likely) && !facts.leaving_in_plan {
        let message = if surge {
            "Your home is in a storm-surge zone, and the plan never says when to leave."
        } else {
            "Leaving home is likely where you live, and the plan never says when to leave."
        };
        out.push(warning(
            Warning::SURGE_ZONE_STAY_HOME,
            WarningSeverity::Warn,
            message.into(),
            "Water pushed ashore by a hurricane can flood a whole neighbourhood in minutes, and \
             supplies at home do not make it safe to stay. When officials say to leave, go. Look \
             up your evacuation zone and pick where you would go.",
            &["evacuate"],
        ));
    }

    // Pay or benefits that can stop, and no food buffer by month 3.
    if !f.benefits.is_empty() {
        let need = facts
            .food_target
            .map_or(BENEFIT_BUFFER_DAYS, |t| t.min(BENEFIT_BUFFER_DAYS));
        let have = facts.food_at_month_3.unwrap_or(0.0);
        if facts.food_target.is_some() && need > 0.0 && have + 1e-6 < need {
            out.push(warning(
                Warning::BENEFIT_LAPSE,
                WarningSeverity::Warn,
                "No food set aside by month 3 in case pay or benefits stop.".into(),
                "Government pay and food benefits have stopped before, for days to weeks at a \
                 time, during shutdowns and funding lapses. A small store of food built early \
                 bridges the gap; a food bank or 211 can help too.",
                &["supplies", "benefit_interruption"],
            ));
        }
    }

    // A filter with nothing to filter.
    let source_named = household.housing.water == WaterSource::Well
        || household
            .housing
            .raw_water_source
            .is_some_and(|s| s != RawWaterSource::None)
        || facts.rain_in_plan;
    if facts.filter_in_plan && !source_named {
        out.push(warning(
            Warning::NO_RAW_WATER_SOURCE,
            WarningSeverity::Warn,
            "A water filter is in the plan, but no water source to filter is named.".into(),
            "A filter makes raw water safe only if you have some: a well, a creek or pond nearby, \
             a rain barrel or a neighbour's well. Name one in your home details, or store more \
             water instead.",
            &["water_out"],
        ));
    }

    // No way to cook or boil water without power.
    if facts.cooking_needed && !facts.cooking_in_plan {
        out.push(warning(
            Warning::NO_COOKING_CAPABILITY,
            WarningSeverity::Warn,
            "No way to cook or boil water without power.".into(),
            "An electric stove stops in a power cut. A camp stove, used outdoors and never \
             inside, cooks stored food and boils water; unscented bleach makes water safe without \
             heat.",
            &["supplies", "water_boil"],
        ));
    }

    // The full plan would take more than three years: bare-minimum mode.
    if let Some(t) = &facts.too_long {
        let mut why = String::from(
            "The smallest kit that covers three days of water, light, warmth and medicine comes \
             first",
        );
        match t.minimum_month {
            Some(0) => why.push_str(" and is complete this month"),
            Some(m) => why.push_str(&format!(" and is complete by month {m}")),
            None => {}
        }
        why.push_str("; everything else follows in the usual order");
        match t.full_plan_month {
            Some(m) => why.push_str(&format!(", and the whole plan takes until month {m}.")),
            None => why.push_str(", and the whole plan runs past ten years."),
        }
        if !t.deferred.is_empty() {
            let names: Vec<&str> = t.deferred.iter().take(4).map(|(_, n)| n.as_str()).collect();
            let more = if t.deferred.len() > names.len() {
                " and more"
            } else {
                ""
            };
            why.push_str(&format!(
                " Beyond three years at this budget: {}{more}.",
                join_list(&names)
            ));
        }
        why.push_str(" A little more each month, or a less cautious setting, brings them closer.");
        out.push(Warning {
            id: Warning::PLAN_TOO_LONG.to_owned(),
            severity: WarningSeverity::Warn,
            message: "At this budget the full plan would take more than three years, so it starts \
                      with the bare minimum."
                .into(),
            why,
            related: t
                .deferred
                .iter()
                .map(|(id, _)| id.as_str().to_owned())
                .collect(),
        });
    }

    // One event that needs more stored water, food or power at once than the plan holds.
    if let Some(s) = facts.shortfalls.iter().max_by(|a, b| {
        a.short
            .len()
            .cmp(&b.short.len())
            .then(b.event.cmp(&a.event))
    }) {
        let parts: Vec<String> = s
            .short
            .iter()
            .map(|(b, need, have)| {
                format!(
                    "about {} {} (the plan holds {})",
                    days_text(*need),
                    simultaneous_words(*b),
                    days_text(*have)
                )
            })
            .collect();
        let mut related: Vec<String> = s
            .short
            .iter()
            .map(|(b, _, _)| b.as_str().to_owned())
            .collect();
        related.push(s.hazard.as_str().to_owned());
        out.push(warning(
            "simultaneous_need",
            WarningSeverity::Note,
            format!(
                "One event could need more at once than the plan stores: {}.",
                s.event
            ),
            &format!(
                "At the severity that sets one of your targets, it would also mean {}. Storing a \
                 little more, or a way to make more (a water filter, a camp stove, a way to \
                 recharge), closes the gap; it is about storage space as much as money.",
                join_list(&parts.iter().map(String::as_str).collect::<Vec<_>>())
            ),
            &related.iter().map(String::as_str).collect::<Vec<_>>(),
        ));
    }

    for cliff in &context.cliffs {
        let why = match cliff.bucket {
            BucketId::WaterOut | BucketId::WaterBoil => {
                "The plan buys the first days first, because they are needed most often. For the \
                 long tail, a water filter and a water source can cost less than storing it all."
            }
            BucketId::Power => {
                "The plan buys the first days first, because they are needed most often. For the \
                 long tail, a way to recharge (such as a small solar panel) can cost less than \
                 stored fuel."
            }
            _ => {
                "The plan buys the first days first, because they are needed most often. For the \
                 long tail, a way to make more can cost less than storing it all."
            }
        };
        out.push(warning(
            format!("cliff_{}", cliff.bucket),
            WarningSeverity::Note,
            format!(
                "Most of your target for {} comes from one rare event: {}.",
                short_name(cliff.bucket),
                cliff.driver
            ),
            why,
            &[cliff.bucket.as_str()],
        ));
    }

    for b in &facts.uncovered_when_stopped {
        out.push(warning(
            format!("uncovered_{b}"),
            WarningSeverity::Note,
            format!("Nothing in the plan covers {} yet.", short_name(*b)),
            "The catalogue has nothing that adds days here, so this goal is not met.",
            &[b.as_str()],
        ));
    }
    out
}

/// What an event's need is, in a few words after "about N days".
fn simultaneous_words(b: BucketId) -> &'static str {
    match b {
        BucketId::WaterOut => "without tap water",
        BucketId::Supplies => "without reaching a store",
        BucketId::Power => "without power",
        _ => short_name(b),
    }
}

/// "a", "a and b", "a, b and c".
fn join_list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}
