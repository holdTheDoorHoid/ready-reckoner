//! Harm weights w_b (DESIGN §4.7, research risk-model §4.1): how much one uncovered day in a
//! bucket hurts, relative to one day without food. They are expert priors (`Prior`), shown in the
//! expert view with the one-line reason in [`HarmWeightRule::why`].
//!
//! The design fixes: water (no tap water, boil notices) 3; anyone's daily prescription 3 (the
//! planner's decision of 2026-09-25, widening DESIGN §4.7's "a dependent's medication":
//! interrupting insulin, anticonvulsants or psychiatric medicine is dangerous whoever takes it);
//! heat or cold with a vulnerable member 2; food and supplies, power and communications 1; a
//! powered medical device triples `power`. Rows the design does not state are marked
//! `design: false`: medication for a household with no daily prescription and `thermal` with no
//! vulnerable member use the base weight 1; readiness buckets use 1 except `evacuate`, which uses
//! the research prototype's 2 (confirmed by the planner); money buckets are 0 because the
//! supplies budget never pays for them.

use rr_types::{AgeBand, BucketId, Mobility, Person, PlanInput, PoweredDevice};

/// Citation id for the harm weights: an expert prior, rendered as an estimate. The content
/// workstream owns `content/citations.toml`; this id must exist there with `prior = true`.
pub const HARM_WEIGHT_CITATION: &str = "prior_harm_weights";

/// When a harm-weight row applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applies {
    /// Always (the base row for the bucket).
    Always,
    /// Someone takes a prescription medicine every day, or one that must stay cold.
    DailyPrescription,
    /// Someone aged 65 or over, under 4, pregnant or nursing, with limited mobility, or using a
    /// powered medical device lives here.
    VulnerableToHeatOrCold,
    /// Someone uses a powered medical device (a multiplier on the bucket's base weight).
    PoweredDevice,
}

/// One row of the harm-weight table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmWeightRule {
    /// Stable id for the expert view and `explain`.
    pub id: &'static str,
    /// The bucket.
    pub bucket: BucketId,
    /// The weight, or the multiplier when `applies` is [`Applies::PoweredDevice`].
    pub weight: f64,
    /// When the row applies.
    pub applies: Applies,
    /// Whether DESIGN §4.7 states this row (`false`: this crate's fallback, flagged for review).
    pub design: bool,
    /// Why, in one plain sentence.
    pub why: &'static str,
}

