//! The value of a purchase (DESIGN §4.7) and the allocator's constants.
//!
//! Duration buckets: an item that moves bucket *b* from *x* to *x + Δ* days is worth
//!
//! ```text
//! V = 10 · w_b · ∫ₓ^min(x+Δ, target) Λ_b(t) dt
//! ```
//!
//! expected weighted disruption-days covered per decade. Because Λ falls with *d*, the first day of
//! a bucket is always worth more than the fourteenth, so cheap early coverage floats to the top
//! without special cases. For a bucket covered in parts, each part's integral is scaled by the part's
//! share of the bucket.
//!
//! Readiness buckets: `V = 10 · w · r_need · harm_day_equivalents`, with the annual need rate
//! r_need = −ln(1 − P₁₀)/10 recovered from the bucket's ten-year need probability P₁₀.
//!
//! The ten in both formulas is the design's fixed ten-year value horizon; the household's
//! `dials.horizon_years` only changes the natural-frequency sentences.

use rr_types::math;

use crate::curve::BucketCurve;

/// Years in the value horizon ("per decade").
pub const VALUE_HORIZON_YEARS: f64 = 10.0;

/// A readiness capability is included when the ten-year chance of needing it is at least this
/// (`Prior`, DESIGN §4.4), or when its value per dollar beats the current tier's best item.
pub const READINESS_MIN_P_NEED_10YR: f64 = 0.02;

/// A later-tier item is promoted into the current tier when its value per dollar is at least this
/// many times the tier's best (DESIGN §4.7 step 4).
pub const PROMOTION_FACTOR: f64 = 5.0;

/// Sinking-fund rule of the research schedule: save instead of buying something else when the
/// best item costs at most this many months of budget ...
pub const SINKING_FUND_MAX_MONTHS: f64 = 2.0;

/// ... and the best affordable alternative is worth less than this fraction of it per dollar.
pub const SINKING_FUND_VALUE_RATIO: f64 = 0.25;

/// Specialised rare-catastrophe items get at most this share of each month's money, and only on
/// opt-in (DESIGN §4.7; research risk-model §6.3).
pub const RARE_CATASTROPHIC_SHARE: f64 = 0.10;

/// Value of moving a duration bucket (or a part of one with `share` of its disruption) from `x0`
/// to `x1` days, with coverage credited only up to `cap` days.
pub fn duration_value(
    weight: f64,
    share: f64,
    curve: &BucketCurve,
    x0: f64,
    x1: f64,
    cap: f64,
) -> f64 {
    let top = x1.min(cap);
    if top <= x0 {
        return 0.0;
    }
    VALUE_HORIZON_YEARS * weight * share * curve.integral(x0, top)
}

/// The annual rate r behind a ten-year probability P: P = 1 − e^(−10 r).
pub fn annual_rate_from_10yr(p_need_10yr: f64) -> f64 {
    let p = p_need_10yr.clamp(0.0, 1.0 - 1e-12);
    -math::ln_1p(-p) / VALUE_HORIZON_YEARS
}

/// Value of a readiness item: 10 · w · r_need · harm.
pub fn readiness_value(weight: f64, p_need_10yr: f64, harm_day_equivalents: f64) -> f64 {
    VALUE_HORIZON_YEARS * weight * annual_rate_from_10yr(p_need_10yr) * harm_day_equivalents
}

/// Of 100 households like this one, how many face at least one event over `years` when events
/// arrive at `rate` per year: 100 · (1 − e^(−years · rate)) (DESIGN §4.4 point 2).
pub fn per_100(rate: f64, years: f64) -> f64 {
    if rate <= 0.0 || years <= 0.0 {
        return 0.0;
    }
    -100.0 * math::exp_m1(-years * rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_value_is_capped_at_the_target() {
        // Λ = 0.1 per year, flat. Days 0-3 at weight 3: 10 · 3 · 0.1 · 3 = 9.
        let curve = BucketCurve::new(vec![1.0], vec![0.1], 3.0);
        assert!((duration_value(3.0, 1.0, &curve, 0.0, 3.0, 3.0) - 9.0).abs() < 1e-12);
        // Days beyond the cap earn nothing.
        assert!((duration_value(3.0, 1.0, &curve, 0.0, 5.0, 3.0) - 9.0).abs() < 1e-12);
        assert_eq!(duration_value(3.0, 1.0, &curve, 3.0, 5.0, 3.0), 0.0);
        // A part with half the bucket's disruption earns half.
        assert!((duration_value(3.0, 0.5, &curve, 0.0, 3.0, 3.0) - 4.5).abs() < 1e-12);
    }

    #[test]
    fn readiness_value_matches_the_research_prototype() {
        // The prototype's go-bag: rate 0.0082/yr, 2 harm-days, weight 2: V = 10·2·0.0082·2.
        let p10 = 1.0 - math::exp(-10.0 * 0.0082);
        assert!((readiness_value(2.0, p10, 2.0) - 0.328).abs() < 1e-12);
        // First-aid kit: 0.5/yr, 0.1 harm-days, weight 1: V = 0.5.
        let p10 = 1.0 - math::exp(-5.0);
        assert!((readiness_value(1.0, p10, 0.1) - 0.5).abs() < 1e-12);
        assert_eq!(readiness_value(1.0, 0.0, 3.0), 0.0);
        assert!(readiness_value(1.0, 1.0, 1.0).is_finite());
    }

    #[test]
    fn natural_frequencies() {
        // DESIGN §4.4: Λ = 0.0105 per year over 10 years is 10 households in 100.
        assert!((per_100(0.010536, 10.0) - 10.0).abs() < 0.01);
        // Research §6.1: a one-day power cut for Philadelphia, about 26 in 100 over 10 years.
        assert!((per_100(0.02964, 10.0) - 25.6).abs() < 0.1);
        assert_eq!(per_100(0.0, 10.0), 0.0);
        assert!(per_100(50.0, 10.0) <= 100.0);
    }
}
