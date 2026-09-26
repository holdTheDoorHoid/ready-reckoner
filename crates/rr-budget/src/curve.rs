//! Exceedance curves Λ_b(d) as the caller hands them over, and their integral (DESIGN §4.4, §4.7).
//!
//! `rr-consequence` owns the curve; this crate only reads it. The caller tabulates Λ at a set of
//! durations and this module interpolates **linearly in log–log space**, which makes Λ a power law
//! between neighbouring points. That shape suits exceedance curves (they fall over orders of
//! magnitude) and integrates in closed form, so the value of a purchase is exact for the table it
//! was given rather than an approximation of an approximation.
//!
//! Outside the table:
//! - below the first duration, Λ is held at the first value (Λ is highest for the shortest
//!   durations, and the first tabulated value is the best estimate of the total event rate);
//! - above the last duration, the last segment's power law continues (and stays at zero if the
//!   curve has reached zero).
//!
//! A segment with a zero end point cannot be a power law; it is interpolated linearly instead.

use std::collections::BTreeMap;

use rr_types::math;
use serde::{Deserialize, Serialize};

/// Why a curve was rejected. A bad curve is a bug in the caller, not a user error.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CurveError {
    /// The table is empty, or `days` and `lambda` have different lengths.
    #[error(
        "the curve needs at least one point and as many lambda values as days ({days} days, {lambda} lambda values)"
    )]
    Shape {
        /// Number of durations.
        days: usize,
        /// Number of Λ values.
        lambda: usize,
    },
    /// A duration is not a finite number above zero, or durations do not strictly increase.
    #[error("days must be finite, above zero and strictly increasing (problem at index {index})")]
    Days {
        /// Index of the offending duration.
        index: usize,
    },
    /// A Λ value is negative, not finite, or larger than the one before it.
    #[error("lambda must be finite, zero or more and never increasing (problem at index {index})")]
    Lambda {
        /// Index of the offending value.
        index: usize,
    },
    /// The target is negative or not finite.
    #[error("target_days must be a finite number of days, zero or more (got {0})")]
    Target(f64),
    /// A part share is negative or not finite.
    #[error("part share for `{0}` must be a finite number, zero or more")]
    PartShare(String),
}

/// One duration bucket's exceedance curve: Λ(d), the expected number of times per year this
/// household faces a disruption in the bucket that lasts longer than `d` days, given as a table.
///
/// Read as piecewise linear in log–log space (see the module docs). `rr-plan` fills it from
/// `rr-consequence`; tests fill it from the research prototype's parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BucketCurve {
    /// Durations in days: finite, above zero, strictly increasing.
    pub days: Vec<f64>,
    /// Λ at each duration, in events per year: finite, zero or more, never increasing.
    pub lambda: Vec<f64>,
    /// The bucket's target in days at the household's return period (use the value shown to the
    /// user, rounded to the day ladder). No value is credited for coverage beyond it.
    pub target_days: f64,
    /// Parts of the bucket that are covered separately, with the share of the bucket's disruption
    /// each part addresses (for `thermal`: heat and cold, from the heat-wave versus winter hazards'
    /// shares). Optional. Parts that items name but this map leaves out split whatever share is
    /// left equally; when the map is empty, every part gets an equal share.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub part_shares: BTreeMap<String, f64>,
}

/// Relative slack when checking that Λ never increases, so rounding noise in a curve that is
/// mathematically flat is not rejected.
const MONOTONE_SLACK: f64 = 1e-9;

impl BucketCurve {
    /// A curve with no parts.
    pub fn new(days: Vec<f64>, lambda: Vec<f64>, target_days: f64) -> Self {
        Self {
            days,
            lambda,
            target_days,
            part_shares: BTreeMap::new(),
        }
    }

    /// Checks the table's shape and values.
    pub fn validate(&self) -> Result<(), CurveError> {
        if self.days.is_empty() || self.days.len() != self.lambda.len() {
            return Err(CurveError::Shape {
                days: self.days.len(),
                lambda: self.lambda.len(),
            });
        }
        for (i, &d) in self.days.iter().enumerate() {
            let increasing = i == 0 || d > self.days[i - 1];
            if !d.is_finite() || d <= 0.0 || !increasing {
                return Err(CurveError::Days { index: i });
            }
        }
        for (i, &l) in self.lambda.iter().enumerate() {
            let falling = i == 0 || l <= self.lambda[i - 1] * (1.0 + MONOTONE_SLACK);
            if !l.is_finite() || l < 0.0 || !falling {
                return Err(CurveError::Lambda { index: i });
            }
        }
        if !self.target_days.is_finite() || self.target_days < 0.0 {
            return Err(CurveError::Target(self.target_days));
        }
        for (part, &share) in &self.part_shares {
            if !share.is_finite() || share < 0.0 {
                return Err(CurveError::PartShare(part.clone()));
            }
        }
        Ok(())
    }