/// The harm-weight table, most specific row first within each bucket.
pub const HARM_WEIGHTS: &[HarmWeightRule] = &[
    HarmWeightRule {
        id: "water_out",
        bucket: BucketId::WaterOut,
        weight: 3.0,
        applies: Applies::Always,
        design: true,
        why: "Nobody can go long without drinking water, and nothing else can stand in for it.",
    },
    HarmWeightRule {
        id: "water_boil",
        bucket: BucketId::WaterBoil,
        weight: 3.0,
        applies: Applies::Always,
        design: true,
        why: "Drinking untreated water during a boil notice can make the whole household sick.",
    },
    HarmWeightRule {
        id: "medication_rx",
        bucket: BucketId::Medication,
        weight: 3.0,
        applies: Applies::DailyPrescription,
        design: true,
        why: "Stopping a daily prescription, such as insulin, seizure or mental-health medicine, can quickly become an emergency, whoever takes it.",
    },
    HarmWeightRule {
        id: "medication",
        bucket: BucketId::Medication,
        weight: 1.0,
        applies: Applies::Always,
        design: false,
        why: "Nobody here takes a daily prescription, so running short of medical supplies is a hardship rather than an emergency.",
    },
    HarmWeightRule {
        id: "thermal_vulnerable",
        bucket: BucketId::Thermal,
        weight: 2.0,
        applies: Applies::VulnerableToHeatOrCold,
        design: true,
        why: "Heat and cold hurt older adults, babies, pregnant women and people with health conditions first.",
    },
    HarmWeightRule {
        id: "thermal",
        bucket: BucketId::Thermal,
        weight: 1.0,
        applies: Applies::Always,
        design: false,
        why: "Healthy adults can usually move to a cooler or warmer place before heat or cold harms them.",
    },
    HarmWeightRule {
        id: "supplies",
        bucket: BucketId::Supplies,
        weight: 1.0,
        applies: Applies::Always,
        design: true,
        why: "Most households can stretch food for a few days; going without is hard before it is dangerous.",
    },
    HarmWeightRule {
        id: "power_device",
        bucket: BucketId::Power,
        weight: 3.0,
        applies: Applies::PoweredDevice,
        design: true,
        why: "Someone here relies on electricity for a medical device, so a power cut is a health risk, not an inconvenience.",
    },
    HarmWeightRule {
        id: "power",
        bucket: BucketId::Power,
        weight: 1.0,
        applies: Applies::Always,
        design: true,
        why: "A power cut is disruptive, but most of its harm shows up in water, heat and medicine, which carry their own weights.",
    },
    HarmWeightRule {
        id: "comms",
        bucket: BucketId::Comms,
        weight: 1.0,
        applies: Applies::Always,
        design: true,
        why: "Losing phone and internet cuts you off from alerts and help, but rarely causes harm by itself.",
    },
    HarmWeightRule {
        id: "evacuate",
        bucket: BucketId::Evacuate,
        weight: 2.0,
        applies: Applies::Always,
        design: false,
        why: "Leaving in minutes without a bag or a plan is when people get hurt or lose what they need most (research prototype weight).",
    },
    HarmWeightRule {
        id: "get_home",
        bucket: BucketId::GetHome,
        weight: 1.0,
        applies: Applies::Always,
        design: false,
        why: "Readiness items carry their harm in the per-use estimate, so the weight stays at the base value.",
    },
    HarmWeightRule {
        id: "medical_emergency",
        bucket: BucketId::MedicalEmergency,
        weight: 1.0,
        applies: Applies::Always,
        design: false,
        why: "Readiness items carry their harm in the per-use estimate, so the weight stays at the base value.",
    },
    HarmWeightRule {
        id: "fire",
        bucket: BucketId::Fire,
        weight: 1.0,
        applies: Applies::Always,
        design: false,
        why: "Readiness items carry their harm in the per-use estimate, so the weight stays at the base value.",
    },
    HarmWeightRule {
        id: "security",
        bucket: BucketId::Security,
        weight: 1.0,
        applies: Applies::Always,
        design: false,
        why: "Readiness items carry their harm in the per-use estimate, so the weight stays at the base value.",
    },
    HarmWeightRule {
        id: "income",
        bucket: BucketId::Income,
        weight: 0.0,
        applies: Applies::Always,
        design: true,
        why: "Income is covered by the savings track, never by the supplies budget.",
    },
    HarmWeightRule {
        id: "home_loss",
        bucket: BucketId::HomeLoss,
        weight: 0.0,
        applies: Applies::Always,
        design: true,
        why: "A damaged home is covered by insurance and savings, never by the supplies budget.",
    },
];

/// The harm weight of `bucket` for this household, with the rows that produced it (the base row
/// first, then any multiplier).
pub fn harm_weight(bucket: BucketId, household: &PlanInput) -> (f64, Vec<&'static HarmWeightRule>) {
    let mut base: Option<&'static HarmWeightRule> = None;
    let mut multiplier: Option<&'static HarmWeightRule> = None;
    for rule in HARM_WEIGHTS.iter().filter(|r| r.bucket == bucket) {
        let holds = applies(rule.applies, household);
        match rule.applies {
            Applies::PoweredDevice => {
                if holds && multiplier.is_none() {
                    multiplier = Some(rule);
                }
            }
            _ => {
                if holds && base.is_none() {
                    base = Some(rule);
                }
            }
        }
    }
    let mut rules = Vec::new();
    let mut weight = 1.0;
    if let Some(b) = base {
        weight = b.weight;
        rules.push(b);
    }
    if let Some(m) = multiplier {
        weight *= m.weight;
        rules.push(m);
    }
    (weight, rules)
}

