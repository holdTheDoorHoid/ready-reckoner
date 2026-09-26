//! Severity (how bad one event is for the household, 0 to 1) and confidence.
//!
//! **Severity** is the loss from one household-significant event on a fixed log scale: $50 or
//! less is 0, $500,000 or more is 1. A fixed scale lets a hazard's severity mean the same thing
//! in every county, and keeps likelihood and severity separate, as the rare-catastrophe box needs
//! (DESIGN §4.2). For natural hazards the loss per event comes from the National Risk Index:
//! expected annual loss per household (`EALT` ÷ households) divided by the county-average
//! household rate, never below 0.1 (even a windstorm that only cuts the power is more than
//! nothing). Personal and societal hazards use a documented per-event loss (PRIOR, except house
//! fire: USFA's $11.27 billion over 344,600 fires, about $32,700 a fire). Nuclear attack is 1 and
//! terrorism 0.9 by definition.
//!
//! **Confidence** follows the evidence and the width of the range: data within a factor of 1.6
//! is `high`, data within a factor of 3 is `medium`; anything wider, or resting partly on expert
//! judgement, is `low` (or `medium` if the range is narrow); a rate resting only on expert
//! judgement is `prior`.

use rr_types::{
    AgeBand, Cooling, DataConfidence, Evidence, HazardId, HazardTier, Heating, PlanInput,
    PoweredDevice, math,
};

use crate::cite;
use crate::estimate::Estimate;
use crate::params::{
    AT_RISK_TEMPERATURE_SEVERITY, NATURAL_SEVERITY_FLOOR, SEVERITY_LOSS_ONE_USD,
    SEVERITY_LOSS_ZERO_USD,
};
use crate::rate::HazardRate;

/// Severity from a per-event loss in US dollars, on the fixed log scale.
pub(crate) fn from_loss(loss_usd: f64) -> f64 {
    if !(loss_usd.is_finite() && loss_usd > SEVERITY_LOSS_ZERO_USD) {
        return 0.0;
    }
    let span = math::ln(SEVERITY_LOSS_ONE_USD / SEVERITY_LOSS_ZERO_USD);
    (math::ln(loss_usd / SEVERITY_LOSS_ZERO_USD) / span).clamp(0.0, 1.0)
}

/// The severity of one hazard for this household.
pub(crate) fn of(rate: &HazardRate) -> f64 {
    if let Some(s) = rate.fixed_severity {
        return s;
    }
    let loss = match rate.eal_per_household {
        Some(eal) if rate.county_average > 0.0 && eal > 0.0 => eal / rate.county_average,
        _ => rate.default_loss_usd,
    };
    let s = from_loss(loss);
    if rate.hazard.tier() == HazardTier::Natural {
        s.max(NATURAL_SEVERITY_FLOOR)
    } else {
        s
    }
}

/// Why a heat or cold wave is worse for this household than for the average one, if it is:
/// someone 65 or older or a baby (and, for heat, someone pregnant), someone on a powered medical
/// device, or a home without air conditioning (heat) or heating (cold). CDC names older adults,
/// babies, pregnancy and chronic illness as the groups heat and cold hurt most; in Chicago's 1995
/// heat wave, having no air conditioning raised the risk of death (Semenza et al. 1996).
pub(crate) fn at_risk_reason(input: &PlanInput, hazard: HazardId) -> Option<&'static str> {
    let heat = match hazard {
        HazardId::HeatWave => true,
        HazardId::ColdWave => false,
        _ => return None,
    };
    let people = &input.people;
    if people.iter().any(|p| matches!(p.age_band, AgeBand::Senior)) {
        return Some("someone in it is 65 or older");
    }
    if people.iter().any(|p| matches!(p.age_band, AgeBand::Infant)) {
        return Some("there is a baby");
    }
    if heat && people.iter().any(|p| p.pregnant_or_nursing) {
        return Some("someone in it is pregnant or nursing");
    }
    if people
        .iter()
        .any(|p| p.medical.powered_device != PoweredDevice::None)
    {
        return Some("someone in it relies on a powered medical device");
    }
    if heat && input.housing.cooling == Cooling::None {
        return Some("the home has no air conditioning");
    }
    if !heat && input.housing.heating == Heating::None {
        return Some("the home has no heating");
    }
    None
}

