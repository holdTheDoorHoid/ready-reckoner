//! Guardrails (DESIGN §4.7, PRINCIPLES §11): the plan looks off, so say so, and never block.
//!
//! | id | severity | fires when |
//! | --- | --- | --- |
//! | `zero_budget` | note | no monthly and no one-off money |
//! | `device_power_plan` | warn | a powered medical device, no backup power at home, and no device-power item by month 3 |
//! | `cold_chain_plan` | warn | refrigerated medicine and no way to keep it cold by month 3 (the design gives no month; this reuses the device rule's) |
//! | `no_water_after_month_1` | warn | no stored water at the end of month 1 |
//! | `evacuation_no_go_bag` | warn | a ten-year chance of having to leave of 10 % or more (`Prior`) and no go-bag by month 6 (`Prior`) |
//! | `insurance_flood` / `insurance_quake` | warn | an owner in a flood- or quake-prone area without that policy |
//! | `cliff_<bucket>` | note | `rr-consequence` found one rare event driving the bucket's target |
//! | `uncovered_<bucket>` | note | the plan ran out of things to buy but the bucket is still short of its goal (a catalogue gap) |

use rr_types::{BackupPower, BucketId, PlanInput, Tenure, Warning, WarningSeverity};

use crate::explain::{households_phrase, short_name};
use crate::input::{GuardrailContext, Risks};
use crate::value::{annual_rate_from_10yr, per_100};

/// Month by which a powered medical device (and refrigerated medicine) should have a plan.
pub const DEVICE_PLAN_BY_MONTH: u16 = 3;

/// Ten-year chance of having to leave at or above which a household counts as evacuation-heavy
/// (`Prior`).
pub const EVACUATION_HEAVY_P10: f64 = 0.10;

/// Month by which an evacuation-heavy household should have a go-bag (`Prior`).
pub const GO_BAG_BY_MONTH: u16 = 6;

/// What the allocator found, for the checks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Facts {
    /// First month a device-power item is in hand (owned, free, or bought), if ever.
    pub device_power_month: Option<u16>,
    /// First month a cold-chain item is in hand.
    pub cold_chain_month: Option<u16>,
    /// First month a go-bag is in hand.
    pub go_bag_month: Option<u16>,
    /// Days of `water_out` covered at the end of month 1.
    pub water_after_month_1: f64,
    /// Buckets still short of their goal when the plan ran out of things to buy.
    pub uncovered_when_stopped: Vec<BucketId>,
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
    if cold && late(facts.cold_chain_month, DEVICE_PLAN_BY_MONTH) {
        out.push(warning(
            "cold_chain_plan",
            WarningSeverity::Warn,
            "No way to keep refrigerated medicine cold by month 3.".into(),
            "Some medicines, like insulin, spoil without cooling in a long power cut. A cooler and \
             a plan for ice cost little. Ask your pharmacist how long yours can stay out of the \
             fridge.",
            &["medication", "power"],
        ));
    }

    if facts.water_after_month_1 <= 1e-9 {
        out.push(warning(
            "no_water_after_month_1",
            WarningSeverity::Warn,
            "No stored water after the first month.".into(),
            "Water is the one supply you cannot go long without. Filling clean bottles from the tap \
             costs nothing.",
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