fn applies(a: Applies, household: &PlanInput) -> bool {
    let people = &household.people;
    match a {
        Applies::Always => true,
        Applies::DailyPrescription => people
            .iter()
            .any(|p| p.medical.daily_rx || p.medical.refrigerated_rx),
        Applies::VulnerableToHeatOrCold => people.iter().any(is_vulnerable_to_heat_or_cold),
        Applies::PoweredDevice => people.iter().any(has_powered_device),
    }
}

/// Someone heat and cold hurt first: aged 65 or over, under 4, pregnant or nursing, with limited
/// mobility, or using a powered medical device.
pub fn is_vulnerable_to_heat_or_cold(p: &Person) -> bool {
    matches!(
        p.age_band,
        AgeBand::Infant | AgeBand::Toddler | AgeBand::Senior
    ) || p.pregnant_or_nursing
        || p.medical.mobility != Mobility::None
        || has_powered_device(p)
}

/// The person uses a medical device that needs electricity.
pub fn has_powered_device(p: &Person) -> bool {
    !matches!(p.medical.powered_device, PoweredDevice::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rr_types::fixtures;

    fn w(bucket: BucketId, fixture: &str) -> f64 {
        harm_weight(bucket, &fixtures::get(fixture).unwrap()).0
    }

    #[test]
    fn design_weights_for_the_fixture_households() {
        // Philadelphia: a 70-year-old on daily medicine with limited mobility.
        let phl = "philadelphia-renters-4";
        assert_eq!(w(BucketId::WaterOut, phl), 3.0);
        assert_eq!(w(BucketId::WaterBoil, phl), 3.0);
        assert_eq!(w(BucketId::Medication, phl), 3.0);
        assert_eq!(w(BucketId::Thermal, phl), 2.0);
        assert_eq!(w(BucketId::Supplies, phl), 1.0);
        assert_eq!(w(BucketId::Power, phl), 1.0);
        assert_eq!(w(BucketId::Comms, phl), 1.0);
        assert_eq!(w(BucketId::Income, phl), 0.0);
        // Phoenix: one adult with a CPAP and a daily prescription: power tripled; their own
        // medicine weighs 3 like anyone's; the device also makes them vulnerable to heat.
        let phx = "phoenix-apartment-cpap-1";
        assert_eq!(w(BucketId::Power, phx), 3.0);
        assert_eq!(w(BucketId::Medication, phx), 3.0);
        assert_eq!(w(BucketId::Thermal, phx), 2.0);
        // Chicago student: nobody vulnerable, no prescription.
        let chi = "chicago-student-zero-budget-1";
        assert_eq!(w(BucketId::Thermal, chi), 1.0);
        assert_eq!(w(BucketId::Medication, chi), 1.0);
        assert_eq!(w(BucketId::Evacuate, chi), 2.0);
        // A working-age adult on refrigerated medicine (insulin) counts too.
        let mut h = fixtures::get(chi).unwrap();
        h.people[0].medical.refrigerated_rx = true;
        assert_eq!(harm_weight(BucketId::Medication, &h).0, 3.0);
    }

    #[test]
    fn every_bucket_has_exactly_one_base_row_that_always_applies() {
        for b in BucketId::ALL {
            let always = HARM_WEIGHTS
                .iter()
                .filter(|r| r.bucket == *b && r.applies == Applies::Always)
                .count();
            assert_eq!(always, 1, "{b}");
        }
        for r in HARM_WEIGHTS {
            assert!(!r.why.is_empty() && r.why.len() < 160, "{}", r.id);
            assert!(r.weight >= 0.0);
        }
    }

    #[test]
    fn rules_are_reported_with_the_weight() {
        let phx = fixtures::get("phoenix-apartment-cpap-1").unwrap();
        let (weight, rules) = harm_weight(BucketId::Power, &phx);
        assert_eq!(weight, 3.0);
        let ids: Vec<&str> = rules.iter().map(|r| r.id).collect();
        assert_eq!(ids, ["power", "power_device"]);
    }
}
