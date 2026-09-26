//! The exceedance curve Λ_b(d) = Σ r · q · S(d) and what follows from it: the target for a
//! return period, the ladder value, natural frequencies, expected unmet days and the value of
//! covering days x₀ to x₁ (DESIGN §4.4, research §3.2).

use rr_types::math;
use rr_types::{BucketId, HazardId, TARGET_LADDER_DAYS};
use serde::Serialize;

use crate::model::{Model, Owner, Term, UParam};
use crate::survival::Survival;

/// Longest duration the solver looks at, in days (ten years).
pub(crate) const MAX_DAYS: f64 = 3650.0;
/// Shortest duration the solver resolves, in days (about 9 seconds).
pub(crate) const MIN_DAYS: f64 = 1e-4;

/// One bucket's terms, set up for evaluation under central values or one uncertainty draw.
pub(crate) struct Eval<'m> {
    terms: Vec<&'m Term>,
    ln_thr: Vec<f64>,
    w: Vec<f64>,
    ln_scale: Vec<f64>,
}

impl<'m> Eval<'m> {
    /// Central values for one bucket.
    pub fn central(model: &'m Model, bucket: BucketId) -> Eval<'m> {
        let terms: Vec<&Term> = model.bucket_terms(bucket).collect();
        Eval::from_terms(terms)
    }

    /// Central values for an explicit set of terms.
    pub fn from_terms(terms: Vec<&'m Term>) -> Eval<'m> {
        let ln_thr = terms
            .iter()
            .map(|t| {
                if t.threshold > 0.0 {
                    math::ln(t.threshold)
                } else {
                    f64::NEG_INFINITY
                }
            })
            .collect();
        let w = terms.iter().map(|t| t.rate * t.q).collect();
        let ln_scale = vec![0.0; terms.len()];
        Eval {
            terms,
            ln_thr,
            w,
            ln_scale,
        }
    }

    /// Resets the weights and duration scales for one draw: `z(param)` gives the standard-normal
    /// score of each uncertain parameter (0 = central).
    pub fn set_draw(&mut self, params: &[UParam], z: impl Fn(usize) -> f64) {
        for (i, t) in self.terms.iter().enumerate() {
            let mut rate = t.rate;
            if let Some(p) = t.rate_param {
                rate *= math::exp(params[p].ln_mult(z(p)));
            }
            let mut q = t.q;
            if let Some(p) = t.q_param {
                q = (q * math::exp(params[p].ln_mult(z(p)))).min(1.0);
            }
            self.w[i] = rate * q;
            self.ln_scale[i] = t.dur_param.map_or(0.0, |p| params[p].ln_mult(z(p)));
        }
    }

    /// Multiplies the current weights of the flagged terms by `factor`.
    pub fn scale_weights(&mut self, which: &[bool], factor: f64) {
        for (w, f) in self.w.iter_mut().zip(which) {
            if *f {
                *w *= factor;
            }
        }
    }

    /// The terms.
    pub fn terms(&self) -> &[&'m Term] {
        &self.terms
    }

    /// Λ(d): disruptions per year lasting longer than `d` days.
    pub fn lambda(&self, d: f64) -> f64 {
        if d.is_nan() || d <= 0.0 {
            return self.lambda0();
        }
        let ln_d = math::ln(d);
        let mut total = 0.0;
        for i in 0..self.terms.len() {
            let w = self.w[i];
            if w == 0.0 {
                continue;
            }
            let ln_x = if self.ln_thr[i] > ln_d {
                self.ln_thr[i]
            } else {
                ln_d
            };
            total += w * self.terms[i].survival.sf_ln(ln_x, self.ln_scale[i]);
        }
        total
    }

    /// Λ(0⁺): disruptions per year of any length.
    pub fn lambda0(&self) -> f64 {
        let mut total = 0.0;
        for i in 0..self.terms.len() {
            let w = self.w[i];
            if w == 0.0 {
                continue;
            }
            total += if self.ln_thr[i] > f64::NEG_INFINITY {
                w * self.terms[i]
                    .survival
                    .sf_ln(self.ln_thr[i], self.ln_scale[i])
            } else {
                w
            };
        }
        total
    }

    /// Each term's share of Λ(d) (all zero when Λ(d) is zero).
    pub fn shares(&self, d: f64) -> Vec<f64> {
        let parts: Vec<f64> = if d > 0.0 {
            let ln_d = math::ln(d);
            (0..self.terms.len())
                .map(|i| {
                    let ln_x = self.ln_thr[i].max(ln_d);
                    self.w[i] * self.terms[i].survival.sf_ln(ln_x, self.ln_scale[i])
                })
                .collect()
        } else {
            (0..self.terms.len())
                .map(|i| {
                    if self.ln_thr[i] > f64::NEG_INFINITY {
                        self.w[i]
                            * self.terms[i]
                                .survival
                                .sf_ln(self.ln_thr[i], self.ln_scale[i])
                    } else {
                        self.w[i]
                    }
                })
                .collect()
        };
        let total: f64 = parts.iter().sum();
        if total > 0.0 {
            parts.iter().map(|p| p / total).collect()
        } else {
            vec![0.0; parts.len()]
        }
    }

    /// The design duration: the smallest d with Λ(d) ≤ `rate` (continuous). 0 when disruptions
    /// of any length are rarer than `rate`; [`MAX_DAYS`] when Λ is still above `rate` at ten
    /// years. `iterations` bisection steps on ln d (30 give about 8 significant digits).
    pub fn target(&self, rate: f64, iterations: u32) -> f64 {
        if self.lambda0() <= rate {
            return 0.0;
        }
        if self.lambda(MAX_DAYS) > rate {
            return MAX_DAYS;
        }
        if self.lambda(MIN_DAYS) <= rate {
            return MIN_DAYS;
        }
        let (mut lo, mut hi) = (math::ln(MIN_DAYS), math::ln(MAX_DAYS));
        for _ in 0..iterations {
            let mid = 0.5 * (lo + hi);
            if self.lambda(math::exp(mid)) > rate {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        math::exp(hi)
    }

    /// The target on the day ladder: the smallest ladder value L with Λ(L) ≤ `rate`
    /// ([`TARGET_LADDER_DAYS`]); 0 when disruptions of any length are rarer than `rate`; 365 when
    /// even a year is not enough.
    pub fn ladder_target(&self, rate: f64) -> f32 {
        if self.lambda0() <= rate {
            return 0.0;
        }
        let ladder = &TARGET_LADDER_DAYS;
        let (mut lo, mut hi) = (0usize, ladder.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.lambda(f64::from(ladder[mid])) <= rate {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        if lo == ladder.len() {
            ladder[ladder.len() - 1]
        } else {
            ladder[lo]
        }
    }

    /// Out of 100 households like this one, how many face a disruption longer than `d` days in
    /// `years` years: 100 · (1 − e^(−years · Λ(d))).
    pub fn natural_frequency(&self, d: f64, years: f64) -> f64 {
        natural_frequency(self.lambda(d), years)
    }
}

/// 100 · (1 − e^(−years · λ)), computed without cancellation.
pub(crate) fn natural_frequency(lambda: f64, years: f64) -> f64 {
    (-100.0 * math::exp_m1(-years * lambda)).clamp(0.0, 100.0)
}

/// The yearly rate for a return period of `years` years: 1 / N.
pub fn dial_rate(return_period_years: u16) -> f64 {
    1.0 / f64::from(return_period_years)
}

/// Rounds a continuous duration up to the day ladder (the rule [`Eval::ladder_target`] applies
/// to Λ directly): 0 stays 0, anything above a year is 365.
pub fn round_up_to_ladder(days: f64) -> f32 {
    if days.is_nan() || days <= 0.0 {
        return 0.0;
    }
    for &l in &TARGET_LADDER_DAYS {
        if f64::from(l) >= days {
            return l;
        }
    }
    TARGET_LADDER_DAYS[TARGET_LADDER_DAYS.len() - 1]
}

/// One term of an exported curve.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveTerm {
    /// Events per year that cause this disruption (r · q).
    pub weight: f64,
    /// How long they last.
    pub survival: Survival,
    /// The disruption counts only when the underlying event outlasts this many days.
    pub threshold: f64,
    /// The hazard it belongs to.
    pub hazard: HazardId,
}

/// A bucket's exceedance curve at central parameter values, self-contained so the budget and plan
/// crates can integrate it (DESIGN §4.7: the value of covering day x is Λ(x)).
#[derive(Debug, Clone, PartialEq)]
pub struct ExceedanceCurve {
    /// Which bucket.
    pub bucket: BucketId,
    /// The terms Λ is summed over.
    pub terms: Vec<CurveTerm>,
    /// The design duration at the household's return period, before ladder rounding (days).
    pub target_days: f64,
    /// The target on the day ladder.
    pub ladder_days: f32,
}

impl ExceedanceCurve {
    pub(crate) fn from_eval(bucket: BucketId, eval: &Eval<'_>, rate: f64) -> ExceedanceCurve {
        let terms = eval
            .terms()
            .iter()
            .map(|t| CurveTerm {
                weight: t.rate * t.q,
                survival: t.survival.clone(),
                threshold: t.threshold,
                hazard: t.hazard,
            })
            .collect();
        ExceedanceCurve {
            bucket,
            terms,
            target_days: eval.target(rate, 40),
            ladder_days: eval.ladder_target(rate),
        }
    }

    /// Λ(d): disruptions per year lasting longer than `d` days.
    pub fn lambda(&self, d: f64) -> f64 {
        self.terms
            .iter()
            .map(|t| t.weight * t.survival.sf(d.max(t.threshold), 0.0))
            .sum()
    }

    /// Expected days per year of this disruption that last beyond day `x`:
    /// U(x) = ∫ₓ^∞ Λ(t) dt. `unmet_days(0)` is the consumption rate (days per year of
    /// disruption, the Σ r·q·E[D] of DESIGN §4.4).
    pub fn unmet_days(&self, x: f64) -> f64 {
        let x = x.max(0.0);
        self.terms
            .iter()
            .map(|t| {
                if t.weight == 0.0 {
                    0.0
                } else if x >= t.threshold {
                    t.weight * t.survival.expected_excess(x, 0.0)
                } else {
                    t.weight
                        * ((t.threshold - x) * t.survival.sf(t.threshold, 0.0)
                            + t.survival.expected_excess(t.threshold, 0.0))
                }
            })
            .sum()
    }

    /// Expected disruption-days per year covered by holding days `x0` to `x1` of supplies:
    /// ∫_{x0}^{x1} Λ(t) dt = U(x0) − U(x1). Zero when `x1 <= x0`. The budget multiplies by ten
    /// years and the harm weight and caps `x1` at the target (DESIGN §4.7).
    pub fn value_between(&self, x0: f64, x1: f64) -> f64 {
        if x1.is_nan() || x1 <= x0 {
            return 0.0;
        }
        (self.unmet_days(x0) - self.unmet_days(x1)).max(0.0)
    }

    /// Days of this disruption per year on average (for rotation and cost).
    pub fn consumption_days_per_year(&self) -> f64 {
        self.unmet_days(0.0)
    }

    /// Out of 100 households like this one, how many face a disruption longer than `d` days in
    /// `years` years.
    pub fn natural_frequency(&self, d: f64, years: f64) -> f64 {
        natural_frequency(self.lambda(d), years)
    }

    /// The design duration for any yearly rate (continuous days): the smallest d with Λ(d) ≤
    /// `rate`; 0 when disruptions of any length are rarer than `rate`.
    pub fn target_at(&self, rate: f64) -> f64 {
        let total: f64 = self
            .terms
            .iter()
            .map(|t| {
                if t.threshold > 0.0 {
                    t.weight * t.survival.sf(t.threshold, 0.0)
                } else {
                    t.weight
                }
            })
            .sum();
        if total <= rate {
            return 0.0;
        }
        if self.lambda(MAX_DAYS) > rate {
            return MAX_DAYS;
        }
        let (mut lo, mut hi) = (math::ln(MIN_DAYS), math::ln(MAX_DAYS));
        if self.lambda(MIN_DAYS) <= rate {
            return MIN_DAYS;
        }
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            if self.lambda(math::exp(mid)) > rate {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        math::exp(hi)
    }

    /// The ladder target for any yearly rate (the smallest ladder value L with Λ(L) ≤ `rate`).
    pub fn ladder_at(&self, rate: f64) -> f32 {
        if self.target_at(rate) == 0.0 {
            return 0.0;
        }
        TARGET_LADDER_DAYS
            .iter()
            .copied()
            .find(|&l| self.lambda(f64::from(l)) <= rate)
            .unwrap_or(TARGET_LADDER_DAYS[TARGET_LADDER_DAYS.len() - 1])
    }

    /// Samples Λ at `n` log-spaced durations from one hour to `max_days` (for the budget crate's
    /// piecewise-linear curve and for charts): `(days, lambda)`.
    pub fn sample(&self, n: usize, max_days: f64) -> (Vec<f64>, Vec<f64>) {
        let n = n.max(2);
        let lo = math::ln(1.0 / 24.0);
        let hi = math::ln(max_days.max(1.0));
        let days: Vec<f64> = (0..n)
            .map(|i| math::exp(lo + (hi - lo) * i as f64 / (n - 1) as f64))
            .collect();
        let lambda = days.iter().map(|&d| self.lambda(d)).collect();
        (days, lambda)
    }
}

/// Shares of Λ(d) by owner (hazard or scenario), largest first; ties in owner order.
pub(crate) fn owner_shares(eval: &Eval<'_>, d: f64) -> Vec<(Owner, f64)> {
    let shares = eval.shares(d);
    let mut by: std::collections::BTreeMap<Owner, f64> = std::collections::BTreeMap::new();
    for (t, s) in eval.terms().iter().zip(shares) {
        *by.entry(t.owner()).or_insert(0.0) += s;
    }
    let mut v: Vec<(Owner, f64)> = by.into_iter().filter(|(_, s)| *s > 0.0).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// Serializable dial-table row for the expert view and the calibration report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DialPoint {
    /// Return period in years.
    pub return_period_years: u16,
    /// Continuous design duration in days.
    pub target_days: f64,
    /// On the day ladder.
    pub ladder_days: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_rounding_is_a_ceiling() {
        assert_eq!(round_up_to_ladder(0.0), 0.0);
        assert_eq!(round_up_to_ladder(0.2), 0.5);
        assert_eq!(round_up_to_ladder(2.8), 3.0);
        assert_eq!(round_up_to_ladder(3.0), 3.0);
        assert_eq!(round_up_to_ladder(3.001), 5.0);
        assert_eq!(round_up_to_ladder(12.0), 14.0);
        assert_eq!(round_up_to_ladder(200.0), 365.0);
        assert_eq!(round_up_to_ladder(5000.0), 365.0);
    }

    #[test]
    fn dial_rates_are_one_over_n() {
        assert_eq!(dial_rate(100), 0.01);
        assert_eq!(dial_rate(10), 0.1);
    }

    #[test]
    fn natural_frequency_matches_the_formula() {
        // Λ = 0.0105 over 10 years: 100·(1 − e^(−0.105)) = 9.9675… (Python math.expm1).
        let n = natural_frequency(0.0105, 10.0);
        assert!((n - 9.967_547_741_373_437).abs() < 1e-12, "{n}");
        assert_eq!(natural_frequency(0.0, 10.0), 0.0);
        assert!(natural_frequency(50.0, 10.0) <= 100.0);
    }
}