    /// Λ(d): events per year that last longer than `d` days.
    pub fn lambda_at(&self, d: f64) -> f64 {
        let n = self.days.len();
        if n == 0 {
            return 0.0;
        }
        if d <= self.days[0] {
            return self.lambda[0];
        }
        match self.days.iter().position(|&x| x >= d) {
            Some(i) => {
                if d == self.days[i] {
                    self.lambda[i]
                } else {
                    self.segment(i).value(d)
                }
            }
            None => self.tail().value(d),
        }
    }

    /// ∫ₐᵇ Λ(t) dt: expected disruption-days per year that fall between day `a` and day `b` of an
    /// event (DESIGN §4.4, point 3). Zero when `b <= a`; `a` below zero is treated as zero.
    pub fn integral(&self, a: f64, b: f64) -> f64 {
        let a = a.max(0.0);
        let n = self.days.len();
        if n == 0 || b <= a {
            return 0.0;
        }
        let mut total = 0.0;
        // Flat part below the first tabulated duration.
        let first = self.days[0];
        if a < first {
            total += self.lambda[0] * (b.min(first) - a);
        }
        // Tabulated segments.
        for i in 1..n {
            let lo = a.max(self.days[i - 1]);
            let hi = b.min(self.days[i]);
            if lo < hi {
                total += self.segment(i).integral(lo, hi);
            }
        }
        // Beyond the last tabulated duration.
        let last = self.days[n - 1];
        if b > last {
            total += self.tail().integral(a.max(last), b);
        }
        total
    }

    /// The segment between table points `i - 1` and `i` (`i >= 1`).
    fn segment(&self, i: usize) -> Segment {
        Segment::between(
            self.days[i - 1],
            self.days[i],
            self.lambda[i - 1],
            self.lambda[i],
        )
    }

    /// The extrapolation beyond the last point: the last segment's power law, or flat for a
    /// one-point table.
    fn tail(&self) -> Segment {
        let n = self.days.len();
        let (d, l) = (self.days[n - 1], self.lambda[n - 1]);
        if n == 1 || l <= 0.0 {
            return Segment::Power {
                d0: d,
                l0: l,
                alpha: 0.0,
            };
        }
        match self.segment(n - 1) {
            Segment::Power { alpha, .. } => Segment::Power {
                d0: d,
                l0: l,
                alpha,
            },
            // Unreachable for a validated curve (a linear segment ends at zero); stay flat.
            Segment::Linear { .. } => Segment::Power {
                d0: d,
                l0: l,
                alpha: 0.0,
            },
        }
    }
}

/// One piece of the interpolated curve.
#[derive(Debug, Clone, Copy)]
enum Segment {
    /// Λ(t) = l0 · (t / d0)^alpha.
    Power { d0: f64, l0: f64, alpha: f64 },
    /// Straight line from (d0, l0) to (d1, l1), used when an end point is zero.
    Linear { d0: f64, d1: f64, l0: f64, l1: f64 },
}

impl Segment {
    fn between(d0: f64, d1: f64, l0: f64, l1: f64) -> Segment {
        if l0 > 0.0 && l1 > 0.0 {
            let alpha = math::ln(l1 / l0) / math::ln(d1 / d0);
            Segment::Power { d0, l0, alpha }
        } else {
            Segment::Linear { d0, d1, l0, l1 }
        }
    }

    fn value(&self, t: f64) -> f64 {
        match *self {
            Segment::Power { d0, l0, alpha } => {
                if alpha == 0.0 {
                    l0
                } else {
                    l0 * math::exp(alpha * math::ln(t / d0))
                }
            }
            Segment::Linear { d0, d1, l0, l1 } => l0 + (l1 - l0) * (t - d0) / (d1 - d0),
        }
    }

