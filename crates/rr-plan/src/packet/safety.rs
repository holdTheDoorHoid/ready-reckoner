//! The life-safety rules every packet prints (review S3b, RR-P03): the key warning of each free
//! step whose mistake kills, one line each, right after the free steps to start now. The steps
//! themselves print by name only, so without these lines the warnings never reached paper.

use super::cite_all;

/// One life-safety rule: a short label, the sentence the packet prints, the catalogue step it
/// belongs to, and the sources behind the sentence.
#[derive(Debug, Clone, Copy)]
pub struct SafetyRule {
    /// The label the line starts with ("Fire").
    pub label: &'static str,
    /// The sentence, as printed.
    pub text: &'static str,
    /// The catalogue step whose warning it is.
    pub step: &'static str,
    /// Citation ids for the sentence.
    pub citations: &'static [&'static str],
}

/// The rules, in the order they print. Every packet carries every one
/// (`every_packet_prints_the_life_safety_rules` in `tests/round2.rs`).
pub const SAFETY_RULES: [SafetyRule; 6] = [
    SafetyRule {
        label: "Fire",
        text: "Know two ways out of every room. Once you are out, stay out, and call 911.",
        step: "fire_test_alarms",
        citations: &["ready_gov_home_fires", "usfa_home_fire_escape_plans"],
    },
    SafetyRule {
        label: "Gas",
        text: "If your gas was shut off, only the gas company or a professional should turn it \
               back on.",
        step: "fire_learn_shutoffs",
        citations: &["fema_food_and_water_2004"],
    },
    SafetyRule {
        label: "Water heater",
        text: "Turn off its power or gas before you drain it for water.",
        step: "water_boil_method",
        citations: &["fema_food_and_water_2004"],
    },
    SafetyRule {
        label: "Food",
        text: "In a power cut, throw out food that has been at 40°F or warmer for 2 hours.",
        step: "water_boil_method",
        citations: &["ready_gov_power_outages"],
    },
    SafetyRule {
        label: "Generator",
        text: "Run it outside, 20 feet from windows and doors. Never plug it into a wall outlet or \
               the house wiring: that can electrocute utility workers and neighbors.",
        step: "power_generator",
        citations: &["ready_gov_power_outages", "cpsc_generator_alert_2021"],
    },
    SafetyRule {
        label: "CPR",
        text: "Take a first-aid and CPR class.",
        step: "community_know_two_neighbours",
        citations: &["ready_gov_safety_skills"],
    },
];

pub(super) fn write(out: &mut Vec<String>) {
    out.push("### Safety rules to learn now".to_owned());
    out.push(String::new());
    for r in &SAFETY_RULES {
        let ids: Vec<rr_types::CitationId> = r
            .citations
            .iter()
            .map(|c| rr_types::CitationId::from(*c))
            .collect();
        out.push(format!("- **{}:** {}{}", r.label, r.text, cite_all(&ids)));
    }
    out.push(String::new());
}