/// The severity of one hazard for this household: [`of`], raised to "Serious" for a heat or
/// cold wave when someone in the household is at higher risk ([`at_risk_reason`]).
pub(crate) fn for_household(rate: &HazardRate, input: &PlanInput) -> f64 {
    let s = of(rate);
    if at_risk_reason(input, rate.hazard).is_some() {
        s.max(AT_RISK_TEMPERATURE_SEVERITY)
    } else {
        s
    }
}

/// How much to trust a rate.
pub(crate) fn confidence(e: &Estimate) -> DataConfidence {
    let k = e.range_factor();
    let only_priors = !e.sources.is_empty() && e.sources.iter().all(|s| s == cite::RR_PRIORS);
    match e.evidence {
        Evidence::Empirical if k <= 1.6 => DataConfidence::High,
        Evidence::Empirical if k <= 3.0 => DataConfidence::Medium,
        Evidence::Empirical => DataConfidence::Low,
        Evidence::Prior if only_priors => DataConfidence::Prior,
        Evidence::Prior if k <= 3.0 => DataConfidence::Medium,
        Evidence::Prior => DataConfidence::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_is_logarithmic_between_fifty_dollars_and_half_a_million() {
        assert_eq!(from_loss(10.0), 0.0);
        assert_eq!(from_loss(50.0), 0.0);
        assert_eq!(from_loss(500_000.0), 1.0);
        assert_eq!(from_loss(5_000_000.0), 1.0);
        // $5,000 is two of the four decades: 0.5.
        assert!((from_loss(5_000.0) - 0.5).abs() < 1e-12);
        // A house fire, $32,700 (USFA 2023): about 0.70.
        assert!((from_loss(11.27e9 / 344_600.0) - 0.704).abs() < 0.001);
    }

    #[test]
    fn heat_and_cold_are_serious_for_households_at_risk() {
        let mut input = rr_types::fixtures::get("philadelphia-renters-4").unwrap();
        // A senior lives there: heat and cold waves are at least "Serious".
        assert_eq!(
            at_risk_reason(&input, HazardId::HeatWave),
            Some("someone in it is 65 or older")
        );
        assert!(at_risk_reason(&input, HazardId::ColdWave).is_some());
        assert!(at_risk_reason(&input, HazardId::StrongWind).is_none());
        // Adults only, with air conditioning and gas heat: no floor.
        input
            .people
            .retain(|p| matches!(p.age_band, AgeBand::Adult));
        assert!(at_risk_reason(&input, HazardId::HeatWave).is_none());
        // Take the air conditioning away and heat is serious again; cold is not.
        input.housing.cooling = Cooling::None;
        assert!(at_risk_reason(&input, HazardId::HeatWave).is_some());
        assert!(at_risk_reason(&input, HazardId::ColdWave).is_none());
        // 0.4 is about $2,000 an event on the fixed scale, an emergency-department visit.
        assert!((AT_RISK_TEMPERATURE_SEVERITY - from_loss(2_000.0)).abs() < 0.001);
    }

    #[test]
    fn confidence_follows_evidence_and_range() {
        let d = |k: f64| Estimate::data(1.0, 1.0 / k, k, &["x"]);
        assert_eq!(confidence(&d(1.5)), DataConfidence::High);
        assert_eq!(confidence(&d(2.0)), DataConfidence::Medium);
        assert_eq!(confidence(&d(5.0)), DataConfidence::Low);
        let p = Estimate::prior(1.0, 0.3, 3.0, &[cite::RR_PRIORS]);
        assert_eq!(confidence(&p), DataConfidence::Prior);
        let mixed = p.times(&Estimate::data(2.0, 2.0, 2.0, &[cite::NRI]));
        assert_eq!(confidence(&mixed), DataConfidence::Low);
    }
}