    /// ∫ᵤᵛ of the segment, `0 < u < v`.
    fn integral(&self, u: f64, v: f64) -> f64 {
        match *self {
            Segment::Power { alpha, .. } => {
                // Λ(t) = Λ(u)·(t/u)^α, so ∫ᵤᵛ Λ = Λ(u)·u·((v/u)^(α+1) − 1)/(α+1), with the
                // α = −1 limit Λ(u)·u·ln(v/u). expm1 keeps precision near that limit.
                let lu = self.value(u);
                if lu == 0.0 {
                    return 0.0;
                }
                let beta = alpha + 1.0;
                let l = math::ln(v / u);
                let x = beta * l;
                let factor = if x.abs() < 1e-9 {
                    l * (1.0 + 0.5 * x)
                } else {
                    math::exp_m1(x) / beta
                };
                lu * u * factor
            }
            Segment::Linear { .. } => 0.5 * (self.value(u) + self.value(v)) * (v - u),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * a.abs().max(b.abs()).max(1e-300)
    }

    /// A pure power law Λ(d) = c·d^α tabulated at a few points must interpolate and integrate
    /// exactly: ∫ₐᵇ c t^α dt = c (b^(α+1) − a^(α+1)) / (α+1).
    #[test]
    fn power_law_tables_integrate_exactly() {
        for &alpha in &[-0.5_f64, -1.0, -1.7, -3.0] {
            let c = 0.2;
            let days = vec![0.5, 1.0, 3.0, 10.0, 30.0];
            let lambda: Vec<f64> = days.iter().map(|&d| c * math::pow(d, alpha)).collect();
            let curve = BucketCurve::new(days, lambda, 30.0);
            curve.validate().unwrap();
            for &(a, b) in &[
                (0.5, 30.0),
                (0.7, 2.0),
                (1.0, 3.0),
                (2.5, 27.0),
                (30.0, 60.0),
            ] {
                let exact = if alpha == -1.0 {
                    c * math::ln(b / a)
                } else {
                    c * (math::pow(b, alpha + 1.0) - math::pow(a, alpha + 1.0)) / (alpha + 1.0)
                };
                let got = curve.integral(a, b);
                assert!(
                    close(got, exact, 1e-12),
                    "alpha {alpha} [{a},{b}]: {got} vs {exact}"
                );
            }
            assert!(close(
                curve.lambda_at(2.0),
                c * math::pow(2.0, alpha),
                1e-12
            ));
            // Past the last point the power law continues.
            assert!(close(
                curve.lambda_at(90.0),
                c * math::pow(90.0, alpha),
                1e-12
            ));
        }
    }

    #[test]
    fn flat_head_and_linear_segments_to_zero() {
        let curve = BucketCurve::new(vec![1.0, 2.0, 4.0], vec![0.5, 0.25, 0.0], 4.0);
        curve.validate().unwrap();
        // Held flat at 0.5 below day 1.
        assert_eq!(curve.lambda_at(0.0), 0.5);
        assert!(close(curve.integral(0.0, 1.0), 0.5, 1e-15));
        // Days 2-4: linear from 0.25 to 0: area 0.25.
        assert!(close(curve.integral(2.0, 4.0), 0.25, 1e-15));
        assert_eq!(curve.lambda_at(3.0), 0.125);
        // Zero beyond the end.
        assert_eq!(curve.lambda_at(10.0), 0.0);
        assert_eq!(curve.integral(4.0, 100.0), 0.0);
        // A constant Λ integrates to Λ·length: V = 10·3·0.1·3 = 9 for three days at weight 3.
        let flat = BucketCurve::new(vec![1.0], vec![0.1], 3.0);
        assert!(close(10.0 * 3.0 * flat.integral(0.0, 3.0), 9.0, 1e-14));
    }

    /// The closed form agrees with brute-force integration of the interpolant, and the integral is
    /// additive over adjacent ranges.
    #[test]
    fn closed_form_matches_numeric_and_is_additive() {
        let curve = BucketCurve::new(
            vec![0.04, 0.25, 1.0, 3.0, 7.0, 14.0, 30.0, 90.0],
            vec![0.97, 0.3, 0.03, 0.0098, 0.003, 0.0011, 0.0003, 0.00002],
            3.0,
        );
        curve.validate().unwrap();
        let (a, b) = (0.01, 60.0);
        let n = 200_000;
        let mut numeric = 0.0;
        let (la, lb) = (math::ln(a), math::ln(b));
        let mut prev_t = a;
        let mut prev_v = curve.lambda_at(a);
        for i in 1..=n {
            let t = math::exp(la + (lb - la) * f64::from(i) / f64::from(n));
            let v = curve.lambda_at(t);
            numeric += 0.5 * (prev_v + v) * (t - prev_t);
            prev_t = t;
            prev_v = v;
        }
        assert!(close(curve.integral(a, b), numeric, 1e-6));
        let split = curve.integral(a, 2.2) + curve.integral(2.2, 17.0) + curve.integral(17.0, b);
        assert!(close(curve.integral(a, b), split, 1e-12));
    }

    #[test]
    fn rejects_bad_tables() {
        let ok = BucketCurve::new(vec![1.0, 2.0], vec![0.2, 0.1], 2.0);
        assert!(ok.validate().is_ok());
        let cases = [
            BucketCurve::new(vec![], vec![], 1.0),
            BucketCurve::new(vec![1.0], vec![0.1, 0.2], 1.0),
            BucketCurve::new(vec![0.0, 1.0], vec![0.2, 0.1], 1.0),
            BucketCurve::new(vec![2.0, 1.0], vec![0.2, 0.1], 1.0),
            BucketCurve::new(vec![1.0, 2.0], vec![0.1, 0.2], 1.0),
            BucketCurve::new(vec![1.0, 2.0], vec![0.2, f64::NAN], 1.0),
            BucketCurve::new(vec![1.0, 2.0], vec![0.2, -0.1], 1.0),
            BucketCurve::new(vec![1.0, 2.0], vec![0.2, 0.1], -1.0),
        ];
        for c in cases {
            assert!(c.validate().is_err(), "{c:?}");
        }
        let mut shares = ok.clone();
        shares.part_shares.insert("heat".into(), -0.5);
        assert!(shares.validate().is_err());
    }
}
