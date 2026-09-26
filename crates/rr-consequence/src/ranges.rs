//! Ranges under parameter uncertainty (research §3.6) and the parameters that drive them.
//!
//! **Monte Carlo.** Every uncertain input (a hazard's rate, an effect's share, a duration scale)
//! is a named [`UParam`]. Draws use Latin hypercube sampling: the `N` standard-normal scores
//! Φ⁻¹((k + ½)/N) are shuffled separately for each parameter by a [`SplitMix64`] seeded from a
//! fixed constant and the parameter's name. So:
//!
//! - the output is deterministic (same input, same draws, same ranges, on every target);
//! - a parameter's draws do not depend on which other parameters exist (turning on a well does
//!   not reshuffle the power draws), and a term derived by a coupling rule shares its source's
//!   draws, so a well household's water curve is never below its power curve in any draw;
//! - the same draws serve every dial setting, so the 10th and 90th percentiles move with the dial
//!   in the same direction as the central value.
//!
//! **Drivers.** One at a time, each input that feeds a bucket's design event is set to its 10th
//! and then its 90th percentile with everything else central; the one or two that move the
//! target most are named.

use rr_types::Evidence;
use rr_types::math::Z_90;
use rr_types::rng::SplitMix64;

use crate::curve::Eval;
use crate::income::IncomeEval;
use crate::model::UParam;
use crate::survival::probit;

/// Draws per assessment. Chosen so `assess` stays well under 40 ms in WebAssembly for the
/// fixtures (see docs/RISK_MODEL.md, "Ranges"); 400 draws put the 10th and 90th percentiles
/// within about ±1.5 percentile points.
pub const DRAWS: usize = 400;

/// Seed for every draw sequence (combined with each parameter's name).
pub const BASE_SEED: u64 = 0x5EED_2026_0925_0001;

/// FNV-1a, 64-bit: a stable hash of a parameter name.
fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// Standard-normal scores for every parameter and draw.
pub(crate) struct Draws {
    n: usize,
    z: Vec<f64>,
}

impl Draws {
    pub fn new(params: &[UParam], n: usize) -> Draws {
        let nf = n as f64;
        let grid: Vec<f64> = (0..n).map(|k| probit((k as f64 + 0.5) / nf)).collect();
        let mut z = vec![0.0; params.len() * n];
        let mut perm: Vec<usize> = Vec::with_capacity(n);
        for (p, param) in params.iter().enumerate() {
            let mut rng = SplitMix64::new(BASE_SEED ^ fnv1a(&param.key));
            perm.clear();
            perm.extend(0..n);
            for i in (1..n).rev() {
                let j = rng.next_below((i + 1) as u64) as usize;
                perm.swap(i, j);
            }
            for (i, &k) in perm.iter().enumerate() {
                z[p * n + i] = grid[k];
            }
        }
        Draws { n, z }
    }

    pub fn n(&self) -> usize {
        self.n
    }

    #[inline]
    pub fn z(&self, param: usize, draw: usize) -> f64 {
        self.z[param * self.n + draw]
    }
}

/// The `q`-quantile of `values` by the nearest-rank rule (sorts in place).
pub(crate) fn quantile(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let n = values.len();
    let rank = ((q * n as f64).ceil() as usize).clamp(1, n);
    values[rank - 1]
}

/// 10th and 90th percentiles of a duration bucket's ladder target, and of its natural frequencies
/// at `days`.
pub(crate) struct DurationRange {
    pub low: f32,
    pub high: f32,
    /// (low, high) natural frequency for each of the requested durations.
    pub freq: Vec<(f64, f64)>,
}

pub(crate) fn duration_range(
    eval: &mut Eval<'_>,
    params: &[UParam],
    draws: &Draws,
    rate: f64,
    days: &[f64],
    years: f64,
) -> DurationRange {
    let n = draws.n();
    let mut targets = Vec::with_capacity(n);
    let mut freqs: Vec<Vec<f64>> = vec![Vec::with_capacity(n); days.len()];
    for i in 0..n {
        eval.set_draw(params, |p| draws.z(p, i));
        targets.push(f64::from(eval.ladder_target(rate)));
        for (j, &d) in days.iter().enumerate() {
            freqs[j].push(eval.natural_frequency(d, years));
        }
    }
    eval.set_draw(params, |_| 0.0);
    let low = quantile(&mut targets, 0.1) as f32;
    let high = quantile(&mut targets, 0.9) as f32;
    let freq = freqs
        .iter_mut()
        .map(|v| (quantile(v, 0.1), quantile(v, 0.9)))
        .collect();
    DurationRange { low, high, freq }
}

