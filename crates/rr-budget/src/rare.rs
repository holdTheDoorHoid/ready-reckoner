//! The rare-event allowance by family (REVIEW §2.4; hazard-expansion report, Deliverable C.2).
//!
//! Specialised items for rare catastrophes (a radiation dosimeter card, Faraday storage) never
//! enter the main plan. The household ticks rare families (`Dials::rare_opt_in`, or the v1 switch
//! for all of them), and then:
//!
//! 1. **Eligibility is local.** An item is a candidate only through a family it names in
//!    `hazard_extras` that the household ticked and whose local ten-year chance (the register's
//!    central estimate) is at least [`RARE_MIN_P10`], 1 in 1,000. Most families sell nothing, so
//!    the allowance cannot be hijacked. Potassium iodide is free and keeps its own planning-zone
//!    rule in `rr-supply` (offered only near a nuclear plant); it is a free step, not an allowance
//!    purchase.
//! 2. **Value comes from the family, not from a bucket:** `V = P₁₀(family, here) ×
//!    harm-days avoided` ([`RARE_HARM_DAYS`], an expert prior per item), ranked by `V ÷ cost`.
//! 3. **No family takes more than half the allowance** ([`RARE_FAMILY_MAX_SHARE`]) over the plan
//!    horizon: the allowance's total is a tenth of the one-off money plus a tenth of every month's
//!    money up to `BudgetOptions::max_months`.
//! 4. **Basics first.** The allowance receives nothing until the three-day tier's life-safety
//!    items are all in hand (the allocator checks at the start of each month).
//! 5. **Say it in words:** each allowance purchase says which ticked family it is for and why it
//!    qualifies here ([`allowance_sentence`]).

use rr_types::HazardId;

use crate::value::VALUE_HORIZON_YEARS;

/// An item is eligible only through a family whose local ten-year chance, the register's central
/// estimate, is at least this: 1 in 1,000 (REVIEW §2.4, hazard-expansion C.2 point 2).
pub const RARE_MIN_P10: f64 = 0.001;

/// No family may take more than this share of the allowance over the plan horizon (REVIEW §2.4,
/// hazard-expansion C.2 point 3).
pub const RARE_FAMILY_MAX_SHARE: f64 = 0.5;

/// Day-equivalents of harm a specialised item avoids if its family's event reaches the household
/// (an expert prior, cited as `prior_harm_weights`, on the scale of rr-plan's readiness harm
/// estimates: go-bag 2, get-home bag 0.5, first-aid kit 0.1). They only order the allowance's own
/// queue and never reach the user as numbers.
///
/// - A dosimeter card tells a sheltering household when it is safe to go outside, the main
///   decision after fallout (`ready_gov_nuclear`: get inside, stay inside, stay tuned): 2, like
///   the go-bag.
/// - Faraday storage keeps a spare radio or phone working after an electromagnetic pulse, a
///   supporting item: 0.5, like the get-home bag.
pub const RARE_HARM_DAYS: &[(&str, f64)] =
    &[("rare_radiation_meter", 2.0), ("rare_faraday_storage", 0.5)];

/// The harm-days prior for a specialised item the table does not list: that of a supporting item.
pub const RARE_DEFAULT_HARM_DAYS: f64 = 0.5;

/// The citation for [`RARE_HARM_DAYS`]: the allocator's expert priors (prior = true).
pub const RARE_HARM_CITATION: &str = "prior_harm_weights";

/// The harm-days prior for item `id`.
pub fn harm_days(id: &str) -> f64 {
    RARE_HARM_DAYS
        .iter()
        .find(|(item, _)| *item == id)
        .map_or(RARE_DEFAULT_HARM_DAYS, |(_, h)| *h)
}

/// The ten-year chance behind a yearly rate: 1 − e^(−10 r).
pub fn p10_from_rate(rate: f64) -> f64 {
    if rate.is_finite() && rate > 0.0 {
        -rr_types::math::exp_m1(-VALUE_HORIZON_YEARS * rate)
    } else {
        0.0
    }
}

/// The rare families an item names in its `hazard_extras`, in declaration order, each once.
pub fn families_of(hazard_extras: &[HazardId]) -> Vec<HazardId> {
    let mut out: Vec<HazardId> = Vec::new();
    for h in HazardId::RARE {
        if hazard_extras.contains(h) && !out.contains(h) {
            out.push(*h);
        }
    }
    out
}

/// The plan-screen sentence for an allowance purchase: what it is for, and why it qualifies here.
/// `families` are the ticked families that make it eligible (at least one). No chance is given as
/// a number: rare rows are shown as ranges only, so the sentence names the threshold instead.
pub fn allowance_sentence(families: &[HazardId], monthly_allowance_usd: f64) -> String {
    let names: Vec<String> = families.iter().map(|f| lower_first(f.name())).collect();
    let what = join_or(&names);
    let allowance = if monthly_allowance_usd >= 0.5 {
        format!(
            "your rare-event allowance ({} a month, a tenth of your monthly money)",
            crate::explain::dollars(monthly_allowance_usd)
        )
    } else {
        "your rare-event allowance (a tenth of your money)".to_owned()
    };
    let rows = if families.len() == 1 { "row" } else { "rows" };
    format!(
        "Paid from {allowance}, after your three-day basics. It is for the {what} {rows} you \
         ticked: where you live, the chance passes 1 in 1,000 households over 10 years, the least \
         the allowance needs. Rare rows it does not buy for are covered by your basics."
    )
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn join_or(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eligibility_threshold_and_chances() {
        // Philadelphia's nuclear family, 2.4e-4 a year: about 2.4 in 1,000 over ten years.
        assert!(p10_from_rate(2.4e-4) > RARE_MIN_P10);
        // Coos Bay's (class E), 1.2e-5 a year: about 1.2 in 10,000, under the line.
        assert!(p10_from_rate(1.2e-5) < RARE_MIN_P10);
        assert_eq!(p10_from_rate(0.0), 0.0);
        assert_eq!(p10_from_rate(f64::NAN), 0.0);
    }

    #[test]
    fn families_come_from_hazard_extras() {
        assert_eq!(
            families_of(&[HazardId::NuclearPlantIncident, HazardId::NuclearAttack]),
            [HazardId::NuclearAttack]
        );
        assert!(families_of(&[HazardId::NuclearPlantIncident]).is_empty());
        assert_eq!(harm_days("rare_radiation_meter"), 2.0);
        assert_eq!(harm_days("something_else"), RARE_DEFAULT_HARM_DAYS);
    }

    #[test]
    fn the_sentence_names_the_family_and_no_point_estimate() {
        let s = allowance_sentence(&[HazardId::NuclearAttack], 6.0);
        assert!(s.contains("nuclear attack row you ticked"), "{s}");
        assert!(s.contains("($6 a month"), "{s}");
        assert!(s.contains("1 in 1,000"), "{s}");
        let two = allowance_sentence(&[HazardId::NuclearAttack, HazardId::GeomagneticStorm], 0.0);
        assert!(
            two.contains("nuclear attack and severe solar storm rows"),
            "{two}"
        );
    }
}
