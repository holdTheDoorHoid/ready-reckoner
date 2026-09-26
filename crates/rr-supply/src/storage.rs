//! Where it lives: how much room and weight the stored supplies take at each tier (round-2
//! practitioner review P-20). The plan prints a "Where it lives" table from these figures; the
//! bulky consumables are counted (water, food, pet food and toilet paper), gear is not.
//!
//! Each tier counts every need line of those kinds up to the tier's days: a line that grows with
//! days (`SizedLine::per_day`) counts its share for the shorter of the tier and the line's own days,
//! so a ten-day food line fills three days of the three-day tier and all ten of the two-week tier.
//! Water weighs a kilogram a litre; food, pet food and toilet paper use planning estimates
//! (`food_storage_l_per_2000kcal` and the rest), so every figure is an estimate.

use rr_types::{CitationId, TierId};

use crate::basis::Basis;
use crate::constants::keys;
use crate::format::{L_PER_GAL, num, round_dp};
use crate::lines::{LineKind, SizedLine};

/// Litres in a cubic foot (a definition).
const L_PER_CUBIC_FOOT: f64 = 28.316_846_592;
/// Pounds in a kilogram (a definition).
const LB_PER_KG: f64 = 2.204_622_621_8;
/// Kilograms in a pound (a definition).
const KG_PER_LB: f64 = 0.453_592_37;

/// One kind of stored supply at one tier.
#[derive(Debug, Clone, PartialEq)]
pub struct StoragePart {
    /// What it is, in plain words ("water", "food").
    pub label: &'static str,
    /// Litres of space.
    pub volume_l: f64,
    /// Kilograms.
    pub mass_kg: f64,
}

/// The stored supplies' space and weight when the household reaches one tier.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageEstimate {
    /// The tier.
    pub tier: TierId,
    /// Litres of space, all parts.
    pub volume_l: f64,
    /// Kilograms, all parts.
    pub mass_kg: f64,
    /// The parts, in a fixed order (water, food, pet food, toilet paper); empty parts are left out.
    pub parts: Vec<StoragePart>,
    /// Sources of the numbers used.
    pub citations: Vec<CitationId>,
    /// At least one number is a planning estimate (always true: food and paper are estimates).
    pub prior: bool,
    /// The estimate in plain words, with cubic feet and pounds.
    pub plain: String,
}

/// The quantity of a need line up to `days` of it: its share for the shorter of the tier and its
/// own days, or the whole line when it does not grow with days.
fn up_to(line: &SizedLine, days: f64) -> f64 {
    match (line.days, line.per_day) {
        (Some(d), Some(_)) => line.quantity_for_days(days.min(d)).unwrap_or(line.quantity),
        _ => line.quantity,
    }
}

/// The space and weight of the household's stored supplies at each tier from three days up to the
/// highest tier any of those lines reaches.
pub fn storage_by_tier(lines: &[SizedLine]) -> Vec<StorageEstimate> {
    let kinds: [(&str, &'static str); 4] = [
        ("water_out.water_gallons", "water"),
        ("supplies.food_kcal", "food"),
        ("supplies.pet_food_lb", "pet food"),
        ("supplies.toilet_paper_rolls", "toilet paper"),
    ];
    let stored: Vec<(&SizedLine, &'static str)> = lines
        .iter()
        .filter(|l| l.kind == LineKind::Need && l.quantity > 0.0)
        .filter_map(|l| {
            kinds
                .iter()
                .find(|(id, _)| *id == l.line.id)
                .map(|(_, label)| (l, *label))
        })
        .collect();
    let Some(top) = stored.iter().map(|(l, _)| l.tier).max() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for &tier in TierId::ALL {
        if tier == TierId::Now || tier > top.max(TierId::H72) {
            continue;
        }
        let days = f64::from(tier.days());
        let mut b = Basis::new();
        let mut parts = Vec::new();
        for (line, label) in &stored {
            let q = up_to(line, days);
            let (volume_l, mass_kg) = match *label {
                "water" => (q * L_PER_GAL, q * L_PER_GAL),
                "food" => (
                    q / 2000.0 * b.k(keys::FOOD_STORAGE_L_PER_2000KCAL),
                    q / 2000.0 * b.k(keys::FOOD_STORAGE_KG_PER_2000KCAL),
                ),
                "pet food" => (q * b.k(keys::PET_FOOD_L_PER_LB), q * KG_PER_LB),
                _ => (
                    q * b.k(keys::TOILET_PAPER_L_PER_ROLL),
                    q * b.k(keys::TOILET_PAPER_KG_PER_ROLL),
                ),
            };
            if volume_l > 0.0 {
                parts.push(StoragePart {
                    label,
                    volume_l,
                    mass_kg,
                });
            }
        }
        b.cite("ready_gov_kit");
        let volume_l: f64 = parts.iter().map(|p| p.volume_l).sum();
        let mass_kg: f64 = parts.iter().map(|p| p.mass_kg).sum();
        let list = parts
            .iter()
            .map(|p| format!("{} {} L", p.label, num(p.volume_l, 0)))
            .collect::<Vec<_>>();
        let plain = format!(
            "At the {} step, the stored supplies take about {} litres (about {} cubic feet) and weigh about {} kg ({} lb): {}. Gear, go-bags and kits come on top. Keep them in a cool, dry place, and off the floor if water could get in.",
            tier_words(tier),
            num(volume_l, 0),
            num(round_dp(volume_l / L_PER_CUBIC_FOOT, 1), 1),
            num(mass_kg, 0),
            num(mass_kg * LB_PER_KG, 0),
            crate::format::and_list(&list)
        );
        out.push(StorageEstimate {
            tier,
            volume_l,
            mass_kg,
            parts,
            citations: b.cites().to_vec(),
            prior: b.is_prior(),
            plain,
        });
    }
    out
}

fn tier_words(tier: TierId) -> &'static str {
    match tier {
        TierId::Now => "free",
        TierId::H72 => "three-day",
        TierId::W2 => "two-week",
        TierId::M1 => "one-month",
        TierId::M3 => "three-month",
        TierId::M6 => "six-month",
        TierId::Y1 => "one-year",
    }
}