/// 10th and 90th percentiles of a readiness bucket's yearly need rate.
pub(crate) fn rate_range(eval: &mut Eval<'_>, params: &[UParam], draws: &Draws) -> (f64, f64) {
    let n = draws.n();
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        eval.set_draw(params, |p| draws.z(p, i));
        v.push(eval.lambda0());
    }
    eval.set_draw(params, |_| 0.0);
    (quantile(&mut v, 0.1), quantile(&mut v, 0.9))
}

/// 10th and 90th percentiles of the income target (months) and of Λ at `months`.
pub(crate) fn income_range(
    eval: &mut IncomeEval<'_>,
    params: &[UParam],
    draws: &Draws,
    rate: f64,
    months: f64,
) -> (f32, f32, f64, f64) {
    let n = draws.n();
    let mut t = Vec::with_capacity(n);
    let mut l = Vec::with_capacity(n);
    for i in 0..n {
        eval.set_draw(params, |p| draws.z(p, i));
        t.push(f64::from(eval.ladder_target(rate)));
        l.push(eval.lambda(months));
    }
    eval.set_draw(params, |_| 0.0);
    (
        quantile(&mut t, 0.1) as f32,
        quantile(&mut t, 0.9) as f32,
        quantile(&mut l, 0.1),
        quantile(&mut l, 0.9),
    )
}

/// An input that moves a bucket's target.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Driver {
    pub param: usize,
    pub label: String,
    pub evidence: Evidence,
    /// Target with this input at its 10th and 90th percentile (continuous units of the bucket).
    pub at_low: f64,
    pub at_high: f64,
    swing: f64,
}

impl Driver {
    /// "how long power cuts from big ice storms last (expert estimate)".
    pub fn phrase(&self) -> String {
        let tag = match self.evidence {
            Evidence::Prior => "expert estimate",
            Evidence::Empirical => "from records",
        };
        format!("{} ({tag})", self.label)
    }
}

/// Evaluates the target with the uncertain inputs at the given standard-normal scores.
pub(crate) type TargetAt<'a> = dyn FnMut(&dyn Fn(usize) -> f64) -> f64 + 'a;

/// The inputs that move the target most, largest first (at most `keep`, and only those that move
/// it by 10 % or more). `candidates` are parameter ids in order of how much their terms feed the
/// design event.
pub(crate) fn drivers(
    candidates: &[usize],
    params: &[UParam],
    keep: usize,
    target_at: &mut TargetAt<'_>,
) -> Vec<Driver> {
    let eps = 0.05;
    let mut out: Vec<Driver> = Vec::new();
    for &p in candidates {
        let lo = target_at(&|q| if q == p { -Z_90 } else { 0.0 });
        let hi = target_at(&|q| if q == p { Z_90 } else { 0.0 });
        let swing = rr_types::math::ln((hi.max(lo) + eps) / (hi.min(lo) + eps));
        out.push(Driver {
            param: p,
            label: params[p].label.clone(),
            evidence: params[p].evidence,
            at_low: lo,
            at_high: hi,
            swing,
        });
    }
    out.sort_by(|a, b| b.swing.total_cmp(&a.swing).then(a.param.cmp(&b.param)));
    let min_swing = rr_types::math::ln(1.1);
    out.retain(|d| d.swing >= min_swing);
    out.truncate(keep);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(key: &str) -> UParam {
        UParam {
            key: key.to_owned(),
            sigma_lo: 0.5,
            sigma_hi: 0.5,
            label: key.to_owned(),
            evidence: Evidence::Prior,
        }
    }

    #[test]
    fn draws_are_stratified_and_keyed_by_name() {
        let a = Draws::new(&[param("rate:x"), param("rate:y")], 100);
        let b = Draws::new(&[param("rate:y")], 100);
        // Same name, same draws, whatever else is in the set.
        for i in 0..100 {
            assert_eq!(a.z(1, i).to_bits(), b.z(0, i).to_bits());
        }
        // Each parameter uses every stratum exactly once.
        let mut v: Vec<f64> = (0..100).map(|i| a.z(0, i)).collect();
        v.sort_by(|x, y| x.total_cmp(y));
        for (k, z) in v.iter().enumerate() {
            assert_eq!(z.to_bits(), probit((k as f64 + 0.5) / 100.0).to_bits());
        }
        // Different names shuffle differently.
        assert!((0..100).any(|i| a.z(0, i) != a.z(1, i)));
    }

    #[test]
    fn nearest_rank_quantiles() {
        let mut v: Vec<f64> = (1..=10).map(f64::from).collect();
        assert_eq!(quantile(&mut v, 0.1), 1.0);
        assert_eq!(quantile(&mut v, 0.9), 9.0);
        assert_eq!(quantile(&mut v, 0.5), 5.0);
        assert_eq!(quantile(&mut [], 0.5), 0.0);
    }
}
