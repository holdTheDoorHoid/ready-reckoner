//! The bare-minimum kit (contract v2's `Dials::minimum_kit`; REVIEW R6, model review M-12): the
//! smallest set of the household's own need lines that covers three days of water, light, warmth
//! and medicine continuity. `rr-budget` schedules the items that meet these shares first when the
//! household asks for the bare minimum or the plan would run past 36 months.
//!
//! | Function | Line | Share in the kit |
//! | --- | --- | --- |
//! | Water | `water_out.water_gallons` | 3 days of the household's water (or the target, if shorter) |
//! | Safe water | `*.bleach_bottles` | the one bottle |
//! | Light | `power.lights`, `power.battery_packs` | 1 light and 1 pack of batteries |
//! | Warmth | `thermal.blankets`, `thermal.warm_layers`, `thermal.cooling_plan` | all of them (most homes have them; the heat plan is free) |
//! | Medicine | `medication.medication_days`, `power.medical_device_wh` | 3 days (or the target) |
//! | Medicine | `medication.rx_cold_storage` | the cooler bag's day |
//! | Medicine | `power.device_battery_units` | every spare battery |
//! | A baby's food | `supplies.infant_formula_rtf` | the three days of ready-to-feed formula |
//!
//! A baby's ready-to-feed formula is in the kit because for a formula-fed baby it is water and food
//! at once. Smoke and carbon monoxide alarms are not in it: they are life-safety lines, which the
//! allocator already orders first.

use crate::format::round_dp;
use crate::lines::{LineKind, SizedLine};
use crate::rules::round_quantity;

/// Days of each duration line the kit covers.
pub const MINIMUM_KIT_DAYS: f64 = 3.0;

/// The kit's share of one line, or `None` when the line is not part of the kit.
fn share(line: &SizedLine) -> Option<f64> {
    if line.kind != LineKind::Need || line.quantity <= 0.0 {
        return None;
    }
    let three_days = || {
        let days = line.days.unwrap_or(MINIMUM_KIT_DAYS).min(MINIMUM_KIT_DAYS);
        line.quantity_for_days(days)
            .map(|q| round_quantity(&line.line.unit, q).min(line.quantity))
            .or(Some(line.quantity))
    };
    let id = line.line.id.as_str();
    match id {
        "water_out.water_gallons" | "medication.medication_days" | "power.medical_device_wh" => {
            three_days()
        }
        "water_boil.bleach_bottles"
        | "water_out.bleach_bottles"
        | "power.device_battery_units"
        | "thermal.blankets"
        | "thermal.warm_layers"
        | "thermal.cooling_plan"
        | "supplies.infant_formula_rtf" => Some(line.quantity),
        "power.lights" | "power.battery_packs" => Some(1.0_f64.min(line.quantity)),
        "medication.rx_cold_storage" => Some(round_dp(line.quantity.min(1.0), 1)),
        _ => None,
    }
}

/// Marks every line's share of the kit ([`SizedLine::minimum`]).
pub(crate) fn mark(lines: &mut [SizedLine]) {
    for l in lines.iter_mut() {
        l.minimum = share(l);
    }
}

/// The lines in the bare-minimum kit, in output order.
pub fn minimum_kit(lines: &[SizedLine]) -> Vec<&SizedLine> {
    lines.iter().filter(|l| l.minimum.is_some()).collect()
}
