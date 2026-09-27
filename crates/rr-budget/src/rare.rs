//! The rare-event allowance by family (REVIEW §2.4; hazard-expansion report, Deliverable C.2).
//!
//! Specialised items for rare catastrophes (a radiation dosimeter card, Faraday storage) never
//! enter the main plan. The household ticks rare families (`Dials::rare_opt_in`, or the v1 switch
//! for all of them), and then:
//!
//! 1. **Eligibility is local.** An item is a candidate only through a family it names in
//!    `hazard_extras` that the household ticked and whose local ten-year chance (the register's
//!    central estimate) is at least [`RARE_MIN_P10`], 1 in 1,000. An item that protects against
//!    one cause inside its family ([`RARE_CAUSES`]: Faraday storage and an electromagnetic pulse)
//!    is gated on that cause's chance instead of the family total. Most families sell nothing, so
//!    the allowance cannot be hijacked. Potassium iodide is free and keeps its own planning-zone
//!    rule in `rr-supply` (offered only near a nuclear plant); it is a free step, not an allowance
//!    purchase.
//! 2. **Value comes from the family (or the cause), not from a bucket:** `V = P₁₀(family or
//!    cause, here) × harm-days avoided` ([`RARE_HARM_DAYS`], an expert prior per item), ranked by
//!    `V ÷ cost`.
//! 3. **No family takes more than half the allowance** ([`RARE_FAMILY_MAX_SHARE`]) over the plan
//!    horizon: the allowance's total is a tenth of the one-off money plus a tenth of every month's
//!    money up to `BudgetOptions::max_months`.
//! 4. **Basics first.** The allowance receives nothing until the three-day tier's life-safety
//!    items are all in hand (the allocator checks at the start of each month).
//! 5. **Say it in words:** each allowance purchase says which ticked family it is for and why it
//!    qualifies here ([`allowance_sentence`]).

use rr_types::{HazardId, SubCause};

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
///   supporting item: 0.5, like the get-home bag. It belongs to the months-long blackout family
///   (`multi_month_blackout`), not the nuclear one: a pulse reaches far beyond any blast or
///   fallout zone, so the local nuclear chance says nothing about it.
pub const RARE_HARM_DAYS: &[(&str, f64)] =
    &[("rare_radiation_meter", 2.0), ("rare_faraday_storage", 0.5)];

/// Items that protect against one cause inside their family, gated on that cause's chance rather
/// than the family total: (item, the sub-cause id its family lists in `HazardProfile::sub_causes`).
///
/// Faraday storage protects small electronics from an electromagnetic pulse: the `emp` sub-cause
/// of its family, the months-long blackout (planner, 2026-09-26). That family's total is mostly
/// the household's own power curve (about 8 in 100 over ten years in Coos Bay, with Cascadia), which
/// says nothing about a pulse.
pub const RARE_CAUSES: &[(&str, &str)] = &[("rare_faraday_storage", "emp")];

/// The sub-cause item `id` is gated on, if it names one ([`RARE_CAUSES`]).
pub fn cause_of(id: &str) -> Option<&'static str> {
    RARE_CAUSES
        .iter()
        .find(|(item, _)| *item == id)
        .map(|(_, cause)| *cause)
}

/// A sub-cause's central yearly rate. Sub-causes publish a range only (`SubCause::rate_range`,
/// built low by low and high by high), so the central value is the range's geometric middle, as
/// the rare box sorts by the middle of a range; 0 when the range is missing or an end is 0.
pub fn sub_cause_rate(s: &SubCause) -> f64 {
    match s.rate_range {
        Some([lo, hi]) if lo.is_finite() && hi.is_finite() && lo > 0.0 && hi > 0.0 => {
            rr_types::math::exp(0.5 * (rr_types::math::ln(lo) + rr_types::math::ln(hi)))
        }
        _ => 0.0,
    }
}

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
/// `families` are the ticked families that make it eligible (at least one); `cause` is the name of
/// the sub-cause it is gated on, when it is ("EMP from a nuclear attack"). No chance is given as a
/// number: rare rows are shown as ranges only, so the sentence names the threshold instead.
pub fn allowance_sentence(
    families: &[HazardId],
    cause: Option<&str>,
    monthly_allowance_usd: f64,
) -> String {
    let names: Vec<String> = families.iter().map(|f| family_words(*f)).collect();
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
    let what_for = match cause {
        Some(c) => format!("{}, part of the {what} {rows} you ticked", sentence_case(c)),
        None => format!("the {what} {rows} you ticked"),
    };
    format!(
        "Paid from {allowance}, after your three-day basics. It is for {what_for}: where you live, \
         more than 1 in 1,000 households like yours face it in 10 years, the least the allowance \
         needs. Your basics cover the rare rows it does not buy for."
    )
}

