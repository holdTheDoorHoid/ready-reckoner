//! The exceedance curve Λ_b(d) = Σ r · q · S(d) and what follows from it: the target for a
//! return period, the ladder value, natural frequencies, expected unmet days and the value of
//! covering days x₀ to x₁ (DESIGN §4.4, research §3.2).

use rr_types::math;
use rr_types::{BucketId, HazardId, ReturnPeriod, TARGET_LADDER_DAYS};
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
    /// Threshold and scale of each term's floor (see [`crate::model::MaxWith`]).
    ln_thr2: Vec<f64>,
    ln_scale2: Vec<f64>,
}

fn ln_threshold(t: f64) -> f64 {
    if t > 0.0 {
        math::ln(t)
    } else {
        f64::NEG_INFINITY
    }
}

impl<'m> Eval<'m> {
    /// Central values for one bucket.
    pub fn central(model: &'m Model, bucket: BucketId) -> Eval<'m> {
        let terms: Vec<&Term> = model.bucket_terms(bucket).collect();
        Eval::from_terms(terms)
    }

    /// Central values for an explicit set of terms.
    pub fn from_terms(terms: Vec<&'m Term>) -> Eval<'m> {
        let ln_thr = terms.iter().map(|t| ln_threshold(t.threshold)).collect();
        let ln_thr2 = terms
            .iter()
            .map(|t| {
                t.max_with
                    .as_ref()
                    .map_or(f64::NEG_INFINITY, |m| ln_threshold(m.threshold))
            })
            .collect();
        let w = terms.iter().map(|t| t.rate * t.q).collect();
        let n = terms.len();
        Eval {
            terms,
            ln_thr,
            w,
            ln_scale: vec![0.0; n],
            ln_thr2,
            ln_scale2: vec![0.0; n],
        }
    }

    /// Back to central values.
    pub fn reset(&mut self) {
        for (k, t) in self.terms.iter().enumerate() {
            self.w[k] = t.rate * t.q;
            self.ln_scale[k] = 0.0;
            self.ln_scale2[k] = 0.0;
        }
    }

    /// Sets the weights and duration scales for draw `i` from precomputed multipliers (the fast
    /// path of [`Eval::set_draw`]; same result).
    pub fn set_draw_from(&mut self, draws: &crate::ranges::Draws, i: usize) {
        for (k, t) in self.terms.iter().enumerate() {
            let mut rate = t.rate;
            if let Some(p) = t.rate_param {
                rate *= draws.mult(p, i);
            }
            let mut q = t.q;
            if let Some(p) = t.q_param {
                q = (q * draws.mult(p, i)).min(1.0);
            }
            self.w[k] = rate * q;
            self.ln_scale[k] = t.dur_param.map_or(0.0, |p| draws.lnm(p, i));
            self.ln_scale2[k] = t
                .max_with
                .as_ref()
                .and_then(|m| m.dur_param)
                .map_or(0.0, |p| draws.lnm(p, i));
        }
    }

    /// Like [`Eval::target`], searching first between `lo` and `hi` days (a bracket around a known
    /// answer needs fewer steps); falls back to the full search when the answer is outside.
    pub fn target_near(&self, rate: f64, lo: f64, hi: f64, iterations: u32) -> f64 {
        if self.lambda0() <= rate {
            return 0.0;
        }
        let (lo, hi) = (lo.max(MIN_DAYS), hi.min(MAX_DAYS));
        if lo >= hi || self.lambda(lo) <= rate || self.lambda(hi) > rate {
            return self.target(rate, iterations + 12);
        }
        let (mut a, mut b) = (math::ln(lo), math::ln(hi));
        for _ in 0..iterations {
            let mid = 0.5 * (a + b);
            if self.lambda(math::exp(mid)) > rate {
                a = mid;
            } else {
                b = mid;
            }
        }
        math::exp(b)
    }

    /// Term `i`'s survival at ln d (thresholds, draws and floor applied).
    #[inline]
    fn term_sf(&self, i: usize, ln_d: f64) -> f64 {
        let t = self.terms[i];
        let ln_x = if self.ln_thr[i] > ln_d {
            self.ln_thr[i]
        } else {
            ln_d
        };
        let s = t.survival.sf_ln(ln_x, self.ln_scale[i]);
        match &t.max_with {
            None => s,
            Some(m) => {
                let ln_x2 = if self.ln_thr2[i] > ln_d {
                    self.ln_thr2[i]
                } else {
                    ln_d
                };
                s.max(m.survival.sf_ln(ln_x2, self.ln_scale2[i]))
            }
        }
    }

    /// Term `i`'s survival as d → 0⁺.
    fn term_sf0(&self, i: usize) -> f64 {
        let t = self.terms[i];
        let s = if self.ln_thr[i] > f64::NEG_INFINITY {
            t.survival.sf_ln(self.ln_thr[i], self.ln_scale[i])
        } else {
            1.0
        };
        match &t.max_with {
            None => s,
            Some(m) => {
                let s2 = if self.ln_thr2[i] > f64::NEG_INFINITY {
                    m.survival.sf_ln(self.ln_thr2[i], self.ln_scale2[i])
                } else {
                    1.0
                };
                s.max(s2)
            }
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
            self.ln_scale2[i] = t
                .max_with
                .as_ref()
                .and_then(|m| m.dur_param)
                .map_or(0.0, |p| params[p].ln_mult(z(p)));
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
            if self.w[i] != 0.0 {
                total += self.w[i] * self.term_sf(i, ln_d);
            }
        }
        total
    }

    /// Λ(0⁺): disruptions per year of any length.
    pub fn lambda0(&self) -> f64 {
        let mut total = 0.0;
        for i in 0..self.terms.len() {
            if self.w[i] != 0.0 {
                total += self.w[i] * self.term_sf0(i);
            }
        }
        total
    }

    /// Each term's share of Λ(d) (all zero when Λ(d) is zero).
    pub fn shares(&self, d: f64) -> Vec<f64> {
        let parts: Vec<f64> = if d > 0.0 {
            let ln_d = math::ln(d);
            (0..self.terms.len())
                .map(|i| self.w[i] * self.term_sf(i, ln_d))
                .collect()
        } else {
            (0..self.terms.len())
                .map(|i| self.w[i] * self.term_sf0(i))
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

    /// The target on the day ladder ([`TARGET_LADDER_DAYS`]): the raw design duration rounded up,
    /// where a raw value within [`LADDER_TOLERANCE`] above a step counts as that step. In terms of
    /// Λ: the smallest ladder value L with Λ(L · 1.03) ≤ `rate`. 0 when disruptions of any length
    /// are rarer than `rate`; 365 when even a year is not enough.
    pub fn ladder_target(&self, rate: f64) -> f32 {
        if self.lambda0() <= rate {
            return 0.0;
        }
        let ladder = &TARGET_LADDER_DAYS;
        let (mut lo, mut hi) = (0usize, ladder.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.lambda(step_limit(ladder[mid])) <= rate {
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

    /// The ladder target for one draw, searching outward from ladder index `start` (the central
    /// answer, where most draws land) and remembering Λ at each step's limit in `cache` (NaN =
    /// not computed yet). Same result as [`Eval::ladder_target`].
    pub fn ladder_target_from(&self, rate: f64, start: usize, cache: &mut [f64; 15]) -> f32 {
        let ladder = &TARGET_LADDER_DAYS;
        let n = ladder.len();
        let at = |k: usize, cache: &mut [f64; 15]| -> f64 {
            if cache[k].is_nan() {
                cache[k] = self.lambda(step_limit(ladder[k]));
            }
            cache[k]
        };
        let mut k = start.min(n - 1);
        if at(k, cache) <= rate {
            // Walk down while the next step down still meets the rate.
            while k > 0 && at(k - 1, cache) <= rate {
                k -= 1;
            }
            if k == 0 && self.lambda0() <= rate {
                return 0.0;
            }
            ladder[k]
        } else {
            // Walk up to the first step that meets the rate.
            while k + 1 < n {
                k += 1;
                if at(k, cache) <= rate {
                    return ladder[k];
                }
            }
            ladder[n - 1]
        }
    }
}

/// A raw target within this fraction above a ladder step counts as that step (planner decision,
/// 2026-09-25: 3.003 days is "3 days", not 5).
pub const LADDER_TOLERANCE: f64 = 0.03;

/// The largest raw value that still rounds to ladder step `step`.
#[inline]
pub(crate) fn step_limit(step: f32) -> f64 {
    f64::from(step) * (1.0 + LADDER_TOLERANCE)
}

/// Index of a ladder value (or of the smallest ladder value at or above `days`).
pub(crate) fn ladder_index(days: f64) -> usize {
    TARGET_LADDER_DAYS
        .iter()
        .position(|&l| f64::from(l) >= days)
        .unwrap_or(TARGET_LADDER_DAYS.len() - 1)
}

/// 100 · (1 − e^(−years · λ)), computed without cancellation.
pub(crate) fn natural_frequency(lambda: f64, years: f64) -> f64 {
    (-100.0 * math::exp_m1(-years * lambda)).clamp(0.0, 100.0)
}

/// The default dial, "90 % sure nothing in the next ten years is worse": Λ* = −ln(0.9)/10
/// = 0.010536051565782628 a year, shown as "about 1 in 100" (planner decision, 2026-09-25;
/// research §3.3).
pub const ONE_IN_100_RATE: f64 = 0.010_536_051_565_782_628;

/// The yearly rate behind each dial setting: `one_in_10` 0.1, `one_in_50` 0.02, `one_in_100`
/// [`ONE_IN_100_RATE`] (≈ 1 in 95), `one_in_500` 0.002.
pub fn dial_rate(rp: ReturnPeriod) -> f64 {
    match rp {
        ReturnPeriod::OneIn10 => 0.1,
        ReturnPeriod::OneIn50 => 0.02,
        ReturnPeriod::OneIn100 => ONE_IN_100_RATE,
        ReturnPeriod::OneIn500 => 0.002,
    }
}

/// Rounds a raw duration up to the day ladder, a value within [`LADDER_TOLERANCE`] above a step
/// counting as that step (the rule [`Eval::ladder_target`] applies through Λ): 0 stays 0,
/// anything above a year is 365.
pub fn round_up_to_ladder(days: f64) -> f32 {
    if days.is_nan() || days <= 0.0 {
        return 0.0;
    }
    for &l in &TARGET_LADDER_DAYS {
        if step_limit(l) >= days {
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
    /// A floor: the disruption lasts at least as long as this other one (survival and threshold),
    /// for example a well's water outage and the power cut that stops its pump.
    pub floor: Option<(Survival, f64)>,
    /// The hazard it belongs to.
    pub hazard: HazardId,
}

impl CurveTerm {
    /// P(this disruption lasts longer than `d`), thresholds and floor applied.
    pub fn sf(&self, d: f64) -> f64 {
        let s = self.survival.sf(d.max(self.threshold), 0.0);
        match &self.floor {
            None => s,
            Some((f, thr)) => s.max(f.sf(d.max(*thr), 0.0)),
        }
    }

    /// ∫ₓ^∞ P(duration > t) dt for this term (unweighted).
    pub fn excess(&self, x: f64) -> f64 {
        let x = x.max(0.0);
        match &self.floor {
            None => {
                if x >= self.threshold {
                    self.survival.expected_excess(x, 0.0)
                } else {
                    (self.threshold - x) * self.survival.sf(self.threshold, 0.0)
                        + self.survival.expected_excess(self.threshold, 0.0)
                }
            }
            Some(_) => integrate_tail(|t| self.sf(t), x),
        }
    }
}

/// ∫ₓ^∞ f(t) dt for a non-increasing survival-like f, by Simpson's rule on ln t from max(x, 10⁻⁴)
/// to 10⁵ days (the part below 10⁻⁴ days is f(x) times its length).
fn integrate_tail(f: impl Fn(f64) -> f64, x: f64) -> f64 {
    const LO: f64 = 1e-4;
    const HI: f64 = 1e5;
    const N: usize = 600; // even; ~0.035 per step in ln t
    let start = x.max(LO);
    if start >= HI {
        return 0.0;
    }
    let (a, b) = (math::ln(start), math::ln(HI));
    let h = (b - a) / N as f64;
    let g = |u: f64| {
        let t = math::exp(u);
        f(t) * t
    };
    let mut sum = g(a) + g(b);
    for k in 1..N {
        let u = a + h * k as f64;
        sum += if k % 2 == 1 { 4.0 } else { 2.0 } * g(u);
    }
    let head = if x < LO { (LO - x) * f(x) } else { 0.0 };
    head + sum * h / 3.0
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
    pub(crate) fn from_eval(
        bucket: BucketId,
        eval: &Eval<'_>,
        target_days: f64,
        ladder_days: f32,
    ) -> ExceedanceCurve {
        let terms = eval
            .terms()
            .iter()
            .map(|t| CurveTerm {
                weight: t.rate * t.q,
                survival: t.survival.clone(),
                threshold: t.threshold,
                floor: t
                    .max_with
                    .as_ref()
                    .map(|m| (m.survival.clone(), m.threshold)),
                hazard: t.hazard,
            })
            .collect();
        ExceedanceCurve {
            bucket,
            terms,
            target_days,
            ladder_days,
        }
    }

    /// Λ(d): disruptions per year lasting longer than `d` days.
    pub fn lambda(&self, d: f64) -> f64 {
        self.terms.iter().map(|t| t.weight * t.sf(d)).sum()
    }

    /// Expected days per year of this disruption that last beyond day `x`:
    /// U(x) = ∫ₓ^∞ Λ(t) dt. `unmet_days(0)` is the consumption rate (days per year of
    /// disruption, the Σ r·q·E[D] of DESIGN §4.4).
    pub fn unmet_days(&self, x: f64) -> f64 {
        self.terms
            .iter()
            .filter(|t| t.weight != 0.0)
            .map(|t| t.weight * t.excess(x))
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
        let total: f64 = self.terms.iter().map(|t| t.weight * t.sf(0.0)).sum();
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

    /// The ladder target for any yearly rate (same rule as [`Eval::ladder_target`]).
    pub fn ladder_at(&self, rate: f64) -> f32 {
        if self.target_at(rate) == 0.0 {
            return 0.0;
        }
        TARGET_LADDER_DAYS
            .iter()
            .copied()
            .find(|&l| self.lambda(step_limit(l)) <= rate)
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
        // Within 3 % above a step counts as the step.
        assert_eq!(round_up_to_ladder(3.003), 3.0);
        assert_eq!(round_up_to_ladder(3.09), 3.0);
        assert_eq!(round_up_to_ladder(3.1), 5.0);
        assert_eq!(round_up_to_ladder(61.0), 60.0);
        assert_eq!(round_up_to_ladder(62.0), 90.0);
        assert_eq!(round_up_to_ladder(12.0), 14.0);
        assert_eq!(round_up_to_ladder(200.0), 365.0);
        assert_eq!(round_up_to_ladder(5000.0), 365.0);
    }

    #[test]
    fn dial_rates_follow_the_planner_decision() {
        assert_eq!(dial_rate(ReturnPeriod::OneIn10), 0.1);
        assert_eq!(dial_rate(ReturnPeriod::OneIn50), 0.02);
        assert_eq!(dial_rate(ReturnPeriod::OneIn500), 0.002);
        // −ln(0.9)/10, "90 % sure nothing in the next ten years is worse".
        let r = dial_rate(ReturnPeriod::OneIn100);
        assert!((r - (-math::ln(0.9) / 10.0)).abs() < 1e-18, "{r}");
        assert!((natural_frequency(r, 10.0) - 10.0).abs() < 1e-12);
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