/// A name inside a sentence: its first letter in lower case unless the first word is an acronym
/// ("EMP from a nuclear attack" stays; "A severe solar storm" reads "a severe solar storm").
fn sentence_case(name: &str) -> String {
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(_), Some(second)) if second.is_uppercase() => name.to_owned(),
        _ => lower_first(name),
    }
}

/// A family's name inside a sentence: lower case, without a trailing aside ("Power out for months
/// (any cause)" reads "power out for months").
pub fn family_words(family: HazardId) -> String {
    let name = family.name();
    let short = name.split(" (").next().unwrap_or(name);
    lower_first(short)
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
        let s = allowance_sentence(&[HazardId::NuclearAttack], None, 6.0);
        assert!(s.contains("nuclear attack row you ticked"), "{s}");
        assert!(s.contains("($6 a month"), "{s}");
        assert!(s.contains("1 in 1,000"), "{s}");
        let two = allowance_sentence(
            &[HazardId::NuclearAttack, HazardId::GeomagneticStorm],
            None,
            0.0,
        );
        assert!(
            two.contains("nuclear attack and severe solar storm rows"),
            "{two}"
        );
        // Faraday storage's family, named without its aside.
        let blackout = allowance_sentence(&[HazardId::MultiMonthBlackout], None, 5.0);
        assert!(
            blackout.contains("It is for the power out for months row you ticked"),
            "{blackout}"
        );
        assert_eq!(
            family_words(HazardId::MultiMonthBlackout),
            "power out for months"
        );
        // Gated on a cause: it names the cause, then its family; an acronym keeps its capitals.
        let emp = allowance_sentence(
            &[HazardId::MultiMonthBlackout],
            Some("EMP from a nuclear attack"),
            5.0,
        );
        assert!(
            emp.contains(
                "It is for EMP from a nuclear attack, part of the power out for months row you \
                 ticked"
            ),
            "{emp}"
        );
        assert_eq!(
            sentence_case("A severe solar storm"),
            "a severe solar storm"
        );
    }

    #[test]
    fn a_cause_is_gated_on_the_middle_of_its_range() {
        assert_eq!(cause_of("rare_faraday_storage"), Some("emp"));
        assert_eq!(cause_of("rare_radiation_meter"), None);
        let emp = |range: Option<[f64; 2]>| SubCause {
            id: "emp".into(),
            name: "EMP from a nuclear attack".into(),
            note: String::new(),
            rate_range: range,
            sources: Vec::new(),
        };
        // The months-long blackout family's EMP part in the lower 48 (rr-hazards: the EMP rate
        // times the share of EMP outages lasting two months or more): 4.4e-7 to 1.035e-3 a year,
        // middle about 2.1e-5, about 0.21 in 1,000 over ten years, under the line.
        let r = sub_cause_rate(&emp(Some([4.4e-7, 1.035e-3])));
        assert!((r - 2.134e-5).abs() < 1e-8, "{r}");
        assert!(p10_from_rate(r) < RARE_MIN_P10);
        // Outside the lower 48 the family counts no EMP at all.
        assert_eq!(sub_cause_rate(&emp(Some([0.0, 0.0]))), 0.0);
        assert_eq!(sub_cause_rate(&emp(None)), 0.0);
    }
}
